//! Joystick events through scripted input and the sound builtins, on small
//! projects built in code (headless).

use instance_format as fmt;
use instance_runner::joystick::parse_scripted;
use instance_runner::{Game, InputEvent};
use gml::{Value, Vm};

fn code(src: &str) -> fmt::Action {
    let mut a = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    a.args[0] = src.to_string();
    a
}

fn event(t: i32, n: i32, src: &str) -> fmt::Event {
    fmt::Event { event_type: t, number: n, actions: vec![code(src)] }
}

fn game(events: Vec<fmt::Event>) -> (Game, Vm) {
    let mut p = fmt::Project::default();
    p.settings.display_errors = true;
    p.objects.push(Some(fmt::Object { name: "o".into(), sprite: -1, visible: true, events, ..Default::default() }));
    let mut r = fmt::Room { name: "r".into(), ..Default::default() };
    r.instances.push(fmt::Instance { x: 0, y: 0, object: 0, id: 100001, creation_code: String::new(), locked: false });
    p.last_instance_id = 102000;
    p.rooms.push(Some(r));
    p.room_order.push(0);
    let mut vm = Vm::new();
    let mut g = Game::new(p, &mut vm, true).expect("game");
    g.start(&mut vm).expect("start");
    (g, vm)
}

fn global(vm: &mut Vm, name: &str) -> Value {
    let id = vm.intern(name);
    vm.globals().get_value(id).cloned().unwrap_or(Value::Real(-12345.0))
}

fn greal(vm: &mut Vm, name: &str) -> f64 {
    match global(vm, name) {
        Value::Real(r) => r,
        v => panic!("{} is {:?}", name, v),
    }
}

fn gstr(vm: &mut Vm, name: &str) -> String {
    match global(vm, name) {
        Value::Str(s) => s.to_string_lossy(),
        v => format!("{:?}", v),
    }
}

fn joy(step: u64, s: &str) -> (u64, InputEvent) {
    let parts: Vec<&str> = s.split(':').collect();
    (step, InputEvent::Joy(parse_scripted(&parts).expect("joystick item")))
}

#[test]
fn joystick_scripted_input_functions_and_events() {
    let (mut g, mut vm) = game(vec![
        event(0, 0, "global.log = ''; global.dir = ''; global.ex = '';"),
        // ev_mouse joystick sub-events
        event(6, 16, "global.log += 'L';"),
        event(6, 17, "global.log += 'R';"),
        event(6, 18, "global.log += 'U';"),
        event(6, 19, "global.log += 'D';"),
        event(6, 21, "global.log += '1';"),
        event(6, 28, "global.log += '8';"),
        event(6, 31, "global.log += 'l';"),
        event(6, 36, "global.log += '!';"),
        // the step event comes after the mouse phase
        event(
            3,
            0,
            "global.log += '|'; global.dir += string(joystick_direction(1)) + ','; \
             global.ex += string(joystick_exists(1)) + string(joystick_exists(2)); \
             global.x1 = joystick_xpos(1); global.b3 = joystick_check_button(1, 3); \
             global.pov = joystick_pov(2); global.name = joystick_name(1); global.nb = joystick_buttons(1);",
        ),
    ]);
    g.scripted_input = vec![
        joy(1, "joy1:axis:x:-1"),
        joy(2, "joy1:axis:y:-0.8"),
        joy(2, "joy1:button:1:down"),
        joy(3, "joy1:axis:x:0.3"),
        joy(3, "joy1:button:1:up"),
        joy(3, "joy1:button:3:down"),
        joy(3, "joy1:button:8:down"),
        joy(4, "joy2:button:1:down"),
        joy(4, "joy2:axis:x:-0.9"),
        joy(4, "joy2:pov:270"),
        joy(5, "joy1:disconnect"),
    ];
    let start = g.frame;
    for s in &mut g.scripted_input {
        s.0 += start;
    }
    for _ in 0..6 {
        g.step(&mut vm).expect("step");
    }
    // step 0: nothing; 1: left; 2: left+up+button1; 3: up+button8 (x 0.3 is centred,
    // button 3 has no event); 4: joy1 same + joy2 left/button1; 5: joy1 gone
    assert_eq!(gstr(&mut vm, "log"), "|L|LU1|U8|U8l!|l!|");
    assert_eq!(gstr(&mut vm, "dir"), "101,100,103,104,104,101,");
    assert_eq!(gstr(&mut vm, "ex"), "001010101101");
    assert_eq!(greal(&mut vm, "x1"), 0.0);
    assert_eq!(greal(&mut vm, "b3"), 0.0);
    assert_eq!(greal(&mut vm, "pov"), 270.0);
    assert_eq!(gstr(&mut vm, "name"), "");
    assert_eq!(greal(&mut vm, "nb"), 0.0);
}

