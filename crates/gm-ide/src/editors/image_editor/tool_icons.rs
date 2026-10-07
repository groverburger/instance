//! Original vector icons for the image editor's tool palette and toolbar, plus the
//! fixed-size icon buttons that show them.
//!
//! All artwork is authored for this project: simple shapes on a 16×16 design grid
//! scaled to the target rect, in the same flat-with-outline style as `crate::icons`.

use eframe::egui::{
    self, epaint::PathShape, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense,
    Shape, Stroke, StrokeKind, Ui, Vec2,
};

/// Every icon of the image editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TIcon {
    Pencil,
    Spray,
    Eraser,
    Picker,
    Line,
    Rect,
    Ellipse,
    Fill,
    ChangeColor,
    Text,
    Select,
    ZoomIn,
    ZoomOut,
    ZoomOne,
    ZoomFit,
    Undo,
    Redo,
    CloseSave,
    Prev,
    Next,
    Grid,
    Onion,
    Swap,
    RectOutline,
    RectFilled,
    RectBoth,
    EllipseOutline,
    EllipseFilled,
    EllipseBoth,
}

const INK: Color32 = Color32::from_rgb(40, 40, 48);
const OUTLINE: Color32 = Color32::from_rgb(70, 70, 80);

/// Maps the 16×16 design grid into a rect.
struct G {
    o: Pos2,
    s: f32,
}

impl G {
    fn new(rect: Rect) -> Self {
        let side = rect.width().min(rect.height());
        G {
            o: rect.center() - Vec2::splat(side / 2.0),
            s: side / 16.0,
        }
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
    fn st(&self, w: f32, c: Color32) -> Stroke {
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

/// A frame along a diagonal axis: `(along, across)` → design-grid point.
struct Axis {
    o: (f32, f32),
    u: (f32, f32),
    v: (f32, f32),
}

impl Axis {
    fn new(from: (f32, f32), to: (f32, f32)) -> Self {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let l = (dx * dx + dy * dy).sqrt();
        let u = (dx / l, dy / l);
        Axis {
            o: from,
            u,
            v: (-u.1, u.0),
        }
    }
    fn at(&self, a: f32, b: f32) -> (f32, f32) {
        (
            self.o.0 + self.u.0 * a + self.v.0 * b,
            self.o.1 + self.u.1 * a + self.v.1 * b,
        )
    }
    /// Quad spanning `a0..a1` along the axis with half-width `hw`.
    fn quad(&self, a0: f32, a1: f32, hw: f32) -> [(f32, f32); 4] {
        [
            self.at(a0, -hw),
            self.at(a1, -hw),
            self.at(a1, hw),
            self.at(a0, hw),
        ]
    }
}

fn ellipse_pts(g: &G, c: (f32, f32), rx: f32, ry: f32) -> Vec<Pos2> {
    (0..32)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 32.0;
            g.p(c.0 + rx * a.cos(), c.1 + ry * a.sin())
        })
        .collect()
}

fn ellipse(p: &Painter, g: &G, c: (f32, f32), rx: f32, ry: f32, fill: Color32, stroke: Stroke) {
    p.add(Shape::Path(PathShape::convex_polygon(
        ellipse_pts(g, c, rx, ry),
        fill,
        stroke,
    )));
}

/// Arc of a circle around `c` from angle `a0` to `a1` (radians, y down).
fn arc(c: (f32, f32), r: f32, a0: f32, a1: f32) -> Vec<(f32, f32)> {
    (0..=16)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / 16.0;
            (c.0 + r * a.cos(), c.1 + r * a.sin())
        })
        .collect()
}

fn arrow_head(p: &Painter, g: &G, tip: (f32, f32), dir: (f32, f32), size: f32, fill: Color32) {
    let l = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
    let u = (dir.0 / l, dir.1 / l);
    let v = (-u.1, u.0);
    let b = (tip.0 - u.0 * size, tip.1 - u.1 * size);
    p.add(g.poly(
        &[
            tip,
            (b.0 + v.0 * size * 0.75, b.1 + v.1 * size * 0.75),
            (b.0 - v.0 * size * 0.75, b.1 - v.1 * size * 0.75),
        ],
        fill,
        g.st(0.5, fill.gamma_multiply(0.7)),
    ));
}

