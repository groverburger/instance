//! bb_rooms: room_goto inside an event, persistent room re-entry, persistent
//! objects placed in re-entered rooms, room-end/destroy, defaults and caption.

use super::events::{clock, tag_events, RED};
use crate::builder::{ev, Gen};
use crate::framework::*;

pub fn build() -> Spec {
    let mut g = Gen::new("bb_rooms", 81008);
    let fw = install(&mut g);
    let s16 = g.sprite("spr16", 16, 16, &[RED], (0, 0));
    let clk = clock(
        &mut g,
        "rm",
        18,
        true,
        r#"if (global.bb_step == 5) { bb_ev('k.goto_r2'); room_goto(rm_r2); }
if (global.bb_step == 4) with (rm_pi) mark = 42;
if (global.bb_step == 8) { bb_ev('k.goto_r1'); room_goto(rm_r1); }
if (global.bb_step == 11) { bb_ev('k.goto_r2'); room_goto(rm_r2); }
if (global.bb_step == 14) { bb_ev('k.goto_r3'); room_goto(rm_r3); }
if (global.bb_step == 15) { room_caption = 'Cap'; show_score = true; score = 12; show_lives = true; lives = 3; show_health = true; health = 50; }
if (global.bb_step == 17) bb_log('rm.caption_shown', window_get_caption());"#,
    );
    g.event(
        clk,
        ev::CREATE,
        &[r#"bb_log('rm.default.score', score);
bb_log('rm.default.lives', lives);
bb_log('rm.default.health', health);
bb_log('rm.default.show_score', show_score);
bb_log('rm.default.show_lives', show_lives);
bb_log('rm.default.show_health', show_health);
bb_log('rm.default.caption_score', caption_score);
bb_log('rm.default.caption_lives', caption_lives);
bb_log('rm.default.caption_health', caption_health);
bb_log('rm.default.room_caption', room_caption);
bb_log('rm.default.room_speed', room_speed);
bb_log('rm.default.room_persistent', room_persistent);
bb_log('rm.default.transition_kind', transition_kind);
bb_log('rm.default.background_color', background_color);
bb_log('rm.default.instance_count', instance_count);
bb_log('rm.default.fps_is_real', is_real(fps));
bb_log('rm.default.view_enabled', view_enabled);
bb_log('rm.default.cursor_sprite', cursor_sprite);
bb_log('rm.default.mouse_x_is_real', is_real(mouse_x));
bb_log('rm.default.room_first', room_first == rm_r0);
bb_log('rm.default.room_last', room_last == rm_r3);"#],
    );

    // r0: gy (before go), go (calls room_goto), gx (after go)
    let gy = g.object("rm_gy");
    let go = g.object("rm_go");
    let gx = g.object("rm_gx");
    g.event(
        go,
        ev::STEP,
        &[
            "if (global.bb_step == 2) { bb_ev('go.ST'); room_goto(rm_r1); bb_ev('go.after_goto'); }",
            "if (global.bb_step == 2) bb_ev('go.next_action');",
        ],
    );
    for (o, t) in [(gy, "gy"), (go, "go"), (gx, "gx")] {
        tag_events(
            &mut g,
            o,
            t,
            "global.bb_step <= 3",
            &[
                (ev::STEP, "ST"),
                (ev::END_STEP, "ES"),
                (ev::DRAW, "DR"),
                (ev::ROOM_END, "RE"),
                (ev::DESTROY, "DE"),
            ],
        );
    }
    g.event(gx, ev::collision(gy), &["if (global.bb_step <= 3) bb_ev('gx.CO');"]);
    g.obj(gx).sprite = s16;
    g.obj(gy).sprite = s16;

    // r1 (persistent): pi
    let pi = g.object("rm_pi");
    g.event(pi, ev::CREATE, &["mark = 0; global.picount += 1; bb_ev('pi.CR' + string(global.picount));"]);
    g.event(pi, ev::ROOM_START, &["bb_ev('pi.RS' + string(mark));"]);
    tag_events(&mut g, pi, "pi", "", &[(ev::ROOM_END, "RE"), (ev::DESTROY, "DE")]);
    g.event(pi, ev::BEGIN_STEP, &["if (global.bb_step == 3 || global.bb_step == 9) bb_ev('pi.BS');"]);
    // a runtime-created instance in the persistent room: restored too?
    g.event(pi, ev::CREATE, &["if (global.picount == 1) instance_create(10, 10, rm_pc);"]);
    let pc = g.object("rm_pc");
    g.event(pc, ev::CREATE, &["bb_ev('pc.CR');"]);
    g.event(pc, ev::ROOM_START, &["bb_ev('pc.RS');"]);

    // r2: persistent object placed in the room (duplicated on re-entry?), plain instance
    let pp = g.object("rm_pp");
    g.obj(pp).persistent = true;
    g.event(pp, ev::CREATE, &["bb_ev('pp.CR' + string(instance_number(rm_pp)));"]);
    g.event(pp, ev::ROOM_START, &["bb_ev('pp.RS' + string(instance_number(rm_pp)));"]);
    g.event(pp, ev::ROOM_END, &["bb_ev('pp.RE');"]);
    let tq = g.object("rm_tq");
    g.event(tq, ev::CREATE, &["bb_ev('tq.CR');"]);
    tag_events(&mut g, tq, "tq", "", &[(ev::ROOM_START, "RS"), (ev::ROOM_END, "RE"), (ev::DESTROY, "DE")]);
    g.event(clk, ev::CREATE, &["global.picount = 0;"]);

    let r0 = g.room("rm_r0", ROOM_W, ROOM_H);
    g.room_mut(r0).creation_code = "bb_ev('r0.CC');".into();
    g.place(r0, fw.out_obj, 0, 0, "");
    g.place(r0, clk, 0, 0, "");
    g.place(r0, gy, 100, 100, "");
    g.place(r0, go, 0, 0, "");
    g.place(r0, gx, 104, 100, "");
    let r1 = g.room("rm_r1", ROOM_W, ROOM_H);
    {
        let r = g.room_mut(r1);
        r.persistent = true;
        r.creation_code = "if (!variable_global_exists('cc1')) global.cc1 = 0; global.cc1 += 1; bb_ev('r1.CC' + string(global.cc1));".into();
    }
    g.place(r1, pi, 50, 50, "bb_ev('pi.IC');");
    let r2 = g.room("rm_r2", ROOM_W, ROOM_H);
    g.room_mut(r2).creation_code = "bb_ev('r2.CC');".into();
    g.place(r2, pp, 50, 50, "bb_ev('pp.IC');");
    g.place(r2, tq, 60, 60, "");
    let r3 = g.room("rm_r3", ROOM_W, ROOM_H);
    g.room_mut(r3).creation_code = "bb_ev('r3.CC');".into();

    Spec {
        name: "bb_rooms",
        title: "room_goto inside events, persistent rooms/objects, defaults, caption",
        lite: "Lite-safe.",
        frames: 40,
        keys: vec![
            k("rm.s", "RT-02, RT-03, RT-19, R-1, R-10, runtime.md §3.1/§3.2", "per-step sequence across room changes"),
            k("rm.default.", "RT-12, runtime.md §1.1/§12", "defaults of global/room variables"),
            ki("rm.caption_shown", "RT-12", "window caption with show_score/lives/health"),
        ],
        project: g.finish(),
    }
}
