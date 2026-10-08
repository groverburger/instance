//! game_save / game_load (runtime.md §3.5). The file format is
//! implementation-defined: a small binary dump of the "basic game data".

use crate::game::Game;
use crate::instance::Instance;
use gml::{Value, Var, VarTable, Vm};

const MAGIC: &[u8; 8] = b"GM81RSV1";

struct W(Vec<u8>);

impl W {
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn bool(&mut self, v: bool) {
        self.0.push(v as u8);
    }
    fn bytes(&mut self, b: &[u8]) {
        self.i32(b.len() as i32);
        self.0.extend_from_slice(b);
    }
    fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
    fn val(&mut self, v: &Value) {
        match v {
            Value::Real(r) => {
                self.0.push(0);
                self.f64(*r);
            }
            Value::Str(s) => {
                self.0.push(1);
                self.bytes(s.as_bytes());
            }
        }
    }
    fn vars(&mut self, vm: &Vm, t: &VarTable) {
        self.i32(t.len() as i32);
        for (id, var) in t.iter() {
            self.str(vm.var_name(id));
            match var {
                Var::Scalar(v) => {
                    self.0.push(0);
                    self.val(v);
                }
                Var::Array(a) => {
                    self.0.push(1);
                    self.i32(a.rows().len() as i32);
                    for row in a.rows() {
                        self.i32(row.len() as i32);
                        for v in row {
                            self.val(v);
                        }
                    }
                }
            }
        }
    }
}

struct R<'a>(&'a [u8], usize);

impl R<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        if self.1 + n > self.0.len() {
            return Err("Save file is corrupt".into());
        }
        let s = &self.0[self.1..self.1 + n];
        self.1 += n;
        Ok(s)
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn bool(&mut self) -> Result<bool, String> {
        Ok(self.take(1)?[0] != 0)
    }
    fn bytes(&mut self) -> Result<Vec<u8>, String> {
        let n = self.i32()?.max(0) as usize;
        Ok(self.take(n)?.to_vec())
    }
    fn str(&mut self) -> Result<String, String> {
        Ok(String::from_utf8_lossy(&self.bytes()?).into_owned())
    }
    fn val(&mut self) -> Result<Value, String> {
        match self.take(1)?[0] {
            0 => Ok(Value::Real(self.f64()?)),
            _ => Ok(Value::Str(self.bytes()?.into())),
        }
    }
    fn vars(&mut self, vm: &mut Vm) -> Result<VarTable, String> {
        let mut t = VarTable::new();
        let n = self.i32()?;
        for _ in 0..n {
            let name = self.str()?;
            let id = vm.intern(&name);
            match self.take(1)?[0] {
                0 => t.set_value(id, self.val()?),
                _ => {
                    let rows = self.i32()?;
                    for i in 0..rows {
                        let cols = self.i32()?;
                        for j in 0..cols {
                            let v = self.val()?;
                            let _ = t.set_elem(id, i as usize, j as usize, v);
                        }
                    }
                }
            }
        }
        Ok(t)
    }
}

fn write_inst(w: &mut W, vm: &Vm, i: &Instance) {
    w.i32(i.id);
    w.i32(i.object_index);
    for v in [
        i.x, i.y, i.xprevious, i.yprevious, i.xstart, i.ystart, i.hspeed, i.vspeed, i.speed, i.direction,
        i.friction, i.gravity, i.gravity_direction, i.depth, i.image_index, i.image_speed, i.image_xscale,
        i.image_yscale, i.image_angle, i.image_alpha, i.timeline_position, i.timeline_speed, i.path_position,
        i.path_positionprevious, i.path_speed, i.path_scale, i.path_orientation, i.path_xstart, i.path_ystart,
        i.path_anchor_x, i.path_anchor_y,
    ] {
        w.f64(v);
    }
    for a in i.alarm {
        w.f64(a);
    }
    for b in [i.solid, i.visible, i.persistent, i.timeline_running, i.timeline_loop, i.active] {
        w.bool(b);
    }
    for v in [i.sprite_index, i.mask_index, i.image_blend, i.timeline_index, i.path_index, i.path_endaction] {
        w.i32(v);
    }
    w.vars(vm, &i.vars);
}

fn read_inst(r: &mut R, vm: &mut Vm) -> Result<Instance, String> {
    let id = r.i32()?;
    let obj = r.i32()?;
    let mut i = Instance::new(id, obj, 0.0, 0.0);
    let mut f = [0.0; 31];
    for x in f.iter_mut() {
        *x = r.f64()?;
    }
    [
        i.x, i.y, i.xprevious, i.yprevious, i.xstart, i.ystart, i.hspeed, i.vspeed, i.speed, i.direction,
        i.friction, i.gravity, i.gravity_direction, i.depth, i.image_index, i.image_speed, i.image_xscale,
        i.image_yscale, i.image_angle, i.image_alpha, i.timeline_position, i.timeline_speed, i.path_position,
        i.path_positionprevious, i.path_speed, i.path_scale, i.path_orientation, i.path_xstart, i.path_ystart,
        i.path_anchor_x, i.path_anchor_y,
    ] = f;
    for a in i.alarm.iter_mut() {
        *a = r.f64()?;
    }
    i.solid = r.bool()?;
    i.visible = r.bool()?;
    i.persistent = r.bool()?;
    i.timeline_running = r.bool()?;
    i.timeline_loop = r.bool()?;
    i.active = r.bool()?;
    i.sprite_index = r.i32()?;
    i.mask_index = r.i32()?;
    i.image_blend = r.i32()?;
    i.timeline_index = r.i32()?;
    i.path_index = r.i32()?;
    i.path_endaction = r.i32()?;
    i.vars = r.vars(vm)?;
    Ok(i)
}

