//! Corpus regression harness (ignored by default; run with
//! `cargo test --release -p instance-runner --test corpus -- --ignored --nocapture`).
//!
//! Runs every `.gmk`/`.gm81` found (recursively) in the directories listed in
//! `GM81_CORPUS` (separated by `;`) headless for `GM81_CORPUS_FRAMES` steps
//! (default 300) with pseudo-random keyboard/mouse input, each in its own
//! `instance-runner` process with a time limit (`GM81_CORPUS_SECONDS`, default 120),
//! and fails if any game panics, hits a fatal error, aborts or times out.
//! Projects that the runner refuses cleanly (e.g. no rooms) are reported but
//! not counted as failures. `GM81_CORPUS_SKIP` (comma-separated file-name
//! substrings) leaves out known-bad projects. The projects themselves are never
//! part of the repo.

use std::path::{Path, PathBuf};
use std::process::Command;

fn corpus_dirs() -> Vec<PathBuf> {
    let v = std::env::var("GM81_CORPUS").unwrap_or_else(|_| {
        r"C:\Users\Grob\Downloads\old_gamemaker_projects;C:\Users\Grob\Documents\gamemaker\GameMaker\Examples".into()
    });
    v.split(';').filter(|s| !s.trim().is_empty()).map(PathBuf::from).filter(|p| p.is_dir()).collect()
}

fn find(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            find(&p, out);
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            let ext = ext.to_ascii_lowercase();
            if ext == "gmk" || ext == "gm81" {
                out.push(p);
            }
        }
    }
}

fn env_num(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[test]
#[ignore]
fn corpus_runs_without_fatal_errors() {
    let mut games = Vec::new();
    for d in corpus_dirs() {
        find(&d, &mut games);
    }
    if games.is_empty() {
        eprintln!("no corpus projects found (set GM81_CORPUS)");
        return;
    }
    let frames = env_num("GM81_CORPUS_FRAMES", 300);
    let seconds = env_num("GM81_CORPUS_SECONDS", 120);
    // known-bad projects to leave out: comma-separated file-name substrings
    let skip: Vec<String> = std::env::var("GM81_CORPUS_SKIP")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    games.retain(|g| {
        let n = g.file_name().unwrap().to_string_lossy().to_ascii_lowercase();
        !skip.iter().any(|s| n.contains(s.as_str()))
    });
    let exe = env!("CARGO_BIN_EXE_instance-runner");
    let mut failures = Vec::new();
    for (k, g) in games.iter().enumerate() {
        let t0 = std::time::Instant::now();
        let out = Command::new(exe)
            .arg(g)
            .args(["--headless", "--frames", &frames.to_string()])
            .args(["--max-seconds", &seconds.to_string()])
            .args(["--random-input", &(k as u64 + 1).to_string()])
            .output()
            .expect("run instance-runner");
        let err = String::from_utf8_lossy(&out.stderr);
        let code = out.status.code().unwrap_or(-1);
        let summary = err.lines().find(|l| l.starts_with("ran ")).unwrap_or("").to_string();
        let errors = err.lines().filter(|l| l.starts_with("ERROR")).count();
        let refused = code == 1 && err.contains("has no rooms");
        let bad = if refused {
            None
        } else if err.contains("panicked") {
            Some("panic")
        } else if code == 4 || err.contains("TIMEOUT") {
            Some("timeout")
        } else if err.contains("FATAL") {
            Some("fatal error")
        } else if code != 0 {
            Some("non-zero exit")
        } else {
            None
        };
        eprintln!(
            "{:<45} {:>6.1}s exit {:>2} {:>3} errors {} {}",
            g.file_name().unwrap().to_string_lossy(),
            t0.elapsed().as_secs_f64(),
            code,
            errors,
            bad.map(|b| format!("FAIL ({})", b)).unwrap_or_else(|| if refused { "refused".into() } else { "ok".into() }),
            summary
        );
        if let Some(b) = bad {
            let tail: Vec<&str> = err.lines().filter(|l| !l.starts_with("[")).take(12).collect();
            failures.push(format!("{}: {} (exit {})\n    {}", g.display(), b, code, tail.join("\n    ")));
        }
    }
    assert!(failures.is_empty(), "{} of {} corpus games failed:\n{}", failures.len(), games.len(), failures.join("\n"));
}
