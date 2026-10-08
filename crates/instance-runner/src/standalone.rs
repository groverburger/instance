//! Standalone executables (specs/exe-format.md): when started without a game file,
//! the runner looks for a game appended to its own executable.

use std::path::{Path, PathBuf};

/// Environment variable set to the path of the extracted extension bundle (v2 footers),
/// for the extension loader.
pub const BUNDLE_ENV: &str = "GM81CLEAN_EXTENSION_BUNDLE";

/// Read the embedded game (payload and optional extension bundle) of `exe`.
pub fn embedded_in(exe: &Path) -> Option<instance_format::exe::Embedded> {
    let data = std::fs::read(exe).ok()?;
    instance_format::exe::read_embedded(&data)
}

/// If the running executable carries a game, extract it to a temporary `.gm81` and
/// return its path. The current directory becomes the executable's directory, so the
/// game's working directory is where the executable is.
pub fn extract_own_game() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let e = embedded_in(&exe)?;
    let dir = std::env::temp_dir()
        .join("gm81-clean-standalone")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&dir).ok()?;
    let game = dir.join("game.gm81");
    std::fs::write(&game, &e.payload).ok()?;
    if let Some(b) = &e.bundle {
        let bp = dir.join("extensions.bundle");
        if std::fs::write(&bp, b).is_ok() {
            std::env::set_var(BUNDLE_ENV, &bp);
        }
    }
    if let Some(d) = exe.parent() {
        let _ = std::env::set_current_dir(d);
    }
    Some(game)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_payload_of_built_exe() {
        let p = instance_format::Project::default();
        let payload = instance_format::write_project(&p, instance_format::FormatVersion::V810).unwrap();
        let dir = std::env::temp_dir().join(format!("instance-runner-standalone-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for bundle in [None, Some(instance_format::exe::write_bundle(&[]))] {
            let exe = dir.join("game.exe");
            std::fs::write(&exe, instance_format::exe::build(b"MZ fake runner", &payload, bundle.as_deref())).unwrap();
            let e = embedded_in(&exe).expect("payload found");
            assert_eq!(e.payload, payload);
            assert_eq!(e.bundle, bundle);
            assert!(instance_format::read_project(&e.payload).unwrap() == p);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
