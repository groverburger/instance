//! A backend that forwards every call to two backends (renderer comparison):
//! reads are answered by the first, so game logic sees exactly one renderer.

use super::backend::{Backend, Prim, RState, Target, Tex, Vertex};
use super::{Clip, Texture};

pub struct DualBackend {
    pub a: Box<dyn Backend>,
    pub b: Box<dyn Backend>,
}

impl Backend for DualBackend {
    fn name(&self) -> &'static str {
        "dual"
    }
    fn resize_back(&mut self, w: u32, h: u32) {
        self.a.resize_back(w, h);
        self.b.resize_back(w, h);
    }
    fn back_size(&self) -> (u32, u32) {
        self.a.back_size()
    }
    fn surface_create(&mut self, id: i32, w: u32, h: u32) {
        self.a.surface_create(id, w, h);
        self.b.surface_create(id, w, h);
    }
    fn surface_free(&mut self, id: i32) {
        self.a.surface_free(id);
        self.b.surface_free(id);
    }
    fn set_target(&mut self, t: Target) {
        self.a.set_target(t);
        self.b.set_target(t);
    }
    fn target(&self) -> Target {
        self.a.target()
    }
    fn clear(&mut self, clip: Clip, c: [u8; 4], depth: bool) {
        self.a.clear(clip, c, depth);
        self.b.clear(clip, c, depth);
    }
    fn clear_depth(&mut self) {
        self.a.clear_depth();
        self.b.clear_depth();
    }
    fn draw(&mut self, st: &RState, tex: Option<&Tex>, prim: Prim, v: &[Vertex]) {
        self.a.draw(st, tex, prim, v);
        self.b.draw(st, tex, prim, v);
    }
    fn read(&mut self, t: Target, x: i32, y: i32, w: u32, h: u32) -> Texture {
        self.a.read(t, x, y, w, h)
    }
    fn write(&mut self, t: Target, x: i32, y: i32, img: &Texture) {
        self.a.write(t, x, y, img);
        self.b.write(t, x, y, img);
    }
    fn copy(&mut self, dst: Target, dx: i32, dy: i32, src: Target, sx: i32, sy: i32, w: i32, h: i32) {
        self.a.copy(dst, dx, dy, src, sx, sy, w, h);
        self.b.copy(dst, dx, dy, src, sx, sy, w, h);
    }
    fn flush(&mut self) {
        self.a.flush();
        self.b.flush();
    }
    fn pixel(&mut self, t: Target, x: i32, y: i32) -> [u8; 4] {
        self.a.pixel(t, x, y)
    }
    fn read_back(&mut self) -> Texture {
        self.a.read_back()
    }
    fn supports_3d(&self) -> bool {
        self.a.supports_3d() && self.b.supports_3d()
    }
    fn read_back_secondary(&mut self) -> Option<Texture> {
        Some(self.b.read_back())
    }
}

/// Per-channel comparison of two images: (max abs difference, number of pixels
/// differing by more than `tol` in any colour channel).
pub fn compare(a: &Texture, b: &Texture, tol: u8) -> (u8, usize) {
    if a.w != b.w || a.h != b.h {
        return (255, a.px.len().max(b.px.len()));
    }
    let mut mx = 0u8;
    let mut n = 0;
    for (p, q) in a.px.iter().zip(b.px.iter()) {
        let d = (0..3).map(|i| p[i].abs_diff(q[i])).max().unwrap_or(0);
        mx = mx.max(d);
        if d > tol {
            n += 1;
        }
    }
    (mx, n)
}

/// Visualises differences: white where pixels differ by more than `tol`.
pub fn diff_image(a: &Texture, b: &Texture, tol: u8) -> Texture {
    let mut out = Texture::new(a.w, a.h);
    for (i, (p, q)) in a.px.iter().zip(b.px.iter()).enumerate() {
        let d = (0..3).map(|k| p[k].abs_diff(q[k])).max().unwrap_or(0);
        let g = (p[0] as u32 + p[1] as u32 + p[2] as u32) / 12;
        out.px[i] = if d > tol { [255, 0, 255, 255] } else { [g as u8, g as u8, g as u8, 255] };
    }
    out
}
