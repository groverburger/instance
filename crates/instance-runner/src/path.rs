//! Path geometry and path following (runtime.md §10.1).

use crate::events::{ev, ev_other};
use crate::game::Game;
use gml::{Error, Vm};

/// A path resource with its sampled polyline.
#[derive(Clone, Debug)]
pub struct PathRes {
    pub name: String,
    pub kind: i32,
    pub closed: bool,
    pub precision: i32,
    /// Control points: x, y, speed (percent).
    pub points: Vec<(f64, f64, f64)>,
    /// Sampled polyline with cumulative length.
    pub poly: Vec<(f64, f64, f64, f64)>,
    pub length: f64,
}

impl PathRes {
    pub fn from_format(p: &instance_format::Path) -> Self {
        let mut r = PathRes {
            name: p.name.clone(),
            kind: p.connection_kind,
            closed: p.closed,
            precision: p.precision,
            points: p.points.iter().map(|q| (q.x, q.y, q.speed)).collect(),
            poly: Vec::new(),
            length: 0.0,
        };
        r.rebuild();
        r
    }

    pub fn new_empty(name: String) -> Self {
        PathRes {
            name,
            kind: 0,
            closed: true,
            precision: 4,
            points: Vec::new(),
            poly: Vec::new(),
            length: 0.0,
        }
    }

    /// Recomputes the polyline. Smooth paths use quadratic B-spline segments
    /// between the midpoints of consecutive control points (the first and last
    /// points of an open path are kept as end points), subdivided 2^precision times.
    pub fn rebuild(&mut self) {
        let pts = &self.points;
        let mut out: Vec<(f64, f64, f64)> = Vec::new();
        if pts.is_empty() {
            self.poly.clear();
            self.length = 0.0;
            return;
        }
        if self.kind == 1 && pts.len() > 2 {
            let n = pts.len();
            let steps = 1usize << self.precision.clamp(1, 8);
            let mid = |a: (f64, f64, f64), b: (f64, f64, f64)| ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, (a.2 + b.2) / 2.0);
            let quad = |out: &mut Vec<(f64, f64, f64)>, p0: (f64, f64, f64), c: (f64, f64, f64), p1: (f64, f64, f64)| {
                for s in 0..steps {
                    let t = s as f64 / steps as f64;
                    let u = 1.0 - t;
                    out.push((
                        u * u * p0.0 + 2.0 * u * t * c.0 + t * t * p1.0,
                        u * u * p0.1 + 2.0 * u * t * c.1 + t * t * p1.1,
                        u * u * p0.2 + 2.0 * u * t * c.2 + t * t * p1.2,
                    ));
                }
            };
            if self.closed {
                for i in 0..n {
                    let a = pts[i];
                    let b = pts[(i + 1) % n];
                    let c = pts[(i + 2) % n];
                    quad(&mut out, mid(a, b), b, mid(b, c));
                }
                let f = out[0];
                out.push(f);
            } else {
                out.push(pts[0]);
                let first_mid = mid(pts[0], pts[1]);
                out.push(first_mid);
                for i in 0..n - 2 {
                    let a = pts[i];
                    let b = pts[i + 1];
                    let c = pts[i + 2];
                    quad(&mut out, mid(a, b), b, mid(b, c));
                }
                out.push(pts[n - 1]);
            }
        } else {
            out.extend(pts.iter().copied());
            if self.closed && pts.len() > 1 {
                out.push(pts[0]);
            }
        }
        let mut poly = Vec::with_capacity(out.len());
        let mut len = 0.0;
        for (i, p) in out.iter().enumerate() {
            if i > 0 {
                let q = out[i - 1];
                len += ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
            }
            poly.push((p.0, p.1, p.2, len));
        }
        self.poly = poly;
        self.length = len;
    }

    /// Point at relative position `t` (0..1): x, y, speed percent.
    pub fn at(&self, t: f64) -> (f64, f64, f64) {
        if self.poly.is_empty() {
            return (0.0, 0.0, 100.0);
        }
        if self.poly.len() == 1 || self.length <= 0.0 {
            let p = self.poly[0];
            return (p.0, p.1, p.2);
        }
        let d = t.clamp(0.0, 1.0) * self.length;
        let idx = match self.poly.binary_search_by(|p| p.3.partial_cmp(&d).unwrap()) {
            Ok(i) => i,
            Err(i) => i,
        }
        .clamp(1, self.poly.len() - 1);
        let (a, b) = (self.poly[idx - 1], self.poly[idx]);
        let seg = b.3 - a.3;
        let f = if seg > 0.0 { (d - a.3) / seg } else { 0.0 };
        (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f, a.2 + (b.2 - a.2) * f)
    }
}

impl Game {
    fn path(&self, i: i32) -> Option<&PathRes> {
        if i < 0 {
            return None;
        }
        self.paths.get(i as usize).and_then(|p| p.as_ref())
    }

    /// `path_start` (§10.1).
    pub fn path_start(&mut self, id: i32, path: i32, speed: f64, endaction: i32, absolute: bool) {
        let Some(p) = self.path(path).cloned() else {
            if let Some(i) = self.inst_mut(id) {
                i.path_index = -1;
            }
            return;
        };
        let Some(i) = self.inst_mut(id) else { return };
        i.path_index = path;
        i.path_speed = speed;
        i.path_endaction = endaction;
        i.path_position = if speed < 0.0 { 1.0 } else { 0.0 };
        i.path_positionprevious = i.path_position;
        i.path_scale = 1.0;
        i.path_orientation = 0.0;
        i.speed = 0.0;
        i.hspeed = 0.0;
        i.vspeed = 0.0;
        // VERIFIED black-box (bb_motion mo.pa4.*): a relative path is anchored at
        // its first point (position 0) even when a negative speed starts it at
        // position 1; the instance does not move at path_start.
        let (ax, ay, _) = p.at(0.0);
        i.path_anchor_x = ax;
        i.path_anchor_y = ay;
        if absolute {
            let (px, py, _) = p.at(i.path_position);
            i.path_xstart = ax;
            i.path_ystart = ay;
            i.x = px;
            i.y = py;
        } else {
            i.path_xstart = i.x;
            i.path_ystart = i.y;
        }
    }

