//! Chapter 410: text/binary files, INI files, file system, parameters,
//! environment, a file-backed "registry", included files and program execution.

use super::{bo, r, st, zero, Reg, A};
use crate::game::Game;
use gml::{Error, Value};
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom, Write};

/// An open text file.
pub struct TextFile {
    /// Read mode: content and cursor; write mode: output handle.
    pub data: Vec<u8>,
    pub pos: usize,
    pub out: Option<std::fs::File>,
}

/// Open files and other file-related state.
#[derive(Default)]
pub struct FileState {
    pub text: Vec<Option<TextFile>>,
    pub bin: Vec<Option<std::fs::File>>,
    /// Open INI file: path and sections.
    pub ini: Option<(std::path::PathBuf, Vec<(String, Vec<(String, String)>)>, bool)>,
    pub find: Vec<String>,
    pub find_pos: usize,
    pub registry: Option<BTreeMap<String, String>>,
    pub reg_root: i32,
}

fn alloc<T>(v: &mut Vec<Option<T>>, t: T) -> i32 {
    // GM file ids start at 1
    if let Some(p) = v.iter().skip(1).position(|x| x.is_none()) {
        v[p + 1] = Some(t);
        return (p + 1) as i32;
    }
    if v.is_empty() {
        v.push(None);
    }
    v.push(Some(t));
    (v.len() - 1) as i32
}

fn text_file<'a>(g: &'a mut Game, a: &A) -> Result<&'a mut TextFile, Error> {
    let i = a.i(0)?;
    g.files
        .text
        .get_mut(i.max(0) as usize)
        .and_then(|f| f.as_mut())
        .ok_or_else(|| Error::new(format!("File is not opened: {}", i)))
}

fn bin_file<'a>(g: &'a mut Game, a: &A) -> Result<&'a mut std::fs::File, Error> {
    let i = a.i(0)?;
    g.files
        .bin
        .get_mut(i.max(0) as usize)
        .and_then(|f| f.as_mut())
        .ok_or_else(|| Error::new(format!("File is not opened: {}", i)))
}

