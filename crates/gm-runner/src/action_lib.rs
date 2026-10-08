//! The `action_*` functions behind the built-in drag-and-drop actions
//! (specs/actions.json). Each receives the evaluated arguments and the action's
//! "relative" flag.

use crate::actions::ActionFn;
use crate::game::{Game, Pending};
use crate::instance::normalize_angle;
use gml::{Ctx, Error, Value, Vm};
use std::collections::HashMap;
use std::sync::OnceLock;

type R = Result<Value, Error>;

fn real(a: &[Value], i: usize) -> f64 {
    match a.get(i) {
        Some(Value::Real(x)) => *x,
        _ => 0.0,
    }
}

fn int(a: &[Value], i: usize) -> i32 {
    gml::to_int(real(a, i)) as i32
}

fn text(a: &[Value], i: usize) -> Vec<u8> {
    match a.get(i) {
        Some(Value::Str(s)) => s.as_bytes().to_vec(),
        Some(Value::Real(r)) => gml::real_to_string(*r).into_bytes(),
        None => Vec::new(),
    }
}

fn text_s(a: &[Value], i: usize) -> String {
    gm_format::text::decode_cp1252(&text(a, i))
}

fn ok() -> R {
    Ok(Value::Real(0.0))
}

fn b(x: bool) -> R {
    Ok(Value::Real(if x { 1.0 } else { 0.0 }))
}

fn me<'a>(g: &'a mut Game, c: &Ctx) -> Result<&'a mut crate::instance::Instance, Error> {
    g.inst_req(c.self_id)
}

fn pos(g: &Game, c: &Ctx, rel: bool, x: f64, y: f64) -> (f64, f64) {
    if rel {
        if let Some(i) = g.inst(c.self_id) {
            return (i.x + x, i.y + y);
        }
    }
    (x, y)
}

fn compare(a: f64, b: f64, op: i32) -> bool {
    match op {
        1 => a < b,
        2 => a > b,
        3 => a <= b,
        4 => a >= b,
        _ => a == b,
    }
}

fn compare_v(a: &Value, b: &Value, op: i32) -> bool {
    match (a, b) {
        (Value::Real(x), Value::Real(y)) => compare(*x, *y, op),
        (Value::Str(x), Value::Str(y)) => match op {
            1 => x.as_bytes() < y.as_bytes(),
            2 => x.as_bytes() > y.as_bytes(),
            _ => x == y,
        },
        _ => false,
    }
}

fn color_const(name: &str) -> i32 {
    gml::builtins::constant_value(name).unwrap_or(0.0) as i32
}

const MENU_COLORS: [&str; 16] = [
    "c_black", "c_gray", "c_silver", "c_white", "c_maroon", "c_green", "c_olive", "c_navy", "c_purple", "c_teal", "c_red",
    "c_lime", "c_yellow", "c_blue", "c_fuchsia", "c_aqua",
];

