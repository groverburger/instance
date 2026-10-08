//! Image encodings: v800 BGRA images, v701 zlib-compressed BMP images (spec §0.2).

use crate::error::{Error, Result};
use crate::io::{inflate, Reader, Writer};
use crate::model::RgbaImage;

/// Swap the R and B channels in place (BGRA <-> RGBA).
fn swap_rb(data: &mut [u8]) {
    for px in data.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
}

/// Read an "img800" record.
pub(crate) fn read_img800(r: &mut Reader<'_>) -> Result<RgbaImage> {
    r.expect(800, "image version")?;
    let at = r.offset();
    let w = r.i32()?;
    let h = r.i32()?;
    if w < 0 || h < 0 {
        return Err(r.invalid_at(at, format!("negative image size {w}x{h}")));
    }
    let (w, h) = (w as u32, h as u32);
    let expected = w as usize * h as usize * 4;
    if expected == 0 {
        // [I] no pixel blob for empty images.
        return Ok(RgbaImage {
            width: w,
            height: h,
            data: Vec::new(),
        });
    }
    let at = r.offset();
    let mut data = r.blob()?;
    if data.len() != expected {
        return Err(r.invalid_at(
            at,
            format!(
                "image pixel blob is {} bytes, expected {expected}",
                data.len()
            ),
        ));
    }
    swap_rb(&mut data);
    Ok(RgbaImage {
        width: w,
        height: h,
        data,
    })
}

/// Write an "img800" record.
pub(crate) fn write_img800(w: &mut Writer, img: &RgbaImage) -> Result<()> {
    if !img.is_consistent() {
        return Err(Error::InvalidModel(format!(
            "image {}x{} has {} bytes of pixel data",
            img.width,
            img.height,
            img.data.len()
        )));
    }
    w.i32(800);
    w.i32(img.width as i32);
    w.i32(img.height as i32);
    if !img.data.is_empty() {
        let mut bgra = img.data.clone();
        swap_rb(&mut bgra);
        w.blob(&bgra);
    }
    Ok(())
}

/// Read an "img701" record: `-1` = none, `10` = zlib-compressed BMP file.
pub(crate) fn read_img701(r: &mut Reader<'_>) -> Result<Option<RgbaImage>> {
    let at = r.offset();
    let marker = r.i32()?;
    match marker {
        -1 => Ok(None),
        10 => {
            let at = r.offset();
            let z = r.blob()?;
            let bmp = inflate(&z).map_err(|message| Error::Zlib {
                context: r.context().to_string(),
                offset: at,
                message,
            })?;
            decode_bmp(&bmp)
                .map(Some)
                .map_err(|m| r.invalid_at(at, format!("bad BMP image: {m}")))
        }
        m => Err(r.invalid_at(at, format!("unknown v701 image marker {m}"))),
    }
}

/// Decode an uncompressed Windows BMP file (1/4/8/16/24/32 bpp, BI_RGB or
/// BI_BITFIELDS for 16/32 bpp) into RGBA. Alpha is always 255.
pub fn decode_bmp(b: &[u8]) -> std::result::Result<RgbaImage, String> {
    let u16_at = |o: usize| -> std::result::Result<u16, String> {
        b.get(o..o + 2)
            .map(|s| u16::from_le_bytes([s[0], s[1]]))
            .ok_or_else(|| "truncated header".to_string())
    };
    let u32_at = |o: usize| -> std::result::Result<u32, String> {
        b.get(o..o + 4)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
            .ok_or_else(|| "truncated header".to_string())
    };
    if b.get(0..2) != Some(b"BM") {
        return Err("missing BM signature".into());
    }
    let pix_off = u32_at(10)? as usize;
    let hdr_size = u32_at(14)? as usize;
    let (width, height, bpp, compression, colors_used);
    if hdr_size == 12 {
        width = u16_at(18)? as i32;
        height = u16_at(20)? as i16 as i32;
        bpp = u16_at(24)?;
        compression = 0;
        colors_used = 0;
    } else if hdr_size >= 40 {
        width = u32_at(18)? as i32;
        height = u32_at(22)? as i32;
        bpp = u16_at(28)?;
        compression = u32_at(30)?;
        colors_used = u32_at(46)?;
    } else {
        return Err(format!("unsupported BMP header size {hdr_size}"));
    }
    if width < 0 {
        return Err("negative width".into());
    }
    let top_down = height < 0;
    let (w, h) = (width as usize, height.unsigned_abs() as usize);
    if compression != 0 && !(compression == 3 && (bpp == 16 || bpp == 32)) {
        return Err(format!("unsupported BMP compression {compression}"));
    }
    // Palette.
    let pal_entry = if hdr_size == 12 { 3 } else { 4 };
    let pal_off = 14 + hdr_size;
    let mut palette = Vec::new();
    if bpp <= 8 {
        let n = if colors_used == 0 {
            1usize << bpp
        } else {
            colors_used as usize
        };
        for i in 0..n {
            let o = pal_off + i * pal_entry;
            let e = b.get(o..o + 3).ok_or("truncated palette")?;
            palette.push([e[2], e[1], e[0]]);
        }
    }
    // Bit masks for 16/32 bpp.
    let masks = if compression == 3 {
        [u32_at(54)?, u32_at(58)?, u32_at(62)?]
    } else if bpp == 16 {
        [0x7C00, 0x03E0, 0x001F]
    } else {
        [0xFF0000, 0x00FF00, 0x0000FF]
    };
    let extract = |v: u32, m: u32| -> u8 {
        if m == 0 {
            return 0;
        }
        let shift = m.trailing_zeros();
        let max = m >> shift;
        (((v & m) >> shift) * 255 / max) as u8
    };
    let stride = (w * bpp as usize).div_ceil(32) * 4;
    let mut out = vec![0u8; w * h * 4];
    for row in 0..h {
        let src_row = if top_down { row } else { h - 1 - row };
        let o = pix_off + src_row * stride;
        let line = b.get(o..o + stride).ok_or("truncated pixel data")?;
        for x in 0..w {
            let rgb: [u8; 3] = match bpp {
                1 | 4 | 8 => {
                    let bit = x * bpp as usize;
                    let byte = line[bit / 8];
                    let shift = 8 - bpp as usize - (bit % 8);
                    let idx = (byte >> shift) as usize & ((1 << bpp) - 1);
                    *palette.get(idx).ok_or("palette index out of range")?
                }
                16 => {
                    let v = u16::from_le_bytes([line[x * 2], line[x * 2 + 1]]) as u32;
                    [
                        extract(v, masks[0]),
                        extract(v, masks[1]),
                        extract(v, masks[2]),
                    ]
                }
                24 => [line[x * 3 + 2], line[x * 3 + 1], line[x * 3]],
                32 => {
                    let p = &line[x * 4..x * 4 + 4];
                    let v = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
                    [
                        extract(v, masks[0]),
                        extract(v, masks[1]),
                        extract(v, masks[2]),
                    ]
                }
                _ => return Err(format!("unsupported bit depth {bpp}")),
            };
            let d = (row * w + x) * 4;
            out[d..d + 3].copy_from_slice(&rgb);
            out[d + 3] = 255;
        }
    }
    Ok(RgbaImage {
        width: w as u32,
        height: h as u32,
        data: out,
    })
}

