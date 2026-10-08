//! Runner-level micro-benchmarks mirroring instance-blackbox's bb_perf (instance access,
//! with, instance_nearest, string appends):
//! `cargo run --release -p instance-runner --example perf`.

use instance_format as fmt;
use instance_runner::Game;
use gml::{Ctx, Vm};

fn main() {
    let mut p = fmt::Project::default();
    p.settings.display_errors = false;
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
    for name in ["ctl", "pf_inst", "pf_wall"] {
        p.objects.push(Some(fmt::Object {
            name: name.into(),
            sprite: if name == "ctl" { -1 } else { 0 },
            visible: true,
            solid: name == "pf_wall",
            ..Default::default()
        }));
    }
    let mut r = fmt::Room {
        name: "r0".into(),
        width: 1024,
        height: 768,
        ..Default::default()
    };
    r.instances.push(fmt::Instance {
        x: 0,
        y: 0,
        object: 0,
        id: 100001,
        creation_code: String::new(),
        locked: false,
    });
    p.last_instance_id = 100002;
    p.rooms.push(Some(r));
    p.room_order.push(0);
    let mut vm = Vm::new();
    let mut g = Game::new(p, &mut vm, true).expect("game");
    g.start(&mut vm).expect("start");
    let me = g.instances[0].id;

    let cases: &[(&str, u32, &str, &str)] = &[
        ("instance_var", 500_000, "pf_v = 0;", "pf_v += 1; pf_w = pf_v;"),
        ("global_var", 500_000, "global.pf_g = 0;", "global.pf_g += 1;"),
        ("string_concat", 20_000, "", "s += 'ab';"),
        ("real_to_string", 100_000, "", "s = string(i * 0.25);"),
        ("with_instances", 2_000, "with (pf_inst) instance_destroy(); repeat (300) instance_create(random(1000), random(700), pf_inst);", "with (pf_inst) x += 1;"),
        ("instance_nearest", 5_000, "with (pf_wall) instance_destroy(); repeat (500) instance_create(floor(random(64)) * 16, floor(random(48)) * 16, pf_wall);", "v = instance_nearest(random(1000), random(700), pf_wall);"),
        ("distance_to_object", 5_000, "", "x = random(1000); y = random(700); v = distance_to_object(pf_wall);"),
        ("with_empty", 2_000, "", "with (pf_inst) {}"),
        ("with_user_var", 2_000, "with (pf_inst) pf_v = 0;", "with (pf_inst) pf_v += 1;"),
        ("with_read_x", 2_000, "", "with (pf_inst) v = x;"),
        ("move_only", 5_000, "", "x = random(1000); y = random(700);"),
        ("distance_static", 5_000, "", "v = distance_to_object(pf_wall);"),
    ];
    for &(name, n, setup, body) in cases {
        // setup once (destroyed instances purged), then time the loop
        let prog = vm.compile(name, setup.as_bytes(), &g).expect("compile");
        vm.run_code(&mut g, &prog, Ctx::new(me, me)).expect("run");
        g.purge_destroyed();
        let src = format!(
            "var i, n, v, w, s, a; n = {n}; v = 0; w = 0; s = ''; a = 0; for (i = 0; i < n; i += 1) {{ {body} }}"
        );
        let prog = vm.compile(name, src.as_bytes(), &g).expect("compile");
        let mut best = f64::MAX;
        for _ in 0..5 {
            let t = std::time::Instant::now();
            vm.run_code(&mut g, &prog, Ctx::new(me, me)).expect("run");
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
        }
        println!("{name:20} {best:>8.1} ms (best of 5)");
    }
}
