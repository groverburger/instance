//! Common widgets (specs/ide.md §4): resource picker, colour box, image previews.

pub mod action_text;
pub mod actions;
pub mod code_editor;
pub mod event_selector;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use eframe::egui::{self, Color32, Context, Rect, TextureHandle, TextureId, Ui, Vec2};
use gm_format::{Project, ResourceKind, RgbaImage};

use crate::project_ops::{res_name, tree_order};

// ------------------------------------------------------------------ textures

/// Cache of GPU textures for model images, keyed by (owner, frame) and invalidated by
/// a generation number supplied by the caller.
#[derive(Default)]
pub struct TextureCache {
    map: HashMap<(u64, usize), (u64, TextureHandle)>,
}

impl TextureCache {
    /// Texture for `img`; re-uploaded when `gen` differs from the cached one.
    pub fn get(
        &mut self,
        ctx: &Context,
        owner: u64,
        frame: usize,
        gen: u64,
        img: &RgbaImage,
    ) -> Option<TextureId> {
        if img.width == 0 || img.height == 0 || !img.is_consistent() {
            return None;
        }
        if let Some((g, h)) = self.map.get(&(owner, frame)) {
            if *g == gen {
                return Some(h.id());
            }
        }
        let ci = egui::ColorImage::from_rgba_unmultiplied(
            [img.width as usize, img.height as usize],
            &img.data,
        );
        let h = ctx.load_texture(
            format!("tex-{owner}-{frame}"),
            ci,
            egui::TextureOptions::NEAREST,
        );
        let id = h.id();
        self.map.insert((owner, frame), (gen, h));
        Some(id)
    }

    /// Drop every texture.
    pub fn clear(&mut self) {
        self.map.clear();
    }
}

/// Owner key for textures of project resources.
pub fn owner_key(tag: &str, idx: usize) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    tag.hash(&mut h);
    idx.hash(&mut h);
    h.finish()
}

/// Paint a transparency checkerboard (spec §4 "image preview with transparency background").
pub fn checker(painter: &egui::Painter, rect: Rect, cell: f32) {
    painter.rect_filled(rect, 0.0, Color32::from_gray(255));
    let cell = cell.max(2.0);
    let nx = (rect.width() / cell).ceil() as i32;
    let ny = (rect.height() / cell).ceil() as i32;
    if nx * ny > 40_000 {
        return;
    }
    for y in 0..ny {
        for x in 0..nx {
            if (x + y) % 2 == 1 {
                let r = Rect::from_min_size(
                    rect.min + Vec2::new(x as f32 * cell, y as f32 * cell),
                    Vec2::splat(cell),
                )
                .intersect(rect);
                painter.rect_filled(r, 0.0, Color32::from_gray(204));
            }
        }
    }
}

/// Show an image scaled to fit `max` (never enlarged beyond `max_zoom`) over a checkerboard.
/// Returns the response and the screen rect plus scale used.
pub fn image_preview(
    ui: &mut Ui,
    tex: Option<TextureId>,
    w: u32,
    h: u32,
    max: Vec2,
    max_zoom: f32,
    sense: egui::Sense,
) -> (egui::Response, Rect, f32) {
    let (w, h) = (w.max(1) as f32, h.max(1) as f32);
    let scale = (max.x / w).min(max.y / h).min(max_zoom).max(0.01);
    let size = Vec2::new(w * scale, h * scale);
    let (resp, painter) = ui.allocate_painter(size, sense);
    let rect = resp.rect;
    checker(&painter, rect, 8.0);
    if let Some(t) = tex {
        painter.image(
            t,
            rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    (resp, rect, scale)
}

// ------------------------------------------------------------------ colours

/// Model colour (`0x00BBGGRR`) to egui colour. System colours (high byte set) are shown grey.
pub fn bgr_to_color(c: u32) -> Color32 {
    if c >> 24 != 0 {
        return Color32::from_gray(212);
    }
    Color32::from_rgb(
        (c & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        ((c >> 16) & 0xFF) as u8,
    )
}

/// Colour box editing a model colour. Returns true when changed.
pub fn color_box(ui: &mut Ui, c: &mut u32) -> bool {
    let col = bgr_to_color(*c);
    let mut rgb = [col.r(), col.g(), col.b()];
    let r = ui.color_edit_button_srgb(&mut rgb);
    if r.changed() {
        *c = rgb[0] as u32 | (rgb[1] as u32) << 8 | (rgb[2] as u32) << 16;
        true
    } else {
        false
    }
}

// ------------------------------------------------------------------ resource picker

/// Placeholder text of a picker of `kind`.
pub fn none_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Sprite => "<no sprite>",
        ResourceKind::Sound => "<no sound>",
        ResourceKind::Background => "<no background>",
        ResourceKind::Path => "<no path>",
        ResourceKind::Script => "<no script>",
        ResourceKind::Font => "<no font>",
        ResourceKind::Timeline => "<no time line>",
        ResourceKind::Object => "<no object>",
        ResourceKind::Room => "<no room>",
        _ => "<none>",
    }
}

/// Resource picker (spec §4): a combo box with "none" and all resources of `kind` in
/// tree order (groups shown as headings). `none_value` is stored for "none".
pub fn resource_picker(
    ui: &mut Ui,
    id_salt: impl Hash + std::fmt::Debug,
    p: &Project,
    kind: ResourceKind,
    value: &mut i32,
    none_text: &str,
    none_value: i32,
) -> bool {
    let current = if *value == none_value || *value < 0 {
        none_text.to_string()
    } else {
        res_name(p, kind, *value as usize)
            .map_or_else(|| format!("<missing {}>", *value), str::to_string)
    };
    let mut changed = false;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(current)
        .width(160.0)
        .height(400.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(*value == none_value, none_text)
                .clicked()
            {
                *value = none_value;
                changed = true;
            }
            for (idx, name, depth) in tree_order(p, kind) {
                let indent = "    ".repeat(depth);
                if idx < 0 {
                    ui.label(egui::RichText::new(format!("{indent}{name}")).weak());
                } else if ui
                    .selectable_label(*value == idx, format!("{indent}{name}"))
                    .clicked()
                {
                    *value = idx;
                    changed = true;
                }
            }
        });
    changed
}

