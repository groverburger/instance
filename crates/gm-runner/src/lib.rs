//! Clean-room GameMaker 8.1 runner (specs/runtime.md).
//!
//! The [`Game`] holds the loaded project and all runtime state and implements
//! [`gml::Host`]. A frontend (windowed or headless, see `main.rs`) drives it by
//! calling [`Game::step`] at `room_speed` and presenting [`Game::frame_image`].

pub mod action_lib;
pub mod actions;
pub mod audio;
pub mod builtins;
pub mod collision;
pub mod d3d;
pub mod draw;
pub mod events;
pub mod ext;
pub mod frontend;
pub mod game;
pub mod host;
pub mod input;
pub mod instance;
pub mod net;
pub mod joystick;
pub mod particles;
pub mod path;
pub mod profile;
pub mod render;
pub mod res;
pub mod room;
pub mod save;
pub mod spatial;
pub mod vfs;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use game::{Game, HeadlessUi, Pending, UiHooks};
pub use input::InputEvent;

/// Native stack size for the thread running game logic (deep GML recursion).
pub const GAME_STACK_SIZE: usize = 256 * 1024 * 1024;

/// GML call depth allowed before "stack overflow" errors. Desktop runs use a
/// [`GAME_STACK_SIZE`] thread. In the browser two stacks matter: the wasm
/// shadow stack in linear memory (32 MiB, set in `.cargo/config.toml`) and the
/// engine's own call stack, which is the real limit (about 850 levels of a
/// simple recursive script in Edge/Chrome); 512 leaves room for complex
/// expressions, so deep recursion ends in a GML error rather than a crash.
pub const MAX_CALL_DEPTH: u32 = if cfg!(target_arch = "wasm32") { 512 } else { 4096 };

/// Prints a warning on stderr (desktop) or the browser console (web).
pub fn warn(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::warn_1(&msg.into());
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{}", msg);
}

/// Prepares a game from the bytes of a project file (no room entered yet);
/// `dir` becomes the working and program directory. Used by the web runner,
/// where `dir` lives in the virtual file system ([`vfs`]).
pub fn load_game_bytes(data: &[u8], dir: &str, vm: &mut gml::Vm, headless: bool) -> Result<Game, String> {
    ext::bundle::set_project_dir(None);
    let (project, _) = gm_format::read_project_versioned(data).map_err(|e| e.to_string())?;
    // saved files of different games must not collide in the browser's storage
    vfs::set_namespace(&project.game_id.to_string());
    vm.settings.max_call_depth = MAX_CALL_DEPTH;
    let mut g = Game::new(project, vm, headless)?;
    g.working_directory = dir.to_string();
    g.program_directory = dir.to_string();
    g.export_included_files();
    Ok(g)
}

/// Loads a project file and prepares a game (no room entered yet).
pub fn load_game(path: &std::path::Path, vm: &mut gml::Vm, headless: bool) -> Result<Game, String> {
    ext::bundle::set_project_dir(path.parent().map(|p| p.to_path_buf()));
    let (project, _) = gm_format::read_project_file(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    vm.settings.max_call_depth = MAX_CALL_DEPTH;
    let mut g = Game::new(project, vm, headless)?;
    let dir = path
        .parent()
        .map(|p| if p.as_os_str().is_empty() { std::path::Path::new(".") } else { p })
        .unwrap_or(std::path::Path::new("."));
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut d = dir.to_string_lossy().trim_start_matches("\\?\\").to_string();
    // a game started from the IDE lives in the temp folder: use the current directory
    let tmp = std::fs::canonicalize(std::env::temp_dir()).unwrap_or_else(|_| std::env::temp_dir());
    if dir.starts_with(&tmp) {
        if let Ok(cwd) = std::env::current_dir() {
            d = cwd.to_string_lossy().to_string();
        }
    }
    g.working_directory = d.clone();
    g.program_directory = d;
    g.export_included_files();
    Ok(g)
}
