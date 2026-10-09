//! `cargo run -p instance-format --example resave -- IN.gmk|IN.gm81 OUT.gm81 [--v800]`
//! — reads a project (v701 / v800 / v810) and writes it again through
//! instance-format, as GameMaker 8.1 (v810) by default or 8.0 (v800) with `--v800`.
//! Re-reads the written file and checks it round-trips to the same model.

use instance_format::{read_project_file, read_project_versioned, write_project, FormatVersion};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let [inp, out] = files[..] else {
        eprintln!("usage: resave IN.gmk|IN.gm81 OUT.gm81 [--v800]");
        std::process::exit(2);
    };
    let version = if args.iter().any(|a| a == "--v800") { FormatVersion::V800 } else { FormatVersion::V810 };
    let (p, v) = read_project_file(inp).unwrap_or_else(|e| {
        eprintln!("resave: {inp}: {e}");
        std::process::exit(1)
    });
    let bytes = write_project(&p, version).expect("write");
    let (back, _) = read_project_versioned(&bytes).expect("re-read written file");
    if back != p {
        eprintln!("resave: warning: written file does not read back identically");
    }
    std::fs::write(out, &bytes).expect("write file");
    println!("{inp}: {v:?} -> {out}: {version:?} ({} bytes)", bytes.len());
}
