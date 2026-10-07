//! The renderer abstraction. Drawing code (painter, d3d) produces batches of
//! vertices in *target pixel homogeneous space* and hands them to a [`Backend`]:
//! the software rasteriser ([`super::soft::SoftBackend`]) or OpenGL
//! ([`super::gl::GlBackend`]).
//!
//! Vertex positions `(x, y, z, w)`: the target pixel is `(x/w, y/w)` with y
//! growing downwards; `z/w` in 0..1 is the depth (0 = near). Plain 2D drawing
//! uses `w = 1`, `z = 0`.

use super::{Blend, Clip, Texture};
use std::rc::Rc;

/// A render target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Back,
    Surface(i32),
}

/// A texture source: an immutable image (sprite frame, background, glyph, …)
/// identified by its allocation, or the current contents of a surface.
#[derive(Clone, Debug)]
pub enum Tex {
    Img(Rc<Texture>),
    Surf { id: i32, w: u32, h: u32 },
}

impl Tex {
    pub fn w(&self) -> u32 {
        match self {
            Tex::Img(t) => t.w,
            Tex::Surf { w, .. } => *w,
        }
    }
    pub fn h(&self) -> u32 {
        match self {
            Tex::Img(t) => t.h,
            Tex::Surf { h, .. } => *h,
        }
    }
}

/// A vertex for the backends.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    /// Normalised texture coordinates (0..1 over the texture).
    pub u: f32,
    pub v: f32,
    /// RGBA 0..1 (already lit).
    pub c: [f32; 4],
    /// Fog factor 0 (none) .. 1 (fully fog colour).
    pub fog: f32,
}

/// Sub-pixel grid of 2D vertex positions. Both backends snap plain 2D
/// vertices (w = 1) to it so that the GPU's fixed-point rasterisation sees
/// exactly the coordinates the software rasteriser uses.
pub const SNAP: f32 = 64.0;

/// Snaps a plain 2D vertex position to the shared sub-pixel grid.
#[inline]
pub fn snap(v: &Vertex) -> Vertex {
    if v.w != 1.0 {
        return *v;
    }
    Vertex {
        x: (v.x * SNAP).round() / SNAP,
        y: (v.y * SNAP).round() / SNAP,
        ..*v
    }
}

/// Primitive kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prim {
    /// Every three vertices form a triangle.
    Triangles,
    /// Every four vertices form a quad (corners in order around the quad).
    Quads,
    /// Every two vertices form a 1-pixel line.
    Lines,
    /// Every vertex is a single pixel.
    Points,
}

/// Face culling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Cull {
    #[default]
    None,
    /// Draw only triangles that appear counter-clockwise on screen (the
    /// front faces of specs/d3d.md §3.2); clockwise ones are culled.
    KeepCcw,
}

/// Fixed-function state of one draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RState {
    pub blend: Blend,
    /// Scissor rectangle in target pixels.
    pub clip: Clip,
    pub bilinear: bool,
    pub repeat: bool,
    pub depth_test: bool,
    pub depth_write: bool,
    pub cull: Cull,
    /// Fog colour (RGB 0..1) when fog is on.
    pub fog: Option<[f32; 3]>,
    /// Discard fragments with alpha <= this (None: no alpha test).
    pub alpha_test: Option<f32>,
    /// Colour channel write mask (r, g, b, a).
    pub mask: [bool; 4],
}

impl RState {
    pub fn new_2d(blend: Blend, clip: Clip, bilinear: bool, repeat: bool) -> RState {
        RState {
            blend,
            clip,
            bilinear,
            repeat,
            depth_test: false,
            depth_write: false,
            cull: Cull::None,
            fog: None,
            alpha_test: None,
            mask: [true; 4],
        }
    }

    /// Whether this is plain 2D state (no depth, culling or fog).
    pub fn is_plain(&self) -> bool {
        !self.depth_test && !self.depth_write && self.cull == Cull::None && self.fog.is_none() && self.alpha_test.is_none()
    }
}

/// A rendering backend.
pub trait Backend {
    /// Short name for logs ("software", "opengl").
    fn name(&self) -> &'static str;

    /// Resizes (and clears) the back buffer.
    fn resize_back(&mut self, w: u32, h: u32);
    fn back_size(&self) -> (u32, u32);

    /// Creates surface `id` (contents undefined; we clear to transparent black).
    fn surface_create(&mut self, id: i32, w: u32, h: u32);
    fn surface_free(&mut self, id: i32);

    /// Selects the target subsequent draws go to.
    fn set_target(&mut self, t: Target);
    fn target(&self) -> Target;

    /// Fills `clip` of the current target with a colour (no blending). Also
    /// clears the depth buffer when `depth` is set.
    fn clear(&mut self, clip: Clip, c: [u8; 4], depth: bool);
    /// Clears only the depth buffer of the current target.
    fn clear_depth(&mut self);

    /// Draws primitives into the current target.
    fn draw(&mut self, st: &RState, tex: Option<&Tex>, prim: Prim, v: &[Vertex]);

    /// Reads a rectangle of a target as RGBA (rows top first). Out-of-range
    /// pixels are transparent black.
    fn read(&mut self, t: Target, x: i32, y: i32, w: u32, h: u32) -> Texture;
    /// Replaces a rectangle of a target with an image (no blending).
    fn write(&mut self, t: Target, x: i32, y: i32, img: &Texture);
    /// Copies a rectangle between targets (no blending).
    #[allow(clippy::too_many_arguments)]
    fn copy(&mut self, dst: Target, dx: i32, dy: i32, src: Target, sx: i32, sy: i32, w: i32, h: i32) {
        if w <= 0 || h <= 0 {
            return;
        }
        let img = self.read(src, sx, sy, w as u32, h as u32);
        // clip the destination to the source image area actually inside the source
        self.write(dst, dx, dy, &img);
    }

    /// Executes pending draws.
    fn flush(&mut self) {}

    /// Reads one pixel of a target (RGBA).
    fn pixel(&mut self, t: Target, x: i32, y: i32) -> [u8; 4] {
        self.read(t, x, y, 1, 1).px[0]
    }

    /// Whole back buffer.
    fn read_back(&mut self) -> Texture {
        let (w, h) = self.back_size();
        self.read(Target::Back, 0, 0, w, h)
    }

    /// Whether 3D (homogeneous w != 1, depth buffer) is supported.
    fn supports_3d(&self) -> bool {
        false
    }

    /// For the comparison backend: the second renderer's back buffer.
    fn read_back_secondary(&mut self) -> Option<Texture> {
        None
    }

    /// Downcast helper for window presentation.
    fn as_gl(&mut self) -> Option<&mut super::gl::GlBackend> {
        None
    }
}
