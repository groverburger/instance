//! gm81-clean brand artwork, shared by the IDE and the runner.
//!
//! The app icon is a red "pac" ball (a shaded disc with a wedge-shaped mouth opening to
//! the right and one eye), the same motif as the wordmark banner in the IDE. It is drawn
//! here from simple geometry with a small supersampling rasteriser, so every size is
//! rendered crisply from the vector description; no bitmap artwork is stored anywhere.
//!
//! Also here: writers for Windows `.ico` files and compiled resource (`.res`) files (so
//! the build scripts can embed the icon into the executables without a resource
//! compiler), an `.ico` decoder for game icons, and detection of the stock icon that
//! GameMaker 8.1 projects carry by default (by SHA-1 of the icon bytes).

mod icns;
mod ico;
mod sha1;

pub use icns::{icns_file, icon_png};
pub use ico::{decode_ico, ico_file, res_file};
pub use sha1::sha1;

/// Icon sizes written to the `.ico` / `.res` files.
pub const ICO_SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// SHA-1 digests of the icon bytes of the stock icon that GameMaker 8.1 puts into the
/// settings of every new project (it was found in the large majority of sample and user
/// projects). Only the digests are kept; the icon itself is not part of this repository.
pub const STOCK_ICON_SHA1: &[&str] = &["07f14546d6c7963e35df273cc8df8f83494bc5f6"];

/// Whether `icon` (the `.ico` bytes from a project's settings) is GameMaker's stock
/// default icon, or empty, i.e. the game did not choose an icon of its own.
pub fn is_stock_icon(icon: &[u8]) -> bool {
    if icon.is_empty() {
        return true;
    }
    let h = hex(&sha1(icon));
    STOCK_ICON_SHA1.iter().any(|s| s.eq_ignore_ascii_case(&h))
}

/// Lower-case hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ------------------------------------------------------------------ artwork

/// Geometry of the logo in a 16×16 design grid (y down).
struct Art {
    cx: f32,
    cy: f32,
    r: f32,
    /// Half the opening angle of the mouth (radians), opening towards +x.
    mouth: f32,
    eye: (f32, f32, f32),
    outline: f32,
}

const ART: Art = Art {
    cx: 7.7,
    cy: 8.0,
    r: 7.1,
    mouth: 0.62,
    eye: (8.5, 4.2, 1.05),
    outline: 0.5,
};

