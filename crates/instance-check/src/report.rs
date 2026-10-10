//! Output formats: human-readable text and JSON.

use crate::{Diagnostic, Report, Severity};
use std::collections::HashMap;
use std::fmt::Write;

/// Orders diagnostics by resource (in order of first appearance; project-level
/// findings first), keeping the original order within a resource.
pub fn group(diags: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut order: HashMap<String, usize> = HashMap::new();
    order.insert("project".into(), 0);
    for d in &diags {
        let n = order.len();
        order.entry(d.resource.clone()).or_insert(n);
    }
    let mut v = diags;
    v.sort_by_key(|d| order[&d.resource]);
    v
}

/// `resource / event / action N` for a diagnostic.
pub fn location(d: &Diagnostic) -> String {
    let mut s = d.resource.clone();
    if let Some(e) = &d.event {
        s.push_str(" / ");
        s.push_str(e);
    }
    if let Some(a) = d.action {
        let _ = write!(s, " / action {}", a);
    }
    if let (Some(l), Some(c)) = (d.line, d.col) {
        let _ = write!(s, ", line {} col {}", l, c);
    }
    s
}

/// The summary line: `N errors, M warnings in K code pieces`.
pub fn summary(r: &Report) -> String {
    let e = r.errors();
    let w = r.warnings();
    format!(
        "{} error{}, {} warning{} in {} code piece{}",
        e,
        if e == 1 { "" } else { "s" },
        w,
        if w == 1 { "" } else { "s" },
        r.code_pieces,
        if r.code_pieces == 1 { "" } else { "s" }
    )
}

/// Human-readable report, grouped per resource, ending with the summary line.
/// With `quiet`, only the summary line.
pub fn human(r: &Report, quiet: bool) -> String {
    let mut s = String::new();
    if !quiet {
        let mut last: Option<&str> = None;
        for d in &r.diagnostics {
            if last.is_some() && last != Some(d.resource.as_str()) {
                s.push('\n');
            }
            last = Some(&d.resource);
            let sev = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            let _ = writeln!(s, "{}: {}: {} [{}]", sev, location(d), d.message, d.kind);
            if let Some(src) = &d.source_line {
                let _ = writeln!(s, "    {}", src);
                if let Some(c) = d.col {
                    // the column counts bytes; tabs are kept so the caret lines up
                    let pad: String = src
                        .chars()
                        .take(c.saturating_sub(1) as usize)
                        .map(|ch| if ch == '\t' { '\t' } else { ' ' })
                        .collect();
                    let _ = writeln!(s, "    {}^", pad);
                }
            }
        }
        if !r.diagnostics.is_empty() {
            s.push('\n');
        }
    }
    s.push_str(&summary(r));
    s.push('\n');
    s
}

/// The diagnostics as a JSON array of
/// `{severity, kind, resource, event, action, line, col, message, source_line}`.
pub fn json(r: &Report) -> String {
    serde_json::to_string_pretty(&r.diagnostics).expect("diagnostics serialize")
}
