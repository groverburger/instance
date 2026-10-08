//! Small original vector icons (a 16×16 design grid scaled to the target rect) and
//! the flat toolbar button / separator of the classic toolbars.
//!
//! Toolbar buttons have a fixed size and paint their hover frame inside it, so
//! hovering never changes the layout.

use egui::{
    epaint::PathShape, pos2, vec2, Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke,
    Ui, Vec2,
};

/// Generic glyphs shared by the IDE and the runner's windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// Green triangle (run / continue).
    Play,
    Pause,
    /// Triangle and bar (single step).
    Step,
    /// Circular arrow.
    Restart,
    /// Red square.
    Stop,
    /// Green plus.
    Add,
    /// Pencil.
    Edit,
    /// Red cross.
    Delete,
    /// Waste bin.
    Clear,
    /// Red disc with a white cross.
    Error,
    /// Yellow triangle with an exclamation mark.
    Warning,
    /// Two overlapping pages.
    Copy,
    /// Floppy disk.
    Save,
    /// A page with text lines (output log).
    Log,
}

const OUTLINE: Color32 = Color32::from_rgb(70, 70, 80);

/// Maps the 16×16 design grid into a rect.
pub struct Grid {
    pub o: Pos2,
    pub s: f32,
}

impl Grid {
    pub fn new(rect: Rect) -> Self {
        let side = rect.width().min(rect.height());
        let o = rect.center() - Vec2::splat(side / 2.0);
        Grid { o, s: side / 16.0 }
    }
    pub fn p(&self, x: f32, y: f32) -> Pos2 {
        self.o + vec2(x * self.s, y * self.s)
    }
    pub fn r(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.p(x0, y0), self.p(x1, y1))
    }
    pub fn w(&self, w: f32) -> f32 {
        (w * self.s).max(1.0)
    }
    pub fn stroke(&self, w: f32, c: Color32) -> Stroke {
        Stroke::new(self.w(w), c)
    }
    pub fn poly(&self, pts: &[(f32, f32)], fill: Color32, stroke: Stroke) -> Shape {
        Shape::Path(PathShape::convex_polygon(
            pts.iter().map(|&(x, y)| self.p(x, y)).collect(),
            fill,
            stroke,
        ))
    }
    pub fn line(&self, pts: &[(f32, f32)], stroke: Stroke) -> Shape {
        Shape::line(pts.iter().map(|&(x, y)| self.p(x, y)).collect(), stroke)
    }
}

/// Washed-out version of a colour (disabled icons).
pub fn disabled(c: Color32) -> Color32 {
    let g = ((c.r() as u32 + c.g() as u32 + c.b() as u32) / 3) as u8;
    let g = 160 + g / 4;
    Color32::from_rgba_unmultiplied(g, g, g, c.a())
}

