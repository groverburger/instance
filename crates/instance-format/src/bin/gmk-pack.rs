//! `gmk-pack DIR GAME.gm81 [--version 800|810] [--allow-unlisted]` — build a project
//! file from a project folder written by `gmk-unpack` (and possibly edited; see
//! `docs/project-folder.md`).
//!
//! The format defaults to the `format_version` recorded in `project.toml` (8.1 for new
//! folders); `--version 800` writes a GameMaker 8.0 `.gmk`.
//!
//! Packing fails when a resource has no leaf in `resource_tree.txt`: it would be in the
//! game but not shown in the IDE. `--allow-unlisted` packs anyway, for a project that
//! was already like that.

use std::path::Path;

use instance_format::folder::read_folder_versioned;
use instance_format::*;

fn usage() -> ! {
    eprintln!("usage: gmk-pack DIR GAME.gm81 [--version 800|810] [--allow-unlisted]");
    std::process::exit(2);
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut pos = Vec::new();
    let mut version = None;
    let mut allow_unlisted = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--version" => {
                version = match args.next().as_deref() {
                    Some("800") => Some(FormatVersion::V800),
                    Some("810") => Some(FormatVersion::V810),
                    _ => usage(),
                }
            }
            "--allow-unlisted" => allow_unlisted = true,
            a if a.starts_with("--") => usage(),
            _ => pos.push(a),
        }
    }
    let [dir, game] = pos.as_slice() else { usage() };
    if let Err(e) = run(Path::new(dir), Path::new(game), version, allow_unlisted) {
        eprintln!("gmk-pack: {e}");
        std::process::exit(1);
    }
}

fn run(
    dir: &Path,
    game: &Path,
    version: Option<FormatVersion>,
    allow_unlisted: bool,
) -> Result<(), String> {
    let (p, recorded) =
        read_folder_versioned(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    // The IDE lists only what is in the resource tree: a resource with no leaf would be
    // in the game but invisible in the IDE.
    let missing = p.missing_from_tree();
    if !missing.is_empty() && !allow_unlisted {
        let roots = TreeNode::standard_roots();
        let mut msg = format!(
            "{}: resource_tree.txt does not list these resources, so the IDE would not show them:",
            dir.display()
        );
        for root in &roots {
            let names: Vec<&str> = missing
                .iter()
                .filter(|(k, _, _)| *k == root.kind)
                .map(|(_, _, n)| n.as_str())
                .collect();
            if !names.is_empty() {
                msg += &format!("\n  {}: {}", root.name, names.join(", "));
            }
        }
        msg += "\nList each one in resource_tree.txt: its name on its own line, indented under \
                its root or a group. (--allow-unlisted packs anyway.)";
        return Err(msg);
    }
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
