//! Classic window chrome painted with egui: raised double bevels, the horizontal
//! gradient caption, caption buttons (minimise / maximise / restore / close) and the
//! frame of dialog boxes. The IDE's MDI child windows and dialogs and the runner's
//! error dialog are built from these.

use egui::{
    pos2, vec2, Align2, Color32, FontId, Id, Painter, Rect, Sense, Stroke, StrokeKind, Ui, Vec2,
};

/// Height of a caption (title bar).
pub const TITLE_H: f32 = 20.0;
/// Width of the raised window border around the caption and the contents.
pub const BORDER: f32 = 4.0;
/// Window background.
pub const FACE: Color32 = Color32::from_gray(240);

/// A 1px bevel along the inside of `r`: light top-left and dark bottom-right
/// (`raised`), or the other way round.
pub fn bevel(p: &Painter, r: Rect, raised: bool) {
    let (tl, br) = if raised {
        (Color32::WHITE, Color32::from_gray(105))
    } else {
        (Color32::from_gray(105), Color32::WHITE)
    };
    p.line_segment([r.left_bottom(), r.left_top()], Stroke::new(1.0, tl));
    p.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, tl));
    p.line_segment([r.right_top(), r.right_bottom()], Stroke::new(1.0, br));
    p.line_segment([r.right_bottom(), r.left_bottom()], Stroke::new(1.0, br));
}

/// A sunken 2px edge around a list or text area at `r` (outside of it).
pub fn sunken(p: &Painter, r: Rect) {
    bevel(p, r.expand(1.5), false);
    let inner = r.expand(0.5);
    p.line_segment(
        [inner.left_bottom(), inner.left_top()],
        Stroke::new(1.0, Color32::from_gray(160)),
    );
    p.line_segment(
        [inner.left_top(), inner.right_top()],
        Stroke::new(1.0, Color32::from_gray(160)),
    );
}

/// The face and raised double bevel of a window or dialog occupying `r`.
pub fn paint_window_frame(p: &Painter, r: Rect) {
    p.rect_filled(r, 0.0, FACE);
    bevel(p, r.shrink(0.5), true);
    bevel(p, r.shrink(1.5), true);
}

/// The caption bar inside a window frame at `r`.
pub fn caption_rect(r: Rect) -> Rect {
    Rect::from_min_size(
        r.min + vec2(BORDER, BORDER),
        vec2(r.width() - 2.0 * BORDER, TITLE_H),
    )
}

/// Paint a classic caption: horizontal gradient, an optional icon (painted by
/// `icon` into the given square) and the title, which is clipped `right_reserve`
/// points before the right end (room for the buttons).
pub fn paint_caption(
    p: &Painter,
    bar: Rect,
    title: &str,
    icon: Option<&dyn Fn(&Painter, Rect)>,
    active: bool,
    right_reserve: f32,
) {
    let (a, b) = if active {
        (
            Color32::from_rgb(10, 36, 106),
            Color32::from_rgb(166, 202, 240),
        )
    } else {
        (Color32::from_gray(128), Color32::from_gray(192))
    };
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(bar.left_top(), a);
    mesh.colored_vertex(bar.right_top(), b);
    mesh.colored_vertex(bar.right_bottom(), b);
    mesh.colored_vertex(bar.left_bottom(), a);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(mesh));
    let mut x = bar.min.x + 4.0;
    if let Some(icon) = icon {
        let icon_r = Rect::from_min_size(bar.min + vec2(2.0, 2.0), Vec2::splat(bar.height() - 4.0));
        icon(p, icon_r);
        x = icon_r.max.x + 4.0;
    }
    let text_clip = Rect::from_min_max(
        pos2(x, bar.min.y),
        pos2(bar.max.x - right_reserve, bar.max.y),
    );
    p.with_clip_rect(text_clip).text(
        pos2(text_clip.min.x, bar.center().y),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(12.5),
        if active {
            Color32::WHITE
        } else {
            Color32::from_gray(225)
        },
    );
}

/// The buttons of a caption.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaptionButton {
    Min,
    Max,
    Restore,
    Close,
}

/// Size of a caption button.
pub const CAPTION_BUTTON: Vec2 = Vec2::new(16.0, TITLE_H - 4.0);

/// The rectangle of the close button at the right end of `bar`.
pub fn close_button_rect(bar: Rect) -> Rect {
    Rect::from_min_size(
        pos2(bar.max.x - 2.0 - CAPTION_BUTTON.x, bar.min.y + 2.0),
        CAPTION_BUTTON,
    )
}

