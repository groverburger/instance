//! Sprite editor window (specs/ide.md §6): frame strip, frame operations, animated
//! preview, Transform / Images / Animation menus, strip import (§6.1), and the image
//! editor for single frames.

use eframe::egui::{
    self, Color32, Context, Id, Pos2, Rect, Sense, Stroke, TextureHandle, Ui, Vec2,
};
use instance_format::RgbaImage;

use super::image_editor::effects as fx;
use super::image_editor::history::History;
use super::image_editor::ops::{self, AnimOp, DialogResult, ImageOp, OpDialog};
use super::image_editor::raster::Rgba;
use super::image_editor::{clipboard_get, clipboard_set, EditorAction, ImageEditor};
use crate::imageio::{self, LoadOptions};
use crate::widgets::{owner_key, TextureCache};

/// Result of a sprite editor frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteEditorAction {
    None,
    Close { save: bool },
}

/// How an image of another size is fitted into the sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    TopLeft,
    Center,
    Stretch,
}

/// Fit `img` into `w`×`h` according to `p`.
pub fn adapt(img: &RgbaImage, w: u32, h: u32, p: Placement) -> RgbaImage {
    if (img.width, img.height) == (w, h) {
        return img.clone();
    }
    match p {
        Placement::TopLeft => fx::resize_canvas(img, w, h, (0, 0)),
        Placement::Center => fx::resize_canvas(img, w, h, (1, 1)),
        Placement::Stretch => fx::stretch(img, w, h, true),
    }
}

// ------------------------------------------------------------------ strip import

/// Parameters of the strip dialog (§6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StripParams {
    pub count: u32,
    pub per_row: u32,
    pub width: u32,
    pub height: u32,
    /// Cells to skip horizontally / vertically.
    pub cell_x: u32,
    pub cell_y: u32,
    /// Pixel offset of the first cell.
    pub px_x: u32,
    pub px_y: u32,
    /// Pixels between cells.
    pub sep_x: u32,
    pub sep_y: u32,
}

impl StripParams {
    /// Defaults for a source image: one row of square cells when it is wide.
    pub fn for_image(img: &RgbaImage) -> Self {
        let h = img.height.max(1);
        let n = if img.width >= h {
            (img.width / h).max(1)
        } else {
            1
        };
        StripParams {
            count: n,
            per_row: n,
            width: if n > 1 { h } else { img.width.max(1) },
            height: h,
            cell_x: 0,
            cell_y: 0,
            px_x: 0,
            px_y: 0,
            sep_x: 0,
            sep_y: 0,
        }
    }

    /// Top-left corners of the cells, row-major.
    pub fn cells(&self) -> Vec<(i32, i32)> {
        let per_row = self.per_row.max(1);
        (0..self.count)
            .map(|k| {
                let col = self.cell_x + k % per_row;
                let row = self.cell_y + k / per_row;
                (
                    (self.px_x + col * (self.width + self.sep_x)) as i32,
                    (self.px_y + row * (self.height + self.sep_y)) as i32,
                )
            })
            .collect()
    }

    /// Cut the frames out of `src`.
    pub fn extract(&self, src: &RgbaImage) -> Vec<RgbaImage> {
        self.cells()
            .into_iter()
            .map(|(x, y)| {
                super::image_editor::raster::copy_rect(
                    src,
                    (x, y, self.width.max(1), self.height.max(1)),
                )
            })
            .collect()
    }
}

struct StripDialog {
    src: RgbaImage,
    params: StripParams,
    /// Append instead of replace.
    add: bool,
    tex: Option<TextureHandle>,
    zoom: f32,
}

// ------------------------------------------------------------------ editor

enum Pending {
    /// Frames of another size waiting for a placement choice.
    Fit {
        frames: Vec<RgbaImage>,
        insert_at: usize,
    },
}

/// The sprite editor window.
pub struct SpriteFramesEditor {
    pub frames: Vec<RgbaImage>,
    name: String,
    uid: u64,
    pub sel: usize,
    history: History<(Vec<RgbaImage>, usize)>,
    gen: u64,
    pub modified: bool,
    show_preview: bool,
    speed: f32,
    anim_pos: f64,
    preview_bg: Rgba,
    preview_checker: bool,
    opts: LoadOptions,
    pub image_editor: Option<(usize, ImageEditor)>,
    dialog: Option<OpDialog>,
    anim_dialog: Option<AnimOp>,
    strip: Option<StripDialog>,
    new_size: Option<(u32, u32)>,
    erase: Option<(Rgba, bool)>,
    pending: Option<Pending>,
    placement: Placement,
    clip: Option<RgbaImage>,
    confirm_close: bool,
    status: String,
}

