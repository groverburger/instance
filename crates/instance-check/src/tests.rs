//! Unit tests with small hand-built projects.

use crate::*;
use instance_format::{
    builtin_action, event_type as ev, Action, Background, Constant, Event, Instance, Object, Room,
    Script, Sprite, Tile, Trigger,
};

fn crlf(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\n', "\r\n")
}

fn code(src: &str) -> Action {
    let mut a = builtin_action(1, 603).expect("Execute Code").instantiate(1);
    a.args[0] = crlf(src);
    a
}

/// An empty project with one room (in the room order) and argument checks on.
fn project() -> Project {
    let mut p = Project::default();
    p.settings.argument_errors = true;
    p.rooms.push(Some(Room {
        name: "rm_main".into(),
        ..Room::default()
    }));
    p.room_order = vec![0];
    p
}

fn add_object(p: &mut Project, name: &str, events: Vec<(i32, i32, &str)>) -> i32 {
    let events = events
        .into_iter()
        .map(|(t, n, src)| Event {
            event_type: t,
            number: n,
            actions: vec![code(src)],
        })
        .collect();
    p.objects.push(Some(Object {
        name: name.into(),
        events,
        ..Object::default()
    }));
    p.objects.len() as i32 - 1
}

fn add_script(p: &mut Project, name: &str, src: &str) {
    p.scripts.push(Some(Script {
        name: name.into(),
        code: crlf(src),
        ..Script::default()
    }));
}

fn place(p: &mut Project, object: i32, creation_code: &str) -> i32 {
    p.last_instance_id += 1;
    let id = p.last_instance_id;
    p.rooms[0].as_mut().unwrap().instances.push(Instance {
        x: 0,
        y: 0,
        object,
        id,
        creation_code: crlf(creation_code),
        locked: false,
    });
    id
}

fn run(p: &Project) -> Report {
    check_project(p, &Options::default())
}

fn run_with(p: &Project, pedantic: bool, lite: bool) -> Report {
    check_project(p, &Options { pedantic, lite })
}

fn find<'a>(r: &'a Report, kind: &str) -> Vec<&'a Diagnostic> {
    r.diagnostics.iter().filter(|d| d.kind == kind).collect()
}

fn has(r: &Report, kind: &str, sev: Severity, text: &str) -> bool {
    r.diagnostics
        .iter()
        .any(|d| d.kind == kind && d.severity == sev && d.message.contains(text))
}

#[track_caller]
fn assert_clean(r: &Report) {
    assert!(
        r.diagnostics.is_empty(),
        "expected no findings, got {:#?}",
        r.diagnostics
    );
}

// ------------------------------------------------------------------ clean

#[test]
fn clean_project_has_no_findings() {
    let mut p = project();
    add_script(&mut p, "scr_add", "return argument0 + argument1;");
    let o = add_object(
        &mut p,
        "obj_player",
        vec![
            (
                ev::CREATE,
                0,
                "hp = 3;\nvar i; for (i = 0; i < 3; i += 1) slot[i] = 0;\nglobal.score_total = 0;",
            ),
            (
                ev::STEP,
                0,
                "hp = scr_add(hp, 0);\nif (keyboard_check(vk_left)) x -= 2;\nalarm[0] = 5;",
            ),
            (ev::ALARM, 0, "global.score_total += slot[0];"),
            (
                ev::DRAW,
                0,
                "draw_sprite(sprite_index, image_index, x, y);\ndraw_text(8, 8, string(hp));",
            ),
        ],
    );
    place(&mut p, o, "hp = 5;");
    let r = run(&p);
    assert_clean(&r);
    assert_eq!(r.code_pieces, 6);
}

// ------------------------------------------------------------------ 1. compile errors