/// Paint `glyph` into `rect`; `enabled = false` paints it washed out.
pub fn paint(p: &Painter, rect: Rect, glyph: Glyph, enabled: bool) {
    let g = Grid::new(rect);
    let c = |c: Color32| if enabled { c } else { disabled(c) };
    let outline = c(OUTLINE);
    match glyph {
        Glyph::Play => {
            p.add(g.poly(
                &[(4.0, 2.0), (13.5, 8.0), (4.0, 14.0)],
                c(Color32::from_rgb(40, 170, 60)),
                g.stroke(0.8, c(Color32::from_rgb(20, 100, 30))),
            ));
        }
        Glyph::Pause => {
            for x in [3.5, 9.5] {
                p.rect(
                    g.r(x, 2.5, x + 3.5, 13.5),
                    0.0,
                    c(Color32::from_rgb(60, 110, 200)),
                    g.stroke(0.8, c(Color32::from_rgb(30, 60, 130))),
                    egui::StrokeKind::Inside,
                );
            }
        }
        Glyph::Step => {
            p.add(g.poly(
                &[(2.5, 2.5), (10.0, 8.0), (2.5, 13.5)],
                c(Color32::from_rgb(40, 170, 60)),
                g.stroke(0.8, c(Color32::from_rgb(20, 100, 30))),
            ));
            p.rect(
                g.r(11.0, 2.5, 13.5, 13.5),
                0.0,
                c(Color32::from_rgb(60, 110, 200)),
                g.stroke(0.8, c(Color32::from_rgb(30, 60, 130))),
                egui::StrokeKind::Inside,
            );
        }
        Glyph::Restart => {
            let col = c(Color32::from_rgb(30, 110, 200));
            let pts: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let a = (40.0 + 290.0 * i as f32 / 20.0).to_radians();
                    g.p(8.0 + 5.0 * a.cos(), 8.5 - 5.0 * a.sin())
                })
                .collect();
            p.add(Shape::line(pts, g.stroke(2.0, col)));
            // arrow head at the start of the arc (top right)
            p.add(g.poly(&[(9.5, 1.0), (14.5, 3.0), (10.5, 7.0)], col, Stroke::NONE));
        }
        Glyph::Stop => {
            p.rect(
                g.r(3.0, 3.0, 13.0, 13.0),
                1.0,
                c(Color32::from_rgb(210, 40, 40)),
                g.stroke(0.8, c(Color32::from_rgb(130, 20, 20))),
                egui::StrokeKind::Inside,
            );
        }
        Glyph::Add => {
            let col = c(Color32::from_rgb(30, 150, 50));
            // (concave: filled as two bars, outlined as a closed line)
            p.rect_filled(g.r(6.5, 2.0, 9.5, 14.0), 0.0, col);
            p.rect_filled(g.r(2.0, 6.5, 14.0, 9.5), 0.0, col);
            p.add(Shape::closed_line(
                [
                    (6.5, 2.0),
                    (9.5, 2.0),
                    (9.5, 6.5),
                    (14.0, 6.5),
                    (14.0, 9.5),
                    (9.5, 9.5),
                    (9.5, 14.0),
                    (6.5, 14.0),
                    (6.5, 9.5),
                    (2.0, 9.5),
                    (2.0, 6.5),
                    (6.5, 6.5),
                ]
                .iter()
                .map(|&(x, y)| g.p(x, y))
                .collect(),
                g.stroke(0.7, c(Color32::from_rgb(15, 90, 30))),
            ));
        }
        Glyph::Edit => {
            p.add(g.poly(
                &[(11.0, 1.5), (14.5, 5.0), (5.5, 14.0), (2.0, 14.0), (2.0, 10.5)],
                c(Color32::from_rgb(250, 200, 70)),
                g.stroke(0.8, outline),
            ));
            p.add(g.poly(
                &[(2.0, 10.5), (5.5, 14.0), (2.0, 14.0)],
                c(Color32::from_rgb(90, 70, 50)),
                Stroke::NONE,
            ));
            p.add(g.line(&[(9.5, 3.0), (13.0, 6.5)], g.stroke(0.8, outline)));
        }
        Glyph::Delete => {
            let s = g.stroke(2.6, c(Color32::from_rgb(210, 40, 40)));
            p.add(g.line(&[(3.5, 3.5), (12.5, 12.5)], s));
            p.add(g.line(&[(12.5, 3.5), (3.5, 12.5)], s));
        }
        Glyph::Clear => {
            p.add(g.poly(
                &[(3.5, 4.5), (12.5, 4.5), (11.5, 15.0), (4.5, 15.0)],
                c(Color32::from_rgb(200, 205, 215)),
                g.stroke(0.8, outline),
            ));
            p.rect(
                g.r(2.0, 2.5, 14.0, 4.5),
                0.0,
                c(Color32::from_rgb(170, 175, 190)),
                g.stroke(0.8, outline),
                egui::StrokeKind::Inside,
            );
            p.rect_filled(g.r(6.0, 1.0, 10.0, 2.5), 0.0, outline);
            for x in [6.0, 8.0, 10.0] {
                p.add(g.line(&[(x, 6.5), (x, 13.0)], g.stroke(0.7, outline)));
            }
        }
        Glyph::Error => {
            p.circle(
                g.p(8.0, 8.0),
                7.0 * g.s,
                c(Color32::from_rgb(220, 40, 40)),
                g.stroke(0.7, c(Color32::from_rgb(140, 15, 15))),
            );
            let s = g.stroke(1.8, Color32::WHITE);
            p.add(g.line(&[(5.2, 5.2), (10.8, 10.8)], s));
            p.add(g.line(&[(10.8, 5.2), (5.2, 10.8)], s));
        }
        Glyph::Warning => {
            p.add(g.poly(
                &[(8.0, 1.0), (15.5, 14.5), (0.5, 14.5)],
                c(Color32::from_rgb(250, 200, 30)),
                g.stroke(0.8, c(Color32::from_rgb(150, 110, 0))),
            ));
            p.add(g.line(&[(8.0, 5.0), (8.0, 10.0)], g.stroke(1.8, Color32::BLACK)));
            p.circle_filled(g.p(8.0, 12.3), 1.0 * g.s, Color32::BLACK);
        }
        Glyph::Copy => {
            for (dx, dy) in [(0.0, 0.0), (4.0, 4.0)] {
                p.rect(
                    g.r(2.0 + dx, 1.5 + dy, 10.0 + dx, 11.5 + dy),
                    0.0,
                    Color32::WHITE,
                    g.stroke(0.9, outline),
                    egui::StrokeKind::Inside,
                );
                for k in 0..3 {
                    let y = 4.5 + dy + 2.2 * k as f32;
                    p.add(g.line(&[(4.0 + dx, y), (8.0 + dx, y)], g.stroke(0.6, c(Color32::from_gray(140)))));
                }
            }
        }
        Glyph::Save => {
            p.rect(
                g.r(2.0, 2.0, 14.0, 14.0),
                1.0,
                c(Color32::from_rgb(60, 100, 180)),
                g.stroke(0.8, c(Color32::from_rgb(25, 50, 110))),
                egui::StrokeKind::Inside,
            );
            p.rect_filled(g.r(4.5, 2.5, 11.5, 7.0), 0.0, Color32::WHITE);
            p.rect_filled(g.r(5.0, 10.0, 11.0, 14.0), 0.0, c(Color32::from_gray(210)));
            p.rect_filled(g.r(9.0, 3.0, 10.5, 6.0), 0.0, c(Color32::from_rgb(60, 100, 180)));
        }
        Glyph::Log => {
            p.rect(
                g.r(2.5, 1.5, 13.5, 14.5),
                0.0,
                Color32::WHITE,
                g.stroke(0.9, outline),
                egui::StrokeKind::Inside,
            );
            for (k, w) in [8.0f32, 6.0, 8.0, 5.0].into_iter().enumerate() {
                let y = 4.5 + 2.6 * k as f32;
                p.add(g.line(&[(4.5, y), (4.5 + w, y)], g.stroke(0.9, c(Color32::from_gray(90)))));
            }
        }
    }
}

