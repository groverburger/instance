//! Parameterised whole-image and animation operations with their menus and dialogs,
//! shared by the image editor and the sprite editor (specs/ide.md §6, §7).

use eframe::egui::{self, Ui};
use gm_format::RgbaImage;

use super::effects as fx;
use super::raster::Rgba;

/// A Transform or Image menu operation with its parameters.
#[derive(Debug, Clone, PartialEq)]
pub enum ImageOp {
    Shift {
        dx: i32,
        dy: i32,
    },
    Mirror,
    Flip,
    Rotate {
        deg: f32,
        smooth: bool,
    },
    Scale {
        sx: f32,
        sy: f32,
        smooth: bool,
    },
    Skew {
        h: f32,
        v: f32,
    },
    ResizeCanvas {
        w: u32,
        h: u32,
        anchor: (u8, u8),
    },
    Stretch {
        w: u32,
        h: u32,
        smooth: bool,
    },
    Crop {
        border: i32,
    },
    BlackWhite,
    Colorize {
        hue: f32,
        sat: f32,
    },
    ShiftHue {
        deg: f32,
    },
    ColorizePartial {
        src: f32,
        tol: f32,
        dst: f32,
    },
    Intensity {
        sat: f32,
        val: f32,
    },
    Invert,
    MakeOpaque,
    EraseColor {
        color: Rgba,
        tol: u8,
    },
    SmoothEdges,
    Opacity {
        amount: f32,
        relative: bool,
    },
    Fade {
        color: Rgba,
        amount: u8,
    },
    Blur {
        radius: u32,
        color: bool,
        alpha: bool,
    },
    Sharpen {
        amount: f32,
        color: bool,
        alpha: bool,
    },
    Outline {
        color: Rgba,
        thickness: u32,
    },
}

