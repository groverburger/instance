//! User-Defined Constants dialog (specs/ide.md §18.1).

use eframe::egui::{self, Ui, Vec2};
use gm_format::{Constant, Project};

use super::{Cx, Editor, Request, ResKey};
use crate::project_ops::now_timestamp;

pub struct ConstantsEditor {
    work: Vec<Constant>,
    sel: Option<usize>,
}

impl ConstantsEditor {
    pub fn new(p: &Project) -> Self {
        ConstantsEditor {
            work: p.constants.clone(),
            sel: None,
        }
    }
}

/// Parse `NAME=VALUE` lines.
pub fn parse_constants(text: &str) -> Vec<Constant> {
    text.lines()
        .filter_map(|l| {
            let (n, v) = l.split_once('=')?;
            let n = n.trim();
            (!n.is_empty()).then(|| Constant {
                name: n.to_string(),
                value: v.trim().to_string(),
            })
        })
        .collect()
}

impl Editor for ConstantsEditor {
    fn key(&self) -> ResKey {
        ResKey::Constants
    }
    fn title(&self) -> String {
        "User-Defined Constants".into()
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(520.0, 420.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        ui.horizontal_wrapped(|ui| {
            let sel = self.sel.filter(|&i| i < self.work.len());
            if ui.button("Insert").clicked() {
                let at = sel.unwrap_or(0);
                self.work.insert(at, Constant::default());
                self.sel = Some(at);
            }
            if ui.button("Add").clicked() {
                self.work.push(Constant::default());
                self.sel = Some(self.work.len() - 1);
            }
            if ui
                .add_enabled(sel.is_some(), egui::Button::new("Delete"))
                .clicked()
            {
                self.work.remove(sel.unwrap_or(0));
                self.sel = None;
            }
            if ui.button("Clear").clicked() {
                self.work.clear();
                self.sel = None;
            }
            if ui
                .add_enabled(sel.is_some_and(|i| i > 0), egui::Button::new("Up"))
                .clicked()
            {
                let i = sel.unwrap_or(0);
                self.work.swap(i, i - 1);
                self.sel = Some(i - 1);
            }
            if ui
                .add_enabled(
                    sel.is_some_and(|i| i + 1 < self.work.len()),
                    egui::Button::new("Down"),
                )
                .clicked()
            {
                let i = sel.unwrap_or(0);
                self.work.swap(i, i + 1);
                self.sel = Some(i + 1);
            }
            if ui.button("Sort").clicked() {
                self.work.sort_by(|a, b| a.name.cmp(&b.name));
            }
            if ui.button("Load…").clicked() {
                if let Some(f) = rfd::FileDialog::new()
                    .add_filter("Text", &["txt"])
                    .pick_file()
                {
                    match std::fs::read(f) {
                        Ok(b) => self
                            .work
                            .extend(parse_constants(&gm_format::text::decode_cp1252(&b))),
                        Err(e) => cx.requests.push(Request::Message(e.to_string())),
                    }
                }
            }
            if ui.button("Save…").clicked() {
                if let Some(f) = rfd::FileDialog::new()
                    .add_filter("Text", &["txt"])
                    .save_file()
                {
                    let text: String = self
                        .work
                        .iter()
                        .map(|c| format!("{}={}\r\n", c.name, c.value))
                        .collect();
                    if let Err(e) = std::fs::write(f, gm_format::text::encode_cp1252(&text)) {
                        cx.requests.push(Request::Message(e.to_string()));
                    }
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("consts")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("");
                        ui.strong("Name");
                        ui.strong("Value");
                        ui.end_row();
                        for (i, c) in self.work.iter_mut().enumerate() {
                            if ui
                                .selectable_label(self.sel == Some(i), format!("{i}"))
                                .clicked()
                            {
                                self.sel = Some(i);
                            }
                            if ui
                                .add(egui::TextEdit::singleline(&mut c.name).desired_width(150.0))
                                .gained_focus()
                            {
                                self.sel = Some(i);
                            }
                            if ui
                                .add(egui::TextEdit::singleline(&mut c.value).desired_width(220.0))
                                .gained_focus()
                            {
                                self.sel = Some(i);
                            }
                            ui.end_row();
                        }
                    });
            });
    }
    fn is_dirty(&self, p: &Project) -> bool {
        p.constants != self.work
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        if p.constants == self.work {
            return false;
        }
        p.constants = self.work.clone();
        p.constants_last_changed = now_timestamp();
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse() {
        let c = super::parse_constants("A=1\r\nB = \"x=y\"\nbad\n=3\n");
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].name, "B");
        assert_eq!(c[1].value, "\"x=y\"");
    }
}
