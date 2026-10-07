//! Event constants, dispatch and the per-step event phases (runtime.md §4–§6).

use crate::actions::{self, ActionList};
use crate::game::Game;
use gml::{Ctx, Error, Vm};
use std::rc::Rc;

/// Event types.
pub mod ev {
    pub const CREATE: i32 = 0;
    pub const DESTROY: i32 = 1;
    pub const ALARM: i32 = 2;
    pub const STEP: i32 = 3;
    pub const COLLISION: i32 = 4;
    pub const KEYBOARD: i32 = 5;
    pub const MOUSE: i32 = 6;
    pub const OTHER: i32 = 7;
    pub const DRAW: i32 = 8;
    pub const KEYPRESS: i32 = 9;
    pub const KEYRELEASE: i32 = 10;
    pub const TRIGGER: i32 = 11;
}

/// Step sub-events.
pub mod step {
    pub const NORMAL: i32 = 0;
    pub const BEGIN: i32 = 1;
    pub const END: i32 = 2;
}

/// `ev_other` sub-events.
pub mod ev_other {
    pub const OUTSIDE: i32 = 0;
    pub const BOUNDARY: i32 = 1;
    pub const GAME_START: i32 = 2;
    pub const GAME_END: i32 = 3;
    pub const ROOM_START: i32 = 4;
    pub const ROOM_END: i32 = 5;
    pub const NO_MORE_LIVES: i32 = 6;
    pub const ANIMATION_END: i32 = 7;
    pub const END_OF_PATH: i32 = 8;
    pub const NO_MORE_HEALTH: i32 = 9;
    pub const USER0: i32 = 10;
    pub const CLOSE_BUTTON: i32 = 30;
    pub const OUTSIDE_VIEW0: i32 = 40;
    pub const BOUNDARY_VIEW0: i32 = 50;
}

/// `ev_mouse` sub-events.
pub mod ev_mouse {
    pub const LEFT_BUTTON: i32 = 0;
    pub const NO_BUTTON: i32 = 3;
    pub const LEFT_PRESS: i32 = 4;
    pub const LEFT_RELEASE: i32 = 7;
    pub const ENTER: i32 = 10;
    pub const LEAVE: i32 = 11;
    pub const GLOBAL_LEFT_BUTTON: i32 = 50;
    pub const GLOBAL_LEFT_PRESS: i32 = 53;
    pub const GLOBAL_LEFT_RELEASE: i32 = 56;
    pub const WHEEL_UP: i32 = 60;
    pub const WHEEL_DOWN: i32 = 61;
}

/// The currently running event (`event_type` etc.).
#[derive(Clone, Copy, Debug)]
pub struct EventCtx {
    pub etype: i32,
    pub number: i32,
    pub object: i32,
    pub action: i32,
}

impl Default for EventCtx {
    fn default() -> Self {
        EventCtx {
            etype: -1,
            number: 0,
            object: -1,
            action: 0,
        }
    }
}

/// Human-readable event name for error messages.
pub fn event_name(t: i32, n: i32, g: &Game) -> String {
    match t {
        ev::CREATE => "Create".into(),
        ev::DESTROY => "Destroy".into(),
        ev::ALARM => format!("Alarm {}", n),
        ev::STEP => match n {
            1 => "Begin Step".into(),
            2 => "End Step".into(),
            _ => "Step".into(),
        },
        ev::COLLISION => format!(
            "Collision with {}",
            g.object(n).map(|o| o.name.clone()).unwrap_or_else(|| n.to_string())
        ),
        ev::KEYBOARD => format!("Keyboard <{}>", crate::input::key_name(n)),
        ev::MOUSE => format!("Mouse {}", n),
        ev::OTHER => match n {
            0 => "Outside Room".into(),
            1 => "Intersect Boundary".into(),
            2 => "Game Start".into(),
            3 => "Game End".into(),
            4 => "Room Start".into(),
            5 => "Room End".into(),
            6 => "No More Lives".into(),
            7 => "Animation End".into(),
            8 => "End of Path".into(),
            9 => "No More Health".into(),
            10..=25 => format!("User Defined {}", n - 10),
            30 => "Close Button".into(),
            40..=47 => format!("Outside View {}", n - 40),
            50..=57 => format!("Boundary View {}", n - 50),
            _ => format!("Other {}", n),
        },
        ev::DRAW => "Draw".into(),
        ev::KEYPRESS => format!("Key Press <{}>", crate::input::key_name(n)),
        ev::KEYRELEASE => format!("Key Release <{}>", crate::input::key_name(n)),
        ev::TRIGGER => format!("Trigger {}", n),
        _ => format!("Event {}/{}", t, n),
    }
}

