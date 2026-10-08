//! `gmx2gm81 <input.gmx-dir | project.gmx | game.gmz> [-o out.gm81] [options]`
//!
//! Converts a GameMaker: Studio 1.x project to a GameMaker 8.1 `.gm81` file.

use std::path::PathBuf;
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!(
        "usage: gmx2gm81 <input.gmx-dir | file.project.gmx | file.gmz> [-o out.gm81]\n\
         \x20      [--quiet] [--no-compat] [--gm8-create-order] [--string-escapes]\n\
         \x20      [--uninit-zero (default) | --uninit-error | --uninit-studio]\n\
         \x20      [--system-fonts] [--dump-code DIR]"
    );
    ExitCode::from(2)
}

/// Write every converted code piece to `dir` (for reviewing the translation).
fn dump_code(p: &instance_format::Project, dir: &std::path::Path) -> std::io::Result<()> {
    use instance_format::action_kind;
    let clean = |s: &str| s.replace(|c: char| !c.is_ascii_alphanumeric() && c != '_', "_");
    std::fs::create_dir_all(dir.join("scripts"))?;
    std::fs::create_dir_all(dir.join("objects"))?;
    std::fs::create_dir_all(dir.join("rooms"))?;
    for s in p.scripts.iter().flatten() {
        std::fs::write(dir.join("scripts").join(format!("{}.gml", clean(&s.name))), &s.code)?;
    }
    for o in p.objects.iter().flatten() {
        let mut out = String::new();
        for e in &o.events {
            for (i, a) in e.actions.iter().enumerate() {
                out.push_str(&format!("// ---- event {}/{} action {} (lib {} id {})\n", e.event_type, e.number, i + 1, a.library_id, a.action_id));
                if a.kind == action_kind::CODE {
                    out.push_str(&a.args[0]);
                    out.push('\n');
                } else {
                    let n = (a.argument_count.max(0) as usize).min(a.args.len());
                    out.push_str(&format!("// {} args: {:?}\n", a.function_name, &a.args[..n]));
                }
            }
        }
        std::fs::write(dir.join("objects").join(format!("{}.gml", clean(&o.name))), out)?;
    }
    for r in p.rooms.iter().flatten() {
        let mut out = format!("// ---- room creation code\n{}\n", r.creation_code);
        for i in &r.instances {
            if !i.creation_code.is_empty() {
                out.push_str(&format!("// ---- instance {} (object {})\n{}\n", i.id, i.object, i.creation_code));
            }
        }
        std::fs::write(dir.join("rooms").join(format!("{}.gml", clean(&r.name))), out)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut quiet = false;
    let mut dump: Option<PathBuf> = None;
    let mut opts = instance_gmx::Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-o" | "--output" => match args.next() {
                Some(o) => output = Some(o.into()),
                None => return usage(),
            },
            "-q" | "--quiet" => quiet = true,
            "--dump-code" => match args.next() {
                Some(o) => dump = Some(PathBuf::from(o)),
                None => return usage(),
            },
            "--no-compat" => opts.compat = false,
            "--gm8-create-order" => opts.studio_create_order = false,
            "--string-escapes" => opts.string_escapes = true,
            "--system-fonts" => opts.sprite_fonts = false,
            "--uninit-zero" => opts.uninit_as_zero = Some(true),
            "--uninit-error" => opts.uninit_as_zero = Some(false),
            "--uninit-studio" => opts.uninit_as_zero = None,
            "-h" | "--help" => return usage(),
            s if s.starts_with('-') => {
                eprintln!("unknown option {s}");
                return usage();
            }
            _ if input.is_none() => input = Some(a.into()),
            _ => return usage(),
        }
    }
    let Some(input) = input else { return usage() };
    let output = output.unwrap_or_else(|| {
        let mut name = input
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "game".into());
        for suf in [".project.gmx", ".gmx", ".gmz", ".zip", ".7z"] {
            if name.to_ascii_lowercase().ends_with(suf) {
                name.truncate(name.len() - suf.len());
                break;
            }
        }
        let dir = if input.is_dir() { input.parent() } else { input.parent() };
        dir.unwrap_or(std::path::Path::new(".")).join(format!("{name}.gm81"))
    });
    let (project, warnings) = match instance_gmx::load_gmx_with(&input, &opts) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    if !quiet {
        for w in &warnings {
            eprintln!("warning: {w}");
        }
    }
    if let Some(d) = &dump {
        if let Err(e) = dump_code(&project, d) {
            eprintln!("error: dump: {e}");
        }
    }
    let bytes = match instance_format::write_project(&project, instance_format::FormatVersion::V810) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot write project: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::write(&output, bytes) {
        eprintln!("error: {}: {e}", output.display());
        return ExitCode::FAILURE;
    }
    let c = project.slot_counts();
    let summary: Vec<String> = c.iter().map(|(n, s)| format!("{} {n}", s.used)).collect();
    println!(
        "wrote {} ({}; {} warning(s))",
        output.display(),
        summary.join(", "),
        warnings.len()
    );
    ExitCode::SUCCESS
}
