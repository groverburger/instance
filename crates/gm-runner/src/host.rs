//! `gml::Host` implementation: built-in variables (runtime.md §1, §7) and the
//! dispatch of world functions and scripts.

use crate::game::Game;
use gml::builtins::variables as v;
use gml::{Ctx, Error, FunctionId, Host, Value, VarTable, VariableId, Vm};

/// A host with no world, used to evaluate user constants at load time.
pub struct ConstHost;

impl Host for ConstHost {
    fn instance_vars(&mut self, _: i32) -> Option<&mut VarTable> {
        None
    }
    fn instance_exists(&mut self, _: i32) -> bool {
        false
    }
    fn select_instances(&mut self, _: i32, _: &mut Vec<i32>) {}
    fn get_builtin(&mut self, _: &Ctx, _: i32, var: VariableId, _: i32) -> Result<Value, Error> {
        Err(Error::new(format!("{} cannot be used in a constant", var.name())))
    }
    fn set_builtin(&mut self, _: &Ctx, _: i32, var: VariableId, _: i32, _: Value) -> Result<(), Error> {
        Err(Error::new(format!("{} cannot be used in a constant", var.name())))
    }
    fn call_function(&mut self, _: &mut Vm, _: &Ctx, f: FunctionId, _: &[Value]) -> Result<Value, Error> {
        Err(Error::new(format!("{} cannot be used in a constant", f.name())))
    }
    fn call_script(&mut self, _: &mut Vm, _: &Ctx, _: i32, _: &[Value]) -> Result<Value, Error> {
        Err(Error::new("Scripts cannot be used in a constant"))
    }
}

/// A real built-in variable's new value. VERIFIED black-box (bb_lang type.*): a
/// string assigned to a real built-in stores 0 without an error; boolean built-ins
/// are set when the value is above 0.5 (`solid = 0.5` reads back 0, `visible = 2`
/// reads back 1).
fn real(var: VariableId, x: &Value) -> Result<f64, Error> {
    let _ = var;
    match x {
        Value::Real(r) => Ok(*r),
        Value::Str(_) => Ok(0.0),
    }
}

fn string(var: VariableId, x: &Value) -> Result<String, Error> {
    match x {
        Value::Str(s) => Ok(s.to_string_lossy()),
        Value::Real(_) => Err(Error::new(format!("Wrong type of value assigned to variable {}.", var.name()))),
    }
}

#[inline]
fn b(x: bool) -> Value {
    Value::Real(if x { 1.0 } else { 0.0 })
}

fn s(x: &str) -> Value {
    Value::Str(gm_format::text::encode_cp1252(x).into())
}

/// The current local date-time as a GML date value.
pub fn local_now() -> f64 {
    #[cfg(windows)]
    {
        #[repr(C)]
        #[derive(Default)]
        struct St {
            year: u16,
            month: u16,
            dow: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            ms: u16,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetLocalTime(st: *mut St);
        }
        let mut st = St::default();
        // SAFETY: GetLocalTime writes only the provided struct.
        unsafe { GetLocalTime(&mut st) };
        days_from_civil(st.year as i64, st.month as i64, st.day as i64) as f64
            + (st.hour as f64 * 3600.0 + st.minute as f64 * 60.0 + st.second as f64 + st.ms as f64 / 1000.0) / 86400.0
    }
    #[cfg(not(windows))]
    {
        let secs = gml::clock::unix_seconds() + gml::clock::local_offset_seconds();
        25569.0 + secs / 86400.0
    }
}

/// Days since 1899-12-30 of a civil date.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468 + 25569
}

/// Splits a GML date value into (year, month, day, weekday 1=Sunday, hour, minute, second).
pub fn date_parts(d: f64) -> (i64, i64, i64, i64, i64, i64, i64) {
    let days = d.floor() as i64;
    let mut secs = ((d - d.floor()) * 86400.0).round() as i64;
    let mut days = days;
    if secs >= 86400 {
        secs -= 86400;
        days += 1;
    }
    // days since 1899-12-30 → civil date
    let z = days - 25569 + 719468; // unix epoch offset then civil-from-days
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dd = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let wd = (days + 6).rem_euclid(7) + 1; // 1899-12-30 was a Saturday (7)
    (y, m, dd, wd, secs / 3600, (secs / 60) % 60, secs % 60)
}

