//! bb_smoke1..4: a ladder of tiny projects, each adding one ingredient of the full
//! harness, to find what makes a generated project close instantly in GM 8.1.
//!
//! 1 = settings + one object drawing text with the default font
//! 2 = + a font resource (Courier New 8) used for drawing
//! 3 = + a generated sprite drawn on screen
//! 4 = + the full bb_* framework (scripts, output object, saver) with one test

use crate::builder::{ev, Gen};
use crate::framework::*;

const RED: [u8; 4] = [220, 40, 40, 255];

fn base(n: u32) -> (Gen, i32, i32) {
    let mut g = Gen::new(&format!("bb_smoke{n}"), 81100 + n as i32);
    let s = &mut g.p.settings;
    s.display_errors = true;
    s.write_errors_to_log = true;
    let o = g.object("smoke");
    let r = g.room("rm_smoke", 640, 480);
    (g, o, r)
}

fn save_code(n: u32) -> String {
    format!(
        r#"var f;
f = file_text_open_write('results_bb_smoke{n}.txt');
file_text_write_string(f, 'smoke{n}=ok');
file_text_writeln(f);
file_text_close(f);"#
    )
}

fn simple(n: u32, font: bool, sprite: bool) -> Spec {
    let (mut g, o, r) = base(n);
    let font_line = if font {
        let f = g.font("smoke_font", "Courier New", 8);
        format!("draw_set_font({f});")
    } else {
        String::new()
    };
    if sprite {
        let s = g.sprite("smoke_spr", 32, 32, &[RED], (0, 0));
        g.obj(o).sprite = s;
    }
    let draw_sprite_line = if sprite { "draw_sprite(sprite_index, 0, 300, 200);" } else { "" };
    g.event(o, ev::CREATE, &[&save_code(n)]);
    g.event(
        o,
        ev::DRAW,
        &[&format!(
            "{font_line}\ndraw_set_color(c_white);\ndraw_text(20, 20, 'SMOKE TEST {n} RUNNING - if you can read this, step {n} works');\n{draw_sprite_line}"
        )],
    );
    g.place(r, o, 0, 0, "");
    Spec {
        name: ["bb_smoke1", "bb_smoke2", "bb_smoke3", "bb_smoke4"][n as usize - 1],
        title: "Smoke test",
        lite: "Lite-safe",
        frames: 10,
        keys: vec![k("smoke", "-", "smoke test ladder (diagnostic)")],
        project: g.finish(),
    }
}

pub fn build1() -> Spec {
    simple(1, false, false)
}
pub fn build2() -> Spec {
    simple(2, true, false)
}
pub fn build3() -> Spec {
    simple(3, true, true)
}

pub fn build4() -> Spec {
    let mut g = Gen::new("bb_smoke4", 81104);
    g.p.settings.display_errors = true;
    g.p.settings.write_errors_to_log = true;
    let fw = install(&mut g);
    let tests = vec![vals("smoke", &[("smoke.four", "4")])];
    let frames = unit_project(&mut g, &fw, tests, 30);
    Spec {
        name: "bb_smoke4",
        title: "Smoke test 4 (full harness)",
        lite: "Lite-safe",
        frames,
        keys: vec![k("smoke", "-", "smoke test ladder (diagnostic)")],
        project: g.finish(),
    }
}
