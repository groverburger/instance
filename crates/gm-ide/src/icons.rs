//! Original vector icons for the IDE chrome, painted with the egui painter.
//!
//! All artwork here is authored for this project (simple geometric shapes in a 16×16
//! design grid scaled to the target rect); no third-party images are used.

use eframe::egui::{
    self, epaint::PathShape, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Response,
    Sense, Shape, Stroke, Ui, Vec2,
};
use gm_format::ResourceKind;

/// Every icon the IDE can paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    New,
    Open,
    Save,
    CreateExe,
    Publish,
    Run,
    Debug,
    Sprite,
    Sound,
    Background,
    Path,
    Script,
    Font,
    Timeline,
    Object,
    Room,
    GameInfo,
    Settings,
    Extensions,
    Help,
    Folder,
    FolderOpen,
    Constants,
}

impl Icon {
    /// Icon of a resource kind (leaf items).
    pub fn of_kind(kind: ResourceKind) -> Icon {
        match kind {
            ResourceKind::Sprite => Icon::Sprite,
            ResourceKind::Sound => Icon::Sound,
            ResourceKind::Background => Icon::Background,
            ResourceKind::Path => Icon::Path,
            ResourceKind::Script => Icon::Script,
            ResourceKind::Font => Icon::Font,
            ResourceKind::Timeline => Icon::Timeline,
            ResourceKind::Object => Icon::Object,
            ResourceKind::Room => Icon::Room,
            ResourceKind::GameInfo => Icon::GameInfo,
            ResourceKind::Settings => Icon::Settings,
            ResourceKind::Extensions => Icon::Extensions,
            _ => Icon::Script,
        }
    }
}

const OUTLINE: Color32 = Color32::from_rgb(70, 70, 80);

/// Maps the 16×16 design grid into `rect`.
struct G {
    o: Pos2,
    s: f32,
}

impl G {
    fn new(rect: Rect) -> Self {
        let side = rect.width().min(rect.height());
        let o = rect.center() - Vec2::splat(side / 2.0);
        G { o, s: side / 16.0 }
    }
    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.o + vec2(x * self.s, y * self.s)
    }
    fn r(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.p(x0, y0), self.p(x1, y1))
    }
    fn w(&self, w: f32) -> f32 {
        (w * self.s).max(1.0)
    }
    fn stroke(&self, w: f32, c: Color32) -> Stroke {
        Stroke::new(self.w(w), c)
    }
    fn poly(&self, pts: &[(f32, f32)], fill: Color32, stroke: Stroke) -> Shape {
        Shape::Path(PathShape::convex_polygon(
            pts.iter().map(|&(x, y)| self.p(x, y)).collect(),
            fill,
            stroke,
        ))
    }
    fn line(&self, pts: &[(f32, f32)], stroke: Stroke) -> Shape {
        Shape::line(pts.iter().map(|&(x, y)| self.p(x, y)).collect(), stroke)
    }
}

/// A shaded ball: dark rim to light highlight towards the top-left.
fn ball(p: &Painter, g: &G, cx: f32, cy: f32, r: f32, dark: Color32, light: Color32) {
    let steps = 8;
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let rr = r * (1.0 - t * 0.85);
        let off = (r - rr) * 0.45;
        let c = lerp(dark, light, t);
        p.circle_filled(g.p(cx - off, cy - off), rr * g.s, c);
    }
    p.circle_stroke(
        g.p(cx, cy),
        r * g.s,
        Stroke::new(g.w(0.6), dark.gamma_multiply(0.8)),
    );
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

fn page(p: &Painter, g: &G) {
    p.add(g.poly(
        &[
            (3.0, 1.0),
            (10.0, 1.0),
            (13.0, 4.0),
            (13.0, 15.0),
            (3.0, 15.0),
        ],
        Color32::WHITE,
        g.stroke(1.0, OUTLINE),
    ));
    p.add(g.poly(
        &[(10.0, 1.0), (13.0, 4.0), (10.0, 4.0)],
        Color32::from_gray(215),
        g.stroke(0.8, OUTLINE),
    ));
}