impl Game {
    fn inst_var(&self, id: i32, var: VariableId) -> Result<Value, Error> {
        let Some(i) = self.inst(id) else {
            return Err(Error::new(format!("Unknown variable {} (instance {} does not exist)", var.name(), id)));
        };
        let spr = self.sprite(i.sprite_index);
        Ok(match var {
            v::X => Value::Real(i.x),
            v::Y => Value::Real(i.y),
            v::XPREVIOUS => Value::Real(i.xprevious),
            v::YPREVIOUS => Value::Real(i.yprevious),
            v::XSTART => Value::Real(i.xstart),
            v::YSTART => Value::Real(i.ystart),
            v::HSPEED => Value::Real(i.hspeed),
            v::VSPEED => Value::Real(i.vspeed),
            v::DIRECTION => Value::Real(i.direction),
            v::SPEED => Value::Real(i.speed),
            v::FRICTION => Value::Real(i.friction),
            v::GRAVITY => Value::Real(i.gravity),
            v::GRAVITY_DIRECTION => Value::Real(i.gravity_direction),
            v::PATH_INDEX => Value::Real(i.path_index as f64),
            v::PATH_POSITION => Value::Real(i.path_position),
            v::PATH_POSITIONPREVIOUS => Value::Real(i.path_positionprevious),
            v::PATH_SPEED => Value::Real(i.path_speed),
            v::PATH_SCALE => Value::Real(i.path_scale),
            v::PATH_ORIENTATION => Value::Real(i.path_orientation),
            v::PATH_ENDACTION => Value::Real(i.path_endaction as f64),
            v::OBJECT_INDEX => Value::Real(i.object_index as f64),
            v::ID => Value::Real(i.id as f64),
            v::SOLID => b(i.solid),
            v::PERSISTENT => b(i.persistent),
            v::MASK_INDEX => Value::Real(i.mask_index as f64),
            v::TIMELINE_INDEX => Value::Real(i.timeline_index as f64),
            v::TIMELINE_POSITION => Value::Real(i.timeline_position),
            v::TIMELINE_SPEED => Value::Real(i.timeline_speed),
            v::TIMELINE_RUNNING => b(i.timeline_running),
            v::TIMELINE_LOOP => b(i.timeline_loop),
            v::VISIBLE => b(i.visible),
            v::SPRITE_INDEX => Value::Real(i.sprite_index as f64),
            v::SPRITE_WIDTH => Value::Real(spr.map(|s| s.width as f64).unwrap_or(0.0) * i.image_xscale),
            v::SPRITE_HEIGHT => Value::Real(spr.map(|s| s.height as f64).unwrap_or(0.0) * i.image_yscale),
            v::SPRITE_XOFFSET => Value::Real(spr.map(|s| s.origin_x as f64).unwrap_or(0.0)),
            v::SPRITE_YOFFSET => Value::Real(spr.map(|s| s.origin_y as f64).unwrap_or(0.0)),
            v::IMAGE_NUMBER => Value::Real(spr.map(|s| s.frames.len() as f64).unwrap_or(0.0)),
            v::IMAGE_INDEX => Value::Real(i.image_index),
            v::IMAGE_SPEED => Value::Real(i.image_speed),
            v::DEPTH => Value::Real(i.depth),
            v::IMAGE_XSCALE => Value::Real(i.image_xscale),
            v::IMAGE_YSCALE => Value::Real(i.image_yscale),
            v::IMAGE_ANGLE => Value::Real(i.image_angle),
            v::IMAGE_ALPHA => Value::Real(i.image_alpha),
            v::IMAGE_BLEND => Value::Real(i.image_blend as f64),
            v::BBOX_LEFT => Value::Real(self.bbox_of(id).0),
            v::BBOX_TOP => Value::Real(self.bbox_of(id).1),
            v::BBOX_RIGHT => Value::Real(self.bbox_of(id).2),
            v::BBOX_BOTTOM => Value::Real(self.bbox_of(id).3),
            _ => return Err(Error::new(format!("Unknown built-in variable {}", var.name()))),
        })
    }

