//! Run normally / Run in Debug mode (specs/ide.md §20.1, §20.3): save the project
//! to a temporary `.gm81` and launch the runner executable with it. The runner
//! never shows a console (`--no-console`, and no console window on Windows): its
//! stdout / stderr are piped into the IDE's Output Log (`output_log.rs`);
//! debug runs add `--debug` (the runner's Debug Information window).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use instance_format::Project;

/// File name of the runner executable.
pub fn runner_file_name() -> &'static str {
    if cfg!(windows) {
        "instance-runner.exe"
    } else {
        "instance-runner"
    }
}

/// Locate the runner: the configured path, next to the IDE executable, or in a cargo
/// `target/{debug,release}` directory above the IDE executable.
pub fn find_runner(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(c) = configured.filter(|c| !c.trim().is_empty()) {
        let p = PathBuf::from(c.trim());
        // a path saved by an earlier build names the runner by its old name
        // (gm-runner): prefer the renamed runner next to it
        if p.file_stem().is_some_and(|s| s == "gm-runner") {
            if let Some(renamed) = p.parent().map(|d| d.join(runner_file_name())) {
                if renamed.is_file() {
                    return Some(renamed);
                }
            }
        }
        return p.is_file().then_some(p);
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let here = dir.join(runner_file_name());
    if here.is_file() {
        return Some(here);
    }
    for anc in dir.ancestors() {
        if anc.file_name().is_some_and(|n| n == "target") {
            for profile in ["debug", "release"] {
                let p = anc.join(profile).join(runner_file_name());
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// Temporary folder of this IDE instance.
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir()
        .join("instance-ide")
        .join(std::process::id().to_string())
}

/// Write the project to a temporary game file.
pub fn write_temp_game(p: &Project) -> Result<PathBuf, String> {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = dir.join("game.gm81");
    let bytes = crate::project_ops::project_bytes(p)?;
    std::fs::write(&file, bytes).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(file)
}

/// Runner arguments for playing `game`.
pub fn runner_args(game: &Path, debug: bool) -> Vec<OsString> {
    let mut a: Vec<OsString> = vec![game.into(), "--no-console".into()];
    if debug {
        a.push("--debug".into());
    }
    a
}

/// Check the preconditions, write the game and build the runner command (not
/// yet started).
pub fn prepare(
    p: &Project,
    runner: Option<&str>,
    project_path: Option<&Path>,
    debug: bool,
) -> Result<Command, String> {
    if p.rooms.iter().flatten().next().is_none() {
        return Err("A game needs at least one room to run.".into());
    }
    let exe = runner_exe(runner)?;
    let game = write_temp_game(p)?;
    let workdir = project_path
        .and_then(|p| p.parent())
        .filter(|d| d.is_dir())
        .map(Path::to_path_buf)
        .unwrap_or_else(temp_dir);
    Ok(command(&exe, &game, &workdir, debug))
}

/// The command that plays an example game (welcome.rs): `bytes` saved as `name` in
/// the temporary folder, which is also its working directory.
#[cfg(not(target_arch = "wasm32"))]
pub fn prepare_example(name: &str, bytes: &[u8], runner: Option<&str>) -> Result<Command, String> {
    let exe = runner_exe(runner)?;
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let game = dir.join(name);
    std::fs::write(&game, bytes).map_err(|e| format!("{}: {e}", game.display()))?;
    Ok(command(&exe, &game, &dir, false))
}

/// The runner executable, or why there is none.
fn runner_exe(runner: Option<&str>) -> Result<PathBuf, String> {
    find_runner(runner).ok_or_else(|| {
        format!(
            "Runner executable '{}' not found next to the IDE or in a cargo target \
             directory. Set its path in File > Preferences.",
            runner_file_name()
        )
    })
}

/// The runner playing `game` in `workdir`, its output piped (for the Output Log).
fn command(exe: &Path, game: &Path, workdir: &Path, debug: bool) -> Command {
    let mut c = Command::new(exe);
    c.args(runner_args(game, debug))
        .current_dir(workdir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        // never a console window, even if the runner were a console program
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

/// Remove this instance's temporary folder.
pub fn cleanup() {
    let _ = std::fs::remove_dir_all(temp_dir());
}
