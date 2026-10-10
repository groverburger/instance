//! Compiles a [`SongSrc`] into a [`Song`]: absolute-tick notes and
//! controller messages per track, a tempo map, section markers and the loop
//! range. Musical checks (bar checks, part lengths) produce warnings.

use crate::song::{bar_ticks, ArrEntry, Ctl, Item, Mods, Pitch, SongSrc, TrackDef, PPQ};
use crate::theory::{voice_chord, Chord, Key};

/// A sounding note.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub tick: u64,
    /// Sounding length in ticks (after gate/staccato).
    pub len: u64,
    pub key: u8,
    pub vel: u8,
}

/// A compiled track.
#[derive(Clone, Debug)]
pub struct Track {
    pub def: TrackDef,
    /// 0-based MIDI channel.
    pub channel: u8,
    pub notes: Vec<Note>,
    /// Channel messages other than notes (controllers, program changes,
    /// pitch bend), status byte included.
    pub ctls: Vec<(u64, Vec<u8>)>,
}

/// A compiled song.
#[derive(Clone, Debug)]
pub struct Song {
    pub title: String,
    pub variant: Option<String>,
    /// (tick, BPM)
    pub tempos: Vec<(u64, f64)>,
    pub times: Vec<(u64, (u32, u32))>,
    pub key: Option<Key>,
    pub tracks: Vec<Track>,
    /// Section starts (tick, name).
    pub markers: Vec<(u64, String)>,
    pub end: u64,
    /// Loop region; the whole song unless `|:`/`:|` were used.
    pub loop_range: (u64, u64),
    /// Whether the arrangement marks a loop (`|:` or `:|`).
    pub looping: bool,
    pub warnings: Vec<String>,
}

impl Song {
    /// Seconds at `tick`.
    pub fn secs_at(&self, tick: u64) -> f64 {
        let mut secs = 0.0;
        let mut last = 0u64;
        let mut bpm = self.tempos.first().map(|t| t.1).unwrap_or(120.0);
        for &(t, b) in &self.tempos {
            if t >= tick {
                break;
            }
            secs += (t - last) as f64 * 60.0 / (bpm * PPQ as f64);
            last = t;
            bpm = b;
        }
        secs + (tick - last) as f64 * 60.0 / (bpm * PPQ as f64)
    }

    /// Length in seconds.
    pub fn duration(&self) -> f64 {
        self.secs_at(self.end)
    }

    /// Bar length (ticks) at `tick`.
    pub fn bar_at(&self, tick: u64) -> u64 {
        let mut ts = (4, 4);
        for &(t, s) in &self.times {
            if t <= tick {
                ts = s;
            }
        }
        bar_ticks(ts) as u64
    }

    /// The MIDI messages of a track, sorted: (tick, message). At equal ticks
    /// note-offs come first, then controllers, then note-ons. Includes the
    /// track's set-up messages at tick 0.
    pub fn track_events(&self, tr: &Track) -> Vec<(u64, Vec<u8>)> {
        let ch = tr.channel;
        let d = &tr.def;
        let mut ev: Vec<(u64, u8, Vec<u8>)> = Vec::new();
        let cc = |n: u8, v: u8| vec![0xB0 | ch, n, v.min(127)];
        let mut init = vec![cc(0, 0), cc(32, 0), vec![0xC0 | ch, d.program], cc(7, d.vol), cc(10, pan_value(d.pan as f64)), cc(11, d.expr), cc(91, d.reverb), cc(93, d.chorus)];
        // synth options that are also inline controls start at the track's value
        for (k, v) in &d.synth_opts {
            let n = match k.as_str() {
                "cutoff" => 74,
                "reso" => 71,
                "attack" => 73,
                "release" => 72,
                "delay" => 94,
                _ => continue,
            };
            init.push(cc(n, v.round().clamp(0.0, 127.0) as u8));
        }
        if d.bend_range != 2 {
            init.extend([cc(101, 0), cc(100, 0), cc(6, d.bend_range), cc(38, 0), cc(101, 127), cc(100, 127)]);
        }
        for m in init {
            ev.push((0, 1, m));
        }
        for (t, m) in &tr.ctls {
            ev.push((*t, 1, m.clone()));
        }
        for n in &tr.notes {
            ev.push((n.tick, 2, vec![0x90 | ch, n.key, n.vel]));
            ev.push((n.tick + n.len, 0, vec![0x80 | ch, n.key, 0]));
        }
        ev.sort_by_key(|e| (e.0, e.1));
        ev.into_iter().map(|(t, _, m)| (t, m)).collect()
    }
}

