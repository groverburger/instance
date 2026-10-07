//! Windows icon containers: `.ico` files, compiled resource (`.res`) files holding an
//! icon group, and decoding `.ico` images (BMP or PNG entries) to RGBA.

use crate::icon_rgba;

/// One 32-bit DIB icon image (BITMAPINFOHEADER, bottom-up BGRA rows, AND mask).
fn dib(size: u32) -> Vec<u8> {
    let rgba = icon_rgba(size);
    let mask_stride = size.div_ceil(32) * 4;
    let mut out = Vec::new();
    let le32 = |v: u32| v.to_le_bytes();
    out.extend_from_slice(&le32(40));
    out.extend_from_slice(&le32(size));
    out.extend_from_slice(&le32(size * 2)); // XOR + AND masks
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&le32(0)); // BI_RGB
    out.extend_from_slice(&le32(size * size * 4 + mask_stride * size));
    out.extend_from_slice(&[0; 16]); // resolution, colours used / important
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            out.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    for y in (0..size).rev() {
        let mut row = vec![0u8; mask_stride as usize];
        for x in 0..size {
            if rgba[((y * size + x) * 4 + 3) as usize] == 0 {
                row[(x / 8) as usize] |= 0x80 >> (x % 8);
            }
        }
        out.extend_from_slice(&row);
    }
    out
}

/// Directory entry fields shared by `.ico` and RT_GROUP_ICON: width, height, colours,
/// reserved, planes, bit count, size.
fn entry_head(size: u32, len: usize) -> Vec<u8> {
    let b = if size >= 256 { 0 } else { size as u8 };
    let mut e = vec![b, b, 0, 0];
    e.extend_from_slice(&1u16.to_le_bytes());
    e.extend_from_slice(&32u16.to_le_bytes());
    e.extend_from_slice(&(len as u32).to_le_bytes());
    e
}

/// A complete `.ico` file with the app icon at `sizes`.
pub fn ico_file(sizes: &[u32]) -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = sizes.iter().map(|&s| (s, dib(s))).collect();
    let mut out = vec![0, 0, 1, 0];
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (s, d) in &images {
        out.extend_from_slice(&entry_head(*s, d.len()));
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += d.len();
    }
    for (_, d) in &images {
        out.extend_from_slice(d);
    }
    out
}

/// One resource entry of a `.res` file with ordinal type and name.
fn res_entry(out: &mut Vec<u8>, ty: u16, name: u16, flags: u16, data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&ty.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&name.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // data version
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&0x0409u16.to_le_bytes()); // en-US
    out.extend_from_slice(&0u32.to_le_bytes()); // version
    out.extend_from_slice(&0u32.to_le_bytes()); // characteristics
    out.extend_from_slice(data);
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

/// A compiled resource file (the output format of a resource compiler) containing the
/// app icon at `sizes` as icon group 1. The MSVC linker accepts it as an input file.
pub fn res_file(sizes: &[u32]) -> Vec<u8> {
    const RT_ICON: u16 = 3;
    const RT_GROUP_ICON: u16 = 14;
    let mut out = Vec::new();
    // the leading empty entry that marks a 32-bit resource file
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF, 0, 0, 0xFF, 0xFF, 0, 0]);
    out.extend_from_slice(&[0; 16]);
    let mut group = vec![0, 0, 1, 0];
    group.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    for (i, &s) in sizes.iter().enumerate() {
        let d = dib(s);
        let id = (i + 1) as u16;
        res_entry(&mut out, RT_ICON, id, 0x1010, &d);
        group.extend_from_slice(&entry_head(s, d.len()));
        group.extend_from_slice(&id.to_le_bytes());
    }
    res_entry(&mut out, RT_GROUP_ICON, 1, 0x1030, &group);
    out
}

fn u16_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as u32)
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// Decode the image of an `.ico` file whose size is closest to `want` (preferring the
/// smallest one at least `want` wide, else the largest). Returns straight RGBA, width,
/// height.
pub fn decode_ico(bytes: &[u8], want: u32) -> Option<(Vec<u8>, u32, u32)> {
    if u16_at(bytes, 0)? != 0 || u16_at(bytes, 2)? != 1 {
        return None;
    }
    let n = u16_at(bytes, 4)? as usize;
    let mut entries = Vec::new();
    for i in 0..n {
        let e = 6 + i * 16;
        let w = match *bytes.get(e)? {
            0 => 256,
            w => w as u32,
        };
        let bpp = u16_at(bytes, e + 6)?;
        let len = u32_at(bytes, e + 8)? as usize;
        let off = u32_at(bytes, e + 12)? as usize;
        let data = bytes.get(off..off.checked_add(len)?)?;
        entries.push((w, bpp, data));
    }
    // best size first, then the deepest colour
    entries.sort_by_key(|&(w, bpp, _)| {
        let size_rank = if w >= want {
            w - want
        } else {
            10_000 + want - w
        };
        (size_rank, u32::MAX - bpp)
    });
    entries.iter().find_map(|&(_, _, data)| decode_entry(data))
}

