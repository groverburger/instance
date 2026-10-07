//! Collision geometry and tests (runtime.md §8).

use crate::game::Game;
use crate::instance::Instance;
use crate::res::Mask;

/// Collision geometry of an instance at a position.
pub struct Geo<'a> {
    pub mask: &'a Mask,
    pub x: f64,
    pub y: f64,
    pub xs: f64,
    pub ys: f64,
    pub cos: f64,
    pub sin: f64,
    pub rotated: bool,
    pub ox: f64,
    pub oy: f64,
    /// Inclusive integer bounding box: left, top, right, bottom.
    pub bbox: [i32; 4],
    pub precise: bool,
}

impl Geo<'_> {
    /// Whether room point (px, py) lies in the mask.
    #[inline]
    pub fn contains(&self, px: f64, py: f64) -> bool {
        if (px.floor() as i32) < self.bbox[0]
            || (px.floor() as i32) > self.bbox[2]
            || (py.floor() as i32) < self.bbox[1]
            || (py.floor() as i32) > self.bbox[3]
        {
            return false;
        }
        if !self.precise && !self.rotated {
            return true;
        }
        let (dx, dy) = (px - self.x, py - self.y);
        let (rx, ry) = if self.rotated {
            (dx * self.cos - dy * self.sin, dx * self.sin + dy * self.cos)
        } else {
            (dx, dy)
        };
        if self.xs == 0.0 || self.ys == 0.0 {
            return false;
        }
        let lx = rx / self.xs + self.ox;
        let ly = ry / self.ys + self.oy;
        self.mask.get(lx.floor() as i32, ly.floor() as i32)
    }
}

/// Computes the instance bounding box from a sprite-local box (§8.2).
pub fn compute_bbox(
    x: f64,
    y: f64,
    xs: f64,
    ys: f64,
    angle: f64,
    ox: f64,
    oy: f64,
    l: i32,
    t: i32,
    r: i32,
    b: i32,
) -> [i32; 4] {
    let lx0 = (l as f64 - ox) * xs;
    let lx1 = (r as f64 + 1.0 - ox) * xs;
    let ly0 = (t as f64 - oy) * ys;
    let ly1 = (b as f64 + 1.0 - oy) * ys;
    let (minx, maxx, miny, maxy) = if angle.rem_euclid(360.0) == 0.0 {
        (x + lx0.min(lx1), x + lx0.max(lx1), y + ly0.min(ly1), y + ly0.max(ly1))
    } else {
        let a = angle.to_radians();
        let (c, s) = (a.cos(), a.sin());
        let mut xs_ = [0.0; 4];
        let mut ys_ = [0.0; 4];
        for (k, (px, py)) in [(lx0, ly0), (lx1, ly0), (lx1, ly1), (lx0, ly1)].iter().enumerate() {
            xs_[k] = x + px * c + py * s;
            ys_[k] = y - px * s + py * c;
        }
        (
            xs_.iter().cloned().fold(f64::MAX, f64::min),
            xs_.iter().cloned().fold(f64::MIN, f64::max),
            ys_.iter().cloned().fold(f64::MAX, f64::min),
            ys_.iter().cloned().fold(f64::MIN, f64::max),
        )
    };
    [
        minx.round() as i32,
        miny.round() as i32,
        (maxx.round() as i32 - 1).max(minx.round() as i32),
        (maxy.round() as i32 - 1).max(miny.round() as i32),
    ]
}

/// A resolved object argument of a query function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sel {
    Nothing,
    All,
    Id(i32),
    Obj(i32),
}

/// A query shape for collision_* functions.
#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Point(f64, f64),
    Rect(f64, f64, f64, f64),
    Ellipse(f64, f64, f64, f64),
    Circle(f64, f64, f64),
    Line(f64, f64, f64, f64),
}

impl Shape {
    pub fn bbox(&self) -> [i32; 4] {
        let f = |a: f64| a.floor() as i32;
        match *self {
            Shape::Point(x, y) => [f(x), f(y), f(x), f(y)],
            Shape::Rect(x1, y1, x2, y2) | Shape::Ellipse(x1, y1, x2, y2) => {
                [f(x1.min(x2)), f(y1.min(y2)), f(x1.max(x2)), f(y1.max(y2))]
            }
            Shape::Circle(x, y, r) => [f(x - r), f(y - r), f(x + r), f(y + r)],
            Shape::Line(x1, y1, x2, y2) => [f(x1.min(x2)), f(y1.min(y2)), f(x1.max(x2)), f(y1.max(y2))],
        }
    }

