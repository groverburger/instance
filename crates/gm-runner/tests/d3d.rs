//! 3D mode (specs/d3d.md) on small projects built in code, rendered with the
//! software backend and — when an OpenGL context can be created — compared
//! with the OpenGL backend.

use gm_format as fmt;
use gm_runner::draw::backend::Backend;
use gm_runner::draw::Texture;
use gm_runner::Game;
use gml::Vm;

fn code(src: &str) -> fmt::Action {
    let mut a = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    a.args[0] = src.to_string();
    a
}

fn event(t: i32, n: i32, src: &str) -> fmt::Event {
    fmt::Event {
        event_type: t,
        number: n,
        actions: vec![code(src)],
    }
}

/// A checkerboard background (texture source).
fn checker(w: u32, h: u32) -> fmt::Background {
    let mut img = fmt::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let on = ((x / 8) + (y / 8)) % 2 == 0;
            let c = if on { [230, 230, 230, 255] } else { [60, 120, 200, 255] };
            let i = ((y * w + x) * 4) as usize;
            img.data[i..i + 4].copy_from_slice(&c);
        }
    }
    fmt::Background {
        name: "bgcheck".into(),
        image: img,
        ..Default::default()
    }
}

/// One object whose Create and Draw events are given; one room.
fn game(create: &str, draw: &str) -> (Game, Vm) {
    let mut p = fmt::Project::default();
    p.settings.display_errors = true;
    p.backgrounds.push(Some(checker(64, 64)));
    p.objects.push(Some(fmt::Object {
        name: "cam".into(),
        sprite: -1,
        visible: true,
        events: vec![event(0, 0, create), event(8, 0, draw)],
        ..Default::default()
    }));
    let mut r = fmt::Room {
        name: "room0".into(),
        ..Default::default()
    };
    r.instances.push(fmt::Instance {
        x: 0,
        y: 0,
        object: 0,
        id: 100001,
        creation_code: String::new(),
        locked: false,
    });
    p.last_instance_id = 101001;
    p.rooms.push(Some(r));
    p.room_order.push(0);
    let mut vm = Vm::new();
    let g = Game::new(p, &mut vm, true).expect("game");
    (g, vm)
}

fn render(create: &str, draw: &str, be: Option<Box<dyn Backend>>) -> Texture {
    let (mut g, mut vm) = game(create, draw);
    if let Some(b) = be {
        g.gfx.set_backend(b);
    }
    g.start(&mut vm).expect("start");
    g.step(&mut vm).expect("step");
    g.step(&mut vm).expect("step");
    g.frame_image()
}

const SCENE_CREATE: &str = "d3d_start(); d3d_set_culling(true); texture_set_repeat(true);
d3d_set_lighting(true); d3d_light_define_ambient(make_color_rgb(70,70,70));
d3d_light_define_direction(1, 0.5, 1, -1, c_white); d3d_light_enable(1, true);
d3d_set_fog(true, c_gray, 200, 900);";

const SCENE_DRAW: &str = "draw_set_color(c_white);
d3d_set_projection_ext(40, 240, 60, 400, 240, 20, 0, 0, 1, 60, 640/480, 1, 2000);
var t; t = background_get_texture(bgcheck);
d3d_draw_floor(0, 0, 0, 1200, 480, 0, t, 12, 6);
d3d_draw_block(200, 150, 0, 260, 210, 60, t, 1, 1);
draw_set_color(c_red);
d3d_draw_cylinder(300, 260, 0, 350, 310, 80, -1, 1, 1, true, 24);
draw_set_color(c_yellow);
d3d_draw_ellipsoid(420, 160, 20, 480, 220, 80, -1, 1, 1, 24);
draw_set_color(c_lime);
d3d_draw_cone(520, 280, 0, 580, 340, 90, -1, 1, 1, true, 16);
draw_set_color(c_white);
d3d_draw_wall(700, 0, 0, 700, 480, 120, t, 4, 1);
d3d_transform_set_rotation_z(30); d3d_transform_add_translation(330, 120, 30);
d3d_draw_block(-20, -20, -20, 20, 20, 20, -1, 1, 1);
d3d_transform_set_identity();
d3d_set_projection_ortho(0, 0, 640, 480, 0);
d3d_set_hidden(false); d3d_set_lighting(false); d3d_set_fog(false, 0, 0, 0);
draw_set_color(c_black); draw_rectangle(8, 8, 200, 30, false);
draw_set_color(c_white); draw_text(12, 10, 'HUD overlay');
d3d_set_hidden(true); d3d_set_lighting(true); d3d_set_fog(true, c_gray, 200, 900);";

fn out_dir() -> std::path::PathBuf {
    let d = std::env::var_os("GM_D3D_OUT").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let _ = std::fs::create_dir_all(&d);
    d
}

fn count_colour(t: &Texture, f: impl Fn([u8; 4]) -> bool) -> usize {
    t.px.iter().filter(|p| f(**p)).count()
}