#[test]
fn compile_error_in_code_action_has_location() {
    let mut p = project();
    add_object(
        &mut p,
        "obj_player",
        vec![(ev::STEP, 0, "x += 1;\n\nyy = 2;\ni++;")],
    );
    let r = run(&p);
    let e = find(&r, "compile");
    assert_eq!(e.len(), 1, "{:#?}", r.diagnostics);
    let d = e[0];
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.resource, "obj_player");
    assert_eq!(d.event.as_deref(), Some("Step"));
    assert_eq!(d.action, Some(1));
    assert_eq!(d.line, Some(4));
    assert_eq!(d.col, Some(2));
    assert_eq!(d.message, "Assignment operator expected.");
    assert_eq!(d.source_line.as_deref(), Some("i++;"));
    let text = report::human(&r, false);
    assert!(
        text.contains("error: obj_player / Step / action 1, line 4 col"),
        "{}",
        text
    );
    assert!(text.contains("\n    i++;\n"), "{}", text);
    assert!(
        text.ends_with("1 error, 0 warnings in 1 code piece\n"),
        "{}",
        text
    );
}

#[test]
fn compile_errors_everywhere() {
    let mut p = project();
    add_script(&mut p, "scr_bad", "return 1 / 0;");
    p.constants.push(Constant {
        name: "BAD".into(),
        value: "'a' + 1".into(),
    });
    p.triggers.push(Some(Trigger {
        name: "trg".into(),
        condition: "global.x == 1".into(),
        moment: 0,
        constant_name: String::new(),
    }));
    let mut test = builtin_action(1, 408)
        .expect("Test Expression")
        .instantiate(1);
    test.args[0] = "x >".into();
    let mut setv = builtin_action(1, 611).expect("Set Variable").instantiate(1);
    setv.args[0] = "x".into();
    setv.args[1] = "foo(".into();
    p.objects.push(Some(Object {
        name: "obj_a".into(),
        events: vec![Event {
            event_type: ev::STEP,
            number: 0,
            actions: vec![test, setv],
        }],
        ..Object::default()
    }));
    p.rooms[0].as_mut().unwrap().creation_code = "{ x = 1; };".into();
    place(&mut p, 0, "unknown_fn(1);");
    let r = run(&p);
    let errs = find(&r, "compile");
    let msgs: Vec<String> = errs
        .iter()
        .map(|d| format!("{} | {:?} | {}", d.resource, d.event, d.message))
        .collect();
    assert_eq!(errs.len(), 7, "{:#?}", msgs);
    assert!(has(&r, "compile", Severity::Error, "Division by 0."));
    assert!(has(
        &r,
        "compile",
        Severity::Error,
        "Wrong type of arguments to +."
    ));
    assert!(has(
        &r,
        "compile",
        Severity::Error,
        "Program ends before end of the code."
    ));
    assert!(has(
        &r,
        "compile",
        Severity::Error,
        "Unknown function or script: unknown_fn"
    ));
    let trig = errs
        .iter()
        .find(|d| d.resource == "trigger trg")
        .expect("trigger");
    assert_eq!(trig.message, "Assignment operator expected.");
    let arg = errs
        .iter()
        .find(|d| d.event.as_deref() == Some("Step (argument 1)"))
        .expect("test expression");
    assert_eq!(arg.action, Some(1));
    assert!(errs
        .iter()
        .any(|d| d.event.as_deref() == Some("Step (Set Variable)") && d.action == Some(2)));
    assert!(errs
        .iter()
        .any(|d| d.resource == "rm_main" && d.event.as_deref() == Some("creation code")));
    assert!(errs.iter().any(|d| d.resource == "rm_main"
        && d.event.as_deref() == Some("instance 100001 (obj_a) creation code")));
}

#[test]
fn unknown_function_with_extension_packages_is_a_warning() {
    let mut p = project();
    p.extension_packages.push("MyExt".into());
    add_object(&mut p, "obj_a", vec![(ev::CREATE, 0, "ext_do_thing(1);")]);
    let r = run(&p);
    assert_eq!(r.errors(), 0);
    assert!(has(&r, "compile", Severity::Warning, "extension package"));
}

