//! Original icons for the built-in drag-and-drop actions and for the event types,
//! painted procedurally with the egui painter on a 32×32 design grid.
//!
//! All artwork here is authored for this project: bold, outlined glyphs in a small
//! fixed palette (arrows for motion, clocks for alarms, speakers for sounds, …). No
//! image data from any action library is used.

// drawing primitives take coordinates and colours as plain arguments
#![allow(clippy::too_many_arguments)]

use eframe::egui::{
    epaint::PathShape, vec2, Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke,
    Vec2,
};
use gm_format::{action_kind as ak, event_type as et};

const INK: Color32 = Color32::from_rgb(34, 34, 44);
const RED: Color32 = Color32::from_rgb(222, 44, 34);
const DARKRED: Color32 = Color32::from_rgb(150, 20, 16);
const BLUE: Color32 = Color32::from_rgb(38, 96, 214);
const LBLUE: Color32 = Color32::from_rgb(120, 180, 250);
const GREEN: Color32 = Color32::from_rgb(34, 160, 58);
const LGREEN: Color32 = Color32::from_rgb(130, 214, 110);
const ORANGE: Color32 = Color32::from_rgb(246, 142, 22);
const YELLOW: Color32 = Color32::from_rgb(252, 214, 46);
const PURPLE: Color32 = Color32::from_rgb(140, 72, 196);
const BROWN: Color32 = Color32::from_rgb(150, 96, 44);
const GREY: Color32 = Color32::from_rgb(140, 142, 150);
const LGREY: Color32 = Color32::from_rgb(214, 216, 222);
const WHITE: Color32 = Color32::WHITE;
const PINK: Color32 = Color32::from_rgb(232, 84, 150);

/// Maps the 32×32 design grid into a rect and offers drawing primitives.
struct D<'a> {
    p: &'a Painter,
    o: Pos2,
    s: f32,
}

impl<'a> D<'a> {
    fn new(p: &'a Painter, rect: Rect) -> Self {
        let side = rect.width().min(rect.height());
        D {
            p,
            o: rect.center() - Vec2::splat(side / 2.0),
            s: side / 32.0,
        }
    }
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        self.o + vec2(x * self.s, y * self.s)
    }
    fn w(&self, w: f32) -> f32 {
        (w * self.s).max(0.8)
    }
    fn st(&self, w: f32, c: Color32) -> Stroke {
        Stroke::new(self.w(w), c)
    }
    fn ink(&self) -> Stroke {
        self.st(1.2, INK)
    }
    fn poly(&self, pts: &[(f32, f32)], fill: Color32, stroke: Stroke) {
        self.p.add(Shape::Path(PathShape::convex_polygon(
            pts.iter().map(|&(x, y)| self.pt(x, y)).collect(),
            fill,
            stroke,
        )));
    }
    fn line(&self, pts: &[(f32, f32)], stroke: Stroke) {
        self.p.add(Shape::line(
            pts.iter().map(|&(x, y)| self.pt(x, y)).collect(),
            stroke,
        ));
    }
    fn closed(&self, pts: &[(f32, f32)], stroke: Stroke) {
        self.p.add(Shape::closed_line(
            pts.iter().map(|&(x, y)| self.pt(x, y)).collect(),
            stroke,
        ));
    }
    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, fill: Color32, stroke: Stroke) {
        self.p.rect(
            Rect::from_min_max(self.pt(x0, y0), self.pt(x1, y1)),
            self.s * 1.2,
            fill,
            stroke,
            eframe::egui::StrokeKind::Middle,
        );
    }
    fn sharp_rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, fill: Color32, stroke: Stroke) {
        self.p.rect(
            Rect::from_min_max(self.pt(x0, y0), self.pt(x1, y1)),
            0.0,
            fill,
            stroke,
            eframe::egui::StrokeKind::Middle,
        );
    }
    fn circle(&self, x: f32, y: f32, r: f32, fill: Color32, stroke: Stroke) {
        self.p.circle(self.pt(x, y), r * self.s, fill, stroke);
    }
    fn ellipse(&self, x: f32, y: f32, rx: f32, ry: f32, fill: Color32, stroke: Stroke) {
        self.p.add(Shape::ellipse_filled(
            self.pt(x, y),
            vec2(rx, ry) * self.s,
            fill,
        ));
        if stroke.width > 0.0 {
            self.p.add(Shape::ellipse_stroke(
                self.pt(x, y),
                vec2(rx, ry) * self.s,
                stroke,
            ));
        }
    }
    fn text(&self, x: f32, y: f32, s: &str, size: f32, c: Color32) {
        let font = FontId::proportional(size * self.s);
        // drawn twice, half a pixel apart, for a bold look
        self.p
            .text(self.pt(x, y), Align2::CENTER_CENTER, s, font.clone(), c);
        self.p.text(
            self.pt(x, y) + vec2(0.5, 0.0),
            Align2::CENTER_CENTER,
            s,
            font,
            c,
        );
    }
    /// Shaded ball, light at the top-left.
    fn ball(&self, x: f32, y: f32, r: f32, c: Color32) {
        let dark = scale(c, 0.55);
        let light = mix(c, WHITE, 0.6);
        let steps = 7;
        for i in 0..steps {
            let t = i as f32 / steps as f32;
            let rr = r * (1.0 - t * 0.8);
            let off = (r - rr) * 0.45;
            self.p.circle_filled(
                self.pt(x - off, y - off),
                rr * self.s,
                mix(dark, light, t * 0.9 + 0.1),
            );
        }
        self.p
            .circle_stroke(self.pt(x, y), r * self.s, self.st(1.0, scale(c, 0.35)));
    }
    /// A filled arrow from `a` to `b`, shaft width `w`, head length/width `hl`/`hw`.
    fn arrow(&self, a: (f32, f32), b: (f32, f32), w: f32, hl: f32, hw: f32, c: Color32) {
        let (a, b) = (vec2(a.0, a.1), vec2(b.0, b.1));
        let len = (b - a).length().max(0.01);
        let d = (b - a) / len;
        let n = vec2(-d.y, d.x);
        let base = b - d * hl.min(len);
        let q = |v: Vec2| (v.x, v.y);
        self.poly(
            &[
                q(a + n * w / 2.0),
                q(base + n * w / 2.0),
                q(base - n * w / 2.0),
                q(a - n * w / 2.0),
            ],
            c,
            Stroke::NONE,
        );
        self.poly(
            &[q(base + n * hw / 2.0), q(b), q(base - n * hw / 2.0)],
            c,
            Stroke::NONE,
        );
        self.closed(
            &[
                q(a + n * w / 2.0),
                q(base + n * w / 2.0),
                q(base + n * hw / 2.0),
                q(b),
                q(base - n * hw / 2.0),
                q(base - n * w / 2.0),
                q(a - n * w / 2.0),
            ],
            self.st(1.0, scale(c, 0.4)),
        );
    }
    /// A curved (arc) arrow around centre (cx, cy) with radius r from angle a0 to a1
    /// (degrees, clockwise on screen).
    fn arc_arrow(&self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32, w: f32, c: Color32) {
        let n = 14;
        let pts: Vec<(f32, f32)> = (0..=n)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        self.line(&pts, self.st(w + 2.0, scale(c, 0.4)));
        self.line(&pts, self.st(w, c));
        let e = a1.to_radians();
        let tip = (cx + r * e.cos(), cy + r * e.sin());
        let dir = if a1 > a0 { 1.0 } else { -1.0 };
        let t = vec2(-e.sin(), e.cos()) * dir;
        let nrm = vec2(e.cos(), e.sin());
        let hl = w * 1.6 + 2.0;
        let tipv = vec2(tip.0, tip.1) + t * hl * 0.6;
        let base = vec2(tip.0, tip.1) - t * hl * 0.4;
        let q = |v: Vec2| (v.x, v.y);
        self.poly(
            &[
                q(base + nrm * (w + 1.8)),
                q(tipv),
                q(base - nrm * (w + 1.8)),
            ],
            c,
            self.st(1.0, scale(c, 0.4)),
        );
    }
    fn gradient_rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, a: Color32, b: Color32, hor: bool) {
        let r = Rect::from_min_max(self.pt(x0, y0), self.pt(x1, y1));
        let mut m = Mesh::default();
        let (c0, c1, c2, c3) = if hor { (a, b, b, a) } else { (a, a, b, b) };
        m.colored_vertex(r.left_top(), c0);
        m.colored_vertex(r.right_top(), c1);
        m.colored_vertex(r.right_bottom(), c2);
        m.colored_vertex(r.left_bottom(), c3);
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        self.p.add(Shape::mesh(m));
        self.p
            .rect_stroke(r, 0.0, self.ink(), eframe::egui::StrokeKind::Middle);
    }
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

