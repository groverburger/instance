//! Path form with canvas editing (specs/ide.md §10).

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use gm_format::{Path, PathPoint, Project, ResourceKind};

use super::{commit_slot, slot_dirty, Cx, Editor, Request, ResKey};
use crate::widgets::{resource_picker, show_prompt, Prompt, PromptResult};

/// Points of the drawn curve (straight or smoothed), for display only.
/// Smooth curves use a quadratic B-spline through the edge midpoints (see
/// OPEN_QUESTIONS [IMPL-IDE]); `precision` sets subdivisions per segment.
pub fn curve(p: &Path) -> Vec<(f64, f64)> {
    let pts = &p.points;
    let n = pts.len();
    if n < 2 || p.connection_kind == 0 {
        let mut v: Vec<(f64, f64)> = pts.iter().map(|q| (q.x, q.y)).collect();
        if p.closed && n > 2 {
            v.push((pts[0].x, pts[0].y));
        }
        return v;
    }
    let steps = 1usize << p.precision.clamp(1, 8);
    let mid = |a: &PathPoint, b: &PathPoint| ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let quad = |out: &mut Vec<(f64, f64)>, a: (f64, f64), c: (f64, f64), b: (f64, f64)| {
        for s in 1..=steps {
            let t = s as f64 / steps as f64;
            let u = 1.0 - t;
            out.push((
                u * u * a.0 + 2.0 * u * t * c.0 + t * t * b.0,
                u * u * a.1 + 2.0 * u * t * c.1 + t * t * b.1,
            ));
        }
    };
    let mut out = Vec::new();
    if p.closed {
        let start = mid(&pts[n - 1], &pts[0]);
        out.push(start);
        for i in 0..n {
            let a = mid(&pts[(i + n - 1) % n], &pts[i]);
            let b = mid(&pts[i], &pts[(i + 1) % n]);
            quad(&mut out, a, (pts[i].x, pts[i].y), b);
        }
    } else {
        out.push((pts[0].x, pts[0].y));
        let mut a = (pts[0].x, pts[0].y);
        for i in 1..n - 1 {
            let b = if i + 1 == n - 1 {
                (pts[n - 1].x, pts[n - 1].y)
            } else {
                mid(&pts[i], &pts[i + 1])
            };
            quad(&mut out, a, (pts[i].x, pts[i].y), b);
            a = b;
        }
        if n == 2 {
            out.push((pts[1].x, pts[1].y));
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Shift,
    Rotate,
    Scale,
}

pub struct PathEditor {
    idx: usize,
    work: Path,
    sel: Option<usize>,
    offset: Vec2,
    zoom: f32,
    drag: Option<usize>,
    show_grid: bool,
    prompt: Option<(Op, Prompt)>,
    undo: Vec<Vec<PathPoint>>,
}

impl PathEditor {
    pub fn new(idx: usize, work: Path) -> Self {
        PathEditor {
            idx,
            work,
            sel: None,
            offset: Vec2::new(16.0, 16.0),
            zoom: 1.0,
            drag: None,
            show_grid: true,
            prompt: None,
            undo: Vec::new(),
        }
    }

    fn snapshot(&mut self) {
        self.undo.push(self.work.points.clone());
        if self.undo.len() > 32 {
            self.undo.remove(0);
        }
    }

    fn center(&self) -> (f64, f64) {
        let n = self.work.points.len().max(1) as f64;
        let sx: f64 = self.work.points.iter().map(|p| p.x).sum();
        let sy: f64 = self.work.points.iter().map(|p| p.y).sum();
        (sx / n, sy / n)
    }

    fn snap(&self, v: Vec2, alt: bool) -> (f64, f64) {
        if alt {
            return (v.x.round() as f64, v.y.round() as f64);
        }
        let sx = self.work.snap_x.max(1) as f32;
        let sy = self.work.snap_y.max(1) as f32;
        (
            ((v.x / sx).round() * sx) as f64,
            ((v.y / sy).round() * sy) as f64,
        )
    }

    fn canvas(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let size = ui.available_size().max(Vec2::splat(100.0));
        let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "path canvas")
        });
        let rect = resp.rect;
        painter.rect_filled(rect, 0.0, Color32::from_gray(40));
        let z = self.zoom;
        let origin = rect.min + self.offset;
        let to_screen = |x: f64, y: f64| origin + Vec2::new(x as f32, y as f32) * z;
        let to_world = |p: Pos2| (p - origin) / z;
        // Backdrop room.
        if let Some(room) = usize::try_from(self.work.backdrop_room)
            .ok()
            .and_then(|r| cx.project.rooms.get(r))
            .and_then(|r| r.as_ref())
        {
            let rr = Rect::from_min_max(
                to_screen(0.0, 0.0),
                to_screen(room.width as f64, room.height as f64),
            );
            painter.rect_filled(
                rr,
                0.0,
                crate::widgets::bgr_to_color(room.background_color).gamma_multiply(0.5),
            );
            for inst in &room.instances {
                let spr = cx
                    .project
                    .objects
                    .get(inst.object as usize)
                    .and_then(|o| o.as_ref())
                    .map_or(-1, |o| o.sprite);
                if let Some((t, sz, org)) = cx.sprite_tex(spr, 0) {
                    let min = to_screen(inst.x as f64 - org.x as f64, inst.y as f64 - org.y as f64);
                    painter.image(
                        t,
                        Rect::from_min_size(min, sz * z),
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::from_white_alpha(128),
                    );
                }
            }
        }
        // Grid.
        if self.show_grid {
            let gx = self.work.snap_x.max(2) as f32 * z;
            let gy = self.work.snap_y.max(2) as f32 * z;
            if gx >= 4.0 && gy >= 4.0 {
                let st = Stroke::new(1.0, Color32::from_gray(60));
                let mut x = origin.x - ((origin.x - rect.left()) / gx).floor() * gx;
                while x < rect.right() {
                    painter.vline(x, rect.y_range(), st);
                    x += gx;
                }
                let mut y = origin.y - ((origin.y - rect.top()) / gy).floor() * gy;
                while y < rect.bottom() {
                    painter.hline(rect.x_range(), y, st);
                    y += gy;
                }
            }
        }
        // Curve.
        let c: Vec<Pos2> = curve(&self.work)
            .iter()
            .map(|&(x, y)| to_screen(x, y))
            .collect();
        if c.len() >= 2 {
            painter.add(egui::Shape::line(
                c,
                Stroke::new(2.0, Color32::from_rgb(230, 230, 230)),
            ));
        }
        for (i, p) in self.work.points.iter().enumerate() {
            let col = if Some(i) == self.sel {
                Color32::RED
            } else if i == 0 {
                Color32::GREEN
            } else {
                Color32::from_rgb(80, 120, 255)
            };
            painter.circle_filled(to_screen(p.x, p.y), 4.0, col);
        }
        // Interaction.
        let alt = ui.input(|i| i.modifiers.alt);
        let hit = |pts: &[PathPoint], pos: Pos2| {
            pts.iter()
                .enumerate()
                .map(|(i, p)| (i, (to_screen(p.x, p.y) - pos).length()))
                .filter(|(_, d)| *d < 7.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|x| x.0)
        };
        if resp.drag_started_by(egui::PointerButton::Primary) {
            if let Some(pos) = resp.interact_pointer_pos() {
                if let Some(i) = hit(&self.work.points, pos) {
                    self.snapshot();
                    self.drag = Some(i);
                    self.sel = Some(i);
                }
            }
        }
        if let (Some(i), Some(pos)) = (self.drag, resp.interact_pointer_pos()) {
            if resp.dragged_by(egui::PointerButton::Primary) {
                let (x, y) = self.snap(to_world(pos), alt);
                if let Some(p) = self.work.points.get_mut(i) {
                    p.x = x;
                    p.y = y;
                }
            }
        }
        if resp.drag_stopped() {
            self.drag = None;
        }
        if resp.dragged_by(egui::PointerButton::Middle) {
            self.offset += resp.drag_delta();
        }
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let shift = ui.input(|i| i.modifiers.shift);
                match hit(&self.work.points, pos) {
                    Some(i) if shift => {
                        self.snapshot();
                        let p = self.work.points[i];
                        self.work.points.insert(i, p);
                        self.sel = Some(i);
                    }
                    Some(i) => self.sel = Some(i),
                    None => {
                        self.snapshot();
                        let (x, y) = self.snap(to_world(pos), alt);
                        self.work.points.push(PathPoint { x, y, speed: 100.0 });
                        self.sel = Some(self.work.points.len() - 1);
                    }
                }
            }
        }
        if resp.secondary_clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                if let Some(i) = hit(&self.work.points, pos) {
                    self.snapshot();
                    self.work.points.remove(i);
                    self.sel = None;
                }
            }
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let nz = (self.zoom * if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 }).clamp(0.1, 16.0);
                if let Some(hp) = resp.hover_pos() {
                    let w = to_world(hp);
                    self.zoom = nz;
                    self.offset = hp - rect.min - w * nz;
                }
            }
            if let Some(hp) = resp.hover_pos() {
                let w = to_world(hp);
                painter.text(
                    rect.left_bottom() + Vec2::new(4.0, -4.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("x: {:.0}  y: {:.0}", w.x, w.y),
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );
            }
        }
    }
}