impl Game {
    /// The effective handler of object `obj` for an event.
    pub fn handler(&self, obj: i32, t: i32, n: i32) -> Option<(i32, Rc<ActionList>)> {
        if obj < 0 {
            return None;
        }
        self.effective.get(obj as usize)?.get(&(t, n)).cloned()
    }

    #[inline]
    pub fn has_handler(&self, obj: i32, t: i32, n: i32) -> bool {
        obj >= 0
            && self
                .effective
                .get(obj as usize)
                .map(|m| m.contains_key(&(t, n)))
                .unwrap_or(false)
    }

    pub fn has_handler_inst(&self, id: i32, t: i32, n: i32) -> bool {
        match self.inst(id) {
            Some(i) => self.has_handler(i.object_index, t, n),
            None => false,
        }
    }

    /// Runs the effective handler of `id`'s object for (t, n).
    pub fn run_event(&mut self, vm: &mut Vm, id: i32, t: i32, n: i32, other: i32) -> Result<(), Error> {
        let obj = match self.inst(id) {
            Some(i) => i.object_index,
            None => return Ok(()),
        };
        if let Some((owner, list)) = self.handler(obj, t, n) {
            self.run_handler(vm, id, owner, &list, t, n, other)?;
        }
        Ok(())
    }

    /// Runs one handler with event context saved/restored (§6.1).
    pub fn run_handler(
        &mut self,
        vm: &mut Vm,
        id: i32,
        owner: i32,
        list: &Rc<ActionList>,
        t: i32,
        n: i32,
        other: i32,
    ) -> Result<(), Error> {
        let saved = self.ev;
        let saved_tl = self.in_timeline;
        self.in_timeline = false;
        self.ev = EventCtx {
            etype: t,
            number: n,
            object: owner,
            action: 0,
        };
        let t0 = self.prof.as_ref().map(|_| std::time::Instant::now());
        let r = actions::execute(self, vm, list, id, other);
        if let (Some(p), Some(t0)) = (self.prof.as_mut(), t0) {
            crate::profile::Profiler::add(&mut p.events, (owner, t, n), t0);
        }
        self.ev = saved;
        self.in_timeline = saved_tl;
        r
    }

    /// `event_inherited()`: the parent of the running handler's object.
    pub fn event_inherited(&mut self, vm: &mut Vm, ctx: &Ctx) -> Result<(), Error> {
        let cur = self.ev;
        let parent = match self.object(cur.object) {
            Some(o) => o.parent,
            None => return Ok(()),
        };
        if cur.etype < 0 {
            return Ok(());
        }
        if let Some((owner, list)) = self.handler(parent, cur.etype, cur.number) {
            self.run_handler(vm, ctx.self_id, owner, &list, cur.etype, cur.number, ctx.other_id)?;
        }
        Ok(())
    }