impl SpriteFramesEditor {
    pub fn new(frames: Vec<RgbaImage>, name: &str, uid: u64) -> Self {
        SpriteFramesEditor {
            frames,
            name: name.to_string(),
            uid,
            sel: 0,
            history: History::new(16),
            gen: 1,
            modified: false,
            show_preview: true,
            speed: 10.0,
            anim_pos: 0.0,
            preview_bg: [255, 255, 255, 255],
            preview_checker: true,
            opts: LoadOptions::SPRITE,
            image_editor: None,
            dialog: None,
            anim_dialog: None,
            strip: None,
            new_size: None,
            erase: None,
            pending: None,
            placement: Placement::TopLeft,
            clip: None,
            confirm_close: false,
            status: String::new(),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        self.frames
            .first()
            .map_or((32, 32), |f| (f.width, f.height))
    }

    /// Record the state before a change.
    fn checkpoint(&mut self) {
        self.history.push((self.frames.clone(), self.sel));
        self.modified = true;
        self.gen += 1;
    }

    fn changed(&mut self) {
        self.gen += 1;
        self.sel = self.sel.min(self.frames.len().saturating_sub(1));
    }

    pub fn undo(&mut self) {
        if let Some((f, s)) = self.history.undo((self.frames.clone(), self.sel)) {
            self.frames = f;
            self.sel = s;
            self.changed();
        }
    }

    pub fn redo(&mut self) {
        if let Some((f, s)) = self.history.redo((self.frames.clone(), self.sel)) {
            self.frames = f;
            self.sel = s;
            self.changed();
        }
    }

    // ------------------------------------------------------------ frame operations

    pub fn add_empty(&mut self) {
        self.checkpoint();
        let (w, h) = self.size();
        self.frames.push(RgbaImage::new(w, h));
        self.sel = self.frames.len() - 1;
        self.changed();
    }

    pub fn insert_empty(&mut self) {
        self.checkpoint();
        let (w, h) = self.size();
        let at = self.sel.min(self.frames.len());
        self.frames.insert(at, RgbaImage::new(w, h));
        self.sel = at;
        self.changed();
    }

    pub fn duplicate(&mut self) {
        if let Some(f) = self.frames.get(self.sel).cloned() {
            self.checkpoint();
            self.frames.insert(self.sel + 1, f);
            self.sel += 1;
            self.changed();
        }
    }

    pub fn delete(&mut self) {
        if self.sel < self.frames.len() {
            self.checkpoint();
            self.frames.remove(self.sel);
            self.changed();
        }
    }

    pub fn move_left(&mut self) {
        if self.sel > 0 && self.sel < self.frames.len() {
            self.checkpoint();
            self.frames.swap(self.sel, self.sel - 1);
            self.sel -= 1;
            self.changed();
        }
    }

    pub fn move_right(&mut self) {
        if self.sel + 1 < self.frames.len() {
            self.checkpoint();
            self.frames.swap(self.sel, self.sel + 1);
            self.sel += 1;
            self.changed();
        }
    }

    /// Insert frames at `at`, fitting them to the sprite size with the current
    /// placement (the first frames of an empty sprite set its size).
    pub fn insert_frames(&mut self, new: Vec<RgbaImage>, at: usize, placement: Placement) {
        if new.is_empty() {
            return;
        }
        self.checkpoint();
        let (w, h) = if self.frames.is_empty() {
            (new[0].width, new[0].height)
        } else {
            self.size()
        };
        let at = at.min(self.frames.len());
        for (k, f) in new.iter().enumerate() {
            self.frames.insert(at + k, adapt(f, w, h, placement));
        }
        self.sel = at;
        self.changed();
    }

    /// Insert frames, asking for a placement when their size differs.
    fn offer_frames(&mut self, new: Vec<RgbaImage>, at: usize) {
        if new.is_empty() {
            return;
        }
        let (w, h) = self.size();
        if !self.frames.is_empty() && new.iter().any(|f| (f.width, f.height) != (w, h)) {
            self.pending = Some(Pending::Fit {
                frames: new,
                insert_at: at,
            });
        } else {
            self.insert_frames(new, at, Placement::TopLeft);
        }
    }

    /// Replace all frames.
    pub fn replace_frames(&mut self, new: Vec<RgbaImage>) {
        self.checkpoint();
        self.frames = new;
        self.sel = 0;
        self.changed();
    }

    pub fn apply_op(&mut self, op: &ImageOp, all: bool) {
        if self.frames.is_empty() {
            return;
        }
        self.checkpoint();
        ops::apply_to_frames(op, &mut self.frames, self.sel, all);
        self.changed();
    }

    pub fn apply_anim(&mut self, op: &AnimOp) {
        if self.frames.is_empty() {
            return;
        }
        self.checkpoint();
        self.frames = op.apply(&self.frames);
        self.changed();
    }

    fn start_op(&mut self, op: ImageOp) {
        if op.has_dialog() || self.frames.len() > 1 {
            self.dialog = Some(OpDialog::new(op));
        } else {
            self.apply_op(&op, true);
        }
    }

    /// Open the image editor on frame `i`.
    pub fn edit_frame(&mut self, i: usize) {
        let Some(f) = self.frames.get(i) else {
            return;
        };
        let onion = i.checked_sub(1).and_then(|p| self.frames.get(p)).cloned();
        let ed = ImageEditor::for_frame(f.clone(), onion, i, self.frames.len(), self.name.clone());
        self.sel = i;
        self.image_editor = Some((i, ed));
    }

    /// Commit the image editor's image to its frame.
    fn commit_editor(&mut self) {
        let Some((i, ed)) = &self.image_editor else {
            return;
        };
        let (i, img) = (*i, ed.image.clone());
        if ed.modified && self.frames.get(i).is_some_and(|f| *f != img) {
            self.checkpoint();
            if self.frames.len() == 1 || (img.width, img.height) == self.size() {
                self.frames[i] = img;
            } else {
                let (w, h) = self.size();
                self.frames[i] = adapt(&img, w, h, Placement::TopLeft);
            }
            self.changed();
        }
    }

    fn copy_frame(&mut self) {
        if let Some(f) = self.frames.get(self.sel) {
            clipboard_set(f);
            self.clip = Some(f.clone());
        }
    }

    fn paste_frame(&mut self) {
        if let Some(img) = clipboard_get().or_else(|| self.clip.clone()) {
            let at = (self.sel + 1).min(self.frames.len());
            self.offer_frames(vec![img], at);
        }
    }

    // ------------------------------------------------------------ UI

    /// Show the window (and the image editor, when open).
    pub fn show(&mut self, ctx: &Context, tex: &mut TextureCache, id: Id) -> SpriteEditorAction {
        let mut action = SpriteEditorAction::None;
        let title = format!("Sprite Editor: {}", self.name);
        let out = crate::mdi::floating(
            ctx,
            id,
            &title,
            crate::icons::Icon::Sprite,
            Vec2::new(820.0, 560.0),
            |ui| {
                if self.image_editor.is_some() || self.has_modal() {
                    ui.disable();
                }
                action = self.window_ui(ui, tex);
            },
        );
        if self.image_editor.is_some() {
            // the image editor stays above the sprite editor that owns it
            crate::mdi::bring_to_front(ctx, id.with("image"));
        }
        if out.close {
            if self.modified {
                self.confirm_close = true;
            } else {
                action = SpriteEditorAction::Close { save: false };
            }
        }
        self.modals(ctx, id, &mut action);
        // image editor
        let ed_action = match &mut self.image_editor {
            Some((_, ed)) => ed.show(ctx, id.with("image")),
            None => EditorAction::None,
        };
        match ed_action {
            EditorAction::None => {}
            EditorAction::Close { save } => {
                if save {
                    self.commit_editor();
                }
                self.image_editor = None;
            }
            EditorAction::Prev | EditorAction::Next => {
                self.commit_editor();
                let n = self.frames.len();
                if let Some((i, ed)) = &mut self.image_editor {
                    let j = if ed_action == EditorAction::Next {
                        (*i + 1).min(n.saturating_sub(1))
                    } else {
                        i.saturating_sub(1)
                    };
                    if let Some(f) = self.frames.get(j) {
                        let onion = j.checked_sub(1).and_then(|p| self.frames.get(p)).cloned();
                        ed.load_frame(f.clone(), onion, j, n);
                        *i = j;
                        self.sel = j;
                    }
                }
            }
        }
        action
    }

    fn has_modal(&self) -> bool {
        self.dialog.is_some()
            || self.anim_dialog.is_some()
            || self.strip.is_some()
            || self.new_size.is_some()
            || self.erase.is_some()
            || self.pending.is_some()
            || self.confirm_close
    }

    fn modals(&mut self, ctx: &Context, id: Id, action: &mut SpriteEditorAction) {
        if self.confirm_close {
            let d = crate::mdi::dialog(ctx, id.with("confirm"), "Sprite Editor", |ui| {
                ui.label("Save the changes to the sprite?");
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        *action = SpriteEditorAction::Close { save: true };
                    }
                    if ui.button("No").clicked() {
                        *action = SpriteEditorAction::Close { save: false };
                    }
                    if ui.button("Cancel").clicked() {
                        self.confirm_close = false;
                    }
                });
            });
            if d.close {
                self.confirm_close = false;
            }
        }
        let multi = self.frames.len() > 1;
        if let Some(d) = &mut self.dialog {
            match d.show(ctx, id.with("op"), multi) {
                DialogResult::Open => {}
                DialogResult::Ok => {
                    let (op, all) = (d.op.clone(), d.all_frames);
                    self.dialog = None;
                    self.apply_op(&op, all);
                }
                DialogResult::Cancel => self.dialog = None,
            }
        }
        if let Some(op) = &mut self.anim_dialog {
            let mut res = DialogResult::Open;
            let title = op.name().to_string();
            let m = crate::mdi::dialog(ctx, id.with("anim"), &title, |ui| {
                ui.set_min_width(280.0);
                op.ui(ui);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        res = DialogResult::Ok;
                    }
                    if ui.button("Cancel").clicked() {
                        res = DialogResult::Cancel;
                    }
                });
            });
            if m.should_close() && res == DialogResult::Open {
                res = DialogResult::Cancel;
            }
            match res {
                DialogResult::Ok => {
                    let op = op.clone();
                    self.anim_dialog = None;
                    self.apply_anim(&op);
                }
                DialogResult::Cancel => self.anim_dialog = None,
                DialogResult::Open => {}
            }
        }
        if let Some((w, h)) = &mut self.new_size {
            let mut res = DialogResult::Open;
            let d = crate::mdi::dialog(ctx, id.with("new"), "New Sprite", |ui| {
                egui::Grid::new("new_sprite").show(ui, |ui| {
                    ui.label("Width");
                    ui.add(egui::DragValue::new(w).range(1..=4096));
                    ui.end_row();
                    ui.label("Height");
                    ui.add(egui::DragValue::new(h).range(1..=4096));
                    ui.end_row();
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        res = DialogResult::Ok;
                    }
                    if ui.button("Cancel").clicked() {
                        res = DialogResult::Cancel;
                    }
                });
            });
            if d.close && res == DialogResult::Open {
                res = DialogResult::Cancel;
            }
            match res {
                DialogResult::Ok => {
                    let (w, h) = (*w, *h);
                    self.new_size = None;
                    self.replace_frames(vec![RgbaImage::new(w, h)]);
                }
                DialogResult::Cancel => self.new_size = None,
                DialogResult::Open => {}
            }
        }
        if let Some((c, all)) = &mut self.erase {
            let mut res = DialogResult::Open;
            let d = crate::mdi::dialog(ctx, id.with("erase"), "Erase", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Fill colour (with alpha)");
                    ui.color_edit_button_srgba_unmultiplied(c);
                });
                ui.horizontal(|ui| {
                    ui.radio_value(all, false, "Current frame");
                    ui.radio_value(all, true, "All frames");
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        res = DialogResult::Ok;
                    }
                    if ui.button("Cancel").clicked() {
                        res = DialogResult::Cancel;
                    }
                });
            });
            if d.close && res == DialogResult::Open {
                res = DialogResult::Cancel;
            }
            match res {
                DialogResult::Ok => {
                    let (c, all) = (*c, *all);
                    self.erase = None;
                    if !self.frames.is_empty() {
                        self.checkpoint();
                        for (i, f) in self.frames.iter_mut().enumerate() {
                            if all || i == self.sel {
                                for px in f.data.as_chunks_mut::<4>().0 {
                                    px.copy_from_slice(&c);
                                }
                            }
                        }
                        self.changed();
                    }
                }
                DialogResult::Cancel => self.erase = None,
                DialogResult::Open => {}
            }
        }
        if let Some(Pending::Fit { frames, insert_at }) = &self.pending {
            let mut res = None;
            let (w, h) = self.size();
            let (nw, nh) = (frames[0].width, frames[0].height);
            let d = crate::mdi::dialog(ctx, id.with("fit"), "Image Size", |ui| {
                ui.label(format!(
                    "The image ({nw}×{nh}) differs from the sprite size ({w}×{h})."
                ));
                ui.horizontal(|ui| {
                    if ui.button("Place top-left").clicked() {
                        res = Some(Some(Placement::TopLeft));
                    }
                    if ui.button("Center").clicked() {
                        res = Some(Some(Placement::Center));
                    }
                    if ui.button("Stretch").clicked() {
                        res = Some(Some(Placement::Stretch));
                    }
                    if ui.button("Cancel").clicked() {
                        res = Some(None);
                    }
                });
            });
            if d.close && res.is_none() {
                res = Some(None);
            }
            if let Some(r) = res {
                let (frames, at) = (frames.clone(), *insert_at);
                self.pending = None;
                if let Some(p) = r {
                    self.placement = p;
                    self.insert_frames(frames, at, p);
                }
            }
        }
        self.strip_dialog(ctx, id);
    }

    fn strip_dialog(&mut self, ctx: &Context, id: Id) {
        let Some(sd) = &mut self.strip else {
            return;
        };
        let mut res = DialogResult::Open;
        let tex = sd
            .tex
            .get_or_insert_with(|| {
                ctx.load_texture(
                    "strip-src",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [sd.src.width as usize, sd.src.height as usize],
                        &sd.src.data,
                    ),
                    egui::TextureOptions::NEAREST,
                )
            })
            .id();
        let title = if sd.add {
            "Add from Strip"
        } else {
            "Create from Strip"
        };
        let d = crate::mdi::dialog(ctx, id.with("strip"), title, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    let p = &mut sd.params;
                    egui::Grid::new("strip_params")
                        .num_columns(2)
                        .show(ui, |ui| {
                            for (l, v, max) in [
                                ("Number of images", &mut p.count, 4096),
                                ("Images per row", &mut p.per_row, 4096),
                                ("Image width", &mut p.width, 8192),
                                ("Image height", &mut p.height, 8192),
                                ("Horizontal cell offset", &mut p.cell_x, 4096),
                                ("Vertical cell offset", &mut p.cell_y, 4096),
                                ("Horizontal pixel offset", &mut p.px_x, 8192),
                                ("Vertical pixel offset", &mut p.px_y, 8192),
                                ("Horizontal separation", &mut p.sep_x, 8192),
                                ("Vertical separation", &mut p.sep_y, 8192),
                            ] {
                                ui.label(l);
                                ui.add(egui::DragValue::new(v).range(0..=max));
                                ui.end_row();
                            }
                        });
                    p.count = p.count.max(1);
                    p.per_row = p.per_row.max(1);
                    p.width = p.width.max(1);
                    p.height = p.height.max(1);
                    ui.horizontal(|ui| {
                        ui.label("Zoom");
                        ui.add(egui::Slider::new(&mut sd.zoom, 0.25..=8.0).logarithmic(true));
                    });
                    ui.label("Click the image to set the pixel offset.");
                });
                let (w, h) = (
                    sd.src.width as f32 * sd.zoom,
                    sd.src.height as f32 * sd.zoom,
                );
                egui::ScrollArea::both()
                    .id_salt("strip_scroll")
                    .max_width(520.0)
                    .max_height(420.0)
                    .show(ui, |ui| {
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
                        let painter = ui.painter_at(r);
                        crate::widgets::checker(&painter, r, 8.0);
                        painter.image(
                            tex,
                            r,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        let p = sd.params;
                        for (x, y) in p.cells() {
                            let cr = Rect::from_min_size(
                                r.min + Vec2::new(x as f32, y as f32) * sd.zoom,
                                Vec2::new(p.width as f32, p.height as f32) * sd.zoom,
                            );
                            painter.rect_stroke(
                                cr,
                                0.0,
                                Stroke::new(1.0, Color32::from_rgb(255, 0, 160)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if resp.clicked() {
                            if let Some(pos) = resp.interact_pointer_pos() {
                                let v = (pos - r.min) / sd.zoom;
                                sd.params.px_x = v.x.max(0.0) as u32;
                                sd.params.px_y = v.y.max(0.0) as u32;
                            }
                        }
                    });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    res = DialogResult::Ok;
                }
                if ui.button("Cancel").clicked() {
                    res = DialogResult::Cancel;
                }
            });
        });
        if d.close && res == DialogResult::Open {
            res = DialogResult::Cancel;
        }
        match res {
            DialogResult::Ok => {
                let sd = self.strip.take().expect("strip dialog");
                let frames = sd.params.extract(&sd.src);
                if sd.add {
                    let at = self.frames.len();
                    self.offer_frames(frames, at);
                } else {
                    self.replace_frames(frames);
                }
            }
            DialogResult::Cancel => self.strip = None,
            DialogResult::Open => {}
        }
    }

    fn open_strip(&mut self, add: bool) {
        crate::dialogs::request_open(self.strip_key(add), crate::dialogs::IMAGES, false);
    }

    fn strip_key(&self, add: bool) -> u64 {
        crate::dialogs::key(
            if add {
                "frames-strip-add"
            } else {
                "frames-strip-new"
            },
            self as *const Self,
        )
    }

    /// Picked files of the File menu's dialogs (polled every frame).
    fn poll_dialogs(&mut self) {
        let me = self as *const Self;
        let picked = |purpose: &str| -> Option<Result<Vec<RgbaImage>, String>> {
            let r = crate::dialogs::take_opened(crate::dialogs::key(purpose, me))?;
            Some(r.and_then(|files| {
                let files: Vec<_> = files.into_iter().map(|f| (f.name, f.bytes)).collect();
                imageio::frames_from_bytes(&files, self.opts)
            }))
        };
        let create = picked("frames-create");
        let add = picked("frames-add");
        match create {
            Some(Ok(f)) => self.replace_frames(f),
            Some(Err(e)) => self.status = e,
            None => {}
        }
        match add {
            Some(Ok(f)) => {
                let at = self.frames.len();
                self.offer_frames(f, at);
            }
            Some(Err(e)) => self.status = e,
            None => {}
        }
        for add in [false, true] {
            let Some(r) = crate::dialogs::take_opened(self.strip_key(add)) else {
                continue;
            };
            let r = r.and_then(|files| {
                let files: Vec<_> = files.into_iter().map(|f| (f.name, f.bytes)).collect();
                imageio::frames_from_bytes(&files, self.opts)
            });
            {
                match r {
                    Ok(mut fr) if !fr.is_empty() => {
                        let src = fr.swap_remove(0);
                        self.strip = Some(StripDialog {
                            params: StripParams::for_image(&src),
                            src,
                            add,
                            tex: None,
                            zoom: 1.0,
                        });
                    }
                    Ok(_) => {}
                    Err(e) => self.status = e,
                }
            }
        }
    }

    /// Open the strip dialog on an already loaded image (used by tests too).
    #[allow(dead_code)]
    pub fn open_strip_image(&mut self, src: RgbaImage, add: bool) {
        self.strip = Some(StripDialog {
            params: StripParams::for_image(&src),
            src,
            add,
            tex: None,
            zoom: 1.0,
        });
    }

    fn window_ui(&mut self, ui: &mut Ui, tex: &mut TextureCache) -> SpriteEditorAction {
        let mut action = SpriteEditorAction::None;
        egui::MenuBar::new().ui(ui, |ui| {
            action = self.menus(ui);
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("✔ Close Saving").clicked() {
                action = SpriteEditorAction::Close { save: true };
            }
            ui.separator();
            if ui
                .add_enabled(self.history.can_undo(), egui::Button::new("⟲ Undo"))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(self.history.can_redo(), egui::Button::new("⟳ Redo"))
                .clicked()
            {
                self.redo();
            }
            ui.separator();
            let n = self.frames.len();
            if ui
                .button("Add")
                .on_hover_text("Add an empty frame at the end")
                .clicked()
            {
                self.add_empty();
            }
            if ui
                .button("Insert")
                .on_hover_text("Insert an empty frame before the selected one")
                .clicked()
            {
                self.insert_empty();
            }
            if ui
                .add_enabled(n > 0, egui::Button::new("Duplicate"))
                .clicked()
            {
                self.duplicate();
            }
            if ui.add_enabled(n > 0, egui::Button::new("Delete")).clicked() {
                self.delete();
            }
            if ui
                .add_enabled(self.sel > 0 && n > 1, egui::Button::new("◀ Move"))
                .clicked()
            {
                self.move_left();
            }
            if ui
                .add_enabled(self.sel + 1 < n, egui::Button::new("Move ▶"))
                .clicked()
            {
                self.move_right();
            }
            ui.separator();
            if ui
                .add_enabled(n > 0, egui::Button::new("✏ Edit Frame"))
                .clicked()
            {
                self.edit_frame(self.sel);
            }
        });
        let (w, h) = self.size();
        egui::Panel::bottom(ui.id().with("sprite_status"))
            .exact_size(22.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let n = self.frames.len();
                    let mem = n as u64 * w as u64 * h as u64 * 4;
                    ui.label(format!(
                        "Frames: {n}   Size: {w}×{h}   Memory: {} KB",
                        mem.div_ceil(1024)
                    ));
                    if !self.status.is_empty() {
                        ui.separator();
                        ui.colored_label(ui.visuals().warn_fg_color, &self.status);
                    }
                });
            });
        egui::Panel::left(ui.id().with("sprite_preview"))
            .exact_size(220.0)
            .resizable(false)
            .show(ui, |ui| self.preview_ui(ui, tex));
        egui::CentralPanel::default().show(ui, |ui| self.strip_ui(ui, tex));
        action
    }

    fn menus(&mut self, ui: &mut Ui) -> SpriteEditorAction {
        self.poll_dialogs();
        let me = self as *const Self;
        let mut action = SpriteEditorAction::None;
        let (w, h) = self.size();
        let n = self.frames.len();
        ui.menu_button("File", |ui| {
            if ui.button("New…").clicked() {
                self.new_size = Some((w, h));
            }
            if ui.button("Create from File…").clicked() {
                pick_files(crate::dialogs::key("frames-create", me));
            }
            if ui.button("Add from File…").clicked() {
                pick_files(crate::dialogs::key("frames-add", me));
            }
            ui.separator();
            ui.checkbox(&mut self.opts.make_opaque, "Make opaque");
            ui.checkbox(&mut self.opts.remove_background, "Remove background");
            ui.checkbox(&mut self.opts.smooth_edges, "Smooth edges");
            ui.separator();
            if ui.button("Save as PNG File…").clicked() {
                let file = if n > 1 {
                    format!("{}_strip{n}.png", self.name)
                } else {
                    format!("{}.png", self.name)
                };
                if let Err(e) = crate::dialogs::save_encoded(
                    &file,
                    crate::dialogs::PNG,
                    imageio::strip_png_bytes(&self.frames),
                ) {
                    self.status = e;
                }
            }
            if ui.button("Create from Strip…").clicked() {
                self.open_strip(false);
            }
            if ui.button("Add from Strip…").clicked() {
                self.open_strip(true);
            }
            ui.separator();
            if ui.button("Close Saving Changes").clicked() {
                action = SpriteEditorAction::Close { save: true };
            }
            if ui.button("Close Discarding Changes").clicked() {
                action = SpriteEditorAction::Close { save: false };
            }
        });
        ui.menu_button("Edit", |ui| {
            if ui
                .add_enabled(self.history.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(self.history.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                self.redo();
            }
            ui.separator();
            if ui.add_enabled(n > 0, egui::Button::new("Cut")).clicked() {
                self.copy_frame();
                self.delete();
            }
            if ui.add_enabled(n > 0, egui::Button::new("Copy")).clicked() {
                self.copy_frame();
            }
            if ui.button("Paste").clicked() {
                self.paste_frame();
            }
            if ui.add_enabled(n > 0, egui::Button::new("Erase…")).clicked() {
                self.erase = Some(([0, 0, 0, 0], false));
            }
            if ui.add_enabled(n > 0, egui::Button::new("Delete")).clicked() {
                self.delete();
            }
            ui.separator();
            if ui
                .add_enabled(self.sel > 0, egui::Button::new("Move Left"))
                .clicked()
            {
                self.move_left();
            }
            if ui
                .add_enabled(self.sel + 1 < n, egui::Button::new("Move Right"))
                .clicked()
            {
                self.move_right();
            }
            ui.separator();
            if ui.button("Add Empty").clicked() {
                self.add_empty();
            }
            if ui.button("Insert Empty").clicked() {
                self.insert_empty();
            }
            if ui
                .add_enabled(n > 0, egui::Button::new("Duplicate"))
                .clicked()
            {
                self.duplicate();
            }
            if ui.add_enabled(n > 0, egui::Button::new("Edit…")).clicked() {
                self.edit_frame(self.sel);
            }
        });
        ui.menu_button("Transform", |ui| {
            if let Some(op) = ops::transform_menu(ui, w, h, true) {
                self.start_op(op);
            }
        });
        ui.menu_button("Images", |ui| {
            if ui
                .add_enabled(n > 1, egui::Button::new("Cycle Left"))
                .clicked()
            {
                self.checkpoint();
                fx::cycle_left(&mut self.frames);
                self.changed();
            }
            if ui
                .add_enabled(n > 1, egui::Button::new("Cycle Right"))
                .clicked()
            {
                self.checkpoint();
                fx::cycle_right(&mut self.frames);
                self.changed();
            }
            ui.separator();
            if let Some(op) = ops::image_menu(ui, [255, 255, 255, 255]) {
                self.start_op(op);
            }
        });
        ui.menu_button("Animation", |ui| {
            if let Some(op) = ops::animation_menu(ui, n) {
                if op.has_dialog() {
                    self.anim_dialog = Some(op);
                } else {
                    self.apply_anim(&op);
                }
            }
        });
        action
    }

    fn preview_ui(&mut self, ui: &mut Ui, tex: &mut TextureCache) {
        ui.checkbox(&mut self.show_preview, "Show Preview");
        ui.horizontal(|ui| {
            ui.label("Speed");
            ui.add(egui::Slider::new(&mut self.speed, 0.5..=60.0).suffix(" fps"));
        });
        ui.horizontal(|ui| {
            ui.label("Background");
            ui.color_edit_button_srgba_unmultiplied(&mut self.preview_bg);
            ui.checkbox(&mut self.preview_checker, "Checker");
        });
        let n = self.frames.len();
        if !self.show_preview || n == 0 {
            return;
        }
        let dt = ui.input(|i| i.stable_dt).min(0.1) as f64;
        self.anim_pos = (self.anim_pos + dt * self.speed as f64) % n as f64;
        let k = (self.anim_pos.floor() as usize).min(n - 1);
        ui.ctx().request_repaint();
        let f = &self.frames[k];
        let t = tex.get(ui.ctx(), self.owner(), k, self.gen, f);
        let box_size = Vec2::splat(200.0);
        let (r, _) = ui.allocate_exact_size(box_size, Sense::hover());
        let p = ui.painter_at(r);
        if self.preview_checker {
            crate::widgets::checker(&p, r, 8.0);
        } else {
            let c = self.preview_bg;
            p.rect_filled(
                r,
                0.0,
                Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 255),
            );
        }
        let (fw, fh) = (f.width.max(1) as f32, f.height.max(1) as f32);
        let s = (r.width() / fw).min(r.height() / fh).min(4.0);
        let ir = Rect::from_center_size(r.center(), Vec2::new(fw, fh) * s);
        if let Some(t) = t {
            p.image(
                t,
                ir,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        ui.label(format!("Preview: frame {k}"));
    }

    fn owner(&self) -> u64 {
        owner_key("sprite-frames", self.uid as usize)
    }

    fn strip_ui(&mut self, ui: &mut Ui, tex: &mut TextureCache) {
        let owner = self.owner();
        let mut edit = None;
        egui::ScrollArea::vertical()
            .id_salt("frame_strip")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for i in 0..self.frames.len() {
                        let f = &self.frames[i];
                        let t = tex.get(ui.ctx(), owner, i, self.gen, f);
                        let (r, resp) =
                            ui.allocate_exact_size(Vec2::new(84.0, 100.0), Sense::click());
                        resp.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                format!("image {i}"),
                            )
                        });
                        let resp = resp.on_hover_text("Double-click to edit");
                        let p = ui.painter_at(r);
                        let img_r =
                            Rect::from_min_size(r.min + Vec2::new(2.0, 2.0), Vec2::splat(80.0));
                        let (fw, fh) = (f.width.max(1) as f32, f.height.max(1) as f32);
                        let s = (80.0 / fw).min(80.0 / fh).min(4.0);
                        let ir = Rect::from_center_size(img_r.center(), Vec2::new(fw, fh) * s);
                        crate::widgets::checker(&p, ir, 6.0);
                        if let Some(t) = t {
                            p.image(
                                t,
                                ir,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        }
                        if i == self.sel {
                            p.rect_stroke(
                                img_r,
                                2.0,
                                Stroke::new(2.0, ui.visuals().selection.stroke.color),
                                egui::StrokeKind::Inside,
                            );
                        }
                        p.text(
                            Pos2::new(r.center().x, r.max.y - 9.0),
                            egui::Align2::CENTER_CENTER,
                            format!("image {i}"),
                            egui::FontId::proportional(11.0),
                            ui.visuals().text_color(),
                        );
                        if resp.clicked() {
                            self.sel = i;
                        }
                        if resp.double_clicked() {
                            edit = Some(i);
                        }
                    }
                });
            });
        if let Some(i) = edit {
            self.edit_frame(i);
        }
    }
}

