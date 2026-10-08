//! Event dispatch rules verified in GM 8.1 with bb_events / bb_instances
//! (runtime.md §4.3a). Small projects built in code; events log through the
//! script `l` into `global.log`.

use gm_format as fmt;
use gm_runner::Game;
use gml::{Value, Vm};

fn code(src: &str) -> fmt::Action {
    let mut a = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    a.args[0] = src.to_string();
    a
}

fn ev(t: i32, n: i32, srcs: &[&str]) -> fmt::Event {
    fmt::Event {
        event_type: t,
        number: n,
        actions: srcs.iter().map(|s| code(s)).collect(),
    }
}

const CREATE: i32 = 0;
const DESTROY: i32 = 1;
const ALARM: i32 = 2;
const STEP: i32 = 3;
const OTHER: i32 = 7;
const DRAW: i32 = 8;
const BEGIN: i32 = 1;
const KEYPRESS: i32 = 9;

struct B {
    p: fmt::Project,
}

impl B {
    fn new() -> Self {
        let mut p = fmt::Project::default();
        p.settings.display_errors = false;
        p.scripts.push(Some(fmt::Script {
            name: "l".into(),
            last_changed: 0.0,
            code: "if (!variable_global_exists('log')) global.log = ''; global.log += argument0 + ' ';".into(),
        }));
        let mut img = fmt::RgbaImage::new(16, 16);
        for px in img.data.chunks_exact_mut(4) {
            px.copy_from_slice(&[255, 0, 0, 255]);
        }
        p.sprites.push(Some(fmt::Sprite {
            name: "spr".into(),
            frames: vec![img],
            collision_shape: 1,
            bbox_mode: 1,
            bbox_right: 15,
            bbox_bottom: 15,
            ..Default::default()
        }));
        B { p }
    }
    fn object(&mut self, name: &str, events: Vec<fmt::Event>) -> i32 {
        self.p.objects.push(Some(fmt::Object {
            name: name.into(),
            sprite: -1,
            visible: true,
            events,
            ..Default::default()
        }));
        self.p.objects.len() as i32 - 1
    }
    fn room(&mut self, w: i32, insts: &[(i32, i32, i32)]) {
        let mut r = fmt::Room {
            name: "r0".into(),
            width: w,
            height: 480,
            ..Default::default()
        };
        for (k, &(o, x, y)) in insts.iter().enumerate() {
            r.instances.push(fmt::Instance {
                x,
                y,
                object: o,
                id: 100001 + k as i32,
                creation_code: String::new(),
                locked: false,
            });
        }
        self.p.last_instance_id = 100001 + insts.len() as i32;
        self.p.rooms.push(Some(r));
        self.p.room_order.push(0);
    }
    fn start(self) -> (Game, Vm) {
        let mut vm = Vm::new();
        let mut g = Game::new(self.p, &mut vm, true).expect("game");
        g.start(&mut vm).expect("start");
        (g, vm)
    }
}

fn take(vm: &mut Vm) -> String {
    let id = vm.intern("log");
    let s = match vm.globals().get_value(id).cloned() {
        Some(Value::Str(s)) => s.to_string_lossy(),
        _ => String::new(),
    };
    vm.globals_mut().set_value(id, Value::Str("".into()));
    s.trim_end().to_string()
}

fn global(vm: &mut Vm, name: &str) -> Value {
    let id = vm.intern(name);
    vm.globals().get_value(id).cloned().unwrap_or_default()
}