    fn set_inst_var(&mut self, id: i32, var: VariableId, index: i32, x: Value) -> Result<(), Error> {
        let Some(i) = self.inst_mut(id) else {
            return Err(Error::new(format!("Cannot assign to {}: instance {} does not exist", var.name(), id)));
        };
        match var {
            v::X => i.x = real(var, &x)?,
            v::Y => i.y = real(var, &x)?,
            v::XPREVIOUS => i.xprevious = real(var, &x)?,
            v::YPREVIOUS => i.yprevious = real(var, &x)?,
            v::XSTART => i.xstart = real(var, &x)?,
            v::YSTART => i.ystart = real(var, &x)?,
            v::HSPEED => i.set_hspeed(real(var, &x)?),
            v::VSPEED => i.set_vspeed(real(var, &x)?),
            v::DIRECTION => i.set_direction(real(var, &x)?),
            v::SPEED => i.set_speed(real(var, &x)?),
            v::FRICTION => i.friction = real(var, &x)?,
            v::GRAVITY => i.gravity = real(var, &x)?,
            v::GRAVITY_DIRECTION => i.gravity_direction = real(var, &x)?,
            v::PATH_POSITION => i.path_position = real(var, &x)?.clamp(0.0, 1.0),
            v::PATH_POSITIONPREVIOUS => i.path_positionprevious = real(var, &x)?,
            v::PATH_SPEED => i.path_speed = real(var, &x)?,
            v::PATH_SCALE => i.path_scale = real(var, &x)?,
            v::PATH_ORIENTATION => i.path_orientation = real(var, &x)?,
            v::PATH_ENDACTION => i.path_endaction = gml::to_int(real(var, &x)?) as i32,
            v::SOLID => i.solid = real(var, &x)? > 0.5,
            v::PERSISTENT => i.persistent = real(var, &x)? > 0.5,
            v::MASK_INDEX => i.mask_index = gml::to_int(real(var, &x)?) as i32,
            v::TIMELINE_INDEX => i.timeline_index = gml::to_int(real(var, &x)?) as i32,
            v::TIMELINE_POSITION => i.timeline_position = real(var, &x)?,
            v::TIMELINE_SPEED => i.timeline_speed = real(var, &x)?,
            v::TIMELINE_RUNNING => i.timeline_running = real(var, &x)? > 0.5,
            v::TIMELINE_LOOP => i.timeline_loop = real(var, &x)? > 0.5,
            v::VISIBLE => i.visible = real(var, &x)? > 0.5,
            v::SPRITE_INDEX => i.sprite_index = gml::to_int(real(var, &x)?) as i32,
            v::IMAGE_INDEX => i.image_index = real(var, &x)?,
            v::IMAGE_SPEED => i.image_speed = real(var, &x)?,
            v::DEPTH => i.depth = real(var, &x)?,
            v::IMAGE_XSCALE => i.image_xscale = real(var, &x)?,
            v::IMAGE_YSCALE => i.image_yscale = real(var, &x)?,
            v::IMAGE_ANGLE => i.image_angle = real(var, &x)?,
            v::IMAGE_ALPHA => i.image_alpha = real(var, &x)?,
            v::IMAGE_BLEND => i.image_blend = gml::to_int(real(var, &x)?) as i32,
            v::ALARM => {
                if (0..12).contains(&index) {
                    // VERIFIED black-box (bb_events ev.alarms.s01): alarms hold integers
                    // (1.5 reads back as 2, 0.5 as 0: round half to even)
                    i.alarm[index as usize] = gml::to_int(real(var, &x)?) as f64;
                }
            }
            _ => return Err(Error::new(format!("Cannot assign to the variable {}", var.name()))),
        }
        Ok(())
    }

    fn view_get(&self, var: VariableId, k: usize) -> Option<Value> {
        let vs = &self.rs.views[k.min(7)];
        Some(match var {
            v::VIEW_VISIBLE => b(vs.visible),
            v::VIEW_XVIEW => Value::Real(vs.xview),
            v::VIEW_YVIEW => Value::Real(vs.yview),
            v::VIEW_WVIEW => Value::Real(vs.wview),
            v::VIEW_HVIEW => Value::Real(vs.hview),
            v::VIEW_XPORT => Value::Real(vs.xport),
            v::VIEW_YPORT => Value::Real(vs.yport),
            v::VIEW_WPORT => Value::Real(vs.wport),
            v::VIEW_HPORT => Value::Real(vs.hport),
            v::VIEW_ANGLE => Value::Real(vs.angle),
            v::VIEW_HBORDER => Value::Real(vs.hborder),
            v::VIEW_VBORDER => Value::Real(vs.vborder),
            v::VIEW_HSPEED => Value::Real(vs.hspeed),
            v::VIEW_VSPEED => Value::Real(vs.vspeed),
            v::VIEW_OBJECT => Value::Real(vs.object as f64),
            _ => return None,
        })
    }

