//! Included Files dialog (specs/ide.md §18.3).

use eframe::egui::{self, Ui, Vec2};
use gm_format::{IncludedFile, Project};

use super::{Cx, Editor, Request, ResKey};
use crate::project_ops::now_timestamp;

pub struct IncludedEditor {
    work: Vec<IncludedFile>,
    sel: Option<usize>,
}

impl IncludedEditor {
    pub fn new(p: &Project) -> Self {
        IncludedEditor {
            work: p.included_files.clone(),
            sel: None,
        }
    }
}

/// Load a picked file into an included-file record.
pub fn load_into(f: &mut IncludedFile, picked: crate::dialogs::Picked, store: bool) {
    let data = picked.bytes;
    f.file_name = picked.name.clone();
    f.original_path = picked.path.map_or(picked.name, |p| p.display().to_string());
    f.original_file_chosen = true;
    f.original_size = data.len() as i32;
    f.data = store.then_some(data);
    f.last_changed = now_timestamp();
}

impl Editor for IncludedEditor {
    fn key(&self) -> ResKey {
        ResKey::IncludedFiles
    }
    fn title(&self) -> String {
        "Included Files".into()
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(680.0, 420.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let (k_add, k_load) = (
            crate::dialogs::key("included-add", self as *const Self),
            crate::dialogs::key("included-load", self as *const Self),
        );
        match crate::dialogs::take_opened(k_add) {
            Some(Ok(files)) => {
                for p in files {
                    let mut f = IncludedFile {
                        free_memory_after_export: true,
                        export_kind: 2,
                        ..Default::default()
                    };
                    load_into(&mut f, p, true);
                    self.work.push(f);
                    self.sel = Some(self.work.len() - 1);
                }
            }
            Some(Err(e)) => cx.requests.push(Request::Message(e)),
            None => {}
        }
        match crate::dialogs::take_opened(k_load) {
            Some(Ok(files)) => {
                if let (Some(p), Some(f)) = (
                    files.into_iter().next(),
                    self.sel.and_then(|i| self.work.get_mut(i)),
                ) {
                    let store = f.data.is_some();
                    load_into(f, p, store);
                }
            }
            Some(Err(e)) => cx.requests.push(Request::Message(e)),
            None => {}
        }
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(220.0);
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for (i, f) in self.work.iter().enumerate() {
                            if ui
                                .selectable_label(self.sel == Some(i), &f.file_name)
                                .clicked()
                            {
                                self.sel = Some(i);
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.button("Add…").clicked() {
                        crate::dialogs::request_open(k_add, crate::dialogs::ALL, true);
                    }
                    if ui
                        .add_enabled(self.sel.is_some(), egui::Button::new("Delete"))
                        .clicked()
                    {
                        if let Some(i) = self.sel.filter(|&i| i < self.work.len()) {
                            self.work.remove(i);
                        }
                        self.sel = None;
                    }
                    if ui.button("Clear").clicked() {
                        self.work.clear();
                        self.sel = None;
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                let Some(f) = self.sel.and_then(|i| self.work.get_mut(i)) else {
                    ui.label(egui::RichText::new("Select or add a file.").weak());
                    return;
                };
                egui::Grid::new("incl").num_columns(2).show(ui, |ui| {
                    ui.label("File name:");
                    ui.text_edit_singleline(&mut f.file_name);
                    ui.end_row();
                    ui.label("Original file:");
                    ui.label(&f.original_path);
                    ui.end_row();
                    ui.label("Size:");
                    ui.label(f.original_size.to_string());
                    ui.end_row();
                });
                if ui.button("Load…").clicked() {
                    crate::dialogs::request_open(k_load, crate::dialogs::ALL, false);
                }
                let mut store = f.data.is_some();
                if ui
                    .checkbox(&mut store, "Store in the editable gm81 file")
                    .changed()
                {
                    if store {
                        match std::fs::read(&f.original_path) {
                            Ok(d) => f.data = Some(d),
                            Err(e) => cx
                                .requests
                                .push(Request::Message(format!("{}: {e}", f.original_path))),
                        }
                    } else {
                        f.data = None;
                    }
                }
                ui.checkbox(&mut f.free_memory_after_export, "Free memory after export");
                ui.checkbox(&mut f.overwrite_existing, "Overwrite the file if it exists");
                ui.checkbox(&mut f.remove_at_game_end, "Remove at game end");
                ui.label("Export:");
                ui.radio_value(&mut f.export_kind, 0, "Don't export automatically");
                ui.radio_value(&mut f.export_kind, 1, "Export to the temporary folder");
                ui.radio_value(&mut f.export_kind, 2, "Export to the folder of the game");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut f.export_kind, 3, "Export to the following folder:");
                    ui.add_enabled(
                        f.export_kind == 3,
                        egui::TextEdit::singleline(&mut f.export_folder),
                    );
                });
            });
        });
    }
    fn is_dirty(&self, p: &Project) -> bool {
        p.included_files != self.work
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        if p.included_files == self.work {
            return false;
        }
        p.included_files = self.work.clone();
        true
    }
}