#[test]
fn step_events_are_dispatched_object_by_object() {
    // bb_events ev.s02 / bb_instances in.s01: object index order, instance order
    // within an object; an instance created in the phase runs in the same phase
    // only if its object comes later (or is the current object).
    let mut b = B::new();
    let early = b.object("early", vec![ev(STEP, BEGIN, &["l('early');"])]);
    let a = b.object("a", vec![ev(STEP, BEGIN, &["l('a' + string(x)); if (x == 0 && !variable_global_exists('done')) { global.done = 1; instance_create(9, 0, c); instance_create(8, 0, early); }"])]);
    let c = b.object("c", vec![ev(STEP, BEGIN, &["l('c' + string(x));"])]);
    let _ = early;
    b.room(640, &[(c, 1, 0), (a, 0, 0), (a, 2, 0)]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    g.step(&mut vm).unwrap();
    // `early` (object before a) is not visited in this phase, `c` (after a) is
    assert_eq!(take(&mut vm), "a0 a2 c1 c9");
    g.step(&mut vm).unwrap();
    assert_eq!(take(&mut vm), "early a0 a2 c1 c9");
}

#[test]
fn alarms_are_integers_fire_at_zero_and_stay_zero_one_step() {
    // bb_events ev.alarms.*: 1.5 -> 2, 0.5 -> 0; an alarm at 0 becomes -1 without
    // firing; after firing it reads 0 for one step.
    let mut b = B::new();
    let a = b.object(
        "a",
        vec![
            ev(CREATE, 0, &["alarm[1] = 1.5; alarm[4] = 0.5; alarm[2] = 0;"]),
            ev(STEP, BEGIN, &["l(string(alarm[1]) + ',' + string(alarm[2]) + ',' + string(alarm[4]));"]),
            ev(ALARM, 1, &["l('A1[' + string(alarm[1]) + ']');"]),
            ev(ALARM, 2, &["l('A2');"]),
            ev(ALARM, 4, &["l('A4');"]),
        ],
    );
    // no alarm event: the alarm does not count down
    let z = b.object("z", vec![ev(CREATE, 0, &["alarm[0] = 2;"]), ev(STEP, BEGIN, &["l('z' + string(alarm[0]));"])]);
    b.room(640, &[(a, 0, 0), (z, 0, 0)]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    let mut seq = Vec::new();
    for _ in 0..4 {
        g.step(&mut vm).unwrap();
        seq.push(take(&mut vm));
    }
    assert_eq!(seq, ["2,0,0 z2", "1,-1,-1 z2 A1[0]", "0,-1,-1 z2", "-1,-1,-1 z2"]);
}

#[test]
fn alarms_run_alarm_by_alarm_then_object_by_object() {
    // bb_events ev.s02+1 "q.A0inh a.A1[0]": alarm 0 of every object before alarm 1
    let mut b = B::new();
    let a = b.object("a", vec![ev(CREATE, 0, &["alarm[1] = 1;"]), ev(ALARM, 1, &["l('a.A1');"])]);
    let qp = b.object("qparent", vec![ev(ALARM, 0, &["l('q.A0inh');"])]);
    let q = b.object("q", vec![ev(CREATE, 0, &["alarm[0] = 1;"])]);
    b.p.objects[q as usize].as_mut().unwrap().parent = qp;
    b.room(640, &[(a, 0, 0), (q, 0, 0)]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    g.step(&mut vm).unwrap();
    assert_eq!(take(&mut vm), "q.A0inh a.A1");
}

#[test]
fn other_is_self_in_create_and_destroy_and_destroy_twice_runs_twice() {
    // bb_instances other.create_from_code, other.in_destroy, destroy.twice
    let mut b = B::new();
    let oc = b.object(
        "oc",
        vec![
            ev(CREATE, 0, &["global.c = (other.id == id);"]),
            ev(DESTROY, 0, &["global.d = (other.id == id); global.n += 1;"]),
        ],
    );
    let ctl = b.object(
        "ctl",
        vec![ev(CREATE, 0, &["global.n = 0; var u; u = instance_create(0, 0, oc); with (u) { instance_destroy(); instance_destroy(); }"])],
    );
    let _ = oc;
    b.room(640, &[(ctl, 0, 0)]);
    let (_g, mut vm) = b.start();
    assert_eq!(global(&mut vm, "c"), Value::Real(1.0));
    assert_eq!(global(&mut vm, "d"), Value::Real(1.0));
    assert_eq!(global(&mut vm, "n"), Value::Real(2.0));
}

#[test]
fn reading_a_destroyed_instance_from_outside_is_an_error() {
    // bb_instances destroy.read_after / with.destroy_self_then_read
    let mut b = B::new();
    let ou = b.object("ou", vec![]);
    let ctl = b.object(
        "ctl",
        vec![ev(
            CREATE,
            0,
            &[
                "var u; u = instance_create(5, 0, ou); with (u) { instance_destroy(); global.selfx = x; } global.r = u.x;",
                "global.after = error_last;",
            ],
        )],
    );
    let _ = ou;
    b.room(640, &[(ctl, 0, 0)]);
    let (_g, mut vm) = b.start();
    assert_eq!(global(&mut vm, "selfx"), Value::Real(5.0));
    assert_eq!(global(&mut vm, "r"), Value::Real(0.0)); // not assigned
    let msg = global(&mut vm, "after").to_gml_string().to_string_lossy();
    assert!(msg.contains("Unknown variable x"), "{msg}");
}

#[test]
fn equal_depth_draw_order_is_stable_across_depth_changes() {
    // bb_instances in.s02..in.s04: q goes to depth 5 and back, p3 to -5 and back;
    // the order q p1 p2 p3 stays after both return to depth 0.
    let mut b = B::new();
    let p = b.object(
        "p",
        vec![
            ev(DRAW, 0, &["l(tag);"]),
            ev(STEP, 0, &["if (global.t == 1 && tag == 'q') depth = 5; if (global.t == 1 && tag == 'p3') depth = -5; if (global.t == 2) depth = 0;"]),
        ],
    );
    let ctl = b.object("ctl", vec![ev(CREATE, 0, &["global.t = 0;"]), ev(STEP, BEGIN, &["global.t += 1;"])]);
    b.room(640, &[(ctl, 0, 0), (p, 0, 0), (p, 0, 0), (p, 0, 0), (p, 0, 0)]);
    let tags = ["p1", "q", "p2", "p3"];
    b.p.rooms[0].as_mut().unwrap().instances[1..].iter_mut().zip(tags).for_each(|(i, t)| i.creation_code = format!("tag = '{t}';"));
    let (mut g, mut vm) = b.start();
    assert_eq!(take(&mut vm), "p1 q p2 p3");
    g.step(&mut vm).unwrap();
    assert_eq!(take(&mut vm), "q p1 p2 p3");
    g.step(&mut vm).unwrap();
    assert_eq!(take(&mut vm), "q p1 p2 p3");
}

#[test]
fn instance_created_in_draw_is_drawn_in_the_same_frame() {
    // bb_events ev.s02+5 "h.DR"
    let mut b = B::new();
    let h = b.object("h", vec![ev(DRAW, 0, &["l('h.DR');"])]);
    let a = b.object("a", vec![ev(DRAW, 0, &["l('a.DR'); if (!variable_global_exists('mk')) { global.mk = 1; instance_create(0, 0, h); }"])]);
    let _ = h;
    b.room(640, &[(a, 0, 0)]);
    let (_g, mut vm) = b.start();
    assert_eq!(take(&mut vm), "a.DR h.DR");
}

#[test]
fn outside_and_intersect_boundary_rules() {
    // bb_events c.OR / c.IB: 16 px box moving right by 4 in a 64 px wide room
    let mut b = B::new();
    let c = b.object(
        "c",
        vec![
            ev(CREATE, 0, &["hspeed = 4;"]),
            ev(OTHER, 0, &["l('OR' + string(x));"]),
            ev(OTHER, 1, &["l('IB' + string(x));"]),
        ],
    );
    b.p.objects[c as usize].as_mut().unwrap().sprite = 0;
    b.room(64, &[(c, 40, 0)]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    let mut all = Vec::new();
    for _ in 0..8 {
        g.step(&mut vm).unwrap();
        let s = take(&mut vm);
        if !s.is_empty() {
            all.push(s);
        }
    }
    assert_eq!(all, ["IB52", "IB56", "IB60", "IB64", "OR68 IB68", "OR72 IB72"]);
}

#[test]
fn keyboard_key_press_takes_effect_next_step() {
    // bb_events ev.s05 / ev.s06 / ev.keyboard_string
    let mut b = B::new();
    let a = b.object(
        "a",
        vec![
            ev(STEP, 0, &["global.t += 1; if (global.t == 1) keyboard_key_press(ord('K')); l('chk' + string(keyboard_check(ord('K'))));"]),
            ev(KEYPRESS, 75, &["l('KP');"]),
            ev(CREATE, 0, &["global.t = 0;"]),
        ],
    );
    b.room(640, &[(a, 0, 0)]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    g.step(&mut vm).unwrap();
    assert_eq!(take(&mut vm), "chk0");
    g.step(&mut vm).unwrap();
    assert_eq!(take(&mut vm), "KP chk1");
    assert_eq!(g.input.keyboard_string, "k");
}

#[test]
fn no_more_lives_runs_at_the_assignment() {
    // bb_events ev.s06+1 "a.lives0 a.NML a.lives0done"
    let mut b = B::new();
    let a = b.object(
        "a",
        vec![
            ev(CREATE, 0, &["lives = 3;"]),
            ev(STEP, 0, &["l('before'); lives = 0; l('after');"]),
            ev(OTHER, 6, &["l('NML');"]),
        ],
    );
    b.room(640, &[(a, 0, 0)]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    g.step(&mut vm).unwrap();
    assert_eq!(take(&mut vm), "before NML after");
}