fn magnifier(p: &Painter, g: &G) {
    // handle
    let ax = Axis::new((9.5, 9.5), (15.0, 15.0));
    p.add(g.poly(
        &ax.quad(0.5, 7.0, 1.3),
        Color32::from_rgb(120, 70, 30),
        g.st(0.6, Color32::from_rgb(60, 30, 10)),
    ));
    ellipse(
        p,
        g,
        (6.5, 6.5),
        5.0,
        5.0,
        Color32::from_rgb(225, 240, 252),
        g.st(1.3, Color32::from_rgb(60, 70, 90)),
    );
    p.circle_filled(
        g.p(5.0, 4.8),
        1.2 * g.s,
        Color32::from_rgba_unmultiplied(255, 255, 255, 220),
    );
}

fn shape_variant(p: &Painter, g: &G, ellipse_shape: bool, outline: bool, fill: bool) {
    let fc = Color32::from_rgb(110, 160, 220);
    let sc = Color32::from_rgb(20, 20, 30);
    let fill_c = if fill { fc } else { Color32::TRANSPARENT };
    let stroke = if outline { g.st(1.4, sc) } else { Stroke::NONE };
    if ellipse_shape {
        ellipse(p, g, (8.0, 8.0), 6.5, 5.0, fill_c, stroke);
    } else {
        let r = g.r(1.5, 3.0, 14.5, 13.0);
        p.rect(r, 0.0, fill_c, stroke, StrokeKind::Inside);
    }
}

