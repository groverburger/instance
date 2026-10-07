//! Drawing state, the per-frame draw (runtime.md §13), text layout, surfaces and
//! room transitions.

use crate::draw::font::{split_lines, Face, FontSystem};
use crate::draw::painter::{Painter, Xf};
use crate::draw::{rgba, rgba8, Blend, Canvas, Clip, Texture};
use crate::events::ev;
use crate::game::Game;
use gml::{Error, Vm};
use std::rc::Rc;

/// Global draw state and render targets.
pub struct Gfx {
    pub back: Canvas,
    pub surfaces: Vec<Option<Canvas>>,
    /// -1 = back buffer.
    pub target: i32,
    pub pass_xf: Xf,
    pub pass_clip: Clip,
    pub color: i32,
    pub alpha: f64,
    pub font: i32,
    pub halign: i32,
    pub valign: i32,
    pub blend: Blend,
    pub circle_precision: i32,
    pub interpolate: bool,
    pub texture_repeat: bool,
    pub fonts: FontSystem,
    /// Hook used to show intermediate frames (transitions, screen_refresh,
    /// message boxes). None in headless mode.
    pub present: Option<Box<dyn FnMut(&Texture)>>,
    /// Texture ids (sprite_get_texture etc.).
    pub textures: Vec<TexRef>,
    pub tex_ids: std::collections::HashMap<usize, i32>,
    /// Primitive under construction: kind, texture, vertices (x, y, u, v, colour, alpha).
    pub prim: Option<(i32, i32, Vec<(f64, f64, f64, f64, i32, f64)>)>,
}

/// A texture id target.
#[derive(Clone)]
pub enum TexRef {
    Static(Rc<Texture>),
    Surface(i32),
}

impl Gfx {
    pub fn new(interpolate: bool) -> Self {
        let back = Canvas::new(640, 480);
        let clip = Clip::full(&back);
        Gfx {
            back,
            surfaces: Vec::new(),
            target: -1,
            pass_xf: Xf::IDENTITY,
            pass_clip: clip,
            color: 0,
            alpha: 1.0,
            font: -1,
            halign: 0,
            valign: 0,
            blend: Blend::NORMAL,
            circle_precision: 24,
            interpolate,
            texture_repeat: false,
            fonts: FontSystem::new(),
            present: None,
            textures: Vec::new(),
            tex_ids: std::collections::HashMap::new(),
            prim: None,
        }
    }

    pub fn resize_back(&mut self, w: u32, h: u32) {
        self.back = Canvas::new(w, h);
        self.pass_clip = Clip::full(&self.back);
        self.pass_xf = Xf::IDENTITY;
    }

    /// The current target canvas, transform and clip.
    pub fn painter(&mut self) -> Painter<'_> {
        let blend = self.blend;
        let bilinear = self.interpolate;
        if self.target >= 0 {
            if let Some(Some(s)) = self.surfaces.get_mut(self.target as usize) {
                let clip = Clip::full(s);
                return Painter {
                    target: s,
                    xf: Xf::IDENTITY,
                    clip,
                    blend,
                    bilinear,
                };
            }
        }
        Painter {
            target: &mut self.back,
            xf: self.pass_xf,
            clip: self.pass_clip,
            blend,
            bilinear,
        }
    }

    /// Current target size.
    pub fn target_size(&self) -> (u32, u32) {
        if self.target >= 0 {
            if let Some(Some(s)) = self.surfaces.get(self.target as usize) {
                return (s.w, s.h);
            }
        }
        (self.back.w, self.back.h)
    }

    /// Registers a texture and returns its id.
    pub fn texture_id(&mut self, t: &Rc<Texture>) -> i32 {
        let key = Rc::as_ptr(t) as usize;
        if let Some(&id) = self.tex_ids.get(&key) {
            return id;
        }
        self.textures.push(TexRef::Static(t.clone()));
        let id = self.textures.len() as i32 - 1;
        self.tex_ids.insert(key, id);
        id
    }

    /// Resolves a texture id.
    pub fn texture(&self, id: i32) -> Option<Rc<Texture>> {
        match self.textures.get(id.max(0) as usize)? {
            TexRef::Static(t) if id >= 0 => Some(t.clone()),
            TexRef::Surface(s) => self.surfaces.get(*s as usize)?.as_ref().map(|c| Rc::new(c.clone())),
            _ => None,
        }
    }

    pub fn present_now(&mut self) {
        if let Some(p) = self.present.as_mut() {
            p(&self.back);
        }
    }

    /// Draws a texture region transformed: the part (left, top, w, h) of `tex` is
    /// placed with its origin offset (ox, oy) at (x, y), scaled and rotated.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_tex(
        &mut self,
        tex: &Texture,
        left: f64,
        top: f64,
        w: f64,
        h: f64,
        ox: f64,
        oy: f64,
        x: f64,
        y: f64,
        xs: f64,
        ys: f64,
        angle: f64,
        cols: [i32; 4],
        alpha: f64,
    ) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let a = angle.to_radians();
        let (c, s) = (a.cos(), a.sin());
        let corner = |lx: f64, ly: f64| -> (f64, f64) {
            let px = (lx - ox) * xs;
            let py = (ly - oy) * ys;
            (x + px * c + py * s, y - px * s + py * c)
        };
        let p = [corner(0.0, 0.0), corner(w, 0.0), corner(w, h), corner(0.0, h)];
        let (u0, v0, u1, v1) = (left as f32, top as f32, (left + w) as f32, (top + h) as f32);
        let uv = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)];
        let cs = [
            rgba(cols[0], alpha),
            rgba(cols[1], alpha),
            rgba(cols[2], alpha),
            rgba(cols[3], alpha),
        ];
        let repeat = self.texture_repeat;
        self.painter().quad_tex(tex, p, uv, cs, repeat);
    }
}

