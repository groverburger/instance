//! Original vector icons for the room form toolbar, painted with the egui painter in a
//! 16×16 design grid (same style as `crate::icons`). No third-party artwork.

use eframe::egui::{
    self, epaint::PathShape, pos2, vec2, Color32, Painter, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomIcon {
    Undo,
    Clear,
    Shift,
    SortX,
    SortY,
    Lock,
    Unlock,
    Grid,
    IsoGrid,
    Show,
    ZoomOut,
    ZoomNormal,
    ZoomIn,
    Picker,
}

const OUTLINE: Color32 = Color32::from_rgb(70, 70, 80);

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
    fn st(&self, w: f32, c: Color32) -> Stroke {
        Stroke::new((w * self.s).max(1.0), c)
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

/// Filled arrow head at `tip` pointing along (dx, dy).
fn head(p: &Painter, g: &G, tip: (f32, f32), dir: (f32, f32), size: f32, c: Color32) {
    let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt().max(1e-3);
    let (ux, uy) = (dir.0 / len, dir.1 / len);
    let (nx, ny) = (-uy, ux);
    let back = (tip.0 - ux * size, tip.1 - uy * size);
    p.add(g.poly(
        &[
            tip,
            (back.0 + nx * size * 0.65, back.1 + ny * size * 0.65),
            (back.0 - nx * size * 0.65, back.1 - ny * size * 0.65),
        ],
        c,
        Stroke::NONE,
    ));
}

fn magnifier(p: &Painter, g: &G) {
    p.add(g.line(
        &[(9.6, 9.6), (14.5, 14.5)],
        g.st(2.6, Color32::from_rgb(120, 80, 40)),
    ));
    p.circle_filled(g.p(6.5, 6.5), 5.0 * g.s, Color32::from_rgb(205, 230, 250));
    p.circle_stroke(g.p(6.5, 6.5), 5.0 * g.s, g.st(1.3, Color32::from_gray(80)));
    p.add(g.line(
        &[(4.0, 5.2), (5.0, 3.9), (6.3, 3.4)],
        g.st(0.8, Color32::WHITE),
    ));
}

fn padlock(p: &Painter, g: &G, open: bool) {
    let shackle = g.st(1.5, Color32::from_gray(110));
    if open {
        p.add(g.line(
            &[
                (10.0, 7.5),
                (10.0, 4.0),
                (11.0, 2.2),
                (12.5, 1.6),
                (14.0, 2.2),
                (15.0, 4.0),
                (15.0, 5.5),
            ],
            shackle,
        ));
    } else {
        p.add(g.line(
            &[
                (4.5, 7.5),
                (4.5, 4.0),
                (5.5, 2.2),
                (8.0, 1.4),
                (10.5, 2.2),
                (11.5, 4.0),
                (11.5, 7.5),
            ],
            shackle,
        ));
    }
    let body = g.r(2.5, 7.0, 13.5, 15.0);
    p.rect(
        body,
        1.0 * g.s,
        Color32::from_rgb(236, 190, 50),
        g.st(0.8, Color32::from_rgb(150, 100, 10)),
        StrokeKind::Inside,
    );
    p.rect_filled(
        g.r(3.3, 7.8, 12.7, 9.0),
        0.0,
        Color32::from_rgb(255, 228, 130),
    );
    p.circle_filled(g.p(8.0, 10.8), 1.2 * g.s, Color32::from_rgb(90, 60, 10));
    p.rect_filled(
        g.r(7.5, 11.0, 8.5, 13.4),
        0.0,
        Color32::from_rgb(90, 60, 10),
    );
}

/// Paint `icon` into `rect`.
pub fn paint(p: &Painter, rect: Rect, icon: RoomIcon, enabled: bool) {
    let mut q = p.clone();
    if !enabled {
        q.multiply_opacity(0.35);
    }
    let p = &q;
    let g = G::new(rect);
    let blue = Color32::from_rgb(30, 90, 200);
    let green = Color32::from_rgb(30, 150, 50);
    match icon {
        RoomIcon::Undo => {
            // curved arrow sweeping left
            let pts: Vec<(f32, f32)> = (0..=12)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.05 - i as f32 / 12.0 * 1.1);
                    (8.5 + 5.0 * a.cos(), 10.0 - 5.0 * a.sin())
                })
                .collect();
            p.add(g.line(&pts, g.st(2.2, blue)));
            head(p, &g, (2.0, 12.5), (-0.6, 1.0), 4.2, blue);
        }
        RoomIcon::Clear => {
            p.rect(
                g.r(1.5, 1.5, 14.5, 14.5),
                0.0,
                Color32::WHITE,
                g.st(0.9, OUTLINE),
                StrokeKind::Inside,
            );
            let gs = g.st(0.5, Color32::from_gray(200));
            for v in [5.5, 10.5] {
                p.add(g.line(&[(v, 2.0), (v, 14.0)], gs));
                p.add(g.line(&[(2.0, v), (14.0, v)], gs));
            }
            let red = g.st(2.2, Color32::from_rgb(210, 30, 30));
            p.add(g.line(&[(4.0, 4.0), (12.0, 12.0)], red));
            p.add(g.line(&[(12.0, 4.0), (4.0, 12.0)], red));
        }
        RoomIcon::Shift => {
            let s = g.st(1.6, green);
            p.add(g.line(&[(8.0, 3.0), (8.0, 13.0)], s));
            p.add(g.line(&[(3.0, 8.0), (13.0, 8.0)], s));
            head(p, &g, (8.0, 0.8), (0.0, -1.0), 3.0, green);
            head(p, &g, (8.0, 15.2), (0.0, 1.0), 3.0, green);
            head(p, &g, (0.8, 8.0), (-1.0, 0.0), 3.0, green);
            head(p, &g, (15.2, 8.0), (1.0, 0.0), 3.0, green);
        }
        RoomIcon::SortX => {
            for (i, h) in [4.0f32, 7.0, 10.0].iter().enumerate() {
                let x = 2.0 + i as f32 * 4.5;
                p.rect(
                    g.r(x, 11.0 - h, x + 3.0, 11.0),
                    0.0,
                    Color32::from_rgb(120, 170, 235),
                    g.st(0.7, OUTLINE),
                    StrokeKind::Inside,
                );
            }
            p.add(g.line(&[(1.5, 13.5), (12.5, 13.5)], g.st(1.4, blue)));
            head(p, &g, (15.5, 13.5), (1.0, 0.0), 3.2, blue);
        }
        RoomIcon::SortY => {
            for (i, w) in [4.0f32, 7.0, 10.0].iter().enumerate() {
                let y = 1.5 + i as f32 * 4.5;
                p.rect(
                    g.r(1.0, y, 1.0 + w, y + 3.0),
                    0.0,
                    Color32::from_rgb(120, 170, 235),
                    g.st(0.7, OUTLINE),
                    StrokeKind::Inside,
                );
            }
            p.add(g.line(&[(13.5, 1.5), (13.5, 12.5)], g.st(1.4, blue)));
            head(p, &g, (13.5, 15.5), (0.0, 1.0), 3.2, blue);
        }
        RoomIcon::Lock => padlock(p, &g, false),
        RoomIcon::Unlock => padlock(p, &g, true),
        RoomIcon::Grid => {
            p.rect(
                g.r(1.5, 1.5, 14.5, 14.5),
                0.0,
                Color32::WHITE,
                g.st(0.9, OUTLINE),
                StrokeKind::Inside,
            );
            let gs = g.st(0.8, Color32::from_gray(90));
            for v in [4.75, 8.0, 11.25] {
                p.add(g.line(&[(v, 1.5), (v, 14.5)], gs));
                p.add(g.line(&[(1.5, v), (14.5, v)], gs));
            }
        }
        RoomIcon::IsoGrid => {
            p.rect(
                g.r(1.5, 1.5, 14.5, 14.5),
                0.0,
                Color32::WHITE,
                g.st(0.9, OUTLINE),
                StrokeKind::Inside,
            );
            let clip = p.with_clip_rect(g.r(1.6, 1.6, 14.4, 14.4));
            let gs = g.st(0.8, Color32::from_gray(90));
            for k in -3..=3 {
                let x0 = k as f32 * 6.0;
                clip.add(g.line(&[(x0 - 2.0, 0.0), (x0 + 14.0, 16.0)], gs));
                clip.add(g.line(&[(x0 + 18.0, 0.0), (x0 + 2.0, 16.0)], gs));
            }
        }
        RoomIcon::Show => {
            // eye
            let pts: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.0 * std::f32::consts::TAU;
                    // almond: pointed ends from the cubed cosine
                    let y = -t.sin() * 4.5;
                    g.p(
                        8.0 + 7.5 * t.cos().signum() * t.cos().abs().powf(0.8),
                        8.0 + y,
                    )
                })
                .collect();
            p.add(Shape::convex_polygon(
                pts,
                Color32::WHITE,
                g.st(0.9, OUTLINE),
            ));
            p.circle_filled(g.p(8.0, 8.0), 3.2 * g.s, Color32::from_rgb(40, 120, 200));
            p.circle_filled(g.p(8.0, 8.0), 1.5 * g.s, Color32::from_gray(20));
            p.circle_filled(g.p(7.0, 7.0), 0.7 * g.s, Color32::WHITE);
        }
        RoomIcon::ZoomOut | RoomIcon::ZoomIn | RoomIcon::ZoomNormal => {
            magnifier(p, &g);
            let s = g.st(1.5, Color32::from_gray(30));
            match icon {
                RoomIcon::ZoomOut => {
                    p.add(g.line(&[(4.0, 6.5), (9.0, 6.5)], s));
                }
                RoomIcon::ZoomIn => {
                    p.add(g.line(&[(4.0, 6.5), (9.0, 6.5)], s));
                    p.add(g.line(&[(6.5, 4.0), (6.5, 9.0)], s));
                }
                _ => {
                    p.add(g.line(&[(4.3, 5.4), (8.7, 5.4)], g.st(1.1, Color32::from_gray(30))));
                    p.add(g.line(&[(4.3, 7.6), (8.7, 7.6)], g.st(1.1, Color32::from_gray(30))));
                }
            }
        }
        RoomIcon::Picker => {
            // small "menu" glyph: a list with a drop arrow
            let s = g.st(1.2, OUTLINE);
            for y in [4.0, 7.0, 10.0] {
                p.add(g.line(&[(3.0, y), (10.0, y)], s));
            }
            head(p, &g, (13.0, 11.0), (0.0, 1.0), 3.0, Color32::BLACK);
        }
    }
}

