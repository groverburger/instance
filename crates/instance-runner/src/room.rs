//! Room state, room transitions and views (runtime.md §3, §12).

use crate::events::{ev, ev_other};
use crate::game::{Game, Snapshot};
use gml::{Ctx, Error, Vm};

/// A background / foreground layer.
#[derive(Clone, Debug)]
pub struct BgLayer {
    pub visible: bool,
    pub foreground: bool,
    pub index: i32,
    pub x: f64,
    pub y: f64,
    pub htiled: bool,
    pub vtiled: bool,
    pub xscale: f64,
    pub yscale: f64,
    pub hspeed: f64,
    pub vspeed: f64,
    pub blend: i32,
    pub alpha: f64,
    pub stretch: bool,
}

impl Default for BgLayer {
    fn default() -> Self {
        BgLayer {
            visible: false,
            foreground: false,
            index: -1,
            x: 0.0,
            y: 0.0,
            htiled: true,
            vtiled: true,
            xscale: 1.0,
            yscale: 1.0,
            hspeed: 0.0,
            vspeed: 0.0,
            blend: 0xFFFFFF,
            alpha: 1.0,
            stretch: false,
        }
    }
}

/// A view.
#[derive(Clone, Debug)]
pub struct ViewState {
    pub visible: bool,
    pub xview: f64,
    pub yview: f64,
    pub wview: f64,
    pub hview: f64,
    pub xport: f64,
    pub yport: f64,
    pub wport: f64,
    pub hport: f64,
    pub angle: f64,
    pub hborder: f64,
    pub vborder: f64,
    pub hspeed: f64,
    pub vspeed: f64,
    pub object: i32,
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState {
            visible: false,
            xview: 0.0,
            yview: 0.0,
            wview: 640.0,
            hview: 480.0,
            xport: 0.0,
            yport: 0.0,
            wport: 640.0,
            hport: 480.0,
            angle: 0.0,
            hborder: 32.0,
            vborder: 32.0,
            hspeed: -1.0,
            vspeed: -1.0,
            object: -1,
        }
    }
}

/// A tile.
#[derive(Clone, Debug)]
pub struct TileState {
    pub id: i32,
    pub background: i32,
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub depth: f64,
    pub visible: bool,
    pub xscale: f64,
    pub yscale: f64,
    pub blend: i32,
    pub alpha: f64,
}

/// Properties of the current room.
#[derive(Clone, Debug)]
pub struct RoomState {
    pub width: i32,
    pub height: i32,
    pub caption: String,
    pub speed: f64,
    pub persistent: bool,
    pub bg_color: i32,
    pub show_color: bool,
    pub backgrounds: Vec<BgLayer>,
    pub views_enabled: bool,
    pub views: Vec<ViewState>,
    pub tiles: Vec<TileState>,
    pub view_current: i32,
}

impl Default for RoomState {
    fn default() -> Self {
        RoomState {
            width: 640,
            height: 480,
            caption: String::new(),
            speed: 30.0,
            persistent: false,
            bg_color: 0xC0C0C0,
            show_color: true,
            backgrounds: vec![BgLayer::default(); 8],
            views_enabled: false,
            views: vec![ViewState::default(); 8],
            tiles: Vec::new(),
            view_current: 0,
        }
    }
}

