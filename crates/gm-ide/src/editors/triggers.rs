//! Triggers dialog (specs/ide.md §18.2).

use eframe::egui::{self, Ui, Vec2};
use gm_format::{Project, Trigger};

use super::{Cx, Editor, ResKey};
use crate::project_ops::now_timestamp;
use crate::widgets::code_editor::{code_editor, CodeEditorState};

pub struct TriggersEditor {
    work: Vec<Option<Trigger>>,
    sel: Option<usize>,
    code_state: CodeEditorState,
}

impl TriggersEditor {
    pub fn new(p: &Project) -> Self {
        TriggersEditor {
            work: p.triggers.clone(),
            sel: p.triggers.iter().position(Option::is_some),
            code_state: CodeEditorState::default(),
        }
    }
}

impl Editor for TriggersEditor {
    fn key(&self) -> ResKey {
        ResKey::Triggers
    }
    fn title(&self) -> String {
        "Triggers".into()
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(720.0, 440.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let h = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(180.0);
                egui::ScrollArea::vertical()
                    .max_height(h - 40.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for (i, t) in self.work.iter().enumerate() {
                            if let Some(t) = t {
                                let name = if t.name.is_empty() {
                                    "<unnamed>"
                                } else {
                                    &t.name
                                };
                                if ui.selectable_label(self.sel == Some(i), name).clicked() {
                                    self.sel = Some(i);
                                }
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.button("Add").clicked() {
                        // Reuse the first free slot: the slot index is the event number.
                        let t = Trigger {
                            name: format!("trigger{}", self.work.iter().flatten().count()),
                            ..Default::default()
                        };
                        let i = match self.work.iter().position(Option::is_none) {
                            Some(i) => {
                                self.work[i] = Some(t);
                                i
                            }
                            None => {
                                self.work.push(Some(t));
                                self.work.len() - 1
                            }
                        };
                        self.sel = Some(i);
                    }
                    if ui
                        .add_enabled(self.sel.is_some(), egui::Button::new("Delete"))
                        .clicked()
                    {
                        if let Some(i) = self.sel {
                            if let Some(s) = self.work.get_mut(i) {
                                *s = None;
                            }
                            while self.work.last().is_some_and(Option::is_none) {
                                self.work.pop();
                            }
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
                let Some(t) = self
                    .sel
                    .and_then(|i| self.work.get_mut(i))
                    .and_then(|t| t.as_mut())
                else {
                    ui.label(egui::RichText::new("Select or add a trigger.").weak());
                    return;
                };
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut t.name);
                });
                ui.label("Condition:");
                ui.allocate_ui(
                    Vec2::new(ui.available_width(), (h - 120.0).max(120.0)),
                    |ui| {
                        code_editor(
                            ui,
                            "trigger_cond",
                            &mut t.condition,
                            &mut self.code_state,
                            cx.names,
                            cx.prefs,
                            8,
                        );
                    },
                );
                ui.horizontal(|ui| {
                    ui.label("Moment of checking:");
                    ui.radio_value(&mut t.moment, 1, "Begin of step");
                    ui.radio_value(&mut t.moment, 0, "Middle of step");
                    ui.radio_value(&mut t.moment, 2, "End of step");
                });
                ui.horizontal(|ui| {
                    ui.label("Constant name:");
                    ui.text_edit_singleline(&mut t.constant_name);
                });
            });
        });
    }
    fn is_dirty(&self, p: &Project) -> bool {
        p.triggers != self.work
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        if p.triggers == self.work {
            return false;
        }
        p.triggers = self.work.clone();
        p.triggers_last_changed = now_timestamp();
        true
    }
}
