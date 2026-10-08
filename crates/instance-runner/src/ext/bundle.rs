//! Finding extension packages: the bundle appended to a standalone executable
//! (exe-format.md, footer v2) and the package search path (extensions.md §2.1).

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// One package's installed files (`.ged` and `.dat` bytes).
#[derive(Clone, Debug)]
pub struct PackageFiles {
    pub name: String,
    pub ged: Vec<u8>,
    pub dat: Vec<u8>,
    /// Where it came from (diagnostics).
    pub origin: String,
}

/// Reads the extension bundle of a standalone executable (footer v2; the magic of
/// earlier builds is accepted too).
/// Returns `None` when the file has no v2 footer.
pub fn read_exe_bundle(exe: &Path) -> Option<Vec<PackageFiles>> {
    let mut f = std::fs::File::open(exe).ok()?;
    let len = f.seek(SeekFrom::End(0)).ok()?;
    if len < 40 {
        return None;
    }
    f.seek(SeekFrom::Start(len - 40)).ok()?;
    let mut foot = [0u8; 40];
    f.read_exact(&mut foot).ok()?;
    if !instance_format::exe::is_magic_v2(&foot[..8]) {
        return None;
    }
    let off = u64::from_le_bytes(foot[24..32].try_into().ok()?);
    let blen = u64::from_le_bytes(foot[32..40].try_into().ok()?);
    if off.checked_add(blen)? > len - 40 {
        return None;
    }
    f.seek(SeekFrom::Start(off)).ok()?;
    let mut b = vec![0u8; blen as usize];
    f.read_exact(&mut b).ok()?;
    parse_bundle(&b)
}

/// Parses a bundle: `u32 count`, then per package `name`, `.ged`, `.dat` (each
/// `u32 length` + bytes).
pub fn parse_bundle(b: &[u8]) -> Option<Vec<PackageFiles>> {
    let mut p = 0usize;
    let u32_at = |p: &mut usize| -> Option<u32> {
        let s = b.get(*p..*p + 4)?;
        *p += 4;
        Some(u32::from_le_bytes(s.try_into().ok()?))
    };
    let count = u32_at(&mut p)?;
    let mut out = Vec::new();
    for _ in 0..count {
        let mut parts = Vec::with_capacity(3);
        for _ in 0..3 {
            let n = u32_at(&mut p)? as usize;
            parts.push(b.get(p..p.checked_add(n)?)?.to_vec());
            p += n;
        }
        let dat = parts.pop()?;
        let ged = parts.pop()?;
        let name = String::from_utf8_lossy(&parts.pop()?).to_string();
        out.push(PackageFiles {
            name,
            ged,
            dat,
            origin: "executable bundle".into(),
        });
    }
    Some(out)
}

/// Builds a bundle (inverse of [`parse_bundle`]); used by tests and exporters.
pub fn write_bundle(pkgs: &[PackageFiles]) -> Vec<u8> {
    let mut b = (pkgs.len() as u32).to_le_bytes().to_vec();
    for p in pkgs {
        for part in [p.name.as_bytes(), &p.ged, &p.dat] {
            b.extend_from_slice(&(part.len() as u32).to_le_bytes());
            b.extend_from_slice(part);
        }
    }
    b
}

static PROJECT_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
static PRELOADED: Mutex<Vec<PackageFiles>> = Mutex::new(Vec::new());

/// Records the directory of the project being loaded (searched for `extensions/`).
pub fn set_project_dir(dir: Option<PathBuf>) {
    *PROJECT_DIR.lock().unwrap() = dir;
}

/// Supplies packages ahead of the search path (e.g. a bundle read by the frontend).
pub fn preload(pkgs: Vec<PackageFiles>) {
    PRELOADED.lock().unwrap().extend(pkgs);
}

/// Dev fallback: the GameMaker installation's extensions folder on the development
/// machine (only used if it exists).
const DEV_FALLBACK: &str = r"C:\Users\Grob\Documents\gamemaker\GameMaker\extensions";

/// Directories searched for `<name>.ged` / `<name>.dat`, in order.
pub fn search_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            v.push(d.join("extensions"));
        }
    }
    if let Some(e) = instance_brand::appdirs::env_var_os("EXTENSIONS") {
        v.extend(std::env::split_paths(&e));
    }
    if let Some(d) = instance_brand::appdirs::data_dir() {
        v.push(d.join("extensions"));
    }
    if let Some(d) = PROJECT_DIR.lock().unwrap().clone() {
        v.push(d.join("extensions"));
    }
    v.push(PathBuf::from(DEV_FALLBACK));
    v
}

/// Finds a package by name: preloaded/bundled packages first, then the search path.
pub fn find_package(name: &str) -> Option<PackageFiles> {
    if let Some(p) = PRELOADED.lock().unwrap().iter().find(|p| p.name.eq_ignore_ascii_case(name)) {
        return Some(p.clone());
    }
    static EXE_BUNDLE: std::sync::OnceLock<Vec<PackageFiles>> = std::sync::OnceLock::new();
    let bundle = EXE_BUNDLE.get_or_init(|| std::env::current_exe().ok().and_then(|e| read_exe_bundle(&e)).unwrap_or_default());
    if let Some(p) = bundle.iter().find(|p| p.name.eq_ignore_ascii_case(name)) {
        return Some(p.clone());
    }
    for d in search_dirs() {
        let ged = d.join(format!("{name}.ged"));
        let dat = d.join(format!("{name}.dat"));
        if let (Ok(g), Ok(t)) = (std::fs::read(&ged), std::fs::read(&dat)) {
            return Some(PackageFiles {
                name: name.to_string(),
                ged: g,
                dat: t,
                origin: d.display().to_string(),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_roundtrip_and_footer() {
        let pk = vec![PackageFiles {
            name: "A b".into(),
            ged: vec![1, 2],
            dat: vec![3],
            origin: String::new(),
        }];
        let b = write_bundle(&pk);
        let back = parse_bundle(&b).unwrap();
        assert_eq!(back[0].name, "A b");
        assert_eq!(back[0].dat, vec![3]);
        // a fake executable: runner bytes, payload, bundle, v2 footer
        let mut exe = b"MZfake-runner".to_vec();
        let po = exe.len() as u64;
        exe.extend_from_slice(b"payload");
        let bo = exe.len() as u64;
        exe.extend_from_slice(&b);
        let dir = std::env::temp_dir().join(format!("instance_bundle_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // the current footer magic and the one of earlier builds
        for magic in [instance_format::exe::MAGIC_V2, instance_format::exe::LEGACY_MAGIC_V2] {
            let mut exe = exe.clone();
            exe.extend_from_slice(&magic);
            for v in [po, 7, bo, b.len() as u64] {
                exe.extend_from_slice(&v.to_le_bytes());
            }
            let p = dir.join("game.exe");
            std::fs::write(&p, &exe).unwrap();
            let got = read_exe_bundle(&p).unwrap();
            assert_eq!(got[0].ged, vec![1, 2]);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
