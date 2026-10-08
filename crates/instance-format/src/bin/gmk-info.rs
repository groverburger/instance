//! `gmk-info FILE [--tree]` — print a resource summary of a .gmk / .gm81 project file,
//! or of a .lib action library.

use instance_format::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let show_tree = args.iter().any(|a| a == "--tree");
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("usage: gmk-info FILE.gmk|FILE.gm81|FILE.lib [--tree]");
        std::process::exit(2);
    };
    if let Err(e) = run(path, show_tree) {
        eprintln!("gmk-info: {path}: {e}");
        std::process::exit(1);
    }
}

fn list<T>(label: &str, items: &[Option<T>], name: impl Fn(&T) -> String) {
    let used = items.iter().filter(|x| x.is_some()).count();
    println!("{label}: {used} ({} slots)", items.len());
    for (i, it) in items.iter().enumerate() {
        if let Some(t) = it {
            println!("  [{i}] {}", name(t));
        }
    }
}

fn run(path: &str, show_tree: bool) -> Result<()> {
    if path.to_ascii_lowercase().ends_with(".lib") {
        let l = read_library_file(path)?;
        println!(
            "library '{}' id {} version {} ({} entries)",
            l.tab_caption,
            l.library_id,
            l.version,
            l.actions.len()
        );
        for a in l.actions.iter().filter(|a| !a.is_decoration()) {
            println!("  {} {} ({})", a.action_id, a.name, a.palette_title);
        }
        return Ok(());
    }
    let (p, v) = read_project_file(path)?;
    println!("format: v{}  game id: {}", v.number(), p.game_id);
    println!(
        "triggers: {}  constants: {}  included files: {}  extensions: {:?}",
        p.triggers.iter().flatten().count(),
        p.constants.len(),
        p.included_files.len(),
        p.extension_packages
    );
    list("sounds", &p.sounds, |s| {
        format!("{} ({})", s.name, s.file_name)
    });
    list("sprites", &p.sprites, |s| {
        format!(
            "{} {}x{} x{}",
            s.name,
            s.width(),
            s.height(),
            s.frames.len()
        )
    });
    list("backgrounds", &p.backgrounds, |b| {
        format!("{} {}x{}", b.name, b.image.width, b.image.height)
    });
    list("paths", &p.paths, |x| {
        format!("{} ({} points)", x.name, x.points.len())
    });
    list("scripts", &p.scripts, |s| {
        format!("{} ({} bytes)", s.name, s.code.len())
    });
    list("fonts", &p.fonts, |f| {
        format!("{} ({} {})", f.name, f.font_name, f.size)
    });
    list("timelines", &p.timelines, |t| {
        format!("{} ({} moments)", t.name, t.moments.len())
    });
    list("objects", &p.objects, |o| {
        let n: usize = o.events.iter().map(|e| e.actions.len()).sum();
        format!("{} ({} events, {n} actions)", o.name, o.events.len())
    });
    list("rooms", &p.rooms, |r| {
        format!(
            "{} {}x{} ({} instances, {} tiles)",
            r.name,
            r.width,
            r.height,
            r.instances.len(),
            r.tiles.len()
        )
    });
    println!("room order: {:?}", p.room_order);
    if show_tree {
        println!("resource tree:");
        for root in &p.resource_tree {
            root.walk(&mut |n, d| {
                println!(
                    "  {}{} [{:?} {}]",
                    "  ".repeat(d),
                    n.name,
                    n.status,
                    n.index
                )
            });
        }
    }
    Ok(())
}
