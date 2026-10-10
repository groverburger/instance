//! A musical report of a compiled song: duration, sections, per-track note
//! statistics and warnings an author who cannot listen should look at.

use crate::compile::Song;
use crate::gm;
use crate::song::PPQ;
use crate::theory::note_name;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct TrackInfo {
    pub name: String,
    pub channel: u8,
    pub instrument: String,
    pub notes: usize,
    pub lowest: Option<String>,
    pub highest: Option<String>,
    pub avg_velocity: f64,
    pub min_velocity: u8,
    pub max_velocity: u8,
    pub max_polyphony: usize,
    pub notes_per_second: f64,
    pub out_of_range: usize,
    pub out_of_key: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct SongInfo {
    pub title: String,
    pub variant: Option<String>,
    pub duration: f64,
    pub bars: u64,
    pub tempo: String,
    pub key: Option<String>,
    pub loop_start: f64,
    pub loop_end: f64,
    /// (start seconds, bar number, name)
    pub sections: Vec<(f64, u64, String)>,
    pub tracks: Vec<TrackInfo>,
    pub max_total_polyphony: usize,
    pub warnings: Vec<String>,
}

fn bar_of(song: &Song, tick: u64) -> u64 {
    // bar number (1-based), honouring time signature changes
    let mut bar = 1u64;
    let mut t = 0u64;
    let mut times = song.times.clone();
    times.push((u64::MAX, (4, 4)));
    for w in times.windows(2) {
        let (start, ts) = w[0];
        let next = w[1].0;
        let bl = crate::song::bar_ticks(ts) as u64;
        if tick < next {
            return bar + (tick - start.max(t)) / bl;
        }
        bar += (next - start) / bl;
        t = next;
    }
    bar
}

fn polyphony(mut evs: Vec<(u64, i32)>) -> usize {
    evs.sort();
    let mut cur = 0i32;
    let mut mx = 0;
    for (_, d) in evs {
        cur += d;
        mx = mx.max(cur);
    }
    mx.max(0) as usize
}

/// Collects statistics and warnings.
pub fn song_info(song: &Song) -> SongInfo {
    let mut warnings = song.warnings.clone();
    let secs = |t: u64| song.secs_at(t);
    let mut tracks = Vec::new();
    let mut all_evs = Vec::new();
    for tr in &song.tracks {
        let d = &tr.def;
        let instrument = if d.drums {
            format!("drums ({} kit)", gm::KITS.iter().find(|k| k.1 == d.program).map(|k| k.0).unwrap_or("?"))
        } else {
            format!("{} ({})", gm::PROGRAMS[d.program as usize], d.program as u32 + 1)
        };
        let n = &tr.notes;
        let (lo, hi) = if d.drums { (0, 127) } else { gm::range(d.program) };
        let out_of_range = n.iter().filter(|x| (x.key as i32) < lo || (x.key as i32) > hi).count();
        let out_of_key = match (&song.key, d.drums) {
            (Some(k), false) => n.iter().filter(|x| !k.contains(x.key as i32)).count(),
            _ => 0,
        };
        let evs: Vec<(u64, i32)> = n.iter().flat_map(|x| [(x.tick, 1), (x.tick + x.len, -1)]).collect();
        all_evs.extend(evs.iter().cloned());
        let max_polyphony = polyphony(evs);
        let avg = if n.is_empty() { 0.0 } else { n.iter().map(|x| x.vel as f64).sum::<f64>() / n.len() as f64 };
        let (vmin, vmax) = n.iter().fold((127u8, 0u8), |(a, b), x| (a.min(x.vel), b.max(x.vel)));
        if out_of_range > 0 {
            warnings.push(format!(
                "track {}: {out_of_range} notes outside the usual range of {} ({}..{})",
                d.name,
                gm::PROGRAMS[d.program as usize],
                note_name(lo),
                note_name(hi)
            ));
        }
        if song.key.is_some() && !d.drums && !n.is_empty() && out_of_key * 5 > n.len() {
            warnings.push(format!("track {}: {out_of_key} of {} notes are outside the key (intended?)", d.name, n.len()));
        }
        if n.len() >= 16 && vmin == vmax {
            warnings.push(format!("track {}: every note has velocity {vmin} — add dynamics (pp..ff, > accents, human=)", d.name));
        }
        // overlapping notes of the same key on the same channel
        let mut sorted: Vec<_> = n.iter().collect();
        sorted.sort_by_key(|x| (x.key, x.tick));
        let overlaps = sorted.windows(2).filter(|w| w[0].key == w[1].key && w[1].tick < w[0].tick + w[0].len).count();
        if overlaps > 0 && !d.drums {
            warnings.push(format!("track {}: {overlaps} overlapping notes of the same pitch (the second cuts the first)", d.name));
        }
        // density per second
        let mut per_sec: std::collections::BTreeMap<u64, usize> = Default::default();
        for x in n {
            *per_sec.entry(secs(x.tick) as u64).or_default() += 1;
        }
        let limit = if d.drums { 24 } else { 16 };
        let dense: Vec<u64> = per_sec.iter().filter(|(_, c)| **c > limit).map(|(s, _)| *s).collect();
        if !dense.is_empty() {
            warnings.push(format!("track {}: very dense (>{limit} notes/s) at seconds {}", d.name, ranges(&dense)));
        }
        {
            let lasting = n.iter().filter(|x| x.tick < song.loop_range.1 && x.tick + x.len > song.loop_range.1 + PPQ as u64 / 4).count();
            if lasting > 0 && song.looping && song.loop_range.1 == song.end {
                warnings.push(format!(
                    "track {}: {lasting} notes ring past the loop end; a looping MIDI player cuts them (WAV --loop keeps them)",
                    d.name
                ));
            }
        }
        tracks.push(TrackInfo {
            name: d.name.clone(),
            channel: tr.channel + 1,
            instrument,
            notes: n.len(),
            lowest: n.iter().map(|x| x.key).min().map(|k| if d.drums { gm::drum_name(k).map(String::from).unwrap_or(k.to_string()) } else { note_name(k as i32) }),
            highest: n.iter().map(|x| x.key).max().map(|k| if d.drums { gm::drum_name(k).map(String::from).unwrap_or(k.to_string()) } else { note_name(k as i32) }),
            avg_velocity: avg,
            min_velocity: if n.is_empty() { 0 } else { vmin },
            max_velocity: vmax,
            max_polyphony,
            notes_per_second: n.len() as f64 / song.duration().max(0.001),
            out_of_range,
            out_of_key,
        });
    }
    let max_total = polyphony(all_evs);
    if max_total > 32 {
        warnings.push(format!("up to {max_total} notes sound at once; the Windows GS synth (GameMaker's MIDI player) may drop notes above ~32"));
    }
    // silent bars (no note sounding anywhere)
    let total_bars = bar_of(song, song.end.saturating_sub(1));
    let mut sounding = vec![false; total_bars as usize + 2];
    for tr in &song.tracks {
        for x in &tr.notes {
            let (a, b) = (bar_of(song, x.tick), bar_of(song, (x.tick + x.len).saturating_sub(1)));
            for k in a..=b.min(total_bars) {
                sounding[k as usize] = true;
            }
        }
    }
    let silent: Vec<u64> = (1..=total_bars).filter(|b| !sounding[*b as usize]).collect();
    if !silent.is_empty() {
        warnings.push(format!("silent bars: {} (nothing sounds)", ranges(&silent)));
    }
    let dur = song.duration();
    if song.looping && dur < 20.0 && song.end > 0 {
        warnings.push(format!("only {dur:.1} s long: a background loop this short gets repetitive (aim for 60-120 s)"));
    }
    let tmin = song.tempos.iter().map(|t| t.1).fold(f64::MAX, f64::min);
    let tmax = song.tempos.iter().map(|t| t.1).fold(0.0, f64::max);
    SongInfo {
        title: song.title.clone(),
        variant: song.variant.clone(),
        duration: dur,
        bars: total_bars,
        tempo: if (tmax - tmin).abs() < 1e-6 { format!("{tmin:.0} BPM") } else { format!("{tmin:.0}..{tmax:.0} BPM") },
        key: song.key.as_ref().map(|k| format!("{} {}", crate::theory::pc_name(k.tonic), k.mode)),
        loop_start: secs(song.loop_range.0),
        loop_end: secs(song.loop_range.1),
        sections: song.markers.iter().map(|(t, n)| (secs(*t), bar_of(song, *t), n.clone())).collect(),
        tracks,
        max_total_polyphony: max_total,
        warnings,
    }
}

fn ranges(v: &[u64]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < v.len() {
        let mut j = i;
        while j + 1 < v.len() && v[j + 1] == v[j] + 1 {
            j += 1;
        }
        out.push(if i == j { v[i].to_string() } else { format!("{}-{}", v[i], v[j]) });
        i = j + 1;
    }
    if out.len() > 12 {
        let n = out.len();
        out.truncate(12);
        out.push(format!("… ({n} runs)"));
    }
    out.join(", ")
}

fn mmss(s: f64) -> String {
    format!("{}:{:04.1}", (s / 60.0) as u64, s % 60.0)
}

/// Human-readable report.
pub fn format_info(i: &SongInfo) -> String {
    let mut s = String::new();
    s += &format!(
        "{}{}: {} ({:.1} s), {} bars, {}{}\n",
        if i.title.is_empty() { "(untitled)" } else { &i.title },
        i.variant.as_ref().map(|v| format!(" [variant {v}]")).unwrap_or_default(),
        mmss(i.duration),
        i.duration,
        i.bars,
        i.tempo,
        i.key.as_ref().map(|k| format!(", key {k}")).unwrap_or_default()
    );
    if i.loop_start > 0.0 || (i.loop_end - i.duration).abs() > 1e-6 {
        s += &format!("loop: {} .. {}\n", mmss(i.loop_start), mmss(i.loop_end));
    }
    s += "sections: ";
    s += &i.sections.iter().map(|(t, b, n)| format!("{n}@{}(bar {b})", mmss(*t))).collect::<Vec<_>>().join(" ");
    s += "\n\n";
    s += &format!("{:<10} {:>2}  {:<30} {:>5} {:>11} {:>9} {:>4} {:>6}\n", "track", "ch", "instrument", "notes", "range", "vel", "poly", "n/s");
    for t in &i.tracks {
        s += &format!(
            "{:<10} {:>2}  {:<30} {:>5} {:>11} {:>9} {:>4} {:>6.2}\n",
            t.name,
            t.channel,
            t.instrument,
            t.notes,
            match (&t.lowest, &t.highest) {
                (Some(a), Some(b)) => format!("{a}-{b}"),
                _ => "-".into(),
            },
            format!("{}-{} ~{:.0}", t.min_velocity, t.max_velocity, t.avg_velocity),
            t.max_polyphony,
            t.notes_per_second
        );
    }
    s += &format!("max notes sounding at once: {}\n", i.max_total_polyphony);
    if i.warnings.is_empty() {
        s += "\nno warnings\n";
    } else {
        s += &format!("\n{} warning(s):\n", i.warnings.len());
        for w in &i.warnings {
            s += &format!("  - {w}\n");
        }
    }
    s
}
