//! Create Executable (specs/exe-format.md): runner bytes + v810 project payload
//! (+ extension bundle) + footer.

use std::path::{Path, PathBuf};

use gm_format::exe::{self, BundledPackage};
use gm_format::Project;

/// Extension package search path (specs/extensions.md §2.1), first hit wins.
pub fn extension_dirs(project_path: Option<&Path>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        v.push(dir.join("extensions"));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        v.push(PathBuf::from(appdata).join("gm81-clean").join("extensions"));
    }
    if let Some(dir) = project_path.and_then(Path::parent) {
        v.push(dir.join("extensions"));
    }
    v
}

/// Find and read the packages the project uses. Missing packages produce warnings.
pub fn collect_packages(p: &Project, dirs: &[PathBuf]) -> (Vec<BundledPackage>, Vec<String>) {
    let mut found = Vec::new();
    let mut warnings = Vec::new();
    for name in &p.extension_packages {
        let hit = dirs.iter().find_map(|d| {
            let ged = std::fs::read(d.join(format!("{name}.ged"))).ok()?;
            let dat = std::fs::read(d.join(format!("{name}.dat"))).ok()?;
            Some(BundledPackage {
                name: name.clone(),
                ged,
                dat,
            })
        });
        match hit {
            Some(b) => found.push(b),
            None => warnings.push(format!(
                "Extension package \"{name}\" was not found; it is not included in the executable."
            )),
        }
    }
    (found, warnings)
}

/// Bytes of a standalone executable for `p`.
pub fn executable_bytes(
    p: &Project,
    runner: &[u8],
    dirs: &[PathBuf],
) -> Result<(Vec<u8>, Vec<String>), String> {
    let payload = crate::project_ops::project_bytes(p)?;
    let (packages, warnings) = collect_packages(p, dirs);
    let bundle = (!p.extension_packages.is_empty()).then(|| exe::write_bundle(&packages));
    Ok((exe::build(runner, &payload, bundle.as_deref()), warnings))
}

/// Write the executable to `out`; returns warnings (e.g. missing packages).
pub fn create_executable(
    p: &Project,
    runner: &Path,
    out: &Path,
    project_path: Option<&Path>,
) -> Result<Vec<String>, String> {
    if p.rooms.iter().flatten().next().is_none() {
        return Err("A game needs at least one room.".into());
    }
    let runner_bytes = std::fs::read(runner).map_err(|e| format!("{}: {e}", runner.display()))?;
    if exe::read_embedded(&runner_bytes).is_some() {
        return Err(format!(
            "{} already contains a game; choose the plain runner.",
            runner.display()
        ));
    }
    let (bytes, warnings) = executable_bytes(p, &runner_bytes, &extension_dirs(project_path))?;
    std::fs::write(out, bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(warnings)
}