fn wav_file(secs: f64) -> std::path::PathBuf {
    let rate = 8000u32;
    let n = (secs * rate as f64) as u32;
    let mut v = b"RIFF".to_vec();
    v.extend((36 + n * 2).to_le_bytes());
    v.extend(b"WAVEfmt ");
    v.extend(16u32.to_le_bytes());
    v.extend(1u16.to_le_bytes());
    v.extend(1u16.to_le_bytes());
    v.extend(rate.to_le_bytes());
    v.extend((rate * 2).to_le_bytes());
    v.extend(2u16.to_le_bytes());
    v.extend(16u16.to_le_bytes());
    v.extend(b"data");
    v.extend((n * 2).to_le_bytes());
    v.resize(v.len() + n as usize * 2, 0);
    let p = std::env::temp_dir().join(format!("gm81_audio_test_{}.wav", std::process::id()));
    std::fs::write(&p, v).unwrap();
    p
}

#[test]
fn sound_builtins_headless() {
    // the current state, not GM's delayed report (that is tested in audio::tests)
    instance_runner::audio::set_status_delay_ms(0);
    let wav = wav_file(0.2);
    let path = wav.to_string_lossy().replace('\\', "/");
    let src = format!(
        "s = sound_add('{}', 0, true); global.s = s; global.name = sound_get_name(s); \
         global.kind = sound_get_kind(s); \
         sound_play(s); global.p1 = sound_isplaying(s); \
         sound_volume(s, 0.5); sound_pan(s, -1); sound_fade(s, 0, 100); \
         sound_effect_set(s, se_echo | se_reverb); sound_effect_echo(s, 50, 50, 100, 100, 0); \
         sound_3d_set_sound_position(s, 1, 2, 3); sound_background_tempo(2); sound_global_volume(0.5); \
         sound_play(9999); sound_volume(9999, 1); sound_stop(9999); global.p9 = sound_isplaying(9999); \
         global.cdp = cd_paused(); global.cdn = cd_number(); global.mci = is_string(MCI_command('')); \
         t = sound_add('{}', 7, false); global.tkind = sound_get_kind(t); \
         sound_delete(t); global.texists = sound_exists(t); global.gone = sound_get_name(t); \
         sound_discard(s); global.p2 = sound_isplaying(s); sound_restore(s); sound_loop(s);",
        path, path
    );
    let (mut g, mut vm) = game(vec![event(0, 0, &src), event(3, 0, "global.p3 = sound_isplaying(global.s);")]);
    g.step(&mut vm).expect("step");
    let _ = std::fs::remove_file(&wav);
    let s = greal(&mut vm, "s");
    assert!(s >= 0.0);
    assert_eq!(gstr(&mut vm, "name"), format!("__newsound{}", s));
    assert_eq!(greal(&mut vm, "kind"), 0.0);
    assert_eq!(greal(&mut vm, "p1"), 1.0);
    assert_eq!(greal(&mut vm, "p9"), 0.0);
    assert_eq!(greal(&mut vm, "cdp"), 1.0);
    assert_eq!(greal(&mut vm, "cdn"), 0.0);
    assert_eq!(greal(&mut vm, "mci"), 1.0);
    assert_eq!(greal(&mut vm, "tkind"), 0.0, "kind out of range → 0");
    assert_eq!(greal(&mut vm, "texists"), 0.0);
    assert_eq!(gstr(&mut vm, "gone"), "");
    assert_eq!(greal(&mut vm, "p2"), 0.0, "discard stops the sound");
    assert_eq!(greal(&mut vm, "p3"), 1.0, "looping after restore");
    assert_eq!(g.audio.global_volume(), 0.5);
    let res = g.sound_res(s as i32).unwrap();
    assert_eq!(res.effects, 2 | 16);
    // the fade runs in real time from 0.5 to 0 over 100 ms
    let v0 = g.audio.volume(s as i32, &res);
    assert!(v0 <= 0.5);
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert_eq!(g.audio.volume(s as i32, &res), 0.0);
}