impl ImageOp {
    pub fn name(&self) -> &'static str {
        match self {
            ImageOp::Shift { .. } => "Shift",
            ImageOp::Mirror => "Mirror Horizontal",
            ImageOp::Flip => "Flip Vertical",
            ImageOp::Rotate { .. } => "Rotate",
            ImageOp::Scale { .. } => "Scale",
            ImageOp::Skew { .. } => "Skew",
            ImageOp::ResizeCanvas { .. } => "Resize Canvas",
            ImageOp::Stretch { .. } => "Stretch",
            ImageOp::Crop { .. } => "Crop",
            ImageOp::BlackWhite => "Black and White",
            ImageOp::Colorize { .. } => "Colorize",
            ImageOp::ShiftHue { .. } => "Shift Hue",
            ImageOp::ColorizePartial { .. } => "Colorize Partial",
            ImageOp::Intensity { .. } => "Intensity",
            ImageOp::Invert => "Invert",
            ImageOp::MakeOpaque => "Make Opaque",
            ImageOp::EraseColor { .. } => "Erase a Color",
            ImageOp::SmoothEdges => "Smooth Edges",
            ImageOp::Opacity { .. } => "Opacity",
            ImageOp::Fade { .. } => "Fade",
            ImageOp::Blur { .. } => "Blur",
            ImageOp::Sharpen { .. } => "Sharpen",
            ImageOp::Outline { .. } => "Outline",
        }
    }

    /// Whether the operation has parameters (opens a dialog).
    pub fn has_dialog(&self) -> bool {
        !matches!(
            self,
            ImageOp::Mirror
                | ImageOp::Flip
                | ImageOp::BlackWhite
                | ImageOp::Invert
                | ImageOp::MakeOpaque
                | ImageOp::SmoothEdges
        )
    }

    /// Whether applying to an image of size `w`×`h` changes its size (in a sprite all
    /// frames must then be changed together).
    pub fn changes_size(&self, w: u32, h: u32) -> bool {
        match self {
            ImageOp::ResizeCanvas { .. } | ImageOp::Stretch { .. } | ImageOp::Crop { .. } => true,
            ImageOp::Rotate { deg, .. } => {
                let d = deg.rem_euclid(360.0);
                w != h && (d == 90.0 || d == 270.0)
            }
            _ => false,
        }
    }

    pub fn apply(&self, img: &RgbaImage) -> RgbaImage {
        match *self {
            ImageOp::Shift { dx, dy } => fx::shift(img, dx, dy),
            ImageOp::Mirror => fx::mirror(img),
            ImageOp::Flip => fx::flip(img),
            ImageOp::Rotate { deg, smooth } => {
                let d = deg.rem_euclid(360.0);
                if d == 90.0 {
                    fx::rotate90_ccw(img)
                } else if d == 270.0 {
                    fx::rotate90_cw(img)
                } else {
                    fx::rotate(img, deg, smooth)
                }
            }
            ImageOp::Scale { sx, sy, smooth } => fx::scale(img, sx / 100.0, sy / 100.0, smooth),
            ImageOp::Skew { h, v } => fx::skew(img, h, v, true),
            ImageOp::ResizeCanvas { w, h, anchor } => fx::resize_canvas(img, w, h, anchor),
            ImageOp::Stretch { w, h, smooth } => fx::stretch(img, w, h, smooth),
            ImageOp::Crop { border } => fx::crop(img, border),
            ImageOp::BlackWhite => fx::black_and_white(img),
            ImageOp::Colorize { hue, sat } => fx::colorize(img, hue, sat / 100.0),
            ImageOp::ShiftHue { deg } => fx::shift_hue(img, deg),
            ImageOp::ColorizePartial { src, tol, dst } => fx::colorize_partial(img, src, tol, dst),
            ImageOp::Intensity { sat, val } => fx::intensity(img, sat, val),
            ImageOp::Invert => fx::invert(img),
            ImageOp::MakeOpaque => fx::make_opaque(img),
            ImageOp::EraseColor { color, tol } => fx::erase_color(img, color, tol),
            ImageOp::SmoothEdges => fx::smooth_edges(img),
            ImageOp::Opacity { amount, relative } => fx::opacity(img, amount, relative),
            ImageOp::Fade { color, amount } => fx::fade(img, color, amount),
            ImageOp::Blur {
                radius,
                color,
                alpha,
            } => fx::blur(img, radius, color, alpha),
            ImageOp::Sharpen {
                amount,
                color,
                alpha,
            } => fx::sharpen(img, amount, color, alpha),
            ImageOp::Outline { color, thickness } => fx::outline(img, color, thickness),
        }
    }

    /// Parameter widgets.
    pub fn ui(&mut self, ui: &mut Ui) {
        let drag = |ui: &mut Ui, label: &str, v: &mut f32, r: std::ops::RangeInclusive<f32>| {
            ui.label(label);
            ui.add(egui::DragValue::new(v).range(r).speed(1.0));
            ui.end_row();
        };
        egui::Grid::new("op_params")
            .num_columns(2)
            .show(ui, |ui| match self {
                ImageOp::Shift { dx, dy } => {
                    ui.label("Horizontal");
                    ui.add(egui::DragValue::new(dx));
                    ui.end_row();
                    ui.label("Vertical");
                    ui.add(egui::DragValue::new(dy));
                    ui.end_row();
                }
                ImageOp::Rotate { deg, smooth } => {
                    ui.label("Angle");
                    ui.horizontal(|ui| {
                        for a in [90.0, 180.0, 270.0] {
                            if ui.selectable_label(*deg == a, format!("{a}°")).clicked() {
                                *deg = a;
                            }
                        }
                        ui.add(egui::DragValue::new(deg).range(-360.0..=360.0).suffix("°"));
                    });
                    ui.end_row();
                    ui.label("");
                    ui.checkbox(smooth, "Smooth (bilinear)");
                    ui.end_row();
                }
                ImageOp::Scale { sx, sy, smooth } => {
                    drag(ui, "Horizontal %", sx, 1.0..=1000.0);
                    drag(ui, "Vertical %", sy, 1.0..=1000.0);
                    ui.label("");
                    ui.checkbox(smooth, "Smooth");
                    ui.end_row();
                }
                ImageOp::Skew { h, v } => {
                    drag(ui, "Horizontal (px)", h, -1000.0..=1000.0);
                    drag(ui, "Vertical (px)", v, -1000.0..=1000.0);
                }
                ImageOp::ResizeCanvas { w, h, anchor } => {
                    ui.label("Width");
                    ui.add(egui::DragValue::new(w).range(1..=8192));
                    ui.end_row();
                    ui.label("Height");
                    ui.add(egui::DragValue::new(h).range(1..=8192));
                    ui.end_row();
                    ui.label("Position");
                    egui::Grid::new("anchor").show(ui, |ui| {
                        for ay in 0..3u8 {
                            for ax in 0..3u8 {
                                let sel = *anchor == (ax, ay);
                                if ui
                                    .selectable_label(sel, if sel { "■" } else { "□" })
                                    .clicked()
                                {
                                    *anchor = (ax, ay);
                                }
                            }
                            ui.end_row();
                        }
                    });
                    ui.end_row();
                }
                ImageOp::Stretch { w, h, smooth } => {
                    ui.label("Width");
                    ui.add(egui::DragValue::new(w).range(1..=8192));
                    ui.end_row();
                    ui.label("Height");
                    ui.add(egui::DragValue::new(h).range(1..=8192));
                    ui.end_row();
                    ui.label("Quality");
                    ui.horizontal(|ui| {
                        ui.radio_value(smooth, false, "Poor (nearest)");
                        ui.radio_value(smooth, true, "Good (bilinear)");
                    });
                    ui.end_row();
                }
                ImageOp::Crop { border } => {
                    ui.label("Border (px)");
                    ui.add(egui::DragValue::new(border).range(-1000..=1000));
                    ui.end_row();
                }
                ImageOp::Colorize { hue, sat } => {
                    ui.label("Hue");
                    ui.add(egui::Slider::new(hue, 0.0..=360.0));
                    ui.end_row();
                    ui.label("Saturation %");
                    ui.add(egui::Slider::new(sat, 0.0..=100.0));
                    ui.end_row();
                }
                ImageOp::ShiftHue { deg } => {
                    ui.label("Shift");
                    ui.add(egui::Slider::new(deg, -180.0..=180.0));
                    ui.end_row();
                }
                ImageOp::ColorizePartial { src, tol, dst } => {
                    ui.label("Source hue");
                    ui.add(egui::Slider::new(src, 0.0..=360.0));
                    ui.end_row();
                    ui.label("Tolerance");
                    ui.add(egui::Slider::new(tol, 0.0..=180.0));
                    ui.end_row();
                    ui.label("New hue");
                    ui.add(egui::Slider::new(dst, 0.0..=360.0));
                    ui.end_row();
                }
                ImageOp::Intensity { sat, val } => {
                    ui.label("Saturation");
                    ui.add(egui::Slider::new(sat, -100.0..=100.0));
                    ui.end_row();
                    ui.label("Value");
                    ui.add(egui::Slider::new(val, -100.0..=100.0));
                    ui.end_row();
                }
                ImageOp::EraseColor { color, tol } => {
                    ui.label("Color");
                    ui.color_edit_button_srgba_unmultiplied(color);
                    ui.end_row();
                    ui.label("Tolerance");
                    ui.add(egui::Slider::new(tol, 0..=255));
                    ui.end_row();
                }
                ImageOp::Opacity { amount, relative } => {
                    ui.label("Mode");
                    ui.horizontal(|ui| {
                        ui.radio_value(relative, true, "Relative %");
                        ui.radio_value(relative, false, "Absolute (0-255)");
                    });
                    ui.end_row();
                    let max = if *relative { 200.0 } else { 255.0 };
                    ui.label("Amount");
                    ui.add(egui::Slider::new(amount, 0.0..=max));
                    ui.end_row();
                }
                ImageOp::Fade { color, amount } => {
                    ui.label("Color");
                    ui.color_edit_button_srgba_unmultiplied(color);
                    ui.end_row();
                    ui.label("Amount");
                    ui.add(egui::Slider::new(amount, 0..=255));
                    ui.end_row();
                }
                ImageOp::Blur {
                    radius,
                    color,
                    alpha,
                } => {
                    ui.label("Amount");
                    ui.horizontal(|ui| {
                        ui.radio_value(radius, 1, "Small");
                        ui.radio_value(radius, 2, "Medium");
                        ui.radio_value(radius, 4, "Large");
                    });
                    ui.end_row();
                    ui.label("");
                    ui.horizontal(|ui| {
                        ui.checkbox(color, "Color");
                        ui.checkbox(alpha, "Transparency");
                    });
                    ui.end_row();
                }
                ImageOp::Sharpen {
                    amount,
                    color,
                    alpha,
                } => {
                    ui.label("Amount");
                    ui.horizontal(|ui| {
                        ui.radio_value(amount, 0.5, "Subtle");
                        ui.radio_value(amount, 1.5, "Strong");
                        ui.radio_value(amount, 3.0, "Special");
                    });
                    ui.end_row();
                    ui.label("");
                    ui.horizontal(|ui| {
                        ui.checkbox(color, "Color");
                        ui.checkbox(alpha, "Transparency");
                    });
                    ui.end_row();
                }
                ImageOp::Outline { color, thickness } => {
                    ui.label("Color");
                    ui.color_edit_button_srgba_unmultiplied(color);
                    ui.end_row();
                    ui.label("Thickness");
                    ui.add(egui::DragValue::new(thickness).range(1..=32));
                    ui.end_row();
                }
                ImageOp::Mirror
                | ImageOp::Flip
                | ImageOp::BlackWhite
                | ImageOp::Invert
                | ImageOp::MakeOpaque
                | ImageOp::SmoothEdges => {}
            });
    }
}