fn parse_ini(text: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with(';') {
            continue;
        }
        if l.starts_with('[') && l.ends_with(']') {
            out.push((l[1..l.len() - 1].to_string(), Vec::new()));
        } else if let Some((k, v)) = l.split_once('=') {
            if out.is_empty() {
                out.push((String::new(), Vec::new()));
            }
            out.last_mut().unwrap().1.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    out
}

fn ini_flush(g: &mut Game) {
    if let Some((p, secs, dirty)) = &mut g.files.ini {
        if *dirty {
            let mut s = String::new();
            for (name, keys) in secs.iter() {
                s.push_str(&format!("[{}]\r\n", name));
                for (k, v) in keys {
                    s.push_str(&format!("{}={}\r\n", k, v));
                }
            }
            let _ = std::fs::write(p, s);
            *dirty = false;
        }
    }
}

fn ini_get(g: &Game, sec: &str, key: &str) -> Option<String> {
    let (_, secs, _) = g.files.ini.as_ref()?;
    let s = secs.iter().find(|s| s.0.eq_ignore_ascii_case(sec))?;
    s.1.iter().find(|k| k.0.eq_ignore_ascii_case(key)).map(|k| k.1.clone())
}

fn ini_set(g: &mut Game, sec: &str, key: &str, val: String) -> Result<(), Error> {
    let (_, secs, dirty) = g.files.ini.as_mut().ok_or_else(|| Error::new("No INI file is open."))?;
    let si = match secs.iter().position(|s| s.0.eq_ignore_ascii_case(sec)) {
        Some(i) => i,
        None => {
            secs.push((sec.to_string(), Vec::new()));
            secs.len() - 1
        }
    };
    match secs[si].1.iter_mut().find(|k| k.0.eq_ignore_ascii_case(key)) {
        Some(k) => k.1 = val,
        None => secs[si].1.push((key.to_string(), val)),
    }
    *dirty = true;
    Ok(())
}

fn reg_path(g: &Game) -> std::path::PathBuf {
    let base = std::env::var("APPDATA").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join("gm81-runner").join(format!("registry_{}.txt", g.game_id))
}

fn registry(g: &mut Game) -> &mut BTreeMap<String, String> {
    if g.files.registry.is_none() {
        let mut m = BTreeMap::new();
        if let Ok(t) = std::fs::read_to_string(reg_path(g)) {
            for l in t.lines() {
                if let Some((k, v)) = l.split_once('\t') {
                    m.insert(k.to_string(), v.replace("\\n", "\n"));
                }
            }
        }
        g.files.registry = Some(m);
    }
    g.files.registry.as_mut().unwrap()
}

fn reg_save(g: &mut Game) {
    let p = reg_path(g);
    let s: String = registry(g).iter().map(|(k, v)| format!("{}\t{}\n", k, v.replace('\n', "\\n"))).collect();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(p, s);
}

fn reg_key(g: &Game, key: Option<&str>, name: &str) -> String {
    match key {
        Some(k) => format!("{}|{}\\{}", g.files.reg_root, k, name),
        None => format!("game\\{}", name),
    }
}

fn wildcard(mask: &str, name: &str) -> bool {
    fn m(p: &[char], s: &[char]) -> bool {
        match (p.first(), s.first()) {
            (None, None) => true,
            (Some('*'), _) => m(&p[1..], s) || (!s.is_empty() && m(p, &s[1..])),
            (Some('?'), Some(_)) => m(&p[1..], &s[1..]),
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b) && m(&p[1..], &s[1..]),
            _ => false,
        }
    }
    let p: Vec<char> = mask.chars().collect();
    let s: Vec<char> = name.chars().collect();
    m(&p, &s) || (mask.ends_with("*.*") && !name.contains('.') && m(&p[..p.len() - 2], &s))
}

impl Game {
    /// Exports included files flagged for automatic export (§2 step 3).
    pub fn export_included_files(&mut self) {
        let files = self.included_files.clone();
        for f in files {
            if f.export_kind == 0 {
                continue;
            }
            let dir = match f.export_kind {
                1 => self.temp_directory.clone(),
                2 => self.working_directory.clone(),
                _ => f.export_folder.clone(),
            };
            let p = std::path::Path::new(&dir).join(&f.file_name);
            if p.exists() && !f.overwrite_existing {
                continue;
            }
            if let Some(d) = &f.data {
                let _ = std::fs::write(&p, d);
            }
        }
    }

    fn included(&self, name: &str) -> Option<gm_format::IncludedFile> {
        self.included_files.iter().find(|f| f.file_name.eq_ignore_ascii_case(name)).cloned()
    }
}

/// Windows clipboard (CF_TEXT, i.e. ANSI bytes as GML strings use).
#[cfg(windows)]
mod clip {
    use std::ffi::c_void;
    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(h: *mut c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn GetClipboardData(f: u32) -> *mut c_void;
        fn SetClipboardData(f: u32, h: *mut c_void) -> *mut c_void;
        fn IsClipboardFormatAvailable(f: u32) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(flags: u32, n: usize) -> *mut c_void;
        fn GlobalLock(h: *mut c_void) -> *mut c_void;
        fn GlobalUnlock(h: *mut c_void) -> i32;
    }
    const CF_TEXT: u32 = 1;

    pub fn has_text() -> bool {
        // SAFETY: plain Win32 query.
        unsafe { IsClipboardFormatAvailable(CF_TEXT) != 0 }
    }