fn scale(c: Color32, k: f32) -> Color32 {
    let f = |x: u8| (x as f32 * k).round() as u8;
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

// ------------------------------------------------------------------ reusable glyphs

/// Eight arrows radiating from the centre.
fn star_arrows(d: &D, c: Color32, len: f32) {
    for k in 0..8 {
        let a = (k as f32 * 45.0).to_radians();
        let (dx, dy) = (a.cos(), a.sin());
        let l = if k % 2 == 1 { len * 0.9 } else { len };
        d.arrow(
            (16.0 + dx * 4.0, 16.0 + dy * 4.0),
            (16.0 + dx * l, 16.0 + dy * l),
            2.6,
            5.0,
            6.4,
            c,
        );
    }
}

fn speaker(d: &D, x: f32, y: f32, k: f32) {
    let p = |a: f32, b: f32| (x + a * k, y + b * k);
    d.poly(
        &[p(0.0, -3.5), p(4.0, -3.5), p(4.0, 3.5), p(0.0, 3.5)],
        GREY,
        d.ink(),
    );
    d.poly(
        &[p(4.0, -3.5), p(10.0, -9.0), p(10.0, 9.0), p(4.0, 3.5)],
        LGREY,
        d.ink(),
    );
}

fn sound_waves(d: &D, x: f32, y: f32, n: usize, c: Color32) {
    for i in 0..n {
        let r = 4.0 + i as f32 * 4.0;
        let pts: Vec<(f32, f32)> = (-4..=4)
            .map(|j| {
                let a = (j as f32 * 12.0).to_radians();
                (x + r * a.cos(), y + r * a.sin())
            })
            .collect();
        d.line(&pts, d.st(2.2, c));
    }
}

fn red_x(d: &D, x: f32, y: f32, r: f32) {
    for s in [d.st(4.6, WHITE), d.st(3.0, RED)] {
        d.line(&[(x - r, y - r), (x + r, y + r)], s);
        d.line(&[(x - r, y + r), (x + r, y - r)], s);
    }
}

/// Small round "?" badge at the bottom right (question actions).
fn q_badge(d: &D) {
    d.circle(24.5, 24.5, 6.8, YELLOW, d.st(1.2, scale(ORANGE, 0.6)));
    d.text(24.5, 24.7, "?", 11.0, INK);
}

/// Small pencil at the bottom right ("draw …" actions).
fn pencil_badge(d: &D) {
    d.poly(
        &[(19.0, 27.0), (27.0, 19.0), (30.0, 22.0), (22.0, 30.0)],
        YELLOW,
        d.ink(),
    );
    d.poly(&[(19.0, 27.0), (22.0, 30.0), (17.5, 31.5)], LGREY, d.ink());
    d.poly(
        &[(27.0, 19.0), (28.5, 17.5), (31.5, 20.5), (30.0, 22.0)],
        PINK,
        d.ink(),
    );
}

/// Small folder at the bottom right ("replace from file" actions).
fn folder_badge(d: &D) {
    d.poly(
        &[
            (17.0, 20.0),
            (22.0, 20.0),
            (23.5, 22.0),
            (31.0, 22.0),
            (31.0, 30.0),
            (17.0, 30.0),
        ],
        Color32::from_rgb(232, 186, 60),
        d.ink(),
    );
    d.sharp_rect(
        17.0,
        23.5,
        31.0,
        30.0,
        Color32::from_rgb(252, 218, 104),
        d.ink(),
    );
}

/// Explosion / burst star.
fn burst(d: &D, x: f32, y: f32, r: f32, outer: Color32, inner: Color32) {
    let n = 10;
    let pts: Vec<(f32, f32)> = (0..n * 2)
        .map(|i| {
            let a = (i as f32 * 180.0 / n as f32 - 90.0).to_radians();
            let rr = if i % 2 == 0 { r } else { r * 0.55 };
            (x + rr * a.cos(), y + rr * a.sin())
        })
        .collect();
    // concave: fill as triangles around the centre
    for i in 0..pts.len() {
        let j = (i + 1) % pts.len();
        d.poly(&[(x, y), pts[i], pts[j]], outer, Stroke::NONE);
    }
    d.closed(&pts, d.st(1.0, scale(outer, 0.45)));
    d.circle(x, y, r * 0.42, inner, Stroke::NONE);
}

fn star5(d: &D, x: f32, y: f32, r: f32, c: Color32) {
    let pts: Vec<(f32, f32)> = (0..10)
        .map(|i| {
            let a = (i as f32 * 36.0 - 90.0).to_radians();
            let rr = if i % 2 == 0 { r } else { r * 0.45 };
            (x + rr * a.cos(), y + rr * a.sin())
        })
        .collect();
    for i in 0..10 {
        let j = (i + 1) % 10;
        d.poly(&[(x, y), pts[i], pts[j]], c, Stroke::NONE);
    }
    d.closed(&pts, d.st(1.1, scale(c, 0.45)));
}

fn heart(d: &D, x: f32, y: f32, k: f32, c: Color32) {
    let p = |a: f32, b: f32| (x + a * k, y + b * k);
    d.circle(x - 3.6 * k, y - 2.5 * k, 4.2 * k, c, Stroke::NONE);
    d.circle(x + 3.6 * k, y - 2.5 * k, 4.2 * k, c, Stroke::NONE);
    d.poly(&[p(-7.6, -1.2), p(7.6, -1.2), p(0.0, 8.5)], c, Stroke::NONE);
    // outline
    let mut pts = Vec::new();
    let left: Vec<(f32, f32)> = (0..=9)
        .map(|i| {
            let a = (155.0 + i as f32 * 22.0).to_radians();
            p(-3.6 + 4.2 * a.cos(), -2.5 + 4.2 * a.sin())
        })
        .collect();
    let right: Vec<(f32, f32)> = (0..=9)
        .map(|i| {
            let a = (185.0 + i as f32 * 22.0).to_radians();
            p(3.6 + 4.2 * a.cos(), -2.5 + 4.2 * a.sin())
        })
        .collect();
    pts.push(p(0.0, 8.5));
    pts.extend(left);
    pts.extend(right);
    d.closed(&pts, d.st(1.0, scale(c, 0.45)));
    d.circle(
        x - 4.6 * k,
        y - 3.6 * k,
        1.3 * k,
        mix(c, WHITE, 0.6),
        Stroke::NONE,
    );
}

/// A small window (room) glyph.
fn room(d: &D, x0: f32, y0: f32, x1: f32, y1: f32) {
    d.sharp_rect(x0, y0, x1, y1, Color32::from_rgb(180, 220, 250), d.ink());
    d.sharp_rect(x0, y0, x1, y0 + 3.5, BLUE, d.ink());
    // floor and a block
    d.sharp_rect(x0 + 0.6, y1 - 3.0, x1 - 0.6, y1 - 0.6, LGREEN, Stroke::NONE);
    d.sharp_rect(x0 + 3.0, y1 - 7.0, x0 + 6.5, y1 - 3.0, BROWN, Stroke::NONE);
}

/// A horizontal ruler with ticks (time line).
fn ruler(d: &D, y: f32) {
    d.rect(
        2.0,
        y,
        30.0,
        y + 8.0,
        Color32::from_rgb(250, 236, 180),
        d.ink(),
    );
    for i in 0..7 {
        let x = 5.0 + i as f32 * 3.7;
        let h = if i % 2 == 0 { 4.0 } else { 2.5 };
        d.line(&[(x, y), (x, y + h)], d.st(1.0, INK));
    }
}

/// Clock face with hands.
fn clock(d: &D, x: f32, y: f32, r: f32, bells: bool) {
    if bells {
        d.circle(x - r * 0.75, y - r * 0.8, r * 0.38, RED, d.ink());
        d.circle(x + r * 0.75, y - r * 0.8, r * 0.38, RED, d.ink());
        d.line(
            &[(x - r * 0.6, y + r * 0.8), (x - r * 0.9, y + r * 1.15)],
            d.st(2.0, INK),
        );
        d.line(
            &[(x + r * 0.6, y + r * 0.8), (x + r * 0.9, y + r * 1.15)],
            d.st(2.0, INK),
        );
    }
    d.circle(x, y, r, if bells { RED } else { BLUE }, d.ink());
    d.circle(x, y, r * 0.78, WHITE, d.st(0.8, GREY));
    d.line(&[(x, y), (x, y - r * 0.6)], d.st(1.8, INK));
    d.line(&[(x, y), (x + r * 0.45, y + r * 0.15)], d.st(1.8, INK));
    d.circle(x, y, 1.0, INK, Stroke::NONE);
}

fn page(d: &D, x0: f32, y0: f32, x1: f32, y1: f32) {
    d.poly(
        &[(x0, y0), (x1 - 5.0, y0), (x1, y0 + 5.0), (x1, y1), (x0, y1)],
        WHITE,
        d.ink(),
    );
    d.poly(
        &[(x1 - 5.0, y0), (x1, y0 + 5.0), (x1 - 5.0, y0 + 5.0)],
        LGREY,
        d.ink(),
    );
}

fn text_lines(d: &D, x0: f32, x1: f32, y0: f32, n: usize, colors: &[Color32]) {
    for i in 0..n {
        let y = y0 + i as f32 * 3.6;
        let w = [1.0, 0.7, 0.85, 0.55, 0.9][i % 5];
        let c = colors[i % colors.len()];
        d.line(&[(x0, y), (x0 + (x1 - x0) * w, y)], d.st(1.6, c));
    }
}

/// Classic "sprite" figure: a round yellow chomper.
fn chomper(d: &D, x: f32, y: f32, r: f32) {
    let n = 16;
    let mut pts = vec![(x, y)];
    for i in 0..=n {
        let a = (35.0 + i as f32 * (290.0 / n as f32)).to_radians();
        pts.push((x + r * a.cos(), y + r * a.sin()));
    }
    for i in 1..pts.len() - 1 {
        d.poly(&[pts[0], pts[i], pts[i + 1]], YELLOW, Stroke::NONE);
    }
    d.closed(&pts, d.st(1.1, scale(ORANGE, 0.55)));
    d.circle(x - r * 0.1, y - r * 0.5, r * 0.14, INK, Stroke::NONE);
}

fn landscape(d: &D, x0: f32, y0: f32, x1: f32, y1: f32) {
    d.gradient_rect(
        x0,
        y0,
        x1,
        y1,
        Color32::from_rgb(110, 170, 240),
        Color32::from_rgb(200, 230, 255),
        false,
    );
    let h = y1 - y0;
    let w = x1 - x0;
    d.poly(
        &[
            (x0 + 0.5, y1 - 0.5),
            (x0 + w * 0.35, y0 + h * 0.35),
            (x0 + w * 0.7, y1 - 0.5),
        ],
        GREEN,
        Stroke::NONE,
    );
    d.poly(
        &[
            (x0 + w * 0.45, y1 - 0.5),
            (x0 + w * 0.72, y0 + h * 0.5),
            (x1 - 0.5, y1 - 0.5),
        ],
        scale(GREEN, 0.75),
        Stroke::NONE,
    );
    d.circle(x0 + w * 0.78, y0 + h * 0.28, h * 0.13, YELLOW, Stroke::NONE);
    d.sharp_rect(x0, y0, x1, y1, Color32::TRANSPARENT, d.ink());
}

/// A wavy path with dots.
fn path_curve(d: &D, c: Color32) {
    let pts: Vec<(f32, f32)> = (0..=20)
        .map(|i| {
            let t = i as f32 / 20.0;
            (
                4.0 + t * 24.0,
                18.0 - 9.0 * (t * std::f32::consts::TAU * 0.9).sin(),
            )
        })
        .collect();
    d.line(&pts, d.st(4.0, scale(c, 0.45)));
    d.line(&pts, d.st(2.4, c));
    for &i in &[0usize, 7, 14, 20] {
        d.circle(pts[i].0, pts[i].1, 2.2, WHITE, d.st(1.0, scale(c, 0.45)));
    }
}

fn footprint(d: &D, x: f32, y: f32, rot: f32, c: Color32) {
    footprint_k(d, x, y, rot, c, 1.0);
}

fn footprint_k(d: &D, x: f32, y: f32, rot: f32, c: Color32, k: f32) {
    let (s, co) = rot.to_radians().sin_cos();
    let r = |a: f32, b: f32| (x + (a * co - b * s) * k, y + (a * s + b * co) * k);
    let (px, py) = r(0.0, 0.0);
    d.ellipse(px, py, 2.6 * k, 3.6 * k, c, d.st(0.8, scale(c, 0.5)));
    let (hx, hy) = r(0.0, 5.8);
    d.ellipse(hx, hy, 1.9 * k, 1.9 * k, c, d.st(0.8, scale(c, 0.5)));
}

fn footsteps(d: &D, c: Color32) {
    footprint(d, 10.0, 21.0, 20.0, c);
    footprint(d, 18.0, 15.0, 20.0, c);
    footprint(d, 24.0, 7.0, 20.0, c);
}

fn dice(d: &D, x: f32, y: f32, k: f32) {
    d.rect(
        x - 6.0 * k,
        y - 6.0 * k,
        x + 6.0 * k,
        y + 6.0 * k,
        WHITE,
        d.ink(),
    );
    for (a, b) in [
        (-3.0, -3.0),
        (3.0, 3.0),
        (0.0, 0.0),
        (3.0, -3.0),
        (-3.0, 3.0),
    ] {
        d.circle(x + a * k, y + b * k, 1.3 * k, RED, Stroke::NONE);
    }
}

fn cd(d: &D, x: f32, y: f32, r: f32) {
    d.circle(x, y, r, Color32::from_rgb(220, 224, 236), d.ink());
    d.circle(
        x,
        y,
        r * 0.7,
        Color32::from_rgb(196, 214, 250),
        Stroke::NONE,
    );
    d.circle(
        x - r * 0.3,
        y - r * 0.3,
        r * 0.3,
        Color32::from_rgb(250, 220, 240),
        Stroke::NONE,
    );
    d.circle(x, y, r * 0.25, WHITE, d.ink());
}

fn particles(d: &D, x: f32, y: f32, k: f32) {
    let dots = [
        (0.0, 0.0, 2.6, YELLOW),
        (-6.0, -4.0, 1.8, ORANGE),
        (5.0, -6.0, 1.6, RED),
        (7.0, 3.0, 1.9, YELLOW),
        (-4.0, 6.0, 1.7, PINK),
        (2.0, 8.0, 1.3, ORANGE),
        (-8.0, 2.0, 1.2, RED),
        (8.0, -1.0, 1.0, PINK),
    ];
    for (a, b, r, c) in dots {
        d.circle(x + a * k, y + b * k, r * k, c, d.st(0.6, scale(c, 0.5)));
    }
}

fn sparkle(d: &D, x: f32, y: f32, r: f32, c: Color32) {
    d.poly(
        &[(x, y - r), (x + r * 0.25, y), (x, y + r), (x - r * 0.25, y)],
        c,
        d.st(0.8, scale(c, 0.5)),
    );
    d.poly(
        &[(x - r, y), (x, y - r * 0.25), (x + r, y), (x, y + r * 0.25)],
        c,
        d.st(0.8, scale(c, 0.5)),
    );
    d.circle(x, y, r * 0.22, WHITE, Stroke::NONE);
}

/// A small cannon-like particle emitter at the bottom left, aimed up-right.
fn emitter(d: &D) {
    d.poly(
        &[(5.0, 19.0), (12.0, 12.0), (19.0, 19.0), (12.0, 26.0)],
        GREY,
        d.ink(),
    );
    d.poly(
        &[(12.0, 12.0), (16.0, 8.0), (23.0, 15.0), (19.0, 19.0)],
        LGREY,
        d.ink(),
    );
    d.rect(2.0, 25.0, 17.0, 30.0, scale(BROWN, 0.9), d.ink());
}

fn bar(d: &D, y: f32, fill: f32) {
    d.sharp_rect(3.0, y, 29.0, y + 8.0, WHITE, d.ink());
    d.gradient_rect(3.0, y, 3.0 + 26.0 * fill, y + 8.0, RED, GREEN, true);
}

fn mouse(d: &D, x: f32, y: f32, k: f32) {
    d.ellipse(x, y, 6.0 * k, 8.5 * k, WHITE, d.ink());
    d.line(
        &[(x - 6.0 * k, y - 1.5 * k), (x + 6.0 * k, y - 1.5 * k)],
        d.st(1.0, INK),
    );
    d.line(&[(x, y - 8.5 * k), (x, y - 1.5 * k)], d.st(1.0, INK));
    d.sharp_rect(
        x - 5.2 * k,
        y - 6.5 * k,
        x - 0.5 * k,
        y - 2.0 * k,
        LBLUE,
        Stroke::NONE,
    );
    d.line(
        &[
            (x, y - 8.5 * k),
            (x + 3.0 * k, y - 12.0 * k),
            (x + 7.0 * k, y - 12.5 * k),
        ],
        d.st(1.2, INK),
    );
}

fn cursor(d: &D, x: f32, y: f32, k: f32) {
    let p = |a: f32, b: f32| (x + a * k, y + b * k);
    let pts = [
        p(0.0, 0.0),
        p(0.0, 16.0),
        p(4.0, 12.5),
        p(7.0, 19.0),
        p(9.5, 18.0),
        p(6.5, 11.5),
        p(11.5, 11.5),
    ];
    d.poly(&[pts[0], pts[1], pts[2], pts[6]], WHITE, Stroke::NONE);
    d.poly(&[pts[2], pts[3], pts[4], pts[5]], WHITE, Stroke::NONE);
    d.closed(&pts, d.st(1.2, INK));
}

fn bubble(d: &D, x0: f32, y0: f32, x1: f32, y1: f32) {
    d.rect(x0, y0, x1, y1, WHITE, d.ink());
    d.poly(
        &[
            (x0 + 5.0, y1 - 0.6),
            (x0 + 11.0, y1 - 0.6),
            (x0 + 4.0, y1 + 6.0),
        ],
        WHITE,
        Stroke::NONE,
    );
    d.line(
        &[(x0 + 5.0, y1), (x0 + 4.0, y1 + 6.0), (x0 + 11.0, y1)],
        d.st(1.2, INK),
    );
}

fn floppy(d: &D) {
    d.rect(5.0, 4.0, 27.0, 28.0, BLUE, d.ink());
    d.sharp_rect(9.0, 4.0, 23.0, 13.0, WHITE, d.ink());
    d.sharp_rect(19.0, 6.0, 21.5, 11.0, BLUE, Stroke::NONE);
    d.sharp_rect(9.0, 18.0, 23.0, 28.0, LGREY, d.ink());
}

fn var_box(d: &D, y: f32) {
    d.rect(
        2.0,
        y,
        30.0,
        y + 13.0,
        Color32::from_rgb(255, 250, 220),
        d.ink(),
    );
    d.text(16.0, y + 6.6, "VAR", 10.5, DARKRED);
}

fn block_bar(d: &D, top: bool) {
    let c = Color32::from_rgb(70, 110, 200);
    if top {
        d.rect(4.0, 5.0, 28.0, 10.0, c, d.ink());
        d.poly(&[(8.0, 12.0), (24.0, 12.0), (16.0, 26.0)], LBLUE, d.ink());
    } else {
        d.poly(&[(8.0, 20.0), (24.0, 20.0), (16.0, 6.0)], LBLUE, d.ink());
        d.rect(4.0, 22.0, 28.0, 27.0, c, d.ink());
    }
}

fn octagon(d: &D, x: f32, y: f32, r: f32, c: Color32) {
    let pts: Vec<(f32, f32)> = (0..8)
        .map(|i| {
            let a = (22.5 + i as f32 * 45.0).to_radians();
            (x + r * a.cos(), y + r * a.sin())
        })
        .collect();
    d.poly(&pts, c, d.st(1.2, scale(c, 0.45)));
    let inner: Vec<(f32, f32)> = pts
        .iter()
        .map(|&(a, b)| (x + (a - x) * 0.82, y + (b - y) * 0.82))
        .collect();
    d.closed(&inner, d.st(1.0, WHITE));
}

fn trophy(d: &D) {
    d.poly(
        &[(9.0, 4.0), (23.0, 4.0), (21.0, 14.0), (11.0, 14.0)],
        YELLOW,
        d.ink(),
    );
    d.ellipse(16.0, 14.0, 5.0, 3.0, YELLOW, Stroke::NONE);
    d.line(
        &[(9.0, 6.0), (5.0, 6.0), (6.0, 11.0), (11.0, 13.0)],
        d.st(1.5, scale(ORANGE, 0.7)),
    );
    d.line(
        &[(23.0, 6.0), (27.0, 6.0), (26.0, 11.0), (21.0, 13.0)],
        d.st(1.5, scale(ORANGE, 0.7)),
    );
    d.sharp_rect(14.5, 16.0, 17.5, 21.0, ORANGE, d.ink());
    d.rect(9.0, 21.0, 23.0, 26.0, BROWN, d.ink());
}

fn camera(d: &D) {
    d.rect(3.0, 9.0, 29.0, 26.0, Color32::from_rgb(70, 74, 84), d.ink());
    d.rect(
        10.0,
        5.0,
        19.0,
        9.5,
        Color32::from_rgb(90, 94, 104),
        d.ink(),
    );
    d.circle(16.0, 17.5, 6.5, LGREY, d.ink());
    d.circle(16.0, 17.5, 4.0, BLUE, Stroke::NONE);
    d.circle(14.6, 16.0, 1.3, WHITE, Stroke::NONE);
    d.circle(25.0, 12.5, 1.5, YELLOW, Stroke::NONE);
}

fn monitor(d: &D) {
    d.rect(3.0, 4.0, 29.0, 23.0, GREY, d.ink());
    d.sharp_rect(
        5.0,
        6.0,
        27.0,
        21.0,
        Color32::from_rgb(40, 60, 120),
        Stroke::NONE,
    );
    d.poly(
        &[(12.0, 23.0), (20.0, 23.0), (22.0, 28.0), (10.0, 28.0)],
        GREY,
        d.ink(),
    );
}

fn globe(d: &D, x: f32, y: f32, r: f32) {
    d.circle(x, y, r, LBLUE, d.ink());
    d.ellipse(x, y, r * 0.45, r, Color32::TRANSPARENT, d.st(1.0, BLUE));
    d.line(&[(x - r, y), (x + r, y)], d.st(1.0, BLUE));
    d.line(
        &[(x - r * 0.85, y - r * 0.5), (x + r * 0.85, y - r * 0.5)],
        d.st(0.8, BLUE),
    );
    d.line(
        &[(x - r * 0.85, y + r * 0.5), (x + r * 0.85, y + r * 0.5)],
        d.st(0.8, BLUE),
    );
    d.ellipse(
        x - r * 0.35,
        y - r * 0.15,
        r * 0.3,
        r * 0.4,
        GREEN,
        Stroke::NONE,
    );
}

fn hourglass(d: &D, x: f32, y: f32) {
    d.poly(
        &[(x - 4.5, y - 7.0), (x + 4.5, y - 7.0), (x, y)],
        LGREY,
        d.ink(),
    );
    d.poly(
        &[(x, y), (x + 4.5, y + 7.0), (x - 4.5, y + 7.0)],
        YELLOW,
        d.ink(),
    );
    d.line(&[(x - 5.5, y - 7.0), (x + 5.5, y - 7.0)], d.st(1.6, BROWN));
    d.line(&[(x - 5.5, y + 7.0), (x + 5.5, y + 7.0)], d.st(1.6, BROWN));
}

fn grid_glyph(d: &D) {
    d.sharp_rect(3.0, 3.0, 29.0, 29.0, WHITE, d.ink());
    for i in 1..4 {
        let v = 3.0 + i as f32 * 6.5;
        d.line(&[(v, 3.0), (v, 29.0)], d.st(0.8, GREY));
        d.line(&[(3.0, v), (29.0, v)], d.st(0.8, GREY));
    }
}

fn film(d: &D) {
    d.sharp_rect(4.0, 6.0, 28.0, 26.0, Color32::from_rgb(50, 50, 56), d.ink());
    for i in 0..5 {
        let x = 6.0 + i as f32 * 4.8;
        d.sharp_rect(x, 7.5, x + 2.4, 9.5, WHITE, Stroke::NONE);
        d.sharp_rect(x, 22.5, x + 2.4, 24.5, WHITE, Stroke::NONE);
    }
    d.sharp_rect(7.0, 11.0, 15.0, 21.0, LBLUE, Stroke::NONE);
    d.sharp_rect(17.0, 11.0, 25.0, 21.0, LBLUE, Stroke::NONE);
}

fn play(d: &D, x: f32, y: f32, r: f32) {
    d.poly(
        &[(x - r * 0.7, y - r), (x + r, y), (x - r * 0.7, y + r)],
        GREEN,
        d.ink(),
    );
}

fn stop_sq(d: &D, x: f32, y: f32, r: f32) {
    d.sharp_rect(x - r, y - r, x + r, y + r, RED, d.ink());
}

fn pause_bars(d: &D, x: f32, y: f32, r: f32) {
    d.sharp_rect(x - r, y - r, x - r * 0.25, y + r, BLUE, d.ink());
    d.sharp_rect(x + r * 0.25, y - r, x + r, y + r, BLUE, d.ink());
}

fn wall(d: &D, x0: f32, x1: f32) {
    d.sharp_rect(x0, 3.0, x1, 29.0, Color32::from_rgb(170, 120, 90), d.ink());
    for i in 0..4 {
        let y = 3.0 + i as f32 * 6.5;
        d.line(&[(x0, y), (x1, y)], d.st(0.8, scale(BROWN, 0.6)));
    }
}

fn crosshair(d: &D, x: f32, y: f32, r: f32, c: Color32) {
    d.circle(x, y, r, Color32::TRANSPARENT, d.st(1.6, c));
    d.line(&[(x - r - 2.0, y), (x + r + 2.0, y)], d.st(1.4, c));
    d.line(&[(x, y - r - 2.0), (x, y + r + 2.0)], d.st(1.4, c));
}

fn letter_a(d: &D, x: f32, y: f32, size: f32, c: Color32) {
    d.text(x, y, "A", size, c);
}

fn code_page(d: &D) {
    page(d, 5.0, 2.0, 27.0, 30.0);
    text_lines(d, 8.0, 23.0, 9.0, 6, &[BLUE, INK, GREEN, INK, RED]);
}

fn script_scroll(d: &D) {
    d.rect(
        6.0,
        4.0,
        26.0,
        28.0,
        Color32::from_rgb(252, 240, 200),
        d.ink(),
    );
    d.ellipse(
        6.0,
        6.0,
        3.0,
        2.5,
        Color32::from_rgb(232, 210, 150),
        d.ink(),
    );
    d.ellipse(
        26.0,
        26.0,
        3.0,
        2.5,
        Color32::from_rgb(232, 210, 150),
        d.ink(),
    );
    text_lines(d, 9.5, 23.0, 10.0, 5, &[BROWN, INK]);
}

// ------------------------------------------------------------------ action icons

/// Paint the icon of action `(lib_id, action_id)` into `rect`. `kind` and
/// `is_question` select a generic glyph for actions without a built-in icon.
pub fn paint_action(
    p: &Painter,
    rect: Rect,
    lib_id: i32,
    action_id: i32,
    kind: i32,
    is_question: bool,
) {
    let d = D::new(p, rect);
    let d = &d;
    if lib_id != 1 || !paint_builtin(d, action_id) {
        generic(d, kind, is_question);
    }
}

fn generic(d: &D, kind: i32, is_question: bool) {
    match kind {
        ak::BEGIN_GROUP => block_bar(d, true),
        ak::END_GROUP => block_bar(d, false),
        ak::CODE => code_page(d),
        ak::VARIABLE => var_box(d, 9.0),
        _ => {
            d.rect(5.0, 5.0, 27.0, 27.0, LGREY, d.ink());
            d.text(16.0, 16.0, "fx", 13.0, PURPLE);
            if is_question {
                q_badge(d);
            }
        }
    }
}

fn paint_builtin(d: &D, id: i32) -> bool {
    match id {
        // ---------------------------------------------------------- move
        101 => {
            star_arrows(d, RED, 14.5);
            d.sharp_rect(13.0, 13.0, 19.0, 19.0, RED, d.st(1.0, DARKRED));
        }
        102 => {
            star_arrows(d, BLUE, 14.5);
            d.circle(16.0, 16.0, 3.4, BLUE, d.st(1.0, scale(BLUE, 0.45)));
        }
        105 => {
            crosshair(d, 23.0, 9.0, 5.0, RED);
            d.arrow((4.0, 28.0), (19.5, 12.5), 3.4, 7.0, 9.0, BLUE);
        }
        103 => d.arrow_pair(true, GREEN),
        104 => d.arrow_pair(false, GREEN),
        107 => {
            d.ball(16.0, 8.0, 6.0, PURPLE);
            d.arrow((16.0, 14.0), (16.0, 25.0), 3.2, 6.0, 9.0, PURPLE);
            d.line(&[(4.0, 28.5), (28.0, 28.5)], d.st(2.0, BROWN));
        }
        113 => {
            d.sharp_rect(15.0, 4.0, 17.0, 28.0, GREY, Stroke::NONE);
            d.arrow((13.0, 11.0), (2.5, 11.0), 3.4, 6.0, 9.0, ORANGE);
            d.arrow((19.0, 21.0), (29.5, 21.0), 3.4, 6.0, 9.0, ORANGE);
        }
        114 => {
            d.sharp_rect(4.0, 15.0, 28.0, 17.0, GREY, Stroke::NONE);
            d.arrow((11.0, 13.0), (11.0, 2.5), 3.4, 6.0, 9.0, ORANGE);
            d.arrow((21.0, 19.0), (21.0, 29.5), 3.4, 6.0, 9.0, ORANGE);
        }
        108 => {
            d.sharp_rect(2.0, 23.0, 30.0, 29.0, LGREY, d.ink());
            for i in 0..7 {
                let x = 4.0 + i as f32 * 4.0;
                d.line(&[(x, 29.0), (x + 3.0, 23.0)], d.st(0.9, GREY));
            }
            d.rect(9.0, 12.0, 23.0, 23.0, ORANGE, d.ink());
            for (y, l) in [(14.5, 6.0), (17.5, 4.0), (20.5, 6.5)] {
                d.line(&[(7.0 - l, y), (7.0, y)], d.st(1.4, GREY));
            }
        }
        109 => {
            crosshair(d, 24.0, 22.0, 4.5, RED);
            jump_arc(d, (4.0, 24.0), (21.0, 18.0), GREEN);
        }
        110 => {
            // home flag
            d.line(&[(23.0, 29.0), (23.0, 12.0)], d.st(1.6, INK));
            d.poly(&[(23.0, 12.0), (30.0, 15.0), (23.0, 18.0)], GREEN, d.ink());
            jump_arc(d, (3.0, 26.0), (20.0, 22.0), GREEN);
        }
        111 => {
            dice(d, 24.0, 22.0, 0.75);
            jump_arc(d, (3.0, 26.0), (17.0, 17.0), GREEN);
        }
        117 => {
            grid_glyph(d);
            d.sharp_rect(9.5, 9.5, 22.5, 22.5, ORANGE, d.ink());
            d.arrow((27.0, 27.0), (23.5, 23.5), 1.6, 3.5, 4.5, BLUE);
        }
        112 => {
            d.sharp_rect(
                6.0,
                6.0,
                26.0,
                26.0,
                Color32::from_rgb(220, 236, 255),
                d.st(1.4, BLUE),
            );
            d.arrow((18.0, 16.0), (31.0, 16.0), 3.0, 5.0, 7.0, GREEN);
            d.arrow((1.0, 16.0), (12.0, 16.0), 3.0, 5.0, 7.0, GREEN);
        }
        116 => {
            wall(d, 22.0, 29.0);
            d.arrow((3.0, 16.0), (20.5, 16.0), 3.6, 6.5, 9.5, BLUE);
        }
        115 => {
            d.sharp_rect(2.0, 25.0, 30.0, 29.0, BROWN, d.ink());
            d.line(&[(4.0, 6.0), (15.0, 23.0), (26.0, 8.0)], d.st(1.4, GREY));
            d.arrow((22.5, 13.0), (26.5, 7.0), 1.4, 4.0, 5.0, GREY);
            d.ball(15.0, 19.0, 5.0, RED);
        }
        119 => path_curve(d, BLUE),
        124 => {
            path_curve(d, BLUE);
            octagon(d, 23.5, 24.0, 6.5, RED);
        }
        122 => {
            path_curve(d, BLUE);
            d.poly(&[(13.0, 2.0), (19.0, 2.0), (16.0, 8.0)], RED, d.ink());
        }
        123 => {
            path_curve(d, BLUE);
            d.arrow((14.0, 27.0), (24.0, 27.0), 2.0, 4.0, 6.0, ORANGE);
            d.arrow((19.0, 27.0), (29.0, 27.0), 2.0, 4.0, 6.0, ORANGE);
        }
        120 => {
            footsteps(d, BROWN);
            crosshair(d, 27.0, 26.0, 3.0, RED);
        }
        121 => {
            footprint(d, 6.0, 24.0, 40.0, BROWN);
            footprint(d, 9.0, 14.0, 10.0, BROWN);
            footprint(d, 17.0, 7.0, 80.0, BROWN);
            d.rect(17.0, 16.0, 26.0, 27.0, GREY, d.ink());
        }
        // ---------------------------------------------------------- main1
        201 => {
            d.ball(14.0, 18.0, 9.0, BLUE);
            sparkle(d, 24.0, 7.0, 6.0, YELLOW);
        }
        206 => {
            d.ball(10.0, 21.0, 7.5, BLUE);
            sparkle(d, 8.0, 7.0, 5.0, YELLOW);
            d.arrow((17.0, 15.0), (29.0, 6.0), 3.0, 6.0, 8.0, GREEN);
        }
        207 => {
            d.ball(12.0, 18.0, 8.5, BLUE);
            sparkle(d, 7.0, 6.0, 5.0, YELLOW);
            dice(d, 24.5, 9.0, 0.7);
        }
        202 => {
            d.ball(8.0, 22.0, 6.0, BLUE);
            d.ball(24.0, 22.0, 6.0, RED);
            d.arc_arrow(16.0, 18.0, 9.0, 200.0, 335.0, 2.2, GREEN);
        }
        203 => burst(d, 16.0, 16.0, 14.0, ORANGE, YELLOW),
        204 => {
            burst(d, 13.0, 18.0, 11.0, ORANGE, YELLOW);
            crosshair(d, 24.0, 8.0, 4.0, RED);
        }
        541 => {
            chomper(d, 12.0, 13.0, 9.0);
            d.arc_arrow(16.0, 16.0, 11.0, 30.0, 110.0, 2.0, BLUE);
        }
        542 => {
            chomper(d, 13.0, 15.0, 10.0);
            d.arrow((22.0, 27.0), (30.0, 30.0), 1.8, 3.5, 5.0, BLUE);
            d.arc_arrow(16.0, 16.0, 13.0, -60.0, 10.0, 1.8, GREEN);
        }
        543 => {
            chomper(d, 12.0, 13.0, 9.5);
            for (x, c) in [(18.0, RED), (23.0, GREEN), (28.0, BLUE)] {
                d.circle(x, 26.0, 3.0, c, d.ink());
            }
        }
        205 => {
            chomper(d, 13.0, 15.0, 10.0);
            d.text(25.0, 25.0, "×", 12.0, INK);
        }
        211 => {
            speaker(d, 3.0, 16.0, 1.0);
            sound_waves(d, 14.0, 16.0, 3, GREEN);
        }
        212 => {
            speaker(d, 3.0, 16.0, 1.0);
            red_x(d, 22.0, 16.0, 5.0);
        }
        213 => {
            speaker(d, 3.0, 13.0, 0.95);
            sound_waves(d, 13.0, 13.0, 2, GREEN);
            q_badge(d);
        }
        221 => {
            room(d, 9.0, 5.0, 30.0, 27.0);
            d.arrow((14.0, 17.0), (1.5, 17.0), 4.0, 6.5, 10.0, GREEN);
        }
        222 => {
            room(d, 2.0, 5.0, 23.0, 27.0);
            d.arrow((18.0, 17.0), (30.5, 17.0), 4.0, 6.5, 10.0, GREEN);
        }
        223 => {
            room(d, 3.0, 5.0, 29.0, 27.0);
            d.arc_arrow(16.0, 18.0, 6.0, -60.0, 220.0, 2.2, GREEN);
        }
        224 => {
            room(d, 13.0, 9.0, 31.0, 28.0);
            jump_arc(d, (1.0, 22.0), (14.0, 6.0), GREEN);
        }
        225 => {
            room(d, 9.0, 3.0, 30.0, 22.0);
            d.arrow((14.0, 13.0), (1.5, 13.0), 4.0, 6.5, 10.0, GREEN);
            q_badge(d);
        }
        226 => {
            room(d, 2.0, 3.0, 23.0, 22.0);
            d.arrow((14.0, 13.0), (26.5, 13.0), 4.0, 6.5, 10.0, GREEN);
            q_badge(d);
        }
        // ---------------------------------------------------------- main2
        301 => clock(d, 16.0, 17.0, 10.0, true),
        302 => {
            d.text(11.0, 20.0, "z", 12.0, BLUE);
            d.text(18.0, 13.0, "z", 15.0, BLUE);
            d.text(25.0, 6.0, "Z", 13.0, BLUE);
            d.circle(8.0, 9.0, 5.0, YELLOW, d.st(1.0, scale(ORANGE, 0.6)));
            d.circle(10.5, 7.0, 4.2, mix(LGREY, WHITE, 0.5), Stroke::NONE);
        }
        303 | 305 => {
            ruler(d, 18.0);
            clock(d, 16.0, 10.0, 7.0, false);
        }
        304 => {
            ruler(d, 15.0);
            d.poly(&[(14.0, 4.0), (22.0, 4.0), (18.0, 13.0)], RED, d.ink());
            d.line(&[(18.0, 13.0), (18.0, 25.0)], d.st(1.4, RED));
        }
        309 => {
            ruler(d, 4.0);
            d.arrow((4.0, 22.0), (17.0, 22.0), 3.0, 5.0, 8.0, ORANGE);
            d.arrow((14.0, 22.0), (28.0, 22.0), 3.0, 5.0, 8.0, ORANGE);
        }
        306 => {
            ruler(d, 4.0);
            play(d, 16.0, 22.0, 7.0);
        }
        307 => {
            ruler(d, 4.0);
            pause_bars(d, 16.0, 22.0, 6.5);
        }
        308 => {
            ruler(d, 4.0);
            stop_sq(d, 16.0, 22.0, 6.0);
        }
        321 => {
            bubble(d, 3.0, 4.0, 29.0, 22.0);
            text_lines(d, 7.0, 25.0, 9.0, 3, &[INK]);
        }
        322 => {
            d.circle(16.0, 16.0, 13.0, BLUE, d.st(1.2, scale(BLUE, 0.45)));
            d.circle(16.0, 9.0, 2.2, WHITE, Stroke::NONE);
            d.rect(13.8, 13.0, 18.2, 25.0, WHITE, Stroke::NONE);
        }
        323 => film(d),
        324 => {
            page(d, 6.0, 2.0, 26.0, 30.0);
            text_lines(d, 9.0, 22.0, 10.0, 5, &[INK]);
        }
        325 => landscape(d, 3.0, 6.0, 29.0, 26.0),
        326 => globe(d, 16.0, 16.0, 12.5),
        327 => film(d),
        328 => {
            d.sharp_rect(3.0, 5.0, 29.0, 27.0, WHITE, d.ink());
            d.sharp_rect(3.0, 5.0, 29.0, 9.5, BLUE, d.ink());
            for (i, on) in [(0, true), (1, false), (2, true)] {
                let y = 13.0 + i as f32 * 4.6;
                d.sharp_rect(6.0, y, 9.5, y + 3.5, WHITE, d.st(0.8, INK));
                if on {
                    d.line(
                        &[(6.5, y + 1.8), (7.6, y + 3.0), (9.2, y + 0.6)],
                        d.st(1.0, GREEN),
                    );
                }
                d.line(&[(12.0, y + 1.8), (25.0, y + 1.8)], d.st(1.4, GREY));
            }
        }
        331 => d.arc_arrow(16.0, 16.0, 10.0, -50.0, 250.0, 3.4, GREEN),
        332 => {
            d.circle(16.0, 16.0, 13.0, RED, d.st(1.2, DARKRED));
            d.rect(8.0, 13.5, 24.0, 18.5, WHITE, Stroke::NONE);
        }
        333 => floppy(d),
        334 => {
            folder_big(d);
            d.arrow((16.0, 3.0), (16.0, 14.0), 3.4, 5.0, 8.0, GREEN);
        }
        803 => {
            chomper(d, 11.0, 11.0, 9.0);
            folder_badge(d);
        }
        804 => {
            speaker(d, 2.0, 11.0, 0.9);
            sound_waves(d, 11.0, 11.0, 2, GREEN);
            folder_badge(d);
        }
        805 => {
            landscape(d, 2.0, 3.0, 22.0, 19.0);
            folder_badge(d);
        }
        // ---------------------------------------------------------- control
        401 => {
            d.sharp_rect(5.0, 5.0, 23.0, 23.0, WHITE, Stroke::NONE);
            for s in Shape::dashed_line(
                &[
                    d.pt(5.0, 5.0),
                    d.pt(23.0, 5.0),
                    d.pt(23.0, 23.0),
                    d.pt(5.0, 23.0),
                    d.pt(5.0, 5.0),
                ],
                d.st(1.4, INK),
                3.0 * d.s,
                2.0 * d.s,
            ) {
                d.p.add(s);
            }
            q_badge(d);
        }
        402 => {
            d.rect(3.0, 3.0, 16.0, 16.0, BLUE, d.ink());
            d.rect(10.0, 10.0, 23.0, 23.0, RED, d.ink());
            burst(d, 13.0, 13.0, 4.0, YELLOW, WHITE);
            q_badge(d);
        }
        403 => {
            d.ball(13.0, 13.0, 9.5, BLUE);
            q_badge(d);
        }
        404 => {
            d.ball(8.0, 9.0, 5.0, BLUE);
            d.ball(19.0, 8.0, 5.0, BLUE);
            d.ball(12.0, 18.0, 5.0, BLUE);
            q_badge(d);
        }
        405 => {
            dice(d, 13.0, 13.0, 1.4);
            q_badge(d);
        }
        407 => {
            bubble(d, 3.0, 3.0, 27.0, 20.0);
            d.text(15.0, 11.5, "?", 15.0, BLUE);
            d.text(25.5, 26.0, "Y/N", 7.5, INK);
        }
        408 => {
            d.rect(
                2.0,
                6.0,
                26.0,
                20.0,
                Color32::from_rgb(232, 240, 255),
                d.ink(),
            );
            d.text(14.0, 13.0, "a<b", 10.5, BLUE);
            q_badge(d);
        }
        409 => {
            mouse(d, 12.0, 15.0, 1.0);
            q_badge(d);
        }
        410 => {
            grid_glyph(d);
            d.sharp_rect(9.5, 9.5, 16.0, 16.0, ORANGE, d.ink());
            q_badge(d);
        }
        422 => block_bar(d, true),
        424 => block_bar(d, false),
        421 => {
            d.rect(
                2.0,
                9.0,
                30.0,
                23.0,
                Color32::from_rgb(220, 245, 220),
                d.st(1.2, GREEN),
            );
            d.text(16.0, 16.0, "ELSE", 10.0, scale(GREEN, 0.7));
        }
        425 => {
            octagon(d, 16.0, 16.0, 14.0, RED);
            d.text(16.0, 16.0, "EXIT", 8.5, WHITE);
        }
        423 => {
            d.arc_arrow(16.0, 16.0, 11.0, -70.0, 230.0, 3.0, BLUE);
            d.text(16.0, 16.5, "×n", 10.0, INK);
        }
        604 => {
            d.rect(9.0, 2.0, 23.0, 11.0, ORANGE, d.ink());
            d.rect(9.0, 21.0, 23.0, 30.0, BLUE, d.ink());
            d.arrow((16.0, 20.0), (16.0, 11.5), 2.8, 5.0, 8.0, GREEN);
        }
        603 => code_page(d),
        601 => script_scroll(d),
        605 => {
            d.poly(
                &[
                    (4.0, 4.0),
                    (28.0, 4.0),
                    (28.0, 22.0),
                    (22.0, 28.0),
                    (4.0, 28.0),
                ],
                YELLOW,
                d.ink(),
            );
            d.poly(
                &[(22.0, 28.0), (22.0, 22.0), (28.0, 22.0)],
                scale(YELLOW, 0.8),
                d.ink(),
            );
            text_lines(d, 7.5, 24.0, 9.0, 4, &[BROWN]);
        }
        611 => var_box(d, 9.5),
        612 => {
            var_box(d, 5.0);
            q_badge(d);
        }
        613 => {
            var_box(d, 4.0);
            pencil_badge(d);
        }
        // ---------------------------------------------------------- score
        701 => star5(d, 16.0, 16.5, 14.0, YELLOW),
        702 => {
            star5(d, 13.0, 13.0, 11.5, YELLOW);
            q_badge(d);
        }
        703 => {
            star5(d, 13.0, 13.0, 11.5, YELLOW);
            pencil_badge(d);
        }
        709 => trophy(d),
        707 => {
            trophy(d);
            red_x(d, 24.0, 24.0, 4.5);
        }
        711 => heart(d, 16.0, 16.0, 1.6, RED),
        712 => {
            heart(d, 13.0, 13.0, 1.3, RED);
            q_badge(d);
        }
        713 => {
            heart(d, 13.0, 13.0, 1.3, RED);
            pencil_badge(d);
        }
        714 => {
            heart(d, 7.0, 10.0, 0.75, RED);
            heart(d, 16.0, 10.0, 0.75, RED);
            heart(d, 25.0, 10.0, 0.75, RED);
            pencil_badge(d);
        }
        721 => {
            bar(d, 12.0, 0.7);
            d.text(16.0, 6.0, "+", 12.0, RED);
        }
        722 => {
            bar(d, 6.0, 0.6);
            q_badge(d);
        }
        723 => {
            bar(d, 6.0, 0.6);
            pencil_badge(d);
        }
        731 => {
            d.sharp_rect(2.0, 4.0, 30.0, 28.0, WHITE, d.ink());
            d.sharp_rect(2.0, 4.0, 30.0, 15.0, BLUE, d.ink());
            star5(d, 8.5, 9.5, 4.6, YELLOW);
            heart(d, 18.0, 9.8, 0.55, RED);
            d.sharp_rect(23.0, 8.0, 28.0, 11.5, LGREEN, d.st(0.8, INK));
            text_lines(d, 5.0, 26.0, 20.0, 2, &[GREY]);
        }
        // ---------------------------------------------------------- extra
        820 => {
            d.rect(3.0, 3.0, 29.0, 29.0, Color32::from_rgb(30, 34, 60), d.ink());
            particles(d, 16.0, 16.0, 1.2);
        }
        821 => {
            d.rect(3.0, 3.0, 29.0, 29.0, Color32::from_rgb(30, 34, 60), d.ink());
            particles(d, 14.0, 14.0, 1.0);
            red_x(d, 22.0, 22.0, 5.0);
        }
        822 => {
            d.rect(3.0, 3.0, 29.0, 29.0, Color32::from_rgb(30, 34, 60), d.ink());
            particles(d, 11.0, 12.0, 0.8);
            // eraser
            d.poly(
                &[(14.0, 24.0), (22.0, 16.0), (28.0, 22.0), (20.0, 30.0)],
                PINK,
                d.ink(),
            );
        }
        823 | 825 => {
            sparkle(d, 14.0, 16.0, 12.0, YELLOW);
            d.text(26.0, 6.0, "+", 13.0, GREEN);
        }
        824 => {
            sparkle(d, 10.0, 10.0, 8.0, RED);
            sparkle(d, 22.0, 13.0, 7.0, GREEN);
            sparkle(d, 14.0, 23.0, 7.0, BLUE);
        }
        826 => {
            sparkle(d, 11.0, 14.0, 10.0, YELLOW);
            hourglass(d, 24.0, 21.0);
        }
        827 => {
            sparkle(d, 9.0, 21.0, 8.0, YELLOW);
            d.arrow((14.0, 16.0), (29.0, 4.0), 2.6, 5.0, 7.0, BLUE);
        }
        828 => {
            sparkle(d, 12.0, 10.0, 9.0, YELLOW);
            d.arrow((22.0, 6.0), (22.0, 28.0), 2.8, 5.5, 8.0, PURPLE);
        }
        829 => {
            sparkle(d, 9.0, 10.0, 7.5, YELLOW);
            d.arrow((14.0, 14.0), (20.0, 20.0), 1.8, 4.0, 5.0, GREY);
            sparkle(d, 24.0, 24.0, 6.0, ORANGE);
        }
        831 => {
            emitter(d);
            particles(d, 25.0, 8.0, 0.75);
            d.text(6.0, 6.0, "+", 13.0, GREEN);
        }
        832 => {
            emitter(d);
            red_x(d, 25.0, 8.0, 5.0);
        }
        833 => {
            emitter(d);
            burst(d, 25.0, 8.0, 7.5, ORANGE, YELLOW);
        }
        834 => {
            emitter(d);
            for (i, c) in [(0, YELLOW), (1, ORANGE), (2, RED)] {
                let k = i as f32;
                let end = (26.0 + k * 2.0, 3.0 + k * 4.0);
                d.line(&[(21.0, 11.0), end], d.st(1.4, c));
                d.circle(end.0, end.1, 1.8, c, Stroke::NONE);
            }
        }
        808 => {
            cd(d, 13.0, 14.0, 11.0);
            play(d, 25.0, 25.0, 5.5);
        }
        809 => {
            cd(d, 13.0, 14.0, 11.0);
            stop_sq(d, 25.0, 25.0, 4.5);
        }
        810 => {
            cd(d, 13.0, 14.0, 11.0);
            pause_bars(d, 25.0, 25.0, 5.0);
        }
        811 => {
            cd(d, 13.0, 14.0, 11.0);
            d.arc_arrow(24.0, 24.0, 5.0, -60.0, 220.0, 1.8, GREEN);
        }
        812 => {
            cd(d, 13.0, 13.0, 11.0);
            q_badge(d);
        }
        813 => {
            cd(d, 12.0, 12.0, 10.0);
            play(d, 23.0, 7.0, 4.0);
            q_badge(d);
        }
        801 => {
            cursor(d, 8.0, 5.0, 1.15);
        }
        807 => globe(d, 16.0, 16.0, 12.5),
        // ---------------------------------------------------------- draw
        501 => {
            chomper(d, 13.0, 13.0, 10.5);
            pencil_badge(d);
        }
        502 => {
            landscape(d, 2.0, 3.0, 26.0, 22.0);
            pencil_badge(d);
        }
        514 => {
            letter_a(d, 13.0, 13.0, 24.0, BLUE);
            pencil_badge(d);
        }
        519 => {
            letter_a(d, 11.0, 12.0, 15.0, BLUE);
            letter_a(d, 18.0, 17.0, 26.0, mix(BLUE, WHITE, 0.25));
            d.arrow((3.0, 28.0), (12.0, 23.0), 1.6, 3.5, 5.0, GREEN);
        }
        511 => d.rect(4.0, 7.0, 28.0, 25.0, Color32::TRANSPARENT, d.st(2.6, BLUE)),
        516 => d.gradient_rect(3.0, 7.0, 29.0, 25.0, RED, YELLOW, true),
        517 => d.gradient_rect(3.0, 5.0, 29.0, 27.0, BLUE, LBLUE, false),
        512 => d.ellipse(16.0, 16.0, 12.5, 9.5, Color32::TRANSPARENT, d.st(2.6, RED)),
        518 => {
            for i in 0..8 {
                let t = i as f32 / 8.0;
                d.ellipse(
                    16.0,
                    16.0,
                    13.0 * (1.0 - t),
                    10.0 * (1.0 - t),
                    mix(PURPLE, PINK, t),
                    Stroke::NONE,
                );
            }
            d.ellipse(16.0, 16.0, 13.0, 10.0, Color32::TRANSPARENT, d.ink());
        }
        513 => d.line(&[(4.0, 27.0), (28.0, 5.0)], d.st(2.6, GREEN)),
        515 => d.arrow((4.0, 27.0), (28.0, 5.0), 2.4, 8.0, 10.0, GREEN),
        524 => {
            d.ellipse(
                16.0,
                17.0,
                13.5,
                11.0,
                Color32::from_rgb(232, 196, 140),
                d.ink(),
            );
            d.circle(21.0, 22.0, 2.6, WHITE, d.ink());
            for (x, y, c) in [
                (9.0, 13.0, RED),
                (15.0, 10.0, YELLOW),
                (21.5, 12.0, GREEN),
                (9.5, 20.0, BLUE),
            ] {
                d.circle(x, y, 2.9, c, d.st(0.8, INK));
            }
        }
        526 => {
            d.text(11.0, 15.0, "A", 22.0, INK);
            d.text(23.0, 18.0, "a", 15.0, RED);
            d.line(&[(4.0, 28.0), (28.0, 28.0)], d.st(1.4, GREY));
        }
        531 => {
            monitor(d);
            for (a, b) in [
                ((13.0, 11.0), (8.0, 8.0)),
                ((19.0, 11.0), (24.0, 8.0)),
                ((13.0, 16.0), (8.0, 19.0)),
                ((19.0, 16.0), (24.0, 19.0)),
            ] {
                d.arrow(a, b, 1.4, 3.0, 4.0, WHITE);
            }
        }
        802 => camera(d),
        532 => {
            burst(d, 12.0, 13.0, 10.0, PINK, WHITE);
            sparkle(d, 25.0, 8.0, 5.0, YELLOW);
            sparkle(d, 24.0, 24.0, 5.5, LBLUE);
        }
        _ => return false,
    }
    true
}

impl D<'_> {
    /// Two arrows pointing apart (horizontal or vertical speed).
    fn arrow_pair(&self, horizontal: bool, c: Color32) {
        if horizontal {
            self.arrow((16.0, 16.0), (2.0, 16.0), 4.0, 7.0, 11.0, c);
            self.arrow((16.0, 16.0), (30.0, 16.0), 4.0, 7.0, 11.0, c);
        } else {
            self.arrow((16.0, 16.0), (16.0, 2.0), 4.0, 7.0, 11.0, c);
            self.arrow((16.0, 16.0), (16.0, 30.0), 4.0, 7.0, 11.0, c);
        }
    }
}