/// Paint `icon` into `rect` (greyed out when disabled).
pub fn paint(p: &Painter, rect: Rect, icon: TIcon, enabled: bool) {
    let p = if enabled {
        p.clone()
    } else {
        let mut q = p.clone();
        q.multiply_opacity(0.35);
        q
    };
    let p = &p;
    let g = G::new(rect);
    let g = &g;
    match icon {
        TIcon::Pencil => {
            let ax = Axis::new((1.5, 14.5), (14.0, 2.0));
            // wooden cone + graphite point
            p.add(g.poly(
                &[ax.at(0.0, 0.0), ax.at(4.2, -2.1), ax.at(4.2, 2.1)],
                Color32::from_rgb(240, 205, 150),
                g.st(0.6, OUTLINE),
            ));
            p.add(g.poly(
                &[ax.at(0.0, 0.0), ax.at(1.6, -0.8), ax.at(1.6, 0.8)],
                INK,
                Stroke::NONE,
            ));
            // body: two-tone yellow
            p.add(g.poly(
                &ax.quad(4.2, 12.6, 2.1),
                Color32::from_rgb(250, 200, 40),
                g.st(0.6, OUTLINE),
            ));
            p.add(g.poly(
                &[
                    ax.at(4.2, -0.4),
                    ax.at(12.6, -0.4),
                    ax.at(12.6, 2.1),
                    ax.at(4.2, 2.1),
                ],
                Color32::from_rgb(215, 150, 20),
                Stroke::NONE,
            ));
            p.add(g.poly(
                &ax.quad(4.2, 12.6, 2.1),
                Color32::TRANSPARENT,
                g.st(0.6, OUTLINE),
            ));
            // ferrule + eraser
            p.add(g.poly(
                &ax.quad(12.6, 14.0, 2.1),
                Color32::from_rgb(190, 190, 200),
                g.st(0.6, OUTLINE),
            ));
            p.add(g.poly(
                &ax.quad(14.0, 16.6, 2.1),
                Color32::from_rgb(240, 130, 150),
                g.st(0.6, OUTLINE),
            ));
        }
        TIcon::Spray => {
            // can body
            p.rect(
                g.r(2.5, 6.0, 9.5, 15.0),
                g.w(1.0),
                Color32::from_rgb(70, 120, 200),
                g.st(0.8, Color32::from_rgb(25, 45, 90)),
                StrokeKind::Inside,
            );
            p.rect_filled(
                g.r(3.5, 7.0, 5.0, 14.0),
                0.0,
                Color32::from_rgb(150, 190, 240),
            );
            p.rect_filled(g.r(2.5, 9.0, 9.5, 11.5), 0.0, Color32::WHITE);
            p.rect_stroke(
                g.r(2.5, 6.0, 9.5, 15.0),
                g.w(1.0),
                g.st(0.8, Color32::from_rgb(25, 45, 90)),
                StrokeKind::Inside,
            );
            // shoulder + nozzle
            p.add(g.poly(
                &[(3.0, 6.0), (4.5, 4.0), (7.5, 4.0), (9.0, 6.0)],
                Color32::from_gray(190),
                g.st(0.7, OUTLINE),
            ));
            p.rect(
                g.r(5.0, 1.8, 7.5, 4.0),
                0.0,
                Color32::from_gray(225),
                g.st(0.7, OUTLINE),
                StrokeKind::Inside,
            );
            // spray dots
            let dots = [
                (10.0, 2.5),
                (12.0, 1.5),
                (14.0, 2.0),
                (11.0, 4.0),
                (13.0, 3.5),
                (15.0, 4.0),
                (12.5, 5.5),
                (14.5, 6.0),
                (10.5, 6.0),
                (13.5, 7.5),
            ];
            for (x, y) in dots {
                p.circle_filled(g.p(x, y), 0.55 * g.s, Color32::from_rgb(40, 90, 200));
            }
        }
        TIcon::Eraser => {
            let ax = Axis::new((2.5, 13.0), (13.5, 3.0));
            p.add(g.poly(
                &ax.quad(0.0, 6.5, 3.0),
                Color32::from_rgb(70, 110, 200),
                g.st(0.7, OUTLINE),
            ));
            p.add(g.poly(
                &ax.quad(6.5, 14.5, 3.0),
                Color32::from_rgb(245, 160, 175),
                g.st(0.7, OUTLINE),
            ));
            p.add(g.poly(
                &[
                    ax.at(0.0, -3.0),
                    ax.at(14.5, -3.0),
                    ax.at(14.5, -1.8),
                    ax.at(0.0, -1.8),
                ],
                Color32::from_rgba_unmultiplied(255, 255, 255, 110),
                Stroke::NONE,
            ));
            p.add(g.line(
                &[(1.0, 15.2), (9.0, 15.2)],
                g.st(0.8, Color32::from_gray(140)),
            ));
        }
        TIcon::Picker => {
            let ax = Axis::new((1.5, 14.5), (14.0, 2.0));
            // glass tube with liquid
            p.add(g.poly(
                &ax.quad(1.2, 10.0, 1.3),
                Color32::from_rgb(230, 240, 250),
                g.st(0.6, OUTLINE),
            ));
            p.add(g.poly(
                &ax.quad(1.2, 5.5, 1.3),
                Color32::from_rgb(200, 40, 60),
                g.st(0.6, OUTLINE),
            ));
            p.add(g.poly(
                &[ax.at(0.0, 0.0), ax.at(1.4, -0.7), ax.at(1.4, 0.7)],
                OUTLINE,
                Stroke::NONE,
            ));
            // collar + rubber bulb
            p.add(g.poly(
                &ax.quad(10.0, 11.5, 2.2),
                Color32::from_gray(150),
                g.st(0.6, OUTLINE),
            ));
            let c = ax.at(14.0, 0.0);
            ellipse(
                p,
                g,
                c,
                2.6,
                2.6,
                Color32::from_rgb(60, 60, 70),
                g.st(0.6, INK),
            );
            p.circle_filled(
                g.p(c.0 - 0.8, c.1 - 0.8),
                0.8 * g.s,
                Color32::from_gray(140),
            );
        }
        TIcon::Line => {
            p.add(g.line(&[(2.5, 13.5), (13.5, 2.5)], g.st(1.6, INK)));
            for (x, y) in [(2.5, 13.5), (13.5, 2.5)] {
                p.rect(
                    g.r(x - 1.4, y - 1.4, x + 1.4, y + 1.4),
                    0.0,
                    Color32::WHITE,
                    g.st(0.7, Color32::from_rgb(30, 80, 190)),
                    StrokeKind::Inside,
                );
            }
        }
        TIcon::Rect => {
            p.rect(
                g.r(1.5, 3.0, 14.5, 13.0),
                0.0,
                Color32::from_rgb(200, 225, 250),
                g.st(1.4, INK),
                StrokeKind::Inside,
            );
        }
        TIcon::Ellipse => {
            ellipse(
                p,
                g,
                (8.0, 8.0),
                6.6,
                5.2,
                Color32::from_rgb(200, 225, 250),
                g.st(1.4, INK),
            );
        }
        TIcon::Fill => {
            // tilted bucket
            p.add(g.poly(
                &[(2.0, 7.0), (8.5, 3.0), (12.5, 9.0), (6.5, 14.0)],
                Color32::from_gray(205),
                g.st(0.8, OUTLINE),
            ));
            p.add(g.poly(
                &[(2.0, 7.0), (8.5, 3.0), (9.6, 4.7), (3.2, 8.8)],
                Color32::from_gray(150),
                g.st(0.6, OUTLINE),
            ));
            // handle
            p.add(g.line(&arc((5.6, 6.0), 4.0, 3.6, 5.6), g.st(0.8, OUTLINE)));
            // pouring paint
            let blue = Color32::from_rgb(30, 110, 220);
            p.add(g.poly(
                &[(9.6, 4.7), (12.5, 9.0), (13.6, 12.0), (12.8, 6.0)],
                blue,
                Stroke::NONE,
            ));
            ellipse(p, g, (13.5, 13.4), 1.5, 1.9, blue, Stroke::NONE);
        }
        TIcon::ChangeColor => {
            p.rect(
                g.r(1.0, 1.0, 9.0, 9.0),
                0.0,
                Color32::from_rgb(220, 50, 50),
                g.st(0.8, OUTLINE),
                StrokeKind::Inside,
            );
            p.rect(
                g.r(7.0, 7.0, 15.0, 15.0),
                0.0,
                Color32::from_rgb(40, 110, 220),
                g.st(0.8, OUTLINE),
                StrokeKind::Inside,
            );
            // curved arrow from red to blue
            let a = arc((4.5, 11.5), 4.0, 3.3, 1.75);
            p.add(g.line(&a, g.st(1.2, INK)));
            arrow_head(p, g, (6.0, 15.4), (1.0, 0.0), 2.0, INK);
        }
        TIcon::Text => {
            p.text(
                g.p(7.0, 8.0),
                Align2::CENTER_CENTER,
                "A",
                FontId::proportional(15.0 * g.s),
                INK,
            );
            // text cursor
            p.add(g.line(
                &[(13.0, 2.5), (13.0, 13.5)],
                g.st(0.9, Color32::from_rgb(30, 80, 190)),
            ));
            p.add(g.line(
                &[(11.8, 2.5), (14.2, 2.5)],
                g.st(0.9, Color32::from_rgb(30, 80, 190)),
            ));
            p.add(g.line(
                &[(11.8, 13.5), (14.2, 13.5)],
                g.st(0.9, Color32::from_rgb(30, 80, 190)),
            ));
        }
        TIcon::Select => {
            let r = g.r(1.5, 1.5, 12.5, 12.5);
            p.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(51, 153, 255, 40));
            dashed_rect(p, g, r);
            // arrow cursor
            p.add(Shape::Path(PathShape::closed_line(
                [
                    (9.0, 8.0),
                    (9.0, 15.5),
                    (10.8, 13.7),
                    (12.2, 16.0),
                    (13.4, 15.4),
                    (12.1, 13.1),
                    (14.6, 13.1),
                ]
                .iter()
                .map(|&(x, y)| g.p(x, y))
                .collect(),
                g.st(0.7, INK),
            )));
            p.add(g.poly(
                &[(9.0, 8.0), (9.0, 15.5), (10.8, 13.7), (14.6, 13.1)],
                Color32::WHITE,
                Stroke::NONE,
            ));
            p.add(g.poly(
                &[(10.8, 13.7), (12.2, 16.0), (13.4, 15.4), (12.1, 13.1)],
                Color32::WHITE,
                Stroke::NONE,
            ));
            p.add(Shape::Path(PathShape::closed_line(
                [
                    (9.0, 8.0),
                    (9.0, 15.5),
                    (10.8, 13.7),
                    (12.2, 16.0),
                    (13.4, 15.4),
                    (12.1, 13.1),
                    (14.6, 13.1),
                ]
                .iter()
                .map(|&(x, y)| g.p(x, y))
                .collect(),
                g.st(0.7, INK),
            )));
        }
        TIcon::ZoomIn | TIcon::ZoomOut | TIcon::ZoomOne => {
            magnifier(p, g);
            let s = g.st(1.4, Color32::from_rgb(30, 60, 150));
            match icon {
                TIcon::ZoomIn => {
                    p.add(g.line(&[(4.0, 6.5), (9.0, 6.5)], s));
                    p.add(g.line(&[(6.5, 4.0), (6.5, 9.0)], s));
                }
                TIcon::ZoomOut => {
                    p.add(g.line(&[(4.0, 6.5), (9.0, 6.5)], s));
                }
                _ => {
                    p.text(
                        g.p(6.5, 6.7),
                        Align2::CENTER_CENTER,
                        "1",
                        FontId::proportional(8.5 * g.s),
                        Color32::from_rgb(30, 60, 150),
                    );
                }
            }
        }
        TIcon::ZoomFit => {
            p.rect(
                g.r(4.5, 4.5, 11.5, 11.5),
                0.0,
                Color32::from_rgb(200, 225, 250),
                g.st(0.8, OUTLINE),
                StrokeKind::Inside,
            );
            let c = Color32::from_rgb(30, 60, 150);
            for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let tip = (8.0 + sx * 4.6, 8.0 + sy * 4.6);
                let tail = (8.0 + sx * 7.4, 8.0 + sy * 7.4);
                p.add(g.line(&[tail, tip], g.st(1.1, c)));
                arrow_head(p, g, tip, (-sx, -sy), 2.0, c);
            }
        }
        TIcon::Undo | TIcon::Redo => {
            // a hooked arrow: along the top, round the side, back along the bottom
            let flip = icon == TIcon::Redo;
            let f = |(x, y): (f32, f32)| if flip { (16.0 - x, y) } else { (x, y) };
            let c = Color32::from_rgb(30, 90, 200);
            let mut pts = vec![f((5.0, 4.5))];
            pts.extend(
                arc(
                    (9.5, 8.25),
                    3.75,
                    -std::f32::consts::FRAC_PI_2,
                    std::f32::consts::FRAC_PI_2,
                )
                .into_iter()
                .map(f),
            );
            pts.push(f((4.0, 12.0)));
            p.add(g.line(&pts, g.st(1.9, c)));
            let dir = if flip { (1.0, 0.0) } else { (-1.0, 0.0) };
            arrow_head(p, g, f((1.0, 4.5)), dir, 3.6, c);
        }
        TIcon::CloseSave => {
            let c = Color32::from_rgb(20, 140, 40);
            p.add(g.line(&[(2.0, 8.5), (6.0, 12.5), (14.0, 3.5)], g.st(2.6, c)));
        }
        TIcon::Prev | TIcon::Next => {
            // a film frame with a big play-direction triangle
            p.rect(
                g.r(1.0, 2.0, 15.0, 14.0),
                g.w(0.8),
                Color32::from_gray(90),
                g.st(0.6, INK),
                StrokeKind::Inside,
            );
            for i in 0..5 {
                let x = 2.0 + i as f32 * 2.7;
                p.rect_filled(g.r(x, 2.8, x + 1.3, 4.0), 0.0, Color32::WHITE);
                p.rect_filled(g.r(x, 12.0, x + 1.3, 13.2), 0.0, Color32::WHITE);
            }
            p.rect_filled(
                g.r(1.6, 4.7, 14.4, 11.3),
                0.0,
                Color32::from_rgb(230, 240, 250),
            );
            let tri = if icon == TIcon::Next {
                [(5.5, 5.2), (11.5, 8.0), (5.5, 10.8)]
            } else {
                [(10.5, 5.2), (4.5, 8.0), (10.5, 10.8)]
            };
            p.add(g.poly(
                &tri,
                Color32::from_rgb(40, 170, 60),
                g.st(0.5, Color32::from_rgb(20, 100, 30)),
            ));
        }
        TIcon::Grid => {
            let r = g.r(1.5, 1.5, 14.5, 14.5);
            p.rect_filled(r, 0.0, Color32::WHITE);
            for i in 1..4 {
                let t = 1.5 + i as f32 * 13.0 / 4.0;
                let s = g.st(0.6, Color32::from_gray(150));
                p.add(g.line(&[(t, 1.5), (t, 14.5)], s));
                p.add(g.line(&[(1.5, t), (14.5, t)], s));
            }
            p.rect_filled(
                g.r(4.75, 4.75, 8.0, 8.0),
                0.0,
                Color32::from_rgb(40, 110, 220),
            );
            p.rect_stroke(r, 0.0, g.st(0.8, OUTLINE), StrokeKind::Inside);
        }
        TIcon::Onion => {
            p.rect(
                g.r(1.0, 1.0, 10.5, 10.5),
                0.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 150),
                g.st(0.8, Color32::from_gray(160)),
                StrokeKind::Inside,
            );
            p.circle_filled(
                g.p(5.5, 5.5),
                2.6 * g.s,
                Color32::from_rgba_unmultiplied(220, 60, 60, 90),
            );
            p.rect(
                g.r(5.5, 5.5, 15.0, 15.0),
                0.0,
                Color32::WHITE,
                g.st(0.8, OUTLINE),
                StrokeKind::Inside,
            );
            p.circle_filled(g.p(10.8, 10.2), 2.6 * g.s, Color32::from_rgb(220, 60, 60));
        }
        TIcon::Swap => {
            let c = Color32::from_rgb(30, 60, 150);
            p.add(g.line(&[(2.0, 5.0), (12.5, 5.0)], g.st(1.2, c)));
            arrow_head(p, g, (14.5, 5.0), (1.0, 0.0), 2.5, c);
            p.add(g.line(&[(14.0, 11.0), (3.5, 11.0)], g.st(1.2, c)));
            arrow_head(p, g, (1.5, 11.0), (-1.0, 0.0), 2.5, c);
        }
        TIcon::RectOutline => shape_variant(p, g, false, true, false),
        TIcon::RectFilled => shape_variant(p, g, false, false, true),
        TIcon::RectBoth => shape_variant(p, g, false, true, true),
        TIcon::EllipseOutline => shape_variant(p, g, true, true, false),
        TIcon::EllipseFilled => shape_variant(p, g, true, false, true),
        TIcon::EllipseBoth => shape_variant(p, g, true, true, true),
    }
}