#[test]
fn timeline_moment_location() {
    let mut p = project();
    p.timelines.push(Some(instance_format::Timeline {
        name: "tl_intro".into(),
        moments: vec![instance_format::Moment {
            step: 30,
            actions: vec![code("a = ;")],
        }],
        ..Default::default()
    }));
    let r = run(&p);
    let d = find(&r, "compile")[0];
    assert_eq!(d.resource, "tl_intro");
    assert_eq!(d.event.as_deref(), Some("moment 30"));
    assert_eq!(d.action, Some(1));
}

// ------------------------------------------------------------------ 2. script arguments

#[test]
fn script_argument_counts() {
    let mut p = project();
    add_script(&mut p, "scr_two", "return argument0 + argument1;");
    add_script(&mut p, "scr_none", "return 1;");
    add_script(&mut p, "scr_var", "var i, s; s = argument0; for (i = 1; i < argument_count; i += 1) s += argument[i]; return s;");
    add_object(
        &mut p,
        "obj_a",
        vec![(
            ev::CREATE,
            0,
            "a = scr_two(1, 2);\nb = scr_two(1);\nc = scr_none(5);\nd = scr_var(1, 2, 3);\ne = scr_var();\nf = script_execute(scr_two, 1, 2, 3);\ng = a + b + c + d + e + f;",
        )],
    );
    let r = run(&p);
    let e = find(&r, "script-args");
    let lines: Vec<u32> = e.iter().map(|d| d.line.unwrap()).collect();
    assert_eq!(lines, vec![2, 3, 5, 6], "{:#?}", e);
    assert!(e[0].message.contains("requires 2 arguments, 1 supplied"));
    assert!(e[2].message.contains("at least 1"));
    assert!(e[3].message.contains("script_execute"));

    p.settings.argument_errors = false;
    assert!(find(&run(&p), "script-args").is_empty());
}

#[test]
fn builtin_arity_is_a_compile_error() {
    let mut p = project();
    add_object(&mut p, "obj_a", vec![(ev::CREATE, 0, "a = abs(1, 2);")]);
    assert!(has(
        &run(&p),
        "compile",
        Severity::Error,
        "Wrong number of arguments to function abs."
    ));
}

// ------------------------------------------------------------------ 3. structure

#[test]
fn bad_references() {
    let mut p = project();
    p.sprites.push(None); // deleted slot 0
    p.backgrounds.push(Some(Background {
        name: "bg_a".into(),
        ..Default::default()
    }));
    p.objects.push(Some(Object {
        name: "obj_a".into(),
        sprite: 0,
        mask_sprite: 7,
        parent: 9,
        events: vec![Event {
            event_type: ev::COLLISION,
            number: 4,
            actions: vec![],
        }],
        ..Object::default()
    }));
    {
        let r = p.rooms[0].as_mut().unwrap();
        r.instances.push(Instance {
            object: 3,
            id: 100001,
            ..Default::default()
        });
        r.instances.push(Instance {
            object: 0,
            id: 100001,
            ..Default::default()
        });
        r.tiles.push(Tile {
            background: 2,
            id: 10000001,
            ..Default::default()
        });
        r.backgrounds[0].background = 5;
        r.views[0].follow_object = 8;
    }
    p.last_instance_id = 100000;
    p.last_tile_id = 10000001;
    p.room_order = vec![0, 4];
    let r = run(&p);
    let msgs: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
    for want in [
        "sprite index 0 is a deleted slot",
        "mask sprite index 7 is out of range",
        "parent object index 9 is out of range",
        "collision event with object index 4",
        "instance of object index 3",
        "duplicate instance id 100001",
        "last_instance_id is 100000",
        "tile uses background index 2",
        "background layer uses background index 5",
        "view follows object index 8",
        "room order entry 1 is room index 4",
    ] {
        assert!(
            msgs.iter().any(|m| m.contains(want)),
            "missing {:?} in {:#?}",
            want,
            msgs
        );
    }
    assert!(r.diagnostics.iter().all(|d| d.severity == Severity::Error));
}

