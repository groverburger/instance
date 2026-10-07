//! High-level primitives drawn in user (room) coordinates through a transform
//! onto the current target of a [`Backend`].

use super::backend::{Backend, Prim, RState, Tex, Vertex};
use super::mat::M4;
use super::Clip;

/// An affine transform `x' = a·x + b·y + tx`, `y' = c·x + d·y + ty`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xf {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Default for Xf {
    fn default() -> Self {
        Xf::IDENTITY
    }
}

impl Xf {
    pub const IDENTITY: Xf = Xf {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    #[inline]
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.b * y + self.tx, self.c * x + self.d * y + self.ty)
    }

    /// Maps the room rectangle (vx,vy,vw,vh) rotated by `angle` degrees about its
    /// centre onto the port (px,py,pw,ph).
    #[allow(clippy::too_many_arguments)]
    pub fn view(vx: f64, vy: f64, vw: f64, vh: f64, angle: f64, px: f64, py: f64, pw: f64, ph: f64) -> Xf {
        let sx = if vw != 0.0 { pw / vw } else { 1.0 };
        let sy = if vh != 0.0 { ph / vh } else { 1.0 };
        let (cx, cy) = (vx + vw / 2.0, vy + vh / 2.0);
        let r = angle.to_radians();
        let (cs, sn) = (r.cos(), r.sin());
        // translate centre to origin, rotate (counter-clockwise on screen), scale,
        // translate to port centre
        let a = sx * cs;
        let b = sx * -sn;
        let c = sy * sn;
        let d = sy * cs;
        let (pcx, pcy) = (px + pw / 2.0, py + ph / 2.0);
        Xf {
            a,
            b,
            c,
            d,
            tx: pcx - (a * cx + b * cy),
            ty: pcy - (c * cx + d * cy),
        }
    }

    /// Inverse transform (None if singular).
    pub fn inverse(&self) -> Option<Xf> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-12 {
            return None;
        }
        let ia = self.d / det;
        let ib = -self.b / det;
        let ic = -self.c / det;
        let id = self.a / det;
        Some(Xf {
            a: ia,
            b: ib,
            c: ic,
            d: id,
            tx: -(ia * self.tx + ib * self.ty),
            ty: -(ic * self.tx + id * self.ty),
        })
    }

    /// As a 4×4 matrix (z passes through, w = 1).
    pub fn to_m4(&self) -> M4 {
        [[self.a, self.b, 0.0, self.tx], [self.c, self.d, 0.0, self.ty], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
    }
}

/// Room → target transform state shared by all drawing calls.
#[derive(Clone, Copy, Debug)]
pub struct Pipe {
    /// 2D affine room → pixel transform (used when `m` is None).
    pub xf: Xf,
    /// Full room (x, y, z) → pixel-homogeneous transform (3D mode or a world
    /// transform); overrides `xf`.
    pub m: Option<M4>,
    /// z coordinate given to 2D drawing (the depth in 3D mode).
    pub z: f64,
    /// Fog: (room → fog-distance row, start, end) giving the per-vertex factor.
    pub fog: Option<([f64; 4], f64, f64)>,
    /// Colour multiplier (ambient light on normal-less geometry).
    pub tint: Option<[f32; 3]>,
}

impl Default for Pipe {
    fn default() -> Self {
        Pipe {
            xf: Xf::IDENTITY,
            m: None,
            z: 0.0,
            fog: None,
            tint: None,
        }
    }
}

impl Pipe {
    /// Linear fog factor for a room-space point.
    #[inline]
    pub fn fog_at(&self, x: f64, y: f64, z: f64) -> f32 {
        match &self.fog {
            Some((row, s, e)) => {
                let d = row[0] * x + row[1] * y + row[2] * z + row[3];
                // unclamped: interpolated per pixel, clamped by the backend
                if e > s {
                    ((d - s) / (e - s)) as f32
                } else if d >= *s {
                    1.0
                } else {
                    0.0
                }
            }
            None => 0.0,
        }
    }

    #[inline]
    pub fn vert3(&self, x: f64, y: f64, z: f64, u: f32, v: f32, c: [f32; 4]) -> Vertex {
        let fog = self.fog_at(x, y, z);
        let c = match self.tint {
            Some(t) => [c[0] * t[0], c[1] * t[1], c[2] * t[2], c[3]],
            None => c,
        };
        match &self.m {
            None => {
                let (tx, ty) = self.xf.apply(x, y);
                Vertex {
                    x: tx as f32,
                    y: ty as f32,
                    z: 0.5,
                    w: 1.0,
                    u,
                    v,
                    c,
                    fog,
                }
            }
            Some(m) => {
                let p = super::mat::apply(m, [x, y, z, 1.0]);
                Vertex {
                    x: p[0] as f32,
                    y: p[1] as f32,
                    z: p[2] as f32,
                    w: p[3] as f32,
                    u,
                    v,
                    c,
                    fog,
                }
            }
        }
    }

