//! bb_events: order of events within a step, alarms, creation during a phase,
//! keyboard (simulated), triggers, timelines, path end, lives/health, room change.

use crate::builder::{ev, Gen};
use crate::framework::*;

pub const RED: [u8; 4] = [220, 40, 40, 255];
pub const GREEN: [u8; 4] = [40, 200, 40, 255];
pub const BLUE: [u8; 4] = [40, 40, 220, 255];

/// Adds `if (cond) bb_ev('tag.LABEL');` for each event.
pub fn tag_events(g: &mut Gen, obj: i32, tag: &str, cond: &str, events: &[((i32, i32), &str)]) {
    for (e, label) in events {
        let code = if cond.is_empty() {
            format!("bb_ev('{tag}.{label}');")
        } else {
            format!("if ({cond}) bb_ev('{tag}.{label}');")
        };
        g.event(obj, *e, &[&code]);
    }
}

/// The step clock: flushes the event sequence at each Begin Step as
/// `<prefix>.sNN` (NN = number of the step that just ended; s00 = before the first
/// step) up to step `stop`; then global.bb_step stays at stop + 1 and, if
/// `finish`, the run is marked done.
pub fn clock(g: &mut Gen, prefix: &str, stop: i32, finish: bool, extra_bs: &str) -> i32 {
    let done = if finish { "global.bb_done = 1;" } else { "" };
    let k = g.object("bb_clock");
    g.obj(k).persistent = true;
    g.event(
        k,
        ev::CREATE,
        &["global.bb_step = 0; if (!variable_global_exists('bb_seq')) global.bb_seq = ''; bb_ev('k.CR');"],
    );
    g.event(
        k,
        ev::BEGIN_STEP,
        &[&format!(
            r#"if (global.bb_step > {stop}) exit;
bb_flush('{prefix}.s' + bb_pad(global.bb_step));
global.bb_step += 1;
if (global.bb_step > {stop}) {{ {done} exit; }}
bb_ev('k.BS');
{extra_bs}"#
        )],
    );
    tag_events(
        g,
        k,
        "k",
        &format!("global.bb_step <= {stop}"),
        &[(ev::GAME_START, "GS"), (ev::ROOM_START, "RS"), (ev::ROOM_END, "RE")],
    );
    k
}

