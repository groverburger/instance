//! Particle systems (runtime.md §16, builtins chapter 412): types, systems,
//! emitters, attractors, destroyers, deflectors, changers and the built-in
//! effects.

use crate::draw::Texture;
use crate::game::Game;
use std::rc::Rc;

/// A particle type.
#[derive(Clone, Debug)]
pub struct PartType {
    pub shape: i32,
    pub sprite: i32,
    pub spr_animat: bool,
    pub spr_stretch: bool,
    pub spr_random: bool,
    pub size_min: f64,
    pub size_max: f64,
    pub size_incr: f64,
    pub size_wiggle: f64,
    pub xscale: f64,
    pub yscale: f64,
    pub ang_min: f64,
    pub ang_max: f64,
    pub ang_incr: f64,
    pub ang_wiggle: f64,
    pub ang_relative: bool,
    pub life_min: f64,
    pub life_max: f64,
    pub step_number: i32,
    pub step_type: i32,
    pub death_number: i32,
    pub death_type: i32,
    pub speed_min: f64,
    pub speed_max: f64,
    pub speed_incr: f64,
    pub speed_wiggle: f64,
    pub dir_min: f64,
    pub dir_max: f64,
    pub dir_incr: f64,
    pub dir_wiggle: f64,
    pub grav_amount: f64,
    pub grav_dir: f64,
    /// 0 single, 1 two-colour, 2 three-colour, 3 mix, 4 rgb range, 5 hsv range.
    pub color_mode: i32,
    pub colors: [i32; 3],
    pub color_ranges: [f64; 6],
    pub alphas: [f64; 3],
    pub alpha_count: i32,
    pub additive: bool,
}

impl Default for PartType {
    fn default() -> Self {
        PartType {
            shape: 0,
            sprite: -1,
            spr_animat: true,
            spr_stretch: false,
            spr_random: false,
            size_min: 1.0,
            size_max: 1.0,
            size_incr: 0.0,
            size_wiggle: 0.0,
            xscale: 1.0,
            yscale: 1.0,
            ang_min: 0.0,
            ang_max: 0.0,
            ang_incr: 0.0,
            ang_wiggle: 0.0,
            ang_relative: false,
            life_min: 100.0,
            life_max: 100.0,
            step_number: 0,
            step_type: 0,
            death_number: 0,
            death_type: 0,
            speed_min: 0.0,
            speed_max: 0.0,
            speed_incr: 0.0,
            speed_wiggle: 0.0,
            dir_min: 0.0,
            dir_max: 0.0,
            dir_incr: 0.0,
            dir_wiggle: 0.0,
            grav_amount: 0.0,
            grav_dir: 270.0,
            color_mode: 0,
            colors: [0xFFFFFF; 3],
            color_ranges: [0.0; 6],
            alphas: [1.0; 3],
            alpha_count: 1,
            additive: false,
        }
    }
}

/// One particle.
#[derive(Clone, Debug)]
pub struct Particle {
    pub ptype: i32,
    pub x: f64,
    pub y: f64,
    pub speed: f64,
    pub dir: f64,
    pub ang: f64,
    pub size: f64,
    pub life: f64,
    pub age: f64,
    pub color: i32,
    pub fixed_color: bool,
    pub alpha: f64,
    pub subimg: f64,
    pub wiggle_phase: f64,
}