    /// Delivers an event to all instances having a handler (§4.3).
    pub fn broadcast(&mut self, vm: &mut Vm, t: i32, n: i32) -> Result<(), Error> {
        // objects having a handler for this event (none: nothing to do)
        let Some(mask) = self.event_objs.get(&(t, n)).cloned() else { return Ok(()) };
        let mut k = 0;
        while k < self.instances.len() {
            let (id, obj, alive) = {
                let i = &self.instances[k];
                (i.id, i.object_index, i.alive())
            };
            k += 1;
            if !alive || obj < 0 || !mask.get(obj as usize).copied().unwrap_or(false) {
                continue;
            }
            if let Some((owner, list)) = self.handler(obj, t, n) {
                self.run_handler(vm, id, owner, &list, t, n, id)?;
                if self.pending.is_some() {
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// Alarm phase (§5.3).
    pub fn run_alarms(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let mut k = 0;
        while k < self.instances.len() {
            let (id, obj) = (self.instances[k].id, self.instances[k].object_index);
            k += 1;
            if !self.instances[k - 1].alarm.iter().any(|&a| a > 0.0) {
                continue;
            }
            for a in 0..crate::instance::ALARMS {
                let Some(i) = self.inst(id) else { break };
                if !i.alive() {
                    break;
                }
                if i.alarm[a] <= 0.0 {
                    continue;
                }
                let has = self.has_handler(obj, ev::ALARM, a as i32);
                if !has {
                    continue;
                }
                // (alarms do not affect collision geometry: no need to mark the instance)
                let kk = self.inst_index[&id];
                let i = &mut self.instances[kk];
                i.alarm[a] -= 1.0;
                if i.alarm[a] <= 0.0 {
                    i.alarm[a] = -1.0;
                    // object may have changed during an earlier alarm
                    let obj_now = i.object_index;
                    if let Some((owner, list)) = self.handler(obj_now, ev::ALARM, a as i32) {
                        self.run_handler(vm, id, owner, &list, ev::ALARM, a as i32, id)?;
                        if self.pending.is_some() {
                            return Ok(());
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn key_phase(&mut self, vm: &mut Vm, t: i32, set: &[bool; 256]) -> Result<(), Error> {
        let mut any = false;
        for k in 2..256 {
            if set[k] {
                any = true;
                self.broadcast(vm, t, k as i32)?;
                if self.pending.is_some() {
                    return Ok(());
                }
            }
        }
        if any {
            self.broadcast(vm, t, 1)?;
        } else {
            self.broadcast(vm, t, 0)?;
        }
        Ok(())
    }

    /// Keyboard, key press and key release phases (§11.3).
    pub fn keyboard_events(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let held = self.input.held;
        let pressed = self.input.pressed;
        let released = self.input.released;
        self.key_phase(vm, ev::KEYBOARD, &held)?;
        if self.pending.is_some() {
            return Ok(());
        }
        self.key_phase(vm, ev::KEYPRESS, &pressed)?;
        if self.pending.is_some() {
            return Ok(());
        }
        self.key_phase(vm, ev::KEYRELEASE, &released)
    }

    /// Mouse phase (§11.5).
    pub fn mouse_events(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let (mx, my) = self.mouse_room_pos();
        let held = self.input.mb_held;
        let pressed = self.input.mb_pressed;
        let released = self.input.mb_released;
        let (wu, wd) = (self.input.wheel_up, self.input.wheel_down);
        let any_held = held[1] || held[2] || held[3];
        // joystick events (specs/input-joystick.md §4): level-triggered, first
        let joy = self.input.joy.events();
        // button index mapping: GM mb_left=1, mb_right=2, mb_middle=3 → events 0,1,2
        let mut k = 0;
        while k < self.instances.len() {
            let (id, obj, alive) = {
                let i = &self.instances[k];
                (i.id, i.object_index, i.alive())
            };
            k += 1;
            if !alive || obj < 0 {
                continue;
            }
            let has_any = self.has_mouse.get(obj as usize).copied().unwrap_or(false);
            if !has_any {
                continue;
            }
            let over = self.point_in_instance(id, mx, my, true);
            let was = self.inst(id).map(|i| i.mouse_over).unwrap_or(false);
            if let Some(i) = self.inst_mut(id) {
                i.mouse_over = over;
            }
            let mut evs: Vec<i32> = joy.clone();
            if over {
                for b in 0..3 {
                    if held[b + 1] {
                        evs.push(b as i32);
                    }
                }
                if !any_held {
                    evs.push(3);
                }
                for b in 0..3 {
                    if pressed[b + 1] {
                        evs.push(4 + b as i32);
                    }
                }
                for b in 0..3 {
                    if released[b + 1] {
                        evs.push(7 + b as i32);
                    }
                }
            }
            if over && !was {
                evs.push(10);
            }
            if !over && was {
                evs.push(11);
            }
            for b in 0..3 {
                if held[b + 1] {
                    evs.push(50 + b as i32);
                }
                if pressed[b + 1] {
                    evs.push(53 + b as i32);
                }
                if released[b + 1] {
                    evs.push(56 + b as i32);
                }
            }
            if wu {
                evs.push(60);
            }
            if wd {
                evs.push(61);
            }
            for n in evs {
                let obj_now = match self.inst(id) {
                    Some(i) if i.alive() => i.object_index,
                    _ => break,
                };
                if let Some((owner, list)) = self.handler(obj_now, ev::MOUSE, n) {
                    self.run_handler(vm, id, owner, &list, ev::MOUSE, n, id)?;
                    if self.pending.is_some() {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    /// Triggers of the given moment (0 step, 1 begin step, 2 end step).
    pub fn run_triggers(&mut self, vm: &mut Vm, moment: i32) -> Result<(), Error> {
        if self.triggers.is_empty() {
            return Ok(());
        }
        for t in 0..self.triggers.len() {
            let (cond, m) = match &self.triggers[t] {
                Some(tr) => (tr.condition.clone(), tr.moment),
                None => continue,
            };
            if m != moment {
                continue;
            }
            let Some(cond) = cond else { continue };
            let mut k = 0;
            while k < self.instances.len() {
                let (id, obj, alive) = {
                    let i = &self.instances[k];
                    (i.id, i.object_index, i.alive())
                };
                k += 1;
                if !alive {
                    continue;
                }
                let Some((owner, list)) = self.handler(obj, ev::TRIGGER, t as i32) else { continue };
                let saved = self.ev;
                self.ev = EventCtx {
                    etype: ev::TRIGGER,
                    number: t as i32,
                    object: owner,
                    action: 0,
                };
                let r = vm.run(self, &cond, Ctx::new(id, id), &[]);
                self.ev = saved;
                let fire = match r {
                    Ok(v) => actions::truthy(&v),
                    Err(e) => {
                        self.report_error(e, 0)?;
                        false
                    }
                };
                if fire {
                    self.run_handler(vm, id, owner, &list, ev::TRIGGER, t as i32, id)?;
                    if self.pending.is_some() {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    /// Timeline advance for every instance (§10.2).
    pub fn advance_timelines(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let mut k = 0;
        while k < self.instances.len() {
            let id = self.instances[k].id;
            k += 1;
            let (tl, p0, sp, running, looping) = {
                let i = &self.instances[k - 1];
                if !i.alive() {
                    continue;
                }
                (i.timeline_index, i.timeline_position, i.timeline_speed, i.timeline_running, i.timeline_loop)
            };
            if !running || tl < 0 {
                continue;
            }
            let Some(Some(tlr)) = self.timelines.get(tl as usize) else { continue };
            let last = tlr.moments.keys().next_back().copied().unwrap_or(0);
            let p1 = p0 + sp;
            let moments: Vec<(i32, Rc<ActionList>)> = if sp > 0.0 {
                tlr.moments
                    .iter()
                    .filter(|(m, _)| (**m as f64) >= p0 && (**m as f64) < p1 && **m >= 0)
                    .map(|(m, l)| (*m, l.clone()))
                    .collect()
            } else if sp < 0.0 {
                tlr.moments
                    .iter()
                    .rev()
                    .filter(|(m, _)| (**m as f64) > p1 && (**m as f64) <= p0 && **m >= 0)
                    .map(|(m, l)| (*m, l.clone()))
                    .collect()
            } else {
                Vec::new()
            };
            if let Some(i) = self.inst_mut(id) {
                i.timeline_position = p1;
            }
            for (_m, list) in moments {
                if !self.inst(id).map(|i| i.alive()).unwrap_or(false) {
                    break;
                }
                let saved = self.ev;
                let saved_tl = self.in_timeline;
                self.in_timeline = true;
                self.ev = EventCtx {
                    etype: -1,
                    number: 0,
                    object: self.inst(id).map(|i| i.object_index).unwrap_or(-1),
                    action: 0,
                };
                let r = actions::execute(self, vm, &list, id, id);
                self.ev = saved;
                self.in_timeline = saved_tl;
                r?;
                if self.pending.is_some() {
                    return Ok(());
                }
            }
            if let Some(i) = self.inst_mut(id) {
                if i.timeline_index == tl {
                    if sp > 0.0 && i.timeline_position > last as f64 && looping {
                        i.timeline_position = 0.0;
                    }
                    if sp < 0.0 && i.timeline_position < 0.0 && looping {
                        i.timeline_position = last as f64;
                    }
                }
            }
        }
        Ok(())
    }

    /// Outside-room / boundary events (§5.5).
    pub fn boundary_events(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let (rw, rh) = (self.rs.width as f64, self.rs.height as f64);
        let mut k = 0;
        while k < self.instances.len() {
            let (id, obj, alive) = {
                let i = &self.instances[k];
                (i.id, i.object_index, i.alive())
            };
            k += 1;
            if !alive || obj < 0 || !self.has_boundary.get(obj as usize).copied().unwrap_or(false) {
                continue;
            }
            let Some(eff) = self.effective.get(obj as usize) else { continue };
            let wants: Vec<i32> = eff
                .keys()
                .filter(|(t, n)| *t == ev::OTHER && (*n == 0 || *n == 1 || (40..58).contains(n)))
                .map(|(_, n)| *n)
                .collect();
            if wants.is_empty() {
                continue;
            }
            let mut fire = Vec::new();
            let (l, t, r, b) = self.bbox_of(id);
            let test = |x0: f64, y0: f64, x1: f64, y1: f64| -> (bool, bool) {
                let outside = r < x0 || l >= x1 || b < y0 || t >= y1;
                let boundary = !outside && (l < x0 || r >= x1 || t < y0 || b >= y1);
                (outside, boundary)
            };
            let (o, bd) = test(0.0, 0.0, rw, rh);
            if o && wants.contains(&0) {
                fire.push(0);
            }
            if bd && wants.contains(&1) {
                fire.push(1);
            }
            if self.rs.views_enabled {
                for v in 0..8 {
                    let vs = &self.rs.views[v];
                    if !vs.visible {
                        continue;
                    }
                    let (o, bd) = test(vs.xview, vs.yview, vs.xview + vs.wview, vs.yview + vs.hview);
                    if o && wants.contains(&(40 + v as i32)) {
                        fire.push(40 + v as i32);
                    }
                    if bd && wants.contains(&(50 + v as i32)) {
                        fire.push(50 + v as i32);
                    }
                }
            }
            for n in fire {
                let obj_now = match self.inst(id) {
                    Some(i) if i.alive() => i.object_index,
                    _ => break,
                };
                if let Some((owner, list)) = self.handler(obj_now, ev::OTHER, n) {
                    self.run_handler(vm, id, owner, &list, ev::OTHER, n, id)?;
                    if self.pending.is_some() {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    /// Collision event phase (§8.6).
    pub fn collision_events(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let n = self.instances.len();
        // objects with at least one live instance (exact object); refreshed after
        // handlers run, as they may create instances
        let present_now = |g: &Game| -> Vec<bool> {
            let mut p = vec![false; g.objects.len()];
            for i in &g.instances {
                if i.alive() && i.object_index >= 0 {
                    if let Some(x) = p.get_mut(i.object_index as usize) {
                        *x = true;
                    }
                }
            }
            p
        };
        let mut present = present_now(self);
        let mut handlers_ran = false;
        for ai in 0..n {
            if ai >= self.instances.len() {
                break;
            }
            let (a, aobj, alive) = {
                let i = &self.instances[ai];
                (i.id, i.object_index, i.alive())
            };
            if !alive || aobj < 0 || !self.has_collision.get(aobj as usize).copied().unwrap_or(false) {
                continue;
            }
            if std::mem::take(&mut handlers_ran) {
                present = present_now(self);
            }
            if !self.collision_partners[aobj as usize].iter().any(|&b| present.get(b as usize).copied().unwrap_or(false)) {
                continue;
            }
            // Broad phase: the instances (after position `from`) whose boxes overlap
            // a's box. Recomputed after every handler, since handlers may move
            // instances or create new ones; this visits the same partners in the
            // same order as a scan over the whole list.
            let cands = |g: &Game, from: usize| -> Vec<u32> {
                let Some(ia) = g.inst(a) else { return Vec::new() };
                let Some(geo) = g.geo_at(ia, ia.x, ia.y) else { return Vec::new() };
                let mut c = g.candidates(geo.bbox);
                c.retain(|&k| k as usize >= from);
                c
            };
            let mut cl = cands(self, 0);
            let mut li = 0;
            while li < cl.len() {
                let bi = cl[li] as usize;
                li += 1;
                if bi >= self.instances.len() {
                    break;
                }
                let (b, bobj, balive) = {
                    let i = &self.instances[bi];
                    (i.id, i.object_index, i.alive())
                };
                if b == a || !balive {
                    continue;
                }
                let aobj_now = match self.inst(a) {
                    Some(i) if i.alive() => i.object_index,
                    _ => break,
                };
                let Some((owner, list)) = self.collision_table.get(&(aobj_now, bobj)).cloned() else {
                    continue;
                };
                if !self.instances_collide(a, b) {
                    continue;
                }
                let other_solid = self.inst(b).map(|i| i.solid).unwrap_or(false);
                if other_solid {
                    let i = self.inst_mut(a).unwrap();
                    i.x = i.xprevious;
                    i.y = i.yprevious;
                }
                handlers_ran = true;
                self.run_handler(vm, a, owner, &list, ev::COLLISION, bobj, b)?;
                if self.pending.is_some() {
                    return Ok(());
                }
                cl = cands(self, bi + 1);
                li = 0;
                if other_solid {
                    let on_path = match self.inst(a) {
                        Some(i) if !i.destroyed => i.path_index >= 0,
                        _ => continue,
                    };
                    if on_path {
                        // move to the new path position; revert if blocked
                        let pos = self.inst(a).unwrap().path_position;
                        let (xp, yp) = (self.inst(a).unwrap().x, self.inst(a).unwrap().y);
                        let (nx, ny) = self.path_point_for(a, pos).unwrap_or((xp, yp));
                        {
                            let i = self.inst_mut(a).unwrap();
                            i.x = nx;
                            i.y = ny;
                        }
                        if self.inst(b).map(|i| i.alive()).unwrap_or(false) && self.instances_collide(a, b) {
                            let i = self.inst_mut(a).unwrap();
                            i.x = i.xprevious;
                            i.y = i.yprevious;
                            i.path_position = i.path_positionprevious;
                        }
                    } else {
                        {
                            let i = self.inst_mut(a).unwrap();
                            i.x += i.hspeed;
                            i.y += i.vspeed;
                        }
                        if self.inst(b).map(|i| i.alive()).unwrap_or(false) && self.instances_collide(a, b) {
                            let i = self.inst_mut(a).unwrap();
                            i.x = i.xprevious;
                            i.y = i.yprevious;
                        }
                    }
                    cl = cands(self, bi + 1);
                    li = 0;
                }
            }
        }
        Ok(())
    }

    /// Animation advance and Animation End events (§7.3).
    pub fn advance_animation(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let anim_end = self.event_objs.get(&(ev::OTHER, ev_other::ANIMATION_END)).cloned();
        let mut k = 0;
        while k < self.instances.len() {
            let (id, alive, spr, speed) = {
                let i = &self.instances[k];
                (i.id, i.alive(), i.sprite_index, i.image_speed)
            };
            k += 1;
            if !alive {
                continue;
            }
            let (n, masks) = self.sprite(spr).map(|s| (s.frames.len(), s.masks.len())).unwrap_or((0, 0));
            let n = n as f64;
            if n == 0.0 {
                continue;
            }
            let mut ended = false;
            {
                // the collision box only depends on image_index with per-frame masks
                let i = if masks > 1 { self.inst_at_mut(k - 1) } else { &mut self.instances[k - 1] };
                i.image_index += speed;
                if i.image_index >= n {
                    i.image_index -= n;
                    ended = true;
                } else if i.image_index < 0.0 && speed < 0.0 {
                    i.image_index += n;
                    ended = true;
                }
            }
            let obj = self.instances[k - 1].object_index;
            let has_end = obj >= 0 && anim_end.as_ref().map(|m| m.get(obj as usize).copied().unwrap_or(false)).unwrap_or(false);
            if ended && has_end {
                self.run_event(vm, id, ev::OTHER, ev_other::ANIMATION_END, id)?;
                if self.pending.is_some() {
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// Lives/health edge events (§5.4).
    pub fn set_lives(&mut self, vm: &mut Vm, v: f64) -> Result<(), Error> {
        let old = self.lives;
        self.lives = v;
        if old > 0.0 && v <= 0.0 {
            self.broadcast_now(vm, ev::OTHER, ev_other::NO_MORE_LIVES)?;
        }
        Ok(())
    }

    pub fn set_health(&mut self, vm: &mut Vm, v: f64) -> Result<(), Error> {
        let old = self.health;
        self.health = v;
        if old > 0.0 && v <= 0.0 {
            self.broadcast_now(vm, ev::OTHER, ev_other::NO_MORE_HEALTH)?;
        }
        Ok(())
    }

    /// Broadcast that does not stop at a pending room change (used for events
    /// raised synchronously from code).
    pub fn broadcast_now(&mut self, vm: &mut Vm, t: i32, n: i32) -> Result<(), Error> {
        let mut k = 0;
        while k < self.instances.len() {
            let (id, obj, alive) = {
                let i = &self.instances[k];
                (i.id, i.object_index, i.alive())
            };
            k += 1;
            if !alive {
                continue;
            }
            if let Some((owner, list)) = self.handler(obj, t, n) {
                self.run_handler(vm, id, owner, &list, t, n, id)?;
            }
        }
        Ok(())
    }
}
