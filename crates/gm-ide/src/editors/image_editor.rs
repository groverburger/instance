//! Built-in image editor (specs/ide.md §7): edits one sprite frame or a background
//! image in its own window with zoom/pan, drawing tools, selections, clipboard,
//! undo/redo and the Transform / Image menus.

pub mod effects;
pub mod history;
pub mod ops;
pub mod raster;
pub mod text;
pub mod tool_icons;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use eframe::egui::{
    self, ecolor::Hsva, Color32, Context, Id, PointerButton, Pos2, Rect, Sense, Stroke,
    TextureHandle, Ui, Vec2,
};
use gm_format::RgbaImage;

use history::History;
use ops::{DialogResult, ImageOp, OpDialog};
use raster::{ColorMode, Mask, Px, Rgba};
use text::{Align, FontSource};
use tool_icons::TIcon;

/// Drawing tools (specs/ide.md §7 tool table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pencil,
    Spray,
    Eraser,
    Picker,
    Line,
    Rect,
    Ellipse,
    Fill,
    ChangeColor,
    Text,
    Select,
}

impl Tool {
    pub const ALL: [Tool; 11] = [
        Tool::Pencil,
        Tool::Spray,
        Tool::Eraser,
        Tool::Picker,
        Tool::Line,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Fill,
        Tool::ChangeColor,
        Tool::Text,
        Tool::Select,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Pencil => "Pencil",
            Tool::Spray => "Spray",
            Tool::Eraser => "Eraser",
            Tool::Picker => "Picker",
            Tool::Line => "Line",
            Tool::Rect => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Fill => "Fill",
            Tool::ChangeColor => "Change Color",
            Tool::Text => "Text",
            Tool::Select => "Select",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Tool::Pencil => "Draw with the left/right colour; Shift = straight, Ctrl = pick colour",
            Tool::Spray => "Spray with the left/right colour",
            Tool::Eraser => "Erase to transparency",
            Tool::Picker => "Click to pick the left/right colour",
            Tool::Line => "Drag to draw a line; Shift = 0/45/90°",
            Tool::Rect => "Drag to draw a rectangle; Shift = square",
            Tool::Ellipse => "Drag to draw an ellipse; Shift = circle",
            Tool::Fill => "Flood fill the connected area",
            Tool::ChangeColor => "Replace all pixels of the clicked colour",
            Tool::Text => "Click to place text, then Apply (Enter)",
            Tool::Select => "Drag to select; drag inside to move (right = copy)",
        }
    }
}

/// Palette icon of a tool.
fn tool_icon(t: Tool) -> TIcon {
    match t {
        Tool::Pencil => TIcon::Pencil,
        Tool::Spray => TIcon::Spray,
        Tool::Eraser => TIcon::Eraser,
        Tool::Picker => TIcon::Picker,
        Tool::Line => TIcon::Line,
        Tool::Rect => TIcon::Rect,
        Tool::Ellipse => TIcon::Ellipse,
        Tool::Fill => TIcon::Fill,
        Tool::ChangeColor => TIcon::ChangeColor,
        Tool::Text => TIcon::Text,
        Tool::Select => TIcon::Select,
    }
}

/// Outline / filled / both for rectangle and ellipse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeFill {
    Outline,
    Filled,
    Both,
}

/// Result of showing the editor for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAction {
    None,
    /// Close; `save` commits the image to the owner.
    Close {
        save: bool,
    },
    /// Commit and move to the previous / next frame (multi-frame sprites).
    Prev,
    Next,
}

struct StrokeState {
    button: usize,
    start: (i32, i32),
    last: (i32, i32),
    mask: Mask,
    /// Fill mask of "both" shapes (drawn with the other colour).
    fill: Mask,
    seed: u64,
}

struct Floating {
    img: RgbaImage,
    x: i32,
    y: i32,
}

#[derive(Clone, Copy)]
enum SelDrag {
    Making { start: (i32, i32) },
    Moving { grab: (i32, i32), orig: (i32, i32) },
}

const PALETTE: [[u8; 3]; 32] = [
    [0, 0, 0],
    [64, 64, 64],
    [128, 128, 128],
    [192, 192, 192],
    [255, 255, 255],
    [128, 0, 0],
    [255, 0, 0],
    [255, 128, 128],
    [128, 64, 0],
    [255, 128, 0],
    [255, 192, 128],
    [128, 128, 0],
    [255, 255, 0],
    [255, 255, 128],
    [0, 128, 0],
    [0, 255, 0],
    [128, 255, 128],
    [0, 128, 128],
    [0, 255, 255],
    [128, 255, 255],
    [0, 0, 128],
    [0, 0, 255],
    [128, 128, 255],
    [64, 0, 128],
    [128, 0, 255],
    [192, 128, 255],
    [128, 0, 128],
    [255, 0, 255],
    [255, 128, 255],
    [96, 48, 24],
    [160, 96, 48],
    [224, 176, 128],
];

/// The image editor window state.
pub struct ImageEditor {
    /// The committed image (without the pending stroke / floating selection).
    pub image: RgbaImage,
    /// Previous frame shown faintly (sprite frames).
    pub onion: Option<RgbaImage>,
    pub show_onion: bool,
    /// (frame index, frame count) when editing a sprite frame.
    pub frame: Option<(usize, usize)>,
    title: String,
    history: History<RgbaImage>,
    /// Whether anything was changed since opening.
    pub modified: bool,

    zoom: f32,
    pan: Vec2,
    fit_pending: bool,
    pub grid: bool,

    pub tool: Tool,
    pub left: Rgba,
    pub right: Rgba,
    hsva: [Hsva; 2],
    active: usize,
    pub mode: ColorMode,
    pub brush: u32,
    pub shape_fill: ShapeFill,
    pub tolerance: u8,
    pub tol_alpha: bool,
    pub eraser_strength: u8,
    pub spray_density: u32,

    pub text: String,
    pub text_size: f32,
    text_align: Align,
    text_aa: bool,
    fonts: Option<Vec<FontSource>>,
    font_src: FontSource,
    font: Option<Arc<fontdue::Font>>,
    text_pos: Option<(i32, i32)>,

    stroke: Option<StrokeState>,
    pub selection: Option<(i32, i32, u32, u32)>,
    floating: Option<Floating>,
    sel_drag: Option<SelDrag>,
    clip: Option<RgbaImage>,

    dialog: Option<OpDialog>,
    new_size: Option<(u32, u32)>,
    confirm_close: bool,

    display_dirty: bool,
    display: RgbaImage,
    tex: Option<TextureHandle>,
    onion_tex: Option<TextureHandle>,
    checker_tex: Option<TextureHandle>,
    hover: Option<(i32, i32)>,
    active_button: Option<usize>,
    status: String,
}

