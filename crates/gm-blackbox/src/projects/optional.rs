//! bb_optional: tests that may need Pro features (sprite scaling/rotation,
//! draw_getpixel, file text formats) and bb_clash (resource names that clash
//! with built-in functions / constants).

use super::events::{BLUE, GREEN, RED};
use crate::builder::{ev, Gen};
use crate::framework::*;

pub fn build() -> Spec {
    let mut g = Gen::new("bb_optional", 81010);
    let fw = install(&mut g);
    let s16 = g.sprite("spr16", 16, 16, &[RED], (0, 0));
    let s16o = g.sprite("spr16o", 16, 16, &[GREEN], (8, 8));
    let sblue = g.sprite("spr16blue", 16, 16, &[BLUE], (0, 0));
    let o0 = g.object("op_o0");
    g.obj(o0).sprite = s16;
    let o8 = g.object("op_o8");
    g.obj(o8).sprite = s16o;
    let bb4 = "bb_fmt(bbox_left) + ',' + bb_fmt(bbox_top) + ',' + bb_fmt(bbox_right) + ',' + bb_fmt(bbox_bottom)";
    let mut t: Vec<Test> = Vec::new();
    for (l, xs, ys, a) in [
        ("xs2", "2", "1", "0"),
        ("xsm1", "-1", "1", "0"),
        ("ysm1", "1", "-1", "0"),
        ("s0_5", "0.5", "0.5", "0"),
        ("xs1_5", "1.5", "1", "0"),
        ("a45", "1", "1", "45"),
        ("a90", "1", "1", "90"),
        ("a30", "1", "1", "30"),
        ("a180", "1", "1", "180"),
        ("xs2_a45", "2", "1", "45"),
        ("xs0_3", "0.3", "1", "0"),
    ] {
        for (obj, on) in [("op_o0", "o0"), ("op_o8", "o8")] {
            t.push(err_stmt(
                &format!("xform.{on}.{l}"),
                &format!("var i; i = instance_create(100, 100, {obj}); with (i) {{ image_xscale = {xs}; image_yscale = {ys}; image_angle = {a}; global.bb_r = {bb4} + ' w=' + bb_fmt(sprite_width) + ' h=' + bb_fmt(sprite_height); instance_destroy(); }}"),
            ));
        }
    }
    // file_text_write_real number format (R-26)
    t.push(raw(
        "file.real_format",
        &[
            r#"var f, i;
f = file_text_open_write('bb_tmp_real.txt');
file_text_write_real(f, 1.5); file_text_writeln(f);
file_text_write_real(f, 3); file_text_writeln(f);
file_text_write_real(f, -0.25); file_text_writeln(f);
file_text_write_real(f, 1 / 3); file_text_writeln(f);
file_text_write_real(f, 100000000000); file_text_writeln(f);
file_text_write_real(f, 1.5); file_text_write_real(f, 2.5); file_text_writeln(f);
file_text_write_string(f, 'abc'); file_text_write_real(f, 7);
file_text_close(f);
f = file_text_open_read('bb_tmp_real.txt');
for (i = 0; i < 7; i += 1) { bb_log('file.real_line' + string(i), file_text_read_string(f)); file_text_readln(f); }
file_text_close(f);"#,
            r#"var f;
f = file_text_open_read('bb_tmp_real.txt');
bb_log('file.read_real_1', file_text_read_real(f));
file_text_readln(f);
bb_log('file.read_real_2', file_text_read_real(f));
bb_log('file.eof_mid', file_text_eof(f));
file_text_close(f);
file_delete('bb_tmp_real.txt');"#,
        ],
    ));
    t.push(err("file.open_missing", "file_text_open_read('bb_does_not_exist.txt')"));
    t.push(vals("op.done", &[("op.done", "1")]));
    let ntests = t.len() as u64;
    let driver = unit_tests(&mut g, t);

    // draw_getpixel probe: two equal-depth overlapping instances (placed red then
    // blue), and a placed red + runtime-created blue pair.
    let pr = g.object("op_px_red");
    g.obj(pr).sprite = s16;
    let pb = g.object("op_px_blue");
    g.obj(pb).sprite = sblue;
    let probe = g.object("op_probe");
    g.obj(probe).depth = -10;
    g.event(probe, ev::CREATE, &["n = 0; with (instance_create(600, 100, op_px_blue)) {}"]);
    g.event(
        probe,
        ev::DRAW,
        &[r#"n += 1;
if (n == 3)
{
    var c;
    c = draw_getpixel(508, 108);
    bb_log('px.placed_red_then_blue', string(color_get_red(c)) + ',' + string(color_get_blue(c)));
    c = draw_getpixel(608, 108);
    bb_log('px.placed_red_runtime_blue', string(color_get_red(c)) + ',' + string(color_get_blue(c)));
    c = draw_getpixel(708, 108);
    bb_log('px.placed_blue_then_red', string(color_get_red(c)) + ',' + string(color_get_blue(c)));
}"#],
    );
    let rm = g.room("rm_optional", ROOM_W, ROOM_H);
    g.room_mut(rm).speed = 60;
    g.place(rm, fw.out_obj, 0, 0, "");
    g.place(rm, pr, 500, 100, "");
    g.place(rm, pb, 500, 100, "");
    g.place(rm, pr, 600, 100, "");
    g.place(rm, pb, 700, 100, "");
    g.place(rm, pr, 700, 100, "");
    g.place(rm, probe, 0, 0, "");
    g.place(rm, driver, 0, 0, "");
    Spec {
        name: "bb_optional",
        title: "Possibly Pro-only: bbox under scale/rotation, draw_getpixel draw order, file_text_write_real",
        lite: "May fail in Lite (image_xscale/angle, draw_getpixel, file functions may be Pro-only). Failures are recorded per test.",
        frames: 30 + ntests,
        keys: vec![
            k("xform.", "RT-11, R-11", "bbox_* and sprite_width/height under image_xscale/yscale/angle (origin 0,0 and 8,8)"),
            k("px.", "RT-16, D3D-IMPL-2", "pixel colour where equal-depth instances overlap (which one drew last)"),
            k("file.", "R-26", "file_text_write_real format and read-back"),
            ki(".msg", "GML-18", "error message text"),
            k("op.done", "-", "end marker"),
        ],
        project: g.finish(),
    }
}

pub fn build_clash() -> Spec {
    let mut g = Gen::new("bb_clash", 81011);
    let fw = install(&mut g);
    // a script with a built-in function's name, a constant with a resource name
    g.script("abs", "return 42;");
    let _obj = g.object("bb_clashobj");
    g.constant("bb_clashobj", "77");
    g.constant("c_red", "5");
    let t = vec![
        err("clash.script_abs", "abs(-3)"),
        err("clash.const_vs_object", "bb_clashobj"),
        err("clash.const_vs_builtin_const", "c_red"),
        vals("clash.done", &[("clash.done", "1")]),
    ];
    let frames = unit_project(&mut g, &fw, t, 30);
    Spec {
        name: "bb_clash",
        title: "Name clashes: script named abs, constant named like an object / built-in constant",
        lite: "Lite-safe, but GM may refuse to load or run the project because of the clashes; if so, report the message (that itself answers GML-20).",
        frames,
        keys: vec![
            k("clash.", "GML-20", "resolution priority of clashing names"),
            ki(".msg", "GML-18", "error message text"),
        ],
        project: g.finish(),
    }
}
