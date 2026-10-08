//! Where the project files come from: a directory on disk, or an in-memory copy of a
//! `.gmz` archive (zip or 7-Zip). Paths inside a GMX project use `\` separators and
//! Windows case-insensitive matching, so lookups normalise both.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

/// A read-only file tree.
pub enum Source {
    /// Files on disk below `root`.
    Dir(PathBuf),
    /// Files extracted from an archive: normalised path -> (original path, bytes).
    Mem(BTreeMap<String, (String, Vec<u8>)>),
}

/// Normalise a relative path for lookup: `/` separators, lower case, no leading `./`.
pub fn norm(p: &str) -> String {
    let p = p.replace('\\', "/");
    let mut out = Vec::new();
    for part in p.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s.to_ascii_lowercase()),
        }
    }
    out.join("/")
}

impl Source {
    /// Open an archive (zip or 7z, detected by magic bytes).
    pub fn from_archive(data: &[u8]) -> Result<Source, String> {
        let mut files = BTreeMap::new();
        if data.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
            let mut r = sevenz_rust::SevenZReader::new(
                Cursor::new(data),
                data.len() as u64,
                sevenz_rust::Password::empty(),
            )
            .map_err(|e| format!("7z: {e}"))?;
            r.for_each_entries(|entry, rd| {
                if !entry.is_directory() {
                    let mut buf = Vec::new();
                    rd.read_to_end(&mut buf)?;
                    let name = entry.name().to_string();
                    files.insert(norm(&name), (name, buf));
                }
                Ok(true)
            })
            .map_err(|e| format!("7z: {e}"))?;
        } else if data.starts_with(b"PK") {
            let mut z = zip::ZipArchive::new(Cursor::new(data)).map_err(|e| format!("zip: {e}"))?;
            for i in 0..z.len() {
                let mut f = z.by_index(i).map_err(|e| format!("zip: {e}"))?;
                if f.is_dir() {
                    continue;
                }
                let name = f.name().to_string();
                let mut buf = Vec::new();
                f.read_to_end(&mut buf).map_err(|e| format!("zip: {e}"))?;
                files.insert(norm(&name), (name, buf));
            }
        } else {
            return Err("not a zip or 7z archive".into());
        }
        Ok(Source::Mem(files))
    }

    /// Read a file by project-relative path (case-insensitive).
    pub fn read(&self, rel: &str) -> Option<Vec<u8>> {
        match self {
            Source::Dir(root) => {
                let rel_fs = rel.replace('\\', "/");
                let p = root.join(&rel_fs);
                if let Ok(b) = std::fs::read(&p) {
                    return Some(b);
                }
                // Case-insensitive fallback, component by component.
                let mut cur = root.clone();
                for part in rel_fs.split('/').filter(|s| !s.is_empty() && *s != ".") {
                    let want = part.to_ascii_lowercase();
                    let found = std::fs::read_dir(&cur).ok()?.flatten().find(|e| {
                        e.file_name().to_string_lossy().to_ascii_lowercase() == want
                    })?;
                    cur = found.path();
                }
                std::fs::read(cur).ok()
            }
            Source::Mem(m) => m.get(&norm(rel)).map(|(_, b)| b.clone()),
        }
    }

    pub fn read_string(&self, rel: &str) -> Option<String> {
        self.read(rel).map(|b| decode_text(&b))
    }

    /// List files below a directory (relative paths, `/`-separated, original case).
    pub fn list(&self, dir: &str) -> Vec<String> {
        match self {
            Source::Dir(root) => {
                let mut out = Vec::new();
                fn walk(base: &Path, d: &Path, out: &mut Vec<String>) {
                    if let Ok(rd) = std::fs::read_dir(d) {
                        for e in rd.flatten() {
                            let p = e.path();
                            if p.is_dir() {
                                walk(base, &p, out);
                            } else if let Ok(r) = p.strip_prefix(base) {
                                out.push(r.to_string_lossy().replace('\\', "/"));
                            }
                        }
                    }
                }
                walk(root, &root.join(dir.replace('\\', "/")), &mut out);
                out.sort();
                out
            }
            Source::Mem(m) => {
                let pre = format!("{}/", norm(dir));
                m.iter()
                    .filter(|(k, _)| k.starts_with(&pre))
                    .map(|(_, (orig, _))| orig.replace('\\', "/"))
                    .collect()
            }
        }
    }

    /// All `*.project.gmx` files (relative paths), shallowest first.
    pub fn project_files(&self) -> Vec<String> {
        let mut v: Vec<String> = match self {
            Source::Dir(_) => self.list(""),
            Source::Mem(m) => m.values().map(|(o, _)| o.replace('\\', "/")).collect(),
        };
        v.retain(|p| p.to_ascii_lowercase().ends_with(".project.gmx"));
        v.sort_by_key(|p| (p.matches('/').count(), p.clone()));
        v
    }
}

/// Decode a text file: UTF-8 (with or without BOM), falling back to Windows-1252.
pub fn decode_text(b: &[u8]) -> String {
    let b = b.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(b);
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => gm_format::text::decode_cp1252(b),
    }
}