fn bevel(p: &Painter, r: Rect, raised: bool) {
    let (tl, br) = if raised {
        (Color32::WHITE, Color32::from_gray(140))
    } else {
        (Color32::from_gray(140), Color32::WHITE)
    };
    p.line_segment([r.left_bottom(), r.left_top()], Stroke::new(1.0, tl));
    p.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, tl));
    p.line_segment([r.right_top(), r.right_bottom()], Stroke::new(1.0, br));
    p.line_segment([r.right_bottom(), r.left_bottom()], Stroke::new(1.0, br));
}

/// Fixed-size flat toolbar button. Hover/pressed/checked frames are drawn inside the
/// allocated rectangle, so the layout never changes with the state.
pub fn tool_button(
    ui: &mut Ui,
    icon: RoomIcon,
    tip: &str,
    enabled: bool,
    checked: bool,
    width: f32,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        vec2(width, 24.0),
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
        let hover = enabled && resp.hovered();
        let frame = rect.shrink(0.5);
        if checked || down {
            p.rect_filled(rect, 0.0, Color32::from_rgb(214, 230, 247));
            bevel(p, frame, false);
        } else if hover {
            p.rect(
                rect,
                2.0,
                Color32::from_rgb(229, 241, 251),
                Stroke::new(1.0, Color32::from_rgb(0, 120, 215)),
                StrokeKind::Inside,
            );
        }
        let off = if checked || down {
            vec2(1.0, 1.0)
        } else {
            Vec2::ZERO
        };
        let icon_r = Rect::from_min_size(rect.min + vec2(4.0, 4.0) + off, Vec2::splat(16.0));
        paint(p, icon_r, icon, enabled);
        if width > 28.0 {
            // drop-down arrow
            let c = pos2(rect.right() - 6.0, rect.center().y + 1.0) + off;
            p.add(Shape::convex_polygon(
                vec![
                    c + vec2(-3.0, -2.0),
                    c + vec2(3.0, -2.0),
                    c + vec2(0.0, 1.5),
                ],
                Color32::BLACK,
                Stroke::NONE,
            ));
        }
    }
    resp.widget_info(|| {
        if checked {
            egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, true, tip)
        } else {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tip)
        }
    });
    resp
}

/// Vertical etched separator.
pub fn separator(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(6.0, 24.0), Sense::hover());
    let x = rect.center().x.round();
    let p = ui.painter();
    p.vline(
        x - 0.5,
        rect.y_range().shrink(2.0),
        Stroke::new(1.0, Color32::from_gray(160)),
    );
    p.vline(
        x + 0.5,
        rect.y_range().shrink(2.0),
        Stroke::new(1.0, Color32::WHITE),
    );
}

/// Sunken 3D edge (classic "client edge").
pub fn sunken(p: &Painter, r: Rect) {
    bevel(p, r.shrink(0.5), false);
}

/// Raised 3D edge.
pub fn raised(p: &Painter, r: Rect) {
    bevel(p, r.shrink(0.5), true);
}
