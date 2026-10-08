//! Script form (specs/ide.md §11).

use eframe::egui::{self, Ui, Vec2};
use gm_format::{Project, ResourceKind, Script};

use super::{commit_slot, slot_dirty, Cx, Editor, Request, ResKey};
use crate::widgets::code_editor::{code_editor, CodeEditorState};

pub struct ScriptEditor {
    idx: usize,
    work: Script,
    code_state: CodeEditorState,
}

impl ScriptEditor {
    pub fn new(idx: usize, work: Script) -> Self {
        ScriptEditor {
            idx,
            work,
            code_state: CodeEditorState::default(),
        }
    }
}

impl Editor for ScriptEditor {
    fn key(&self) -> ResKey {
        ResKey::Res(ResourceKind::Script, self.idx)
    }
    fn title(&self) -> String {
        format!("Script Properties: {}", self.work.name)
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(640.0, 480.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let load_key = crate::dialogs::key("script-load", self as *const Self);
        match crate::dialogs::take_opened(load_key) {
            Some(Ok(files)) => {
                if let Some(f) = files.first() {
                    self.work.code = gm_format::text::decode_cp1252(&f.bytes);
                }
            }
            Some(Err(e)) => cx.requests.push(Request::Message(e)),
            None => {}
        }
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(egui::TextEdit::singleline(&mut self.work.name).desired_width(180.0));
            if ui.button("Load from file…").clicked() {
                crate::dialogs::request_open(load_key, &[("Text", &["txt", "gml"])], false);
            }
            if ui.button("Save to file…").clicked() {
                if let Err(e) = crate::dialogs::save_bytes(
                    &format!("{}.gml", self.work.name),
                    None,
                    &[("Text", &["txt", "gml"])],
                    &gm_format::text::encode_cp1252(&self.work.code),
                ) {
                    cx.requests.push(Request::Message(e));
                }
            }
        });
        code_editor(
            ui,
            ("script", self.idx),
            &mut self.work.code,
            &mut self.code_state,
            cx.names,
            cx.prefs,
            20,
        );
    }
    fn on_renamed(&mut self, name: &str) {
        self.work.name = name.to_string();
    }
    fn is_dirty(&self, p: &Project) -> bool {
        slot_dirty(p.scripts.get(self.idx), &self.work)
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        commit_slot(p.scripts.get_mut(self.idx), &self.work)
    }
}