    pub fn get() -> Vec<u8> {
        // SAFETY: the clipboard is opened/closed around the access; the locked
        // memory is a NUL-terminated CF_TEXT buffer.
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return Vec::new();
            }
            let mut out = Vec::new();
            let h = GetClipboardData(CF_TEXT);
            if !h.is_null() {
                let p = GlobalLock(h) as *const u8;
                if !p.is_null() {
                    let mut i = 0;
                    while *p.add(i) != 0 {
                        out.push(*p.add(i));
                        i += 1;
                    }
                    GlobalUnlock(h);
                }
            }
            CloseClipboard();
            out
        }
    }

    pub fn set(b: &[u8]) {
        // SAFETY: allocate a movable global block, copy the bytes plus NUL, and hand
        // ownership to the clipboard.
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return;
            }
            EmptyClipboard();
            let h = GlobalAlloc(2, b.len() + 1);
            if !h.is_null() {
                let p = GlobalLock(h) as *mut u8;
                if !p.is_null() {
                    std::ptr::copy_nonoverlapping(b.as_ptr(), p, b.len());
                    *p.add(b.len()) = 0;
                    GlobalUnlock(h);
                    SetClipboardData(CF_TEXT, h);
                }
            }
            CloseClipboard();
        }
    }
}

#[cfg(not(windows))]
mod clip {
    pub fn has_text() -> bool {
        false
    }
    pub fn get() -> Vec<u8> {
        Vec::new()
    }
    pub fn set(_: &[u8]) {}
}

