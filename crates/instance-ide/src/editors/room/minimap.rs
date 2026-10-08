//! Room preview minimap: a downscaled software render of the whole room (background
//! colour and images, tiles, instance sprites), and the mapping between minimap
//! points and the canvas scroll offset.

use std::hash::{Hash, Hasher};

use eframe::egui::{Pos2, Rect, Vec2};
use instance_format::{Project, RgbaImage, Room};

/// What the room canvas (and the minimap) draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Show {
    pub objects: bool,
    pub tiles: bool,
    pub backgrounds: bool,
    pub foregrounds: bool,
    pub views: bool,
}

impl Default for Show {
    fn default() -> Self {
        Show {
            objects: true,
            tiles: true,
            backgrounds: true,
            foregrounds: true,
            views: true,
        }
    }
}

/// Rectangle of the room image inside `area`: the room scaled to fit, aspect kept,
/// centred.
pub fn fit(area: Rect, room: Vec2) -> Rect {
    let room = room.max(Vec2::splat(1.0));
    let s = (area.width() / room.x)
        .min(area.height() / room.y)
        .max(1e-6);
    Rect::from_center_size(area.center(), room * s)
}

/// Room point under minimap point `p` (clamped to the room).
pub fn to_room(img: Rect, room: Vec2, p: Pos2) -> Pos2 {
    let t = (p - img.min) / img.size().max(Vec2::splat(1e-6));
    let w = t * room;
    Pos2::new(w.x.clamp(0.0, room.x), w.y.clamp(0.0, room.y))
}

/// Canvas offset (room origin relative to the canvas' top-left, in screen points)
/// that centres room point `w` in a canvas of size `canvas` at zoom `zoom`.
pub fn offset_centering(w: Pos2, canvas: Vec2, zoom: f32) -> Vec2 {
    canvas / 2.0 - (w.to_vec2()) * zoom
}

/// Clamp a canvas offset so that the room never scrolls past its edges; a room
/// smaller than the canvas sits at the top-left corner (like GM's scrolled view).
pub fn clamp_offset(offset: Vec2, room: Vec2, canvas: Vec2, zoom: f32) -> Vec2 {
    let lo = (canvas - room * zoom).min(Vec2::ZERO);
    Vec2::new(offset.x.clamp(lo.x, 0.0), offset.y.clamp(lo.y, 0.0))
}

/// Clicking minimap point `p` scrolls the canvas so that the room point under it is
/// centred (as far as the room edges allow). Returns the new canvas offset.
pub fn click_to_offset(img: Rect, room: Vec2, p: Pos2, canvas: Vec2, zoom: f32) -> Vec2 {
    let w = to_room(img, room, p);
    clamp_offset(offset_centering(w, canvas, zoom), room, canvas, zoom)
}

/// The part of the room visible in the canvas, as a minimap rectangle.
pub fn visible_rect(img: Rect, room: Vec2, canvas: Vec2, offset: Vec2, zoom: f32) -> Rect {
    let room = room.max(Vec2::splat(1.0));
    let s = img.size() / room;
    let w0 = (-offset) / zoom;
    let w1 = (canvas - offset) / zoom;
    Rect::from_min_max(img.min + w0 * s, img.min + w1 * s).intersect(img)
}

/// Fingerprint of everything the minimap depends on.
pub fn fingerprint(room: &Room, show: Show, gen: u64, size: (u32, u32)) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (gen, size, show).hash(&mut h);
    (
        room.width,
        room.height,
        room.background_color,
        room.draw_background_color,
    )
        .hash(&mut h);
    for b in &room.backgrounds {
        (
            b.visible,
            b.is_foreground,
            b.background,
            b.x,
            b.y,
            b.tile_h,
            b.tile_v,
            b.stretch,
        )
            .hash(&mut h);
    }
    for i in &room.instances {
        (i.x, i.y, i.object).hash(&mut h);
    }
    for t in &room.tiles {
        (
            t.x,
            t.y,
            t.background,
            t.tile_x,
            t.tile_y,
            t.width,
            t.height,
            t.depth,
        )
            .hash(&mut h);
    }
    h.finish()
}