impl RoomState {
    pub fn from_format(r: &instance_format::Room, g: &Game) -> RoomState {
        let mut rs = RoomState {
            width: r.width,
            height: r.height,
            caption: r.caption.clone(),
            speed: r.speed as f64,
            persistent: r.persistent,
            bg_color: r.background_color as i32 & 0xFFFFFF,
            show_color: r.draw_background_color,
            ..Default::default()
        };
        for (i, b) in r.backgrounds.iter().take(8).enumerate() {
            rs.backgrounds[i] = BgLayer {
                visible: b.visible,
                foreground: b.is_foreground,
                index: b.background,
                x: b.x as f64,
                y: b.y as f64,
                htiled: b.tile_h,
                vtiled: b.tile_v,
                xscale: 1.0,
                yscale: 1.0,
                hspeed: b.hspeed as f64,
                vspeed: b.vspeed as f64,
                blend: 0xFFFFFF,
                alpha: 1.0,
                stretch: b.stretch,
            };
            if b.stretch {
                if let Some(bg) = g.background(b.background) {
                    let (w, h) = (bg.tex.w.max(1) as f64, bg.tex.h.max(1) as f64);
                    rs.backgrounds[i].xscale = r.width as f64 / w;
                    rs.backgrounds[i].yscale = r.height as f64 / h;
                }
            }
        }
        rs.views_enabled = r.views_enabled;
        for (i, v) in r.views.iter().take(8).enumerate() {
            rs.views[i] = ViewState {
                visible: v.visible,
                xview: v.view_x as f64,
                yview: v.view_y as f64,
                wview: v.view_w as f64,
                hview: v.view_h as f64,
                xport: v.port_x as f64,
                yport: v.port_y as f64,
                wport: v.port_w as f64,
                hport: v.port_h as f64,
                angle: 0.0,
                hborder: v.h_border as f64,
                vborder: v.v_border as f64,
                hspeed: v.h_speed as f64,
                vspeed: v.v_speed as f64,
                object: v.follow_object,
            };
        }
        rs.tiles = r
            .tiles
            .iter()
            .map(|t| TileState {
                id: t.id,
                background: t.background,
                left: t.tile_x as f64,
                top: t.tile_y as f64,
                width: t.width as f64,
                height: t.height as f64,
                x: t.x as f64,
                y: t.y as f64,
                depth: t.depth as f64,
                visible: true,
                xscale: 1.0,
                yscale: 1.0,
                blend: 0xFFFFFF,
                alpha: 1.0,
            })
            .collect();
        rs
    }
}