    /// Room position of instance `id`'s path at position `pos`.
    pub fn path_point_for(&self, id: i32, pos: f64) -> Option<(f64, f64)> {
        let i = self.inst(id)?;
        let p = self.path(i.path_index)?;
        let (px, py, _) = p.at(pos);
        let (dx, dy) = ((px - i.path_anchor_x) * i.path_scale, (py - i.path_anchor_y) * i.path_scale);
        let a = i.path_orientation.to_radians();
        let (c, s) = (a.cos(), a.sin());
        Some((i.path_xstart + dx * c + dy * s, i.path_ystart - dx * s + dy * c))
    }

    /// Per-step motion (§7.4) including path following and End of Path events.
    pub fn motion_update(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let mut k = 0;
        while k < self.instances.len() {
            let id = self.instances[k].id;
            k += 1;
            if !self.instances[k - 1].alive() {
                continue;
            }
            let path_index = self.instances[k - 1].path_index;
            if path_index >= 0 {
                if self.advance_path(vm, id)? && self.pending.is_some() {
                    return Ok(());
                }
                continue;
            }
            let i = &self.instances[k - 1];
            if i.hspeed == 0.0 && i.vspeed == 0.0 && i.friction == 0.0 && i.gravity == 0.0 {
                continue;
            }
            let i = self.inst_at_mut(k - 1);
            i.apply_friction_gravity();
            i.x += i.hspeed;
            i.y += i.vspeed;
        }
        Ok(())
    }

    /// Advances one instance along its path. Returns true if an event ran.
    fn advance_path(&mut self, vm: &mut Vm, id: i32) -> Result<bool, Error> {
        let (pi, pos, spd, scale) = {
            let i = self.inst(id).unwrap();
            (i.path_index, i.path_position, i.path_speed, i.path_scale)
        };
        let Some(p) = self.path(pi) else {
            self.inst_mut(id).unwrap().path_index = -1;
            return Ok(false);
        };
        let len = p.length * scale.abs();
        let factor = p.at(pos).2 / 100.0;
        let delta = if len > 0.0 { spd * factor / len } else { 1.0f64.copysign(spd) };
        let mut np = pos + delta;
        let closed = p.closed;
        let mut ended = false;
        if spd >= 0.0 && np >= 1.0 || spd < 0.0 && np <= 0.0 {
            ended = true;
        }
        let endaction = self.inst(id).unwrap().path_endaction;
        if ended {
            match endaction {
                1 => {
                    // restart from the start, carrying overshoot
                    if spd >= 0.0 {
                        np -= 1.0;
                    } else {
                        np += 1.0;
                    }
                    if !closed {
                        // jump: re-anchor stays the same
                    }
                }
                2 => {
                    // continue from current position: re-anchor
                    let (ex, ey) = self.path_point_for(id, if spd >= 0.0 { 1.0 } else { 0.0 }).unwrap_or((0.0, 0.0));
                    let (sx, sy, _) = p.at(if spd >= 0.0 { 0.0 } else { 1.0 });
                    let i = self.inst_mut(id).unwrap();
                    i.path_xstart = ex;
                    i.path_ystart = ey;
                    i.path_anchor_x = sx;
                    i.path_anchor_y = sy;
                    if spd >= 0.0 {
                        np -= 1.0;
                    } else {
                        np += 1.0;
                    }
                }
                3 => {
                    np = if spd >= 0.0 { 1.0 - (np - 1.0) } else { -np };
                    self.inst_mut(id).unwrap().path_speed = -spd;
                }
                0 => {
                    // VERIFIED black-box (bb_motion mo.pa0/pa4 PE): stop puts the
                    // instance at position 1 (also when it ran backwards to 0);
                    // path_speed keeps its value.
                    np = 1.0;
                }
                _ => {
                    np = np.clamp(0.0, 1.0);
                }
            }
        }
        let old = {
            let i = self.inst(id).unwrap();
            (i.x, i.y)
        };
        self.inst_mut(id).unwrap().path_position = np.clamp(0.0, 1.0);
        if let Some((nx, ny)) = self.path_point_for(id, np.clamp(0.0, 1.0)) {
            let i = self.inst_mut(id).unwrap();
            i.x = nx;
            i.y = ny;
            let (dx, dy) = (nx - old.0, ny - old.1);
            if dx != 0.0 || dy != 0.0 {
                i.direction = crate::instance::normalize_angle((-dy).atan2(dx).to_degrees());
            }
        }
        if ended {
            if endaction == 0 {
                self.inst_mut(id).unwrap().path_index = -1;
            }
            self.run_event(vm, id, ev::OTHER, ev_other::END_OF_PATH, id)?;
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_path() {
        let mut p = PathRes::new_empty("p".into());
        p.closed = false;
        p.points = vec![(0.0, 0.0, 100.0), (100.0, 0.0, 100.0), (100.0, 100.0, 50.0)];
        p.rebuild();
        assert_eq!(p.length, 200.0);
        assert_eq!(p.at(0.25), (50.0, 0.0, 100.0));
        assert_eq!(p.at(0.75), (100.0, 50.0, 75.0));
        p.kind = 1;
        p.rebuild();
        let (x, y, _) = p.at(1.0);
        assert_eq!((x, y), (100.0, 100.0));
    }
}