/// An emitter.
#[derive(Clone, Debug, Default)]
pub struct Emitter {
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub shape: i32,
    pub distr: i32,
    pub stream_type: i32,
    pub stream_number: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Attractor {
    pub x: f64,
    pub y: f64,
    pub force: f64,
    pub dist: f64,
    pub kind: i32,
    pub additive: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Region {
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub shape: i32,
    pub kind: i32,
    pub friction: f64,
    pub t1: i32,
    pub t2: i32,
}

/// A particle system.
#[derive(Clone, Debug)]
pub struct PartSystem {
    pub depth: f64,
    pub x: f64,
    pub y: f64,
    pub old_to_new: bool,
    pub auto_update: bool,
    pub auto_draw: bool,
    pub particles: Vec<Particle>,
    pub emitters: Vec<Option<Emitter>>,
    pub attractors: Vec<Option<Attractor>>,
    pub destroyers: Vec<Option<Region>>,
    pub deflectors: Vec<Option<Region>>,
    pub changers: Vec<Option<Region>>,
}

impl Default for PartSystem {
    fn default() -> Self {
        PartSystem {
            depth: 0.0,
            x: 0.0,
            y: 0.0,
            old_to_new: true,
            auto_update: true,
            auto_draw: true,
            particles: Vec::new(),
            emitters: Vec::new(),
            attractors: Vec::new(),
            destroyers: Vec::new(),
            deflectors: Vec::new(),
            changers: Vec::new(),
        }
    }
}

/// All particle state.
pub struct Particles {
    pub types: Vec<Option<PartType>>,
    pub systems: Vec<Option<PartSystem>>,
    rng: u64,
    shapes: Vec<Rc<Texture>>,
    /// Internal systems/types for effect_create_*: (below, above).
    pub effect_systems: Option<(i32, i32)>,
    pub effect_types: Vec<i32>,
    /// Action library particle system (action_partsyst_*), types 0..15 and emitters 0..7.
    pub action_system: i32,
    pub action_types: [i32; 16],
    pub action_emitters: [i32; 8],
}

impl Default for Particles {
    fn default() -> Self {
        Particles {
            types: Vec::new(),
            systems: Vec::new(),
            rng: 0x2545F4914F6CDD1D,
            shapes: Vec::new(),
            effect_systems: None,
            effect_types: Vec::new(),
            action_system: -1,
            action_types: [-1; 16],
            action_emitters: [-1; 8],
        }
    }
}

fn alloc<T>(v: &mut Vec<Option<T>>, t: T) -> i32 {
    if let Some(p) = v.iter().position(|x| x.is_none()) {
        v[p] = Some(t);
        p as i32
    } else {
        v.push(Some(t));
        (v.len() - 1) as i32
    }
}

/// Procedural shape textures (64×64, white with alpha).
fn make_shape(shape: i32) -> Texture {
    let n = 64u32;
    let mut t = Texture::new(n, n);
    let c = (n as f64 - 1.0) / 2.0;
    for y in 0..n {
        for x in 0..n {
            let dx = (x as f64 - c) / c;
            let dy = (y as f64 - c) / c;
            let d = (dx * dx + dy * dy).sqrt();
            let a: f64 = match shape {
                0 => {
                    if (x == n / 2 || x == n / 2 - 1) && (y == n / 2 || y == n / 2 - 1) {
                        1.0
                    } else {
                        0.0
                    }
                }
                1 => (d <= 1.0) as i32 as f64,
                2 => 1.0,
                3 => ((dy.abs() < 0.06) as i32 as f64) * (1.0 - dx.abs()).max(0.0),
                4 => {
                    let a = dy.atan2(dx);
                    let r = 0.45 + 0.55 * ((a * 5.0).cos().abs()).powf(6.0);
                    ((d <= r) as i32 as f64) * (1.0 - d / r).max(0.0).sqrt()
                }
                5 => ((d <= 1.0 && d >= 0.85) as i32) as f64,
                6 => (1.0 - ((d - 0.75).abs() / 0.25)).max(0.0),
                7 => (1.0 - d).max(0.0).sqrt(),
                8 => (1.0 - d).max(0.0).powf(2.0),
                9 => {
                    let l = (1.0 - dx.abs()).max(0.0) * (1.0 - dy.abs() * 8.0).max(0.0)
                        + (1.0 - dy.abs()).max(0.0) * (1.0 - dx.abs() * 8.0).max(0.0);
                    l.min(1.0)
                }
                10 => {
                    let a = dy.atan2(dx);
                    let r = 0.7 + 0.3 * (a * 7.0).sin();
                    (1.0 - d / r).max(0.0)
                }
                11 | 12 => {
                    let noise = ((x * 7 + y * 13) % 17) as f64 / 17.0;
                    ((1.0 - d).max(0.0) * (0.6 + 0.4 * noise)).min(1.0)
                }
                13 => {
                    let a = dy.atan2(dx);
                    let arm = ((a * 3.0).sin().abs() < 0.15) as i32 as f64;
                    (arm * (d <= 1.0) as i32 as f64).max((d < 0.15) as i32 as f64)
                }
                _ => (d <= 1.0) as i32 as f64,
            };
            t.px[(y * n + x) as usize] = [255, 255, 255, (a.clamp(0.0, 1.0) * 255.0) as u8];
        }
    }
    t
}

impl Particles {
    pub fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.rand()
    }

    pub fn shape_tex(&mut self, shape: i32) -> Rc<Texture> {
        if self.shapes.is_empty() {
            self.shapes = (0..14).map(|s| Rc::new(make_shape(s))).collect();
        }
        self.shapes[shape.clamp(0, 13) as usize].clone()
    }

    pub fn type_create(&mut self) -> i32 {
        alloc(&mut self.types, PartType::default())
    }

    pub fn system_create(&mut self) -> i32 {
        alloc(&mut self.systems, PartSystem::default())
    }

    pub fn ty(&mut self, i: i32) -> Option<&mut PartType> {
        if i < 0 {
            return None;
        }
        self.types.get_mut(i as usize).and_then(|t| t.as_mut())
    }

    pub fn sys(&mut self, i: i32) -> Option<&mut PartSystem> {
        if i < 0 {
            return None;
        }
        self.systems.get_mut(i as usize).and_then(|t| t.as_mut())
    }

    pub fn emitter_create(&mut self, ps: i32) -> i32 {
        match self.sys(ps) {
            Some(s) => alloc(&mut s.emitters, Emitter::default()),
            None => -1,
        }
    }

    /// Systems drawn automatically: (index, depth).
    pub fn drawable_systems(&self) -> Vec<(usize, f64)> {
        self.systems
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().filter(|s| s.auto_draw).map(|s| (i, s.depth)))
            .collect()
    }

