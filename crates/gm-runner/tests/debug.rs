//! Debug mode (specs/ide.md §20.3) driven headlessly through the programmatic
//! API the Debug Information window uses: show_debug_message capture, watches,
//! pause/step, Execute code, inspection lists, restart/end.

use gm_format as fmt;
use gm_runner::debug::{Command, Scope, Wants};
use gm_runner::{frontend, Game};
use gml::Vm;

fn code(src: &str) -> fmt::Action {
    let mut a = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    a.args[0] = src.to_string();
    a
}

fn event(t: i32, n: i32, src: &str) -> fmt::Event {
    fmt::Event { event_type: t, number: n, actions: vec![code(src)] }
}

/// One room with an instance of `obj_counter` (id 100001) at (32, 48) that
/// counts its steps in `count` and `global.steps`.
fn project() -> fmt::Project {
    let mut p = fmt::Project::default();
    p.settings.display_errors = true;
    p.objects.push(Some(fmt::Object {
        name: "obj_counter".into(),
        sprite: -1,
        visible: true,
        events: vec![
            event(0, 0, "count = 0; global.steps = 0; show_debug_message('created, debug_mode=' + string(debug_mode));"),
            event(3, 0, "count += 1; global.steps += 1; if (count == 2) show_debug_message('second step');"),
        ],
        ..Default::default()
    }));
    let mut r = fmt::Room { name: "room_main".into(), ..Default::default() };
    r.instances.push(fmt::Instance { x: 32, y: 48, object: 0, id: 100001, creation_code: String::new(), locked: false });
    p.last_instance_id = 101000;
    p.rooms.push(Some(r));
    p.room_order.push(0);
    p
}

fn start(debug: bool) -> (Game, Vm) {
    let mut vm = Vm::new();
    let mut g = Game::new(project(), &mut vm, true).expect("game");
    if debug {
        g.enable_debug();
    }
    g.start(&mut vm).expect("start");
    (g, vm)
}

fn messages(g: &Game) -> Vec<String> {
    g.debug.as_ref().unwrap().messages.iter().cloned().collect()
}

#[test]
fn show_debug_message_is_logged_in_debug_mode() {
    let (mut g, mut vm) = start(true);
    for _ in 0..3 {
        assert!(frontend::tick(&mut g, &mut vm).unwrap());
    }
    assert_eq!(messages(&g), ["created, debug_mode=1", "second step"]);
    let d = g.debug.as_ref().unwrap();
    assert_eq!(d.messages_since(1), ["second step"]);
    assert!(d.messages_since(2).is_empty());

    // without debug mode: debug_mode is false and nothing is kept
    let (mut g, mut vm) = start(false);
    assert!(frontend::tick(&mut g, &mut vm).unwrap());
    assert!(g.debug.is_none());
    assert_eq!(g.debug_eval(&mut vm, "debug_mode", &Scope::Global).unwrap(), "0");
}

#[test]
fn watches_are_evaluated_each_step() {
    let (mut g, mut vm) = start(true);
    g.debug_command(&mut vm, Command::AddWatch("global.steps * 10".into(), Scope::Global));
    g.debug_command(&mut vm, Command::AddWatch("count".into(), Scope::Object("obj_counter".into())));
    g.debug_command(&mut vm, Command::AddWatch("x + y".into(), Scope::Target(100001)));
    g.debug_command(&mut vm, Command::AddWatch("nosuchvar".into(), Scope::Target(100001)));
    g.debug_command(&mut vm, Command::AddWatch("1 +".into(), Scope::Global));
    g.debug_command(&mut vm, Command::AddWatch("x".into(), Scope::Target(123456)));
    let vals = |g: &Game| g.debug.as_ref().unwrap().watches.iter().map(|w| w.value.clone()).collect::<Vec<_>>();
    // evaluated as soon as they are added
    assert_eq!(vals(&g)[0], Ok("0".to_string()));
    frontend::tick(&mut g, &mut vm).unwrap();
    frontend::tick(&mut g, &mut vm).unwrap();
    let v = vals(&g);
    assert_eq!(v[0], Ok("20".to_string()));
    assert_eq!(v[1], Ok("2".to_string()));
    assert_eq!(v[2], Ok("80".to_string()));
    assert!(v[3].as_ref().is_err_and(|e| e.contains("nosuchvar")), "{:?}", v[3]);
    assert!(v[4].is_err(), "compile error expected");
    assert!(v[5].as_ref().is_err_and(|e| e.contains("123456")), "{:?}", v[5]);
    // watch errors do not leak into the game's error state
    assert!(!g.error_occurred);

    g.debug_command(&mut vm, Command::ChangeWatch(0, "'n=' + string(global.steps)".into(), Scope::Global));
    assert_eq!(vals(&g)[0], Ok("\"n=2\"".to_string()));
    g.debug_command(&mut vm, Command::DeleteWatch(1));
    assert_eq!(g.debug.as_ref().unwrap().watches.len(), 5);
    let snap = g.debug_snapshot(&vm, &Wants::default());
    assert_eq!(snap.watches[0].0, "'n=' + string(global.steps)");
    assert_eq!(snap.watches[1].1, "100001");
    g.debug_command(&mut vm, Command::ClearWatches);
    assert!(g.debug.as_ref().unwrap().watches.is_empty());
}