#[test]
fn parent_cycle_and_bad_action_references() {
    let mut p = project();
    add_object(&mut p, "obj_a", vec![]);
    add_object(&mut p, "obj_b", vec![]);
    p.objects[0].as_mut().unwrap().parent = 1;
    p.objects[1].as_mut().unwrap().parent = 0;
    let mut create = builtin_action(1, 201)
        .expect("Create Instance")
        .instantiate(1);
    create.args[0] = "12".into(); // object argument
    create.applies_to = 9;
    p.objects[0].as_mut().unwrap().events.push(Event {
        event_type: ev::STEP,
        number: 0,
        actions: vec![create],
    });
    let r = run(&p);
    assert!(has(&r, "reference", Severity::Error, "cycle"));
    assert!(has(
        &r,
        "reference",
        Severity::Error,
        "argument 1 refers to object index 12"
    ));
    assert!(has(
        &r,
        "reference",
        Severity::Error,
        "\"applies to\" object index 9"
    ));
}

#[test]
fn name_clashes() {
    let mut p = project();
    p.sprites.push(Some(Sprite {
        name: "thing".into(),
        ..Sprite::default()
    }));
    add_object(&mut p, "thing", vec![]);
    add_object(&mut p, "score", vec![]);
    add_object(&mut p, "c_red", vec![]);
    add_object(&mut p, "while", vec![]);
    add_script(&mut p, "abs", "return argument0;");
    p.constants.push(Constant {
        name: "rm_main".into(),
        value: "1".into(),
    });
    let r = run(&p);
    let clash = |t: &str| has(&r, "name-clash", Severity::Error, t);
    assert!(clash("object name thing is also used by a sprite"));
    assert!(clash("score is a built-in variable"));
    assert!(clash("replaces the built-in constant c_red"));
    assert!(clash("while is a GML keyword"));
    assert!(clash("script abs replaces the built-in function abs"));
    assert!(has(
        &r,
        "name-clash",
        Severity::Warning,
        "the resource wins"
    ));
}

// ------------------------------------------------------------------ 4. read but never assigned

#[test]
fn variables_read_but_never_assigned() {
    let mut p = project();
    add_script(
        &mut p,
        "scr_get",
        "var t; t = argument0; return t + argument[0] + argument_count;",
    );
    add_object(
        &mut p,
        "obj_a",
        vec![
            (ev::CREATE, 0, "hp = 1;\nglobal.lvl = 1;\nglobalvar gv; gv = 2;\nvariable_global_set('dyn', 3);\nexecute_string('es = 1;');"),
            (ev::STEP, 0, "var k; k = hp + global.lvl + gv + global.gv + global.dyn + es + scr_get(1);\nx = k + speed + room_speed + c_red + pi;\nother.marked = 1;\nm = marked;"),
            (ev::DRAW, 0, "draw_text(0, 0, string(hpp) + string(global.levl) + string(other.zzz));\nwith (obj_a) q = 1;\nn = obj_a.q;"),
        ],
    );
    let r = run(&p);
    let w = find(&r, "unassigned");
    let msgs: Vec<&str> = w.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(msgs.len(), 3, "{:#?}", msgs);
    assert!(msgs[0].starts_with("hpp is read but never assigned"));
    assert!(msgs[1].starts_with("global.levl is read"));
    assert!(msgs[2].starts_with("zzz is read"));
    assert_eq!(w[0].event.as_deref(), Some("Draw"));
    assert!(w.iter().all(|d| d.severity == Severity::Warning));
    assert_eq!(r.errors(), 0);
}

#[test]
fn var_locals_do_not_carry_over_to_the_next_action() {
    let mut p = project();
    p.objects.push(Some(Object {
        name: "obj_a".into(),
        events: vec![Event {
            event_type: ev::CREATE,
            number: 0,
            actions: vec![code("var t; t = 1;"), code("x = t;")],
        }],
        ..Object::default()
    }));
    let r = run(&p);
    let w = find(&r, "unassigned");
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].action, Some(2));
}