impl ImageEditor {
    pub fn new(image: RgbaImage, title: impl Into<String>) -> Self {
        let left = [0, 0, 0, 255];
        let right = [255, 255, 255, 255];
        ImageEditor {
            display: image.clone(),
            image,
            onion: None,
            show_onion: true,
            frame: None,
            title: title.into(),
            history: History::new(32),
            modified: false,
            zoom: 1.0,
            pan: Vec2::ZERO,
            fit_pending: true,
            grid: true,
            tool: Tool::Pencil,
            left,
            right,
            hsva: [
                Hsva::from_srgba_unmultiplied(left),
                Hsva::from_srgba_unmultiplied(right),
            ],
            active: 0,
            mode: ColorMode::Blend,
            brush: 1,
            shape_fill: ShapeFill::Outline,
            tolerance: 0,
            tol_alpha: true,
            eraser_strength: 255,
            spray_density: 8,
            text: "Text".into(),
            text_size: 16.0,
            text_align: Align::Left,
            text_aa: true,
            fonts: None,
            font_src: FontSource::Builtin,
            font: None,
            text_pos: None,
            stroke: None,
            selection: None,
            floating: None,
            sel_drag: None,
            clip: None,
            dialog: None,
            new_size: None,
            confirm_close: false,
            display_dirty: true,
            tex: None,
            onion_tex: None,
            checker_tex: None,
            hover: None,
            active_button: None,
            status: String::new(),
        }
    }

    /// Editor for frame `index` of `count` with the previous frame as onion skin.
    pub fn for_frame(
        image: RgbaImage,
        onion: Option<RgbaImage>,
        index: usize,
        count: usize,
        title: impl Into<String>,
    ) -> Self {
        let mut e = ImageEditor::new(image, title);
        e.set_frame(image_or_none(onion), index, count);
        e
    }

    /// Switch to another frame (Previous/Next Image), keeping tool settings.
    pub fn load_frame(
        &mut self,
        image: RgbaImage,
        onion: Option<RgbaImage>,
        index: usize,
        count: usize,
    ) {
        self.image = image;
        self.set_frame(image_or_none(onion), index, count);
        self.history = History::new(32);
        self.modified = false;
        self.stroke = None;
        self.floating = None;
        self.selection = None;
        self.text_pos = None;
        self.onion_tex = None;
        self.display_dirty = true;
    }

    fn set_frame(&mut self, onion: Option<RgbaImage>, index: usize, count: usize) {
        self.onion = onion;
        self.frame = Some((index, count));
    }

    fn color(&self, button: usize) -> Rgba {
        if button == 0 {
            self.left
        } else {
            self.right
        }
    }

    fn set_color(&mut self, button: usize, c: Rgba) {
        if button == 0 {
            self.left = c;
        } else {
            self.right = c;
        }
        self.hsva[button] = Hsva::from_srgba_unmultiplied(c);
    }

    // ------------------------------------------------------------ editing core

    /// Record the current image for undo and mark modified.
    fn checkpoint(&mut self) {
        self.history.push(self.image.clone());
        self.modified = true;
        self.display_dirty = true;
    }

    /// Replace the image as one undoable step.
    pub fn set_image(&mut self, img: RgbaImage) {
        self.commit_pending();
        self.checkpoint();
        if (img.width, img.height) != (self.image.width, self.image.height) {
            self.selection = None;
        }
        self.image = img;
    }

    /// Apply a whole-image operation as one undoable step.
    pub fn apply_op(&mut self, op: &ImageOp) {
        if self.frame.is_some_and(|(_, n)| n > 1)
            && op.changes_size(self.image.width, self.image.height)
        {
            self.status =
                "Size-changing operations apply to all frames: use the sprite editor".into();
            return;
        }
        let out = op.apply(&self.image);
        self.set_image(out);
    }

    pub fn undo(&mut self) {
        self.commit_pending();
        if let Some(prev) = self.history.undo(self.image.clone()) {
            self.image = prev;
            self.after_history();
        }
    }

    pub fn redo(&mut self) {
        self.commit_pending();
        if let Some(next) = self.history.redo(self.image.clone()) {
            self.image = next;
            self.after_history();
        }
    }

    fn after_history(&mut self) {
        self.modified = true;
        self.display_dirty = true;
        if let Some((x, y, w, h)) = self.selection {
            if x + w as i32 > self.image.width as i32 || y + h as i32 > self.image.height as i32 {
                self.selection = None;
            }
        }
    }

    /// Finish anything in progress: the text being placed and a floating selection.
    pub fn commit_pending(&mut self) {
        if self.text_pos.is_some() {
            self.commit_text();
        }
        if let Some(f) = self.floating.take() {
            raster::paste(&mut self.image, &f.img, f.x, f.y, ColorMode::Blend);
            self.display_dirty = true;
            self.modified = true;
        }
        self.stroke = None;
        self.sel_drag = None;
    }

    fn text_mask(&mut self) -> Option<Mask> {
        let (x, y) = self.text_pos?;
        if self.font.is_none() {
            match text::load_font(&self.font_src) {
                Ok(f) => self.font = Some(f),
                Err(e) => {
                    self.status = e;
                    return None;
                }
            }
        }
        let font = self.font.clone()?;
        let mut m = Mask::for_image(&self.image);
        text::draw_text(
            &mut m,
            &font,
            &self.text,
            self.text_size,
            x,
            y,
            self.text_align,
            self.text_aa,
        );
        Some(m)
    }

    /// Draw the pending text into the image.
    pub fn commit_text(&mut self) {
        if let Some(m) = self.text_mask() {
            if !m.is_empty() {
                self.checkpoint();
                raster::composite_into(&mut self.image, &m, self.left, self.mode);
            }
        }
        self.text_pos = None;
        self.display_dirty = true;
    }

    /// Place text at (x, y) (the text tool's click).
    pub fn place_text(&mut self, x: i32, y: i32) {
        self.text_pos = Some((x, y));
        self.display_dirty = true;
    }

    /// The image as currently displayed (with pending stroke, text, floating).
    pub fn composed(&mut self) -> RgbaImage {
        let mut d = match &self.stroke {
            Some(st) if self.tool == Tool::Eraser => {
                let mut d = self.image.clone();
                raster::erase_into(&mut d, &st.mask, self.eraser_strength);
                d
            }
            Some(st) => {
                let c = self.color(st.button);
                let other = self.color(1 - st.button);
                let mut d = self.image.clone();
                if !st.fill.is_empty() {
                    raster::composite_into(&mut d, &st.fill, other, self.mode);
                }
                raster::composite_into(&mut d, &st.mask, c, self.mode);
                d
            }
            None => self.image.clone(),
        };
        if self.text_pos.is_some() {
            if let Some(m) = self.text_mask() {
                raster::composite_into(&mut d, &m, self.left, self.mode);
            }
        }
        if let Some(f) = &self.floating {
            raster::paste(&mut d, &f.img, f.x, f.y, ColorMode::Blend);
        }
        d
    }

