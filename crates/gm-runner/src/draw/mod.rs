//! Software rasteriser: textures/canvases, triangles with per-vertex colour and
//! optional texture, and the D3D-style blend equation used by GameMaker
//! (runtime.md §13.4).

pub mod backend;
pub mod dual;
pub mod font;
pub mod gl;
pub mod mat;
pub mod soft;
pub mod painter;

pub use backend::{Backend, Prim, RState, Target, Tex, Vertex};
pub use painter::{Painter, Pipe};

/// An RGBA image (sprite frame, background, surface or back buffer).
#[derive(Clone, Debug, Default)]
pub struct Texture {
    pub w: u32,
    pub h: u32,
    pub px: Vec<[u8; 4]>,
}

/// Canvases are plain textures.
pub type Canvas = Texture;

impl Texture {
    pub fn new(w: u32, h: u32) -> Self {
        Texture {
            w,
            h,
            px: vec![[0, 0, 0, 0]; (w as usize) * (h as usize)],
        }
    }

    pub fn from_rgba(img: &gm_format::RgbaImage) -> Self {
        let mut px = Vec::with_capacity((img.width * img.height) as usize);
        for c in img.data.chunks_exact(4) {
            px.push([c[0], c[1], c[2], c[3]]);
        }
        px.resize((img.width * img.height) as usize, [0, 0, 0, 0]);
        Texture {
            w: img.width,
            h: img.height,
            px,
        }
    }

    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        self.px.iter().flat_map(|p| p.iter().copied()).collect()
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        self.px[(y * self.w + x) as usize]
    }

    pub fn fill(&mut self, c: [u8; 4]) {
        for p in self.px.iter_mut() {
            *p = c;
        }
    }

    /// Copies a part of `src` into self at (dx,dy) without blending.
    pub fn blit(&mut self, src: &Texture, sx: i32, sy: i32, w: i32, h: i32, dx: i32, dy: i32) {
        for y in 0..h {
            let (syy, dyy) = (sy + y, dy + y);
            if syy < 0 || dyy < 0 || syy >= src.h as i32 || dyy >= self.h as i32 {
                continue;
            }
            for x in 0..w {
                let (sxx, dxx) = (sx + x, dx + x);
                if sxx < 0 || dxx < 0 || sxx >= src.w as i32 || dxx >= self.w as i32 {
                    continue;
                }
                self.px[(dyy as u32 * self.w + dxx as u32) as usize] =
                    src.px[(syy as u32 * src.w + sxx as u32) as usize];
            }
        }
    }

    /// Writes the texture as a PNG file.
    pub fn save_png(&self, path: &std::path::Path, opaque: bool) -> std::io::Result<()> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, self.w.max(1), self.h.max(1));
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(std::io::Error::other)?;
        let mut data = self.to_rgba_bytes();
        if opaque {
            for c in data.chunks_exact_mut(4) {
                c[3] = 255;
            }
        }
        if data.is_empty() {
            data = vec![0, 0, 0, 255];
        }
        writer.write_image_data(&data).map_err(std::io::Error::other)?;
        writer.finish().map_err(std::io::Error::other)?;
        crate::vfs::write(path, &out)
    }

    /// Loads an image file (PNG or BMP) as a texture.
    pub fn load_file(path: &std::path::Path) -> Option<Texture> {
        let bytes = crate::vfs::read(path).ok()?;
        Self::load_bytes(&bytes)
    }

    pub fn load_bytes(bytes: &[u8]) -> Option<Texture> {
        if bytes.starts_with(b"\x89PNG") {
            let dec = png::Decoder::new(std::io::Cursor::new(bytes));
            let mut reader = dec.read_info().ok()?;
            let mut buf = vec![0; reader.output_buffer_size()?];
            let info = reader.next_frame(&mut buf).ok()?;
            let (w, h) = (info.width, info.height);
            let mut t = Texture::new(w, h);
            let data = &buf[..info.buffer_size()];
            let bd = info.bit_depth;
            if bd != png::BitDepth::Eight {
                return None;
            }
            match info.color_type {
                png::ColorType::Rgba => {
                    for (i, c) in data.chunks_exact(4).enumerate() {
                        t.px[i] = [c[0], c[1], c[2], c[3]];
                    }
                }
                png::ColorType::Rgb => {
                    for (i, c) in data.chunks_exact(3).enumerate() {
                        t.px[i] = [c[0], c[1], c[2], 255];
                    }
                }
                png::ColorType::Grayscale => {
                    for (i, c) in data.iter().enumerate() {
                        t.px[i] = [*c, *c, *c, 255];
                    }
                }
                png::ColorType::GrayscaleAlpha => {
                    for (i, c) in data.chunks_exact(2).enumerate() {
                        t.px[i] = [c[0], c[0], c[0], c[1]];
                    }
                }
                png::ColorType::Indexed => return None,
            }
            Some(t)
        } else if bytes.starts_with(b"BM") {
            let img = gm_format::image::decode_bmp(bytes).ok()?;
            Some(Texture::from_rgba(&img))
        } else {
            None
        }
    }
}

