//! Loading and saving images for sprites and backgrounds (specs/ide.md §5.1, §9).

use std::path::Path;

use gm_format::RgbaImage;
use image::AnimationDecoder;

/// Options of the image file dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadOptions {
    /// Drop the alpha channel.
    pub make_opaque: bool,
    /// The colour of the bottom-left pixel becomes transparent.
    pub remove_background: bool,
    /// Soften edges after removing the background.
    pub smooth_edges: bool,
}

impl LoadOptions {
    /// Defaults of the sprite dialog (remove background on).
    pub const SPRITE: LoadOptions = LoadOptions {
        make_opaque: false,
        remove_background: true,
        smooth_edges: false,
    };
    /// Defaults of the background dialog (remove background off).
    pub const BACKGROUND: LoadOptions = LoadOptions {
        make_opaque: false,
        remove_background: false,
        smooth_edges: false,
    };
}

fn to_model(img: image::RgbaImage) -> RgbaImage {
    RgbaImage {
        width: img.width(),
        height: img.height(),
        data: img.into_raw(),
    }
}

/// Number N of a file name ending in `_stripN` (before the extension).
pub fn strip_count(path: &Path) -> Option<u32> {
    let stem = path.file_stem()?.to_str()?;
    let lower = stem.to_ascii_lowercase();
    let pos = lower.rfind("_strip")?;
    let n: u32 = stem[pos + 6..].parse().ok()?;
    (n > 0).then_some(n)
}

/// Split an image horizontally into `n` equal frames.
pub fn split_strip(img: &RgbaImage, n: u32) -> Vec<RgbaImage> {
    let n = n.max(1);
    let fw = img.width / n;
    (0..n)
        .map(|k| crop(img, k * fw, 0, fw, img.height))
        .collect()
}

/// Copy a sub-rectangle (clipped to the image).
pub fn crop(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
    let mut out = RgbaImage::new(w, h);
    for yy in 0..h {
        for xx in 0..w {
            let (sx, sy) = (x + xx, y + yy);
            if sx < img.width && sy < img.height {
                let si = ((sy * img.width + sx) * 4) as usize;
                let di = ((yy * w + xx) * 4) as usize;
                out.data[di..di + 4].copy_from_slice(&img.data[si..si + 4]);
            }
        }
    }
    out
}

/// Place `img` on a transparent canvas of the given size (top-left aligned).
pub fn fit_canvas(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    if img.width == w && img.height == h {
        return img.clone();
    }
    crop(img, 0, 0, w, h)
}

/// Decode the frames of one image file. Animated GIFs give one frame per GIF frame;
/// other files ending in `_stripN` are split into N frames.
pub fn decode_frames(path: &Path, bytes: &[u8]) -> Result<Vec<RgbaImage>, String> {
    let is_gif = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("gif"));
    if is_gif {
        let dec = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes))
            .map_err(|e| e.to_string())?;
        let frames = dec
            .into_frames()
            .collect_frames()
            .map_err(|e| e.to_string())?;
        if frames.is_empty() {
            return Err("GIF has no frames".into());
        }
        return Ok(frames
            .into_iter()
            .map(|f| to_model(f.into_buffer()))
            .collect());
    }
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let img = to_model(img.to_rgba8());
    Ok(match strip_count(path) {
        Some(n) if n > 1 => split_strip(&img, n),
        _ => vec![img],
    })
}

/// Apply the dialog options to one frame.
pub fn apply_options(img: &mut RgbaImage, opt: LoadOptions) {
    if opt.make_opaque {
        for px in img.data.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
    }
    if opt.remove_background {
        gm_format::image::key_out_bottom_left(img);
        if opt.smooth_edges {
            gm_format::image::smooth_edges(img);
        }
    }
}

/// Load one or more files as frames; all frames are adapted to a common canvas.
pub fn load_frames(
    paths: &[std::path::PathBuf],
    opt: LoadOptions,
) -> Result<Vec<RgbaImage>, String> {
    let mut files = Vec::new();
    for p in paths {
        let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
        files.push((p.display().to_string(), bytes));
    }
    frames_from_bytes(&files, opt)
}

