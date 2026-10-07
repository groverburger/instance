//! Chapter 403 motion functions, paths and motion planning (runtime.md §9).

use super::{bo, r, zero, Reg, A};
use crate::game::Game;
use crate::instance::normalize_angle;
use gml::{Ctx, Error, Value, Vm};

/// Motion-planning state.
#[derive(Default)]
pub struct MpState {
    pub maxrot: f64,
    pub rotstep: f64,
    pub ahead: f64,
    pub onspot: bool,
    pub configured: bool,
    pub grids: Vec<Option<Grid>>,
}

/// An mp grid.
#[derive(Clone)]
pub struct Grid {
    pub left: f64,
    pub top: f64,
    pub hcells: i32,
    pub vcells: i32,
    pub cw: f64,
    pub ch: f64,
    pub cells: Vec<bool>,
}

impl MpState {
    fn settings(&self) -> (f64, f64, f64, bool) {
        if self.configured {
            (self.maxrot, self.rotstep, self.ahead, self.onspot)
        } else {
            (30.0, 10.0, 3.0, true)
        }
    }
}

fn me(g: &mut Game, c: &Ctx) -> Result<(), Error> {
    if g.inst(c.self_id).is_none() {
        return Err(Error::new("This function needs an instance (self does not exist)."));
    }
    Ok(())
}

impl Game {
    /// Whether `id` at (x,y) is blocked (solid instances, all instances, or a target set).
    fn blocked(&self, id: i32, x: f64, y: f64, mode: &Block) -> bool {
        match mode {
            Block::Solid => !self.place_free(id, x, y),
            Block::All => !self.place_empty(id, x, y),
            Block::Set(t) => self.place_meeting_target(id, x, y, t).is_some(),
        }
    }

    pub fn move_contact(&mut self, id: i32, dir: f64, maxdist: f64, all: bool) {
        let mode = if all { Block::All } else { Block::Solid };
        let Some(i) = self.inst(id) else { return };
        let (mut x, mut y) = (i.x, i.y);
        if self.blocked(id, x, y, &mode) {
            return;
        }
        let max = if maxdist <= 0.0 { 1000.0 } else { maxdist };
        let (dx, dy) = (dir.to_radians().cos(), -dir.to_radians().sin());
        let mut d = 0.0;
        while d < max {
            let step = (max - d).min(1.0);
            let (nx, ny) = (x + dx * step, y + dy * step);
            if self.blocked(id, nx, ny, &mode) {
                break;
            }
            x = nx;
            y = ny;
            d += step;
        }
        let i = self.inst_mut(id).unwrap();
        i.x = x;
        i.y = y;
    }

    pub fn move_outside(&mut self, id: i32, dir: f64, maxdist: f64, all: bool) {
        let mode = if all { Block::All } else { Block::Solid };
        let Some(i) = self.inst(id) else { return };
        let (mut x, mut y) = (i.x, i.y);
        let max = if maxdist <= 0.0 { 1000.0 } else { maxdist };
        let (dx, dy) = (dir.to_radians().cos(), -dir.to_radians().sin());
        let mut d = 0.0;
        while d < max && self.blocked(id, x, y, &mode) {
            x += dx;
            y += dy;
            d += 1.0;
        }
        let i = self.inst_mut(id).unwrap();
        i.x = x;
        i.y = y;
    }