impl Game {
    /// Resolves the font of `draw_set_font`.
    pub fn current_face(&mut self) -> Option<Face> {
        let (name, size, bold, italic) = match self.fonts.get(self.gfx.font.max(0) as usize).and_then(|f| f.as_ref()) {
            Some(f) if self.gfx.font >= 0 => (f.font_name.clone(), f.size, f.bold, f.italic),
            _ => ("Arial".to_string(), 12, false, false),
        };
        self.gfx.fonts.face(&name, size, bold, italic, true)
    }

    /// Splits text into lines with word wrapping; returns lines with widths.
    pub fn layout_text(&mut self, s: &[u8], w: f64) -> (Option<Face>, Vec<(Vec<char>, f32)>) {
        let face = self.current_face();
        let Some(f) = face.clone() else {
            return (None, split_lines(s).into_iter().map(|l| (l, 0.0)).collect());
        };
        let mut out = Vec::new();
        for line in split_lines(s) {
            let width = |fs: &mut FontSystem, l: &[char]| -> f32 { l.iter().map(|c| fs.advance(&f, *c)).sum() };
            if w <= 0.0 {
                let lw = width(&mut self.gfx.fonts, &line);
                out.push((line, lw));
                continue;
            }
            // word wrap on spaces
            let mut cur: Vec<char> = Vec::new();
            let mut words: Vec<Vec<char>> = Vec::new();
            let mut word = Vec::new();
            for c in line {
                if c == ' ' {
                    word.push(c);
                    words.push(std::mem::take(&mut word));
                } else {
                    word.push(c);
                }
            }
            if !word.is_empty() {
                words.push(word);
            }
            for wd in words {
                let mut cand = cur.clone();
                cand.extend(wd.iter());
                let trimmed: Vec<char> = {
                    let mut t = cand.clone();
                    while t.last() == Some(&' ') {
                        t.pop();
                    }
                    t
                };
                if !cur.is_empty() && width(&mut self.gfx.fonts, &trimmed) as f64 > w {
                    let mut t = std::mem::take(&mut cur);
                    while t.last() == Some(&' ') {
                        t.pop();
                    }
                    let lw = width(&mut self.gfx.fonts, &t);
                    out.push((t, lw));
                    cur = wd;
                } else {
                    cur = cand;
                }
            }
            while cur.last() == Some(&' ') {
                cur.pop();
            }
            let lw = width(&mut self.gfx.fonts, &cur);
            out.push((cur, lw));
        }
        (face, out)
    }

    /// Line height of the current font (or `sep` if >= 0).
    fn line_sep(face: &Option<Face>, sep: f64) -> f64 {
        if sep >= 0.0 {
            return sep;
        }
        face.as_ref().map(|f| f.line_height as f64).unwrap_or(16.0)
    }