    fn new_particle(&mut self, t: i32, x: f64, y: f64, color: Option<i32>) -> Option<Particle> {
        let pt = self.types.get(t.max(0) as usize).and_then(|x| x.clone())?;
        if t < 0 {
            return None;
        }
        let dir = self.range(pt.dir_min, pt.dir_max);
        let mut p = Particle {
            ptype: t,
            x,
            y,
            speed: self.range(pt.speed_min, pt.speed_max),
            dir,
            ang: self.range(pt.ang_min, pt.ang_max),
            size: self.range(pt.size_min, pt.size_max),
            life: self.range(pt.life_min, pt.life_max).round().max(1.0),
            age: 0.0,
            color: pt.colors[0],
            fixed_color: false,
            alpha: pt.alphas[0],
            subimg: 0.0,
            wiggle_phase: self.rand() * std::f64::consts::TAU,
        };
        match pt.color_mode {
            3 => {
                let f = self.rand();
                p.color = mix(pt.colors[0], pt.colors[1], f);
                p.fixed_color = true;
            }
            4 => {
                let r = self.range(pt.color_ranges[0], pt.color_ranges[1]) as i32;
                let g = self.range(pt.color_ranges[2], pt.color_ranges[3]) as i32;
                let b = self.range(pt.color_ranges[4], pt.color_ranges[5]) as i32;
                p.color = r.clamp(0, 255) | (g.clamp(0, 255) << 8) | (b.clamp(0, 255) << 16);
                p.fixed_color = true;
            }
            5 => {
                let h = self.range(pt.color_ranges[0], pt.color_ranges[1]);
                let s = self.range(pt.color_ranges[2], pt.color_ranges[3]);
                let v = self.range(pt.color_ranges[4], pt.color_ranges[5]);
                p.color = hsv(h, s, v);
                p.fixed_color = true;
            }
            _ => {}
        }
        if let Some(c) = color {
            p.color = c;
            p.fixed_color = true;
        }
        if pt.spr_random {
            p.subimg = self.rand() * 1000.0;
        }
        Some(p)
    }

    pub fn create_particles(&mut self, ps: i32, x: f64, y: f64, t: i32, n: i32, color: Option<i32>) {
        for _ in 0..n.max(0) {
            if let Some(p) = self.new_particle(t, x, y, color) {
                if let Some(s) = self.sys(ps) {
                    s.particles.push(p);
                }
            }
        }
    }

