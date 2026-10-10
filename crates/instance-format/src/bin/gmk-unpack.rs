//! `gmk-unpack GAME.gm81 DIR [--force]` — convert a .gm81 / .gmk project into a
//! project folder (GML as .gml files, images as PNG, metadata as TOML; see
//! `docs/project-folder.md`). `gmk-pack` converts it back losslessly.
//!
//! DIR must not exist or be empty unless `--force` is given; with `--force` an
//! existing folder is updated in place (stale resource files are removed, other files
//! such as `.git` are kept).

use std::path::Path;

use instance_format::folder::write_folder_versioned;
use instance_format::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let force = args.iter().any(|a| a == "--force");
    let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let bad_flag = args
        .iter()
        .find(|a| a.starts_with("--") && a.as_str() != "--force");
    let (Some(game), Some(dir), None, None) = (pos.first(), pos.get(1), pos.get(2), bad_flag)
    else {
        eprintln!("usage: gmk-unpack GAME.gm81|GAME.gmk DIR [--force]");
        std::process::exit(2);
    };
    if let Err(e) = run(Path::new(game), Path::new(dir), force) {
        eprintln!("gmk-unpack: {e}");
        std::process::exit(1);
    }
}

fn run(game: &Path, dir: &Path, force: bool) -> Result<(), String> {
    if !force {
        if let Ok(mut rd) = std::fs::read_dir(dir) {
            if rd.next().is_some() {
                return Err(format!(
                    "{}: directory is not empty (use --force to update it in place)",
                    dir.display()
                ));
            }
        }
    }
    let (p, v) = read_project_file(game).map_err(|e| format!("{}: {e}", game.display()))?;
    let v = if v == FormatVersion::V701 {
        FormatVersion::V800
    } else {
        v
    };
    write_folder_versioned(&p, dir, v).map_err(|e| format!("{}: {e}", dir.display()))?;
    let used: usize = p.slot_counts().iter().map(|(_, c)| c.used).sum();
    println!(
        "unpacked {} (v{}, {used} resources) into {}",
        game.display(),
        v.number(),
        dir.display()
    );
    Ok(())
}