    /// Begin a stroke at pixel `p` with `button` (0 = left, 1 = right).
    pub fn press(&mut self, p: (i32, i32), button: usize, shift: bool, ctrl: bool) {
        if self.tool != Tool::Text && self.text_pos.is_some() {
            self.commit_text();
        }
        if self.tool != Tool::Select && self.floating.is_some() {
            self.commit_pending();
        }
        let ctrl_pick = ctrl && matches!(self.tool, Tool::Pencil | Tool::Spray);
        match self.tool {
            _ if ctrl_pick => {
                if let Some(c) = self.image.get(p.0, p.1) {
                    self.set_color(button, c);
                }
            }
            Tool::Picker => {
                if let Some(c) = self.composed().get(p.0, p.1) {
                    self.set_color(button, c);
                }
            }
            Tool::Fill => {
                if self.image.in_bounds(p.0, p.1) {
                    self.checkpoint();
                    let c = self.color(button);
                    raster::flood_fill(
                        &mut self.image,
                        p.0,
                        p.1,
                        c,
                        self.tolerance,
                        self.tol_alpha,
                        self.mode,
                    );
                }
            }
            Tool::ChangeColor => {
                if let Some(from) = self.image.get(p.0, p.1) {
                    self.checkpoint();
                    let c = self.color(button);
                    raster::replace_color(&mut self.image, from, c, self.tolerance, self.tol_alpha);
                }
            }
            Tool::Text => {
                if button == 1 || self.text_pos.is_none() {
                    self.place_text(p.0, p.1);
                } else {
                    // a second click elsewhere moves the text box
                    self.place_text(p.0, p.1);
                }
            }
            Tool::Select => self.select_press(p, button),
            Tool::Pencil | Tool::Spray | Tool::Eraser | Tool::Line | Tool::Rect | Tool::Ellipse => {
                let mut st = StrokeState {
                    button,
                    start: p,
                    last: p,
                    mask: Mask::for_image(&self.image),
                    fill: Mask::for_image(&self.image),
                    seed: 0x9E37_79B9_7F4A_7C15 ^ ((p.0 as u64) << 32 | p.1 as u32 as u64),
                };
                self.extend_stroke(&mut st, p, shift);
                self.stroke = Some(st);
            }
        }
        self.display_dirty = true;
    }

    /// Continue the stroke to pixel `p`.
    pub fn drag(&mut self, p: (i32, i32), shift: bool) {
        if self.tool == Tool::Select {
            self.select_drag(p);
            return;
        }
        if let Some(mut st) = self.stroke.take() {
            if p != st.last || self.tool == Tool::Spray {
                self.extend_stroke(&mut st, p, shift);
                self.display_dirty = true;
            }
            self.stroke = Some(st);
        }
    }

    /// Finish the stroke (commits it as one undo step).
    pub fn release(&mut self) {
        if self.tool == Tool::Select {
            self.select_release();
            return;
        }
        if self.stroke.is_some() {
            let img = self.composed_stroke_only();
            self.stroke = None;
            self.checkpoint();
            self.image = img;
        }
    }

    fn composed_stroke_only(&mut self) -> RgbaImage {
        let tp = self.text_pos.take();
        let fl = self.floating.take();
        let d = self.composed();
        self.text_pos = tp;
        self.floating = fl;
        d
    }