#[test]
fn pause_and_single_step() {
    let (mut g, mut vm) = start(true);
    frontend::tick(&mut g, &mut vm).unwrap();
    assert_eq!(g.frame, 1);
    g.debug_command(&mut vm, Command::Pause);
    for _ in 0..5 {
        assert!(!frontend::tick(&mut g, &mut vm).unwrap(), "paused: no step");
    }
    assert_eq!(g.frame, 1);
    assert!(g.debug_info().paused);
    // one step per Step command
    g.debug_command(&mut vm, Command::Step);
    assert!(frontend::tick(&mut g, &mut vm).unwrap());
    assert!(!frontend::tick(&mut g, &mut vm).unwrap());
    assert_eq!(g.frame, 2);
    g.debug_command(&mut vm, Command::Step);
    g.debug_command(&mut vm, Command::Step);
    frontend::tick(&mut g, &mut vm).unwrap();
    frontend::tick(&mut g, &mut vm).unwrap();
    frontend::tick(&mut g, &mut vm).unwrap();
    assert_eq!(g.frame, 4);
    g.debug_command(&mut vm, Command::Run);
    assert!(frontend::tick(&mut g, &mut vm).unwrap());
    assert_eq!(g.frame, 5);
    // Step while running pauses
    g.debug_command(&mut vm, Command::Step);
    assert!(!frontend::tick(&mut g, &mut vm).unwrap());
    assert_eq!(g.frame, 5);
}

#[test]
fn execute_code_and_inspection() {
    let (mut g, mut vm) = start(true);
    frontend::tick(&mut g, &mut vm).unwrap();
    g.debug_command(&mut vm, Command::Execute("global.answer = 42; show_debug_message('hi ' + string(id));".into(), Scope::Target(100001)));
    g.debug_command(&mut vm, Command::Execute("x = 100;".into(), Scope::Object("obj_counter".into())));
    g.debug_command(&mut vm, Command::Execute("instance_create(5, 6, obj_counter);".into(), Scope::Global));
    g.debug_command(&mut vm, Command::Execute("this is not gml".into(), Scope::Global));
    let m = messages(&g);
    assert!(m.contains(&"hi 100001".to_string()), "{m:?}");
    assert!(m.iter().any(|l| l.starts_with("> this is not gml") && l.contains("error")), "{m:?}");
    assert_eq!(g.debug_eval(&mut vm, "x", &Scope::Target(100001)).unwrap(), "100");

    let wants = Wants { globals: true, instances: true, locals: Some(Scope::parse("100001")) };
    let s = g.debug_snapshot(&vm, &wants);
    assert!(s.globals.contains(&("global.answer".into(), "42".into())), "{:?}", s.globals);
    assert_eq!(s.instances.len(), 2);
    assert_eq!((s.instances[0].id, s.instances[0].object.as_str(), s.instances[0].x), (100001, "obj_counter", 100.0));
    assert_eq!((s.instances[1].x, s.instances[1].y), (5.0, 6.0));
    let (id, vars) = s.locals.unwrap().unwrap();
    assert_eq!(id, 100001);
    assert!(vars.contains(&("x".into(), "100".into())), "{vars:?}");
    assert!(vars.contains(&("count".into(), "1".into())), "{vars:?}");
    assert_eq!(s.info.room, "room_main");
    assert_eq!(s.info.instances, 2);
    assert_eq!(s.info.step, 1);
    let bad = g.debug_snapshot(&vm, &Wants { locals: Some(Scope::parse("obj_nothing")), ..Default::default() });
    assert!(bad.locals.unwrap().is_err());

    // game speed
    g.debug_command(&mut vm, Command::SetSpeed(5.0));
    assert_eq!(g.debug_info().room_speed, 5.0);
}

#[test]
fn restart_and_end() {
    let (mut g, mut vm) = start(true);
    for _ in 0..3 {
        frontend::tick(&mut g, &mut vm).unwrap();
    }
    g.debug_command(&mut vm, Command::Execute("instance_create(0, 0, obj_counter);".into(), Scope::Global));
    assert_eq!(g.debug_instances().len(), 2);
    g.debug_command(&mut vm, Command::Restart);
    assert_eq!(g.debug_instances().len(), 1);
    assert_eq!(g.debug_eval(&mut vm, "global.steps", &Scope::Global).unwrap(), "0");
    assert!(!g.ended);
    g.debug_command(&mut vm, Command::End);
    assert!(g.ended);
}

/// The runner binary prints show_debug_message output on stderr (debug_mode
/// true with `--debug`), and output redirected by the caller is captured.
#[test]
fn runner_prints_debug_messages() {
    let dir = std::env::temp_dir().join(format!("gm81_debug_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("debug.gm81");
    fmt::write_project_file(&path, &project(), fmt::FormatVersion::V810).unwrap();
    for (debug, expect) in [(false, "0"), (true, "1")] {
        let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_gm-runner"));
        c.arg(&path).args(["--headless", "--frames", "3"]);
        if debug {
            c.arg("--debug");
        }
        let out = c.output().unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{err}");
        assert!(err.contains(&format!("[debug] created, debug_mode={expect}")), "{err}");
        assert!(err.contains("[debug] second step"), "{err}");
    }
    // stdout is captured too (--help prints there)
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_gm-runner")).arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("--debug"));
    let _ = std::fs::remove_dir_all(&dir);
}
