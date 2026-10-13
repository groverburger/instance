//! `gmk-web GAME OUT.html [--title TEXT] [--runner DIR] [--soundfont FILE.sf2 | --no-soundfont]`
//! — export a game (a `.gm81` / `.gmk` file or a project folder) as one self-contained
//! web page that plays in any modern browser, opened from disk or served.
//!
//! The page holds the web runner, built by `scripts/build-web.*` into `dist/web/pkg`
//! (found next to this program or above it, or given with `--runner` or
//! `$INSTANCE_WEB_RUNNER`). A game with MIDI sounds also gets a SoundFont
//! (`--soundfont`, `$INSTANCE_SOUNDFONT`, or `soundfonts/default.sf2` beside the web
//! runner); without one its MIDI music is silent.

use std::path::{Path, PathBuf};

use instance_format::web::{self, WebRunner};
use instance_format::*;

fn usage() -> ! {
    eprintln!(
        "usage: gmk-web GAME OUT.html [--title TEXT] [--runner DIR] [--soundfont FILE.sf2 | --no-soundfont]"
    );
    std::process::exit(2);
}

struct Opts {
    title: Option<String>,
    runner: Option<PathBuf>,
    soundfont: Option<PathBuf>,
    no_soundfont: bool,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut pos = Vec::new();
    let mut o = Opts { title: None, runner: None, soundfont: None, no_soundfont: false };
    while let Some(a) = args.next() {
        match a.as_str() {
            "--title" => o.title = Some(args.next().unwrap_or_else(|| usage())),
            "--runner" => o.runner = Some(args.next().unwrap_or_else(|| usage()).into()),
            "--soundfont" => o.soundfont = Some(args.next().unwrap_or_else(|| usage()).into()),
            "--no-soundfont" => o.no_soundfont = true,
            a if a.starts_with("--") => usage(),
            _ => pos.push(a),
        }
    }
    let [game, out] = pos.as_slice() else { usage() };
    if let Err(e) = run(Path::new(game), Path::new(out), o) {
        eprintln!("gmk-web: {e}");
        std::process::exit(1);
    }
}

fn run(game: &Path, out: &Path, o: Opts) -> Result<(), String> {
    let p = read_project_any(game).map_err(|e| format!("{}: {e}", game.display()))?;
    if p.rooms.iter().flatten().next().is_none() {
        return Err("a game needs at least one room".into());
    }
    let Some(runner_dir) = web::find_runner(o.runner.as_deref()) else {
        return Err("the web runner was not found: build it with scripts/build-web.sh (or .ps1), \
                    or pass --runner DIR (the folder with instance_runner.js and instance_runner_bg.wasm)"
            .into());
    };
    let runner = WebRunner::read(&runner_dir)?;
    let mut soundfont = None;
    if web::uses_midi(&p) && !o.no_soundfont {
        match o.soundfont.or_else(|| web::find_soundfont(Some(&runner_dir))) {
            Some(sf) => soundfont = Some(std::fs::read(&sf).map_err(|e| format!("{}: {e}", sf.display()))?),
            None => eprintln!(
                "gmk-web: warning: no SoundFont found (--soundfont FILE.sf2); the game's MIDI music will be silent"
            ),
        }
    }
    if !p.extension_packages.is_empty() {
        eprintln!("gmk-web: warning: extension packages are not included in web pages");
    }
    let bytes = write_project(&p, FormatVersion::V810).map_err(|e| e.to_string())?;
    let stem = |q: &Path| q.file_stem().map(|s| s.to_string_lossy().into_owned());
    let title = o.title.or_else(|| stem(game)).unwrap_or_default();
    let html = web::page(&title, &runner, &bytes, soundfont.as_deref());
    std::fs::write(out, &html).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{} ({:.1} MB)", out.display(), html.len() as f64 / 1e6);
    Ok(())
}