fn folder(p: &Painter, g: &G, open: bool) {
    let back = Color32::from_rgb(222, 178, 52);
    let front = Color32::from_rgb(252, 214, 96);
    let edge = Color32::from_rgb(160, 118, 20);
    p.add(g.poly(
        &[
            (1.0, 3.5),
            (6.0, 3.5),
            (7.5, 5.0),
            (15.0, 5.0),
            (15.0, 13.5),
            (1.0, 13.5),
        ],
        back,
        g.stroke(0.8, edge),
    ));
    if open {
        p.add(g.poly(
            &[(3.5, 7.5), (16.0, 7.5), (13.5, 13.5), (1.0, 13.5)],
            front,
            g.stroke(0.8, edge),
        ));
    } else {
        p.rect(
            g.r(1.0, 6.5, 15.0, 13.5),
            0.0,
            front,
            g.stroke(0.8, edge),
            egui::StrokeKind::Inside,
        );
        p.line_segment(
            [g.p(1.6, 7.3), g.p(14.4, 7.3)],
            g.stroke(0.6, Color32::from_rgb(255, 236, 170)),
        );
    }
}

/// Paint `icon` into `rect`; `enabled = false` paints it washed out.
pub fn paint(p: &Painter, rect: Rect, icon: Icon, enabled: bool) {
    let p = if enabled {
        p.clone()
    } else {
        let mut q = p.clone();
        q.multiply_opacity(0.35);
        q
    };
    let p = &p;
    let g = G::new(rect);
    match icon {
        Icon::New => {
            page(p, &g);
            p.circle_filled(g.p(12.0, 12.5), 2.6 * g.s, Color32::from_rgb(250, 200, 40));
            p.circle_stroke(
                g.p(12.0, 12.5),
                2.6 * g.s,
                g.stroke(0.6, Color32::from_rgb(190, 130, 0)),
            );
        }
        Icon::Open => folder(p, &g, true),
        Icon::Save => {
            let body = Color32::from_rgb(52, 92, 170);
            p.rect(
                g.r(1.5, 1.5, 14.5, 14.5),
                g.w(1.0),
                body,
                g.stroke(0.8, Color32::from_rgb(20, 40, 90)),
                egui::StrokeKind::Inside,
            );
            p.rect_filled(g.r(3.5, 1.5, 12.5, 7.5), 0.0, Color32::WHITE);
            for y in [3.5, 5.5] {
                p.line_segment(
                    [g.p(4.5, y), g.p(11.5, y)],
                    g.stroke(0.6, Color32::from_gray(170)),
                );
            }
            p.rect_filled(g.r(4.5, 10.0, 11.5, 14.5), 0.0, Color32::from_gray(200));
            p.rect_filled(g.r(9.0, 11.0, 10.5, 13.5), 0.0, body);
        }
        Icon::CreateExe => {
            p.rect(
                g.r(1.0, 2.0, 15.0, 14.0),
                0.0,
                Color32::from_gray(245),
                g.stroke(1.0, OUTLINE),
                egui::StrokeKind::Inside,
            );
            p.rect_filled(
                g.r(1.5, 2.5, 14.5, 5.0),
                0.0,
                Color32::from_rgb(40, 90, 200),
            );
            p.rect_filled(
                g.r(12.0, 3.0, 14.0, 4.5),
                0.0,
                Color32::from_rgb(230, 80, 60),
            );
            // gear-ish cog
            let c = g.p(8.0, 9.5);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment(
                    [c + d * 1.5 * g.s, c + d * 3.6 * g.s],
                    g.stroke(1.6, Color32::from_gray(110)),
                );
            }
            p.circle_filled(c, 2.6 * g.s, Color32::from_gray(140));
            p.circle_filled(c, 1.1 * g.s, Color32::from_gray(245));
        }
        Icon::Publish => {
            ball(
                p,
                &g,
                8.0,
                8.0,
                6.8,
                Color32::from_rgb(20, 90, 170),
                Color32::from_rgb(150, 210, 255),
            );
            let s = g.stroke(0.7, Color32::from_rgba_unmultiplied(255, 255, 255, 170));
            p.line_segment([g.p(1.4, 8.0), g.p(14.6, 8.0)], s);
            p.line_segment([g.p(8.0, 1.3), g.p(8.0, 14.7)], s);
            p.add(Shape::ellipse_stroke(
                g.p(8.0, 8.0),
                vec2(3.0 * g.s, 6.6 * g.s),
                s,
            ));
            p.add(g.poly(
                &[(4.0, 5.0), (7.0, 4.0), (7.5, 7.0), (5.0, 9.0), (3.5, 7.5)],
                Color32::from_rgb(80, 170, 70),
                Stroke::NONE,
            ));
            p.add(g.poly(
                &[(9.5, 9.0), (12.5, 8.5), (12.0, 12.0), (10.0, 12.5)],
                Color32::from_rgb(80, 170, 70),
                Stroke::NONE,
            ));
        }
        Icon::Run | Icon::Debug => {
            let (fill, edge) = if icon == Icon::Run {
                (
                    Color32::from_rgb(40, 170, 60),
                    Color32::from_rgb(15, 100, 30),
                )
            } else {
                (
                    Color32::from_rgb(220, 60, 40),
                    Color32::from_rgb(130, 25, 15),
                )
            };
            p.add(g.poly(
                &[(3.5, 1.5), (14.0, 8.0), (3.5, 14.5)],
                fill,
                g.stroke(1.0, edge),
            ));
            p.add(g.poly(
                &[(5.0, 4.0), (9.0, 6.5), (5.0, 7.5)],
                fill.gamma_multiply(0.6).additive(),
                Stroke::NONE,
            ));
            if icon == Icon::Debug {
                p.circle_filled(g.p(12.5, 12.5), 2.8 * g.s, Color32::from_gray(40));
                p.circle_filled(g.p(12.5, 12.0), 1.4 * g.s, Color32::from_rgb(250, 210, 60));
            }
        }
        Icon::Sprite => {
            // a red "mouth" ball with a wedge cut out
            let c = g.p(8.0, 8.0);
            let r = 6.8 * g.s;
            let mut pts = vec![c];
            let n = 20;
            let (a0, a1) = (0.6_f32, std::f32::consts::TAU - 0.6);
            for i in 0..=n {
                let a = a0 + (a1 - a0) * i as f32 / n as f32;
                pts.push(c + vec2(a.cos(), -a.sin()) * r);
            }
            // filled as a triangle fan (shape is not convex as a whole)
            let fill = Color32::from_rgb(222, 40, 40);
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(c, fill);
            for (i, q) in pts.iter().enumerate().skip(1) {
                let t = (q.y - (c.y - r)) / (2.0 * r);
                mesh.colored_vertex(
                    *q,
                    lerp(
                        Color32::from_rgb(255, 120, 110),
                        Color32::from_rgb(170, 20, 20),
                        t,
                    ),
                );
                if i >= 2 {
                    mesh.add_triangle(0, (i - 1) as u32, i as u32);
                }
            }
            p.add(Shape::mesh(mesh));
            let mut outline = pts.clone();
            outline.push(c);
            p.add(Shape::line(
                outline,
                Stroke::new(g.w(0.7), Color32::from_rgb(120, 10, 10)),
            ));
            p.circle_filled(g.p(8.5, 4.6), 1.1 * g.s, Color32::from_gray(20));
        }
        Icon::Sound => {
            let body = Color32::from_gray(90);
            p.rect_filled(g.r(1.5, 6.0, 4.5, 10.0), 0.0, body);
            p.add(g.poly(
                &[(4.5, 6.0), (8.5, 2.5), (8.5, 13.5), (4.5, 10.0)],
                Color32::from_gray(130),
                g.stroke(0.7, Color32::from_gray(50)),
            ));
            let blue = Color32::from_rgb(40, 110, 220);
            for (r, w) in [(3.0, 1.1), (5.5, 1.1)] {
                let mut pts = Vec::new();
                for i in 0..=10 {
                    let a = -0.8 + 1.6 * i as f32 / 10.0;
                    pts.push(g.p(8.5 + r * a.cos(), 8.0 + r * a.sin()));
                }
                p.add(Shape::line(pts, g.stroke(w, blue)));
            }
        }
        Icon::Background => {
            p.rect_filled(
                g.r(1.0, 2.0, 15.0, 14.0),
                0.0,
                Color32::from_rgb(120, 180, 240),
            );
            p.circle_filled(g.p(11.5, 5.5), 1.8 * g.s, Color32::from_rgb(255, 220, 60));
            p.add(g.poly(
                &[
                    (1.0, 14.0),
                    (1.0, 10.0),
                    (5.0, 7.0),
                    (9.0, 10.5),
                    (9.0, 14.0),
                ],
                Color32::from_rgb(60, 150, 60),
                Stroke::NONE,
            ));
            p.add(g.poly(
                &[(6.0, 14.0), (11.0, 9.0), (15.0, 11.5), (15.0, 14.0)],
                Color32::from_rgb(40, 120, 45),
                Stroke::NONE,
            ));
            p.rect_stroke(
                g.r(1.0, 2.0, 15.0, 14.0),
                0.0,
                g.stroke(1.0, OUTLINE),
                egui::StrokeKind::Inside,
            );
        }
        Icon::Path => {
            p.rect(
                g.r(1.0, 1.0, 15.0, 15.0),
                0.0,
                Color32::WHITE,
                g.stroke(0.8, Color32::from_gray(150)),
                egui::StrokeKind::Inside,
            );
            let pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let t = i as f32 / 16.0;
                    g.p(
                        2.5 + 11.0 * t,
                        8.0 + 4.5 * (t * std::f32::consts::TAU).sin(),
                    )
                })
                .collect();
            p.add(Shape::line(
                pts,
                g.stroke(1.3, Color32::from_rgb(30, 90, 200)),
            ));
            for (x, y) in [(2.5, 8.0), (8.0, 8.0), (13.5, 8.0)] {
                p.circle_filled(g.p(x, y), 1.3 * g.s, Color32::from_rgb(220, 50, 40));
            }
        }
        Icon::Script => {
            page(p, &g);
            let cols = [
                Color32::from_rgb(30, 60, 180),
                Color32::from_gray(80),
                Color32::from_rgb(0, 130, 0),
                Color32::from_gray(80),
                Color32::from_rgb(170, 30, 30),
            ];
            for (i, c) in cols.iter().enumerate() {
                let y = 6.0 + i as f32 * 1.9;
                let x0 = if i % 2 == 1 { 6.0 } else { 4.8 };
                let x1 = [10.5, 11.5, 9.5, 11.0, 8.5][i];
                p.line_segment([g.p(x0, y), g.p(x1, y)], g.stroke(0.9, *c));
            }
        }
        Icon::Font => {
            p.rect(
                g.r(0.5, 1.0, 15.5, 15.0),
                g.w(1.0),
                Color32::WHITE,
                g.stroke(0.8, Color32::from_gray(150)),
                egui::StrokeKind::Inside,
            );
            p.text(
                g.p(5.2, 8.6),
                Align2::CENTER_CENTER,
                "T",
                FontId::proportional(13.5 * g.s),
                Color32::from_rgb(20, 40, 140),
            );
            p.text(
                g.p(11.2, 10.0),
                Align2::CENTER_CENTER,
                "t",
                FontId::proportional(10.5 * g.s),
                Color32::from_rgb(190, 30, 30),
            );
        }
        Icon::Timeline => {
            let wood = Color32::from_rgb(140, 90, 40);
            let glass = Color32::from_rgb(225, 240, 250);
            let sand = Color32::from_rgb(235, 190, 80);
            p.add(g.poly(
                &[(4.0, 2.5), (12.0, 2.5), (8.0, 8.0)],
                glass,
                g.stroke(0.7, Color32::from_gray(110)),
            ));
            p.add(g.poly(
                &[(8.0, 8.0), (12.0, 13.5), (4.0, 13.5)],
                glass,
                g.stroke(0.7, Color32::from_gray(110)),
            ));
            p.add(g.poly(&[(5.8, 5.0), (10.2, 5.0), (8.0, 8.0)], sand, Stroke::NONE));
            p.add(g.poly(
                &[(8.0, 10.5), (10.5, 13.5), (5.5, 13.5)],
                sand,
                Stroke::NONE,
            ));
            p.line_segment([g.p(8.0, 8.0), g.p(8.0, 13.0)], g.stroke(0.6, sand));
            p.rect_filled(g.r(2.5, 1.0, 13.5, 2.8), g.w(0.5), wood);
            p.rect_filled(g.r(2.5, 13.2, 13.5, 15.0), g.w(0.5), wood);
        }
        Icon::Object => {
            ball(
                p,
                &g,
                8.0,
                8.0,
                6.6,
                Color32::from_rgb(20, 60, 170),
                Color32::from_rgb(160, 205, 255),
            );
            p.circle_filled(
                g.p(5.8, 5.6),
                1.3 * g.s,
                Color32::from_rgba_unmultiplied(255, 255, 255, 200),
            );
        }
        Icon::Room => {
            p.rect(
                g.r(0.5, 2.0, 15.5, 14.0),
                0.0,
                Color32::WHITE,
                g.stroke(1.0, OUTLINE),
                egui::StrokeKind::Inside,
            );
            let grid = g.stroke(0.5, Color32::from_gray(200));
            for x in [4.25, 8.0, 11.75] {
                p.line_segment([g.p(x, 2.5), g.p(x, 13.5)], grid);
            }
            for y in [6.0, 10.0] {
                p.line_segment([g.p(1.0, y), g.p(15.0, y)], grid);
            }
            p.circle_filled(g.p(6.0, 8.0), 1.5 * g.s, Color32::from_rgb(40, 90, 210));
            p.rect_filled(
                g.r(9.5, 10.5, 14.0, 13.5),
                0.0,
                Color32::from_rgb(130, 90, 50),
            );
        }
        Icon::GameInfo => {
            ball(
                p,
                &g,
                8.0,
                8.0,
                6.8,
                Color32::from_rgb(20, 80, 190),
                Color32::from_rgb(110, 170, 255),
            );
            p.circle_filled(g.p(8.0, 4.6), 1.1 * g.s, Color32::WHITE);
            p.rect_filled(g.r(7.0, 6.6, 9.0, 12.2), 0.0, Color32::WHITE);
        }
        Icon::Settings => {
            // monitor with a cog
            p.rect(
                g.r(1.0, 1.5, 15.0, 11.5),
                g.w(0.8),
                Color32::from_rgb(200, 220, 245),
                g.stroke(1.0, OUTLINE),
                egui::StrokeKind::Inside,
            );
            p.rect_filled(g.r(6.5, 11.5, 9.5, 13.5), 0.0, Color32::from_gray(120));
            p.rect_filled(
                g.r(4.0, 13.5, 12.0, 15.0),
                g.w(0.5),
                Color32::from_gray(120),
            );
            let c = g.p(8.0, 6.5);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment(
                    [c + d * 1.2 * g.s, c + d * 3.6 * g.s],
                    g.stroke(1.5, Color32::from_gray(90)),
                );
            }
            p.circle_filled(c, 2.5 * g.s, Color32::from_gray(110));
            p.circle_filled(c, 1.0 * g.s, Color32::from_rgb(200, 220, 245));
        }
        Icon::Extensions => {
            // a package box
            let top = Color32::from_rgb(235, 190, 120);
            let left = Color32::from_rgb(200, 150, 80);
            let right = Color32::from_rgb(170, 120, 60);
            let e = g.stroke(0.6, Color32::from_rgb(100, 65, 20));
            p.add(g.poly(&[(8.0, 1.5), (15.0, 4.5), (8.0, 7.5), (1.0, 4.5)], top, e));
            p.add(g.poly(&[(1.0, 4.5), (8.0, 7.5), (8.0, 15.0), (1.0, 12.0)], left, e));
            p.add(g.poly(
                &[(8.0, 7.5), (15.0, 4.5), (15.0, 12.0), (8.0, 15.0)],
                right,
                e,
            ));
            p.add(g.line(
                &[(4.5, 3.0), (11.5, 6.0), (11.5, 9.0)],
                g.stroke(1.2, Color32::from_rgb(200, 40, 40)),
            ));
        }
        Icon::Help => {
            ball(
                p,
                &g,
                8.0,
                8.0,
                6.8,
                Color32::from_rgb(200, 140, 0),
                Color32::from_rgb(255, 230, 120),
            );
            p.text(
                g.p(8.0, 8.3),
                Align2::CENTER_CENTER,
                "?",
                FontId::proportional(11.5 * g.s),
                Color32::from_rgb(90, 50, 0),
            );
        }
        Icon::Folder => folder(p, &g, false),
        Icon::FolderOpen => folder(p, &g, true),
        Icon::Constants => {
            p.rect(
                g.r(1.0, 2.0, 15.0, 14.0),
                g.w(1.0),
                Color32::WHITE,
                g.stroke(0.8, OUTLINE),
                egui::StrokeKind::Inside,
            );
            p.text(
                g.p(8.0, 8.0),
                Align2::CENTER_CENTER,
                "=",
                FontId::monospace(12.0 * g.s),
                Color32::from_rgb(30, 60, 180),
            );
        }
    }
}

