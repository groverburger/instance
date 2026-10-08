//! Global Game Settings form (specs/ide.md §17).

use eframe::egui::{self, Ui, Vec2};
use instance_format::{Project, Settings};

use super::{Cx, Editor, Request, ResKey};
use crate::project_ops::now_timestamp;
use crate::widgets::color_box;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Graphics,
    Resolution,
    Other,
    Loading,
    Errors,
    Info,
}

pub struct SettingsEditor {
    work: Settings,
    game_id: i32,
    tab: Tab,
}

impl SettingsEditor {
    pub fn new(p: &Project) -> Self {
        SettingsEditor {
            work: p.settings.clone(),
            game_id: p.game_id,
            tab: Tab::Graphics,
        }
    }
}

const IMAGE_EXTS: &[&str] = &["bmp", "png", "jpg", "gif"];

/// Asks for a file whose bytes become a setting (fetched by [`take_blob`]).
fn load_blob(key: u64, exts: &[&str]) {
    crate::dialogs::request_open(key, &[("Files", exts)], false);
}

/// The bytes picked for `key`, if any arrived.
fn take_blob(cx: &mut Cx, key: u64) -> Option<Vec<u8>> {
    match crate::dialogs::take_opened(key)? {
        Ok(files) => files.into_iter().next().map(|f| f.bytes),
        Err(e) => {
            cx.requests.push(Request::Message(e));
            None
        }
    }
}

