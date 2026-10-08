//! Multiplayer from GML: two games in one process, and two headless runner
//! processes running the demo project (examples/mplay_demo).

#[path = "../examples/mplay_demo/project.rs"]
mod demo;

use instance_format as fmt;
use instance_runner::Game;
use gml::{Value, Vm};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn game(port: u16, create: &str, step: &str) -> (Game, Vm) {
    let mut p = fmt::Project::default();
    p.game_id = 777;
    p.settings.display_errors = true;
    let act = |src: &str| {
        let mut a = fmt::builtin_action(1, 603).unwrap().instantiate(1);
        a.args[0] = src.to_string();
        a
    };
    p.objects.push(Some(fmt::Object {
        name: "o".into(),
        sprite: -1,
        visible: true,
        events: vec![
            fmt::Event { event_type: 0, number: 0, actions: vec![act(create)] },
            fmt::Event { event_type: 3, number: 0, actions: vec![act(step)] },
        ],
        ..Default::default()
    }));
    p.rooms.push(Some(fmt::Room {
        name: "r".into(),
        instances: vec![fmt::Instance { x: 0, y: 0, object: 0, id: 100001, creation_code: String::new(), locked: false }],
        ..Default::default()
    }));
    p.last_instance_id = 100002;
    p.room_order.push(0);
    let mut vm = Vm::new();
    let mut g = Game::new(p, &mut vm, true).unwrap();
    g.net.cfg.port = port;
    g.net.cfg.find_lan = Duration::from_millis(300);
    g.start(&mut vm).unwrap();
    (g, vm)
}

fn global(vm: &mut Vm, name: &str) -> Value {
    let id = vm.intern(name);
    vm.globals().get_value(id).cloned().unwrap_or(Value::Real(-1.0))
}

#[test]
fn two_games_in_process() {
    let (mut h, mut hvm) = game(
        41310,
        "global.ok = mplay_init_tcpip('') && mplay_session_create('s', 0, 'H'); global.got = '';",
        "if (mplay_message_receive(0)) { global.got = mplay_message_value(); global.from = mplay_message_name(); global.mid = mplay_message_id(); }
         global.players = mplay_player_find(); global.slot = mplay_data_read(3);",
    );
    assert_eq!(global(&mut hvm, "ok"), Value::Real(1.0));
    let (mut c, mut cvm) = game(
        41310,
        "mplay_init_tcpip(''); global.n = mplay_session_find(); global.name = mplay_session_name(0);
         global.joined = mplay_session_join(0, 'C'); global.st = mplay_session_status();
         mplay_data_write(3, 'shared'); mplay_message_send_guaranteed('H', 9, 'yo');",
        "global.players = mplay_player_find(); global.p1 = mplay_player_name(1); global.ipx = mplay_init_ipx();",
    );
    assert_eq!(global(&mut cvm, "n"), Value::Real(1.0));
    assert_eq!(global(&mut cvm, "name"), Value::from("s"));
    assert_eq!(global(&mut cvm, "joined"), Value::Real(1.0));
    assert_eq!(global(&mut cvm, "st"), Value::Real(2.0));
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(10) {
        h.step(&mut hvm).unwrap();
        c.step(&mut cvm).unwrap();
        if global(&mut hvm, "got") == Value::from("yo") && global(&mut hvm, "slot") == Value::from("shared") {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(global(&mut hvm, "got"), Value::from("yo"));
    assert_eq!(global(&mut hvm, "from"), Value::from("C"));
    assert_eq!(global(&mut hvm, "mid"), Value::Real(9.0));
    assert_eq!(global(&mut hvm, "slot"), Value::from("shared"));
    assert_eq!(global(&mut hvm, "players"), Value::Real(2.0));
    assert_eq!(global(&mut cvm, "players"), Value::Real(2.0));
    assert_eq!(global(&mut cvm, "p1"), Value::from("H"));
    assert_eq!(global(&mut cvm, "ipx"), Value::Real(0.0));
    // the host's game ends: the client's session is over
    h.game_end(&mut hvm).unwrap();
    let t0 = Instant::now();
    while c.net.session_status() != 0 && t0.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(c.net.session_status(), 0);
}

#[test]
fn two_runner_processes_demo() {
    let dir = std::env::temp_dir().join(format!("gm81_mplay_demo_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mplay_demo.gm81");
    fmt::write_project_file(&path, &demo::project(), fmt::FormatVersion::V810).unwrap();
    let spawn = |role: Option<&str>| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_instance-runner"));
        c.arg(&path).args(["--headless", "--frames", "5"]).env("GM81_MPLAY_PORT", "41330");
        if let Some(r) = role {
            c.arg(r);
        }
        c.stderr(Stdio::piped()).stdout(Stdio::null()).spawn().unwrap()
    };
    let host = spawn(Some("host"));
    std::thread::sleep(Duration::from_millis(200));
    let client = spawn(None);
    let c = client.wait_with_output().unwrap();
    let h = host.wait_with_output().unwrap();
    let hs = String::from_utf8_lossy(&h.stderr).to_string();
    let cs = String::from_utf8_lossy(&c.stderr).to_string();
    eprintln!("---- host ----\n{hs}\n---- client ----\n{cs}");
    assert!(hs.contains("host: player joined: ClientPlayer"), "{hs}");
    assert!(hs.contains("host: got message 2 \"pong\" from ClientPlayer"), "{hs}");
    assert!(hs.contains("host: shared slot 1 = 42"), "{hs}");
    assert!(hs.contains("host: players now 1"), "{hs}");
    assert!(cs.contains("client: found session \"gm81 demo\""), "{cs}");
    assert!(cs.contains("client: got message 1 \"ping\" from HostPlayer"), "{cs}");
    assert!(cs.contains("client: shared slot 0 = hello from host"), "{cs}");
    assert!(cs.contains("client: left, status 0"), "{cs}");
    let _ = std::fs::remove_dir_all(&dir);
}
