//! MDI child windows: resource forms live inside the workspace area of the main window,
//! with a classic title bar (icon, caption, minimise / maximise / close), resize
//! borders, and minimised strips along the bottom of the workspace.
//!
//! The window rectangle is owned by [`MdiState`], never derived from the content: the
//! content is laid out inside a fixed, clipped child rectangle. This rules out the
//! feedback loop where content sized from "available space" makes an auto-sizing
//! window grow every frame.

use eframe::egui::{
    self, pos2, vec2, Color32, Context, CursorIcon, Id, LayerId, Order, Rect,
    Sense, Ui, UiBuilder, Vec2,
};

use crate::icons::{self, Icon};

pub const TITLE_H: f32 = gm_ui::chrome::TITLE_H;
pub const BORDER: f32 = gm_ui::chrome::BORDER;
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

use gm_ui::chrome::{bevel, caption_button, CaptionButton as Caption};
/// Modal dialog boxes with the classic chrome (shared with the runner).
pub use gm_ui::chrome::dialog;

/// Paint a classic caption (the shared `gm-ui` chrome) with an optional IDE icon.
pub fn paint_caption(
    p: &egui::Painter,
    bar: Rect,
    title: &str,
    icon: Option<Icon>,
    active: bool,
    right_reserve: f32,
) {
    let paint_icon = |p: &egui::Painter, r: Rect| {
        if let Some(i) = icon {
            icons::paint(p, r, i, true)
        }
    };
    let icon_fn: Option<&dyn Fn(&egui::Painter, Rect)> = icon.map(|_| &paint_icon as _);
    gm_ui::chrome::paint_caption(p, bar, title, icon_fn, active, right_reserve);
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
    min_button: bool,
) {
    let n_btn = if min_button { 3.0 } else { 2.0 };
    let bw = 16.0;
    let bh = bar.height() - 4.0;
    paint_caption(
        ui.painter(),
        bar,
        title,
        Some(icon),
        active,
        n_btn * (bw + 2.0) + 6.0,
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
    if !min_button {
        return;
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
    show_impl(
        ctx,
        id,
        area,
        Some(min_slot),
        st,
        title,
        icon,
        active,
        add_contents,
    )
}

#[allow(clippy::too_many_arguments)]
fn show_impl(
    ctx: &Context,
    id: Id,
    area: Rect,
    min_slot: Option<Rect>,
    st: &mut MdiState,
    title: &str,
    icon: Icon,
    active: bool,
    add_contents: impl FnOnce(&mut Ui),
) -> MdiOut {
    let mut out = MdiOut::default();
    if min_slot.is_none() {
        st.minimised = false;
    }
    let r = if let (true, Some(slot)) = (st.minimised, min_slot) {
        slot
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
            title_bar(
                ui,
                id,
                bar,
                title,
                icon,
                active,
                st,
                &mut out,
                min_slot.is_some(),
            );
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

fn workspace_key() -> Id {
    Id::new("mdi_workspace")
}

/// Record the workspace rectangle (where child windows live) for this frame.
pub fn set_workspace(ctx: &Context, area: Rect) {
    ctx.data_mut(|d| d.insert_temp(workspace_key(), area));
}

/// The workspace rectangle, or the whole content area when there is none.
pub fn workspace(ctx: &Context) -> Rect {
    ctx.data(|d| d.get_temp::<Rect>(workspace_key()))
        .unwrap_or_else(|| ctx.content_rect())
}

/// A child window owned by an editor rather than by the IDE's list of forms (the
/// sprite and image editors). It has the same chrome as the forms, lives in the
/// workspace, comes to the front when clicked and has no minimise button. Its placement
/// is kept in egui memory under `id`; it opens centred with `default_size` (outer).
pub fn floating(
    ctx: &Context,
    id: Id,
    title: &str,
    icon: Icon,
    default_size: Vec2,
    add_contents: impl FnOnce(&mut Ui),
) -> MdiOut {
    let area = workspace(ctx);
    let key = id.with("mdi_state");
    let mut st = ctx
        .data(|d| d.get_temp::<MdiState>(key))
        .unwrap_or_else(|| {
            MdiState::new(Rect::from_center_size(
                area.center(),
                default_size.min(area.size()),
            ))
        });
    // Come to the front before painting, so that popups opened by this click stay
    // above the window.
    let layer = LayerId::new(Order::Middle, id);
    if ctx.input(|i| i.pointer.any_pressed()) {
        if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
            if ctx.layer_id_at(pos) == Some(layer) {
                bring_to_front(ctx, id);
            }
        }
    }
    let out = show_impl(
        ctx,
        id,
        area,
        None,
        &mut st,
        title,
        icon,
        true,
        add_contents,
    );
    ctx.data_mut(|d| d.insert_temp(key, st));
    out
}

