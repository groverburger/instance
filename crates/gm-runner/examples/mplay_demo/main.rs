//! Multiplayer demo: two headless runners talking over gm81-clean's protocol.
//!
//! ```text
//! cargo build -p gm-runner
//! cargo run -p gm-runner --example mplay_demo            # writes the project, runs host + client
//! cargo run -p gm-runner --example mplay_demo -- out.gm81 --no-run
//! ```
//! Manually: `gm-runner out.gm81 --headless --frames 5 host` in one terminal and
//! `gm-runner out.gm81 --headless --frames 5` in another (same machine or LAN).
//! `GM81_MPLAY_PORT` changes the port (default 40810).

mod project;

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("mplay_demo.gm81"));
    gm_format::write_project_file(&out, &project::project(), gm_format::FormatVersion::V810).expect("write project");
    println!("wrote {}", out.display());
    if args.iter().any(|a| a == "--no-run") {
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let runner = exe
        .parent()
        .and_then(|d| d.parent())
        .map(|d| d.join(if cfg!(windows) { "gm-runner.exe" } else { "gm-runner" }))
        .filter(|p| p.is_file());
    let Some(runner) = runner else {
        eprintln!("gm-runner binary not found next to the example; run `cargo build -p gm-runner` first");
        std::process::exit(1);
    };
    let spawn = |extra: Option<&str>| {
        let mut c = Command::new(&runner);
        c.arg(&out).args(["--headless", "--frames", "5"]);
        if let Some(e) = extra {
            c.arg(e);
        }
        c.stderr(Stdio::piped()).spawn().expect("start runner")
    };
    let host = spawn(Some("host"));
    std::thread::sleep(std::time::Duration::from_millis(300));
    let client = spawn(None);
    let c = client.wait_with_output().unwrap();
    let h = host.wait_with_output().unwrap();
    println!("---- host ----\n{}", String::from_utf8_lossy(&h.stderr));
    println!("---- client ----\n{}", String::from_utf8_lossy(&c.stderr));
}