/// A dashed jump arc from `a` to `b` with an arrow head.
fn jump_arc(d: &D, a: (f32, f32), b: (f32, f32), c: Color32) {
    let mid = ((a.0 + b.0) / 2.0, a.1.min(b.1) - 12.0);
    let pts: Vec<Pos2> = (0..=16)
        .map(|i| {
            let t = i as f32 / 16.0;
            let u = 1.0 - t;
            d.pt(
                u * u * a.0 + 2.0 * u * t * mid.0 + t * t * b.0,
                u * u * a.1 + 2.0 * u * t * mid.1 + t * t * b.1,
            )
        })
        .collect();
    for s in Shape::dashed_line(&pts, d.st(2.6, c), 3.0 * d.s, 2.0 * d.s) {
        d.p.add(s);
    }
    let n = pts.len();
    let dir = (pts[n - 1] - pts[n - 3]).normalized();
    let tip = pts[n - 1] + dir * 3.0 * d.s;
    let nrm = vec2(-dir.y, dir.x);
    let base = pts[n - 1] - dir * 3.0 * d.s;
    d.p.add(Shape::convex_polygon(
        vec![base + nrm * 4.0 * d.s, tip, base - nrm * 4.0 * d.s],
        c,
        d.st(1.0, scale(c, 0.4)),
    ));
}