#[derive(Clone, Copy)]
struct Px {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Px {
    const CLEAR: Px = Px {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    fn rgb(r: u8, g: u8, b: u8) -> Px {
        Px {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }
    fn alpha(mut self, a: f32) -> Px {
        self.a *= a;
        self
    }
    /// `self` over `under` (straight alpha in, straight alpha out).
    fn over(self, under: Px) -> Px {
        let a = self.a + under.a * (1.0 - self.a);
        if a <= 0.0 {
            return Px::CLEAR;
        }
        let m = |s: f32, u: f32| (s * self.a + u * under.a * (1.0 - self.a)) / a;
        Px {
            r: m(self.r, under.r),
            g: m(self.g, under.g),
            b: m(self.b, under.b),
            a,
        }
    }
}

fn mix(a: Px, b: Px, t: f32) -> Px {
    let t = t.clamp(0.0, 1.0);
    Px {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

/// Distance from `p` to the segment `a`–`b`.
fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0);
    let (qx, qy) = (a.0 + t * dx - p.0, a.1 + t * dy - p.1);
    (qx * qx + qy * qy).sqrt()
}

/// Colour of the logo at design-grid point (`x`, `y`). `px` is the size of one output
/// pixel in grid units (fine details are dropped at small sizes).
fn sample(x: f32, y: f32, px: f32) -> Px {
    let a = &ART;
    let mut out = Px::CLEAR;
    // soft ground shadow
    if px < 0.5 {
        let (sx, sy) = ((x - a.cx) / 5.6, (y - 15.2) / 0.75);
        let d = sx * sx + sy * sy;
        if d < 1.0 {
            out = Px::rgb(0, 0, 0).alpha(0.28 * (1.0 - d));
        }
    }
    let (dx, dy) = (x - a.cx, y - a.cy);
    let dist = (dx * dx + dy * dy).sqrt();
    let ang = (-dy).atan2(dx); // counter-clockwise from +x, y up
    let in_mouth = ang.abs() < a.mouth;
    if dist > a.r || in_mouth {
        return out;
    }
    // body: vertical gradient plus a soft light from the upper left
    let t = (y - (a.cy - a.r)) / (2.0 * a.r);
    let mut c = mix(Px::rgb(255, 128, 116), Px::rgb(166, 18, 22), t);
    let (lx, ly) = (x - (a.cx - 2.6), y - (a.cy - 2.9));
    let l = (lx * lx + ly * ly).sqrt() / (a.r * 1.1);
    c = mix(c, Px::rgb(255, 196, 186), (1.0 - l).max(0.0) * 0.55);
    // highlight crescent (larger sizes)
    if px < 0.6 {
        let (hx, hy) = (x - (a.cx - 2.9), y - (a.cy - 3.5));
        let (u, v) = (hx * 0.866 + hy * 0.5, -hx * 0.5 + hy * 0.866);
        let e = (u / 2.1).powi(2) + (v / 1.05).powi(2);
        if e < 1.0 {
            c = Px::rgb(255, 255, 255)
                .alpha(0.55 * (1.0 - e).sqrt())
                .over(c);
        }
    }
    // eye
    let (ex, ey, er) = a.eye;
    let (edx, edy) = (x - ex, y - ey);
    let ed = (edx * edx + edy * edy).sqrt();
    let er = er.max(px * 0.9);
    if ed < er {
        c = Px::rgb(24, 10, 12);
        if px < 0.5 {
            let (gx, gy) = (x - (ex - 0.35), y - (ey - 0.35));
            if (gx * gx + gy * gy).sqrt() < er * 0.36 {
                c = Px::rgb(255, 255, 255);
            }
        }
    }
    // outline along the rim and the mouth edges
    let rim = a.r - dist;
    let tip = |s: f32| (a.cx + a.r * a.mouth.cos(), a.cy - s * a.r * a.mouth.sin());
    let edge = rim
        .min(seg_dist((x, y), (a.cx, a.cy), tip(1.0)))
        .min(seg_dist((x, y), (a.cx, a.cy), tip(-1.0)));
    if edge < a.outline.max(px * 0.85) {
        c = Px::rgb(112, 10, 14);
    }
    c.over(out)
}

/// Render the app icon as straight-alpha RGBA, `size`×`size` pixels.
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let size = size.max(1);
    let n: u32 = if size <= 64 { 8 } else { 4 };
    let px = 16.0 / size as f32;
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for j in 0..size {
        for i in 0..size {
            let (mut r, mut g, mut b, mut a) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for sj in 0..n {
                for si in 0..n {
                    let x = (i as f32 + (si as f32 + 0.5) / n as f32) * px;
                    let y = (j as f32 + (sj as f32 + 0.5) / n as f32) * px;
                    let s = sample(x, y, px);
                    r += s.r * s.a;
                    g += s.g * s.a;
                    b += s.b * s.a;
                    a += s.a;
                }
            }
            let k = (n * n) as f32;
            let to8 = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
            if a > 0.0 {
                out.extend_from_slice(&[to8(r / a), to8(g / a), to8(b / a), to8(a / k)]);
            } else {
                out.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_is_opaque_inside_and_clear_in_the_mouth() {
        let s = 64;
        let img = icon_rgba(s);
        let at = |x: u32, y: u32| img[((y * s + x) * 4 + 3) as usize];
        assert_eq!(at(20, 40), 255, "body");
        assert_eq!(at(63, 32), 0, "mouth");
        assert_eq!(at(1, 1), 0, "corner");
    }

    #[test]
    fn ico_round_trips() {
        let ico = ico_file(&[16, 32, 256]);
        let (rgba, w, h) = decode_ico(&ico, 32).unwrap();
        assert_eq!((w, h), (32, 32));
        assert_eq!(rgba, icon_rgba(32));
        let (_, w, _) = decode_ico(&ico, 300).unwrap();
        assert_eq!(w, 256);
    }

    #[test]
    fn res_file_has_group_icon() {
        let res = res_file(&[16, 32]);
        // empty leading entry + 2 icons + 1 group
        assert_eq!(&res[..8], &[0, 0, 0, 0, 32, 0, 0, 0]);
        assert!(res.len() > 16 * 16 * 4 + 32 * 32 * 4);
    }

    #[test]
    fn sha1_known_vectors() {
        assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            hex(&sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        let long = vec![b'a'; 1_000_000];
        assert_eq!(
            hex(&sha1(&long)),
            "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
        );
    }

    /// `GM_BRAND_DUMP=dir cargo test -p instance-brand -- --ignored dump` writes `app.ico`.
    #[test]
    #[ignore]
    fn dump() {
        let dir = std::env::var_os("GM_BRAND_DUMP").expect("GM_BRAND_DUMP");
        std::fs::write(
            std::path::Path::new(&dir).join("app.ico"),
            ico_file(&ICO_SIZES),
        )
        .unwrap();
    }

    #[test]
    fn empty_icon_counts_as_stock() {
        assert!(is_stock_icon(&[]));
        assert!(!is_stock_icon(&ico_file(&[16])));
    }
}