    fn bg_get(&self, var: VariableId, k: usize) -> Option<Value> {
        let bg = &self.rs.backgrounds[k.min(7)];
        let res = self.background(bg.index);
        Some(match var {
            v::BACKGROUND_VISIBLE => b(bg.visible),
            v::BACKGROUND_FOREGROUND => b(bg.foreground),
            v::BACKGROUND_INDEX => Value::Real(bg.index as f64),
            v::BACKGROUND_X => Value::Real(bg.x),
            v::BACKGROUND_Y => Value::Real(bg.y),
            v::BACKGROUND_WIDTH => Value::Real(res.map(|r| r.tex.w as f64).unwrap_or(0.0)),
            v::BACKGROUND_HEIGHT => Value::Real(res.map(|r| r.tex.h as f64).unwrap_or(0.0)),
            v::BACKGROUND_HTILED => b(bg.htiled),
            v::BACKGROUND_VTILED => b(bg.vtiled),
            v::BACKGROUND_XSCALE => Value::Real(bg.xscale),
            v::BACKGROUND_YSCALE => Value::Real(bg.yscale),
            v::BACKGROUND_HSPEED => Value::Real(bg.hspeed),
            v::BACKGROUND_VSPEED => Value::Real(bg.vspeed),
            v::BACKGROUND_BLEND => Value::Real(bg.blend as f64),
            v::BACKGROUND_ALPHA => Value::Real(bg.alpha),
            _ => return None,
        })
    }

    fn global_get(&mut self, var: VariableId, index: i32) -> Result<Value, Error> {
        let k = index.clamp(0, 7) as usize;
        if let Some(x) = self.view_get(var, k) {
            return Ok(x);
        }
        if let Some(x) = self.bg_get(var, k) {
            return Ok(x);
        }
        Ok(match var {
            v::INSTANCE_COUNT => Value::Real(self.instances.iter().filter(|i| i.active).count() as f64),
            v::INSTANCE_ID => {
                let list: Vec<i32> = self.instances.iter().filter(|i| i.active).map(|i| i.id).collect();
                match list.get(index.max(0) as usize) {
                    Some(&id) if index >= 0 => Value::Real(id as f64),
                    _ => return Err(Error::new(format!("instance_id index {} out of range", index))),
                }
            }
            v::ROOM_SPEED => Value::Real(self.rs.speed),
            v::FPS => Value::Real(self.fps),
            v::CURRENT_TIME => Value::Real(self.start_time.elapsed().as_millis() as f64),
            v::CURRENT_YEAR => Value::Real(date_parts(local_now()).0 as f64),
            v::CURRENT_MONTH => Value::Real(date_parts(local_now()).1 as f64),
            v::CURRENT_DAY => Value::Real(date_parts(local_now()).2 as f64),
            v::CURRENT_WEEKDAY => Value::Real(date_parts(local_now()).3 as f64),
            v::CURRENT_HOUR => Value::Real(date_parts(local_now()).4 as f64),
            v::CURRENT_MINUTE => Value::Real(date_parts(local_now()).5 as f64),
            v::CURRENT_SECOND => Value::Real(date_parts(local_now()).6 as f64),
            v::ROOM => Value::Real(self.room as f64),
            v::ROOM_FIRST => Value::Real(*self.room_order.first().unwrap_or(&-1) as f64),
            v::ROOM_LAST => Value::Real(*self.room_order.last().unwrap_or(&-1) as f64),
            v::ROOM_WIDTH => Value::Real(self.rs.width as f64),
            v::ROOM_HEIGHT => Value::Real(self.rs.height as f64),
            v::ROOM_CAPTION => s(&self.rs.caption),
            v::ROOM_PERSISTENT => b(self.rs.persistent),
            v::TRANSITION_KIND => Value::Real(self.transition_kind as f64),
            v::TRANSITION_STEPS => Value::Real(self.transition_steps as f64),
            v::TRANSITION_COLOR => Value::Real(self.transition_color as f64),
            v::SCORE => Value::Real(self.score),
            v::LIVES => Value::Real(self.lives),
            v::HEALTH => Value::Real(self.health),
            v::SHOW_SCORE => b(self.show_score),
            v::SHOW_LIVES => b(self.show_lives),
            v::SHOW_HEALTH => b(self.show_health),
            v::CAPTION_SCORE => s(&self.caption_score),
            v::CAPTION_LIVES => s(&self.caption_lives),
            v::CAPTION_HEALTH => s(&self.caption_health),
            v::EVENT_TYPE => Value::Real(self.ev.etype as f64),
            v::EVENT_NUMBER => Value::Real(self.ev.number as f64),
            v::EVENT_OBJECT => Value::Real(self.ev.object as f64),
            v::EVENT_ACTION => Value::Real(self.ev.action as f64),
            v::GAMEMAKER_PRO => b(true),
            v::GAMEMAKER_REGISTERED => b(true),
            v::GAMEMAKER_VERSION => Value::Real(810.0),
            v::ERROR_OCCURRED => b(self.error_occurred),
            v::ERROR_LAST => s(&self.error_last),
            v::DEBUG_MODE => b(self.debug.is_some()),
            v::KEYBOARD_KEY => Value::Real(self.input.keyboard_key as f64),
            v::KEYBOARD_LASTKEY => Value::Real(self.input.keyboard_lastkey as f64),
            v::KEYBOARD_LASTCHAR => s(&self.input.keyboard_lastchar),
            v::KEYBOARD_STRING => s(&self.input.keyboard_string),
            v::MOUSE_X => Value::Real(self.mouse_room_pos().0.floor()),
            v::MOUSE_Y => Value::Real(self.mouse_room_pos().1.floor()),
            v::MOUSE_BUTTON => Value::Real(self.input.mouse_button as f64),
            v::MOUSE_LASTBUTTON => Value::Real(self.input.mouse_lastbutton as f64),
            v::CURSOR_SPRITE => Value::Real(self.cursor_sprite as f64),
            v::BACKGROUND_COLOR => Value::Real(self.rs.bg_color as f64),
            v::BACKGROUND_SHOWCOLOR => b(self.rs.show_color),
            v::VIEW_ENABLED => b(self.rs.views_enabled),
            v::VIEW_CURRENT => Value::Real(self.rs.view_current as f64),
            v::GAME_ID => Value::Real(self.game_id as f64),
            v::WORKING_DIRECTORY => s(&self.working_directory),
            v::TEMP_DIRECTORY => s(&self.temp_directory),
            v::PROGRAM_DIRECTORY => s(&self.program_directory),
            v::SECURE_MODE => b(false),
            v::OS_TYPE => Value::Real(0.0),
            v::OS_DEVICE => Value::Real(0.0),
            v::DISPLAY_AA => Value::Real(0.0),
            _ => return Err(Error::new(format!("Unknown built-in variable {}", var.name()))),
        })
    }