    /// The current font if it is a sprite font: (sprite, first char, proportional, sep).
    fn sprite_font(&self) -> Option<(i32, i32, bool, i32)> {
        if self.gfx.font < 0 {
            return None;
        }
        self.fonts.get(self.gfx.font as usize)?.as_ref()?.sprite
    }

    /// Advance and glyph frame of a character in a sprite font.
    fn sprite_glyph(&self, sf: (i32, i32, bool, i32), ch: char) -> (f64, Option<usize>) {
        let Some(spr) = self.sprite(sf.0) else { return (0.0, None) };
        let code = gm_format::text::encode_cp1252(&ch.to_string()).first().copied().unwrap_or(b'?') as i32;
        let k = code - sf.1;
        if k < 0 || k as usize >= spr.frames.len() {
            return (spr.width as f64 / 2.0 + sf.3 as f64, None);
        }
        let f = &spr.frames[k as usize];
        let adv = if sf.2 {
            // proportional: opaque columns
            let cols: Vec<u32> = (0..f.w).filter(|&x| (0..f.h).any(|y| f.get(x, y)[3] > 0)).collect();
            match (cols.first(), cols.last()) {
                (Some(a), Some(b)) => (b - a + 1) as f64,
                _ => spr.width as f64 / 2.0,
            }
        } else {
            spr.width as f64
        };
        (adv + sf.3 as f64, Some(k as usize))
    }

    fn sprite_text_lines(&self, s: &[u8], sf: (i32, i32, bool, i32)) -> Vec<(Vec<char>, f64)> {
        split_lines(s)
            .into_iter()
            .map(|l| {
                let w = l.iter().map(|c| self.sprite_glyph(sf, *c).0).sum();
                (l, w)
            })
            .collect()
    }

    /// `string_width_ext` / `string_height_ext`.
    pub fn text_size(&mut self, s: &[u8], sep: f64, w: f64) -> (f64, f64) {
        if let Some(sf) = self.sprite_font() {
            let lines = self.sprite_text_lines(s, sf);
            let lh = if sep >= 0.0 { sep } else { self.sprite(sf.0).map(|x| x.height as f64).unwrap_or(16.0) };
            return (lines.iter().map(|l| l.1).fold(0.0, f64::max), lh * lines.len() as f64);
        }
        let (face, lines) = self.layout_text(s, w);
        let lh = Self::line_sep(&face, sep);
        let width = lines.iter().map(|l| l.1).fold(0.0f32, f32::max) as f64;
        (width.ceil(), lh * lines.len() as f64)
    }