impl Game {
    /// Room transition algorithm (§3.2).
    pub fn goto_room(&mut self, vm: &mut Vm, new_room: i32, is_game_start: bool) -> Result<(), Error> {
        let Some(Some(rres)) = self.rooms.get(new_room.max(0) as usize).map(|r| r.as_ref().map(|r| r.data.clone()))
        else {
            return Err(Error::new(format!("Unexisting room number: {}", new_room)));
        };
        if new_room < 0 {
            return Err(Error::new(format!("Unexisting room number: {}", new_room)));
        }
        let had_room = self.room >= 0;
        if had_room {
            // 1. Room End
            let ids: Vec<i32> = self.instances.iter().filter(|i| i.alive()).map(|i| i.id).collect();
            for id in ids {
                if self.inst(id).map(|i| i.alive()).unwrap_or(false) {
                    self.run_event(vm, id, ev::OTHER, ev_other::ROOM_END, id)?;
                }
            }
        }
        // 2. capture old image for transitions
        let old_image = if had_room && self.transition_kind != 0 {
            Some(self.gfx.read_back())
        } else {
            None
        };
        // 3. remove non-persistent instances, keep persistent active ones
        let mut carried = Vec::new();
        if had_room {
            let all = std::mem::take(&mut self.instances);
            let mut stay = Vec::new();
            for i in all {
                if i.destroyed {
                    continue;
                }
                if i.persistent && i.active {
                    carried.push(i);
                } else {
                    stay.push(i);
                }
            }
            if self.rs.persistent {
                self.snapshots.insert(
                    self.room,
                    Snapshot {
                        state: self.rs.clone(),
                        instances: stay,
                    },
                );
            }
            self.inst_index.clear();
        }
        // 4. load the new room
        self.room = new_room;
        let snapshot = self.snapshots.remove(&new_room);
        let restored = snapshot.is_some();
        match snapshot {
            Some(s) => {
                self.rs = s.state;
                self.instances = s.instances;
            }
            None => {
                self.rs = RoomState::from_format(&rres, self);
                self.instances = Vec::new();
            }
        }
        // 5./6. (runtime.md §3.2a rule 2) the room's placed instances are all added
        // to the list first, without events; a placed instance whose id already
        // exists (a carried persistent instance) is not created again. Carried
        // persistent instances follow the placed (or restored) ones.
        self.reindex();
        let mut placed: Vec<(i32, i32, i32)> = Vec::new(); // (id, stored id, object)
        if !restored {
            for pi in &rres.instances {
                if self.object(pi.object).is_none() {
                    continue;
                }
                if pi.id >= gml::special::FIRST_INSTANCE_ID && carried.iter().any(|c: &crate::instance::Instance| c.id == pi.id) {
                    continue;
                }
                let id = if pi.id >= gml::special::FIRST_INSTANCE_ID && !self.inst_index.contains_key(&pi.id) {
                    pi.id
                } else {
                    self.alloc_id()
                };
                self.add_instance(id, pi.object, pi.x as f64, pi.y as f64);
                placed.push((id, pi.id, pi.object));
            }
        }
        for c in carried {
            self.push_instance(c);
        }
        self.reindex();
        self.compute_region();
        // then per placed instance: its creation code, then its Create event
        for (id, stored, obj) in placed {
            if !self.inst(id).map(|i| i.alive()).unwrap_or(false) {
                continue;
            }
            let cc = self.rooms[new_room as usize]
                .as_ref()
                .and_then(|r| r.inst_creation.get(&stored).cloned());
            if let Some(p) = cc {
                let saved = self.ev;
                self.ev = crate::events::EventCtx {
                    etype: ev::CREATE,
                    number: 0,
                    object: obj,
                    action: 0,
                };
                let r = vm.run(self, &p, Ctx::new(id, id), &[]);
                self.ev = saved;
                if let Err(e) = r {
                    self.report_error(e, 0)?;
                }
            }
            if self.inst(id).map(|i| !i.destroyed).unwrap_or(false) {
                self.run_event(vm, id, ev::CREATE, 0, id)?;
            }
        }
        // 7. game start
        if is_game_start {
            self.broadcast_now(vm, ev::OTHER, ev_other::GAME_START)?;
        }
        // 8. room creation code
        if !restored {
            let cc = self.rooms[new_room as usize].as_ref().and_then(|r| r.creation.clone());
            if let Some(p) = cc {
                let saved = self.ev;
                self.ev = crate::events::EventCtx::default();
                let r = vm.run(self, &p, Ctx::new(gml::special::NOONE, gml::special::NOONE), &[]);
                self.ev = saved;
                if let Err(e) = r {
                    self.report_error(e, 0)?;
                }
            }
        }
        // 9. room start
        self.broadcast_now(vm, ev::OTHER, ev_other::ROOM_START)?;
        // 10. views, caption, transition
        self.update_views();
        self.window_dirty = true;
        let mut drawn = false;
        if let Some(old) = old_image {
            let kind = std::mem::take(&mut self.transition_kind);
            drawn = self.play_transition(vm, old, kind)?;
        } else {
            self.transition_kind = 0;
        }
        // 11. a frame is drawn right after entering the room, before its first
        // step (runtime.md §3.2a rule 3); not when another change is already
        // pending (it is performed at once, by the caller).
        if !drawn && self.pending.is_none() && self.automatic_draw {
            let t0 = web_time::Instant::now();
            self.draw_frame(vm)?;
            self.draw_time += t0.elapsed().as_secs_f64();
        }
        Ok(())
    }

    /// Region size from the room / visible views (§12.1).
    pub fn compute_region(&mut self) {
        let (w, h) = if self.rs.views_enabled && self.rs.views.iter().any(|v| v.visible) {
            let mut w: f64 = 0.0;
            let mut h: f64 = 0.0;
            for v in self.rs.views.iter().filter(|v| v.visible) {
                w = w.max(v.xport + v.wport);
                h = h.max(v.yport + v.hport);
            }
            (w, h)
        } else {
            (self.rs.width as f64, self.rs.height as f64)
        };
        let (w, h) = (w.max(1.0) as u32, h.max(1.0) as u32);
        if w != self.region_w || h != self.region_h || self.gfx.back_size() != (w, h) {
            self.region_w = w;
            self.region_h = h;
            self.gfx.resize_back(w, h);
            self.window_dirty = true;
        }
    }

