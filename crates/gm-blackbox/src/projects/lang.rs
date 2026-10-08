//! bb_lang: GML language semantics (truthiness, comparison, formatting,
//! rounding, arrays, arguments, syntax, errors, string/math/date functions).
//! bb_args0: the argument tests again with the "uninitialized" flag bit 1 cleared.

use crate::builder::Gen;
use crate::framework::*;

/// err() for each (key, expr).
fn errs(list: &[(&str, &str)]) -> Vec<Test> {
    list.iter().map(|(k, e)| err(k, e)).collect()
}

/// A source snippet run through execute_string (compile errors become
/// runtime errors there). `src` must only use single quotes and set global.bb_r.
fn sx(key: &str, src: &str) -> Test {
    err_stmt(key, &format!("execute_string(\"{src}\");"))
}

fn truth(label: &str, expr: &str) -> Test {
    err_stmt(
        &format!("truth.{label}"),
        &format!("global.bb_r = 0; if ({expr}) global.bb_r = 1;"),
    )
}

/// Scripts used by the argument tests.
pub fn arg_scripts(g: &mut Gen) {
    g.script("sc_count", "return argument_count;");
    g.script("sc_arg0", "return argument0;");
    g.script("sc_arg2", "return argument2;");
    g.script("sc_argarr5", "return argument[5];");
    g.script("sc_argarr_i", "return argument[argument0];");
    g.script("sc_write_arg", "argument0 = 9; return argument0 + argument[0];");
    g.script("sc_arg15", "return argument15;");
    g.script("sc_rel", "return argument_relative;");
    g.script("sc_noreturn", "global.bb_q = 1;");
    g.script("sc_exit", "global.bb_q = 2; exit; global.bb_q = 3;");
    g.script("sc_with_arg", "with (other) global.bb_q = argument0; return global.bb_q;");
    g.script("sc_err", "global.bb_q2 = 1; global.bb_tmp = chr(97) + 1; global.bb_q2 = 2;");
    g.script("sc_fact", "if (argument0 <= 1) return 1; return argument0 * sc_fact(argument0 - 1);");
}

pub fn arg_tests() -> Vec<Test> {
    let mut t = errs(&[
        ("args.count2", "sc_count(1, 2)"),
        ("args.count0", "sc_count()"),
        ("args.count16", "sc_count(0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15)"),
        ("args.unpassed_arg2", "sc_arg2(1, 2)"),
        ("args.unpassed_arg0", "sc_arg0()"),
        ("args.unpassed_arr5", "sc_argarr5(1)"),
        ("args.arr_index_i", "sc_argarr_i(1, 22)"),
        ("args.arr_index_16", "sc_argarr_i(16)"),
        ("args.arr_index_neg", "sc_argarr_i(-1)"),
        ("args.write_arg", "sc_write_arg(1)"),
        ("args.arg15", "sc_arg15(0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15)"),
        ("args.relative_from_code", "sc_rel()"),
        ("args.noreturn", "sc_noreturn()"),
        ("args.exit", "sc_exit()"),
        ("args.with_other_arg", "sc_with_arg(7)"),
        ("args.recursion", "sc_fact(10)"),
        ("args.code_action_arg0", "argument0"),
        ("args.code_action_count", "argument_count"),
        ("args.code_action_arr3", "argument[3]"),
    ]);
    t.push(sx(
        "args.17_args",
        "global.bb_r = sc_count(0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16);",
    ));
    t.push(err_stmt(
        "args.write_count",
        "execute_string(\"argument_count = 5; global.bb_r = argument_count;\", 1, 2);",
    ));
    t.push(err_stmt(
        "args.exec_string_args",
        "execute_string(\"global.bb_r = string(argument_count) + '/' + string(argument1);\", 'a', 'b');",
    ));
    t.push(err_stmt(
        "args.exec_string_return",
        "global.bb_r = execute_string(\"return 42;\");",
    ));
    t
}

pub fn build() -> Spec {
    build_with("bb_lang", 81003, true, None)
}

/// Diagnostic variants for locating why bb_lang closes instantly in GM 8.1:
/// without the expression constants, the constants alone, and the tests in quarters.
pub fn diagnostics() -> Vec<Spec> {
    let mut v = vec![build_with("bb_lang_noconst", 81020, false, None), constexpr()];
    for (i, name) in ["bb_lang_q1", "bb_lang_q2", "bb_lang_q3", "bb_lang_q4"].into_iter().enumerate() {
        v.push(build_with(name, 81021 + i as i32, false, Some((i, 4))));
    }
    v
}

/// Only bb_lang's expression-valued constants plus one trivial test.
fn constexpr() -> Spec {
    let mut g = Gen::new("bb_constexpr", 81025);
    let fw = install(&mut g);
    g.constant("bb_k1", "3+4");
    g.constant("bb_k2", "bb_k1*2");
    g.constant("bb_k3", "'str'");
    let t = vec![vals("cx", &[("cx.k1", "bb_k1"), ("cx.k2", "bb_k2"), ("cx.k3", "bb_k3")])];
    let frames = unit_project(&mut g, &fw, t, 30);
    Spec {
        name: "bb_constexpr",
        title: "Diagnostic: expression-valued constants only",
        lite: "Lite-safe",
        frames,
        keys: vec![k("cx.", "GML-19", "constants defined by expressions")],
        project: g.finish(),
    }
}