/// Like [`load_frames`] for files already read: (file name, bytes).
pub fn frames_from_bytes(
    files: &[(String, Vec<u8>)],
    opt: LoadOptions,
) -> Result<Vec<RgbaImage>, String> {
    let mut frames = Vec::new();
    for (name, bytes) in files {
        let mut f = decode_frames(Path::new(name), bytes).map_err(|e| format!("{name}: {e}"))?;
        frames.append(&mut f);
    }
    let w = frames.iter().map(|f| f.width).max().unwrap_or(0);
    let h = frames.iter().map(|f| f.height).max().unwrap_or(0);
    Ok(frames
        .iter()
        .map(|f| {
            let mut f = fit_canvas(f, w, h);
            apply_options(&mut f, opt);
            f
        })
        .collect())
}

/// Encode frames as a horizontal strip PNG.
pub fn strip_png_bytes(frames: &[RgbaImage]) -> Result<Vec<u8>, String> {
    let w = frames.first().map_or(0, |f| f.width);
    let h = frames.first().map_or(0, |f| f.height);
    let n = frames.len() as u32;
    let mut out = image::RgbaImage::new((w * n).max(1), h.max(1));
    for (k, f) in frames.iter().enumerate() {
        for y in 0..f.height.min(h) {
            for x in 0..f.width.min(w) {
                out.put_pixel(k as u32 * w + x, y, image::Rgba(f.pixel(x, y)));
            }
        }
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    out.write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}

/// Save frames as a horizontal strip PNG.
#[allow(dead_code)]
pub fn save_strip_png(frames: &[RgbaImage], path: &Path) -> Result<(), String> {
    std::fs::write(path, strip_png_bytes(frames)?).map_err(|e| e.to_string())
}

/// Tight bounding box of pixels with alpha > tolerance over all frames
/// (`(left, top, right, bottom)`), or None when all pixels are transparent.
pub fn auto_bbox(frames: &[RgbaImage], tolerance: i32) -> Option<(i32, i32, i32, i32)> {
    let mut bb: Option<(i32, i32, i32, i32)> = None;
    for f in frames {
        for y in 0..f.height {
            for x in 0..f.width {
                if f.pixel(x, y)[3] as i32 > tolerance {
                    let (x, y) = (x as i32, y as i32);
                    bb = Some(match bb {
                        None => (x, y, x, y),
                        Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x), b.max(y)),
                    });
                }
            }
        }
    }
    bb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_names() {
        assert_eq!(strip_count(Path::new("walk_strip4.png")), Some(4));
        assert_eq!(strip_count(Path::new("C:/x/Walk_STRIP12.bmp")), Some(12));
        assert_eq!(strip_count(Path::new("walk.png")), None);
        assert_eq!(strip_count(Path::new("walk_stripx.png")), None);
    }

    #[test]
    fn split_and_bbox() {
        let mut img = RgbaImage::new(8, 2);
        // one opaque pixel at (5,1)
        let i = (8 + 5) * 4;
        img.data[i..i + 4].copy_from_slice(&[1, 2, 3, 255]);
        let fr = split_strip(&img, 2);
        assert_eq!(fr.len(), 2);
        assert_eq!((fr[1].width, fr[1].height), (4, 2));
        assert_eq!(fr[1].pixel(1, 1), [1, 2, 3, 255]);
        assert_eq!(auto_bbox(&fr, 0), Some((1, 1, 1, 1)));
    }

    #[test]
    fn png_round_trip() {
        let mut img = RgbaImage::new(3, 2);
        img.data[0..4].copy_from_slice(&[10, 20, 30, 255]);
        let dir = std::env::temp_dir().join(format!("gm-ide-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t_strip2.png");
        save_strip_png(&[img.clone(), img.clone()], &p).unwrap();
        let fr = load_frames(std::slice::from_ref(&p), LoadOptions::BACKGROUND).unwrap();
        assert_eq!(fr.len(), 2);
        assert_eq!(fr[0], img);
        let _ = std::fs::remove_dir_all(dir);
    }
}
