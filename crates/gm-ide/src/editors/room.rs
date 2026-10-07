//! Room form (specs/ide.md §15): objects, settings, tiles, backgrounds and views tabs
//! and the room canvas.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use gm_format::{Instance, Project, ResourceKind, Room, Tile};

use super::{commit_slot, slot_dirty, Cx, Editor, Request, ResKey};
use crate::widgets::code_editor::{code_editor, CodeEditorState};
use crate::widgets::{
    bgr_to_color, color_box, image_preview, resource_picker, show_prompt, Prompt, PromptResult,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Objects,
    Settings,
    Tiles,
    Backgrounds,
    Views,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CodeTarget {
    Room,
    Instance(usize),
}

#[derive(Clone, Copy)]
enum Drag {
    /// Moving instance/tile `i`, with the grab offset in room units.
    MoveInstance(usize, Vec2),
    MoveTile(usize, Vec2),
    /// Shift-painting: last cell painted.
    Paint(i32, i32),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PromptOp {
    ShiftAll,
    Position(usize),
    AddLayer,
}

pub struct RoomEditor {
    idx: usize,
    work: Room,
    tab: Tab,
    obj_to_add: i32,
    zoom: f32,
    offset: Vec2,
    show_grid: bool,
    delete_underlying: bool,
    drag: Option<Drag>,
    ctx_target: Option<usize>,
    bg_sel: usize,
    view_sel: usize,
    code: Option<(CodeTarget, String, CodeEditorState)>,
    prompt: Option<(PromptOp, Prompt)>,
    tileset: i32,
    tile_sel: (i32, i32, i32, i32),
    layer: i32,
    extra_layers: Vec<i32>,
    undo: Vec<(Vec<Instance>, Vec<Tile>)>,
    status: String,
}

/// Snap a room coordinate to the grid.
pub fn snap(v: f32, grid: i32) -> i32 {
    let g = grid.max(1) as f32;
    (v / g).floor() as i32 * grid.max(1)
}

impl RoomEditor {
    pub fn new(idx: usize, work: Room, p: &Project) -> Self {
        let obj_to_add = crate::project_ops::tree_order(p, ResourceKind::Object)
            .into_iter()
            .find(|x| x.0 >= 0)
            .map_or(-1, |x| x.0);
        let layer = work.tiles.first().map_or(1_000_000, |t| t.depth);
        RoomEditor {
            idx,
            work,
            tab: Tab::Objects,
            obj_to_add,
            zoom: 1.0,
            offset: Vec2::new(8.0, 8.0),
            show_grid: true,
            delete_underlying: false,
            drag: None,
            ctx_target: None,
            bg_sel: 0,
            view_sel: 0,
            code: None,
            prompt: None,
            tileset: -1,
            tile_sel: (0, 0, 16, 16),
            layer,
            extra_layers: Vec::new(),
            undo: Vec::new(),
            status: String::new(),
        }
    }

    fn snapshot(&mut self) {
        self.undo
            .push((self.work.instances.clone(), self.work.tiles.clone()));
        if self.undo.len() > 32 {
            self.undo.remove(0);
        }
    }

    fn layers(&self) -> Vec<i32> {
        let mut v: Vec<i32> = self.work.tiles.iter().map(|t| t.depth).collect();
        v.extend(&self.extra_layers);
        v.push(self.layer);
        v.sort_unstable_by(|a, b| b.cmp(a));
        v.dedup();
        v
    }

    /// Screen rect of instance `i` in room coordinates (sprite box or 16x16 placeholder).
    fn inst_box(cx: &mut Cx, inst: &Instance) -> (Rect, Option<egui::TextureId>) {
        let spr = cx
            .project
            .objects
            .get(inst.object as usize)
            .and_then(|o| o.as_ref())
            .map_or(-1, |o| o.sprite);
        match cx.sprite_tex(spr, 0) {
            Some((t, size, org)) => (
                Rect::from_min_size(
                    Pos2::new(inst.x as f32 - org.x, inst.y as f32 - org.y),
                    size,
                ),
                Some(t),
            ),
            None => (
                Rect::from_min_size(Pos2::new(inst.x as f32, inst.y as f32), Vec2::splat(16.0)),
                None,
            ),
        }
    }

    fn instance_at(&self, cx: &mut Cx, w: Pos2) -> Option<usize> {
        // Topmost = last drawn = last in list among smallest depth.
        let mut best: Option<(usize, i32)> = None;
        for (i, inst) in self.work.instances.iter().enumerate() {
            let (r, _) = Self::inst_box(cx, inst);
            if r.contains(w) {
                let d = obj_depth(cx.project, inst.object);
                if best.is_none_or(|(_, bd)| d <= bd) {
                    best = Some((i, d));
                }
            }
        }
        best.map(|b| b.0)
    }

    fn tile_at(&self, w: Pos2) -> Option<usize> {
        self.work
            .tiles
            .iter()
            .enumerate()
            .rev()
            .find(|(_, t)| {
                t.depth == self.layer
                    && Rect::from_min_size(
                        Pos2::new(t.x as f32, t.y as f32),
                        Vec2::new(t.width as f32, t.height as f32),
                    )
                    .contains(w)
            })
            .map(|x| x.0)
    }

    fn add_instance(&mut self, cx: &mut Cx, x: i32, y: i32) -> Option<usize> {
        if self.obj_to_add < 0 {
            self.status = "Select an object to add first.".into();
            return None;
        }
        if self.delete_underlying {
            self.work
                .instances
                .retain(|i| i.locked || i.x != x || i.y != y);
        }
        let id = cx.alloc_instance_id();
        self.work.instances.push(Instance {
            x,
            y,
            object: self.obj_to_add,
            id,
            creation_code: String::new(),
            locked: false,
        });
        Some(self.work.instances.len() - 1)
    }

    fn add_tile(&mut self, cx: &mut Cx, x: i32, y: i32) {
        if self.tileset < 0 {
            self.status = "Select a tile set first.".into();
            return;
        }
        let (tx, ty, w, h) = self.tile_sel;
        if self.delete_underlying {
            let layer = self.layer;
            self.work
                .tiles
                .retain(|t| t.locked || t.depth != layer || t.x != x || t.y != y);
        }
        let id = cx.alloc_tile_id();
        self.work.tiles.push(Tile {
            x,
            y,
            background: self.tileset,
            tile_x: tx,
            tile_y: ty,
            width: w,
            height: h,
            depth: self.layer,
            id,
            locked: false,
        });
    }

    fn draw_background_layer(
        &self,
        cx: &mut Cx,
        painter: &egui::Painter,
        to_screen: &dyn Fn(f32, f32) -> Pos2,
        foreground: bool,
    ) {
        let room_rect = Rect::from_min_max(
            to_screen(0.0, 0.0),
            to_screen(self.work.width as f32, self.work.height as f32),
        );
        for b in &self.work.backgrounds {
            if !b.visible || b.is_foreground != foreground {
                continue;
            }
            let Some((t, size)) = cx.background_tex(b.background) else {
                continue;
            };
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            if b.stretch {
                painter.image(t, room_rect, uv, Color32::WHITE);
                continue;
            }
            let (w, h) = (size.x.max(1.0), size.y.max(1.0));
            let p = painter.with_clip_rect(room_rect.intersect(painter.clip_rect()));
            let xs: Vec<f32> = if b.tile_h {
                let start = (b.x as f32).rem_euclid(w) - w;
                (0..)
                    .map(|k| start + k as f32 * w)
                    .take_while(|x| *x < self.work.width as f32)
                    .collect()
            } else {
                vec![b.x as f32]
            };
            let ys: Vec<f32> = if b.tile_v {
                let start = (b.y as f32).rem_euclid(h) - h;
                (0..)
                    .map(|k| start + k as f32 * h)
                    .take_while(|y| *y < self.work.height as f32)
                    .collect()
            } else {
                vec![b.y as f32]
            };
            if xs.len() * ys.len() > 20_000 {
                continue;
            }
            for &y in &ys {
                for &x in &xs {
                    let min = to_screen(x, y);
                    let max = to_screen(x + w, y + h);
                    p.image(t, Rect::from_min_max(min, max), uv, Color32::WHITE);
                }
            }
        }
    }

    fn canvas(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let size = ui.available_size().max(Vec2::splat(100.0));
        let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "room canvas")
        });
        let rect = resp.rect;
        let painter = painter.with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, Color32::from_gray(64));
        let z = self.zoom;
        let origin = rect.min + self.offset;
        let to_screen = |x: f32, y: f32| origin + Vec2::new(x, y) * z;
        let to_world = |p: Pos2| Pos2::ZERO + (p - origin) / z;
        let room_rect = Rect::from_min_max(
            to_screen(0.0, 0.0),
            to_screen(self.work.width as f32, self.work.height as f32),
        );
        // background colour
        let bg = if self.work.draw_background_color {
            bgr_to_color(self.work.background_color)
        } else {
            Color32::from_gray(160)
        };
        painter.rect_filled(room_rect, 0.0, bg);
        self.draw_background_layer(cx, &painter, &to_screen, false);
        // tiles and instances sorted by depth (deepest first)
        enum Item {
            Tile(usize),
            Inst(usize),
        }
        let mut items: Vec<(i32, usize, Item)> = Vec::new();
        for (i, t) in self.work.tiles.iter().enumerate() {
            items.push((t.depth, i, Item::Tile(i)));
        }
        for (i, inst) in self.work.instances.iter().enumerate() {
            items.push((obj_depth(cx.project, inst.object), i, Item::Inst(i)));
        }
        items.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let uv1 = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        for (_, _, it) in items {
            match it {
                Item::Tile(i) => {
                    let t = &self.work.tiles[i];
                    if let Some((tex, size)) = cx.background_tex(t.background) {
                        let uv = Rect::from_min_size(
                            Pos2::new(t.tile_x as f32 / size.x, t.tile_y as f32 / size.y),
                            Vec2::new(t.width as f32 / size.x, t.height as f32 / size.y),
                        );
                        let r = Rect::from_min_max(
                            to_screen(t.x as f32, t.y as f32),
                            to_screen((t.x + t.width) as f32, (t.y + t.height) as f32),
                        );
                        let tint = if self.tab == Tab::Tiles && t.depth != self.layer {
                            Color32::from_white_alpha(110)
                        } else {
                            Color32::WHITE
                        };
                        painter.image(tex, r, uv, tint);
                    }
                }
                Item::Inst(i) => {
                    let inst = self.work.instances[i].clone();
                    let (b, tex) = Self::inst_box(cx, &inst);
                    let r = Rect::from_min_max(
                        to_screen(b.min.x, b.min.y),
                        to_screen(b.max.x, b.max.y),
                    );
                    match tex {
                        Some(t) => {
                            painter.image(t, r, uv1, Color32::WHITE);
                        }
                        None => {
                            painter.circle_filled(
                                r.center(),
                                r.width() / 2.0,
                                Color32::from_rgb(40, 90, 220),
                            );
                            painter.text(
                                r.center(),
                                egui::Align2::CENTER_CENTER,
                                "?",
                                egui::FontId::proportional(12.0 * z),
                                Color32::WHITE,
                            );
                        }
                    }
                    if inst.locked {
                        painter.rect_stroke(
                            r,
                            0.0,
                            Stroke::new(1.0, Color32::RED),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }
        }
        self.draw_background_layer(cx, &painter, &to_screen, true);
        // grid
        if self.show_grid {
            let st = Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 70));
            let gx = self.work.snap_x.max(1) as f32;
            let gy = self.work.snap_y.max(1) as f32;
            if gx * z >= 4.0 && gy * z >= 4.0 {
                let mut x = 0.0;
                while x <= self.work.width as f32 {
                    painter.vline(to_screen(x, 0.0).x, room_rect.y_range(), st);
                    x += gx;
                }
                let mut y = 0.0;
                while y <= self.work.height as f32 {
                    painter.hline(room_rect.x_range(), to_screen(0.0, y).y, st);
                    y += gy;
                }
            }
        }
        // views
        if self.tab == Tab::Views && self.work.views_enabled {
            for (i, v) in self.work.views.iter().enumerate() {
                if v.visible {
                    let r = Rect::from_min_max(
                        to_screen(v.view_x as f32, v.view_y as f32),
                        to_screen((v.view_x + v.view_w) as f32, (v.view_y + v.view_h) as f32),
                    );
                    let col = if i == self.view_sel {
                        Color32::YELLOW
                    } else {
                        Color32::WHITE
                    };
                    painter.rect_stroke(r, 0.0, Stroke::new(1.5, col), egui::StrokeKind::Inside);
                }
            }
        }
        painter.rect_stroke(
            room_rect,
            0.0,
            Stroke::new(1.0, Color32::BLACK),
            egui::StrokeKind::Outside,
        );

        // ---------------- interaction
        let mods = ui.input(|i| i.modifiers);
        let snapped = |w: Pos2, alt: bool, sx: i32, sy: i32| {
            if alt {
                (w.x.floor() as i32, w.y.floor() as i32)
            } else {
                (snap(w.x, sx), snap(w.y, sy))
            }
        };
        let (sx, sy) = (self.work.snap_x, self.work.snap_y);
        let edit_tab = matches!(self.tab, Tab::Objects | Tab::Tiles);
        let tiles = self.tab == Tab::Tiles;
        if edit_tab && resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let w = to_world(pos);
                let (x, y) = snapped(w, mods.alt, sx, sy);
                self.snapshot();
                if tiles {
                    self.add_tile(cx, x, y);
                } else if !mods.ctrl {
                    self.add_instance(cx, x, y);
                }
            }
        }
        if edit_tab && resp.drag_started_by(egui::PointerButton::Primary) {
            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                let w = to_world(pos);
                let (x, y) = snapped(w, mods.alt, sx, sy);
                self.snapshot();
                if mods.ctrl {
                    if tiles {
                        if let Some(i) = self.tile_at(w).filter(|&i| !self.work.tiles[i].locked) {
                            let t = &self.work.tiles[i];
                            self.drag =
                                Some(Drag::MoveTile(i, w - Pos2::new(t.x as f32, t.y as f32)));
                        }
                    } else if let Some(i) = self
                        .instance_at(cx, w)
                        .filter(|&i| !self.work.instances[i].locked)
                    {
                        let inst = &self.work.instances[i];
                        self.drag = Some(Drag::MoveInstance(
                            i,
                            w - Pos2::new(inst.x as f32, inst.y as f32),
                        ));
                    }
                } else if mods.shift {
                    if tiles {
                        self.add_tile(cx, x, y);
                    } else {
                        self.add_instance(cx, x, y);
                    }
                    self.drag = Some(Drag::Paint(x, y));
                } else if tiles {
                    self.add_tile(cx, x, y);
                    let i = self.work.tiles.len() - 1;
                    if self.tileset >= 0 {
                        self.drag = Some(Drag::MoveTile(i, Vec2::ZERO));
                    }
                } else if let Some(i) = self.add_instance(cx, x, y) {
                    self.drag = Some(Drag::MoveInstance(i, Vec2::ZERO));
                }
            }
        }
        if let (Some(d), Some(pos)) = (self.drag, resp.interact_pointer_pos()) {
            if resp.dragged_by(egui::PointerButton::Primary) {
                let w = to_world(pos);
                match d {
                    Drag::MoveInstance(i, off) => {
                        let (x, y) = snapped(w - off, mods.alt, sx, sy);
                        if let Some(inst) = self.work.instances.get_mut(i) {
                            inst.x = x;
                            inst.y = y;
                        }
                    }
                    Drag::MoveTile(i, off) => {
                        let (x, y) = snapped(w - off, mods.alt, sx, sy);
                        if let Some(t) = self.work.tiles.get_mut(i) {
                            t.x = x;
                            t.y = y;
                        }
                    }
                    Drag::Paint(lx, ly) => {
                        let (x, y) = snapped(w, false, sx, sy);
                        if (x, y) != (lx, ly) {
                            if tiles {
                                self.add_tile(cx, x, y);
                            } else {
                                self.add_instance(cx, x, y);
                            }
                            self.drag = Some(Drag::Paint(x, y));
                        }
                    }
                }
            }
        }
        if resp.drag_stopped() {
            self.drag = None;
        }
        if resp.dragged_by(egui::PointerButton::Middle) {
            self.offset += resp.drag_delta();
        }
        if edit_tab && resp.secondary_clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let w = to_world(pos);
                if tiles {
                    if let Some(i) = self.tile_at(w).filter(|&i| !self.work.tiles[i].locked) {
                        self.snapshot();
                        self.work.tiles.remove(i);
                    }
                    self.ctx_target = None;
                } else if mods.shift {
                    self.snapshot();
                    let mut k = 0;
                    while let Some(i) = self
                        .instance_at(cx, w)
                        .filter(|&i| !self.work.instances[i].locked)
                    {
                        self.work.instances.remove(i);
                        k += 1;
                        if k > 10_000 {
                            break;
                        }
                    }
                    self.ctx_target = None;
                } else if mods.ctrl {
                    if let Some(i) = self
                        .instance_at(cx, w)
                        .filter(|&i| !self.work.instances[i].locked)
                    {
                        self.snapshot();
                        self.work.instances.remove(i);
                    }
                    self.ctx_target = None;
                } else {
                    self.ctx_target = self.instance_at(cx, w);
                }
            }
        }
        if self.tab == Tab::Objects {
            if let Some(i) = self.ctx_target {
                let mut close_target = false;
                resp.context_menu(|ui| {
                    let Some(inst) = self.work.instances.get(i).cloned() else {
                        ui.close();
                        return;
                    };
                    let oname = crate::project_ops::res_name(
                        cx.project,
                        ResourceKind::Object,
                        inst.object as usize,
                    )
                    .unwrap_or("?")
                    .to_string();
                    ui.label(format!("{oname} (id {})", inst.id));
                    ui.separator();
                    if ui
                        .add_enabled(!inst.locked, egui::Button::new("Delete"))
                        .clicked()
                    {
                        self.snapshot();
                        self.work.instances.remove(i);
                        close_target = true;
                        ui.close();
                    }
                    if ui.button("Change position…").clicked() {
                        self.prompt = Some((
                            PromptOp::Position(i),
                            Prompt {
                                title: "Instance position".into(),
                                fields: vec![
                                    ("X:".into(), inst.x.to_string()),
                                    ("Y:".into(), inst.y.to_string()),
                                ],
                            },
                        ));
                        ui.close();
                    }
                    if ui.button("Move to front").clicked() {
                        let it = self.work.instances.remove(i);
                        self.work.instances.push(it);
                        close_target = true;
                        ui.close();
                    }
                    if ui.button("Move to back").clicked() {
                        let it = self.work.instances.remove(i);
                        self.work.instances.insert(0, it);
                        close_target = true;
                        ui.close();
                    }
                    if ui
                        .button(if inst.locked { "Unlock" } else { "Lock" })
                        .clicked()
                    {
                        self.work.instances[i].locked = !inst.locked;
                        ui.close();
                    }
                    if ui.button("Creation code…").clicked() {
                        self.code = Some((
                            CodeTarget::Instance(i),
                            inst.creation_code.clone(),
                            CodeEditorState::default(),
                        ));
                        ui.close();
                    }
                    if ui.button("Edit object").clicked() {
                        cx.requests.push(Request::Open(ResKey::Res(
                            ResourceKind::Object,
                            inst.object as usize,
                        )));
                        ui.close();
                    }
                });
                if close_target {
                    self.ctx_target = None;
                }
            }
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let nz = (self.zoom * if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 }).clamp(0.1, 8.0);
                if let Some(hp) = resp.hover_pos() {
                    let w = to_world(hp);
                    self.zoom = nz;
                    self.offset = hp - rect.min - (w - Pos2::ZERO) * nz;
                }
            }
            if let Some(hp) = resp.hover_pos() {
                let w = to_world(hp);
                let (x, y) = snapped(w, mods.alt, sx, sy);
                let under = self
                    .instance_at(cx, w)
                    .map(|i| {
                        let inst = &self.work.instances[i];
                        format!(
                            "   {} (id {})",
                            crate::project_ops::res_name(
                                cx.project,
                                ResourceKind::Object,
                                inst.object as usize
                            )
                            .unwrap_or("?"),
                            inst.id
                        )
                    })
                    .unwrap_or_default();
                self.status = format!("x: {x}  y: {y}{under}");
            }
        }
    }

    fn tabs_ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Objects, "objects");
            ui.selectable_value(&mut self.tab, Tab::Settings, "settings");
            ui.selectable_value(&mut self.tab, Tab::Tiles, "tiles");
            ui.selectable_value(&mut self.tab, Tab::Backgrounds, "backgrounds");
            ui.selectable_value(&mut self.tab, Tab::Views, "views");
        });
        ui.separator();
        egui::ScrollArea::vertical().id_salt("room_tab").auto_shrink([false, false]).show(ui, |ui| match self.tab {
            Tab::Objects => {
                ui.label("Object to add with left mouse:");
                resource_picker(ui, "room_obj", cx.project, ResourceKind::Object, &mut self.obj_to_add, "<no object>", -1);
                let spr = cx.project.objects.get(self.obj_to_add as usize).and_then(|o| o.as_ref()).map_or(-1, |o| o.sprite);
                if let Some((t, size, _)) = cx.sprite_tex(spr, 0) {
                    image_preview(ui, Some(t), size.x as u32, size.y as u32, Vec2::splat(96.0), 4.0, Sense::hover());
                }
                ui.label(egui::RichText::new("Left: add (+Alt no snap, +Shift paint, +Ctrl move)\nRight: menu (+Shift delete all, +Ctrl delete)").small().weak());
                ui.checkbox(&mut self.delete_underlying, "Delete underlying");
                ui.label(format!("{} instances", self.work.instances.len()));
            }
            Tab::Settings => {
                let r = &mut self.work;
                egui::Grid::new("room_settings").num_columns(2).show(ui, |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut r.name);
                    ui.end_row();
                    ui.label("Caption:");
                    ui.text_edit_singleline(&mut r.caption);
                    ui.end_row();
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut r.width).range(1..=100_000));
                    ui.end_row();
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut r.height).range(1..=100_000));
                    ui.end_row();
                    ui.label("Speed:");
                    ui.add(egui::DragValue::new(&mut r.speed).range(1..=9999));
                    ui.end_row();
                });
                ui.checkbox(&mut r.persistent, "Persistent");
                if ui.button("Creation code…").clicked() {
                    self.code = Some((CodeTarget::Room, r.creation_code.clone(), CodeEditorState::default()));
                }
            }
            Tab::Tiles => self.tiles_tab(ui, cx),
            Tab::Backgrounds => {
                let r = &mut self.work;
                ui.horizontal(|ui| {
                    ui.checkbox(&mut r.draw_background_color, "Draw background color");
                    color_box(ui, &mut r.background_color);
                });
                for i in 0..r.backgrounds.len() {
                    let b = &r.backgrounds[i];
                    let mut t = egui::RichText::new(format!("Background {i}"));
                    if b.visible {
                        t = t.strong();
                    }
                    if ui.selectable_label(self.bg_sel == i, t).clicked() {
                        self.bg_sel = i;
                    }
                }
                if let Some(b) = r.backgrounds.get_mut(self.bg_sel) {
                    ui.separator();
                    ui.checkbox(&mut b.visible, "Visible when room starts");
                    ui.checkbox(&mut b.is_foreground, "Foreground image");
                    resource_picker(ui, "room_bg", cx.project, ResourceKind::Background, &mut b.background, "<no background>", -1);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut b.tile_h, "Tile Hor.");
                        ui.checkbox(&mut b.tile_v, "Tile Vert.");
                        ui.checkbox(&mut b.stretch, "Stretch");
                    });
                    egui::Grid::new("bgpos").num_columns(4).show(ui, |ui| {
                        ui.label("X:");
                        ui.add(egui::DragValue::new(&mut b.x));
                        ui.label("Y:");
                        ui.add(egui::DragValue::new(&mut b.y));
                        ui.end_row();
                        ui.label("Hor. Speed:");
                        ui.add(egui::DragValue::new(&mut b.hspeed));
                        ui.label("Vert. Speed:");
                        ui.add(egui::DragValue::new(&mut b.vspeed));
                        ui.end_row();
                    });
                }
            }
            Tab::Views => {
                let r = &mut self.work;
                ui.checkbox(&mut r.views_enabled, "Enable the use of Views");
                for i in 0..r.views.len() {
                    let mut t = egui::RichText::new(format!("View {i}"));
                    if r.views[i].visible {
                        t = t.strong();
                    }
                    if ui.selectable_label(self.view_sel == i, t).clicked() {
                        self.view_sel = i;
                    }
                }
                if let Some(v) = r.views.get_mut(self.view_sel) {
                    ui.separator();
                    ui.checkbox(&mut v.visible, "Visible when room starts");
                    egui::Grid::new("viewg").num_columns(4).show(ui, |ui| {
                        ui.label("View in room");
                        ui.end_row();
                        ui.label("X");
                        ui.add(egui::DragValue::new(&mut v.view_x));
                        ui.label("W");
                        ui.add(egui::DragValue::new(&mut v.view_w));
                        ui.end_row();
                        ui.label("Y");
                        ui.add(egui::DragValue::new(&mut v.view_y));
                        ui.label("H");
                        ui.add(egui::DragValue::new(&mut v.view_h));
                        ui.end_row();
                        ui.label("Port on screen");
                        ui.end_row();
                        ui.label("X");
                        ui.add(egui::DragValue::new(&mut v.port_x));
                        ui.label("W");
                        ui.add(egui::DragValue::new(&mut v.port_w));
                        ui.end_row();
                        ui.label("Y");
                        ui.add(egui::DragValue::new(&mut v.port_y));
                        ui.label("H");
                        ui.add(egui::DragValue::new(&mut v.port_h));
                        ui.end_row();
                    });
                    ui.label("Object following:");
                    resource_picker(ui, "view_follow", cx.project, ResourceKind::Object, &mut v.follow_object, "<no object>", -1);
                    egui::Grid::new("viewb").num_columns(4).show(ui, |ui| {
                        ui.label("Hbor:");
                        ui.add(egui::DragValue::new(&mut v.h_border));
                        ui.label("Hsp:");
                        ui.add(egui::DragValue::new(&mut v.h_speed));
                        ui.end_row();
                        ui.label("Vbor:");
                        ui.add(egui::DragValue::new(&mut v.v_border));
                        ui.label("Vsp:");
                        ui.add(egui::DragValue::new(&mut v.v_speed));
                        ui.end_row();
                    });
                }
            }
        });
    }

    fn tiles_tab(&mut self, ui: &mut Ui, cx: &mut Cx) {
        ui.label("Tile set:");
        resource_picker(
            ui,
            "room_tileset",
            cx.project,
            ResourceKind::Background,
            &mut self.tileset,
            "<no background>",
            -1,
        );
        let bg = cx
            .project
            .backgrounds
            .get(self.tileset as usize)
            .and_then(|b| b.clone());
        if let Some(bg) = bg {
            let (tw, th) = if bg.use_as_tileset {
                (bg.tile_width.max(1), bg.tile_height.max(1))
            } else {
                (self.work.snap_x.max(1), self.work.snap_y.max(1))
            };
            let (ox, oy, sx, sy) = if bg.use_as_tileset {
                (
                    bg.tile_h_offset,
                    bg.tile_v_offset,
                    bg.tile_h_sep,
                    bg.tile_v_sep,
                )
            } else {
                (0, 0, 0, 0)
            };
            if let Some((t, size)) = cx.background_tex(self.tileset) {
                egui::ScrollArea::both()
                    .id_salt("tileset_view")
                    .max_height(260.0)
                    .show(ui, |ui| {
                        let (resp, rect, scale) = image_preview(
                            ui,
                            Some(t),
                            size.x as u32,
                            size.y as u32,
                            size,
                            1.0,
                            Sense::click(),
                        );
                        let (x, y, w, h) = self.tile_sel;
                        let r = Rect::from_min_size(
                            rect.min + Vec2::new(x as f32, y as f32) * scale,
                            Vec2::new(w as f32, h as f32) * scale,
                        );
                        ui.painter().rect_stroke(
                            r,
                            0.0,
                            Stroke::new(2.0, Color32::YELLOW),
                            egui::StrokeKind::Inside,
                        );
                        if resp.clicked() {
                            if let Some(p) = resp.interact_pointer_pos() {
                                let v = (p - rect.min) / scale;
                                let cx_ = ((v.x as i32 - ox).max(0)) / (tw + sx);
                                let cy_ = ((v.y as i32 - oy).max(0)) / (th + sy);
                                self.tile_sel =
                                    (ox + cx_ * (tw + sx), oy + cy_ * (th + sy), tw, th);
                            }
                        }
                    });
            }
        }
        ui.separator();
        ui.label("Current tile layer:");
        ui.horizontal(|ui| {
            let layers = self.layers();
            egui::ComboBox::from_id_salt("layer")
                .selected_text(self.layer.to_string())
                .show_ui(ui, |ui| {
                    for l in layers {
                        ui.selectable_value(&mut self.layer, l, l.to_string());
                    }
                });
            if ui.button("Add").clicked() {
                self.prompt = Some((
                    PromptOp::AddLayer,
                    Prompt {
                        title: "New tile layer".into(),
                        fields: vec![("Depth:".into(), (self.layer - 100).to_string())],
                    },
                ));
            }
            if ui.button("Delete").clicked() && self.layers().len() > 1 {
                self.snapshot();
                let l = self.layer;
                self.work.tiles.retain(|t| t.depth != l);
                self.extra_layers.retain(|&x| x != l);
                self.layer = self
                    .layers()
                    .into_iter()
                    .find(|&x| x != l)
                    .unwrap_or(1_000_000);
            }
        });
        ui.checkbox(&mut self.delete_underlying, "Delete underlying");
        ui.label(format!("{} tiles", self.work.tiles.len()));
    }
}