pub fn register(l: &mut Reg) {
    l.push(("clipboard_has_text", |_, _, _, _| bo(clip::has_text())));
    l.push(("clipboard_get_text", |_, _, _, _| Ok(Value::Str(clip::get().into()))));
    l.push(("clipboard_set_text", |_, _, _, a| {
        clip::set(&a.text(0));
        zero()
    }));
    fn open_text(g: &mut Game, a: &A, mode: u8) -> Result<Value, Error> {
        let p = g.resolve_path(&a.s(0)?);
        let tf = match mode {
            0 => match std::fs::read(&p) {
                Ok(d) => TextFile { data: d, pos: 0, out: None },
                Err(_) => return Err(Error::new(format!("Could not open file {}", p.display()))),
            },
            1 => TextFile {
                data: Vec::new(),
                pos: 0,
                out: Some(std::fs::File::create(&p).map_err(|e| Error::new(e.to_string()))?),
            },
            _ => TextFile {
                data: Vec::new(),
                pos: 0,
                out: Some(
                    std::fs::OpenOptions::new().create(true).append(true).open(&p).map_err(|e| Error::new(e.to_string()))?,
                ),
            },
        };
        r(alloc(&mut g.files.text, tf) as f64)
    }
    l.push(("file_text_open_read", |g, _, _, a| open_text(g, a, 0)));
    l.push(("file_text_open_write", |g, _, _, a| open_text(g, a, 1)));
    l.push(("file_text_open_append", |g, _, _, a| open_text(g, a, 2)));
    l.push(("file_text_close", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(f) = g.files.text.get_mut(i.max(0) as usize) {
            *f = None;
        }
        zero()
    }));
    l.push(("file_text_write_string", |g, _, _, a| {
        let s = a.text(1);
        let f = text_file(g, a)?;
        if let Some(o) = f.out.as_mut() {
            o.write_all(&s).map_err(|e| Error::new(e.to_string()))?;
        }
        zero()
    }));
    l.push(("file_text_write_real", |g, _, _, a| {
        let v = a.r(1)?;
        let s = if v.fract() == 0.0 && v.abs() < 1e15 { format!(" {}", v as i64) } else { format!(" {:.6}", v) };
        let f = text_file(g, a)?;
        if let Some(o) = f.out.as_mut() {
            o.write_all(s.as_bytes()).map_err(|e| Error::new(e.to_string()))?;
        }
        zero()
    }));
    l.push(("file_text_writeln", |g, _, _, a| {
        let f = text_file(g, a)?;
        if let Some(o) = f.out.as_mut() {
            o.write_all(b"\r\n").map_err(|e| Error::new(e.to_string()))?;
        }
        zero()
    }));
    l.push(("file_text_read_string", |g, _, _, a| {
        let f = text_file(g, a)?;
        let start = f.pos;
        while f.pos < f.data.len() && f.data[f.pos] != b'\r' && f.data[f.pos] != b'\n' {
            f.pos += 1;
        }
        Ok(Value::Str(f.data[start..f.pos].to_vec().into()))
    }));
    l.push(("file_text_read_real", |g, _, _, a| {
        let f = text_file(g, a)?;
        while f.pos < f.data.len() && (f.data[f.pos] == b' ' || f.data[f.pos] == b'\t') {
            f.pos += 1;
        }
        let start = f.pos;
        while f.pos < f.data.len() && (f.data[f.pos].is_ascii_digit() || b"+-.eE".contains(&f.data[f.pos])) {
            f.pos += 1;
        }
        let s = String::from_utf8_lossy(&f.data[start..f.pos]).into_owned();
        r(s.parse::<f64>().unwrap_or(0.0))
    }));
    l.push(("file_text_readln", |g, _, _, a| {
        let f = text_file(g, a)?;
        while f.pos < f.data.len() && f.data[f.pos] != b'\n' {
            f.pos += 1;
        }
        if f.pos < f.data.len() {
            f.pos += 1;
        }
        zero()
    }));
    l.push(("file_text_eof", |g, _, _, a| {
        let f = text_file(g, a)?;
        bo(f.out.is_some() || f.pos >= f.data.len())
    }));
    l.push(("file_text_eoln", |g, _, _, a| {
        let f = text_file(g, a)?;
        bo(f.pos >= f.data.len() || f.data[f.pos] == b'\r' || f.data[f.pos] == b'\n')
    }));
    l.push(("file_exists", |g, _, _, a| bo(g.resolve_path(&a.s(0)?).is_file())));
    l.push(("file_delete", |g, _, _, a| bo(std::fs::remove_file(g.resolve_path(&a.s(0)?)).is_ok())));
    l.push(("file_rename", |g, _, _, a| {
        bo(std::fs::rename(g.resolve_path(&a.s(0)?), g.resolve_path(&a.s(1)?)).is_ok())
    }));
    l.push(("file_copy", |g, _, _, a| bo(std::fs::copy(g.resolve_path(&a.s(0)?), g.resolve_path(&a.s(1)?)).is_ok())));
    l.push(("directory_exists", |g, _, _, a| bo(g.resolve_path(&a.s(0)?).is_dir())));
    l.push(("directory_create", |g, _, _, a| bo(std::fs::create_dir_all(g.resolve_path(&a.s(0)?)).is_ok())));
    l.push(("file_find_first", |g, _, _, a| {
        let mask = a.s(0)?;
        let attr = a.i(1)?;
        let p = g.resolve_path(&mask);
        let dir = p.parent().map(|d| d.to_path_buf()).unwrap_or_default();
        let pat = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let mut names = Vec::new();
        if let Ok(rd) = std::fs::read_dir(if dir.as_os_str().is_empty() { std::path::Path::new(".") } else { &dir }) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                let is_dir = e.path().is_dir();
                if is_dir && attr & 16 == 0 {
                    continue;
                }
                if wildcard(&pat, &n) {
                    names.push(n);
                }
            }
        }
        names.sort();
        g.files.find = names;
        g.files.find_pos = 0;
        let n = g.files.find.first().cloned().unwrap_or_default();
        g.files.find_pos = 1;
        st(&n)
    }));
    l.push(("file_find_next", |g, _, _, _| {
        let n = g.files.find.get(g.files.find_pos).cloned().unwrap_or_default();
        g.files.find_pos += 1;
        st(&n)
    }));
    l.push(("file_find_close", |g, _, _, _| {
        g.files.find.clear();
        zero()
    }));
    l.push(("file_attributes", |g, _, _, a| {
        let p = g.resolve_path(&a.s(0)?);
        let attr = a.i(1)?;
        let Ok(m) = std::fs::metadata(&p) else { return bo(false) };
        let mut have = 0;
        if m.permissions().readonly() {
            have |= 1;
        }
        if m.is_dir() {
            have |= 16;
        } else {
            have |= 32;
        }
        bo(have & attr == attr)
    }));
    fn split(f: &str) -> (String, String) {
        match f.rfind(['\\', '/']) {
            Some(i) => (f[..=i].to_string(), f[i + 1..].to_string()),
            None => (String::new(), f.to_string()),
        }
    }
    l.push(("filename_name", |_, _, _, a| st(&split(&a.s(0)?).1)));
    l.push(("filename_path", |_, _, _, a| st(&split(&a.s(0)?).0)));
    l.push(("filename_dir", |_, _, _, a| {
        let p = split(&a.s(0)?).0;
        st(p.trim_end_matches(['\\', '/']))
    }));
    l.push(("filename_drive", |_, _, _, a| {
        let s = a.s(0)?;
        st(if s.len() >= 2 && s.as_bytes()[1] == b':' { &s[..2] } else { "" })
    }));
    l.push(("filename_ext", |_, _, _, a| {
        let (_, n) = split(&a.s(0)?);
        st(n.rfind('.').map(|i| &n[i..]).unwrap_or(""))
    }));
    l.push(("filename_change_ext", |_, _, _, a| {
        let s = a.s(0)?;
        let ext = a.s(1)?;
        let (d, n) = split(&s);
        let base = n.rfind('.').map(|i| &n[..i]).unwrap_or(&n).to_string();
        st(&format!("{}{}{}", d, base, ext))
    }));
    l.push(("file_bin_open", |g, _, _, a| {
        let p = g.resolve_path(&a.s(0)?);
        let mode = a.i(1)?;
        let f = match mode {
            0 => std::fs::File::open(&p),
            1 => std::fs::OpenOptions::new().write(true).create(true).truncate(false).open(&p),
            _ => std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&p),
        }
        .map_err(|e| Error::new(format!("Cannot open file {}: {}", p.display(), e)))?;
        r(alloc(&mut g.files.bin, f) as f64)
    }));
    l.push(("file_bin_rewrite", |g, _, _, a| {
        let f = bin_file(g, a)?;
        let _ = f.set_len(0);
        let _ = f.seek(SeekFrom::Start(0));
        zero()
    }));
    l.push(("file_bin_close", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(f) = g.files.bin.get_mut(i.max(0) as usize) {
            *f = None;
        }
        zero()
    }));
    l.push(("file_bin_position", |g, _, _, a| {
        let f = bin_file(g, a)?;
        r(f.stream_position().unwrap_or(0) as f64)
    }));
    l.push(("file_bin_size", |g, _, _, a| {
        let f = bin_file(g, a)?;
        r(f.metadata().map(|m| m.len()).unwrap_or(0) as f64)
    }));
    l.push(("file_bin_seek", |g, _, _, a| {
        let p = a.r(1)?.max(0.0) as u64;
        let f = bin_file(g, a)?;
        let _ = f.seek(SeekFrom::Start(p));
        zero()
    }));
    l.push(("file_bin_write_byte", |g, _, _, a| {
        let b = (a.i(1)? & 0xFF) as u8;
        let f = bin_file(g, a)?;
        f.write_all(&[b]).map_err(|e| Error::new(e.to_string()))?;
        zero()
    }));
    l.push(("file_bin_read_byte", |g, _, _, a| {
        let f = bin_file(g, a)?;
        let mut b = [0u8];
        match f.read(&mut b) {
            Ok(1) => r(b[0] as f64),
            _ => zero(),
        }
    }));
    l.push(("export_include_file", |g, _, _, a| {
        let n = a.s(0)?;
        if let Some(f) = g.included(&n) {
            if let Some(d) = &f.data {
                let p = std::path::Path::new(&g.working_directory).join(&f.file_name);
                let _ = std::fs::write(p, d);
            }
        }
        zero()
    }));
    l.push(("export_include_file_location", |g, _, _, a| {
        let n = a.s(0)?;
        let loc = a.s(1)?;
        if let Some(f) = g.included(&n) {
            if let Some(d) = &f.data {
                let _ = std::fs::write(g.resolve_path(&loc), d);
            }
        }
        zero()
    }));
    l.push(("discard_include_file", |g, _, _, a| {
        let n = a.s(0)?;
        for f in g.included_files.iter_mut() {
            if f.file_name.eq_ignore_ascii_case(&n) {
                f.data = None;
            }
        }
        zero()
    }));
    l.push(("parameter_count", |g, _, _, _| r(g.parameters.len().saturating_sub(1) as f64)));
    l.push(("parameter_string", |g, _, _, a| {
        let n = a.i(0)?;
        st(if n >= 0 { g.parameters.get(n as usize).map(|s| s.as_str()).unwrap_or("") } else { "" })
    }));
    l.push(("environment_get_variable", |_, _, _, a| st(&std::env::var(a.s(0)?).unwrap_or_default())));
    l.push(("disk_free", |_, _, _, _| r(-1.0)));
    l.push(("disk_size", |_, _, _, _| r(-1.0)));
    // registry (file-backed)
    l.push(("registry_write_string", |g, _, _, a| {
        let k = reg_key(g, None, &a.s(0)?);
        let v = a.s(1)?;
        registry(g).insert(k, format!("s{}", v));
        reg_save(g);
        zero()
    }));
    l.push(("registry_write_real", |g, _, _, a| {
        let k = reg_key(g, None, &a.s(0)?);
        let v = a.r(1)?;
        registry(g).insert(k, format!("r{}", v));
        reg_save(g);
        zero()
    }));
    fn reg_read(g: &mut Game, k: String, real: bool) -> Result<Value, Error> {
        let v = registry(g).get(&k).cloned().unwrap_or_default();
        if real {
            r(v.strip_prefix('r').and_then(|x| x.parse().ok()).unwrap_or(0.0))
        } else {
            st(v.strip_prefix('s').unwrap_or(""))
        }
    }
    l.push(("registry_read_string", |g, _, _, a| {
        let k = reg_key(g, None, &a.s(0)?);
        reg_read(g, k, false)
    }));
    l.push(("registry_read_real", |g, _, _, a| {
        let k = reg_key(g, None, &a.s(0)?);
        reg_read(g, k, true)
    }));
    l.push(("registry_exists", |g, _, _, a| {
        let k = reg_key(g, None, &a.s(0)?);
        bo(registry(g).contains_key(&k))
    }));
    l.push(("registry_write_string_ext", |g, _, _, a| {
        let k = reg_key(g, Some(&a.s(0)?), &a.s(1)?);
        let v = a.s(2)?;
        registry(g).insert(k, format!("s{}", v));
        reg_save(g);
        zero()
    }));
    l.push(("registry_write_real_ext", |g, _, _, a| {
        let k = reg_key(g, Some(&a.s(0)?), &a.s(1)?);
        let v = a.r(2)?;
        registry(g).insert(k, format!("r{}", v));
        reg_save(g);
        zero()
    }));
    l.push(("registry_read_string_ext", |g, _, _, a| {
        let k = reg_key(g, Some(&a.s(0)?), &a.s(1)?);
        reg_read(g, k, false)
    }));
    l.push(("registry_read_real_ext", |g, _, _, a| {
        let k = reg_key(g, Some(&a.s(0)?), &a.s(1)?);
        reg_read(g, k, true)
    }));
    l.push(("registry_exists_ext", |g, _, _, a| {
        let k = reg_key(g, Some(&a.s(0)?), &a.s(1)?);
        bo(registry(g).contains_key(&k))
    }));
    l.push(("registry_set_root", |g, _, _, a| {
        g.files.reg_root = a.i(0)?;
        zero()
    }));
    // ini
    l.push(("ini_open", |g, _, _, a| {
        ini_flush(g);
        let p = g.resolve_path(&a.s(0)?);
        let text = std::fs::read(&p).map(|b| gm_format::text::decode_cp1252(&b)).unwrap_or_default();
        g.files.ini = Some((p, parse_ini(&text), false));
        zero()
    }));
    l.push(("ini_close", |g, _, _, _| {
        ini_flush(g);
        g.files.ini = None;
        zero()
    }));
    l.push(("ini_read_string", |g, _, _, a| {
        let v = ini_get(g, &a.s(0)?, &a.s(1)?);
        match v {
            Some(s) => st(&s),
            None => Ok(Value::Str(a.text(2).into())),
        }
    }));
    l.push(("ini_read_real", |g, _, _, a| {
        let v = ini_get(g, &a.s(0)?, &a.s(1)?);
        match v.and_then(|s| s.trim().parse::<f64>().ok()) {
            Some(x) => r(x),
            None => r(a.r(2)?),
        }
    }));
    l.push(("ini_write_string", |g, _, _, a| {
        let v = a.s(2)?;
        ini_set(g, &a.s(0)?, &a.s(1)?, v)?;
        zero()
    }));
    l.push(("ini_write_real", |g, _, _, a| {
        let v = gml::real_to_string(a.r(2)?);
        ini_set(g, &a.s(0)?, &a.s(1)?, v)?;
        zero()
    }));
    l.push(("ini_key_exists", |g, _, _, a| bo(ini_get(g, &a.s(0)?, &a.s(1)?).is_some())));
    l.push(("ini_section_exists", |g, _, _, a| {
        let s = a.s(0)?;
        bo(g.files.ini.as_ref().map(|i| i.1.iter().any(|x| x.0.eq_ignore_ascii_case(&s))).unwrap_or(false))
    }));
    l.push(("ini_key_delete", |g, _, _, a| {
        let (s, k) = (a.s(0)?, a.s(1)?);
        if let Some((_, secs, dirty)) = g.files.ini.as_mut() {
            for sec in secs.iter_mut().filter(|x| x.0.eq_ignore_ascii_case(&s)) {
                sec.1.retain(|x| !x.0.eq_ignore_ascii_case(&k));
            }
            *dirty = true;
        }
        zero()
    }));
    l.push(("ini_section_delete", |g, _, _, a| {
        let s = a.s(0)?;
        if let Some((_, secs, dirty)) = g.files.ini.as_mut() {
            secs.retain(|x| !x.0.eq_ignore_ascii_case(&s));
            *dirty = true;
        }
        zero()
    }));
    l.push(("execute_program", |g, _, _, a| {
        let prog = a.s(0)?;
        let args = a.s(1)?;
        let wait = a.b(2)?;
        let mut cmd = std::process::Command::new(&prog);
        cmd.args(args.split_whitespace());
        if !g.working_directory.is_empty() {
            cmd.current_dir(&g.working_directory);
        }
        if wait {
            let _ = cmd.status();
        } else {
            let _ = cmd.spawn();
        }
        zero()
    }));
    l.push(("execute_shell", |_, _, _, a| {
        let prog = a.s(0)?;
        let args = a.s(1)?;
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", &prog]).args(args.split_whitespace()).spawn();
        zero()
    }));
}

#[cfg(test)]
mod tests {
    #[test]
    fn wildcards() {
        assert!(super::wildcard("*.txt", "a.TXT"));
        assert!(super::wildcard("a?c", "abc"));
        assert!(!super::wildcard("*.txt", "a.png"));
    }
}
