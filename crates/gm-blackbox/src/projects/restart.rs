//! bb_restart: what game_restart() runs and resets.

use super::events::RED;
use crate::builder::{ev, Gen};
use crate::framework::*;

pub fn build() -> Spec {
    let mut g = Gen::new("bb_restart", 81009);
    let fw = install(&mut g);
    let s16 = g.sprite("spr16", 16, 16, &[RED], (0, 0));
    let rx = g.object("rs_x");
    g.obj(rx).sprite = s16;
    let rs = g.object("rs_ctl");
    g.obj(rs).persistent = true;
    g.event(
        rs,
        ev::CREATE,
        &[
            // phase detection: globals, score, marker file, clipboard (whatever survives)
            "global.rs_g = variable_global_exists('rs_phase'); global.rs_f = 0; global.rs_c = 0; global.rs_s = (score == 55);",
            "global.rs_f = file_exists('bb_restart_marker.txt');",
            "global.rs_c = (clipboard_get_text() == 'bb_restart_marker');",
            r#"t = 0;
if (global.rs_g || global.rs_s || global.rs_f || global.rs_c) phase = 2; else phase = 1;
if (!variable_global_exists('rs_step')) global.rs_step = 0;
if (!variable_global_exists('bb_seq')) global.bb_seq = '';
bb_ev('rs.CR' + string(phase));
if (phase == 1)
{
    global.rs_phase = 1;
    score = 55; lives = 3; health = 40;
    global.keep = 123;
    room_speed = 45;
    global.rs_id1 = (instance_create(0, 0, rs_x)).id;
}
else
{
    bb_log('rs.p2.detect.globals', global.rs_g);
    bb_log('rs.p2.detect.score', global.rs_s);
    bb_log('rs.p2.detect.file', global.rs_f);
    bb_log('rs.p2.detect.clipboard', global.rs_c);
    bb_log('rs.p2.score', score);
    bb_log('rs.p2.lives', lives);
    bb_log('rs.p2.health', health);
    bb_log('rs.p2.keep_exists', variable_global_exists('keep'));
    bb_log('rs.p2.room', room_get_name(room));
    bb_log('rs.p2.room_speed', room_speed);
    bb_log('rs.p2.ctl_count', instance_number(rs_ctl));
    bb_log('rs.p2.out_count', instance_number(bb_out));
    bb_log('rs.p2.x_count', instance_number(rs_x));
    if (variable_global_exists('rs_id1')) bb_log('rs.p2.new_id_minus_first', (instance_create(0, 0, rs_x)).id - global.rs_id1);
}"#,
        ],
    );
    g.event(
        rs,
        ev::BEGIN_STEP,
        &[r#"if (global.bb_done) exit;
bb_flush('rs.p' + string(phase) + '.s' + bb_pad(t));
t += 1;
if (phase == 1 && t == 2) room_goto(rm_rs1);
if (phase == 2 && t == 4) global.bb_done = 1;"#],
    );
    g.event(
        rs,
        ev::STEP,
        &[
            "if (phase == 1 && t == 4) { bb_log('rs.p1.marker', 1); }",
            "if (phase == 1 && t == 4) { var f; f = file_text_open_write('bb_restart_marker.txt'); file_text_write_string(f, 'x'); file_text_close(f); }",
            "if (phase == 1 && t == 4) clipboard_set_text('bb_restart_marker');",
            "if (phase == 1 && t == 4) { bb_ev('rs.before_restart'); game_restart(); bb_ev('rs.after_restart_call'); }",
            "if (phase == 1 && t == 4) bb_ev('rs.next_action');",
            "if (phase == 2 && t == 2) { file_delete('bb_restart_marker.txt'); }",
            "if (phase == 2 && t == 2) { clipboard_set_text(''); }",
        ],
    );
    for (e, l) in [
        (ev::GAME_START, "GS"),
        (ev::GAME_END, "GE"),
        (ev::ROOM_START, "RS"),
        (ev::ROOM_END, "RE"),
        (ev::DESTROY, "DE"),
        (ev::END_STEP, "ES"),
    ] {
        g.event(rs, e, &[&format!("bb_ev('rs.{l}' + string(phase));")]);
    }
    g.event(rx, ev::DESTROY, &["bb_ev('x.DE');"]);
    g.event(rx, ev::ROOM_END, &["bb_ev('x.RE');"]);
    g.obj(rx).persistent = true;

    let r0 = g.room("rm_rs0", ROOM_W, ROOM_H);
    g.place(r0, fw.out_obj, 0, 0, "");
    g.place(r0, rs, 0, 0, "");
    let r1 = g.room("rm_rs1", ROOM_W, ROOM_H);
    let _ = r1;
    Spec {
        name: "bb_restart",
        title: "game_restart(): events run, values reset",
        lite: "Lite-safe for the core; uses file_exists/file_text_* and clipboard as restart markers. If the game restarts endlessly, all markers were reset: close it with Esc and report that.",
        frames: 40,
        keys: vec![
            k("rs.p1.", "RT-04", "phase 1 (before restart): event sequence including Game End / Room End / Destroy during game_restart"),
            k("rs.p2.", "RT-04", "after restart: what survived (globals, score, lives, health, persistent instances, id counter)"),
        ],
        project: g.finish(),
    }
}