/// Items of a resource menu: `tree_order` entries (index, name, depth; groups < 0).
type TreeItems = [(i32, String, usize)];

/// Menu entries for the resources in `items` (groups become submenus).
fn resource_menu_items(ui: &mut Ui, items: &TreeItems, value: &mut i32, changed: &mut bool) {
    let mut i = 0;
    while i < items.len() {
        let (idx, name, depth) = &items[i];
        // children of a group: the following items deeper than it
        let end = items[i + 1..]
            .iter()
            .position(|x| x.2 <= *depth)
            .map_or(items.len(), |k| i + 1 + k);
        if *idx < 0 {
            let children = &items[i + 1..end];
            if children.is_empty() {
                ui.add_enabled(false, egui::Button::new(name.as_str()));
            } else {
                ui.menu_button(name.as_str(), |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(480.0)
                        .show(ui, |ui| {
                            resource_menu_items(ui, children, value, changed);
                        });
                });
            }
            i = end;
        } else {
            if ui.selectable_label(*value == *idx, name.as_str()).clicked() {
                *value = *idx;
                *changed = true;
                ui.close();
            }
            i += 1;
        }
    }
}

/// GameMaker-style resource selector: a read-only field showing the current resource
/// and a small menu button; clicking either opens a menu of the resources (groups as
/// submenus, plus the "none" entry). Fixed size `width` × 20. Returns true when changed.
#[allow(clippy::too_many_arguments)]
pub fn resource_menu_field(
    ui: &mut Ui,
    id_salt: impl Hash + std::fmt::Debug,
    p: &Project,
    kind: ResourceKind,
    value: &mut i32,
    none_text: &str,
    none_value: i32,
    width: f32,
) -> bool {
    let current = if *value == none_value || *value < 0 {
        none_text.to_string()
    } else {
        res_name(p, kind, *value as usize)
            .map_or_else(|| format!("<missing {}>", *value), str::to_string)
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 20.0), egui::Sense::hover());
    let resp = ui.interact(
        rect,
        ui.id().with(("res_menu", id_salt)),
        egui::Sense::click(),
    );
    let enabled = ui.is_enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, enabled, &current));
    let field = Rect::from_min_max(rect.min, egui::pos2(rect.right() - 21.0, rect.bottom()));
    let button = Rect::from_min_max(egui::pos2(rect.right() - 20.0, rect.top()), rect.max);
    if ui.is_rect_visible(rect) {
        let pt = ui.painter();
        let ink = if enabled {
            Color32::from_gray(10)
        } else {
            Color32::from_gray(140)
        };
        pt.rect_filled(field, 0.0, Color32::from_rgb(246, 246, 246));
        pt.rect_stroke(
            field,
            0.0,
            egui::Stroke::new(1.0, Color32::from_gray(170)),
            egui::StrokeKind::Inside,
        );
        let galley = egui::WidgetText::from(current.as_str()).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            field.width() - 8.0,
            egui::TextStyle::Body,
        );
        pt.galley(
            egui::pos2(field.left() + 4.0, field.center().y - galley.size().y / 2.0),
            galley,
            ink,
        );
        let hot = enabled && resp.hovered();
        let down = enabled
            && (resp.is_pointer_button_down_on()
                || egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp)));
        let fill = if down {
            Color32::from_rgb(204, 228, 247)
        } else if hot {
            Color32::from_rgb(229, 241, 251)
        } else {
            Color32::from_rgb(225, 225, 225)
        };
        let stroke = if hot || down {
            Color32::from_rgb(0, 120, 215)
        } else {
            Color32::from_gray(173)
        };
        pt.rect(
            button,
            2.0,
            fill,
            egui::Stroke::new(1.0, stroke),
            egui::StrokeKind::Inside,
        );
        // menu glyph: three short lines and a small triangle
        let c = button.center();
        for k in 0..3 {
            let y = c.y - 4.0 + k as f32 * 3.0;
            pt.hline(
                egui::Rangef::new(c.x - 5.0, c.x + 1.0),
                y,
                egui::Stroke::new(1.0, ink),
            );
        }
        pt.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(c.x + 2.0, c.y + 1.0),
                egui::pos2(c.x + 7.0, c.y + 1.0),
                egui::pos2(c.x + 4.5, c.y + 4.5),
            ],
            ink,
            egui::Stroke::NONE,
        ));
    }
    let mut changed = false;
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(width.max(140.0));
        if ui
            .selectable_label(*value == none_value || *value < 0, none_text)
            .clicked()
        {
            *value = none_value;
            changed = true;
            ui.close();
        }
        let items = tree_order(p, kind);
        egui::ScrollArea::vertical()
            .max_height(480.0)
            .show(ui, |ui| {
                resource_menu_items(ui, &items, value, &mut changed);
            });
    });
    changed
}