    /// Draws text (all draw_text variants).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_full(
        &mut self,
        x: f64,
        y: f64,
        s: &[u8],
        sep: f64,
        w: f64,
        xs: f64,
        ys: f64,
        angle: f64,
        cols: [i32; 4],
        alpha: f64,
    ) {
        if let Some(sf) = self.sprite_font() {
            let lines = self.sprite_text_lines(s, sf);
            let Some(spr) = self.sprite(sf.0).cloned() else { return };
            let lh = if sep >= 0.0 { sep } else { spr.height as f64 };
            let total = lh * lines.len() as f64;
            let y0 = match self.gfx.valign {
                1 => -total / 2.0,
                2 => -total,
                _ => 0.0,
            };
            let a = angle.to_radians();
            let (c, sn) = (a.cos(), a.sin());
            for (li, (line, lw)) in lines.iter().enumerate() {
                let mut pen = match self.gfx.halign {
                    1 => -lw / 2.0,
                    2 => -lw,
                    _ => 0.0,
                };
                let ly = y0 + li as f64 * lh;
                for ch in line {
                    let (adv, frame) = self.sprite_glyph(sf, *ch);
                    if let Some(k) = frame {
                        let f = spr.frames[k].clone();
                        let left = if sf.2 { (0..f.w).find(|&x| (0..f.h).any(|y| f.get(x, y)[3] > 0)).unwrap_or(0) as f64 } else { 0.0 };
                        let (px, py) = ((pen - left) * xs, ly * ys);
                        let (gx, gy) = (x + px * c + py * sn, y - px * sn + py * c);
                        self.gfx.draw_tex(&f, 0.0, 0.0, f.w as f64, f.h as f64, 0.0, 0.0, gx, gy, xs, ys, angle, cols, alpha);
                    }
                    pen += adv;
                }
            }
            return;
        }
        let (face, lines) = self.layout_text(s, w);
        let Some(face) = face else { return };
        let lh = Self::line_sep(&Some(face.clone()), sep);
        let total_h = lh * lines.len() as f64;
        let max_w = lines.iter().map(|l| l.1).fold(0.0f32, f32::max) as f64;
        let y0 = match self.gfx.valign {
            1 => -total_h / 2.0,
            2 => -total_h,
            _ => 0.0,
        };
        let a = angle.to_radians();
        let (c, sn) = (a.cos(), a.sin());
        let to_room = |lx: f64, ly: f64| -> (f64, f64) {
            let (px, py) = (lx * xs, ly * ys);
            (x + px * c + py * sn, y - px * sn + py * c)
        };
        let gradient = !(cols[0] == cols[1] && cols[1] == cols[2] && cols[2] == cols[3]);
        let halign0 = self.gfx.halign;
        let col_at = |lx: f64, ly: f64| -> [f32; 4] {
            if !gradient {
                return rgba(cols[0], alpha);
            }
            // bilinear over the text box
            let bx0 = match halign0 {
                1 => -max_w / 2.0,
                2 => -max_w,
                _ => 0.0,
            };
            let fx = if max_w > 0.0 { ((lx - bx0) / max_w).clamp(0.0, 1.0) as f32 } else { 0.0 };
            let fy = if total_h > 0.0 { ((ly - y0) / total_h).clamp(0.0, 1.0) as f32 } else { 0.0 };
            let c = [rgba(cols[0], alpha), rgba(cols[1], alpha), rgba(cols[2], alpha), rgba(cols[3], alpha)];
            let mut out = [0.0; 4];
            for i in 0..4 {
                let top = c[0][i] + (c[1][i] - c[0][i]) * fx;
                let bot = c[3][i] + (c[2][i] - c[3][i]) * fx;
                out[i] = top + (bot - top) * fy;
            }
            out
        };
        let halign = self.gfx.halign;
        for (li, (line, lw)) in lines.iter().enumerate() {
            let lx0 = match halign {
                1 => -(*lw as f64 / 2.0).round(),
                2 => -(*lw as f64),
                _ => 0.0,
            };
            let base = y0 + li as f64 * lh + face.ascent as f64;
            let mut pen = lx0;
            for ch in line {
                let g = self.gfx.fonts.glyph(&face, *ch);
                if g.tex.w > 0 && g.tex.h > 0 {
                    let gx = (pen + g.xmin as f64).round();
                    let gy = (base + g.ytop as f64).round();
                    let (gw, gh) = (g.tex.w as f64, g.tex.h as f64);
                    let pts = [
                        to_room(gx, gy),
                        to_room(gx + gw, gy),
                        to_room(gx + gw, gy + gh),
                        to_room(gx, gy + gh),
                    ];
                    let cs = [
                        col_at(gx, gy),
                        col_at(gx + gw, gy),
                        col_at(gx + gw, gy + gh),
                        col_at(gx, gy + gh),
                    ];
                    let uv = [(0.0, 0.0), (gw as f32, 0.0), (gw as f32, gh as f32), (0.0, gh as f32)];
                    let tex: Rc<Texture> = g.tex.clone();
                    let mut p = self.gfx.painter();
                    p.bilinear = xs != 1.0 || ys != 1.0 || angle != 0.0;
                    p.quad_tex(&tex, pts, uv, cs, false);
                }
                pen += g.advance as f64;
            }
        }
    }

    /// `draw_sprite_ext` semantics.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_sprite_ext(&mut self, spr: i32, sub: f64, x: f64, y: f64, xs: f64, ys: f64, rot: f64, col: i32, alpha: f64) {
        let Some(s) = self.sprite(spr) else { return };
        let n = s.frames.len();
        if n == 0 {
            return;
        }
        let f = (sub.floor() as i64).rem_euclid(n as i64) as usize;
        let tex = s.frames[f].clone();
        let (ox, oy) = (s.origin_x as f64, s.origin_y as f64);
        self.gfx.texture_repeat = false;
        self.gfx.draw_tex(
            &tex,
            0.0,
            0.0,
            tex.w as f64,
            tex.h as f64,
            ox,
            oy,
            x,
            y,
            xs,
            ys,
            rot,
            [col; 4],
            alpha,
        );
    }

    /// Draws a background layer over the visible area (§13.1 step 3.2).
    fn draw_layer(&mut self, i: usize, area: (f64, f64, f64, f64)) {
        let b = self.rs.backgrounds[i].clone();
        let Some(bg) = self.background(b.index) else { return };
        let tex = bg.tex.clone();
        let (tw, th) = (tex.w as f64 * b.xscale, tex.h as f64 * b.yscale);
        if tw <= 0.0 || th <= 0.0 {
            return;
        }
        let (ax0, ay0, ax1, ay1) = area;
        let xs: Vec<f64> = if b.htiled {
            let mut start = b.x - ((b.x - ax0) / tw).ceil() * tw;
            if start > ax0 {
                start -= tw;
            }
            let mut v = Vec::new();
            let mut xx = start;
            while xx < ax1 {
                v.push(xx);
                xx += tw;
            }
            v
        } else {
            vec![b.x]
        };
        let ys: Vec<f64> = if b.vtiled {
            let mut start = b.y - ((b.y - ay0) / th).ceil() * th;
            if start > ay0 {
                start -= th;
            }
            let mut v = Vec::new();
            let mut yy = start;
            while yy < ay1 {
                v.push(yy);
                yy += th;
            }
            v
        } else {
            vec![b.y]
        };
        self.gfx.texture_repeat = false;
        for &yy in &ys {
            for &xx in &xs {
                self.gfx.draw_tex(
                    &tex,
                    0.0,
                    0.0,
                    tex.w as f64,
                    tex.h as f64,
                    0.0,
                    0.0,
                    xx,
                    yy,
                    b.xscale,
                    b.yscale,
                    0.0,
                    [b.blend; 4],
                    b.alpha,
                );
            }
        }
    }

    fn draw_tile(&mut self, t: &crate::room::TileState) {
        let Some(bg) = self.background(t.background) else { return };
        let tex = bg.tex.clone();
        self.gfx.draw_tex(
            &tex, t.left, t.top, t.width, t.height, 0.0, 0.0, t.x, t.y, t.xscale, t.yscale, 0.0, [t.blend; 4], t.alpha,
        );
    }

    /// One drawing pass (§13.1 step 3).
    fn draw_pass(&mut self, vm: &mut Vm, area: (f64, f64, f64, f64)) -> Result<(), Error> {
        if self.rs.show_color {
            let c = rgba8(self.rs.bg_color, 1.0);
            self.gfx.painter().clear_clip(c);
        }
        for i in 0..8 {
            let b = &self.rs.backgrounds[i];
            if b.visible && !b.foreground {
                self.draw_layer(i, area);
            }
        }
        // depth-sorted list: (depth, kind, index); kind 0 tile, 1 instance, 2 particles
        let mut items: Vec<(f64, u8, usize, i32)> = Vec::new();
        for (k, t) in self.rs.tiles.iter().enumerate() {
            if t.visible {
                items.push((t.depth, 0, k, t.id));
            }
        }
        for (k, i) in self.instances.iter().enumerate() {
            if i.alive() && i.visible {
                items.push((i.depth, 1, k, i.id));
            }
        }
        for (k, d) in self.particles.drawable_systems() {
            items.push((d, 2, k, k as i32));
        }
        items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        for (_, kind, k, id) in items {
            match kind {
                0 => {
                    if let Some(t) = self.rs.tiles.get(k).cloned() {
                        self.draw_tile(&t);
                    }
                }
                1 => {
                    let Some(i) = self.inst(id) else { continue };
                    if !i.alive() || !i.visible {
                        continue;
                    }
                    let obj = i.object_index;
                    if let Some((owner, list)) = self.handler(obj, ev::DRAW, 0) {
                        self.run_handler(vm, id, owner, &list, ev::DRAW, 0, id)?;
                    } else {
                        let i = self.inst(id).unwrap();
                        let (s, ii, x, y, xs, ys, a, c, al) = (
                            i.sprite_index,
                            i.image_index,
                            i.x,
                            i.y,
                            i.image_xscale,
                            i.image_yscale,
                            i.image_angle,
                            i.image_blend,
                            i.image_alpha,
                        );
                        self.draw_sprite_ext(s, ii, x, y, xs, ys, a, c, al);
                    }
                }
                _ => self.draw_particle_system(k as i32),
            }
        }
        for i in 0..8 {
            let b = &self.rs.backgrounds[i];
            if b.visible && b.foreground {
                self.draw_layer(i, area);
            }
        }
        if self.cursor_sprite >= 0 {
            let (mx, my) = self.mouse_room_pos();
            let sub = (self.frame as f64) * 1.0;
            let cs = self.cursor_sprite;
            self.draw_sprite_ext(cs, sub, mx, my, 1.0, 1.0, 0.0, 0xFFFFFF, 1.0);
        }
        Ok(())
    }

    /// Renders the whole frame into the back buffer (§13.1).
    pub fn draw_frame(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let saved_target = self.gfx.target;
        self.gfx.target = -1;
        let outside = rgba8(self.settings.color_outside_room as i32, 1.0);
        self.gfx.back.fill(outside);
        let full = Clip::full(&self.gfx.back);
        let use_views = self.rs.views_enabled && self.rs.views.iter().any(|v| v.visible);
        if use_views {
            for v in 0..8 {
                if !self.rs.views[v].visible {
                    continue;
                }
                let vs = self.rs.views[v].clone();
                self.rs.view_current = v as i32;
                self.gfx.pass_xf = Xf::view(
                    vs.xview, vs.yview, vs.wview, vs.hview, vs.angle, vs.xport, vs.yport, vs.wport, vs.hport,
                );
                self.gfx.pass_clip = Clip {
                    x0: vs.xport as i32,
                    y0: vs.yport as i32,
                    x1: (vs.xport + vs.wport) as i32,
                    y1: (vs.yport + vs.hport) as i32,
                }
                .intersect(full);
                // visible room area (rotation: use the bounding square)
                let ext = if vs.angle != 0.0 { (vs.wview.max(vs.hview)) / 2.0 } else { 0.0 };
                let area = (vs.xview - ext, vs.yview - ext, vs.xview + vs.wview + ext, vs.yview + vs.hview + ext);
                let r = self.draw_pass(vm, area);
                if r.is_err() {
                    self.gfx.pass_xf = Xf::IDENTITY;
                    self.gfx.pass_clip = full;
                    return r;
                }
            }
        } else {
            self.rs.view_current = 0;
            let sx = self.region_w as f64 / self.rs.width.max(1) as f64;
            let sy = self.region_h as f64 / self.rs.height.max(1) as f64;
            self.gfx.pass_xf = Xf {
                a: sx,
                b: 0.0,
                c: 0.0,
                d: sy,
                tx: 0.0,
                ty: 0.0,
            };
            self.gfx.pass_clip = full;
            let area = (0.0, 0.0, self.rs.width as f64, self.rs.height as f64);
            self.draw_pass(vm, area)?;
        }
        // drawing outside the draw event uses plain room coordinates
        self.gfx.pass_xf = Xf::IDENTITY;
        self.gfx.pass_clip = full;
        self.gfx.target = if saved_target >= 0 { saved_target } else { -1 };
        Ok(())
    }

    /// Plays a room transition between the old frame and the new room (§3.4).
    pub fn play_transition(&mut self, vm: &mut Vm, old: Canvas, kind: i32) -> Result<(), Error> {
        if self.gfx.present.is_none() || kind == 0 {
            return Ok(());
        }
        self.draw_frame(vm)?;
        let new = self.gfx.back.clone();
        let steps = self.transition_steps.max(1);
        let (w, h) = (new.w as f64, new.h as f64);
        let dt = std::time::Duration::from_secs_f64(1.0 / self.rs.speed.max(1.0));
        let script = self.transitions.get(&kind).copied();
        for k in 1..=steps {
            let f = k as f64 / steps as f64;
            let t0 = std::time::Instant::now();
            if let Some(scr) = script {
                // user transition: surfaces with old/new images
                let sa = self.alloc_surface(old.clone());
                let sb = self.alloc_surface(new.clone());
                let args = [
                    gml::Value::Real(sa as f64),
                    gml::Value::Real(sb as f64),
                    gml::Value::Real(w),
                    gml::Value::Real(h),
                    gml::Value::Real(f),
                ];
                let ctx = gml::Ctx::new(gml::special::NOONE, gml::special::NOONE);
                let r = <Game as gml::Host>::call_script(self, vm, &ctx, scr, &args);
                self.gfx.surfaces[sa as usize] = None;
                self.gfx.surfaces[sb as usize] = None;
                r?;
            } else {
                let mut out = old.clone();
                let blit = |out: &mut Canvas, src: &Canvas, dx: i32, dy: i32, sx: i32, sy: i32, cw: i32, ch: i32| {
                    out.blit(src, sx, sy, cw, ch, dx, dy);
                };
                let (wi, hi) = (new.w as i32, new.h as i32);
                match kind {
                    1 => { let cw = (w * f) as i32; blit(&mut out, &new, 0, 0, 0, 0, cw, hi); }
                    2 => { let cw = (w * f) as i32; blit(&mut out, &new, wi - cw, 0, wi - cw, 0, cw, hi); }
                    3 => { let ch = (h * f) as i32; blit(&mut out, &new, 0, 0, 0, 0, wi, ch); }
                    4 => { let ch = (h * f) as i32; blit(&mut out, &new, 0, hi - ch, 0, hi - ch, wi, ch); }
                    5 | 10..=13 | 18 | 19 => {
                        let (cw, ch) = ((w * f) as i32, (h * f) as i32);
                        blit(&mut out, &new, (wi - cw) / 2, (hi - ch) / 2, (wi - cw) / 2, (hi - ch) / 2, cw, ch);
                    }
                    6 => { let cw = (w * f) as i32; blit(&mut out, &new, cw - wi, 0, 0, 0, wi, hi); }
                    7 => { let cw = (w * f) as i32; blit(&mut out, &new, wi - cw, 0, 0, 0, wi, hi); }
                    8 => { let ch = (h * f) as i32; blit(&mut out, &new, 0, ch - hi, 0, 0, wi, hi); }
                    9 => { let ch = (h * f) as i32; blit(&mut out, &new, 0, hi - ch, 0, 0, wi, hi); }
                    14 => { let cw = (w * f) as i32; blit(&mut out, &old, cw, 0, 0, 0, wi, hi); blit(&mut out, &new, cw - wi, 0, 0, 0, wi, hi); }
                    15 => { let cw = (w * f) as i32; blit(&mut out, &old, -cw, 0, 0, 0, wi, hi); blit(&mut out, &new, wi - cw, 0, 0, 0, wi, hi); }
                    16 => { let ch = (h * f) as i32; blit(&mut out, &old, 0, ch, 0, 0, wi, hi); blit(&mut out, &new, 0, ch - hi, 0, 0, wi, hi); }
                    17 => { let ch = (h * f) as i32; blit(&mut out, &old, 0, -ch, 0, 0, wi, hi); blit(&mut out, &new, 0, hi - ch, 0, 0, wi, hi); }
                    21 => {
                        // fade out then in
                        let (src, a) = if f < 0.5 { (&old, 1.0 - f * 2.0) } else { (&new, (f - 0.5) * 2.0) };
                        for (o, s) in out.px.iter_mut().zip(src.px.iter()) {
                            *o = [(s[0] as f64 * a) as u8, (s[1] as f64 * a) as u8, (s[2] as f64 * a) as u8, 255];
                        }
                    }
                    _ => {
                        for (o, s) in out.px.iter_mut().zip(new.px.iter()) {
                            for c in 0..3 {
                                o[c] = (o[c] as f64 * (1.0 - f) + s[c] as f64 * f) as u8;
                            }
                        }
                    }
                }
                self.gfx.back = out;
            }
            self.gfx.present_now();
            let el = t0.elapsed();
            if el < dt {
                std::thread::sleep(dt - el);
            }
        }
        Ok(())
    }

    /// Stores a canvas as a new surface; returns its id.
    pub fn alloc_surface(&mut self, c: Canvas) -> i32 {
        if let Some(p) = self.gfx.surfaces.iter().position(|s| s.is_none()) {
            self.gfx.surfaces[p] = Some(c);
            p as i32
        } else {
            self.gfx.surfaces.push(Some(c));
            (self.gfx.surfaces.len() - 1) as i32
        }
    }
}
