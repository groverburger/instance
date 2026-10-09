//! `cargo run -p instance-format --example extract_sounds -- GAME.gm81 OUT_DIR [--kind N]`
//! — writes the embedded data of every sound resource to OUT_DIR as
//! `<name><file_type>` (e.g. the MIDI files of background music), optionally only
//! sounds of one kind (0 normal, 1 background music, 2 3D, 3 multimedia player).

use instance_format::read_project_file;

fn main() {
    let mut kind: Option<i32> = None;
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--kind" {
            kind = args.next().and_then(|k| k.parse().ok());
        } else {
            files.push(a);
        }
    }
    let [inp, out] = &files[..] else {
        eprintln!("usage: extract_sounds GAME.gm81 OUT_DIR [--kind N]");
        std::process::exit(2);
    };
    let (p, _) = read_project_file(inp).expect("read project");
    std::fs::create_dir_all(out).expect("create OUT_DIR");
    for s in p.sounds.iter().flatten() {
        if kind.is_some_and(|k| k != s.kind) {
            continue;
        }
        let Some(data) = &s.data else { continue };
        let safe: String = s.name.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
        let path = std::path::Path::new(out).join(format!("{safe}{}", s.file_type));
        std::fs::write(&path, data).expect("write sound");
        println!("{} kind {} {} bytes -> {}", s.name, s.kind, data.len(), path.display());
    }
}
