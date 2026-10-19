//! Runtime resource tables (sprites, backgrounds, paths, ...), built from the
//! loaded [`instance_format::Project`].

use crate::actions::ActionList;
use crate::draw::Texture;
use instance_format as fmt;
use gml::Program;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

/// A collision mask: one bit per pixel of the sprite (sprite-local coordinates).
#[derive(Clone, Debug)]
pub struct Mask {
    pub w: u32,
    pub h: u32,
    pub bits: Vec<bool>,
    /// Inclusive sprite-local bounding box.
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Mask {
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> bool {
        if x < self.left || y < self.top || x > self.right || y > self.bottom {
            return false;
        }
        if x < 0 || y < 0 || x as u32 >= self.w || y as u32 >= self.h {
            return false;
        }
        self.bits[(y as u32 * self.w + x as u32) as usize]
    }
}

impl std::ops::Deref for SpriteMasks {
    type Target = [Mask];
    fn deref(&self) -> &[Mask] {
        &self.masks
    }
}

/// The masks of a sprite: one per frame, or a single combined one.
#[derive(Clone, Debug, Default)]
pub struct SpriteMasks {
    pub masks: Vec<Mask>,
}

/// A sprite resource.
#[derive(Clone, Debug)]
pub struct SpriteRes {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub frames: Vec<Rc<Texture>>,
    pub masks: SpriteMasks,
    /// Collision shape: 0 precise, 1 rectangle, 2 ellipse, 3 diamond.
    pub shape: i32,
    pub separate: bool,
    pub bbox_mode: i32,
    pub tolerance: i32,
    pub bbox_left: i32,
    pub bbox_top: i32,
    pub bbox_right: i32,
    pub bbox_bottom: i32,
    pub smooth: bool,
    pub preload: bool,
}

impl SpriteRes {
    pub fn from_format(s: &fmt::Sprite) -> Self {
        let frames: Vec<Rc<Texture>> = s.frames.iter().map(|f| Rc::new(Texture::from_rgba(f))).collect();
        let width = s.frames.first().map(|f| f.width).unwrap_or(0);
        let height = s.frames.first().map(|f| f.height).unwrap_or(0);
        let mut r = SpriteRes {
            name: s.name.clone(),
            width,
            height,
            origin_x: s.origin_x,
            origin_y: s.origin_y,
            frames,
            masks: SpriteMasks::default(),
            shape: s.collision_shape,
            separate: s.separate_collision_masks,
            bbox_mode: s.bbox_mode,
            tolerance: s.alpha_tolerance,
            bbox_left: s.bbox_left,
            bbox_top: s.bbox_top,
            bbox_right: s.bbox_right,
            bbox_bottom: s.bbox_bottom,
            smooth: false,
            preload: true,
        };
        r.rebuild_masks();
        r
    }

    /// Builds a sprite from frames (sprite_add etc.).
    pub fn from_frames(name: String, frames: Vec<Texture>, ox: i32, oy: i32, precise: bool) -> Self {
        let width = frames.first().map(|f| f.w).unwrap_or(0);
        let height = frames.first().map(|f| f.h).unwrap_or(0);
        let mut r = SpriteRes {
            name,
            width,
            height,
            origin_x: ox,
            origin_y: oy,
            frames: frames.into_iter().map(Rc::new).collect(),
            masks: SpriteMasks::default(),
            shape: if precise { 0 } else { 1 },
            separate: false,
            bbox_mode: 0,
            tolerance: 0,
            bbox_left: 0,
            bbox_top: 0,
            bbox_right: width as i32 - 1,
            bbox_bottom: height as i32 - 1,
            smooth: false,
            preload: true,
        };
        r.rebuild_masks();
        r
    }

