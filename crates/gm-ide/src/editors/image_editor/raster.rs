//! Pixel-level drawing primitives of the image editor (specs/ide.md §7): coverage
//! masks for strokes and shapes, compositing, flood fill and colour replacement.
//!
//! Strokes are drawn into a [`Mask`] and composited once onto the image as it was when
//! the stroke started, so semi-transparent colours do not accumulate where a stroke
//! overlaps itself.

use gm_format::RgbaImage;

/// An RGBA colour (not premultiplied).
pub type Rgba = [u8; 4];

/// How a colour is put onto the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Alpha-blend over the existing pixel.
    #[default]
    Blend,
    /// Replace the pixel (including its alpha).
    Replace,
}

/// Pixel access helpers on [`RgbaImage`] with signed, bounds-checked coordinates.
pub trait Px {
    fn get(&self, x: i32, y: i32) -> Option<Rgba>;
    fn put(&mut self, x: i32, y: i32, c: Rgba);
    fn in_bounds(&self, x: i32, y: i32) -> bool;
}

impl Px for RgbaImage {
    fn get(&self, x: i32, y: i32) -> Option<Rgba> {
        self.in_bounds(x, y).then(|| self.pixel(x as u32, y as u32))
    }
    fn put(&mut self, x: i32, y: i32, c: Rgba) {
        if self.in_bounds(x, y) {
            let i = (y as usize * self.width as usize + x as usize) * 4;
            self.data[i..i + 4].copy_from_slice(&c);
        }
    }
    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }
}

/// Source-over compositing of `src` onto `dst` (both straight alpha).
pub fn blend_over(dst: Rgba, src: Rgba) -> Rgba {
    let sa = src[3] as u32;
    if sa == 0 {
        return dst;
    }
    if sa == 255 {
        return src;
    }
    let da = dst[3] as u32;
    // out_a = sa + da * (1 - sa), in 0..=255*255
    let oa = sa * 255 + da * (255 - sa);
    if oa == 0 {
        return [0, 0, 0, 0];
    }
    let mut out = [0u8; 4];
    for k in 0..3 {
        let v = (src[k] as u32 * sa * 255 + dst[k] as u32 * da * (255 - sa) + oa / 2) / oa;
        out[k] = v.min(255) as u8;
    }
    out[3] = ((oa + 127) / 255).min(255) as u8;
    out
}

/// Put `c` with coverage `cov` (0..=255) onto `dst` according to `mode`.
pub fn apply_color(dst: Rgba, c: Rgba, cov: u8, mode: ColorMode) -> Rgba {
    if cov == 0 {
        return dst;
    }
    match mode {
        ColorMode::Blend => {
            let a = (c[3] as u32 * cov as u32 + 127) / 255;
            blend_over(dst, [c[0], c[1], c[2], a as u8])
        }
        ColorMode::Replace => {
            if cov == 255 {
                c
            } else {
                lerp(dst, c, cov as f32 / 255.0)
            }
        }
    }
}

/// Linear interpolation between two colours (t = 0 gives `a`).
pub fn lerp(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let mut o = [0u8; 4];
    for k in 0..4 {
        o[k] = (a[k] as f32 + (b[k] as f32 - a[k] as f32) * t).round() as u8;
    }
    o
}

/// Largest per-channel difference between two colours; `alpha` includes alpha.
pub fn color_distance(a: Rgba, b: Rgba, alpha: bool) -> u8 {
    let n = if alpha { 4 } else { 3 };
    (0..n).map(|k| a[k].abs_diff(b[k])).max().unwrap_or(0)
}

