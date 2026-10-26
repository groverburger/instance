//! Clean-room GameMaker 8.1 compatible IDE (specs/ide.md), built with eframe/egui.
//!
//! The native launcher is `main.rs` ([`run_native`]); on the web
//! (`wasm32-unknown-unknown`) the IDE is started from JavaScript with
//! `start_ide(canvas_id)` (see `web/ide.html`).

mod app;
mod dialogs;
mod editors;
mod events;
mod exe;
mod help;
mod icons;
mod imageio;
mod mdi;
mod platform;
mod prefs;
mod output_log;
mod project_ops;
mod rtf;
mod run;
mod sound_preview;
mod theme;
mod tree;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(not(target_arch = "wasm32"))]
mod welcome;
mod widgets;

#[cfg(target_arch = "wasm32")]
pub use web::{new_project, open_project_bytes, open_project_url, start_ide};

/// Title of the main window.
pub const APP_TITLE: &str = "Instance";

/// Runs the IDE in a native window, optionally opening `open`.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native(open: Option<std::path::PathBuf>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0])
            .with_drag_and_drop(true)
            .with_icon(eframe::egui::IconData {
                rgba: instance_brand::icon_rgba(48),
                width: 48,
                height: 48,
            }),
        ..Default::default()
    };
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(move |cc| Ok(Box::new(app::IdeApp::new(cc, open)))),
    )
}

#[cfg(test)]
mod tests;
