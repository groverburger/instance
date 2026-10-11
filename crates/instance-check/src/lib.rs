//! Static checker for GameMaker 8.1 projects (`gmk-check`).
//!
//! Finds mistakes in a project without running it: GML compile errors (with the
//! messages of the [`gml`] compiler, which match GM 8.1's), script calls with a wrong
//! argument count, broken resource references, duplicate instance ids, names that
//! clash with each other or with built-ins, and — as warnings — variables that are
//! read but never assigned, no-short-circuit hazards, Lite-edition-only features
//! and LF-only line endings. The rules come from `specs/gml-language.md`,
//! `specs/runtime.md`, `specs/file-formats.md` and `specs/lite-edition.md`.
//!
//! ```no_run
//! let project = instance_check::load_project("game.gm81".as_ref()).unwrap();
//! let report = instance_check::check_project(&project, &instance_check::Options::default());
//! print!("{}", instance_check::report::human(&report, false));
//! ```

mod analysis;
mod names;
mod pieces;
pub mod report;
mod structure;

use instance_format::Project;
use serde::Serialize;
use std::path::Path;

/// Which optional checks to run.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Also report "possibly uninitialised" instance variables (heuristic).
    pub pedantic: bool,
    /// Also report features unavailable in GM 8.1 Lite.
    pub lite: bool,
}

/// How serious a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The game will fail (compile error, wrong reference, ...).
    Error,
    /// Probably a mistake.
    Warning,
}

/// One finding.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Diagnostic {
    /// Error or warning.
    pub severity: Severity,
    /// Short machine-readable category (`compile`, `script-args`, `reference`,
    /// `instance-id`, `name-clash`, `unassigned`, `uninitialised`, `short-circuit`,
    /// `lite`, `line-endings`, ...).
    pub kind: &'static str,
    /// The resource (object, script, room, time line, trigger, constant) or `project`.
    pub resource: String,
    /// Event / moment / part of the resource, if any (`Step`, `moment 30`,
    /// `creation code`, ...).
    pub event: Option<String>,
    /// 1-based action number in the event or moment, if any.
    pub action: Option<usize>,
    /// 1-based line in the code piece.
    pub line: Option<u32>,
    /// 1-based column in the line.
    pub col: Option<u32>,
    /// The message.
    pub message: String,
    /// The text of the offending source line.
    pub source_line: Option<String>,
}

/// The result of checking a project.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Findings, grouped by resource in project order.
    pub diagnostics: Vec<Diagnostic>,
    /// Number of code pieces examined (scripts, code actions, expression arguments,
    /// creation codes, trigger conditions, constants).
    pub code_pieces: usize,
}

impl Report {
    /// Number of errors.
    pub fn errors(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }
    /// Number of warnings.
    pub fn warnings(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }
}

/// Loads a project to check: a project folder (`gmk-unpack`) or a `.gmk` / `.gm81` file (any version `instance_format`
/// reads).
pub fn load_project(path: &Path) -> Result<Project, String> {
    if path.is_dir() {
        return instance_format::folder::read_folder(path).map_err(|e| e.to_string());
    }
    instance_format::read_project_file(path)
        .map(|(p, _)| p)
        .map_err(|e| e.to_string())
}

/// Runs every check on `project`.
pub fn check_project(project: &Project, opts: &Options) -> Report {
    let mut diags = Vec::new();
    structure::check(project, opts, &mut diags);
    let code_pieces = analysis::check(project, opts, &mut diags);
    Report {
        diagnostics: report::group(diags),
        code_pieces,
    }
}

#[cfg(test)]
mod tests;
