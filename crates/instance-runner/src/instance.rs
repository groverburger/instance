//! Instance records and the motion-coupling rules of runtime.md §1.2 and §7.

use gml::VarTable;

/// Number of alarms per instance.
pub const ALARMS: usize = 12;

/// One live instance of an object.
#[derive(Clone, Debug)]
pub struct Instance {
    pub id: i32,
    pub object_index: i32,
    pub x: f64,
    pub y: f64,
    pub xprevious: f64,
    pub yprevious: f64,
    pub xstart: f64,
    pub ystart: f64,
    pub hspeed: f64,
    pub vspeed: f64,
    pub speed: f64,
    pub direction: f64,
    pub friction: f64,
    pub gravity: f64,
    pub gravity_direction: f64,
    pub solid: bool,
    pub visible: bool,
    pub persistent: bool,
    pub depth: f64,
    pub sprite_index: i32,
    pub mask_index: i32,
    pub image_index: f64,
    pub image_speed: f64,
    pub image_xscale: f64,
    pub image_yscale: f64,
    pub image_angle: f64,
    pub image_alpha: f64,
    pub image_blend: i32,
    pub alarm: [f64; ALARMS],
    pub timeline_index: i32,
    pub timeline_position: f64,
    pub timeline_speed: f64,
    pub timeline_running: bool,
    pub timeline_loop: bool,
    pub path_index: i32,
    pub path_position: f64,
    pub path_positionprevious: f64,
    pub path_speed: f64,
    pub path_scale: f64,
    pub path_orientation: f64,
    pub path_endaction: i32,
    /// Room position of the path's anchor point (where path point 0, or the last
    /// point for reversed starts, is placed); the path is transformed around it.
    pub path_xstart: f64,
    pub path_ystart: f64,
    /// Path-local coordinates of the anchor point.
    pub path_anchor_x: f64,
    pub path_anchor_y: f64,
    /// User variables.
    pub vars: VarTable,
    /// `false` while deactivated (§18).
    pub active: bool,
    /// Set by instance_destroy; removed from the list at the end of the step.
    pub destroyed: bool,
    /// Mouse-over state of the previous step (mouse enter/leave events).
    pub mouse_over: bool,
}

impl Instance {
    /// A new instance with the defaults of runtime.md §1.2.
    pub fn new(id: i32, object_index: i32, x: f64, y: f64) -> Self {
        Instance {
            id,
            object_index,
            x,
            y,
            xprevious: x,
            yprevious: y,
            xstart: x,
            ystart: y,
            hspeed: 0.0,
            vspeed: 0.0,
            speed: 0.0,
            direction: 0.0,
            friction: 0.0,
            gravity: 0.0,
            gravity_direction: 270.0,
            solid: false,
            visible: true,
            persistent: false,
            depth: 0.0,
            sprite_index: -1,
            mask_index: -1,
            image_index: 0.0,
            image_speed: 1.0,
            image_xscale: 1.0,
            image_yscale: 1.0,
            image_angle: 0.0,
            image_alpha: 1.0,
            image_blend: 0xFFFFFF,
            alarm: [-1.0; ALARMS],
            timeline_index: -1,
            timeline_position: 0.0,
            timeline_speed: 1.0,
            timeline_running: false,
            timeline_loop: false,
            path_index: -1,
            path_position: 0.0,
            path_positionprevious: 0.0,
            path_speed: 0.0,
            path_scale: 1.0,
            path_orientation: 0.0,
            path_endaction: 0,
            path_xstart: x,
            path_ystart: y,
            path_anchor_x: 0.0,
            path_anchor_y: 0.0,
            vars: VarTable::new(),
            active: true,
            destroyed: false,
            mouse_over: false,
        }
    }

    /// Whether the instance takes part in events and queries.
    #[inline]
    pub fn alive(&self) -> bool {
        self.active && !self.destroyed
    }

    /// Recomputes `speed`/`direction` from `hspeed`/`vspeed` (§7.1).
    pub fn update_polar(&mut self) {
        clean(&mut self.hspeed);
        clean(&mut self.vspeed);
        self.speed = (self.hspeed * self.hspeed + self.vspeed * self.vspeed).sqrt();
        if self.hspeed != 0.0 || self.vspeed != 0.0 {
            self.direction = normalize_angle((-self.vspeed).atan2(self.hspeed).to_degrees());
        }
    }

    /// Recomputes `hspeed`/`vspeed` from `speed`/`direction` (§7.1).
    pub fn update_cartesian(&mut self) {
        let r = self.direction.to_radians();
        self.hspeed = self.speed * r.cos();
        self.vspeed = -self.speed * r.sin();
        clean(&mut self.hspeed);
        clean(&mut self.vspeed);
    }

    pub fn set_hspeed(&mut self, v: f64) {
        self.hspeed = v;
        self.update_polar();
    }
    pub fn set_vspeed(&mut self, v: f64) {
        self.vspeed = v;
        self.update_polar();
    }
    pub fn set_speed(&mut self, v: f64) {
        self.speed = v;
        self.update_cartesian();
    }
    pub fn set_direction(&mut self, v: f64) {
        self.direction = normalize_angle(v);
        self.update_cartesian();
    }
    /// `motion_set`.
    pub fn motion_set(&mut self, dir: f64, sp: f64) {
        self.direction = normalize_angle(dir);
        self.speed = sp;
        self.update_cartesian();
    }
    /// `motion_add`.
    pub fn motion_add(&mut self, dir: f64, sp: f64) {
        let r = dir.to_radians();
        self.hspeed += sp * r.cos();
        self.vspeed -= sp * r.sin();
        self.update_polar();
    }

    /// Friction and gravity (§7.4 steps 2–3).
    pub fn apply_friction_gravity(&mut self) {
        if self.friction != 0.0 && self.speed != 0.0 {
            if self.speed.abs() <= self.friction {
                self.speed = 0.0;
            } else {
                self.speed -= self.friction * self.speed.signum();
            }
            self.update_cartesian();
        }
        if self.gravity != 0.0 {
            let r = self.gravity_direction.to_radians();
            self.hspeed += self.gravity * r.cos();
            self.vspeed -= self.gravity * r.sin();
            self.update_polar();
        }
    }
}

/// Normalises an angle into [0, 360).
pub fn normalize_angle(a: f64) -> f64 {
    if !a.is_finite() {
        return a;
    }
    let r = a.rem_euclid(360.0);
    if r >= 360.0 {
        0.0
    } else {
        r
    }
}

#[inline]
fn clean(v: &mut f64) {
    if v.abs() < 1e-10 {
        *v = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coupling() {
        let mut i = Instance::new(100000, 0, 0.0, 0.0);
        i.set_hspeed(3.0);
        i.set_vspeed(-4.0);
        assert_eq!(i.speed, 5.0);
        assert!((i.direction - 53.130102354).abs() < 1e-6);
        i.set_direction(180.0);
        assert!((i.hspeed + 5.0).abs() < 1e-9 && i.vspeed == 0.0);
        i.set_speed(0.0);
        assert_eq!(i.direction, 180.0);
        i.set_hspeed(0.0);
        assert_eq!(i.direction, 180.0);
        i.motion_set(-90.0, 2.0);
        assert_eq!(i.direction, 270.0);
        assert!((i.vspeed - 2.0).abs() < 1e-9);
        i.friction = 0.5;
        i.apply_friction_gravity();
        assert!((i.speed - 1.5).abs() < 1e-9);
        i.friction = 0.0;
        i.gravity = 1.0;
        i.apply_friction_gravity();
        assert!((i.vspeed - 2.5).abs() < 1e-9);
    }
}