/// A raised caption button at `r`; returns true when clicked.
pub fn caption_button(ui: &mut Ui, id: Id, r: Rect, kind: CaptionButton) -> bool {
    let resp = ui.interact(r, id, Sense::click());
    let label = match kind {
        CaptionButton::Min => "Minimise",
        CaptionButton::Max => "Maximise",
        CaptionButton::Restore => "Restore",
        CaptionButton::Close => "Close",
    };
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let p = ui.painter();
    let down = resp.is_pointer_button_down_on();
    let close_hot = kind == CaptionButton::Close && resp.hovered();
    let fill = if close_hot {
        Color32::from_rgb(232, 17, 35)
    } else if resp.hovered() {
        Color32::from_rgb(250, 250, 250)
    } else {
        Color32::from_gray(236)
    };
    p.rect_filled(r, 0.0, fill);
    bevel(p, r.shrink(0.5), !down);
    let ink = if close_hot {
        Color32::WHITE
    } else {
        Color32::BLACK
    };
    let c = r.center() + if down { vec2(1.0, 1.0) } else { Vec2::ZERO };
    let s = Stroke::new(1.4, ink);
    match kind {
        CaptionButton::Min => {
            p.line_segment(
                [c + vec2(-3.5, 3.0), c + vec2(3.0, 3.0)],
                Stroke::new(2.0, ink),
            );
        }
        CaptionButton::Max => {
            let b = Rect::from_center_size(c, vec2(8.0, 7.0));
            p.rect_stroke(b, 0.0, Stroke::new(1.0, ink), StrokeKind::Inside);
            p.line_segment(
                [
                    b.left_top() + vec2(0.0, 1.0),
                    b.right_top() + vec2(0.0, 1.0),
                ],
                Stroke::new(2.0, ink),
            );
        }
        CaptionButton::Restore => {
            let b1 = Rect::from_min_size(c + vec2(-2.0, -4.5), vec2(6.0, 5.0));
            let b2 = Rect::from_min_size(c + vec2(-4.5, -1.5), vec2(6.0, 5.0));
            p.rect_stroke(b1, 0.0, Stroke::new(1.0, ink), StrokeKind::Inside);
            p.rect_filled(b2, 0.0, fill);
            p.rect_stroke(b2, 0.0, Stroke::new(1.0, ink), StrokeKind::Inside);
        }
        CaptionButton::Close => {
            p.line_segment([c + vec2(-3.5, -3.0), c + vec2(3.0, 3.5)], s);
            p.line_segment([c + vec2(3.0, -3.0), c + vec2(-3.5, 3.5)], s);
        }
    }
    resp.clicked()
}

/// The soft drop shadow of dialogs, as a shape around `r`.
pub fn dialog_shadow(r: Rect) -> egui::Shape {
    let shadow = egui::epaint::Shadow {
        offset: [3, 3],
        blur: 10,
        spread: 0,
        color: Color32::from_black_alpha(70),
    };
    shadow.as_shape(r, egui::CornerRadius::ZERO).into()
}

/// Result of [`dialog`].
pub struct DialogOut<T> {
    pub inner: T,
    /// The close box was clicked or Escape pressed.
    pub close: bool,
}

impl<T> DialogOut<T> {
    pub fn should_close(&self) -> bool {
        self.close
    }
}

/// A modal dialog box with the classic chrome: raised frame, gradient caption with
/// `title` and a close box. The contents decide the size.
pub fn dialog<T>(
    ctx: &egui::Context,
    id: Id,
    title: &str,
    add_contents: impl FnOnce(&mut Ui) -> T,
) -> DialogOut<T> {
    let mut close = false;
    let m = egui::Modal::new(id)
        .frame(egui::Frame::NONE)
        .backdrop_color(Color32::from_black_alpha(24))
        .show(ctx, |ui| {
            ui.ctx().move_to_top(ui.layer_id());
            let bg = ui.painter().add(egui::Shape::Noop);
            let side = (BORDER + 10.0) as i8;
            let inner = egui::Frame::NONE
                .inner_margin(egui::Margin {
                    left: side,
                    right: side,
                    top: (BORDER + TITLE_H + 10.0) as i8,
                    bottom: side,
                })
                .show(ui, |ui| {
                    ui.set_min_width(240.0);
                    add_contents(ui)
                });
            let r = {
                use egui::emath::GuiRounding as _;
                inner
                    .response
                    .rect
                    .round_to_pixels(ui.ctx().pixels_per_point())
                    .round_ui()
            };
            let p = ui.painter().clone();
            p.set(
                bg,
                egui::Shape::Vec(vec![dialog_shadow(r), egui::Shape::rect_filled(r, 0.0, FACE)]),
            );
            bevel(&p, r.shrink(0.5), true);
            bevel(&p, r.shrink(1.5), true);
            let bar = caption_rect(r);
            paint_caption(&p, bar, title, None, true, 24.0);
            if caption_button(ui, id.with("close"), close_button_rect(bar), CaptionButton::Close) {
                close = true;
            }
            inner.inner
        });
    if m.is_top_modal && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close = true;
    }
    DialogOut {
        inner: m.inner,
        close,
    }
}

/// A sunken status-bar cell of width `w` showing `text`.
pub fn status_cell(ui: &mut Ui, w: f32, text: &str) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
    let p = ui.painter();
    let shadow = Stroke::new(1.0, crate::theme::SHADOW);
    let light = Stroke::new(1.0, Color32::WHITE);
    p.hline(r.x_range(), r.top() + 0.5, shadow);
    p.vline(r.left() + 0.5, r.y_range(), shadow);
    p.hline(r.x_range(), r.bottom() - 0.5, light);
    p.vline(r.right() - 0.5, r.y_range(), light);
    p.with_clip_rect(r.shrink(2.0)).text(
        r.left_center() + vec2(5.0, 0.0),
        Align2::LEFT_CENTER,
        text,
        egui::TextStyle::Body.resolve(ui.style()),
        Color32::from_gray(20),
    );
    resp
}

/// A dialog button of the classic minimum size (75×23); `default` draws the
/// darker frame of the default button around it.
pub fn dialog_button(ui: &mut Ui, text: &str, default: bool) -> egui::Response {
    let r = ui.add(egui::Button::new(text).min_size(vec2(75.0, 22.0)));
    if default {
        ui.painter().rect_stroke(
            r.rect.expand(2.0),
            2.0,
            Stroke::new(1.0, crate::theme::HOVER_STROKE),
            StrokeKind::Inside,
        );
    }
    r
}