    fn extend_stroke(&mut self, st: &mut StrokeState, p: (i32, i32), shift: bool) {
        let size = self.brush;
        match self.tool {
            Tool::Pencil | Tool::Eraser => {
                let p = if shift {
                    // constrain to horizontal / vertical from the start point
                    if (p.0 - st.start.0).abs() >= (p.1 - st.start.1).abs() {
                        (p.0, st.start.1)
                    } else {
                        (st.start.0, p.1)
                    }
                } else {
                    p
                };
                st.mask.line(st.last, p, size);
                st.last = p;
            }
            Tool::Spray => {
                st.mask
                    .spray(p.0, p.1, size.max(3) * 2, self.spray_density, &mut st.seed);
                st.last = p;
            }
            Tool::Line => {
                let p = if shift {
                    raster::constrain_45(st.start, p)
                } else {
                    p
                };
                st.mask.clear();
                st.mask.line(st.start, p, size);
                st.last = p;
            }
            Tool::Rect | Tool::Ellipse => {
                let p = if shift {
                    raster::constrain_square(st.start, p)
                } else {
                    p
                };
                st.mask.clear();
                st.fill.clear();
                let rect = self.tool == Tool::Rect;
                let draw = |m: &mut Mask, filled: bool| {
                    if rect {
                        m.rect(st.start, p, size, filled);
                    } else {
                        m.ellipse(st.start, p, size, filled);
                    }
                };
                match self.shape_fill {
                    ShapeFill::Outline => draw(&mut st.mask, false),
                    ShapeFill::Filled => draw(&mut st.mask, true),
                    ShapeFill::Both => {
                        draw(&mut st.fill, true);
                        draw(&mut st.mask, false);
                    }
                }
                st.last = p;
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ selection

    fn in_selection(&self, p: (i32, i32)) -> bool {
        if let Some(f) = &self.floating {
            return p.0 >= f.x
                && p.1 >= f.y
                && p.0 < f.x + f.img.width as i32
                && p.1 < f.y + f.img.height as i32;
        }
        self.selection.is_some_and(|(x, y, w, h)| {
            p.0 >= x && p.1 >= y && p.0 < x + w as i32 && p.1 < y + h as i32
        })
    }

    fn select_press(&mut self, p: (i32, i32), button: usize) {
        if self.in_selection(p) {
            if self.floating.is_none() {
                // lift the selected pixels; right button copies, left button cuts
                let r = self.selection.unwrap_or((0, 0, 0, 0));
                let img = raster::copy_rect(&self.image, r);
                self.checkpoint();
                if button == 0 {
                    raster::clear_rect(&mut self.image, r);
                }
                self.floating = Some(Floating {
                    img,
                    x: r.0,
                    y: r.1,
                });
            }
            let f = self.floating.as_ref().map_or((0, 0), |f| (f.x, f.y));
            self.sel_drag = Some(SelDrag::Moving { grab: p, orig: f });
        } else {
            self.commit_pending();
            self.selection = None;
            self.sel_drag = Some(SelDrag::Making { start: p });
        }
    }

    fn select_drag(&mut self, p: (i32, i32)) {
        match self.sel_drag {
            Some(SelDrag::Making { start }) => {
                let clamp = |v: i32, m: u32| v.clamp(0, m as i32 - 1);
                let (x0, y0) = (
                    clamp(start.0, self.image.width),
                    clamp(start.1, self.image.height),
                );
                let (x1, y1) = (clamp(p.0, self.image.width), clamp(p.1, self.image.height));
                let (x, y) = (x0.min(x1), y0.min(y1));
                self.selection = Some((
                    x,
                    y,
                    (x0 - x1).unsigned_abs() + 1,
                    (y0 - y1).unsigned_abs() + 1,
                ));
            }
            Some(SelDrag::Moving { grab, orig }) => {
                if let Some(f) = &mut self.floating {
                    f.x = orig.0 + p.0 - grab.0;
                    f.y = orig.1 + p.1 - grab.1;
                    self.selection = Some((f.x, f.y, f.img.width, f.img.height));
                    self.display_dirty = true;
                }
            }
            None => {}
        }
    }

    fn select_release(&mut self) {
        if let Some(SelDrag::Making { start }) = self.sel_drag {
            if self.selection.is_none() {
                let _ = start;
            }
        }
        self.sel_drag = None;
    }

    /// The selected pixels (floating, selection rectangle, or whole image).
    pub fn selected_image(&self) -> RgbaImage {
        if let Some(f) = &self.floating {
            return f.img.clone();
        }
        match self.selection {
            Some(r) => raster::copy_rect(&self.image, r),
            None => self.image.clone(),
        }
    }

    pub fn select_all(&mut self) {
        self.commit_pending();
        self.tool = Tool::Select;
        self.selection = Some((0, 0, self.image.width, self.image.height));
    }

    /// Delete the selection (or the floating pixels).
    pub fn delete_selection(&mut self) {
        if self.floating.take().is_some() {
            self.display_dirty = true;
            self.selection = None;
            return;
        }
        if let Some(r) = self.selection {
            self.checkpoint();
            raster::clear_rect(&mut self.image, r);
        }
    }

    pub fn copy(&mut self) {
        let img = self.selected_image();
        clipboard_set(&img);
        self.clip = Some(img);
    }

    pub fn cut(&mut self) {
        self.copy();
        if self.floating.is_some() || self.selection.is_some() {
            self.delete_selection();
        } else {
            self.checkpoint();
            self.image = RgbaImage::new(self.image.width, self.image.height);
        }
    }

    /// Paste the system clipboard image (or the internal one) as a floating selection.
    pub fn paste(&mut self) {
        let img = clipboard_get().or_else(|| self.clip.clone());
        if let Some(img) = img {
            self.paste_image(img);
        }
    }

    /// Paste `img` as a floating selection at the top-left (or the selection origin).
    pub fn paste_image(&mut self, img: RgbaImage) {
        self.commit_pending();
        self.history.push(self.image.clone());
        self.modified = true;
        let (x, y) = self.selection.map_or((0, 0), |s| (s.0, s.1));
        self.selection = Some((x, y, img.width, img.height));
        self.floating = Some(Floating { img, x, y });
        self.tool = Tool::Select;
        self.display_dirty = true;
    }

    /// Fill the whole image with the left colour (Erase to Left Color).
    pub fn erase_to_left(&mut self) {
        self.commit_pending();
        self.checkpoint();
        let c = self.left;
        for px in self.image.data.as_chunks_mut::<4>().0 {
            px.copy_from_slice(&c);
        }
    }

    // ------------------------------------------------------------ view

    fn fit(&mut self, avail: Vec2) {
        let (w, h) = (
            self.image.width.max(1) as f32,
            self.image.height.max(1) as f32,
        );
        let z = ((avail.x - 24.0) / w).min((avail.y - 24.0) / h);
        self.zoom = if z >= 1.0 {
            z.floor().min(32.0)
        } else {
            z.max(0.05)
        };
        self.pan = Vec2::ZERO;
    }

    fn zoom_by(&mut self, f: f32, about: Option<Vec2>) {
        let old = self.zoom;
        let new = (old * f).clamp(0.05, 64.0);
        let new = if new >= 1.0 {
            new.round().max(1.0)
        } else {
            new
        };
        if let Some(a) = about {
            // keep the image point under `a` (relative to the canvas centre) fixed
            self.pan = a - (a - self.pan) * (new / old);
        } else {
            self.pan *= new / old;
        }
        self.zoom = new;
    }

    fn upload(&mut self, ctx: &Context) {
        if !self.display_dirty && self.tex.is_some() {
            return;
        }
        self.display = self.composed();
        self.display_dirty = false;
        let ci = color_image(&self.display);
        match &mut self.tex {
            Some(t) => t.set(ci, egui::TextureOptions::NEAREST),
            None => {
                self.tex = Some(ctx.load_texture("image-editor", ci, egui::TextureOptions::NEAREST))
            }
        }
        if self.onion_tex.is_none() {
            if let Some(o) = &self.onion {
                self.onion_tex = Some(ctx.load_texture(
                    "image-editor-onion",
                    color_image(o),
                    egui::TextureOptions::NEAREST,
                ));
            }
        }
        if self.checker_tex.is_none() {
            let mut ci = egui::ColorImage::new([2, 2], vec![Color32::from_gray(255); 4]);
            ci.pixels[1] = Color32::from_gray(204);
            ci.pixels[2] = Color32::from_gray(204);
            self.checker_tex = Some(ctx.load_texture(
                "image-editor-checker",
                ci,
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    minification: egui::TextureFilter::Nearest,
                    wrap_mode: egui::TextureWrapMode::Repeat,
                    mipmap_mode: None,
                },
            ));
        }
    }

    // ------------------------------------------------------------ UI

    /// Show the editor window. `id` must be stable while the editor is open.
    pub fn show(&mut self, ctx: &Context, id: Id) -> EditorAction {
        let mut action = EditorAction::None;
        let mut open = true;
        self.upload(ctx);
        let title = match self.frame {
            Some((i, n)) if n > 1 => format!("Image Editor: {} (image {i} of {n})", self.title),
            _ => format!("Image Editor: {}", self.title),
        };
        egui::Window::new(title)
            .id(id)
            .open(&mut open)
            .default_size([1000.0, 700.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .min_size([640.0, 420.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                action = self.window_ui(ui);
            });
        if !open {
            if self.modified {
                self.confirm_close = true;
            } else {
                action = EditorAction::Close { save: false };
            }
        }
        if self.confirm_close {
            egui::Modal::new(id.with("confirm")).show(ctx, |ui| {
                ui.label("Save the changes to the image?");
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        action = EditorAction::Close { save: true };
                    }
                    if ui.button("No").clicked() {
                        action = EditorAction::Close { save: false };
                    }
                    if ui.button("Cancel").clicked() {
                        self.confirm_close = false;
                    }
                });
            });
        }
        if let Some(d) = &mut self.dialog {
            match d.show(ctx, id.with("op"), false) {
                DialogResult::Open => {}
                DialogResult::Ok => {
                    let op = d.op.clone();
                    self.dialog = None;
                    self.apply_op(&op);
                }
                DialogResult::Cancel => self.dialog = None,
            }
        }
        if let Some((w, h)) = &mut self.new_size {
            let mut res = DialogResult::Open;
            egui::Modal::new(id.with("new")).show(ctx, |ui| {
                ui.heading("New image");
                egui::Grid::new("new_size").show(ui, |ui| {
                    ui.label("Width");
                    ui.add(egui::DragValue::new(w).range(1..=8192));
                    ui.end_row();
                    ui.label("Height");
                    ui.add(egui::DragValue::new(h).range(1..=8192));
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
            match res {
                DialogResult::Ok => {
                    let (w, h) = (*w, *h);
                    self.new_size = None;
                    self.set_image(RgbaImage::new(w, h));
                    self.fit_pending = true;
                }
                DialogResult::Cancel => self.new_size = None,
                DialogResult::Open => {}
            }
        }
        if !matches!(action, EditorAction::None) {
            self.commit_pending();
        }
        action
    }

    fn shortcuts(&mut self, ctx: &Context) {
        if ctx.memory(|m| m.focused().is_some()) || self.dialog.is_some() {
            return;
        }
        use egui::{Key, KeyboardShortcut, Modifiers};
        let sc = |m, k| KeyboardShortcut::new(m, k);
        let hit = |s: KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(&s));
        if hit(sc(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z))
            || hit(sc(Modifiers::COMMAND, Key::Y))
        {
            self.redo();
        } else if hit(sc(Modifiers::COMMAND, Key::Z)) {
            self.undo();
        }
        if hit(sc(Modifiers::COMMAND, Key::A)) {
            self.select_all();
        }
        // Copy/Cut/Paste arrive as events (egui turns the shortcuts into them).
        let (copy, cut, paste) = ctx.input(|i| {
            let mut r = (false, false, false);
            for e in &i.events {
                match e {
                    egui::Event::Copy => r.0 = true,
                    egui::Event::Cut => r.1 = true,
                    egui::Event::Paste(_) => r.2 = true,
                    _ => {}
                }
            }
            r
        });
        if copy {
            self.copy();
        }
        if cut {
            self.cut();
        }
        if paste {
            self.paste();
        }
        if hit(sc(Modifiers::NONE, Key::Delete)) {
            self.delete_selection();
        }
        if hit(sc(Modifiers::NONE, Key::Escape)) {
            if self.text_pos.is_some() {
                self.text_pos = None;
                self.display_dirty = true;
            } else {
                self.commit_pending();
                self.selection = None;
            }
        }
        if hit(sc(Modifiers::NONE, Key::Enter)) && self.text_pos.is_some() {
            self.commit_text();
        }
    }

    fn window_ui(&mut self, ui: &mut Ui) -> EditorAction {
        let mut action = EditorAction::None;
        let ctx = ui.ctx().clone();
        self.shortcuts(&ctx);
        egui::MenuBar::new().ui(ui, |ui| {
            action = self.menus(ui);
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            let tb = |ui: &mut Ui, icon: TIcon, label: &str, tip: &str, enabled: bool| {
                tool_icons::button(ui, icon, 24.0, label, tip, enabled, None)
            };
            if tb(
                ui,
                TIcon::CloseSave,
                "✔ Close Saving",
                "Close saving changes",
                true,
            )
            .clicked()
            {
                action = EditorAction::Close { save: true };
            }
            crate::icons::tool_separator(ui);
            if tb(
                ui,
                TIcon::Undo,
                "Undo",
                "Undo (Ctrl+Z)",
                self.history.can_undo(),
            )
            .clicked()
            {
                self.undo();
            }
            if tb(
                ui,
                TIcon::Redo,
                "Redo",
                "Redo (Ctrl+Y)",
                self.history.can_redo(),
            )
            .clicked()
            {
                self.redo();
            }
            crate::icons::tool_separator(ui);
            if tb(ui, TIcon::ZoomOut, "Zoom Out", "Zoom out", true).clicked() {
                self.zoom_by(0.5, None);
            }
            // fixed-width zoom read-out so the row never reflows
            let (zr, _) = ui.allocate_exact_size(Vec2::new(46.0, 24.0), Sense::hover());
            ui.painter().text(
                zr.center(),
                egui::Align2::CENTER_CENTER,
                format!("{:.0}%", self.zoom * 100.0),
                egui::TextStyle::Body.resolve(ui.style()),
                ui.visuals().text_color(),
            );
            if tb(ui, TIcon::ZoomIn, "Zoom In", "Zoom in", true).clicked() {
                self.zoom_by(2.0, None);
            }
            if tb(ui, TIcon::ZoomOne, "No Zoom", "No zoom (1:1)", true).clicked() {
                self.zoom = 1.0;
                self.pan = Vec2::ZERO;
            }
            if tb(
                ui,
                TIcon::ZoomFit,
                "Zoom to Fit",
                "Zoom to fit the window",
                true,
            )
            .clicked()
            {
                self.fit_pending = true;
            }
            crate::icons::tool_separator(ui);
            if tool_icons::button(
                ui,
                TIcon::Grid,
                24.0,
                "Grid",
                "Show the pixel grid",
                true,
                Some(self.grid),
            )
            .clicked()
            {
                self.grid = !self.grid;
            }
            if self.onion.is_some()
                && tool_icons::button(
                    ui,
                    TIcon::Onion,
                    24.0,
                    "Onion skin",
                    "Show the previous frame (onion skin)",
                    true,
                    Some(self.show_onion),
                )
                .clicked()
            {
                self.show_onion = !self.show_onion;
            }
            if let Some((i, n)) = self.frame {
                if n > 1 {
                    crate::icons::tool_separator(ui);
                    if tb(ui, TIcon::Prev, "◀ Prev", "Previous image", i > 0).clicked() {
                        action = EditorAction::Prev;
                    }
                    if tb(ui, TIcon::Next, "Next ▶", "Next image", i + 1 < n).clicked() {
                        action = EditorAction::Next;
                    }
                }
            }
        });
        egui::Panel::bottom(ui.id().with("status"))
            .exact_size(22.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.tool.hint());
                    ui.separator();
                    match self.hover {
                        Some((x, y)) => ui.label(format!("({x}, {y})")),
                        None => ui.label("—"),
                    };
                    ui.separator();
                    ui.label(format!("{}×{}", self.image.width, self.image.height));
                    if let Some((_, _, w, h)) = self.selection {
                        ui.separator();
                        ui.label(format!("sel {w}×{h}"));
                    }
                    if !self.status.is_empty() {
                        ui.separator();
                        ui.colored_label(ui.visuals().warn_fg_color, &self.status);
                    }
                });
            });
        egui::Panel::left(ui.id().with("tools"))
            .exact_size(170.0)
            .resizable(false)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("tools_scroll")
                    .show(ui, |ui| {
                        self.tools_ui(ui);
                    });
            });
        egui::Panel::right(ui.id().with("colors"))
            .exact_size(250.0)
            .resizable(false)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("colors_scroll")
                    .show(ui, |ui| {
                        self.colors_ui(ui);
                    });
            });
        egui::CentralPanel::default().show(ui, |ui| {
            self.canvas_ui(ui);
        });
        action
    }

    fn menus(&mut self, ui: &mut Ui) -> EditorAction {
        let mut action = EditorAction::None;
        let multi = self.frame.is_some_and(|(_, n)| n > 1);
        ui.menu_button("File", |ui| {
            if ui.add_enabled(!multi, egui::Button::new("New…")).clicked() {
                self.new_size = Some((self.image.width.max(1), self.image.height.max(1)));
            }
            if ui.add_enabled(!multi, egui::Button::new("Open…")).clicked() {
                if let Some(img) = pick_image() {
                    self.set_image(img);
                    self.fit_pending = true;
                }
            }
            if let Some((i, n)) = self.frame {
                ui.separator();
                if ui
                    .add_enabled(multi && i > 0, egui::Button::new("Previous Image"))
                    .clicked()
                {
                    action = EditorAction::Prev;
                }
                if ui
                    .add_enabled(multi && i + 1 < n, egui::Button::new("Next Image"))
                    .clicked()
                {
                    action = EditorAction::Next;
                }
            }
            ui.separator();
            if ui.button("Save as PNG File…").clicked() {
                self.commit_pending();
                if let Some(f) = rfd::FileDialog::new()
                    .add_filter("PNG", &["png"])
                    .set_file_name("image.png")
                    .save_file()
                {
                    if let Err(e) =
                        crate::imageio::save_strip_png(std::slice::from_ref(&self.image), &f)
                    {
                        self.status = e;
                    }
                }
            }
            ui.separator();
            if ui.button("Close Saving Changes").clicked() {
                action = EditorAction::Close { save: true };
            }
            if ui.button("Close Discarding Changes").clicked() {
                action = EditorAction::Close { save: false };
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
            if ui.button("Cut").clicked() {
                self.cut();
            }
            if ui.button("Copy").clicked() {
                self.copy();
            }
            if ui.button("Paste").clicked() {
                self.paste();
            }
            if ui.button("Delete").clicked() {
                self.delete_selection();
            }
            ui.separator();
            if ui.button("Select All").clicked() {
                self.select_all();
            }
            if ui.button("Erase to Left Color").clicked() {
                self.erase_to_left();
            }
            if ui.button("Paste from File…").clicked() {
                if let Some(img) = pick_image() {
                    self.paste_image(img);
                }
            }
        });
        ui.menu_button("View", |ui| {
            if ui.button("Zoom In").clicked() {
                self.zoom_by(2.0, None);
            }
            if ui.button("Zoom Out").clicked() {
                self.zoom_by(0.5, None);
            }
            if ui.button("No Zoom").clicked() {
                self.zoom = 1.0;
                self.pan = Vec2::ZERO;
            }
            if ui.button("Zoom to Fit").clicked() {
                self.fit_pending = true;
            }
            ui.separator();
            ui.checkbox(&mut self.grid, "Show Grid");
            if self.onion.is_some() {
                ui.checkbox(&mut self.show_onion, "Show Previous Frame (onion skin)");
            }
        });
        ui.menu_button("Transform", |ui| {
            if let Some(op) = ops::transform_menu(ui, self.image.width, self.image.height, !multi) {
                self.start_op(op);
            }
        });
        ui.menu_button("Image", |ui| {
            if let Some(op) = ops::image_menu(ui, self.left) {
                self.start_op(op);
            }
            ui.separator();
            if ui.button("Set Transparency from Left Color").clicked() {
                self.apply_op(&ImageOp::EraseColor {
                    color: self.left,
                    tol: self.tolerance,
                });
            }
        });
        action
    }

    fn start_op(&mut self, op: ImageOp) {
        self.commit_pending();
        if op.has_dialog() {
            self.dialog = Some(OpDialog::new(op));
        } else {
            self.apply_op(&op);
        }
    }

    fn tools_ui(&mut self, ui: &mut Ui) {
        tool_icons::caption(ui, "Tools");
        // classic paint-program tool box: two columns of fixed-size buttons
        ui.vertical_centered(|ui| {
            tool_icons::well(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(1.0);
                for pair in Tool::ALL.chunks(2) {
                    ui.horizontal(|ui| {
                        for t in pair {
                            let tip = format!("{} — {}", t.label(), t.hint());
                            if tool_icons::button(
                                ui,
                                tool_icon(*t),
                                30.0,
                                t.label(),
                                &tip,
                                true,
                                Some(self.tool == *t),
                            )
                            .clicked()
                                && self.tool != *t
                            {
                                self.commit_pending();
                                self.tool = *t;
                            }
                        }
                    });
                }
            });
        });
        ui.add_space(4.0);
        tool_icons::caption(ui, self.tool.label());
        match self.tool {
            Tool::Pencil | Tool::Line | Tool::Eraser | Tool::Spray | Tool::Rect | Tool::Ellipse => {
                tool_icons::well(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(1.0);
                    for row in [[1u32, 2, 3], [5, 8, 12]] {
                        ui.horizontal(|ui| {
                            for s in row {
                                if tool_icons::brush_button(ui, s, self.brush == s).clicked() {
                                    self.brush = s;
                                }
                            }
                        });
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Size");
                    ui.add(egui::DragValue::new(&mut self.brush).range(1..=64));
                });
            }
            _ => {}
        }
        match self.tool {
            Tool::Rect | Tool::Ellipse => {
                let ell = self.tool == Tool::Ellipse;
                let variants = [
                    (
                        ShapeFill::Outline,
                        if ell {
                            TIcon::EllipseOutline
                        } else {
                            TIcon::RectOutline
                        },
                        "Outline",
                    ),
                    (
                        ShapeFill::Filled,
                        if ell {
                            TIcon::EllipseFilled
                        } else {
                            TIcon::RectFilled
                        },
                        "Filled",
                    ),
                    (
                        ShapeFill::Both,
                        if ell {
                            TIcon::EllipseBoth
                        } else {
                            TIcon::RectBoth
                        },
                        "Outline + fill (other colour)",
                    ),
                ];
                tool_icons::well(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(1.0);
                    ui.horizontal(|ui| {
                        for (f, icon, label) in variants {
                            if tool_icons::button(
                                ui,
                                icon,
                                30.0,
                                label,
                                label,
                                true,
                                Some(self.shape_fill == f),
                            )
                            .clicked()
                            {
                                self.shape_fill = f;
                            }
                        }
                    });
                });
            }
            Tool::Eraser => {
                ui.label("Strength");
                ui.add(egui::Slider::new(&mut self.eraser_strength, 1..=255));
            }
            Tool::Spray => {
                ui.label("Density");
                ui.add(egui::Slider::new(&mut self.spray_density, 1..=64));
            }
            Tool::Fill | Tool::ChangeColor => {
                ui.label("Tolerance");
                ui.add(egui::Slider::new(&mut self.tolerance, 0..=255));
                ui.checkbox(&mut self.tol_alpha, "Compare alpha too");
            }
            Tool::Text => self.text_options(ui),
            Tool::Select => {
                ui.label("Del deletes, Esc ends.\nCtrl+C / X / V clipboard.");
                if ui.button("Select All").clicked() {
                    self.select_all();
                }
            }
            _ => {}
        }
    }
    fn text_options(&mut self, ui: &mut Ui) {
        let changed_before = (
            self.text.clone(),
            self.text_size,
            self.text_align,
            self.text_aa,
        );
        ui.label("Text (# = new line)");
        ui.add(
            egui::TextEdit::multiline(&mut self.text)
                .desired_rows(3)
                .desired_width(150.0),
        );
        ui.horizontal(|ui| {
            ui.label("Size");
            ui.add(egui::DragValue::new(&mut self.text_size).range(4.0..=256.0));
        });
        let fonts = self.fonts.get_or_insert_with(|| {
            let mut v = vec![FontSource::Builtin];
            v.extend(text::system_fonts());
            v
        });
        let mut pick = None;
        egui::ComboBox::from_id_salt("text_font")
            .width(150.0)
            .selected_text(self.font_src.label())
            .show_ui(ui, |ui| {
                for f in fonts.iter() {
                    if ui
                        .selectable_label(*f == self.font_src, f.label())
                        .clicked()
                    {
                        pick = Some(f.clone());
                    }
                }
            });
        if let Some(f) = pick {
            self.font_src = f;
            self.font = None;
            self.display_dirty = true;
        }
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.text_align, Align::Left, "L");
            ui.radio_value(&mut self.text_align, Align::Center, "C");
            ui.radio_value(&mut self.text_align, Align::Right, "R");
        });
        ui.checkbox(&mut self.text_aa, "Anti-alias");
        ui.add_enabled_ui(self.text_pos.is_some(), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Apply").clicked() {
                    self.commit_text();
                }
                if ui.button("Cancel").clicked() {
                    self.text_pos = None;
                    self.display_dirty = true;
                }
            });
        });
        if changed_before
            != (
                self.text.clone(),
                self.text_size,
                self.text_align,
                self.text_aa,
            )
        {
            self.display_dirty = true;
        }
    }

    fn colors_ui(&mut self, ui: &mut Ui) {
        ui.spacing_mut().slider_width = 200.0;
        ui.spacing_mut().interact_size.x = 36.0;
        ui.horizontal(|ui| {
            for b in 0..2 {
                let c = self.color(b);
                let (r, resp) = ui.allocate_exact_size(Vec2::new(48.0, 32.0), Sense::click());
                let p = ui.painter_at(r);
                crate::widgets::checker(&p, r, 6.0);
                p.rect_filled(
                    r,
                    2.0,
                    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]),
                );
                let st = if self.active == b {
                    Stroke::new(2.0, ui.visuals().selection.stroke.color)
                } else {
                    Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
                };
                p.rect_stroke(r, 2.0, st, egui::StrokeKind::Outside);
                if resp
                    .on_hover_text(if b == 0 {
                        "Left colour"
                    } else {
                        "Right colour"
                    })
                    .clicked()
                {
                    self.active = b;
                }
            }
            if tool_icons::button(
                ui,
                TIcon::Swap,
                24.0,
                "Swap colours",
                "Swap the left and right colours",
                true,
                None,
            )
            .clicked()
            {
                let (l, r) = (self.left, self.right);
                self.set_color(0, r);
                self.set_color(1, l);
            }
        });
        ui.label(if self.active == 0 {
            "Editing: left"
        } else {
            "Editing: right"
        });
        let a = self.active;
        if egui::color_picker::color_picker_hsva_2d(
            ui,
            &mut self.hsva[a],
            egui::color_picker::Alpha::OnlyBlend,
        ) {
            let c = self.hsva[a].to_srgba_unmultiplied();
            if a == 0 {
                self.left = c;
            } else {
                self.right = c;
            }
            self.display_dirty |= self.text_pos.is_some();
        }
        ui.horizontal(|ui| {
            ui.label("Opacity");
            let mut alpha = self.color(a)[3];
            if ui
                .add(egui::DragValue::new(&mut alpha).range(0..=255))
                .changed()
            {
                let mut c = self.color(a);
                c[3] = alpha;
                self.set_color(a, c);
            }
        });
        ui.label("Palette (left / right click)");
        egui::Grid::new("palette")
            .spacing([2.0, 2.0])
            .min_col_width(20.0)
            .show(ui, |ui| {
                for (i, rgb) in PALETTE.iter().enumerate() {
                    let c = [rgb[0], rgb[1], rgb[2], 255];
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
                    ui.painter()
                        .rect_filled(r, 1.0, Color32::from_rgb(c[0], c[1], c[2]));
                    if resp.clicked() {
                        self.set_color(0, c);
                    }
                    if resp.secondary_clicked() {
                        self.set_color(1, c);
                    }
                    if i % 8 == 7 {
                        ui.end_row();
                    }
                }
            });
        if ui.button("Transparent as right colour").clicked() {
            self.set_color(1, [0, 0, 0, 0]);
        }
        ui.separator();
        ui.label("Color mode");
        ui.radio_value(&mut self.mode, ColorMode::Blend, "Blend");
        ui.radio_value(&mut self.mode, ColorMode::Replace, "Replace");
    }

    fn canvas_ui(&mut self, ui: &mut Ui) {
        let size = ui.available_size().max(Vec2::splat(16.0));
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "image canvas")
        });
        let resp = resp.on_hover_cursor(egui::CursorIcon::Crosshair);
        if self.fit_pending {
            self.fit(rect.size());
            self.fit_pending = false;
        }
        // zoom with the wheel, pan with the middle button
        if resp.hovered() {
            let (scroll, zoom_d, hover) = ui.input(|i| {
                let wheel: f32 = i
                    .events
                    .iter()
                    .map(|e| match e {
                        egui::Event::MouseWheel {
                            delta, modifiers, ..
                        } if !modifiers.command => delta.y,
                        _ => 0.0,
                    })
                    .sum();
                (wheel, i.zoom_delta(), i.pointer.hover_pos())
            });
            let about = hover.map(|h| h - rect.center());
            if zoom_d != 1.0 {
                self.zoom_by(zoom_d, about);
            } else if scroll != 0.0 {
                self.zoom_by(if scroll > 0.0 { 2.0 } else { 0.5 }, about);
            }
        }
        if resp.dragged_by(PointerButton::Middle) {
            self.pan += resp.drag_delta();
        }
        let isz = Vec2::new(self.image.width as f32, self.image.height as f32) * self.zoom;
        let img_rect = Rect::from_min_size(rect.center() + self.pan - isz / 2.0, isz);
        let to_px = |p: Pos2, z: f32| {
            let v = (p - img_rect.min) / z;
            (v.x.floor() as i32, v.y.floor() as i32)
        };
        // pointer handling
        let (pos, mods) = ui.input(|i| (i.pointer.interact_pos(), i.modifiers));
        self.hover = resp
            .hover_pos()
            .map(|p| to_px(p, self.zoom))
            .filter(|&(x, y)| self.image.in_bounds(x, y));
        let buttons = [PointerButton::Primary, PointerButton::Secondary];
        for (bi, b) in buttons.iter().enumerate() {
            let (pressed, released) =
                ui.input(|i| (i.pointer.button_pressed(*b), i.pointer.button_released(*b)));
            if pressed && resp.hovered() {
                if let Some(p) = pos {
                    self.press(to_px(p, self.zoom), bi, mods.shift, mods.command);
                    self.active_button = Some(bi);
                }
            }
            if self.active_button == Some(bi) {
                if let Some(p) = pos {
                    let px = to_px(p, self.zoom);
                    self.drag(px, mods.shift);
                }
                if released {
                    self.release();
                    self.active_button = None;
                }
            }
        }
        self.upload(ui.ctx());
        if self.stroke.is_some() || self.floating.is_some() {
            ui.ctx().request_repaint();
        }

        // paint
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
        let vis = img_rect.intersect(rect);
        if vis.is_positive() {
            if let Some(t) = &self.checker_tex {
                let cell = 8.0;
                let uv = Rect::from_min_max(
                    Pos2::new(
                        (vis.min.x - img_rect.min.x) / (2.0 * cell),
                        (vis.min.y - img_rect.min.y) / (2.0 * cell),
                    ),
                    Pos2::new(
                        (vis.max.x - img_rect.min.x) / (2.0 * cell),
                        (vis.max.y - img_rect.min.y) / (2.0 * cell),
                    ),
                );
                painter.image(t.id(), vis, uv, Color32::WHITE);
            }
        }
        let full_uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        if let Some(t) = &self.tex {
            painter.image(t.id(), img_rect, full_uv, Color32::WHITE);
        }
        if self.show_onion {
            if let (Some(t), Some(o)) = (&self.onion_tex, &self.onion) {
                if (o.width, o.height) == (self.image.width, self.image.height) {
                    painter.image(t.id(), img_rect, full_uv, Color32::from_white_alpha(70));
                }
            }
        }
        // pixel grid
        if self.grid && self.zoom >= 6.0 && vis.is_positive() {
            let gc = Stroke::new(1.0, Color32::from_black_alpha(50));
            let x0 = ((vis.min.x - img_rect.min.x) / self.zoom).floor() as i32;
            let x1 = ((vis.max.x - img_rect.min.x) / self.zoom).ceil() as i32;
            for x in x0..=x1 {
                let sx = img_rect.min.x + x as f32 * self.zoom;
                painter.line_segment([Pos2::new(sx, vis.min.y), Pos2::new(sx, vis.max.y)], gc);
            }
            let y0 = ((vis.min.y - img_rect.min.y) / self.zoom).floor() as i32;
            let y1 = ((vis.max.y - img_rect.min.y) / self.zoom).ceil() as i32;
            for y in y0..=y1 {
                let sy = img_rect.min.y + y as f32 * self.zoom;
                painter.line_segment([Pos2::new(vis.min.x, sy), Pos2::new(vis.max.x, sy)], gc);
            }
        }
        painter.rect_stroke(
            img_rect,
            0.0,
            Stroke::new(1.0, Color32::from_gray(110)),
            egui::StrokeKind::Outside,
        );
        let px_rect = |x: i32, y: i32, w: u32, h: u32| {
            Rect::from_min_size(
                img_rect.min + Vec2::new(x as f32, y as f32) * self.zoom,
                Vec2::new(w as f32, h as f32) * self.zoom,
            )
        };
        // selection
        if let Some((x, y, w, h)) = self.selection {
            let r = px_rect(x, y, w, h);
            painter.rect_stroke(
                r,
                0.0,
                Stroke::new(3.0, Color32::BLACK),
                egui::StrokeKind::Middle,
            );
            painter.rect_stroke(
                r,
                0.0,
                Stroke::new(1.0, Color32::WHITE),
                egui::StrokeKind::Middle,
            );
        }
        if let Some((x, y)) = self.text_pos {
            let p = img_rect.min + Vec2::new(x as f32, y as f32) * self.zoom;
            painter.circle_stroke(p, 4.0, Stroke::new(1.5, Color32::from_rgb(0, 140, 255)));
        }
        // brush outline under the cursor
        if let Some((x, y)) = self.hover {
            if matches!(
                self.tool,
                Tool::Pencil | Tool::Eraser | Tool::Line | Tool::Rect | Tool::Ellipse
            ) && self.stroke.is_none()
            {
                let s = self.brush.max(1) as i32;
                let lo = -(s - 1) / 2;
                let r = px_rect(x + lo, y + lo, s as u32, s as u32);
                painter.rect_stroke(
                    r,
                    0.0,
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 120, 255, 200)),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }
}

