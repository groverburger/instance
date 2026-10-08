//! macOS `.icns` container with PNG-encoded entries (for the .app bundles made by
//! `scripts/macos-bundle.sh`).

use crate::icon_rgba;

/// PNG encoding of the app icon at `size`.
pub fn icon_png(size: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, size, size);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().expect("png header");
        w.write_image_data(&icon_rgba(size)).expect("png data");
    }
    out
}

/// OSType of each PNG entry and its pixel size.
const ENTRIES: [(&[u8; 4], u32); 10] = [
    (b"icp4", 16),
    (b"icp5", 32),
    (b"icp6", 64),
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic09", 512),
    (b"ic10", 1024),
    (b"ic11", 32),
    (b"ic12", 64),
    (b"ic13", 256),
];

/// A complete `.icns` file with the app icon from 16 to 1024 pixels.
pub fn icns_file() -> Vec<u8> {
    let mut body = Vec::new();
    for (ty, size) in ENTRIES {
        let png = icon_png(size);
        body.extend_from_slice(ty);
        body.extend_from_slice(&(png.len() as u32 + 8).to_be_bytes());
        body.extend_from_slice(&png);
    }
    let mut out = b"icns".to_vec();
    out.extend_from_slice(&(body.len() as u32 + 8).to_be_bytes());
    out.extend_from_slice(&body);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn icns_layout() {
        let f = super::icns_file();
        assert_eq!(&f[..4], b"icns");
        assert_eq!(u32::from_be_bytes(f[4..8].try_into().unwrap()) as usize, f.len());
        assert_eq!(&f[8..12], b"icp4");
        assert_eq!(&f[16..20], b"\x89PNG");
    }
}