struct Canvas {
    w: usize,
    h: usize,
    /// Room units per pixel.
    inv: f32,
    px: Vec<[f32; 3]>,
}

impl Canvas {
    #[inline]
    fn blend(&mut self, x: usize, y: usize, c: [u8; 4]) {
        if c[3] == 0 {
            return;
        }
        let a = c[3] as f32 / 255.0;
        let d = &mut self.px[y * self.w + x];
        for k in 0..3 {
            d[k] += (c[k] as f32 - d[k]) * a;
        }
    }

    /// Pixel range covering room span [a, b).
    fn span(&self, a: f32, b: f32, n: usize) -> (usize, usize) {
        let lo = (a / self.inv).floor().max(0.0) as usize;
        let hi = ((b / self.inv).ceil().max(0.0) as usize).min(n);
        (lo.min(n), hi)
    }

    /// Draw a region of `img` (src rect sx, sy, sw, sh) at room rect (x, y, w, h).
    #[allow(clippy::too_many_arguments)]
    fn image(
        &mut self,
        img: &RgbaImage,
        (sx, sy, sw, sh): (f32, f32, f32, f32),
        (x, y, w, h): (f32, f32, f32, f32),
    ) {
        if !img.is_consistent() || img.width == 0 || img.height == 0 || w <= 0.0 || h <= 0.0 {
            return;
        }
        let (x0, x1) = self.span(x, x + w, self.w);
        let (y0, y1) = self.span(y, y + h, self.h);
        for py in y0..y1 {
            let ry = (py as f32 + 0.5) * self.inv;
            if ry < y || ry >= y + h {
                continue;
            }
            let iy = (sy + (ry - y) / h * sh).floor() as i64;
            if iy < 0 || iy >= img.height as i64 {
                continue;
            }
            for px in x0..x1 {
                let rx = (px as f32 + 0.5) * self.inv;
                if rx < x || rx >= x + w {
                    continue;
                }
                let ix = (sx + (rx - x) / w * sw).floor() as i64;
                if ix < 0 || ix >= img.width as i64 {
                    continue;
                }
                self.blend(px, py, img.pixel(ix as u32, iy as u32));
            }
        }
    }
}

fn bgr(c: u32) -> [f32; 3] {
    if c >> 24 != 0 {
        return [212.0; 3];
    }
    [
        (c & 0xFF) as f32,
        ((c >> 8) & 0xFF) as f32,
        ((c >> 16) & 0xFF) as f32,
    ]
}

fn draw_backgrounds(c: &mut Canvas, p: &Project, room: &Room, foreground: bool) {
    let (rw, rh) = (room.width.max(1) as f32, room.height.max(1) as f32);
    for b in &room.backgrounds {
        if !b.visible || b.is_foreground != foreground {
            continue;
        }
        let Some(bg) = p
            .backgrounds
            .get(usize::try_from(b.background).unwrap_or(usize::MAX))
            .and_then(|x| x.as_ref())
        else {
            continue;
        };
        let img = &bg.image;
        if !img.is_consistent() || img.width == 0 || img.height == 0 {
            continue;
        }
        let (iw, ih) = (img.width as f32, img.height as f32);
        for py in 0..c.h {
            let ry = (py as f32 + 0.5) * c.inv;
            let sy = if b.stretch {
                ry / rh * ih
            } else if b.tile_v {
                (ry - b.y as f32).rem_euclid(ih)
            } else {
                ry - b.y as f32
            };
            if sy < 0.0 || sy >= ih {
                continue;
            }
            for px in 0..c.w {
                let rx = (px as f32 + 0.5) * c.inv;
                let sx = if b.stretch {
                    rx / rw * iw
                } else if b.tile_h {
                    (rx - b.x as f32).rem_euclid(iw)
                } else {
                    rx - b.x as f32
                };
                if sx < 0.0 || sx >= iw {
                    continue;
                }
                let px_ = img.pixel(sx as u32, sy as u32);
                c.blend(px, py, px_);
            }
        }
    }
}

