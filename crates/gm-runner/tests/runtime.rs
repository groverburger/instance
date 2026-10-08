//! Runtime behaviour tests on small projects built in code.

use gm_format as fmt;
use gm_runner::Game;
use gml::{Value, Vm};

/// A code action.
fn code(src: &str) -> fmt::Action {
    let mut a = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    a.args[0] = src.to_string();
    a
}

fn event(t: i32, n: i32, src: &str) -> fmt::Event {
    fmt::Event {
        event_type: t,
        number: n,
        actions: vec![code(src)],
    }
}

/// A sprite: w×h opaque square, origin (0,0), rectangle mask.
fn square(w: u32, h: u32) -> fmt::Sprite {
    let mut img = fmt::RgbaImage::new(w, h);
    for p in img.data.chunks_exact_mut(4) {
        p.copy_from_slice(&[255, 0, 0, 255]);
    }
    fmt::Sprite {
        name: format!("spr{}x{}", w, h),
        frames: vec![img],
        collision_shape: 1,
        bbox_mode: 1,
        bbox_right: w as i32 - 1,
        bbox_bottom: h as i32 - 1,
        ..Default::default()
    }
}

struct B {
    p: fmt::Project,
}

impl B {
    fn new() -> Self {
        let mut p = fmt::Project::default();
        p.settings.display_errors = true;
        B { p }
    }
    fn sprite(&mut self, s: fmt::Sprite) -> i32 {
        self.p.sprites.push(Some(s));
        self.p.sprites.len() as i32 - 1
    }
    fn object(&mut self, name: &str, sprite: i32, events: Vec<fmt::Event>) -> i32 {
        self.p.objects.push(Some(fmt::Object {
            name: name.into(),
            sprite,
            visible: true,
            events,
            ..Default::default()
        }));
        self.p.objects.len() as i32 - 1
    }
    fn obj_mut(&mut self, i: i32) -> &mut fmt::Object {
        self.p.objects[i as usize].as_mut().unwrap()
    }
    fn room(&mut self, insts: &[(i32, i32, i32)]) -> i32 {
        let mut r = fmt::Room {
            name: format!("room{}", self.p.rooms.len()),
            ..Default::default()
        };
        for (k, &(o, x, y)) in insts.iter().enumerate() {
            r.instances.push(fmt::Instance {
                x,
                y,
                object: o,
                id: 100001 + (self.p.rooms.len() as i32) * 1000 + k as i32,
                creation_code: String::new(),
                locked: false,
            });
        }
        self.p.last_instance_id = 100001 + 1000 * (self.p.rooms.len() as i32 + 1);
        self.p.rooms.push(Some(r));
        let i = self.p.rooms.len() as i32 - 1;
        self.p.room_order.push(i);
        i
    }
    fn start(self) -> (Game, Vm) {
        let mut vm = Vm::new();
        let mut g = Game::new(self.p, &mut vm, true).expect("game");
        g.start(&mut vm).expect("start");
        (g, vm)
    }
}

fn global(vm: &mut Vm, name: &str) -> Value {
    let id = vm.intern(name);
    vm.globals().get_value(id).cloned().unwrap_or(Value::Real(-12345.0))
}

fn gstr(vm: &mut Vm, name: &str) -> String {
    match global(vm, name) {
        Value::Str(s) => s.to_string_lossy(),
        v => format!("{:?}", v),
    }
}

fn greal(vm: &mut Vm, name: &str) -> f64 {
    match global(vm, name) {
        Value::Real(r) => r,
        v => panic!("{} is {:?}", name, v),
    }
}

fn run(g: &mut Game, vm: &mut Vm, n: usize) {
    for _ in 0..n {
        g.step(vm).expect("step");
    }
}