/// Apply `op` to the frames of a sprite: to frame `sel` only, or to all frames when
/// `all` is set or the operation changes the image size. Crop uses the common
/// bounding box of all frames.
pub fn apply_to_frames(op: &ImageOp, frames: &mut [RgbaImage], sel: usize, all: bool) {
    let Some(first) = frames.first() else {
        return;
    };
    let all = all || op.changes_size(first.width, first.height);
    if let ImageOp::Crop { border } = *op {
        if let Some(r) = fx::crop_box(frames, border) {
            for f in frames.iter_mut() {
                *f = super::raster::copy_rect(f, r);
            }
        }
        return;
    }
    for (i, f) in frames.iter_mut().enumerate() {
        if all || i == sel {
            *f = op.apply(f);
        }
    }
}

/// Transform menu entries (default parameters for an image of size `w`×`h`).
pub fn transform_menu(ui: &mut Ui, w: u32, h: u32, allow_resize: bool) -> Option<ImageOp> {
    let mut r = None;
    let mut item = |ui: &mut Ui, enabled: bool, op: ImageOp| {
        let label = if op.has_dialog() {
            format!("{}…", op.name())
        } else {
            op.name().to_string()
        };
        if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
            r = Some(op);
        }
    };
    item(ui, true, ImageOp::Shift { dx: 0, dy: 0 });
    item(ui, true, ImageOp::Mirror);
    item(ui, true, ImageOp::Flip);
    item(
        ui,
        true,
        ImageOp::Rotate {
            deg: 90.0,
            smooth: true,
        },
    );
    item(
        ui,
        true,
        ImageOp::Scale {
            sx: 100.0,
            sy: 100.0,
            smooth: true,
        },
    );
    item(ui, true, ImageOp::Skew { h: 0.0, v: 0.0 });
    ui.separator();
    item(
        ui,
        allow_resize,
        ImageOp::ResizeCanvas {
            w,
            h,
            anchor: (1, 1),
        },
    );
    item(ui, allow_resize, ImageOp::Stretch { w, h, smooth: true });
    item(ui, allow_resize, ImageOp::Crop { border: 0 });
    r
}

