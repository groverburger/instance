//! Object form (specs/ide.md §13).

use eframe::egui::{self, pos2, Color32, Rect, Sense, Ui, UiBuilder, Vec2};
use instance_format::{
    action_kind as ak, event_type as et, Event, Object, Project, ResourceKind, NO_PARENT,
};

use super::{commit_slot, slot_dirty, Cx, Editor, Request, ResKey};
use crate::events::event_name;
use crate::project_ops::res_name;
use crate::theme;
use crate::widgets::actions::{
    action_label, action_list, arg_dialog, indent_levels, list_box, palette, ActionEditState,
};
use crate::widgets::event_selector::{event_selector, Selection};
use crate::widgets::{checker, group_box, resource_menu_field};

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
    /// The "Show Information" window is open.
    info_open: bool,
    /// The form's OK button was pressed (see [`Editor::take_ok`]).
    ok_pressed: bool,
    /// "New" sprite was requested: the slot the new sprite will get.
    pending_sprite: Option<usize>,
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
            info_open: false,
            ok_pressed: false,
            pending_sprite: None,
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

/// Content size of the form (GM 8.1's object form is about this big).
const FORM_W: f32 = 1090.0;
const FORM_H: f32 = 540.0;
/// Column widths: properties, events, palette (the action list takes the rest).
const PROPS_W: f32 = 182.0;
const EVENTS_W: f32 = 200.0;
const PALETTE_W: f32 = 162.0;
const GAP: f32 = 10.0;
/// Event list row height and icon size.
const EVENT_ROW_H: f32 = 22.0;
const EVENT_ICON: f32 = 16.0;

