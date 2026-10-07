//! Object form (specs/ide.md §13).

use eframe::egui::{self, Sense, Ui, Vec2};
use gm_format::{Event, Object, Project, ResourceKind, NO_PARENT};

use super::{commit_slot, slot_dirty, Cx, Editor, Request, ResKey};
use crate::events::event_name;
use crate::widgets::actions::{action_list, arg_dialog, palette, ActionEditState};
use crate::widgets::event_selector::{event_selector, Selection};
use crate::widgets::{image_preview, resource_picker};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectorMode {
    Add,
    Change,
}

pub struct ObjectEditor {
    idx: usize,
    work: Object,
    event: Option<usize>,
    actions: ActionEditState,
    selector: Option<SelectorMode>,
    confirm_delete: bool,
}

impl ObjectEditor {
    pub fn new(idx: usize, work: Object) -> Self {
        let event = if work.events.is_empty() {
            None
        } else {
            Some(0)
        };
        ObjectEditor {
            idx,
            work,
            event,
            actions: ActionEditState::default(),
            selector: None,
            confirm_delete: false,
        }
    }

    /// The working copy as it would be committed: events without actions that were
    /// added in this form are dropped (spec §13.1).
    fn normalized(&self, p: &Project) -> Object {
        let orig = p.objects.get(self.idx).and_then(|o| o.as_ref());
        let mut o = self.work.clone();
        o.events.retain(|e| {
            !e.actions.is_empty() || orig.is_some_and(|x| x.event(e.event_type, e.number).is_some())
        });
        o
    }
}

/// Insert an event keeping events grouped by type (ascending), returns its index.
pub fn insert_event(events: &mut Vec<Event>, ev: Event) -> usize {
    let pos = events
        .iter()
        .position(|e| (e.event_type, e.number) > (ev.event_type, ev.number))
        .unwrap_or(events.len());
    events.insert(pos, ev);
    pos
}

/// Whether making `parent` the parent of object `idx` would create a cycle.
pub fn parent_cycle(p: &Project, idx: usize, parent: i32) -> bool {
    let mut cur = parent;
    let mut steps = 0;
    while cur >= 0 && steps <= p.objects.len() {
        if cur as usize == idx {
            return true;
        }
        cur = p
            .objects
            .get(cur as usize)
            .and_then(|o| o.as_ref())
            .map_or(NO_PARENT, |o| o.parent);
        steps += 1;
    }
    false
}

