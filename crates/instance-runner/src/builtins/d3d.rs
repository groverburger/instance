//! Chapter 415: 3D graphics (specs/d3d.md).

use super::{bo, r, zero, Reg, A};
use crate::d3d::{look_at, perspective, shape, Cmd, Light, Model, Proj, MV, STACK_MAX};
use crate::draw::mat::{self, M4};
use crate::draw::{Target, Tex};
use crate::game::Game;
use gml::{Error, Value};

type R = Result<Value, Error>;

impl Game {
    /// Draws recorded vertices with the current state and draw colour.
    pub fn d3d_draw(&mut self, kind: i32, tex: Option<(Tex, f32, f32)>, verts: &[MV], offset: [f64; 3]) {
        let (pipe, mut st) = self.gfx.pipe_state();
        let gfx = &mut self.gfx;
        st.cull = gfx.d3d.cull_state();
        gfx.d3d.draw_mesh(&mut *gfx.be, &pipe, &st, kind, tex.as_ref(), verts, offset, (gfx.color, gfx.alpha));
    }

    fn model_mut(&mut self, id: i32) -> Result<&mut Model, Error> {
        self.gfx
            .d3d
            .models
            .get_mut(id.max(0) as usize)
            .filter(|_| id >= 0)
            .and_then(|m| m.as_mut())
            .ok_or_else(|| Error::new(format!("Model does not exist: {}", id)))
    }
}

/// Shape arguments (after the optional model id) as the model file's 10 values.
fn shape_args(a: &A, base: usize, kind: u8, textured: bool) -> Result<[f64; 10], Error> {
    let t = if textured { 1 } else { 0 };
    let mut v = [0.0; 10];
    for (i, x) in v.iter_mut().enumerate().take(6) {
        *x = a.r(base + i)?;
    }
    v[6] = a.r(base + 6 + t)?;
    v[7] = a.r(base + 7 + t)?;
    match kind {
        11 | 12 => {
            v[8] = a.r(base + 8 + t)?;
            v[9] = a.r(base + 9 + t)?;
        }
        13 => {
            v[9] = a.r(base + 8 + t)?;
        }
        _ => {}
    }
    Ok(v)
}

fn draw_shape(g: &mut Game, a: &A, kind: u8) -> R {
    let v = shape_args(a, 0, kind, true)?;
    let tex = g.gfx.texture(a.i(6)?);
    let tris = shape(kind, &v);
    g.d3d_draw(4, tex, &tris, [0.0; 3]);
    zero()
}

fn model_shape(g: &mut Game, a: &A, kind: u8) -> R {
    let v = shape_args(a, 1, kind, false)?;
    g.model_mut(a.i(0)?)?.push(Cmd::Shape(kind, v));
    zero()
}

/// Vertex of file kind `k` from arguments starting at `base`; `col` is the draw
/// colour used for colourless vertices (None for models).
fn vertex(a: &A, base: usize, k: u8, col: Option<(i32, f64)>) -> Result<MV, Error> {
    let f = |i: usize| a.r(base + i);
    let mut v = MV { p: [f(0)?, f(1)?, f(2)?], col, ..Default::default() };
    let colat = |i: usize| -> Result<Option<(i32, f64)>, Error> { Ok(Some((a.i(base + i)?, a.r(base + i + 1)?))) };
    match k {
        3 => v.col = colat(3)?,
        4 => v.uv = [f(3)?, f(4)?],
        5 => {
            v.uv = [f(3)?, f(4)?];
            v.col = colat(5)?;
        }
        6 => v.n = [f(3)?, f(4)?, f(5)?],
        7 => {
            v.n = [f(3)?, f(4)?, f(5)?];
            v.col = colat(6)?;
        }
        8 => {
            v.n = [f(3)?, f(4)?, f(5)?];
            v.uv = [f(6)?, f(7)?];
        }
        9 => {
            v.n = [f(3)?, f(4)?, f(5)?];
            v.uv = [f(6)?, f(7)?];
            v.col = colat(8)?;
        }
        _ => {}
    }
    Ok(v)
}

fn push_vertex(g: &mut Game, a: &A, k: u8) -> R {
    let col = Some((g.gfx.color, g.gfx.alpha));
    let v = vertex(a, 0, k, col)?;
    if let Some(p) = g.gfx.d3d.prim.as_mut() {
        p.2.push(v);
    }
    zero()
}

fn model_vertex(g: &mut Game, a: &A, k: u8) -> R {
    let v = vertex(a, 1, k, None)?;
    g.model_mut(a.i(0)?)?.push(Cmd::Vertex(k, v));
    zero()
}

/// Rotation about an axis with GM's sense (d3d.md §6.2).
fn rot(x: f64, y: f64, z: f64, deg: f64) -> M4 {
    mat::rotate_axis(x, y, z, -deg)
}