/// Image menu entries.
pub fn image_menu(ui: &mut Ui, left: Rgba) -> Option<ImageOp> {
    let mut r = None;
    let mut item = |ui: &mut Ui, op: ImageOp| {
        let label = if op.has_dialog() {
            format!("{}…", op.name())
        } else {
            op.name().to_string()
        };
        if ui.button(label).clicked() {
            r = Some(op);
        }
    };
    item(ui, ImageOp::BlackWhite);
    item(
        ui,
        ImageOp::Colorize {
            hue: 0.0,
            sat: 75.0,
        },
    );
    item(ui, ImageOp::ShiftHue { deg: 30.0 });
    item(
        ui,
        ImageOp::ColorizePartial {
            src: 0.0,
            tol: 30.0,
            dst: 120.0,
        },
    );
    item(ui, ImageOp::Intensity { sat: 0.0, val: 0.0 });
    item(ui, ImageOp::Invert);
    item(ui, ImageOp::MakeOpaque);
    item(
        ui,
        ImageOp::EraseColor {
            color: left,
            tol: 0,
        },
    );
    item(ui, ImageOp::SmoothEdges);
    item(
        ui,
        ImageOp::Opacity {
            amount: 50.0,
            relative: true,
        },
    );
    item(
        ui,
        ImageOp::Fade {
            color: [255, 255, 255, 255],
            amount: 128,
        },
    );
    item(
        ui,
        ImageOp::Blur {
            radius: 1,
            color: true,
            alpha: true,
        },
    );
    item(
        ui,
        ImageOp::Sharpen {
            amount: 0.5,
            color: true,
            alpha: false,
        },
    );
    item(
        ui,
        ImageOp::Outline {
            color: [0, 0, 0, 255],
            thickness: 1,
        },
    );
    r
}

