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

/// Directories holding system fonts (searched recursively by callers that list fonts).
pub fn font_dirs() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    if cfg!(target_arch = "wasm32") {
        return v;
    }
    if cfg!(windows) {
        let root =
            std::env::var_os("WINDIR").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        v.push(root.join("Fonts"));
        if let Some(l) = std::env::var_os("LOCALAPPDATA") {
            v.push(
                PathBuf::from(l)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts"),
            );
        }
    } else if cfg!(target_os = "macos") {
        v.push("/System/Library/Fonts".into());
        v.push("/System/Library/Fonts/Supplemental".into());
        v.push("/Library/Fonts".into());
        if let Some(h) = home() {
            v.push(h.join("Library").join("Fonts"));
        }
    } else {
        if let Some(d) = std::env::var_os("XDG_DATA_HOME").filter(|x| !x.is_empty()) {
            v.push(PathBuf::from(d).join("fonts"));
        }
        if let Some(h) = home() {
            v.push(h.join(".local").join("share").join("fonts"));
            v.push(h.join(".fonts"));
        }
        v.push("/usr/share/fonts".into());
        v.push("/usr/local/share/fonts".into());
    }
    v
}

/// Finds the first of `names` (file names, case-insensitive) in the system font
/// directories, searching subdirectories too, and returns its bytes.
pub fn find_font(names: &[&str]) -> Option<Vec<u8>> {
    // Direct hits first (cheap), then a bounded recursive walk.
    for d in font_dirs() {
        for n in names {
            if let Ok(b) = std::fs::read(d.join(n)) {
                return Some(b);
            }
        }
    }
    let lower: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    let mut found: Vec<(usize, PathBuf)> = Vec::new();
    let mut stack = font_dirs();
    let mut budget = 6000usize;
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            budget = budget.saturating_sub(1);
            if budget == 0 {
                break;
            }
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Some(f) = p.file_name().and_then(|f| f.to_str()) {
                let f = f.to_lowercase();
                if let Some(rank) = lower.iter().position(|n| *n == f) {
                    found.push((rank, p));
                }
            }
        }
    }
    found.sort();
    found.into_iter().find_map(|(_, p)| std::fs::read(p).ok())
}