fn image_or_none(o: Option<RgbaImage>) -> Option<RgbaImage> {
    o.filter(|i| i.width > 0 && i.height > 0)
}

fn color_image(img: &RgbaImage) -> egui::ColorImage {
    if img.width == 0 || img.height == 0 || !img.is_consistent() {
        return egui::ColorImage::new([1, 1], vec![Color32::TRANSPARENT]);
    }
    egui::ColorImage::from_rgba_unmultiplied([img.width as usize, img.height as usize], &img.data)
}

fn pick_image() -> Option<RgbaImage> {
    let f = rfd::FileDialog::new()
        .add_filter("Images", &["png", "bmp", "gif", "jpg", "jpeg"])
        .pick_file()?;
    let mut fr = crate::imageio::load_frames(&[f], crate::imageio::LoadOptions::BACKGROUND).ok()?;
    (!fr.is_empty()).then(|| fr.swap_remove(0))
}

/// Put an image on the system clipboard (errors are ignored).
pub fn clipboard_set(img: &RgbaImage) {
    if img.width == 0 || img.height == 0 {
        return;
    }
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_image(arboard::ImageData {
            width: img.width as usize,
            height: img.height as usize,
            bytes: std::borrow::Cow::Borrowed(&img.data),
        });
    }
}

/// Image from the system clipboard, if any.
pub fn clipboard_get() -> Option<RgbaImage> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let d = cb.get_image().ok()?;
    let img = RgbaImage {
        width: d.width as u32,
        height: d.height as u32,
        data: d.bytes.into_owned(),
    };
    img.is_consistent().then_some(img)
}
