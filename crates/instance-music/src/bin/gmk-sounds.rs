//! `gmk-sounds` — objective analysis of game sounds for authors who cannot
//! listen: every sound of a .gm81/.gmk project (or plain audio/MIDI files)
//! with levels, spectrum, envelope, a one-line description, loop-seam
//! checks, loudness over time, and a similarity matrix that flags sound
//! effects that sound alike.
//!
//! ```text
//! gmk-sounds GAME.gm81 [FILE.wav ...] [--per-second] [--window SECS] [--json]
//!            [--soundfont PATH] [--threshold 0.85] [--loop] [--no-similarity]
//! ```

use instance_music::analysis::{analyse, analyse_music, similarity, windows, Report, Window, BANDS};
use instance_music::render::{find_soundfont, load_soundfont, render_midi_file, SoundFont};
use instance_music::smf::{midi_stats, MidiStats};
use instance_music::wav;
use serde::Serialize;
use std::sync::Arc;

const USAGE: &str = "usage: gmk-sounds GAME.gm81|FILE.wav|FILE.mid|FILE.mp3 ... [--per-second] [--window SECS]
                  [--json] [--soundfont PATH] [--threshold 0.85] [--loop] [--no-similarity]
  --per-second   loudness (RMS/peak dBFS, centroid) per window, for music
  --loop         treat plain files as loops (check the seam)
  --threshold    similarity above which two sound effects are reported as alike";

#[derive(Serialize)]
struct Entry {
    /// Index within its project, if any.
    index: Option<usize>,
    name: String,
    source: String,
    kind: Option<i32>,
    kind_name: Option<String>,
    file_type: String,
    /// Why the sound is treated as a loop (kind 1, name, sound_loop in code, --loop).
    looped: Option<String>,
    report: Option<Report>,
    midi: Option<MidiStats>,
    note: Option<String>,
    per_second: Option<Vec<Window>>,
}

#[derive(Serialize)]
struct Output {
    sounds: Vec<Entry>,
    /// One similarity table per project (plain files form one group).
    similarity: Vec<SimGroup>,
    threshold: f64,
}

#[derive(Serialize)]
struct SimGroup {
    source: String,
    names: Vec<String>,
    matrix: Vec<Vec<f64>>,
    /// Pairs at or above the threshold, most similar first.
    similar_pairs: Vec<(String, String, f64)>,
    /// The five most similar pairs regardless of threshold.
    top_pairs: Vec<(String, String, f64)>,
}

/// Similarity among the sound effects (not music) of `idx`.
fn sim_group(source: &str, entries: &[Entry], idx: &[usize], threshold: f64) -> SimGroup {
    let fx: Vec<usize> = idx
        .iter()
        .copied()
        .filter(|&i| {
            let e = &entries[i];
            e.report.as_ref().map(|r| r.duration < 30.0 && r.description != "silent").unwrap_or(false) && e.midi.is_none() && e.kind.map(|k| k != 1 && k != 3).unwrap_or(true)
        })
        .collect();
    let mut matrix = vec![vec![0.0; fx.len()]; fx.len()];
    let mut pairs = Vec::new();
    for a in 0..fx.len() {
        for b in 0..fx.len() {
            let fa = &entries[fx[a]].report.as_ref().unwrap().fingerprint;
            let fb = &entries[fx[b]].report.as_ref().unwrap().fingerprint;
            matrix[a][b] = if a == b { 1.0 } else { similarity(fa, fb) };
            if b > a {
                pairs.push((a, b, matrix[a][b]));
            }
        }
    }
    pairs.sort_by(|x, y| y.2.partial_cmp(&x.2).unwrap());
    let names: Vec<String> = fx.iter().map(|i| entries[*i].name.clone()).collect();
    let named = |p: &(usize, usize, f64)| (names[p.0].clone(), names[p.1].clone(), p.2);
    SimGroup {
        source: source.to_string(),
        similar_pairs: pairs.iter().filter(|p| p.2 >= threshold).map(named).collect(),
        top_pairs: pairs.iter().take(5).map(named).collect(),
        names,
        matrix,
    }
}