/// Per-pixel coverage (0..=255) of the size of an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Mask {
    pub fn new(width: u32, height: u32) -> Self {
        Mask {
            width,
            height,
            data: vec![0; width as usize * height as usize],
        }
    }

    pub fn for_image(img: &RgbaImage) -> Self {
        Mask::new(img.width, img.height)
    }

    #[allow(dead_code)]
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            self.data[y as usize * self.width as usize + x as usize]
        } else {
            0
        }
    }

    /// Raise the coverage at (x, y) to at least `v`.
    pub fn set_max(&mut self, x: i32, y: i32, v: u8) {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            let i = y as usize * self.width as usize + x as usize;
            self.data[i] = self.data[i].max(v);
        }
    }

    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    pub fn is_empty(&self) -> bool {
        self.data.iter().all(|&v| v == 0)
    }

    /// Square brush of `size` pixels centred on (x, y); sizes > 2 are rounded.
    pub fn dab(&mut self, x: i32, y: i32, size: u32) {
        let size = size.max(1) as i32;
        let lo = -(size - 1) / 2;
        let hi = lo + size - 1;
        let r = size as f32 / 2.0;
        for dy in lo..=hi {
            for dx in lo..=hi {
                if size > 2 {
                    // centre of the pixel relative to the brush centre
                    let cx = dx as f32 - (lo + hi) as f32 / 2.0;
                    let cy = dy as f32 - (lo + hi) as f32 / 2.0;
                    if cx * cx + cy * cy > r * r {
                        continue;
                    }
                }
                self.set_max(x + dx, y + dy, 255);
            }
        }
    }

    /// Line of brush dabs from `a` to `b`.
    pub fn line(&mut self, a: (i32, i32), b: (i32, i32), size: u32) {
        for (x, y) in line_points(a, b) {
            self.dab(x, y, size);
        }
    }

    /// Axis-aligned rectangle between two corners (inclusive); outline of
    /// `width` pixels, or filled.
    pub fn rect(&mut self, a: (i32, i32), b: (i32, i32), width: u32, filled: bool) {
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        let w = width.max(1) as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let edge = x - x0 < w || x1 - x < w || y - y0 < w || y1 - y < w;
                if filled || edge {
                    self.set_max(x, y, 255);
                }
            }
        }
    }

    /// Ellipse inscribed in the rectangle between two corners (inclusive).
    pub fn ellipse(&mut self, a: (i32, i32), b: (i32, i32), width: u32, filled: bool) {
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        let cx = (x0 + x1) as f32 / 2.0;
        let cy = (y0 + y1) as f32 / 2.0;
        let rx = (x1 - x0) as f32 / 2.0 + 0.5;
        let ry = (y1 - y0) as f32 / 2.0 + 0.5;
        let w = width.max(1) as f32;
        let (irx, iry) = (rx - w, ry - w);
        let inside = |x: f32, y: f32, rx: f32, ry: f32| {
            rx > 0.0 && ry > 0.0 && {
                let (dx, dy) = ((x - cx) / rx, (y - cy) / ry);
                dx * dx + dy * dy <= 1.0
            }
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (fx, fy) = (x as f32, y as f32);
                if inside(fx, fy, rx, ry) && (filled || !inside(fx, fy, irx, iry)) {
                    self.set_max(x, y, 255);
                }
            }
        }
    }

    /// Random dots within a disk of diameter `size` around (x, y); `seed` is advanced.
    pub fn spray(&mut self, x: i32, y: i32, size: u32, dots: u32, seed: &mut u64) {
        let r = (size.max(1) as f32) / 2.0;
        for _ in 0..dots {
            let a = rand01(seed) * std::f32::consts::TAU;
            let d = rand01(seed).sqrt() * r;
            let (px, py) = (x as f32 + a.cos() * d, y as f32 + a.sin() * d);
            self.set_max(px.round() as i32, py.round() as i32, 255);
        }
    }

    /// Paint coverage values of a glyph bitmap at (x, y).
    pub fn blit_coverage(&mut self, x: i32, y: i32, w: usize, cov: &[u8]) {
        if w == 0 {
            return;
        }
        for (i, &v) in cov.iter().enumerate() {
            self.set_max(x + (i % w) as i32, y + (i / w) as i32, v);
        }
    }
}

/// Xorshift step returning a float in [0, 1).
pub fn rand01(seed: &mut u64) -> f32 {
    let mut s = *seed | 1;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    *seed = s;
    ((s >> 40) as f32) / (1u64 << 24) as f32
}

/// Points of a Bresenham line from `a` to `b` (inclusive).
pub fn line_points(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = a;
    let dx = (b.0 - a.0).abs();
    let dy = -(b.1 - a.1).abs();
    let sx = if a.0 < b.0 { 1 } else { -1 };
    let sy = if a.1 < b.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut pts = Vec::with_capacity((dx - dy) as usize + 1);
    loop {
        pts.push((x, y));
        if x == b.0 && y == b.1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    pts
}

/// Snap `b` so that the segment a→b is horizontal, vertical or diagonal.
pub fn constrain_45(a: (i32, i32), b: (i32, i32)) -> (i32, i32) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (ax, ay) = (dx.abs(), dy.abs());
    if ax > 2 * ay {
        (b.0, a.1)
    } else if ay > 2 * ax {
        (a.0, b.1)
    } else {
        let m = ax.max(ay);
        (a.0 + m * dx.signum(), a.1 + m * dy.signum())
    }
}

/// Snap `b` so that the box a→b is square.
pub fn constrain_square(a: (i32, i32), b: (i32, i32)) -> (i32, i32) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let m = dx.abs().max(dy.abs());
    let sx = if dx < 0 { -1 } else { 1 };
    let sy = if dy < 0 { -1 } else { 1 };
    (a.0 + m * sx, a.1 + m * sy)
}