    pub fn move_bounce(&mut self, id: i32, advanced: bool, all: bool) {
        let mode = if all { Block::All } else { Block::Solid };
        let Some(i) = self.inst(id) else { return };
        let (x, y, hs, vs) = (i.x, i.y, i.hspeed, i.vspeed);
        if hs == 0.0 && vs == 0.0 {
            return;
        }
        if !self.blocked(id, x + hs, y + vs, &mode) {
            return;
        }
        if advanced {
            // probe the surface normal by testing directions around the motion
            let sp = (hs * hs + vs * vs).sqrt();
            let dir = (-vs).atan2(hs).to_degrees();
            let mut free_l = None;
            let mut free_r = None;
            let mut a = 10.0;
            while a <= 180.0 {
                if free_l.is_none() {
                    let d = (dir + a).to_radians();
                    if !self.blocked(id, x + sp * d.cos(), y - sp * d.sin(), &mode) {
                        free_l = Some(a);
                    }
                }
                if free_r.is_none() {
                    let d = (dir - a).to_radians();
                    if !self.blocked(id, x + sp * d.cos(), y - sp * d.sin(), &mode) {
                        free_r = Some(a);
                    }
                }
                if free_l.is_some() && free_r.is_some() {
                    break;
                }
                a += 10.0;
            }
            let (l, r) = (free_l.unwrap_or(180.0), free_r.unwrap_or(180.0));
            // wall tangent lies halfway between the first free directions
            let tangent = dir + (l - r) / 2.0;
            let normal = tangent + 90.0;
            let newdir = 2.0 * normal - dir + 180.0;
            let i = self.inst_mut(id).unwrap();
            i.direction = normalize_angle(newdir);
            i.update_cartesian();
            return;
        }
        let hblock = self.blocked(id, x + hs, y, &mode);
        let vblock = self.blocked(id, x, y + vs, &mode);
        let i = self.inst_mut(id).unwrap();
        if hblock {
            i.hspeed = -hs;
        }
        if vblock {
            i.vspeed = -vs;
        }
        if !hblock && !vblock {
            i.hspeed = -hs;
            i.vspeed = -vs;
        }
        i.update_polar();
    }

    pub fn move_random(&mut self, vm: &mut Vm, id: i32, hs: f64, vs: f64) {
        let (rw, rh) = (self.rs.width as f64, self.rs.height as f64);
        for _ in 0..100 {
            let mut x = vm.random() * rw;
            let mut y = vm.random() * rh;
            if hs > 0.0 {
                x = (x / hs).floor() * hs;
            }
            if vs > 0.0 {
                y = (y / vs).floor() * vs;
            }
            if self.place_free(id, x, y) {
                let i = self.inst_mut(id).unwrap();
                i.x = x;
                i.y = y;
                return;
            }
        }
    }

    fn linear_step(&mut self, id: i32, gx: f64, gy: f64, sp: f64, mode: Block) -> bool {
        let Some(i) = self.inst(id) else { return false };
        let (x, y) = (i.x, i.y);
        let d = (gx - x).hypot(gy - y);
        if d <= sp {
            if self.blocked(id, gx, gy, &mode) {
                return false;
            }
            let i = self.inst_mut(id).unwrap();
            i.x = gx;
            i.y = gy;
            return true;
        }
        let (nx, ny) = (x + (gx - x) / d * sp, y + (gy - y) / d * sp);
        let dir = normalize_angle((y - gy).atan2(gx - x).to_degrees());
        if self.blocked(id, nx, ny, &mode) {
            return false;
        }
        let i = self.inst_mut(id).unwrap();
        i.x = nx;
        i.y = ny;
        i.direction = dir;
        false
    }

    fn potential_step(&mut self, id: i32, gx: f64, gy: f64, sp: f64, mode: Block) -> bool {
        let (maxrot, rotstep, ahead, onspot) = self.mp.settings();
        let Some(i) = self.inst(id) else { return false };
        let (x, y, cur) = (i.x, i.y, i.direction);
        let d = (gx - x).hypot(gy - y);
        if d <= sp {
            if !self.blocked(id, gx, gy, &mode) {
                let i = self.inst_mut(id).unwrap();
                i.x = gx;
                i.y = gy;
                return true;
            }
        }
        let goal = normalize_angle((y - gy).atan2(gx - x).to_degrees());
        let free = |g: &Game, dir: f64| -> bool {
            let r = dir.to_radians();
            for k in 1..=(ahead.max(1.0) as i32) {
                let s = sp * k as f64;
                if g.blocked(id, x + r.cos() * s, y - r.sin() * s, &mode) {
                    return false;
                }
            }
            true
        };
        let mut off = 0.0;
        let mut chosen = None;
        while off <= 180.0 {
            for cand in [goal + off, goal - off] {
                let diff = ((cand - cur + 540.0) % 360.0) - 180.0;
                let cand = if diff.abs() > maxrot { cur + maxrot * diff.signum() } else { cand };
                if free(self, cand) {
                    chosen = Some(cand);
                    break;
                }
            }
            if chosen.is_some() {
                break;
            }
            off += rotstep.max(1.0);
        }
        let i = self.inst_mut(id).unwrap();
        match chosen {
            Some(dir) => {
                let r = dir.to_radians();
                i.x += r.cos() * sp;
                i.y -= r.sin() * sp;
                i.direction = normalize_angle(dir);
            }
            None => {
                if onspot {
                    i.direction = normalize_angle(cur + maxrot);
                }
            }
        }
        false
    }
}

