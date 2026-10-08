//! bb_motion: friction/gravity order, speed/direction coupling, animation,
//! paths, timelines, solid collisions, bbox rounding, view following.

use super::events::{clock, BLUE, GREEN, RED};
use crate::builder::{ev, Gen};
use crate::framework::*;
use instance_format::View;

const POS: &str = "bb_fmt(x) + ',' + bb_fmt(y) + ',' + bb_fmt(hspeed) + ',' + bb_fmt(vspeed)";

pub fn build() -> Spec {
    let mut g = Gen::new("bb_motion", 81007);
    let fw = install(&mut g);
    let s16 = g.sprite("spr16", 16, 16, &[RED], (0, 0));
    let s16o = g.sprite("spr16o", 16, 16, &[GREEN], (8, 8));
    let s3 = g.sprite("spr3f", 16, 16, &[RED, GREEN, BLUE], (0, 0));
    // manual bbox reaching outside the image
    let smb = g.sprite_frames(
        "spr_manualbox",
        vec![{
            let mut i = instance_format::RgbaImage::new(16, 16);
            for p in i.data.chunks_mut(4) {
                p.copy_from_slice(&BLUE);
            }
            i
        }],
        (0, 0),
        |s| {
            s.bbox_mode = 2;
            s.bbox_left = -4;
            s.bbox_right = 20;
            s.bbox_top = -2;
            s.bbox_bottom = 18;
        },
    );
    // only the centre 8x8 opaque: automatic bbox 4..11
    let spart = g.sprite_frames(
        "spr_partial",
        vec![{
            let mut i = instance_format::RgbaImage::new(16, 16);
            for y in 4..12u32 {
                for x in 4..12u32 {
                    let k = ((y * 16 + x) * 4) as usize;
                    i.data[k..k + 4].copy_from_slice(&RED);
                }
            }
            i
        }],
        (0, 0),
        |s| s.collision_shape = 0,
    );
    let pth = g.path("pth_m", false, false, &[(0.0, 0.0, 100.0), (20.0, 0.0, 100.0)]);
    let _ = pth;
    let tlm = g.timeline(
        "tl_m",
        &(0..5)
            .map(|k| {
                (
                    k,
                    format!("bb_log('mo.tl.' + tag + '.m{k}.s' + bb_pad(global.bb_step), timeline_position);"),
                )
            })
            .collect::<Vec<_>>(),
    );
    let _ = tlm;

    // clock: scenario steps 1..12 in rm_motion, then the view room, then unit tests
    let clk = clock(&mut g, "mo.seq", 12, false, "");
    // ---- friction / gravity
    let mfg = g.object("mo_fg");
    g.event(mfg, ev::CREATE, &["speed = 5; direction = 0; friction = 1; gravity = 0.5; gravity_direction = 270;"]);
    g.event(mfg, ev::BEGIN_STEP, &[&format!("if (global.bb_step <= 7) bb_log('mo.fg.s' + bb_pad(global.bb_step), {POS});")]);
    let mneg = g.object("mo_neg");
    g.event(mneg, ev::CREATE, &["speed = -3; friction = 1; bb_log('mo.neg.create', bb_fmt(speed) + ',' + bb_fmt(direction) + ',' + bb_fmt(hspeed) + ',' + bb_fmt(vspeed));"]);
    g.event(mneg, ev::BEGIN_STEP, &["if (global.bb_step <= 5) bb_log('mo.neg.s' + bb_pad(global.bb_step), bb_fmt(x) + ',' + bb_fmt(speed) + ',' + bb_fmt(direction));"]);
    // gravity only with friction larger than speed (does friction cancel gravity?)
    let mgf = g.object("mo_gf");
    g.event(mgf, ev::CREATE, &["gravity = 0.5; gravity_direction = 270; friction = 2;"]);
    g.event(mgf, ev::BEGIN_STEP, &[&format!("if (global.bb_step <= 4) bb_log('mo.gf.s' + bb_pad(global.bb_step), {POS});")]);

    // ---- animation
    let mut anims = Vec::new();
    for (tag, spd) in [("an1", "0.4"), ("an2", "1.5"), ("an3", "-1"), ("an4", "1")] {
        let o = g.object(&format!("mo_{tag}"));
        g.obj(o).sprite = s3;
        g.event(o, ev::CREATE, &[&format!("image_speed = {spd};")]);
        g.event(o, ev::STEP, &[&format!("if (global.bb_step <= 8) bb_log('mo.{tag}.st.s' + bb_pad(global.bb_step), image_index);")]);
        g.event(o, ev::DRAW, &[&format!("if (global.bb_step <= 4) bb_log('mo.{tag}.dr.s' + bb_pad(global.bb_step), image_index); draw_sprite(sprite_index, image_index, x, y);")]);
        g.event(o, ev::ANIM_END, &[&format!("bb_log('mo.{tag}.AE.s' + bb_pad(global.bb_step), image_index);")]);
        anims.push(o);
    }

    // ---- paths: end actions 0..3, negative speed
    let mut paths = Vec::new();
    for (tag, spd, end) in [("pa0", 3, 0), ("pa1", 3, 1), ("pa2", 3, 2), ("pa3", 3, 3), ("pa4", -3, 0)] {
        let o = g.object(&format!("mo_{tag}"));
        g.event(o, ev::CREATE, &[&format!("path_start(pth_m, {spd}, {end}, false); bb_log('mo.{tag}.create', bb_fmt(x) + ',' + bb_fmt(path_position));")]);
        g.event(o, ev::STEP, &[&format!("if (global.bb_step <= 11) bb_log('mo.{tag}.s' + bb_pad(global.bb_step), bb_fmt(x) + ',' + bb_fmt(path_position) + ',' + bb_fmt(path_speed));")]);
        g.event(o, ev::PATH_END, &[&format!("bb_log('mo.{tag}.PE.s' + bb_pad(global.bb_step), bb_fmt(x) + ',' + bb_fmt(path_position));")]);
        paths.push(o);
    }

    // ---- timelines
    let mut tls = Vec::new();
    for (tag, setup) in [
        ("tm1", ""),
        ("tm2", "timeline_speed = 2;"),
        ("tm3", "timeline_speed = 0.5;"),
        ("tm4", "timeline_loop = true;"),
        ("tm5", "timeline_position = 10;"),
        ("tm6", "timeline_speed = -1; timeline_position = 4;"),
        ("tm7", "timeline_position = 2.5;"),
    ] {
        let o = g.object(&format!("mo_{tag}"));
        g.event(
            o,
            ev::CREATE,
            &[
                &format!("tag = '{tag}'; timeline_index = tl_m; timeline_position = 0;"),
                "timeline_running = true;",
                setup,
            ],
        );
        g.event(o, ev::END_STEP, &[&format!("if (global.bb_step <= 7) bb_log('mo.tl.{tag}.pos.s' + bb_pad(global.bb_step), timeline_position);")]);
        tls.push(o);
    }

    // ---- solid collisions: 4 rows (mover solid?, wall solid?)
    let mv_n = g.object("mo_mover");
    let mv_s = g.object("mo_mover_solid");
    let wl_n = g.object("mo_wall");
    let wl_s = g.object("mo_wall_solid");
    g.obj(mv_s).solid = true;
    g.obj(wl_s).solid = true;
    for o in [mv_n, mv_s, wl_n, wl_s] {
        g.obj(o).sprite = s16;
    }
    for mv in [mv_n, mv_s] {
        g.event(mv, ev::CREATE, &["hspeed = 6;"]);
        for wl in [wl_n, wl_s] {
            g.event(mv, ev::collision(wl), &["if (global.bb_step <= 7) { bb_ev(tag + '.CO' + bb_fmt(x)); bb_log('mo.sol.' + tag + '.co.s' + bb_pad(global.bb_step), bb_fmt(x) + ',' + bb_fmt(hspeed) + ',' + bb_fmt(other.x)); }"]);
        }
        g.event(mv, ev::END_STEP, &["if (global.bb_step <= 7) bb_log('mo.sol.' + tag + '.es.s' + bb_pad(global.bb_step), bb_fmt(x) + ',' + bb_fmt(hspeed));"]);
        g.event(mv, ev::STEP, &["if (global.bb_step <= 7) bb_ev(tag + '.ST' + bb_fmt(x));"]);
    }
    for wl in [wl_n, wl_s] {
        for mv in [mv_n, mv_s] {
            g.event(wl, ev::collision(mv), &["if (global.bb_step <= 7) { bb_ev(tag + '.CO' + bb_fmt(x)); bb_log('mo.sol.' + tag + '.co.s' + bb_pad(global.bb_step), bb_fmt(x)); }"]);
        }
    }
    // a mover hitting two solid walls at once: collision event order per pair
    let rm = g.room("rm_motion", ROOM_W, ROOM_H);
    g.place(rm, fw.out_obj, 0, 0, "");
    g.place(rm, clk, 0, 0, "");
    g.place(rm, mfg, 100, 100, "");
    g.place(rm, mneg, 300, 100, "");
    g.place(rm, mgf, 400, 100, "");
    for (i, o) in anims.iter().enumerate() {
        g.place(rm, *o, 500 + 20 * i as i32, 100, "");
    }
    for (i, o) in paths.iter().enumerate() {
        g.place(rm, *o, 600, 150 + 10 * i as i32, "");
    }
    for o in &tls {
        g.place(rm, *o, 0, 0, "");
    }
    for (row, (mv, wl)) in [(mv_n, wl_s), (mv_s, wl_s), (mv_s, wl_n), (mv_n, wl_n)].iter().enumerate() {
        let y = 300 + 40 * row as i32;
        g.place(rm, *mv, 100, y, &format!("tag = 'r{}m';", row + 1));
        g.place(rm, *wl, 130, y, &format!("tag = 'r{}w';", row + 1));
    }
    // row 5: non-solid mover between two solid walls reached in the same step
    g.place(rm, mv_n, 100, 500, "tag = 'r5m';");
    g.place(rm, wl_s, 124, 500, "tag = 'r5w1';");
    g.place(rm, wl_s, 128, 504, "tag = 'r5w2';");

    // ---- view following (separate room)
    let vf = g.object("mo_viewfollow");
    g.obj(vf).sprite = s16;
    g.event(vf, ev::CREATE, &["hspeed = 37; t = 0;"]);
    g.event(
        vf,
        ev::BEGIN_STEP,
        &[r#"t += 1;
if (t <= 30) bb_log('mo.view.t' + bb_pad(t), bb_fmt(view_xview[0]) + ',' + bb_fmt(view_yview[0]) + ',' + bb_fmt(x));
if (t == 12) { x = 2900; }
if (t == 16) { view_hborder[0] = 100; view_hspeed[0] = 8; x = 1500; hspeed = 0; }
if (t == 22) { x = -50; y = -40; }
if (t == 26) { view_hborder[0] = 600; view_hspeed[0] = -1; x = 1500; y = 384; }
if (t == 31) room_goto(rm_motion_tests);"#],
    );
    let rv = g.room("rm_view", 3000, 1000);
    {
        let r = g.room_mut(rv);
        r.views_enabled = true;
        r.views[0] = View {
            visible: true,
            view_x: 0,
            view_y: 0,
            view_w: ROOM_W,
            view_h: ROOM_H,
            port_x: 0,
            port_y: 0,
            port_w: ROOM_W,
            port_h: ROOM_H,
            h_border: 600,
            v_border: 32,
            h_speed: -1,
            v_speed: -1,
            follow_object: vf,
        };
    }
    g.place(rv, vf, 100, 384, "");
    // clock step 13 -> view room
    g.event(clk, ev::BEGIN_STEP, &["if (global.bb_step == 13 && room == rm_motion) room_goto(rm_view);"]);

    // ---- unit tests (coupling, image_index, bbox, place_meeting)
    let bbo = g.object("mo_bbox_o");
    g.obj(bbo).sprite = s16o;
    let bbt = g.object("mo_bbox_t");
    g.obj(bbt).sprite = s16;
    let bbm = g.object("mo_bbox_manual");
    g.obj(bbm).sprite = smb;
    let bbp = g.object("mo_bbox_partial");
    g.obj(bbp).sprite = spart;
    let img = g.object("mo_img");
    g.obj(img).sprite = s3;
    let bb4 = "bb_fmt(bbox_left) + ',' + bb_fmt(bbox_top) + ',' + bb_fmt(bbox_right) + ',' + bb_fmt(bbox_bottom)";
    let mut t: Vec<Test> = vec![
        err_stmt("mo.speed_neg", "direction = 0; speed = -3; global.bb_r = bb_fmt(speed) + ',' + bb_fmt(direction) + ',' + bb_fmt(hspeed) + ',' + bb_fmt(vspeed); speed = 0;"),
        err_stmt("mo.dir_neg", "direction = -90; global.bb_r = direction;"),
        err_stmt("mo.dir_370", "direction = 370; global.bb_r = direction;"),
        err_stmt("mo.dir_720_5", "direction = 720.5; global.bb_r = direction;"),
        err_stmt("mo.dir_preserved_at_0", "direction = 45; speed = 0; global.bb_r = direction; speed = 2; global.bb_r = bb_fmt(global.bb_r) + ',' + bb_fmt(hspeed) + ',' + bb_fmt(vspeed); speed = 0;"),
        err_stmt("mo.hv_to_dir", "speed = 0; hspeed = -3; vspeed = 0; global.bb_r = bb_fmt(direction) + ',' + bb_fmt(speed);"),
        err_stmt("mo.hv_zero_dir", "direction = 30; hspeed = 0; vspeed = 0; global.bb_r = bb_fmt(direction) + ',' + bb_fmt(speed);"),
        err_stmt("mo.hv_diag", "hspeed = 1; vspeed = -1; global.bb_r = bb_fmt(direction) + ',' + bb_fmt(speed); speed = 0;"),
        err_stmt("mo.motion_add", "motion_set(0, 2); motion_add(180, 3); global.bb_r = bb_fmt(speed) + ',' + bb_fmt(direction); speed = 0;"),
        err_stmt("mo.motion_set_neg", "motion_set(45, -2); global.bb_r = bb_fmt(speed) + ',' + bb_fmt(direction) + ',' + bb_fmt(hspeed) + ',' + bb_fmt(vspeed); speed = 0;"),
        err_stmt("mo.speed_after_hspeed", "speed = 5; direction = 90; hspeed = 3; global.bb_r = bb_fmt(speed) + ',' + bb_fmt(direction); speed = 0;"),
        err_stmt("mo.dir_precision", "speed = 1; direction = 33.3; global.bb_r = bb_fmt(direction) + ',' + bb_fmt(hspeed); speed = 0;"),
        err_stmt("img.index_5", "var i; i = instance_create(0, 0, mo_img); i.image_index = 5; global.bb_r = i.image_index;"),
        err_stmt("img.index_m1", "var i; i = instance_create(0, 0, mo_img); i.image_index = -1; global.bb_r = i.image_index;"),
        err_stmt("img.index_2_7", "var i; i = instance_create(0, 0, mo_img); i.image_index = 2.7; global.bb_r = i.image_index;"),
        err_stmt("img.index_3", "var i; i = instance_create(0, 0, mo_img); i.image_index = 3; global.bb_r = i.image_index;"),
        err_stmt("img.number", "var i; i = instance_create(0, 0, mo_img); global.bb_r = i.image_number;"),
        err_stmt("img.speed_default", "var i; i = instance_create(0, 0, mo_img); global.bb_r = i.image_speed;"),
    ];
    for (l, x, y) in [("10_4", "10.4", "20.6"), ("10_5", "10.5", "20.5"), ("11_5", "11.5", "21.5"), ("m0_5", "-0.5", "-1.5"), ("10_6", "10.6", "20.4")] {
        t.push(err_stmt(
            &format!("bbox.origin8.{l}"),
            &format!("var i; i = instance_create({x}, {y}, mo_bbox_o); with (i) global.bb_r = {bb4};"),
        ));
    }
    t.push(err_stmt("bbox.manual_outside_image", &format!("var i; i = instance_create(100, 100, mo_bbox_manual); with (i) global.bb_r = {bb4};")));
    t.push(err_stmt("bbox.partial_auto", &format!("var i; i = instance_create(200, 200, mo_bbox_partial); with (i) global.bb_r = {bb4};")));
    t.push(err_stmt("bbox.sprite_get", "global.bb_r = string(sprite_get_bbox_left(spr_manualbox)) + ',' + string(sprite_get_bbox_right(spr_manualbox)) + ',' + string(sprite_get_bbox_top(spr_manualbox)) + ',' + string(sprite_get_bbox_bottom(spr_manualbox));"));
    t.push(raw("bbox.collide_setup", &["instance_create(0, 0, mo_bbox_t); instance_create(100, 100, mo_bbox_manual); instance_create(200, 200, mo_bbox_partial);"]));
    for (l, x) in [("16", "16"), ("15_6", "15.6"), ("15_5", "15.5"), ("15_4", "15.4"), ("15", "15"), ("m16", "-16"), ("m15_5", "-15.5"), ("m15_4", "-15.4")] {
        t.push(err_stmt(
            &format!("coll.place_meeting_x{l}"),
            &format!("var j; j = instance_create(0, 300, mo_img); with (j) global.bb_r = place_meeting({x}, 0, mo_bbox_t); with (j) instance_destroy();"),
        ));
    }
    for (l, x, y) in [("15_5", "15.5", "5"), ("15_9", "15.9", "5"), ("16", "16", "5"), ("m0_5", "-0.5", "5"), ("m0_1", "-0.1", "5"), ("0", "0", "0")] {
        t.push(err_stmt(&format!("coll.point_{l}"), &format!("global.bb_r = (collision_point({x}, {y}, mo_bbox_t, false, false) != noone);")));
    }
    for (l, x, y) in [("in_box_out_image", "97", "108"), ("in_image", "101", "108"), ("box_edge_right", "120", "108"), ("box_edge_right_out", "121", "108")] {
        t.push(err_stmt(&format!("coll.manual_{l}"), &format!("global.bb_r = string(collision_point({x}, {y}, mo_bbox_manual, false, false) != noone) + '/' + string(collision_point({x}, {y}, mo_bbox_manual, true, false) != noone);")));
    }
    for (l, x, y) in [("transparent_corner", "201", "201"), ("opaque", "208", "208"), ("bbox_edge", "204", "204"), ("bbox_edge_out", "203", "204")] {
        t.push(err_stmt(&format!("coll.partial_{l}"), &format!("global.bb_r = string(collision_point({x}, {y}, mo_bbox_partial, false, false) != noone) + '/' + string(collision_point({x}, {y}, mo_bbox_partial, true, false) != noone);")));
    }
    t.push(vals("mo.done", &[("mo.done", "1")]));
    let ntests = t.len() as u64;
    let driver = unit_tests(&mut g, t);
    let rt = g.room("rm_motion_tests", ROOM_W, ROOM_H);
    g.room_mut(rt).speed = 60;
    g.place(rt, driver, 0, 0, "");

    Spec {
        name: "bb_motion",
        title: "Motion, animation, paths, timelines, solid collisions, bbox, view follow",
        lite: "Lite-safe (no scaling/rotation; those are in bb_optional). timeline_running/timeline_loop are GM 8.1 variables.",
        frames: 60 + ntests,
        keys: vec![
            k("mo.fg.", "RT-10, runtime.md §7.4", "x,y,hspeed,vspeed per step with friction 1 + gravity 0.5"),
            k("mo.gf.", "RT-10", "gravity with friction > speed"),
            k("mo.neg.", "RT-10", "negative speed with friction"),
            k("mo.an", "RT-01, runtime.md §7.5", "image_index per step (step/draw) and Animation End timing for image_speed 0.4/1.5/-1/1"),
            k("mo.pa", "RT-17", "path end actions 0..3 and negative path speed"),
            k("mo.tl.", "RT-17", "timeline moments with timeline_speed 1/2/0.5/loop/beyond end/negative/fractional start"),
            k("mo.sol.", "RT-06, R-14", "solid collision handling: position in collision event and after"),
            k("mo.seq.", "RT-06", "collision event order per step"),
            k("mo.view.", "RT-15", "view_xview/yview while following with hborder 600 > half view, then 100 with speed 8, clamping"),
            k("mo.", "RT-10", "speed/direction coupling unit tests"),
            k("img.", "runtime.md §7.5", "image_index wrap on assignment"),
            k("bbox.", "RT-11, R-11, R-12", "bbox rounding at fractional positions, manual box outside image"),
            k("coll.", "RT-11, R-11, R-12, R-13", "collision tests on bbox edges, manual box, precise vs bbox"),
            ki(".msg", "GML-18", "error message text"),
            k("mo.done", "-", "end marker"),
        ],
        project: g.finish(),
    }
}