/// Side of a toolbar button.
pub const TOOL_BUTTON: f32 = 24.0;

/// A flat toolbar button of fixed size painted by `paint_icon` (into the given
/// square, enabled flag), raised on hover and sunken when pressed. The hover frame
/// is painted inside the button's rect, so hovering never moves anything.
pub fn tool_button_with(
    ui: &mut Ui,
    tip: &str,
    enabled: bool,
    paint_icon: impl FnOnce(&Painter, Rect, bool),
) -> Response {
    let size = Vec2::splat(TOOL_BUTTON);
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
                    crate::theme::PRESSED_FILL
                } else {
                    crate::theme::HOVER_FILL
                },
                Stroke::new(1.0, crate::theme::HOVER_STROKE),
                egui::StrokeKind::Inside,
            );
        }
        paint_icon(p, rect.shrink(4.0), enabled);
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tip));
    resp
}

/// A flat toolbar button showing `glyph` (see [`tool_button_with`]).
pub fn tool_button(ui: &mut Ui, glyph: Glyph, tip: &str, enabled: bool) -> Response {
    tool_button_with(ui, tip, enabled, |p, r, en| paint(p, r, glyph, en))
}

/// Vertical etched separator for toolbars.
pub fn tool_separator(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(6.0, TOOL_BUTTON), Sense::hover());
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
