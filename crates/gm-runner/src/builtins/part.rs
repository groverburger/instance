//! Chapter 412: particles.

use super::{bo, r, zero, Reg, A};
use crate::game::Game;
use crate::particles::{Attractor, PartSystem, PartType, Region};
use gml::Error;

fn ty<'a>(g: &'a mut Game, a: &A) -> Result<&'a mut PartType, Error> {
    let i = a.i(0)?;
    g.particles.ty(i).ok_or_else(|| Error::new(format!("Particle type does not exist: {}", i)))
}

fn sys<'a>(g: &'a mut Game, a: &A) -> Result<&'a mut PartSystem, Error> {
    let i = a.i(0)?;
    g.particles.sys(i).ok_or_else(|| Error::new(format!("Particle system does not exist: {}", i)))
}

macro_rules! sub_kind {
    ($l:ident, $field:ident, $T:ty, $create:literal, $destroy:literal, $destroy_all:literal, $exists:literal, $clear:literal) => {
        $l.push(($create, |g, _, _, a| {
            let s = sys(g, a)?;
            s.$field.push(Some(<$T>::default()));
            r((s.$field.len() - 1) as f64)
        }));
        $l.push(($destroy, |g, _, _, a| {
            let i = a.i(1)?;
            let s = sys(g, a)?;
            if let Some(x) = s.$field.get_mut(i.max(0) as usize) {
                *x = None;
            }
            zero()
        }));
        $l.push(($destroy_all, |g, _, _, a| {
            sys(g, a)?.$field.clear();
            zero()
        }));
        $l.push(($exists, |g, _, _, a| {
            let i = a.i(1)?;
            let ok = g.particles.sys(a.i(0)?).map(|s| i >= 0 && s.$field.get(i as usize).map(|x| x.is_some()).unwrap_or(false)).unwrap_or(false);
            bo(ok)
        }));
        $l.push(($clear, |g, _, _, a| {
            let i = a.i(1)?;
            let s = sys(g, a)?;
            if let Some(Some(x)) = s.$field.get_mut(i.max(0) as usize) {
                *x = <$T>::default();
            }
            zero()
        }));
    };
}

fn region<'a>(v: &'a mut [Option<Region>], i: i32) -> Result<&'a mut Region, Error> {
    v.get_mut(i.max(0) as usize).and_then(|x| x.as_mut()).ok_or_else(|| Error::new("Particle element does not exist"))
}