    fn emit_point(&mut self, e: &Emitter) -> (f64, f64) {
        let pick = |s: &mut Self, distr: i32| -> f64 {
            match distr {
                1 => {
                    // gaussian-ish
                    let v = (s.rand() + s.rand() + s.rand()) / 3.0;
                    v
                }
                2 => {
                    let v = (s.rand() + s.rand() + s.rand()) / 3.0;
                    if v < 0.5 {
                        0.5 - v
                    } else {
                        1.5 - v
                    }
                }
                _ => s.rand(),
            }
        };
        loop {
            let fx = pick(self, e.distr);
            let fy = pick(self, e.distr);
            let ok = match e.shape {
                1 => {
                    let (dx, dy) = (fx * 2.0 - 1.0, fy * 2.0 - 1.0);
                    dx * dx + dy * dy <= 1.0
                }
                2 => {
                    let (dx, dy) = (fx * 2.0 - 1.0, fy * 2.0 - 1.0);
                    dx.abs() + dy.abs() <= 1.0
                }
                _ => true,
            };
            if ok {
                if e.shape == 3 {
                    return (e.xmin + (e.xmax - e.xmin) * fx, e.ymin + (e.ymax - e.ymin) * fx);
                }
                return (e.xmin + (e.xmax - e.xmin) * fx, e.ymin + (e.ymax - e.ymin) * fy);
            }
        }
    }

    pub fn emitter_burst(&mut self, ps: i32, em: i32, t: i32, n: i32) {
        let Some(e) = self.sys(ps).and_then(|s| s.emitters.get(em.max(0) as usize).cloned().flatten()) else {
            return;
        };
        let n = if n < 0 {
            if self.rand() * (-n as f64) < 1.0 {
                1
            } else {
                0
            }
        } else {
            n
        };
        for _ in 0..n {
            let (x, y) = self.emit_point(&e);
            if let Some(p) = self.new_particle(t, x, y, None) {
                if let Some(s) = self.sys(ps) {
                    s.particles.push(p);
                }
            }
        }
    }

