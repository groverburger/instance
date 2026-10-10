//! `gmk-music` — compile songs written in the instance-music notation.
//!
//! ```text
//! gmk-music render SONG.txt -o OUT.mid|OUT.wav [--variant NAME|all] [--soundfont PATH]
//!                  [--rate 44100|22050] [--mono] [--loop] [--normalize DBFS|off] [--tail SECS]
//! gmk-music info SONG.txt [--variant NAME|all] [--json]
//! gmk-music instruments        list General MIDI instrument names/numbers
//! gmk-music drums              list drum sound names
//! gmk-music synths             list built-in synth presets and options
//! ```

use instance_music::analysis;
use instance_music::render::{find_soundfont, load_soundfont, needs_soundfont, render_song_with, RenderOpts};
use instance_music::{compile, gm, info, smf, song, wav};
use std::path::{Path, PathBuf};

const USAGE: &str = "usage:
  gmk-music render SONG.txt -o OUT.mid|OUT.wav [--variant NAME|all] [--soundfont PATH]
                   [--rate 44100|22050] [--mono] [--loop] [--normalize DBFS|off] [--tail SECS]
  gmk-music info SONG.txt [--variant NAME|all] [--json]
  gmk-music instruments | drums | synths
WAV output is peak-normalised to -1 dBFS unless --normalize says otherwise; the
SoundFont comes from --soundfont or INSTANCE_SOUNDFONT (not needed when every
track uses the built-in synth, synth=PRESET).";

fn die(msg: &str) -> ! {
    eprintln!("gmk-music: {msg}");
    std::process::exit(1);
}

struct Args {
    cmd: String,
    input: Option<String>,
    out: Option<String>,
    variant: Option<String>,
    soundfont: Option<String>,
    rate: u32,
    mono: bool,
    looped: bool,
    json: bool,
    normalize: Option<f64>,
    tail: f64,
}

fn parse_args() -> Args {
    let mut a = Args { cmd: String::new(), input: None, out: None, variant: None, soundfont: None, rate: 44100, mono: false, looped: false, json: false, normalize: Some(-1.0), tail: 3.0 };
    let mut it = std::env::args().skip(1);
    let need = |it: &mut dyn Iterator<Item = String>, f: &str| it.next().unwrap_or_else(|| die(&format!("{f} needs a value\n{USAGE}")));
    while let Some(x) = it.next() {
        match x.as_str() {
            "-o" | "--out" => a.out = Some(need(&mut it, "-o")),
            "--variant" => a.variant = Some(need(&mut it, "--variant")),
            "--soundfont" | "--sf" => a.soundfont = Some(need(&mut it, "--soundfont")),
            "--rate" => a.rate = need(&mut it, "--rate").parse().ok().filter(|r| (16000..=96000).contains(r)).unwrap_or_else(|| die("--rate must be 16000..96000")),
            "--mono" => a.mono = true,
            "--loop" => a.looped = true,
            "--json" => a.json = true,
            "--normalize" => {
                let v = need(&mut it, "--normalize");
                a.normalize = if v == "off" { None } else { Some(v.parse().unwrap_or_else(|_| die("--normalize DBFS (e.g. -1) or off"))) };
            }
            "--tail" => a.tail = need(&mut it, "--tail").parse().unwrap_or_else(|_| die("--tail SECS")),
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            _ if a.cmd.is_empty() => a.cmd = x,
            _ if a.input.is_none() => a.input = Some(x),
            _ => die(&format!("unexpected argument '{x}'\n{USAGE}")),
        }
    }
    a
}

fn variants(src: &song::SongSrc, v: &Option<String>) -> Vec<Option<String>> {
    match v.as_deref() {
        Some("all") => std::iter::once(None).chain(src.variants.iter().map(|v| Some(v.name.clone()))).collect(),
        Some(x) => vec![Some(x.to_string())],
        None => vec![None],
    }
}

fn out_path(out: &str, variant: &Option<String>, many: bool) -> PathBuf {
    let p = Path::new(out);
    match (variant, many) {
        (Some(v), true) => {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            p.with_file_name(format!("{stem}_{v}.{ext}"))
        }
        _ => p.to_path_buf(),
    }
}