/// Depth of an object (0 when missing).
fn obj_depth(p: &Project, obj: i32) -> i32 {
    p.objects
        .get(obj as usize)
        .and_then(|o| o.as_ref())
        .map_or(0, |o| o.depth)
}

impl Editor for RoomEditor {
    fn key(&self) -> ResKey {
        ResKey::Res(ResourceKind::Room, self.idx)
    }
    fn title(&self) -> String {
        format!("Room Properties: {}", self.work.name)
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(1000.0, 640.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let ctx = cx.ctx.clone();
        let base = ui.id().with(("room", self.idx));
        // creation code modal
        if let Some((target, mut code, mut st)) = self.code.take() {
            let mut done: Option<bool> = None;
            egui::Modal::new(base.with("code")).show(&ctx, |ui| {
                ui.set_min_size(Vec2::new(640.0, 400.0));
                ui.heading(match target {
                    CodeTarget::Room => "Room creation code".to_string(),
                    CodeTarget::Instance(i) => format!(
                        "Creation code of instance {}",
                        self.work.instances.get(i).map_or(0, |x| x.id)
                    ),
                });
                ui.allocate_ui(Vec2::new(640.0, 360.0), |ui| {
                    code_editor(
                        ui,
                        ("room_code", self.idx),
                        &mut code,
                        &mut st,
                        cx.names,
                        cx.prefs,
                        18,
                    );
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        done = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        done = Some(false);
                    }
                });
            });
            match done {
                Some(true) => match target {
                    CodeTarget::Room => self.work.creation_code = code,
                    CodeTarget::Instance(i) => {
                        if let Some(inst) = self.work.instances.get_mut(i) {
                            inst.creation_code = code;
                        }
                    }
                },
                Some(false) => {}
                None => self.code = Some((target, code, st)),
            }
        }
        if let Some((op, mut pr)) = self.prompt.take() {
            match show_prompt(&ctx, base.with("prompt"), &mut pr) {
                PromptResult::Ok(v) => {
                    let n = |i: usize| v.get(i).and_then(|s| s.trim().parse::<i32>().ok());
                    match (op, n(0), n(1)) {
                        (PromptOp::ShiftAll, Some(dx), Some(dy)) => {
                            self.snapshot();
                            if self.tab == Tab::Tiles {
                                for t in &mut self.work.tiles {
                                    t.x += dx;
                                    t.y += dy;
                                }
                            } else {
                                for i in &mut self.work.instances {
                                    i.x += dx;
                                    i.y += dy;
                                }
                            }
                        }
                        (PromptOp::Position(i), Some(x), Some(y)) => {
                            if let Some(inst) = self.work.instances.get_mut(i) {
                                inst.x = x;
                                inst.y = y;
                            }
                        }
                        (PromptOp::AddLayer, Some(d), _) => {
                            self.extra_layers.push(d);
                            self.layer = d;
                        }
                        _ => cx.requests.push(Request::Message("Invalid values.".into())),
                    }
                }
                PromptResult::Cancel => {}
                PromptResult::None => self.prompt = Some((op, pr)),
            }
        }
        // toolbar
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                .clicked()
            {
                if let Some((i, t)) = self.undo.pop() {
                    self.work.instances = i;
                    self.work.tiles = t;
                }
            }
            if ui.button("Clear").clicked() {
                self.snapshot();
                if self.tab == Tab::Tiles {
                    let l = self.layer;
                    self.work.tiles.retain(|t| t.depth != l);
                } else {
                    self.work.instances.clear();
                }
            }
            if ui.button("Shift").clicked() {
                self.prompt = Some((
                    PromptOp::ShiftAll,
                    Prompt {
                        title: "Shift all".into(),
                        fields: vec![("dx:".into(), "0".into()), ("dy:".into(), "0".into())],
                    },
                ));
            }
            if ui.button("Sort X").clicked() {
                self.work.instances.sort_by_key(|i| i.x);
            }
            if ui.button("Sort Y").clicked() {
                self.work.instances.sort_by_key(|i| i.y);
            }
            if ui.button("Lock all").clicked() {
                if self.tab == Tab::Tiles {
                    self.work.tiles.iter_mut().for_each(|t| t.locked = true);
                } else {
                    self.work.instances.iter_mut().for_each(|i| i.locked = true);
                }
            }
            if ui.button("Unlock all").clicked() {
                if self.tab == Tab::Tiles {
                    self.work.tiles.iter_mut().for_each(|t| t.locked = false);
                } else {
                    self.work
                        .instances
                        .iter_mut()
                        .for_each(|i| i.locked = false);
                }
            }
            ui.separator();
            ui.label("Snap X");
            ui.add(egui::DragValue::new(&mut self.work.snap_x).range(1..=1024));
            ui.label("Y");
            ui.add(egui::DragValue::new(&mut self.work.snap_y).range(1..=1024));
            ui.checkbox(&mut self.show_grid, "Grid");
            ui.checkbox(&mut self.work.isometric_grid, "Iso");
            ui.separator();
            if ui.button("−").clicked() {
                self.zoom = (self.zoom / 2.0).max(0.125);
            }
            if ui.button("1:1").clicked() {
                self.zoom = 1.0;
            }
            if ui.button("+").clicked() {
                self.zoom = (self.zoom * 2.0).min(8.0);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
        });
        ui.separator();
        let h = ui.available_height() - 22.0;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(250.0);
                ui.set_height(h);
                self.tabs_ui(ui, cx);
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_height(h);
                self.canvas(ui, cx);
            });
        });
        ui.label(&self.status);
    }
    fn on_renamed(&mut self, name: &str) {
        self.work.name = name.to_string();
    }
    fn is_dirty(&self, p: &Project) -> bool {
        slot_dirty(p.rooms.get(self.idx), &self.work)
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        let changed = commit_slot(p.rooms.get_mut(self.idx), &self.work);
        if changed {
            if let Some(m) = self.work.instances.iter().map(|i| i.id).max() {
                p.last_instance_id = p.last_instance_id.max(m);
            }
            if let Some(m) = self.work.tiles.iter().map(|t| t.id).max() {
                p.last_tile_id = p.last_tile_id.max(m);
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::snap;

    #[test]
    fn snapping() {
        assert_eq!(snap(17.0, 16), 16);
        assert_eq!(snap(-1.0, 16), -16);
        assert_eq!(snap(31.9, 32), 0);
        assert_eq!(snap(5.0, 0), 5);
    }
}