/// An open parameter dialog.
pub struct OpDialog {
    pub op: ImageOp,
    /// Apply to all frames (sprite editor only).
    pub all_frames: bool,
}

/// Outcome of a dialog frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogResult {
    Open,
    Ok,
    Cancel,
}

impl OpDialog {
    pub fn new(op: ImageOp) -> Self {
        OpDialog {
            op,
            all_frames: true,
        }
    }

    /// Show as a modal; `scope` shows the current/all frames choice.
    pub fn show(&mut self, ctx: &egui::Context, id: egui::Id, scope: bool) -> DialogResult {
        let mut res = DialogResult::Open;
        let title = self.op.name().to_string();
        let m = crate::mdi::dialog(ctx, id, &title, |ui| {
            ui.set_min_width(280.0);
            self.op.ui(ui);
            if scope {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.all_frames, false, "Current frame");
                    ui.radio_value(&mut self.all_frames, true, "All frames");
                });
            }
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
        if res == DialogResult::Open && m.should_close() {
            res = DialogResult::Cancel;
        }
        res
    }
}

// ------------------------------------------------------------------ animation ops

/// An Animation menu operation with its parameters.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimOp {
    SetLength { n: usize },
    Stretch { n: usize },
    Reverse,
    AddReverse,
    Translation { n: usize, dx: i32, dy: i32 },
    Rotation { n: usize, angle: f32, cw: bool },
    Colorize { n: usize, hue: f32 },
    FadeTo { n: usize, color: Rgba },
    Disappear { n: usize },
    Shrink { n: usize },
    Grow { n: usize },
    Flatten { n: usize, horizontal: bool },
    Raise { n: usize, horizontal: bool },
    Morph { n: usize, target: Option<RgbaImage> },
}

