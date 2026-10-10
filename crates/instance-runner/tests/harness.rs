//! The headless test harness: `--exec` / `--watch` context, `--exec-room`,
//! `--no-draw`, `--save-dir`, `--seed`, `--screenshot-at`, `--audio-out` and
//! `--log`, in process and through the runner binary.

use gml::{Value, Vm};
use instance_format as fmt;
use instance_runner::harness::{Exec, Watch};
use instance_runner::Game;

fn code(src: &str) -> fmt::Action {
    let mut a = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    a.args[0] = src.to_string();
    a
}

fn event(t: i32, n: i32, src: &str) -> fmt::Event {
    fmt::Event { event_type: t, number: n, actions: vec![code(src)] }
}

/// A 2-frame 4x4 sprite.
fn sprite() -> fmt::Sprite {
    let frame = |c: u8| fmt::RgbaImage { width: 4, height: 4, data: [c, 0, 0, 255].repeat(16) };
    fmt::Sprite { name: "spr".into(), frames: vec![frame(200), frame(100)], ..Default::default() }
}

/// Rooms `room_a` (obj_ctrl, obj_anim) and `room_b` (obj_ctrl). obj_ctrl
/// counts steps and draws, obj_anim counts Animation End events.
fn project() -> fmt::Project {
    let mut p = fmt::Project::default();
    p.settings.display_errors = true;
    p.sprites.push(Some(sprite()));
    p.objects.push(Some(fmt::Object {
        name: "obj_ctrl".into(),
        sprite: -1,
        visible: true,
        events: vec![
            // (Create runs before Game Start)
            event(0, 0, "steps = 0; draws = 0; global.made = 1; if (!variable_global_exists('gs')) global.gs = 0;"),
            event(7, 2, "global.gs += 1;"),
            event(3, 0, "steps += 1;"),
            // logic in the Draw event must still run with --no-draw
            event(8, 0, "draws += 1; draw_set_color(c_white); draw_rectangle(0, 0, 10, 10, false);"),
        ],
        ..Default::default()
    }));
    p.objects.push(Some(fmt::Object {
        name: "obj_anim".into(),
        sprite: 0,
        visible: true,
        events: vec![event(0, 0, "image_speed = 0.5; ends = 0;"), event(7, 7, "ends += 1;")],
        ..Default::default()
    }));
    let mut a = fmt::Room { name: "room_a".into(), width: 64, height: 48, ..Default::default() };
    a.instances.push(fmt::Instance { x: 0, y: 0, object: 0, id: 100001, creation_code: String::new(), locked: false });
    a.instances.push(fmt::Instance { x: 20, y: 20, object: 1, id: 100002, creation_code: String::new(), locked: false });
    let mut b = fmt::Room { name: "room_b".into(), width: 64, height: 48, ..Default::default() };
    b.instances.push(fmt::Instance { x: 5, y: 5, object: 0, id: 100003, creation_code: String::new(), locked: false });
    p.last_instance_id = 101000;
    p.rooms.push(Some(a));
    p.rooms.push(Some(b));
    p.room_order = vec![0, 1];
    p
}

fn start(room: Option<&str>, direct: bool) -> (Game, Vm) {
    let mut vm = Vm::new();
    let mut g = Game::new(project(), &mut vm, true).expect("game");
    let r = room.map(|n| g.room_by_name(n).expect("room"));
    g.start_in(&mut vm, r, direct).expect("start");
    (g, vm)
}

fn eval(g: &mut Game, vm: &mut Vm, expr: &str) -> Value {
    let p = vm.compile_expression("t", expr.as_bytes(), &*g).expect("compiles");
    g.harness_eval(vm, &p).expect("evaluates")
}

fn real(v: Value) -> f64 {
    match v {
        Value::Real(r) => r,
        v => panic!("{v:?}"),
    }
}

