//! MDI child windows: resource forms live inside the workspace area of the main window,
//! with a classic title bar (icon, caption, minimise / maximise / close), resize
//! borders, and minimised strips along the bottom of the workspace.
//!
//! The window rectangle is owned by [`MdiState`], never derived from the content: the
//! content is laid out inside a fixed, clipped child rectangle. This rules out the
//! feedback loop where content sized from "available space" makes an auto-sizing
//! window grow every frame.

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, Context, CursorIcon, FontId, Id, LayerId, Order, Rect,
    Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};

use crate::icons::{self, Icon};

pub const TITLE_H: f32 = 20.0;
pub const BORDER: f32 = 4.0;
pub const MIN_SIZE: Vec2 = Vec2::new(220.0, 120.0);
pub const MINIMISED_W: f32 = 170.0;

/// Placement of one child window.
#[derive(Debug, Clone, Copy)]
pub struct MdiState {
    /// Outer rectangle (including border and title bar) when neither minimised nor maximised.
    pub rect: Rect,
    pub minimised: bool,
    pub maximised: bool,
}

impl MdiState {
    pub fn new(rect: Rect) -> Self {
        MdiState {
            rect,
            minimised: false,
            maximised: false,
        }
    }
}

/// What happened to a child window this frame.
#[derive(Default)]
pub struct MdiOut {
    pub close: bool,
    /// The user clicked into the window (it should become the active one).
    pub activated: bool,
}

/// Outer size for a given content size.
pub fn outer_size(content: Vec2) -> Vec2 {
    content + vec2(2.0 * BORDER, 2.0 * BORDER + TITLE_H + 2.0)
}

/// Clamp `r` into `area`: size limited to the area, position moved inside.
pub fn clamp_into(r: Rect, area: Rect) -> Rect {
    let size = r.size().min(area.size()).max(MIN_SIZE.min(area.size()));
    let x = r
        .min
        .x
        .clamp(area.min.x, (area.max.x - size.x).max(area.min.x));
    let y = r
        .min
        .y
        .clamp(area.min.y, (area.max.y - size.y).max(area.min.y));
    Rect::from_min_size(pos2(x, y), size)
}