    /// View following (§12.2).
    pub fn update_views(&mut self) {
        if !self.rs.views_enabled {
            return;
        }
        for v in 0..8 {
            let target = self.rs.views[v].object;
            if target < 0 {
                continue;
            }
            let mut ids = Vec::new();
            self.select(target, &mut ids);
            let Some(&id) = ids.first() else { continue };
            let (cx, cy) = {
                let i = self.inst(id).unwrap();
                (i.x, i.y)
            };
            let (rw, rh) = (self.rs.width as f64, self.rs.height as f64);
            let vs = &mut self.rs.views[v];
            let mut tx = follow_axis(vs.xview, vs.wview, vs.hborder, cx);
            let mut ty = follow_axis(vs.yview, vs.hview, vs.vborder, cy);
            if vs.hspeed >= 0.0 {
                tx = vs.xview + (tx - vs.xview).clamp(-vs.hspeed, vs.hspeed);
            }
            if vs.vspeed >= 0.0 {
                ty = vs.yview + (ty - vs.yview).clamp(-vs.vspeed, vs.vspeed);
            }
            if rw > vs.wview {
                tx = tx.clamp(0.0, rw - vs.wview);
            }
            if rh > vs.hview {
                ty = ty.clamp(0.0, rh - vs.hview);
            }
            vs.xview = tx;
            vs.yview = ty;
        }
    }

    /// Mouse position in room coordinates (§12.3).
    pub fn mouse_room_pos(&self) -> (f64, f64) {
        let (mx, my) = (self.input.mouse_x, self.input.mouse_y);
        if !self.rs.views_enabled {
            return (mx, my);
        }
        let pick = (0..8)
            .rev()
            .find(|&v| {
                let vs = &self.rs.views[v];
                vs.visible && mx >= vs.xport && my >= vs.yport && mx < vs.xport + vs.wport && my < vs.yport + vs.hport
            })
            .unwrap_or(0);
        self.mouse_in_view(pick, mx, my)
    }

    pub fn mouse_in_view(&self, v: usize, mx: f64, my: f64) -> (f64, f64) {
        let vs = &self.rs.views[v.min(7)];
        let xf = crate::draw::painter::Xf::view(
            vs.xview, vs.yview, vs.wview, vs.hview, vs.angle, vs.xport, vs.yport, vs.wport, vs.hport,
        );
        match xf.inverse() {
            Some(i) => i.apply(mx, my),
            None => (mx, my),
        }
    }

    /// Room order helpers.
    pub fn room_next(&self, r: i32) -> i32 {
        match self.room_order.iter().position(|&x| x == r) {
            Some(p) if p + 1 < self.room_order.len() => self.room_order[p + 1],
            _ => -1,
        }
    }

    pub fn room_previous(&self, r: i32) -> i32 {
        match self.room_order.iter().position(|&x| x == r) {
            Some(p) if p > 0 => self.room_order[p - 1],
            _ => -1,
        }
    }

    pub fn room_exists(&self, r: i32) -> bool {
        r >= 0 && self.rooms.get(r as usize).map(|x| x.is_some()).unwrap_or(false)
    }
}

/// Target view position on one axis for an instance at `c` (runtime.md §12.2).
///
/// The view moves just far enough to keep the instance `border` away from both
/// edges. When `2 * border >= size` both edges can't be satisfied at once; the view
/// is then centred on the instance instead of alternating between the two edges
/// every step (border = half the view is the usual GM idiom for a centred camera).
pub(crate) fn follow_axis(view: f64, size: f64, border: f64, c: f64) -> f64 {
    if 2.0 * border >= size {
        return c - size / 2.0;
    }
    if c - border < view {
        c - border
    } else if c + border > view + size {
        c + border - size
    } else {
        view
    }
}

#[cfg(test)]
mod follow_tests {
    use super::follow_axis;

    #[test]
    fn follow_keeps_border_and_settles() {
        // normal borders: move just enough
        assert_eq!(follow_axis(0.0, 240.0, 32.0, 100.0), 0.0);
        assert_eq!(follow_axis(0.0, 240.0, 32.0, 230.0), 22.0);
        assert_eq!(follow_axis(100.0, 240.0, 32.0, 110.0), 78.0);
        // oversized borders (pokemonEngine15: view 240, border 240): centred, stable
        let mut v = 0.0;
        for _ in 0..5 {
            v = follow_axis(v, 240.0, 240.0, 200.0);
            assert_eq!(v, 80.0);
        }
        // border of exactly half the view centres too
        assert_eq!(follow_axis(0.0, 240.0, 120.0, 300.0), 180.0);
    }
}