    fn global_set(&mut self, var: VariableId, index: i32, x: Value) -> Result<(), Error> {
        let k = index.clamp(0, 7) as usize;
        let int = |x: &Value| -> Result<i32, Error> { Ok(gml::to_int(real(var, x)?) as i32) };
        {
            let vs = &mut self.rs.views[k];
            let done = match var {
                v::VIEW_VISIBLE => {
                    vs.visible = real(var, &x)? > 0.5;
                    true
                }
                v::VIEW_XVIEW => {
                    vs.xview = real(var, &x)?;
                    true
                }
                v::VIEW_YVIEW => {
                    vs.yview = real(var, &x)?;
                    true
                }
                v::VIEW_WVIEW => {
                    vs.wview = real(var, &x)?;
                    true
                }
                v::VIEW_HVIEW => {
                    vs.hview = real(var, &x)?;
                    true
                }
                v::VIEW_XPORT => {
                    vs.xport = real(var, &x)?;
                    true
                }
                v::VIEW_YPORT => {
                    vs.yport = real(var, &x)?;
                    true
                }
                v::VIEW_WPORT => {
                    vs.wport = real(var, &x)?;
                    true
                }
                v::VIEW_HPORT => {
                    vs.hport = real(var, &x)?;
                    true
                }
                v::VIEW_ANGLE => {
                    vs.angle = real(var, &x)?;
                    true
                }
                v::VIEW_HBORDER => {
                    vs.hborder = real(var, &x)?;
                    true
                }
                v::VIEW_VBORDER => {
                    vs.vborder = real(var, &x)?;
                    true
                }
                v::VIEW_HSPEED => {
                    vs.hspeed = real(var, &x)?;
                    true
                }
                v::VIEW_VSPEED => {
                    vs.vspeed = real(var, &x)?;
                    true
                }
                v::VIEW_OBJECT => {
                    vs.object = int(&x)?;
                    true
                }
                _ => false,
            };
            if done {
                return Ok(());
            }
        }
        {
            let bg = &mut self.rs.backgrounds[k];
            let done = match var {
                v::BACKGROUND_VISIBLE => {
                    bg.visible = real(var, &x)? > 0.5;
                    true
                }
                v::BACKGROUND_FOREGROUND => {
                    bg.foreground = real(var, &x)? > 0.5;
                    true
                }
                v::BACKGROUND_INDEX => {
                    bg.index = int(&x)?;
                    true
                }
                v::BACKGROUND_X => {
                    bg.x = real(var, &x)?;
                    true
                }
                v::BACKGROUND_Y => {
                    bg.y = real(var, &x)?;
                    true
                }
                v::BACKGROUND_HTILED => {
                    bg.htiled = real(var, &x)? > 0.5;
                    true
                }
                v::BACKGROUND_VTILED => {
                    bg.vtiled = real(var, &x)? > 0.5;
                    true
                }
                v::BACKGROUND_XSCALE => {
                    bg.xscale = real(var, &x)?;
                    true
                }
                v::BACKGROUND_YSCALE => {
                    bg.yscale = real(var, &x)?;
                    true
                }
                v::BACKGROUND_HSPEED => {
                    bg.hspeed = real(var, &x)?;
                    true
                }
                v::BACKGROUND_VSPEED => {
                    bg.vspeed = real(var, &x)?;
                    true
                }
                v::BACKGROUND_BLEND => {
                    bg.blend = int(&x)?;
                    true
                }
                v::BACKGROUND_ALPHA => {
                    bg.alpha = real(var, &x)?;
                    true
                }
                _ => false,
            };
            if done {
                return Ok(());
            }
        }
        match var {
            v::ROOM_SPEED => self.rs.speed = real(var, &x)?.max(1.0),
            v::ROOM => {
                let r = int(&x)?;
                if !self.room_exists(r) {
                    return Err(Error::new(format!("Unexisting room number: {}", r)));
                }
                self.request_goto(r);
            }
            v::ROOM_CAPTION => {
                self.rs.caption = string(var, &x)?;
                self.window_dirty = true;
            }
            v::ROOM_PERSISTENT => self.rs.persistent = real(var, &x)? > 0.5,
            v::TRANSITION_KIND => self.transition_kind = int(&x)?,
            v::TRANSITION_STEPS => self.transition_steps = int(&x)?,
            v::TRANSITION_COLOR => self.transition_color = int(&x)?,
            v::SCORE => {
                self.score = real(var, &x)?;
                self.window_dirty = true;
            }
            v::LIVES => {
                let nv = real(var, &x)?;
                if self.lives > 0.0 && nv <= 0.0 {
                    self.pending_lives_event = true;
                }
                self.lives = nv;
                self.window_dirty = true;
            }
            v::HEALTH => {
                let nv = real(var, &x)?;
                if self.health > 0.0 && nv <= 0.0 {
                    self.pending_health_event = true;
                }
                self.health = nv;
                self.window_dirty = true;
            }
            v::SHOW_SCORE => self.show_score = real(var, &x)? > 0.5,
            v::SHOW_LIVES => self.show_lives = real(var, &x)? > 0.5,
            v::SHOW_HEALTH => self.show_health = real(var, &x)? > 0.5,
            v::CAPTION_SCORE => self.caption_score = string(var, &x)?,
            v::CAPTION_LIVES => self.caption_lives = string(var, &x)?,
            v::CAPTION_HEALTH => self.caption_health = string(var, &x)?,
            v::ERROR_OCCURRED => self.error_occurred = real(var, &x)? > 0.5,
            v::ERROR_LAST => self.error_last = string(var, &x)?,
            v::KEYBOARD_KEY => self.input.keyboard_key = int(&x)?,
            v::KEYBOARD_LASTKEY => self.input.keyboard_lastkey = int(&x)?,
            v::KEYBOARD_LASTCHAR => self.input.keyboard_lastchar = string(var, &x)?,
            v::KEYBOARD_STRING => self.input.keyboard_string = string(var, &x)?,
            v::MOUSE_BUTTON => self.input.mouse_button = int(&x)?,
            v::MOUSE_LASTBUTTON => self.input.mouse_lastbutton = int(&x)?,
            v::CURSOR_SPRITE => self.cursor_sprite = int(&x)?,
            v::BACKGROUND_COLOR => self.rs.bg_color = int(&x)?,
            v::BACKGROUND_SHOWCOLOR => self.rs.show_color = real(var, &x)? > 0.5,
            v::VIEW_ENABLED => self.rs.views_enabled = real(var, &x)? > 0.5,
            _ => return Err(Error::new(format!("Cannot assign to the variable {}", var.name()))),
        }
        Ok(())
    }
}