/// A vertex in target pixel coordinates.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vtx {
    pub x: f32,
    pub y: f32,
    /// Texel coordinates (in texture pixels).
    pub u: f32,
    pub v: f32,
    /// RGBA, 0..1.
    pub c: [f32; 4],
}

/// Blend factors (`bm_zero`=1 … `bm_src_alpha_sat`=11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blend {
    pub src: u8,
    pub dst: u8,
}

impl Blend {
    pub const NORMAL: Blend = Blend { src: 5, dst: 6 };
    /// Blend mode for `draw_set_blend_mode(mode)`.
    pub fn from_mode(mode: i32) -> Blend {
        match mode {
            1 => Blend { src: 5, dst: 2 }, // add
            2 => Blend { src: 5, dst: 4 }, // max
            3 => Blend { src: 1, dst: 4 }, // subtract
            _ => Blend::NORMAL,
        }
    }
}

/// Texture sampling options.
#[derive(Clone, Copy, Debug)]
pub struct Sampler<'a> {
    pub tex: &'a Texture,
    pub bilinear: bool,
    pub repeat: bool,
}

impl Sampler<'_> {
    #[inline]
    fn texel(&self, x: i32, y: i32) -> [f32; 4] {
        let t = self.tex;
        let (w, h) = (t.w as i32, t.h as i32);
        let (x, y) = if self.repeat {
            (x.rem_euclid(w), y.rem_euclid(h))
        } else {
            (x.clamp(0, w - 1), y.clamp(0, h - 1))
        };
        let p = t.px[(y * w + x) as usize];
        [
            p[0] as f32 / 255.0,
            p[1] as f32 / 255.0,
            p[2] as f32 / 255.0,
            p[3] as f32 / 255.0,
        ]
    }

    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        if self.tex.w == 0 || self.tex.h == 0 {
            return [0.0; 4];
        }
        if !self.bilinear {
            return self.texel(u.floor() as i32, v.floor() as i32);
        }
        let (fu, fv) = (u - 0.5, v - 0.5);
        let (x0, y0) = (fu.floor(), fv.floor());
        let (ax, ay) = (fu - x0, fv - y0);
        let (x0, y0) = (x0 as i32, y0 as i32);
        let a = self.texel(x0, y0);
        let b = self.texel(x0 + 1, y0);
        let c = self.texel(x0, y0 + 1);
        let d = self.texel(x0 + 1, y0 + 1);
        let mut out = [0.0; 4];
        for i in 0..4 {
            let top = a[i] + (b[i] - a[i]) * ax;
            let bot = c[i] + (d[i] - c[i]) * ax;
            out[i] = top + (bot - top) * ay;
        }
        out
    }
}

/// Clip rectangle in target pixels: x0, y0 inclusive; x1, y1 exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clip {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Clip {
    pub fn full(c: &Canvas) -> Clip {
        Clip {
            x0: 0,
            y0: 0,
            x1: c.w as i32,
            y1: c.h as i32,
        }
    }
    pub fn intersect(self, o: Clip) -> Clip {
        Clip {
            x0: self.x0.max(o.x0),
            y0: self.y0.max(o.y0),
            x1: self.x1.min(o.x1),
            y1: self.y1.min(o.y1),
        }
    }
}

