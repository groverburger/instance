//! bb_perf / bb_perf_fps: performance baselines. Each micro-benchmark times a loop
//! with current_time (milliseconds) inside one action; bb_perf_fps counts how many
//! steps per second the game reaches with many active instances. Results are
//! informational: `gm-blackbox compare` prints them as GM-vs-ours ratios.

use super::events::RED;
use crate::builder::{ev, Gen};
use crate::framework::*;

/// A timed loop: `setup` runs first (untimed), then `body` runs `n` times inside
/// `for (i = 0; i < n; i += 1)`.
fn bench(key: &str, n: u32, setup: &str, body: &str) -> Test {
    raw(
        key,
        &[&format!(
            r#"var i, t, n, x, y, s, a;
n = {n};
x = 0; y = 0; s = ''; a = 0;
{setup}
t = current_time;
for (i = 0; i < n; i += 1) {{ {body} }}
bb_log('perf.{key}.ms', current_time - t);
bb_log('perf.{key}.n', n);"#
        )],
    )
}

pub fn build() -> Spec {
    let mut g = Gen::new("bb_perf", 81030);
    let fw = install(&mut g);
    g.script("pf_add", "return argument0 + argument1;");
    let s16 = g.sprite("pf_spr", 16, 16, &[RED], (0, 0));
    let inst = g.object("pf_inst");
    g.obj(inst).sprite = s16;
    let wall = g.object("pf_wall");
    g.obj(wall).sprite = s16;
    g.obj(wall).solid = true;
    let t = vec![
        bench("empty_loop", 1_000_000, "", ""),
        bench("arith", 500_000, "", "x = x * 0.5 + i mod 7 - (i div 3);"),
        bench("local_var", 500_000, "", "x += 1; y = x;"),
        bench("instance_var", 500_000, "pf_v = 0;", "pf_v += 1; pf_w = pf_v;"),
        bench("global_var", 500_000, "global.pf_g = 0;", "global.pf_g += 1;"),
        bench("array", 300_000, "for (x = 0; x < 1000; x += 1) pf_arr[x] = 0; x = 0;", "pf_arr[i mod 1000] = i; x = pf_arr[(i * 7) mod 1000];"),
        bench("if_compare", 500_000, "", "if (i mod 3 == 0 && i > 10) x += 1; else y += 1;"),
        bench("string_concat", 20_000, "", "s += 'ab';"),
        bench("string_funcs", 100_000, "s = 'hello world, this is a string';", "x = string_pos('this', s) + string_length(string_copy(s, 3, 5));"),
        bench("real_to_string", 100_000, "", "s = string(i * 0.25);"),
        bench("math_funcs", 300_000, "", "x = sin(i) + sqrt(i) + abs(-i) + point_distance(0, 0, i, 3);"),
        bench("script_call", 200_000, "", "x = pf_add(i, 1);"),
        bench("repeat_nested", 1, "", "repeat (1000) repeat (500) x += 1;"),
        bench(
            "instance_create_destroy",
            20_000,
            "",
            "with (instance_create(0, 0, pf_inst)) instance_destroy();",
        ),
        bench(
            "with_instances",
            2_000,
            "repeat (300) instance_create(random(1000), random(700), pf_inst);",
            "with (pf_inst) x += 1;",
        ),
        bench(
            "place_free",
            20_000,
            "with (pf_inst) instance_destroy(); repeat (500) instance_create(floor(random(64)) * 16, floor(random(48)) * 16, pf_wall);",
            "x += place_free(random(1000), random(700));",
        ),
        bench("instance_nearest", 5_000, "", "x = instance_nearest(random(1000), random(700), pf_wall);"),
        bench(
            "ds_list",
            100_000,
            "a = ds_list_create();",
            "ds_list_add(a, i); x = ds_list_find_value(a, i div 2);",
        ),
        bench("execute_string", 5_000, "", "execute_string('global.pf_e = 1 + 2;');"),
        vals("perf.done", &[("perf.done", "1")]),
    ];
    let frames = unit_project(&mut g, &fw, t, 30);
    Spec {
        name: "bb_perf",
        title: "Performance baselines (timed loops; informational)",
        lite: "Lite-safe. Takes roughly 10-30 s in GM 8.1; please don't touch anything until DONE.",
        frames,
        keys: vec![
            ki("perf.", "perf", "milliseconds per benchmark (timing, informational)"),
            k("perf.done", "-", "end marker"),
        ],
        project: g.finish(),
    }
}

/// Steps per second with 1000 moving, drawn, colliding instances at room_speed 9999.
pub fn build_fps() -> Spec {
    let mut g = Gen::new("bb_perf_fps", 81031);
    let fw = install(&mut g);
    let s16 = g.sprite("pf_spr", 16, 16, &[RED], (0, 0));
    let inst = g.object("pf_mover");
    g.obj(inst).sprite = s16;
    g.event(inst, ev::CREATE, &["hspeed = random(4) - 2; vspeed = random(4) - 2;"]);
    g.event(
        inst,
        ev::STEP,
        &["if (x < 0 || x > 1000) hspeed = -hspeed; if (y < 0 || y > 740) vspeed = -vspeed; image_angle += 1;"],
    );
    g.event(inst, ev::collision(inst), &["image_blend = c_yellow;"]);
    let ctl = g.object("pf_ctl");
    g.event(
        ctl,
        ev::CREATE,
        &["room_speed = 9999; repeat (1000) instance_create(random(1000), random(740), pf_mover); phase = 0; t0 = current_time; frames = 0;"],
    );
    g.event(
        ctl,
        ev::STEP,
        &[r#"if (phase == 0 && current_time - t0 >= 500) { phase = 1; t0 = current_time; frames = 0; }
else if (phase == 1)
{
    frames += 1;
    if (current_time - t0 >= 3000)
    {
        bb_log('perf.fps_1000_instances', frames * 1000 / (current_time - t0));
        phase = 2;
        with (pf_mover) instance_destroy();
        room_speed = 30;
        global.bb_done = 1;
    }
}"#],
    );
    let r = g.room("rm_fps", ROOM_W, ROOM_H);
    g.place(r, fw.out_obj, 0, 0, "");
    g.place(r, ctl, 0, 0, "");
    Spec {
        name: "bb_perf_fps",
        title: "Performance baseline: steps per second with 1000 instances (informational)",
        lite: "Lite-safe. Runs about 4 seconds.",
        frames: 400_000,
        keys: vec![ki("perf.", "perf", "steps per second (timing, informational)")],
        project: g.finish(),
    }
}