/// Writes the save file.
pub fn save_game(g: &Game, vm: &Vm, path: &str) -> Result<(), String> {
    let mut w = W(Vec::new());
    w.0.extend_from_slice(MAGIC);
    w.i32(g.game_id);
    w.i32(g.room);
    for v in [g.score, g.lives, g.health, g.rs.speed] {
        w.f64(v);
    }
    w.bool(g.show_score);
    w.bool(g.show_lives);
    w.bool(g.show_health);
    w.str(&g.caption_score);
    w.str(&g.caption_lives);
    w.str(&g.caption_health);
    w.i32(g.next_instance_id);
    w.vars(vm, vm.globals());
    // room state essentials
    w.str(&g.rs.caption);
    w.i32(g.rs.bg_color);
    w.bool(g.rs.show_color);
    for b in &g.rs.backgrounds {
        w.bool(b.visible);
        w.bool(b.foreground);
        w.i32(b.index);
        for v in [b.x, b.y, b.xscale, b.yscale, b.hspeed, b.vspeed, b.alpha] {
            w.f64(v);
        }
        w.bool(b.htiled);
        w.bool(b.vtiled);
        w.i32(b.blend);
    }
    w.bool(g.rs.views_enabled);
    for v in &g.rs.views {
        w.bool(v.visible);
        for x in [
            v.xview, v.yview, v.wview, v.hview, v.xport, v.yport, v.wport, v.hport, v.angle, v.hborder, v.vborder,
            v.hspeed, v.vspeed,
        ] {
            w.f64(x);
        }
        w.i32(v.object);
    }
    let live: Vec<&Instance> = g.instances.iter().filter(|i| !i.destroyed).collect();
    w.i32(live.len() as i32);
    for i in live {
        write_inst(&mut w, vm, i);
    }
    crate::vfs::write(std::path::Path::new(path), &w.0).map_err(|e| format!("Cannot save game to {}: {}", path, e))
}

/// Loads a save file (applied at the end of a step).
pub fn load_game(g: &mut Game, vm: &mut Vm, path: &str) -> Result<(), String> {
    let data = crate::vfs::read(std::path::Path::new(path)).map_err(|e| format!("Cannot load game from {}: {}", path, e))?;
    if data.len() < 8 || &data[..8] != MAGIC {
        return Err("Save file is from another game or version.".into());
    }
    let mut r = R(&data, 8);
    if r.i32()? != g.game_id {
        return Err("Save file is from another game.".into());
    }
    let room = r.i32()?;
    if !g.room_exists(room) {
        return Err("Save file refers to an unknown room.".into());
    }
    // rebuild the room's static parts, then overwrite
    let rdata = g.rooms[room as usize].as_ref().unwrap().data.clone();
    g.room = room;
    g.rs = crate::room::RoomState::from_format(&rdata, g);
    g.score = r.f64()?;
    g.lives = r.f64()?;
    g.health = r.f64()?;
    g.rs.speed = r.f64()?;
    g.show_score = r.bool()?;
    g.show_lives = r.bool()?;
    g.show_health = r.bool()?;
    g.caption_score = r.str()?;
    g.caption_lives = r.str()?;
    g.caption_health = r.str()?;
    g.next_instance_id = r.i32()?;
    let globals = r.vars(vm)?;
    *vm.globals_mut() = globals;
    g.rs.caption = r.str()?;
    g.rs.bg_color = r.i32()?;
    g.rs.show_color = r.bool()?;
    for b in g.rs.backgrounds.iter_mut() {
        b.visible = r.bool()?;
        b.foreground = r.bool()?;
        b.index = r.i32()?;
        b.x = r.f64()?;
        b.y = r.f64()?;
        b.xscale = r.f64()?;
        b.yscale = r.f64()?;
        b.hspeed = r.f64()?;
        b.vspeed = r.f64()?;
        b.alpha = r.f64()?;
        b.htiled = r.bool()?;
        b.vtiled = r.bool()?;
        b.blend = r.i32()?;
    }
    g.rs.views_enabled = r.bool()?;
    for v in g.rs.views.iter_mut() {
        v.visible = r.bool()?;
        v.xview = r.f64()?;
        v.yview = r.f64()?;
        v.wview = r.f64()?;
        v.hview = r.f64()?;
        v.xport = r.f64()?;
        v.yport = r.f64()?;
        v.wport = r.f64()?;
        v.hport = r.f64()?;
        v.angle = r.f64()?;
        v.hborder = r.f64()?;
        v.vborder = r.f64()?;
        v.hspeed = r.f64()?;
        v.vspeed = r.f64()?;
        v.object = r.i32()?;
    }
    let n = r.i32()?;
    let mut insts = Vec::new();
    for _ in 0..n {
        insts.push(read_inst(&mut r, vm)?);
    }
    g.instances = insts;
    g.reindex();
    g.snapshots.clear();
    g.compute_region();
    g.window_dirty = true;
    Ok(())
}