/// A flat toolbar button showing `icon`, raised on hover and sunken when pressed.
pub fn tool_button(ui: &mut Ui, icon: Icon, tip: &str, enabled: bool) -> Response {
    let size = Vec2::splat(24.0);
    let (rect, resp) = ui.allocate_exact_size(
        size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let resp = resp.on_hover_text(tip);
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        if enabled && (resp.hovered() || resp.is_pointer_button_down_on()) {
            let down = resp.is_pointer_button_down_on();
            p.rect(
                rect,
                2.0,
                if down {
                    Color32::from_rgb(204, 228, 247)
                } else {
                    Color32::from_rgb(229, 241, 251)
                },
                Stroke::new(1.0, Color32::from_rgb(0, 120, 215)),
                egui::StrokeKind::Inside,
            );
        }
        paint(p, rect.shrink(4.0), icon, enabled);
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tip));
    resp
}

/// Vertical etched separator for toolbars.
pub fn tool_separator(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(6.0, 24.0), Sense::hover());
    let x = rect.center().x.round();
    let p = ui.painter();
    p.line_segment(
        [
            pos2(x - 0.5, rect.top() + 2.0),
            pos2(x - 0.5, rect.bottom() - 2.0),
        ],
        Stroke::new(1.0, Color32::from_gray(160)),
    );
    p.line_segment(
        [
            pos2(x + 0.5, rect.top() + 2.0),
            pos2(x + 0.5, rect.bottom() - 2.0),
        ],
        Stroke::new(1.0, Color32::WHITE),
    );
}
