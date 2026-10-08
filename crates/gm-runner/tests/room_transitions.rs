//! Room-transition rules verified in GM 8.1 (runtime.md §3.2a), on small
//! projects built in code. Every test logs events through the script `l`
//! into `global.log`.

use gm_format as fmt;
use gm_runner::Game;
use gml::{Value, Vm};

fn code(src: &str) -> fmt::Action {
    let mut a = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    a.args[0] = src.to_string();
    a
}

/// An event with one code action per source string.
fn ev(t: i32, n: i32, srcs: &[&str]) -> fmt::Event {
    fmt::Event {
        event_type: t,
        number: n,
        actions: srcs.iter().map(|s| code(s)).collect(),
    }
}

fn square() -> fmt::Sprite {
    let mut img = fmt::RgbaImage::new(16, 16);
    for p in img.data.chunks_exact_mut(4) {
        p.copy_from_slice(&[255, 0, 0, 255]);
    }
    fmt::Sprite {
        name: "spr".into(),
        frames: vec![img],
        collision_shape: 1,
        bbox_mode: 1,
        bbox_right: 15,
        bbox_bottom: 15,
        ..Default::default()
    }
}

// event numbers
const CREATE: i32 = 0;
const ALARM: i32 = 2;
const STEP: i32 = 3;
const COLLISION: i32 = 4;
const OTHER: i32 = 7;
const DRAW: i32 = 8;
const BEGIN: i32 = 1;
const END: i32 = 2;
const GAME_START: i32 = 2;
const ROOM_START: i32 = 4;
const ROOM_END: i32 = 5;
const USER0: i32 = 10;

struct B {
    p: fmt::Project,
}

