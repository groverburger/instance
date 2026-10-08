//! File access used by the game's file functions (`file_*`, `ini_*`, saves,
//! included files, ...).
//!
//! On desktop targets every call forwards to `std::fs`. In the browser
//! (`wasm32`) there is no file system: files live in an in-memory tree that is
//! persisted to `localStorage` (one key per file, namespaced by game id), so
//! saved games, INI files and highscores survive a page reload. Paths there are
//! case-insensitive and `\` and `/` are equivalent, like on Windows.

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// How a [`File`] is opened.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Existing file, read only.
    Read,
    /// Create or truncate, write only.
    Create,
    /// Create if missing, append.
    Append,
    /// Create if missing, write without truncating.
    Write,
    /// Create if missing, read and write without truncating.
    ReadWrite,
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;

    pub struct File(std::fs::File);

    impl File {
        pub fn open(p: &Path, mode: Mode) -> io::Result<File> {
            let mut o = std::fs::OpenOptions::new();
            match mode {
                Mode::Read => o.read(true),
                Mode::Create => o.write(true).create(true).truncate(true),
                Mode::Append => o.create(true).append(true),
                Mode::Write => o.write(true).create(true).truncate(false),
                Mode::ReadWrite => o.read(true).write(true).create(true).truncate(false),
            };
            o.open(p).map(File)
        }
        pub fn len(&self) -> u64 {
            self.0.metadata().map(|m| m.len()).unwrap_or(0)
        }
        pub fn set_len(&mut self, n: u64) -> io::Result<()> {
            self.0.set_len(n)
        }
    }
    impl Read for File {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            self.0.read(b)
        }
    }
    impl Write for File {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.0.write(b)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
    impl Seek for File {
        fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
            self.0.seek(p)
        }
    }

    pub fn read(p: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(p)
    }
    pub fn write(p: &Path, d: &[u8]) -> io::Result<()> {
        std::fs::write(p, d)
    }
    pub fn remove_file(p: &Path) -> io::Result<()> {
        std::fs::remove_file(p)
    }
    pub fn rename(a: &Path, b: &Path) -> io::Result<()> {
        std::fs::rename(a, b)
    }
    pub fn copy(a: &Path, b: &Path) -> io::Result<()> {
        std::fs::copy(a, b).map(|_| ())
    }
    pub fn create_dir_all(p: &Path) -> io::Result<()> {
        std::fs::create_dir_all(p)
    }
    pub fn remove_dir_all(p: &Path) -> io::Result<()> {
        std::fs::remove_dir_all(p)
    }
    pub fn is_file(p: &Path) -> bool {
        p.is_file()
    }
    pub fn is_dir(p: &Path) -> bool {
        p.is_dir()
    }
    pub fn exists(p: &Path) -> bool {
        p.exists()
    }
    pub fn len(p: &Path) -> Option<u64> {
        std::fs::metadata(p).ok().map(|m| m.len())
    }
    pub fn read_only(p: &Path) -> bool {
        std::fs::metadata(p).map(|m| m.permissions().readonly()).unwrap_or(false)
    }
    pub fn read_dir(p: &Path) -> io::Result<Vec<(String, bool)>> {
        let p = if p.as_os_str().is_empty() { Path::new(".") } else { p };
        let mut out = Vec::new();
        for e in std::fs::read_dir(p)?.flatten() {
            let dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            out.push((e.file_name().to_string_lossy().into_owned(), dir));
        }
        Ok(out)
    }
    pub fn temp_dir() -> PathBuf {
        std::env::temp_dir()
    }
    pub fn set_namespace(_: &str) {}
    pub fn persist() {}
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use std::cell::RefCell;
    use std::collections::{BTreeMap, BTreeSet};

    /// Files larger than this stay in memory only (localStorage is small).
    const PERSIST_MAX: usize = 1 << 20;

    #[derive(Default)]
    struct Store {
        ns: String,
        /// key → (display path, contents)
        files: BTreeMap<String, (String, Vec<u8>)>,
        dirs: BTreeSet<String>,
        dirty: BTreeSet<String>,
    }

    thread_local! {
        static STORE: RefCell<Store> = RefCell::new(Store::default());
    }

    /// Normalised, lower-case key of a path ("/game/data/a.txt").
    fn key(p: &Path) -> String {
        let s = p.to_string_lossy().replace('\\', "/");
        let mut parts: Vec<&str> = Vec::new();
        for c in s.split('/') {
            match c {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                c => parts.push(c),
            }
        }
        format!("/{}", parts.join("/")).to_lowercase()
    }

    fn display(p: &Path) -> String {
        p.to_string_lossy().replace('\\', "/")
    }

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    fn not_found() -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, "file not found")
    }

    pub fn set_namespace(ns: &str) {
        STORE.with(|s| {
            let mut s = s.borrow_mut();
            *s = Store { ns: format!("gm81vfs/{}/", ns), ..Default::default() };
            let Some(ls) = storage() else { return };
            let n = ls.length().unwrap_or(0);
            for i in 0..n {
                let Ok(Some(k)) = ls.key(i) else { continue };
                let Some(rest) = k.strip_prefix(s.ns.as_str()) else { continue };
                let Ok(Some(v)) = ls.get_item(&k) else { continue };
                if let Some(d) = rest.strip_prefix("d:") {
                    s.dirs.insert(d.to_string());
                } else if let Some(f) = rest.strip_prefix("f:") {
                    let (name, b64) = v.split_once('\n').unwrap_or((f, v.as_str()));
                    if let Some(data) = b64_decode(b64) {
                        s.files.insert(f.to_string(), (name.to_string(), data));
                    }
                }
            }
        });
    }

    /// Writes changed files to localStorage.
    pub fn persist() {
        STORE.with(|s| {
            let mut s = s.borrow_mut();
            if s.dirty.is_empty() {
                return;
            }
            let Some(ls) = storage() else {
                s.dirty.clear();
                return;
            };
            let dirty = std::mem::take(&mut s.dirty);
            for k in dirty {
                if let Some(d) = k.strip_prefix("d:") {
                    let item = format!("{}d:{}", s.ns, d);
                    if s.dirs.contains(d) {
                        let _ = ls.set_item(&item, "");
                    } else {
                        let _ = ls.remove_item(&item);
                    }
                    continue;
                }
                let item = format!("{}f:{}", s.ns, k);
                match s.files.get(&k) {
                    Some((name, data)) if data.len() <= PERSIST_MAX => {
                        if ls.set_item(&item, &format!("{}\n{}", name, b64_encode(data))).is_err() {
                            web_sys::console::warn_1(&format!("instance-runner: localStorage is full; {} is not saved", name).into());
                        }
                    }
                    _ => {
                        let _ = ls.remove_item(&item);
                    }
                }
            }
        });
    }

    fn put(p: &Path, d: Vec<u8>) {
        let k = key(p);
        STORE.with(|s| {
            let mut s = s.borrow_mut();
            s.files.insert(k.clone(), (display(p), d));
            s.dirty.insert(k);
        });
    }

    pub fn read(p: &Path) -> io::Result<Vec<u8>> {
        let k = key(p);
        STORE.with(|s| s.borrow().files.get(&k).map(|f| f.1.clone())).ok_or_else(not_found)
    }
    pub fn write(p: &Path, d: &[u8]) -> io::Result<()> {
        put(p, d.to_vec());
        Ok(())
    }
    pub fn remove_file(p: &Path) -> io::Result<()> {
        let k = key(p);
        STORE.with(|s| {
            let mut s = s.borrow_mut();
            if s.files.remove(&k).is_some() {
                s.dirty.insert(k);
                Ok(())
            } else {
                Err(not_found())
            }
        })
    }
    pub fn rename(a: &Path, b: &Path) -> io::Result<()> {
        let d = read(a)?;
        remove_file(a)?;
        write(b, &d)
    }
    pub fn copy(a: &Path, b: &Path) -> io::Result<()> {
        let d = read(a)?;
        write(b, &d)
    }
    pub fn create_dir_all(p: &Path) -> io::Result<()> {
        let k = key(p);
        STORE.with(|s| {
            let mut s = s.borrow_mut();
            s.dirs.insert(k.clone());
            s.dirty.insert(format!("d:{}", k));
        });
        Ok(())
    }
    pub fn remove_dir_all(p: &Path) -> io::Result<()> {
        let k = key(p);
        let pre = format!("{}/", k.trim_end_matches('/'));
        STORE.with(|s| {
            let mut s = s.borrow_mut();
            let gone: Vec<String> = s.files.keys().filter(|f| f.starts_with(&pre)).cloned().collect();
            for f in gone {
                s.files.remove(&f);
                s.dirty.insert(f);
            }
            let dirs: Vec<String> = s.dirs.iter().filter(|d| **d == k || d.starts_with(&pre)).cloned().collect();
            for d in dirs {
                s.dirs.remove(&d);
                s.dirty.insert(format!("d:{}", d));
            }
        });
        Ok(())
    }
    pub fn is_file(p: &Path) -> bool {
        let k = key(p);
        STORE.with(|s| s.borrow().files.contains_key(&k))
    }
    pub fn is_dir(p: &Path) -> bool {
        let k = key(p);
        if k == "/" {
            return true;
        }
        let pre = format!("{}/", k);
        STORE.with(|s| {
            let s = s.borrow();
            s.dirs.contains(&k) || s.files.keys().any(|f| f.starts_with(&pre)) || s.dirs.iter().any(|d| d.starts_with(&pre))
        })
    }
    pub fn exists(p: &Path) -> bool {
        is_file(p) || is_dir(p)
    }
    pub fn len(p: &Path) -> Option<u64> {
        let k = key(p);
        STORE.with(|s| s.borrow().files.get(&k).map(|f| f.1.len() as u64))
    }
    pub fn read_only(_: &Path) -> bool {
        false
    }
    pub fn read_dir(p: &Path) -> io::Result<Vec<(String, bool)>> {
        let k = key(p);
        let pre = if k == "/" { k.clone() } else { format!("{}/", k) };
        let mut out: BTreeMap<String, (String, bool)> = BTreeMap::new();
        STORE.with(|s| {
            let s = s.borrow();
            for (f, (name, _)) in &s.files {
                if let Some(rest) = f.strip_prefix(&pre) {
                    let dn: Vec<&str> = name.rsplit('/').collect();
                    match rest.split_once('/') {
                        None => {
                            out.insert(rest.to_string(), (dn.first().unwrap_or(&rest).to_string(), false));
                        }
                        Some((d, _)) => {
                            out.entry(d.to_string()).or_insert((d.to_string(), true));
                        }
                    }
                }
            }
            for d in &s.dirs {
                if let Some(rest) = d.strip_prefix(&pre) {
                    let first = rest.split('/').next().unwrap_or(rest);
                    if !first.is_empty() {
                        out.entry(first.to_string()).or_insert((first.to_string(), true));
                    }
                }
            }
        });
        Ok(out.into_values().collect())
    }
    pub fn temp_dir() -> PathBuf {
        PathBuf::from("/tmp")
    }

    /// An open file: a private copy, written back on flush and drop.
    pub struct File {
        path: PathBuf,
        data: Vec<u8>,
        pos: usize,
        write: bool,
        append: bool,
        dirty: bool,
    }

    impl File {
        pub fn open(p: &Path, mode: Mode) -> io::Result<File> {
            let existing = read(p);
            let data = match mode {
                Mode::Read => existing?,
                Mode::Create => Vec::new(),
                _ => existing.unwrap_or_default(),
            };
            let pos = if mode == Mode::Append { data.len() } else { 0 };
            let f = File {
                path: p.to_path_buf(),
                data,
                pos,
                write: mode != Mode::Read,
                append: mode == Mode::Append,
                dirty: mode == Mode::Create || (mode != Mode::Read && !is_file(p)),
            };
            if f.dirty {
                put(&f.path, f.data.clone());
            }
            Ok(f)
        }
        pub fn len(&self) -> u64 {
            self.data.len() as u64
        }
        pub fn set_len(&mut self, n: u64) -> io::Result<()> {
            self.data.resize(n as usize, 0);
            self.dirty = true;
            Ok(())
        }
    }
    impl Read for File {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            let n = b.len().min(self.data.len().saturating_sub(self.pos));
            b[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
            self.pos += n;
            Ok(n)
        }
    }
    impl Write for File {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if !self.write {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "file is open for reading"));
            }
            if self.append {
                self.pos = self.data.len();
            }
            let end = self.pos + b.len();
            if end > self.data.len() {
                self.data.resize(end, 0);
            }
            self.data[self.pos..end].copy_from_slice(b);
            self.pos = end;
            self.dirty = true;
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.dirty {
                put(&self.path, self.data.clone());
                self.dirty = false;
            }
            Ok(())
        }
    }
    impl Seek for File {
        fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
            let n = match p {
                SeekFrom::Start(n) => n as i64,
                SeekFrom::End(d) => self.data.len() as i64 + d,
                SeekFrom::Current(d) => self.pos as i64 + d,
            };
            if n < 0 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek before start"));
            }
            self.pos = n as usize;
            Ok(n as u64)
        }
    }
    impl Drop for File {
        fn drop(&mut self) {
            let _ = self.flush();
        }
    }

    const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn b64_encode(d: &[u8]) -> String {
        let mut s = String::with_capacity(d.len().div_ceil(3) * 4);
        for c in d.chunks(3) {
            let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
            for i in 0..4 {
                if i <= c.len() {
                    s.push(B64[(n >> (18 - 6 * i)) as usize & 63] as char);
                } else {
                    s.push('=');
                }
            }
        }
        s
    }

    fn b64_decode(s: &str) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(s.len() / 4 * 3);
        let (mut acc, mut bits) = (0u32, 0);
        for c in s.bytes() {
            if c == b'=' {
                break;
            }
            let v = B64.iter().position(|&b| b == c)? as u32;
            acc = acc << 6 | v;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
            }
        }
        Some(out)
    }
}

