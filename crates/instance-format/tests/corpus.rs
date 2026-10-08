//! Whole-corpus validation (ignored by default; run with `cargo test -p instance-format
//! --test corpus -- --ignored`).
//!
//! Walks every directory listed in `GM81_CORPUS` (an OS path list, `;`-separated on
//! Windows; default `C:\Users\Grob\Downloads\old_gamemaker_projects`) recursively and,
//! for every `.gm81` / `.gmk` file, checks that it
//!   (a) reads,
//!   (b) round-trips write → read to an equal project, and
//!   (c) re-written in its original version (v800/v810) has identical inflated contents
//!       ([`canonical_bytes`]). v701 files cannot be written; they are round-tripped
//!       through v800 and (c) is skipped.
//! The files are opaque test inputs and are never copied into the repository.

use std::path::{Path, PathBuf};

use instance_format::*;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| x.eq_ignore_ascii_case("gm81") || x.eq_ignore_ascii_case("gmk"))
        {
            out.push(p);
        }
    }
}

fn check(d: &[u8]) -> std::result::Result<FormatVersion, String> {
    let (p, v) = read_project_versioned(d).map_err(|e| format!("read: {e}"))?;
    let out_v = if v == FormatVersion::V701 {
        FormatVersion::V800
    } else {
        v
    };
    let out = write_project(&p, out_v).map_err(|e| format!("write: {e}"))?;
    let q = read_project(&out).map_err(|e| format!("re-read: {e}"))?;
    if p != q {
        return Err("project changed in round trip".into());
    }
    if v != FormatVersion::V701 {
        let a = canonical_bytes(d).map_err(|e| format!("canonical(original): {e}"))?;
        let b = canonical_bytes(&out).map_err(|e| format!("canonical(rewritten): {e}"))?;
        if a != b {
            let i = a
                .iter()
                .zip(&b)
                .position(|(x, y)| x != y)
                .unwrap_or(a.len().min(b.len()));
            let ctx = |x: &[u8]| {
                let lo = i.saturating_sub(16);
                let hi = (i + 16).min(x.len());
                x[lo..hi]
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            return Err(format!(
                "canonical bytes differ at {i:#x} (len {} vs {})
  original:  {}
  rewritten: {}",
                a.len(),
                b.len(),
                ctx(&a),
                ctx(&b)
            ));
        }
    }
    Ok(v)
}

/// The two user projects that first exposed the 0-frame sprite layout (spec §5.2
/// addendum). Skipped when the files are absent.
#[test]
fn user_projects_with_zero_frame_sprites() {
    let dir = PathBuf::from(r"C:\Users\Grob\Downloads\old_gamemaker_projects\super_old_projects");
    for (name, idx) in [("Amathyst.gm81", 19), ("JetpackGame.gm81", 23)] {
        let Ok(d) = std::fs::read(dir.join(name)) else {
            eprintln!("skipping: {name} not found");
            continue;
        };
        let p = read_project(&d).unwrap_or_else(|e| panic!("{name}: {e}"));
        let s = p.sprites[idx].as_ref().expect("sprite slot exists");
        assert!(s.frames.is_empty(), "{name}");
        check(&d).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
#[ignore = "needs a local project corpus (GM81_CORPUS)"]
fn corpus_reads_and_round_trips() {
    let dirs: Vec<PathBuf> = match std::env::var_os("GM81_CORPUS") {
        Some(v) => std::env::split_paths(&v).collect(),
        None => vec![PathBuf::from(
            r"C:\Users\Grob\Downloads\old_gamemaker_projects",
        )],
    };
    let mut files = Vec::new();
    for d in &dirs {
        walk(d, &mut files);
    }
    files.sort();
    if files.is_empty() {
        eprintln!("skipping: no project files under {dirs:?}");
        return;
    }
    let mut failures = Vec::new();
    for f in &files {
        let d = std::fs::read(f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        match check(&d) {
            Ok(v) => eprintln!("ok   {v:?} {}", f.display()),
            Err(e) => {
                eprintln!("FAIL {}: {e}", f.display());
                failures.push(format!("{}: {e}", f.display()));
            }
        }
    }
    eprintln!("{} files, {} failures", files.len(), failures.len());
    assert!(
        failures.is_empty(),
        "corpus failures:\n{}",
        failures.join("\n")
    );
}