pub fn register(l: &mut Reg) {
    l.push(("effect_create_below", |g, _, _, a| {
        g.particles.effect(a.i(0)?, a.r(1)?, a.r(2)?, a.i(3)?, a.i(4)?, false);
        zero()
    }));
    l.push(("effect_create_above", |g, _, _, a| {
        g.particles.effect(a.i(0)?, a.r(1)?, a.r(2)?, a.i(3)?, a.i(4)?, true);
        zero()
    }));
    l.push(("effect_clear", |g, _, _, _| {
        if let Some((b, a)) = g.particles.effect_systems {
            for s in [b, a] {
                if let Some(s) = g.particles.sys(s) {
                    s.particles.clear();
                }
            }
        }
        zero()
    }));
    l.push(("part_type_create", |g, _, _, _| r(g.particles.type_create() as f64)));
    l.push(("part_type_destroy", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(t) = g.particles.types.get_mut(i.max(0) as usize) {
            *t = None;
        }
        zero()
    }));
    l.push(("part_type_exists", |g, _, _, a| bo(g.particles.ty(a.i(0)?).is_some())));
    l.push(("part_type_clear", |g, _, _, a| {
        *ty(g, a)? = PartType::default();
        zero()
    }));
    l.push(("part_type_shape", |g, _, _, a| {
        let v = a.i(1)?;
        ty(g, a)?.shape = v;
        zero()
    }));
    l.push(("part_type_sprite", |g, _, _, a| {
        let (s, an, stt, rn) = (a.i(1)?, a.b(2)?, a.b(3)?, a.b(4)?);
        let t = ty(g, a)?;
        t.sprite = s;
        t.spr_animat = an;
        t.spr_stretch = stt;
        t.spr_random = rn;
        zero()
    }));
    l.push(("part_type_size", |g, _, _, a| {
        let v = (a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?);
        let t = ty(g, a)?;
        (t.size_min, t.size_max, t.size_incr, t.size_wiggle) = v;
        zero()
    }));
    l.push(("part_type_scale", |g, _, _, a| {
        let v = (a.r(1)?, a.r(2)?);
        let t = ty(g, a)?;
        (t.xscale, t.yscale) = v;
        zero()
    }));
    l.push(("part_type_orientation", |g, _, _, a| {
        let v = (a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?, a.b(5)?);
        let t = ty(g, a)?;
        (t.ang_min, t.ang_max, t.ang_incr, t.ang_wiggle, t.ang_relative) = v;
        zero()
    }));
    l.push(("part_type_life", |g, _, _, a| {
        let v = (a.r(1)?, a.r(2)?);
        let t = ty(g, a)?;
        (t.life_min, t.life_max) = v;
        zero()
    }));
    l.push(("part_type_step", |g, _, _, a| {
        let v = (a.i(1)?, a.i(2)?);
        let t = ty(g, a)?;
        (t.step_number, t.step_type) = v;
        zero()
    }));
    l.push(("part_type_death", |g, _, _, a| {
        let v = (a.i(1)?, a.i(2)?);
        let t = ty(g, a)?;
        (t.death_number, t.death_type) = v;
        zero()
    }));
    l.push(("part_type_speed", |g, _, _, a| {
        let v = (a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?);
        let t = ty(g, a)?;
        (t.speed_min, t.speed_max, t.speed_incr, t.speed_wiggle) = v;
        zero()
    }));
    l.push(("part_type_direction", |g, _, _, a| {
        let v = (a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?);
        let t = ty(g, a)?;
        (t.dir_min, t.dir_max, t.dir_incr, t.dir_wiggle) = v;
        zero()
    }));
    l.push(("part_type_gravity", |g, _, _, a| {
        let v = (a.r(1)?, a.r(2)?);
        let t = ty(g, a)?;
        (t.grav_amount, t.grav_dir) = v;
        zero()
    }));
    l.push(("part_type_color1", |g, _, _, a| {
        let c = a.i(1)?;
        let t = ty(g, a)?;
        t.color_mode = 0;
        t.colors = [c; 3];
        zero()
    }));
    l.push(("part_type_color2", |g, _, _, a| {
        let (c1, c2) = (a.i(1)?, a.i(2)?);
        let t = ty(g, a)?;
        t.color_mode = 1;
        t.colors = [c1, c2, c2];
        zero()
    }));
    l.push(("part_type_color3", |g, _, _, a| {
        let c = [a.i(1)?, a.i(2)?, a.i(3)?];
        let t = ty(g, a)?;
        t.color_mode = 2;
        t.colors = c;
        zero()
    }));
    l.push(("part_type_color_mix", |g, _, _, a| {
        let (c1, c2) = (a.i(1)?, a.i(2)?);
        let t = ty(g, a)?;
        t.color_mode = 3;
        t.colors = [c1, c2, c2];
        zero()
    }));
    l.push(("part_type_color_rgb", |g, _, _, a| {
        let v = [a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?, a.r(5)?, a.r(6)?];
        let t = ty(g, a)?;
        t.color_mode = 4;
        t.color_ranges = v;
        zero()
    }));
    l.push(("part_type_color_hsv", |g, _, _, a| {
        let v = [a.r(1)?, a.r(2)?, a.r(3)?, a.r(4)?, a.r(5)?, a.r(6)?];
        let t = ty(g, a)?;
        t.color_mode = 5;
        t.color_ranges = v;
        zero()
    }));
    l.push(("part_type_alpha1", |g, _, _, a| {
        let v = a.r(1)?;
        let t = ty(g, a)?;
        t.alphas = [v; 3];
        t.alpha_count = 1;
        zero()
    }));
    l.push(("part_type_alpha2", |g, _, _, a| {
        let v = [a.r(1)?, a.r(2)?, a.r(2)?];
        let t = ty(g, a)?;
        t.alphas = v;
        t.alpha_count = 2;
        zero()
    }));
    l.push(("part_type_alpha3", |g, _, _, a| {
        let v = [a.r(1)?, a.r(2)?, a.r(3)?];
        let t = ty(g, a)?;
        t.alphas = v;
        t.alpha_count = 3;
        zero()
    }));
    l.push(("part_type_blend", |g, _, _, a| {
        let v = a.b(1)?;
        ty(g, a)?.additive = v;
        zero()
    }));
    // systems
    l.push(("part_system_create", |g, _, _, _| r(g.particles.system_create() as f64)));
    l.push(("part_system_destroy", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(s) = g.particles.systems.get_mut(i.max(0) as usize) {
            *s = None;
        }
        zero()
    }));
    l.push(("part_system_exists", |g, _, _, a| bo(g.particles.sys(a.i(0)?).is_some())));
    l.push(("part_system_clear", |g, _, _, a| {
        *sys(g, a)? = PartSystem::default();
        zero()
    }));
    l.push(("part_system_draw_order", |g, _, _, a| {
        let v = a.b(1)?;
        sys(g, a)?.old_to_new = v;
        zero()
    }));
    l.push(("part_system_depth", |g, _, _, a| {
        let v = a.r(1)?;
        sys(g, a)?.depth = v;
        zero()
    }));
    l.push(("part_system_position", |g, _, _, a| {
        let v = (a.r(1)?, a.r(2)?);
        let s = sys(g, a)?;
        (s.x, s.y) = v;
        zero()
    }));
    l.push(("part_system_automatic_update", |g, _, _, a| {
        let v = a.b(1)?;
        sys(g, a)?.auto_update = v;
        zero()
    }));
    l.push(("part_system_automatic_draw", |g, _, _, a| {
        let v = a.b(1)?;
        sys(g, a)?.auto_draw = v;
        zero()
    }));
    l.push(("part_system_update", |g, _, _, a| {
        g.particles.update_system(a.i(0)?);
        zero()
    }));
    l.push(("part_system_drawit", |g, _, _, a| {
        g.draw_particle_system(a.i(0)?);
        zero()
    }));
    l.push(("part_particles_create", |g, _, _, a| {
        g.particles.create_particles(a.i(0)?, a.r(1)?, a.r(2)?, a.i(3)?, a.i(4)?, None);
        zero()
    }));
    l.push(("part_particles_create_color", |g, _, _, a| {
        g.particles.create_particles(a.i(0)?, a.r(1)?, a.r(2)?, a.i(3)?, a.i(5)?, Some(a.i(4)?));
        zero()
    }));
    l.push(("part_particles_clear", |g, _, _, a| {
        sys(g, a)?.particles.clear();
        zero()
    }));
    l.push(("part_particles_count", |g, _, _, a| r(sys(g, a)?.particles.len() as f64)));
    // emitters
    l.push(("part_emitter_create", |g, _, _, a| r(g.particles.emitter_create(a.i(0)?) as f64)));
    l.push(("part_emitter_destroy", |g, _, _, a| {
        let i = a.i(1)?;
        if let Some(e) = sys(g, a)?.emitters.get_mut(i.max(0) as usize) {
            *e = None;
        }
        zero()
    }));
    l.push(("part_emitter_destroy_all", |g, _, _, a| {
        sys(g, a)?.emitters.clear();
        zero()
    }));
    l.push(("part_emitter_exists", |g, _, _, a| {
        let i = a.i(1)?;
        bo(g.particles.sys(a.i(0)?).map(|s| i >= 0 && s.emitters.get(i as usize).map(|e| e.is_some()).unwrap_or(false)).unwrap_or(false))
    }));
    l.push(("part_emitter_clear", |g, _, _, a| {
        let i = a.i(1)?;
        if let Some(Some(e)) = sys(g, a)?.emitters.get_mut(i.max(0) as usize) {
            *e = Default::default();
        }
        zero()
    }));
    l.push(("part_emitter_region", |g, _, _, a| {
        let i = a.i(1)?;
        let v = (a.r(2)?, a.r(3)?, a.r(4)?, a.r(5)?, a.i(6)?, a.i(7)?);
        if let Some(Some(e)) = sys(g, a)?.emitters.get_mut(i.max(0) as usize) {
            (e.xmin, e.xmax, e.ymin, e.ymax, e.shape, e.distr) = v;
        }
        zero()
    }));
    l.push(("part_emitter_burst", |g, _, _, a| {
        g.particles.emitter_burst(a.i(0)?, a.i(1)?, a.i(2)?, a.i(3)?);
        zero()
    }));
    l.push(("part_emitter_stream", |g, _, _, a| {
        let i = a.i(1)?;
        let (t, n) = (a.i(2)?, a.i(3)?);
        if let Some(Some(e)) = sys(g, a)?.emitters.get_mut(i.max(0) as usize) {
            e.stream_type = t;
            e.stream_number = n;
        }
        zero()
    }));
    // attractors
    sub_kind!(l, attractors, Attractor, "part_attractor_create", "part_attractor_destroy", "part_attractor_destroy_all", "part_attractor_exists", "part_attractor_clear");
    l.push(("part_attractor_position", |g, _, _, a| {
        let i = a.i(1)?;
        let v = (a.r(2)?, a.r(3)?);
        if let Some(Some(x)) = sys(g, a)?.attractors.get_mut(i.max(0) as usize) {
            (x.x, x.y) = v;
        }
        zero()
    }));
    l.push(("part_attractor_force", |g, _, _, a| {
        let i = a.i(1)?;
        let v = (a.r(2)?, a.r(3)?, a.i(4)?, a.b(5)?);
        if let Some(Some(x)) = sys(g, a)?.attractors.get_mut(i.max(0) as usize) {
            (x.force, x.dist, x.kind, x.additive) = v;
        }
        zero()
    }));
    sub_kind!(l, destroyers, Region, "part_destroyer_create", "part_destroyer_destroy", "part_destroyer_destroy_all", "part_destroyer_exists", "part_destroyer_clear");
    l.push(("part_destroyer_region", |g, _, _, a| {
        let i = a.i(1)?;
        let v = (a.r(2)?, a.r(3)?, a.r(4)?, a.r(5)?, a.i(6)?);
        let x = region(&mut sys(g, a)?.destroyers, i)?;
        (x.xmin, x.xmax, x.ymin, x.ymax, x.shape) = v;
        zero()
    }));
    sub_kind!(l, deflectors, Region, "part_deflector_create", "part_deflector_destroy", "part_deflector_destroy_all", "part_deflector_exists", "part_deflector_clear");
    l.push(("part_deflector_region", |g, _, _, a| {
        let i = a.i(1)?;
        let v = (a.r(2)?, a.r(3)?, a.r(4)?, a.r(5)?);
        let x = region(&mut sys(g, a)?.deflectors, i)?;
        (x.xmin, x.xmax, x.ymin, x.ymax) = v;
        zero()
    }));
    l.push(("part_deflector_kind", |g, _, _, a| {
        let i = a.i(1)?;
        let v = a.i(2)?;
        region(&mut sys(g, a)?.deflectors, i)?.kind = v;
        zero()
    }));
    l.push(("part_deflector_friction", |g, _, _, a| {
        let i = a.i(1)?;
        let v = a.r(2)?;
        region(&mut sys(g, a)?.deflectors, i)?.friction = v;
        zero()
    }));
    sub_kind!(l, changers, Region, "part_changer_create", "part_changer_destroy", "part_changer_destroy_all", "part_changer_exists", "part_changer_clear");
    l.push(("part_changer_region", |g, _, _, a| {
        let i = a.i(1)?;
        let v = (a.r(2)?, a.r(3)?, a.r(4)?, a.r(5)?, a.i(6)?);
        let x = region(&mut sys(g, a)?.changers, i)?;
        (x.xmin, x.xmax, x.ymin, x.ymax, x.shape) = v;
        zero()
    }));
    l.push(("part_changer_kind", |g, _, _, a| {
        let i = a.i(1)?;
        let v = a.i(2)?;
        region(&mut sys(g, a)?.changers, i)?.kind = v;
        zero()
    }));
    l.push(("part_changer_types", |g, _, _, a| {
        let i = a.i(1)?;
        let v = (a.i(2)?, a.i(3)?);
        let x = region(&mut sys(g, a)?.changers, i)?;
        (x.t1, x.t2) = v;
        zero()
    }));
}
