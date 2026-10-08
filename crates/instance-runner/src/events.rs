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
            action: -1,
        };
        let t0 = self.prof.as_ref().map(|_| web_time::Instant::now());
        let r = actions::execute(self, vm, list, id, other);
        if let (Some(p), Some(t0)) = (self.prof.as_mut(), t0) {
            crate::profile::Profiler::add(&mut p.events, (owner, t, n), t0);
        }
        // VERIFIED black-box (bb_instances destroy.read_after): the event, object and
        // instance are restored after a nested event, but the action number is not.
        // An error later in the calling action names the nested event's last action
        // ("action number 1" after `with (u) instance_destroy()` ran u's one-action
        // Destroy event, although the code was the Create event's action 3).
        let nested_action = self.ev.action;
        self.ev = saved;
        if nested_action >= 0 {
            self.ev.action = nested_action;
        }
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

    /// The instances (from list position `from` on) whose object is set in `mask`,
    /// as (object, id) sorted by object index, instance order within an object.
    fn object_major(&self, mask: &[bool], from: usize) -> Vec<(i32, i32)> {
        let mut v: Vec<(i32, i32)> = self.instances[from.min(self.instances.len())..]
            .iter()
            .filter(|i| i.alive() && i.object_index >= 0 && mask.get(i.object_index as usize).copied().unwrap_or(false))
            .map(|i| (i.object_index, i.id))
            .collect();
        v.sort_by_key(|e| e.0); // stable: instance order within an object
        v
    }

    /// Delivers an event to all instances having a handler (§4.3).
    ///
    /// VERIFIED black-box (bb_events ev.s02/ev.s03, bb_instances in.s01): GM 8.1
    /// dispatches object by object (ascending object index; within an object in
    /// instance order). An instance created during the phase is visited in the same
    /// phase if its object comes after the object being processed (or is the same
    /// object).
    pub fn broadcast(&mut self, vm: &mut Vm, t: i32, n: i32) -> Result<(), Error> {
        // objects having a handler for this event (none: nothing to do)
        let Some(mask) = self.event_objs.get(&(t, n)).cloned() else { return Ok(()) };
        self.broadcast_masked(vm, t, n, &mask)
    }

    fn broadcast_masked(&mut self, vm: &mut Vm, t: i32, n: i32, mask: &[bool]) -> Result<(), Error> {
        let mut order = self.object_major(mask, 0);
        let mut seen = self.instances.len();
        let mut idx = 0;
        while idx < order.len() {
            let (cur_obj, id) = order[idx];
            idx += 1;
            let obj = match self.inst(id) {
                Some(i) if i.alive() => i.object_index,
                _ => continue,
            };
            if let Some((owner, list)) = self.handler(obj, t, n) {
                self.run_handler(vm, id, owner, &list, t, n, id)?;
                if self.pending.is_some() {
                    return Ok(());
                }
            }
            if self.instances.len() > seen {
                for (o, nid) in self.object_major(mask, seen) {
                    if o < cur_obj {
                        continue;
                    }
                    let at = idx + order[idx..].partition_point(|e| e.0 <= o);
                    order.insert(at, (o, nid));
                }
                seen = self.instances.len();
            }
        }
        Ok(())
    }

    /// Alarm phase (§5.3).
    ///
    /// VERIFIED black-box (bb_events ev.alarms.*, ev.s02+1, ev.s03+1): alarm by
    /// alarm (0..11), each object by object as in [`Game::broadcast`]. Only
    /// instances whose object has (or inherits) the alarm event count down: a
    /// positive alarm is decremented and fires when it reaches 0; the alarm then
    /// stays 0 (read as 0 in the event) and becomes -1 one step later without
    /// firing. Alarm values are integers (assignment rounds, see host.rs).
    pub fn run_alarms(&mut self, vm: &mut Vm) -> Result<(), Error> {
        for a in 0..crate::instance::ALARMS {
            let Some(mask) = self.event_objs.get(&(ev::ALARM, a as i32)).cloned() else { continue };
            for (_, id) in self.object_major(&mask, 0) {
                let Some(&kk) = self.inst_index.get(&id) else { continue };
                let i = &mut self.instances[kk];
                if !i.alive() || !mask.get(i.object_index.max(0) as usize).copied().unwrap_or(false) {
                    continue;
                }
                // (alarms do not affect collision geometry: no need to mark the instance)
                if i.alarm[a] == 0.0 {
                    i.alarm[a] = -1.0;
                    continue;
                }
                if i.alarm[a] < 0.0 {
                    continue;
                }
                i.alarm[a] -= 1.0;
                if i.alarm[a] <= 0.0 {
                    i.alarm[a] = 0.0;
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
            // VERIFIED black-box (bb_events c.OR / c.IB, 16 px sprite moving right in a
            // 1024 wide room): Outside fires once bbox_left > room_width (x 1028, not
            // 1024); Intersect Boundary fires every step the box is not completely
            // inside — also while completely outside (x 1012 .. 1036). Outside is
            // dispatched before Intersect Boundary.
            let test = |x0: f64, y0: f64, x1: f64, y1: f64| -> (bool, bool) {
                let outside = r < x0 || l > x1 || b < y0 || t > y1;
                let boundary = l < x0 || r >= x1 || t < y0 || b >= y1;
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

    /// Collision event phase (§8.6). VERIFIED black-box (bb_motion mo.seq /
    /// mo.sol): objects in index order; per object its collision events in
    /// ascending target-object order; per event the object's instances in
    /// instance order, each against the colliding target instances in instance
    /// order. A colliding pair is handled once per step: if either instance is
    /// solid both are put back to their previous positions, then the first
    /// instance's event runs and right after it the partner's event for the
    /// first instance's object (if any); then (solid case) both are moved to
    /// their new positions and put back again if they still collide.
    pub fn collision_events(&mut self, vm: &mut Vm) -> Result<(), Error> {
        let nobj = self.objects.len();
        let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); nobj];
        let mut present = vec![false; nobj];
        let mut any = false;
        for (k, i) in self.instances.iter().enumerate() {
            let o = i.object_index;
            if !i.alive() || o < 0 || o as usize >= nobj {
                continue;
            }
            present[o as usize] = true;
            if self.has_collision[o as usize] {
                buckets[o as usize].push(k);
                any = true;
            }
        }
        if !any {
            return Ok(());
        }
        let mut done: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
        let mut handlers_ran = false;
        for aobj in 0..nobj {
            if buckets[aobj].is_empty() {
                continue;
            }
            if std::mem::take(&mut handlers_ran) {
                present = vec![false; nobj];
                for i in &self.instances {
                    if i.alive() && i.object_index >= 0 && (i.object_index as usize) < nobj {
                        present[i.object_index as usize] = true;
                    }
                }
            }
            if !self.collision_partners[aobj].iter().any(|&b| present.get(b as usize).copied().unwrap_or(false)) {
                continue;
            }
            let targets = self.collision_targets[aobj].clone();
            let bucket = std::mem::take(&mut buckets[aobj]);
            for &target in &targets {
                for &ai in &bucket {
                    let (a, alive, o) = match self.instances.get(ai) {
                        Some(i) => (i.id, i.alive(), i.object_index),
                        None => continue,
                    };
                    if !alive || o != aobj as i32 {
                        continue;
                    }
                    // Broad phase: the instances (after position `from`) whose boxes
                    // overlap a's box; recomputed after handlers ran (they may move
                    // or create instances), visiting partners in instance order.
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
                        let Some((owner, list, tg)) = self.collision_table.get(&(aobj_now, bobj)).cloned() else {
                            continue;
                        };
                        if tg != target {
                            continue;
                        }
                        let key = (a.min(b), a.max(b));
                        if done.contains(&key) || !self.instances_collide(a, b) {
                            continue;
                        }
                        done.insert(key);
                        handlers_ran = true;
                        self.collision_pair(vm, a, b, owner, &list, bobj)?;
                        if self.pending.is_some() {
                            return Ok(());
                        }
                        cl = cands(self, bi + 1);
                        li = 0;
                    }
                }
            }
        }
        Ok(())
    }

    /// One colliding pair: `a`'s handler (owner, list) for `b`, then `b`'s
    /// handler for `a` (if any), with the solid put-back around them.
    fn collision_pair(
        &mut self,
        vm: &mut Vm,
        a: i32,
        b: i32,
        owner: i32,
        list: &Rc<ActionList>,
        bobj: i32,
    ) -> Result<(), Error> {
        let solid = self.inst(a).map(|i| i.solid).unwrap_or(false) || self.inst(b).map(|i| i.solid).unwrap_or(false);
        if solid {
            for id in [a, b] {
                if let Some(i) = self.inst_mut(id) {
                    i.x = i.xprevious;
                    i.y = i.yprevious;
                }
            }
        }
        self.run_handler(vm, a, owner, list, ev::COLLISION, bobj, b)?;
        if self.pending.is_some() {
            return Ok(());
        }
        let alive = |g: &Game, id: i32| g.inst(id).map(|i| i.alive()).unwrap_or(false);
        if alive(self, a) && alive(self, b) {
            let (ao, bo) = (self.inst(a).unwrap().object_index, self.inst(b).unwrap().object_index);
            if let Some((owner_b, list_b, _)) = self.collision_table.get(&(bo, ao)).cloned() {
                self.run_handler(vm, b, owner_b, &list_b, ev::COLLISION, ao, a)?;
                if self.pending.is_some() {
                    return Ok(());
                }
            }
        }
        if solid {
            for id in [a, b] {
                if !alive(self, id) {
                    continue;
                }
                let on_path = self.inst(id).unwrap().path_index >= 0;
                if on_path {
                    let pos = self.inst(id).unwrap().path_position;
                    let (xp, yp) = (self.inst(id).unwrap().x, self.inst(id).unwrap().y);
                    let (nx, ny) = self.path_point_for(id, pos).unwrap_or((xp, yp));
                    let i = self.inst_mut(id).unwrap();
                    i.x = nx;
                    i.y = ny;
                } else {
                    let i = self.inst_mut(id).unwrap();
                    i.x += i.hspeed;
                    i.y += i.vspeed;
                }
            }
            if alive(self, a) && alive(self, b) && self.instances_collide(a, b) {
                for id in [a, b] {
                    let i = self.inst_mut(id).unwrap();
                    i.x = i.xprevious;
                    i.y = i.yprevious;
                    if i.path_index >= 0 {
                        i.path_position = i.path_positionprevious;
                    }
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
                // VERIFIED black-box (bb_motion mo.an3.*): a negative image_speed
                // is not wrapped (image_index goes -1, -2, ...) and never fires
                // Animation End.
                if i.image_index >= n {
                    i.image_index -= n;
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
    /// Instance order (VERIFIED black-box for Room Start, bb_events ev.s10+1: the
    /// placed instances of the new room before the carried persistent ones).
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
