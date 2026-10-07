//! Sprite form with mask settings and basic frame operations (specs/ide.md §5, §6).

use eframe::egui::{self, Color32, Sense, Stroke, Ui, Vec2};
use gm_format::{Project, ResourceKind, Sprite};

use super::sprite_frames::{SpriteEditorAction, SpriteFramesEditor};
use super::{commit_slot, name_field, slot_dirty, Cx, Editor, Request, ResKey};
use crate::imageio::{self, LoadOptions};
use crate::widgets::{image_preview, owner_key};

pub struct SpriteEditor {
    idx: usize,
    work: Sprite,
    uid: u64,
    gen: u64,
    frame: usize,
    opts: LoadOptions,
    zoom: f32,
    /// The sprite editor window (Edit Sprite), when open.
    frames_editor: Option<SpriteFramesEditor>,
}

impl SpriteEditor {
    pub fn new(idx: usize, work: Sprite, uid: u64) -> Self {
        SpriteEditor {
            idx,
            work,
            uid,
            gen: 1,
            frame: 0,
            opts: LoadOptions::SPRITE,
            zoom: 0.0,
            frames_editor: None,
        }
    }

    fn frames_changed(&mut self) {
        self.gen += 1;
        self.frame = self.frame.min(self.work.frames.len().saturating_sub(1));
        update_bbox(&mut self.work);
    }
}

/// Recompute the stored bounding box for automatic / full-image modes.
pub fn update_bbox(s: &mut Sprite) {
    let (w, h) = (s.width() as i32, s.height() as i32);
    match s.bbox_mode {
        0 => {
            if let Some((l, t, r, b)) = imageio::auto_bbox(&s.frames, s.alpha_tolerance) {
                s.bbox_left = l;
                s.bbox_top = t;
                s.bbox_right = r;
                s.bbox_bottom = b;
            } else {
                (s.bbox_left, s.bbox_top, s.bbox_right, s.bbox_bottom) = (0, 0, w - 1, h - 1);
            }
        }
        1 => {
            (s.bbox_left, s.bbox_top, s.bbox_right, s.bbox_bottom) = (0, 0, w - 1, h - 1);
        }
        _ => {}
    }
}

