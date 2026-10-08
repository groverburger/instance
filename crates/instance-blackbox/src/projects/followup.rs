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

/// bb_lang3: narrows what bb_lang2 left open (tolerance constant, Latin-1 case
/// mapping, ord of 0x80-0x9F, script_execute with too few arguments) and probes
/// instance_change timing further.
pub fn build_lang3() -> Spec {
    let mut g = Gen::new("bb_lang3", 81033);
    let fw = install(&mut g);
    g.script("sc_side", "global.q = 7; return argument0;");
    let ch1 = g.object("ch1");
    let ch2 = g.object("ch2");
    g.obj(ch1).depth = 3;
    g.obj(ch2).depth = 7;
    let mut t = vec![vals(
        "eq3",
        &[
            ("eq3.0_vs_1e-14", "(0 == power(10, -14))"),
            ("eq3.0_vs_5e-15", "(0 == 5 * power(10, -15))"),
            ("eq3.0_vs_2e-15", "(0 == 2 * power(10, -15))"),
            ("eq3.0_vs_1e-15", "(0 == power(10, -15))"),
            ("eq3.0_vs_5e-16", "(0 == 5 * power(10, -16))"),
            ("eq3.1_vs_2e-15", "(1 == 1 + 2 * power(10, -15))"),
            ("eq3.1_vs_1e-14", "(1 == 1 + power(10, -14))"),
            ("eq3.1_vs_5e-14", "(1 == 1 + 5 * power(10, -14))"),
            ("eq3.100_plus_1p5e-13", "(100 == 100 + 1.5 * power(10, -13))"),
            ("eq3.1e6_plus_5e-9", "(1000000 == 1000000 + 5 * power(10, -9))"),
            ("eq3.1e6_plus_5e-10", "(1000000 == 1000000 + 5 * power(10, -10))"),
        ],
    )];
    t.push(vals(
        "case3",
        &[
            ("case3.lower_192_is_224", "(string_lower(chr(192)) == chr(224))"),
            ("case3.upper_233_is_201", "(string_upper(chr(233)) == chr(201))"),
            ("case3.lower_215_is_215", "(string_lower(chr(215)) == chr(215))"),
            ("case3.upper_255", "ord(string_char_at(string_upper(chr(255)), 1))"),
            ("case3.ord_127", "ord(chr(127))"),
            ("case3.ord_128", "ord(chr(128))"),
            ("case3.ord_159", "ord(chr(159))"),
            ("case3.ord_160", "ord(chr(160))"),
            ("case3.ord_255", "ord(chr(255))"),
        ],
    ));
    t.push(err_stmt(
        "args3.execute_few_runs",
        "global.q = 0; global.bb_r = script_execute(sc_side); global.bb_r = string(global.bb_r) + '/' + string(global.q);",
    ));
    t.push(err_stmt(
        "change2.same_action",
        "var i; i = instance_create(0, 0, ch1); with (i) instance_change(ch2, false); global.v = 0; with (ch2) global.v += 1; global.w = 0; with (i) global.w += 1; global.bb_r = string(instance_number(ch1)) + '/' + string(instance_number(ch2)) + '/' + string(global.v) + '/' + string(global.w) + '/' + string(instance_find(ch2, 0) == i) + '/' + string(i.depth); with (ch1) instance_destroy(); with (ch2) instance_destroy();",
    ));
    t.push(raw(
        "change2.self_write",
        &[
            "var i; i = instance_create(0, 0, ch1); with (i) { instance_change(ch2, false); x = 5; myvar = 3; }",
            "with (ch2) bb_log('change2.self_write', string(x) + '/' + string(variable_local_exists('myvar')) + '/' + string(depth)); with (ch1) instance_destroy(); with (ch2) instance_destroy();",
        ],
    ));
    // instance changed in one step, addressed by id in the next test (next step)
    t.push(raw(
        "change2.next_step_a",
        &["global.ch_id = instance_create(0, 0, ch1); with (global.ch_id) instance_change(ch2, false);"],
    ));
    t.push(raw(
        "change2.next_step_b",
        &["global.v = 0; with (global.ch_id) global.v += 1; bb_log('change2.next_step_with_id', global.v); with (ch2) instance_destroy();"],
    ));
    t.push(raw(
        "change2.perf",
        &[
            "global.ch_id = instance_create(0, 0, ch1); with (global.ch_id) { instance_change(ch2, true); global.bb_r = object_get_name(object_index); }",
            "global.v = 0; with (global.ch_id) global.v += 1; bb_log('change2.perf', global.bb_r + '/' + string(global.v)); with (ch2) instance_destroy();",
        ],
    ));
    t.push(vals("lang3.done", &[("lang3.done", "1")]));
    let frames = unit_project(&mut g, &fw, t, 60);
    Spec {
        name: "bb_lang3",
        title: "Follow-up: tolerance constant, Latin-1 case/ord, script_execute, instance_change timing",
        lite: "Lite-safe.",
        frames,
        keys: vec![
            k("eq3.", "GML-3", "the equality tolerance constant; relative above 1?"),
            k("case3.", "G-24", "Latin-1 case mapping (string compare) and ord of 0x80-0xFF"),
            k("args3.", "GML-11", "script_execute with too few arguments: does the script run?"),
            k("change2.", "RT-21", "instance_change(false): same-action queries, writes after the call, with(id) next step, perf=true"),
            ki(".msg", "GML-18", "error message text"),
            k("lang3.done", "-", "end marker"),
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