fn print_group(g: &SimGroup, entries: &[Entry], threshold: f64) {
    if g.names.len() < 2 {
        return;
    }
    println!("similarity of sound effects in {} (0-100; spectral shape 70% + envelope 30%; two loops: spectrum only)", g.source);
    let n = g.names.len();
    if n <= 40 {
        print!("{:>24} ", "");
        for k in 0..n {
            print!("{:>4}", k);
        }
        println!();
        for a in 0..n {
            let name: String = g.names[a].chars().take(19).collect();
            print!("{:>19} {:>3}: ", name, a);
            for b in 0..n {
                if a == b {
                    print!("   -");
                } else {
                    let v = (g.matrix[a][b] * 100.0).round() as i64;
                    print!("{:>3}{}", v, if g.matrix[a][b] >= threshold { "*" } else { " " });
                }
            }
            println!();
        }
    }
    println!();
    if g.similar_pairs.is_empty() {
        println!("no pair of sound effects at or above {:.0}; most similar:", threshold * 100.0);
        for (a, b, v) in &g.top_pairs {
            println!("  {:.0}  {a} ~ {b}", v * 100.0);
        }
    } else {
        println!("THESE SOUND ALIKE (similarity >= {:.0}) — consider making them more distinct:", threshold * 100.0);
        for (a, b, v) in &g.similar_pairs {
            let find = |n: &str| entries.iter().find(|e| e.name == n && (e.source == g.source || g.source == "files")).and_then(|e| e.report.as_ref());
            let (ra, rb) = (find(a).unwrap(), find(b).unwrap());
            println!(
                "  {:.0}  {a} ~ {b}
       {a}: \"{}\", centroid {:.0} Hz
       {b}: \"{}\", centroid {:.0} Hz",
                v * 100.0,
                ra.description,
                ra.centroid_hz,
                rb.description,
                rb.centroid_hz
            );
        }
    }
    println!();
}

fn kind_name(k: i32) -> &'static str {
    match k {
        0 => "normal",
        1 => "background music",
        2 => "3D",
        3 => "multimedia",
        _ => "?",
    }
}