fn dashed_rect(p: &Painter, g: &G, r: Rect) {
    let dash = 2.0 * g.s;
    let mut segs = Vec::new();
    for (a, b) in [
        (r.left_top(), r.right_top()),
        (r.right_top(), r.right_bottom()),
        (r.right_bottom(), r.left_bottom()),
        (r.left_bottom(), r.left_top()),
    ] {
        let len = (b - a).length();
        let d = (b - a) / len;
        let mut t = 0.0;
        let mut on = true;
        while t < len {
            let e = (t + dash).min(len);
            segs.push((a + d * t, a + d * e, on));
            on = !on;
            t = e;
        }
    }
    for (a, b, on) in segs {
        let c = if on { INK } else { Color32::WHITE };
        p.line_segment([a, b], Stroke::new(g.w(0.8), c));
    }
}

/// Paint the classic button chrome of an icon button into its fixed rect. Every state
/// draws inside `rect`, so hovering never changes the button's size.
fn chrome(p: &Painter, rect: Rect, hovered: bool, down: bool, selected: bool) {
    let r = rect.shrink(0.5);
    if selected || down {
        p.rect_filled(
            rect,
            0.0,
            if selected && !down {
                Color32::from_rgb(214, 232, 250)
            } else {
                Color32::from_rgb(196, 222, 246)
            },
        );
        // sunken bevel
        let (tl, br) = (Color32::from_gray(110), Color32::WHITE);
        p.line_segment([r.left_bottom(), r.left_top()], Stroke::new(1.0, tl));
        p.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, tl));
        p.line_segment([r.right_top(), r.right_bottom()], Stroke::new(1.0, br));
        p.line_segment([r.right_bottom(), r.left_bottom()], Stroke::new(1.0, br));
        if hovered {
            p.rect_stroke(
                rect.shrink(1.0),
                0.0,
                Stroke::new(1.0, Color32::from_rgb(0, 120, 215)),
                StrokeKind::Inside,
            );
        }
    } else if hovered {
        p.rect(
            rect,
            2.0,
            Color32::from_rgb(229, 241, 251),
            Stroke::new(1.0, Color32::from_rgb(0, 120, 215)),
            StrokeKind::Inside,
        );
    }
}