    /// One update step of a system.
    pub fn update_system(&mut self, ps: i32) {
        let Some(sys) = self.sys(ps) else { return };
        // streams
        let streams: Vec<(usize, i32, i32)> = sys
            .emitters
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.as_ref().filter(|e| e.stream_number != 0).map(|e| (i, e.stream_type, e.stream_number)))
            .collect();
        for (i, t, n) in streams {
            self.emitter_burst(ps, i as i32, t, n);
        }
        let mut parts = std::mem::take(&mut self.sys(ps).unwrap().particles);
        let mut spawned: Vec<(i32, f64, f64, i32)> = Vec::new();
        let types = self.types.clone();
        let sys = self.sys(ps).unwrap().clone();
        for p in parts.iter_mut() {
            p.age += 1.0;
            let Some(Some(pt)) = types.get(p.ptype as usize) else { continue };
            if p.age >= p.life {
                if pt.death_number != 0 {
                    spawned.push((pt.death_type, p.x, p.y, pt.death_number));
                }
                continue;
            }
            p.speed = (p.speed + pt.speed_incr).max(0.0);
            p.dir += pt.dir_incr;
            p.ang += pt.ang_incr;
            p.size = (p.size + pt.size_incr).max(0.0);
            let r = p.dir.to_radians();
            let mut hs = p.speed * r.cos();
            let mut vs = -p.speed * r.sin();
            if pt.grav_amount != 0.0 {
                let g = pt.grav_dir.to_radians();
                hs += pt.grav_amount * g.cos();
                vs -= pt.grav_amount * g.sin();
                p.speed = (hs * hs + vs * vs).sqrt();
                if hs != 0.0 || vs != 0.0 {
                    p.dir = (-vs).atan2(hs).to_degrees();
                }
            }
            for a in sys.attractors.iter().flatten() {
                let (dx, dy) = (a.x - p.x, a.y - p.y);
                let d = (dx * dx + dy * dy).sqrt();
                if d > 0.0 && d < a.dist {
                    let f = match a.kind {
                        1 => a.force * (1.0 - d / a.dist),
                        2 => a.force * (1.0 - d / a.dist).powi(2),
                        _ => a.force,
                    };
                    if a.additive {
                        hs += dx / d * f;
                        vs += dy / d * f;
                        p.speed = (hs * hs + vs * vs).sqrt();
                        p.dir = (-vs).atan2(hs).to_degrees();
                    } else {
                        p.x += dx / d * f.min(d);
                        p.y += dy / d * f.min(d);
                    }
                }
            }
            p.x += hs;
            p.y += vs;
            for d in sys.deflectors.iter().flatten() {
                if p.x >= d.xmin && p.x <= d.xmax && p.y >= d.ymin && p.y <= d.ymax {
                    if d.kind == 0 {
                        p.dir = 180.0 - p.dir;
                    } else {
                        p.dir = -p.dir;
                    }
                    p.speed = (p.speed - d.friction).max(0.0);
                }
            }
            if pt.step_number != 0 {
                spawned.push((pt.step_type, p.x, p.y, pt.step_number));
            }
        }
        let alive_parts: Vec<Particle> = parts
            .into_iter()
            .filter(|p| p.age < p.life)
            .filter(|p| {
                !sys.destroyers.iter().flatten().any(|d| in_region(d, p.x, p.y))
            })
            .map(|mut p| {
                for c in sys.changers.iter().flatten() {
                    if p.ptype == c.t1 && in_region(c, p.x, p.y) {
                        p.ptype = c.t2;
                    }
                }
                p
            })
            .collect();
        self.sys(ps).unwrap().particles = alive_parts;
        for (t, x, y, n) in spawned {
            let n = if n < 0 {
                if self.rand() * (-n as f64) < 1.0 {
                    1
                } else {
                    0
                }
            } else {
                n
            };
            self.create_particles(ps, x, y, t, n, None);
        }
    }

    /// The internal effect systems (created on demand).
    pub fn effect_systems(&mut self) -> (i32, i32) {
        if let Some(s) = self.effect_systems {
            return s;
        }
        let below = self.system_create();
        let above = self.system_create();
        self.sys(below).unwrap().depth = 100000.0;
        self.sys(above).unwrap().depth = -100000.0;
        self.effect_systems = Some((below, above));
        (below, above)
    }

    /// `effect_create_below/above`.
    pub fn effect(&mut self, kind: i32, x: f64, y: f64, size: i32, color: i32, above: bool) {
        let (b, a) = self.effect_systems();
        let ps = if above { a } else { b };
        let sz = [0.5, 1.0, 2.0][size.clamp(0, 2) as usize];
        let t = self.type_create();
        self.effect_types.push(t);
        let rnd = self.rand();
        {
            let pt = self.ty(t).unwrap();
            pt.colors = [color; 3];
            pt.alpha_count = 2;
            pt.alphas = [1.0, 0.0, 0.0];
            match kind {
                0 => {
                    pt.shape = 1;
                    pt.size_min = 0.3 * sz;
                    pt.size_max = 0.6 * sz;
                    pt.size_incr = 0.02 * sz;
                    pt.speed_min = 1.0 * sz;
                    pt.speed_max = 3.0 * sz;
                    pt.dir_max = 360.0;
                    pt.life_min = 15.0;
                    pt.life_max = 25.0;
                    pt.speed_incr = -0.1;
                }
                1 => {
                    pt.shape = 6;
                    pt.size_min = 0.2 * sz;
                    pt.size_max = 0.2 * sz;
                    pt.size_incr = 0.08 * sz;
                    pt.life_min = 15.0;
                    pt.life_max = 15.0;
                }
                2 => {
                    pt.shape = 5;
                    pt.xscale = 1.0;
                    pt.yscale = 0.5;
                    pt.size_min = 0.2 * sz;
                    pt.size_max = 0.2 * sz;
                    pt.size_incr = 0.08 * sz;
                    pt.life_min = 15.0;
                    pt.life_max = 15.0;
                }
                3 | 7 => {
                    pt.shape = if kind == 3 { 4 } else { 9 };
                    pt.size_min = 0.1 * sz;
                    pt.size_max = 0.2 * sz;
                    pt.speed_min = 2.0 * sz;
                    pt.speed_max = 4.0 * sz;
                    pt.dir_max = 360.0;
                    pt.grav_amount = 0.1;
                    pt.life_min = 20.0;
                    pt.life_max = 30.0;
                }
                4 | 5 | 9 => {
                    pt.shape = 12;
                    pt.size_min = 0.4 * sz;
                    pt.size_max = 0.7 * sz;
                    pt.size_incr = 0.01;
                    pt.speed_min = if kind == 5 { 1.0 } else { 0.1 };
                    pt.speed_max = if kind == 5 { 2.0 } else { 0.5 };
                    pt.dir_min = if kind == 5 { 80.0 } else { 0.0 };
                    pt.dir_max = if kind == 5 { 100.0 } else { 360.0 };
                    pt.life_min = 30.0;
                    pt.life_max = 50.0;
                }
                6 | 8 => {
                    pt.shape = if kind == 6 { 4 } else { 8 };
                    pt.size_min = 0.5 * sz;
                    pt.size_max = 0.5 * sz;
                    pt.size_incr = -0.02 * sz;
                    pt.life_min = 20.0;
                    pt.life_max = 20.0;
                }
                10 | 11 => {
                    pt.shape = if kind == 10 { 3 } else { 13 };
                    pt.size_min = 0.1 * sz;
                    pt.size_max = 0.2 * sz;
                    pt.speed_min = if kind == 10 { 6.0 } else { 1.0 };
                    pt.speed_max = if kind == 10 { 8.0 } else { 2.0 };
                    pt.dir_min = if kind == 10 { 260.0 } else { 250.0 };
                    pt.dir_max = if kind == 10 { 260.0 } else { 290.0 };
                    pt.ang_min = if kind == 10 { 260.0 } else { 0.0 };
                    pt.ang_max = pt.ang_min;
                    pt.life_min = 60.0;
                    pt.life_max = 100.0;
                    pt.alphas = [0.8, 0.8, 0.8];
                }
                _ => {}
            }
        }
        let n = match kind {
            0 => (20.0 * sz) as i32,
            1 | 2 | 6 | 8 => 1,
            3 | 7 => (30.0 * sz) as i32,
            4 | 5 | 9 => (8.0 * sz) as i32,
            _ => (15.0 * sz) as i32,
        };
        if kind == 10 || kind == 11 {
            for _ in 0..n {
                let px = self.rand() * 800.0 - 80.0;
                let py = -10.0 - self.rand() * 40.0;
                self.create_particles(ps, px + x * 0.0, py, t, 1, None);
            }
        } else {
            let _ = rnd;
            self.create_particles(ps, x, y, t, n, None);
        }
    }
}