impl Editor for SpriteEditor {
    fn key(&self) -> ResKey {
        ResKey::Res(ResourceKind::Sprite, self.idx)
    }
    fn title(&self) -> String {
        format!("Sprite Properties: {}", self.work.name)
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(720.0, 520.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let mut frames_changed = false;
        let mut mask_changed = false;
        if let Some(ed) = &mut self.frames_editor {
            let id = egui::Id::new(("sprite-frames-editor", self.uid));
            match ed.show(&cx.ctx, cx.tex, id) {
                SpriteEditorAction::None => {}
                SpriteEditorAction::Close { save } => {
                    if save && ed.frames != self.work.frames {
                        self.work.frames = std::mem::take(&mut ed.frames);
                        frames_changed = true;
                    }
                    self.frames_editor = None;
                }
            }
        }
        if self.frames_editor.is_some() {
            ui.disable();
        }
        ui.horizontal_top(|ui| {
            // ---------------- left column: properties
            ui.vertical(|ui| {
                ui.set_width(250.0);
                name_field(ui, &mut self.work.name);
                ui.horizontal(|ui| {
                    if ui.button("Load Sprite…").clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "bmp", "gif", "jpg", "jpeg"])
                            .add_filter("GameMaker sprite", &["gmspr"])
                            .pick_files()
                        {
                            let gmspr = files.len() == 1
                                && files[0]
                                    .extension()
                                    .is_some_and(|e| e.eq_ignore_ascii_case("gmspr"));
                            if gmspr {
                                match std::fs::read(&files[0])
                                    .map_err(|e| e.to_string())
                                    .and_then(|b| {
                                        gm_format::read_sprite_export(&b).map_err(|e| e.to_string())
                                    }) {
                                    Ok(mut s) => {
                                        s.name = self.work.name.clone();
                                        s.last_changed = self.work.last_changed;
                                        self.work = s;
                                        self.gen += 1;
                                    }
                                    Err(e) => cx.requests.push(Request::Message(e)),
                                }
                            } else {
                                match imageio::load_frames(&files, self.opts) {
                                    Ok(f) => {
                                        self.work.frames = f;
                                        frames_changed = true;
                                    }
                                    Err(e) => cx.requests.push(Request::Message(e)),
                                }
                            }
                        }
                    }
                    if ui.button("Edit Sprite…").clicked() {
                        self.frames_editor = Some(SpriteFramesEditor::new(
                            self.work.frames.clone(),
                            &self.work.name,
                            self.uid,
                        ));
                    }
                    if ui.button("Save…").clicked() {
                        if let Some(f) = rfd::FileDialog::new()
                            .add_filter("PNG strip", &["png"])
                            .add_filter("GameMaker sprite", &["gmspr"])
                            .set_file_name(format!("{}.png", self.work.name))
                            .save_file()
                        {
                            let r = if f
                                .extension()
                                .is_some_and(|e| e.eq_ignore_ascii_case("gmspr"))
                            {
                                gm_format::write_sprite_export(&self.work)
                                    .map_err(|e| e.to_string())
                                    .and_then(|b| std::fs::write(&f, b).map_err(|e| e.to_string()))
                            } else {
                                imageio::save_strip_png(&self.work.frames, &f)
                            };
                            if let Err(e) = r {
                                cx.requests.push(Request::Message(e));
                            }
                        }
                    }
                });
                ui.checkbox(&mut self.opts.remove_background, "Remove background");
                ui.checkbox(&mut self.opts.smooth_edges, "Smooth edges");
                ui.checkbox(&mut self.opts.make_opaque, "Make opaque");
                ui.label(format!(
                    "Width: {}   Height: {}   Subimages: {}",
                    self.work.width(),
                    self.work.height(),
                    self.work.frames.len()
                ));
                ui.separator();
                ui.label("Origin");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut self.work.origin_x));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.work.origin_y));
                    if ui.button("Center").clicked() {
                        self.work.origin_x = self.work.width() as i32 / 2;
                        self.work.origin_y = self.work.height() as i32 / 2;
                    }
                });
                ui.separator();
                ui.label("Collision checking");
                let mut precise = self.work.collision_shape == 0;
                if ui
                    .checkbox(&mut precise, "Precise collision checking")
                    .changed()
                {
                    self.work.collision_shape = if precise { 0 } else { 1 };
                }
                ui.checkbox(
                    &mut self.work.separate_collision_masks,
                    "Separate collision masks",
                );
                egui::CollapsingHeader::new("Mask properties")
                    .default_open(false)
                    .show(ui, |ui| {
                        let s = &mut self.work;
                        ui.label("Bounding box");
                        ui.horizontal(|ui| {
                            mask_changed |=
                                ui.radio_value(&mut s.bbox_mode, 0, "Automatic").changed();
                            mask_changed |=
                                ui.radio_value(&mut s.bbox_mode, 1, "Full image").changed();
                            mask_changed |= ui.radio_value(&mut s.bbox_mode, 2, "Manual").changed();
                        });
                        ui.add_enabled_ui(s.bbox_mode == 2, |ui| {
                            egui::Grid::new("bbox").num_columns(4).show(ui, |ui| {
                                ui.label("Left");
                                ui.add(egui::DragValue::new(&mut s.bbox_left));
                                ui.label("Right");
                                ui.add(egui::DragValue::new(&mut s.bbox_right));
                                ui.end_row();
                                ui.label("Top");
                                ui.add(egui::DragValue::new(&mut s.bbox_top));
                                ui.label("Bottom");
                                ui.add(egui::DragValue::new(&mut s.bbox_bottom));
                                ui.end_row();
                            });
                        });
                        ui.label("Shape");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut s.collision_shape, 0, "Precise");
                            ui.radio_value(&mut s.collision_shape, 1, "Rectangle");
                            ui.radio_value(&mut s.collision_shape, 2, "Disk");
                            ui.radio_value(&mut s.collision_shape, 3, "Diamond");
                        });
                        mask_changed |= ui
                            .add(
                                egui::Slider::new(&mut s.alpha_tolerance, 0..=255)
                                    .text("Alpha tolerance"),
                            )
                            .changed();
                    });
            });
            ui.separator();
            // ---------------- right column: frames and preview
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let n = self.work.frames.len();
                    ui.label(format!(
                        "Show: {} / {}",
                        self.frame.min(n.saturating_sub(1)),
                        n
                    ));
                    if ui.button("◀").clicked() && n > 0 {
                        self.frame = (self.frame + n - 1) % n;
                    }
                    if ui.button("▶").clicked() && n > 0 {
                        self.frame = (self.frame + 1) % n;
                    }
                    ui.separator();
                    if ui
                        .add_enabled(n > 0, egui::Button::new("Delete frame"))
                        .clicked()
                    {
                        self.work.frames.remove(self.frame.min(n - 1));
                        frames_changed = true;
                    }
                    if ui
                        .add_enabled(self.frame > 0 && n > 1, egui::Button::new("Move left"))
                        .clicked()
                    {
                        self.work.frames.swap(self.frame, self.frame - 1);
                        self.frame -= 1;
                        self.gen += 1;
                    }
                    if ui
                        .add_enabled(self.frame + 1 < n, egui::Button::new("Move right"))
                        .clicked()
                    {
                        self.work.frames.swap(self.frame, self.frame + 1);
                        self.frame += 1;
                        self.gen += 1;
                    }
                    if ui.button("Add from file…").clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "bmp", "gif", "jpg", "jpeg"])
                            .pick_files()
                        {
                            match imageio::load_frames(&files, self.opts) {
                                Ok(f) => {
                                    let (w, h) = if n > 0 {
                                        (self.work.width(), self.work.height())
                                    } else {
                                        (f[0].width, f[0].height)
                                    };
                                    self.work
                                        .frames
                                        .extend(f.iter().map(|x| imageio::fit_canvas(x, w, h)));
                                    frames_changed = true;
                                }
                                Err(e) => cx.requests.push(Request::Message(e)),
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Zoom");
                    ui.add(egui::Slider::new(&mut self.zoom, 0.0..=8.0).text("(0 = fit)"));
                });
                let owner = owner_key("sprite-editor", self.uid as usize);
                let (w, h) = (self.work.width(), self.work.height());
                let tex = self
                    .work
                    .frames
                    .get(self.frame)
                    .and_then(|f| cx.tex.get(&cx.ctx, owner, self.frame, self.gen, f));
                let avail = ui.available_size() - Vec2::new(0.0, 80.0);
                let max_zoom = if self.zoom > 0.0 { self.zoom } else { 8.0 };
                let max = if self.zoom > 0.0 {
                    Vec2::new(w as f32, h as f32) * self.zoom
                } else {
                    avail.max(Vec2::splat(32.0))
                };
                egui::ScrollArea::both()
                    .id_salt("sprite_preview")
                    .max_height(avail.y.max(64.0))
                    .show(ui, |ui| {
                        let (resp, rect, scale) =
                            image_preview(ui, tex, w, h, max, max_zoom, Sense::click());
                        let p = ui.painter_at(rect.expand(1.0));
                        // bounding box
                        let s = &self.work;
                        let bb = egui::Rect::from_min_max(
                            rect.min + Vec2::new(s.bbox_left as f32, s.bbox_top as f32) * scale,
                            rect.min
                                + Vec2::new(s.bbox_right as f32 + 1.0, s.bbox_bottom as f32 + 1.0)
                                    * scale,
                        );
                        p.rect_stroke(
                            bb,
                            0.0,
                            Stroke::new(1.0, Color32::from_rgb(0, 160, 255)),
                            egui::StrokeKind::Inside,
                        );
                        // origin
                        if cx.prefs.show_origin {
                            let o =
                                rect.min + Vec2::new(s.origin_x as f32, s.origin_y as f32) * scale;
                            let st = Stroke::new(1.0, Color32::RED);
                            p.line_segment([o - Vec2::new(6.0, 0.0), o + Vec2::new(6.0, 0.0)], st);
                            p.line_segment([o - Vec2::new(0.0, 6.0), o + Vec2::new(0.0, 6.0)], st);
                        }
                        if resp.clicked() {
                            if let Some(pos) = resp.interact_pointer_pos() {
                                let v = (pos - rect.min) / scale;
                                self.work.origin_x = v.x.floor() as i32;
                                self.work.origin_y = v.y.floor() as i32;
                            }
                        }
                    });
                // frame thumbnails
                egui::ScrollArea::horizontal()
                    .id_salt("sprite_frames")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for i in 0..self.work.frames.len() {
                                let f = &self.work.frames[i];
                                let t = cx.tex.get(&cx.ctx, owner, i, self.gen, f);
                                ui.vertical(|ui| {
                                    let (r, rect, _) = image_preview(
                                        ui,
                                        t,
                                        f.width,
                                        f.height,
                                        Vec2::splat(48.0),
                                        4.0,
                                        Sense::click(),
                                    );
                                    if i == self.frame {
                                        ui.painter().rect_stroke(
                                            rect.expand(2.0),
                                            0.0,
                                            Stroke::new(2.0, ui.visuals().selection.stroke.color),
                                            egui::StrokeKind::Outside,
                                        );
                                    }
                                    if r.clicked() {
                                        self.frame = i;
                                    }
                                    ui.small(format!("image {i}"));
                                });
                            }
                        });
                    });
            });
        });
        if frames_changed {
            self.frames_changed();
        } else if mask_changed {
            update_bbox(&mut self.work);
        }
    }
    fn on_renamed(&mut self, name: &str) {
        self.work.name = name.to_string();
    }
    fn is_dirty(&self, p: &Project) -> bool {
        slot_dirty(p.sprites.get(self.idx), &self.work)
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        commit_slot(p.sprites.get_mut(self.idx), &self.work)
    }
}