pub fn build() -> Spec {
    let mut g = Gen::new("bb_events", 81005);
    let fw = install(&mut g);
    let s16 = g.sprite("spr16", 16, 16, &[RED], (0, 0));
    let s3 = g.sprite("spr3f", 16, 16, &[RED, GREEN, BLUE], (0, 0));
    let pth = g.path("pth_ev", false, false, &[(0.0, 0.0, 100.0), (12.0, 0.0, 100.0)]);
    let _ = pth;
    let tl = g.timeline(
        "tl_ev",
        &[
            (0, "bb_ev('a.TL0');".into()),
            (1, "bb_ev('a.TL1');".into()),
            (3, "bb_ev('a.TL3');".into()),
        ],
    );
    let _ = tl;
    g.trigger("trg_step", "global.bb_step == 3", 0);
    g.trigger("trg_begin", "global.bb_step == 3", 1);
    g.trigger("trg_end", "global.bb_step == 3", 2);

    let clk = clock(&mut g, "ev", 17, true, "");
    // extra defaults logged by the clock at creation
    g.event(
        clk,
        ev::CREATE,
        &[r#"bb_log('ev.default.lives', lives);
bb_log('ev.default.health', health);
bb_log('ev.default.score', score);"#],
    );

    // ---------------- a: the instrumented instance
    let a = g.object("ev_a");
    let b = g.object("ev_b");
    let e = g.object("ev_e");
    let f = g.object("ev_f");
    let gg = g.object("ev_g");
    let hh = g.object("ev_h");
    {
        let o = g.obj(a);
        o.sprite = s16;
        o.persistent = true;
    }
    g.obj(b).sprite = s16;
    g.event(
        a,
        ev::CREATE,
        &[
            "bb_ev('a.CR'); lives = 3; health = 100; alarm[0] = 3; alarm[1] = 1.5; alarm[2] = 0; alarm[3] = 1; alarm[4] = 0.5;",
            "timeline_index = tl_ev; timeline_position = 0; bb_log('ev.timeline_running_default', timeline_running);",
            "timeline_running = true;",
        ],
    );
    tag_events(&mut g, a, "a", "", &[(ev::DESTROY, "DE")]);
    g.event(a, ev::alarm(0), &["bb_ev('a.A0[' + bb_fmt(alarm[0]) + ']');"]);
    g.event(a, ev::alarm(1), &["bb_ev('a.A1[' + bb_fmt(alarm[1]) + ']');"]);
    g.event(a, ev::alarm(2), &["bb_ev('a.A2');"]);
    g.event(a, ev::alarm(3), &["bb_ev('a.A3'); if (global.bb_step < 6) alarm[3] = 1;"]);
    g.event(a, ev::alarm(4), &["bb_ev('a.A4');"]);
    g.event(
        a,
        ev::BEGIN_STEP,
        &[&format!(
            r#"bb_ev('a.BS');
if (global.bb_step == 2) instance_create(0, 0, ev_f);
if (global.bb_step <= 6) bb_log('ev.alarms.s' + bb_pad(global.bb_step), bb_fmt(alarm[0]) + ',' + bb_fmt(alarm[1]) + ',' + bb_fmt(alarm[2]) + ',' + bb_fmt(alarm[3]) + ',' + bb_fmt(alarm[4]));"#
        )],
    );
    g.event(
        a,
        ev::STEP,
        &[
            r#"bb_ev('a.ST');
if (global.bb_step == 2) instance_create(0, 0, ev_e);
if (global.bb_step == 3) event_user(0);
if (global.bb_step == 4) with (ev_b) instance_destroy();
if (global.bb_step == 6) { bb_ev('a.lives0'); lives = 0; bb_ev('a.lives0done'); }
if (global.bb_step == 7) { bb_ev('a.health0'); health = 0; bb_ev('a.health0done'); }
if (global.bb_step == 8) { lives = 3; health = 100; }"#,
            "if (global.bb_step == 10) { room_goto(rm_ev2); bb_ev('a.after_goto'); }",
            "if (global.bb_step == 10) bb_ev('a.next_action');",
        ],
    );
    g.event(
        a,
        ev::END_STEP,
        &["bb_ev('a.ES'); if (global.bb_step == 2) instance_create(0, 0, ev_g);"],
    );
    g.event(
        a,
        ev::DRAW,
        &["bb_ev('a.DR'); if (global.bb_step == 2) instance_create(0, 0, ev_h);"],
    );
    tag_events(
        &mut g,
        a,
        "a",
        "",
        &[
            (ev::collision(b), "CO"),
            (ev::ROOM_START, "RS"),
            (ev::ROOM_END, "RE"),
            (ev::GAME_START, "GS"),
            (ev::NO_LIVES, "NML"),
            (ev::NO_HEALTH, "NMH"),
            (ev::user(0), "U0"),
            (ev::trigger(0), "TRst"),
            (ev::trigger(1), "TRbs"),
            (ev::trigger(2), "TRes"),
            (ev::key_press(75), "KP"),
            (ev::keyboard(75), "KB"),
            (ev::key_release(75), "KR"),
            (ev::keyboard(1), "KBany"),
        ],
    );

    // ---------------- b: overlapping a, destroyed at step 4
    tag_events(
        &mut g,
        b,
        "b",
        "global.bb_step <= 5",
        &[
            (ev::CREATE, "CR"),
            (ev::DESTROY, "DE"),
            (ev::BEGIN_STEP, "BS"),
            (ev::STEP, "ST"),
            (ev::END_STEP, "ES"),
            (ev::DRAW, "DR"),
            (ev::collision(a), "CO"),
        ],
    );

    // ---------------- e/f/g/h: created during step / begin step / end step / draw
    for (o, t) in [(e, "e"), (f, "f"), (gg, "g"), (hh, "h")] {
        tag_events(
            &mut g,
            o,
            t,
            "global.bb_step <= 4",
            &[
                (ev::CREATE, "CR"),
                (ev::BEGIN_STEP, "BS"),
                (ev::STEP, "ST"),
                (ev::END_STEP, "ES"),
                (ev::DRAW, "DR"),
            ],
        );
    }
    g.event(e, ev::CREATE, &["alarm[0] = 1;"]);
    g.event(e, ev::alarm(0), &["bb_ev('e.A0');"]);

    // ---------------- c: leaves the room to the right (outside / boundary, xprevious)
    let c = g.object("ev_c");
    g.obj(c).sprite = s16;
    g.event(c, ev::CREATE, &["hspeed = 4; bb_ev('c.CR');"]);
    for (e2, l) in [(ev::BEGIN_STEP, "BS"), (ev::STEP, "ST"), (ev::END_STEP, "ES")] {
        g.event(
            c,
            e2,
            &[&format!(
                "if (global.bb_step <= 3) bb_ev('c.{l}' + bb_fmt(x) + '/' + bb_fmt(xprevious));"
            )],
        );
    }
    g.event(c, ev::DRAW, &["if (global.bb_step <= 3) bb_ev('c.DR' + bb_fmt(x));"]);
    g.event(c, ev::OUTSIDE, &["bb_ev('c.OR' + bb_fmt(x));"]);
    g.event(c, ev::BOUNDARY, &["bb_ev('c.IB' + bb_fmt(x));"]);

    // ---------------- d: animation (3 frames, image_speed 1)
    let d = g.object("ev_d");
    g.obj(d).sprite = s3;
    g.event(d, ev::CREATE, &["image_speed = 1; bb_ev('d.CR');"]);
    for (e2, l) in [
        (ev::BEGIN_STEP, "BS"),
        (ev::STEP, "ST"),
        (ev::END_STEP, "ES"),
        (ev::DRAW, "DR"),
    ] {
        g.event(
            d,
            e2,
            &[&format!("if (global.bb_step <= 7) bb_ev('d.{l}' + bb_fmt(image_index));")],
        );
    }
    g.event(d, ev::ANIM_END, &["bb_ev('d.AE' + bb_fmt(image_index));"]);

    // ---------------- z: alarm set without any alarm event; q: alarm only in parent
    let z = g.object("ev_z");
    g.event(z, ev::CREATE, &["alarm[0] = 2;"]);
    g.event(z, ev::BEGIN_STEP, &["if (global.bb_step <= 4) bb_ev('z.al' + bb_fmt(alarm[0]));"]);
    let qp = g.object("ev_qparent");
    let q = g.object("ev_q");
    g.obj(q).parent = qp;
    g.event(qp, ev::alarm(0), &["bb_ev('q.A0inh');"]);
    g.event(q, ev::CREATE, &["alarm[0] = 2;"]);

    // ---------------- p: path with end action stop
    let p = g.object("ev_p");
    g.event(p, ev::CREATE, &["path_start(pth_ev, 5, 0, false); bb_ev('p.CR');"]);
    g.event(p, ev::STEP, &["if (global.bb_step <= 5) bb_ev('p.ST' + bb_fmt(x));"]);
    g.event(p, ev::PATH_END, &["bb_ev('p.PE' + bb_fmt(x));"]);

    // ---------------- kb: simulated keyboard (keyboard_key_press may be unsupported)
    let kb = g.object("ev_kb");
    g.event(
        kb,
        ev::STEP,
        &[
            "if (global.bb_step == 5) { bb_ev('kb.press'); keyboard_key_press(ord('K')); }",
            "if (global.bb_step == 8) { bb_ev('kb.release'); keyboard_key_release(ord('K')); }",
            "if (global.bb_step >= 5 && global.bb_step <= 9) bb_ev('kb.chk' + string(keyboard_check(ord('K'))));",
            "if (global.bb_step == 9) bb_log('ev.keyboard_string', keyboard_string);",
        ],
    );

    // ---------------- second room
    let r2o = g.object("ev_r2");
    tag_events(
        &mut g,
        r2o,
        "r2",
        "global.bb_step <= 12",
        &[
            (ev::CREATE, "CR"),
            (ev::ROOM_START, "RS"),
            (ev::BEGIN_STEP, "BS"),
            (ev::STEP, "ST"),
        ],
    );
    // instances of room 1 that are not persistent: do they get Room End / Destroy?
    tag_events(
        &mut g,
        c,
        "c",
        "",
        &[(ev::ROOM_END, "RE"), (ev::DESTROY, "DE")],
    );

    let rm1 = g.room("rm_ev1", ROOM_W, ROOM_H);
    g.room_mut(rm1).creation_code = "bb_ev('rm1.CC');".into();
    g.place(rm1, fw.out_obj, 0, 0, "");
    g.place(rm1, clk, 0, 0, "");
    g.place(rm1, a, 100, 100, "bb_ev('a.IC');");
    g.place(rm1, b, 108, 100, "bb_ev('b.IC');");
    g.place(rm1, c, 1000, 300, "");
    g.place(rm1, d, 200, 300, "");
    g.place(rm1, z, 300, 500, "");
    g.place(rm1, q, 340, 500, "");
    g.place(rm1, p, 300, 600, "");
    g.place(rm1, kb, 0, 0, "");
    let rm2 = g.room("rm_ev2", ROOM_W, ROOM_H);
    g.room_mut(rm2).creation_code = "bb_ev('rm2.CC');".into();
    g.place(rm2, r2o, 50, 50, "bb_ev('r2.IC');");

    Spec {
        name: "bb_events",
        title: "Event order within a step, alarms, creation during a phase, room change",
        lite: "Lite-safe except possibly keyboard_key_press/release (simulated K key; isolated in its own actions) and timeline_running (GM 8.1 variable). Triggers need advanced mode.",
        frames: 30,
        keys: vec![
            k("ev.s00", "RT-01, runtime.md §2/§3.2", "creation order: create events, instance creation code, room creation code, game start, room start"),
            k("ev.s", "RT-01, RT-08, RT-09, RT-05, RT-02, RT-13, RT-12, RT-17, R-1", "per-step event sequence (tags tag.EVENT; see manifest notes)"),
            k("ev.alarms.", "RT-08", "alarm[0..4] values at begin step (3, 1.5, 0, re-armed 1, 0.5)"),
            k("ev.default.", "RT-12, runtime.md §1.1", "default lives/health/score"),
            k("ev.timeline_running_default", "RT-17", "timeline_running after setting timeline_index"),
            ki("ev.keyboard_string", "RT-13", "keyboard_string after simulated K"),
        ],
        project: g.finish(),
    }
}