#[test]
fn event_order_in_step_and_room_start() {
    let mut b = B::new();
    let log = |s: &str| format!("global.log += '{}';", s);
    let o = b.object(
        "o",
        -1,
        vec![
            event(0, 0, "if (!variable_global_exists('log')) global.log = ''; global.log += 'C'; alarm[0] = 1;"),
            event(7, 2, &log("G")),
            event(7, 4, &log("R")),
            event(3, 1, &log("b")),
            event(2, 0, &log("a")),
            event(3, 0, &log("s")),
            event(3, 2, &log("e")),
            event(8, 0, &log("d")),
            event(5, 0, &log("k")), // keyboard <no key>
        ],
    );
    b.room(&[(o, 0, 0)]);
    let (mut g, mut vm) = b.start();
    assert_eq!(gstr(&mut vm, "log"), "CGRd");
    run(&mut g, &mut vm, 2);
    assert_eq!(gstr(&mut vm, "log"), "CGRdbaksedbksed");
}

#[test]
fn motion_friction_gravity() {
    let mut b = B::new();
    let o = b.object("o", -1, vec![event(0, 0, "speed = 4; direction = 0; friction = 1; gravity = 0.5;")]);
    b.room(&[(o, 100, 100)]);
    let (mut g, mut vm) = b.start();
    run(&mut g, &mut vm, 1);
    let i = &g.instances[0];
    // friction: 4 -> 3, then gravity adds vspeed 0.5
    assert!((i.x - 103.0).abs() < 1e-9, "x = {}", i.x);
    assert!((i.y - 100.5).abs() < 1e-9, "y = {}", i.y);
    assert!((i.hspeed - 3.0).abs() < 1e-9);
    assert!((i.vspeed - 0.5).abs() < 1e-9);
    assert_eq!(i.xprevious, 100.0);
}

#[test]
fn alarms_count_down_and_rearm() {
    let mut b = B::new();
    let o = b.object(
        "o",
        -1,
        vec![
            event(0, 0, "global.fired = 0; global.t = 0; alarm[3] = 3;"),
            event(3, 0, "global.t += 1;"),
            event(2, 3, "global.fired += 1; global.at = global.t; alarm[3] = 2;"),
        ],
    );
    b.room(&[(o, 0, 0)]);
    let (mut g, mut vm) = b.start();
    run(&mut g, &mut vm, 3);
    assert_eq!(greal(&mut vm, "fired"), 1.0);
    // alarms run before the step event of the same step: two steps had passed
    assert_eq!(greal(&mut vm, "at"), 2.0);
    run(&mut g, &mut vm, 2);
    assert_eq!(greal(&mut vm, "fired"), 2.0);
}

#[test]
fn solid_collision_stops_and_event_runs() {
    let mut b = B::new();
    let s = b.sprite(square(16, 16));
    let wall = b.object("wall", s, vec![]);
    b.obj_mut(wall).solid = true;
    let ball = b.object(
        "ball",
        s,
        vec![event(0, 0, "hspeed = 5; global.hits = 0;"), event(4, wall, "global.hits += 1; hspeed = 0;")],
    );
    b.room(&[(ball, 0, 0), (wall, 30, 0)]);
    let (mut g, mut vm) = b.start();
    run(&mut g, &mut vm, 6);
    let x = g.instances[0].x;
    assert_eq!(greal(&mut vm, "hits"), 1.0);
    // ball (16 px wide) never overlaps the wall at x=30
    assert!(x + 16.0 <= 30.0, "x = {}", x);
    assert_eq!(x, 10.0);
}

#[test]
fn with_and_instance_functions() {
    let mut b = B::new();
    let c = b.object("c", -1, vec![event(0, 0, "v = 1;")]);
    let ctl = b.object(
        "ctl",
        -1,
        vec![event(
            3,
            0,
            "with (c) v += x; global.n = instance_number(c); global.sum = 0; with (c) global.sum += v; \
             global.first = instance_find(c, 0) == (c).id;",
        )],
    );
    b.room(&[(c, 1, 0), (c, 2, 0), (c, 3, 0), (ctl, 0, 0)]);
    let (mut g, mut vm) = b.start();
    run(&mut g, &mut vm, 1);
    assert_eq!(greal(&mut vm, "n"), 3.0);
    assert_eq!(greal(&mut vm, "sum"), 9.0);
    assert_eq!(greal(&mut vm, "first"), 1.0);
}