#[test]
fn compound_assignment_alone_is_not_an_initialisation() {
    let mut p = project();
    add_object(&mut p, "obj_a", vec![(ev::STEP, 0, "counter += 1;")]);
    assert!(has(
        &run(&p),
        "unassigned",
        Severity::Warning,
        "counter is read"
    ));
}

// ------------------------------------------------------------------ 5. possibly uninitialised

#[test]
fn possibly_uninitialised_is_pedantic() {
    let mut p = project();
    add_script(&mut p, "scr_init", "speedup = 1;");
    let parent = add_object(
        &mut p,
        "obj_parent",
        vec![(ev::CREATE, 0, "from_parent = 1;")],
    );
    let o = add_object(
        &mut p,
        "obj_a",
        vec![
            (ev::CREATE, 0, "event_inherited();\nscr_init();"),
            (ev::ALARM, 0, "late = 1;\nlist[3] = 1;"),
            (
                ev::STEP,
                0,
                "x += late + from_parent + speedup + pushed + list[0];",
            ),
        ],
    );
    p.objects[o as usize].as_mut().unwrap().parent = parent;
    add_object(
        &mut p,
        "obj_b",
        vec![(ev::CREATE, 0, "with (obj_a) pushed = 1;")],
    );
    assert!(find(&run(&p), "uninitialised").is_empty());
    let r = run_with(&p, true, false);
    let w = find(&r, "uninitialised");
    assert_eq!(w.len(), 1, "{:#?}", w);
    assert!(w[0]
        .message
        .starts_with("late is possibly uninitialised: obj_a"));
    assert_eq!(w[0].event.as_deref(), Some("Step"));

    // instance creation code initialises it too
    place(&mut p, o, "late = 0;");
    assert!(find(&run_with(&p, true, false), "uninitialised").is_empty());
}

// ------------------------------------------------------------------ 6. short-circuit

#[test]
fn no_short_circuit_hazards() {
    let mut p = project();
    add_object(
        &mut p,
        "obj_a",
        vec![(
            ev::STEP,
            0,
            "target = noone;\nif (instance_exists(target) && target.hp > 0) x = 1;\nif (target != noone and target.hp > 0) x = 2;\nif (!instance_exists(target) || target.hp <= 0) x = 3;\nif (instance_exists(target)) { if (target.hp > 0) x = 4; }\nif (instance_exists(target) && hp > 0) x = 5;",
        )],
    );
    let r = run(&p);
    let w = find(&r, "short-circuit");
    let lines: Vec<u32> = w.iter().map(|d| d.line.unwrap()).collect();
    assert_eq!(lines, vec![2, 3, 4], "{:#?}", w);
    assert!(w[0].message.starts_with("target.hp is evaluated"));
}

// ------------------------------------------------------------------ 7. Lite

#[test]
fn lite_features_only_with_flag() {
    let mut p = project();
    add_object(
        &mut p,
        "obj_a",
        vec![
            (
                ev::CREATE,
                0,
                "l = ds_list_create();\nds_list_add(l, 1);\nimage_blend = c_red;",
            ),
            (
                ev::DRAW,
                0,
                "draw_sprite_ext(sprite_index, 0, x, y, 1, 1, 0, c_white, 1);\nds_list_destroy(l);",
            ),
        ],
    );
    let xform = builtin_action(1, 542)
        .expect("Transform Sprite")
        .instantiate(1);
    p.objects[0].as_mut().unwrap().events[0].actions.push(xform);
    assert!(find(&run(&p), "lite").is_empty());
    let r = run_with(&p, false, true);
    let w = find(&r, "lite");
    let msgs: Vec<&str> = w.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(msgs.len(), 4, "{:#?}", msgs);
    assert!(msgs
        .iter()
        .any(|m| m.starts_with("ds_* (data structure functions)")
            && m.ends_with("(first of 3 uses)")));
    assert!(msgs.iter().any(|m| m.contains("image_blend")));
    assert!(msgs.iter().any(|m| m.starts_with("draw_sprite_ext")));
    assert!(msgs.iter().any(|m| m.contains("Transform Sprite")));
}