impl Editor for SettingsEditor {
    fn key(&self) -> ResKey {
        ResKey::Settings
    }
    fn title(&self) -> String {
        "Global Game Settings".into()
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(560.0, 460.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let me = self as *const Self;
        let (k_load, k_back, k_front, k_icon) = (
            crate::dialogs::key("settings-load-image", me),
            crate::dialogs::key("settings-bar-back", me),
            crate::dialogs::key("settings-bar-front", me),
            crate::dialogs::key("settings-icon", me),
        );
        if let Some(b) = take_blob(cx, k_load) {
            self.work.custom_load_image = Some(b);
        }
        if let Some(b) = take_blob(cx, k_back) {
            self.work.loading_bar_back = Some(b);
        }
        if let Some(b) = take_blob(cx, k_front) {
            self.work.loading_bar_front = Some(b);
        }
        if let Some(b) = take_blob(cx, k_icon) {
            self.work.icon = b;
        }
        ui.horizontal_wrapped(|ui| {
            for (t, l) in [
                (Tab::Graphics, "Graphics"),
                (Tab::Resolution, "Resolution"),
                (Tab::Other, "Other"),
                (Tab::Loading, "Loading"),
                (Tab::Errors, "Errors"),
                (Tab::Info, "Info"),
            ] {
                ui.selectable_value(&mut self.tab, t, l);
            }
        });
        ui.separator();
        let s = &mut self.work;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.tab {
                Tab::Graphics => {
                    ui.checkbox(&mut s.start_fullscreen, "Start in fullscreen mode");
                    ui.label("Scaling");
                    let mut mode = match s.scaling {
                        x if x > 0 => 0,
                        -1 => 1,
                        _ => 2,
                    };
                    let old = mode;
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut mode, 0, "Fixed scale (%)");
                        if mode == 0 {
                            ui.add(egui::DragValue::new(&mut s.scaling).range(1..=1000));
                        }
                    });
                    ui.radio_value(&mut mode, 1, "Keep aspect ratio");
                    ui.radio_value(&mut mode, 2, "Full scale");
                    if mode != old {
                        s.scaling = match mode {
                            0 => 100,
                            1 => -1,
                            _ => 0,
                        };
                    }
                    ui.checkbox(
                        &mut s.interpolate_pixels,
                        "Interpolate colors between pixels",
                    );
                    ui.horizontal(|ui| {
                        ui.label("Color outside the room region:");
                        color_box(ui, &mut s.color_outside_room);
                    });
                    ui.checkbox(
                        &mut s.allow_window_resize,
                        "Allow the player to resize the game window",
                    );
                    ui.checkbox(
                        &mut s.always_on_top,
                        "Let the game window always stay on top",
                    );
                    ui.checkbox(
                        &mut s.dont_draw_border,
                        "Don't draw a border in windowed mode",
                    );
                    ui.checkbox(
                        &mut s.dont_show_buttons,
                        "Don't show the buttons in the window caption",
                    );
                    ui.checkbox(&mut s.display_cursor, "Display the cursor");
                    ui.checkbox(
                        &mut s.freeze_on_lose_focus,
                        "Freeze the game when the form loses focus",
                    );
                    ui.checkbox(
                        &mut s.disable_screensaver,
                        "Disable screensavers and power saving actions",
                    );
                }
                Tab::Resolution => {
                    ui.checkbox(&mut s.set_resolution, "Set the resolution of the screen");
                    ui.add_enabled_ui(s.set_resolution, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Color depth:");
                            ui.radio_value(&mut s.color_depth, 0, "No change");
                            ui.radio_value(&mut s.color_depth, 1, "16-bit");
                            ui.radio_value(&mut s.color_depth, 2, "32-bit");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Resolution (index, 0 = no change):");
                            ui.add(egui::DragValue::new(&mut s.resolution).range(0..=20));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Frequency (index, 0 = no change):");
                            ui.add(egui::DragValue::new(&mut s.frequency).range(0..=20));
                        });
                    });
                    let mut vs = s.vsync & 1 != 0;
                    if ui
                        .checkbox(&mut vs, "Use synchronization to avoid tearing")
                        .changed()
                    {
                        s.vsync ^= 1;
                    }
                }
                Tab::Other => {
                    ui.label("Default keys");
                    ui.checkbox(&mut s.let_esc_end_game, "Let <Esc> end the game");
                    ui.checkbox(
                        &mut s.treat_close_as_esc,
                        "Treat the close button as <Esc> key",
                    );
                    ui.checkbox(
                        &mut s.let_f1_game_info,
                        "Let <F1> show the game information",
                    );
                    ui.checkbox(
                        &mut s.let_f4_fullscreen,
                        "Let <F4> switch between screen modes",
                    );
                    ui.checkbox(
                        &mut s.let_f5_save_f6_load,
                        "Let <F5> save the game and <F6> load a game",
                    );
                    ui.checkbox(
                        &mut s.let_f9_screenshot,
                        "Let <F9> take a screenshot of the game",
                    );
                    ui.horizontal(|ui| {
                        ui.label("Game process priority:");
                        ui.radio_value(&mut s.process_priority, 0, "Normal");
                        ui.radio_value(&mut s.process_priority, 1, "High");
                        ui.radio_value(&mut s.process_priority, 2, "Highest");
                    });
                    ui.separator();
                    ui.label("Version information");
                    ui.horizontal(|ui| {
                        for (l, v) in [
                            ("Major", &mut s.version_major),
                            ("Minor", &mut s.version_minor),
                            ("Release", &mut s.version_release),
                            ("Build", &mut s.version_build),
                        ] {
                            ui.label(l);
                            ui.add(egui::DragValue::new(v).range(0..=65535));
                        }
                    });
                    egui::Grid::new("verinfo").num_columns(2).show(ui, |ui| {
                        for (l, v) in [
                            ("Company:", &mut s.company),
                            ("Product:", &mut s.product),
                            ("Copyright:", &mut s.copyright),
                            ("Description:", &mut s.description),
                        ] {
                            ui.label(l);
                            ui.add(egui::TextEdit::singleline(v).char_limit(64));
                            ui.end_row();
                        }
                    });
                }
                Tab::Loading => {
                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut s.show_custom_load_image,
                            "Show your own image while loading",
                        );
                        if ui.button("Change image…").clicked() {
                            load_blob(k_load, IMAGE_EXTS);
                        }
                        if ui.button("Clear").clicked() {
                            s.custom_load_image = None;
                        }
                    });
                    ui.add_enabled_ui(s.show_custom_load_image, |ui| {
                        ui.checkbox(
                            &mut s.load_image_transparent,
                            "Make image partially transparent",
                        );
                        ui.add(
                            egui::Slider::new(&mut s.load_image_alpha, 0..=255)
                                .text("Translucency alpha"),
                        );
                    });
                    ui.separator();
                    ui.label("Loading progress bar");
                    ui.radio_value(&mut s.loading_bar, 0, "No loading progress bar");
                    ui.radio_value(&mut s.loading_bar, 1, "Default loading progress bar");
                    ui.radio_value(&mut s.loading_bar, 2, "Own loading progress bar");
                    ui.add_enabled_ui(s.loading_bar == 2, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Back image…").clicked() {
                                load_blob(k_back, IMAGE_EXTS);
                            }
                            if ui.button("Front image…").clicked() {
                                load_blob(k_front, IMAGE_EXTS);
                            }
                        });
                        ui.checkbox(&mut s.scale_progress_bar, "Scale progress bar image");
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(format!("Game icon: {} bytes", s.icon.len()));
                        if ui.button("Change icon…").clicked() {
                            load_blob(k_icon, &["ico"]);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Game identifier:");
                        ui.add(egui::DragValue::new(&mut self.game_id));
                        if ui.button("Change").clicked() {
                            self.game_id = crate::project_ops::new_project().game_id;
                        }
                    });
                }
                Tab::Errors => {
                    ui.checkbox(&mut s.display_errors, "Display error messages");
                    ui.checkbox(
                        &mut s.write_errors_to_log,
                        "Write error messages to file game_errors.log",
                    );
                    ui.checkbox(&mut s.abort_on_all_errors, "Abort on all error messages");
                    ui.checkbox(
                        &mut s.treat_uninitialized_as_zero,
                        "Treat uninitialized variables as value 0",
                    );
                    ui.checkbox(
                        &mut s.argument_errors,
                        "Throw an error when arguments aren't initialised correctly.",
                    );
                }
                Tab::Info => {
                    egui::Grid::new("info").num_columns(2).show(ui, |ui| {
                        ui.label("Author:");
                        ui.text_edit_singleline(&mut s.author);
                        ui.end_row();
                        ui.label("Version:");
                        ui.text_edit_singleline(&mut s.version_text);
                        ui.end_row();
                    });
                    ui.label("Information:");
                    ui.add(
                        egui::TextEdit::multiline(&mut s.information)
                            .desired_rows(10)
                            .desired_width(f32::INFINITY),
                    );
                }
            });
    }
    fn is_dirty(&self, p: &Project) -> bool {
        p.settings != self.work || p.game_id != self.game_id
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        if !self.is_dirty(p) {
            return false;
        }
        let now = now_timestamp();
        let mut w = self.work.clone();
        if w.author != p.settings.author
            || w.version_text != p.settings.version_text
            || w.information != p.settings.information
        {
            w.info_last_changed = now;
        }
        w.last_changed = now;
        p.settings = w;
        p.game_id = self.game_id;
        true
    }
}