#[test]
fn parent_inheritance_and_event_inherited() {
    let mut b = B::new();
    let parent = b.object("parent", -1, vec![event(3, 0, "global.p += 1;"), event(0, 0, "global.p = 0; global.c = 0;")]);
    let child = b.object("child", -1, vec![event(3, 0, "event_inherited(); global.c += 1;")]);
    b.obj_mut(child).parent = parent;
    let ctl = b.object("ctl", -1, vec![event(3, 2, "global.n = instance_number(parent);")]);
    b.room(&[(child, 0, 0), (parent, 0, 0), (ctl, 0, 0)]);
    let (mut g, mut vm) = b.start();
    run(&mut g, &mut vm, 2);
    // child inherits Create; parent step runs for both instances each step
    assert_eq!(greal(&mut vm, "p"), 4.0);
    assert_eq!(greal(&mut vm, "c"), 2.0);
    assert_eq!(greal(&mut vm, "n"), 2.0);
}

#[test]
fn create_destroy_semantics() {
    let mut b = B::new();
    let bullet = b.object("bullet", -1, vec![event(3, 0, "global.bsteps += 1; instance_destroy();"), event(1, 0, "global.destroyed += 1;")]);
    let gun = b.object(
        "gun",
        -1,
        vec![
            event(0, 0, "global.bsteps = 0; global.destroyed = 0;"),
            event(3, 0, &format!("if (!instance_exists({0})) instance_create(0, 0, {0});", bullet)),
        ],
    );
    b.room(&[(gun, 0, 0)]);
    let (mut g, mut vm) = b.start();
    run(&mut g, &mut vm, 1);
    // created during gun's Step: bullet's object (index 0) was already processed,
    // so the bullet gets no Step in this step (runtime.md 4.3a rule 2)
    assert_eq!(greal(&mut vm, "bsteps"), 0.0);
    assert_eq!(g.instances.len(), 2);
    run(&mut g, &mut vm, 1);
    // bullet steps first and destroys itself; gun then creates a new one
    assert_eq!(greal(&mut vm, "bsteps"), 1.0);
    assert_eq!(greal(&mut vm, "destroyed"), 1.0);
    assert_eq!(g.instances.len(), 2);
}

#[test]
fn room_change_with_persistent_instance() {
    let mut b = B::new();
    let keeper = b.object("keeper", -1, vec![event(0, 0, "global.creates += 1;"), event(7, 4, "global.starts += 1;")]);
    b.obj_mut(keeper).persistent = true;
    let go = b.object("go", -1, vec![event(3, 0, "room_goto_next();"), event(7, 5, "global.ends += 1;")]);
    let init = b.object("init", -1, vec![event(0, 0, "global.creates = 0; global.starts = 0; global.ends = 0;")]);
    b.room(&[(init, 0, 0), (keeper, 0, 0), (go, 0, 0)]);
    b.room(&[]);
    let (mut g, mut vm) = b.start();
    run(&mut g, &mut vm, 1);
    assert_eq!(g.room, 1);
    assert_eq!(greal(&mut vm, "creates"), 1.0);
    assert_eq!(greal(&mut vm, "starts"), 2.0);
    assert_eq!(greal(&mut vm, "ends"), 1.0);
    assert_eq!(g.instances.len(), 1);
}

