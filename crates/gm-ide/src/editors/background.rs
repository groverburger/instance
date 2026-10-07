//! Background form (specs/ide.md §9).

use eframe::egui::{self, Color32, Sense, Stroke, Ui, Vec2};
use gm_format::{Background, Project, ResourceKind};

use super::image_editor::{EditorAction, ImageEditor};
use super::{commit_slot, name_field, slot_dirty, Cx, Editor, Request, ResKey};
use crate::imageio::{self, LoadOptions};
use crate::widgets::{image_preview, owner_key};

pub struct BackgroundEditor {
    idx: usize,
    work: Background,
    uid: u64,
    gen: u64,
    opts: LoadOptions,
    /// The image editor (Edit Background), when open.
    image_editor: Option<ImageEditor>,
}

impl BackgroundEditor {
    pub fn new(idx: usize, work: Background, uid: u64) -> Self {
        BackgroundEditor {
            idx,
            work,
            uid,
            gen: 1,
            opts: LoadOptions::BACKGROUND,
            image_editor: None,
        }
    }
}

impl Editor for BackgroundEditor {
    fn key(&self) -> ResKey {
        ResKey::Res(ResourceKind::Background, self.idx)
    }
    fn title(&self) -> String {
        format!("Background Properties: {}", self.work.name)
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(680.0, 480.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        if let Some(ed) = &mut self.image_editor {
            match ed.show(&cx.ctx, egui::Id::new(("bg-image-editor", self.uid))) {
                EditorAction::Close { save } => {
                    if save && ed.image != self.work.image {
                        self.work.image = std::mem::take(&mut ed.image);
                        self.gen += 1;
                    }
                    self.image_editor = None;
                }
                EditorAction::None | EditorAction::Prev | EditorAction::Next => {}
            }
        }
        if self.image_editor.is_some() {
            ui.disable();
        }
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(230.0);
                let b = &mut self.work;
                name_field(ui, &mut b.name);
                ui.horizontal(|ui| {
                    if ui.button("Load Background…").clicked() {
                        if let Some(f) = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "bmp", "gif", "jpg", "jpeg"])
                            .pick_file()
                        {
                            match imageio::load_frames(&[f], self.opts) {
                                Ok(mut fr) => {
                                    b.image = fr.swap_remove(0);
                                    self.gen += 1;
                                }
                                Err(e) => cx.requests.push(Request::Message(e)),
                            }
                        }
                    }
                    if ui.button("Edit Background…").clicked() {
                        let img = if b.image.width == 0 || b.image.height == 0 {
                            gm_format::RgbaImage::new(64, 64)
                        } else {
                            b.image.clone()
                        };
                        self.image_editor = Some(ImageEditor::new(img, b.name.clone()));
                    }
                    if ui.button("Save…").clicked() {
                        if let Some(f) = rfd::FileDialog::new()
                            .add_filter("PNG", &["png"])
                            .set_file_name(format!("{}.png", b.name))
                            .save_file()
                        {
                            if let Err(e) =
                                imageio::save_strip_png(std::slice::from_ref(&b.image), &f)
                            {
                                cx.requests.push(Request::Message(e));
                            }
                        }
                    }
                });
                ui.checkbox(&mut self.opts.remove_background, "Remove background");
                ui.checkbox(&mut self.opts.smooth_edges, "Smooth edges");
                ui.checkbox(&mut self.opts.make_opaque, "Make opaque");
                ui.label(format!(
                    "Width: {}   Height: {}",
                    b.image.width, b.image.height
                ));
                ui.separator();
                ui.checkbox(&mut b.use_as_tileset, "Use as tile set");
                ui.add_enabled_ui(b.use_as_tileset, |ui| {
                    egui::Grid::new("tileset").num_columns(2).show(ui, |ui| {
                        for (l, v) in [
                            ("Tile width", &mut b.tile_width),
                            ("Tile height", &mut b.tile_height),
                            ("Horizontal offset", &mut b.tile_h_offset),
                            ("Vertical offset", &mut b.tile_v_offset),
                            ("Horizontal sep", &mut b.tile_h_sep),
                            ("Vertical sep", &mut b.tile_v_sep),
                        ] {
                            ui.label(l);
                            ui.add(egui::DragValue::new(v).range(0..=4096));
                            ui.end_row();
                        }
                    });
                });
            });
            ui.separator();
            let b = &self.work;
            let tex = cx.tex.get(
                &cx.ctx,
                owner_key("bg-editor", self.uid as usize),
                0,
                self.gen,
                &b.image,
            );
            egui::ScrollArea::both().id_salt("bg_prev").show(ui, |ui| {
                let (_, rect, scale) = image_preview(
                    ui,
                    tex,
                    b.image.width,
                    b.image.height,
                    Vec2::new(b.image.width as f32, b.image.height as f32),
                    1.0,
                    Sense::hover(),
                );
                if b.use_as_tileset && b.tile_width > 0 && b.tile_height > 0 {
                    let p = ui.painter_at(rect);
                    let st = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 160));
                    let mut y = b.tile_v_offset;
                    while y < b.image.height as i32 {
                        let mut x = b.tile_h_offset;
                        while x < b.image.width as i32 {
                            let r = egui::Rect::from_min_size(
                                rect.min + Vec2::new(x as f32, y as f32) * scale,
                                Vec2::new(b.tile_width as f32, b.tile_height as f32) * scale,
                            );
                            p.rect_stroke(r, 0.0, st, egui::StrokeKind::Inside);
                            x += b.tile_width + b.tile_h_sep;
                        }
                        y += b.tile_height + b.tile_v_sep;
                    }
                }
            });
        });
    }
    fn on_renamed(&mut self, name: &str) {
        self.work.name = name.to_string();
    }
    fn is_dirty(&self, p: &Project) -> bool {
        slot_dirty(p.backgrounds.get(self.idx), &self.work)
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        commit_slot(p.backgrounds.get_mut(self.idx), &self.work)
    }
}
