//! 3D state and geometry (specs/d3d.md): mode flags, projection, world
//! transform and stack, fog, lighting, primitives, shapes and models.
//!
//! Matrices here use column vectors (`p' = M·p`, see `draw::mat`); the spec's
//! row-vector matrices are the transposes. The whole pipeline (world →
//! view → projection → viewport) runs on the CPU and produces vertices in pixel
//! homogeneous space for the backends; lighting is per vertex.

use crate::draw::backend::{Backend, Cull, Prim, RState, Tex, Vertex};
use crate::draw::mat::{self, M4, IDENT};
use crate::draw::painter::{Pipe, Xf};
use crate::draw::rgba;
use std::collections::BTreeMap;
use std::rc::Rc;

/// A light (§8.2).
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub point: bool,
    /// Direction (directional) or position (point).
    pub v: [f64; 3],
    pub range: f64,
    pub col: i32,
    pub enabled: bool,
    pub defined: bool,
}

/// The projection of the current pass (§5).
#[derive(Clone, Copy, Debug)]
pub enum Proj {
    /// Default for the pass (perspective/ortho in 3D, the 2D view mapping otherwise).
    Default,
    /// `d3d_set_projection(_ext)`: view and projection matrices.
    Look { view: M4, proj: M4 },
    Ortho { x: f64, y: f64, w: f64, h: f64, angle: f64 },
    Persp { x: f64, y: f64, w: f64, h: f64, angle: f64 },
}

/// The current pass: view rectangle and port.
#[derive(Clone, Copy, Debug)]
pub struct Pass {
    pub vx: f64,
    pub vy: f64,
    pub vw: f64,
    pub vh: f64,
    pub angle: f64,
    pub px: f64,
    pub py: f64,
    pub pw: f64,
    pub ph: f64,
}

impl Default for Pass {
    fn default() -> Self {
        Pass { vx: 0.0, vy: 0.0, vw: 640.0, vh: 480.0, angle: 0.0, px: 0.0, py: 0.0, pw: 640.0, ph: 480.0 }
    }
}

/// A recorded vertex: position, texture coordinate, colour (None = draw colour
/// at draw time) and normal.
#[derive(Clone, Copy, Debug, Default)]
pub struct MV {
    pub p: [f64; 3],
    pub uv: [f64; 2],
    pub col: Option<(i32, f64)>,
    pub n: [f64; 3],
}

/// A model command (§9); also the file representation.
#[derive(Clone, Debug)]
pub enum Cmd {
    Begin(i32),
    End,
    /// Vertex with file kind 2..9 and its values.
    Vertex(u8, MV),
    /// Shape kind 10..15: x1 y1 z1 x2 y2 z2 hrep vrep closed steps.
    Shape(u8, [f64; 10]),
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub cmds: Vec<Cmd>,
    /// Compiled primitives: (kind, vertices).
    cache: Option<Rc<Vec<(i32, Vec<MV>)>>>,
}

impl Model {
    pub fn push(&mut self, c: Cmd) {
        self.cmds.push(c);
        self.cache = None;
    }
    pub fn clear(&mut self) {
        self.cmds.clear();
        self.cache = None;
    }

    /// Tessellated primitives (cached).
    pub fn compiled(&mut self) -> Rc<Vec<(i32, Vec<MV>)>> {
        if let Some(c) = &self.cache {
            return c.clone();
        }
        let mut out: Vec<(i32, Vec<MV>)> = Vec::new();
        let mut cur: Option<(i32, Vec<MV>)> = None;
        for c in &self.cmds {
            match c {
                Cmd::Begin(k) => cur = Some((*k, Vec::new())),
                Cmd::End => {
                    if let Some(p) = cur.take() {
                        out.push(p);
                    }
                }
                Cmd::Vertex(_, v) => {
                    if let Some(p) = cur.as_mut() {
                        p.1.push(*v);
                    }
                }
                Cmd::Shape(k, a) => {
                    let tris = shape(*k, a);
                    // merge consecutive shape triangles into one list
                    match out.last_mut() {
                        Some((4, l)) if l.len() % 3 == 0 => l.extend(tris),
                        _ => out.push((4, tris)),
                    }
                }
            }
        }
        let rc = Rc::new(out);
        self.cache = Some(rc.clone());
        rc
    }