    #[inline]
    pub fn vert(&self, x: f64, y: f64, u: f32, v: f32, c: [f32; 4]) -> Vertex {
        self.vert3(x, y, self.z, u, v, c)
    }
}

/// Drawing onto the current target of a backend.
pub struct Painter<'a> {
    pub be: &'a mut dyn Backend,
    pub pipe: Pipe,
    pub st: RState,
}

impl Painter<'_> {
    #[inline]
    fn v(&self, x: f64, y: f64, u: f32, v: f32, c: [f32; 4]) -> Vertex {
        self.pipe.vert(x, y, u, v, c)
    }

    /// Clears the whole target (ignores blending and clip).
    pub fn clear(&mut self, c: [u8; 4]) {
        let all = Clip {
            x0: 0,
            y0: 0,
            x1: i32::MAX / 2,
            y1: i32::MAX / 2,
        };
        self.be.clear(all, c, true);
    }

    /// Fills the clip area with a colour (no blending); also clears depth.
    pub fn clear_clip(&mut self, c: [u8; 4]) {
        self.be.clear(self.st.clip, c, true);
    }

    /// A coloured triangle.
    pub fn tri(&mut self, p: [(f64, f64); 3], c: [[f32; 4]; 3]) {
        let v = [
            self.v(p[0].0, p[0].1, 0.0, 0.0, c[0]),
            self.v(p[1].0, p[1].1, 0.0, 0.0, c[1]),
            self.v(p[2].0, p[2].1, 0.0, 0.0, c[2]),
        ];
        self.be.draw(&self.st, None, Prim::Triangles, &v);
    }

    /// A textured triangle (uv in texel coordinates).
    pub fn tri_tex(&mut self, tex: &Tex, p: [(f64, f64); 3], uv: [(f32, f32); 3], c: [[f32; 4]; 3], repeat: bool) {
        let (tw, th) = (tex.w().max(1) as f32, tex.h().max(1) as f32);
        let v = [
            self.v(p[0].0, p[0].1, uv[0].0 / tw, uv[0].1 / th, c[0]),
            self.v(p[1].0, p[1].1, uv[1].0 / tw, uv[1].1 / th, c[1]),
            self.v(p[2].0, p[2].1, uv[2].0 / tw, uv[2].1 / th, c[2]),
        ];
        let mut st = self.st;
        st.repeat = repeat;
        self.be.draw(&st, Some(tex), Prim::Triangles, &v);
    }

    /// A textured quad; corners in order top-left, top-right, bottom-right,
    /// bottom-left; uv in texel coordinates.
    pub fn quad_tex(&mut self, tex: &Tex, p: [(f64, f64); 4], uv: [(f32, f32); 4], c: [[f32; 4]; 4], repeat: bool) {
        if tex.w() == 0 || tex.h() == 0 {
            return;
        }
        let (tw, th) = (tex.w() as f32, tex.h() as f32);
        let v = [
            self.v(p[0].0, p[0].1, uv[0].0 / tw, uv[0].1 / th, c[0]),
            self.v(p[1].0, p[1].1, uv[1].0 / tw, uv[1].1 / th, c[1]),
            self.v(p[2].0, p[2].1, uv[2].0 / tw, uv[2].1 / th, c[2]),
            self.v(p[3].0, p[3].1, uv[3].0 / tw, uv[3].1 / th, c[3]),
        ];
        let mut st = self.st;
        st.repeat = repeat;
        self.be.draw(&st, Some(tex), Prim::Quads, &v);
    }

    /// A coloured quad (corners tl, tr, br, bl).
    pub fn quad(&mut self, p: [(f64, f64); 4], c: [[f32; 4]; 4]) {
        let v = [
            self.v(p[0].0, p[0].1, 0.0, 0.0, c[0]),
            self.v(p[1].0, p[1].1, 0.0, 0.0, c[1]),
            self.v(p[2].0, p[2].1, 0.0, 0.0, c[2]),
            self.v(p[3].0, p[3].1, 0.0, 0.0, c[3]),
        ];
        self.be.draw(&self.st, None, Prim::Quads, &v);
    }

    /// `draw_rectangle` family: inclusive pixel corners; colours tl, tr, br, bl.
    pub fn rect(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, c: [[f32; 4]; 4], outline: bool) {
        let (x1, x2) = (x1.min(x2), x1.max(x2));
        let (y1, y2) = (y1.min(y2), y1.max(y2));
        if !outline {
            self.quad([(x1, y1), (x2 + 1.0, y1), (x2 + 1.0, y2 + 1.0), (x1, y2 + 1.0)], c);
        } else {
            self.line(x1, y1, x2 + 1.0, y1, 1.0, c[0], c[1]);
            self.line(x1, y2, x2 + 1.0, y2, 1.0, c[3], c[2]);
            if y2 - y1 > 1.0 {
                self.line(x1, y1 + 1.0, x1, y2, 1.0, c[0], c[3]);
                self.line(x2, y1 + 1.0, x2, y2, 1.0, c[1], c[2]);
            }
        }
    }

    /// A line of width `w` (pixel-centred).
    #[allow(clippy::too_many_arguments)]
    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, w: f64, c1: [f32; 4], c2: [f32; 4]) {
        let (dx, dy) = (x2 - x1, y2 - y1);
        let len = (dx * dx + dy * dy).sqrt();
        let hw = w.max(1.0) / 2.0;
        let (x1, y1, x2, y2) = if w <= 1.0 {
            (x1 + 0.5, y1 + 0.5, x2 + 0.5, y2 + 0.5)
        } else {
            (x1, y1, x2, y2)
        };
        if len < 1e-9 {
            self.quad(
                [(x1 - hw, y1 - hw), (x1 + hw, y1 - hw), (x1 + hw, y1 + hw), (x1 - hw, y1 + hw)],
                [c1, c1, c1, c1],
            );
            return;
        }
        let (nx, ny) = (-dy / len * hw, dx / len * hw);
        self.quad(
            [(x1 + nx, y1 + ny), (x2 + nx, y2 + ny), (x2 - nx, y2 - ny), (x1 - nx, y1 - ny)],
            [c1, c2, c2, c1],
        );
    }

    /// A single pixel.
    pub fn point(&mut self, x: f64, y: f64, c: [f32; 4]) {
        let (x, y) = (x.floor(), y.floor());
        self.quad([(x, y), (x + 1.0, y), (x + 1.0, y + 1.0), (x, y + 1.0)], [c, c, c, c]);
    }

    /// Ellipse inscribed in the box; centre colour `cc`, edge colour `ce`.
    #[allow(clippy::too_many_arguments)]
    pub fn ellipse(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, cc: [f32; 4], ce: [f32; 4], outline: bool, precision: i32) {
        let (cx, cy) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
        let (rx, ry) = ((x2 - x1).abs() / 2.0, (y2 - y1).abs() / 2.0);
        let n = precision.clamp(4, 64) as usize;
        let pts: Vec<(f64, f64)> = (0..=n)
            .map(|i| {
                let a = i as f64 / n as f64 * std::f64::consts::TAU;
                (cx + rx * a.cos(), cy + ry * a.sin())
            })
            .collect();
        if outline {
            for i in 0..n {
                self.line(pts[i].0, pts[i].1, pts[i + 1].0, pts[i + 1].1, 1.0, ce, ce);
            }
        } else {
            // shift by half a pixel so the filled shape is centred on pixel centres
            let mut v = Vec::with_capacity(n * 3);
            for i in 0..n {
                v.push(self.v(cx + 0.5, cy + 0.5, 0.0, 0.0, cc));
                v.push(self.v(pts[i].0 + 0.5, pts[i].1 + 0.5, 0.0, 0.0, ce));
                v.push(self.v(pts[i + 1].0 + 0.5, pts[i + 1].1 + 0.5, 0.0, 0.0, ce));
            }
            self.be.draw(&self.st, None, Prim::Triangles, &v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::soft::SoftBackend;
    use crate::draw::Blend;

    #[test]
    fn view_xf() {
        let x = Xf::view(100.0, 50.0, 200.0, 100.0, 0.0, 0.0, 0.0, 400.0, 200.0);
        assert_eq!(x.apply(100.0, 50.0), (0.0, 0.0));
        assert_eq!(x.apply(300.0, 150.0), (400.0, 200.0));
        let i = x.inverse().unwrap();
        let (a, b) = i.apply(400.0, 200.0);
        assert!((a - 300.0).abs() < 1e-9 && (b - 150.0).abs() < 1e-9);
    }

    #[test]
    fn rect_pixels() {
        let col = [1.0, 1.0, 1.0, 1.0];
        let st = RState::new_2d(Blend::NORMAL, Clip { x0: 0, y0: 0, x1: 10, y1: 10 }, false, false);
        let mut be = SoftBackend::new(10, 10);
        let mut p = Painter { be: &mut be, pipe: Pipe::default(), st };
        p.rect(1.0, 1.0, 3.0, 3.0, [col; 4], false);
        let c = be.read_back();
        assert_eq!(c.px.iter().filter(|p| p[3] > 0).count(), 9);
        let mut be = SoftBackend::new(10, 10);
        let mut p = Painter { be: &mut be, pipe: Pipe::default(), st };
        p.rect(1.0, 1.0, 4.0, 4.0, [col; 4], true);
        let c = be.read_back();
        assert_eq!(c.px.iter().filter(|p| p[3] > 0).count(), 12);
        assert_eq!(c.get(2, 2)[3], 0);
    }
}
