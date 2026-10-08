//! Chapter 405: drawing, colours, text, primitives, surfaces, tiles, display,
//! window and screen functions.

use super::{bo, r, st, zero, Reg, A};
use crate::draw::{rgba, rgba8, Blend, Target, Tex, Texture};
use crate::game::Game;
use crate::render::TexRef;
use gml::{Ctx, Error, Value, Vm};
use std::rc::Rc;

/// Draws (sprite/background/surface) as a texture source with its origin.
type Src = Option<(Tex, f64, f64)>;

type R = Result<Value, Error>;

impl Game {
    fn col(&self) -> [f32; 4] {
        rgba(self.gfx.color, self.gfx.alpha)
    }

    /// Resolves sprite subimage -1 to the instance's current image.
    fn sub(&self, c: &Ctx, sub: f64) -> f64 {
        if sub < 0.0 {
            self.inst(c.self_id).map(|i| i.image_index).unwrap_or(0.0)
        } else {
            sub
        }
    }

    fn sprite_tex(&self, spr: i32, sub: f64) -> Option<(Rc<Texture>, f64, f64)> {
        let s = self.sprite(spr)?;
        let n = s.frames.len();
        if n == 0 {
            return None;
        }
        let f = (sub.floor() as i64).rem_euclid(n as i64) as usize;
        Some((s.frames[f].clone(), s.origin_x as f64, s.origin_y as f64))
    }

    fn bg_tex(&self, b: i32) -> Option<Rc<Texture>> {
        self.background(b).map(|b| b.tex.clone())
    }

    fn spr_src(&self, spr: i32, sub: f64) -> Src {
        self.sprite_tex(spr, sub).map(|(t, ox, oy)| (Tex::Img(t), ox, oy))
    }

    fn bg_src(&self, b: i32) -> Src {
        self.bg_tex(b).map(|t| (Tex::Img(t), 0.0, 0.0))
    }

    fn surf_src(&self, s: i32) -> Src {
        self.gfx.surface_tex(s).map(|t| (t, 0.0, 0.0))
    }

    /// Tiles a texture over the visible area of the current pass.
    #[allow(clippy::too_many_arguments)]
    fn draw_tiled(&mut self, tex: &Tex, ox: f64, oy: f64, x: f64, y: f64, xs: f64, ys: f64, col: i32, alpha: f64) {
        let (tw, th) = (tex.w() as f64 * xs, tex.h() as f64 * ys);
        if tw.abs() < 0.5 || th.abs() < 0.5 {
            return;
        }
        let (tw, th) = (tw.abs(), th.abs());
        // visible room area: invert pass transform on the clip rectangle
        let (x0, y0, x1, y1) = if self.gfx.target >= 0 {
            let (w, h) = self.gfx.target_size();
            (0.0, 0.0, w as f64, h as f64)
        } else {
            let c = self.gfx.pass_clip;
            match self.gfx.pass_xf.inverse() {
                Some(inv) => {
                    let pts = [
                        inv.apply(c.x0 as f64, c.y0 as f64),
                        inv.apply(c.x1 as f64, c.y0 as f64),
                        inv.apply(c.x1 as f64, c.y1 as f64),
                        inv.apply(c.x0 as f64, c.y1 as f64),
                    ];
                    (
                        pts.iter().map(|p| p.0).fold(f64::MAX, f64::min),
                        pts.iter().map(|p| p.1).fold(f64::MAX, f64::min),
                        pts.iter().map(|p| p.0).fold(f64::MIN, f64::max),
                        pts.iter().map(|p| p.1).fold(f64::MIN, f64::max),
                    )
                }
                None => (0.0, 0.0, self.rs.width as f64, self.rs.height as f64),
            }
        };
        let sx = x - ox * xs;
        let sy = y - oy * ys;
        let startx = sx - ((sx - x0) / tw).ceil() * tw;
        let starty = sy - ((sy - y0) / th).ceil() * th;
        let mut yy = starty;
        while yy < y1 {
            let mut xx = startx;
            while xx < x1 {
                self.gfx.draw_tex(tex, 0.0, 0.0, tex.w() as f64, tex.h() as f64, 0.0, 0.0, xx, yy, xs, ys, 0.0, [col; 4], alpha);
                xx += tw;
            }
            yy += th;
        }
    }

    /// Draws a texture (sprite/background/surface) in one of the standard forms.
    fn draw_form(&mut self, tex: Src, form: Form, a: &A, base: usize) -> R {
        let Some((t, ox, oy)) = tex else { return zero() };
        let (tw, th) = (t.w() as f64, t.h() as f64);
        let g = &mut self.gfx;
        g.texture_repeat = false;
        let w = |i: usize| a.r(base + i);
        match form {
            Form::Plain => g.draw_tex(&t, 0.0, 0.0, tw, th, ox, oy, w(0)?, w(1)?, 1.0, 1.0, 0.0, [0xFFFFFF; 4], 1.0),
            Form::Ext => g.draw_tex(&t, 0.0, 0.0, tw, th, ox, oy, w(0)?, w(1)?, w(2)?, w(3)?, w(4)?, [a.i(base + 5)?; 4], w(6)?),
            Form::Stretched => {
                let (sx, sy) = (w(2)? / tw.max(1.0), w(3)? / th.max(1.0));
                g.draw_tex(&t, 0.0, 0.0, tw, th, 0.0, 0.0, w(0)?, w(1)?, sx, sy, 0.0, [0xFFFFFF; 4], 1.0)
            }
            Form::StretchedExt => {
                let (sx, sy) = (w(2)? / tw.max(1.0), w(3)? / th.max(1.0));
                g.draw_tex(&t, 0.0, 0.0, tw, th, 0.0, 0.0, w(0)?, w(1)?, sx, sy, 0.0, [a.i(base + 4)?; 4], w(5)?)
            }
            Form::Part => g.draw_tex(&t, w(0)?, w(1)?, w(2)?, w(3)?, 0.0, 0.0, w(4)?, w(5)?, 1.0, 1.0, 0.0, [0xFFFFFF; 4], 1.0),
            Form::PartExt => g.draw_tex(
                &t,
                w(0)?,
                w(1)?,
                w(2)?,
                w(3)?,
                0.0,
                0.0,
                w(4)?,
                w(5)?,
                w(6)?,
                w(7)?,
                0.0,
                [a.i(base + 8)?; 4],
                w(9)?,
            ),
            Form::General => g.draw_tex(
                &t,
                w(0)?,
                w(1)?,
                w(2)?,
                w(3)?,
                0.0,
                0.0,
                w(4)?,
                w(5)?,
                w(6)?,
                w(7)?,
                w(8)?,
                [a.i(base + 9)?, a.i(base + 10)?, a.i(base + 11)?, a.i(base + 12)?],
                w(13)?,
            ),
            Form::Tiled => {
                let (x, y) = (w(0)?, w(1)?);
                self.draw_tiled(&t, ox, oy, x, y, 1.0, 1.0, 0xFFFFFF, 1.0);
            }
            Form::TiledExt => {
                let (x, y, xs, ys, c, al) = (w(0)?, w(1)?, w(2)?, w(3)?, a.i(base + 4)?, w(5)?);
                self.draw_tiled(&t, ox, oy, x, y, xs, ys, c, al);
            }
        }
        zero()
    }