/// Render the room into a `w`×`h` image (2×2 supersampled).
pub fn render(p: &Project, room: &Room, show: Show, w: u32, h: u32) -> RgbaImage {
    const SS: usize = 2;
    let (w, h) = (w.max(1) as usize, h.max(1) as usize);
    let (cw, ch) = (w * SS, h * SS);
    let inv = (room.width.max(1) as f32 / cw as f32).max(room.height.max(1) as f32 / ch as f32);
    let base = if room.draw_background_color {
        bgr(room.background_color)
    } else {
        [160.0; 3]
    };
    let mut c = Canvas {
        w: cw,
        h: ch,
        inv,
        px: vec![base; cw * ch],
    };
    if show.backgrounds {
        draw_backgrounds(&mut c, p, room, false);
    }
    enum Item {
        Tile(usize),
        Inst(usize),
    }
    let depth = |o: i32| {
        p.objects
            .get(usize::try_from(o).unwrap_or(usize::MAX))
            .and_then(|x| x.as_ref())
            .map_or(0, |o| o.depth)
    };
    let mut items: Vec<(i32, usize, Item)> = Vec::new();
    if show.tiles {
        for (i, t) in room.tiles.iter().enumerate() {
            items.push((t.depth, i, Item::Tile(i)));
        }
    }
    if show.objects {
        for (i, inst) in room.instances.iter().enumerate() {
            items.push((depth(inst.object), i, Item::Inst(i)));
        }
    }
    items.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, _, it) in items {
        match it {
            Item::Tile(i) => {
                let t = &room.tiles[i];
                if let Some(bg) = p
                    .backgrounds
                    .get(usize::try_from(t.background).unwrap_or(usize::MAX))
                    .and_then(|x| x.as_ref())
                {
                    c.image(
                        &bg.image,
                        (
                            t.tile_x as f32,
                            t.tile_y as f32,
                            t.width as f32,
                            t.height as f32,
                        ),
                        (t.x as f32, t.y as f32, t.width as f32, t.height as f32),
                    );
                }
            }
            Item::Inst(i) => {
                let inst = &room.instances[i];
                let spr = p
                    .objects
                    .get(usize::try_from(inst.object).unwrap_or(usize::MAX))
                    .and_then(|o| o.as_ref())
                    .and_then(|o| p.sprites.get(usize::try_from(o.sprite).ok()?))
                    .and_then(|s| s.as_ref());
                match spr.and_then(|s| s.frames.first().map(|f| (s, f))) {
                    Some((s, f)) => {
                        let (fw, fh) = (f.width as f32, f.height as f32);
                        c.image(
                            f,
                            (0.0, 0.0, fw, fh),
                            (
                                (inst.x - s.origin_x) as f32,
                                (inst.y - s.origin_y) as f32,
                                fw,
                                fh,
                            ),
                        );
                    }
                    None => {
                        // placeholder: a blue 16×16 dot
                        let dot = RgbaImage {
                            width: 1,
                            height: 1,
                            data: vec![40, 90, 220, 255],
                        };
                        c.image(
                            &dot,
                            (0.0, 0.0, 1.0, 1.0),
                            (inst.x as f32, inst.y as f32, 16.0, 16.0),
                        );
                    }
                }
            }
        }
    }
    if show.foregrounds {
        draw_backgrounds(&mut c, p, room, true);
    }
    // downsample
    let mut out = RgbaImage::new(w as u32, h as u32);
    let n = (SS * SS) as f32;
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 3];
            for sy in 0..SS {
                for sx in 0..SS {
                    let s = c.px[(y * SS + sy) * cw + x * SS + sx];
                    for k in 0..3 {
                        acc[k] += s[k];
                    }
                }
            }
            let o = (y * w + x) * 4;
            for (d, a) in out.data[o..o + 3].iter_mut().zip(acc) {
                *d = (a / n).round().clamp(0.0, 255.0) as u8;
            }
            out.data[o + 3] = 255;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    fn approx(a: Pos2, b: Pos2) -> bool {
        (a - b).length() < 0.01
    }

    #[test]
    fn fit_keeps_aspect_and_centres() {
        let area = Rect::from_min_size(pos2(10.0, 20.0), vec2(200.0, 100.0));
        let r = fit(area, vec2(1600.0, 400.0));
        assert!((r.width() - 200.0).abs() < 1e-3);
        assert!((r.height() - 50.0).abs() < 1e-3);
        assert!(approx(r.center(), area.center()));
        let r = fit(area, vec2(320.0, 640.0));
        assert!((r.height() - 100.0).abs() < 1e-3);
        assert!((r.width() - 50.0).abs() < 1e-3);
    }

    #[test]
    fn click_centres_the_canvas_on_the_room_point() {
        // room 2000×1000 shown in a 200×100 minimap at (0, 0): 1 px = 10 room units
        let img = Rect::from_min_size(Pos2::ZERO, vec2(200.0, 100.0));
        let room = vec2(2000.0, 1000.0);
        let canvas = vec2(400.0, 300.0);
        let off = click_to_offset(img, room, pos2(100.0, 50.0), canvas, 1.0);
        // room point (1000, 500) is at the canvas centre
        assert_eq!(off, vec2(200.0 - 1000.0, 150.0 - 500.0));
        // the visible rectangle on the minimap is centred on the click
        let v = visible_rect(img, room, canvas, off, 1.0);
        assert!(approx(v.center(), pos2(100.0, 50.0)));
        assert!((v.width() - 40.0).abs() < 1e-3 && (v.height() - 30.0).abs() < 1e-3);
        // zoom 2: half as much room is visible
        let off = click_to_offset(img, room, pos2(100.0, 50.0), canvas, 2.0);
        assert_eq!(off, vec2(200.0 - 2000.0, 150.0 - 1000.0));
        let v = visible_rect(img, room, canvas, off, 2.0);
        assert!(approx(v.center(), pos2(100.0, 50.0)));
        assert!((v.width() - 20.0).abs() < 1e-3);
    }

    #[test]
    fn click_near_edges_is_clamped_to_the_room() {
        let img = Rect::from_min_size(pos2(5.0, 5.0), vec2(200.0, 100.0));
        let room = vec2(2000.0, 1000.0);
        let canvas = vec2(400.0, 300.0);
        // top-left corner (and beyond) → no scroll
        assert_eq!(
            click_to_offset(img, room, pos2(0.0, 0.0), canvas, 1.0),
            Vec2::ZERO
        );
        assert_eq!(
            click_to_offset(img, room, pos2(6.0, 6.0), canvas, 1.0),
            Vec2::ZERO
        );
        // bottom-right → scrolled to the end
        let off = click_to_offset(img, room, pos2(500.0, 500.0), canvas, 1.0);
        assert_eq!(off, vec2(400.0 - 2000.0, 300.0 - 1000.0));
        let v = visible_rect(img, room, canvas, off, 1.0);
        assert!(approx(v.max, img.max));
        // a room smaller than the canvas never scrolls
        let small = vec2(100.0, 100.0);
        let img = fit(img, small);
        assert_eq!(
            click_to_offset(img, small, img.center(), canvas, 1.0),
            Vec2::ZERO
        );
    }

    #[test]
    fn render_draws_background_colour_and_instances() {
        let mut p = Project::default();
        let mut room = Room {
            width: 64,
            height: 32,
            background_color: 0x0000FF, // red (BGR)
            ..Default::default()
        };
        room.instances.push(instance_format::Instance {
            x: 0,
            y: 0,
            object: -1,
            id: 1,
            creation_code: String::new(),
            locked: false,
        });
        p.rooms.push(Some(room.clone()));
        let img = render(&p, &room, Show::default(), 8, 4);
        assert_eq!((img.width, img.height), (8, 4));
        // placeholder (blue) covers 16×16 room units = top-left 2×2 px
        assert_eq!(img.pixel(0, 0), [40, 90, 220, 255]);
        assert_eq!(img.pixel(7, 3), [255, 0, 0, 255]);
        // hiding objects leaves the background colour
        let img = render(
            &p,
            &room,
            Show {
                objects: false,
                ..Show::default()
            },
            8,
            4,
        );
        assert_eq!(img.pixel(0, 0), [255, 0, 0, 255]);
    }
}
