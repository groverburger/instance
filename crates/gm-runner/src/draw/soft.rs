//! Software backend: the CPU rasteriser of `draw/mod.rs` behind [`Backend`].
//! 2D batches (w = 1, no depth) use the exact 2D rasteriser; homogeneous /
//! depth-tested batches (3D mode) are clipped against the near plane and
//! rasterised with a float depth buffer and perspective-correct attributes.

use super::backend::{snap, Backend, Cull, Prim, RState, Target, Tex, Vertex};
use super::{blend_pixel, rect_fast, triangle, Canvas, Clip, Sampler, Texture, Vtx};
use std::collections::HashMap;

pub struct SoftBackend {
    back: Canvas,
    surfaces: HashMap<i32, Canvas>,
    depth: HashMap<Target, Vec<f32>>,
    target: Target,
}

impl Default for SoftBackend {
    fn default() -> Self {
        Self::new(640, 480)
    }
}

impl SoftBackend {
    pub fn new(w: u32, h: u32) -> Self {
        SoftBackend {
            back: Canvas::new(w, h),
            surfaces: HashMap::new(),
            depth: HashMap::new(),
            target: Target::Back,
        }
    }

    fn canvas(&self, t: Target) -> Option<&Canvas> {
        match t {
            Target::Back => Some(&self.back),
            Target::Surface(id) => self.surfaces.get(&id),
        }
    }

    fn canvas_mut(&mut self, t: Target) -> Option<&mut Canvas> {
        match t {
            Target::Back => Some(&mut self.back),
            Target::Surface(id) => self.surfaces.get_mut(&id),
        }
    }

    fn draw_2d(target: &mut Canvas, st: &RState, tex: Option<&Texture>, prim: Prim, v: &[Vertex]) {
        let (tw, th) = tex.map(|t| (t.w as f32, t.h as f32)).unwrap_or((0.0, 0.0));
        let rv = |p: &Vertex| {
            let p = snap(p);
            Vtx { x: p.x, y: p.y, u: p.u * tw, v: p.v * th, c: p.c }
        };
        let sampler = tex.map(|t| Sampler { tex: t, bilinear: st.bilinear, repeat: st.repeat });
        match prim {
            Prim::Triangles => {
                for t in v.chunks_exact(3) {
                    triangle(target, st.clip, st.blend, [rv(&t[0]), rv(&t[1]), rv(&t[2])], sampler);
                }
            }
            Prim::Quads => {
                for q in v.chunks_exact(4) {
                    let r = [rv(&q[0]), rv(&q[1]), rv(&q[2]), rv(&q[3])];
                    if let Some((x0, y0, x1, y1, uv)) = axis_rect(&r) {
                        rect_fast(target, st.clip, st.blend, x0, y0, x1, y1, uv, r[0].c, sampler);
                    } else {
                        triangle(target, st.clip, st.blend, [r[0], r[1], r[2]], sampler);
                        triangle(target, st.clip, st.blend, [r[0], r[2], r[3]], sampler);
                    }
                }
            }
            Prim::Lines => {
                for l in v.chunks_exact(2) {
                    for q in line_quad(&l[0], &l[1]).chunks_exact(4) {
                        let r = [rv(&q[0]), rv(&q[1]), rv(&q[2]), rv(&q[3])];
                        triangle(target, st.clip, st.blend, [r[0], r[1], r[2]], sampler);
                        triangle(target, st.clip, st.blend, [r[0], r[2], r[3]], sampler);
                    }
                }
            }
            Prim::Points => {
                for p in v {
                    let (x, y) = (p.x.floor(), p.y.floor());
                    let r = rv(p);
                    rect_fast(target, st.clip, st.blend, x, y, x + 1.0, y + 1.0, (r.u, r.v, r.u, r.v), r.c, sampler);
                }
            }
        }
    }
}

/// Expands a line (pixel space, w = 1) into a 1-pixel-wide quad through pixel
/// centres, like `Painter::line`.
pub fn line_quad(a: &Vertex, b: &Vertex) -> [Vertex; 4] {
    let (x1, y1, x2, y2) = (a.x + 0.5, a.y + 0.5, b.x + 0.5, b.y + 0.5);
    let (dx, dy) = (x2 - x1, y2 - y1);
    let len = (dx * dx + dy * dy).sqrt();
    let (nx, ny) = if len < 1e-6 { (0.5, 0.0) } else { (-dy / len * 0.5, dx / len * 0.5) };
    let mk = |p: &Vertex, x: f32, y: f32| Vertex { x, y, ..*p };
    if len < 1e-6 {
        return [
            mk(a, x1 - 0.5, y1 - 0.5),
            mk(a, x1 + 0.5, y1 - 0.5),
            mk(a, x1 + 0.5, y1 + 0.5),
            mk(a, x1 - 0.5, y1 + 0.5),
        ];
    }
    [mk(a, x1 + nx, y1 + ny), mk(b, x2 + nx, y2 + ny), mk(b, x2 - nx, y2 - ny), mk(a, x1 - nx, y1 - ny)]
}