#[test]
fn exec_context_is_the_first_instance() {
    let (mut g, mut vm) = start(None, false);
    // self = first instance (obj_ctrl, 100001), other = self
    g.harness_exec(&mut vm, "x = 33; global.who = id; global.oth = other.id;").unwrap();
    assert_eq!(g.inst(100001).unwrap().x, 33.0);
    assert_eq!(real(eval(&mut g, &mut vm, "global.who")), 100001.0);
    assert_eq!(real(eval(&mut g, &mut vm, "global.oth")), 100001.0);
    // with() reaches other objects; errors come back, the game's error state is untouched
    g.harness_exec(&mut vm, "with (obj_anim) y = 7;").unwrap();
    assert_eq!(g.inst(100002).unwrap().y, 7.0);
    assert!(g.harness_exec(&mut vm, "global.z = nosuch + 1;").is_err());
    assert!(g.harness_exec(&mut vm, "this is not gml").is_err());
    // execs run before the step they name
    let execs = vec![
        Exec { step: 0, origin: "--exec".into(), code: "global.at0 = steps;".into() },
        Exec { step: 2, origin: "--exec".into(), code: "global.at2 = steps; global.bad = 1 + undefined_thing;".into() },
    ];
    for _ in 0..3 {
        g.harness_run_execs(&mut vm, &execs);
        g.step(&mut vm).unwrap();
    }
    assert_eq!(real(eval(&mut g, &mut vm, "global.at0")), 0.0);
    assert_eq!(real(eval(&mut g, &mut vm, "global.at2")), 2.0);
    assert!(!g.ended && !g.aborted, "an exec error does not stop the game");
    // no instances: the global context
    g.harness_exec(&mut vm, "with (all) instance_destroy();").unwrap();
    assert_eq!(g.harness_ctx().self_id, gml::special::GLOBAL);
    g.harness_exec(&mut vm, "global.g = instance_number(obj_ctrl) + 5;").unwrap();
    assert_eq!(real(eval(&mut g, &mut vm, "global.g")), 5.0);
    let mut w = vec![Watch::new(1, "global.g * 2"), Watch::new(2, "nosuch")];
    g.harness_watch(&mut vm, &mut w); // prints on stderr; must not panic or set error state
    assert!(!g.error_occurred);
}

#[test]
fn exec_room_keeps_first_room_setup() {
    let (mut g, mut vm) = start(Some("room_b"), false);
    assert_eq!(g.room, 1);
    assert_eq!(real(eval(&mut g, &mut vm, "global.made")), 1.0);
    assert_eq!(real(eval(&mut g, &mut vm, "global.gs")), 1.0, "Game Start ran once, in the first room");
    assert_eq!(g.instances.iter().filter(|i| i.alive()).count(), 1);
    let (mut g, mut vm) = start(Some("room_b"), true);
    assert_eq!(g.room, 1);
    assert_eq!(real(eval(&mut g, &mut vm, "global.gs")), 1.0, "Game Start ran in room_b");
    assert!(g.inst(100001).is_none());
    assert_eq!(g.room_by_name("ROOM_A"), Some(0));
    assert_eq!(g.room_by_name("1"), Some(1));
    assert_eq!(g.room_by_name("room_c"), None);
}

#[test]
fn no_draw_runs_draw_events_and_animation() {
    let run = |skip: bool| {
        let (mut g, mut vm) = start(None, false);
        if skip {
            let inner = std::mem::replace(&mut g.gfx.be, Box::new(instance_runner::draw::soft::SoftBackend::default()));
            g.gfx.be = Box::new(instance_runner::draw::discard::DiscardBackend { inner, discard: false });
        }
        let first = g.frame_image();
        g.skip_render = skip;
        for _ in 0..10 {
            g.step(&mut vm).unwrap();
        }
        let draws = real(eval(&mut g, &mut vm, "obj_ctrl.draws"));
        let ends = real(eval(&mut g, &mut vm, "obj_anim.ends"));
        let idx = g.inst(100002).unwrap().image_index;
        let img = g.frame_image();
        (draws, ends, idx, first.px == img.px)
    };
    let normal = run(false);
    let skipped = run(true);
    assert_eq!(normal.0, 11.0, "room entry + 10 steps");
    assert_eq!((normal.0, normal.1, normal.2), (skipped.0, skipped.1, skipped.2));
    assert_eq!(normal.1, 2.0, "Animation End every 4 steps (2 frames, speed 0.5)");
    assert!(skipped.3, "the back buffer keeps the room-entry frame");
}

