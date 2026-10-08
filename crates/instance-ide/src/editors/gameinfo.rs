//! Game Information editor (specs/ide.md §16). P0 stopgap: the RTF document is edited
//! as plain text; the original RTF is kept byte-for-byte unless the text changes, and
//! the header (font/colour tables, initial formatting) is kept when it does.

use eframe::egui::{self, Ui, Vec2};
use instance_format::{GameInfo, Project};

use super::{Cx, Editor, ResKey};
use crate::project_ops::now_timestamp;
use crate::rtf::{plain_to_rtf, rtf_to_plain};
use crate::widgets::color_box;

pub struct GameInfoEditor {
    work: GameInfo,
    text: String,
    show_options: bool,
}

impl GameInfoEditor {
    pub fn new(g: &GameInfo) -> Self {
        GameInfoEditor {
            work: g.clone(),
            text: rtf_to_plain(&g.rtf),
            show_options: false,
        }
    }

    fn result(&self, p: &Project) -> GameInfo {
        let mut g = self.work.clone();
        g.rtf = plain_to_rtf(&p.game_info.rtf, &self.text);
        g
    }
}

impl Editor for GameInfoEditor {
    fn key(&self) -> ResKey {
        ResKey::GameInfo
    }
    fn title(&self) -> String {
        "Game Information".into()
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(600.0, 440.0)
    }
    fn ui(&mut self, ui: &mut Ui, _cx: &mut Cx) {
        ui.horizontal(|ui| {
            ui.toggle_value(&mut self.show_options, "Options");
            ui.label("Background color:");
            color_box(ui, &mut self.work.background_color);
        });
        if self.show_options {
            let g = &mut self.work;
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.checkbox(
                    &mut g.show_in_separate_window,
                    "Show help in a separate window",
                );
                ui.add_enabled_ui(g.show_in_separate_window, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Caption:");
                        ui.text_edit_singleline(&mut g.window_caption);
                    });
                    ui.horizontal(|ui| {
                        for (l, v) in [
                            ("Left", &mut g.left),
                            ("Top", &mut g.top),
                            ("Width", &mut g.width),
                            ("Height", &mut g.height),
                        ] {
                            ui.label(l);
                            ui.add(egui::DragValue::new(v));
                        }
                    });
                    ui.checkbox(
                        &mut g.show_window_border,
                        "Show the window border and caption",
                    );
                    ui.checkbox(&mut g.allow_resize, "Allow the player to resize the window");
                    ui.checkbox(&mut g.stay_on_top, "Stay always on top");
                });
                ui.checkbox(&mut g.pause_game, "Stop the game while showing help");
            });
        }
        let bg = crate::widgets::bgr_to_color(self.work.background_color);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new().fill(bg).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.text)
                            .desired_width(f32::INFINITY)
                            .desired_rows(20),
                    );
                });
            });
    }
    fn is_dirty(&self, p: &Project) -> bool {
        self.result(p) != p.game_info
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        let mut g = self.result(p);
        if g == p.game_info {
            return false;
        }
        g.last_changed = now_timestamp();
        p.game_info = g;
        true
    }
}