pub use imp::*;

/// Reads a whole file as (lossy) UTF-8.
pub fn read_to_string(p: &Path) -> io::Result<String> {
    read(p).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Folder for the runner's per-user data (emulated registry, highscores): the shared
/// per-user folder of `instance_brand::appdirs` (`%APPDATA%\Instance`,
/// `~/Library/Application Support/Instance`, `$XDG_CONFIG_HOME/Instance` or
/// `~/.config/Instance`; the `gm81-runner` folder of earlier builds is migrated on first
/// use); `/config/Instance` in the browser (migrated from `/config/gm81-runner`).
pub fn config_dir() -> PathBuf {
    if cfg!(target_arch = "wasm32") {
        let dir = PathBuf::from("/config").join(instance_brand::appdirs::DATA_DIR_NAME);
        migrate_web_config(&PathBuf::from("/config/gm81-runner"), &dir);
        return dir;
    }
    instance_brand::appdirs::data_dir()
        .unwrap_or_else(|| temp_dir().join(instance_brand::appdirs::DATA_DIR_NAME))
}

/// Browser: copies the files of the old per-game config folder once (the old folder is
/// kept; files already in the new folder win).
fn migrate_web_config(old: &Path, new: &Path) {
    let marker = new.join(".migrated-from-gm81-runner");
    if !is_dir(old) || exists(&marker) {
        return;
    }
    let _ = create_dir_all(new);
    if let Ok(entries) = read_dir(old) {
        for (name, is_dir) in entries {
            let dst = new.join(&name);
            if !is_dir && !exists(&dst) {
                let _ = copy(&old.join(&name), &dst);
            }
        }
    }
    let _ = write(&marker, b"");
}
