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
pub mod draw;
pub mod events;
pub mod game;
pub mod host;
pub mod input;
pub mod instance;
pub mod particles;
pub mod path;
pub mod profile;
pub mod render;
pub mod res;
pub mod room;
pub mod save;
pub mod spatial;

pub use game::{Game, HeadlessUi, Pending, UiHooks};
pub use input::InputEvent;

/// Native stack size for the thread running game logic (deep GML recursion).
pub const GAME_STACK_SIZE: usize = 256 * 1024 * 1024;

/// Loads a project file and prepares a game (no room entered yet).
pub fn load_game(path: &std::path::Path, vm: &mut gml::Vm, headless: bool) -> Result<Game, String> {
    let (project, _) = gm_format::read_project_file(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    vm.settings.max_call_depth = 4096;
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