/// Maps pan -100..100 to CC10 0..127.
pub fn pan_value(p: f64) -> u8 {
    (64.0 + p.clamp(-100.0, 100.0) * 0.635).round().clamp(0.0, 127.0) as u8
}

#[derive(Clone, Debug)]
struct RawNote {
    tick: u64,
    len: u64,
    key: i32,
    vel: i32,
    staccato: bool,
    tie: bool,
}

#[derive(Clone, Debug)]
struct St {
    dur: u32,
    oct: i32,
    vel: u8,
    voicing: Vec<i32>,
}

#[derive(Clone, Copy)]
struct Cx<'a> {
    src: &'a SongSrc,
    td: &'a TrackDef,
    chord: Option<&'a Chord>,
    transpose: i32,
    depth: u32,
    bar: u32,
    /// Start of the enclosing stream (for bar checks).
    origin: u64,
    channel: u8,
    section: &'a str,
}

struct Compiler {
    notes: Vec<RawNote>,
    ctls: Vec<(u64, Vec<u8>)>,
    warnings: Vec<String>,
}

impl Compiler {
    fn items(&mut self, items: &[Item], t: &mut u64, st: &mut St, cx: Cx) -> Result<(), String> {
        for it in items {
            self.item(it, t, st, cx)?;
        }
        Ok(())
    }

    fn resolve(&mut self, p: &Pitch, st: &St, cx: Cx) -> Result<i32, String> {
        let chord_root = |c: &Chord| (st.oct + 1) * 12 + c.root;
        Ok(match p {
            Pitch::Abs(n) => *n + cx.transpose,
            Pitch::Rel(pc) => (st.oct + 1) * 12 + pc + cx.transpose,
            Pitch::Drum(k) => {
                if !cx.td.drums {
                    return Err(format!("track {}: drum sound used on a melodic track", cx.td.name));
                }
                *k as i32
            }
            Pitch::Degree(d, o) => {
                let c = cx.chord.ok_or_else(|| format!("track {}, section {}: chord degree {d} used outside `PATTERN over PROGRESSION`", cx.td.name, cx.section))?;
                chord_root(c) + c.degree(*d) + 12 * o + cx.transpose
            }
            Pitch::Bass(o) => {
                let c = cx.chord.ok_or_else(|| format!("track {}, section {}: bass degree 0 used outside `over`", cx.td.name, cx.section))?;
                let root = chord_root(c);
                let b = c.bass.unwrap_or(c.root);
                root - (root - b).rem_euclid(12) + 12 * o + cx.transpose
            }
        })
    }

    fn velocity(m: &Mods, st: &St) -> i32 {
        let v = m.vel.unwrap_or(st.vel) as i32;
        let v = if m.accent < 0 { v / 2 } else { v + 16 * m.accent };
        v.clamp(1, 127)
    }

    fn push_note(&mut self, tick: u64, len: u32, key: i32, vel: i32, m: &Mods, cx: Cx) {
        let key = if cx.td.drums { key } else { key + cx.td.transpose };
        if !(0..=127).contains(&key) {
            self.warnings.push(format!("track {}, section {}: note {key} outside MIDI range dropped", cx.td.name, cx.section));
            return;
        }
        self.notes.push(RawNote { tick, len: len as u64, key, vel, staccato: m.staccato, tie: m.tie });
    }