// ------------------------------------------------------------------ 8. line endings

#[test]
fn lf_only_code_gives_one_summary_warning() {
    let mut p = project();
    add_object(
        &mut p,
        "obj_a",
        vec![(ev::CREATE, 0, "a = 1;"), (ev::STEP, 0, "a = 2;")],
    );
    for e in &mut p.objects[0].as_mut().unwrap().events {
        e.actions[0].args[0] = "a = 1;\nb = a;".into();
    }
    let r = run(&p);
    let w = find(&r, "line-endings");
    assert_eq!(w.len(), 1);
    assert!(
        w[0].message.starts_with("2 code piece(s) use LF-only"),
        "{}",
        w[0].message
    );
    assert_eq!(w[0].resource, "project");
}

// ------------------------------------------------------------------ names and output

#[test]
fn event_names_are_human_friendly() {
    let mut p = project();
    let wall = add_object(&mut p, "obj_wall", vec![]);
    p.triggers.push(Some(Trigger {
        name: "trg_boss".into(),
        condition: "return 0;".into(),
        moment: 0,
        constant_name: "ev_boss".into(),
    }));
    add_object(
        &mut p,
        "obj_a",
        vec![
            (ev::COLLISION, wall, "a ="),
            (ev::KEYBOARD, 37, "a ="),
            (ev::KEY_PRESS, 32, "a ="),
            (ev::MOUSE, 4, "a ="),
            (ev::OTHER, 4, "a ="),
            (ev::OTHER, 12, "a ="),
            (ev::STEP, 1, "a ="),
            (ev::ALARM, 3, "a ="),
            (ev::TRIGGER, 0, "a ="),
        ],
    );
    let r = run(&p);
    let events: Vec<&str> = find(&r, "compile")
        .iter()
        .map(|d| d.event.as_deref().unwrap())
        .collect();
    assert_eq!(
        events,
        vec![
            "Collision with obj_wall",
            "Keyboard <Left>",
            "Key Press <Space>",
            "Mouse Left Pressed",
            "Other: Room Start",
            "User Defined 2",
            "Begin Step",
            "Alarm 3",
            "Trigger trg_boss"
        ]
    );
}

#[test]
fn trigger_constant_name_is_a_constant() {
    let mut p = project();
    p.triggers.push(Some(Trigger {
        name: "trg".into(),
        condition: "return 0;".into(),
        moment: 0,
        constant_name: "ev_mine".into(),
    }));
    add_object(
        &mut p,
        "obj_a",
        vec![(ev::CREATE, 0, "event_perform(ev_trigger, ev_mine);")],
    );
    assert_clean(&run(&p));
}

#[test]
fn json_output_fields() {
    let mut p = project();
    add_object(&mut p, "obj_a", vec![(ev::STEP, 0, "i++;")]);
    let r = run(&p);
    let v: serde_json::Value = serde_json::from_str(&report::json(&r)).unwrap();
    let d = &v[0];
    assert_eq!(d["severity"], "error");
    assert_eq!(d["kind"], "compile");
    assert_eq!(d["resource"], "obj_a");
    assert_eq!(d["event"], "Step");
    assert_eq!(d["action"], 1);
    assert_eq!(d["line"], 1);
    assert_eq!(d["message"], "Assignment operator expected.");
    assert_eq!(d["source_line"], "i++;");
    assert_eq!(
        report::human(&r, true),
        "1 error, 0 warnings in 1 code piece\n"
    );
}

#[test]
fn no_rooms_is_an_error() {
    let mut p = Project::default();
    p.room_order.clear();
    assert!(has(&run(&p), "room-order", Severity::Error, "no rooms"));
}