impl Editor for ObjectEditor {
    fn key(&self) -> ResKey {
        ResKey::Res(ResourceKind::Object, self.idx)
    }
    fn title(&self) -> String {
        format!("Object Properties: {}", self.work.name)
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(1040.0, 560.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let ctx = cx.ctx.clone();
        let base_id = ui.id().with(("object", self.idx));
        // ---------------- modals
        if let Some(mode) = self.selector {
            match event_selector(&ctx, base_id.with("selector"), cx.project) {
                Selection::Chosen(t, n) => {
                    self.selector = None;
                    if let Some(i) = self
                        .work
                        .events
                        .iter()
                        .position(|e| e.event_type == t && e.number == n)
                    {
                        if mode == SelectorMode::Add {
                            self.event = Some(i);
                        } else {
                            cx.requests
                                .push(Request::Message("That event already exists.".into()));
                        }
                    } else if mode == SelectorMode::Add {
                        let i = insert_event(
                            &mut self.work.events,
                            Event {
                                event_type: t,
                                number: n,
                                actions: Vec::new(),
                            },
                        );
                        self.event = Some(i);
                        self.actions.selected = None;
                    } else if let Some(cur) = self.event {
                        let mut ev = self.work.events.remove(cur);
                        ev.event_type = t;
                        ev.number = n;
                        self.event = Some(insert_event(&mut self.work.events, ev));
                    }
                }
                Selection::Cancel => self.selector = None,
                Selection::None => {}
            }
        }
        if let Some(e) = self.event.and_then(|i| self.work.events.get_mut(i)) {
            arg_dialog(
                &ctx,
                base_id.with("args"),
                &mut e.actions,
                &mut self.actions,
                cx,
            );
        }
        if self.confirm_delete {
            egui::Modal::new(base_id.with("confirm")).show(&ctx, |ui| {
                ui.label("Delete this event and all its actions?");
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        if let Some(i) = self.event {
                            self.work.events.remove(i);
                            self.event = if self.work.events.is_empty() {
                                None
                            } else {
                                Some(i.min(self.work.events.len() - 1))
                            };
                        }
                        self.confirm_delete = false;
                    }
                    if ui.button("No").clicked() {
                        self.confirm_delete = false;
                    }
                });
            });
        }

        let h = ui.available_height();
        ui.horizontal_top(|ui| {
            // ---------------- properties
            ui.vertical(|ui| {
                ui.set_width(200.0);
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.work.name);
                ui.label("Sprite");
                ui.horizontal(|ui| {
                    resource_picker(
                        ui,
                        "obj_sprite",
                        cx.project,
                        ResourceKind::Sprite,
                        &mut self.work.sprite,
                        "<no sprite>",
                        -1,
                    );
                    if ui
                        .add_enabled(self.work.sprite >= 0, egui::Button::new("Edit"))
                        .clicked()
                    {
                        cx.requests.push(Request::Open(ResKey::Res(
                            ResourceKind::Sprite,
                            self.work.sprite as usize,
                        )));
                    }
                });
                if let Some((t, size, _)) = cx.sprite_tex(self.work.sprite, 0) {
                    image_preview(
                        ui,
                        Some(t),
                        size.x as u32,
                        size.y as u32,
                        Vec2::splat(64.0),
                        2.0,
                        Sense::hover(),
                    );
                }
                ui.checkbox(&mut self.work.visible, "Visible");
                ui.checkbox(&mut self.work.solid, "Solid");
                ui.horizontal(|ui| {
                    ui.label("Depth:");
                    ui.add(egui::DragValue::new(&mut self.work.depth));
                });
                ui.checkbox(&mut self.work.persistent, "Persistent");
                ui.label("Parent:");
                let mut parent = self.work.parent;
                if resource_picker(
                    ui,
                    "obj_parent",
                    cx.project,
                    ResourceKind::Object,
                    &mut parent,
                    "<no parent>",
                    NO_PARENT,
                ) {
                    if parent >= 0 && parent_cycle(cx.project, self.idx, parent) {
                        cx.requests
                            .push(Request::Message("This parent would create a cycle.".into()));
                    } else {
                        self.work.parent = parent;
                    }
                }
                ui.label("Mask:");
                resource_picker(
                    ui,
                    "obj_mask",
                    cx.project,
                    ResourceKind::Sprite,
                    &mut self.work.mask_sprite,
                    "<same as sprite>",
                    -1,
                );
            });
            ui.separator();
            // ---------------- events
            ui.vertical(|ui| {
                ui.set_width(180.0);
                ui.label("Events:");
                egui::ScrollArea::vertical()
                    .id_salt("events")
                    .max_height(h - 60.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for (i, e) in self.work.events.iter().enumerate() {
                            let name = event_name(cx.project, e.event_type, e.number);
                            if ui.selectable_label(self.event == Some(i), name).clicked()
                                && self.event != Some(i)
                            {
                                self.event = Some(i);
                                self.actions.selected = None;
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.button("Add Event").clicked() {
                        self.selector = Some(SelectorMode::Add);
                    }
                    if ui
                        .add_enabled(self.event.is_some(), egui::Button::new("Delete"))
                        .clicked()
                    {
                        let has_actions = self
                            .event
                            .and_then(|i| self.work.events.get(i))
                            .is_some_and(|e| !e.actions.is_empty());
                        if has_actions {
                            self.confirm_delete = true;
                        } else if let Some(i) = self.event {
                            self.work.events.remove(i);
                            self.event = None;
                        }
                    }
                    if ui
                        .add_enabled(self.event.is_some(), egui::Button::new("Change"))
                        .clicked()
                    {
                        self.selector = Some(SelectorMode::Change);
                    }
                });
            });
            ui.separator();
            // ---------------- actions
            let act_w = (ui.available_width() - 240.0).max(200.0);
            ui.vertical(|ui| {
                ui.set_width(act_w);
                ui.set_height(h);
                ui.label("Actions:");
                match self.event.and_then(|i| self.work.events.get_mut(i)) {
                    Some(e) => action_list(ui, &mut e.actions, &mut self.actions, cx),
                    None => {
                        ui.label(egui::RichText::new("Add or select an event first.").weak());
                    }
                }
            });
            ui.separator();
            // ---------------- palette
            ui.vertical(|ui| {
                ui.set_width(220.0);
                match self.event.and_then(|i| self.work.events.get_mut(i)) {
                    Some(e) => palette(ui, &mut e.actions, &mut self.actions),
                    None => {
                        let mut dummy = Vec::new();
                        ui.add_enabled_ui(false, |ui| palette(ui, &mut dummy, &mut self.actions));
                    }
                }
            });
        });
    }
    fn on_renamed(&mut self, name: &str) {
        self.work.name = name.to_string();
    }
    fn is_dirty(&self, p: &Project) -> bool {
        slot_dirty(p.objects.get(self.idx), &self.normalized(p))
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        let n = self.normalized(p);
        commit_slot(p.objects.get_mut(self.idx), &n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_insert_sorted() {
        let mut ev = Vec::new();
        let mk = |t, n| Event {
            event_type: t,
            number: n,
            actions: vec![],
        };
        insert_event(&mut ev, mk(3, 0));
        insert_event(&mut ev, mk(0, 0));
        insert_event(&mut ev, mk(2, 5));
        insert_event(&mut ev, mk(2, 1));
        let k: Vec<(i32, i32)> = ev.iter().map(|e| (e.event_type, e.number)).collect();
        assert_eq!(k, vec![(0, 0), (2, 1), (2, 5), (3, 0)]);
    }

    #[test]
    fn cycles() {
        let mut p = Project::default();
        let a = Object {
            parent: 1,
            ..Default::default()
        };
        p.objects = vec![Some(a), Some(Object::default())];
        // object 1 may not get parent 0 (0's parent is 1)
        assert!(parent_cycle(&p, 1, 0));
        assert!(!parent_cycle(&p, 0, 1));
        assert!(parent_cycle(&p, 0, 0));
    }
}