impl B {
    fn new() -> Self {
        let mut p = fmt::Project::default();
        p.settings.display_errors = true;
        p.scripts.push(Some(fmt::Script {
            name: "l".into(),
            last_changed: 0.0,
            code: "if (!variable_global_exists('log')) global.log = ''; global.log += argument0 + ' ';".into(),
        }));
        B { p }
    }
    fn sprite(&mut self) -> i32 {
        self.p.sprites.push(Some(square()));
        self.p.sprites.len() as i32 - 1
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
    fn obj_mut(&mut self, i: i32) -> &mut fmt::Object {
        self.p.objects[i as usize].as_mut().unwrap()
    }
    /// A room (named `rN`) with placed instances (object, instance creation code).
    fn room(&mut self, cc: &str, insts: &[(i32, &str)]) -> i32 {
        let n = self.p.rooms.len() as i32;
        let mut r = fmt::Room {
            name: format!("r{}", n),
            creation_code: cc.into(),
            ..Default::default()
        };
        for (k, &(o, icc)) in insts.iter().enumerate() {
            r.instances.push(fmt::Instance {
                x: 32 * k as i32,
                y: 32,
                object: o,
                id: 100001 + n * 1000 + k as i32,
                creation_code: icc.into(),
                locked: false,
            });
        }
        self.p.last_instance_id = 100001 + 1000 * (n + 1);
        self.p.rooms.push(Some(r));
        self.p.room_order.push(n);
        n
    }
    fn start(self) -> (Game, Vm) {
        let mut vm = Vm::new();
        let mut g = Game::new(self.p, &mut vm, true).expect("game");
        g.start(&mut vm).expect("start");
        (g, vm)
    }
}

/// The log so far; clears it.
fn take(vm: &mut Vm) -> String {
    let id = vm.intern("log");
    let s = match vm.globals().get_value(id).cloned() {
        Some(Value::Str(s)) => s.to_string_lossy(),
        _ => String::new(),
    };
    vm.globals_mut().set_value(id, Value::Str("".into()));
    s.trim_end().to_string()
}

fn step(g: &mut Game, vm: &mut Vm) {
    g.step(vm).expect("step");
}

fn objects_with_tags(b: &mut B, names: &[&str]) -> Vec<i32> {
    names
        .iter()
        .map(|t| {
            b.object(
                t,
                vec![
                    ev(STEP, 0, &[&format!("l('{}.ST');", t)]),
                    ev(STEP, END, &[&format!("l('{}.ES');", t)]),
                    ev(DRAW, 0, &[&format!("l('{}.DR');", t)]),
                    ev(OTHER, ROOM_END, &[&format!("l('{}.RE');", t)]),
                ],
            )
        })
        .collect()
}

#[test]
fn goto_finishes_action_skips_rest_of_event_and_step() {
    let mut b = B::new();
    let tags = objects_with_tags(&mut b, &["y", "x"]);
    let go = b.object(
        "go",
        vec![
            ev(STEP, 0, &["l('go.ST'); room_goto(r1); l('go.after');", "l('go.next_action');"]),
            ev(OTHER, ROOM_END, &["l('go.RE');"]),
        ],
    );
    let n = b.object("n", vec![ev(CREATE, 0, &["l('n.CR');"]), ev(OTHER, ROOM_START, &["l('n.RS');"]), ev(DRAW, 0, &["l('n.DR');"])]);
    b.room("", &[(tags[0], ""), (go, ""), (tags[1], "")]);
    b.room("l('r1.CC');", &[(n, "")]);
    let (mut g, mut vm) = b.start();
    assert_eq!(take(&mut vm), "y.DR x.DR");
    step(&mut g, &mut vm);
    assert_eq!(g.room, 1);
    assert_eq!(take(&mut vm), "y.ST go.ST go.after y.RE go.RE x.RE n.CR r1.CC n.RS n.DR");
}

#[test]
fn goto_inside_with_block_finishes_the_with() {
    let mut b = B::new();
    let t = b.object("t", vec![ev(OTHER, ROOM_END, &["l('t.RE');"])]);
    let go = b.object(
        "go",
        vec![ev(
            STEP,
            0,
            &["with (t) { l('w' + string(x)); if (x == 0) room_goto(r1); } l('go.after');", "l('go.next');"],
        )],
    );
    let late = b.object("late", vec![ev(STEP, 0, &["l('late.ST');"])]);
    b.room("", &[(t, ""), (t, ""), (go, ""), (late, "")]);
    b.room("", &[]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    step(&mut g, &mut vm);
    assert_eq!(take(&mut vm), "w0 w32 go.after t.RE t.RE");
}

#[test]
fn goto_in_script_and_user_event_stops_all_running_events() {
    let mut b = B::new();
    b.p.scripts.push(Some(fmt::Script {
        name: "jump".into(),
        last_changed: 0.0,
        code: "room_goto(r1); l('jump.after');".into(),
    }));
    let go = b.object(
        "go",
        vec![
            ev(STEP, 0, &["l('a1'); event_user(0); l('a1.after');", "l('a2');"]),
            ev(OTHER, USER0, &["l('u1'); jump(); l('u1.after');", "l('u2');"]),
        ],
    );
    let other = b.object("other", vec![ev(STEP, 0, &["l('other.ST');"])]);
    b.room("", &[(go, ""), (other, "")]);
    b.room("l('r1.CC');", &[]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    step(&mut g, &mut vm);
    assert_eq!(take(&mut vm), "a1 u1 jump.after u1.after a1.after r1.CC");
}

#[test]
fn goto_in_event_perform_and_event_inherited() {
    let mut b = B::new();
    let parent = b.object("parent", vec![ev(STEP, END, &["l('p1'); room_goto(r1);", "l('p2');"])]);
    let child = b.object(
        "child",
        vec![
            ev(STEP, 0, &["event_perform(ev_step, ev_step_end); l('c.after_perform');", "l('c2');"]),
            ev(STEP, END, &["event_inherited(); l('c.after_inherited');", "l('ce2');"]),
        ],
    );
    b.obj_mut(child).parent = parent;
    b.room("", &[(child, "")]);
    b.room("l('r1.CC');", &[(child, "")]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    step(&mut g, &mut vm);
    assert_eq!(take(&mut vm), "p1 c.after_inherited c.after_perform r1.CC");
}

#[test]
fn instance_created_after_goto_runs_its_whole_create_event() {
    let mut b = B::new();
    let made = b.object("made", vec![ev(CREATE, 0, &["l('m1');", "l('m2');"])]);
    let go = b.object("go", vec![ev(STEP, 0, &["room_goto(r1); instance_create(0, 0, made);", "l('go2');"])]);
    let _ = made;
    b.room("", &[(go, "")]);
    b.room("l('r1.CC');", &[]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    step(&mut g, &mut vm);
    assert_eq!(take(&mut vm), "m1 m2 r1.CC");
}

#[test]
fn goto_in_alarm_timeline_collision_and_draw() {
    // alarm
    let mut b = B::new();
    let a = b.object(
        "a",
        vec![ev(CREATE, 0, &["alarm[0] = 1;"]), ev(ALARM, 0, &["l('al1'); room_goto(r1);", "l('al2');"])],
    );
    let late = b.object("late", vec![ev(CREATE, 0, &["alarm[0] = 1;"]), ev(ALARM, 0, &["l('late.AL');"]), ev(STEP, 0, &["l('late.ST');"])]);
    b.room("", &[(a, ""), (late, "")]);
    b.room("l('r1.CC');", &[]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    step(&mut g, &mut vm);
    assert_eq!(take(&mut vm), "al1 r1.CC");

    // timeline moment
    let mut b = B::new();
    b.p.timelines.push(Some(fmt::Timeline {
        name: "tl".into(),
        last_changed: 0.0,
        moments: vec![fmt::Moment {
            step: 0,
            actions: vec![code("l('tl1'); room_goto(r1);"), code("l('tl2');")],
        }],
    }));
    let t = b.object(
        "t",
        vec![ev(CREATE, 0, &["timeline_index = tl; timeline_position = 0; timeline_running = true;"]), ev(STEP, BEGIN, &["l('t.BS');"])],
    );
    b.room("", &[(t, "")]);
    b.room("l('r1.CC');", &[]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    step(&mut g, &mut vm);
    assert_eq!(take(&mut vm), "tl1 r1.CC");

    // collision
    let mut b = B::new();
    let s = b.sprite();
    let wall = b.object("wall", vec![]);
    b.obj_mut(wall).sprite = s;
    let hit = b.object(
        "hit",
        vec![ev(COLLISION, wall, &["l('co1'); room_goto(r1);", "l('co2');"]), ev(STEP, END, &["l('hit.ES');"])],
    );
    b.obj_mut(hit).sprite = s;
    b.room("", &[(hit, ""), (hit, "")]);
    // both 'hit' instances overlap one wall
    b.p.rooms[0].as_mut().unwrap().instances[1].x = 0;
    b.p.rooms[0].as_mut().unwrap().instances.push(fmt::Instance {
        x: 4,
        y: 32,
        object: wall,
        id: 100010,
        creation_code: String::new(),
        locked: false,
    });
    b.room("l('r1.CC');", &[]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    step(&mut g, &mut vm);
    assert_eq!(take(&mut vm), "co1 r1.CC");

    // draw event: later draw events of the frame do not run
    let mut b = B::new();
    let d1 = b.object("d1", vec![ev(DRAW, 0, &["l('d1'); if (room == r0) room_goto(r1);", "l('d1b');"])]);
    let d2 = b.object("d2", vec![ev(DRAW, 0, &["l('d2');"])]);
    b.obj_mut(d1).depth = 10;
    b.room("", &[(d1, ""), (d2, "")]);
    b.room("l('r1.CC');", &[]);
    let (mut g, mut vm) = b.start();
    // the entry frame of r0 already changes the room
    assert_eq!(g.room, 1);
    assert_eq!(take(&mut vm), "d1 r1.CC");
    step(&mut g, &mut vm);
    assert_eq!(take(&mut vm), "");
}

#[test]
fn goto_in_create_during_room_entry() {
    let mut b = B::new();
    let first = b.object("first", vec![ev(CREATE, 0, &["l('f1'); room_goto(r1);", "l('f2');"]), ev(DRAW, 0, &["l('f.DR');"])]);
    let second = b.object("second", vec![ev(CREATE, 0, &["l('s1');", "l('s2');"]), ev(OTHER, ROOM_START, &["l('s.RS');"]), ev(OTHER, ROOM_END, &["l('s.RE');"])]);
    let n = b.object("n", vec![ev(CREATE, 0, &["l('n.CR');"]), ev(DRAW, 0, &["l('n.DR');"])]);
    b.room("l('r0.CC');", &[(first, ""), (second, "")]);
    b.room("l('r1.CC');", &[(n, "")]);
    let (g, mut vm) = b.start();
    assert_eq!(g.room, 1);
    // the entry completes (no frame is drawn for it), then the new room is entered
    assert_eq!(take(&mut vm), "f1 s1 s2 r0.CC s.RS s.RE n.CR r1.CC n.DR");
}

#[test]
fn room_entry_adds_placed_instances_before_any_create() {
    let mut b = B::new();
    let keep = b.object(
        "keep",
        vec![
            ev(CREATE, 0, &["l('k.CR');"]),
            ev(OTHER, GAME_START, &["l('k.GS');"]),
            ev(OTHER, ROOM_START, &["l('k.RS');"]),
            ev(OTHER, ROOM_END, &["l('k.RE');"]),
            ev(STEP, BEGIN, &["l('k.BS'); if (room == r0) room_goto(r1);"]),
        ],
    );
    b.obj_mut(keep).persistent = true;
    let made = b.object("made", vec![ev(CREATE, 0, &["l('m.CR');"]), ev(OTHER, ROOM_START, &["l('m.RS');"])]);
    let a = b.object(
        "a",
        vec![
            ev(CREATE, 0, &["l('a.CR' + string(instance_count)); instance_create(0, 0, made);"]),
            ev(OTHER, ROOM_START, &["l('a.RS');"]),
            ev(OTHER, GAME_START, &["l('a.GS');"]),
        ],
    );
    let c = b.object("c", vec![ev(CREATE, 0, &["l('c.CR');"]), ev(OTHER, ROOM_START, &["l('c.RS');"])]);
    b.room("l('r0.CC');", &[(keep, "")]);
    b.room("l('r1.CC');", &[(a, "l('a.IC');"), (c, "l('c.IC');")]);
    let (mut g, mut vm) = b.start();
    assert_eq!(take(&mut vm), "k.CR k.GS r0.CC k.RS");
    step(&mut g, &mut vm);
    // a, c placed; keep carried; made created by a: list order a, c, keep, made.
    // instance_count in a's Create: a, c and keep. No Game Start in later rooms.
    assert_eq!(take(&mut vm), "k.BS k.RE a.IC a.CR3 m.CR c.IC c.CR r1.CC a.RS c.RS k.RS m.RS");
    let objs: Vec<i32> = g.instances.iter().map(|i| i.object_index).collect();
    assert_eq!(objs, vec![a, c, keep, made]);
}

#[test]
fn persistent_rooms_and_placed_persistent_instances() {
    let mut b = B::new();
    // clock: persistent, moves between rooms on fixed steps
    let k = b.object(
        "k",
        vec![
            ev(CREATE, 0, &["global.t = 0;"]),
            ev(
                STEP,
                BEGIN,
                &["global.t += 1; if (global.t == 1) room_goto(r1); if (global.t == 2) room_goto(r2); if (global.t == 3) room_goto(r1); if (global.t == 4) room_goto(r2);"],
            ),
            ev(OTHER, ROOM_START, &["l('k.RS');"]),
        ],
    );
    b.obj_mut(k).persistent = true;
    let pi = b.object("pi", vec![ev(CREATE, 0, &["l('pi.CR');"]), ev(OTHER, ROOM_START, &["l('pi.RS');"])]);
    let pp = b.object(
        "pp",
        vec![
            ev(CREATE, 0, &["l('pp.CR' + string(instance_number(pp)));"]),
            ev(OTHER, ROOM_START, &["l('pp.RS' + string(instance_number(pp)));"]),
        ],
    );
    b.obj_mut(pp).persistent = true;
    let tq = b.object("tq", vec![ev(CREATE, 0, &["l('tq.CR');"]), ev(OTHER, ROOM_START, &["l('tq.RS');"])]);
    b.room("", &[(k, "")]);
    let r1 = b.room("l('r1.CC');", &[(pi, "")]);
    b.p.rooms[r1 as usize].as_mut().unwrap().persistent = true;
    b.room("l('r2.CC');", &[(pp, "l('pp.IC');"), (tq, "")]);
    let (mut g, mut vm) = b.start();
    take(&mut vm);
    step(&mut g, &mut vm); // -> r1 (first visit)
    assert_eq!(take(&mut vm), "pi.CR r1.CC pi.RS k.RS");
    step(&mut g, &mut vm); // -> r2: placed first, carried after
    assert_eq!(take(&mut vm), "pp.IC pp.CR1 tq.CR r2.CC pp.RS1 tq.RS k.RS");
    step(&mut g, &mut vm); // -> r1 re-entry: restored, then carried (pp, k)
    assert_eq!(take(&mut vm), "pi.RS pp.RS1 k.RS");
    step(&mut g, &mut vm); // -> r2 again: pp not created twice
    assert_eq!(take(&mut vm), "tq.CR r2.CC tq.RS pp.RS1 k.RS");
    assert_eq!(g.count_of(pp), 1);
}

#[test]
fn caption_shows_score_lives_health_with_single_spaces() {
    let mut b = B::new();
    let o = b.object(
        "o",
        vec![ev(
            CREATE,
            0,
            &["room_caption = 'Cap'; show_score = true; score = 12; show_lives = true; lives = 3; show_health = true; health = 50; global.c = window_get_caption();"],
        )],
    );
    b.room("", &[(o, "")]);
    let (g, mut vm) = b.start();
    assert_eq!(g.caption(), "Cap Score: 12 Lives: 3 Health: 50");
    let id = vm.intern("c");
    match vm.globals().get_value(id).cloned() {
        Some(Value::Str(s)) => assert_eq!(s.to_string_lossy(), "Cap Score: 12 Lives: 3 Health: 50"),
        v => panic!("{:?}", v),
    }
}