/// If a quad (pixel space) is an axis-aligned rectangle with axis-aligned
/// texture coordinates and one colour, returns (x0, y0, x1, y1, uv) with
/// x0 < x1, y0 < y1.
#[allow(clippy::type_complexity)]
fn axis_rect(r: &[Vtx; 4]) -> Option<(f32, f32, f32, f32, (f32, f32, f32, f32))> {
    if !(r[0].c == r[1].c && r[1].c == r[2].c && r[2].c == r[3].c) {
        return None;
    }
    let eq = |a: f32, b: f32| (a - b).abs() < 1e-4;
    let shape = eq(r[0].y, r[1].y) && eq(r[1].x, r[2].x) && eq(r[2].y, r[3].y) && eq(r[3].x, r[0].x);
    let uvs = r[0].v == r[1].v && r[1].u == r[2].u && r[2].v == r[3].v && r[3].u == r[0].u;
    if !(shape && uvs) {
        return None;
    }
    let (mut ua, mut ub, mut va, mut vb) = (r[0].u, r[2].u, r[0].v, r[2].v);
    let (mut xa, mut xb, mut ya, mut yb) = (r[0].x, r[2].x, r[0].y, r[2].y);
    if xa > xb {
        std::mem::swap(&mut xa, &mut xb);
        std::mem::swap(&mut ua, &mut ub);
    }
    if ya > yb {
        std::mem::swap(&mut ya, &mut yb);
        std::mem::swap(&mut va, &mut vb);
    }
    Some((xa, ya, xb, yb, (ua, va, ub, vb)))
}