    /// Text form (§9.2).
    pub fn to_text(&self) -> String {
        let mut s = String::from("100\r\n");
        s.push_str(&format!("{}\r\n", self.cmds.len()));
        let f = |x: f64| format!("{:.4}", x);
        for c in &self.cmds {
            let (k, vals): (u8, [f64; 10]) = match c {
                Cmd::Begin(k) => (0, [*k as f64, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
                Cmd::End => (1, [0.0; 10]),
                Cmd::Vertex(k, v) => {
                    let (col, al) = v.col.map(|(c, a)| (c as f64, a)).unwrap_or((0.0, 1.0));
                    let [x, y, z] = v.p;
                    let [nx, ny, nz] = v.n;
                    let [u, vv] = v.uv;
                    let vals = match k {
                        2 => [x, y, z, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                        3 => [x, y, z, col, al, 0.0, 0.0, 0.0, 0.0, 0.0],
                        4 => [x, y, z, u, vv, 0.0, 0.0, 0.0, 0.0, 0.0],
                        5 => [x, y, z, u, vv, col, al, 0.0, 0.0, 0.0],
                        6 => [x, y, z, nx, ny, nz, 0.0, 0.0, 0.0, 0.0],
                        7 => [x, y, z, nx, ny, nz, col, al, 0.0, 0.0],
                        8 => [x, y, z, nx, ny, nz, u, vv, 0.0, 0.0],
                        _ => [x, y, z, nx, ny, nz, u, vv, col, al],
                    };
                    (*k, vals)
                }
                Cmd::Shape(k, a) => (*k, *a),
            };
            s.push_str(&k.to_string());
            for v in vals {
                s.push(' ');
                s.push_str(&f(v));
            }
            s.push_str("\r\n");
        }
        s
    }

    /// Parses the text form; None on a malformed file.
    pub fn from_text(s: &str) -> Option<Model> {
        let mut nums = s.split_ascii_whitespace().map(|t| t.parse::<f64>().ok());
        let ver = nums.next()??;
        if ver != 100.0 {
            return None;
        }
        let n = nums.next()?? as usize;
        let mut m = Model::default();
        for _ in 0..n {
            let k = nums.next()?? as i64;
            let mut a = [0.0; 10];
            for x in a.iter_mut() {
                *x = nums.next().flatten().unwrap_or(0.0);
            }
            let ci = |v: f64| v as i64 as i32;
            let cmd = match k {
                0 => Cmd::Begin(ci(a[0])),
                1 => Cmd::End,
                2..=9 => {
                    let mut v = MV { p: [a[0], a[1], a[2]], ..Default::default() };
                    match k {
                        3 => v.col = Some((ci(a[3]), a[4])),
                        4 => v.uv = [a[3], a[4]],
                        5 => {
                            v.uv = [a[3], a[4]];
                            v.col = Some((ci(a[5]), a[6]));
                        }
                        6 => v.n = [a[3], a[4], a[5]],
                        7 => {
                            v.n = [a[3], a[4], a[5]];
                            v.col = Some((ci(a[6]), a[7]));
                        }
                        8 => {
                            v.n = [a[3], a[4], a[5]];
                            v.uv = [a[6], a[7]];
                        }
                        9 => {
                            v.n = [a[3], a[4], a[5]];
                            v.uv = [a[6], a[7]];
                            v.col = Some((ci(a[8]), a[9]));
                        }
                        _ => {}
                    }
                    Cmd::Vertex(k as u8, v)
                }
                10..=15 => Cmd::Shape(k as u8, a),
                _ => return None,
            };
            m.cmds.push(cmd);
        }
        Some(m)
    }
}

/// All 3D state.
pub struct D3d {
    pub on: bool,
    pub hidden: bool,
    pub zwrite: bool,
    pub perspective: bool,
    pub culling: bool,
    pub lighting: bool,
    pub smooth: bool,
    pub fog_on: bool,
    pub fog_col: i32,
    pub fog_start: f64,
    pub fog_end: f64,
    pub ambient: i32,
    pub lights: BTreeMap<i32, Light>,
    /// World transform (column-vector convention).
    pub world: M4,
    pub stack: Vec<M4>,
    /// z of 2D drawing.
    pub depth: f64,
    pub proj: Proj,
    /// Pass projection saved while a surface is the target.
    pub saved_proj: Option<Proj>,
    pub pass: Pass,
    pub models: Vec<Option<Model>>,
    /// Primitive under construction: kind, texture id, vertices.
    pub prim: Option<(i32, i32, Vec<MV>)>,
    /// `texture_set_blending`.
    pub tex_blend: bool,
    /// Warned that the backend has no 3D support.
    pub warned: bool,
    /// Depth test/write temporarily off (room backgrounds, cursor).
    pub no_depth: bool,
}

impl Default for D3d {
    fn default() -> Self {
        D3d {
            on: false,
            hidden: false,
            zwrite: true,
            perspective: true,
            culling: false,
            lighting: false,
            smooth: true,
            fog_on: false,
            fog_col: 0,
            fog_start: 0.0,
            fog_end: 0.0,
            ambient: 0,
            lights: BTreeMap::new(),
            world: IDENT,
            stack: Vec::new(),
            depth: 0.0,
            proj: Proj::Default,
            saved_proj: None,
            pass: Pass::default(),
            models: Vec::new(),
            prim: None,
            tex_blend: true,
            warned: false,
            no_depth: false,
        }
    }
}

pub const STACK_MAX: usize = 32;

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt();
    if l < 1e-12 {
        [0.0; 3]
    } else {
        [a[0] / l, a[1] / l, a[2] / l]
    }
}

/// Left-handed look-at view matrix (column convention), None if degenerate.
pub fn look_at(from: [f64; 3], to: [f64; 3], up: [f64; 3]) -> Option<M4> {
    let z = norm(sub(to, from));
    if z == [0.0; 3] {
        return None;
    }
    let x = norm(cross(up, z));
    if x == [0.0; 3] {
        return None;
    }
    let y = cross(z, x);
    Some([
        [x[0], x[1], x[2], -dot(x, from)],
        [y[0], y[1], y[2], -dot(y, from)],
        [z[0], z[1], z[2], -dot(z, from)],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

/// Left-handed perspective projection (column convention); fov in degrees.
pub fn perspective(fov: f64, aspect: f64, zn: f64, zf: f64) -> M4 {
    let ys = 1.0 / (fov.to_radians() / 2.0).tan();
    let xs = ys / if aspect != 0.0 { aspect } else { 1.0 };
    let q = zf / (zf - zn);
    [[xs, 0.0, 0.0, 0.0], [0.0, ys, 0.0, 0.0], [0.0, 0.0, q, -zn * q], [0.0, 0.0, 1.0, 0.0]]
}

/// NDC → pixel-homogeneous viewport mapping for a port.
pub fn viewport(px: f64, py: f64, pw: f64, ph: f64) -> M4 {
    [
        [pw / 2.0, 0.0, 0.0, px + pw / 2.0],
        [0.0, -ph / 2.0, 0.0, py + ph / 2.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Room → pixel orthographic mapping (2D view) with z ∈ [−32000, 32000] → [0, 1].
fn ortho(x: f64, y: f64, w: f64, h: f64, angle: f64, port: (f64, f64, f64, f64)) -> M4 {
    let xf = Xf::view(x, y, w, h, angle, port.0, port.1, port.2, port.3);
    let mut m = xf.to_m4();
    m[2] = [0.0, 0.0, 1.0 / 64000.0, 0.5];
    m
}

/// Default-style perspective for a rectangle: (view, proj).
fn persp_rect(x: f64, y: f64, w: f64, h: f64, angle: f64) -> (M4, M4) {
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let t = angle.to_radians();
    let view = look_at([cx, cy, -w], [cx, cy, 0.0], [-t.sin(), t.cos(), 0.0]).unwrap_or(IDENT);
    let fov = (2.0 * (h / (2.0 * w)).atan()).to_degrees();
    (view, perspective(fov, if h != 0.0 { w / h } else { 1.0 }, 1.0, 32000.0))
}

impl D3d {
    /// Whether drawing needs the full matrix pipeline.
    pub fn active(&self) -> bool {
        self.on || !mat::is_identity(&self.world) || !matches!(self.proj, Proj::Default)
    }

    /// Room → pixel-homogeneous matrix and the room → view-depth row, for the
    /// given target port. None = plain 2D mapping by `xf`.
    fn matrices(&self, xf: Xf, size: (u32, u32), surf: bool) -> Option<(M4, Option<[f64; 4]>)> {
        if !self.active() {
            return None;
        }
        let port = if surf { (0.0, 0.0, size.0 as f64, size.1 as f64) } else { (self.pass.px, self.pass.py, self.pass.pw, self.pass.ph) };
        let p = &self.pass;
        let (vp, view): (M4, Option<M4>) = match self.proj {
            Proj::Default => {
                if surf {
                    (ortho(0.0, 0.0, size.0 as f64, size.1 as f64, 0.0, port), None)
                } else if !self.on {
                    let mut m = xf.to_m4();
                    m[2] = [0.0, 0.0, 0.0, 0.5];
                    (m, None)
                } else if self.perspective {
                    let (v, pr) = persp_rect(p.vx, p.vy, p.vw, p.vh, p.angle);
                    (mat::mul(&mat::mul(&viewport(port.0, port.1, port.2, port.3), &pr), &v), Some(v))
                } else {
                    (ortho(p.vx, p.vy, p.vw, p.vh, p.angle, port), None)
                }
            }
            Proj::Ortho { x, y, w, h, angle } => (ortho(x, y, w, h, angle, port), None),
            Proj::Persp { x, y, w, h, angle } => {
                let (v, pr) = persp_rect(x, y, w, h, angle);
                (mat::mul(&mat::mul(&viewport(port.0, port.1, port.2, port.3), &pr), &v), Some(v))
            }
            Proj::Look { view, proj } => (mat::mul(&mat::mul(&viewport(port.0, port.1, port.2, port.3), &proj), &view), Some(view)),
        };
        let m = mat::mul(&vp, &self.world);
        let fog_row = if self.on && self.fog_on {
            let vw = match view {
                Some(v) => mat::mul(&v, &self.world),
                None => self.world,
            };
            Some(vw[2])
        } else {
            None
        };
        Some((m, fog_row))
    }

    /// Fills the pipe and render state for 2D drawing.
    pub fn configure(&self, pipe: &mut Pipe, st: &mut RState, xf: Xf, size: (u32, u32), surf: bool) {
        let Some((m, fog_row)) = self.matrices(xf, size, surf) else { return };
        pipe.m = Some(m);
        pipe.z = self.depth;
        if self.on {
            st.depth_test = self.hidden && !self.no_depth;
            st.depth_write = self.hidden && self.zwrite && !self.no_depth;
            // 2D-emitted geometry is never culled (deviation from d3d.md §3.2:
            // under the 2D-oriented ortho projection every sprite/rectangle is
            // clockwise and HUD overlays would vanish); see `cull_state`.
            if let Some(row) = fog_row {
                let c = rgba(self.fog_col, 1.0);
                st.fog = Some([c[0], c[1], c[2]]);
                pipe.fog = Some((row, self.fog_start, self.fog_end));
            }
            if self.lighting {
                let a = rgba(self.ambient, 1.0);
                pipe.tint = Some([a[0], a[1], a[2]]);
            }
        }
    }

    /// Culling for 3D primitives, shapes and models.
    pub fn cull_state(&self) -> Cull {
        if self.on && self.culling {
            Cull::KeepCcw
        } else {
            Cull::None
        }
    }

    /// Lit colour of a vertex (world-space position `wp` and normal `wn`).
    fn light(&self, c: [f32; 4], wp: [f64; 3], wn: [f64; 3]) -> [f32; 4] {
        let a = rgba(self.ambient, 1.0);
        let mut sum = [a[0] as f64, a[1] as f64, a[2] as f64];
        if wn != [0.0; 3] {
            for l in self.lights.values().filter(|l| l.enabled && l.defined).take(8) {
                let (dir, att) = if l.point {
                    let d = sub(l.v, wp);
                    let dist = dot(d, d).sqrt();
                    if dist >= l.range || l.range <= 0.0 {
                        continue;
                    }
                    (norm(d), 1.0 - dist / l.range)
                } else {
                    (norm([-l.v[0], -l.v[1], -l.v[2]]), 1.0)
                };
                let k = dot(wn, dir).max(0.0) * att;
                if k > 0.0 {
                    let lc = rgba(l.col, 1.0);
                    for i in 0..3 {
                        sum[i] += lc[i] as f64 * k;
                    }
                }
            }
        }
        [
            c[0] * sum[0].clamp(0.0, 1.0) as f32,
            c[1] * sum[1].clamp(0.0, 1.0) as f32,
            c[2] * sum[2].clamp(0.0, 1.0) as f32,
            c[3],
        ]
    }

    /// Draws recorded vertices as primitive `kind` with an optional texture
    /// (`uv_scale` maps 0..1 padded texture coordinates onto the image).
    /// `offset` is added in model space; `draw_col` colours vertices without
    /// their own colour.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_mesh(
        &self,
        be: &mut dyn Backend,
        pipe: &Pipe,
        st: &RState,
        kind: i32,
        tex: Option<&(Tex, f32, f32)>,
        verts: &[MV],
        offset: [f64; 3],
        draw_col: (i32, f64),
    ) {
        if verts.is_empty() {
            return;
        }
        let lit = self.on && self.lighting;
        let nmat = if lit { mat::invert(&self.world).map(|m| mat::transpose(&m)).unwrap_or(self.world) } else { IDENT };
        let (su, sv) = tex.map(|t| (t.1, t.2)).unwrap_or((1.0, 1.0));
        let modulate = self.tex_blend || tex.is_none();
        let out: Vec<Vertex> = verts
            .iter()
            .map(|v| {
                let (x, y, z) = (v.p[0] + offset[0], v.p[1] + offset[1], v.p[2] + offset[2]);
                let (col, al) = v.col.unwrap_or(draw_col);
                let mut c = if modulate { rgba(col, al) } else { [1.0, 1.0, 1.0, 1.0] };
                if lit {
                    let wp = mat::point(&self.world, x, y, z);
                    let wn = norm(mat::dir(&nmat, v.n[0], v.n[1], v.n[2]));
                    c = self.light(c, wp, wn);
                }
                let mut p = *pipe;
                p.tint = None;
                p.vert3(x, y, z, v.uv[0] as f32 * su, v.uv[1] as f32 * sv, c)
            })
            .collect();
        let t = tex.map(|t| &t.0);
        emit(be, st, t, kind, &out, self.on && !self.smooth);
    }
}

/// Assembles a GM primitive kind into backend primitives.
pub fn emit(be: &mut dyn Backend, st: &RState, tex: Option<&Tex>, kind: i32, v: &[Vertex], flat: bool) {
    let n = v.len();
    let mut tris: Vec<Vertex> = Vec::new();
    let mut push = |a: &Vertex, b: &Vertex, c: &Vertex| {
        if flat {
            tris.push(*a);
            tris.push(Vertex { c: a.c, ..*b });
            tris.push(Vertex { c: a.c, ..*c });
        } else {
            tris.push(*a);
            tris.push(*b);
            tris.push(*c);
        }
    };
    match kind {
        1 => be.draw(st, tex, Prim::Points, v),
        2 => {
            let m = n / 2 * 2;
            be.draw(st, tex, Prim::Lines, &v[..m]);
        }
        3 => {
            let mut l = Vec::with_capacity(n * 2);
            for i in 1..n {
                l.push(v[i - 1]);
                l.push(v[i]);
            }
            be.draw(st, tex, Prim::Lines, &l);
        }
        4 => {
            for t in v.chunks_exact(3) {
                push(&t[0], &t[1], &t[2]);
            }
            be.draw(st, tex, Prim::Triangles, &tris);
        }
        5 => {
            for i in 2..n {
                if i % 2 == 0 {
                    push(&v[i - 2], &v[i - 1], &v[i]);
                } else {
                    // odd triangles of a strip keep the strip's winding
                    push(&v[i - 1], &v[i - 2], &v[i]);
                }
            }
            be.draw(st, tex, Prim::Triangles, &tris);
        }
        6 => {
            for i in 2..n {
                push(&v[0], &v[i - 1], &v[i]);
            }
            be.draw(st, tex, Prim::Triangles, &tris);
        }
        _ => {}
    }
}

/// A triangle of a shape whose front faces direction `n` (§3.2: front faces
/// appear counter-clockwise; in the left-handed view space that means the
/// geometric normal (b−a)×(c−a) points away from the viewer).
fn tri(out: &mut Vec<MV>, a: MV, b: MV, c: MV, front: [f64; 3]) {
    let g = cross(sub(b.p, a.p), sub(c.p, a.p));
    if dot(g, front) > 0.0 {
        out.extend([a, c, b]);
    } else {
        out.extend([a, b, c]);
    }
}

fn mv(p: [f64; 3], uv: [f64; 2], n: [f64; 3]) -> MV {
    MV { p, uv, col: None, n }
}

fn quad(out: &mut Vec<MV>, q: [MV; 4], front: [f64; 3]) {
    tri(out, q[0], q[1], q[2], front);
    tri(out, q[0], q[2], q[3], front);
}

/// Tessellates a shape (kind 10..15, args as in the model file) into a
/// triangle list (§4).
pub fn shape(kind: u8, a: &[f64; 10]) -> Vec<MV> {
    let [x1, y1, z1, x2, y2, z2, hr, vr, closed, steps] = *a;
    let mut o = Vec::new();
    let steps = (steps as i64).clamp(3, 128) as usize;
    let closed = closed >= 0.5;
    let (cx, cy, cz) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0, (z1 + z2) / 2.0);
    let (rx, ry, rz) = ((x2 - x1) / 2.0, (y2 - y1) / 2.0, (z2 - z1) / 2.0);
    // outward direction sign per axis (mirrored boxes stay outward)
    let sx = if x2 >= x1 { 1.0 } else { -1.0 };
    let sy = if y2 >= y1 { 1.0 } else { -1.0 };
    let sz = if z2 >= z1 { 1.0 } else { -1.0 };
    match kind {
        10 => {
            // block: sides (v from z2 to z1)
            let side = |o: &mut Vec<MV>, p1: [f64; 2], p2: [f64; 2], n: [f64; 3]| {
                let q = [
                    mv([p1[0], p1[1], z2], [0.0, 0.0], n),
                    mv([p2[0], p2[1], z2], [hr, 0.0], n),
                    mv([p2[0], p2[1], z1], [hr, vr], n),
                    mv([p1[0], p1[1], z1], [0.0, vr], n),
                ];
                quad(o, q, n);
            };
            side(&mut o, [x1, y1], [x2, y1], [0.0, -sy, 0.0]);
            side(&mut o, [x2, y1], [x2, y2], [sx, 0.0, 0.0]);
            side(&mut o, [x2, y2], [x1, y2], [0.0, sy, 0.0]);
            side(&mut o, [x1, y2], [x1, y1], [-sx, 0.0, 0.0]);
            for (z, n) in [(z1, [0.0, 0.0, -sz]), (z2, [0.0, 0.0, sz])] {
                let q = [
                    mv([x1, y1, z], [0.0, 0.0], n),
                    mv([x2, y1, z], [hr, 0.0], n),
                    mv([x2, y2, z], [hr, vr], n),
                    mv([x1, y2, z], [0.0, vr], n),
                ];
                quad(&mut o, q, n);
            }
        }
        11 | 12 => {
            let cone = kind == 12;
            let ring: Vec<(f64, f64, f64)> = (0..=steps)
                .map(|i| {
                    let t = std::f64::consts::TAU * i as f64 / steps as f64;
                    (t.cos(), t.sin(), i as f64 / steps as f64)
                })
                .collect();
            let rmean = (rx.abs() + ry.abs()) / 2.0;
            let k = if cone && (z2 - z1).abs() > 1e-12 { rmean / (z2 - z1).abs() } else { 0.0 };
            for w in ring.windows(2) {
                let (c0, s0, f0) = w[0];
                let (c1, s1, f1) = w[1];
                // gradient of the ellipse: outward whatever the signs of rx, ry
                let nrm = |c: f64, s: f64| norm([if rx != 0.0 { c / rx } else { 0.0 }, if ry != 0.0 { s / ry } else { 0.0 }, 0.0]);
                let (n0, n1) = (nrm(c0, s0), nrm(c1, s1));
                let p = |c: f64, s: f64, z: f64| [cx + rx * c, cy + ry * s, z];
                if cone {
                    let mut n0c = n0;
                    let mut n1c = n1;
                    n0c[2] = k * sz;
                    n1c[2] = k * sz;
                    let (n0c, n1c) = (norm(n0c), norm(n1c));
                    let nm = norm([(n0c[0] + n1c[0]) / 2.0, (n0c[1] + n1c[1]) / 2.0, (n0c[2] + n1c[2]) / 2.0]);
                    let apex = mv([cx, cy, z2], [hr * (f0 + f1) / 2.0, 0.0], nm);
                    let a = mv(p(c0, s0, z1), [hr * f0, vr], n0c);
                    let b = mv(p(c1, s1, z1), [hr * f1, vr], n1c);
                    tri(&mut o, apex, a, b, nm);
                } else {
                    let q = [
                        mv(p(c0, s0, z2), [hr * f0, 0.0], n0),
                        mv(p(c1, s1, z2), [hr * f1, 0.0], n1),
                        mv(p(c1, s1, z1), [hr * f1, vr], n1),
                        mv(p(c0, s0, z1), [hr * f0, vr], n0),
                    ];
                    let nm = norm([(n0[0] + n1[0]) / 2.0, (n0[1] + n1[1]) / 2.0, 0.0]);
                    quad(&mut o, q, nm);
                }
            }
            if closed {
                let caps: Vec<(f64, [f64; 3])> = if cone { vec![(z1, [0.0, 0.0, -sz])] } else { vec![(z1, [0.0, 0.0, -sz]), (z2, [0.0, 0.0, sz])] };
                for (z, n) in caps {
                    let pl = |x: f64, y: f64| {
                        [
                            if x2 != x1 { hr * (x - x1) / (x2 - x1) } else { 0.0 },
                            if y2 != y1 { vr * (y - y1) / (y2 - y1) } else { 0.0 },
                        ]
                    };
                    let c = mv([cx, cy, z], pl(cx, cy), n);
                    for w in ring.windows(2) {
                        let (pa, pb) = ((cx + rx * w[0].0, cy + ry * w[0].1), (cx + rx * w[1].0, cy + ry * w[1].1));
                        tri(&mut o, c, mv([pa.0, pa.1, z], pl(pa.0, pa.1), n), mv([pb.0, pb.1, z], pl(pb.0, pb.1), n), n);
                    }
                }
            }
        }
        13 => {
            let rings = (steps / 2).max(2);
            let pt = |i: usize, j: usize| -> MV {
                let t = std::f64::consts::TAU * i as f64 / steps as f64;
                let ph = std::f64::consts::PI * j as f64 / rings as f64; // 0 at z2 pole
                let (sp, cp) = (ph.sin(), ph.cos());
                let p = [cx + rx * sp * t.cos(), cy + ry * sp * t.sin(), cz + rz * cp];
                let inv = |r: f64| if r.abs() > 1e-12 { 1.0 / (r * r) } else { 0.0 };
                let n = norm([(p[0] - cx) * inv(rx), (p[1] - cy) * inv(ry), (p[2] - cz) * inv(rz)]);
                mv(p, [hr * i as f64 / steps as f64, vr * j as f64 / rings as f64], n)
            };
            for j in 0..rings {
                for i in 0..steps {
                    let (a, b, c, d) = (pt(i, j), pt(i + 1, j), pt(i + 1, j + 1), pt(i, j + 1));
                    let mid = norm([
                        a.n[0] + b.n[0] + c.n[0] + d.n[0],
                        a.n[1] + b.n[1] + c.n[1] + d.n[1],
                        a.n[2] + b.n[2] + c.n[2] + d.n[2],
                    ]);
                    if j > 0 {
                        tri(&mut o, a, b, c, mid);
                    }
                    if j + 1 < rings {
                        tri(&mut o, a, c, d, mid);
                    }
                }
            }
        }
        14 => {
            if x1 == x2 && y1 == y2 {
                return o;
            }
            let n = norm([y2 - y1, -(x2 - x1), 0.0]);
            let q = [
                mv([x1, y1, z1], [0.0, vr], n),
                mv([x2, y2, z1], [hr, vr], n),
                mv([x2, y2, z2], [hr, 0.0], n),
                mv([x1, y1, z2], [0.0, 0.0], n),
            ];
            quad(&mut o, q, n);
        }
        15 => {
            let g = cross(sub([x2, y1, z1], [x1, y1, z1]), sub([x2, y2, z2], [x1, y1, z1]));
            let mut n = norm(g);
            if n[2] < 0.0 || (n[2] == 0.0 && g == [0.0; 3]) {
                n = [-n[0], -n[1], -n[2]];
            }
            if n == [0.0; 3] {
                n = [0.0, 0.0, 1.0];
            }
            let q = [
                mv([x1, y1, z1], [0.0, 0.0], n),
                mv([x2, y1, z1], [hr, 0.0], n),
                mv([x2, y2, z2], [hr, vr], n),
                mv([x1, y2, z2], [0.0, vr], n),
            ];
            quad(&mut o, q, n);
        }
        _ => {}
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_text_roundtrip() {
        let mut m = Model::default();
        m.push(Cmd::Begin(4));
        m.push(Cmd::Vertex(3, MV { p: [1.0, 2.0, 3.0], col: Some((255, 0.5)), ..Default::default() }));
        m.push(Cmd::End);
        m.push(Cmd::Shape(10, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0]));
        let t = m.to_text();
        let m2 = Model::from_text(&t).unwrap();
        assert_eq!(m2.to_text(), t);
        assert!(Model::from_text("99\n0\n").is_none());
    }

    #[test]
    fn block_has_12_outward_triangles() {
        let t = shape(10, &[0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 1.0, 1.0, 0.0, 0.0]);
        assert_eq!(t.len(), 36);
        for tr in t.chunks(3) {
            let g = cross(sub(tr[1].p, tr[0].p), sub(tr[2].p, tr[0].p));
            // geometric normal points inward (front = counter-clockwise from outside)
            assert!(dot(g, tr[0].n) < 0.0);
        }
    }

    #[test]
    fn rotation_z_matches_direction_convention() {
        let r = mat::rotate_axis(0.0, 0.0, 1.0, -90.0);
        let p = mat::point(&r, 1.0, 0.0, 0.0);
        assert!((p[0]).abs() < 1e-9 && (p[1] + 1.0).abs() < 1e-9);
    }
}