#[inline]
fn factor(f: u8, s: [f32; 4], d: [f32; 4], alpha_channel: bool) -> [f32; 4] {
    match f {
        1 => [0.0; 4],
        2 => [1.0; 4],
        3 => s,
        4 => [1.0 - s[0], 1.0 - s[1], 1.0 - s[2], 1.0 - s[3]],
        5 => [s[3]; 4],
        6 => [1.0 - s[3]; 4],
        7 => [d[3]; 4],
        8 => [1.0 - d[3]; 4],
        9 => d,
        10 => [1.0 - d[0], 1.0 - d[1], 1.0 - d[2], 1.0 - d[3]],
        11 => {
            let f = s[3].min(1.0 - d[3]);
            if alpha_channel {
                [f, f, f, 1.0]
            } else {
                [f, f, f, 1.0]
            }
        }
        _ => [1.0; 4],
    }
}

/// Blends one source colour (0..1 floats) into a destination pixel.
#[inline]
pub fn blend_pixel(dst: &mut [u8; 4], s: [f32; 4], blend: Blend) {
    if blend == Blend::NORMAL {
        let a = s[3];
        if a <= 0.0 {
            return;
        }
        if a >= 1.0 {
            *dst = [
                (s[0] * 255.0 + 0.5) as u8,
                (s[1] * 255.0 + 0.5) as u8,
                (s[2] * 255.0 + 0.5) as u8,
                255,
            ];
            return;
        }
        let ia = 1.0 - a;
        let d = [
            dst[0] as f32 / 255.0,
            dst[1] as f32 / 255.0,
            dst[2] as f32 / 255.0,
            dst[3] as f32 / 255.0,
        ];
        *dst = [
            to_u8(s[0] * a + d[0] * ia),
            to_u8(s[1] * a + d[1] * ia),
            to_u8(s[2] * a + d[2] * ia),
            to_u8(a * a + d[3] * ia),
        ];
        return;
    }
    let d = [
        dst[0] as f32 / 255.0,
        dst[1] as f32 / 255.0,
        dst[2] as f32 / 255.0,
        dst[3] as f32 / 255.0,
    ];
    let fs = factor(blend.src, s, d, false);
    let fd = factor(blend.dst, s, d, false);
    *dst = [
        to_u8(s[0] * fs[0] + d[0] * fd[0]),
        to_u8(s[1] * fs[1] + d[1] * fd[1]),
        to_u8(s[2] * fs[2] + d[2] * fd[2]),
        to_u8(s[3] * fs[3] + d[3] * fd[3]),
    ];
}

#[inline]
fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

#[inline]
fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

