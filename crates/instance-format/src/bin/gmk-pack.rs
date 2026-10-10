//! `gmk-pack DIR GAME.gm81 [--version 800|810]` — build a project file from a project
//! folder written by `gmk-unpack` (and possibly edited; see `docs/project-folder.md`).
//!
//! The format defaults to the `format_version` recorded in `project.toml` (8.1 for new
//! folders); `--version 800` writes a GameMaker 8.0 `.gmk`.

use std::path::Path;

use instance_format::folder::read_folder_versioned;
use instance_format::*;

fn usage() -> ! {
    eprintln!("usage: gmk-pack DIR GAME.gm81 [--version 800|810]");
    std::process::exit(2);
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut pos = Vec::new();
    let mut version = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--version" => {
                version = match args.next().as_deref() {
                    Some("800") => Some(FormatVersion::V800),
                    Some("810") => Some(FormatVersion::V810),
                    _ => usage(),
                }
            }
            a if a.starts_with("--") => usage(),
            _ => pos.push(a),
        }
    }
    let [dir, game] = pos.as_slice() else { usage() };
    if let Err(e) = run(Path::new(dir), Path::new(game), version) {
        eprintln!("gmk-pack: {e}");
        std::process::exit(1);
    }
}

fn run(dir: &Path, game: &Path, version: Option<FormatVersion>) -> Result<(), String> {
    let (p, recorded) =
        read_folder_versioned(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let v = version.unwrap_or(recorded);
    let bytes = write_project(&p, v).map_err(|e| e.to_string())?;
    std::fs::write(game, bytes).map_err(|e| format!("{}: {e}", game.display()))?;
    let used: usize = p.slot_counts().iter().map(|(_, c)| c.used).sum();
    println!(
        "packed {} ({used} resources) into {} (v{})",
        dir.display(),
        game.display(),
        v.number()
    );
    Ok(())
}