/// Clips a polygon (homogeneous) against z >= 0 and w > eps.
fn clip_near(poly: &[Vertex]) -> Vec<Vertex> {
    let mut out = poly.to_vec();
    for plane in 0..2 {
        let d = |p: &Vertex| if plane == 0 { p.z } else { p.w - 1e-5 };
        let inp = std::mem::take(&mut out);
        if inp.is_empty() {
            break;
        }
        for i in 0..inp.len() {
            let a = &inp[i];
            let b = &inp[(i + 1) % inp.len()];
            let (da, db) = (d(a), d(b));
            if da >= 0.0 {
                out.push(*a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                let t = da / (da - db);
                out.push(lerp_v(a, b, t));
            }
        }
    }
    out
}

fn lerp_v(a: &Vertex, b: &Vertex, t: f32) -> Vertex {
    let l = |x: f32, y: f32| x + (y - x) * t;
    Vertex {
        x: l(a.x, b.x),
        y: l(a.y, b.y),
        z: l(a.z, b.z),
        w: l(a.w, b.w),
        u: l(a.u, b.u),
        v: l(a.v, b.v),
        c: [l(a.c[0], b.c[0]), l(a.c[1], b.c[1]), l(a.c[2], b.c[2]), l(a.c[3], b.c[3])],
        fog: l(a.fog, b.fog),
    }
}

/// Screen-space vertex for the 3D rasteriser (attributes divided by w).
#[derive(Clone, Copy)]
struct SV {
    x: f32,
    y: f32,
    z: f32,
    iw: f32,
    a: [f32; 7], // u, v, r, g, b, a, fog (each / w)
}

fn to_sv(p: &Vertex) -> SV {
    let iw = 1.0 / p.w;
    SV {
        x: p.x * iw,
        y: p.y * iw,
        z: p.z * iw,
        iw,
        a: [p.u * iw, p.v * iw, p.c[0] * iw, p.c[1] * iw, p.c[2] * iw, p.c[3] * iw, p.fog * iw],
    }
}

#[inline]
fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

#[allow(clippy::too_many_arguments)]
fn tri_3d(target: &mut Canvas, depth: &mut [f32], st: &RState, tex: Option<&Texture>, v: [SV; 3]) {
    let clip = st.clip.intersect(Clip::full(target));
    if clip.x0 >= clip.x1 || clip.y0 >= clip.y1 {
        return;
    }
    let [mut a, mut b, c] = v;
    let mut area = edge(a.x, a.y, b.x, b.y, c.x, c.y);
    if !area.is_finite() || area.abs() < 1e-10 {
        return;
    }
    // y is down: a positive area is clockwise on screen
    if st.cull == Cull::KeepCcw && area > 0.0 {
        return;
    }
    if area < 0.0 {
        std::mem::swap(&mut a, &mut b);
        area = -area;
    }
    let minx = (a.x.min(b.x).min(c.x).floor() as i32).max(clip.x0);
    let maxx = (a.x.max(b.x).max(c.x).ceil() as i32).min(clip.x1);
    let miny = (a.y.min(b.y).min(c.y).floor() as i32).max(clip.y0);
    let maxy = (a.y.max(b.y).max(c.y).ceil() as i32).min(clip.y1);
    if minx >= maxx || miny >= maxy {
        return;
    }
    let tl = |p: &SV, q: &SV| -> bool {
        let (dx, dy) = (q.x - p.x, q.y - p.y);
        (dy == 0.0 && dx > 0.0) || dy < 0.0
    };
    let (t0, t1, t2) = (tl(&b, &c), tl(&c, &a), tl(&a, &b));
    let inv = 1.0 / area;
    let w = target.w as usize;
    let sampler = tex.map(|t| Sampler { tex: t, bilinear: st.bilinear, repeat: st.repeat });
    let (tw, th) = tex.map(|t| (t.w as f32, t.h as f32)).unwrap_or((1.0, 1.0));
    let inside = |w: f32, t: bool| w > 0.0 || (w == 0.0 && t);
    for y in miny..maxy {
        let py = y as f32 + 0.5;
        for x in minx..maxx {
            let px = x as f32 + 0.5;
            let w0 = edge(b.x, b.y, c.x, c.y, px, py);
            let w1 = edge(c.x, c.y, a.x, a.y, px, py);
            let w2 = edge(a.x, a.y, b.x, b.y, px, py);
            if !(inside(w0, t0) && inside(w1, t1) && inside(w2, t2)) {
                continue;
            }
            let (l0, l1, l2) = (w0 * inv, w1 * inv, w2 * inv);
            let z = a.z * l0 + b.z * l1 + c.z * l2;
            let idx = y as usize * w + x as usize;
            if st.depth_test && !(z <= depth[idx]) {
                continue;
            }
            let iw = a.iw * l0 + b.iw * l1 + c.iw * l2;
            let ww = 1.0 / iw;
            let at = |k: usize| (a.a[k] * l0 + b.a[k] * l1 + c.a[k] * l2) * ww;
            let mut col = [at(2), at(3), at(4), at(5)];
            if let Some(s) = &sampler {
                let t = s.sample(at(0) * tw, at(1) * th);
                col = [col[0] * t[0], col[1] * t[1], col[2] * t[2], col[3] * t[3]];
            }
            if let Some(r) = st.alpha_test {
                if col[3] <= r {
                    continue;
                }
            }
            if let Some(fc) = st.fog {
                let f = at(6).clamp(0.0, 1.0);
                for k in 0..3 {
                    col[k] = col[k] * (1.0 - f) + fc[k] * f;
                }
            }
            if st.depth_write {
                depth[idx] = z;
            }
            let dst = &mut target.px[idx];
            if st.mask == [true; 4] {
                blend_pixel(dst, col, st.blend);
            } else {
                let old = *dst;
                blend_pixel(dst, col, st.blend);
                for k in 0..4 {
                    if !st.mask[k] {
                        dst[k] = old[k];
                    }
                }
            }
        }
    }
}

impl Backend for SoftBackend {
    fn name(&self) -> &'static str {
        "software"
    }

    fn resize_back(&mut self, w: u32, h: u32) {
        self.back = Canvas::new(w, h);
        self.depth.remove(&Target::Back);
    }

    fn back_size(&self) -> (u32, u32) {
        (self.back.w, self.back.h)
    }

    fn surface_create(&mut self, id: i32, w: u32, h: u32) {
        self.surfaces.insert(id, Canvas::new(w, h));
        self.depth.remove(&Target::Surface(id));
    }

    fn surface_free(&mut self, id: i32) {
        self.surfaces.remove(&id);
        self.depth.remove(&Target::Surface(id));
        if self.target == Target::Surface(id) {
            self.target = Target::Back;
        }
    }

    fn set_target(&mut self, t: Target) {
        self.target = if self.canvas(t).is_some() { t } else { Target::Back };
    }

    fn target(&self) -> Target {
        self.target
    }

    fn clear(&mut self, clip: Clip, c: [u8; 4], depth: bool) {
        let t = self.target;
        if depth {
            self.clear_depth();
        }
        let Some(cv) = self.canvas_mut(t) else { return };
        let clip = clip.intersect(Clip::full(cv));
        let w = cv.w as i32;
        for y in clip.y0..clip.y1 {
            let row = (y * w) as usize;
            cv.px[row + clip.x0 as usize..row + clip.x1 as usize].fill(c);
        }
    }

    fn clear_depth(&mut self) {
        if let Some(d) = self.depth.get_mut(&self.target) {
            d.fill(1.0);
        }
    }

    fn draw(&mut self, st: &RState, tex: Option<&Tex>, prim: Prim, v: &[Vertex]) {
        let tgt = self.target;
        // resolve the texture first (it may be a surface, even the target)
        let owned: Option<Texture> = match tex {
            Some(Tex::Surf { id, .. }) => match self.surfaces.get(id) {
                Some(c) => Some(c.clone()),
                None => return,
            },
            _ => None,
        };
        let img: Option<&Texture> = match tex {
            Some(Tex::Img(i)) => Some(&**i),
            Some(Tex::Surf { .. }) => owned.as_ref(),
            None => None,
        };
        if let Some(t) = img {
            if t.w == 0 || t.h == 0 {
                return;
            }
        }
        let plain = st.is_plain() && v.iter().all(|p| p.w == 1.0);
        if plain {
            let Some(cv) = self.canvas_mut(tgt) else { return };
            Self::draw_2d(cv, st, img, prim, v);
            return;
        }
        // 3D path
        let (cw, ch) = match self.canvas(tgt) {
            Some(c) => (c.w, c.h),
            None => return,
        };
        let mut depth = self.depth.remove(&tgt).unwrap_or_else(|| vec![1.0; (cw * ch) as usize]);
        if depth.len() != (cw * ch) as usize {
            depth = vec![1.0; (cw * ch) as usize];
        }
        {
            let cv = match tgt {
                Target::Back => &mut self.back,
                Target::Surface(id) => self.surfaces.get_mut(&id).unwrap(),
            };
            let mut tri = |p: &[Vertex]| {
                let poly = clip_near(p);
                if poly.len() < 3 {
                    return;
                }
                let s: Vec<SV> = poly.iter().map(to_sv).collect();
                for i in 1..s.len() - 1 {
                    tri_3d(cv, &mut depth, st, img, [s[0], s[i], s[i + 1]]);
                }
            };
            match prim {
                Prim::Triangles => v.chunks_exact(3).for_each(&mut tri),
                Prim::Quads => v.chunks_exact(4).for_each(&mut tri),
                Prim::Lines | Prim::Points => {
                    let mut st2 = *st;
                    st2.cull = Cull::None;
                    let step = if prim == Prim::Lines { 2 } else { 1 };
                    for l in v.chunks_exact(step) {
                        if l.iter().any(|p| p.w <= 1e-5) {
                            continue;
                        }
                        let d = |p: &Vertex| Vertex { x: p.x / p.w, y: p.y / p.w, z: p.z / p.w, w: 1.0, ..*p };
                        let q = if step == 2 { line_quad(&d(&l[0]), &d(&l[1])) } else { line_quad(&d(&l[0]), &d(&l[0])) };
                        let s: Vec<SV> = q.iter().map(to_sv).collect();
                        tri_3d(cv, &mut depth, &st2, img, [s[0], s[1], s[2]]);
                        tri_3d(cv, &mut depth, &st2, img, [s[0], s[2], s[3]]);
                    }
                }
            }
        }
        self.depth.insert(tgt, depth);
    }

    fn read(&mut self, t: Target, x: i32, y: i32, w: u32, h: u32) -> Texture {
        let mut out = Texture::new(w, h);
        if let Some(c) = self.canvas(t) {
            out.blit(c, x, y, w as i32, h as i32, 0, 0);
        }
        out
    }

    fn write(&mut self, t: Target, x: i32, y: i32, img: &Texture) {
        if let Some(c) = self.canvas_mut(t) {
            c.blit(img, 0, 0, img.w as i32, img.h as i32, x, y);
        }
    }

    fn pixel(&mut self, t: Target, x: i32, y: i32) -> [u8; 4] {
        match self.canvas(t) {
            Some(c) if x >= 0 && y >= 0 && x < c.w as i32 && y < c.h as i32 => c.get(x as u32, y as u32),
            _ => [0; 4],
        }
    }

    fn read_back(&mut self) -> Texture {
        self.back.clone()
    }

    fn supports_3d(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Blend;

    #[test]
    fn depth_test_hides_far_triangle() {
        let mut b = SoftBackend::new(4, 4);
        let mut st = RState::new_2d(Blend::NORMAL, Clip { x0: 0, y0: 0, x1: 4, y1: 4 }, false, false);
        st.depth_test = true;
        st.depth_write = true;
        let q = |z: f32, c: [f32; 4]| {
            let p = |x: f32, y: f32| Vertex { x, y, z, w: 1.0, c, ..Default::default() };
            [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)]
        };
        b.clear(Clip { x0: 0, y0: 0, x1: 4, y1: 4 }, [0, 0, 0, 255], true);
        b.draw(&st, None, Prim::Quads, &q(0.2, [1.0, 0.0, 0.0, 1.0]));
        b.draw(&st, None, Prim::Quads, &q(0.5, [0.0, 1.0, 0.0, 1.0]));
        assert_eq!(b.pixel(Target::Back, 1, 1), [255, 0, 0, 255]);
    }
}
