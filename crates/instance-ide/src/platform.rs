//! Small platform abstractions: wall-clock time, per-user config directory and
//! system font directories (Windows, macOS, Linux; nothing on the web).

use std::path::PathBuf;

/// Seconds since the Unix epoch (`SystemTime::now` panics on wasm32-unknown-unknown).
pub fn unix_secs() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() / 1000.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
}

/// Nanoseconds since the Unix epoch (for seeds).
pub fn unix_nanos() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        // millisecond clock plus a little entropy
        (js_sys::Date::now() * 1e6) as u64 ^ ((js_sys::Math::random() * 1e6) as u64)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
    }
}

/// The per-user configuration directory (`%APPDATA%\Instance` etc., see
/// `instance_brand::appdirs`; the folders of earlier builds are migrated on first use).
/// `None` on the web.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn config_dir() -> Option<PathBuf> {
    instance_brand::appdirs::data_dir()
}

pub use instance_ui::fonts::font_dirs;

/// Opens `path` with the program the system associates with it (e.g. a `.chm` help
/// file in the Windows help viewer). Not available on the web.
pub fn open_with_os(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        Err(format!("{} cannot be opened in the browser.", path.display()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let program = if cfg!(windows) {
            "explorer"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        std::process::Command::new(program)
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("{}: {e}", path.display()))
    }
}