/// Names of sounds passed to `sound_loop(...)` anywhere in the project's code.
fn looped_in_code(p: &instance_format::Project) -> Vec<String> {
    let mut q = p.clone();
    for s in q.sounds.iter_mut().flatten() {
        s.data = None;
    }
    for s in q.sprites.iter_mut().flatten() {
        s.frames.clear();
    }
    for b in q.backgrounds.iter_mut().flatten() {
        b.image = Default::default();
    }
    let text = serde_json::to_string(&q).unwrap_or_default();
    let mut out = Vec::new();
    let mut rest = text.as_str();
    while let Some(i) = rest.find("sound_loop") {
        rest = &rest[i + "sound_loop".len()..];
        let r = rest.trim_start_matches([' ', '\t']);
        if let Some(r) = r.strip_prefix('(') {
            let name: String = r.trim_start().chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

struct Ctx {
    sf: Option<Arc<SoundFont>>,
    sf_tried: bool,
    sf_path: Option<String>,
    per_second: Option<f64>,
}

impl Ctx {
    fn soundfont(&mut self) -> Option<Arc<SoundFont>> {
        if !self.sf_tried {
            self.sf_tried = true;
            if let Some(p) = find_soundfont(self.sf_path.as_deref()) {
                match load_soundfont(&p) {
                    Ok(sf) => self.sf = Some(sf),
                    Err(e) => eprintln!("gmk-sounds: {e}"),
                }
            }
        }
        self.sf.clone()
    }
}

fn is_midi(d: &[u8]) -> bool {
    (d.len() >= 4 && &d[0..4] == b"MThd") || (d.len() >= 12 && &d[0..4] == b"RIFF" && &d[8..12] == b"RMID")
}

fn unwrap_rmid(d: &[u8]) -> &[u8] {
    if d.len() >= 12 && &d[0..4] == b"RIFF" && &d[8..12] == b"RMID" {
        if let Some(p) = d.windows(4).position(|w| w == b"MThd") {
            return &d[p..];
        }
    }
    d
}

fn analyse_data(e: &mut Entry, data: &[u8], ctx: &mut Ctx) {
    let looped = e.looped.is_some();
    let audio = if is_midi(data) {
        let d = unwrap_rmid(data);
        match midi_stats(d) {
            Ok(st) => e.midi = Some(st),
            Err(err) => {
                e.note = Some(format!("bad MIDI file: {err}"));
                return;
            }
        }
        match ctx.soundfont() {
            Some(sf) => match render_midi_file(d, &sf, 44100) {
                Ok(a) => a,
                Err(err) => {
                    e.note = Some(format!("MIDI not rendered: {err}"));
                    return;
                }
            },
            None => {
                e.note = Some("MIDI audio not analysed (no SoundFont: pass --soundfont or set INSTANCE_SOUNDFONT); note stats only".into());
                return;
            }
        }
    } else {
        match wav::decode(data, &e.file_type) {
            Ok(a) => a,
            Err(err) => {
                e.note = Some(format!("not analysed: {err}"));
                return;
            }
        }
    };
    if audio.frames() == 0 {
        e.note = Some("empty sound".into());
        return;
    }
    if let Some(w) = ctx.per_second {
        e.per_second = Some(windows(&audio, w));
    }
    let music = is_midi(data) || e.kind == Some(1);
    e.report = Some(if music { analyse_music(&audio, looped && !is_midi(data)) } else { analyse(&audio, looped) });
}

fn print_entry(i: usize, e: &Entry) {
    let idx = e.index.map(|x| format!("#{x} ")).unwrap_or_default();
    let kind = e.kind.map(|k| format!("  kind {k} {}", e.kind_name.clone().unwrap_or_default())).unwrap_or_default();
    println!("[{i}] {idx}{}{kind}{}", e.name, e.looped.as_ref().map(|l| format!("  (loops: {l})")).unwrap_or_default());
    if let Some(m) = &e.midi {
        println!(
            "    midi     {} notes, {} tracks, channels {:?}, programs {}, pitched range {}-{}, max polyphony {}, {:.1} s",
            m.notes,
            m.tracks,
            m.channels,
            m.programs
                .iter()
                .map(|(c, p)| if *c == 10 {
                    format!("ch10:drums({})", instance_music::gm::KITS.iter().find(|k| k.1 + 1 == *p).map(|k| k.0).unwrap_or("kit"))
                } else {
                    format!("ch{c}:{}", instance_music::gm::PROGRAMS.get(*p as usize - 1).copied().unwrap_or("?"))
                })
                .collect::<Vec<_>>()
                .join(" "),
            m.lowest.map(|k| instance_music::theory::note_name(k as i32)).unwrap_or("-".into()),
            m.highest.map(|k| instance_music::theory::note_name(k as i32)).unwrap_or("-".into()),
            m.max_polyphony,
            m.duration
        );
    }
    if let Some(r) = &e.report {
        println!(
            "    format   {}, {} Hz, {}, {:.3} s",
            r.format,
            r.rate,
            if r.channels == 1 { "mono".to_string() } else { format!("{} ch", r.channels) },
            r.duration
        );
        println!(
            "    level    peak {:.1} dBFS, RMS {:.1} dBFS, clipping {:.2}%, DC {:+.4}, silence start {:.0} ms / end {:.0} ms, attack {:.0} ms",
            r.peak_dbfs, r.rms_dbfs, r.clip_percent, r.dc_offset, r.silence_start_ms, r.silence_end_ms, r.attack_ms
        );
        println!(
            "    spectrum centroid {:.0} Hz, flatness {:.2}, bands {}",
            r.centroid_hz,
            r.flatness,
            BANDS.iter().zip(r.bands.iter()).map(|((n, _, _), v)| format!("{n} {v:.0}%")).collect::<Vec<_>>().join(" ")
        );
        println!("    sounds   \"{}\"", r.description);
        if let Some(l) = &r.loop_check {
            println!(
                "    loop     {} (wrap jump {:.4}, steps near the wrap {:.4}, end-vs-start level {:+.1} dB)",
                l.verdict, l.wrap_jump, l.typical_step, l.level_diff_db
            );
        }
        let mut notes = Vec::new();
        if r.peak_dbfs > -0.1 || r.clip_percent > 0.01 {
            notes.push("at/over full scale: may clip when mixed".to_string());
        }
        if r.peak_dbfs < -20.0 && r.peak_dbfs > -150.0 {
            notes.push("very quiet".to_string());
        }
        if r.dc_offset.abs() > 0.02 {
            notes.push("DC offset (clicks at start/stop)".to_string());
        }
        if r.silence_start_ms > 50.0 && e.looped.is_none() {
            notes.push(format!("starts {:.0} ms late (trim leading silence)", r.silence_start_ms));
        }
        if !notes.is_empty() {
            println!("    check    {}", notes.join("; "));
        }
    }
    if let Some(w) = e.per_second.as_ref().filter(|w| w.len() >= 3) {
        println!("    per window: time   RMS dBFS  peak dBFS  centroid");
        for x in w {
            let bar = ((x.rms_db + 60.0).max(0.0) / 60.0 * 40.0) as usize;
            println!("      {:>6.1}s {:>8.1} {:>9.1} {:>8.0} Hz  {}", x.start, x.rms_db.max(-99.0), x.peak_db.max(-99.0), x.centroid_hz, "#".repeat(bar));
        }
    }
    if let Some(n) = &e.note {
        println!("    note     {n}");
    }
    println!();
}

fn main() {
    let mut inputs = Vec::new();
    let mut json = false;
    let mut force_loop = false;
    let mut no_sim = false;
    let mut threshold = 0.85;
    let mut ctx = Ctx { sf: None, sf_tried: false, sf_path: None, per_second: None };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--json" => json = true,
            "--per-second" => ctx.per_second = Some(ctx.per_second.unwrap_or(1.0)),
            "--window" => ctx.per_second = Some(it.next().and_then(|v| v.parse().ok()).filter(|v: &f64| *v > 0.0).expect("--window SECS")),
            "--soundfont" | "--sf" => ctx.sf_path = Some(it.next().expect("--soundfont PATH")),
            "--threshold" => threshold = it.next().and_then(|v| v.parse().ok()).expect("--threshold 0..1"),
            "--loop" => force_loop = true,
            "--no-similarity" => no_sim = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return;
            }
            _ => inputs.push(a),
        }
    }
    if inputs.is_empty() {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
    let mut entries: Vec<Entry> = Vec::new();
    for input in &inputs {
        let lower = input.to_ascii_lowercase();
        if lower.ends_with(".gm81") || lower.ends_with(".gmk") || lower.ends_with(".gm6") {
            let (p, _) = match instance_format::read_project_file(input) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("gmk-sounds: {input}: {e}");
                    std::process::exit(1);
                }
            };
            let in_code = looped_in_code(&p);
            for (i, s) in p.sounds.iter().enumerate() {
                let Some(s) = s else { continue };
                let looped = if s.kind == 1 {
                    Some("kind 1 background music".to_string())
                } else if in_code.contains(&s.name) {
                    Some("sound_loop() in code".to_string())
                } else if s.name.to_ascii_lowercase().contains("loop") {
                    Some("name contains 'loop'".to_string())
                } else {
                    None
                };
                let mut e = Entry {
                    index: Some(i),
                    name: s.name.clone(),
                    source: input.clone(),
                    kind: Some(s.kind),
                    kind_name: Some(kind_name(s.kind).into()),
                    file_type: s.file_type.clone(),
                    looped,
                    report: None,
                    midi: None,
                    note: None,
                    per_second: None,
                };
                match &s.data {
                    Some(d) if !d.is_empty() => analyse_data(&mut e, d, &mut ctx),
                    _ => e.note = Some("no data".into()),
                }
                entries.push(e);
            }
        } else {
            let data = match std::fs::read(input) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("gmk-sounds: {input}: {e}");
                    std::process::exit(1);
                }
            };
            let name = std::path::Path::new(input).file_name().and_then(|n| n.to_str()).unwrap_or(input).to_string();
            let ext = std::path::Path::new(input).extension().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let looped = if force_loop {
                Some("--loop".to_string())
            } else if name.to_ascii_lowercase().contains("loop") {
                Some("name contains 'loop'".to_string())
            } else {
                None
            };
            let mut e = Entry { index: None, name, source: input.clone(), kind: None, kind_name: None, file_type: ext, looped, report: None, midi: None, note: None, per_second: None };
            analyse_data(&mut e, &data, &mut ctx);
            entries.push(e);
        }
    }

    // similarity among sound effects (not music), per project; plain files together
    let mut groups: Vec<SimGroup> = Vec::new();
    if !no_sim {
        let mut sources: Vec<String> = Vec::new();
        for e in &entries {
            let src = if e.index.is_some() { e.source.clone() } else { "files".to_string() };
            if !sources.contains(&src) {
                sources.push(src);
            }
        }
        for src in sources {
            let idx: Vec<usize> = (0..entries.len()).filter(|&i| if entries[i].index.is_some() { entries[i].source == src } else { src == "files" }).collect();
            groups.push(sim_group(&src, &entries, &idx, threshold));
        }
    }
    if json {
        let out = Output { sounds: entries, similarity: groups, threshold };
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
        return;
    }
    for (i, e) in entries.iter().enumerate() {
        print_entry(i, e);
    }
    for g in &groups {
        print_group(g, &entries, threshold);
    }
}
