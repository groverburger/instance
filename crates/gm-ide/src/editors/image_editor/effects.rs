//! Whole-image transforms, colour effects and animation sequence operations of the
//! sprite and image editors (specs/ide.md §6 Transform / Images / Animation menus).
//! The manual does not fix the pixel algorithms; these are straightforward choices.

use gm_format::RgbaImage;

use super::raster::{lerp, Px, Rgba};

// ------------------------------------------------------------------ colour helpers

/// RGB (0..=255) to HSV (h in degrees 0..360, s and v in 0..=1).
pub fn rgb_to_hsv(c: Rgba) -> (f32, f32, f32) {
    let (r, g, b) = (
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

/// HSV to RGB with the given alpha.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32, a: u8) -> Rgba {
    let h = h.rem_euclid(360.0);
    let (s, v) = (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let f = |u: f32| ((u + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    [f(r), f(g), f(b), a]
}

fn hue_distance(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

fn map_pixels(img: &RgbaImage, f: impl Fn(Rgba) -> Rgba) -> RgbaImage {
    let mut out = img.clone();
    for px in out.data.as_chunks_mut::<4>().0 {
        let c = f([px[0], px[1], px[2], px[3]]);
        px.copy_from_slice(&c);
    }
    out
}

// ------------------------------------------------------------------ sampling

/// Nearest-neighbour sample at continuous coordinates (pixel centres at +0.5).
fn sample_nearest(img: &RgbaImage, fx: f32, fy: f32) -> Rgba {
    img.get(fx.floor() as i32, fy.floor() as i32)
        .unwrap_or([0, 0, 0, 0])
}

/// Bilinear sample with premultiplied alpha; outside pixels are transparent.
fn sample_bilinear(img: &RgbaImage, fx: f32, fy: f32, clamp: bool) -> Rgba {
    let (mut x, mut y) = (fx - 0.5, fy - 0.5);
    if clamp {
        x = x.clamp(0.0, img.width.saturating_sub(1) as f32);
        y = y.clamp(0.0, img.height.saturating_sub(1) as f32);
    }
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (tx, ty) = (x - x0 as f32, y - y0 as f32);
    let mut acc = [0.0f32; 4];
    for (dx, dy, w) in [
        (0, 0, (1.0 - tx) * (1.0 - ty)),
        (1, 0, tx * (1.0 - ty)),
        (0, 1, (1.0 - tx) * ty),
        (1, 1, tx * ty),
    ] {
        if w == 0.0 {
            continue;
        }
        let p = img.get(x0 + dx, y0 + dy).unwrap_or([0, 0, 0, 0]);
        let a = p[3] as f32 / 255.0;
        acc[0] += p[0] as f32 * a * w;
        acc[1] += p[1] as f32 * a * w;
        acc[2] += p[2] as f32 * a * w;
        acc[3] += a * w;
    }
    if acc[3] <= 1e-6 {
        return [0, 0, 0, 0];
    }
    let f = |v: f32| (v / acc[3]).round().clamp(0.0, 255.0) as u8;
    [
        f(acc[0]),
        f(acc[1]),
        f(acc[2]),
        (acc[3] * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Build a `w`×`h` image by inverse mapping every destination pixel centre.
fn inverse_map(
    src: &RgbaImage,
    w: u32,
    h: u32,
    smooth: bool,
    f: impl Fn(f32, f32) -> (f32, f32),
) -> RgbaImage {
    inverse_map_ex(src, w, h, smooth, false, f)
}

fn inverse_map_ex(
    src: &RgbaImage,
    w: u32,
    h: u32,
    smooth: bool,
    clamp: bool,
    f: impl Fn(f32, f32) -> (f32, f32),
) -> RgbaImage {
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = f(x as f32 + 0.5, y as f32 + 0.5);
            let c = if smooth {
                sample_bilinear(src, sx, sy, clamp)
            } else {
                sample_nearest(src, sx, sy)
            };
            out.put(x as i32, y as i32, c);
        }
    }
    out
}

// ------------------------------------------------------------------ transforms

/// Mirror left-right.
pub fn mirror(img: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::new(img.width, img.height);
    for y in 0..img.height {
        for x in 0..img.width {
            out.put((img.width - 1 - x) as i32, y as i32, img.pixel(x, y));
        }
    }
    out
}

/// Flip upside down.
pub fn flip(img: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::new(img.width, img.height);
    let row = img.width as usize * 4;
    for y in 0..img.height as usize {
        let s = (img.height as usize - 1 - y) * row;
        out.data[y * row..(y + 1) * row].copy_from_slice(&img.data[s..s + row]);
    }
    out
}

/// Rotate 90 degrees clockwise (width and height swap).
pub fn rotate90_cw(img: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::new(img.height, img.width);
    for y in 0..img.height {
        for x in 0..img.width {
            out.put((img.height - 1 - y) as i32, x as i32, img.pixel(x, y));
        }
    }
    out
}

/// Rotate 90 degrees counter-clockwise.
pub fn rotate90_ccw(img: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::new(img.height, img.width);
    for y in 0..img.height {
        for x in 0..img.width {
            out.put(y as i32, (img.width - 1 - x) as i32, img.pixel(x, y));
        }
    }
    out
}

/// Rotate 180 degrees.
pub fn rotate180(img: &RgbaImage) -> RgbaImage {
    flip(&mirror(img))
}

/// Rotate by `deg` degrees (positive = counter-clockwise on screen) about the centre;
/// the canvas size is kept. Multiples of 90 on square images are exact.
pub fn rotate(img: &RgbaImage, deg: f32, smooth: bool) -> RgbaImage {
    let d = deg.rem_euclid(360.0);
    let square = img.width == img.height;
    if d == 0.0 {
        return img.clone();
    }
    if d == 180.0 {
        return rotate180(img);
    }
    if square && d == 90.0 {
        return rotate90_ccw(img);
    }
    if square && d == 270.0 {
        return rotate90_cw(img);
    }
    let (s, c) = d.to_radians().sin_cos();
    let (cx, cy) = (img.width as f32 / 2.0, img.height as f32 / 2.0);
    inverse_map(img, img.width, img.height, smooth, |x, y| {
        // screen y points down, so a CCW rotation by d maps source → dest with
        // (x', y') = (c*x + s*y, -s*x + c*y); invert it.
        let (dx, dy) = (x - cx, y - cy);
        (cx + c * dx - s * dy, cy + s * dx + c * dy)
    })
}

/// Shift the contents by (dx, dy); uncovered pixels become transparent.
pub fn shift(img: &RgbaImage, dx: i32, dy: i32) -> RgbaImage {
    let mut out = RgbaImage::new(img.width, img.height);
    for y in 0..img.height as i32 {
        for x in 0..img.width as i32 {
            if let Some(c) = img.get(x - dx, y - dy) {
                out.put(x, y, c);
            }
        }
    }
    out
}

/// Scale the contents about the centre by (sx, sy); the canvas is unchanged.
pub fn scale(img: &RgbaImage, sx: f32, sy: f32, smooth: bool) -> RgbaImage {
    if sx <= 0.0 || sy <= 0.0 {
        return RgbaImage::new(img.width, img.height);
    }
    let (cx, cy) = (img.width as f32 / 2.0, img.height as f32 / 2.0);
    inverse_map(img, img.width, img.height, smooth, |x, y| {
        (cx + (x - cx) / sx, cy + (y - cy) / sy)
    })
}

/// Skew: the bottom row moves `h` pixels right relative to the top row and the right
/// column moves `v` pixels down relative to the left column (centred).
pub fn skew(img: &RgbaImage, h: f32, v: f32, smooth: bool) -> RgbaImage {
    let (w, ht) = (img.width.max(1) as f32, img.height.max(1) as f32);
    let (cx, cy) = (w / 2.0, ht / 2.0);
    let (kx, ky) = (h / ht, v / w);
    inverse_map(img, img.width, img.height, smooth, |x, y| {
        // forward: x' = x + kx*(y-cy), y' = y + ky*(x-cx); invert the 2x2 system
        let (dx, dy) = (x - cx, y - cy);
        let det = 1.0 - kx * ky;
        if det.abs() < 1e-6 {
            return (-1.0, -1.0);
        }
        let sx = (dx - kx * dy) / det;
        let sy = (dy - ky * dx) / det;
        (cx + sx, cy + sy)
    })
}

/// Resize the canvas to `w`×`h`; `anchor` (0..=2, 0..=2) chooses where the old image
/// goes (0 = left/top, 1 = centre, 2 = right/bottom).
pub fn resize_canvas(img: &RgbaImage, w: u32, h: u32, anchor: (u8, u8)) -> RgbaImage {
    let off = |old: u32, new: u32, a: u8| -> i32 {
        let d = new as i32 - old as i32;
        match a {
            0 => 0,
            1 => d / 2,
            _ => d,
        }
    };
    let (ox, oy) = (off(img.width, w, anchor.0), off(img.height, h, anchor.1));
    let mut out = RgbaImage::new(w, h);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if let Some(c) = img.get(x - ox, y - oy) {
                out.put(x, y, c);
            }
        }
    }
    out
}

/// Stretch the image to `w`×`h`.
pub fn stretch(img: &RgbaImage, w: u32, h: u32, smooth: bool) -> RgbaImage {
    let (w, h) = (w.max(1), h.max(1));
    if img.width == 0 || img.height == 0 {
        return RgbaImage::new(w, h);
    }
    let (fx, fy) = (img.width as f32 / w as f32, img.height as f32 / h as f32);
    inverse_map_ex(img, w, h, smooth, true, |x, y| (x * fx, y * fy))
}

/// Bounding box (x, y, w, h) of non-transparent pixels over all images, grown by
/// `border` (negative trims), clipped to the canvas. None if everything is empty.
pub fn crop_box(frames: &[RgbaImage], border: i32) -> Option<(i32, i32, u32, u32)> {
    let (l, t, r, b) = crate::imageio::auto_bbox(frames, 0)?;
    let (w, h) = frames.first().map(|f| (f.width as i32, f.height as i32))?;
    let l = (l - border).max(0);
    let t = (t - border).max(0);
    let r = (r + border).min(w - 1);
    let b = (b + border).min(h - 1);
    if r < l || b < t {
        return None;
    }
    Some((l, t, (r - l + 1) as u32, (b - t + 1) as u32))
}

/// Crop a single image to its content plus `border`.
pub fn crop(img: &RgbaImage, border: i32) -> RgbaImage {
    match crop_box(std::slice::from_ref(img), border) {
        Some(r) => super::raster::copy_rect(img, r),
        None => img.clone(),
    }
}

// ------------------------------------------------------------------ colour effects

pub fn black_and_white(img: &RgbaImage) -> RgbaImage {
    map_pixels(img, |c| {
        let l = (0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32).round() as u8;
        [l, l, l, c[3]]
    })
}

pub fn invert(img: &RgbaImage) -> RgbaImage {
    map_pixels(img, |c| [255 - c[0], 255 - c[1], 255 - c[2], c[3]])
}

pub fn make_opaque(img: &RgbaImage) -> RgbaImage {
    map_pixels(img, |c| [c[0], c[1], c[2], 255])
}

/// Give every pixel hue `hue` and saturation `sat` (value kept).
pub fn colorize(img: &RgbaImage, hue: f32, sat: f32) -> RgbaImage {
    map_pixels(img, |c| {
        let (_, _, v) = rgb_to_hsv(c);
        hsv_to_rgb(hue, sat, v, c[3])
    })
}

/// Rotate all hues by `deg`.
pub fn shift_hue(img: &RgbaImage, deg: f32) -> RgbaImage {
    map_pixels(img, |c| {
        let (h, s, v) = rgb_to_hsv(c);
        if s == 0.0 {
            c
        } else {
            hsv_to_rgb(h + deg, s, v, c[3])
        }
    })
}

/// Pixels whose hue is within `tol` degrees of `src` get hue `dst`.
pub fn colorize_partial(img: &RgbaImage, src: f32, tol: f32, dst: f32) -> RgbaImage {
    map_pixels(img, |c| {
        let (h, s, v) = rgb_to_hsv(c);
        if s > 0.0 && hue_distance(h, src) <= tol {
            hsv_to_rgb(dst + (h - src), s, v, c[3])
        } else {
            c
        }
    })
}

/// Change saturation and value by percentages (-100..=100).
pub fn intensity(img: &RgbaImage, sat_pct: f32, val_pct: f32) -> RgbaImage {
    map_pixels(img, |c| {
        let (h, s, v) = rgb_to_hsv(c);
        let adj = |x: f32, p: f32| {
            if p >= 0.0 {
                x + (1.0 - x) * p / 100.0
            } else {
                x * (1.0 + p / 100.0)
            }
        };
        hsv_to_rgb(h, adj(s, sat_pct), adj(v, val_pct), c[3])
    })
}

/// Pixels within `tol` of `color` (RGB only) become fully transparent.
pub fn erase_color(img: &RgbaImage, color: Rgba, tol: u8) -> RgbaImage {
    map_pixels(img, |c| {
        if super::raster::color_distance(c, color, false) <= tol {
            [0, 0, 0, 0]
        } else {
            c
        }
    })
}

/// Opacity: relative multiplies alpha by `amount`% ; absolute sets alpha to `amount`
/// (0..=255). Fully transparent pixels stay transparent.
pub fn opacity(img: &RgbaImage, amount: f32, relative: bool) -> RgbaImage {
    map_pixels(img, |c| {
        if c[3] == 0 {
            return c;
        }
        let a = if relative {
            c[3] as f32 * amount / 100.0
        } else {
            amount
        };
        [c[0], c[1], c[2], a.round().clamp(0.0, 255.0) as u8]
    })
}

/// Move colours towards `color` by `amount` (0..=255); alpha kept.
pub fn fade(img: &RgbaImage, color: Rgba, amount: u8) -> RgbaImage {
    let t = amount as f32 / 255.0;
    map_pixels(img, |c| {
        let m = lerp(c, [color[0], color[1], color[2], c[3]], t);
        [m[0], m[1], m[2], c[3]]
    })
}

/// Box blur with `radius`; affects colour and/or alpha. Colours are weighted by alpha
/// so transparent pixels do not darken edges.
pub fn blur(img: &RgbaImage, radius: u32, color: bool, alpha: bool) -> RgbaImage {
    if radius == 0 || (!color && !alpha) {
        return img.clone();
    }
    let r = radius as i32;
    let mut out = img.clone();
    for y in 0..img.height as i32 {
        for x in 0..img.width as i32 {
            let mut acc = [0.0f32; 4];
            let mut wsum = 0.0f32;
            let mut n = 0.0f32;
            for yy in y - r..=y + r {
                for xx in x - r..=x + r {
                    let p = img
                        .get(
                            xx.clamp(0, img.width as i32 - 1),
                            yy.clamp(0, img.height as i32 - 1),
                        )
                        .unwrap_or([0; 4]);
                    let a = p[3] as f32;
                    acc[0] += p[0] as f32 * a;
                    acc[1] += p[1] as f32 * a;
                    acc[2] += p[2] as f32 * a;
                    acc[3] += a;
                    wsum += a;
                    n += 1.0;
                }
            }
            let mut c = img.pixel(x as u32, y as u32);
            if color && wsum > 0.0 {
                for k in 0..3 {
                    c[k] = (acc[k] / wsum).round().clamp(0.0, 255.0) as u8;
                }
            }
            if alpha {
                c[3] = (acc[3] / n).round().clamp(0.0, 255.0) as u8;
            }
            out.put(x, y, c);
        }
    }
    out
}

/// Unsharp mask: c + amount * (c - blur(c)).
pub fn sharpen(img: &RgbaImage, amount: f32, color: bool, alpha: bool) -> RgbaImage {
    let b = blur(img, 1, true, true);
    let mut out = img.clone();
    for (i, px) in out.data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let bp = &b.data[i * 4..i * 4 + 4];
        let mut f = |k: usize| {
            let v = px[k] as f32 + amount * (px[k] as f32 - bp[k] as f32);
            px[k] = v.round().clamp(0.0, 255.0) as u8;
        };
        if color {
            f(0);
            f(1);
            f(2);
        }
        if alpha {
            f(3);
        }
    }
    out
}

/// Draw an outline of `thickness` pixels in `color` around non-transparent pixels.
pub fn outline(img: &RgbaImage, color: Rgba, thickness: u32) -> RgbaImage {
    let t = thickness.max(1) as i32;
    let mut out = img.clone();
    for y in 0..img.height as i32 {
        for x in 0..img.width as i32 {
            if img.pixel(x as u32, y as u32)[3] != 0 {
                continue;
            }
            let near = (-t..=t).any(|dy| {
                (-t..=t).any(|dx| {
                    dx * dx + dy * dy <= t * t && img.get(x + dx, y + dy).is_some_and(|p| p[3] != 0)
                })
            });
            if near {
                out.put(x, y, color);
            }
        }
    }
    out
}

/// Soften edges against transparency (shared with the file dialog option).
pub fn smooth_edges(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    gm_format::image::smooth_edges(&mut out);
    out
}

// ------------------------------------------------------------------ animation

/// Repeat the frames cyclically to exactly `n` frames.
pub fn set_length(frames: &[RgbaImage], n: usize) -> Vec<RgbaImage> {
    if frames.is_empty() {
        return Vec::new();
    }
    (0..n).map(|i| frames[i % frames.len()].clone()).collect()
}

/// Duplicate or drop frames evenly to get `n` frames.
pub fn stretch_length(frames: &[RgbaImage], n: usize) -> Vec<RgbaImage> {
    if frames.is_empty() {
        return Vec::new();
    }
    (0..n)
        .map(|i| frames[i * frames.len() / n.max(1)].clone())
        .collect()
}

pub fn reverse(frames: &[RgbaImage]) -> Vec<RgbaImage> {
    frames.iter().rev().cloned().collect()
}

/// Append the reversed sequence without repeating the end frames (0 1 2 3 → 0 1 2 3 2 1).
pub fn add_reverse(frames: &[RgbaImage]) -> Vec<RgbaImage> {
    let mut out = frames.to_vec();
    if frames.len() > 2 {
        out.extend(frames[1..frames.len() - 1].iter().rev().cloned());
    }
    out
}

/// Build `n` frames where frame i is `f(source frame, t)` with t going 0 → 1.
fn sequence(
    frames: &[RgbaImage],
    n: usize,
    f: impl Fn(&RgbaImage, f32) -> RgbaImage,
) -> Vec<RgbaImage> {
    if frames.is_empty() || n == 0 {
        return frames.to_vec();
    }
    (0..n)
        .map(|i| {
            let t = if n > 1 {
                i as f32 / (n - 1) as f32
            } else {
                1.0
            };
            f(&frames[i % frames.len()], t)
        })
        .collect()
}

/// Frames moving by (dx, dy) in total.
pub fn translation_sequence(frames: &[RgbaImage], n: usize, dx: i32, dy: i32) -> Vec<RgbaImage> {
    sequence(frames, n, |f, t| {
        shift(
            f,
            (dx as f32 * t).round() as i32,
            (dy as f32 * t).round() as i32,
        )
    })
}

/// Frames rotating by `angle` degrees in total (clockwise when `cw`); a full turn does
/// not repeat the first frame at the end.
pub fn rotation_sequence(frames: &[RgbaImage], n: usize, angle: f32, cw: bool) -> Vec<RgbaImage> {
    let full = (angle.abs() - 360.0).abs() < 1e-3;
    let sign = if cw { -1.0 } else { 1.0 };
    if frames.is_empty() || n == 0 {
        return frames.to_vec();
    }
    (0..n)
        .map(|i| {
            let t = if full {
                i as f32 / n as f32
            } else if n > 1 {
                i as f32 / (n - 1) as f32
            } else {
                1.0
            };
            rotate(&frames[i % frames.len()], sign * angle * t, true)
        })
        .collect()
}

/// Frames gradually taking hue `hue`.
pub fn colorize_sequence(frames: &[RgbaImage], n: usize, hue: f32) -> Vec<RgbaImage> {
    sequence(frames, n, |f, t| {
        let c = colorize(f, hue, 0.8);
        blend_images(f, &c, t)
    })
}

/// Frames gradually fading to `color`.
pub fn fade_sequence(frames: &[RgbaImage], n: usize, color: Rgba) -> Vec<RgbaImage> {
    sequence(frames, n, |f, t| fade(f, color, (t * 255.0).round() as u8))
}

/// Frames gradually becoming transparent.
pub fn disappear_sequence(frames: &[RgbaImage], n: usize) -> Vec<RgbaImage> {
    sequence(frames, n, |f, t| opacity(f, 100.0 * (1.0 - t), true))
}

/// Frames shrinking towards the centre (last frame empty) or growing from it.
pub fn shrink_sequence(frames: &[RgbaImage], n: usize, grow: bool) -> Vec<RgbaImage> {
    sequence(frames, n, |f, t| {
        let s = if grow { t } else { 1.0 - t };
        scale(f, s, s, true)
    })
}

/// Frames flattening (or raising) vertically or horizontally.
pub fn flatten_sequence(
    frames: &[RgbaImage],
    n: usize,
    horizontal: bool,
    raise: bool,
) -> Vec<RgbaImage> {
    sequence(frames, n, |f, t| {
        let s = if raise { t } else { 1.0 - t };
        if horizontal {
            scale(f, s, 1.0, true)
        } else {
            scale(f, 1.0, s, true)
        }
    })
}

/// Frames cross-fading from the source frames towards `target`.
pub fn morph_sequence(frames: &[RgbaImage], n: usize, target: &RgbaImage) -> Vec<RgbaImage> {
    let w = frames.first().map_or(target.width, |f| f.width);
    let h = frames.first().map_or(target.height, |f| f.height);
    let tgt = stretch(target, w, h, true);
    sequence(frames, n, |f, t| blend_images(f, &tgt, t))
}

/// Per-pixel interpolation between two equally sized images.
pub fn blend_images(a: &RgbaImage, b: &RgbaImage, t: f32) -> RgbaImage {
    let mut out = a.clone();
    if (a.width, a.height) != (b.width, b.height) {
        return out;
    }
    for (i, px) in out.data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let pa = [px[0], px[1], px[2], px[3]];
        let pb = [
            b.data[i * 4],
            b.data[i * 4 + 1],
            b.data[i * 4 + 2],
            b.data[i * 4 + 3],
        ];
        // interpolate colour of whichever side is visible
        let pa2 = if pa[3] == 0 {
            [pb[0], pb[1], pb[2], 0]
        } else {
            pa
        };
        let pb2 = if pb[3] == 0 {
            [pa[0], pa[1], pa[2], 0]
        } else {
            pb
        };
        px.copy_from_slice(&lerp(pa2, pb2, t));
    }
    out
}

/// Rotate the frame list one step to the left (first frame moves to the end).
pub fn cycle_left(frames: &mut [RgbaImage]) {
    if !frames.is_empty() {
        frames.rotate_left(1);
    }
}

/// Rotate the frame list one step to the right.
pub fn cycle_right(frames: &mut [RgbaImage]) {
    if !frames.is_empty() {
        frames.rotate_right(1);
    }
}
