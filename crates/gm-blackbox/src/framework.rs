//! Shared GML infrastructure of every test project: result logging, on-screen
//! listing, saving, and a one-test-per-step driver for unit-style tests.

use crate::builder::{ev, Gen};

/// Room size used by every project (the listing needs it).
pub const ROOM_W: i32 = 1024;
pub const ROOM_H: i32 = 768;

/// Indices of the framework resources inside a project.
#[allow(dead_code)]
pub struct Fw {
    pub out_obj: i32,
    pub save_obj: i32,
    pub font: i32,
}

/// A key prefix documented in the manifest.
#[derive(Clone)]
pub struct KeyDoc {
    pub prefix: &'static str,
    /// Open-question ids (OPEN_QUESTIONS.md / spec sections) the keys answer.
    pub ids: &'static str,
    pub desc: &'static str,
    /// Informational only (locale, timing, wording): mismatches are not failures.
    pub info: bool,
}

pub const fn k(prefix: &'static str, ids: &'static str, desc: &'static str) -> KeyDoc {
    KeyDoc { prefix, ids, desc, info: false }
}
pub const fn ki(prefix: &'static str, ids: &'static str, desc: &'static str) -> KeyDoc {
    KeyDoc { prefix, ids, desc, info: true }
}

/// A generated test project with its documentation.
pub struct Spec {
    pub name: &'static str,
    pub title: &'static str,
    pub lite: &'static str,
    /// Steps our headless runner needs to reach the end.
    pub frames: u64,
    pub keys: Vec<KeyDoc>,
    pub project: gm_format::Project,
}

const LOG: &str = r#"// bb_log(key, value): appends "key=value" to the result list.
// Strings are written in double quotes, reals through bb_fmt.
var v;
if (!variable_global_exists('bb_n')) global.bb_n = 0;
v = bb_fmt(argument1);
v = string_replace_all(v, chr(13), '\r');
v = string_replace_all(v, chr(10), '\n');
global.bb_lines[global.bb_n] = argument0 + '=' + v;
global.bb_n += 1;
"#;

const FMT: &str = r#"// bb_fmt(value): strings in double quotes; reals with up to 10 decimals,
// trailing zeros removed (independent of string()).
var v, s, l;
v = argument0;
if (is_string(v)) return '"' + v + '"';
s = string_format(v, 0, 10);
if (string_pos('.', s) > 0 && string_pos('E', s) == 0 && string_pos('e', s) == 0)
{
    l = string_length(s);
    while (string_char_at(s, l) == '0') { s = string_copy(s, 1, l - 1); l -= 1; }
    if (string_char_at(s, l) == '.') s = string_copy(s, 1, l - 1);
}
s = string_replace_all(s, ' ', '');
return s;
"#;

const EV: &str = r#"// bb_ev(tag): appends an event tag to the current step's sequence.
if (!variable_global_exists('bb_seq')) global.bb_seq = '';
global.bb_seq += argument0 + ' ';
"#;

const FLUSH: &str = r#"// bb_flush(key): logs the collected event sequence in chunks of at most 44
// chars, cut after a space where possible.
var s, n, key, c, p;
if (!variable_global_exists('bb_seq')) global.bb_seq = '';
s = global.bb_seq;
n = 0;
if (s == '') bb_log(argument0, '-');
while (s != '')
{
    key = argument0;
    if (n > 0) key = argument0 + '+' + string(n);
    c = string_copy(s, 1, 44);
    if (string_length(s) > 44)
    {
        p = 44;
        while (p > 1 && string_char_at(c, p) != ' ') p -= 1;
        if (p > 1) c = string_copy(c, 1, p);
    }
    bb_log(key, c);
    s = string_delete(s, 1, string_length(c));
    n += 1;
}
global.bb_seq = '';
"#;

const PAD2: &str = r#"// bb_pad(n): two-digit decimal string.
if (argument0 < 10) return '0' + string(argument0);
return string(argument0);
"#;

