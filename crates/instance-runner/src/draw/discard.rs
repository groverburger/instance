//! A backend wrapper that can drop rendering into the back buffer (the
//! headless runner's `--no-draw`): while discarding, clears, draws, writes and
//! copies whose destination is the back buffer do nothing; surfaces are still
//! drawn (their contents persist and games read them back), and reads are
//! answered by the wrapped backend (the back buffer then holds an older frame).

use super::backend::{Backend, Prim, RState, Target, Tex, Vertex};
use super::{Clip, Texture};

pub struct DiscardBackend {
    pub inner: Box<dyn Backend>,
    pub discard: bool,
}

impl DiscardBackend {
    fn dropped(&self, t: Target) -> bool {
        self.discard && t == Target::Back
    }
}

impl Backend for DiscardBackend {
    fn name(&self) -> &'static str {
        self.inner.name()
    }
    fn resize_back(&mut self, w: u32, h: u32) {
        self.inner.resize_back(w, h)
    }
    fn back_size(&self) -> (u32, u32) {
        self.inner.back_size()
    }
    fn surface_create(&mut self, id: i32, w: u32, h: u32) {
        self.inner.surface_create(id, w, h)
    }
    fn surface_free(&mut self, id: i32) {
        self.inner.surface_free(id)
    }
    fn set_target(&mut self, t: Target) {
        self.inner.set_target(t)
    }
    fn target(&self) -> Target {
        self.inner.target()
    }
    fn clear(&mut self, clip: Clip, c: [u8; 4], depth: bool) {
        if !self.dropped(self.inner.target()) {
            self.inner.clear(clip, c, depth)
        }
    }
    fn clear_depth(&mut self) {
        if !self.dropped(self.inner.target()) {
            self.inner.clear_depth()
        }
    }
    fn draw(&mut self, st: &RState, tex: Option<&Tex>, prim: Prim, v: &[Vertex]) {
        if !self.dropped(self.inner.target()) {
            self.inner.draw(st, tex, prim, v)
        }
    }
    fn read(&mut self, t: Target, x: i32, y: i32, w: u32, h: u32) -> Texture {
        self.inner.read(t, x, y, w, h)
    }
    fn write(&mut self, t: Target, x: i32, y: i32, img: &Texture) {
        if !self.dropped(t) {
            self.inner.write(t, x, y, img)
        }
    }
    fn copy(&mut self, dst: Target, dx: i32, dy: i32, src: Target, sx: i32, sy: i32, w: i32, h: i32) {
        if !self.dropped(dst) {
            self.inner.copy(dst, dx, dy, src, sx, sy, w, h)
        }
    }
    fn flush(&mut self) {
        self.inner.flush()
    }
    fn pixel(&mut self, t: Target, x: i32, y: i32) -> [u8; 4] {
        self.inner.pixel(t, x, y)
    }
    fn read_back(&mut self) -> Texture {
        self.inner.read_back()
    }
    fn supports_3d(&self) -> bool {
        self.inner.supports_3d()
    }
    fn stats(&self) -> Option<String> {
        self.inner.stats()
    }
    fn read_back_secondary(&mut self) -> Option<Texture> {
        self.inner.read_back_secondary()
    }
    fn as_gl(&mut self) -> Option<&mut super::gl::GlBackend> {
        self.inner.as_gl()
    }
    fn set_discard(&mut self, on: bool) -> bool {
        self.discard = on;
        true
    }
}