fn in_region(r: &Region, x: f64, y: f64) -> bool {
    if x < r.xmin || x > r.xmax || y < r.ymin || y > r.ymax {
        return false;
    }
    let cx = (r.xmin + r.xmax) / 2.0;
    let cy = (r.ymin + r.ymax) / 2.0;
    let rx = ((r.xmax - r.xmin) / 2.0).max(1e-9);
    let ry = ((r.ymax - r.ymin) / 2.0).max(1e-9);
    let (dx, dy) = ((x - cx) / rx, (y - cy) / ry);
    match r.shape {
        1 => dx * dx + dy * dy <= 1.0,
        2 => dx.abs() + dy.abs() <= 1.0,
        _ => true,
    }
}

/// Linear mix of two GM colours.
pub fn mix(a: i32, b: i32, f: f64) -> i32 {
    let ch = |c: i32, s: i32| ((c >> s) & 0xFF) as f64;
    let m = |s: i32| (ch(a, s) + (ch(b, s) - ch(a, s)) * f).round().clamp(0.0, 255.0) as i32;
    m(0) | (m(8) << 8) | (m(16) << 16)
}

/// HSV (0..255 each) → GM colour.
pub fn hsv(h: f64, s: f64, v: f64) -> i32 {
    let h = (h / 255.0 * 360.0).rem_euclid(360.0);
    let s = (s / 255.0).clamp(0.0, 1.0);
    let v = (v / 255.0).clamp(0.0, 1.0);
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to = |f: f64| ((f + m) * 255.0).round() as i32;
    to(r) | (to(g) << 8) | (to(b) << 16)
}