#[test]
fn drag_and_drop_actions_control_flow() {
    let mut b = B::new();
    // if (x == 5) { set var a = 1 } else { set var a = 2 }; repeat 3 times: a += 10
    let mut acts = Vec::new();
    let mut q = fmt::builtin_action(1, 408).unwrap().instantiate(1);
    q.args[0] = "x == 5".into();
    acts.push(q);
    acts.push(fmt::builtin_action(1, 422).unwrap().instantiate(1));
    let mut v = fmt::builtin_action(1, 611).unwrap().instantiate(1);
    v.args[0] = "global.a".into();
    v.args[1] = "1".into();
    acts.push(v.clone());
    acts.push(fmt::builtin_action(1, 424).unwrap().instantiate(1));
    acts.push(fmt::builtin_action(1, 421).unwrap().instantiate(1));
    let mut v2 = v.clone();
    v2.args[1] = "2".into();
    acts.push(v2);
    let mut rep = fmt::builtin_action(1, 423).unwrap().instantiate(1);
    rep.args[0] = "3".into();
    acts.push(rep);
    let mut v3 = v.clone();
    v3.args[1] = "10".into();
    v3.relative = true;
    acts.push(v3);
    acts.push(fmt::builtin_action(1, 425).unwrap().instantiate(1)); // exit
    acts.push(code("global.a = -1;"));
    let o = b.object("o", -1, vec![fmt::Event { event_type: 0, number: 0, actions: acts }]);
    b.room(&[(o, 5, 0), (o, 6, 0)]);
    let (_g, mut vm) = b.start();
    // second instance: x == 6 -> a = 2, + 30
    assert_eq!(greal(&mut vm, "a"), 32.0);
}

#[test]
fn drawing_rectangles_and_sprites() {
    let mut b = B::new();
    let s = b.sprite(square(4, 4));
    let o = b.object("o", s, vec![]);
    let d = b.object("d", -1, vec![event(8, 0, "draw_set_color(c_lime); draw_rectangle(10, 10, 19, 19, false);")]);
    b.room(&[(o, 0, 0), (d, 0, 0)]);
    let (mut g, mut vm) = b.start();
    run(&mut g, &mut vm, 1);
    let img = g.frame_image();
    assert_eq!(img.get(0, 0), [255, 0, 0, 255]);
    assert_eq!(img.get(3, 3), [255, 0, 0, 255]);
    assert_eq!(img.get(4, 4)[..3], [0xC0, 0xC0, 0xC0]);
    assert_eq!(img.get(10, 10), [0, 255, 0, 255]);
    assert_eq!(img.get(19, 19), [0, 255, 0, 255]);
    assert_eq!(img.get(20, 20)[..3], [0xC0, 0xC0, 0xC0]);
}

#[test]
fn scripts_and_keyboard_input() {
    let mut b = B::new();
    b.p.scripts.push(Some(fmt::Script {
        name: "add".into(),
        code: "return argument0 + argument1;".into(),
        ..Default::default()
    }));
    let o = b.object(
        "o",
        -1,
        vec![event(0, 0, "global.r = add(2, 3); global.k = 0;"), event(9, 37, "global.k += 1;")],
    );
    b.room(&[(o, 0, 0)]);
    let (mut g, mut vm) = b.start();
    assert_eq!(greal(&mut vm, "r"), 5.0);
    g.scripted_input = vec![(1, gm_runner::InputEvent::KeyDown(37)), (3, gm_runner::InputEvent::KeyUp(37))];
    run(&mut g, &mut vm, 5);
    assert_eq!(greal(&mut vm, "k"), 1.0);
}

#[test]
fn runtime_error_is_reported_and_execution_continues() {
    let mut b = B::new();
    let o = b.object("o", -1, vec![event(0, 0, "global.a = 1; global.b = undefined_var + 1; global.c = 2;")]);
    // second action of the same event still runs
    b.obj_mut(o).events[0].actions.push(code("global.d = 4;"));
    b.room(&[(o, 0, 0)]);
    let (g, mut vm) = b.start();
    assert_eq!(greal(&mut vm, "a"), 1.0);
    assert_eq!(greal(&mut vm, "d"), 4.0);
    assert!(g.error_occurred);
    assert!(g.error_last.contains("undefined_var"), "{}", g.error_last);
}

