//! Standalone executables (specs/exe-format.md): the real runner binary with a game
//! appended starts that game when run without a file argument, with the footer magic
//! of this build ("INSTNC\0\x01") and with the one of earlier builds ("GM81CLN\x01").

use instance_format as fmt;
use std::process::Command;

fn project() -> fmt::Project {
    let mut p = fmt::Project::default();
    let mut a = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    a.args[0] = "room_caption = 'standalone-ok ' + string(1 + 1);".to_string();
    p.objects.push(Some(fmt::Object {
        name: "obj_setup".into(),
        sprite: -1,
        visible: true,
        events: vec![fmt::Event { event_type: 0, number: 0, actions: vec![a] }],
        ..Default::default()
    }));
    let mut r = fmt::Room { name: "room_main".into(), ..Default::default() };
    r.instances.push(fmt::Instance { x: 0, y: 0, object: 0, id: 100001, creation_code: String::new(), locked: false });
    p.last_instance_id = 101000;
    p.rooms.push(Some(r));
    p.room_order.push(0);
    p
}

fn run_exe(magic: [u8; 8], tag: &str) -> String {
    let runner = std::fs::read(env!("CARGO_BIN_EXE_instance-runner")).expect("runner binary");
    let payload = fmt::write_project(&project(), fmt::FormatVersion::V810).unwrap();
    let mut exe = fmt::exe::build(&runner, &payload, None);
    let at = exe.len() - fmt::exe::FOOTER_V1_LEN;
    exe[at..at + 8].copy_from_slice(&magic);
    let dir = std::env::temp_dir().join(format!("instance-standalone-exe-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("game{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&path, &exe).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let out = Command::new(&path)
        .args(["--headless", "--frames", "3", "--no-console"])
        .output()
        .expect("run the standalone exe");
    let _ = std::fs::remove_dir_all(&dir);
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "{tag}: exit {:?}\n{err}", out.status);
    err
}

#[test]
fn standalone_exe_with_current_footer_runs() {
    let err = run_exe(fmt::exe::MAGIC_V1, "current");
    assert!(err.contains("standalone-ok 2"), "{err}");
}

#[test]
fn standalone_exe_with_legacy_footer_runs() {
    let err = run_exe(*b"GM81CLN\x01", "legacy");
    assert!(err.contains("standalone-ok 2"), "{err}");
}
