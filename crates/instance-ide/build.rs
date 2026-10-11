//! Embeds the Instance app icon (rendered by instance-brand) into the Windows executable,
//! and lists the user manual (`docs/manual/**/*.md` and its images) for embedding into
//! the IDE's help viewer (`$OUT_DIR/manual_files.rs`, included by `src/help/mod.rs`).

use std::path::{Path, PathBuf};

/// File extensions of the manual that are embedded.
const MANUAL_EXTS: &[&str] = &["md", "png", "jpg", "jpeg", "gif", "bmp"];

fn collect(dir: &Path, rel: &str, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        let r = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        if p.is_dir() {
            collect(&p, &r, out);
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| MANUAL_EXTS.contains(&x.to_ascii_lowercase().as_str()))
        {
            out.push((r, p));
        }
    }
}

fn manual() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let dir = manifest.join("../../docs/manual");
    // (a directory: cargo rebuilds when anything inside it changes, so pages added
    // later are picked up)
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files = Vec::new();
    collect(&dir, "", &mut files);
    files.sort();
    let mut src = String::from("/// Every file of the manual: (path relative to `docs/manual`, contents).\npub static MANUAL_FILES: &[(&str, &[u8])] = &[\n");
    for (rel, path) in &files {
        src.push_str(&format!("    ({rel:?}, include_bytes!({:?})),\n", path.display().to_string()));
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("manual_files.rs");
    std::fs::write(out, src).expect("write manual_files.rs");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    manual();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os == "windows" && env == "msvc" {
        // link.exe accepts a compiled resource file as an input, so no resource
        // compiler is needed.
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("app_icon.res");
        std::fs::write(&out, instance_brand::res_file(&instance_brand::ICO_SIZES)).expect("write app_icon.res");
        println!("cargo:rustc-link-arg-bins={}", out.display());
    }
}