impl ObjectEditor {
    /// Lay out the four columns in the space the window gives the form.
    fn layout(&mut self, ui: &mut Ui, cx: &mut Cx) {
        // a "New" sprite created in the previous frame becomes this object's sprite
        if let Some(slot) = self.pending_sprite {
            if cx.project.sprites.get(slot).is_some_and(Option::is_some) {
                self.work.sprite = slot as i32;
                self.pending_sprite = None;
            }
        }
        let full = ui.available_rect_before_wrap();
        ui.allocate_rect(full, Sense::hover());
        let h = full.height();
        let act_w = (full.width() - PROPS_W - EVENTS_W - PALETTE_W - 3.0 * GAP).max(160.0);
        let mut x = full.left();
        let mut col = |w: f32| {
            let r = Rect::from_min_size(pos2(x, full.top()), Vec2::new(w, h));
            x += w + GAP;
            r
        };
        let (props, events, acts, pal) = (col(PROPS_W), col(EVENTS_W), col(act_w), col(PALETTE_W));
        ui.scope_builder(UiBuilder::new().max_rect(props), |ui| {
            self.properties(ui, cx)
        });
        ui.scope_builder(UiBuilder::new().max_rect(events), |ui| {
            self.events_column(ui, cx)
        });
        ui.scope_builder(UiBuilder::new().max_rect(acts), |ui| {
            ui.label("Actions:");
            match self.event.and_then(|i| self.work.events.get_mut(i)) {
                Some(e) => action_list(ui, &mut e.actions, &mut self.actions, cx),
                None => {
                    let (r, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
                    list_box(ui.painter(), r);
                }
            }
        });
        ui.scope_builder(UiBuilder::new().max_rect(pal), |ui| {
            match self.event.and_then(|i| self.work.events.get_mut(i)) {
                Some(e) => palette(ui, &mut e.actions, &mut self.actions),
                None => {
                    let mut dummy = Vec::new();
                    ui.add_enabled_ui(false, |ui| palette(ui, &mut dummy, &mut self.actions));
                }
            }
        });
    }

    /// Left column: name, sprite, flags, depth, parent, mask, information and OK.
    fn properties(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let w = ui.available_width();
        ui.spacing_mut().item_spacing.y = 6.0;
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(
                egui::TextEdit::singleline(&mut self.work.name).desired_width(ui.available_width()),
            );
        });
        group_box(ui, "Sprite", w, |ui| {
            ui.horizontal_top(|ui| {
                // thumbnail
                let (r, _) = ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
                sprite_thumb(ui, cx, self.work.sprite, r);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let fw = ui.available_width();
                    resource_menu_field(
                        ui,
                        "obj_sprite",
                        cx.project,
                        ResourceKind::Sprite,
                        &mut self.work.sprite,
                        "<no sprite>",
                        -1,
                        fw,
                    );
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let bw = (fw - 6.0) / 2.0;
                        if ui
                            .add(egui::Button::new("New").min_size(Vec2::new(bw, 20.0)))
                            .on_hover_text("Create a new sprite for this object")
                            .clicked()
                        {
                            self.pending_sprite = Some(cx.project.sprites.len());
                            cx.requests.push(Request::Create(ResourceKind::Sprite));
                        }
                        if ui
                            .add_enabled(
                                self.work.sprite >= 0,
                                egui::Button::new("Edit").min_size(Vec2::new(bw, 20.0)),
                            )
                            .clicked()
                        {
                            cx.requests.push(Request::Open(ResKey::Res(
                                ResourceKind::Sprite,
                                self.work.sprite as usize,
                            )));
                        }
                    });
                });
            });
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 18.0;
            ui.checkbox(&mut self.work.visible, "Visible");
            ui.checkbox(&mut self.work.solid, "Solid");
        });
        ui.horizontal(|ui| {
            ui.label("Depth:");
            ui.add_sized(
                Vec2::new(ui.available_width(), 20.0),
                egui::DragValue::new(&mut self.work.depth),
            );
        });
        ui.checkbox(&mut self.work.persistent, "Persistent");
        let lw = 46.0;
        ui.horizontal(|ui| {
            ui.add_sized(Vec2::new(lw, 20.0), egui::Label::new("Parent:"));
            let mut parent = self.work.parent;
            let fw = ui.available_width();
            if resource_menu_field(
                ui,
                "obj_parent",
                cx.project,
                ResourceKind::Object,
                &mut parent,
                "<no parent>",
                NO_PARENT,
                fw,
            ) {
                if parent >= 0 && parent_cycle(cx.project, self.idx, parent) {
                    cx.requests
                        .push(Request::Message("This parent would create a cycle.".into()));
                } else {
                    self.work.parent = parent;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.add_sized(Vec2::new(lw, 20.0), egui::Label::new("Mask:"));
            let fw = ui.available_width();
            resource_menu_field(
                ui,
                "obj_mask",
                cx.project,
                ResourceKind::Sprite,
                &mut self.work.mask_sprite,
                "<same as sprite>",
                -1,
                fw,
            );
        });
        ui.add_space(4.0);
        if icon_button(ui, "Show Information", Vec2::new(w, 26.0), paint_info_icon).clicked() {
            self.info_open = true;
        }
        let ok_h = 34.0;
        let rest = ui.available_height() - ok_h;
        if rest > 0.0 {
            ui.add_space(rest);
        }
        if icon_button(ui, "✔ OK", Vec2::new(w, ok_h), |_, _| {}).clicked() {
            self.ok_pressed = true;
        }
    }

    /// Second column: the event list and its buttons.
    fn events_column(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let w = ui.available_width();
        ui.label("Events:");
        let buttons_h = 2.0 * 24.0 + 2.0 * 6.0;
        let list_h = (ui.available_height() - buttons_h).max(EVENT_ROW_H * 2.0);
        let (outer, _) = ui.allocate_exact_size(Vec2::new(w, list_h), Sense::hover());
        list_box(ui.painter(), outer);
        let mut open_menu_at: Option<usize> = None;
        let mut menu_cmd: Option<SelectorMode> = None;
        let mut delete = false;
        ui.scope_builder(UiBuilder::new().max_rect(outer.shrink(2.0)), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("events")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let rw = ui.available_width();
                    for (i, e) in self.work.events.iter().enumerate() {
                        let (rect, r) =
                            ui.allocate_exact_size(Vec2::new(rw, EVENT_ROW_H), Sense::click());
                        let selected = self.event == Some(i);
                        let name = event_name(cx.project, e.event_type, e.number);
                        r.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::SelectableLabel,
                                true,
                                selected,
                                &name,
                            )
                        });
                        if ui.is_rect_visible(rect) {
                            paint_event_row(ui, cx, rect, e, selected, r.hovered());
                        }
                        if (r.clicked() || r.secondary_clicked()) && self.event != Some(i) {
                            self.event = Some(i);
                            self.actions.selected = None;
                        }
                        r.context_menu(|ui| {
                            open_menu_at = Some(i);
                            if ui.button("Add Event…").clicked() {
                                menu_cmd = Some(SelectorMode::Add);
                                ui.close();
                            }
                            if ui.button("Change Event…").clicked() {
                                menu_cmd = Some(SelectorMode::Change);
                                ui.close();
                            }
                            if ui.button("Delete Event").clicked() {
                                delete = true;
                                ui.close();
                            }
                        });
                    }
                    let rest = ui.available_size().max(Vec2::new(rw, EVENT_ROW_H));
                    let (_, r) = ui.allocate_exact_size(rest, Sense::click());
                    r.context_menu(|ui| {
                        if ui.button("Add Event…").clicked() {
                            menu_cmd = Some(SelectorMode::Add);
                            ui.close();
                        }
                    });
                });
        });
        if let Some(i) = open_menu_at {
            if self.event != Some(i) {
                self.event = Some(i);
                self.actions.selected = None;
            }
        }
        ui.add_space(6.0);
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        if ui
            .add(egui::Button::new("Add Event").min_size(Vec2::new(w, 24.0)))
            .clicked()
        {
            menu_cmd = Some(SelectorMode::Add);
        }
        ui.horizontal(|ui| {
            let bw = (w - 6.0) / 2.0;
            if ui
                .add_enabled(
                    self.event.is_some(),
                    egui::Button::new("Delete").min_size(Vec2::new(bw, 24.0)),
                )
                .clicked()
            {
                delete = true;
            }
            if ui
                .add_enabled(
                    self.event.is_some(),
                    egui::Button::new("Change").min_size(Vec2::new(bw, 24.0)),
                )
                .clicked()
            {
                menu_cmd = Some(SelectorMode::Change);
            }
        });
        if let Some(m) = menu_cmd {
            if m == SelectorMode::Add || self.event.is_some() {
                self.selector = Some(m);
            }
        }
        if delete {
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
    }
}