/// Installs scripts, font, listing object and saver.
pub fn install(g: &mut Gen) -> Fw {
    g.script("bb_log", LOG);
    g.script("bb_fmt", FMT);
    g.script("bb_ev", EV);
    g.script("bb_flush", FLUSH);
    g.script("bb_pad", PAD2);
    let font = g.font("bb_font", "Courier New", 8);
    let name = g.name.clone();
    let save_obj_placeholder = g.object("bb_save");
    let out_obj = g.object("bb_out");
    {
        let o = g.obj(out_obj);
        o.persistent = true;
        o.depth = -1000000;
    }
    let create = format!(
        r#"if (!variable_global_exists('bb_n')) global.bb_n = 0;
if (!variable_global_exists('bb_done')) global.bb_done = 0;
if (!variable_global_exists('bb_saved')) global.bb_saved = '-';
if (!variable_global_exists('bb_clip')) global.bb_clip = '-';
page = 0;
saved = 0;
if (!variable_global_exists('bb_started'))
{{
    global.bb_started = 1;
    global.bb_t0 = current_time;
    global.bb_nsteps = 0;
    bb_log('meta.project', '{name}');
}}"#
    );
    g.event(out_obj, ev::CREATE, &[&create]);
    g.event(
        out_obj,
        ev::END_STEP,
        &[r#"var maxp;
if (!global.bb_done) global.bb_nsteps += 1;
if (keyboard_check_pressed(vk_right) || keyboard_check_pressed(vk_pagedown) || keyboard_check_pressed(vk_down)) page += 1;
if (keyboard_check_pressed(vk_left) || keyboard_check_pressed(vk_pageup) || keyboard_check_pressed(vk_up)) page -= 1;
if (keyboard_check_pressed(vk_home)) page = 0;
maxp = ceil(global.bb_n / 180) - 1;
if (page > maxp) page = maxp;
if (page < 0) page = 0;
if (global.bb_done && !saved)
{
    saved = 1;
    bb_log('meta.elapsed_ms', current_time - global.bb_t0);
    bb_log('meta.steps', global.bb_nsteps);
    bb_log('meta.room_speed', room_speed);
    bb_log('meta.end', 1);
    instance_create(0, 0, bb_save);
}"#],
    );
    g.event(
        out_obj,
        ev::DRAW,
        &[r#"var i, r, s, x0, y0, st;
x0 = 0; y0 = 0;
if (view_enabled) { x0 = view_xview[view_current]; y0 = view_yview[view_current]; }
draw_set_font(bb_font);
draw_set_color(c_black);
draw_rectangle(x0, y0, x0 + 1024, y0 + 768, false);
if (global.bb_done) st = 'DONE  file:' + global.bb_saved + '  clipboard:' + global.bb_clip;
else st = 'RUNNING...';
draw_set_color(c_yellow);
draw_text(x0 + 4, y0 + 2, room_get_name(room) + ' ' + string(global.bb_n) + ' lines  page ' + string(page + 1) + '/' + string(max(1, ceil(global.bb_n / 180))) + '  [Left/Right]  ' + st);
draw_set_color(c_white);
for (i = 0; i < 180; i += 1)
{
    r = page * 180 + i;
    if (r >= global.bb_n) break;
    s = global.bb_lines[r];
    if (string_length(s) > 52) s = string_copy(s, 1, 51) + '~';
    s = string_replace_all(s, '#', '\#');
    draw_text(x0 + 4 + (i div 60) * 340, y0 + 18 + (i mod 60) * 12, s);
}"#],
    );
    // saver: separate actions so a failing (Pro-only?) file function does not stop
    // the clipboard copy.
    let save_obj = save_obj_placeholder;
    let file_code = format!(
        r#"var f, i;
f = file_text_open_write('results_{name}.txt');
for (i = 0; i < global.bb_n; i += 1)
{{
    file_text_write_string(f, global.bb_lines[i]);
    file_text_writeln(f);
}}
file_text_close(f);
global.bb_saved = 'results_{name}.txt';"#
    );
    g.event(
        save_obj,
        ev::CREATE,
        &[
            "global.bb_saved = 'FAILED';",
            &file_code,
            "global.bb_clip = 'FAILED';",
            r#"var s, i;
s = '';
for (i = 0; i < global.bb_n; i += 1) s += global.bb_lines[i] + chr(13) + chr(10);
clipboard_set_text(s);
global.bb_clip = 'copied';"#,
        ],
    );
    Fw {
        out_obj,
        save_obj,
        font,
    }
}

/// One unit-style test: a list of code actions run in the Create event of its
/// own object (one test per step, so an aborted event is detected next step).
pub struct Test {
    pub name: String,
    pub actions: Vec<String>,
}

/// Several `key = expression` values logged from one action.
pub fn vals(name: &str, items: &[(&str, &str)]) -> Test {
    let mut code = String::new();
    for (key, expr) in items {
        code.push_str(&format!("bb_log('{key}', {expr});\n"));
    }
    Test {
        name: name.into(),
        actions: vec![code],
    }
}

/// Free-form test actions.
pub fn raw(name: &str, actions: &[&str]) -> Test {
    Test {
        name: name.into(),
        actions: actions.iter().map(|s| s.to_string()).collect(),
    }
}

/// An expression that may raise an error: logs `key` (value or "<unset>"),
/// `key.err` (error_occurred) and `key.msg` (error_last, if an error occurred).
pub fn err(key: &str, expr: &str) -> Test {
    err_stmt(key, &format!("global.bb_r = {expr};"))
}

/// Like [`err`] but with a statement list that sets `global.bb_r`.
pub fn err_stmt(key: &str, stmt: &str) -> Test {
    Test {
        name: key.into(),
        actions: vec![
            "error_occurred = false; global.bb_r = '<unset>';".into(),
            stmt.into(),
            format!(
                "bb_log('{key}', global.bb_r); bb_log('{key}.err', error_occurred); if (error_occurred) bb_log('{key}.msg', error_last);"
            ),
        ],
    }
}

/// Adds the unit driver and one object per test. Returns the driver object.
pub fn unit_tests(g: &mut Gen, tests: Vec<Test>) -> i32 {
    let driver = g.object("bb_driver");
    let first = g.p.objects.len() as i32;
    let n = tests.len();
    for (i, t) in tests.into_iter().enumerate() {
        let o = g.object(&format!("t{:03}", i));
        let mut acts: Vec<String> = vec![format!("global.bb_pending = '{}';", t.name)];
        acts.extend(t.actions);
        acts.push("global.bb_pending = ''; instance_destroy();".into());
        let refs: Vec<&str> = acts.iter().map(|s| s.as_str()).collect();
        g.event(o, ev::CREATE, &refs);
    }
    g.event(driver, ev::CREATE, &["global.bb_pending = ''; global.bb_k = 0;"]);
    g.event(
        driver,
        ev::STEP,
        &[&format!(
            r#"if (global.bb_pending != '')
{{
    bb_log(global.bb_pending + '.aborted', 1);
    global.bb_pending = '';
}}
if (global.bb_k < {n})
{{
    global.bb_k += 1;
    instance_create(0, 0, {first} + global.bb_k - 1);
}}
else global.bb_done = 1;"#
        )],
    );
    driver
}

/// A standard single-room unit-test project.
pub fn unit_project(g: &mut Gen, fw: &Fw, tests: Vec<Test>, speed: i32) -> u64 {
    let n = tests.len() as u64;
    let driver = unit_tests(g, tests);
    let r = g.room("rm_tests", ROOM_W, ROOM_H);
    g.room_mut(r).speed = speed;
    g.place(r, fw.out_obj, 0, 0, "");
    g.place(r, driver, 0, 0, "");
    n + 30
}
