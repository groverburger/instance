//! Converts a local corpus of Studio projects (not part of the repository).
//!
//! `GMX_CORPUS` lists directories or files separated by `;` (default: the
//! developer's project folder). Every `*.project.gmx` below a directory and every
//! `.gmz` given is converted, written as GM 8.1 and read back.
//!
//! Run with `cargo test -p gm-gmx --test corpus -- --ignored --nocapture`.

use std::path::{Path, PathBuf};

fn find(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = p.file_name().unwrap().to_string_lossy().to_ascii_lowercase();
        if p.is_dir() {
            if name != "converted" {
                find(&p, out);
            }
        } else if name.ends_with(".project.gmx") || name.ends_with(".gmz") {
            out.push(p);
        }
    }
}

#[test]
#[ignore]
fn convert_corpus() {
    let spec = std::env::var("GMX_CORPUS").unwrap_or_else(|_| {
        r"C:\Users\Grob\Downloads\old_gamemaker_projects;C:\Users\Grob\Downloads\SpelunkyClassic-YoYo.gmz".into()
    });
    let mut inputs = Vec::new();
    for part in spec.split(';').filter(|s| !s.is_empty()) {
        let p = PathBuf::from(part);
        if p.is_dir() {
            find(&p, &mut inputs);
        } else if p.exists() {
            inputs.push(p);
        }
    }
    if inputs.is_empty() {
        eprintln!("no corpus found ({spec}); nothing to do");
        return;
    }
    let mut failed = Vec::new();
    for input in &inputs {
        match gm_gmx::load_gmx(input) {
            Ok((p, warnings)) => {
                let bytes = gm_format::write_project(&p, gm_format::FormatVersion::V810).expect("write");
                let back = gm_format::read_project(&bytes).expect("read back");
                assert_eq!(back.objects.len(), p.objects.len());
                let untranslated = warnings.iter().filter(|w| w.message.contains("not translated")).count();
                let stubs: Vec<&gm_gmx::Warning> =
                    warnings.iter().filter(|w| w.context == "unknown functions").collect();
                println!(
                    "{}: {} objects, {} rooms, {} scripts, {} warnings, {} untranslated pieces{}",
                    input.display(),
                    p.objects.iter().flatten().count(),
                    p.rooms.iter().flatten().count(),
                    p.scripts.iter().flatten().count(),
                    warnings.len(),
                    untranslated,
                    stubs.first().map(|w| format!("; {}", w.message)).unwrap_or_default()
                );
                if untranslated > 0 {
                    failed.push(format!("{}: {untranslated} code pieces failed to parse", input.display()));
                }
            }
            Err(e) => failed.push(format!("{}: {e}", input.display())),
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}