/// Paint the sprite of `sprite` scaled into `r` over a checkerboard, in a sunken frame.
fn sprite_thumb(ui: &Ui, cx: &mut Cx, sprite: i32, r: Rect) {
    let p = ui.painter();
    p.rect_filled(r, 0.0, Color32::WHITE);
    let inner = r.shrink(2.0);
    if let Some((t, size, _)) = cx.sprite_tex(sprite, 0) {
        let s = (inner.width() / size.x)
            .min(inner.height() / size.y)
            .min(4.0);
        let img = Rect::from_center_size(inner.center(), size * s);
        checker(p, img, 4.0);
        p.image(
            t,
            img,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    p.rect_stroke(
        r,
        0.0,
        egui::Stroke::new(1.0, Color32::from_gray(130)),
        egui::StrokeKind::Inside,
    );
}

/// One row of the event list: icon, (for collisions) the other object's sprite, name.
fn paint_event_row(ui: &Ui, cx: &mut Cx, rect: Rect, e: &Event, selected: bool, hovered: bool) {
    let icon = Rect::from_min_size(
        pos2(rect.left() + 3.0, rect.center().y - EVENT_ICON / 2.0),
        Vec2::splat(EVENT_ICON),
    );
    crate::icons::actions::paint_event(ui.painter(), icon, e.event_type, e.number);
    let mut x = icon.right() + 5.0;
    let label = if e.event_type == et::COLLISION {
        let other = usize::try_from(e.number)
            .ok()
            .and_then(|i| cx.project.objects.get(i))
            .and_then(|o| o.as_ref());
        let spr = other.map_or(-1, |o| o.sprite);
        let thumb = Rect::from_min_size(
            pos2(x, rect.center().y - EVENT_ICON / 2.0),
            Vec2::splat(EVENT_ICON),
        );
        if let Some((t, size, _)) = cx.sprite_tex(spr, 0) {
            let s = (EVENT_ICON / size.x).min(EVENT_ICON / size.y);
            let img = Rect::from_center_size(thumb.center(), size * s);
            ui.painter().image(
                t,
                img,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        x = thumb.right() + 5.0;
        other.map_or_else(|| format!("<object {}>", e.number), |o| o.name.clone())
    } else {
        event_name(cx.project, e.event_type, e.number)
    };
    let galley = egui::WidgetText::from(label).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        rect.right() - x - 4.0,
        egui::TextStyle::Body,
    );
    let tr = Rect::from_min_size(
        pos2(x - 3.0, rect.center().y - galley.size().y / 2.0 - 1.0),
        galley.size() + Vec2::new(6.0, 2.0),
    );
    if selected {
        ui.painter().rect_filled(tr, 0.0, theme::SELECTION);
    } else if hovered {
        ui.painter()
            .rect_filled(tr, 0.0, Color32::from_rgb(229, 241, 251));
    }
    let color = if selected {
        Color32::WHITE
    } else {
        Color32::from_gray(10)
    };
    ui.painter().galley(
        pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}

/// A button of fixed `size` with an icon painted left of its centred text. The text
/// "✔ …" gets a green check mark.
fn icon_button(
    ui: &mut Ui,
    text: &str,
    size: Vec2,
    icon: impl FnOnce(&egui::Painter, Rect),
) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text));
    if ui.is_rect_visible(rect) {
        let v = ui.style().interact(&r);
        let p = ui.painter();
        p.rect(
            rect,
            v.corner_radius,
            v.weak_bg_fill,
            v.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let font = egui::TextStyle::Button.resolve(ui.style());
        let mut job = egui::text::LayoutJob::default();
        let body = match text.strip_prefix("✔") {
            Some(rest) => {
                let big = egui::FontId::proportional(font.size + 4.0);
                job.append(
                    "✔",
                    0.0,
                    egui::TextFormat::simple(big, Color32::from_rgb(20, 150, 40)),
                );
                rest
            }
            None => text,
        };
        job.append(
            body,
            0.0,
            egui::TextFormat::simple(font, Color32::from_gray(10)),
        );
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let icon_w = 18.0;
        let has_icon = !text.starts_with('✔');
        let total = galley.size().x + if has_icon { icon_w + 4.0 } else { 0.0 };
        let x0 = rect.center().x - total / 2.0;
        if has_icon {
            icon(
                p,
                Rect::from_min_size(
                    pos2(x0, rect.center().y - icon_w / 2.0),
                    Vec2::splat(icon_w),
                ),
            );
        }
        let tx = x0 + if has_icon { icon_w + 4.0 } else { 0.0 };
        p.galley(
            pos2(tx, rect.center().y - galley.size().y / 2.0),
            galley,
            Color32::from_gray(10),
        );
    }
    r
}

/// A blue "i" information disc.
fn paint_info_icon(p: &egui::Painter, r: Rect) {
    let c = r.center();
    let rad = r.width() / 2.0 - 1.0;
    p.circle(
        c,
        rad,
        Color32::from_rgb(40, 100, 210),
        egui::Stroke::new(1.0, Color32::from_rgb(20, 50, 120)),
    );
    p.circle_filled(c + Vec2::new(0.0, -rad * 0.5), rad * 0.16, Color32::WHITE);
    p.rect_filled(
        Rect::from_center_size(
            c + Vec2::new(0.0, rad * 0.2),
            Vec2::new(rad * 0.3, rad * 0.8),
        ),
        0.0,
        Color32::WHITE,
    );
}

/// The text of "Show Information": the object's properties and all its events with
/// their actions (code is listed in full).
pub fn object_info(p: &Project, o: &Object) -> String {
    let name = |k: ResourceKind, i: i32, none: &str| {
        usize::try_from(i)
            .ok()
            .and_then(|i| res_name(p, k, i))
            .unwrap_or(none)
            .to_string()
    };
    let mut s = format!("Information about object: {}\n\n", o.name);
    s += &format!(
        "Sprite: {}\n",
        name(ResourceKind::Sprite, o.sprite, "<no sprite>")
    );
    s += &format!("Solid: {}\n", o.solid);
    s += &format!("Visible: {}\n", o.visible);
    s += &format!("Depth: {}\n", o.depth);
    s += &format!("Persistent: {}\n", o.persistent);
    s += &format!(
        "Parent: {}\n",
        name(ResourceKind::Object, o.parent, "<no parent>")
    );
    let children: Vec<&str> = p
        .objects
        .iter()
        .flatten()
        .filter(|c| {
            c.parent >= 0
                && res_name(p, ResourceKind::Object, c.parent as usize) == Some(o.name.as_str())
        })
        .map(|c| c.name.as_str())
        .collect();
    if !children.is_empty() {
        s += &format!("Children:\n{}\n", children.join("\n"));
    }
    s += &format!(
        "Mask: {}\n",
        name(ResourceKind::Sprite, o.mask_sprite, "<same as sprite>")
    );
    for e in &o.events {
        s += &format!("\n{} Event:\n", event_title(p, e));
        let levels = indent_levels(&e.actions);
        for (a, l) in e.actions.iter().zip(levels) {
            let pad = "    ".repeat(l);
            if a.kind == ak::CODE {
                s += &format!("{pad}execute code:\n\n");
                for line in a.args.first().map_or("", String::as_str).lines() {
                    s += &format!("{pad}{line}\n");
                }
                s += "\n";
            } else {
                s += &format!("{pad}{}\n", action_label(a, p).0);
            }
        }
    }
    s
}

fn event_title(p: &Project, e: &Event) -> String {
    match e.event_type {
        et::KEYBOARD => format!(
            "Keyboard Event for {}",
            event_name(p, e.event_type, e.number)
        ),
        et::KEY_PRESS => format!(
            "Key Press Event for {}",
            &event_name(p, e.event_type, e.number)[6..]
        ),
        et::KEY_RELEASE => format!(
            "Key Release Event for {}",
            &event_name(p, e.event_type, e.number)[8..]
        ),
        et::MOUSE => format!("Mouse Event for {}", event_name(p, e.event_type, e.number)),
        et::OTHER => format!("Other Event: {}", event_name(p, e.event_type, e.number)),
        et::TRIGGER => format!("Trigger Event: {}", event_name(p, e.event_type, e.number)),
        _ => event_name(p, e.event_type, e.number),
    }
}

impl Editor for ObjectEditor {
    fn key(&self) -> ResKey {
        ResKey::Res(ResourceKind::Object, self.idx)
    }
    fn title(&self) -> String {
        format!("Object Properties: {}", self.work.name)
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(FORM_W, FORM_H)
    }
    fn has_own_ok(&self) -> bool {
        true
    }
    fn take_ok(&mut self) -> bool {
        std::mem::take(&mut self.ok_pressed)
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
            let d = crate::mdi::dialog(&ctx, base_id.with("confirm"), "Confirm", |ui| {
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
            if d.close {
                self.confirm_delete = false;
            }
        }

        if self.info_open {
            let text = object_info(cx.project, &self.work);
            let d = crate::mdi::dialog(
                &ctx,
                base_id.with("info"),
                &format!("Information about {}", self.work.name),
                |ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::new(560.0, 400.0), Sense::hover());
                    list_box(ui.painter(), r);
                    ui.scope_builder(UiBuilder::new().max_rect(r.shrink(3.0)), |ui| {
                        egui::ScrollArea::both()
                            .id_salt("info_text")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut text.as_str())
                                        .font(egui::TextStyle::Monospace)
                                        .frame(egui::Frame::NONE)
                                        .desired_width(f32::INFINITY),
                                );
                            });
                    });
                    ui.add_space(4.0);
                    ui.add(egui::Button::new("OK").min_size(Vec2::new(80.0, 22.0)))
                        .clicked()
                },
            );
            if d.close || d.inner {
                self.info_open = false;
            }
        }

        self.layout(ui, cx);
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