#[test]
fn views_follow_and_draw_through_port() {
    let mut b = B::new();
    let s = b.sprite(square(8, 8));
    let o = b.object("o", s, vec![event(0, 0, "hspeed = 10;")]);
    let r = b.room(&[(o, 50, 50)]);
    {
        let rm = b.p.rooms[r as usize].as_mut().unwrap();
        rm.width = 2000;
        rm.height = 480;
        rm.views_enabled = true;
        rm.views[0] = fmt::View {
            visible: true,
            view_w: 320,
            view_h: 240,
            port_w: 640,
            port_h: 480,
            h_border: 100,
            v_border: 100,
            h_speed: -1,
            v_speed: -1,
            follow_object: o,
            ..Default::default()
        };
    }
    let (mut g, mut vm) = b.start();
    assert_eq!((g.region_w, g.region_h), (640, 480));
    run(&mut g, &mut vm, 40);
    let x = g.instances[0].x;
    let vx = g.rs.views[0].xview;
    assert!((x + 100.0 - (vx + 320.0)).abs() < 1e-9, "view did not follow: x={} xview={}", x, vx);
    // the 8x8 sprite appears 16x16 at port scale 2
    let px = ((x - vx) * 2.0) as u32;
    let py = ((50.0 - g.rs.views[0].yview) * 2.0) as u32;
    let img = g.frame_image();
    assert_eq!(img.get(px + 1, py + 1)[..3], [255, 0, 0]);
    assert_eq!(img.get(px + 15, py + 15)[..3], [255, 0, 0]);
}

#[test]
fn surfaces_and_blend_modes() {
    let mut b = B::new();
    let d = b.object(
        "d",
        -1,
        vec![event(
            8,
            0,
            "if (!variable_global_exists('s')) global.s = surface_create(16, 16); \
             surface_set_target(global.s); draw_clear(c_red); draw_set_blend_mode(bm_add); \
             draw_set_color(c_lime); draw_rectangle(0, 0, 7, 15, false); draw_set_blend_mode(bm_normal); \
             surface_reset_target(); draw_surface(global.s, 100, 100); \
             global.px = surface_getpixel(global.s, 2, 2);",
        )],
    );
    b.room(&[(d, 0, 0)]);
    let (mut g, mut vm) = b.start();
    run(&mut g, &mut vm, 1);
    assert_eq!(greal(&mut vm, "px"), 0x00FFFF as f64); // red + lime = yellow
    let img = g.frame_image();
    assert_eq!(img.get(102, 102)[..3], [255, 255, 0]);
    assert_eq!(img.get(110, 102)[..3], [255, 0, 0]);
}

#[test]
fn paths_and_timelines() {
    let mut b = B::new();
    b.p.paths.push(Some(fmt::Path {
        name: "p".into(),
        connection_kind: 0,
        closed: false,
        precision: 4,
        points: vec![
            fmt::PathPoint { x: 0.0, y: 0.0, speed: 100.0 },
            fmt::PathPoint { x: 100.0, y: 0.0, speed: 100.0 },
        ],
        ..Default::default()
    }));
    b.p.timelines.push(Some(fmt::Timeline {
        name: "t".into(),
        moments: vec![
            fmt::Moment { step: 0, actions: vec![code("global.m0 += 1;")] },
            fmt::Moment { step: 3, actions: vec![code("global.m3 = global.steps;")] },
        ],
        ..Default::default()
    }));
    let o = b.object(
        "o",
        -1,
        vec![
            event(0, 0, "global.m0 = 0; global.m3 = -1; global.steps = 0; global.eop = 0; path_start(0, 10, 0, false); timeline_index = 0; timeline_running = true;"),
            event(3, 0, "global.steps += 1;"),
            event(7, 8, "global.eop += 1;"),
        ],
    );
    b.room(&[(o, 50, 20)]);
    let (mut g, mut vm) = b.start();
    run(&mut g, &mut vm, 5);
    assert_eq!(g.instances[0].x, 100.0);
    assert_eq!(g.instances[0].y, 20.0);
    assert_eq!(greal(&mut vm, "m0"), 1.0);
    // moment 3 runs at the start of the 4th step (before its step event)
    assert_eq!(greal(&mut vm, "m3"), 3.0);
    run(&mut g, &mut vm, 10);
    assert_eq!(greal(&mut vm, "eop"), 1.0);
    assert_eq!(g.instances[0].x, 150.0);
    assert_eq!(g.instances[0].path_index, -1);
}