/// What counts as an obstacle.
pub enum Block {
    Solid,
    All,
    Set(Vec<i32>),
}

fn grid_cell(g: &Grid, x: f64, y: f64) -> Option<(i32, i32)> {
    let cx = ((x - g.left) / g.cw).floor() as i32;
    let cy = ((y - g.top) / g.ch).floor() as i32;
    if cx < 0 || cy < 0 || cx >= g.hcells || cy >= g.vcells {
        None
    } else {
        Some((cx, cy))
    }
}

fn grid_mut<'a>(g: &'a mut Game, a: &A) -> Result<&'a mut Grid, Error> {
    let i = a.i(0)?;
    g.mp
        .grids
        .get_mut(i.max(0) as usize)
        .and_then(|x| x.as_mut())
        .ok_or_else(|| Error::new(format!("Unknown mp grid {}", i)))
}

fn set_path_points(g: &mut Game, path: i32, pts: Vec<(f64, f64)>) -> bool {
    match g.paths.get_mut(path.max(0) as usize).and_then(|p| p.as_mut()) {
        Some(p) if path >= 0 => {
            p.points = pts.into_iter().map(|(x, y)| (x, y, 100.0)).collect();
            p.kind = 0;
            p.closed = false;
            p.rebuild();
            true
        }
        _ => false,
    }
}

fn plan_path(g: &mut Game, c: &Ctx, a: &A, potential: bool, mode: Block, factor: f64) -> Result<Value, Error> {
    me(g, c)?;
    let (path, gx, gy, step) = (a.i(0)?, a.r(1)?, a.r(2)?, a.r(3)?);
    let id = c.self_id;
    let saved = g.inst(id).cloned().unwrap();
    let mut pts = vec![(saved.x, saved.y)];
    let straight = (gx - saved.x).hypot(gy - saved.y);
    let limit = if potential { (straight * factor.max(1.0) / step.max(0.1)) as usize + 2 } else { (straight / step.max(0.1)) as usize + 2 };
    let mut ok = false;
    for _ in 0..limit.min(100000) {
        let mode2 = match &mode {
            Block::Solid => Block::Solid,
            Block::All => Block::All,
            Block::Set(v) => Block::Set(v.clone()),
        };
        let reached = if potential { g.potential_step(id, gx, gy, step, mode2) } else { g.linear_step(id, gx, gy, step, mode2) };
        let i = g.inst(id).unwrap();
        if pts.last() != Some(&(i.x, i.y)) {
            pts.push((i.x, i.y));
        } else if !potential {
            break;
        }
        if reached {
            ok = true;
            break;
        }
    }
    // restore instance
    let i = g.inst_mut(id).unwrap();
    i.x = saved.x;
    i.y = saved.y;
    i.direction = saved.direction;
    i.update_cartesian();
    if ok {
        set_path_points(g, path, pts);
    }
    bo(ok)
}