fn bevel(p: &egui::Painter, r: Rect, raised: bool) {
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

#[derive(Clone, Copy, PartialEq)]
enum Caption {
    Min,
    Max,
    Restore,
    Close,
}

fn caption_button(ui: &mut Ui, id: Id, r: Rect, kind: Caption) -> bool {
    let resp = ui.interact(r, id, Sense::click());
    let label = match kind {
        Caption::Min => "Minimise",
        Caption::Max => "Maximise",
        Caption::Restore => "Restore",
        Caption::Close => "Close",
    };
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let p = ui.painter();
    let down = resp.is_pointer_button_down_on();
    let fill = if kind == Caption::Close && resp.hovered() {
        Color32::from_rgb(232, 17, 35)
    } else if resp.hovered() {
        Color32::from_rgb(250, 250, 250)
    } else {
        Color32::from_gray(236)
    };
    p.rect_filled(r, 0.0, fill);
    bevel(p, r.shrink(0.5), !down);
    let ink = if kind == Caption::Close && resp.hovered() {
        Color32::WHITE
    } else {
        Color32::BLACK
    };
    let c = r.center() + if down { vec2(1.0, 1.0) } else { Vec2::ZERO };
    let s = Stroke::new(1.4, ink);
    match kind {
        Caption::Min => {
            p.line_segment(
                [c + vec2(-3.5, 3.0), c + vec2(3.0, 3.0)],
                Stroke::new(2.0, ink),
            );
        }
        Caption::Max => {
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
        Caption::Restore => {
            let b1 = Rect::from_min_size(c + vec2(-2.0, -4.5), vec2(6.0, 5.0));
            let b2 = Rect::from_min_size(c + vec2(-4.5, -1.5), vec2(6.0, 5.0));
            p.rect_stroke(b1, 0.0, Stroke::new(1.0, ink), StrokeKind::Inside);
            p.rect_filled(b2, 0.0, fill);
            p.rect_stroke(b2, 0.0, Stroke::new(1.0, ink), StrokeKind::Inside);
        }
        Caption::Close => {
            p.line_segment([c + vec2(-3.5, -3.0), c + vec2(3.0, 3.5)], s);
            p.line_segment([c + vec2(3.0, -3.0), c + vec2(-3.5, 3.5)], s);
        }
    }
    resp.clicked()
}

/// Paint the classic title bar (gradient, icon, caption) and handle its buttons and dragging.
#[allow(clippy::too_many_arguments)]
fn title_bar(
    ui: &mut Ui,
    id: Id,
    bar: Rect,
    title: &str,
    icon: Icon,
    active: bool,
    st: &mut MdiState,
    out: &mut MdiOut,
) {
    let p = ui.painter().clone();
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
    let icon_r = Rect::from_min_size(bar.min + vec2(2.0, 2.0), Vec2::splat(bar.height() - 4.0));
    icons::paint(&p, icon_r, icon, true);
    let n_btn = 3.0;
    let bw = 16.0;
    let bh = bar.height() - 4.0;
    let text_clip = Rect::from_min_max(
        pos2(icon_r.max.x + 4.0, bar.min.y),
        pos2(bar.max.x - n_btn * (bw + 2.0) - 6.0, bar.max.y),
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
    // drag / double-click on the caption
    let resp = ui.interact(bar, id.with("caption"), Sense::click_and_drag());
    if resp.drag_started() || resp.clicked() {
        out.activated = true;
    }
    if resp.dragged() && !st.maximised && !st.minimised {
        st.rect = st.rect.translate(resp.drag_delta());
    }
    if resp.double_clicked() {
        if st.minimised {
            st.minimised = false;
        } else {
            st.maximised = !st.maximised;
        }
    }
    let mut x = bar.max.x - 2.0 - bw;
    let y = bar.min.y + 2.0;
    let btn = |ui: &mut Ui, kind: Caption, x: f32| {
        caption_button(
            ui,
            id.with(("btn", kind as u8)),
            Rect::from_min_size(pos2(x, y), vec2(bw, bh)),
            kind,
        )
    };
    if btn(ui, Caption::Close, x) {
        out.close = true;
    }
    x -= bw + 3.0;
    let max_kind = if st.maximised && !st.minimised {
        Caption::Restore
    } else {
        Caption::Max
    };
    if btn(ui, max_kind, x) {
        if st.minimised {
            st.minimised = false;
            st.maximised = true;
        } else {
            st.maximised = !st.maximised;
        }
        out.activated = true;
    }
    x -= bw;
    let min_kind = if st.minimised {
        Caption::Restore
    } else {
        Caption::Min
    };
    if btn(ui, min_kind, x) {
        st.minimised = !st.minimised;
        out.activated = !st.minimised;
    }
}

/// Show a child window. `min_slot` is where it goes when minimised.
#[allow(clippy::too_many_arguments)]
pub fn show(
    ctx: &Context,
    id: Id,
    area: Rect,
    min_slot: Rect,
    st: &mut MdiState,
    title: &str,
    icon: Icon,
    active: bool,
    add_contents: impl FnOnce(&mut Ui),
) -> MdiOut {
    let mut out = MdiOut::default();
    let r = if st.minimised {
        min_slot
    } else if st.maximised {
        area
    } else {
        clamp_into(st.rect, area)
    };
    // Keep the frame on whole physical pixels (crisp 1px bevels).
    let r = {
        use egui::emath::GuiRounding as _;
        r.round_to_pixels(ctx.pixels_per_point()).round_ui()
    };
    let layer = LayerId::new(Order::Middle, id);
    egui::Area::new(id)
        .order(Order::Middle)
        .fixed_pos(r.min)
        .movable(false)
        .constrain(false)
        .sense(Sense::click())
        .show(ctx, |ui| {
            ui.set_clip_rect(r.expand(1.0));
            let p = ui.painter().clone();
            // outer frame
            p.rect_filled(r, 0.0, Color32::from_gray(240));
            bevel(&p, r.shrink(0.5), true);
            bevel(&p, r.shrink(1.5), true);
            let bar = Rect::from_min_size(
                r.min + vec2(BORDER, BORDER),
                vec2(r.width() - 2.0 * BORDER, TITLE_H),
            );
            title_bar(ui, id, bar, title, icon, active, st, &mut out);
            if !st.minimised {
                let content = Rect::from_min_max(
                    pos2(r.min.x + BORDER, bar.max.y + 2.0),
                    r.max - vec2(BORDER, BORDER),
                );
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .id_salt("content")
                        .max_rect(content.shrink2(vec2(6.0, 4.0)))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                child.set_clip_rect(content);
                add_contents(&mut child);
                if !st.maximised {
                    resize_handles(ui, id, r, st);
                }
            }
            // The area is exactly the window rect, whatever the content did.
            ui.expand_to_include_rect(r);
        });
    if ctx.input(|i| i.pointer.any_pressed()) {
        if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
            if r.contains(pos) && ctx.layer_id_at(pos) == Some(layer) {
                out.activated = true;
            }
        }
    }
    out
}

fn resize_handles(ui: &mut Ui, id: Id, r: Rect, st: &mut MdiState) {
    let e = BORDER + 1.0;
    let c = 12.0;
    // (dx-min, dy-min, dx-max, dy-max) masks for each handle
    let handles: [(Rect, [f32; 4], CursorIcon); 8] = [
        (
            Rect::from_min_max(r.min, pos2(r.max.x, r.min.y + e)),
            [0., 1., 0., 0.],
            CursorIcon::ResizeNorth,
        ),
        (
            Rect::from_min_max(pos2(r.min.x, r.max.y - e), r.max),
            [0., 0., 0., 1.],
            CursorIcon::ResizeSouth,
        ),
        (
            Rect::from_min_max(r.min, pos2(r.min.x + e, r.max.y)),
            [1., 0., 0., 0.],
            CursorIcon::ResizeWest,
        ),
        (
            Rect::from_min_max(pos2(r.max.x - e, r.min.y), r.max),
            [0., 0., 1., 0.],
            CursorIcon::ResizeEast,
        ),
        (
            Rect::from_min_size(r.min, Vec2::splat(c)),
            [1., 1., 0., 0.],
            CursorIcon::ResizeNorthWest,
        ),
        (
            Rect::from_min_max(pos2(r.max.x - c, r.min.y), pos2(r.max.x, r.min.y + c)),
            [0., 1., 1., 0.],
            CursorIcon::ResizeNorthEast,
        ),
        (
            Rect::from_min_max(pos2(r.min.x, r.max.y - c), pos2(r.min.x + c, r.max.y)),
            [1., 0., 0., 1.],
            CursorIcon::ResizeSouthWest,
        ),
        (
            Rect::from_min_max(r.max - Vec2::splat(c), r.max),
            [0., 0., 1., 1.],
            CursorIcon::ResizeSouthEast,
        ),
    ];
    for (i, (hr, m, cursor)) in handles.into_iter().enumerate() {
        // corners overlap the title bar's caption buttons; keep top corners small
        let resp = ui.interact(hr, id.with(("resize", i)), Sense::drag());
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(cursor);
        }
        if resp.dragged() {
            let d = resp.drag_delta();
            let mut n = clamp_into(st.rect, Rect::EVERYTHING);
            n.min.x += d.x * m[0];
            n.min.y += d.y * m[1];
            n.max.x += d.x * m[2];
            n.max.y += d.y * m[3];
            if n.width() < MIN_SIZE.x {
                if m[0] > 0.0 {
                    n.min.x = n.max.x - MIN_SIZE.x;
                } else {
                    n.max.x = n.min.x + MIN_SIZE.x;
                }
            }
            if n.height() < MIN_SIZE.y {
                if m[1] > 0.0 {
                    n.min.y = n.max.y - MIN_SIZE.y;
                } else {
                    n.max.y = n.min.y + MIN_SIZE.y;
                }
            }
            st.rect = n;
        }
    }
}

/// Slot of the `i`-th minimised window along the bottom of `area`.
pub fn minimised_slot(area: Rect, i: usize) -> Rect {
    let per_row = ((area.width() / (MINIMISED_W + 2.0)).floor() as usize).max(1);
    let row = i / per_row;
    let col = i % per_row;
    let h = TITLE_H + 2.0 * BORDER;
    Rect::from_min_size(
        pos2(
            area.min.x + col as f32 * (MINIMISED_W + 2.0),
            area.max.y - (row + 1) as f32 * (h + 2.0),
        ),
        vec2(MINIMISED_W, h),
    )
}

/// Bring a child window's layer to the top.
pub fn bring_to_front(ctx: &Context, id: Id) {
    ctx.move_to_top(LayerId::new(Order::Middle, id));
}

/// Rectangle of the `i`-th window when cascading.
pub fn cascade_rect(area: Rect, i: usize, size: Vec2) -> Rect {
    let step = TITLE_H + BORDER + 4.0;
    let n = ((area.height() - size.y.min(area.height())) / step)
        .floor()
        .max(1.0) as usize;
    let k = (i % n.max(1)) as f32;
    clamp_into(
        Rect::from_min_size(area.min + vec2(8.0 + k * step, 8.0 + k * step), size),
        area,
    )
}