impl Game {
    /// Automatic particle update (§4.2 step 15).
    pub fn update_particles(&mut self, _vm: &mut gml::Vm) {
        for i in 0..self.particles.systems.len() {
            if self.particles.systems[i].as_ref().map(|s| s.auto_update).unwrap_or(false) {
                self.particles.update_system(i as i32);
            }
        }
        // drop finished effect types
        if !self.particles.effect_types.is_empty() {
            let used: std::collections::HashSet<i32> = self
                .particles
                .systems
                .iter()
                .flatten()
                .flat_map(|s| s.particles.iter().map(|p| p.ptype))
                .collect();
            let types = std::mem::take(&mut self.particles.effect_types);
            for t in types {
                if used.contains(&t) {
                    self.particles.effect_types.push(t);
                } else if let Some(x) = self.particles.types.get_mut(t as usize) {
                    *x = None;
                }
            }
        }
    }

    /// Draws a particle system.
    pub fn draw_particle_system(&mut self, ps: i32) {
        let Some(sys) = self.particles.sys(ps).cloned() else { return };
        let iter: Box<dyn Iterator<Item = &Particle>> = if sys.old_to_new {
            Box::new(sys.particles.iter())
        } else {
            Box::new(sys.particles.iter().rev())
        };
        let saved_blend = self.gfx.blend;
        for p in iter {
            let Some(Some(pt)) = self.particles.types.get(p.ptype as usize).cloned() else { continue };
            let f = (p.age / p.life.max(1.0)).clamp(0.0, 1.0);
            let col = if p.fixed_color {
                p.color
            } else {
                match pt.color_mode {
                    1 => mix(pt.colors[0], pt.colors[1], f),
                    2 => {
                        if f < 0.5 {
                            mix(pt.colors[0], pt.colors[1], f * 2.0)
                        } else {
                            mix(pt.colors[1], pt.colors[2], (f - 0.5) * 2.0)
                        }
                    }
                    _ => pt.colors[0],
                }
            };
            let alpha = match pt.alpha_count {
                2 => pt.alphas[0] + (pt.alphas[1] - pt.alphas[0]) * f,
                3 => {
                    if f < 0.5 {
                        pt.alphas[0] + (pt.alphas[1] - pt.alphas[0]) * f * 2.0
                    } else {
                        pt.alphas[1] + (pt.alphas[2] - pt.alphas[1]) * (f - 0.5) * 2.0
                    }
                }
                _ => pt.alphas[0],
            };
            let wig = |w: f64| w * (p.wiggle_phase + p.age * 0.5).sin();
            let size = (p.size + wig(pt.size_wiggle)).max(0.0);
            let ang = if pt.ang_relative { p.dir + p.ang } else { p.ang } + wig(pt.ang_wiggle);
            self.gfx.blend = if pt.additive { crate::draw::Blend::from_mode(1) } else { saved_blend };
            let (x, y) = (p.x + sys.x, p.y + sys.y);
            if pt.sprite >= 0 {
                let n = self.sprite(pt.sprite).map(|s| s.frames.len()).unwrap_or(1).max(1) as f64;
                let sub = if pt.spr_stretch {
                    f * n
                } else if pt.spr_animat {
                    p.subimg + p.age
                } else {
                    p.subimg
                };
                let spr = pt.sprite;
                self.draw_sprite_ext(spr, sub, x, y, size * pt.xscale, size * pt.yscale, ang, col, alpha);
            } else {
                let tex = self.particles.shape_tex(pt.shape);
                let (w, h) = (tex.w as f64, tex.h as f64);
                self.gfx.draw_tex(
                    &crate::draw::Tex::Img(tex.clone()),
                    0.0,
                    0.0,
                    w,
                    h,
                    w / 2.0,
                    h / 2.0,
                    x,
                    y,
                    size * pt.xscale,
                    size * pt.yscale,
                    ang,
                    [col; 4],
                    alpha,
                );
            }
        }
        self.gfx.blend = saved_blend;
    }
}
