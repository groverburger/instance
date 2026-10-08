//! Conversion of GameMaker: Studio 1.x projects (`*.project.gmx` folders and `.gmz`
//! archives, zip or 7-Zip) into the GM 8.1 project model of `gm-format`.
//!
//! ```no_run
//! let (project, warnings) = gm_gmx::load_gmx("MyGame.gmx").unwrap();
//! for w in &warnings { eprintln!("{w}"); }
//! let bytes = gm_format::write_project(&project, gm_format::FormatVersion::V810).unwrap();
//! ```
//!
//! See `specs/gmx-format.md` for the format notes and the mapping.

pub mod compat;
pub mod convert;
pub mod gml;
pub mod loader;
pub mod source;
pub mod xml;

pub use convert::Options;
use gm_format::Project;
use source::Source;
use std::path::Path;

/// A non-fatal conversion note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// What it is about (resource, code location, or a category).
    pub context: String,
    pub message: String,
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.context, self.message)
    }
}

/// Load a Studio project with default options. `path` may be a `.gmx` project folder,
/// a `*.project.gmx` file, or a `.gmz` archive (zip or 7z).
pub fn load_gmx(path: impl AsRef<Path>) -> Result<(Project, Vec<Warning>), String> {
    load_gmx_with(path, &Options::default())
}

/// [`load_gmx`] with options.
pub fn load_gmx_with(path: impl AsRef<Path>, opts: &Options) -> Result<(Project, Vec<Warning>), String> {
    let path = path.as_ref();
    let (src, proj) = open(path)?;
    let loaded = loader::load(&src, &proj)?;
    Ok(convert::convert(loaded, opts))
}

/// Open the source and locate the project file inside it.
pub fn open(path: &Path) -> Result<(Source, String), String> {
    if path.is_dir() {
        let src = Source::Dir(path.to_path_buf());
        let proj = src
            .project_files()
            .into_iter()
            .next()
            .ok_or_else(|| format!("{}: no *.project.gmx found", path.display()))?;
        return Ok((src, proj));
    }
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if name.ends_with(".project.gmx") {
        let dir = path.parent().unwrap_or(Path::new("."));
        let dir = if dir.as_os_str().is_empty() { Path::new(".") } else { dir };
        return Ok((Source::Dir(dir.to_path_buf()), path.file_name().unwrap().to_string_lossy().into_owned()));
    }
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let src = Source::from_archive(&data).map_err(|e| format!("{}: {e}", path.display()))?;
    let proj = src
        .project_files()
        .into_iter()
        .next()
        .ok_or_else(|| format!("{}: archive contains no *.project.gmx", path.display()))?;
    Ok((src, proj))
}