fn set_world(g: &mut Game, m: M4, add: bool) -> R {
    let d = &mut g.gfx.d3d;
    d.world = if add { mat::mul(&m, &d.world) } else { m };
    zero()
}

pub fn register(l: &mut Reg) {
    l.push(("d3d_start", |g, _, _, _| {
        let d = &mut g.gfx.d3d;
        d.on = true;
        d.hidden = true;
        d.perspective = true;
        d.zwrite = true;
        if !g.gfx.be.supports_3d() && !g.gfx.d3d.warned {
            eprintln!("instance-runner: the {} renderer does not support 3D", g.gfx.be.name());
            g.gfx.d3d.warned = true;
        }
        g.gfx.be.set_target(Target::Back);
        g.gfx.be.clear_depth();
        bo(true)
    }));
    l.push(("d3d_end", |g, _, _, _| {
        g.gfx.d3d.on = false;
        bo(true)
    }));
    l.push(("d3d_set_hidden", |g, _, _, a| {
        g.gfx.d3d.hidden = a.b(0)?;
        zero()
    }));
    l.push(("d3d_set_perspective", |g, _, _, a| {
        g.gfx.d3d.perspective = a.b(0)?;
        zero()
    }));
    l.push(("d3d_set_zwriteenable", |g, _, _, a| {
        g.gfx.d3d.zwrite = a.b(0)?;
        zero()
    }));
    l.push(("d3d_set_depth", |g, _, _, a| {
        g.gfx.d3d.depth = a.r(0)?;
        zero()
    }));
    l.push(("d3d_set_culling", |g, _, _, a| {
        g.gfx.d3d.culling = a.b(0)?;
        zero()
    }));
    l.push(("d3d_set_lighting", |g, _, _, a| {
        g.gfx.d3d.lighting = a.b(0)?;
        zero()
    }));
    l.push(("d3d_set_shading", |g, _, _, a| {
        g.gfx.d3d.smooth = a.b(0)?;
        zero()
    }));
    l.push(("d3d_set_fog", |g, _, _, a| {
        let d = &mut g.gfx.d3d;
        d.fog_on = a.b(0)?;
        d.fog_col = a.i(1)? & 0xFFFFFF;
        d.fog_start = a.r(2)?;
        d.fog_end = a.r(3)?;
        zero()
    }));
    // primitives
    l.push(("d3d_primitive_begin", |g, _, _, a| {
        g.gfx.d3d.prim = Some((a.i(0)?, -1, Vec::new()));
        zero()
    }));
    l.push(("d3d_primitive_begin_texture", |g, _, _, a| {
        g.gfx.d3d.prim = Some((a.i(0)?, a.i(1)?, Vec::new()));
        zero()
    }));
    l.push(("d3d_vertex", |g, _, _, a| push_vertex(g, a, 2)));
    l.push(("d3d_vertex_color", |g, _, _, a| push_vertex(g, a, 3)));
    l.push(("d3d_vertex_texture", |g, _, _, a| push_vertex(g, a, 4)));
    l.push(("d3d_vertex_texture_color", |g, _, _, a| push_vertex(g, a, 5)));
    l.push(("d3d_vertex_normal", |g, _, _, a| push_vertex(g, a, 6)));
    l.push(("d3d_vertex_normal_color", |g, _, _, a| push_vertex(g, a, 7)));
    l.push(("d3d_vertex_normal_texture", |g, _, _, a| push_vertex(g, a, 8)));
    l.push(("d3d_vertex_normal_texture_color", |g, _, _, a| push_vertex(g, a, 9)));
    l.push(("d3d_primitive_end", |g, _, _, _| {
        if let Some((kind, texid, v)) = g.gfx.d3d.prim.take() {
            let tex = g.gfx.texture(texid);
            g.d3d_draw(kind, tex, &v, [0.0; 3]);
        }
        zero()
    }));
    // shapes
    l.push(("d3d_draw_block", |g, _, _, a| draw_shape(g, a, 10)));
    l.push(("d3d_draw_cylinder", |g, _, _, a| draw_shape(g, a, 11)));
    l.push(("d3d_draw_cone", |g, _, _, a| draw_shape(g, a, 12)));
    l.push(("d3d_draw_ellipsoid", |g, _, _, a| draw_shape(g, a, 13)));
    l.push(("d3d_draw_wall", |g, _, _, a| draw_shape(g, a, 14)));
    l.push(("d3d_draw_floor", |g, _, _, a| draw_shape(g, a, 15)));
    // projections
    l.push(("d3d_set_projection", |g, _, _, a| {
        let v: Vec<f64> = (0..9).map(|i| a.r(i)).collect::<Result<_, _>>()?;
        if let Some(view) = look_at([v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]) {
            let p = &g.gfx.d3d.pass;
            let aspect = if p.vh != 0.0 { p.vw / p.vh } else { 1.0 };
            g.gfx.d3d.proj = Proj::Look { view, proj: perspective(45.0, aspect, 1.0, 32000.0) };
        }
        zero()
    }));
    l.push(("d3d_set_projection_ext", |g, _, _, a| {
        let v: Vec<f64> = (0..13).map(|i| a.r(i)).collect::<Result<_, _>>()?;
        let (angle, aspect, mut zn, zf) = (v[9], v[10], v[11], v[12]);
        if zn <= 0.0 {
            zn = 1.0;
        }
        if zf <= zn {
            return zero();
        }
        if let Some(view) = look_at([v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]) {
            g.gfx.d3d.proj = Proj::Look { view, proj: perspective(angle, aspect, zn, zf) };
        }
        zero()
    }));
    l.push(("d3d_set_projection_ortho", |g, _, _, a| {
        g.gfx.d3d.proj = Proj::Ortho { x: a.r(0)?, y: a.r(1)?, w: a.r(2)?, h: a.r(3)?, angle: a.r(4)? };
        zero()
    }));
    l.push(("d3d_set_projection_perspective", |g, _, _, a| {
        g.gfx.d3d.proj = Proj::Persp { x: a.r(0)?, y: a.r(1)?, w: a.r(2)?, h: a.r(3)?, angle: a.r(4)? };
        zero()
    }));
    // transformations
    l.push(("d3d_transform_set_identity", |g, _, _, _| set_world(g, mat::IDENT, false)));
    l.push(("d3d_transform_set_translation", |g, _, _, a| set_world(g, mat::translate(a.r(0)?, a.r(1)?, a.r(2)?), false)));
    l.push(("d3d_transform_set_scaling", |g, _, _, a| set_world(g, mat::scale(a.r(0)?, a.r(1)?, a.r(2)?), false)));
    l.push(("d3d_transform_set_rotation_x", |g, _, _, a| set_world(g, rot(1.0, 0.0, 0.0, a.r(0)?), false)));
    l.push(("d3d_transform_set_rotation_y", |g, _, _, a| set_world(g, rot(0.0, 1.0, 0.0, a.r(0)?), false)));
    l.push(("d3d_transform_set_rotation_z", |g, _, _, a| set_world(g, rot(0.0, 0.0, 1.0, a.r(0)?), false)));
    l.push(("d3d_transform_set_rotation_axis", |g, _, _, a| set_world(g, rot(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?), false)));
    l.push(("d3d_transform_add_translation", |g, _, _, a| set_world(g, mat::translate(a.r(0)?, a.r(1)?, a.r(2)?), true)));
    l.push(("d3d_transform_add_scaling", |g, _, _, a| set_world(g, mat::scale(a.r(0)?, a.r(1)?, a.r(2)?), true)));
    l.push(("d3d_transform_add_rotation_x", |g, _, _, a| set_world(g, rot(1.0, 0.0, 0.0, a.r(0)?), true)));
    l.push(("d3d_transform_add_rotation_y", |g, _, _, a| set_world(g, rot(0.0, 1.0, 0.0, a.r(0)?), true)));
    l.push(("d3d_transform_add_rotation_z", |g, _, _, a| set_world(g, rot(0.0, 0.0, 1.0, a.r(0)?), true)));
    l.push(("d3d_transform_add_rotation_axis", |g, _, _, a| set_world(g, rot(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?), true)));
    l.push(("d3d_transform_stack_clear", |g, _, _, _| {
        g.gfx.d3d.stack.clear();
        zero()
    }));
    l.push(("d3d_transform_stack_empty", |g, _, _, _| bo(g.gfx.d3d.stack.is_empty())));
    l.push(("d3d_transform_stack_push", |g, _, _, _| {
        let d = &mut g.gfx.d3d;
        if d.stack.len() >= STACK_MAX {
            return bo(false);
        }
        d.stack.push(d.world);
        bo(true)
    }));
    l.push(("d3d_transform_stack_pop", |g, _, _, _| {
        let d = &mut g.gfx.d3d;
        match d.stack.pop() {
            Some(m) => {
                d.world = m;
                bo(true)
            }
            None => bo(false),
        }
    }));
    l.push(("d3d_transform_stack_top", |g, _, _, _| {
        let d = &mut g.gfx.d3d;
        match d.stack.last() {
            Some(m) => {
                d.world = *m;
                bo(true)
            }
            None => bo(false),
        }
    }));
    l.push(("d3d_transform_stack_discard", |g, _, _, _| bo(g.gfx.d3d.stack.pop().is_some())));
    // lights
    l.push(("d3d_light_define_ambient", |g, _, _, a| {
        g.gfx.d3d.ambient = a.i(0)? & 0xFFFFFF;
        zero()
    }));
    fn light_entry(g: &mut Game, ind: i32) -> &mut Light {
        g.gfx.d3d.lights.entry(ind).or_insert(Light { point: false, v: [0.0; 3], range: 0.0, col: 0, enabled: false, defined: false })
    }
    l.push(("d3d_light_define_direction", |g, _, _, a| {
        let (dx, dy, dz, col) = (a.r(1)?, a.r(2)?, a.r(3)?, a.i(4)?);
        let l = light_entry(g, a.i(0)?);
        l.point = false;
        l.v = [dx, dy, dz];
        l.col = col & 0xFFFFFF;
        l.defined = true;
        zero()
    }));
    l.push(("d3d_light_define_point", |g, _, _, a| {
        let (x, y, z, range, col) = (a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?, a.i(5)?);
        let l = light_entry(g, a.i(0)?);
        l.point = true;
        l.v = [x, y, z];
        l.range = range;
        l.col = col & 0xFFFFFF;
        l.defined = true;
        zero()
    }));
    l.push(("d3d_light_enable", |g, _, _, a| {
        let on = a.b(1)?;
        light_entry(g, a.i(0)?).enabled = on;
        zero()
    }));
    // models
    l.push(("d3d_model_create", |g, _, _, _| {
        g.gfx.d3d.models.push(Some(Model::default()));
        r((g.gfx.d3d.models.len() - 1) as f64)
    }));
    l.push(("d3d_model_destroy", |g, _, _, a| {
        g.model_mut(a.i(0)?)?;
        g.gfx.d3d.models[a.i(0)? as usize] = None;
        zero()
    }));
    l.push(("d3d_model_clear", |g, _, _, a| {
        g.model_mut(a.i(0)?)?.clear();
        zero()
    }));
    l.push(("d3d_model_save", |g, _, _, a| {
        let p = g.resolve_path(&a.s(1)?);
        let text = g.model_mut(a.i(0)?)?.to_text();
        bo(crate::vfs::write(&p, text.as_bytes()).is_ok())
    }));
    l.push(("d3d_model_load", |g, _, _, a| {
        let p = g.resolve_path(&a.s(1)?);
        let m = g.model_mut(a.i(0)?)?;
        m.clear();
        let Ok(bytes) = crate::vfs::read(&p) else { return bo(false) };
        match Model::from_text(&String::from_utf8_lossy(&bytes)) {
            Some(nm) => {
                *m = nm;
                bo(true)
            }
            None => bo(false),
        }
    }));
    l.push(("d3d_model_draw", |g, _, _, a| {
        let (x, y, z) = (a.r(1)?, a.r(2)?, a.r(3)?);
        let prims = g.model_mut(a.i(0)?)?.compiled();
        let tex = g.gfx.texture(a.i(4)?);
        for (kind, v) in prims.iter() {
            g.d3d_draw(*kind, tex.clone(), v, [x, y, z]);
        }
        zero()
    }));
    l.push(("d3d_model_primitive_begin", |g, _, _, a| {
        let k = a.i(1)?;
        g.model_mut(a.i(0)?)?.push(Cmd::Begin(k));
        zero()
    }));
    l.push(("d3d_model_primitive_end", |g, _, _, a| {
        g.model_mut(a.i(0)?)?.push(Cmd::End);
        zero()
    }));
    l.push(("d3d_model_vertex", |g, _, _, a| model_vertex(g, a, 2)));
    l.push(("d3d_model_vertex_color", |g, _, _, a| model_vertex(g, a, 3)));
    l.push(("d3d_model_vertex_texture", |g, _, _, a| model_vertex(g, a, 4)));
    l.push(("d3d_model_vertex_texture_color", |g, _, _, a| model_vertex(g, a, 5)));
    l.push(("d3d_model_vertex_normal", |g, _, _, a| model_vertex(g, a, 6)));
    l.push(("d3d_model_vertex_normal_color", |g, _, _, a| model_vertex(g, a, 7)));
    l.push(("d3d_model_vertex_normal_texture", |g, _, _, a| model_vertex(g, a, 8)));
    l.push(("d3d_model_vertex_normal_texture_color", |g, _, _, a| model_vertex(g, a, 9)));
    l.push(("d3d_model_block", |g, _, _, a| model_shape(g, a, 10)));
    l.push(("d3d_model_cylinder", |g, _, _, a| model_shape(g, a, 11)));
    l.push(("d3d_model_cone", |g, _, _, a| model_shape(g, a, 12)));
    l.push(("d3d_model_ellipsoid", |g, _, _, a| model_shape(g, a, 13)));
    l.push(("d3d_model_wall", |g, _, _, a| model_shape(g, a, 14)));
    l.push(("d3d_model_floor", |g, _, _, a| model_shape(g, a, 15)));
}
