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
        }
    }
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("gm81-clean").join("ide.json"))
}

impl Prefs {
    pub fn load() -> Prefs {
        config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = config_path() {
            if let Some(d) = p.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            if let Ok(s) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(p, s);
            }
        }
    }

    /// Put `path` at the top of the recent list.
    pub fn add_recent(&mut self, path: PathBuf) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(MAX_RECENT);
    }
}