fn decode_entry(data: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    if data.starts_with(b"\x89PNG") {
        return decode_png(data);
    }
    let hsize = u32_at(data, 0)? as usize;
    let w = u32_at(data, 4)? as i32;
    let h2 = u32_at(data, 8)? as i32;
    let bpp = u16_at(data, 14)?;
    let compression = u32_at(data, 16)?;
    if w <= 0 || h2 <= 0 || compression != 0 || w > 1024 {
        return None;
    }
    let (w, h) = (w as u32, (h2 / 2) as u32);
    let colours = match u32_at(data, 32)? {
        0 if bpp <= 8 => 1u32 << bpp,
        c => c,
    } as usize;
    let pal_off = hsize;
    let palette: Vec<[u8; 4]> = if bpp <= 8 {
        (0..colours)
            .map(|i| {
                let p = data.get(pal_off + i * 4..pal_off + i * 4 + 4)?;
                Some([p[2], p[1], p[0], 255])
            })
            .collect::<Option<_>>()?
    } else {
        Vec::new()
    };
    let xor_off = pal_off + palette.len() * 4;
    let stride = ((w * bpp as u32).div_ceil(32) * 4) as usize;
    let mask_off = xor_off + stride * h as usize;
    let mask_stride = (w.div_ceil(32) * 4) as usize;
    let mut out = vec![0u8; (w * h * 4) as usize];
    let mut any_alpha = false;
    for y in 0..h as usize {
        let row = data.get(xor_off + (h as usize - 1 - y) * stride..)?;
        for x in 0..w as usize {
            let px: [u8; 4] = match bpp {
                32 => {
                    let p = row.get(x * 4..x * 4 + 4)?;
                    any_alpha |= p[3] != 0;
                    [p[2], p[1], p[0], p[3]]
                }
                24 => {
                    let p = row.get(x * 3..x * 3 + 3)?;
                    [p[2], p[1], p[0], 255]
                }
                1 | 4 | 8 => {
                    let bit = x * bpp as usize;
                    let byte = *row.get(bit / 8)?;
                    let shift = 8 - bpp as usize - bit % 8;
                    let idx = (byte >> shift) as usize & ((1 << bpp) - 1);
                    *palette.get(idx)?
                }
                _ => return None,
            };
            out[(y * w as usize + x) * 4..][..4].copy_from_slice(&px);
        }
    }
    // the AND mask gives transparency unless the image has its own alpha
    if bpp < 32 || !any_alpha {
        for y in 0..h as usize {
            let row = data.get(mask_off + (h as usize - 1 - y) * mask_stride..);
            for x in 0..w as usize {
                let transparent = row
                    .and_then(|r| r.get(x / 8))
                    .is_some_and(|b| b & (0x80 >> (x % 8)) != 0);
                out[(y * w as usize + x) * 4 + 3] = if transparent { 0 } else { 255 };
            }
        }
    }
    Some((out, w, h))
}

fn decode_png(data: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(data));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let src = &buf[..info.buffer_size()];
    let n = (w * h) as usize;
    let mut out = Vec::with_capacity(n * 4);
    match info.color_type {
        png::ColorType::Rgba => out.extend_from_slice(src),
        png::ColorType::Rgb => src
            .chunks_exact(3)
            .for_each(|p| out.extend_from_slice(&[p[0], p[1], p[2], 255])),
        png::ColorType::GrayscaleAlpha => src
            .chunks_exact(2)
            .for_each(|p| out.extend_from_slice(&[p[0], p[0], p[0], p[1]])),
        png::ColorType::Grayscale => src
            .iter()
            .for_each(|&v| out.extend_from_slice(&[v, v, v, 255])),
        png::ColorType::Indexed => return None,
    }
    (out.len() == n * 4).then_some((out, w, h))
}
