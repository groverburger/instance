//! High-level primitives drawn in user (room) coordinates through an affine
//! transform onto a canvas.

use super::{triangle, Blend, Canvas, Clip, Sampler, Texture, Vtx};

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
}

/// Drawing onto one canvas.
pub struct Painter<'a> {
    pub target: &'a mut Canvas,
    pub xf: Xf,
    pub clip: Clip,
    pub blend: Blend,
    pub bilinear: bool,
}

impl Painter<'_> {
    #[inline]
    fn v(&self, x: f64, y: f64, u: f32, v: f32, c: [f32; 4]) -> Vtx {
        let (tx, ty) = self.xf.apply(x, y);
        Vtx {
            x: tx as f32,
            y: ty as f32,
            u,
            v,
            c,
        }
    }

    /// Clears the whole target (ignores blending and clip).
    pub fn clear(&mut self, c: [u8; 4]) {
        self.target.fill(c);
    }

    /// Fills the clip area with a colour (no blending).
    pub fn clear_clip(&mut self, c: [u8; 4]) {
        let clip = self.clip.intersect(Clip::full(self.target));
        let w = self.target.w as i32;
        for y in clip.y0..clip.y1 {
            for x in clip.x0..clip.x1 {
                self.target.px[(y * w + x) as usize] = c;
            }
        }
    }

    /// A coloured triangle.
    pub fn tri(&mut self, p: [(f64, f64); 3], c: [[f32; 4]; 3]) {
        let v = [
            self.v(p[0].0, p[0].1, 0.0, 0.0, c[0]),
            self.v(p[1].0, p[1].1, 0.0, 0.0, c[1]),
            self.v(p[2].0, p[2].1, 0.0, 0.0, c[2]),
        ];
        triangle(self.target, self.clip, self.blend, v, None);
    }

    /// A textured triangle (uv in texel coordinates).
    pub fn tri_tex(&mut self, tex: &Texture, p: [(f64, f64); 3], uv: [(f32, f32); 3], c: [[f32; 4]; 3], repeat: bool) {
        let v = [
            self.v(p[0].0, p[0].1, uv[0].0, uv[0].1, c[0]),
            self.v(p[1].0, p[1].1, uv[1].0, uv[1].1, c[1]),
            self.v(p[2].0, p[2].1, uv[2].0, uv[2].1, c[2]),
        ];
        let s = Sampler {
            tex,
            bilinear: self.bilinear,
            repeat,
        };
        triangle(self.target, self.clip, self.blend, v, Some(s));
    }

    /// If the quad maps to an axis-aligned rectangle in target space with
    /// axis-aligned texture coordinates and one colour, returns its target corners.
    fn axis_rect(&self, p: &[(f64, f64); 4], uv: &[(f32, f32); 4], c: &[[f32; 4]; 4]) -> Option<(f32, f32, f32, f32)> {
        if !(c[0] == c[1] && c[1] == c[2] && c[2] == c[3]) {
            return None;
        }
        if !(uv[0].1 == uv[1].1 && uv[1].0 == uv[2].0 && uv[2].1 == uv[3].1 && uv[3].0 == uv[0].0) {
            return None;
        }
        let q: Vec<(f64, f64)> = p.iter().map(|&(x, y)| self.xf.apply(x, y)).collect();
        let eq = |a: f64, b: f64| (a - b).abs() < 1e-6;
        if eq(q[0].1, q[1].1) && eq(q[1].0, q[2].0) && eq(q[2].1, q[3].1) && eq(q[3].0, q[0].0) {
            return Some((q[0].0 as f32, q[0].1 as f32, q[2].0 as f32, q[2].1 as f32));
        }
        None
    }

    /// A textured quad; corners in order top-left, top-right, bottom-right,
    /// bottom-left.
    pub fn quad_tex(&mut self, tex: &Texture, p: [(f64, f64); 4], uv: [(f32, f32); 4], c: [[f32; 4]; 4], repeat: bool) {
        if tex.w == 0 || tex.h == 0 {
            return;
        }
        if let Some((x0, y0, x1, y1)) = self.axis_rect(&p, &uv, &c) {
            // corner 0 maps to uv[0]; corner 2 to uv[2] (handles mirrored quads)
            let (mut ua, mut ub, mut va, mut vb) = (uv[0].0, uv[2].0, uv[0].1, uv[2].1);
            let (mut xa, mut xb, mut ya, mut yb) = (x0, x1, y0, y1);
            if xa > xb {
                std::mem::swap(&mut xa, &mut xb);
                std::mem::swap(&mut ua, &mut ub);
            }
            if ya > yb {
                std::mem::swap(&mut ya, &mut yb);
                std::mem::swap(&mut va, &mut vb);
            }
            let s = Sampler { tex, bilinear: self.bilinear, repeat };
            super::rect_fast(self.target, self.clip, self.blend, xa, ya, xb, yb, (ua, va, ub, vb), c[0], Some(s));
            return;
        }
        self.tri_tex(tex, [p[0], p[1], p[2]], [uv[0], uv[1], uv[2]], [c[0], c[1], c[2]], repeat);
        self.tri_tex(tex, [p[0], p[2], p[3]], [uv[0], uv[2], uv[3]], [c[0], c[2], c[3]], repeat);
    }

    /// A coloured quad (corners tl, tr, br, bl).
    pub fn quad(&mut self, p: [(f64, f64); 4], c: [[f32; 4]; 4]) {
        let uv = [(0.0, 0.0); 4];
        if let Some((x0, y0, x1, y1)) = self.axis_rect(&p, &uv, &c) {
            super::rect_fast(self.target, self.clip, self.blend, x0, y0, x1, y1, (0.0, 0.0, 0.0, 0.0), c[0], None);
            return;
        }
        self.tri([p[0], p[1], p[2]], [c[0], c[1], c[2]]);
        self.tri([p[0], p[2], p[3]], [c[0], c[2], c[3]]);
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
            for i in 0..n {
                self.tri(
                    [
                        (cx + 0.5, cy + 0.5),
                        (pts[i].0 + 0.5, pts[i].1 + 0.5),
                        (pts[i + 1].0 + 0.5, pts[i + 1].1 + 0.5),
                    ],
                    [cc, ce, ce],
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut c = Canvas::new(10, 10);
        let col = [1.0, 1.0, 1.0, 1.0];
        let mut p = Painter {
            target: &mut c,
            xf: Xf::IDENTITY,
            clip: Clip { x0: 0, y0: 0, x1: 10, y1: 10 },
            blend: Blend::NORMAL,
            bilinear: false,
        };
        p.rect(1.0, 1.0, 3.0, 3.0, [col; 4], false);
        assert_eq!(c.px.iter().filter(|p| p[3] > 0).count(), 9);
        let mut c = Canvas::new(10, 10);
        let mut p = Painter {
            target: &mut c,
            xf: Xf::IDENTITY,
            clip: Clip { x0: 0, y0: 0, x1: 10, y1: 10 },
            blend: Blend::NORMAL,
            bilinear: false,
        };
        p.rect(1.0, 1.0, 4.0, 4.0, [col; 4], true);
        assert_eq!(c.px.iter().filter(|p| p[3] > 0).count(), 12);
        assert_eq!(c.get(2, 2)[3], 0);
    }
}