impl Host for Game {
    fn resolve_constant(&self, name: &str) -> Option<Value> {
        self.constants.get(name).cloned()
    }

    fn resolve_script(&self, name: &str) -> Option<i32> {
        self.script_names.get(name).copied().or_else(|| self.ext.resolve(name))
    }

    fn instance_vars(&mut self, id: i32) -> Option<&mut VarTable> {
        match self.inst_index.get(&id) {
            Some(&k) => Some(&mut self.instances[k].vars),
            None => None,
        }
    }

    fn builtin_real_mut(&mut self, inst: i32, var: VariableId) -> Option<&mut f64> {
        // only variables whose write is a plain store (see set_inst_var); the
        // instance is marked for the collision index by inst_mut
        if !matches!(
            var,
            v::X | v::Y
                | v::XPREVIOUS
                | v::YPREVIOUS
                | v::XSTART
                | v::YSTART
                | v::FRICTION
                | v::GRAVITY
                | v::GRAVITY_DIRECTION
                | v::PATH_SPEED
                | v::PATH_SCALE
                | v::PATH_ORIENTATION
                | v::IMAGE_INDEX
                | v::IMAGE_SPEED
                | v::DEPTH
                | v::IMAGE_XSCALE
                | v::IMAGE_YSCALE
                | v::IMAGE_ANGLE
                | v::IMAGE_ALPHA
        ) {
            return None;
        }
        let i = self.inst_mut(inst)?;
        Some(match var {
            v::X => &mut i.x,
            v::Y => &mut i.y,
            v::XPREVIOUS => &mut i.xprevious,
            v::YPREVIOUS => &mut i.yprevious,
            v::XSTART => &mut i.xstart,
            v::YSTART => &mut i.ystart,
            v::FRICTION => &mut i.friction,
            v::GRAVITY => &mut i.gravity,
            v::GRAVITY_DIRECTION => &mut i.gravity_direction,
            v::PATH_SPEED => &mut i.path_speed,
            v::PATH_SCALE => &mut i.path_scale,
            v::PATH_ORIENTATION => &mut i.path_orientation,
            v::IMAGE_INDEX => &mut i.image_index,
            v::IMAGE_SPEED => &mut i.image_speed,
            v::DEPTH => &mut i.depth,
            v::IMAGE_XSCALE => &mut i.image_xscale,
            v::IMAGE_YSCALE => &mut i.image_yscale,
            v::IMAGE_ANGLE => &mut i.image_angle,
            _ => &mut i.image_alpha,
        })
    }