    /// Whether pixel (px, py) belongs to the shape.
    fn contains_pixel(&self, px: i32, py: i32) -> bool {
        let (cx, cy) = (px as f64 + 0.5, py as f64 + 0.5);
        match *self {
            Shape::Point(x, y) => x.floor() as i32 == px && y.floor() as i32 == py,
            Shape::Rect(x1, y1, x2, y2) => {
                let (l, r) = (x1.min(x2).floor() as i32, x1.max(x2).floor() as i32);
                let (t, b) = (y1.min(y2).floor() as i32, y1.max(y2).floor() as i32);
                px >= l && px <= r && py >= t && py <= b
            }
            Shape::Ellipse(x1, y1, x2, y2) => {
                let (mx, my) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
                let (rx, ry) = (((x2 - x1).abs() / 2.0).max(0.5), ((y2 - y1).abs() / 2.0).max(0.5));
                let (dx, dy) = ((cx - mx) / rx, (cy - my) / ry);
                dx * dx + dy * dy <= 1.0
            }
            Shape::Circle(x, y, r) => {
                let (dx, dy) = (cx - x, cy - y);
                dx * dx + dy * dy <= r * r
            }
            Shape::Line(x1, y1, x2, y2) => {
                let (dx, dy) = (x2 - x1, y2 - y1);
                let l2 = dx * dx + dy * dy;
                let t = if l2 == 0.0 {
                    0.0
                } else {
                    (((cx - x1) * dx + (cy - y1) * dy) / l2).clamp(0.0, 1.0)
                };
                let (qx, qy) = (x1 + t * dx, y1 + t * dy);
                (qx - cx).abs() <= 0.5 && (qy - cy).abs() <= 0.5
            }
        }
    }
}