    /// Size of an existing surface (error if it does not exist).
    fn surface_mut(&mut self, id: i32) -> Result<(u32, u32), Error> {
        self.gfx.surface_size(id).ok_or_else(|| Error::new(format!("Surface does not exist: {}", id)))
    }

    fn healthbar(&mut self, a: &A) -> R {
        let (x1, y1, x2, y2) = (a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?);
        let amount = a.r(4)?.clamp(0.0, 100.0);
        let (back, minc, maxc, dir, showback, showborder) = (a.i(5)?, a.i(6)?, a.i(7)?, a.i(8)?, a.b(9)?, a.b(10)?);
        let al = self.gfx.alpha;
        let mut p = self.gfx.painter();
        if showback {
            let c = rgba(back, al);
            p.rect(x1, y1, x2, y2, [c; 4], false);
        }
        let f = amount / 100.0;
        let col = crate::particles::mix(minc, maxc, f);
        let c = rgba(col, al);
        let (bx1, by1, bx2, by2) = match dir {
            1 => (x2 - (x2 - x1) * f, y1, x2, y2),
            2 => (x1, y1, x2, y1 + (y2 - y1) * f),
            3 => (x1, y2 - (y2 - y1) * f, x2, y2),
            _ => (x1, y1, x1 + (x2 - x1) * f, y2),
        };
        if f > 0.0 {
            p.rect(bx1, by1, bx2, by2, [c; 4], false);
        }
        if showborder {
            let k = rgba(0, al);
            p.rect(x1, y1, x2, y2, [k; 4], true);
        }
        zero()
    }

    fn flush_primitive(&mut self) {
        let Some((kind, texid, v)) = self.gfx.prim.take() else { return };
        let tex = if texid >= 0 { self.gfx.texture(texid) } else { None };
        let z = self.gfx.d3d.depth;
        let verts: Vec<crate::d3d::MV> = v
            .iter()
            .map(|p| crate::d3d::MV { p: [p.0, p.1, z], uv: [p.2, p.3], col: Some((p.4, p.5)), n: [0.0; 3] })
            .collect();
        let (pipe, st) = self.gfx.pipe_state();
        let gfx = &mut self.gfx;
        gfx.d3d.draw_mesh(&mut *gfx.be, &pipe, &st, kind, tex.as_ref(), &verts, [0.0; 3], (gfx.color, gfx.alpha));
    }
}

#[derive(Clone, Copy)]
enum Form {
    Plain,
    Ext,
    Stretched,
    StretchedExt,
    Tiled,
    TiledExt,
    Part,
    PartExt,
    General,
}

fn tile_mut<'a>(g: &'a mut Game, a: &A) -> Result<&'a mut crate::room::TileState, Error> {
    let id = a.i(0)?;
    g.rs.tiles.iter_mut().find(|t| t.id == id).ok_or_else(|| Error::new(format!("Tile does not exist: {}", id)))
}

fn tile_get(g: &Game, a: &A, f: fn(&crate::room::TileState) -> f64) -> R {
    let id = a.i(0)?;
    match g.rs.tiles.iter().find(|t| t.id == id) {
        Some(t) => r(f(t)),
        None => Err(Error::new(format!("Tile does not exist: {}", id))),
    }
}

#[cfg(windows)]
fn screen_metric(i: i32) -> i32 {
    #[link(name = "user32")]
    extern "system" {
        fn GetSystemMetrics(n: i32) -> i32;
    }
    // SAFETY: GetSystemMetrics has no preconditions.
    unsafe { GetSystemMetrics(i) }
}
#[cfg(not(windows))]
fn screen_metric(i: i32) -> i32 {
    if i == 0 {
        1920
    } else {
        1080
    }
}

