//! Common widgets (specs/ide.md §4): resource picker, colour box, image previews.

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
    egui::Modal::new(id).show(ctx, |ui| {
        ui.heading(&prompt.title);
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
            if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                res = PromptResult::Cancel;
            }
        });
    });
    res
}