fn main() {
    let a = parse_args();
    match a.cmd.as_str() {
        "instruments" => {
            for (i, n) in gm::PROGRAMS.iter().enumerate() {
                println!("{:>3}  {n}", i + 1);
            }
            println!("\ndrum kits (track option drums=KIT): {}", gm::KITS.iter().map(|k| k.0).collect::<Vec<_>>().join(" "));
            return;
        }
        "synths" => {
            println!("built-in synth presets (track option synth=NAME; WAV renders only):");
            for (n, d, _) in instance_music::synth::PRESETS {
                println!("  {n:<9} {d}");
            }
            println!("
synth track options:");
            for (n, d) in instance_music::synth::OPTIONS {
                println!("  {n:<8} {d}");
            }
            return;
        }
        "drums" => {
            let mut keys: Vec<u8> = gm::DRUMS.iter().map(|d| d.1).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let names: Vec<&str> = gm::DRUMS.iter().filter(|d| d.1 == k).map(|d| d.0).collect();
                println!("{k:>3}  {}", names.join(" / "));
            }
            return;
        }
        "render" | "info" => {}
        "" => die(USAGE),
        c => die(&format!("unknown command '{c}'\n{USAGE}")),
    }
    let input = a.input.clone().unwrap_or_else(|| die(USAGE));
    let text = std::fs::read_to_string(&input).unwrap_or_else(|e| die(&format!("{input}: {e}")));
    let src = song::parse(&text).unwrap_or_else(|e| die(&format!("{input}: {e}")));
    let vs = variants(&src, &a.variant);
    let many = vs.len() > 1;

    if a.cmd == "info" {
        let mut infos = Vec::new();
        for v in &vs {
            let s = compile(&src, v.as_deref()).unwrap_or_else(|e| die(&format!("{input}: {e}")));
            infos.push(info::song_info(&s));
        }
        if a.json {
            println!("{}", serde_json::to_string_pretty(&infos).unwrap());
        } else {
            for (k, i) in infos.iter().enumerate() {
                if k > 0 {
                    println!("\n----------------------------------------\n");
                }
                print!("{}", info::format_info(i));
            }
            if !src.variants.is_empty() && a.variant.is_none() {
                println!("\nvariants: {} (use --variant NAME or --variant all)", src.variants.iter().map(|v| v.name.as_str()).collect::<Vec<_>>().join(", "));
            }
        }
        return;
    }

    let out = a.out.clone().unwrap_or_else(|| die(&format!("render needs -o OUT.mid or -o OUT.wav\n{USAGE}")));
    let is_wav = out.to_ascii_lowercase().ends_with(".wav");
    if !is_wav && !out.to_ascii_lowercase().ends_with(".mid") && !out.to_ascii_lowercase().ends_with(".midi") {
        die("output must end in .mid or .wav");
    }
    // a SoundFont is only needed for tracks the built-in synth does not play
    let need_sf = vs.iter().any(|v| compile(&src, v.as_deref()).map(|s| needs_soundfont(&s)).unwrap_or(true));
    let sf = if is_wav && need_sf {
        let p = find_soundfont(a.soundfont.as_deref()).unwrap_or_else(|| die("WAV output needs a SoundFont (or synth= on every track): pass --soundfont PATH or set INSTANCE_SOUNDFONT"));
        Some(load_soundfont(&p).unwrap_or_else(|e| die(&e)))
    } else {
        None
    };
    for v in &vs {
        let s = compile(&src, v.as_deref()).unwrap_or_else(|e| die(&format!("{input}: {e}")));
        for w in &s.warnings {
            eprintln!("warning: {w}");
        }
        let path = out_path(&out, v, many);
        if is_wav {
            let opts = RenderOpts { rate: a.rate, mono: a.mono, looped: a.looped, tail: a.tail, normalize: a.normalize };
            let (audio, ri) = render_song_with(&s, sf.as_ref(), &opts).unwrap_or_else(|e| die(&e));
            std::fs::write(&path, wav::encode_wav(audio.rate, audio.channels, &audio.samples)).unwrap_or_else(|e| die(&format!("{}: {e}", path.display())));
            let r = analysis::analyse(&audio, a.looped);
            println!(
                "{}: {:.2} s, {} Hz {}, {:.1} MB, peak {:.1} dBFS, RMS {:.1} dBFS{}{}",
                path.display(),
                audio.duration(),
                audio.rate,
                if audio.channels == 1 { "mono" } else { "stereo" },
                (audio.samples.len() * 2) as f64 / 1e6,
                r.peak_dbfs,
                r.rms_dbfs,
                if ri.gain_db.abs() > 0.01 { format!(" (gain {:+.1} dB applied)", ri.gain_db) } else { String::new() },
                r.loop_check.map(|l| format!(", loop seam: {}", l.verdict)).unwrap_or_default()
            );
        } else {
            let data = smf::write_smf(&s);
            std::fs::write(&path, &data).unwrap_or_else(|e| die(&format!("{}: {e}", path.display())));
            println!(
                "{}: {} bytes, {:.1} s, {} tracks, {} notes",
                path.display(),
                data.len(),
                s.duration(),
                s.tracks.len(),
                s.tracks.iter().map(|t| t.notes.len()).sum::<usize>()
            );
        }
    }
}