/// The collision broad phase (spatial index, per-object lists) must give the same
/// answers as a scan over all instances while instances move, scale, rotate, are
/// created, destroyed, changed and deactivated.
#[test]
fn collision_queries_match_brute_force() {
    use gm_runner::collision::Sel;
    let mut b = B::new();
    let s1 = b.sprite(square(16, 16));
    let s2 = b.sprite(square(8, 24));
    let mover = "x += floor(random(41)) - 20; y += floor(random(41)) - 20;
        if (x < 0) x += 640; if (x > 640) x -= 640; if (y < 0) y += 480; if (y > 480) y -= 480;
        r = random(1);
        if (r < 0.02) instance_destroy()
        else if (r < 0.04) instance_create(random(640), random(480), object_index)
        else if (r < 0.06) image_xscale = choose(1, 2, -1)
        else if (r < 0.08) image_angle = random(360)
        else if (r < 0.09) instance_change(choose(0, 1), false)
        else if (r < 0.10) sprite_index = choose(0, 1);";
    let a = b.object("a", s1, vec![event(3, 0, mover)]);
    let c = b.object("c", s2, vec![event(3, 0, mover)]);
    let ctl = b.object(
        "ctl",
        -1,
        vec![event(
            3,
            2,
            "if (random(1) < 0.3) instance_deactivate_object(instance_find(a, 0)); if (random(1) < 0.3) instance_activate_all();",
        )],
    );
    let mut insts = vec![(ctl, 0, 0)];
    for k in 0..300 {
        insts.push((if k % 3 == 0 { c } else { a }, (k * 37) % 640, (k * 53) % 480));
    }
    b.room(&insts);
    let (mut g, mut vm) = b.start();
    let brute_place = |g: &Game, id: i32, x: f64, y: f64, s: Sel| -> Option<i32> {
        let me = g.inst(id)?;
        let geo = g.geo_at(me, x, y)?;
        g.instances
            .iter()
            .filter(|o| o.id != id && o.alive() && g.sel_match(s, o))
            .find(|o| g.geo_at(o, o.x, o.y).map(|go| Game::geos_collide(&geo, &go)).unwrap_or(false))
            .map(|o| o.id)
    };
    let brute_point = |g: &Game, x: f64, y: f64, s: Sel| -> Option<i32> {
        g.instances
            .iter()
            .filter(|o| o.alive() && g.sel_match(s, o))
            .find(|o| g.point_in_instance(o.id, x, y, true))
            .map(|o| o.id)
    };
    let mut checks = 0;
    for step in 0..60 {
        run(&mut g, &mut vm, 1);
        let ids: Vec<i32> = g.instances.iter().filter(|i| i.alive()).map(|i| i.id).collect();
        for (k, &id) in ids.iter().enumerate().step_by(7) {
            let i = g.inst(id).unwrap();
            let (x, y) = (i.x + (k % 5) as f64 - 2.0, i.y);
            for s in [Sel::All, Sel::Obj(a), Sel::Obj(c)] {
                assert_eq!(g.place_meeting_sel(id, x, y, s), brute_place(&g, id, x, y, s), "step {} id {}", step, id);
                checks += 1;
            }
            let (px, py) = ((k * 13 % 640) as f64 + 0.5, (k * 29 % 480) as f64 + 0.5);
            assert_eq!(g.instance_at_point(px, py, Sel::All), brute_point(&g, px, py, Sel::All));
            let mut sel = Vec::new();
            g.select(a, &mut sel);
            let brute: Vec<i32> = g.instances.iter().filter(|i| i.alive() && i.object_index == a).map(|i| i.id).collect();
            assert_eq!(sel, brute);
        }
    }
    assert!(checks > 1000);
}
