//! Per-user IDE preferences (specs/ide.md §19), stored as JSON in the user config dir.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Maximum number of recent files (spec §3.2).
pub const MAX_RECENT: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub recent_files: Vec<PathBuf>,
    /// Explicit runner executable path (empty = auto-detect).
    pub runner_path: String,
    /// Code editor font size.
    pub code_font_size: f32,
    pub show_line_numbers: bool,
    pub syntax_colouring: bool,
    /// Show the origin cross in sprite previews.
    pub show_origin: bool,
    /// Room grid on by default.
    pub room_grid: bool,
    /// The welcome dialog was shown (desktop: it opens by itself only the first time).
    pub welcome_seen: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            recent_files: Vec::new(),
            runner_path: String::new(),
            code_font_size: 13.0,
            show_line_numbers: true,
            syntax_colouring: true,
            show_origin: true,
            room_grid: true,
            welcome_seen: false,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn config_path() -> Option<PathBuf> {
    Some(crate::platform::config_dir()?.join("ide.json"))
}

/// localStorage key of the preferences on the web.
#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "instance/ide.json";

/// localStorage key used by earlier builds, read when [`STORAGE_KEY`] is not set yet.
#[cfg(target_arch = "wasm32")]
const LEGACY_STORAGE_KEY: &str = "gm81-clean/ide.json";

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn read_text() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let st = storage()?;
        match st.get_item(STORAGE_KEY).ok()? {
            Some(s) => Some(s),
            None => st.get_item(LEGACY_STORAGE_KEY).ok()?,
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(config_path()?).ok()
    }
}

fn write_text(s: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(st) = storage() {
            let _ = st.set_item(STORAGE_KEY, s);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(p) = config_path() {
            if let Some(d) = p.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(p, s);
        }
    }
}

impl Prefs {
    pub fn load() -> Prefs {
        read_text()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(s) = serde_json::to_string_pretty(self) {
            write_text(&s);
        }
    }

    /// Put `path` at the top of the recent list.
    pub fn add_recent(&mut self, path: PathBuf) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(MAX_RECENT);
    }
}