    fn builtin_real(&mut self, inst: i32, var: VariableId) -> Option<f64> {
        let i = self.inst(inst)?;
        Some(match var {
            v::X => i.x,
            v::Y => i.y,
            v::XPREVIOUS => i.xprevious,
            v::YPREVIOUS => i.yprevious,
            v::XSTART => i.xstart,
            v::YSTART => i.ystart,
            v::HSPEED => i.hspeed,
            v::VSPEED => i.vspeed,
            v::DIRECTION => i.direction,
            v::SPEED => i.speed,
            v::FRICTION => i.friction,
            v::GRAVITY => i.gravity,
            v::GRAVITY_DIRECTION => i.gravity_direction,
            v::IMAGE_INDEX => i.image_index,
            v::IMAGE_SPEED => i.image_speed,
            v::DEPTH => i.depth,
            v::IMAGE_XSCALE => i.image_xscale,
            v::IMAGE_YSCALE => i.image_yscale,
            v::IMAGE_ANGLE => i.image_angle,
            v::IMAGE_ALPHA => i.image_alpha,
            _ => return None,
        })
    }

    fn instance_exists(&mut self, id: i32) -> bool {
        self.inst(id).map(|i| i.alive()).unwrap_or(false)
    }

    fn select_instances(&mut self, target: i32, out: &mut Vec<i32>) {
        // `with (id)` of an instance changed this step (see Game::change_instance)
        if !self.changed_ids.is_empty() && self.changed_ids.contains(&target) {
            return;
        }
        self.select(target, out);
    }

