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

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// The per-user configuration directory of gm81-clean (`None` on the web).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn config_dir() -> Option<PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().map(|h| h.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .filter(|x| !x.is_empty())
            .map(PathBuf::from)
            .or_else(|| home().map(|h| h.join(".config")))
    }?;
    Some(base.join("gm81-clean"))
}

pub use instance_ui::fonts::font_dirs;