/// Encode an RGBA image as a 32-bpp bottom-up BMP file (BI_RGB, alpha byte kept).
pub fn encode_bmp(img: &RgbaImage) -> Vec<u8> {
    let (w, h) = (img.width as usize, img.height as usize);
    let size = 54 + w * h * 4;
    let mut b = Vec::with_capacity(size);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&(size as u32).to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(w as i32).to_le_bytes());
    b.extend_from_slice(&(h as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(&[0u8; 24]);
    for row in (0..h).rev() {
        for x in 0..w {
            let p = &img.data[(row * w + x) * 4..][..4];
            b.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
    b
}

/// Make every pixel with the colour of the bottom-left pixel fully transparent
/// (v701 "transparent" flag, spec §0.2 [I]).
pub fn key_out_bottom_left(img: &mut RgbaImage) {
    if img.width == 0 || img.height == 0 {
        return;
    }
    let key = img.pixel(0, img.height - 1);
    for px in img.data.as_chunks_mut::<4>().0 {
        if px[..3] == key[..3] {
            px[3] = 0;
        }
    }
}

/// Soften the edges of a keyed-out image (v701 "smooth edges" flag): every opaque pixel
/// loses 32 alpha per fully transparent pixel among its 8 neighbours (pixels outside
/// the image do not count). Derived from comparing a v701 sample with the same project
/// re-saved as v810; see OPEN_QUESTIONS.md [IMPL-FMT].
pub fn smooth_edges(img: &mut RgbaImage) {
    let (w, h) = (img.width as i64, img.height as i64);
    let src = img.data.clone();
    let transparent = |x: i64, y: i64| -> bool {
        x >= 0 && y >= 0 && x < w && y < h && src[((y * w + x) * 4 + 3) as usize] == 0
    };
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4 + 3) as usize;
            if src[i] == 0 {
                continue;
            }
            let mut k = 0i32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if (dx, dy) != (0, 0) && transparent(x + dx, y + dy) {
                        k += 1;
                    }
                }
            }
            img.data[i] = (src[i] as i32 - 32 * k).max(0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RgbaImage {
        RgbaImage {
            width: 3,
            height: 2,
            data: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, //
                10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255,
            ],
        }
    }

    #[test]
    fn bmp32_round_trip() {
        let img = sample();
        assert_eq!(decode_bmp(&encode_bmp(&img)).unwrap(), img);
    }

    #[test]
    fn bmp24_decode() {
        let img = sample();
        // Build a 24-bit bottom-up BMP by hand (stride 12: 9 bytes + 3 padding).
        let mut b = Vec::new();
        b.extend_from_slice(b"BM");
        b.extend_from_slice(&(54u32 + 24).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&54u32.to_le_bytes());
        b.extend_from_slice(&40u32.to_le_bytes());
        b.extend_from_slice(&3i32.to_le_bytes());
        b.extend_from_slice(&2i32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&24u16.to_le_bytes());
        b.extend_from_slice(&[0u8; 24]);
        for row in [1usize, 0] {
            for x in 0..3 {
                let p = img.pixel(x, row as u32);
                b.extend_from_slice(&[p[2], p[1], p[0]]);
            }
            b.extend_from_slice(&[0, 0, 0]);
        }
        assert_eq!(decode_bmp(&b).unwrap(), img);
    }

    #[test]
    fn smoothing() {
        // 3x3, centre and right column opaque, rest transparent.
        let mut img = RgbaImage::new(3, 3);
        for (x, y) in [(1, 1), (2, 0), (2, 1), (2, 2)] {
            let i = (y * 3 + x) * 4;
            img.data[i + 3] = 255;
        }
        smooth_edges(&mut img);
        assert_eq!(img.pixel(1, 1)[3], 255 - 5 * 32);
        assert_eq!(img.pixel(2, 0)[3], 255 - 32);
        assert_eq!(img.pixel(0, 0)[3], 0);
    }

    #[test]
    fn key_out() {
        let mut img = sample();
        key_out_bottom_left(&mut img);
        assert_eq!(img.pixel(0, 1)[3], 0);
        assert_eq!(img.pixel(1, 1)[3], 255);
    }
}