pub fn register(l: &mut Reg) {
    l.push(("draw_self", |g, _, c, _| {
        let Some(i) = g.inst(c.self_id) else { return zero() };
        let (s, ii, x, y, xs, ys, a, b, al) =
            (i.sprite_index, i.image_index, i.x, i.y, i.image_xscale, i.image_yscale, i.image_angle, i.image_blend, i.image_alpha);
        g.draw_sprite_ext(s, ii, x, y, xs, ys, a, b, al);
        zero()
    }));
    // sprites
    l.push(("draw_sprite", |g, _, c, a| {
        let t = g.spr_src(a.i(0)?, g.sub(c, a.r(1)?));
        g.draw_form(t, Form::Plain, a, 2)
    }));
    l.push(("draw_sprite_ext", |g, _, c, a| {
        let t = g.spr_src(a.i(0)?, g.sub(c, a.r(1)?));
        g.draw_form(t, Form::Ext, a, 2)
    }));
    l.push(("draw_sprite_stretched", |g, _, c, a| {
        let t = g.spr_src(a.i(0)?, g.sub(c, a.r(1)?));
        g.draw_form(t, Form::Stretched, a, 2)
    }));
    l.push(("draw_sprite_stretched_ext", |g, _, c, a| {
        let t = g.spr_src(a.i(0)?, g.sub(c, a.r(1)?));
        g.draw_form(t, Form::StretchedExt, a, 2)
    }));
    l.push(("draw_sprite_tiled", |g, _, c, a| {
        let t = g.spr_src(a.i(0)?, g.sub(c, a.r(1)?));
        g.draw_form(t, Form::Tiled, a, 2)
    }));
    l.push(("draw_sprite_tiled_ext", |g, _, c, a| {
        let t = g.spr_src(a.i(0)?, g.sub(c, a.r(1)?));
        g.draw_form(t, Form::TiledExt, a, 2)
    }));
    l.push(("draw_sprite_part", |g, _, c, a| {
        let t = g.spr_src(a.i(0)?, g.sub(c, a.r(1)?));
        g.draw_form(t, Form::Part, a, 2)
    }));
    l.push(("draw_sprite_part_ext", |g, _, c, a| {
        let t = g.spr_src(a.i(0)?, g.sub(c, a.r(1)?));
        g.draw_form(t, Form::PartExt, a, 2)
    }));
    l.push(("draw_sprite_general", |g, _, c, a| {
        let t = g.spr_src(a.i(0)?, g.sub(c, a.r(1)?));
        g.draw_form(t, Form::General, a, 2)
    }));
    l.push(("draw_sprite_pos", |g, _, c, a| {
        let Some((t, _, _)) = g.spr_src(a.i(0)?, g.sub(c, a.r(1)?)) else { return zero() };
        let p = [(a.r(2)?, a.r(3)?), (a.r(4)?, a.r(5)?), (a.r(6)?, a.r(7)?), (a.r(8)?, a.r(9)?)];
        let col = rgba(0xFFFFFF, a.r(10)?);
        let (w, h) = (t.w() as f32, t.h() as f32);
        g.gfx.painter().quad_tex(&t, p, [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)], [col; 4], false);
        zero()
    }));
    // backgrounds
    l.push(("draw_background", |g, _, _, a| {
        let t = g.bg_src(a.i(0)?);
        g.draw_form(t, Form::Plain, a, 1)
    }));
    l.push(("draw_background_ext", |g, _, _, a| {
        let t = g.bg_src(a.i(0)?);
        g.draw_form(t, Form::Ext, a, 1)
    }));
    l.push(("draw_background_stretched", |g, _, _, a| {
        let t = g.bg_src(a.i(0)?);
        g.draw_form(t, Form::Stretched, a, 1)
    }));
    l.push(("draw_background_stretched_ext", |g, _, _, a| {
        let t = g.bg_src(a.i(0)?);
        g.draw_form(t, Form::StretchedExt, a, 1)
    }));
    l.push(("draw_background_tiled", |g, _, _, a| {
        let t = g.bg_src(a.i(0)?);
        g.draw_form(t, Form::Tiled, a, 1)
    }));
    l.push(("draw_background_tiled_ext", |g, _, _, a| {
        let t = g.bg_src(a.i(0)?);
        g.draw_form(t, Form::TiledExt, a, 1)
    }));
    l.push(("draw_background_part", |g, _, _, a| {
        let t = g.bg_src(a.i(0)?);
        g.draw_form(t, Form::Part, a, 1)
    }));
    l.push(("draw_background_part_ext", |g, _, _, a| {
        let t = g.bg_src(a.i(0)?);
        g.draw_form(t, Form::PartExt, a, 1)
    }));
    l.push(("draw_background_general", |g, _, _, a| {
        let t = g.bg_src(a.i(0)?);
        g.draw_form(t, Form::General, a, 1)
    }));
    // surfaces drawn like sprites
    l.push(("draw_surface", |g, _, _, a| {
        let t = g.surf_src(a.i(0)?);
        g.draw_form(t, Form::Plain, a, 1)
    }));
    l.push(("draw_surface_ext", |g, _, _, a| {
        let t = g.surf_src(a.i(0)?);
        g.draw_form(t, Form::Ext, a, 1)
    }));
    l.push(("draw_surface_stretched", |g, _, _, a| {
        let t = g.surf_src(a.i(0)?);
        g.draw_form(t, Form::Stretched, a, 1)
    }));
    l.push(("draw_surface_stretched_ext", |g, _, _, a| {
        let t = g.surf_src(a.i(0)?);
        g.draw_form(t, Form::StretchedExt, a, 1)
    }));
    l.push(("draw_surface_tiled", |g, _, _, a| {
        let t = g.surf_src(a.i(0)?);
        g.draw_form(t, Form::Tiled, a, 1)
    }));
    l.push(("draw_surface_tiled_ext", |g, _, _, a| {
        let t = g.surf_src(a.i(0)?);
        g.draw_form(t, Form::TiledExt, a, 1)
    }));
    l.push(("draw_surface_part", |g, _, _, a| {
        let t = g.surf_src(a.i(0)?);
        g.draw_form(t, Form::Part, a, 1)
    }));
    l.push(("draw_surface_part_ext", |g, _, _, a| {
        let t = g.surf_src(a.i(0)?);
        g.draw_form(t, Form::PartExt, a, 1)
    }));
    l.push(("draw_surface_general", |g, _, _, a| {
        let t = g.surf_src(a.i(0)?);
        g.draw_form(t, Form::General, a, 1)
    }));
    // shapes
    l.push(("draw_clear", |g, _, _, a| {
        let c = rgba8(a.i(0)?, 1.0);
        g.gfx.painter().clear_clip(c);
        zero()
    }));
    l.push(("draw_clear_alpha", |g, _, _, a| {
        let c = rgba8(a.i(0)?, a.r(1)?);
        g.gfx.painter().clear_clip(c);
        zero()
    }));
    l.push(("draw_point", |g, _, _, a| {
        let c = g.col();
        g.gfx.painter().point(a.r(0)?, a.r(1)?, c);
        zero()
    }));
    l.push(("draw_point_color", |g, _, _, a| {
        let c = rgba(a.i(2)?, g.gfx.alpha);
        g.gfx.painter().point(a.r(0)?, a.r(1)?, c);
        zero()
    }));
    l.push(("draw_line", |g, _, _, a| {
        let c = g.col();
        g.gfx.painter().line(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, 1.0, c, c);
        zero()
    }));
    l.push(("draw_line_width", |g, _, _, a| {
        let c = g.col();
        g.gfx.painter().line(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?, c, c);
        zero()
    }));
    l.push(("draw_line_color", |g, _, _, a| {
        let al = g.gfx.alpha;
        g.gfx.painter().line(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, 1.0, rgba(a.i(4)?, al), rgba(a.i(5)?, al));
        zero()
    }));
    l.push(("draw_line_width_color", |g, _, _, a| {
        let al = g.gfx.alpha;
        g.gfx.painter().line(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?, rgba(a.i(5)?, al), rgba(a.i(6)?, al));
        zero()
    }));
    l.push(("draw_rectangle", |g, _, _, a| {
        let c = g.col();
        g.gfx.painter().rect(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, [c; 4], a.b(4)?);
        zero()
    }));
    l.push(("draw_rectangle_color", |g, _, _, a| {
        let al = g.gfx.alpha;
        let cs = [rgba(a.i(4)?, al), rgba(a.i(5)?, al), rgba(a.i(6)?, al), rgba(a.i(7)?, al)];
        g.gfx.painter().rect(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, cs, a.b(8)?);
        zero()
    }));
    fn roundrect(g: &mut Game, a: &A, c1: [f32; 4], c2: [f32; 4], outline: bool) -> R {
        let (x1, y1, x2, y2) = (a.r(0)?.min(a.r(2)?), a.r(1)?.min(a.r(3)?), a.r(0)?.max(a.r(2)?), a.r(1)?.max(a.r(3)?));
        let rad = ((x2 - x1).min(y2 - y1) / 4.0).min(10.0).max(0.0);
        let n = (g.gfx.circle_precision / 4).max(1) as usize;
        let mut pts = Vec::new();
        let corners = [(x2 - rad, y1 + rad, 0.0), (x1 + rad, y1 + rad, 90.0), (x1 + rad, y2 - rad, 180.0), (x2 - rad, y2 - rad, 270.0)];
        for (cx, cy, a0) in corners {
            for k in 0..=n {
                let ang = (a0 + 90.0 * k as f64 / n as f64).to_radians();
                pts.push((cx + rad * ang.cos(), cy - rad * ang.sin()));
            }
        }
        let (mx, my) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
        let mut p = g.gfx.painter();
        let m = pts.len();
        for i in 0..m {
            let (a_, b_) = (pts[i], pts[(i + 1) % m]);
            if outline {
                p.line(a_.0, a_.1, b_.0, b_.1, 1.0, c2, c2);
            } else {
                p.tri([(mx + 0.5, my + 0.5), (a_.0 + 0.5, a_.1 + 0.5), (b_.0 + 0.5, b_.1 + 0.5)], [c1, c2, c2]);
            }
        }
        zero()
    }
    l.push(("draw_roundrect", |g, _, _, a| {
        let c = g.col();
        let o = a.b(4)?;
        roundrect(g, a, c, c, o)
    }));
    l.push(("draw_roundrect_color", |g, _, _, a| {
        let al = g.gfx.alpha;
        let (c1, c2, o) = (rgba(a.i(4)?, al), rgba(a.i(5)?, al), a.b(6)?);
        roundrect(g, a, c1, c2, o)
    }));
    fn triangle(g: &mut Game, a: &A, c: [[f32; 4]; 3], outline: bool) -> R {
        let p = [(a.r(0)?, a.r(1)?), (a.r(2)?, a.r(3)?), (a.r(4)?, a.r(5)?)];
        let mut pa = g.gfx.painter();
        if outline {
            pa.line(p[0].0, p[0].1, p[1].0, p[1].1, 1.0, c[0], c[1]);
            pa.line(p[1].0, p[1].1, p[2].0, p[2].1, 1.0, c[1], c[2]);
            pa.line(p[2].0, p[2].1, p[0].0, p[0].1, 1.0, c[2], c[0]);
        } else {
            pa.tri([(p[0].0 + 0.5, p[0].1 + 0.5), (p[1].0 + 0.5, p[1].1 + 0.5), (p[2].0 + 0.5, p[2].1 + 0.5)], c);
        }
        zero()
    }
    l.push(("draw_triangle", |g, _, _, a| {
        let c = g.col();
        let o = a.b(6)?;
        triangle(g, a, [c; 3], o)
    }));
    l.push(("draw_triangle_color", |g, _, _, a| {
        let al = g.gfx.alpha;
        let c = [rgba(a.i(6)?, al), rgba(a.i(7)?, al), rgba(a.i(8)?, al)];
        let o = a.b(9)?;
        triangle(g, a, c, o)
    }));
    l.push(("draw_circle", |g, _, _, a| {
        let (x, y, rr) = (a.r(0)?, a.r(1)?, a.r(2)?);
        let c = g.col();
        let pr = g.gfx.circle_precision;
        g.gfx.painter().ellipse(x - rr, y - rr, x + rr, y + rr, c, c, a.b(3)?, pr);
        zero()
    }));
    l.push(("draw_circle_color", |g, _, _, a| {
        let (x, y, rr) = (a.r(0)?, a.r(1)?, a.r(2)?);
        let al = g.gfx.alpha;
        let pr = g.gfx.circle_precision;
        g.gfx.painter().ellipse(x - rr, y - rr, x + rr, y + rr, rgba(a.i(3)?, al), rgba(a.i(4)?, al), a.b(5)?, pr);
        zero()
    }));
    l.push(("draw_ellipse", |g, _, _, a| {
        let c = g.col();
        let pr = g.gfx.circle_precision;
        g.gfx.painter().ellipse(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, c, c, a.b(4)?, pr);
        zero()
    }));
    l.push(("draw_ellipse_color", |g, _, _, a| {
        let al = g.gfx.alpha;
        let pr = g.gfx.circle_precision;
        g.gfx.painter().ellipse(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, rgba(a.i(4)?, al), rgba(a.i(5)?, al), a.b(6)?, pr);
        zero()
    }));
    l.push(("draw_set_circle_precision", |g, _, _, a| {
        g.gfx.circle_precision = (a.i(0)? / 4 * 4).clamp(4, 64);
        zero()
    }));
    l.push(("draw_arrow", |g, _, _, a| {
        let (x1, y1, x2, y2, size) = (a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?);
        let c = g.col();
        let len = (x2 - x1).hypot(y2 - y1);
        if len <= 0.0 {
            return zero();
        }
        let (dx, dy) = ((x2 - x1) / len, (y2 - y1) / len);
        let s = size.min(len);
        let (bx, by) = (x2 - dx * s, y2 - dy * s);
        let (nx, ny) = (-dy * s / 3.0, dx * s / 3.0);
        let mut p = g.gfx.painter();
        p.line(x1, y1, bx, by, 1.0, c, c);
        p.tri([(x2 + 0.5, y2 + 0.5), (bx + nx + 0.5, by + ny + 0.5), (bx - nx + 0.5, by - ny + 0.5)], [c; 3]);
        zero()
    }));
    l.push(("draw_button", |g, _, _, a| {
        let (x1, y1, x2, y2, up) = (a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, a.b(4)?);
        let al = g.gfx.alpha;
        let (face, light, dark) = (rgba(0xC0C0C0, al), rgba(0xFFFFFF, al), rgba(0x808080, al));
        let (tl, br) = if up { (light, dark) } else { (dark, light) };
        let mut p = g.gfx.painter();
        p.rect(x1, y1, x2, y2, [face; 4], false);
        p.line(x1, y1, x2, y1, 1.0, tl, tl);
        p.line(x1, y1, x1, y2, 1.0, tl, tl);
        p.line(x1, y2, x2 + 1.0, y2, 1.0, br, br);
        p.line(x2, y1, x2, y2, 1.0, br, br);
        zero()
    }));
    l.push(("draw_path", |g, _, _, a| {
        let (pi, x, y, abs) = (a.i(0)?, a.r(1)?, a.r(2)?, a.b(3)?);
        let Some(Some(path)) = g.paths.get(pi.max(0) as usize).cloned() else { return zero() };
        let c = g.col();
        let (ox, oy) = if abs || path.poly.is_empty() { (0.0, 0.0) } else { (x - path.poly[0].0, y - path.poly[0].1) };
        let mut p = g.gfx.painter();
        for w in path.poly.windows(2) {
            p.line(w[0].0 + ox, w[0].1 + oy, w[1].0 + ox, w[1].1 + oy, 1.0, c, c);
        }
        zero()
    }));
    l.push(("draw_healthbar", |g, _, _, a| g.healthbar(a)));
    l.push(("draw_getpixel", |g, _, _, a| {
        let (x, y) = g.gfx.pass_xf.apply(a.r(0)?, a.r(1)?);
        let (bw, bh) = g.gfx.back_size();
        let (xi, yi) = (x.floor() as i32, y.floor() as i32);
        if xi < 0 || yi < 0 || xi >= bw as i32 || yi >= bh as i32 {
            return zero();
        }
        let p = g.gfx.be.pixel(Target::Back, xi, yi);
        r((p[0] as i32 | (p[1] as i32) << 8 | (p[2] as i32) << 16) as f64)
    }));
    // state
    l.push(("draw_set_color", |g, _, _, a| {
        g.gfx.color = a.i(0)? & 0xFFFFFF;
        zero()
    }));
    l.push(("draw_set_alpha", |g, _, _, a| {
        g.gfx.alpha = a.r(0)?;
        zero()
    }));
    l.push(("draw_get_color", |g, _, _, _| r(g.gfx.color as f64)));
    l.push(("draw_get_alpha", |g, _, _, _| r(g.gfx.alpha)));
    l.push(("make_color_rgb", |_, _, _, a| {
        let c = |v: i32| v.clamp(0, 255);
        r((c(a.i(0)?) | c(a.i(1)?) << 8 | c(a.i(2)?) << 16) as f64)
    }));
    l.push(("make_color_hsv", |_, _, _, a| r(crate::particles::hsv(a.r(0)?, a.r(1)?, a.r(2)?) as f64)));
    l.push(("color_get_red", |_, _, _, a| r((a.i(0)? & 0xFF) as f64)));
    l.push(("color_get_green", |_, _, _, a| r(((a.i(0)? >> 8) & 0xFF) as f64)));
    l.push(("color_get_blue", |_, _, _, a| r(((a.i(0)? >> 16) & 0xFF) as f64)));
    fn hsv_of(c: i32) -> (f64, f64, f64) {
        let (r_, g_, b_) = ((c & 0xFF) as f64 / 255.0, ((c >> 8) & 0xFF) as f64 / 255.0, ((c >> 16) & 0xFF) as f64 / 255.0);
        let mx = r_.max(g_).max(b_);
        let mn = r_.min(g_).min(b_);
        let d = mx - mn;
        let h = if d == 0.0 {
            0.0
        } else if mx == r_ {
            60.0 * ((g_ - b_) / d).rem_euclid(6.0)
        } else if mx == g_ {
            60.0 * ((b_ - r_) / d + 2.0)
        } else {
            60.0 * ((r_ - g_) / d + 4.0)
        };
        let s = if mx == 0.0 { 0.0 } else { d / mx };
        ((h / 360.0 * 255.0).round(), (s * 255.0).round(), (mx * 255.0).round())
    }
    l.push(("color_get_hue", |_, _, _, a| r(hsv_of(a.i(0)?).0)));
    l.push(("color_get_saturation", |_, _, _, a| r(hsv_of(a.i(0)?).1)));
    l.push(("color_get_value", |_, _, _, a| r(hsv_of(a.i(0)?).2)));
    l.push(("merge_color", |_, _, _, a| r(crate::particles::mix(a.i(0)?, a.i(1)?, a.r(2)?.clamp(0.0, 1.0)) as f64)));
    l.push(("screen_save", |g, _, _, a| {
        let p = g.resolve_path(&a.s(0)?);
        g.gfx.read_back().save_png(&p, true).map_err(|e| Error::new(e.to_string()))?;
        zero()
    }));
    l.push(("screen_save_part", |g, _, _, a| {
        let p = g.resolve_path(&a.s(0)?);
        let (x, y, w, h) = (a.i(1)?, a.i(2)?, a.i(3)?.max(1), a.i(4)?.max(1));
        let c = g.gfx.be.read(Target::Back, x, y, w as u32, h as u32);
        c.save_png(&p, true).map_err(|e| Error::new(e.to_string()))?;
        zero()
    }));
    // text
    l.push(("draw_set_font", |g, _, _, a| {
        g.gfx.font = a.i(0)?;
        zero()
    }));
    l.push(("draw_set_halign", |g, _, _, a| {
        g.gfx.halign = a.i(0)?;
        zero()
    }));
    l.push(("draw_set_valign", |g, _, _, a| {
        g.gfx.valign = a.i(0)?;
        zero()
    }));
    fn text(g: &mut Game, a: &A, ext: bool, tr: bool, col: bool) -> R {
        let (x, y) = (a.r(0)?, a.r(1)?);
        let s = a.text(2);
        let mut k = 3;
        let (sep, w) = if ext {
            k += 2;
            (a.r(3)?, a.r(4)?)
        } else {
            (-1.0, -1.0)
        };
        let (xs, ys, ang) = if tr {
            k += 3;
            (a.r(k - 3)?, a.r(k - 2)?, a.r(k - 1)?)
        } else {
            (1.0, 1.0, 0.0)
        };
        let (cols, al) = if col {
            ([a.i(k)?, a.i(k + 1)?, a.i(k + 2)?, a.i(k + 3)?], a.r(k + 4)?)
        } else {
            ([g.gfx.color; 4], g.gfx.alpha)
        };
        g.draw_text_full(x, y, &s, sep, w, xs, ys, ang, cols, al);
        zero()
    }
    l.push(("draw_text", |g, _, _, a| text(g, a, false, false, false)));
    l.push(("draw_text_ext", |g, _, _, a| text(g, a, true, false, false)));
    l.push(("draw_text_transformed", |g, _, _, a| text(g, a, false, true, false)));
    l.push(("draw_text_ext_transformed", |g, _, _, a| text(g, a, true, true, false)));
    l.push(("draw_text_color", |g, _, _, a| text(g, a, false, false, true)));
    l.push(("draw_text_ext_color", |g, _, _, a| text(g, a, true, false, true)));
    l.push(("draw_text_transformed_color", |g, _, _, a| text(g, a, false, true, true)));
    l.push(("draw_text_ext_transformed_color", |g, _, _, a| text(g, a, true, true, true)));
    l.push(("string_width", |g, _, _, a| {
        let s = a.text(0);
        r(g.text_size(&s, -1.0, -1.0).0)
    }));
    l.push(("string_height", |g, _, _, a| {
        let s = a.text(0);
        r(g.text_size(&s, -1.0, -1.0).1)
    }));
    l.push(("string_width_ext", |g, _, _, a| {
        let s = a.text(0);
        r(g.text_size(&s, a.r(1)?, a.r(2)?).0)
    }));
    l.push(("string_height_ext", |g, _, _, a| {
        let s = a.text(0);
        r(g.text_size(&s, a.r(1)?, a.r(2)?).1)
    }));
    // primitives and textures
    l.push(("draw_primitive_begin", |g, _, _, a| {
        g.gfx.prim = Some((a.i(0)?, -1, Vec::new()));
        zero()
    }));
    l.push(("draw_primitive_begin_texture", |g, _, _, a| {
        g.gfx.prim = Some((a.i(0)?, a.i(1)?, Vec::new()));
        zero()
    }));
    l.push(("draw_vertex", |g, _, _, a| {
        let (c, al) = (g.gfx.color, g.gfx.alpha);
        if let Some(p) = g.gfx.prim.as_mut() {
            p.2.push((a.r(0)?, a.r(1)?, 0.0, 0.0, c, al));
        }
        zero()
    }));
    l.push(("draw_vertex_color", |g, _, _, a| {
        if let Some(p) = g.gfx.prim.as_mut() {
            p.2.push((a.r(0)?, a.r(1)?, 0.0, 0.0, a.i(2)?, a.r(3)?));
        }
        zero()
    }));
    l.push(("draw_vertex_texture", |g, _, _, a| {
        let (c, al) = (g.gfx.color, g.gfx.alpha);
        if let Some(p) = g.gfx.prim.as_mut() {
            p.2.push((a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, c, al));
        }
        zero()
    }));
    l.push(("draw_vertex_texture_color", |g, _, _, a| {
        if let Some(p) = g.gfx.prim.as_mut() {
            p.2.push((a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, a.i(4)?, a.r(5)?));
        }
        zero()
    }));
    l.push(("draw_primitive_end", |g, _, _, _| {
        g.flush_primitive();
        zero()
    }));
    l.push(("sprite_get_texture", |g, _, _, a| {
        match g.sprite_tex(a.i(0)?, a.r(1)?) {
            Some((t, _, _)) => r(g.gfx.texture_id(&t) as f64),
            None => r(-1.0),
        }
    }));
    l.push(("background_get_texture", |g, _, _, a| match g.bg_tex(a.i(0)?) {
        Some(t) => r(g.gfx.texture_id(&t) as f64),
        None => r(-1.0),
    }));
    l.push(("surface_get_texture", |g, _, _, a| {
        let s = a.i(0)?;
        if g.gfx.surface_size(s).is_none() {
            return r(-1.0);
        }
        if let Some(i) = g.gfx.textures.iter().position(|t| matches!(t, TexRef::Surface(x) if *x == s)) {
            return r(i as f64);
        }
        g.gfx.textures.push(TexRef::Surface(s));
        r((g.gfx.textures.len() - 1) as f64)
    }));
    l.push(("texture_preload", |_, _, _, _| zero()));
    l.push(("texture_set_priority", |_, _, _, _| zero()));
    l.push(("texture_get_width", |g, _, _, a| r(g.gfx.texture_frac(a.i(0)?).0)));
    l.push(("texture_get_height", |g, _, _, a| r(g.gfx.texture_frac(a.i(0)?).1)));
    l.push(("texture_set_interpolation", |g, _, _, a| {
        g.gfx.interpolate = a.b(0)?;
        zero()
    }));
    l.push(("texture_set_blending", |g, _, _, a| {
        g.gfx.d3d.tex_blend = a.b(0)?;
        zero()
    }));
    l.push(("texture_set_repeat", |g, _, _, a| {
        g.gfx.texture_repeat = a.b(0)?;
        zero()
    }));
    l.push(("draw_set_blend_mode", |g, _, _, a| {
        g.gfx.blend = Blend::from_mode(a.i(0)?);
        zero()
    }));
    l.push(("draw_set_blend_mode_ext", |g, _, _, a| {
        g.gfx.blend = Blend {
            src: a.i(0)?.clamp(1, 11) as u8,
            dst: a.i(1)?.clamp(1, 11) as u8,
        };
        zero()
    }));
    // surfaces
    l.push(("surface_create", |g, _, _, a| {
        let (w, h) = (a.i(0)?.max(1) as u32, a.i(1)?.max(1) as u32);
        r(g.gfx.surface_create(w, h) as f64)
    }));
    l.push(("surface_free", |g, _, _, a| {
        let id = a.i(0)?;
        if g.gfx.target == id {
            g.reset_surface_target();
        }
        g.gfx.surface_free(id);
        zero()
    }));
    l.push(("surface_exists", |g, _, _, a| bo(g.gfx.surface_size(a.i(0)?).is_some())));
    l.push(("surface_get_width", |g, _, _, a| r(g.surface_mut(a.i(0)?)?.0 as f64)));
    l.push(("surface_get_height", |g, _, _, a| r(g.surface_mut(a.i(0)?)?.1 as f64)));
    l.push(("surface_set_target", |g, _, _, a| {
        let id = a.i(0)?;
        g.surface_mut(id)?;
        g.set_surface_target(id);
        zero()
    }));
    l.push(("surface_reset_target", |g, _, _, _| {
        g.reset_surface_target();
        zero()
    }));
    l.push(("surface_getpixel", |g, _, _, a| {
        let (x, y) = (a.i(1)?, a.i(2)?);
        let id = a.i(0)?;
        let (w, h) = g.surface_mut(id)?;
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            return zero();
        }
        let p = g.gfx.be.pixel(Target::Surface(id), x, y);
        r((p[0] as i32 | (p[1] as i32) << 8 | (p[2] as i32) << 16) as f64)
    }));
    l.push(("surface_save", |g, _, _, a| {
        let p = g.resolve_path(&a.s(1)?);
        let id = a.i(0)?;
        let (w, h) = g.surface_mut(id)?;
        let s = g.gfx.be.read(Target::Surface(id), 0, 0, w, h);
        s.save_png(&p, false).map_err(|e| Error::new(e.to_string()))?;
        zero()
    }));
    l.push(("surface_save_part", |g, _, _, a| {
        let p = g.resolve_path(&a.s(1)?);
        let (x, y, w, h) = (a.i(2)?, a.i(3)?, a.i(4)?.max(1), a.i(5)?.max(1));
        let id = a.i(0)?;
        g.surface_mut(id)?;
        let c = g.gfx.be.read(Target::Surface(id), x, y, w as u32, h as u32);
        c.save_png(&p, false).map_err(|e| Error::new(e.to_string()))?;
        zero()
    }));
    l.push(("surface_copy", |g, _, _, a| {
        let (dst, src) = (a.i(0)?, a.i(3)?);
        let (sw, sh) = g.surface_mut(src)?;
        g.surface_mut(dst)?;
        let (x, y) = (a.i(1)?, a.i(2)?);
        g.gfx.be.copy(Target::Surface(dst), x, y, Target::Surface(src), 0, 0, sw as i32, sh as i32);
        zero()
    }));
    l.push(("surface_copy_part", |g, _, _, a| {
        let (dst, src) = (a.i(0)?, a.i(3)?);
        g.surface_mut(src)?;
        g.surface_mut(dst)?;
        let (x, y, xs, ys, ws, hs) = (a.i(1)?, a.i(2)?, a.i(4)?, a.i(5)?, a.i(6)?, a.i(7)?);
        g.gfx.be.copy(Target::Surface(dst), x, y, Target::Surface(src), xs, ys, ws, hs);
        zero()
    }));
    // tiles
    l.push(("tile_add", |g, _, _, a| {
        let id = g.next_tile_id;
        g.next_tile_id += 1;
        g.rs.tiles.push(crate::room::TileState {
            id,
            background: a.i(0)?,
            left: a.r(1)?,
            top: a.r(2)?,
            width: a.r(3)?,
            height: a.r(4)?,
            x: a.r(5)?,
            y: a.r(6)?,
            depth: a.r(7)?,
            visible: true,
            xscale: 1.0,
            yscale: 1.0,
            blend: 0xFFFFFF,
            alpha: 1.0,
        });
        r(id as f64)
    }));
    l.push(("tile_delete", |g, _, _, a| {
        let id = a.i(0)?;
        g.rs.tiles.retain(|t| t.id != id);
        zero()
    }));
    l.push(("tile_exists", |g, _, _, a| {
        let id = a.i(0)?;
        bo(g.rs.tiles.iter().any(|t| t.id == id))
    }));
    l.push(("tile_get_x", |g, _, _, a| tile_get(g, a, |t| t.x)));
    l.push(("tile_get_y", |g, _, _, a| tile_get(g, a, |t| t.y)));
    l.push(("tile_get_left", |g, _, _, a| tile_get(g, a, |t| t.left)));
    l.push(("tile_get_top", |g, _, _, a| tile_get(g, a, |t| t.top)));
    l.push(("tile_get_width", |g, _, _, a| tile_get(g, a, |t| t.width)));
    l.push(("tile_get_height", |g, _, _, a| tile_get(g, a, |t| t.height)));
    l.push(("tile_get_depth", |g, _, _, a| tile_get(g, a, |t| t.depth)));
    l.push(("tile_get_visible", |g, _, _, a| tile_get(g, a, |t| t.visible as i32 as f64)));
    l.push(("tile_get_xscale", |g, _, _, a| tile_get(g, a, |t| t.xscale)));
    l.push(("tile_get_yscale", |g, _, _, a| tile_get(g, a, |t| t.yscale)));
    l.push(("tile_get_background", |g, _, _, a| tile_get(g, a, |t| t.background as f64)));
    l.push(("tile_get_blend", |g, _, _, a| tile_get(g, a, |t| t.blend as f64)));
    l.push(("tile_get_alpha", |g, _, _, a| tile_get(g, a, |t| t.alpha)));
    l.push(("tile_set_position", |g, _, _, a| {
        let (x, y) = (a.r(1)?, a.r(2)?);
        let t = tile_mut(g, a)?;
        t.x = x;
        t.y = y;
        zero()
    }));
    l.push(("tile_set_region", |g, _, _, a| {
        let (l_, t_, w, h) = (a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?);
        let t = tile_mut(g, a)?;
        t.left = l_;
        t.top = t_;
        t.width = w;
        t.height = h;
        zero()
    }));
    l.push(("tile_set_background", |g, _, _, a| {
        let b = a.i(1)?;
        tile_mut(g, a)?.background = b;
        zero()
    }));
    l.push(("tile_set_visible", |g, _, _, a| {
        let b = a.b(1)?;
        tile_mut(g, a)?.visible = b;
        zero()
    }));
    l.push(("tile_set_depth", |g, _, _, a| {
        let d = a.r(1)?;
        tile_mut(g, a)?.depth = d;
        zero()
    }));
    l.push(("tile_set_scale", |g, _, _, a| {
        let (xs, ys) = (a.r(1)?, a.r(2)?);
        let t = tile_mut(g, a)?;
        t.xscale = xs;
        t.yscale = ys;
        zero()
    }));
    l.push(("tile_set_blend", |g, _, _, a| {
        let c = a.i(1)?;
        tile_mut(g, a)?.blend = c;
        zero()
    }));
    l.push(("tile_set_alpha", |g, _, _, a| {
        let c = a.r(1)?;
        tile_mut(g, a)?.alpha = c;
        zero()
    }));
    l.push(("tile_layer_hide", |g, _, _, a| {
        let d = a.r(0)?;
        g.rs.tiles.iter_mut().filter(|t| t.depth == d).for_each(|t| t.visible = false);
        zero()
    }));
    l.push(("tile_layer_show", |g, _, _, a| {
        let d = a.r(0)?;
        g.rs.tiles.iter_mut().filter(|t| t.depth == d).for_each(|t| t.visible = true);
        zero()
    }));
    l.push(("tile_layer_delete", |g, _, _, a| {
        let d = a.r(0)?;
        g.rs.tiles.retain(|t| t.depth != d);
        zero()
    }));
    l.push(("tile_layer_shift", |g, _, _, a| {
        let (d, x, y) = (a.r(0)?, a.r(1)?, a.r(2)?);
        g.rs.tiles.iter_mut().filter(|t| t.depth == d).for_each(|t| {
            t.x += x;
            t.y += y
        });
        zero()
    }));
    fn tile_at(t: &crate::room::TileState, x: f64, y: f64) -> bool {
        x >= t.x && y >= t.y && x < t.x + t.width * t.xscale && y < t.y + t.height * t.yscale
    }
    l.push(("tile_layer_find", |g, _, _, a| {
        let (d, x, y) = (a.r(0)?, a.r(1)?, a.r(2)?);
        r(g.rs.tiles.iter().find(|t| t.depth == d && tile_at(t, x, y)).map(|t| t.id).unwrap_or(-1) as f64)
    }));
    l.push(("tile_layer_delete_at", |g, _, _, a| {
        let (d, x, y) = (a.r(0)?, a.r(1)?, a.r(2)?);
        g.rs.tiles.retain(|t| !(t.depth == d && tile_at(t, x, y)));
        zero()
    }));
    l.push(("tile_layer_depth", |g, _, _, a| {
        let (d, nd) = (a.r(0)?, a.r(1)?);
        g.rs.tiles.iter_mut().filter(|t| t.depth == d).for_each(|t| t.depth = nd);
        zero()
    }));
    // display
    l.push(("display_get_width", |_, _, _, _| r(screen_metric(0) as f64)));
    l.push(("display_get_height", |_, _, _, _| r(screen_metric(1) as f64)));
    l.push(("display_get_colordepth", |_, _, _, _| r(32.0)));
    l.push(("display_get_frequency", |_, _, _, _| r(60.0)));
    l.push(("display_set_size", |_, _, _, _| bo(false)));
    l.push(("display_set_colordepth", |_, _, _, _| bo(false)));
    l.push(("display_set_frequency", |_, _, _, _| bo(false)));
    l.push(("display_set_all", |_, _, _, _| bo(false)));
    l.push(("display_test_all", |_, _, _, _| bo(false)));
    l.push(("display_reset", |g, _, _, _| {
        g.reset_surface_target();
        for id in 0..g.gfx.surfaces.len() as i32 {
            g.gfx.surface_free(id);
        }
        zero()
    }));
    l.push(("display_mouse_get_x", |g, _, _, _| r(g.win.x.max(0) as f64 + g.input.mouse_x)));
    l.push(("display_mouse_get_y", |g, _, _, _| r(g.win.y.max(0) as f64 + g.input.mouse_y)));
    l.push(("display_mouse_set", |g, _, _, a| {
        g.win.set_mouse = Some((a.r(0)? - g.win.x.max(0) as f64, a.r(1)? - g.win.y.max(0) as f64));
        zero()
    }));
    // window
    l.push(("window_set_visible", |g, _, _, a| {
        g.window_visible = a.b(0)?;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_visible", |g, _, _, _| bo(g.window_visible)));
    l.push(("window_set_fullscreen", |g, _, _, a| {
        g.fullscreen = a.b(0)?;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_fullscreen", |g, _, _, _| bo(g.fullscreen)));
    l.push(("window_set_showborder", |g, _, _, a| {
        g.win.showborder = a.b(0)?;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_showborder", |g, _, _, _| bo(g.win.showborder)));
    l.push(("window_set_showicons", |g, _, _, a| {
        g.win.showicons = a.b(0)?;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_showicons", |g, _, _, _| bo(g.win.showicons)));
    l.push(("window_set_stayontop", |g, _, _, a| {
        g.win.stayontop = a.b(0)?;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_stayontop", |g, _, _, _| bo(g.win.stayontop)));
    l.push(("window_set_sizeable", |g, _, _, a| {
        g.win.sizeable = a.b(0)?;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_sizeable", |g, _, _, _| bo(g.win.sizeable)));
    l.push(("window_set_caption", |g, _, _, a| {
        g.rs.caption = a.s(0)?;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_caption", |g, _, _, _| st(&g.caption())));
    l.push(("window_set_cursor", |g, _, _, a| {
        g.cursor = a.i(0)?;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_cursor", |g, _, _, _| r(g.cursor as f64)));
    l.push(("window_set_color", |g, _, _, a| {
        g.win.color = a.i(0)?;
        g.settings.color_outside_room = (a.i(0)? & 0xFFFFFF) as u32;
        zero()
    }));
    l.push(("window_get_color", |g, _, _, _| r(g.win.color as f64)));
    l.push(("window_set_region_scale", |g, _, _, a| {
        g.win.region_scale = a.r(0)?;
        if a.b(1)? {
            g.win.w = 0;
            g.win.h = 0;
        }
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_region_scale", |g, _, _, _| r(g.win.region_scale)));
    l.push(("window_set_position", |g, _, _, a| {
        g.win.x = a.i(0)?;
        g.win.y = a.i(1)?;
        g.win.center = false;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_set_size", |g, _, _, a| {
        g.win.w = a.i(0)?;
        g.win.h = a.i(1)?;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_set_rectangle", |g, _, _, a| {
        g.win.x = a.i(0)?;
        g.win.y = a.i(1)?;
        g.win.w = a.i(2)?;
        g.win.h = a.i(3)?;
        g.win.center = false;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_center", |g, _, _, _| {
        g.win.center = true;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_default", |g, _, _, _| {
        g.win.w = 0;
        g.win.h = 0;
        g.win.center = true;
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_x", |g, _, _, _| r(g.win.x.max(0) as f64)));
    l.push(("window_get_y", |g, _, _, _| r(g.win.y.max(0) as f64)));
    l.push(("window_get_width", |g, _, _, _| {
        r(if g.win.actual_w > 0 { g.win.actual_w } else { g.region_w as i32 } as f64)
    }));
    l.push(("window_get_height", |g, _, _, _| {
        r(if g.win.actual_h > 0 { g.win.actual_h } else { g.region_h as i32 } as f64)
    }));
    l.push(("window_mouse_get_x", |g, _, _, _| r(g.input.mouse_x.floor())));
    l.push(("window_mouse_get_y", |g, _, _, _| r(g.input.mouse_y.floor())));
    l.push(("window_mouse_set", |g, _, _, a| {
        let (x, y) = (a.r(0)?, a.r(1)?);
        g.win.set_mouse = Some((x, y));
        g.input.mouse_x = x;
        g.input.mouse_y = y;
        zero()
    }));
    l.push(("window_set_region_size", |g, _, _, a| {
        let (w, h) = (a.i(0)?.max(1) as u32, a.i(1)?.max(1) as u32);
        g.region_w = w;
        g.region_h = h;
        g.gfx.resize_back(w, h);
        if a.b(2)? {
            g.win.w = 0;
            g.win.h = 0;
        }
        g.window_dirty = true;
        zero()
    }));
    l.push(("window_get_region_width", |g, _, _, _| r(g.region_w as f64)));
    l.push(("window_get_region_height", |g, _, _, _| r(g.region_h as f64)));
    l.push(("window_view_mouse_get_x", |g, _, _, a| {
        let v = a.i(0)?.clamp(0, 7) as usize;
        r(g.mouse_in_view(v, g.input.mouse_x, g.input.mouse_y).0.floor())
    }));
    l.push(("window_view_mouse_get_y", |g, _, _, a| {
        let v = a.i(0)?.clamp(0, 7) as usize;
        r(g.mouse_in_view(v, g.input.mouse_x, g.input.mouse_y).1.floor())
    }));
    l.push(("window_view_mouse_set", |g, _, _, a| {
        let v = a.i(0)?.clamp(0, 7) as usize;
        let vs = g.rs.views[v].clone();
        let xf = crate::draw::painter::Xf::view(vs.xview, vs.yview, vs.wview, vs.hview, vs.angle, vs.xport, vs.yport, vs.wport, vs.hport);
        let (x, y) = xf.apply(a.r(1)?, a.r(2)?);
        g.win.set_mouse = Some((x, y));
        g.input.mouse_x = x;
        g.input.mouse_y = y;
        zero()
    }));
    l.push(("window_views_mouse_get_x", |g, _, _, _| r(g.mouse_room_pos().0.floor())));
    l.push(("window_views_mouse_get_y", |g, _, _, _| r(g.mouse_room_pos().1.floor())));
    l.push(("window_views_mouse_set", |g, _, _, a| {
        let v = (0..8).find(|&v| g.rs.views[v].visible).unwrap_or(0);
        let vs = g.rs.views[v].clone();
        let xf = crate::draw::painter::Xf::view(vs.xview, vs.yview, vs.wview, vs.hview, vs.angle, vs.xport, vs.yport, vs.wport, vs.hport);
        let (x, y) = if g.rs.views_enabled { xf.apply(a.r(0)?, a.r(1)?) } else { (a.r(0)?, a.r(1)?) };
        g.win.set_mouse = Some((x, y));
        g.input.mouse_x = x;
        g.input.mouse_y = y;
        zero()
    }));
    l.push(("screen_redraw", |g, vm, _, _| {
        g.draw_frame(vm)?;
        g.gfx.present_now();
        zero()
    }));
    l.push(("screen_refresh", |g, _, _, _| {
        g.gfx.present_now();
        zero()
    }));
    l.push(("screen_wait_vsync", |_, _, _, _| zero()));
    l.push(("set_automatic_draw", |g, _, _, a| {
        g.automatic_draw = a.b(0)?;
        zero()
    }));
    l.push(("set_synchronization", |g, _, _, a| {
        g.gfx.vsync_change = Some(a.b(0)?);
        zero()
    }));
    l.push(("YoYo_EnableAlphaBlend", |g, _, _, a| {
        g.gfx.blend = if a.b(0)? { Blend::NORMAL } else { Blend { src: 2, dst: 1 } };
        zero()
    }));
    let _ = |_: &mut Vm| ();
}