/// A fixed-size icon button. `selected` (for toggles and tools) draws it sunken.
/// `label` is the accessible name; `tip` the tooltip.
pub fn button(
    ui: &mut Ui,
    icon: TIcon,
    size: f32,
    label: &str,
    tip: &str,
    enabled: bool,
    selected: Option<bool>,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::splat(size),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let resp = resp.on_hover_text(tip);
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let down = enabled && resp.is_pointer_button_down_on();
        let sel = selected.unwrap_or(false);
        chrome(p, rect, enabled && resp.hovered(), down, sel);
        let pad = (size * 0.17).round();
        let off = if down || sel {
            vec2(1.0, 1.0)
        } else {
            Vec2::ZERO
        };
        paint(p, rect.shrink(pad).translate(off), icon, enabled);
    }
    resp.widget_info(|| match selected {
        Some(s) => egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, s, label),
        None => egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label),
    });
    resp
}

/// A fixed-size button showing a round brush of `size` pixels (for the size picker).
pub fn brush_button(ui: &mut Ui, size: u32, selected: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
    let label = format!("Brush size {size}");
    let resp = resp.on_hover_text(&label);
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        chrome(
            p,
            rect,
            resp.hovered(),
            resp.is_pointer_button_down_on(),
            selected,
        );
        let d = (size as f32 * 0.9 + 1.5).min(14.0);
        let c = rect.center();
        p.circle_filled(c, d / 2.0, INK);
    }
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, &label)
    });
    resp
}

/// A sunken panel (classic "well") around `add`.
pub fn well<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let frame = egui::Frame::new()
        .fill(Color32::from_gray(232))
        .inner_margin(egui::Margin::same(3));
    let resp = frame.show(ui, add);
    let r = resp.response.rect.shrink(0.5);
    let p = ui.painter();
    let (tl, br) = (Color32::from_gray(140), Color32::WHITE);
    p.line_segment([r.left_bottom(), r.left_top()], Stroke::new(1.0, tl));
    p.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, tl));
    p.line_segment([r.right_top(), r.right_bottom()], Stroke::new(1.0, br));
    p.line_segment([r.right_bottom(), r.left_bottom()], Stroke::new(1.0, br));
    resp.inner
}

/// A small group caption.
pub fn caption(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).small().weak());
}