pub fn register(l: &mut Reg) {
    l.push(("motion_set", |g, _, c, a| {
        g.inst_req(c.self_id)?.motion_set(a.r(0)?, a.r(1)?);
        zero()
    }));
    l.push(("motion_add", |g, _, c, a| {
        g.inst_req(c.self_id)?.motion_add(a.r(0)?, a.r(1)?);
        zero()
    }));
    l.push(("place_snapped", |g, _, c, a| {
        let (h, v) = (a.r(0)?, a.r(1)?);
        let i = g.inst_req(c.self_id)?;
        let ok = (h <= 0.0 || (i.x / h).fract().abs() < 1e-9 || (i.x - (i.x / h).round() * h).abs() < 1e-9)
            && (v <= 0.0 || (i.y - (i.y / v).round() * v).abs() < 1e-9);
        bo(ok)
    }));
    l.push(("move_random", |g, vm, c, a| {
        me(g, c)?;
        g.move_random(vm, c.self_id, a.r(0)?, a.r(1)?);
        zero()
    }));
    l.push(("move_snap", |g, _, c, a| {
        let (h, v) = (a.r(0)?, a.r(1)?);
        let i = g.inst_req(c.self_id)?;
        if h > 0.0 {
            i.x = (i.x / h).round() * h;
        }
        if v > 0.0 {
            i.y = (i.y / v).round() * v;
        }
        zero()
    }));
    l.push(("move_towards_point", |g, _, c, a| {
        let (x, y, sp) = (a.r(0)?, a.r(1)?, a.r(2)?);
        let i = g.inst_req(c.self_id)?;
        let dir = (i.y - y).atan2(x - i.x).to_degrees();
        i.motion_set(dir, sp);
        zero()
    }));
    l.push(("move_contact_solid", |g, _, c, a| {
        me(g, c)?;
        g.move_contact(c.self_id, a.r(0)?, a.r(1)?, false);
        zero()
    }));
    l.push(("move_contact_all", |g, _, c, a| {
        me(g, c)?;
        g.move_contact(c.self_id, a.r(0)?, a.r(1)?, true);
        zero()
    }));
    l.push(("move_outside_solid", |g, _, c, a| {
        me(g, c)?;
        g.move_outside(c.self_id, a.r(0)?, a.r(1)?, false);
        zero()
    }));
    l.push(("move_outside_all", |g, _, c, a| {
        me(g, c)?;
        g.move_outside(c.self_id, a.r(0)?, a.r(1)?, true);
        zero()
    }));
    l.push(("move_bounce_solid", |g, _, c, a| {
        me(g, c)?;
        g.move_bounce(c.self_id, a.b(0)?, false);
        zero()
    }));
    l.push(("move_bounce_all", |g, _, c, a| {
        me(g, c)?;
        g.move_bounce(c.self_id, a.b(0)?, true);
        zero()
    }));
    l.push(("move_wrap", |g, _, c, a| {
        let (hor, vert, m) = (a.b(0)?, a.b(1)?, a.r(2)?);
        let (rw, rh) = (g.rs.width as f64, g.rs.height as f64);
        let i = g.inst_req(c.self_id)?;
        if hor {
            if i.x < -m {
                i.x += rw + 2.0 * m;
            } else if i.x > rw + m {
                i.x -= rw + 2.0 * m;
            }
        }
        if vert {
            if i.y < -m {
                i.y += rh + 2.0 * m;
            } else if i.y > rh + m {
                i.y -= rh + 2.0 * m;
            }
        }
        zero()
    }));
    l.push(("path_start", |g, _, c, a| {
        me(g, c)?;
        g.path_start(c.self_id, a.i(0)?, a.r(1)?, a.i(2)?, a.b(3)?);
        zero()
    }));
    l.push(("path_end", |g, _, c, _| {
        g.inst_req(c.self_id)?.path_index = -1;
        zero()
    }));
    l.push(("mp_linear_step", |g, _, c, a| {
        me(g, c)?;
        let m = if a.b(3)? { Block::All } else { Block::Solid };
        bo(g.linear_step(c.self_id, a.r(0)?, a.r(1)?, a.r(2)?, m))
    }));
    l.push(("mp_linear_step_object", |g, _, c, a| {
        me(g, c)?;
        let t = g.select_ctx(c, a.i(3)?);
        bo(g.linear_step(c.self_id, a.r(0)?, a.r(1)?, a.r(2)?, Block::Set(t)))
    }));
    l.push(("mp_potential_step", |g, _, c, a| {
        me(g, c)?;
        let m = if a.b(3)? { Block::All } else { Block::Solid };
        bo(g.potential_step(c.self_id, a.r(0)?, a.r(1)?, a.r(2)?, m))
    }));
    l.push(("mp_potential_step_object", |g, _, c, a| {
        me(g, c)?;
        let t = g.select_ctx(c, a.i(3)?);
        bo(g.potential_step(c.self_id, a.r(0)?, a.r(1)?, a.r(2)?, Block::Set(t)))
    }));
    l.push(("mp_potential_settings", |g, _, _, a| {
        g.mp.maxrot = a.r(0)?;
        g.mp.rotstep = a.r(1)?;
        g.mp.ahead = a.r(2)?;
        g.mp.onspot = a.b(3)?;
        g.mp.configured = true;
        zero()
    }));
    l.push(("mp_linear_path", |g, _, c, a| {
        let m = if a.b(4)? { Block::All } else { Block::Solid };
        plan_path(g, c, a, false, m, 1.0)
    }));
    l.push(("mp_potential_path", |g, _, c, a| {
        let m = if a.b(5)? { Block::All } else { Block::Solid };
        let f = a.r(4)?;
        plan_path(g, c, a, true, m, f)
    }));
    l.push(("mp_linear_path_object", |g, _, c, a| {
        let t = g.select_ctx(c, a.i(4)?);
        plan_path(g, c, a, false, Block::Set(t), 1.0)
    }));
    l.push(("mp_potential_path_object", |g, _, c, a| {
        let t = g.select_ctx(c, a.i(5)?);
        let f = a.r(4)?;
        plan_path(g, c, a, true, Block::Set(t), f)
    }));
    l.push(("mp_grid_create", |g, _, _, a| {
        let (hc, vc) = (a.i(2)?.max(1), a.i(3)?.max(1));
        let grid = Grid {
            left: a.r(0)?,
            top: a.r(1)?,
            hcells: hc,
            vcells: vc,
            cw: a.r(4)?.max(1.0),
            ch: a.r(5)?.max(1.0),
            cells: vec![false; (hc * vc) as usize],
        };
        let p = g.mp.grids.iter().position(|x| x.is_none());
        let idx = match p {
            Some(p) => {
                g.mp.grids[p] = Some(grid);
                p
            }
            None => {
                g.mp.grids.push(Some(grid));
                g.mp.grids.len() - 1
            }
        };
        r(idx as f64)
    }));
    l.push(("mp_grid_destroy", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(x) = g.mp.grids.get_mut(i.max(0) as usize) {
            *x = None;
        }
        zero()
    }));
    l.push(("mp_grid_clear_all", |g, _, _, a| {
        let gr = grid_mut(g, a)?;
        gr.cells.iter_mut().for_each(|c| *c = false);
        zero()
    }));
    fn set_cell(g: &mut Game, a: &A, v: bool) -> Result<Value, Error> {
        let (h, vv) = (a.i(1)?, a.i(2)?);
        let gr = grid_mut(g, a)?;
        if h >= 0 && vv >= 0 && h < gr.hcells && vv < gr.vcells {
            let i = (vv * gr.hcells + h) as usize;
            gr.cells[i] = v;
        }
        zero()
    }
    l.push(("mp_grid_clear_cell", |g, _, _, a| set_cell(g, a, false)));
    l.push(("mp_grid_add_cell", |g, _, _, a| set_cell(g, a, true)));
    fn set_rect(g: &mut Game, a: &A, v: bool) -> Result<Value, Error> {
        let (x1, y1, x2, y2) = (a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?);
        let gr = grid_mut(g, a)?;
        for cy in 0..gr.vcells {
            for cx in 0..gr.hcells {
                let (l, t) = (gr.left + cx as f64 * gr.cw, gr.top + cy as f64 * gr.ch);
                if l <= x1.max(x2) && l + gr.cw > x1.min(x2) && t <= y1.max(y2) && t + gr.ch > y1.min(y2) {
                    let i = (cy * gr.hcells + cx) as usize;
                    gr.cells[i] = v;
                }
            }
        }
        zero()
    }
    l.push(("mp_grid_clear_rectangle", |g, _, _, a| set_rect(g, a, false)));
    l.push(("mp_grid_add_rectangle", |g, _, _, a| set_rect(g, a, true)));
    l.push(("mp_grid_add_instances", |g, _, c, a| {
        let ids = g.select_ctx(c, a.i(1)?);
        let prec = a.b(2)?;
        let gi = a.i(0)?;
        let Some(gr) = g.mp.grids.get(gi.max(0) as usize).and_then(|x| x.clone()) else {
            return Err(Error::new("Unknown mp grid"));
        };
        let mut cells = gr.cells.clone();
        for id in ids {
            let (bl, bt, br, bb) = g.bbox_of(id);
            for cy in 0..gr.vcells {
                for cx in 0..gr.hcells {
                    let (l, t) = (gr.left + cx as f64 * gr.cw, gr.top + cy as f64 * gr.ch);
                    if l > br || l + gr.cw <= bl || t > bb || t + gr.ch <= bt {
                        continue;
                    }
                    let hit = if prec {
                        let mut h = false;
                        'p: for py in (t as i32)..((t + gr.ch) as i32) {
                            for px in (l as i32)..((l + gr.cw) as i32) {
                                if g.point_in_instance(id, px as f64 + 0.5, py as f64 + 0.5, true) {
                                    h = true;
                                    break 'p;
                                }
                            }
                        }
                        h
                    } else {
                        true
                    };
                    if hit {
                        cells[(cy * gr.hcells + cx) as usize] = true;
                    }
                }
            }
        }
        g.mp.grids[gi as usize].as_mut().unwrap().cells = cells;
        zero()
    }));
    l.push(("mp_grid_path", |g, _, _, a| {
        let gi = a.i(0)?;
        let path = a.i(1)?;
        let (xs, ys, xg, yg, diag) = (a.r(2)?, a.r(3)?, a.r(4)?, a.r(5)?, a.b(6)?);
        let Some(gr) = g.mp.grids.get(gi.max(0) as usize).and_then(|x| x.clone()) else {
            return Err(Error::new("Unknown mp grid"));
        };
        let (Some(s), Some(e)) = (grid_cell(&gr, xs, ys), grid_cell(&gr, xg, yg)) else { return bo(false) };
        let idx = |c: (i32, i32)| (c.1 * gr.hcells + c.0) as usize;
        if gr.cells[idx(s)] || gr.cells[idx(e)] {
            return bo(false);
        }
        // Dijkstra/A* on the grid
        let n = gr.cells.len();
        let mut dist = vec![f64::INFINITY; n];
        let mut prev = vec![usize::MAX; n];
        let mut open = std::collections::BinaryHeap::new();
        #[derive(PartialEq)]
        struct Node(f64, usize);
        impl Eq for Node {}
        impl PartialOrd for Node {
            fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(o))
            }
        }
        impl Ord for Node {
            fn cmp(&self, o: &Self) -> std::cmp::Ordering {
                o.0.partial_cmp(&self.0).unwrap_or(std::cmp::Ordering::Equal)
            }
        }
        dist[idx(s)] = 0.0;
        open.push(Node(0.0, idx(s)));
        let target = idx(e);
        while let Some(Node(_, u)) = open.pop() {
            if u == target {
                break;
            }
            let (ux, uy) = ((u as i32) % gr.hcells, (u as i32) / gr.hcells);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let diagonal = dx != 0 && dy != 0;
                if diagonal && !diag {
                    continue;
                }
                let (vx, vy) = (ux + dx, uy + dy);
                if vx < 0 || vy < 0 || vx >= gr.hcells || vy >= gr.vcells {
                    continue;
                }
                let v = idx((vx, vy));
                if gr.cells[v] {
                    continue;
                }
                if diagonal && (gr.cells[idx((ux + dx, uy))] || gr.cells[idx((ux, uy + dy))]) {
                    continue;
                }
                let cost = if diagonal { std::f64::consts::SQRT_2 } else { 1.0 };
                if dist[u] + cost < dist[v] {
                    dist[v] = dist[u] + cost;
                    prev[v] = u;
                    let h = (((vx - e.0) as f64).powi(2) + ((vy - e.1) as f64).powi(2)).sqrt();
                    open.push(Node(dist[v] + h, v));
                }
            }
        }
        if !dist[target].is_finite() {
            return bo(false);
        }
        let mut cells = vec![target];
        while *cells.last().unwrap() != idx(s) {
            let p = prev[*cells.last().unwrap()];
            if p == usize::MAX {
                break;
            }
            cells.push(p);
        }
        cells.reverse();
        let mut pts = vec![(xs, ys)];
        for &cidx in &cells[1..cells.len().saturating_sub(1)] {
            let (cx, cy) = ((cidx as i32) % gr.hcells, (cidx as i32) / gr.hcells);
            pts.push((gr.left + (cx as f64 + 0.5) * gr.cw, gr.top + (cy as f64 + 0.5) * gr.ch));
        }
        pts.push((xg, yg));
        let _ = n;
        bo(set_path_points(g, path, pts))
    }));
    l.push(("mp_grid_draw", |g, _, _, a| {
        let gi = a.i(0)?;
        let Some(gr) = g.mp.grids.get(gi.max(0) as usize).and_then(|x| x.clone()) else { return zero() };
        for cy in 0..gr.vcells {
            for cx in 0..gr.hcells {
                let blocked = gr.cells[(cy * gr.hcells + cx) as usize];
                let col = if blocked { 0x0000FF } else { 0x00FF00 };
                let c = crate::draw::rgba(col, g.gfx.alpha);
                let (l, t) = (gr.left + cx as f64 * gr.cw, gr.top + cy as f64 * gr.ch);
                g.gfx.painter().rect(l, t, l + gr.cw - 1.0, t + gr.ch - 1.0, [c; 4], false);
            }
        }
        zero()
    }));
}
