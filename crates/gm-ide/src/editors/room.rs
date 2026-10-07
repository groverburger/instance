//! Room form (specs/ide.md §15), laid out like the classic room form: a toolbar row,
//! a left column with two rows of tabs (backgrounds / views, objects / settings /
//! tiles) and a room preview minimap at its bottom, the room canvas on the right and
//! a status bar along the bottom.

mod icons;
pub mod minimap;

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, TextureHandle, Ui,
    UiBuilder, Vec2,
};
use gm_format::{Instance, Project, ResourceKind, Room, Tile};

use icons::RoomIcon;
use minimap::Show;

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
    show: Show,
    /// Size of the canvas in the last frame (for the minimap and zoom buttons).
    canvas_size: Vec2,
    /// Snap X / Y text fields.
    snap_txt: [String; 2],
    /// Room position (snapped) and instance under the mouse.
    hover: Option<(i32, i32, Option<usize>)>,
    /// Cached minimap texture: (fingerprint, texture).
    minimap: Option<(u64, TextureHandle)>,
    checker: Option<TextureHandle>,
}

/// Width of the left column.
const LEFT_W: f32 = 252.0;
const TOOLBAR_H: f32 = 28.0;
const STATUS_H: f32 = 22.0;
/// Side of the checkerboard texture tiled outside the room.
const CHECKER_TILE: f32 = 64.0;

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
            offset: Vec2::ZERO,
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
            show: Show::default(),
            canvas_size: Vec2::new(600.0, 400.0),
            snap_txt: [String::new(), String::new()],
            hover: None,
            minimap: None,
            checker: None,
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
        self.status.clear();
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
        self.status.clear();
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

    fn room_size(&self) -> Vec2 {
        Vec2::new(
            self.work.width.max(1) as f32,
            self.work.height.max(1) as f32,
        )
    }

    /// Keep the room inside the canvas' scroll range.
    fn clamp_scroll(&mut self) {
        self.offset =
            minimap::clamp_offset(self.offset, self.room_size(), self.canvas_size, self.zoom);
    }

    /// Change the zoom keeping the room point under canvas point `anchor` fixed.
    fn set_zoom(&mut self, nz: f32, anchor: Vec2) {
        let w = (anchor - self.offset) / self.zoom;
        self.zoom = nz;
        self.offset = anchor - w * nz;
        self.clamp_scroll();
    }

    fn checker_tex(&mut self, ctx: &egui::Context) -> egui::TextureId {
        self.checker
            .get_or_insert_with(|| {
                // CHECKER_TILE points square, 8-point cells
                let n = CHECKER_TILE as usize;
                let px: Vec<Color32> = (0..n * n)
                    .map(|i| {
                        if ((i % n) / 8 + (i / n) / 8).is_multiple_of(2) {
                            Color32::from_gray(255)
                        } else {
                            Color32::from_gray(204)
                        }
                    })
                    .collect();
                ctx.load_texture(
                    "room-checker",
                    egui::ColorImage::new([n, n], px),
                    egui::TextureOptions::NEAREST,
                )
            })
            .id()
    }

    fn canvas(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let size = ui.available_size().max(Vec2::splat(100.0));
        let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "room canvas")
        });
        let rect = resp.rect;
        self.canvas_size = rect.size();
        self.clamp_scroll();
        let painter = painter.with_clip_rect(rect);
        // the area outside the room: transparency checkerboard
        let checker = self.checker_tex(ui.ctx());
        let full_uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
        let mut y = rect.top();
        while y < rect.bottom() {
            let mut x = rect.left();
            while x < rect.right() {
                painter.image(
                    checker,
                    Rect::from_min_size(pos2(x, y), Vec2::splat(CHECKER_TILE)),
                    full_uv,
                    Color32::WHITE,
                );
                x += CHECKER_TILE;
            }
            y += CHECKER_TILE;
        }
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
        if self.show.backgrounds {
            self.draw_background_layer(cx, &painter, &to_screen, false);
        }
        // tiles and instances sorted by depth (deepest first)
        enum Item {
            Tile(usize),
            Inst(usize),
        }
        let mut items: Vec<(i32, usize, Item)> = Vec::new();
        if self.show.tiles {
            for (i, t) in self.work.tiles.iter().enumerate() {
                items.push((t.depth, i, Item::Tile(i)));
            }
        }
        if self.show.objects {
            for (i, inst) in self.work.instances.iter().enumerate() {
                items.push((obj_depth(cx.project, inst.object), i, Item::Inst(i)));
            }
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
                        if !r.intersects(rect) {
                            continue;
                        }
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
                    if !r.intersects(rect) {
                        continue;
                    }
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
        if self.show.foregrounds {
            self.draw_background_layer(cx, &painter, &to_screen, true);
        }
        // grid (only the visible part of the room)
        if self.show_grid {
            let st = Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 80));
            let gx = self.work.snap_x.max(1) as f32;
            let gy = self.work.snap_y.max(1) as f32;
            let vis = room_rect.intersect(rect);
            if gx * z >= 4.0 && gy * z >= 4.0 && vis.is_positive() {
                let w0 = to_world(vis.min);
                let w1 = to_world(vis.max);
                let gp = painter.with_clip_rect(vis);
                if self.work.isometric_grid {
                    // diamonds gx wide and gy high: two families of diagonals
                    let k = gy / gx;
                    let n0 = ((w0.x - w1.y / k) / gx).floor() as i64 - 1;
                    let n1 = ((w1.x + w1.y / k) / gx).ceil() as i64 + 1;
                    for n in n0..=n1 {
                        let x = n as f32 * gx;
                        let (ya, yb) = (w0.y, w1.y);
                        gp.line_segment(
                            [
                                to_screen(x + (ya - 0.0) / k, ya),
                                to_screen(x + (yb - 0.0) / k, yb),
                            ],
                            st,
                        );
                        gp.line_segment([to_screen(x - ya / k, ya), to_screen(x - yb / k, yb)], st);
                    }
                } else {
                    let mut x = (w0.x / gx).floor().max(0.0) * gx;
                    while x <= w1.x {
                        painter.vline(to_screen(x, 0.0).x.round() + 0.5, vis.y_range(), st);
                        x += gx;
                    }
                    let mut y = (w0.y / gy).floor().max(0.0) * gy;
                    while y <= w1.y {
                        painter.hline(vis.x_range(), to_screen(0.0, y).y.round() + 0.5, st);
                        y += gy;
                    }
                }
            }
        }
        // views
        if self.show.views && self.tab == Tab::Views && self.work.views_enabled {
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
        // Space held: the left button scrolls the room instead of editing.
        let space = ui.input(|i| i.key_down(egui::Key::Space))
            && ui.ctx().memory(|m| m.focused().is_none());
        if space && (resp.hovered() || resp.dragged()) {
            ui.ctx().set_cursor_icon(if resp.dragged() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        let edit_tab = matches!(self.tab, Tab::Objects | Tab::Tiles) && !space;
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
        if resp.dragged_by(egui::PointerButton::Middle)
            || (space && self.drag.is_none() && resp.dragged_by(egui::PointerButton::Primary))
        {
            self.offset += resp.drag_delta();
            self.clamp_scroll();
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
                    self.set_zoom(nz, hp - rect.min);
                }
            }
            if let Some(hp) = resp.hover_pos() {
                let w = to_world(hp);
                let (x, y) = snapped(w, mods.alt, sx, sy);
                let under = if self.show.objects {
                    self.instance_at(cx, w)
                } else {
                    None
                };
                self.hover = Some((x, y, under));
            }
        } else if !resp.dragged() {
            self.hover = None;
        }
    }

    /// Status bar: mouse position and the instance under the mouse.
    fn status_bar(&self, ui: &mut Ui, cx: &Cx, rect: Rect) {
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 0.0, crate::theme::PANEL);
        let (x, y, under) = match self.hover {
            Some((x, y, u)) => (x.to_string(), y.to_string(), u),
            None => (String::new(), String::new(), None),
        };
        let (obj, id) = match under.and_then(|i| self.work.instances.get(i)) {
            Some(inst) => (
                crate::project_ops::res_name(
                    cx.project,
                    ResourceKind::Object,
                    inst.object as usize,
                )
                .unwrap_or("?")
                .to_string(),
                inst.id.to_string(),
            ),
            None => (String::new(), String::new()),
        };
        let fields = [
            (format!("x: {x}"), 80.0),
            (format!("y: {y}"), 80.0),
            (format!("object: {obj}"), 220.0),
            (format!("id: {id}"), 110.0),
        ];
        let mut left = rect.left() + 2.0;
        let font = FontId::proportional(12.0);
        for (text, w) in fields {
            let r = Rect::from_min_size(pos2(left, rect.top() + 2.0), vec2(w, rect.height() - 3.0));
            icons::sunken(&p, r);
            p.with_clip_rect(r.shrink(1.0)).text(
                pos2(r.left() + 5.0, r.center().y),
                Align2::LEFT_CENTER,
                text,
                font.clone(),
                Color32::from_gray(10),
            );
            left = r.right() + 3.0;
        }
        let r = Rect::from_min_max(
            pos2(left, rect.top() + 2.0),
            pos2(rect.right() - 2.0, rect.bottom() - 1.0),
        );
        if r.width() > 20.0 {
            icons::sunken(&p, r);
            p.with_clip_rect(r.shrink(1.0)).text(
                pos2(r.left() + 5.0, r.center().y),
                Align2::LEFT_CENTER,
                &self.status,
                font,
                Color32::from_rgb(170, 20, 20),
            );
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        use icons::{separator, tool_button as tb};
        ui.spacing_mut().item_spacing.x = 1.0;
        let tiles = self.tab == Tab::Tiles;
        let what = if tiles { "tiles" } else { "instances" };
        if tb(
            ui,
            RoomIcon::Undo,
            "Undo the last change",
            !self.undo.is_empty(),
            false,
            24.0,
        )
        .clicked()
        {
            if let Some((i, t)) = self.undo.pop() {
                self.work.instances = i;
                self.work.tiles = t;
            }
        }
        separator(ui);
        if tb(
            ui,
            RoomIcon::Clear,
            &format!("Clear all {what}"),
            true,
            false,
            24.0,
        )
        .clicked()
        {
            self.snapshot();
            if tiles {
                let l = self.layer;
                self.work.tiles.retain(|t| t.depth != l);
            } else {
                self.work.instances.clear();
            }
        }
        if tb(
            ui,
            RoomIcon::Shift,
            &format!("Shift all {what}"),
            true,
            false,
            24.0,
        )
        .clicked()
        {
            self.prompt = Some((
                PromptOp::ShiftAll,
                Prompt {
                    title: "Shift all".into(),
                    fields: vec![("dx:".into(), "0".into()), ("dy:".into(), "0".into())],
                },
            ));
        }
        if tb(
            ui,
            RoomIcon::SortX,
            "Sort the instances by x",
            true,
            false,
            24.0,
        )
        .clicked()
        {
            self.snapshot();
            self.work.instances.sort_by_key(|i| i.x);
        }
        if tb(
            ui,
            RoomIcon::SortY,
            "Sort the instances by y",
            true,
            false,
            24.0,
        )
        .clicked()
        {
            self.snapshot();
            self.work.instances.sort_by_key(|i| i.y);
        }
        if tb(
            ui,
            RoomIcon::Lock,
            &format!("Lock all {what}"),
            true,
            false,
            24.0,
        )
        .clicked()
        {
            if tiles {
                self.work.tiles.iter_mut().for_each(|t| t.locked = true);
            } else {
                self.work.instances.iter_mut().for_each(|i| i.locked = true);
            }
        }
        if tb(
            ui,
            RoomIcon::Unlock,
            &format!("Unlock all {what}"),
            true,
            false,
            24.0,
        )
        .clicked()
        {
            if tiles {
                self.work.tiles.iter_mut().for_each(|t| t.locked = false);
            } else {
                self.work
                    .instances
                    .iter_mut()
                    .for_each(|i| i.locked = false);
            }
        }
        separator(ui);
        ui.spacing_mut().item_spacing.x = 4.0;
        for (k, label) in [(0usize, "Snap X:"), (1, "Snap Y:")] {
            ui.label(label);
            let v = if k == 0 {
                &mut self.work.snap_x
            } else {
                &mut self.work.snap_y
            };
            let id = ui.id().with(("snap", k));
            let focused = ui.ctx().memory(|m| m.has_focus(id));
            if !focused {
                self.snap_txt[k] = v.to_string();
            }
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.snap_txt[k])
                    .id(id)
                    .desired_width(30.0)
                    .horizontal_align(egui::Align::Center),
            );
            if r.changed() {
                if let Ok(n) = self.snap_txt[k].trim().parse::<i32>() {
                    *v = n.clamp(1, 1024);
                }
            }
        }
        ui.spacing_mut().item_spacing.x = 1.0;
        separator(ui);
        if tb(
            ui,
            RoomIcon::Grid,
            "Toggle the grid",
            true,
            self.show_grid,
            24.0,
        )
        .clicked()
        {
            self.show_grid = !self.show_grid;
        }
        if tb(
            ui,
            RoomIcon::IsoGrid,
            "Toggle the isometric grid",
            true,
            self.work.isometric_grid,
            24.0,
        )
        .clicked()
        {
            self.work.isometric_grid = !self.work.isometric_grid;
        }
        separator(ui);
        let show = tb(ui, RoomIcon::Show, "Show", true, false, 36.0);
        egui::Popup::menu(&show)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.checkbox(&mut self.show.objects, "Show Objects");
                ui.checkbox(&mut self.show.tiles, "Show Tiles");
                ui.checkbox(&mut self.show.backgrounds, "Show Backgrounds");
                ui.checkbox(&mut self.show.foregrounds, "Show Foregrounds");
                ui.checkbox(&mut self.show.views, "Show Views");
            });
        separator(ui);
        let c = self.canvas_size / 2.0;
        if tb(
            ui,
            RoomIcon::ZoomOut,
            "Zoom out",
            self.zoom > 0.13,
            false,
            24.0,
        )
        .clicked()
        {
            self.set_zoom((self.zoom / 2.0).max(0.125), c);
        }
        if tb(ui, RoomIcon::ZoomNormal, "Normal size", true, false, 24.0).clicked() {
            self.set_zoom(1.0, c);
        }
        if tb(
            ui,
            RoomIcon::ZoomIn,
            "Zoom in",
            self.zoom < 7.9,
            false,
            24.0,
        )
        .clicked()
        {
            self.set_zoom((self.zoom * 2.0).min(8.0), c);
        }
        ui.add_space(4.0);
        ui.label(format!("{:.0}%", self.zoom * 100.0));
    }

    /// Room preview at the bottom of the left column: the whole room downscaled, with
    /// the part visible in the canvas outlined. Clicking or dragging scrolls there.
    fn minimap_ui(&mut self, ui: &mut Ui, cx: &mut Cx, area: Rect) {
        let p = ui.painter().clone();
        p.rect_filled(area, 0.0, Color32::from_gray(128));
        icons::sunken(&p, area);
        let room = self.room_size();
        let img = minimap::fit(area.shrink(4.0), room);
        let img = {
            use egui::emath::GuiRounding as _;
            img.round_to_pixels(ui.ctx().pixels_per_point())
        };
        let ppp = ui.ctx().pixels_per_point();
        let px = (
            ((img.width() * ppp).round() as u32).clamp(1, 1024),
            ((img.height() * ppp).round() as u32).clamp(1, 1024),
        );
        let fp = minimap::fingerprint(&self.work, self.show, cx.gen, px);
        if self.minimap.as_ref().is_none_or(|(f, _)| *f != fp) {
            let rgba = minimap::render(cx.project, &self.work, self.show, px.0, px.1);
            let ci = egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width as usize, rgba.height as usize],
                &rgba.data,
            );
            match &mut self.minimap {
                Some((f, tex)) => {
                    tex.set(ci, egui::TextureOptions::LINEAR);
                    *f = fp;
                }
                None => {
                    let tex = ui.ctx().load_texture(
                        format!("room-minimap-{}", self.idx),
                        ci,
                        egui::TextureOptions::LINEAR,
                    );
                    self.minimap = Some((fp, tex));
                }
            }
        }
        if let Some((_, tex)) = &self.minimap {
            p.image(
                tex.id(),
                img,
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        p.rect_stroke(
            img,
            0.0,
            Stroke::new(1.0, Color32::BLACK),
            egui::StrokeKind::Outside,
        );
        let vis = minimap::visible_rect(img, room, self.canvas_size, self.offset, self.zoom);
        if vis.is_positive()
            && (vis.width() < img.width() - 0.5 || vis.height() < img.height() - 0.5)
        {
            // dim what is outside the visible part
            let shade = Color32::from_black_alpha(70);
            for r in [
                Rect::from_min_max(img.min, pos2(img.max.x, vis.min.y)),
                Rect::from_min_max(pos2(img.min.x, vis.max.y), img.max),
                Rect::from_min_max(pos2(img.min.x, vis.min.y), pos2(vis.min.x, vis.max.y)),
                Rect::from_min_max(pos2(vis.max.x, vis.min.y), pos2(img.max.x, vis.max.y)),
            ] {
                if r.is_positive() {
                    p.rect_filled(r, 0.0, shade);
                }
            }
            let cp = p.with_clip_rect(img.expand(1.0));
            cp.rect_stroke(
                vis,
                0.0,
                Stroke::new(1.0, Color32::BLACK),
                egui::StrokeKind::Outside,
            );
            cp.rect_stroke(
                vis,
                0.0,
                Stroke::new(1.0, Color32::WHITE),
                egui::StrokeKind::Inside,
            );
        }
        let resp = ui.interact(area, ui.id().with("room_minimap"), Sense::click_and_drag());
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "room preview")
        });
        if resp.is_pointer_button_down_on() {
            if let Some(pos) = resp.interact_pointer_pos() {
                self.offset = minimap::click_to_offset(img, room, pos, self.canvas_size, self.zoom);
            }
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    /// Two rows of classic tabs; the row holding the selected tab sits in front.
    fn tab_strip(&mut self, ui: &mut Ui, rect: Rect) {
        const ROWS: [&[(Tab, &str)]; 2] = [
            &[(Tab::Backgrounds, "backgrounds"), (Tab::Views, "views")],
            &[
                (Tab::Objects, "objects"),
                (Tab::Settings, "settings"),
                (Tab::Tiles, "tiles"),
            ],
        ];
        let front = ROWS
            .iter()
            .position(|r| r.iter().any(|(t, _)| *t == self.tab))
            .unwrap_or(1);
        let row_h = rect.height() / 2.0;
        let p = ui.painter().clone();
        // back row first so that the front row's selected tab overlaps it
        let order = if front == 1 { [0, 1] } else { [1, 0] };
        for ri in order {
            let row = ROWS[ri];
            let slot = if ri == front { 1.0 } else { 0.0 };
            let y = rect.top() + slot * row_h;
            let inset = if ri == front { 0.0 } else { 4.0 };
            let w = (rect.width() - 2.0 * inset) / row.len() as f32;
            for (ci, (tab, label)) in row.iter().enumerate() {
                let r = Rect::from_min_size(
                    pos2(rect.left() + inset + ci as f32 * w, y + 2.0),
                    vec2(w, row_h - 2.0),
                );
                let sel = self.tab == *tab;
                let resp = ui.interact(r, ui.id().with(("room_tab", *label)), Sense::click());
                resp.widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, sel, *label)
                });
                if resp.clicked() {
                    self.tab = *tab;
                }
                let r = if sel {
                    Rect::from_min_max(r.min - vec2(2.0, 2.0), r.max + vec2(2.0, 1.0))
                } else {
                    r
                };
                let fill = if sel {
                    crate::theme::PANEL
                } else if resp.hovered() {
                    Color32::from_rgb(234, 242, 252)
                } else {
                    Color32::from_gray(226)
                };
                p.rect_filled(r, 0.0, fill);
                let hi = Stroke::new(1.0, Color32::WHITE);
                let lo = Stroke::new(1.0, Color32::from_gray(130));
                let q = r.shrink(0.5);
                p.line_segment([q.left_bottom(), q.left_top() + vec2(0.0, 2.0)], hi);
                p.line_segment(
                    [q.left_top() + vec2(0.0, 2.0), q.left_top() + vec2(2.0, 0.0)],
                    hi,
                );
                p.line_segment(
                    [
                        q.left_top() + vec2(2.0, 0.0),
                        q.right_top() - vec2(2.0, 0.0),
                    ],
                    hi,
                );
                p.line_segment([q.right_top() + vec2(-1.0, 1.0), q.right_bottom()], lo);
                p.text(
                    r.center() + vec2(0.0, if sel { -1.0 } else { 0.0 }),
                    Align2::CENTER_CENTER,
                    *label,
                    FontId::proportional(12.5),
                    Color32::from_gray(10),
                );
            }
        }
    }

    fn objects_tab(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let w = ui.available_width();
        // large preview of the selected object's sprite; clicking opens the picker
        let (prev, painter) = ui.allocate_painter(vec2(w, 132.0), Sense::click());
        let r = prev.rect;
        painter.rect_filled(r, 0.0, Color32::WHITE);
        let inner = r.shrink(2.0);
        let spr = cx
            .project
            .objects
            .get(self.obj_to_add as usize)
            .and_then(|o| o.as_ref())
            .map_or(-1, |o| o.sprite);
        if let Some((t, size, org)) = cx.sprite_tex(spr, 0) {
            let s = ((inner.width() - 8.0) / size.x)
                .min((inner.height() - 8.0) / size.y)
                .clamp(0.05, 4.0);
            let ir = Rect::from_center_size(inner.center(), size * s);
            crate::widgets::checker(&painter.with_clip_rect(inner), ir, 8.0);
            painter.image(
                t,
                ir,
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            if org != Vec2::ZERO {
                let o = ir.min + org * s;
                let st = Stroke::new(1.0, Color32::from_rgb(220, 0, 0));
                let cp = painter.with_clip_rect(inner);
                cp.hline(o.x - 5.0..=o.x + 5.0, o.y, st);
                cp.vline(o.x, o.y - 5.0..=o.y + 5.0, st);
            }
        } else {
            painter.text(
                inner.center(),
                Align2::CENTER_CENTER,
                if self.obj_to_add < 0 {
                    "no object"
                } else {
                    "no sprite"
                },
                FontId::proportional(12.0),
                Color32::from_gray(140),
            );
        }
        icons::sunken(&painter, r);
        ui.add_space(4.0);
        ui.label("Object to add with left mouse:");
        let name = if self.obj_to_add < 0 {
            "<no object>".to_string()
        } else {
            crate::project_ops::res_name(cx.project, ResourceKind::Object, self.obj_to_add as usize)
                .unwrap_or("<missing>")
                .to_string()
        };
        let btn = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let (fr, _) = ui.allocate_exact_size(vec2(w - 26.0, 24.0), Sense::hover());
                let p = ui.painter();
                p.rect_filled(fr, 0.0, Color32::WHITE);
                icons::sunken(p, fr);
                p.with_clip_rect(fr.shrink(2.0)).text(
                    pos2(fr.left() + 5.0, fr.center().y),
                    Align2::LEFT_CENTER,
                    &name,
                    FontId::proportional(12.5),
                    Color32::from_gray(10),
                );
                icons::tool_button(
                    ui,
                    RoomIcon::Picker,
                    "Select the object to add",
                    true,
                    false,
                    24.0,
                )
            })
            .inner;
        let popup_id = egui::Popup::default_response_id(&btn);
        if prev.clicked() {
            egui::Popup::toggle_id(ui.ctx(), popup_id);
        }
        egui::Popup::menu(&btn).show(|ui| {
            ui.set_min_width(200.0);
            egui::ScrollArea::vertical()
                .max_height(420.0)
                .show(ui, |ui| {
                    for (idx, n, depth) in
                        crate::project_ops::tree_order(cx.project, ResourceKind::Object)
                    {
                        let indent = "    ".repeat(depth);
                        if idx < 0 {
                            ui.label(egui::RichText::new(format!("{indent}{n}")).weak());
                        } else if ui
                            .selectable_label(self.obj_to_add == idx, format!("{indent}{n}"))
                            .clicked()
                        {
                            self.obj_to_add = idx;
                            ui.close();
                        }
                    }
                });
        });
        ui.add_space(8.0);
        let help = [
            ("Left mouse button", "= add"),
            ("    + <Alt>", "= no snap"),
            ("    + <Shift>", "= multiple"),
            ("    + <Ctrl>", "= move"),
            ("    + <Space>", "= scroll room"),
            ("Right mouse button", "= menu"),
            ("    + <Shift>", "= delete all"),
            ("    + <Ctrl>", "= delete"),
        ];
        egui::Grid::new("room_help")
            .striped(false)
            .num_columns(2)
            .spacing([6.0, 1.0])
            .show(ui, |ui| {
                for (a, b) in help {
                    ui.label(egui::RichText::new(a).size(11.5));
                    ui.label(egui::RichText::new(b).size(11.5));
                    ui.end_row();
                }
            });
        ui.add_space(6.0);
        ui.checkbox(&mut self.delete_underlying, "Delete underlying");
    }

    fn tabs_ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        egui::ScrollArea::vertical()
            .id_salt("room_tab")
            .auto_shrink([false, false])
            .show(ui, |ui| match self.tab {
                Tab::Objects => self.objects_tab(ui, cx),
                Tab::Settings => {
                    let r = &mut self.work;
                    egui::Grid::new("room_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
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
                        self.code = Some((
                            CodeTarget::Room,
                            r.creation_code.clone(),
                            CodeEditorState::default(),
                        ));
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
                        resource_picker(
                            ui,
                            "room_bg",
                            cx.project,
                            ResourceKind::Background,
                            &mut b.background,
                            "<no background>",
                            -1,
                        );
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
                        resource_picker(
                            ui,
                            "view_follow",
                            cx.project,
                            ResourceKind::Object,
                            &mut v.follow_object,
                            "<no object>",
                            -1,
                        );
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
        // Layout: toolbar row, then the left column and the canvas, then the status bar.
        let full = ui.available_rect_before_wrap();
        let tb = Rect::from_min_size(full.min, vec2(full.width(), TOOLBAR_H));
        let status = Rect::from_min_max(pos2(full.left(), full.bottom() - STATUS_H), full.max);
        let body = Rect::from_min_max(
            pos2(full.left(), tb.bottom() + 2.0),
            pos2(full.right(), status.top() - 2.0),
        );
        let left = Rect::from_min_size(
            body.min,
            vec2(LEFT_W.min(body.width() * 0.5), body.height()),
        );
        let canvas = Rect::from_min_max(pos2(left.right() + 4.0, body.top()), body.max);
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(tb)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| self.toolbar(ui),
        );
        // left column: tabs, tab page, room preview
        let tabs_h = 46.0;
        let mm_h = (left.height() * 0.38).clamp(80.0, 190.0);
        let tabs = Rect::from_min_size(left.min, vec2(left.width(), tabs_h));
        let mm = Rect::from_min_max(pos2(left.left(), left.bottom() - mm_h), left.max);
        let page = Rect::from_min_max(
            pos2(left.left(), tabs.bottom()),
            pos2(left.right(), mm.top() - 4.0),
        );
        {
            let p = ui.painter();
            p.rect_filled(page, 0.0, crate::theme::PANEL);
            icons::raised(p, page);
        }
        self.tab_strip(ui, tabs);
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(page.shrink2(vec2(6.0, 6.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.set_clip_rect(page.shrink(2.0));
                self.tabs_ui(ui, cx);
            },
        );
        self.minimap_ui(ui, cx, mm);
        // canvas with a sunken edge
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(canvas.shrink(2.0))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| self.canvas(ui, cx),
        );
        icons::sunken(ui.painter(), canvas);
        ui.painter().rect_stroke(
            canvas.shrink(1.5),
            0.0,
            Stroke::new(1.0, Color32::from_gray(100)),
            egui::StrokeKind::Middle,
        );
        self.status_bar(ui, cx, status);
        ui.allocate_rect(full, Sense::hover());
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

    /// Renders the room form for a project room to a PNG (manual check of the look).
    /// `GM_ROOM_SHOT=project.gm81;room_name;out.png cargo test -p gm-ide room_form_screenshot -- --ignored`
    #[test]
    #[ignore]
    fn room_form_screenshot() {
        use egui_kittest::kittest::Queryable;
        let Some(spec) = std::env::var_os("GM_ROOM_SHOT") else {
            return;
        };
        let spec = spec.to_string_lossy().to_string();
        let parts: Vec<&str> = spec.split(';').collect();
        let proj = crate::project_ops::load_project(std::path::Path::new(parts[0])).unwrap();
        let idx = proj
            .rooms
            .iter()
            .position(|r| r.as_ref().is_some_and(|r| r.name == parts[1]))
            .expect("room");
        let mut h = egui_kittest::Harness::builder()
            .with_size(eframe::egui::Vec2::new(1440.0, 900.0))
            .with_theme(eframe::egui::Theme::Light)
            .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
        h.state_mut().test_set_project(proj);
        h.run_steps(2);
        h.state_mut()
            .open_form(super::ResKey::Res(gm_format::ResourceKind::Room, idx));
        h.run_steps(3);
        h.get_by_label("Maximise").click();
        h.run_steps(3);
        let frac = |i: usize| -> Option<eframe::egui::Vec2> {
            let v: Vec<f32> = parts
                .get(i)?
                .split(',')
                .filter_map(|s| s.parse().ok())
                .collect();
            Some(eframe::egui::vec2(*v.first()?, *v.get(1)?))
        };
        if let Some(f) = frac(4) {
            // click into the room preview to scroll there
            let r = h.get_by_label("room preview").rect();
            h.get_by_label("room preview").hover();
            let pos = r.min + r.size() * f;
            h.event(eframe::egui::Event::PointerMoved(pos));
            h.event(eframe::egui::Event::PointerButton {
                pos,
                button: eframe::egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            });
            h.run_steps(1);
            h.event(eframe::egui::Event::PointerButton {
                pos,
                button: eframe::egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            });
            h.run_steps(2);
        }
        if let Some(f) = frac(3) {
            // hover over the canvas so the status bar shows something
            let r = h.get_by_label("room canvas").rect();
            h.hover_at(r.min + r.size() * f);
            h.run_steps(2);
        }
        h.run_steps(3);
        let img = h.render().expect("render");
        img.save(parts[2]).unwrap();
    }

    #[test]
    fn snapping() {
        assert_eq!(snap(17.0, 16), 16);
        assert_eq!(snap(-1.0, 16), -16);
        assert_eq!(snap(31.9, 32), 0);
        assert_eq!(snap(5.0, 0), 5);
    }
}