#[test]
fn scene_renders_in_software() {
    let img = render(SCENE_CREATE, SCENE_DRAW, None);
    let p = out_dir().join("gm_d3d_scene_soft.png");
    img.save_png(&p, true).unwrap();
    eprintln!("wrote {}", p.display());
    // red cylinder (lit) is visible
    assert!(count_colour(&img, |p| p[0] > 100 && p[1] < 60 && p[2] < 60) > 200);
    // the floor covers the lower half
    assert!(count_colour(&img, |p| p[2] > 100) > 20000);
}

#[test]
fn depth_buffer_hides_far_block() {
    // a near red wall in front of a far green wall: only red visible at centre
    let draw = "d3d_set_projection(0, 0, 0, 100, 0, 0, 0, 0, 1);
draw_set_color(c_lime); d3d_draw_wall(200, -100, -100, 200, 100, 100, -1, 1, 1);
draw_set_color(c_red); d3d_draw_wall(100, -20, -20, 100, 20, 20, -1, 1, 1);
draw_set_color(c_blue); d3d_draw_wall(300, -500, -500, 300, 500, 500, -1, 1, 1);";
    let img = render("d3d_start();", draw, None);
    let c = img.get(320, 240);
    assert_eq!(&c[..3], &[255, 0, 0], "centre should be the near red wall");
    let side = img.get(320 - 200, 240);
    assert_eq!(&side[..3], &[0, 255, 0], "the green wall shows beside the red one");
}

#[test]
fn culling_drops_back_faces() {
    // a block seen from outside with culling: the near face is drawn; drawing the
    // block from inside (camera inside) with culling shows nothing
    let draw_in = "d3d_set_culling(true); d3d_set_projection(0, 0, 0, 100, 0, 0, 0, 0, 1);
draw_set_color(c_red); d3d_draw_block(-50, -50, -50, 50, 50, 50, -1, 1, 1);";
    let img = render("d3d_start();", draw_in, None);
    assert!(count_colour(&img, |p| p[0] == 255 && p[1] == 0) == 0, "inside faces are culled");
    let draw_out = "d3d_set_culling(true); d3d_set_projection(-200, 0, 0, 0, 0, 0, 0, 0, 1);
draw_set_color(c_red); d3d_draw_block(-50, -50, -50, 50, 50, 50, -1, 1, 1);";
    let img = render("d3d_start();", draw_out, None);
    assert!(count_colour(&img, |p| p[0] == 255 && p[1] == 0) > 1000, "outside faces are drawn");
}

#[test]
fn transform_stack_and_models() {
    let create = "d3d_start(); global.m = d3d_model_create();
d3d_model_block(global.m, -10, -10, -10, 10, 10, 10, 1, 1);
d3d_model_primitive_begin(global.m, pr_trianglelist);
d3d_model_vertex_color(global.m, 0, 0, 0, c_red, 1);
d3d_model_vertex_color(global.m, 10, 0, 0, c_red, 1);
d3d_model_vertex_color(global.m, 0, 10, 0, c_red, 1);
d3d_model_primitive_end(global.m);
global.ok = d3d_model_save(global.m, temp_directory + '\\gm_model_test.d3d');
global.m2 = d3d_model_create();
global.ok2 = d3d_model_load(global.m2, temp_directory + '\\gm_model_test.d3d');
d3d_transform_stack_push(); d3d_transform_set_translation(1, 2, 3);
global.e = d3d_transform_stack_empty(); d3d_transform_stack_pop();";
    let draw = "d3d_set_projection(-100, 0, 0, 0, 0, 0, 0, 0, 1); draw_set_color(c_white);
d3d_model_draw(global.m2, 0, 0, 0, -1);";
    let (mut g, mut vm) = game(create, draw);
    g.start(&mut vm).expect("start");
    g.step(&mut vm).expect("step");
    let img = g.frame_image();
    assert!(count_colour(&img, |p| p == [255, 255, 255, 255]) > 500, "loaded model is drawn");
}

/// Renders the scene with both backends and compares (skipped without OpenGL).
#[test]
fn scene_matches_opengl() {
    let gl = match gm_runner::draw::gl::GlBackend::hidden() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("skipping: no OpenGL ({})", e);
            return;
        }
    };
    let soft = render(SCENE_CREATE, SCENE_DRAW, None);
    let glimg = render(SCENE_CREATE, SCENE_DRAW, Some(Box::new(gl)));
    let p = out_dir().join("gm_d3d_scene_gl.png");
    glimg.save_png(&p, true).unwrap();
    eprintln!("wrote {}", p.display());
    let (mx, n) = gm_runner::draw::dual::compare(&soft, &glimg, 24);
    let frac = n as f64 / soft.px.len() as f64;
    eprintln!("3D soft vs GL: max diff {}, {} px ({:.3}%)", mx, n, frac * 100.0);
    gm_runner::draw::dual::diff_image(&soft, &glimg, 24).save_png(&out_dir().join("gm_d3d_scene_diff.png"), true).unwrap();
    assert!(frac < 0.01, "3D renderers differ in {:.2}% of pixels", frac * 100.0);
}
