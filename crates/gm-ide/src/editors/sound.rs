//! Sound form (specs/ide.md §8).

use eframe::egui::{self, Ui};
use gm_format::{Project, ResourceKind, Sound};

use super::{commit_slot, name_field, slot_dirty, Cx, Editor, Request, ResKey};

pub struct SoundEditor {
    idx: usize,
    work: Sound,
}

impl SoundEditor {
    pub fn new(idx: usize, work: Sound) -> Self {
        SoundEditor { idx, work }
    }
}

/// Load a sound file into `s` (MIDI defaults to background music, spec §8).
pub fn load_sound_file(s: &mut Sound, path: &std::path::Path) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_ascii_lowercase()))
        .unwrap_or_default();
    s.kind = match ext.as_str() {
        ".mid" | ".midi" => 1,
        ".wav" => 0,
        _ => 3,
    };
    s.file_type = ext;
    s.file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    s.data = Some(data);
    Ok(())
}

impl Editor for SoundEditor {
    fn key(&self) -> ResKey {
        ResKey::Res(ResourceKind::Sound, self.idx)
    }
    fn title(&self) -> String {
        format!("Sound Properties: {}", self.work.name)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let s = &mut self.work;
        name_field(ui, &mut s.name);
        ui.horizontal(|ui| {
            if ui.button("Load Sound…").clicked() {
                if let Some(f) = rfd::FileDialog::new()
                    .add_filter("Sounds", &["wav", "mid", "midi", "mp3", "ogg"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    if let Err(e) = load_sound_file(s, &f) {
                        cx.requests.push(Request::Message(e));
                    }
                }
            }
            if ui
                .add_enabled(s.data.is_some(), egui::Button::new("Save Sound…"))
                .clicked()
            {
                if let Some(f) = rfd::FileDialog::new()
                    .set_file_name(&s.file_name)
                    .save_file()
                {
                    if let Err(e) = std::fs::write(f, s.data.as_deref().unwrap_or_default()) {
                        cx.requests.push(Request::Message(e.to_string()));
                    }
                }
            }
        });
        ui.label(format!(
            "File: {} ({}, {} bytes)",
            if s.file_name.is_empty() {
                "<none>"
            } else {
                &s.file_name
            },
            s.file_type,
            s.data.as_ref().map_or(0, Vec::len)
        ));
        ui.separator();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Kind");
                ui.radio_value(&mut s.kind, 0, "Normal Sound");
                ui.radio_value(&mut s.kind, 1, "Background Music");
                ui.radio_value(&mut s.kind, 2, "3D Sound");
                ui.radio_value(&mut s.kind, 3, "Use multimedia player");
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.label("Effects");
                for (bit, name) in [
                    (1, "Chorus"),
                    (2, "Echo"),
                    (4, "Flanger"),
                    (8, "Gargle"),
                    (16, "Reverb"),
                ] {
                    let mut on = s.effects & bit != 0;
                    if ui.checkbox(&mut on, name).changed() {
                        s.effects ^= bit;
                    }
                }
            });
        });
        ui.separator();
        ui.add(egui::Slider::new(&mut s.volume, 0.0..=1.0).text("Volume"));
        ui.add(egui::Slider::new(&mut s.pan, -1.0..=1.0).text("Pan"));
        ui.checkbox(&mut s.preload, "Preload");
    }
    fn on_renamed(&mut self, name: &str) {
        self.work.name = name.to_string();
    }
    fn is_dirty(&self, p: &Project) -> bool {
        slot_dirty(p.sounds.get(self.idx), &self.work)
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        commit_slot(p.sounds.get_mut(self.idx), &self.work)
    }
}