/// The runner binary with every harness option on a project in its own folder.
#[test]
fn runner_harness_options() {
    let dir = std::env::temp_dir().join(format!("gm81_harness_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut p = project();
    // writes a save with a relative name; reads a bundled data file
    p.objects[0].as_mut().unwrap().events.push(event(
        7,
        4, // Room Start: reads data.txt (next to the game) and appends to save.txt
        "var f; f = file_text_open_read('data.txt'); global.data = file_text_read_string(f); file_text_close(f); \
         f = file_text_open_append('save.txt'); file_text_write_string(f, global.data); file_text_writeln(f); file_text_close(f); \
         ini_open('cfg.ini'); ini_write_real('a', 'b', irandom(1000000)); ini_close(); \
         show_debug_message('rnd ' + string(irandom(1000000)));",
    ));
    let game = dir.join("g.gm81");
    fmt::write_project_file(&game, &p, fmt::FormatVersion::V810).unwrap();
    std::fs::write(dir.join("data.txt"), "bundled\r\n").unwrap();
    std::fs::write(dir.join("save.txt"), "original\r\n").unwrap();
    let gml_file = dir.join("t.gml");
    std::fs::write(&gml_file, "with (obj_ctrl) {\n  x = 40;\n}\n").unwrap();
    let run = |extra: &[&str]| {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_instance-runner"))
            .current_dir(&dir)
            .arg(&game)
            .args(["--headless", "--frames", "20", "--save-dir", "sd", "--seed", "5"])
            .args(["--exec", "3:global.cash = 100000", "--exec-file", &format!("5:{}", gml_file.display())])
            .args(["--watch", "10:global.cash", "--watch", "10:obj_ctrl.x", "--watch", "5:nosuch"])
            .args(["--screenshot-at", "4,8", "shots", "--no-draw", "--log", "run.log", "--audio-out", "a.wav"])
            .args(extra)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        assert!(out.status.success(), "{err}");
        err
    };
    let err = run(&[]);
    assert!(err.contains("watch step 10: global.cash = 100000"), "{err}");
    assert!(err.contains("watch step 20: obj_ctrl.x = 40"), "{err}");
    assert_eq!(err.matches("watch step").filter(|_| true).count(), 5, "nosuch printed once: {err}");
    assert!(err.contains("watch step 5: nosuch = ERROR"), "{err}");
    assert!(err.contains("audio-out: a.wav: 0.67 s"), "{err}");
    // the log has everything
    let log = std::fs::read_to_string(dir.join("run.log")).unwrap();
    assert!(log.contains("watch step 10: global.cash = 100000") && log.contains("[debug] rnd"), "{log}");
    // screenshots of the listed steps only
    assert!(dir.join("shots/frame00004.png").is_file() && dir.join("shots/frame00008.png").is_file());
    assert_eq!(std::fs::read_dir(dir.join("shots")).unwrap().count(), 2);
    // the save went to sd, starting from a copy of the original; the original is untouched
    assert_eq!(std::fs::read_to_string(dir.join("sd/save.txt")).unwrap(), "original\r\nbundled\r\n");
    assert_eq!(std::fs::read_to_string(dir.join("save.txt")).unwrap(), "original\r\n");
    assert!(dir.join("sd/cfg.ini").is_file() && !dir.join("cfg.ini").exists());
    // the WAV: 44.1 kHz 16-bit stereo, 20 steps at room speed 30 (silence)
    let wav = std::fs::read(dir.join("a.wav")).unwrap();
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 44100);
    assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize, wav.len() - 44);
    assert_eq!(wav.len() - 44, 29400 * 4);
    // --seed: the same random numbers again; --no-read-fallback: a clean save
    let rnd = |e: &str| e.lines().find(|l| l.contains("[debug] rnd")).map(|l| l.to_string());
    let err2 = run(&["--no-read-fallback"]);
    assert_eq!(rnd(&err), rnd(&err2));
    let _ = std::fs::remove_dir_all(dir.join("sd"));
    let err3 = run(&["--no-read-fallback"]);
    assert!(err3.contains("Room Start Event"), "data.txt is missing without the fallback: {err3}");
    assert!(!std::fs::read_to_string(dir.join("sd/save.txt")).unwrap_or_default().contains("original"));
    let _ = std::fs::remove_dir_all(&dir);
}
