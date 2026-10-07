//! Clean-room GameMaker 8.1 compatible IDE (specs/ide.md), built with eframe/egui.
//!
//! Usage: `gm-ide [project.gm81|project.gmk]`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod editors;
mod events;
mod exe;
mod icons;
mod imageio;
mod mdi;
mod prefs;
mod project_ops;
mod rtf;
mod run;
mod theme;
mod tree;
mod widgets;

fn main() -> eframe::Result<()> {
    let open = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0])
            .with_drag_and_drop(true)
            .with_icon(eframe::egui::IconData {
                rgba: gm_brand::icon_rgba(48),
                width: 48,
                height: 48,
            }),
        ..Default::default()
    };
    eframe::run_native(
        "GM81 Clean IDE",
        options,
        Box::new(move |cc| Ok(Box::new(app::IdeApp::new(cc, open)))),
    )
}

#[cfg(test)]
mod tests;