    fn first_instance(&mut self, target: i32) -> Option<i32> {
        self.first_of(target)
    }

    fn get_builtin(&mut self, ctx: &Ctx, inst: i32, var: VariableId, index: i32) -> Result<Value, Error> {
        use gml::builtins::VarScope;
        match var.info().scope {
            VarScope::Instance => {
                // VERIFIED black-box (bb_instances destroy.read_after vs
                // with.destroy_self_then_read): a destroyed instance's variables can
                // still be read as `self`, but `u.x` from elsewhere is the non-fatal
                // error "Unknown variable x".
                if inst != ctx.self_id && self.inst(inst).map(|i| i.destroyed).unwrap_or(false) {
                    return Err(Error::new(format!("Unknown variable {}", var.name())));
                }
                if var == v::ALARM {
                    let Some(i) = self.inst(inst) else {
                        return Err(Error::new(format!("Unknown variable alarm (instance {} does not exist)", inst)));
                    };
                    return Ok(Value::Real(i.alarm[index.clamp(0, 11) as usize]));
                }
                self.inst_var(inst, var)
            }
            _ => self.global_get(var, index),
        }
    }

    fn set_builtin(&mut self, _ctx: &Ctx, inst: i32, var: VariableId, index: i32, value: Value) -> Result<(), Error> {
        use gml::builtins::VarScope;
        match var.info().scope {
            VarScope::Instance => self.set_inst_var(inst, var, index, value),
            _ => self.global_set(var, index, value),
        }
    }

    fn call_function(&mut self, vm: &mut Vm, ctx: &Ctx, func: FunctionId, args: &[Value]) -> Result<Value, Error> {
        if self.pending_lives_event || self.pending_health_event {
            self.flush_edge_events(vm)?;
        }
        crate::builtins::dispatch(self, vm, ctx, func, args)
    }

    fn call_script(&mut self, vm: &mut Vm, ctx: &Ctx, script: i32, args: &[Value]) -> Result<Value, Error> {
        // VERIFIED black-box (bb_events ev.s06+1 / ev.s07+1): No More Lives / Health
        // run at the assignment, before the next statement of the code. set_builtin
        // has no VM, so they are dispatched at the next host call (or after the
        // action).
        if self.pending_lives_event || self.pending_health_event {
            self.flush_edge_events(vm)?;
        }
        if script >= crate::ext::EXT_SCRIPT_BASE {
            return crate::ext::call(self, vm, ctx, (script - crate::ext::EXT_SCRIPT_BASE) as usize, args);
        }
        let prog = match self.scripts.get(script.max(0) as usize).and_then(|s| s.as_ref()) {
            Some(s) if script >= 0 => match &s.prog {
                Some(p) => p.clone(),
                None => {
                    return Err(Error::new(
                        s.error.clone().unwrap_or_else(|| format!("Script {} failed to compile", s.name)),
                    ))
                }
            },
            _ => return Err(Error::new(format!("Unknown script index {}", script))),
        };
        if self.prof.is_some() {
            let t0 = web_time::Instant::now();
            let r = vm.run_script(self, &prog, *ctx, args);
            crate::profile::Profiler::add(&mut self.prof.as_mut().unwrap().scripts, script, t0);
            return r;
        }
        vm.run_script(self, &prog, *ctx, args)
    }

    fn soft_error(&mut self, e: Error) -> Result<(), Error> {
        let action = self.ev.action;
        self.report_error(e, action)
    }

    fn read_file(&mut self, path: &str) -> Result<Vec<u8>, Error> {
        let p = self.resolve_path(path);
        crate::vfs::read(&p).map_err(|e| Error::new(format!("Cannot read file {}: {}", path, e)))
    }
}

impl Game {
    /// Resolves a file name relative to the working directory.
    pub fn resolve_path(&self, path: &str) -> std::path::PathBuf {
        let p = std::path::Path::new(path);
        if p.is_absolute() || self.working_directory.is_empty() {
            p.to_path_buf()
        } else {
            std::path::Path::new(&self.working_directory).join(p)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        // 2009-09-12 11:39 (Saturday)
        let (y, m, d, wd, h, mi, _) = date_parts(40068.4855);
        assert_eq!((y, m, d, h, mi), (2009, 9, 12, 11, 39));
        assert_eq!(wd, 7);
    }
}
