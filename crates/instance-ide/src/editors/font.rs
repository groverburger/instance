//! Font form (specs/ide.md §12).

use eframe::egui::{self, RichText, Ui};
use instance_format::{Font, Project, ResourceKind};

use super::{commit_slot, name_field, slot_dirty, Cx, Editor, ResKey};

/// Common font families offered in the dropdown (any name may be typed).
const FONTS: &[&str] = &[
    "Arial",
    "Arial Black",
    "Comic Sans MS",
    "Courier New",
    "Georgia",
    "Impact",
    "Lucida Console",
    "Tahoma",
    "Times New Roman",
    "Trebuchet MS",
    "Verdana",
];

/// Character set names (Windows charset ids).
const CHARSETS: &[(u8, &str)] = &[
    (0, "ANSI_CHARSET"),
    (1, "DEFAULT_CHARSET"),
    (2, "SYMBOL_CHARSET"),
    (128, "SHIFTJIS_CHARSET"),
    (129, "HANGEUL_CHARSET"),
    (134, "GB2312_CHARSET"),
    (136, "CHINESEBIG5_CHARSET"),
    (161, "GREEK_CHARSET"),
    (162, "TURKISH_CHARSET"),
    (177, "HEBREW_CHARSET"),
    (178, "ARABIC_CHARSET"),
    (186, "BALTIC_CHARSET"),
    (204, "RUSSIAN_CHARSET"),
    (222, "THAI_CHARSET"),
    (238, "EASTEUROPE_CHARSET"),
    (255, "OEM_CHARSET"),
];

pub struct FontEditor {
    idx: usize,
    work: Font,
    sample: String,
}

impl FontEditor {
    pub fn new(idx: usize, work: Font) -> Self {
        FontEditor {
            idx,
            work,
            sample: "AaBbCcDd 0123".into(),
        }
    }
}

impl Editor for FontEditor {
    fn key(&self) -> ResKey {
        ResKey::Res(ResourceKind::Font, self.idx)
    }
    fn title(&self) -> String {
        format!("Font Properties: {}", self.work.name)
    }
    fn ui(&mut self, ui: &mut Ui, _cx: &mut Cx) {
        let f = &mut self.work;
        name_field(ui, &mut f.name);
        egui::Grid::new("font_grid").num_columns(2).show(ui, |ui| {
            ui.label("Font:");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut f.font_name).desired_width(150.0));
                egui::ComboBox::from_id_salt("font_pick")
                    .selected_text("…")
                    .width(30.0)
                    .show_ui(ui, |ui| {
                        for n in FONTS {
                            if ui.selectable_label(f.font_name == *n, *n).clicked() {
                                f.font_name = n.to_string();
                            }
                        }
                    });
            });
            ui.end_row();
            ui.label("Size:");
            ui.add(egui::DragValue::new(&mut f.size).range(1..=300));
            ui.end_row();
            ui.label("Style:");
            ui.horizontal(|ui| {
                ui.checkbox(&mut f.bold, "Bold");
                ui.checkbox(&mut f.italic, "Italic");
            });
            ui.end_row();
            ui.label("Anti-aliasing:");
            ui.add(egui::Slider::new(&mut f.antialias, 0..=3));
            ui.end_row();
            ui.label("Character set:");
            let cur = CHARSETS
                .iter()
                .find(|c| c.0 == f.charset)
                .map_or_else(|| f.charset.to_string(), |c| c.1.to_string());
            egui::ComboBox::from_id_salt("charset")
                .selected_text(cur)
                .show_ui(ui, |ui| {
                    for (v, n) in CHARSETS {
                        ui.selectable_value(&mut f.charset, *v, *n);
                    }
                });
            ui.end_row();
            ui.label("Character range:");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut f.range_start).range(0..=65535));
                ui.label("till");
                ui.add(egui::DragValue::new(&mut f.range_end).range(0..=65535));
            });
            ui.end_row();
            ui.label("");
            ui.horizontal(|ui| {
                for (l, a, b) in [
                    ("Normal", 32, 127),
                    ("All", 0, 255),
                    ("Digits", 48, 57),
                    ("Letters", 65, 122),
                ] {
                    if ui.button(l).clicked() {
                        f.range_start = a;
                        f.range_end = b;
                    }
                }
            });
            ui.end_row();
        });
        ui.separator();
        ui.text_edit_singleline(&mut self.sample);
        let mut t = RichText::new(&self.sample).size(f.size.clamp(4, 96) as f32 * 4.0 / 3.0);
        if f.bold {
            t = t.family(crate::theme::bold()).strong();
        }
        if f.italic {
            t = t.italics();
        }
        ui.label(t);
        ui.label(
            RichText::new("(preview uses the IDE font, not the selected family)")
                .weak()
                .small(),
        );
    }
    fn on_renamed(&mut self, name: &str) {
        self.work.name = name.to_string();
    }
    fn is_dirty(&self, p: &Project) -> bool {
        slot_dirty(p.fonts.get(self.idx), &self.work)
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        commit_slot(p.fonts.get_mut(self.idx), &self.work)
    }
}