fn build_with(name: &'static str, id: i32, consts: bool, part: Option<(usize, usize)>) -> Spec {
    let mut g = Gen::new(name, id);
    let fw = install(&mut g);
    arg_scripts(&mut g);
    if consts {
        g.constant("bb_k1", "3+4");
        g.constant("bb_k2", "bb_k1*2");
        g.constant("bb_k3", "'str'");
    }
    let noinst = g.object("bb_noinst");
    let _ = noinst;
    let mut t: Vec<Test> = Vec::new();

    // ---- truthiness (GML-1)
    for (l, e) in [
        ("0_5", "0.5"),
        ("0_4999999", "0.4999999"),
        ("0_5000001", "0.5000001"),
        ("0_49999999999999", "0.49999999999999"),
        ("m0_5", "-0.5"),
        ("m1", "-1"),
        ("1", "1"),
        ("0", "0"),
        ("not_0_5", "!0.5"),
        ("not_0_4", "!0.4"),
        ("and_0_5", "0.5 && 0.5"),
        ("or_0_4", "0.4 || 0.4"),
        ("xor_0_6", "0.6 ^^ 0.6"),
        ("eq_true_0_5", "0.5 == true"),
        ("str_a", "'a'"),
        ("str_empty", "''"),
        ("str_1", "'1'"),
        ("str_0", "'0'"),
    ] {
        t.push(truth(l, e));
    }
    t.push(err_stmt(
        "truth.while_2_5",
        "var v, n; v = 2.5; n = 0; while (v) { v -= 1; n += 1; if (n > 10) break; } global.bb_r = n;",
    ));
    t.push(err_stmt(
        "truth.true_plus",
        "global.bb_r = true + 0.5;",
    ));

    // ---- real comparison tolerance (GML-3)
    t.push(vals(
        "eq",
        &[
            ("eq.0_1p0_2_eq_0_3", "(0.1 + 0.2 == 0.3)"),
            ("eq.0_1p0_2_ne_0_3", "(0.1 + 0.2 != 0.3)"),
            ("eq.0_3_lt_0_1p0_2", "(0.3 < 0.1 + 0.2)"),
            ("eq.0_3_ge_0_1p0_2", "(0.3 >= 0.1 + 0.2)"),
            ("eq.1_eq_1p1e-10", "(1 == 1.0000000001)"),
            ("eq.1_eq_1p1e-7", "(1 == 1.0000001)"),
            ("eq.1_eq_1p1e-6", "(1 == 1.000001)"),
            ("eq.1_eq_1p1e-5", "(1 == 1.00001)"),
            ("eq.0_eq_1e-12", "(0 == 0.000000000001)"),
            ("eq.big_half", "(100000000 == 100000000.5)"),
            ("eq.third_x3", "(1 / 3 * 3 == 1)"),
            ("eq.0_1x3_eq_0_3", "(0.1 * 3 == 0.3)"),
        ],
    ));

    // ---- mixed real/string (GML-2)
    t.extend(errs(&[
        ("mix.eq_a_1", "(chr(97) == 1)"),
        ("mix.eq_1s_1", "(chr(49) == 1)"),
        ("mix.ne_a_1", "(chr(97) != 1)"),
        ("mix.lt_a_1", "(chr(97) < 1)"),
        ("mix.gt_1_a", "(1 > chr(97))"),
        ("mix.add_r_s", "(1 + chr(97))"),
        ("mix.add_s_r", "(chr(97) + 1)"),
        ("mix.sub_s_s", "(chr(97) - chr(98))"),
        ("mix.mul_s_r", "(chr(97) * 2)"),
        ("mix.and_s", "(chr(97) && 1)"),
        ("mix.not_s", "(!chr(97))"),
        ("mix.neg_s", "(-chr(97))"),
        ("mix.bitor_s", "(chr(97) | 1)"),
    ]));
    t.push(vals(
        "strcmp",
        &[
            ("strcmp.ab_lt_abc", "('ab' < 'abc')"),
            ("strcmp.B_lt_a", "('B' < 'a')"),
            ("strcmp.a_eq_A", "('a' == 'A')"),
            ("strcmp.abc_gt_abd", "('abc' > 'abd')"),
            ("strcmp.empty_lt_a", "('' < 'a')"),
            ("strcmp.10_lt_9", "('10' < '9')"),
            ("strcmp.concat", "('a' + 'b')"),
        ],
    ));

    // ---- string() formatting (GML-5)
    t.push(vals(
        "str1",
        &[
            ("str.0_5", "string(0.5)"),
            ("str.1div3", "string(1 / 3)"),
            ("str.2div3", "string(2 / 3)"),
            ("str.m0_5", "string(-0.5)"),
            ("str.1_005", "string(1.005)"),
            ("str.2_675", "string(2.675)"),
            ("str.0_125", "string(0.125)"),
            ("str.0_135", "string(0.135)"),
            ("str.0_001", "string(0.001)"),
            ("str.0_005", "string(0.005)"),
            ("str.0_015", "string(0.015)"),
            ("str.0_025", "string(0.025)"),
        ],
    ));
    t.push(vals(
        "str2",
        &[
            ("str.m0_001", "string(-0.001)"),
            ("str.m0_004", "string(-0.004)"),
            ("str.m0_005", "string(-0.005)"),
            ("str.m0_006", "string(-0.006)"),
            ("str.neg_zero_lit", "string(-0)"),
            ("str.neg_zero_mul", "string(0 * -1)"),
            ("str.neg_zero_div", "string(-1 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000 / 100000000000000000000)"),
            ("str.100_5", "string(100.5)"),
            ("str.0_995", "string(0.995)"),
            ("str.9_999", "string(9.999)"),
            ("str.m1_5", "string(-1.5)"),
            ("str.12345_678", "string(12345.678)"),
        ],
    ));
    t.push(vals(
        "str3",
        &[
            ("str.123456789", "string(123456789)"),
            ("str.1234567890123", "string(1234567890123)"),
            ("str.2p31", "string(2147483648)"),
            ("str.2p32_5", "string(4294967296.5)"),
            ("str.1e15", "string(1000000000000000)"),
            ("str.1e20", "string(power(10, 20))"),
            ("str.1e21", "string(power(10, 21))"),
            ("str.m1e15", "string(-power(10, 15))"),
            ("str.2p53", "string(power(2, 53))"),
            ("str.2p53p1", "string(power(2, 53) + 1)"),
            ("str.1e-5", "string(power(10, -5))"),
            ("str.1e11_5", "string(123456789012.5)"),
        ],
    ));
    t.push(vals(
        "str4",
        &[
            ("str.true", "string(true)"),
            ("str.1_0", "string(1.0)"),
            ("str.0_1p0_2", "string(0.1 + 0.2)"),
            ("str.of_string", "string('x')"),
            ("str.huge", "string(power(10, 300))"),
            ("str.tiny", "string(power(10, -300))"),
        ],
    ));
    t.extend(errs(&[
        ("strfmt.2_5_0_0", "string_format(2.5, 0, 0)"),
        ("strfmt.3_5_0_0", "string_format(3.5, 0, 0)"),
        ("strfmt.0_5_0_0", "string_format(0.5, 0, 0)"),
        ("strfmt.m1_5_0_0", "string_format(-1.5, 0, 0)"),
        ("strfmt.0_125_0_2", "string_format(0.125, 0, 2)"),
        ("strfmt.1div3_8_3", "string_format(1 / 3, 8, 3)"),
        ("strfmt.123456_3_2", "string_format(123456, 3, 2)"),
        ("strfmt.m0_001_0_2", "string_format(-0.001, 0, 2)"),
        ("strfmt.1e20_0_2", "string_format(power(10, 20), 0, 2)"),
        ("strfmt.1_5_0_m1", "string_format(1.5, 0, -1)"),
        ("strfmt.1_23456_0_20", "string_format(1.23456, 0, 20)"),
        ("strfmt.1_5_m5_1", "string_format(1.5, -5, 1)"),
        ("strfmt.string_arg", "string_format('a', 0, 1)"),
    ]));

    // ---- rounding / integer conversion (GML-4, G-21)
    t.push(vals(
        "round1",
        &[
            ("round.0_5", "round(0.5)"),
            ("round.1_5", "round(1.5)"),
            ("round.2_5", "round(2.5)"),
            ("round.m0_5", "round(-0.5)"),
            ("round.m1_5", "round(-1.5)"),
            ("round.m2_5", "round(-2.5)"),
            ("round.0_49999999999", "round(0.49999999999)"),
            ("round.str_m0_5", "string(round(-0.5))"),
            ("round.floor_m0_5", "floor(-0.5)"),
            ("round.ceil_m0_5", "ceil(-0.5)"),
            ("round.ceil_0_5", "ceil(0.5)"),
            ("round.frac_m3_25", "frac(-3.25)"),
            ("round.frac_2_5", "frac(2.5)"),
            ("round.sign_0", "sign(0)"),
            ("round.sign_m0_0001", "sign(-0.0001)"),
            ("round.abs_m0_str", "string(abs(-0))"),
        ],
    ));
    for (l, n) in [
        ("0_5", "0.5"),
        ("1_5", "1.5"),
        ("2_5", "2.5"),
        ("3_5", "3.5"),
        ("2_4", "2.4"),
        ("2_6", "2.6"),
        ("m1", "-1"),
        ("str_3", "'3'"),
    ] {
        t.push(err_stmt(
            &format!("repeat.{l}"),
            &format!("var n; n = 0; repeat ({n}) n += 1; global.bb_r = n;"),
        ));
    }
    for (l, i) in [
        ("0_5", "0.5"),
        ("1_5", "1.5"),
        ("2_5", "2.5"),
        ("1_4999", "1.4999"),
        ("1_6", "1.6"),
        ("m0_4", "-0.4"),
        ("m0_5", "-0.5"),
        ("str_1", "'1'"),
    ] {
        t.push(err_stmt(
            &format!("idx.{l}"),
            &format!("aa[0] = 10; aa[1] = 11; aa[2] = 12; aa[3] = 13; global.bb_r = aa[{i}];"),
        ));
    }
    t.extend(errs(&[
        ("bit.1_5_or_0", "(1.5 | 0)"),
        ("bit.2_5_or_0", "(2.5 | 0)"),
        ("bit.m1_5_or_0", "(-1.5 | 0)"),
        ("bit.m2_5_or_0", "(-2.5 | 0)"),
        ("bit.0_5_or_0", "(0.5 | 0)"),
        ("bit.0_6_or_0", "(0.6 | 0)"),
        ("bit.m1_and_255", "((-1) & 255)"),
        ("bit.m1_shr_1", "((-1) >> 1)"),
        ("bit.m8_shr_1", "((-8) >> 1)"),
        ("bit.1_shl_31", "(1 << 31)"),
        ("bit.1_shl_32", "(1 << 32)"),
        ("bit.1_shl_40", "(1 << 40)"),
        ("bit.1_shl_40_shr_8", "((1 << 40) >> 8)"),
        ("bit.1_shl_63", "(1 << 63)"),
        ("bit.1_shl_64", "(1 << 64)"),
        ("bit.1_shl_m1", "(1 << -1)"),
        ("bit.not_0", "(~0)"),
        ("bit.not_1_5", "(~1.5)"),
        ("bit.hex_ffffffff", "($FFFFFFFF)"),
        ("bit.hex_ffffffff_or_0", "($FFFFFFFF | 0)"),
        ("bit.hex_80000000", "($80000000)"),
        ("bit.hex_10_digits", "($FFFFFFFFFF)"),
        ("bit.2p32_or_0", "(4294967296 | 0)"),
        ("bit.2p31_or_0", "(2147483648 | 0)"),
        ("bit.m2p31m1_or_0", "(-2147483649 | 0)"),
        ("bit.2p40_or_1", "(power(2, 40) | 1)"),
        ("bit.m2p40_or_0", "(-power(2, 40) | 0)"),
        ("bit.2p31m0_4_or_0", "(2147483647.6 | 0)"),
        ("bit.2p32m0_5_or_0", "(4294967295.5 | 0)"),
        ("bit.5_xor_3", "(5 ^ 3)"),
        ("bit.m1_xor_0", "((-1) ^ 0)"),
        ("bit.and_frac", "(3.5 & 6.5)"),
    ]));
    t.extend(errs(&[
        ("div.7_2", "(7 div 2)"),
        ("div.m7_2", "(-7 div 2)"),
        ("div.7_m2", "(7 div -2)"),
        ("div.m7_m2", "(-7 div -2)"),
        ("div.7_5_2", "(7.5 div 2)"),
        ("div.4_6_1_4", "(4.6 div 1.4)"),
        ("div.1_0_4", "(1 div 0.4)"),
        ("mod.7_3", "(7 mod 3)"),
        ("mod.m7_3", "(-7 mod 3)"),
        ("mod.7_m3", "(7 mod -3)"),
        ("mod.7_5_2", "(7.5 mod 2)"),
        ("mod.m7_5_2", "(-7.5 mod 2)"),
        ("mod.4_6_1_4", "(4.6 mod 1.4)"),
        ("mod.5_2_5", "(5 mod 2.5)"),
        ("mod.5_0_4", "(5 mod 0.4)"),
    ]));

    // ---- division by zero (GML-6)
    t.extend(errs(&[
        ("div0.1_0", "(1 / 0)"),
        ("div0.0_0", "(0 / 0)"),
        ("div0.m1_0", "(-1 / 0)"),
        ("div0.div", "(5 div 0)"),
        ("div0.mod", "(5 mod 0)"),
        ("div0.mod_frac", "(5.5 mod 0)"),
        ("div0.tiny", "(1 / 0.0000001)"),
    ]));
    t.push(err_stmt(
        "div0.inf_str",
        "var v; v = 1 / 0; global.bb_r = string(v) + '|' + string(v * 0) + '|' + string(-v);",
    ));

    // ---- arrays (GML-7)
    for (key, code) in [
        ("arr.scalar_as_a0", "av = 5; global.bb_r = av[0];"),
        ("arr.a0_as_scalar", "aw[0] = 7; global.bb_r = aw;"),
        ("arr.a0_as_a00", "ax[0] = 8; global.bb_r = ax[0, 0];"),
        ("arr.a00_as_scalar", "ay[0, 0] = 9; global.bb_r = ay;"),
        ("arr.1d_as_2d0", "az[3] = 1; global.bb_r = az[0, 3];"),
        ("arr.2d0_as_1d", "ba[0, 3] = 2; global.bb_r = ba[3];"),
        ("arr.2d1_as_1d", "bb[1, 3] = 3; global.bb_r = bb[3];"),
        ("arr.unwritten_in_range", "bc[5] = 1; global.bb_r = bc[2];"),
        ("arr.out_of_range", "bd[5] = 1; global.bb_r = bd[9];"),
        ("arr.read_neg", "be[0] = 1; global.bb_r = be[-1];"),
        ("arr.write_neg", "bf[-1] = 5; global.bb_r = bf[0];"),
        ("arr.2d_unwritten_cell", "bg[2, 2] = 1; global.bb_r = bg[1, 1];"),
        ("arr.2d_short_row", "bh[0, 5] = 1; bh[1, 1] = 2; global.bb_r = bh[1, 4];"),
        ("arr.index_31999", "bj[31999] = 1; global.bb_r = bj[31999];"),
        ("arr.index_32000", "bi[32000] = 1; global.bb_r = bi[32000];"),
        ("arr.index_2d_1000", "bq[1000, 1000] = 1; global.bb_r = bq[1000, 1000];"),
        ("arr.scalar_after_array", "bk[2] = 5; bk = 7; global.bb_r = bk[2];"),
        ("arr.array_after_scalar", "bn = 3; bn[1] = 4; global.bb_r = bn[0];"),
        ("arr.string_elem_gap", "bl[1] = 'x'; global.bb_r = bl[0];"),
        ("arr.never_set", "global.bb_r = bm_never[0];"),
        ("arr.global_gap", "global.ga[2] = 5; global.bb_r = global.ga[0];"),
        ("arr.index_string", "bo[1] = 1; global.bb_r = bo['1'];"),
    ] {
        t.push(err_stmt(key, code));
    }

    t.push(sx("arr.builtin_alarm_2d", "alarm[0, 3] = 4; global.bb_r = alarm[3]; alarm[3] = -1;"));
    t.push(sx("arr.builtin_x_index", "global.bb_r = x[0];"));
    t.push(sx("arr.builtin_x_index1", "global.bb_r = x[1];"));
    t.push(sx("math.min_none", "global.bb_r = min();"));
    t.push(sx("math.max_none", "global.bb_r = max();"));
    t.push(sx("math.choose_none", "global.bb_r = choose();"));

    // ---- arguments (GML-11, FF-18)
    t.extend(arg_tests());

    // ---- var scoping (GML-12)
    t.push(err_stmt(
        "var.shadow_x",
        "x = 5; execute_string(\"var x; x = 999; global.bb_q = x;\"); global.bb_r = string(global.bb_q) + '/' + string(x);",
    ));
    t.push(err_stmt(
        "var.lexical_unexecuted",
        "execute_string(\"if (false) { var qq; } qq = 3;\"); global.bb_r = variable_local_exists('qq');",
    ));
    t.push(err_stmt(
        "var.assign_before_var",
        "execute_string(\"qr = 3; var qr; qr = 4;\"); global.bb_r = qr;",
    ));
    t.push(Test {
        name: "var.across_actions".into(),
        actions: vec![
            "error_occurred = false; global.bb_r = '<unset>';".into(),
            "var vv; vv = 5;".into(),
            "global.bb_r = vv;".into(),
            "bb_log('var.across_actions', global.bb_r); bb_log('var.across_actions.err', error_occurred);".into(),
        ],
    });
    t.push(err_stmt(
        "var.in_with",
        "var lw; lw = 4; with (bb_out) global.bb_r = lw;",
    ));
    t.push(err_stmt("var.uninit_read", "var lu; global.bb_r = lu;"));
    t.push(err_stmt(
        "var.redeclare",
        "var lr; lr = 2; var lr; global.bb_r = lr;",
    ));

    // ---- local (GML-13)
    t.extend(errs(&[("local.value", "local")]));
    t.push(sx("local.dot", "var lx; lx = 3; global.bb_r = local.lx;"));
    t.push(sx("local.dot_undeclared", "local.ly = 3; global.bb_r = local.ly;"));

    // ---- lexing / syntax (GML-14, GML-15)
    for (key, src) in [
        ("syn.percent", "global.bb_r = 7 % 3;"),
        ("syn.colon_eq_stmt", "global.bb_r := 5;"),
        ("syn.colon_eq_cond", "var t; t = 0; if (t := 3) global.bb_r = 'assign'; else global.bb_r = 'compare';"),
        ("syn.eq_in_expr", "global.bb_r = (3 = 3);"),
        ("syn.leading_dot", "global.bb_r = .5;"),
        ("syn.trailing_dot", "global.bb_r = 12.;"),
        ("syn.double_dot", "global.bb_r = 1.2.3;"),
        ("syn.exp_notation", "global.bb_r = 1e3;"),
        ("syn.hex_lower", "global.bb_r = $ff;"),
        ("syn.hex_12_digits", "global.bb_r = $FFFFFFFFFFFF;"),
        ("syn.plusplus_binary", "global.bb_r = 2 ++ 3;"),
        ("syn.minusminus_binary", "global.bb_r = 2 -- 3;"),
        ("syn.incr_stmt", "var i; i = 1; i++; global.bb_r = i;"),
        ("syn.notnot", "global.bb_r = !!3;"),
        ("syn.ltgt", "global.bb_r = (1 <> 2);"),
        ("syn.begin_end", "begin global.bb_r = 4 end"),
        ("syn.if_then", "if 1 then global.bb_r = 6"),
        ("syn.no_semicolons", "global.bb_r = 3 global.bb_q = 4"),
        ("syn.continue_outside", "global.bb_r = 1; continue; global.bb_r = 2;"),
        ("syn.break_outside", "global.bb_r = 1; break; global.bb_r = 2;"),
        ("syn.bare_return", "global.bb_r = 1; return; global.bb_r = 2;"),
        ("syn.exit", "global.bb_r = 1; exit; global.bb_r = 2;"),
        ("syn.not_keyword", "global.bb_r = not 0;"),
        ("syn.and_or_words", "global.bb_r = 1 and 0 or 1;"),
        ("syn.mod_assign", "var m; m = 7; m mod= 3; global.bb_r = m;"),
        ("syn.or_assign", "var m; m = 4; m |= 1; global.bb_r = m;"),
        ("syn.xor_assign", "var m; m = 5; m ^= 1; global.bb_r = m;"),
        ("syn.string_newline", "global.bb_r = string_length('a\r\nb');"),
        ("syn.ident_long", "abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij = 4; global.bb_r = abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij;"),
        ("syn.case_sensitive", "Zq = 1; zq = 2; global.bb_r = Zq;"),
        ("syn.switch_string", "switch ('b') { case 'a': global.bb_r = 1; break; case 'b': global.bb_r = 2; break; default: global.bb_r = 3; }"),
        ("syn.switch_fallthrough", "global.bb_r = 0; switch (1) { case 1: global.bb_r += 1; case 2: global.bb_r += 10; break; case 3: global.bb_r += 100; }"),
        ("syn.switch_mixed", "global.bb_r = 'none'; switch (1) { case '1': global.bb_r = 'str'; break; case 1: global.bb_r = 'real'; break; }"),
        ("syn.switch_continue", "var i, n; n = 0; for (i = 0; i < 3; i += 1) { switch (i) { case 1: continue; } n += 1; } global.bb_r = n;"),
        ("syn.do_until", "var n; n = 0; do n += 1; until (n >= 3); global.bb_r = n;"),
        ("syn.for_no_semicolons", "var i, n; n = 0; for (i = 0 i < 3 i += 1) n += 1; global.bb_r = n;"),
        ("syn.with_break", "global.bb_r = 0; with (all) { global.bb_r += 1; break; }"),
    ] {
        t.push(sx(key, src));
    }
    // operator precedence (all through execute_string in case one is a parse error)
    for (key, e) in [
        ("prec.add_mul", "2 + 3 * 4"),
        ("prec.sub_sub", "10 - 2 - 3"),
        ("prec.div_mul", "6 / 2 * 3"),
        ("prec.mul_mod", "2 * 3 mod 4"),
        ("prec.mod_mul", "7 mod 4 * 2"),
        ("prec.div_kw_mul", "5 div 2 * 2"),
        ("prec.or_and_bit", "1 | 2 & 3"),
        ("prec.and_or_bit", "3 & 5 | 2"),
        ("prec.xor_and", "1 ^ 3 & 1"),
        ("prec.or_xor", "1 | 2 ^ 3"),
        ("prec.shl_add", "1 << 2 + 1"),
        ("prec.add_shl", "1 + 2 << 1"),
        ("prec.neg_mul", "-2 * -3"),
        ("prec.not_add", "!1 + 1"),
        ("prec.bitnot_add", "~1 + 1"),
        ("prec.neg_mod", "-3 mod 2"),
        ("prec.and_or", "1 && 0 || 1"),
        ("prec.or_and", "1 || 0 && 0"),
        ("prec.lxor_and", "1 ^^ 1 && 0"),
        ("prec.lt_eq", "1 < 2 == 1"),
        ("prec.eq_eq", "2 == 2 == 1"),
        ("prec.gt_gt", "3 > 2 > 1"),
        ("prec.eq_and", "1 == 1 && 2"),
        ("prec.not_eq", "!0 == 1"),
        ("prec.cmp_add", "1 + 1 == 2"),
        ("prec.bit_cmp", "1 | 2 == 3"),
        ("prec.and_cmp", "6 & 3 == 2"),
        ("prec.str_mul", "'ab' + 'c' == 'abc'"),
    ] {
        t.push(sx(key, &format!("global.bb_r = {e};")));
    }

    // ---- wrong types / references (GML-16)
    for (key, code) in [
        ("type.x_string", "x = 'a'; global.bb_r = x; x = 0;"),
        ("type.speed_string", "speed = 'a'; global.bb_r = speed; speed = 0;"),
        ("type.score_string", "score = 'abc'; global.bb_r = score; score = 0;"),
        ("type.depth_frac", "depth = 2.7; global.bb_r = depth; depth = 0;"),
        ("type.visible_2", "visible = 2; global.bb_r = visible; visible = 1;"),
        ("type.solid_0_5", "solid = 0.5; global.bb_r = solid; solid = 0;"),
        ("type.image_index_str", "image_index = '1'; global.bb_r = image_index; image_index = 0;"),
        ("type.readonly_id", "execute_string(\"id = 5;\"); global.bb_r = (id == 5);"),
        ("ref.noinst_read", "global.bb_r = bb_noinst.x;"),
        ("ref.noinst_write", "bb_noinst.zz = 5; global.bb_r = 1;"),
        ("ref.noone_read", "global.bb_r = noone.x;"),
        ("ref.noone_write", "noone.x = 5; global.bb_r = 1;"),
        ("ref.small_number_owner", "global.bb_r = (123).x;"),
        ("ref.bad_instance_id", "global.bb_r = (999999).x;"),
        ("ref.uninit_var", "global.bb_r = zz_never_set;"),
        ("ref.uninit_global", "global.bb_r = global.zz_never_set;"),
        ("ref.uninit_other", "global.bb_r = other.zz_never_set;"),
        ("ref.self_dot", "x = 3; global.bb_r = self.x; x = 0;"),
        ("ref.global_dot_x", "global.x = 5; global.bb_r = global.x;"),
        ("ref.inst_dot_score", "score = 4; global.bb_r = id.score; score = 0;"),
        ("ref.inst_dot_global_builtin_write", "id.score = 6; global.bb_r = score; score = 0;"),
        ("ref.object_name_value", "global.bb_r = bb_noinst;"),
        ("ref.script_name_value", "global.bb_r = sc_count;"),
        ("ref.user_constant", "global.bb_r = string(bb_k1) + '/' + string(bb_k2) + '/' + bb_k3;"),
        ("ref.all_dot_read", "global.bb_r = all.x;"),
        ("ref.other_in_driver_create", "global.bb_r = object_get_name(other.object_index);"),
        ("ref.self_value", "global.bb_r = self;"),
        ("ref.other_value", "global.bb_r = other;"),
    ] {
        t.push(err_stmt(key, code));
    }

    // ---- error recovery (GML-17)
    t.push(Test {
        name: "errcont".into(),
        actions: vec![
            "error_occurred = false; global.bb_q = 0; global.bb_q2 = 0;".into(),
            "global.bb_q = 1; global.bb_tmp = chr(97) + 1; global.bb_q = 2;".into(),
            "bb_log('errcont.rest_of_action', global.bb_q); bb_log('errcont.next_action_runs', 1);".into(),
            "global.bb_q = 1; sc_err(); global.bb_q = 3;".into(),
            "bb_log('errcont.script_rest', global.bb_q2); bb_log('errcont.caller_rest', global.bb_q);".into(),
            "global.bb_q = 0; with (bb_out) { global.bb_tmp = chr(97) + 1; global.bb_q = 1; } global.bb_q += 10;".into(),
            "bb_log('errcont.in_with', global.bb_q);".into(),
            "global.bb_q = 0; repeat (3) { global.bb_q += 1; global.bb_tmp = chr(97) * 2; }".into(),
            "bb_log('errcont.in_loop', global.bb_q);".into(),
        ],
    });
    t.push(err_stmt(
        "errcont.error_last_after_success",
        "error_occurred = false; global.bb_tmp = 1 + 1; global.bb_r = error_occurred;",
    ));

    // ---- string functions (G-22, G-23, G-24)
    t.extend(errs(&[
        ("sf.copy_0_2", "string_copy('abcdef', 0, 2)"),
        ("sf.copy_m1_3", "string_copy('abcdef', -1, 3)"),
        ("sf.copy_2_m1", "string_copy('abcdef', 2, -1)"),
        ("sf.copy_5_10", "string_copy('abcdef', 5, 10)"),
        ("sf.copy_7_1", "string_copy('abcdef', 7, 1)"),
        ("sf.copy_2_5_2", "string_copy('abcdef', 2.5, 2)"),
        ("sf.copy_2_4_2_6", "string_copy('abcdef', 2.4, 2.6)"),
        ("sf.pos_empty", "string_pos('', 'abc')"),
        ("sf.pos_c", "string_pos('c', 'abc')"),
        ("sf.pos_longer", "string_pos('abcd', 'abc')"),
        ("sf.count_aa", "string_count('aa', 'aaaa')"),
        ("sf.count_empty", "string_count('', 'abc')"),
        ("sf.delete_0_1", "string_delete('abc', 0, 1)"),
        ("sf.delete_2_10", "string_delete('abc', 2, 10)"),
        ("sf.delete_m1_2", "string_delete('abc', -1, 2)"),
        ("sf.insert_0", "string_insert('x', 'abc', 0)"),
        ("sf.insert_10", "string_insert('x', 'abc', 10)"),
        ("sf.insert_4", "string_insert('x', 'abc', 4)"),
        ("sf.char_at_0", "string_char_at('abc', 0)"),
        ("sf.char_at_4", "string_char_at('abc', 4)"),
        ("sf.char_at_1_5", "string_char_at('abc', 1.5)"),
        ("sf.replace_first", "string_replace('abab', 'ab', 'x')"),
        ("sf.replace_all_grow", "string_replace_all('aaa', 'a', 'aa')"),
        ("sf.replace_all_empty", "string_replace_all('abc', '', 'x')"),
        ("sf.lower_latin1", "ord(string_lower(chr(192)))"),
        ("sf.upper_latin1", "ord(string_upper(chr(233)))"),
        ("sf.lower_mixed", "string_lower('AbC')"),
        ("sf.digits", "string_digits('a1b2.3-4')"),
        ("sf.letters", "string_letters('a1B2_')"),
        ("sf.lettersdigits", "string_lettersdigits('a_1 B')"),
        ("sf.letters_latin1", "string_length(string_letters(chr(233)))"),
        ("sf.ord_empty", "ord('')"),
        ("sf.ord_ab", "ord('ab')"),
        ("sf.chr_256", "ord(chr(256))"),
        ("sf.chr_65_6", "chr(65.6)"),
        ("sf.chr_m1", "string_length(chr(-1))"),
        ("sf.chr_0_len", "string_length(chr(0))"),
        ("sf.nul_inside", "string_length('a' + chr(0) + 'b')"),
        ("sf.repeat_2_5", "string_repeat('ab', 2.5)"),
        ("sf.repeat_m1", "string_repeat('ab', -1)"),
        ("sf.length_real", "string_length(12.5)"),
        ("sf.upper_real", "string_upper(5)"),
        ("sf.copy_real", "string_copy(12345, 2, 2)"),
    ]));
    t.extend(errs(&[
        ("real.abc", "real('abc')"),
        ("real.12abc", "real('12abc')"),
        ("real.lead_space", "real(' 12')"),
        ("real.trail_space", "real('12 ')"),
        ("real.1e3", "real('1e3')"),
        ("real.0x10", "real('0x10')"),
        ("real.dollar10", "real('$10')"),
        ("real.m_dot5", "real('-.5')"),
        ("real.empty", "real('')"),
        ("real.12_5_6", "real('12.5.6')"),
        ("real.plus3", "real('+3')"),
        ("real.comma", "real('1,5')"),
        ("real.of_real", "real(5)"),
        ("real.inf", "real('inf')"),
        ("real.dot", "real('.')"),
        ("real.minus", "real('-')"),
    ]));

    // ---- math functions (G-20, G-21, G-25)
    t.extend(errs(&[
        ("math.sqrt_m1", "sqrt(-1)"),
        ("math.ln_0", "ln(0)"),
        ("math.ln_m1", "ln(-1)"),
        ("math.log10_0", "log10(0)"),
        ("math.arcsin_2", "arcsin(2)"),
        ("math.arccos_m2", "arccos(-2)"),
        ("math.power_m8_third", "power(-8, 1 / 3)"),
        ("math.power_m8_2", "power(-8, 2)"),
        ("math.power_0_0", "power(0, 0)"),
        ("math.power_0_m1", "power(0, -1)"),
        ("math.exp_1000", "exp(1000)"),
        ("math.log2_8", "log2(8)"),
        ("math.logn_3_9", "logn(3, 9)"),
        ("math.logn_1_5", "logn(1, 5)"),
        ("math.max_mixed", "max('a', 1)"),
        ("math.max_str", "max('a', 'b')"),
        ("math.mean_4", "mean(1, 2, 3, 4)"),
        ("math.median_even", "median(1, 2, 3, 4)"),
        ("math.median_odd", "median(4, 1, 3)"),
        ("math.median_1", "median(5)"),
        ("math.lengthdir_x_90", "lengthdir_x(10, 90)"),
        ("math.lengthdir_y_90", "lengthdir_y(10, 90)"),
        ("math.lengthdir_y_0", "lengthdir_y(10, 0)"),
        ("math.lengthdir_x_90_is0", "(lengthdir_x(10, 90) == 0)"),
        ("math.pdir_1_1", "point_direction(0, 0, 1, 1)"),
        ("math.pdir_same", "point_direction(0, 0, 0, 0)"),
        ("math.pdir_m1_0", "point_direction(0, 0, -1, 0)"),
        ("math.pdir_0_1", "point_direction(0, 0, 0, 1)"),
        ("math.pdist", "point_distance(0, 0, 3, 4)"),
        ("math.arctan2", "arctan2(1, 0)"),
        ("math.degtorad", "degtorad(180)"),
        ("math.is_real_str", "is_real('')"),
        ("math.is_string_0", "is_string(0)"),
        ("math.sin_pi_is0", "(sin(pi) == 0)"),
    ]));

    // ---- dates (G-27); the *_string results depend on the Windows locale
    t.push(raw(
        "date.basic",
        &[r#"var d;
d = date_create_datetime(2020, 2, 29, 13, 5, 9);
bb_log('date.value', d);
bb_log('date.year', date_get_year(d));
bb_log('date.month', date_get_month(d));
bb_log('date.day', date_get_day(d));
bb_log('date.hour', date_get_hour(d));
bb_log('date.minute', date_get_minute(d));
bb_log('date.second', date_get_second(d));
bb_log('date.week', date_get_week(d));
bb_log('date.weekday', date_get_weekday(d));
bb_log('date.day_of_year', date_get_day_of_year(d));
bb_log('date.hour_of_year', date_get_hour_of_year(d));
bb_log('date.minute_of_year', date_get_minute_of_year(d));
bb_log('date.second_of_year', date_get_second_of_year(d));
bb_log('date.days_in_month', date_days_in_month(d));
bb_log('date.days_in_year', date_days_in_year(d));
bb_log('date.date_of', date_date_of(d));
bb_log('date.time_of', date_time_of(d));
bb_log('date.zero_weekday', date_get_weekday(0));
bb_log('date.week_2021_01_01', date_get_week(date_create_date(2021, 1, 1)));
bb_log('date.week_2019_12_30', date_get_week(date_create_date(2019, 12, 30)));"#],
    ));
    t.extend(errs(&[
        ("date.invalid_feb29", "date_create_date(2021, 2, 29)"),
        ("date.invalid_month13", "date_create_datetime(2020, 13, 1, 0, 0, 0)"),
        ("date.valid_feb30", "date_valid_datetime(2020, 2, 30, 0, 0, 0)"),
        ("date.leap_1900", "date_leap_year(date_create_date(1900, 1, 1))"),
        ("date.leap_2000", "date_leap_year(date_create_date(2000, 1, 1))"),
        ("date.inc_month_jan31", "date_get_day(date_inc_month(date_create_date(2020, 1, 31), 1))"),
        ("date.inc_year_feb29", "date_get_day(date_inc_year(date_create_date(2020, 2, 29), 1))"),
        ("date.inc_day_frac", "date_inc_day(date_create_date(2020, 1, 1), 0.5) - date_create_date(2020, 1, 1)"),
        ("date.month_span", "date_month_span(date_create_date(2020, 1, 31), date_create_date(2020, 3, 1))"),
        ("date.year_span", "date_year_span(date_create_date(2019, 6, 1), date_create_date(2020, 6, 1))"),
        ("date.day_span_reverse", "date_day_span(date_create_date(2020, 1, 2), date_create_date(2020, 1, 1))"),
        ("date.week_span", "date_week_span(date_create_date(2020, 1, 1), date_create_date(2020, 1, 11))"),
        ("date.hour_span", "date_hour_span(date_create_datetime(2020, 1, 1, 0, 0, 0), date_create_datetime(2020, 1, 1, 5, 30, 0))"),
        ("date.cmp_date_same_day", "date_compare_date(date_create_datetime(2020, 1, 1, 1, 0, 0), date_create_datetime(2020, 1, 1, 23, 0, 0))"),
        ("date.cmp_datetime", "date_compare_datetime(date_create_datetime(2020, 1, 1, 1, 0, 0), date_create_datetime(2020, 1, 1, 23, 0, 0))"),
        ("date.cmp_time", "date_compare_time(date_create_datetime(2020, 1, 1, 23, 0, 0), date_create_datetime(2020, 1, 2, 1, 0, 0))"),
        ("date.epoch", "date_create_date(1899, 12, 30)"),
        ("date.year1", "date_create_date(1, 1, 1)"),
        ("date.year10000", "date_create_date(10000, 1, 1)"),
        ("date.is_today", "date_is_today(date_current_datetime())"),
        ("date.current_is_real", "is_real(date_current_datetime())"),
        ("date.weekday_var_range", "(current_weekday >= 0 && current_weekday <= 7)"),
        ("date.create_time", "date_create_time(25, 0, 0)"),
    ]));
    t.push(raw(
        "datestr",
        &[r#"var d;
d = date_create_datetime(2020, 2, 29, 13, 5, 9);
bb_log('datestr.datetime', date_datetime_string(d));
bb_log('datestr.date', date_date_string(d));
bb_log('datestr.time', date_time_string(d));"#],
    ));

    // ---- misc (SND-6, sound basics are Lite features)
    t.extend(errs(&[
        ("misc.sound_play_missing", "sound_play(99)"),
        ("misc.object_exists_m1", "object_exists(-1)"),
        ("misc.instance_exists_noone", "instance_exists(noone)"),
        ("misc.instance_number_all", "instance_number(all) >= 2"),
        ("misc.is_real_id", "is_real(id)"),
    ]));
    if let Some((k, n)) = part {
        let per = t.len().div_ceil(n);
        t = t.into_iter().skip(k * per).take(per).collect();
    }
    t.push(vals("lang.done", &[("lang.done", "1")]));

    let frames = unit_project(&mut g, &fw, t, 60);
    Spec {
        name,
        title: "GML language semantics, conversions and library edge cases",
        lite: "Lite-safe (code only; uses execute_string to isolate syntax that may not compile). Errors are not shown (Display errors off) but recorded via error_occurred / error_last.",
        frames,
        keys: lang_keys(),
        project: g.finish(),
    }
}

pub fn lang_keys() -> Vec<KeyDoc> {
    vec![
        k("truth.", "GML-1", "truthiness of reals near 0.5 and of strings in conditions"),
        k("eq.", "GML-3", "exact vs tolerant real comparison"),
        k("mix.", "GML-2", "mixed real/string comparison and arithmetic: error or value"),
        k("strcmp.", "GML-2, gml-language.md §8", "string ordering"),
        k("str.", "GML-5, G-11", "string() formatting"),
        k("strfmt.", "GML-5", "string_format rounding/padding"),
        k("round.", "GML-4, G-21", "round/floor/ceil/frac/sign"),
        k("repeat.", "GML-4", "repeat count conversion"),
        k("idx.", "GML-4", "array index conversion"),
        k("bit.", "GML-4", "bitwise operand conversion, 32 vs 64 bit"),
        k("div.", "GML-4, GML-6", "div with negatives / fractions"),
        k("mod.", "GML-4, GML-6", "mod with negatives / fractions"),
        k("div0.", "GML-6", "division by zero"),
        k("arr.", "GML-7", "array storage model (a vs a[0] vs a[0,0]), bounds"),
        k("args.", "GML-11, G-2, FF-18, IDE-12", "script arguments (with uninitialized flag bit 1 set)"),
        k("var.", "GML-12", "var scoping"),
        k("local.", "GML-13, G-6", "the local constant"),
        k("syn.", "GML-14, GML-15, G-8", "lexer/parser edge cases (via execute_string)"),
        k("prec.", "gml-language.md §4 precedence", "operator precedence"),
        k("type.", "GML-16", "wrong type assigned to built-in variables"),
        k("ref.", "GML-16, G-3, G-9, GML-10, GML-20", "instance references, uninitialised variables"),
        k("errcont.", "GML-17, RT-20, R-9", "what still runs after an ignored error"),
        k("sf.", "G-23, G-24", "string function edge cases"),
        k("real.", "G-22", "real() parsing"),
        k("math.", "G-20, G-21, G-25", "math function edge cases"),
        k("date.", "G-27", "date functions"),
        ki("datestr.", "G-27", "date string formats (locale dependent)"),
        k("misc.", "SND-6", "misc"),
        ki(".msg", "GML-18", "error message text (error_last)"),
        k(".err", "GML-18", "whether an error occurred"),
        k(".aborted", "GML-17", "the whole event was abandoned after an error"),
        k("lang.done", "-", "end marker"),
    ]
}

pub fn build_args0() -> Spec {
    let mut g = Gen::new("bb_args0", 81004);
    g.p.settings.uninitialized_extra_bits = 0;
    let fw = install(&mut g);
    arg_scripts(&mut g);
    let mut t = arg_tests();
    t.push(vals("args0.done", &[("args0.done", "1")]));
    let frames = unit_project(&mut g, &fw, t, 60);
    Spec {
        name: "bb_args0",
        title: "Script argument tests with the settings flag bit 1 (uninitialized arguments) cleared",
        lite: "Lite-safe.",
        frames,
        keys: vec![
            k("args.", "GML-11, FF-18, IDE-12", "same tests as bb_lang with flag bit 1 = 0; compare with bb_lang to learn what the bit does"),
            ki(".msg", "GML-18", "error message text"),
            k("args0.done", "-", "end marker"),
        ],
        project: g.finish(),
    }
}