fn pick_files(key: u64) {
    crate::dialogs::request_open(key, crate::dialogs::IMAGES, true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editors::image_editor::raster::Px;

    fn solid(w: u32, h: u32, c: Rgba) -> RgbaImage {
        let mut i = RgbaImage::new(w, h);
        for p in i.data.as_chunks_mut::<4>().0 {
            p.copy_from_slice(&c);
        }
        i
    }

    #[test]
    fn strip_extract() {
        // 2 rows of 3 cells of 4x4 with 1 px separation and a 2 px left margin
        let mut src = RgbaImage::new(2 + 3 * 5, 2 * 5);
        for row in 0..2 {
            for col in 0..3 {
                let v = (row * 3 + col) as u8 * 10 + 10;
                for y in 0..4 {
                    for x in 0..4 {
                        src.put(2 + col * 5 + x, row * 5 + y, [v, 0, 0, 255]);
                    }
                }
            }
        }
        let p = StripParams {
            count: 4,
            per_row: 3,
            width: 4,
            height: 4,
            cell_x: 0,
            cell_y: 0,
            px_x: 2,
            px_y: 0,
            sep_x: 1,
            sep_y: 1,
        };
        let f = p.extract(&src);
        assert_eq!(f.len(), 4);
        let firsts: Vec<u8> = f.iter().map(|i| i.pixel(0, 0)[0]).collect();
        assert_eq!(firsts, vec![10, 20, 30, 40]);
        assert!(f.iter().all(|i| i.pixel(3, 3)[3] == 255));
        // skipping one cell horizontally
        let f = StripParams {
            cell_x: 1,
            count: 2,
            per_row: 2,
            ..p
        }
        .extract(&src);
        assert_eq!(f[0].pixel(0, 0)[0], 20);
        assert_eq!(f[1].pixel(0, 0)[0], 30);
        // defaults for a horizontal strip of squares
        let d = StripParams::for_image(&RgbaImage::new(32, 8));
        assert_eq!((d.count, d.width, d.height), (4, 8, 8));
    }

    #[test]
    fn frame_operations_and_undo() {
        let r = solid(4, 4, [255, 0, 0, 255]);
        let g = solid(4, 4, [0, 255, 0, 255]);
        let mut ed = SpriteFramesEditor::new(vec![r.clone()], "spr", 1);
        ed.add_empty();
        assert_eq!(ed.frames.len(), 2);
        assert_eq!(ed.sel, 1);
        ed.sel = 0;
        ed.insert_empty();
        assert_eq!(ed.frames[0], RgbaImage::new(4, 4));
        assert_eq!(ed.frames[1], r);
        ed.sel = 1;
        ed.duplicate();
        assert_eq!(ed.frames.len(), 4);
        assert_eq!(ed.frames[2], r);
        ed.move_left();
        assert_eq!(ed.sel, 1);
        ed.delete();
        assert_eq!(ed.frames.len(), 3);
        ed.undo();
        assert_eq!(ed.frames.len(), 4);
        ed.redo();
        assert_eq!(ed.frames.len(), 3);
        // differing sizes are adapted
        ed.insert_frames(vec![solid(2, 2, [0, 255, 0, 255])], 0, Placement::Stretch);
        assert_eq!(ed.frames[0], g);
        ed.insert_frames(vec![solid(2, 2, [0, 255, 0, 255])], 0, Placement::Center);
        assert_eq!(ed.frames[0].pixel(0, 0)[3], 0);
        assert_eq!(ed.frames[0].pixel(1, 1), [0, 255, 0, 255]);
        // whole-sprite ops
        ed.apply_op(
            &ImageOp::Stretch {
                w: 8,
                h: 2,
                smooth: false,
            },
            false,
        );
        assert!(ed.frames.iter().all(|f| (f.width, f.height) == (8, 2)));
        let n = ed.frames.len();
        ed.apply_anim(&AnimOp::AddReverse);
        assert_eq!(ed.frames.len(), 2 * n - 2);
        assert!(ed.modified);
    }

    #[test]
    fn image_editor_round_trip() {
        let r = solid(4, 4, [255, 0, 0, 255]);
        let mut ed = SpriteFramesEditor::new(vec![r.clone(), r.clone()], "spr", 1);
        ed.edit_frame(1);
        let (_, ie) = ed.image_editor.as_mut().unwrap();
        assert!(ie.onion.is_some());
        ie.apply_op(&ImageOp::Invert);
        ed.commit_editor();
        assert_eq!(ed.frames[1].pixel(0, 0), [0, 255, 255, 255]);
        assert_eq!(ed.frames[0], r);
    }
}