    fn item(&mut self, it: &Item, t: &mut u64, st: &mut St, cx: Cx) -> Result<(), String> {
        let ch = cx.channel;
        match it {
            Item::Note { pitch, dur, m } => {
                let d = dur.unwrap_or(st.dur);
                st.dur = d;
                let k = self.resolve(pitch, st, cx)?;
                self.push_note(*t, d, k, Self::velocity(m, st), m, cx);
                *t += d as u64;
            }
            Item::Chord { pitches, dur, m } => {
                let d = dur.unwrap_or(st.dur);
                st.dur = d;
                let mut keys = Vec::new();
                for p in pitches {
                    let k = self.resolve(p, st, cx)?;
                    self.push_note(*t, d, k, Self::velocity(m, st), m, cx);
                    keys.push(k - cx.transpose);
                }
                st.voicing = keys;
                *t += d as u64;
            }
            Item::Symbol { chord, dur, m } => {
                let d = dur.unwrap_or(st.dur);
                st.dur = d;
                let c = match chord.as_ref().or(cx.chord) {
                    Some(c) => c,
                    None => return Err(format!("track {}, section {}: `*` (whole chord) used outside `over`", cx.td.name, cx.section)),
                };
                let v = voice_chord(c, st.oct, &st.voicing);
                for k in &v {
                    self.push_note(*t, d, k + cx.transpose, Self::velocity(m, st), m, cx);
                }
                st.voicing = v;
                *t += d as u64;
            }
            Item::Rest(dur) => {
                let d = dur.unwrap_or(st.dur);
                st.dur = d;
                *t += d as u64;
            }
            Item::Group { items, repeat, transpose } => {
                let sub = Cx { transpose: cx.transpose + transpose, ..cx };
                for _ in 0..*repeat {
                    self.items(items, t, st, sub)?;
                }
            }
            Item::Ref { name, repeat, transpose } => {
                if cx.depth > 24 {
                    return Err(format!("pattern '{name}' nests too deeply (does it refer to itself?)"));
                }
                let body = cx.src.pattern(name).ok_or_else(|| format!("unknown pattern '{name}'"))?;
                let mut sst = St { dur: PPQ, oct: st.oct, vel: st.vel, voicing: st.voicing.clone() };
                for _ in 0..*repeat {
                    let sub = Cx { transpose: cx.transpose + transpose, depth: cx.depth + 1, origin: *t, ..cx };
                    sst.dur = PPQ;
                    self.items(body, t, &mut sst, sub)?;
                }
                st.voicing = sst.voicing;
            }
            Item::Par(lanes) => {
                let start = *t;
                let mut end = start;
                for lane in lanes {
                    let mut tt = start;
                    let mut s2 = st.clone();
                    self.items(lane, &mut tt, &mut s2, cx)?;
                    end = end.max(tt);
                }
                *t = end;
            }
            Item::Over { pattern, prog } => {
                for (chord, len) in prog {
                    let len = *len as u64;
                    let Some(chord) = chord else {
                        *t += len;
                        continue;
                    };
                    let n0 = self.notes.len();
                    let c0 = self.ctls.len();
                    let mut pt = 0u64;
                    let mut sst = St { dur: PPQ, oct: st.oct, vel: st.vel, voicing: st.voicing.clone() };
                    let sub = Cx { chord: Some(chord), depth: cx.depth + 1, origin: 0, ..cx };
                    self.item(pattern, &mut pt, &mut sst, sub)?;
                    st.voicing = sst.voicing;
                    let notes: Vec<RawNote> = self.notes.drain(n0..).collect();
                    let ctls: Vec<(u64, Vec<u8>)> = self.ctls.drain(c0..).collect();
                    if pt == 0 {
                        return Err(format!("track {}, section {}: pattern used with `over` has no length", cx.td.name, cx.section));
                    }
                    let mut k = 0;
                    while k * pt < len {
                        for n in &notes {
                            let nt = k * pt + n.tick;
                            if nt < len {
                                self.notes.push(RawNote { tick: *t + nt, len: n.len.min(len - nt), ..n.clone() });
                            }
                        }
                        for (ct, m) in &ctls {
                            let nt = k * pt + ct;
                            if nt < len {
                                self.ctls.push((*t + nt, m.clone()));
                            }
                        }
                        k += 1;
                    }
                    *t += len;
                }
            }
            Item::Octave { value, relative } => {
                st.oct = if *relative { st.oct + value } else { *value };
                st.oct = st.oct.clamp(-1, 9);
            }
            Item::Vel(v) => st.vel = *v,
            Item::Control { ctl, from, to } => {
                let msg = |v: f64| -> Vec<u8> {
                    match ctl {
                        Ctl::Cc(n) => vec![0xB0 | ch, *n, v.round().clamp(0.0, 127.0) as u8],
                        Ctl::Pan => vec![0xB0 | ch, 10, pan_value(v)],
                        Ctl::Bend => {
                            let r = cx.td.bend_range.max(1) as f64;
                            let b = (8192.0 + v / r * 8192.0).round().clamp(0.0, 16383.0) as u16;
                            vec![0xE0 | ch, (b & 0x7F) as u8, (b >> 7) as u8]
                        }
                    }
                };
                match to {
                    None => self.ctls.push((*t, msg(*from))),
                    Some((b, len)) => {
                        let step = (PPQ / 8) as u64;
                        let n = (*len as u64 / step).max(1);
                        for k in 0..=n {
                            let f = k as f64 / n as f64;
                            self.ctls.push((*t + k * step.min(*len as u64), msg(from + (b - from) * f)));
                        }
                    }
                }
            }
            Item::Program(p) => self.ctls.push((*t, vec![0xC0 | ch, *p])),
            Item::Pedal(on) => self.ctls.push((*t, vec![0xB0 | ch, 64, if *on { 127 } else { 0 }])),
            Item::BarCheck(line) => {
                let rel = *t - cx.origin;
                if !rel.is_multiple_of(cx.bar as u64) {
                    let beat = (rel % cx.bar as u64) as f64 / PPQ as f64;
                    self.warnings.push(format!(
                        "line {line}: bar check failed (track {}, section {}): `|` falls {beat:.3} beats into bar {}",
                        cx.td.name,
                        cx.section,
                        rel / cx.bar as u64 + 1
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Merges tied notes (`~`) with the following note of the same key.
fn merge_ties(notes: &mut Vec<RawNote>) {
    notes.sort_by_key(|n| (n.tick, n.key));
    let mut i = 0;
    while i < notes.len() {
        if notes[i].tie {
            let end = notes[i].tick + notes[i].len;
            if let Some(j) = (i + 1..notes.len()).find(|&j| notes[j].key == notes[i].key && notes[j].tick == end) {
                let nj = notes.remove(j);
                notes[i].len += nj.len;
                notes[i].tie = nj.tie;
                notes[i].staccato = nj.staccato;
                continue;
            }
        }
        i += 1;
    }
}

/// Swing: moves off-beat eighths later (ratio 0.5 = straight).
fn swing_tick(t: u64, r: f64) -> u64 {
    if (r - 0.5).abs() < 1e-6 {
        return t;
    }
    let q = PPQ as u64;
    let base = t / q * q;
    let p = (t - base) as f64;
    let half = q as f64 / 2.0;
    let np = if p < half { p * 2.0 * r } else { r * q as f64 + (p - half) * 2.0 * (1.0 - r) };
    base + np.round() as u64
}

/// Compiles a song; `variant` selects a `variant` block.
pub fn compile(src: &SongSrc, variant: Option<&str>) -> Result<Song, String> {
    let var = match variant {
        Some(v) => Some(src.variants.iter().find(|x| x.name == v).ok_or_else(|| {
            format!("unknown variant '{v}' (known: {})", src.variants.iter().map(|v| v.name.as_str()).collect::<Vec<_>>().join(", "))
        })?),
        None => None,
    };
    // tracks with variant changes applied
    let mut defs: Vec<TrackDef> = src.tracks.clone();
    let mut muted: Vec<String> = Vec::new();
    let mut tempo_scale = 1.0;
    if let Some(v) = var {
        for (t, p) in &v.inst {
            if let Some(d) = defs.iter_mut().find(|d| &d.name == t) {
                d.program = *p;
            }
        }
        for (t, s) in &v.synth {
            if let Some(d) = defs.iter_mut().find(|d| &d.name == t) {
                d.synth = Some(s.clone());
            }
        }
        for (t, vol) in &v.vol {
            if let Some(d) = defs.iter_mut().find(|d| &d.name == t) {
                d.vol = *vol;
            }
        }
        for (t, s) in &v.shift {
            if let Some(d) = defs.iter_mut().find(|d| &d.name == t) {
                d.transpose += s;
            }
        }
        for d in defs.iter_mut().filter(|d| !d.drums) {
            d.transpose += v.transpose;
        }
        muted = v.mute.clone();
        if let Some(bpm) = v.tempo {
            tempo_scale = bpm / src.tempo;
        }
    }
    // channels: drums on 10, others in order skipping 10
    let mut used = [false; 16];
    for d in &defs {
        if let Some(c) = d.channel {
            used[c as usize - 1] = true;
        }
    }
    let mut channels = Vec::new();
    for d in &defs {
        let c = match d.channel {
            Some(c) => c - 1,
            None => {
                let c = (0..16).find(|&c| c != 9 && !used[c]).ok_or("more than 15 melodic tracks; MIDI has 16 channels (10 is drums)")?;
                used[c] = true;
                c as u8
            }
        };
        channels.push(c);
    }
    let arrangement: Vec<ArrEntry> = match var.and_then(|v| v.arrange.clone()).or_else(|| src.arrange.clone()) {
        Some(a) => a,
        None => src.sections.iter().map(|s| ArrEntry::Play { section: s.name.clone(), repeat: 1, transpose: 0, mute: Vec::new() }).collect(),
    };

    let mut warnings = Vec::new();
    type Part = (Vec<RawNote>, Vec<(u64, Vec<u8>)>);
    let mut per_track: Vec<Part> = vec![(Vec::new(), Vec::new()); defs.len()];
    let base_bpm = src.tempo * tempo_scale;
    let mut tempos: Vec<(u64, f64)> = vec![(0, base_bpm)];
    let mut times: Vec<(u64, (u32, u32))> = vec![(0, src.time)];
    let mut markers = Vec::new();
    let mut t: u64 = 0;
    let (mut loop_start, mut loop_end) = (None, None);
    for e in &arrangement {
        match e {
            ArrEntry::LoopStart => loop_start = Some(t),
            ArrEntry::LoopEnd => loop_end = Some(t),
            ArrEntry::Play { section, repeat, transpose, mute } => {
                let sec = src.section(section).ok_or_else(|| format!("unknown section '{section}'"))?;
                let time = sec.time.unwrap_or(src.time);
                let bar = bar_ticks(time);
                for _ in 0..*repeat {
                    if times.last().unwrap().1 != time {
                        times.push((t, time));
                    }
                    markers.push((t, sec.name.clone()));
                    let mut max_end = t;
                    let mut lens: Vec<(String, u64)> = Vec::new();
                    let n_before: Vec<usize> = per_track.iter().map(|p| p.0.len()).collect();
                    for (ti, d) in defs.iter().enumerate() {
                        let parts: Vec<&(String, Vec<Item>, usize)> = sec.parts.iter().filter(|p| p.0 == d.name).collect();
                        if parts.is_empty() || muted.contains(&d.name) || mute.contains(&d.name) {
                            continue;
                        }
                        let mut comp = Compiler { notes: Vec::new(), ctls: Vec::new(), warnings: Vec::new() };
                        let mut st = St { dur: PPQ, oct: d.octave, vel: d.vel, voicing: Vec::new() };
                        let mut tt = t;
                        let cx = Cx { src, td: d, chord: None, transpose: *transpose, depth: 0, bar, origin: t, channel: channels[ti], section: &sec.name };
                        for (_, items, _) in &parts {
                            comp.items(items, &mut tt, &mut st, cx)?;
                        }
                        warnings.append(&mut comp.warnings);
                        merge_ties(&mut comp.notes);
                        per_track[ti].0.extend(comp.notes);
                        per_track[ti].1.extend(comp.ctls);
                        max_end = max_end.max(tt);
                        lens.push((d.name.clone(), tt - t));
                    }
                    let natural = (max_end - t).div_ceil(bar as u64).max(1) * bar as u64;
                    let sec_len = match sec.bars {
                        Some(b) => b as u64 * bar as u64,
                        None => natural,
                    };
                    for (name, l) in &lens {
                        if *l == 0 {
                            continue;
                        }
                        let bars = *l as f64 / bar as f64;
                        if *l % bar as u64 != 0 {
                            warnings.push(format!("section {}, track {name}: part is {bars:.2} bars long (not a whole number of bars)", sec.name));
                        } else if *l > sec_len {
                            warnings.push(format!("section {}, track {name}: part is {bars} bars but the section has {} — the rest is cut", sec.name, sec_len / bar as u64));
                        } else if *l < sec_len && sec.bars.is_none() {
                            warnings.push(format!(
                                "section {}, track {name}: part is {bars} bars, the section {} (rests fill the gap)",
                                sec.name,
                                sec_len / bar as u64
                            ));
                        }
                    }
                    // cut notes past a declared section length
                    if sec.bars.is_some() {
                        for (ti, p) in per_track.iter_mut().enumerate() {
                            let n0 = n_before[ti];
                            let mut k = n0;
                            while k < p.0.len() {
                                if p.0[k].tick >= t + sec_len {
                                    p.0.remove(k);
                                } else {
                                    k += 1;
                                }
                            }
                        }
                    }
                    // tempo
                    let cur = tempos.last().unwrap().1;
                    match sec.tempo {
                        Some((a, ramp)) => {
                            let a = a * tempo_scale;
                            if (a - cur).abs() > 1e-9 {
                                tempos.push((t, a));
                            }
                            if let Some(b) = ramp {
                                let b = b * tempo_scale;
                                let beats = sec_len / PPQ as u64;
                                for k in 1..beats {
                                    tempos.push((t + k * PPQ as u64, a + (b - a) * k as f64 / (beats - 1).max(1) as f64));
                                }
                            }
                        }
                        None => {
                            if (base_bpm - cur).abs() > 1e-9 {
                                tempos.push((t, base_bpm));
                            }
                        }
                    }
                    t += sec_len;
                }
            }
        }
    }
    let end = t;
    // finalize notes: swing, gate, humanize
    let mut tracks = Vec::new();
    for (ti, d) in defs.iter().enumerate() {
        let (raw, mut ctls) = std::mem::take(&mut per_track[ti]);
        let swing = d.swing.unwrap_or(src.swing);
        let mut seed: u32 = 0x9E37_79B9 ^ (ti as u32 + 1).wrapping_mul(0x85EB_CA6B);
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        let mut notes: Vec<Note> = raw
            .into_iter()
            .map(|n| {
                let start = swing_tick(n.tick, swing);
                let stop = swing_tick(n.tick + n.len, swing).max(start + 1);
                let gate = if n.staccato { d.gate * 0.5 } else { d.gate };
                let len = (((stop - start) as f64 * gate).round() as u64).max(10).min(stop - start);
                let mut vel = n.vel;
                if d.human > 0 {
                    let r = (rnd() % (2 * d.human as u32 + 1)) as i32 - d.human as i32;
                    vel += r;
                }
                Note { tick: start, len, key: n.key as u8, vel: vel.clamp(1, 127) as u8 }
            })
            .collect();
        notes.sort_by_key(|n| (n.tick, n.key));
        ctls.sort_by_key(|c| c.0);
        tracks.push(Track { def: d.clone(), channel: channels[ti], notes, ctls });
    }
    let loop_range = (loop_start.unwrap_or(0), loop_end.unwrap_or(end));
    if loop_range.0 >= loop_range.1 {
        return Err("empty loop region (|: must come before :|)".into());
    }
    if loop_range.0 > 0 {
        warnings.push(
            "the loop starts after an intro: GameMaker loops whole files, so MIDI output loops from the start; `render --loop` outputs only the loop region".into(),
        );
    }
    let looping = loop_start.is_some() || loop_end.is_some();
    Ok(Song { title: src.title.clone(), variant: variant.map(String::from), tempos, times, key: src.key.clone(), tracks, markers, end, loop_range, looping, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::parse;

    fn keys(s: &Song, track: usize) -> Vec<(u64, u8)> {
        s.tracks[track].notes.iter().map(|n| (n.tick, n.key)).collect()
    }

    #[test]
    fn basic_timing_and_ties() {
        let src = parse("tempo 60\ntrack a gate=100\na: C4q D4e E4 | F4h~ F4h G4w").unwrap();
        let s = compile(&src, None).unwrap();
        assert_eq!(keys(&s, 0), vec![(0, 60), (480, 62), (720, 64), (960, 65), (1920 + 960, 67)]);
        assert_eq!(s.tracks[0].notes[3].len, 1920);
        assert!(s.warnings.iter().any(|w| w.contains("bar check failed")), "{:?}", s.warnings);
        assert_eq!(s.end, 1920 * 3);
        assert!((s.duration() - 12.0).abs() < 1e-9);
    }

    #[test]
    fn patterns_over_progressions() {
        let src = parse("track b oct=2\npattern walk = 1e 5 8 5\nb: walk over | Am | F G |").unwrap();
        let s = compile(&src, None).unwrap();
        let k: Vec<u8> = s.tracks[0].notes.iter().map(|n| n.key).collect();
        // Am: A2 E3 A3 E3 (twice), F: F2 C3 F3 C3, G: G2 D3 G3 D3
        assert_eq!(k, vec![45, 52, 57, 52, 45, 52, 57, 52, 41, 48, 53, 48, 43, 50, 55, 50]);
    }

    #[test]
    fn arrangement_variants_and_loops() {
        let src = parse(
            "tempo 100\ntrack a\ntrack d drums\nsection A\na: C4w\nd: kick:q snare kick snare\nsection B tempo=50\na: D4w\nvariant v\n tempo 200\n transpose 2\n mute d\narrange A |: B A(+12) :|",
        )
        .unwrap();
        let s = compile(&src, None).unwrap();
        assert_eq!(keys(&s, 0), vec![(0, 60), (1920, 62), (3840, 72)]);
        assert_eq!(s.tracks[1].channel, 9);
        assert_eq!(s.tempos, vec![(0, 100.0), (1920, 50.0), (3840, 100.0)]);
        let s = compile(&src, Some("v")).unwrap();
        assert_eq!(keys(&s, 0), vec![(0, 62), (1920, 64), (3840, 74)]);
        assert!(s.tracks[1].notes.is_empty());
        assert_eq!(s.loop_range, (1920, 5760));
        assert_eq!(s.tempos[0], (0, 200.0));
    }

    #[test]
    fn grids_and_chords() {
        let src = parse("track d drums\ntrack p oct=4 gate=100\ngrid g\n kick x...x...\n hat  x.x.X.xo\nd: g\np: {C}h {F}h").unwrap();
        let s = compile(&src, None).unwrap();
        let d = &s.tracks[0].notes;
        assert_eq!(d.iter().filter(|n| n.key == 36).count(), 2);
        assert_eq!(d.iter().filter(|n| n.key == 42).count(), 5);
        assert!(d.iter().any(|n| n.key == 42 && n.vel == 96));
        assert!(d.iter().any(|n| n.key == 42 && n.vel == 40));
        let p: Vec<u8> = s.tracks[1].notes.iter().map(|n| n.key).collect();
        assert_eq!(p, vec![60, 64, 67, 60, 65, 69]);
    }

    #[test]
    fn swing_moves_offbeats() {
        assert_eq!(swing_tick(240, 2.0 / 3.0), 320);
        assert_eq!(swing_tick(480, 2.0 / 3.0), 480);
        assert_eq!(swing_tick(240, 0.5), 240);
    }
}
