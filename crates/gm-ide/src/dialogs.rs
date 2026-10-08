//! File dialogs that work natively (rfd, synchronous) and in the browser (a hidden
//! `<input type=file>` and Blob downloads, asynchronous).
//!
//! Opening uses a request/take pattern: a button calls [`request_open`] with a key,
//! and the same form calls [`take_opened`] with that key every frame (outside of
//! menus that may close). Natively the dialog runs inside `request_open`, so the
//! result is there in the same frame; on the web it arrives some frames later and a
//! repaint is requested. Saving hands the finished bytes to [`save_bytes`].

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// File type filters: (description, extensions without dot; "*" = all files).
pub type Filters<'a> = &'a [(&'a str, &'a [&'a str])];

pub const IMAGES: Filters<'static> = &[("Images", &["png", "bmp", "gif", "jpg", "jpeg"])];
pub const PNG: Filters<'static> = &[("PNG", &["png"])];
pub const TEXT: Filters<'static> = &[("Text", &["txt"])];
pub const LOG: Filters<'static> = &[("Log files", &["log", "txt"])];
pub const PROJECTS: Filters<'static> = &[
    ("GameMaker projects", &["gm81", "gmk"]),
    ("All files", &["*"]),
];
pub const ALL: Filters<'static> = &[("All files", &["*"])];

/// A file chosen by the user.
#[derive(Debug, Clone)]
pub struct Picked {
    /// File name (no directory).
    pub name: String,
    pub bytes: Vec<u8>,
    /// Full path (native only).
    pub path: Option<PathBuf>,
}

type Outcome = Result<Vec<Picked>, String>;

thread_local! {
    static RESULTS: RefCell<HashMap<u64, Outcome>> = RefCell::new(HashMap::new());
    static CTX: RefCell<Option<eframe::egui::Context>> = const { RefCell::new(None) };
}

/// Remembers the egui context so asynchronous results can request a repaint.
pub fn set_context(ctx: &eframe::egui::Context) {
    CTX.with(|c| *c.borrow_mut() = Some(ctx.clone()));
}

/// The egui context given to [`set_context`].
#[allow(dead_code)]
pub fn context() -> Option<eframe::egui::Context> {
    CTX.with(|c| c.borrow().clone())
}

/// A dialog key from a purpose and the address of the owning form (stable while
/// the form lives).
pub fn key<T: ?Sized>(purpose: &str, owner: *const T) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    purpose.hash(&mut h);
    (owner as *const u8 as usize).hash(&mut h);
    h.finish()
}

#[allow(dead_code)]
fn store(key: u64, r: Outcome) {
    RESULTS.with(|m| m.borrow_mut().insert(key, r));
    CTX.with(|c| {
        if let Some(c) = c.borrow().as_ref() {
            c.request_repaint();
        }
    });
}

/// The files picked for `key`, once available (`Ok(empty)` never happens: a
/// cancelled dialog produces nothing).
pub fn take_opened(key: u64) -> Option<Outcome> {
    RESULTS.with(|m| m.borrow_mut().remove(&key))
}

/// Shows an open-file dialog; the result is fetched with [`take_opened`].
pub fn request_open(key: u64, filters: Filters, multi: bool) {
    imp::request_open(key, filters, multi)
}

/// Saves `bytes` under a user-chosen name (natively a save dialog starting in
/// `dir`; on the web a download named `default_name`). Returns the chosen path
/// (`None` = cancelled; on the web `Some(default_name)`).
pub fn save_bytes(
    default_name: &str,
    dir: Option<&Path>,
    filters: Filters,
    bytes: &[u8],
) -> Result<Option<PathBuf>, String> {
    imp::save_bytes(default_name, dir, filters, bytes)
}

/// [`save_bytes`] of encoded data, with any error as text.
pub fn save_encoded(
    default_name: &str,
    filters: Filters,
    bytes: Result<Vec<u8>, String>,
) -> Result<(), String> {
    save_bytes(default_name, None, filters, &bytes?).map(|_| ())
}

/// Asks for a file path (native only; `None` on the web).
pub fn pick_path() -> Option<PathBuf> {
    imp::pick_path()
}

/// Asks for a path to save to (native only; `None` on the web, where callers
/// use [`save_bytes`] instead).
pub fn save_path(default_name: &str, dir: Option<&Path>, filters: Filters) -> Option<PathBuf> {
    imp::save_path(default_name, dir, filters)
}

/// Whether paths on disk exist (false on the web, where only bytes are available).
pub const HAS_FILESYSTEM: bool = !cfg!(target_arch = "wasm32");

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;

    fn dialog(filters: Filters) -> rfd::FileDialog {
        let mut d = rfd::FileDialog::new();
        for (name, exts) in filters {
            d = d.add_filter(*name, exts);
        }
        d
    }

    fn read(p: PathBuf) -> Result<Picked, String> {
        let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        Ok(Picked {
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            bytes,
            path: Some(p),
        })
    }

    pub fn request_open(key: u64, filters: Filters, multi: bool) {
        let d = dialog(filters);
        let paths = if multi {
            d.pick_files().unwrap_or_default()
        } else {
            d.pick_file().into_iter().collect()
        };
        if paths.is_empty() {
            return;
        }
        let r: Outcome = paths.into_iter().map(read).collect();
        store(key, r);
    }

    pub fn save_path(default_name: &str, dir: Option<&Path>, filters: Filters) -> Option<PathBuf> {
        let mut d = dialog(filters);
        if !default_name.is_empty() {
            d = d.set_file_name(default_name);
        }
        if let Some(dir) = dir {
            d = d.set_directory(dir);
        }
        d.save_file()
    }

    pub fn save_bytes(
        default_name: &str,
        dir: Option<&Path>,
        filters: Filters,
        bytes: &[u8],
    ) -> Result<Option<PathBuf>, String> {
        let Some(p) = save_path(default_name, dir, filters) else {
            return Ok(None);
        };
        std::fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))?;
        Ok(Some(p))
    }

    pub fn pick_path() -> Option<PathBuf> {
        rfd::FileDialog::new().pick_file()
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use crate::web::js;
    use wasm_bindgen::JsCast;

    fn accept(filters: Filters) -> String {
        let mut v: Vec<String> = Vec::new();
        for (_, exts) in filters {
            for e in *exts {
                if *e != "*" {
                    v.push(format!(".{e}"));
                }
            }
        }
        v.join(",")
    }

    pub fn request_open(key: u64, filters: Filters, multi: bool) {
        let promise = js::pick_files(&accept(filters), multi);
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(v) = wasm_bindgen_futures::JsFuture::from(promise).await else {
                return;
            };
            let Ok(arr) = v.dyn_into::<js_sys::Array>() else {
                return; // cancelled
            };
            let mut out = Vec::new();
            for item in arr.iter() {
                let name = js_sys::Reflect::get(&item, &"name".into())
                    .ok()
                    .and_then(|n| n.as_string())
                    .unwrap_or_default();
                let bytes = js_sys::Reflect::get(&item, &"bytes".into())
                    .ok()
                    .map(|b| js_sys::Uint8Array::new(&b).to_vec())
                    .unwrap_or_default();
                out.push(Picked {
                    name,
                    bytes,
                    path: None,
                });
            }
            if !out.is_empty() {
                store(key, Ok(out));
            }
        });
    }

    pub fn save_bytes(
        default_name: &str,
        _dir: Option<&Path>,
        _filters: Filters,
        bytes: &[u8],
    ) -> Result<Option<PathBuf>, String> {
        js::download(default_name, bytes);
        Ok(Some(PathBuf::from(default_name)))
    }

    pub fn pick_path() -> Option<PathBuf> {
        None
    }

    pub fn save_path(_: &str, _: Option<&Path>, _: Filters) -> Option<PathBuf> {
        None
    }
}