fn folder_big(d: &D) {
    d.poly(
        &[
            (2.0, 12.0),
            (11.0, 12.0),
            (13.0, 15.0),
            (30.0, 15.0),
            (30.0, 29.0),
            (2.0, 29.0),
        ],
        Color32::from_rgb(232, 186, 60),
        d.ink(),
    );
    d.sharp_rect(
        2.0,
        18.0,
        30.0,
        29.0,
        Color32::from_rgb(252, 218, 104),
        d.ink(),
    );
}

// ------------------------------------------------------------------ event icons

/// Paint the icon of an event type (and sub-event number) into `rect`.
pub fn paint_event(p: &Painter, rect: Rect, event_type: i32, number: i32) {
    let d = D::new(p, rect);
    let d = &d;
    match event_type {
        et::CREATE => {
            // light bulb
            d.circle(16.0, 12.0, 10.0, YELLOW, d.st(1.6, scale(ORANGE, 0.6)));
            d.circle(13.0, 9.0, 3.0, mix(YELLOW, WHITE, 0.7), Stroke::NONE);
            d.poly(
                &[(11.0, 19.0), (21.0, 19.0), (20.0, 24.0), (12.0, 24.0)],
                YELLOW,
                Stroke::NONE,
            );
            d.rect(11.0, 23.0, 21.0, 30.0, GREY, d.ink());
            d.line(&[(11.0, 26.5), (21.0, 26.5)], d.st(1.2, INK));
        }
        et::DESTROY => burst(d, 16.0, 16.0, 15.0, RED, YELLOW),
        et::ALARM => clock(d, 16.0, 17.5, 11.0, true),
        et::STEP => {
            let c = match number {
                1 => GREEN,
                2 => RED,
                _ => BROWN,
            };
            footprint_k(d, 9.0, 15.0, 12.0, c, 1.9);
            footprint_k(d, 22.0, 8.0, 12.0, c, 1.9);
        }
        et::COLLISION => {
            d.arrow((1.0, 16.0), (15.0, 16.0), 6.0, 8.0, 15.0, RED);
            d.arrow((31.0, 16.0), (17.0, 16.0), 6.0, 8.0, 15.0, RED);
        }
        et::KEYBOARD | et::KEY_PRESS | et::KEY_RELEASE => {
            d.rect(3.0, 5.0, 29.0, 27.0, LGREY, d.st(1.6, INK));
            d.rect(6.0, 7.0, 26.0, 21.0, WHITE, Stroke::NONE);
            match event_type {
                et::KEY_PRESS => d.arrow((16.0, 8.0), (16.0, 20.0), 3.6, 6.0, 11.0, GREEN),
                et::KEY_RELEASE => d.arrow((16.0, 20.0), (16.0, 8.0), 3.6, 6.0, 11.0, RED),
                _ => d.text(16.0, 14.0, "K", 14.0, BLUE),
            }
        }
        et::MOUSE => mouse(d, 15.0, 19.0, 1.35),
        et::OTHER => match number {
            0 | 1 | 40..=47 | 50..=57 => {
                d.sharp_rect(3.0, 3.0, 22.0, 22.0, LBLUE, d.st(1.6, BLUE));
                d.arrow((14.0, 14.0), (30.0, 30.0), 4.0, 8.0, 11.0, ORANGE);
            }
            2 | 3 => {
                d.line(&[(8.0, 31.0), (8.0, 3.0)], d.st(2.4, INK));
                d.poly(
                    &[(8.0, 3.0), (28.0, 9.0), (8.0, 16.0)],
                    if number == 2 { GREEN } else { RED },
                    d.ink(),
                );
            }
            4 | 5 => {
                room(d, 2.0, 4.0, 30.0, 28.0);
            }
            6 => heart(d, 16.0, 16.0, 1.8, RED),
            9 => bar(d, 12.0, 0.15),
            7 => {
                d.poly(
                    &[(16.0, 2.0), (30.0, 16.0), (16.0, 30.0), (2.0, 16.0)],
                    GREEN,
                    d.st(1.6, scale(GREEN, 0.45)),
                );
                d.poly(
                    &[(16.0, 7.0), (25.0, 16.0), (16.0, 25.0), (7.0, 16.0)],
                    LGREEN,
                    Stroke::NONE,
                );
            }
            8 => path_curve(d, BLUE),
            30 => {
                d.rect(3.0, 3.0, 29.0, 29.0, RED, d.st(1.6, DARKRED));
                let s = d.st(4.0, WHITE);
                d.line(&[(9.0, 9.0), (23.0, 23.0)], s);
                d.line(&[(9.0, 23.0), (23.0, 9.0)], s);
            }
            10..=25 => {
                d.circle(16.0, 9.0, 6.5, ORANGE, d.ink());
                d.poly(
                    &[(5.0, 30.0), (8.0, 18.0), (24.0, 18.0), (27.0, 30.0)],
                    BLUE,
                    d.ink(),
                );
            }
            _ => {
                d.poly(
                    &[(16.0, 2.0), (30.0, 16.0), (16.0, 30.0), (2.0, 16.0)],
                    ORANGE,
                    d.ink(),
                );
            }
        },
        et::DRAW => {
            d.poly(
                &[(4.0, 24.0), (22.0, 6.0), (27.0, 11.0), (9.0, 29.0)],
                YELLOW,
                d.st(1.4, INK),
            );
            d.poly(
                &[(4.0, 24.0), (9.0, 29.0), (1.5, 31.0)],
                Color32::from_rgb(240, 210, 170),
                d.st(1.4, INK),
            );
            d.poly(
                &[(22.0, 6.0), (25.0, 3.0), (30.0, 8.0), (27.0, 11.0)],
                PINK,
                d.st(1.4, INK),
            );
        }
        et::TRIGGER => {
            d.poly(
                &[(18.0, 1.0), (6.0, 18.0), (15.0, 18.0)],
                YELLOW,
                Stroke::NONE,
            );
            d.poly(
                &[(15.0, 18.0), (6.0, 18.0), (12.0, 14.0), (26.0, 14.0)],
                YELLOW,
                Stroke::NONE,
            );
            d.poly(
                &[(17.0, 14.0), (26.0, 14.0), (13.0, 31.0)],
                YELLOW,
                Stroke::NONE,
            );
            d.closed(
                &[
                    (18.0, 1.0),
                    (6.0, 18.0),
                    (15.0, 18.0),
                    (13.0, 31.0),
                    (26.0, 14.0),
                    (17.0, 14.0),
                ],
                d.st(1.4, scale(ORANGE, 0.6)),
            );
        }
        _ => d.circle(16.0, 16.0, 10.0, GREY, d.ink()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_builtin_icon(action_id: i32) -> bool {
        let ctx = eframe::egui::Context::default();
        let mut ok = false;
        let mut out = ctx.run_ui(Default::default(), |ui| {
            let p = ui.painter().clone();
            let d = D::new(&p, Rect::from_min_size(Pos2::ZERO, Vec2::splat(32.0)));
            ok = paint_builtin(&d, action_id);
        });
        out.textures_delta.clear();
        ok
    }

    /// Every built-in palette action has an authored icon.
    #[test]
    fn every_builtin_action_has_an_icon() {
        let mut missing = Vec::new();
        for l in gm_format::builtin_libraries() {
            for a in &l.actions {
                if !a.is_decoration() && !has_builtin_icon(a.action_id) {
                    missing.push((a.action_id, a.palette_title.clone()));
                }
            }
        }
        assert!(missing.is_empty(), "no icon for {missing:?}");
    }
}