/// Rasterises one triangle into `target`, sampling pixel centres, with a top-left
/// fill rule so that adjacent triangles do not overlap.
pub fn triangle(
    target: &mut Canvas,
    clip: Clip,
    blend: Blend,
    v: [Vtx; 3],
    tex: Option<Sampler>,
) {
    let clip = clip.intersect(Clip::full(target));
    if clip.x0 >= clip.x1 || clip.y0 >= clip.y1 {
        return;
    }
    let [mut a, mut b, c] = v;
    let mut area = edge(a.x, a.y, b.x, b.y, c.x, c.y);
    if !area.is_finite() || area.abs() < 1e-8 {
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
    // top-left fill rule. With the positive orientation established above (y down),
    // a top edge runs in +x with dy = 0 and a left edge has dy < 0.
    let tl = |p: &Vtx, q: &Vtx| -> bool {
        let (dx, dy) = (q.x - p.x, q.y - p.y);
        (dy == 0.0 && dx > 0.0) || dy < 0.0
    };
    let (t0, t1, t2) = (tl(&b, &c), tl(&c, &a), tl(&a, &b));
    let flat_color = a.c == b.c && b.c == c.c;
    let inv = 1.0 / area;
    let w = target.w as usize;
    #[inline]
    fn inside(w: f32, t: bool) -> bool {
        w > 0.0 || (w == 0.0 && t)
    }
    for y in miny..maxy {
        let py = y as f32 + 0.5;
        let row = y as usize * w;
        for x in minx..maxx {
            let px = x as f32 + 0.5;
            let w0 = edge(b.x, b.y, c.x, c.y, px, py);
            let w1 = edge(c.x, c.y, a.x, a.y, px, py);
            let w2 = edge(a.x, a.y, b.x, b.y, px, py);
            if !(inside(w0, t0) && inside(w1, t1) && inside(w2, t2)) {
                continue;
            }
            let (l0, l1, l2) = (w0 * inv, w1 * inv, w2 * inv);
            let mut col = if flat_color {
                a.c
            } else {
                [
                    a.c[0] * l0 + b.c[0] * l1 + c.c[0] * l2,
                    a.c[1] * l0 + b.c[1] * l1 + c.c[1] * l2,
                    a.c[2] * l0 + b.c[2] * l1 + c.c[2] * l2,
                    a.c[3] * l0 + b.c[3] * l1 + c.c[3] * l2,
                ]
            };
            if let Some(s) = &tex {
                let u = a.u * l0 + b.u * l1 + c.u * l2;
                let vv = a.v * l0 + b.v * l1 + c.v * l2;
                let t = s.sample(u, vv);
                col = [col[0] * t[0], col[1] * t[1], col[2] * t[2], col[3] * t[3]];
            }
            blend_pixel(&mut target.px[row + x as usize], col, blend);
        }
    }
}

/// Fast path: an axis-aligned rectangle [x0,x1)×[y0,y1) in target pixels with a
/// constant colour and an optional texture mapped linearly (u0,v0)-(u1,v1).
#[allow(clippy::too_many_arguments)]
pub fn rect_fast(
    target: &mut Canvas,
    clip: Clip,
    blend: Blend,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    uv: (f32, f32, f32, f32),
    col: [f32; 4],
    tex: Option<Sampler>,
) {
    let clip = clip.intersect(Clip::full(target));
    let (x0, x1) = (x0.min(x1), x0.max(x1));
    let (y0, y1) = (y0.min(y1), y0.max(y1));
    // pixels whose centre lies in [x0, x1)
    let px0 = ((x0 - 0.5).ceil() as i32).max(clip.x0);
    let px1 = ((x1 - 0.5).ceil() as i32).min(clip.x1);
    let py0 = ((y0 - 0.5).ceil() as i32).max(clip.y0);
    let py1 = ((y1 - 0.5).ceil() as i32).min(clip.y1);
    if px0 >= px1 || py0 >= py1 {
        return;
    }
    let (u0, v0, u1, v1) = uv;
    let du = if x1 > x0 { (u1 - u0) / (x1 - x0) } else { 0.0 };
    let dv = if y1 > y0 { (v1 - v0) / (y1 - y0) } else { 0.0 };
    let w = target.w as usize;
    // bilinear sampling of an unscaled, pixel-aligned texture equals nearest sampling
    let tex = tex.map(|mut s| {
        if s.bilinear && du == 1.0 && dv == 1.0 && (u0 - x0).fract() == 0.0 && (v0 - y0).fract() == 0.0 {
            s.bilinear = false;
        }
        s
    });
    let white_opaque = col == [1.0, 1.0, 1.0, 1.0];
    let ca = (col[3] * 256.0) as u32;
    let cr = (col[0] * 256.0) as u32;
    let cg = (col[1] * 256.0) as u32;
    let cb = (col[2] * 256.0) as u32;
    for py in py0..py1 {
        let v = v0 + (py as f32 + 0.5 - y0) * dv;
        let row = py as usize * w;
        for px in px0..px1 {
            let dst = &mut target.px[row + px as usize];
            match &tex {
                Some(s) if !s.bilinear && blend == Blend::NORMAL => {
                    let u = u0 + (px as f32 + 0.5 - x0) * du;
                    let t = s.tex;
                    let (tw, th) = (t.w as i32, t.h as i32);
                    let (mut tx, mut ty) = (u.floor() as i32, v.floor() as i32);
                    if s.repeat {
                        tx = tx.rem_euclid(tw);
                        ty = ty.rem_euclid(th);
                    } else {
                        tx = tx.clamp(0, tw - 1);
                        ty = ty.clamp(0, th - 1);
                    }
                    let p = t.px[(ty * tw + tx) as usize];
                    if white_opaque {
                        match p[3] {
                            0 => {}
                            255 => *dst = p,
                            a => {
                                let a = a as u32;
                                let ia = 255 - a;
                                *dst = [
                                    ((p[0] as u32 * a + dst[0] as u32 * ia + 127) / 255) as u8,
                                    ((p[1] as u32 * a + dst[1] as u32 * ia + 127) / 255) as u8,
                                    ((p[2] as u32 * a + dst[2] as u32 * ia + 127) / 255) as u8,
                                    ((a * a + dst[3] as u32 * ia + 127) / 255) as u8,
                                ];
                            }
                        }
                    } else {
                        let a = (p[3] as u32 * ca) >> 8;
                        if a == 0 {
                            continue;
                        }
                        let r = (p[0] as u32 * cr) >> 8;
                        let g = (p[1] as u32 * cg) >> 8;
                        let b = (p[2] as u32 * cb) >> 8;
                        let a = a.min(255);
                        let ia = 255 - a;
                        *dst = [
                            ((r.min(255) * a + dst[0] as u32 * ia + 127) / 255) as u8,
                            ((g.min(255) * a + dst[1] as u32 * ia + 127) / 255) as u8,
                            ((b.min(255) * a + dst[2] as u32 * ia + 127) / 255) as u8,
                            ((a * a + dst[3] as u32 * ia + 127) / 255) as u8,
                        ];
                    }
                }
                Some(s) => {
                    let u = u0 + (px as f32 + 0.5 - x0) * du;
                    let t = s.sample(u, v);
                    blend_pixel(dst, [col[0] * t[0], col[1] * t[1], col[2] * t[2], col[3] * t[3]], blend);
                }
                None => blend_pixel(dst, col, blend),
            }
        }
    }
}

/// Converts a GM colour (BGR integer) and alpha to float RGBA.
#[inline]
pub fn rgba(col: i32, alpha: f64) -> [f32; 4] {
    [
        (col & 0xFF) as f32 / 255.0,
        ((col >> 8) & 0xFF) as f32 / 255.0,
        ((col >> 16) & 0xFF) as f32 / 255.0,
        alpha.clamp(0.0, 1.0) as f32,
    ]
}

/// GM colour → 8-bit RGBA.
#[inline]
pub fn rgba8(col: i32, alpha: f64) -> [u8; 4] {
    [
        (col & 0xFF) as u8,
        ((col >> 8) & 0xFF) as u8,
        ((col >> 16) & 0xFF) as u8,
        (alpha.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(c: &mut Canvas, x0: f32, y0: f32, x1: f32, y1: f32) {
        let col = [1.0, 0.0, 0.0, 1.0];
        let v = |x, y| Vtx { x, y, u: 0.0, v: 0.0, c: col };
        let clip = Clip::full(c);
        triangle(c, clip, Blend::NORMAL, [v(x0, y0), v(x1, y0), v(x1, y1)], None);
        triangle(c, clip, Blend::NORMAL, [v(x0, y0), v(x1, y1), v(x0, y1)], None);
    }

    #[test]
    fn quad_covers_exact_pixels() {
        let mut c = Canvas::new(8, 8);
        quad(&mut c, 2.0, 2.0, 5.0, 4.0);
        let n = c.px.iter().filter(|p| p[3] == 255).count();
        assert_eq!(n, 6);
        assert_eq!(c.get(2, 2)[0], 255);
        assert_eq!(c.get(4, 3)[0], 255);
        assert_eq!(c.get(5, 3)[0], 0);
        let mut c = Canvas::new(8, 8);
        quad(&mut c, 2.5, 2.5, 4.5, 3.5);
        let n = c.px.iter().filter(|p| p[3] == 255).count();
        assert_eq!(n, 2);
        assert_eq!(c.get(2, 2)[0], 255);
        assert_eq!(c.get(3, 2)[0], 255);
    }

    #[test]
    fn blend_add() {
        let mut d = [100, 100, 100, 255];
        blend_pixel(&mut d, [0.5, 0.5, 0.5, 1.0], Blend::from_mode(1));
        assert_eq!(d[0], 228);
    }
}
