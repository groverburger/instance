//! Product name, per-user data folder and environment variables, shared by the IDE and
//! the runner.
//!
//! The per-user folder is `%APPDATA%\Instance` (Windows),
//! `~/Library/Application Support/Instance` (macOS) or `$XDG_CONFIG_HOME/Instance`
//! (`~/.config/Instance`, other systems). It holds the IDE's preferences (`ide.json`),
//! installed extension packages (`extensions/`) and the runner's per-game data
//! (`registry_<id>.ini`, `highscore_<id>.txt`).
//!
//! Earlier builds used two folders with the old product name: `gm81-clean` (IDE) and
//! `gm81-runner` (runner). The first time [`data_dir`] runs, the contents of each old
//! folder are copied into the new one (files already in the new folder are kept); the old
//! folders are left in place. A marker file per old folder records that it was copied.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Product name, used for window titles and folder names.
pub const APP_NAME: &str = "Instance";

/// Name of the per-user data folder.
pub const DATA_DIR_NAME: &str = "Instance";

/// Folders of earlier builds (same base directory), copied into [`DATA_DIR_NAME`] once.
pub const LEGACY_DATA_DIR_NAMES: &[&str] = &["gm81-clean", "gm81-runner"];

/// Prefix of the environment variables read by the IDE and the runner.
pub const ENV_PREFIX: &str = "INSTANCE_";

/// Prefix accepted as a fallback, from earlier builds.
pub const LEGACY_ENV_PREFIX: &str = "GM81_";

/// Reads `INSTANCE_<name>`, falling back to `GM81_<name>` (earlier builds).
/// Empty values count as unset.
pub fn env_var_os(name: &str) -> Option<OsString> {
    [ENV_PREFIX, LEGACY_ENV_PREFIX]
        .iter()
        .find_map(|p| std::env::var_os(format!("{p}{name}")).filter(|v| !v.is_empty()))
}

/// [`env_var_os`] as a `String` (`None` if unset or not Unicode).
pub fn env_var(name: &str) -> Option<String> {
    env_var_os(name).and_then(|v| v.into_string().ok())
}

/// The base folder for per-user configuration of this system, if known
/// (`%APPDATA%`, `~/Library/Application Support`, `$XDG_CONFIG_HOME` or `~/.config`).
pub fn config_base() -> Option<PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(windows) {
        env("APPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library").join("Application Support"))
    } else {
        env("XDG_CONFIG_HOME").or_else(|| env("HOME").map(|h| h.join(".config")))
    }
}

/// The per-user data folder (`<config base>/Instance`), after migrating the folders of
/// earlier builds into it. `None` where there is no file system (the browser) or no
/// known base folder. The folder itself is not created unless something was migrated.
pub fn data_dir() -> Option<PathBuf> {
    let base = config_base()?;
    Some(data_dir_in(&base))
}

/// [`data_dir`] below an explicit base folder (also used by tests).
pub fn data_dir_in(base: &Path) -> PathBuf {
    let dir = base.join(DATA_DIR_NAME);
    for legacy in LEGACY_DATA_DIR_NAMES {
        migrate(&base.join(legacy), &dir, legacy);
    }
    dir
}

/// Name of the marker file that records that `legacy` was copied.
fn marker_name(legacy: &str) -> String {
    format!(".migrated-from-{legacy}")
}

/// Copies the contents of `old` into `new` once (marker file in `new`), keeping files
/// that already exist in `new`. Errors are ignored: migration is best effort.
fn migrate(old: &Path, new: &Path, legacy: &str) {
    if !old.is_dir() {
        return;
    }
    let marker = new.join(marker_name(legacy));
    if marker.exists() {
        return;
    }
    if std::fs::create_dir_all(new).is_err() {
        return;
    }
    copy_missing(old, new);
    let _ = std::fs::write(&marker, format!("copied from {}\n", old.display()));
}

fn copy_missing(from: &Path, to: &Path) {
    let Ok(rd) = std::fs::read_dir(from) else {
        return;
    };
    for e in rd.flatten() {
        let src = e.path();
        let dst = to.join(e.file_name());
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if std::fs::create_dir_all(&dst).is_ok() {
                copy_missing(&src, &dst);
            }
        } else if ft.is_file() && !dst.exists() {
            let _ = std::fs::copy(&src, &dst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("instance-appdirs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn migrates_both_legacy_folders_once_and_keeps_them() {
        let base = tmp("migrate");
        std::fs::create_dir_all(base.join("gm81-clean/extensions")).unwrap();
        std::fs::write(base.join("gm81-clean/ide.json"), "{\"old\":1}").unwrap();
        std::fs::write(base.join("gm81-clean/extensions/a.ged"), "ged").unwrap();
        std::fs::create_dir_all(base.join("gm81-runner")).unwrap();
        std::fs::write(base.join("gm81-runner/highscore_7.txt"), "hs").unwrap();

        let dir = data_dir_in(&base);
        assert_eq!(dir, base.join("Instance"));
        assert_eq!(std::fs::read_to_string(dir.join("ide.json")).unwrap(), "{\"old\":1}");
        assert_eq!(std::fs::read_to_string(dir.join("extensions/a.ged")).unwrap(), "ged");
        assert_eq!(std::fs::read_to_string(dir.join("highscore_7.txt")).unwrap(), "hs");
        // the old folders stay
        assert!(base.join("gm81-clean/ide.json").is_file());
        assert!(base.join("gm81-runner/highscore_7.txt").is_file());

        // a second start does not copy again (deleted files stay deleted, edits are kept)
        std::fs::remove_file(dir.join("highscore_7.txt")).unwrap();
        std::fs::write(dir.join("ide.json"), "{\"new\":1}").unwrap();
        data_dir_in(&base);
        assert!(!dir.join("highscore_7.txt").exists());
        assert_eq!(std::fs::read_to_string(dir.join("ide.json")).unwrap(), "{\"new\":1}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn existing_files_in_the_new_folder_win() {
        let base = tmp("keep");
        std::fs::create_dir_all(base.join("Instance")).unwrap();
        std::fs::write(base.join("Instance/ide.json"), "new").unwrap();
        std::fs::create_dir_all(base.join("gm81-clean")).unwrap();
        std::fs::write(base.join("gm81-clean/ide.json"), "old").unwrap();
        std::fs::write(base.join("gm81-clean/other.txt"), "x").unwrap();
        let dir = data_dir_in(&base);
        assert_eq!(std::fs::read_to_string(dir.join("ide.json")).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(dir.join("other.txt")).unwrap(), "x");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn nothing_to_migrate_creates_nothing() {
        let base = tmp("empty");
        let dir = data_dir_in(&base);
        assert!(!dir.exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn env_prefers_instance_names() {
        // unique names so parallel tests do not interfere
        std::env::set_var("GM81_APPDIRS_TEST_A", "old");
        assert_eq!(env_var("APPDIRS_TEST_A").as_deref(), Some("old"));
        std::env::set_var("INSTANCE_APPDIRS_TEST_A", "new");
        assert_eq!(env_var("APPDIRS_TEST_A").as_deref(), Some("new"));
        std::env::set_var("INSTANCE_APPDIRS_TEST_A", "");
        assert_eq!(env_var("APPDIRS_TEST_A").as_deref(), Some("old"));
        assert_eq!(env_var("APPDIRS_TEST_UNSET"), None);
    }
}