/// A classic group box: an etched frame of fixed `width` with `title` on its top edge.
pub fn group_box<R>(ui: &mut Ui, title: &str, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(7.0);
    let r = egui::Frame::NONE
        .stroke(egui::Stroke::new(1.0, Color32::from_gray(190)))
        .corner_radius(3)
        .inner_margin(egui::Margin {
            left: 7,
            right: 7,
            top: 11,
            bottom: 7,
        })
        .show(ui, |ui| {
            ui.set_width(width - 16.0);
            add(ui)
        });
    let rect = r.response.rect;
    let pt = ui.painter();
    let galley = pt.layout_no_wrap(
        title.to_string(),
        egui::TextStyle::Body.resolve(ui.style()),
        Color32::from_gray(10),
    );
    let pos = egui::pos2(rect.left() + 8.0, rect.top() - galley.size().y / 2.0);
    pt.rect_filled(
        Rect::from_min_size(
            pos - Vec2::new(3.0, 0.0),
            galley.size() + Vec2::new(6.0, 0.0),
        ),
        0.0,
        crate::theme::PANEL,
    );
    pt.galley(pos, galley, Color32::from_gray(10));
    r.inner
}

/// Simple modal text prompt state used by several editors.
#[derive(Default, Clone)]
pub struct Prompt {
    pub title: String,
    pub fields: Vec<(String, String)>,
}

/// Result of [`show_prompt`].
pub enum PromptResult {
    None,
    Ok(Vec<String>),
    Cancel,
}

/// Show a modal with labelled text fields and OK/Cancel.
pub fn show_prompt(ctx: &Context, id: egui::Id, prompt: &mut Prompt) -> PromptResult {
    let mut res = PromptResult::None;
    let title = prompt.title.clone();
    let d = crate::mdi::dialog(ctx, id, &title, |ui| {
        egui::Grid::new(id.with("g")).num_columns(2).show(ui, |ui| {
            for (i, (label, value)) in prompt.fields.iter_mut().enumerate() {
                ui.label(label.as_str());
                let r = ui.text_edit_singleline(value);
                if i == 0 && !r.has_focus() && ui.memory(|m| m.focused().is_none()) {
                    r.request_focus();
                }
                ui.end_row();
            }
        });
        ui.horizontal(|ui| {
            if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                res = PromptResult::Ok(prompt.fields.iter().map(|f| f.1.clone()).collect());
            }
            if ui.button("Cancel").clicked() {
                res = PromptResult::Cancel;
            }
        });
    });
    if d.close && matches!(res, PromptResult::None) {
        res = PromptResult::Cancel;
    }
    res
}