impl Game {
    /// Collision geometry of an instance placed at (x, y).
    pub fn geo_at<'a>(&'a self, i: &Instance, x: f64, y: f64) -> Option<Geo<'a>> {
        let spr_i = if i.mask_index >= 0 { i.mask_index } else { i.sprite_index };
        let spr = self.sprite(spr_i)?;
        let mask = spr.mask_for(i.image_index)?;
        let rotated = i.image_angle.rem_euclid(360.0) != 0.0;
        let a = i.image_angle.to_radians();
        let bbox = compute_bbox(
            x,
            y,
            i.image_xscale,
            i.image_yscale,
            i.image_angle,
            spr.origin_x as f64,
            spr.origin_y as f64,
            mask.left,
            mask.top,
            mask.right,
            mask.bottom,
        );
        Some(Geo {
            mask,
            x,
            y,
            xs: i.image_xscale,
            ys: i.image_yscale,
            cos: a.cos(),
            sin: a.sin(),
            rotated,
            ox: spr.origin_x as f64,
            oy: spr.origin_y as f64,
            bbox,
            precise: spr.is_precise(),
        })
    }

    /// Bounding box of an instance (bbox_* variables). Without a mask, the point
    /// (x, y).
    pub fn bbox_of(&self, id: i32) -> (f64, f64, f64, f64) {
        let Some(i) = self.inst(id) else { return (0.0, 0.0, 0.0, 0.0) };
        match self.geo_at(i, i.x, i.y) {
            Some(g) => (g.bbox[0] as f64, g.bbox[1] as f64, g.bbox[2] as f64, g.bbox[3] as f64),
            None => (i.x, i.y, i.x, i.y),
        }
    }

    /// Geometry-level collision test.
    pub fn geos_collide(a: &Geo, b: &Geo) -> bool {
        let l = a.bbox[0].max(b.bbox[0]);
        let r = a.bbox[2].min(b.bbox[2]);
        let t = a.bbox[1].max(b.bbox[1]);
        let bt = a.bbox[3].min(b.bbox[3]);
        if l > r || t > bt {
            return false;
        }
        if !a.precise && !b.precise && !a.rotated && !b.rotated {
            return true;
        }
        for py in t..=bt {
            for px in l..=r {
                let (cx, cy) = (px as f64 + 0.5, py as f64 + 0.5);
                if a.contains(cx, cy) && b.contains(cx, cy) {
                    return true;
                }
            }
        }
        false
    }

    /// Whether two instances (at their current positions) collide.
    pub fn instances_collide(&self, a: i32, b: i32) -> bool {
        let (Some(ia), Some(ib)) = (self.inst(a), self.inst(b)) else { return false };
        let (Some(ga), Some(gb)) = (self.geo_at(ia, ia.x, ia.y), self.geo_at(ib, ib.x, ib.y)) else {
            return false;
        };
        Self::geos_collide(&ga, &gb)
    }

    /// Positions (in `self.instances`, ascending) of the live instances with a
    /// collision mask whose bounding box overlaps `bbox` (broad phase).
    pub fn candidates(&self, bbox: [i32; 4]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut sp = self.spatial.borrow_mut();
        sp.flush(self);
        sp.query(bbox, &mut out);
        out
    }

    /// First instance (in instance order) passing `filter` that collides with
    /// instance `id` moved to (x, y).
    pub fn place_check(&self, id: i32, x: f64, y: f64, filter: impl Fn(&Instance) -> bool) -> Option<i32> {
        let me = self.inst(id)?;
        let g = self.geo_at(me, x, y)?;
        for k in self.candidates(g.bbox) {
            let o = &self.instances[k as usize];
            if o.id == id || !o.alive() || !filter(o) {
                continue;
            }
            if let Some(go) = self.geo_at(o, o.x, o.y) {
                if Self::geos_collide(&g, &go) {
                    return Some(o.id);
                }
            }
        }
        None
    }

    /// Resolves an object argument (object, instance id, self, other, all).
    pub fn sel(&self, ctx: &gml::Ctx, target: i32) -> Sel {
        use gml::special::*;
        match target {
            SELF => Sel::Id(ctx.self_id),
            OTHER => Sel::Id(ctx.other_id),
            ALL => Sel::All,
            t if t >= FIRST_INSTANCE_ID => Sel::Id(t),
            t if t >= 0 => Sel::Obj(t),
            _ => Sel::Nothing,
        }
    }

    /// Whether instance `i` is designated by `s` (liveness not checked).
    #[inline]
    pub fn sel_match(&self, s: Sel, i: &Instance) -> bool {
        match s {
            Sel::Nothing => false,
            Sel::All => true,
            Sel::Id(id) => i.id == id,
            Sel::Obj(o) => self.is_a(i.object_index, o),
        }
    }

    /// `place_meeting` / `instance_place` against an object argument.
    pub fn place_meeting_sel(&self, id: i32, x: f64, y: f64, s: Sel) -> Option<i32> {
        if let Sel::Nothing = s {
            return None;
        }
        self.place_check(id, x, y, |o| self.sel_match(s, o))
    }

    /// First instance designated by `s` (in instance order) whose mask contains
    /// point (x, y).
    pub fn instance_at_point(&self, x: f64, y: f64, s: Sel) -> Option<i32> {
        let (fx, fy) = (x.floor() as i32, y.floor() as i32);
        for k in self.candidates([fx, fy, fx, fy]) {
            let o = &self.instances[k as usize];
            if !o.alive() || !self.sel_match(s, o) {
                continue;
            }
            if self.point_in_instance(o.id, x, y, true) {
                return Some(o.id);
            }
        }
        None
    }

    /// All live instances (in instance order) whose mask contains point (x, y).
    pub fn instances_at_point(&self, x: f64, y: f64) -> Vec<i32> {
        let (fx, fy) = (x.floor() as i32, y.floor() as i32);
        let mut v = Vec::new();
        for k in self.candidates([fx, fy, fx, fy]) {
            let o = &self.instances[k as usize];
            if o.alive() && self.point_in_instance(o.id, x, y, true) {
                v.push(o.id);
            }
        }
        v
    }

    /// collision_* query against an object argument.
    pub fn shape_collision_sel(&self, shape: Shape, s: Sel, prec: bool, notme: Option<i32>) -> Option<i32> {
        if let Sel::Nothing = s {
            return None;
        }
        let ids: Vec<i32> = self
            .candidates(shape.bbox())
            .into_iter()
            .map(|k| &self.instances[k as usize])
            .filter(|o| o.alive() && self.sel_match(s, o))
            .map(|o| o.id)
            .collect();
        self.shape_collision(shape, &ids, prec, notme)
    }

    /// `place_free`.
    pub fn place_free(&self, id: i32, x: f64, y: f64) -> bool {
        self.place_check(id, x, y, |o| o.solid).is_none()
    }

    /// `place_empty`.
    pub fn place_empty(&self, id: i32, x: f64, y: f64) -> bool {
        self.place_check(id, x, y, |_| true).is_none()
    }

    /// `instance_place` / `place_meeting` against a target set.
    pub fn place_meeting_target(&self, id: i32, x: f64, y: f64, targets: &[i32]) -> Option<i32> {
        self.place_check(id, x, y, |o| targets.contains(&o.id))
    }

    /// Whether point (x, y) is in the instance's mask (or bbox if `!precise`).
    pub fn point_in_instance(&self, id: i32, x: f64, y: f64, precise: bool) -> bool {
        let Some(i) = self.inst(id) else { return false };
        let Some(g) = self.geo_at(i, i.x, i.y) else { return false };
        let (fx, fy) = (x.floor() as i32, y.floor() as i32);
        if fx < g.bbox[0] || fx > g.bbox[2] || fy < g.bbox[1] || fy > g.bbox[3] {
            return false;
        }
        if !precise {
            return true;
        }
        g.contains(x, y)
    }

    /// collision_* query: first instance among `candidates` colliding with shape.
    pub fn shape_collision(&self, shape: Shape, candidates: &[i32], prec: bool, notme: Option<i32>) -> Option<i32> {
        let sb = shape.bbox();
        for &cid in candidates {
            if Some(cid) == notme {
                continue;
            }
            let Some(i) = self.inst(cid) else { continue };
            if !i.alive() {
                continue;
            }
            let Some(g) = self.geo_at(i, i.x, i.y) else { continue };
            let l = sb[0].max(g.bbox[0]);
            let r = sb[2].min(g.bbox[2]);
            let t = sb[1].max(g.bbox[1]);
            let b = sb[3].min(g.bbox[3]);
            if l > r || t > b {
                continue;
            }
            let use_mask = prec && (g.precise || g.rotated);
            let mut hit = false;
            'px: for py in t..=b {
                for px in l..=r {
                    if shape.contains_pixel(px, py) && (!use_mask || g.contains(px as f64 + 0.5, py as f64 + 0.5)) {
                        hit = true;
                        break 'px;
                    }
                }
            }
            if hit {
                return Some(cid);
            }
        }
        None
    }

    /// Distance from instance bbox to a point (0 if inside).
    pub fn bbox_distance_point(&self, id: i32, x: f64, y: f64) -> f64 {
        let (l, t, r, b) = self.bbox_of(id);
        let dx = if x < l { l - x } else if x > r + 1.0 { x - r - 1.0 } else { 0.0 };
        let dy = if y < t { t - y } else if y > b + 1.0 { y - b - 1.0 } else { 0.0 };
        (dx * dx + dy * dy).sqrt()
    }

    /// Distance between two instance bboxes.
    pub fn bbox_distance(&self, a: i32, b: i32) -> f64 {
        let (l1, t1, r1, b1) = self.bbox_of(a);
        let (l2, t2, r2, b2) = self.bbox_of(b);
        let dx = if r1 < l2 { l2 - r1 } else if r2 < l1 { l1 - r2 } else { 0.0 };
        let dy = if b1 < t2 { t2 - b1 } else if b2 < t1 { t1 - b2 } else { 0.0 };
        (dx * dx + dy * dy).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_rules() {
        assert_eq!(compute_bbox(10.0, 20.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0, 0, 31, 31), [10, 20, 41, 51]);
        assert_eq!(compute_bbox(10.0, 20.0, 2.0, 1.0, 0.0, 16.0, 16.0, 0, 0, 31, 31), [-22, 4, 41, 35]);
        assert_eq!(compute_bbox(0.0, 0.0, -1.0, 1.0, 0.0, 0.0, 0.0, 0, 0, 9, 9), [-10, 0, -1, 9]);
        let b = compute_bbox(0.0, 0.0, 1.0, 1.0, 90.0, 0.0, 0.0, 0, 0, 9, 4);
        assert_eq!(b, [0, -10, 4, -1]);
    }
}
