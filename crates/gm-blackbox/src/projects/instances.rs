//! bb_instances: instance order (step/draw at equal depth), depth changes,
//! collision event selection with parents, `with` / `other` semantics,
//! instance_change, destroy semantics.

use super::events::{clock, BLUE, GREEN, RED};
use crate::builder::{ev, Gen};
use crate::framework::*;

pub fn build() -> Spec {
    let mut g = Gen::new("bb_instances", 81006);
    let fw = install(&mut g);
    let s16 = g.sprite("spr16", 16, 16, &[RED], (0, 0));
    let s8 = g.sprite("spr8", 8, 8, &[BLUE], (0, 0));
    let s3 = g.sprite("spr3f", 16, 16, &[RED, GREEN, BLUE], (0, 0));

    // ---------------------------------------------------------------- scenario
    let clk = clock(&mut g, "in", 7, false, "");
    g.event(clk, ev::CREATE, &["global.pc = 0;"]);

    let po = g.object("in_p");
    let qo = g.object("in_q");
    let ro = g.object("in_r");
    let so = g.object("in_s");
    for o in [po, qo, ro, so] {
        g.obj(o).sprite = s16;
    }
    g.event(
        po,
        ev::CREATE,
        &["global.pc += 1; tag = 'p' + string(global.pc); bb_ev(tag + '.CR'); if (global.pc == 1) instance_create(0, 0, in_s);"],
    );
    g.event(qo, ev::CREATE, &["tag = 'q'; bb_ev('q.CR');"]);
    g.event(ro, ev::CREATE, &["tag = 'r'; bb_ev('r.CR');"]);
    g.event(so, ev::CREATE, &["tag = 's'; bb_ev('s.CR');"]);
    for o in [po, qo, ro, so] {
        g.event(o, ev::STEP, &["if (global.bb_step <= 4) bb_ev(tag + '.ST');"]);
        g.event(o, ev::DRAW, &["if (global.bb_step <= 4) bb_ev(tag + '.DR');"]);
    }
    // controller: order dumps at step 1, depth changes at step 2, new instance at step 3
    let ctl = g.object("in_ctl");
    g.event(
        ctl,
        ev::STEP,
        &[
            r#"var s, i, n;
if (global.bb_step == 1)
{
    s = ''; with (all) if (variable_local_exists('tag')) s += tag + ' ';
    bb_log('in.order.with_all', s);
    s = ''; with (in_p) s += tag + ' ';
    bb_log('in.order.with_p', s);
    s = ''; n = instance_number(in_p);
    for (i = 0; i < n; i += 1) s += (instance_find(in_p, i)).tag + ' ';
    bb_log('in.order.find_p', s);
    s = '';
    for (i = 0; i < instance_count; i += 1) with (instance_id[i]) if (variable_local_exists('tag')) s += tag + ' ';
    bb_log('in.order.instance_id', s);
    bb_log('in.order.nearest_tie', (instance_nearest(100, 100, in_p)).tag);
    bb_log('in.order.position_tie', (instance_position(104, 104, in_p)).tag);
    bb_log('in.order.place_tie', (collision_point(104, 104, all, false, true)).tag);
}"#,
            r#"if (global.bb_step == 2) { with (in_p) if (tag == 'p3') depth = -5; with (in_q) depth = 5; bb_ev('ctl.depths_changed'); }"#,
            r#"if (global.bb_step == 3) { with (instance_create(100, 100, in_p)) bb_ev(tag + '.created_step3'); }"#,
            r#"if (global.bb_step == 4) { with (in_p) depth = 0; with (in_q) depth = 0; }"#,
        ],
    );

    // collision event selection with parents (RT-07) and pair order (RT-06)
    let tp = g.object("in_tparent");
    let t = g.object("in_t");
    g.obj(t).parent = tp;
    let m = g.object("in_m");
    let np = g.object("in_nparent");
    let n = g.object("in_n");
    g.obj(n).parent = np;
    let cx = g.object("in_cx");
    let cy = g.object("in_cy");
    for o in [tp, t, m, np, n, cx, cy] {
        g.obj(o).sprite = s16;
    }
    g.event(m, ev::collision(t), &["if (global.bb_step <= 2) bb_ev('m.CO_t');"]);
    g.event(m, ev::collision(tp), &["if (global.bb_step <= 2) bb_ev('m.CO_tparent');"]);
    g.event(np, ev::collision(t), &["if (global.bb_step <= 2) bb_ev('n.CO_inherited');"]);
    g.event(cx, ev::collision(cy), &["if (global.bb_step <= 2) bb_ev('cx.CO' + other.tag);"]);
    g.event(cy, ev::collision(cx), &["if (global.bb_step <= 2) bb_ev(tag + '.COcx');"]);

    // equal-depth runtime-created vs placed (Pokemon3d: camera created after scenery)
    let rm = g.room("rm_in", ROOM_W, ROOM_H);
    g.room_mut(rm).creation_code = "with (instance_create(100, 100, in_r)) bb_ev('r.from_room_cc');".into();
    g.place(rm, fw.out_obj, 0, 0, "");
    g.place(rm, clk, 0, 0, "");
    g.place(rm, po, 100, 100, "");
    g.place(rm, po, 104, 104, "");
    g.place(rm, qo, 102, 102, "");
    g.place(rm, po, 108, 108, "");
    g.place(rm, ctl, 0, 0, "");
    g.place(rm, m, 300, 400, "");
    g.place(rm, t, 304, 400, "");
    g.place(rm, n, 400, 400, "");
    g.place(rm, t, 404, 400, "");
    g.place(rm, cy, 504, 400, "tag = 'cy1';");
    g.place(rm, cx, 500, 400, "");
    g.place(rm, cy, 496, 400, "tag = 'cy2';");
    let last_placed = g.p.last_instance_id;
    g.event(
        clk,
        ev::BEGIN_STEP,
        &[&format!(
            "if (global.bb_step == 1) {{ bb_log('in.id.r_minus_last_placed', (instance_find(in_r, 0)).id - {last_placed}); bb_log('in.id.s_minus_last_placed', (instance_find(in_s, 0)).id - {last_placed}); }}"
        )],
    );

    // ---------------------------------------------------------------- unit tests
    let names = ["wa", "wb", "wc", "wd", "we", "wf", "wg"];
    let mut wobj = std::collections::HashMap::new();
    for nm in names {
        let o = g.object(&format!("in_{nm}"));
        wobj.insert(nm, o);
    }
    let wd_child = g.object("in_wd_child");
    g.obj(wd_child).parent = wobj["wd"];
    g.event(wobj["we"], ev::CREATE, &["global.wen += 1; n = global.wen;"]);
    let oc = g.object("in_oc");
    g.event(oc, ev::CREATE, &["global.oc_other = other.id; global.oc_self = id; global.oc_other_obj = object_get_name(other.object_index);"]);
    let ou = g.object("in_ou");
    g.event(ou, ev::user(0), &["global.ou_other = other.id; global.ou_self = id;"]);
    g.event(ou, ev::DESTROY, &["global.ou_destroy_other = other.id; global.ou_destroys += 1;"]);
    let ic1 = g.object("in_ic1");
    let ic2 = g.object("in_ic2");
    {
        let o = g.obj(ic1);
        o.sprite = s16;
        o.depth = 3;
        o.solid = true;
        o.visible = true;
    }
    {
        let o = g.obj(ic2);
        o.sprite = s8;
        o.depth = 7;
        o.solid = false;
        o.visible = false;
        o.persistent = true;
        o.mask_sprite = s3;
    }
    g.event(ic1, ev::CREATE, &["global.ic_ev += 'c1 ';"]);
    g.event(ic1, ev::DESTROY, &["global.ic_ev += 'd1 ';"]);
    g.event(ic2, ev::CREATE, &["global.ic_ev += 'c2 ';"]);
    g.event(ic2, ev::DESTROY, &["global.ic_ev += 'd2 ';"]);

    let tests = vec![
        err_stmt(
            "with.snapshot_create",
            "instance_create(0, 0, in_wa); instance_create(0, 0, in_wa); global.v = 0; with (in_wa) { global.v += 1; if (global.v == 1) instance_create(0, 0, in_wa); } global.bb_r = string(global.v) + '/' + string(instance_number(in_wa)); with (in_wa) instance_destroy();",
        ),
        err_stmt(
            "with.destroy_others",
            "repeat (3) instance_create(0, 0, in_wb); global.v = 0; with (in_wb) { global.v += 1; if (global.v == 1) with (in_wb) if (id != other.id) instance_destroy(); } global.bb_r = string(global.v) + '/' + string(instance_number(in_wb));",
        ),
        err_stmt(
            "with.destroy_self_then_read",
            "instance_create(7, 0, in_wc); with (in_wc) { instance_destroy(); global.v = instance_exists(id); global.bb_r = x; } global.bb_r = string(global.bb_r) + '/' + string(global.v);",
        ),
        err_stmt(
            "with.children",
            "instance_create(0, 0, in_wd); instance_create(0, 0, in_wd_child); global.v = 0; with (in_wd) global.v += 1; global.bb_r = string(global.v) + '/' + string(instance_number(in_wd));",
        ),
        err_stmt(
            "with.dot_write_children",
            "in_wd.vv = 5; global.v = 0; with (in_wd_child) if (variable_local_exists('vv')) global.v += 1; global.bb_r = global.v;",
        ),
        err_stmt(
            "with.order_created",
            "global.wen = 0; repeat (4) instance_create(0, 0, in_we); global.bb_r = ''; with (in_we) global.bb_r += string(n) + ' ';",
        ),
        err_stmt(
            "with.find_order_created",
            "var i; global.bb_r = ''; for (i = 0; i < instance_number(in_we); i += 1) global.bb_r += string((instance_find(in_we, i)).n) + ' ';",
        ),
        err_stmt(
            "with.dot_read_multiple",
            "var i; i = instance_create(0, 0, in_wf); i.vv = 1; i = instance_create(0, 0, in_wf); i.vv = 2; global.bb_r = in_wf.vv;",
        ),
        err_stmt(
            "with.dot_write_multiple",
            "in_wf.vv = 9; global.v = 0; with (in_wf) if (vv == 9) global.v += 1; global.bb_r = global.v;",
        ),
        err_stmt(
            "with.noone",
            "global.v = 0; with (noone) global.v += 1; global.bb_r = global.v;",
        ),
        err_stmt(
            "with.other_inside",
            "var i; i = instance_create(0, 0, in_wg); with (i) global.bb_r = (other.id == other.other.id) + 10 * (other.object_index == in_wg);",
        ),
        err_stmt(
            "with.other_is_caller",
            "var me; me = id; with (in_wg) global.bb_r = (other.id == me);",
        ),
        err_stmt(
            "other.create_from_code",
            "instance_create(0, 0, in_oc); global.bb_r = string(global.oc_other == id) + '/' + string(global.oc_other == global.oc_self) + '/' + global.oc_other_obj;",
        ),
        err_stmt(
            "other.create_from_with",
            "var w; w = instance_find(in_wg, 0); with (w) instance_create(0, 0, in_oc); global.bb_r = string(global.oc_other == w) + '/' + string(global.oc_other == id) + '/' + global.oc_other_obj;",
        ),
        err_stmt(
            "other.user_event_in_with",
            "var u; u = instance_create(0, 0, in_ou); with (u) event_user(0); global.bb_r = string(global.ou_other == id) + '/' + string(global.ou_other == global.ou_self);",
        ),
        err_stmt(
            "other.event_perform_in_with",
            "var u; u = instance_find(in_ou, 0); global.ou_other = -1; with (u) event_perform(ev_other, ev_user0); global.bb_r = string(global.ou_other == id) + '/' + string(global.ou_other == global.ou_self);",
        ),
        err_stmt(
            "other.in_destroy",
            "var u; u = instance_find(in_ou, 0); global.ou_destroys = 0; with (u) instance_destroy(); global.bb_r = string(global.ou_destroy_other == id) + '/' + string(global.ou_destroy_other == u);",
        ),
        err_stmt(
            "destroy.immediate_event",
            "var u; global.ou_destroys = 0; u = instance_create(0, 0, in_ou); with (u) instance_destroy(); global.bb_r = string(global.ou_destroys) + '/' + string(instance_exists(u)) + '/' + string(instance_number(in_ou));",
        ),
        err_stmt(
            "destroy.twice",
            "var u; global.ou_destroys = 0; u = instance_create(0, 0, in_ou); with (u) { instance_destroy(); instance_destroy(); } global.bb_r = global.ou_destroys;",
        ),
        err_stmt(
            "destroy.read_after",
            "var u; u = instance_create(5, 0, in_ou); with (u) instance_destroy(); global.bb_r = u.x;",
        ),
        err_stmt(
            "change.props",
            "var i; global.ic_ev = ''; i = instance_create(0, 0, in_ic1); with (i) { image_speed = 0.25; instance_change(in_ic2, false); global.bb_r = object_get_name(object_index) + ' spr=' + sprite_get_name(sprite_index) + ' d=' + string(depth) + ' so=' + string(solid) + ' vi=' + string(visible) + ' pe=' + string(persistent) + ' ma=' + string(mask_index) + ' is=' + string(image_speed); }",
        ),
        err_stmt(
            "change.events_perf",
            "var i; global.ic_ev = ''; i = instance_create(0, 0, in_ic1); with (i) instance_change(in_ic2, true); global.bb_r = global.ic_ev; with (in_ic2) instance_destroy();",
        ),
        err_stmt(
            "change.events_noperf",
            "var i; global.ic_ev = ''; i = instance_create(0, 0, in_ic1); with (i) instance_change(in_ic2, false); global.bb_r = global.ic_ev; with (in_ic2) instance_destroy();",
        ),
        vals("in.done", &[("in.done", "1")]),
    ];
    let ntests = tests.len() as u64;
    let driver = unit_tests(&mut g, tests);
    // the clock creates the driver after the scenario (step 8 = done of scenario)
    // -> override: clock's done_step ends logging; driver runs afterwards.
    g.event(
        ctl,
        ev::STEP,
        &["if (global.bb_step == 6) instance_create(0, 0, bb_driver);"],
    );
    let _ = driver;
    Spec {
        name: "bb_instances",
        title: "Instance order, equal-depth draw order, depth changes, with/other, instance_change",
        lite: "Lite-safe.",
        frames: 60 + ntests,
        keys: vec![
            k("in.s", "RT-16, D3D-IMPL-2, RT-09, RT-07, RT-06, runtime.md §13.2", "per-step sequence: step and draw order of p1..p4/q/r/s, depth changes, collision events"),
            k("in.order.", "RT-16, D3D-IMPL-2, runtime.md §4.3", "with(all)/instance_find/instance_id order of equal-depth instances"),
            k("in.id.", "runtime.md §5.6", "instance id counter after placed instances"),
            k("with.", "GML-8, GML-9, G-10", "with snapshot, destroy during with, children"),
            k("other.", "GML-10", "other in create/user/destroy events"),
            k("destroy.", "runtime.md §5.6", "destroy event timing, double destroy"),
            k("change.", "RT-21", "instance_change"),
            ki(".msg", "GML-18", "error message text"),
            k(".err", "GML-18", "error occurred"),
            k("in.done", "-", "end marker"),
        ],
        project: g.finish(),
    }
}