/// Parses an action font string `"Name",size,bold,italic,...` into a font.
fn parse_font_string(s: &str) -> (String, i32, bool, bool) {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for ch in s.chars() {
        match ch {
            '"' => quoted = !quoted,
            ',' if !quoted => parts.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    parts.push(cur);
    let name = parts.first().cloned().filter(|n| !n.is_empty()).unwrap_or_else(|| "Arial".into());
    let size = parts.get(1).and_then(|x| x.trim().parse().ok()).unwrap_or(12);
    let bold = parts.get(2).map(|x| x.trim() == "1").unwrap_or(false);
    let italic = parts.get(3).map(|x| x.trim() == "1").unwrap_or(false);
    (name, size, bold, italic)
}

fn goto_with_transition(g: &mut Game, room: i32, trans: i32) -> R {
    if room < 0 || !g.room_exists(room) {
        return Err(Error::new(format!("Unexisting room number: {}", room)));
    }
    g.transition_kind = trans;
    g.request_goto(room);
    ok()
}

fn draw_text_at(g: &mut Game, x: f64, y: f64, s: &[u8]) {
    let (c, al) = (g.gfx.color, g.gfx.alpha);
    g.draw_text_full(x, y, s, -1.0, -1.0, 1.0, 1.0, 0.0, [c; 4], al);
}

fn part_sys(g: &mut Game) -> i32 {
    if g.particles.sys(g.particles.action_system).is_none() {
        g.particles.action_system = g.particles.system_create();
    }
    g.particles.action_system
}

fn part_type(g: &mut Game, id: i32) -> i32 {
    let k = id.clamp(0, 15) as usize;
    let t = g.particles.action_types[k];
    if g.particles.ty(t).is_none() {
        g.particles.action_types[k] = g.particles.type_create();
    }
    g.particles.action_types[k]
}

fn table() -> &'static HashMap<&'static str, ActionFn> {
    static T: OnceLock<HashMap<&'static str, ActionFn>> = OnceLock::new();
    T.get_or_init(build)
}

/// Looks up a runner action function by name.
pub fn lookup(name: &str) -> Option<ActionFn> {
    table().get(name).copied()
}

/// Names of all implemented action functions.
pub fn names() -> Vec<&'static str> {
    let mut v: Vec<&str> = table().keys().copied().collect();
    v.sort();
    v
}

#[allow(clippy::type_complexity)]
fn build() -> HashMap<&'static str, ActionFn> {
    let mut m: HashMap<&'static str, ActionFn> = HashMap::new();
    macro_rules! act {
        ($name:literal, $f:expr) => {
            m.insert($name, $f);
        };
    }
    // ------------------------------------------------------------------ move
    act!("action_move", |g, vm, c, a, rel| {
        let mask = text_s(a, 0);
        let sp = real(a, 1);
        let dirs: Vec<usize> = mask.chars().take(9).enumerate().filter(|(_, ch)| *ch == '1').map(|(i, _)| i).collect();
        if dirs.is_empty() {
            return ok();
        }
        let k = dirs[(vm.random() * dirs.len() as f64) as usize % dirs.len()];
        const ANG: [f64; 9] = [225.0, 270.0, 315.0, 180.0, 0.0, 0.0, 135.0, 90.0, 45.0];
        let i = me(g, c)?;
        if k == 4 {
            if !rel {
                i.set_speed(0.0);
            }
        } else if rel {
            i.motion_add(ANG[k], sp);
        } else {
            i.motion_set(ANG[k], sp);
        }
        ok()
    });
    act!("action_set_motion", |g, _, c, a, rel| {
        let i = me(g, c)?;
        if rel {
            i.motion_add(real(a, 0), real(a, 1));
        } else {
            i.motion_set(real(a, 0), real(a, 1));
        }
        ok()
    });
    act!("action_move_point", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 0), real(a, 1));
        let sp = real(a, 2);
        let i = me(g, c)?;
        let dir = (i.y - y).atan2(x - i.x).to_degrees();
        i.motion_set(dir, sp);
        ok()
    });
    act!("action_set_hspeed", |g, _, c, a, rel| {
        let i = me(g, c)?;
        let v = if rel { i.hspeed + real(a, 0) } else { real(a, 0) };
        i.set_hspeed(v);
        ok()
    });
    act!("action_set_vspeed", |g, _, c, a, rel| {
        let i = me(g, c)?;
        let v = if rel { i.vspeed + real(a, 0) } else { real(a, 0) };
        i.set_vspeed(v);
        ok()
    });
    act!("action_set_gravity", |g, _, c, a, rel| {
        let i = me(g, c)?;
        if rel {
            i.gravity_direction += real(a, 0);
            i.gravity += real(a, 1);
        } else {
            i.gravity_direction = real(a, 0);
            i.gravity = real(a, 1);
        }
        ok()
    });
    act!("action_reverse_xdir", |g, _, c, _, _| {
        let i = me(g, c)?;
        let v = -i.hspeed;
        i.set_hspeed(v);
        ok()
    });
    act!("action_reverse_ydir", |g, _, c, _, _| {
        let i = me(g, c)?;
        let v = -i.vspeed;
        i.set_vspeed(v);
        ok()
    });
    act!("action_set_friction", |g, _, c, a, rel| {
        let i = me(g, c)?;
        i.friction = if rel { i.friction + real(a, 0) } else { real(a, 0) };
        ok()
    });
    act!("action_move_to", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 0), real(a, 1));
        let i = me(g, c)?;
        i.x = x;
        i.y = y;
        ok()
    });
    act!("action_move_start", |g, _, c, _, _| {
        let i = me(g, c)?;
        i.x = i.xstart;
        i.y = i.ystart;
        ok()
    });
    act!("action_move_random", |g, vm, c, a, _| {
        me(g, c)?;
        g.move_random(vm, c.self_id, real(a, 0), real(a, 1));
        ok()
    });
    act!("action_snap", |g, _, c, a, _| {
        let (h, v) = (real(a, 0), real(a, 1));
        let i = me(g, c)?;
        if h > 0.0 {
            i.x = (i.x / h).round() * h;
        }
        if v > 0.0 {
            i.y = (i.y / v).round() * v;
        }
        ok()
    });
    act!("action_wrap", |g, _, c, a, _| {
        let dir = int(a, 0);
        let (rw, rh) = (g.rs.width as f64, g.rs.height as f64);
        let (l, t, r, bt) = g.bbox_of(c.self_id);
        let (w, h) = (r - l + 1.0, bt - t + 1.0);
        let i = me(g, c)?;
        if dir == 0 || dir == 2 {
            if i.hspeed <= 0.0 && r < 0.0 {
                i.x += rw + w;
            } else if i.hspeed >= 0.0 && l >= rw {
                i.x -= rw + w;
            }
        }
        if dir == 1 || dir == 2 {
            if i.vspeed <= 0.0 && bt < 0.0 {
                i.y += rh + h;
            } else if i.vspeed >= 0.0 && t >= rh {
                i.y -= rh + h;
            }
        }
        ok()
    });
    act!("action_move_contact", |g, _, c, a, _| {
        me(g, c)?;
        g.move_contact(c.self_id, real(a, 0), real(a, 1), int(a, 2) == 1);
        ok()
    });
    act!("action_bounce", |g, _, c, a, _| {
        me(g, c)?;
        g.move_bounce(c.self_id, int(a, 0) == 1, int(a, 1) == 1);
        ok()
    });
    act!("action_path", |g, _, c, a, _| {
        me(g, c)?;
        g.path_start(c.self_id, int(a, 0), real(a, 1), int(a, 2), int(a, 3) == 1);
        ok()
    });
    act!("action_path_end", |g, _, c, _, _| {
        me(g, c)?.path_index = -1;
        ok()
    });
    act!("action_path_position", |g, _, c, a, rel| {
        let i = me(g, c)?;
        i.path_position = (if rel { i.path_position + real(a, 0) } else { real(a, 0) }).clamp(0.0, 1.0);
        ok()
    });
    act!("action_path_speed", |g, _, c, a, rel| {
        let i = me(g, c)?;
        i.path_speed = if rel { i.path_speed + real(a, 0) } else { real(a, 0) };
        ok()
    });
    act!("action_linear_step", |g, vm, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 0), real(a, 1));
        let f = gml::builtins::function_id("mp_linear_step").unwrap();
        vm.call_function(g, c, f, &[Value::Real(x), Value::Real(y), Value::Real(real(a, 2)), Value::Real((int(a, 3) == 1) as i32 as f64)])
    });
    act!("action_potential_step", |g, vm, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 0), real(a, 1));
        let f = gml::builtins::function_id("mp_potential_step").unwrap();
        vm.call_function(g, c, f, &[Value::Real(x), Value::Real(y), Value::Real(real(a, 2)), Value::Real((int(a, 3) == 1) as i32 as f64)])
    });
    // ------------------------------------------------------------------ objects
    act!("action_create_object", |g, vm, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 1), real(a, 2));
        g.create_instance(vm, x, y, int(a, 0), c.self_id)?;
        ok()
    });
    act!("action_create_object_motion", |g, vm, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 1), real(a, 2));
        let id = g.create_instance(vm, x, y, int(a, 0), c.self_id)?;
        if let Some(i) = g.inst_mut(id) {
            i.motion_set(real(a, 4), real(a, 3));
        }
        ok()
    });
    act!("action_create_object_random", |g, vm, c, a, rel| {
        let objs: Vec<i32> = (0..4).map(|k| int(a, k)).filter(|&o| o >= 0 && g.object(o).is_some()).collect();
        if objs.is_empty() {
            return ok();
        }
        let o = objs[(vm.random() * objs.len() as f64) as usize % objs.len()];
        let (x, y) = pos(g, c, rel, real(a, 4), real(a, 5));
        g.create_instance(vm, x, y, o, c.self_id)?;
        ok()
    });
    act!("action_change_object", |g, vm, c, a, _| {
        g.change_instance(vm, c.self_id, int(a, 0), int(a, 1) == 1)?;
        ok()
    });
    act!("action_kill_object", |g, vm, c, _, _| {
        g.destroy_instance(vm, c.self_id, c.other_id)?;
        ok()
    });
    act!("action_kill_position", |g, vm, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 0), real(a, 1));
        let (fx, fy) = (x.floor() as i32, y.floor() as i32);
        let ids: Vec<i32> = g
            .candidates([fx, fy, fx, fy])
            .into_iter()
            .map(|k| g.instances[k as usize].id)
            .collect();
        for id in ids {
            if g.inst(id).map(|i| i.alive()).unwrap_or(false) && g.point_in_instance(id, x, y, false) {
                g.destroy_instance(vm, id, c.self_id)?;
            }
        }
        ok()
    });
    act!("action_sprite_set", |g, _, c, a, _| {
        let (spr, sub, sp) = (int(a, 0), real(a, 1), real(a, 2));
        let i = me(g, c)?;
        i.sprite_index = spr;
        if sub >= 0.0 {
            i.image_index = sub;
        }
        i.image_speed = sp;
        ok()
    });
    act!("action_sprite_transform", |g, _, c, a, _| {
        let (xs, ys, ang, mirror) = (real(a, 0), real(a, 1), real(a, 2), int(a, 3));
        let i = me(g, c)?;
        i.image_xscale = if mirror == 1 || mirror == 3 { -xs } else { xs };
        i.image_yscale = if mirror == 2 || mirror == 3 { -ys } else { ys };
        i.image_angle = ang;
        ok()
    });
    act!("action_sprite_color", |g, _, c, a, _| {
        let i = me(g, c)?;
        i.image_blend = int(a, 0);
        i.image_alpha = real(a, 1);
        ok()
    });
    act!("action_set_sprite", |g, _, c, a, _| {
        let i = me(g, c)?;
        i.sprite_index = int(a, 0);
        i.image_xscale = real(a, 1);
        i.image_yscale = real(a, 1);
        ok()
    });
    // ------------------------------------------------------------------ sound
    act!("action_sound", |g, _, _, a, _| {
        g.sound_play(int(a, 0), int(a, 1) == 1)?;
        ok()
    });
    act!("action_end_sound", |g, _, _, a, _| {
        g.audio.stop(int(a, 0));
        ok()
    });
    act!("action_if_sound", |g, _, _, a, _| b(g.audio.is_playing(int(a, 0))));
    // ------------------------------------------------------------------ rooms
    act!("action_previous_room", |g, _, _, a, _| {
        let r = g.room_previous(g.room);
        goto_with_transition(g, r, int(a, 0))
    });
    act!("action_next_room", |g, _, _, a, _| {
        let r = g.room_next(g.room);
        goto_with_transition(g, r, int(a, 0))
    });
    act!("action_current_room", |g, _, _, a, _| {
        let r = g.room;
        goto_with_transition(g, r, int(a, 0))
    });
    act!("action_another_room", |g, _, _, a, _| goto_with_transition(g, int(a, 0), int(a, 1)));
    act!("action_if_previous_room", |g, _, _, _, _| b(g.room_previous(g.room) >= 0));
    act!("action_if_next_room", |g, _, _, _, _| b(g.room_next(g.room) >= 0));
    // ------------------------------------------------------------------ main2
    act!("action_set_alarm", |g, _, c, a, rel| {
        let n = int(a, 1).clamp(0, 11) as usize;
        let i = me(g, c)?;
        i.alarm[n] = gml::to_int(if rel { i.alarm[n] + real(a, 0) } else { real(a, 0) }) as f64;
        ok()
    });
    act!("action_sleep", |g, vm, _, a, _| {
        if int(a, 1) == 1 {
            g.draw_frame(vm)?;
            g.gfx.present_now();
        }
        g.do_sleep(real(a, 0));
        ok()
    });
    act!("action_set_timeline", |g, _, c, a, _| {
        let i = me(g, c)?;
        i.timeline_index = int(a, 0);
        i.timeline_position = real(a, 1);
        i.timeline_running = true;
        ok()
    });
    act!("action_timeline_set", |g, _, c, a, _| {
        let i = me(g, c)?;
        i.timeline_index = int(a, 0);
        i.timeline_position = real(a, 1);
        i.timeline_running = int(a, 2) == 0;
        i.timeline_loop = int(a, 3) == 1;
        ok()
    });
    act!("action_set_timeline_position", |g, _, c, a, rel| {
        let i = me(g, c)?;
        i.timeline_position = if rel { i.timeline_position + real(a, 0) } else { real(a, 0) };
        ok()
    });
    act!("action_set_timeline_speed", |g, _, c, a, rel| {
        let i = me(g, c)?;
        i.timeline_speed = if rel { i.timeline_speed + real(a, 0) } else { real(a, 0) };
        ok()
    });
    act!("action_timeline_start", |g, _, c, _, _| {
        me(g, c)?.timeline_running = true;
        ok()
    });
    act!("action_timeline_pause", |g, _, c, _, _| {
        me(g, c)?.timeline_running = false;
        ok()
    });
    act!("action_timeline_stop", |g, _, c, _, _| {
        let i = me(g, c)?;
        i.timeline_running = false;
        i.timeline_position = 0.0;
        ok()
    });
    act!("action_message", |g, _, _, a, _| {
        let s = text_s(a, 0).replace("\\#", "\u{1}").replace('#', "\n").replace('\u{1}', "#");
        g.message(&s);
        ok()
    });
    act!("action_show_info", |g, _, _, _, _| {
        let t = crate::builtins::ui::rtf_to_text(&g.game_info.clone());
        g.message(&t);
        ok()
    });
    for n in ["action_show_video", "action_splash_web", "action_splash_video", "action_splash_settings"] {
        m.insert(n, |_, _, _, _, _| ok());
    }
    act!("action_splash_text", |g, vm, c, a, _| {
        let f = gml::builtins::function_id("splash_show_text").unwrap();
        vm.call_function(g, c, f, &[a.first().cloned().unwrap_or_default(), Value::Real(0.0)])
    });
    act!("action_splash_image", |g, vm, c, a, _| {
        let f = gml::builtins::function_id("splash_show_image").unwrap();
        vm.call_function(g, c, f, &[a.first().cloned().unwrap_or_default(), Value::Real(0.0)])
    });
    act!("action_restart_game", |g, _, _, _, _| {
        g.request_restart();
        ok()
    });
    act!("action_end_game", |g, _, _, _, _| {
        g.pending = Some(Pending::End);
        ok()
    });
    act!("action_save_game", |g, vm, _, a, _| {
        let p = g.resolve_path(&text_s(a, 0));
        crate::save::save_game(g, vm, &p.to_string_lossy()).map_err(Error::new)?;
        ok()
    });
    act!("action_load_game", |g, _, _, a, _| {
        let p = g.resolve_path(&text_s(a, 0));
        g.pending = Some(Pending::Load(p.to_string_lossy().into_owned()));
        ok()
    });
    act!("action_replace_sprite", |g, vm, c, a, _| {
        let f = gml::builtins::function_id("sprite_replace").unwrap();
        let args = [
            Value::Real(real(a, 0)),
            a.get(1).cloned().unwrap_or_default(),
            Value::Real(real(a, 2)),
            Value::Real(0.0),
            Value::Real(0.0),
            Value::Real(0.0),
            Value::Real(0.0),
        ];
        vm.call_function(g, c, f, &args)
    });
    act!("action_replace_sound", |g, vm, c, a, _| {
        let kind = g.sounds.get(int(a, 0).max(0) as usize).and_then(|s| s.as_ref()).map(|s| s.kind).unwrap_or(0);
        let f = gml::builtins::function_id("sound_replace").unwrap();
        let args = [Value::Real(real(a, 0)), a.get(1).cloned().unwrap_or_default(), Value::Real(kind as f64), Value::Real(1.0)];
        vm.call_function(g, c, f, &args)
    });
    act!("action_replace_background", |g, vm, c, a, _| {
        let f = gml::builtins::function_id("background_replace").unwrap();
        let args = [Value::Real(real(a, 0)), a.get(1).cloned().unwrap_or_default(), Value::Real(0.0), Value::Real(0.0)];
        vm.call_function(g, c, f, &args)
    });
    // ------------------------------------------------------------------ control
    act!("action_if_empty", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 0), real(a, 1));
        b(if int(a, 2) == 1 { g.place_empty(c.self_id, x, y) } else { g.place_free(c.self_id, x, y) })
    });
    act!("action_if_collision", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 0), real(a, 1));
        b(!(if int(a, 2) == 1 { g.place_empty(c.self_id, x, y) } else { g.place_free(c.self_id, x, y) }))
    });
    act!("action_if_object", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 1), real(a, 2));
        let s = g.sel(c, int(a, 0));
        b(g.place_meeting_sel(c.self_id, x, y, s).is_some())
    });
    act!("action_if_number", |g, _, c, a, _| {
        let n = g.select_ctx(c, int(a, 0)).len() as f64;
        b(compare(n, real(a, 1), int(a, 2)))
    });
    act!("action_if_dice", |_, vm, _, a, _| {
        let s = real(a, 0);
        b(s <= 1.0 || vm.random() * s < 1.0)
    });
    act!("action_if_question", |g, _, _, a, _| {
        let s = text_s(a, 0).replace('#', "\n");
        let r = g.ui.question(&s);
        g.input.clear_all();
        b(r)
    });
    act!("action_if", |_, _, _, a, _| b(crate::actions::truthy(&a.first().cloned().unwrap_or_default())));
    act!("action_if_mouse", |g, _, _, a, _| {
        let btn = int(a, 0);
        b(g.input.mouse_check(&g.input.mb_held, btn))
    });
    act!("action_if_aligned", |g, _, c, a, _| {
        let (h, v) = (real(a, 0), real(a, 1));
        let i = me(g, c)?;
        let okx = h <= 0.0 || (i.x - (i.x / h).round() * h).abs() < 1e-6;
        let oky = v <= 0.0 || (i.y - (i.y / v).round() * v).abs() < 1e-6;
        b(okx && oky)
    });
    act!("action_inherited", |g, vm, c, _, _| {
        g.event_inherited(vm, c)?;
        ok()
    });
    act!("action_execute_script", |g, vm, c, a, rel| {
        let s = int(a, 0);
        let prog = g
            .scripts
            .get(s.max(0) as usize)
            .and_then(|x| x.as_ref())
            .and_then(|x| x.prog.clone())
            .ok_or_else(|| Error::new(format!("Script does not exist: {}", s)))?;
        let ctx = Ctx {
            self_id: c.self_id,
            other_id: c.other_id,
            argument_relative: rel,
        };
        let args: Vec<Value> = a.iter().skip(1).take(5).cloned().collect();
        vm.run(g, &prog, ctx, &args)?;
        ok()
    });
    act!("action_if_variable", |_, _, _, a, _| {
        let (x, y) = (a.first().cloned().unwrap_or_default(), a.get(1).cloned().unwrap_or_default());
        b(compare_v(&x, &y, int(a, 2)))
    });
    act!("action_draw_variable", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 1), real(a, 2));
        let s = text(a, 0);
        draw_text_at(g, x, y, &s);
        ok()
    });
    // ------------------------------------------------------------------ score
    act!("action_set_score", |g, _, _, a, rel| {
        g.score = if rel { g.score + real(a, 0) } else { real(a, 0) };
        g.window_dirty = true;
        ok()
    });
    act!("action_if_score", |g, _, _, a, _| b(compare(g.score, real(a, 0), int(a, 1))));
    act!("action_draw_score", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 0), real(a, 1));
        let mut s = text(a, 2);
        s.extend(gml::real_to_string(g.score).into_bytes());
        draw_text_at(g, x, y, &s);
        ok()
    });
    act!("action_highscore_show", |g, _, _, a, _| {
        let (font, size, _, _) = parse_font_string(&text_s(a, 4));
        let mut style = g.hs_style.clone();
        style.background = int(a, 0);
        style.border = int(a, 1) == 1;
        style.new_col = int(a, 2);
        style.other_col = int(a, 3);
        style.font = font;
        style.size = size;
        let sc = g.score;
        g.highscore_show(sc, style);
        ok()
    });
    act!("action_highscore_clear", |g, _, _, _, _| {
        g.load_highscores();
        g.highscores.clear();
        g.save_highscores();
        ok()
    });
    act!("action_set_life", |g, vm, _, a, rel| {
        let v = if rel { g.lives + real(a, 0) } else { real(a, 0) };
        g.window_dirty = true;
        g.set_lives(vm, v)?;
        ok()
    });
    act!("action_if_life", |g, _, _, a, _| b(compare(g.lives, real(a, 0), int(a, 1))));
    act!("action_draw_life", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 0), real(a, 1));
        let mut s = text(a, 2);
        s.extend(gml::real_to_string(g.lives).into_bytes());
        draw_text_at(g, x, y, &s);
        ok()
    });
    act!("action_draw_life_images", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 0), real(a, 1));
        let spr = int(a, 2);
        let w = g.sprite(spr).map(|s| s.width as f64).unwrap_or(0.0);
        for k in 0..(g.lives.max(0.0) as i32) {
            g.draw_sprite_ext(spr, 0.0, x + k as f64 * w, y, 1.0, 1.0, 0.0, 0xFFFFFF, 1.0);
        }
        ok()
    });
    act!("action_set_health", |g, vm, _, a, rel| {
        let v = if rel { g.health + real(a, 0) } else { real(a, 0) };
        g.window_dirty = true;
        g.set_health(vm, v)?;
        ok()
    });
    act!("action_if_health", |g, _, _, a, _| b(compare(g.health, real(a, 0), int(a, 1))));
    act!("action_draw_health", |g, vm, c, a, rel| {
        let (x1, y1) = pos(g, c, rel, real(a, 0), real(a, 1));
        let (x2, y2) = pos(g, c, rel, real(a, 2), real(a, 3));
        let back = int(a, 4);
        let bar = int(a, 5);
        let (show_back, backcol) = if back <= 0 { (false, 0) } else { (true, color_const(MENU_COLORS[(back - 1).clamp(0, 15) as usize])) };
        let (minc, maxc) = match bar {
            0 => (color_const("c_red"), color_const("c_green")),
            1 => (color_const("c_black"), color_const("c_white")),
            k => {
                let c_ = color_const(MENU_COLORS[(k - 2).clamp(0, 15) as usize]);
                (c_, c_)
            }
        };
        let f = gml::builtins::function_id("draw_healthbar").unwrap();
        let args = [x1, y1, x2, y2, g.health, backcol as f64, minc as f64, maxc as f64, 0.0, show_back as i32 as f64, 1.0]
            .map(Value::Real);
        vm.call_function(g, c, f, &args)
    });
    act!("action_set_caption", |g, _, _, a, _| {
        g.show_score = int(a, 0) == 1;
        g.caption_score = text_s(a, 1);
        g.show_lives = int(a, 2) == 1;
        g.caption_lives = text_s(a, 3);
        g.show_health = int(a, 4) == 1;
        g.caption_health = text_s(a, 5);
        g.window_dirty = true;
        ok()
    });
    // ------------------------------------------------------------------ particles
    act!("action_partsyst_create", |g, _, _, a, _| {
        let s = part_sys(g);
        g.particles.sys(s).unwrap().depth = real(a, 0);
        ok()
    });
    act!("action_partsyst_destroy", |g, _, _, _, _| {
        let s = g.particles.action_system;
        if s >= 0 {
            if let Some(x) = g.particles.systems.get_mut(s as usize) {
                *x = None;
            }
        }
        g.particles.action_system = -1;
        ok()
    });
    act!("action_partsyst_clear", |g, _, _, _, _| {
        let s = g.particles.action_system;
        if let Some(x) = g.particles.sys(s) {
            x.particles.clear();
        }
        ok()
    });
    act!("action_parttype_create_old", |g, _, _, a, _| {
        let t = part_type(g, int(a, 0));
        let p = g.particles.ty(t).unwrap();
        p.shape = int(a, 1);
        p.size_min = real(a, 2);
        p.size_max = real(a, 3);
        p.colors = [int(a, 4), int(a, 5), int(a, 5)];
        p.color_mode = 1;
        ok()
    });
    act!("action_parttype_create", |g, _, _, a, _| {
        let t = part_type(g, int(a, 0));
        let p = g.particles.ty(t).unwrap();
        p.shape = int(a, 1);
        p.sprite = int(a, 2);
        p.size_min = real(a, 3);
        p.size_max = real(a, 4);
        p.size_incr = real(a, 5);
        ok()
    });
    act!("action_parttype_color", |g, _, _, a, _| {
        let t = part_type(g, int(a, 0));
        let p = g.particles.ty(t).unwrap();
        p.color_mode = if int(a, 1) == 0 { 3 } else { 1 };
        p.colors = [int(a, 2), int(a, 3), int(a, 3)];
        p.alphas = [real(a, 4), real(a, 5), real(a, 5)];
        p.alpha_count = 2;
        ok()
    });
    act!("action_parttype_life", |g, _, _, a, _| {
        let t = part_type(g, int(a, 0));
        let p = g.particles.ty(t).unwrap();
        p.life_min = real(a, 1);
        p.life_max = real(a, 2);
        ok()
    });
    act!("action_parttype_speed", |g, _, _, a, _| {
        let t = part_type(g, int(a, 0));
        let p = g.particles.ty(t).unwrap();
        p.speed_min = real(a, 1);
        p.speed_max = real(a, 2);
        p.dir_min = real(a, 3);
        p.dir_max = real(a, 4);
        p.speed_incr = -real(a, 5);
        ok()
    });
    act!("action_parttype_gravity", |g, _, _, a, _| {
        let t = part_type(g, int(a, 0));
        let p = g.particles.ty(t).unwrap();
        p.grav_amount = real(a, 1);
        p.grav_dir = real(a, 2);
        ok()
    });
    act!("action_parttype_secondary", |g, _, _, a, _| {
        let t = part_type(g, int(a, 0));
        let st = part_type(g, int(a, 1));
        let dt = part_type(g, int(a, 3));
        let p = g.particles.ty(t).unwrap();
        p.step_type = st;
        p.step_number = int(a, 2);
        p.death_type = dt;
        p.death_number = int(a, 4);
        ok()
    });
    act!("action_partemit_create", |g, _, c, a, rel| {
        let s = part_sys(g);
        let k = int(a, 0).clamp(0, 7) as usize;
        let mut e = g.particles.action_emitters[k];
        let exists = g.particles.sys(s).map(|x| e >= 0 && x.emitters.get(e as usize).map(|y| y.is_some()).unwrap_or(false)).unwrap_or(false);
        if !exists {
            e = g.particles.emitter_create(s);
            g.particles.action_emitters[k] = e;
        }
        let (x1, y1) = pos(g, c, rel, real(a, 2), real(a, 4));
        let (x2, y2) = pos(g, c, rel, real(a, 3), real(a, 5));
        if let Some(Some(em)) = g.particles.sys(s).unwrap().emitters.get_mut(e as usize) {
            em.xmin = x1;
            em.xmax = x2;
            em.ymin = y1;
            em.ymax = y2;
            em.shape = int(a, 1);
        }
        ok()
    });
    act!("action_partemit_destroy", |g, _, _, a, _| {
        let s = g.particles.action_system;
        let k = int(a, 0).clamp(0, 7) as usize;
        let e = g.particles.action_emitters[k];
        if let Some(x) = g.particles.sys(s) {
            if let Some(y) = x.emitters.get_mut(e.max(0) as usize) {
                *y = None;
            }
        }
        g.particles.action_emitters[k] = -1;
        ok()
    });
    act!("action_partemit_burst", |g, _, _, a, _| {
        let s = part_sys(g);
        let e = g.particles.action_emitters[int(a, 0).clamp(0, 7) as usize];
        let t = part_type(g, int(a, 1));
        g.particles.emitter_burst(s, e, t, int(a, 2));
        ok()
    });
    act!("action_partemit_stream", |g, _, _, a, _| {
        let s = part_sys(g);
        let e = g.particles.action_emitters[int(a, 0).clamp(0, 7) as usize];
        let t = part_type(g, int(a, 1));
        if let Some(Some(em)) = g.particles.sys(s).unwrap().emitters.get_mut(e.max(0) as usize) {
            em.stream_type = t;
            em.stream_number = int(a, 2);
        }
        ok()
    });
    // ------------------------------------------------------------------ extra
    for n in ["action_cd_play", "action_cd_stop", "action_cd_pause", "action_cd_resume", "action_cd_present", "action_cd_playing"] {
        m.insert(n, |_, _, _, _, _| ok());
    }
    act!("action_set_cursor", |g, _, _, a, _| {
        g.cursor_sprite = int(a, 0);
        g.cursor = if int(a, 1) == 1 { 0 } else { -1 };
        g.window_dirty = true;
        ok()
    });
    act!("action_webpage", |_, _, _, a, _| {
        let url = text_s(a, 0);
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", &url]).spawn();
        ok()
    });
    // ------------------------------------------------------------------ draw
    act!("action_draw_sprite", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 1), real(a, 2));
        let sub = real(a, 3);
        let sub = if sub < 0.0 { g.inst(c.self_id).map(|i| i.image_index).unwrap_or(0.0) } else { sub };
        g.draw_sprite_ext(int(a, 0), sub, x, y, 1.0, 1.0, 0.0, 0xFFFFFF, 1.0);
        ok()
    });
    act!("action_draw_background", |g, vm, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 1), real(a, 2));
        let name = if int(a, 3) == 1 { "draw_background_tiled" } else { "draw_background" };
        let f = gml::builtins::function_id(name).unwrap();
        vm.call_function(g, c, f, &[Value::Real(real(a, 0)), Value::Real(x), Value::Real(y)])
    });
    act!("action_draw_text", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 1), real(a, 2));
        let s = text(a, 0);
        draw_text_at(g, x, y, &s);
        ok()
    });
    act!("action_draw_text_transformed", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 1), real(a, 2));
        let s = text(a, 0);
        let (col, al) = (g.gfx.color, g.gfx.alpha);
        g.draw_text_full(x, y, &s, -1.0, -1.0, real(a, 3), real(a, 4), real(a, 5), [col; 4], al);
        ok()
    });
    fn shape(g: &mut Game, vm: &mut Vm, c: &Ctx, a: &[Value], rel: bool, name: &str, extra: &[f64]) -> R {
        let (x1, y1) = pos(g, c, rel, real(a, 0), real(a, 1));
        let (x2, y2) = pos(g, c, rel, real(a, 2), real(a, 3));
        let mut args = vec![Value::Real(x1), Value::Real(y1), Value::Real(x2), Value::Real(y2)];
        args.extend(extra.iter().map(|v| Value::Real(*v)));
        let f = gml::builtins::function_id(name).unwrap();
        vm.call_function(g, c, f, &args)
    }
    act!("action_draw_rectangle", |g, vm, c, a, rel| {
        let o = (int(a, 4) == 1) as i32 as f64;
        shape(g, vm, c, a, rel, "draw_rectangle", &[o])
    });
    act!("action_draw_gradient_hor", |g, vm, c, a, rel| {
        let (c1, c2) = (real(a, 4), real(a, 5));
        shape(g, vm, c, a, rel, "draw_rectangle_color", &[c1, c2, c2, c1, 0.0])
    });
    act!("action_draw_gradient_vert", |g, vm, c, a, rel| {
        let (c1, c2) = (real(a, 4), real(a, 5));
        shape(g, vm, c, a, rel, "draw_rectangle_color", &[c1, c1, c2, c2, 0.0])
    });
    act!("action_draw_ellipse", |g, vm, c, a, rel| {
        let o = (int(a, 4) == 1) as i32 as f64;
        shape(g, vm, c, a, rel, "draw_ellipse", &[o])
    });
    act!("action_draw_ellipse_gradient", |g, vm, c, a, rel| {
        let (c1, c2) = (real(a, 4), real(a, 5));
        shape(g, vm, c, a, rel, "draw_ellipse_color", &[c1, c2, 0.0])
    });
    act!("action_draw_line", |g, vm, c, a, rel| shape(g, vm, c, a, rel, "draw_line", &[]));
    act!("action_draw_arrow", |g, vm, c, a, rel| {
        let s = real(a, 4);
        shape(g, vm, c, a, rel, "draw_arrow", &[s])
    });
    act!("action_color", |g, _, _, a, _| {
        g.gfx.color = int(a, 0) & 0xFFFFFF;
        ok()
    });
    act!("action_font", |g, _, _, a, _| {
        g.gfx.font = int(a, 0);
        g.gfx.halign = int(a, 1);
        ok()
    });
    act!("action_fullscreen", |g, _, _, a, _| {
        g.fullscreen = match int(a, 0) {
            1 => false,
            2 => true,
            _ => !g.fullscreen,
        };
        g.window_dirty = true;
        ok()
    });
    act!("action_snapshot", |g, _, _, a, _| {
        let p = g.resolve_path(&text_s(a, 0));
        g.gfx.read_back().save_png(&p, true).map_err(|e| Error::new(e.to_string()))?;
        ok()
    });
    act!("action_effect", |g, _, c, a, rel| {
        let (x, y) = pos(g, c, rel, real(a, 1), real(a, 2));
        g.particles.effect(int(a, 0), x, y, int(a, 3), int(a, 4), int(a, 5) == 1);
        ok()
    });
    let _ = normalize_angle;
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtin_action_functions_exist() {
        let mut missing = Vec::new();
        for lib in gm_format::builtin_libraries() {
            for a in &lib.actions {
                if a.exe_type == 1 && !a.function_name.is_empty() && lookup(&a.function_name).is_none() {
                    missing.push(a.function_name.clone());
                }
            }
        }
        assert!(missing.is_empty(), "missing: {:?}", missing);
    }

    #[test]
    fn font_strings() {
        assert_eq!(parse_font_string("\"Times New Roman\",12,0,1,0,0,0"), ("Times New Roman".into(), 12, false, true));
        assert_eq!(parse_font_string("Arial,10,1,0"), ("Arial".into(), 10, true, false));
    }
}
