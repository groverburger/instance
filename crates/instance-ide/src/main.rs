//! Clean-room GameMaker 8.1 compatible IDE (specs/ide.md), built with eframe/egui.
//!
//! Usage: `instance-ide [project.gm81|project.gmk]`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let open = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    instance_ide::run_native(open)
}

/// The web build is a library started from JavaScript (`start_ide`).
#[cfg(target_arch = "wasm32")]
fn main() {}
