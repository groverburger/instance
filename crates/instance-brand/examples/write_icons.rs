//! Writes the app icon files: `cargo run -p instance-brand --example write_icons -- OUT_DIR`
//! produces `gm81-clean.icns` (macOS bundles), `gm81-clean.ico` and `gm81-clean.png` (512 px,
//! Linux desktop entries).

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    std::fs::create_dir_all(&dir).expect("create output folder");
    std::fs::write(dir.join("gm81-clean.icns"), instance_brand::icns_file()).expect("write icns");
    std::fs::write(dir.join("gm81-clean.ico"), instance_brand::ico_file(&instance_brand::ICO_SIZES)).expect("write ico");
    std::fs::write(dir.join("gm81-clean.png"), instance_brand::icon_png(512)).expect("write png");
    println!("icons written to {}", dir.display());
}
