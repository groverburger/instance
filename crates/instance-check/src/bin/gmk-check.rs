//! `gmk-check PROJECT [--strict] [--pedantic] [--lite] [--json] [--quiet]` — static
//! checker for a GameMaker 8.1 project (.gm81 / .gmk): finds GML compile errors
//! (GM's messages, with line and column), script calls with a wrong argument count
//! (when the "argument errors" setting is on), broken resource references, duplicate
//! instance ids, clashing names, variables read but never assigned, no-short-circuit
//! hazards and LF-only code, without running the game.
//!
//! Flags:
//! * `--strict`   warnings also make the exit code 1
//! * `--pedantic` also report possibly uninitialised instance variables (heuristic)
//! * `--lite`     also report features unavailable in GM 8.1 Lite
//! * `--json`     print a JSON array of
//!   `{severity, kind, resource, event, action, line, col, message, source_line}`
//! * `--quiet`    print only the summary line
//!
//! Exit codes: 0 no errors (warnings allowed unless `--strict`), 1 errors,
//! 2 the project could not be loaded.

use instance_check::{check_project, load_project, report, Options};
use std::path::Path;

const USAGE: &str =
    "usage: gmk-check PROJECT.gm81|PROJECT.gmk [--strict] [--pedantic] [--lite] [--json] [--quiet]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut opts = Options::default();
    let (mut strict, mut json, mut quiet) = (false, false, false);
    let mut path = None;
    for a in &args {
        match a.as_str() {
            "--strict" => strict = true,
            "--pedantic" => opts.pedantic = true,
            "--lite" => opts.lite = true,
            "--json" => json = true,
            "--quiet" | "-q" => quiet = true,
            "--help" | "-h" => {
                println!("{USAGE}");
                return;
            }
            s if s.starts_with('-') => {
                eprintln!("gmk-check: unknown option {s}\n{USAGE}");
                std::process::exit(2);
            }
            s => {
                if path.replace(s).is_some() {
                    eprintln!("gmk-check: only one project can be checked at a time\n{USAGE}");
                    std::process::exit(2);
                }
            }
        }
    }
    let Some(path) = path else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    let project = match load_project(Path::new(path)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("gmk-check: {path}: {e}");
            std::process::exit(2);
        }
    };
    let r = check_project(&project, &opts);
    if quiet {
        println!("{}", report::summary(&r));
    } else if json {
        println!("{}", report::json(&r));
        eprintln!("{}", report::summary(&r));
    } else {
        print!("{}", report::human(&r, false));
    }
    let fail = r.errors() > 0 || (strict && r.warnings() > 0);
    std::process::exit(if fail { 1 } else { 0 });
}