impl AnimOp {
    pub fn name(&self) -> &'static str {
        match self {
            AnimOp::SetLength { .. } => "Set Length",
            AnimOp::Stretch { .. } => "Stretch",
            AnimOp::Reverse => "Reverse",
            AnimOp::AddReverse => "Add Reverse",
            AnimOp::Translation { .. } => "Translation Sequence",
            AnimOp::Rotation { .. } => "Rotation Sequence",
            AnimOp::Colorize { .. } => "Colorize",
            AnimOp::FadeTo { .. } => "Fade to Color",
            AnimOp::Disappear { .. } => "Disappear",
            AnimOp::Shrink { .. } => "Shrink",
            AnimOp::Grow { .. } => "Grow",
            AnimOp::Flatten { .. } => "Flatten",
            AnimOp::Raise { .. } => "Raise",
            AnimOp::Morph { .. } => "Morph",
        }
    }

    pub fn has_dialog(&self) -> bool {
        !matches!(self, AnimOp::Reverse | AnimOp::AddReverse)
    }

    pub fn apply(&self, f: &[RgbaImage]) -> Vec<RgbaImage> {
        match self {
            AnimOp::SetLength { n } => fx::set_length(f, *n),
            AnimOp::Stretch { n } => fx::stretch_length(f, *n),
            AnimOp::Reverse => fx::reverse(f),
            AnimOp::AddReverse => fx::add_reverse(f),
            AnimOp::Translation { n, dx, dy } => fx::translation_sequence(f, *n, *dx, *dy),
            AnimOp::Rotation { n, angle, cw } => fx::rotation_sequence(f, *n, *angle, *cw),
            AnimOp::Colorize { n, hue } => fx::colorize_sequence(f, *n, *hue),
            AnimOp::FadeTo { n, color } => fx::fade_sequence(f, *n, *color),
            AnimOp::Disappear { n } => fx::disappear_sequence(f, *n),
            AnimOp::Shrink { n } => fx::shrink_sequence(f, *n, false),
            AnimOp::Grow { n } => fx::shrink_sequence(f, *n, true),
            AnimOp::Flatten { n, horizontal } => fx::flatten_sequence(f, *n, *horizontal, false),
            AnimOp::Raise { n, horizontal } => fx::flatten_sequence(f, *n, *horizontal, true),
            AnimOp::Morph { n, target } => match target {
                Some(t) => fx::morph_sequence(f, *n, t),
                None => f.to_vec(),
            },
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        egui::Grid::new("anim_params")
            .num_columns(2)
            .show(ui, |ui| {
                let frames = |ui: &mut Ui, n: &mut usize| {
                    ui.label("Number of frames");
                    ui.add(egui::DragValue::new(n).range(1..=1000));
                    ui.end_row();
                };
                match self {
                    AnimOp::SetLength { n }
                    | AnimOp::Stretch { n }
                    | AnimOp::Disappear { n }
                    | AnimOp::Shrink { n }
                    | AnimOp::Grow { n } => frames(ui, n),
                    AnimOp::Translation { n, dx, dy } => {
                        frames(ui, n);
                        ui.label("Horizontal");
                        ui.add(egui::DragValue::new(dx));
                        ui.end_row();
                        ui.label("Vertical");
                        ui.add(egui::DragValue::new(dy));
                        ui.end_row();
                    }
                    AnimOp::Rotation { n, angle, cw } => {
                        frames(ui, n);
                        ui.label("Degrees");
                        ui.add(egui::DragValue::new(angle).range(0.0..=3600.0));
                        ui.end_row();
                        ui.label("");
                        ui.horizontal(|ui| {
                            ui.radio_value(cw, true, "Clockwise");
                            ui.radio_value(cw, false, "Counter-clockwise");
                        });
                        ui.end_row();
                    }
                    AnimOp::Colorize { n, hue } => {
                        frames(ui, n);
                        ui.label("Hue");
                        ui.add(egui::Slider::new(hue, 0.0..=360.0));
                        ui.end_row();
                    }
                    AnimOp::FadeTo { n, color } => {
                        frames(ui, n);
                        ui.label("Color");
                        ui.color_edit_button_srgba_unmultiplied(color);
                        ui.end_row();
                    }
                    AnimOp::Flatten { n, horizontal } | AnimOp::Raise { n, horizontal } => {
                        frames(ui, n);
                        ui.label("Direction");
                        ui.horizontal(|ui| {
                            ui.radio_value(horizontal, false, "Vertical");
                            ui.radio_value(horizontal, true, "Horizontal");
                        });
                        ui.end_row();
                    }
                    AnimOp::Morph { n, target } => {
                        frames(ui, n);
                        ui.label("Target image");
                        ui.horizontal(|ui| {
                            if ui.button("Load…").clicked() {
                                if let Some(p) = rfd::FileDialog::new()
                                    .add_filter("Images", &["png", "bmp", "gif", "jpg", "jpeg"])
                                    .pick_file()
                                {
                                    if let Ok(mut fr) = crate::imageio::load_frames(
                                        &[p],
                                        crate::imageio::LoadOptions::SPRITE,
                                    ) {
                                        *target = Some(fr.swap_remove(0));
                                    }
                                }
                            }
                            ui.label(match target {
                                Some(t) => format!("{}×{}", t.width, t.height),
                                None => "none".into(),
                            });
                        });
                        ui.end_row();
                    }
                    AnimOp::Reverse | AnimOp::AddReverse => {}
                }
            });
    }
}

/// Animation menu entries; `n` is the current frame count.
pub fn animation_menu(ui: &mut Ui, n: usize) -> Option<AnimOp> {
    let n = n.max(1);
    let k = n.max(8);
    let mut r = None;
    let ops = [
        AnimOp::SetLength { n },
        AnimOp::Stretch { n },
        AnimOp::Reverse,
        AnimOp::AddReverse,
        AnimOp::Translation {
            n: k,
            dx: 16,
            dy: 0,
        },
        AnimOp::Rotation {
            n: k,
            angle: 360.0,
            cw: true,
        },
        AnimOp::Colorize { n: k, hue: 0.0 },
        AnimOp::FadeTo {
            n: k,
            color: [0, 0, 0, 255],
        },
        AnimOp::Disappear { n: k },
        AnimOp::Shrink { n: k },
        AnimOp::Grow { n: k },
        AnimOp::Flatten {
            n: k,
            horizontal: false,
        },
        AnimOp::Raise {
            n: k,
            horizontal: false,
        },
        AnimOp::Morph { n: k, target: None },
    ];
    for (i, op) in ops.into_iter().enumerate() {
        if i == 4 || i == 6 {
            ui.separator();
        }
        let label = if op.has_dialog() {
            format!("{}…", op.name())
        } else {
            op.name().to_string()
        };
        if ui.button(label).clicked() {
            r = Some(op);
        }
    }
    r
}
