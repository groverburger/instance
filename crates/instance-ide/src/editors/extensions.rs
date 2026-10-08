//! Extension Packages selector (specs/ide.md §18.4). Reading installed `.ged`
//! packages is not implemented yet; the used-package name list can be edited directly.

use eframe::egui::{self, Ui, Vec2};
use instance_format::Project;

use super::{Cx, Editor, ResKey};

pub struct ExtensionsEditor {
    work: Vec<String>,
    new_name: String,
}

impl ExtensionsEditor {
    pub fn new(p: &Project) -> Self {
        ExtensionsEditor {
            work: p.extension_packages.clone(),
            new_name: String::new(),
        }
    }
}

impl Editor for ExtensionsEditor {
    fn key(&self) -> ResKey {
        ResKey::Extensions
    }
    fn title(&self) -> String {
        "Extension Packages".into()
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(400.0, 320.0)
    }
    fn ui(&mut self, ui: &mut Ui, _cx: &mut Cx) {
        ui.label("Used packages:");
        let mut remove = None;
        for (i, n) in self.work.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(n);
                if ui.small_button("Remove").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            self.work.remove(i);
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.new_name);
            if ui.button("Add").clicked() && !self.new_name.trim().is_empty() {
                self.work.push(self.new_name.trim().to_string());
                self.new_name.clear();
            }
        });
        ui.label(
            egui::RichText::new("Installed-package browsing (.ged) is not available yet.")
                .weak()
                .small(),
        );
    }
    fn is_dirty(&self, p: &Project) -> bool {
        p.extension_packages != self.work
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        if p.extension_packages == self.work {
            return false;
        }
        p.extension_packages = self.work.clone();
        true
    }
}