    /// (Re)computes collision masks per runtime.md §8.1.
    pub fn rebuild_masks(&mut self) {
        let (w, h) = (self.width, self.height);
        let tol = self.tolerance.clamp(0, 255) as u8;
        let opaque = |t: &Texture| -> Vec<bool> { t.px.iter().map(|p| p[3] > tol).collect() };
        // raw alpha masks: per frame or combined
        let mut raws: Vec<Vec<bool>> = Vec::new();
        if self.separate {
            for f in &self.frames {
                raws.push(opaque(f));
            }
        } else {
            let mut u = vec![false; (w * h) as usize];
            for f in &self.frames {
                if f.w == w && f.h == h {
                    for (a, b) in u.iter_mut().zip(opaque(f)) {
                        *a |= b;
                    }
                }
            }
            raws.push(u);
        }
        if raws.is_empty() {
            raws.push(vec![false; (w * h) as usize]);
        }
        let mut masks = Vec::new();
        for raw in raws {
            // bounding box
            let (l, t, r, b) = match self.bbox_mode {
                1 => (0, 0, w as i32 - 1, h as i32 - 1),
                // a user box is clipped to the image: the mask is a per-pixel
                // image of the sprite (OPEN_QUESTIONS R-12)
                2 => {
                    let (l, t) = (self.bbox_left.max(0), self.bbox_top.max(0));
                    let r = self.bbox_right.min(w as i32 - 1).max(l);
                    let b = self.bbox_bottom.min(h as i32 - 1).max(t);
                    (l.min(r), t.min(b), r, b)
                }
                _ => {
                    let (mut l, mut t, mut r, mut b) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
                    for y in 0..h as i32 {
                        for x in 0..w as i32 {
                            if raw[(y as u32 * w + x as u32) as usize] {
                                l = l.min(x);
                                r = r.max(x);
                                t = t.min(y);
                                b = b.max(y);
                            }
                        }
                    }
                    if l == i32::MAX {
                        (0, 0, w as i32 - 1, h as i32 - 1)
                    } else {
                        (l, t, r, b)
                    }
                }
            };
            let mut bits = vec![false; (w * h) as usize];
            let cx = (l + r) as f64 / 2.0;
            let cy = (t + b) as f64 / 2.0;
            let rx = (r - l + 1) as f64 / 2.0;
            let ry = (b - t + 1) as f64 / 2.0;
            for y in t.max(0)..=b.min(h as i32 - 1) {
                for x in l.max(0)..=r.min(w as i32 - 1) {
                    let i = (y as u32 * w + x as u32) as usize;
                    bits[i] = match self.shape {
                        0 => raw[i],
                        2 => {
                            let dx = (x as f64 - cx) / rx;
                            let dy = (y as f64 - cy) / ry;
                            dx * dx + dy * dy <= 1.0
                        }
                        3 => {
                            let dx = (x as f64 - cx).abs() / rx;
                            let dy = (y as f64 - cy).abs() / ry;
                            dx + dy <= 1.0
                        }
                        _ => true,
                    };
                }
            }
            masks.push(Mask {
                w,
                h,
                bits,
                left: l,
                top: t,
                right: r,
                bottom: b,
            });
        }
        self.masks = SpriteMasks { masks };
        if let Some(m) = self.masks.first() {
            if !self.separate || self.bbox_mode != 0 {
                self.bbox_left = m.left;
                self.bbox_top = m.top;
                self.bbox_right = m.right;
                self.bbox_bottom = m.bottom;
            } else {
                // union over frames for sprite_get_bbox_*
                self.bbox_left = self.masks.iter().map(|m| m.left).min().unwrap();
                self.bbox_top = self.masks.iter().map(|m| m.top).min().unwrap();
                self.bbox_right = self.masks.iter().map(|m| m.right).max().unwrap();
                self.bbox_bottom = self.masks.iter().map(|m| m.bottom).max().unwrap();
            }
        }
    }

    /// The mask used for subimage `image_index`.
    pub fn mask_for(&self, image_index: f64) -> Option<&Mask> {
        let n = self.masks.len();
        if n == 0 {
            return None;
        }
        if n == 1 {
            return self.masks.first();
        }
        let i = (image_index.floor() as i64).rem_euclid(n as i64) as usize;
        self.masks.get(i)
    }

    /// Whether the collision test needs per-pixel work.
    pub fn is_precise(&self) -> bool {
        self.shape != 1
    }
}

/// A background resource.
#[derive(Clone, Debug)]
pub struct BackgroundRes {
    pub name: String,
    pub tex: Rc<Texture>,
    pub tileset: bool,
    pub tile_width: i32,
    pub tile_height: i32,
    pub tile_hoffset: i32,
    pub tile_voffset: i32,
    pub tile_hsep: i32,
    pub tile_vsep: i32,
    pub smooth: bool,
    pub preload: bool,
}

impl BackgroundRes {
    pub fn from_format(b: &fmt::Background) -> Self {
        BackgroundRes {
            name: b.name.clone(),
            tex: Rc::new(Texture::from_rgba(&b.image)),
            tileset: b.use_as_tileset,
            tile_width: b.tile_width,
            tile_height: b.tile_height,
            tile_hoffset: b.tile_h_offset,
            tile_voffset: b.tile_v_offset,
            tile_hsep: b.tile_h_sep,
            tile_vsep: b.tile_v_sep,
            smooth: false,
            preload: true,
        }
    }
}

/// A sound resource.
pub use instance_audio::SoundRes;

/// A font resource.
#[derive(Clone, Debug)]
pub struct FontRes {
    pub name: String,
    pub font_name: String,
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
    pub first: i32,
    pub last: i32,
    /// A sprite-based font (font_add_sprite): sprite index, first char, proportional, sep.
    pub sprite: Option<(i32, i32, bool, i32)>,
}

/// A script resource.
#[derive(Clone)]
pub struct ScriptRes {
    pub name: String,
    pub code: String,
    pub prog: Option<Rc<Program>>,
    pub error: Option<String>,
}

/// A time line.
#[derive(Clone, Default)]
pub struct TimelineRes {
    pub name: String,
    pub moments: BTreeMap<i32, Rc<ActionList>>,
}

/// An object resource with its own (non-inherited) event table.
#[derive(Clone)]
pub struct ObjectRes {
    pub name: String,
    pub sprite: i32,
    pub solid: bool,
    pub visible: bool,
    pub depth: i32,
    pub persistent: bool,
    pub parent: i32,
    pub mask: i32,
    pub events: HashMap<(i32, i32), Rc<ActionList>>,
}

/// A trigger.
#[derive(Clone)]
pub struct TriggerRes {
    pub name: String,
    pub condition: Option<Rc<Program>>,
    pub moment: i32,
}

/// A room resource with its compiled creation code.
#[derive(Clone)]
pub struct RoomRes {
    pub data: fmt::Room,
    pub creation: Option<Rc<Program>>,
    /// Compiled creation code of placed instances, by instance id.
    pub inst_creation: HashMap<i32, Rc<Program>>,
}