/// Composite `mask` coverage with colour `c` onto a copy of `base`.
#[allow(dead_code)]
pub fn composite(base: &RgbaImage, mask: &Mask, c: Rgba, mode: ColorMode) -> RgbaImage {
    let mut out = base.clone();
    composite_into(&mut out, mask, c, mode);
    out
}

/// Composite `mask` coverage with colour `c` onto `img` in place.
pub fn composite_into(img: &mut RgbaImage, mask: &Mask, c: Rgba, mode: ColorMode) {
    debug_assert_eq!((img.width, img.height), (mask.width, mask.height));
    for (px, &cov) in img.data.as_chunks_mut::<4>().0.iter_mut().zip(&mask.data) {
        if cov > 0 {
            let d = [px[0], px[1], px[2], px[3]];
            px.copy_from_slice(&apply_color(d, c, cov, mode));
        }
    }
}

/// Reduce alpha under the mask by `strength` (0..=255).
pub fn erase_into(img: &mut RgbaImage, mask: &Mask, strength: u8) {
    for (px, &cov) in img.data.as_chunks_mut::<4>().0.iter_mut().zip(&mask.data) {
        if cov > 0 {
            let amt = (strength as u32 * cov as u32 + 127) / 255;
            px[3] = (px[3] as u32).saturating_sub(amt) as u8;
            if px[3] == 0 {
                px[..3].fill(0);
            }
        }
    }
}

/// Flood fill of the area 4-connected to (x, y) whose colour is within `tolerance` of
/// the start pixel. Returns false if (x, y) is outside the image.
pub fn flood_fill(
    img: &mut RgbaImage,
    x: i32,
    y: i32,
    c: Rgba,
    tolerance: u8,
    alpha: bool,
    mode: ColorMode,
) -> bool {
    let Some(target) = img.get(x, y) else {
        return false;
    };
    let mut mask = Mask::for_image(img);
    let (w, h) = (img.width as i32, img.height as i32);
    let mut seen = vec![false; (w * h) as usize];
    let matches = |p: Rgba| color_distance(p, target, alpha) <= tolerance;
    let mut stack = vec![(x, y)];
    seen[(y * w + x) as usize] = true;
    while let Some((px, py)) = stack.pop() {
        mask.data[(py * w + px) as usize] = 255;
        for (nx, ny) in [(px - 1, py), (px + 1, py), (px, py - 1), (px, py + 1)] {
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let i = (ny * w + nx) as usize;
            if !seen[i] && matches(img.pixel(nx as u32, ny as u32)) {
                seen[i] = true;
                stack.push((nx, ny));
            }
        }
    }
    composite_into(img, &mask, c, mode);
    true
}

/// Replace every pixel within `tolerance` of `from` by `to` (the "Change Color" tool).
pub fn replace_color(img: &mut RgbaImage, from: Rgba, to: Rgba, tolerance: u8, alpha: bool) {
    for px in img.data.as_chunks_mut::<4>().0 {
        let p = [px[0], px[1], px[2], px[3]];
        if color_distance(p, from, alpha) <= tolerance {
            px.copy_from_slice(&to);
        }
    }
}

/// Copy `src` onto `dst` at (x, y), blending or replacing.
pub fn paste(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32, mode: ColorMode) {
    for sy in 0..src.height as i32 {
        for sx in 0..src.width as i32 {
            let (dx, dy) = (x + sx, y + sy);
            if let Some(d) = dst.get(dx, dy) {
                let s = src.pixel(sx as u32, sy as u32);
                dst.put(dx, dy, apply_color(d, s, 255, mode));
            }
        }
    }
}

/// Clear a rectangle (x, y, w, h) to transparent.
pub fn clear_rect(img: &mut RgbaImage, r: (i32, i32, u32, u32)) {
    for y in r.1..r.1 + r.3 as i32 {
        for x in r.0..r.0 + r.2 as i32 {
            img.put(x, y, [0, 0, 0, 0]);
        }
    }
}

/// Copy rectangle (x, y, w, h); pixels outside the image are transparent.
pub fn copy_rect(img: &RgbaImage, r: (i32, i32, u32, u32)) -> RgbaImage {
    let mut out = RgbaImage::new(r.2, r.3);
    for y in 0..r.3 as i32 {
        for x in 0..r.2 as i32 {
            if let Some(c) = img.get(r.0 + x, r.1 + y) {
                out.put(x, y, c);
            }
        }
    }
    out
}