impl Editor for PathEditor {
    fn key(&self) -> ResKey {
        ResKey::Res(ResourceKind::Path, self.idx)
    }
    fn title(&self) -> String {
        format!("Path Properties: {}", self.work.name)
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(860.0, 560.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let ctx = cx.ctx.clone();
        if let Some((op, mut pr)) = self.prompt.take() {
            match show_prompt(&ctx, ui.id().with("path_prompt"), &mut pr) {
                PromptResult::Ok(v) => {
                    let f = |i: usize| v.get(i).and_then(|s| s.trim().parse::<f64>().ok());
                    self.snapshot();
                    let (cxp, cyp) = self.center();
                    match (op, f(0), f(1)) {
                        (Op::Shift, Some(dx), Some(dy)) => {
                            for p in &mut self.work.points {
                                p.x += dx;
                                p.y += dy;
                            }
                        }
                        (Op::Rotate, Some(a), _) => {
                            let (s, c) = (-a.to_radians()).sin_cos();
                            for p in &mut self.work.points {
                                let (x, y) = (p.x - cxp, p.y - cyp);
                                p.x = cxp + x * c - y * s;
                                p.y = cyp + x * s + y * c;
                            }
                        }
                        (Op::Scale, Some(sx), Some(sy)) => {
                            for p in &mut self.work.points {
                                p.x = cxp + (p.x - cxp) * sx / 100.0;
                                p.y = cyp + (p.y - cyp) * sy / 100.0;
                            }
                        }
                        _ => cx.requests.push(Request::Message("Invalid values.".into())),
                    }
                }
                PromptResult::Cancel => {}
                PromptResult::None => self.prompt = Some((op, pr)),
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                .clicked()
            {
                if let Some(p) = self.undo.pop() {
                    self.work.points = p;
                    self.sel = None;
                }
            }
            if ui.button("Clear").clicked() {
                self.snapshot();
                self.work.points.clear();
                self.sel = None;
            }
            if ui.button("Reverse").clicked() {
                self.snapshot();
                self.work.points.reverse();
            }
            if ui.button("Shift").clicked() {
                self.prompt = Some((
                    Op::Shift,
                    Prompt {
                        title: "Shift path".into(),
                        fields: vec![("dx:".into(), "0".into()), ("dy:".into(), "0".into())],
                    },
                ));
            }
            if ui.button("Mirror").clicked() {
                self.snapshot();
                let (c, _) = self.center();
                for p in &mut self.work.points {
                    p.x = 2.0 * c - p.x;
                }
            }
            if ui.button("Flip").clicked() {
                self.snapshot();
                let (_, c) = self.center();
                for p in &mut self.work.points {
                    p.y = 2.0 * c - p.y;
                }
            }
            if ui.button("Rotate").clicked() {
                self.prompt = Some((
                    Op::Rotate,
                    Prompt {
                        title: "Rotate path".into(),
                        fields: vec![("Angle (degrees):".into(), "90".into())],
                    },
                ));
            }
            if ui.button("Scale").clicked() {
                self.prompt = Some((
                    Op::Scale,
                    Prompt {
                        title: "Scale path".into(),
                        fields: vec![
                            ("Horizontal %:".into(), "100".into()),
                            ("Vertical %:".into(), "100".into()),
                        ],
                    },
                ));
            }
            ui.separator();
            if ui.button("Center view").clicked() {
                let (x, y) = self.center();
                self.offset = Vec2::new(300.0, 200.0) - Vec2::new(x as f32, y as f32) * self.zoom;
            }
            ui.label("Snap X");
            ui.add(egui::DragValue::new(&mut self.work.snap_x).range(1..=512));
            ui.label("Y");
            ui.add(egui::DragValue::new(&mut self.work.snap_y).range(1..=512));
            ui.checkbox(&mut self.show_grid, "Grid");
            ui.label("Room:");
            resource_picker(
                ui,
                "path_room",
                cx.project,
                ResourceKind::Room,
                &mut self.work.backdrop_room,
                "<no room>",
                -1,
            );
        });
        ui.separator();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(200.0);
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.work.name);
                });
                egui::ScrollArea::vertical().id_salt("pts").max_height(200.0).auto_shrink([false, true]).show(ui, |ui| {
                    for (i, p) in self.work.points.iter().enumerate() {
                        let t = format!("({}, {})   sp: {}", p.x, p.y, p.speed);
                        if ui.selectable_label(self.sel == Some(i), t).clicked() {
                            self.sel = Some(i);
                        }
                    }
                });
                if let Some(p) = self.sel.and_then(|i| self.work.points.get_mut(i)) {
                    ui.horizontal(|ui| {
                        ui.label("X");
                        ui.add(egui::DragValue::new(&mut p.x));
                        ui.label("Y");
                        ui.add(egui::DragValue::new(&mut p.y));
                        ui.label("sp");
                        ui.add(egui::DragValue::new(&mut p.speed));
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Add").clicked() {
                        self.snapshot();
                        let p = self.sel.and_then(|i| self.work.points.get(i).copied()).unwrap_or(PathPoint { x: 0.0, y: 0.0, speed: 100.0 });
                        self.work.points.push(p);
                        self.sel = Some(self.work.points.len() - 1);
                    }
                    if ui.add_enabled(self.sel.is_some(), egui::Button::new("Insert")).clicked() {
                        self.snapshot();
                        let i = self.sel.unwrap_or(0).min(self.work.points.len().saturating_sub(1));
                        if let Some(&p) = self.work.points.get(i) {
                            self.work.points.insert(i, p);
                        }
                    }
                    if ui.add_enabled(self.sel.is_some(), egui::Button::new("Delete")).clicked() {
                        if let Some(i) = self.sel.filter(|&i| i < self.work.points.len()) {
                            self.snapshot();
                            self.work.points.remove(i);
                            self.sel = None;
                        }
                    }
                });
                ui.separator();
                ui.label("Connection kind");
                ui.radio_value(&mut self.work.connection_kind, 0, "Straight lines");
                ui.radio_value(&mut self.work.connection_kind, 1, "Smooth curve");
                ui.checkbox(&mut self.work.closed, "Closed");
                ui.horizontal(|ui| {
                    ui.label("Precision:");
                    ui.add(egui::DragValue::new(&mut self.work.precision).range(1..=8));
                });
                ui.label(egui::RichText::new("Click: add · drag: move · Shift+click: insert · right-click: delete · Alt: no snap · middle-drag: pan · wheel: zoom").small().weak());
            });
            ui.separator();
            self.canvas(ui, cx);
        });
    }
    fn on_renamed(&mut self, name: &str) {
        self.work.name = name.to_string();
    }
    fn is_dirty(&self, p: &Project) -> bool {
        slot_dirty(p.paths.get(self.idx), &self.work)
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        commit_slot(p.paths.get_mut(self.idx), &self.work)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves() {
        let mut p = Path {
            closed: false,
            ..Default::default()
        };
        p.points = vec![
            PathPoint {
                x: 0.0,
                y: 0.0,
                speed: 100.0,
            },
            PathPoint {
                x: 10.0,
                y: 0.0,
                speed: 100.0,
            },
            PathPoint {
                x: 10.0,
                y: 10.0,
                speed: 100.0,
            },
        ];
        assert_eq!(curve(&p).len(), 3);
        p.connection_kind = 1;
        let c = curve(&p);
        assert_eq!(c.first(), Some(&(0.0, 0.0)));
        assert_eq!(c.last(), Some(&(10.0, 10.0)));
        p.closed = true;
        let c = curve(&p);
        assert!((c[0].0 - c.last().unwrap().0).abs() < 1e-9);
    }
}
