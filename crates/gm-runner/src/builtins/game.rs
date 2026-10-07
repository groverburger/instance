//! Chapter 403 (game play): instances, collisions, rooms, events, misc.

use super::{bo, r, zero, Reg, A};
use crate::collision::{Sel, Shape};
use crate::events::ev;
use crate::game::{Game, Pending};
use gml::{special, Ctx, Error, Value, Vm};

/// Number of live instances designated by an object argument.
fn count_ctx(g: &Game, c: &Ctx, obj: i32) -> usize {
    match obj {
        special::SELF | special::OTHER => g.select_ctx(c, obj).len(),
        t => g.count_of(t),
    }
}

pub fn register(l: &mut Reg) {
    l.push(("place_free", |g, _, c, a| bo(g.place_free(c.self_id, a.r(0)?, a.r(1)?))));
    l.push(("place_empty", |g, _, c, a| bo(g.place_empty(c.self_id, a.r(0)?, a.r(1)?))));
    l.push(("place_meeting", |g, _, c, a| {
        let s = g.sel(c, a.i(2)?);
        bo(g.place_meeting_sel(c.self_id, a.r(0)?, a.r(1)?, s).is_some())
    }));
    l.push(("instance_place", |g, _, c, a| {
        let s = g.sel(c, a.i(2)?);
        r(g.place_meeting_sel(c.self_id, a.r(0)?, a.r(1)?, s).unwrap_or(special::NOONE) as f64)
    }));
    l.push(("position_empty", |g, _, _, a| {
        bo(g.instance_at_point(a.r(0)?, a.r(1)?, Sel::All).is_none())
    }));
    l.push(("position_meeting", |g, _, c, a| {
        let s = g.sel(c, a.i(2)?);
        bo(g.instance_at_point(a.r(0)?, a.r(1)?, s).is_some())
    }));
    l.push(("instance_position", |g, _, c, a| {
        let s = g.sel(c, a.i(2)?);
        r(g.instance_at_point(a.r(0)?, a.r(1)?, s).unwrap_or(special::NOONE) as f64)
    }));
    l.push(("distance_to_point", |g, _, c, a| r(g.bbox_distance_point(c.self_id, a.r(0)?, a.r(1)?))));
    l.push(("distance_to_object", |g, _, c, a| {
        let t = g.select_ctx(c, a.i(0)?);
        let d = t
            .into_iter()
            .filter(|&id| id != c.self_id)
            .map(|id| g.bbox_distance(c.self_id, id))
            .fold(f64::INFINITY, f64::min);
        r(if d.is_finite() { d } else { 1000000.0 })
    }));
    // collision_* queries
    fn coll(g: &Game, c: &Ctx, shape: Shape, obj: i32, prec: bool, notme: bool) -> Result<Value, Error> {
        let s = g.sel(c, obj);
        let me = if notme { Some(c.self_id) } else { None };
        r(g.shape_collision_sel(shape, s, prec, me).unwrap_or(special::NOONE) as f64)
    }
    l.push(("collision_point", |g, _, c, a| {
        coll(g, c, Shape::Point(a.r(0)?, a.r(1)?), a.i(2)?, a.b(3)?, a.b(4)?)
    }));
    l.push(("collision_rectangle", |g, _, c, a| {
        coll(g, c, Shape::Rect(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?), a.i(4)?, a.b(5)?, a.b(6)?)
    }));
    l.push(("collision_circle", |g, _, c, a| {
        coll(g, c, Shape::Circle(a.r(0)?, a.r(1)?, a.r(2)?), a.i(3)?, a.b(4)?, a.b(5)?)
    }));
    l.push(("collision_ellipse", |g, _, c, a| {
        coll(g, c, Shape::Ellipse(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?), a.i(4)?, a.b(5)?, a.b(6)?)
    }));
    l.push(("collision_line", |g, _, c, a| {
        coll(g, c, Shape::Line(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?), a.i(4)?, a.b(5)?, a.b(6)?)
    }));
    // instances
    l.push(("instance_find", |g, _, c, a| {
        let t = g.select_ctx(c, a.i(0)?);
        let n = a.i(1)?;
        r(if n >= 0 { t.get(n as usize).copied().unwrap_or(special::NOONE) } else { special::NOONE } as f64)
    }));
    l.push(("instance_exists", |g, _, c, a| bo(count_ctx(g, c, a.i(0)?) > 0)));
    l.push(("instance_number", |g, _, c, a| r(count_ctx(g, c, a.i(0)?) as f64)));
    fn nearest(g: &Game, c: &Ctx, a: &A, far: bool) -> Result<Value, Error> {
        let (x, y) = (a.r(0)?, a.r(1)?);
        let t = g.select_ctx(c, a.i(2)?);
        let mut best = special::NOONE;
        let mut bd = if far { -1.0 } else { f64::INFINITY };
        for id in t {
            let i = g.inst(id).unwrap();
            let d = (i.x - x).hypot(i.y - y);
            if (far && d > bd) || (!far && d < bd) {
                bd = d;
                best = id;
            }
        }
        r(best as f64)
    }
    l.push(("instance_nearest", |g, _, c, a| nearest(g, c, a, false)));
    l.push(("instance_furthest", |g, _, c, a| nearest(g, c, a, true)));
    l.push(("instance_create", |g, vm, c, a| {
        let id = g.create_instance(vm, a.r(0)?, a.r(1)?, a.i(2)?, c.self_id)?;
        r(id as f64)
    }));
    l.push(("instance_copy", |g, vm, c, a| {
        let Some(src) = g.inst(c.self_id).cloned() else { return r(special::NOONE as f64) };
        let id = g.alloc_id();
        let mut n = src;
        n.id = id;
        n.destroyed = false;
        n.active = true;
        g.push_instance(n);
        if a.b(0)? {
            g.run_event(vm, id, ev::CREATE, 0, c.self_id)?;
        }
        r(id as f64)
    }));
    l.push(("instance_change", |g, vm, c, a| {
        g.change_instance(vm, c.self_id, a.i(0)?, a.b(1)?)?;
        zero()
    }));
    l.push(("instance_destroy", |g, vm, c, _| {
        g.destroy_instance(vm, c.self_id, c.other_id)?;
        zero()
    }));
    l.push(("position_destroy", |g, vm, c, a| {
        let (x, y) = (a.r(0)?, a.r(1)?);
        for id in g.instances_at_point(x, y) {
            if g.point_in_instance(id, x, y, true) {
                g.destroy_instance(vm, id, c.self_id)?;
            }
        }
        zero()
    }));
    l.push(("position_change", |g, vm, _, a| {
        let (x, y, obj, perf) = (a.r(0)?, a.r(1)?, a.i(2)?, a.b(3)?);
        for id in g.instances_at_point(x, y) {
            if g.point_in_instance(id, x, y, true) {
                g.change_instance(vm, id, obj, perf)?;
            }
        }
        zero()
    }));
    // deactivation
    l.push(("instance_deactivate_all", |g, _, c, a| {
        let notme = a.b(0)?;
        g.spatial_invalidate();
        for i in g.instances.iter_mut() {
            if !(notme && i.id == c.self_id) {
                i.active = false;
            }
        }
        zero()
    }));
    l.push(("instance_deactivate_object", |g, _, c, a| {
        let t = g.select_ctx(c, a.i(0)?);
        for id in t {
            g.inst_mut(id).unwrap().active = false;
        }
        zero()
    }));
    l.push(("instance_deactivate_region", |g, _, c, a| {
        let (l_, t_, w, h, inside, notme) = (a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, a.b(4)?, a.b(5)?);
        let ids: Vec<i32> = g.instances.iter().filter(|i| i.alive()).map(|i| i.id).collect();
        for id in ids {
            if notme && id == c.self_id {
                continue;
            }
            let (bl, bt, br, bb) = g.bbox_of(id);
            let overlap = br >= l_ && bl < l_ + w && bb >= t_ && bt < t_ + h;
            if overlap == inside {
                g.inst_mut(id).unwrap().active = false;
            }
        }
        zero()
    }));
    l.push(("instance_activate_all", |g, _, _, _| {
        g.spatial_invalidate();
        for i in g.instances.iter_mut() {
            i.active = true;
        }
        zero()
    }));
    l.push(("instance_activate_object", |g, _, _, a| {
        let o = a.i(0)?;
        let ids: Vec<i32> = g
            .instances
            .iter()
            .filter(|i| !i.destroyed && (o == special::ALL || i.id == o || (o >= 0 && o < special::FIRST_INSTANCE_ID && g.is_a(i.object_index, o))))
            .map(|i| i.id)
            .collect();
        for id in ids {
            g.inst_mut(id).unwrap().active = true;
        }
        zero()
    }));
    l.push(("instance_activate_region", |g, _, _, a| {
        let (l_, t_, w, h, inside) = (a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, a.b(4)?);
        let ids: Vec<i32> = g.instances.iter().filter(|i| !i.active && !i.destroyed).map(|i| i.id).collect();
        for id in ids {
            let (bl, bt, br, bb) = g.bbox_of(id);
            let overlap = br >= l_ && bl < l_ + w && bb >= t_ && bt < t_ + h;
            if overlap == inside {
                g.inst_mut(id).unwrap().active = true;
            }
        }
        zero()
    }));
    // timing / rooms / game
    l.push(("sleep", |g, _, _, a| {
        g.do_sleep(a.r(0)?);
        zero()
    }));
    l.push(("room_goto", |g, _, _, a| {
        let rm = a.i(0)?;
        if !g.room_exists(rm) {
            return Err(Error::new(format!("Unexisting room number: {}", rm)));
        }
        g.request_goto(rm);
        zero()
    }));
    l.push(("room_goto_previous", |g, _, _, _| {
        let p = g.room_previous(g.room);
        if p < 0 {
            return Err(Error::new("Moving to the previous room before the first room."));
        }
        g.request_goto(p);
        zero()
    }));
    l.push(("room_goto_next", |g, _, _, _| {
        let n = g.room_next(g.room);
        if n < 0 {
            return Err(Error::new("Moving to the next room after the last room."));
        }
        g.request_goto(n);
        zero()
    }));
    l.push(("room_previous", |g, _, _, a| r(g.room_previous(a.i(0)?) as f64)));
    l.push(("room_next", |g, _, _, a| r(g.room_next(a.i(0)?) as f64)));
    l.push(("room_restart", |g, _, _, _| {
        g.request_goto(g.room);
        zero()
    }));
    l.push(("game_end", |g, _, _, _| {
        g.pending = Some(Pending::End);
        zero()
    }));
    l.push(("game_restart", |g, _, _, _| {
        g.pending = Some(Pending::Restart);
        zero()
    }));
    l.push(("game_load", |g, _, _, a| {
        let f = a.s(0)?;
        g.pending = Some(Pending::Load(g.resolve_path(&f).to_string_lossy().into_owned()));
        zero()
    }));
    l.push(("game_save", |g, vm, _, a| {
        let f = a.s(0)?;
        let p = g.resolve_path(&f);
        crate::save::save_game(g, vm, &p.to_string_lossy()).map_err(Error::new)?;
        zero()
    }));
    l.push(("transition_define", |g, _, _, a| {
        let kind = a.i(0)?;
        let name = a.s(1)?;
        match g.script_names.get(&name) {
            Some(&s) => {
                g.transitions.insert(kind, s);
                zero()
            }
            None => Err(Error::new(format!("transition_define: unknown script {}", name))),
        }
    }));
    l.push(("transition_exists", |g, _, _, a| {
        let k = a.i(0)?;
        bo((0..=21).contains(&k) || g.transitions.contains_key(&k))
    }));
    // events
    l.push(("event_perform", |g, vm, c, a| {
        let (t, n) = (a.i(0)?, a.i(1)?);
        g.run_event(vm, c.self_id, t, n, c.other_id)?;
        zero()
    }));
    l.push(("event_user", |g, vm, c, a| {
        let n = a.i(0)?;
        g.run_event(vm, c.self_id, ev::OTHER, 10 + n, c.other_id)?;
        zero()
    }));
    l.push(("event_perform_object", |g, vm, c, a| {
        let (o, t, n) = (a.i(0)?, a.i(1)?, a.i(2)?);
        if let Some((owner, list)) = g.handler(o, t, n) {
            g.run_handler(vm, c.self_id, owner, &list, t, n, c.other_id)?;
        }
        zero()
    }));
    l.push(("event_inherited", |g, vm, c, _| {
        g.event_inherited(vm, c)?;
        zero()
    }));
    l.push(("show_debug_message", |g, _, _, a| {
        let t = String::from_utf8_lossy(&a.text(0)).into_owned();
        if g.headless {
            eprintln!("[debug] {}", t);
        }
        g.debug_messages.push(t);
        zero()
    }));
    l.push(("set_program_priority", |_, _, _, _| zero()));
    l.push(("set_application_title", |g, _, _, a| {
        g.application_title = a.s(0)?;
        zero()
    }));
    let _ = |_: &mut Game, _: &mut Vm| ();
}

impl Game {
    /// Blocks for `ms` milliseconds (not in headless mode).
    pub fn do_sleep(&mut self, ms: f64) {
        if ms > 0.0 && !self.headless {
            std::thread::sleep(std::time::Duration::from_secs_f64(ms / 1000.0));
        }
    }
}
