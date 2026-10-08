//! Follow-up projects for questions left open by the first round of GM 8.1 results.
//! bb_lang2: comparison tolerance scale, case mapping of Latin-1 letters, min/max
//! with mixed types, implicit script results, mixed argument forms.
//! bb_restart2: what game_restart() runs, carried through a file.

use crate::builder::{ev, Gen};
use crate::framework::*;

pub fn build_lang2() -> Spec {
    let mut g = Gen::new("bb_lang2", 81031);
    let fw = install(&mut g);
    g.script("sc_call_last", "global.q = 1; abs(5);");
    g.script("sc_if_last", "global.q = 3; if (global.q == 0) global.q = 4;");
    g.script("sc_var_last", "var a; a = 6;");
    g.script("sc_mixed", "return argument1 + argument[0];");
    g.script("sc_arg0", "return argument0;");
    let mut t = vec![vals(
        "eq2",
        &[
            // relative (SameValue) vs absolute tolerance
            ("eq2.1e6_plus_1e-7", "(1000000 == 1000000 + power(10, -7))"),
            ("eq2.1e6_plus_1e-5", "(1000000 == 1000000 + power(10, -5))"),
            ("eq2.0_vs_1e-13", "(0 == power(10, -13))"),
            ("eq2.0_vs_5e-13", "(0 == 5 * power(10, -13))"),
            ("eq2.0_vs_2e-12", "(0 == 2 * power(10, -12))"),
            ("eq2.1_vs_1p5e-13", "(1 == 1 + 5 * power(10, -13))"),
            ("eq2.lt_1e6", "(1000000 < 1000000 + power(10, -7))"),
        ],
    )];
    t.push(vals(
        "case",
        &[
            ("case.lower_192_len", "string_length(string_lower(chr(192)))"),
            ("case.lower_192_b2", "ord(string_char_at(string_lower(chr(192)), 2))"),
            ("case.upper_ascii_e9_len", "string_length(string_upper('a' + chr(233)))"),
            ("case.lower_utf8_len", "string_length(string_lower(chr(195) + chr(137)))"),
            ("case.ord_192", "ord(chr(192))"),
            ("case.len_192", "string_length(chr(192))"),
        ],
    ));
    t.extend(
        [
            ("mm.max_real_first", "max(1, 'a')"),
            ("mm.min_str_first", "min('b', 2, 'a')"),
            ("mm.mean_str", "mean(1, 'a')"),
            ("ret.call_last", "sc_call_last()"),
            ("ret.if_last", "sc_if_last()"),
            ("ret.var_last", "sc_var_last()"),
            ("args.mixed_fewer", "sc_mixed(5)"),
            ("args.mixed_exact", "sc_mixed(5, 6)"),
            ("args.mixed_more", "sc_mixed(5, 6, 7)"),
            ("args.script_execute_few", "script_execute(sc_arg0)"),
            ("args.script_execute_more", "script_execute(sc_arg0, 1, 2)"),
        ]
        .iter()
        .map(|(k, e)| err(k, e)),
    );
    t.push(vals("lang2.done", &[("lang2.done", "1")]));
    let frames = unit_project(&mut g, &fw, t, 60);
    Spec {
        name: "bb_lang2",
        title: "Follow-up: comparison tolerance, case mapping, implicit results, argument forms",
        lite: "Lite-safe.",
        frames,
        keys: vec![
            k("eq2.", "GML-3", "relative vs absolute comparison tolerance"),
            k("case.", "G-24", "string_lower/upper of Latin-1 letters (UTF-8?)"),
            k("mm.", "G-25", "min/max/mean with mixed types"),
            k("ret.", "GML-11", "script result without return: last assignment or last expression?"),
            k("args.", "GML-11, FF-18", "argumentN together with argument[i]; script_execute counts"),
            ki(".msg", "GML-18", "error message text"),
            k("lang2.done", "-", "end marker"),
        ],
        project: g.finish(),
    }
}

pub fn build_restart2() -> Spec {
    let mut g = Gen::new("bb_restart2", 81032);
    let fw = install(&mut g);
    let ctl = g.object("rs2_ctl");
    let x = g.object("rs2_x");
    // the log of phase 1 survives in a file (globals are cleared by game_restart)
    g.script(
        "rs2_note",
        "var f; f = file_text_open_append('bb_restart2.txt'); file_text_write_string(f, argument0 + ' '); file_text_close(f);",
    );
    g.event(
        ctl,
        ev::CREATE,
        &[r#"t = 0;
phase = 1 + file_exists('bb_restart2.txt');
if (phase == 1) { rs2_note('CR1'); rs2_note('id' + string(instance_create(0, 0, rs2_x))); }
else
{
    var f, s;
    f = file_text_open_read('bb_restart2.txt'); s = file_text_read_string(f); file_text_close(f);
    file_delete('bb_restart2.txt');
    bb_log('rs2.phase1_log', s);
    bb_log('rs2.p2_new_id', instance_create(0, 0, rs2_x));
    bb_log('rs2.p2_ctl_id', id);
}"#],
    );
    g.event(
        ctl,
        ev::STEP,
        &[
            "t += 1; if (phase == 1 && t == 3) { rs2_note('before'); game_restart(); rs2_note('after_call'); }",
            "if (phase == 1 && t == 3) rs2_note('next_action');",
            "if (phase == 2 && t == 2) global.bb_done = 1;",
        ],
    );
    for (e, l) in [(ev::GAME_END, "GE"), (ev::ROOM_END, "RE"), (ev::DESTROY, "DE")] {
        g.event(ctl, e, &[&format!("if (phase == 1) rs2_note('{l}');")]);
        g.event(x, e, &[&format!("if (file_exists('bb_restart2.txt')) rs2_note('x{l}');")]);
    }
    let r = g.room("rm_rs2", ROOM_W, ROOM_H);
    g.place(r, fw.out_obj, 0, 0, "");
    g.place(r, ctl, 0, 0, "");
    Spec {
        name: "bb_restart2",
        title: "Follow-up: game_restart() events and instance ids (logged through a file)",
        lite: "Lite-safe (file_text_* and file_exists). Delete bb_restart2.txt if a run was interrupted.",
        frames: 30,
        keys: vec![
            k("rs2.phase1_log", "RT-04", "phase 1 sequence: rest of the action / next action / Game End / Room End / Destroy around game_restart"),
            k("rs2.p2_", "RT-04", "instance ids after the restart (counter reset?)"),
        ],
        project: g.finish(),
    }
}
