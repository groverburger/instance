//! 2D drawing: the OpenGL backend must match the software backend (skipped when
//! no OpenGL context can be created).

use instance_format as fmt;
use instance_runner::draw::backend::Backend;
use instance_runner::draw::Texture;
use instance_runner::Game;
use gml::Vm;

fn code(src: &str) -> fmt::Action {
    let mut a = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    a.args[0] = src.to_string();
    a
}

fn sprite() -> fmt::Sprite {
    let (w, h) = (24u32, 16u32);
    let mut img = fmt::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let a = if (x + y) % 5 == 0 { 0 } else { 255 - (x * 6) as u8 };
            img.data[i..i + 4].copy_from_slice(&[(x * 10) as u8, (y * 15) as u8, 200, a]);
        }
    }
    fmt::Sprite {
        name: "spr".into(),
        frames: vec![img],
        origin_x: 12,
        origin_y: 8,
        ..Default::default()
    }
}

const DRAW: &str = "draw_clear(make_color_rgb(30, 40, 50));
if !surface_exists(global.s) global.s = surface_create(100, 70);
surface_set_target(global.s); draw_clear_alpha(c_black, 0);
draw_set_color(c_orange); draw_circle(50, 35, 30, false);
draw_sprite_ext(spr, 0, 50, 35, 2, 2, 30, c_white, 0.7);
surface_reset_target();
draw_surface_ext(global.s, 300, 40, 1.5, 1.5, 0, c_white, 1);
draw_surface_ext(global.s, 120, 300, 1, 1, 45, c_yellow, 0.8);
draw_sprite(spr, 0, 40, 40);
draw_sprite_ext(spr, 0, 100.5, 40.25, 1.5, -1, 0, c_red, 0.5);
draw_sprite_ext(spr, 0, 160, 50, 1, 1, 33, c_white, 1);
draw_set_blend_mode(bm_add); draw_sprite_stretched(spr, 0, 20, 100, 200, 60);
draw_set_blend_mode(bm_subtract); draw_set_color(c_gray); draw_rectangle(30, 120, 90, 180, false);
draw_set_blend_mode_ext(bm_dest_color, bm_zero); draw_set_color(c_aqua); draw_ellipse(100, 110, 200, 170, false);
draw_set_blend_mode(bm_normal);
draw_set_color(c_white); draw_line_width(10, 220, 250, 260, 3); draw_line(10, 270, 250, 230);
draw_rectangle_color(260, 200, 380, 280, c_red, c_lime, c_blue, c_white, false);
draw_roundrect(400, 200, 500, 260, true); draw_triangle(520, 200, 600, 220, 540, 280, false);
draw_set_alpha(0.5); draw_circle_color(450, 330, 40, c_fuchsia, c_navy, false); draw_set_alpha(1);
draw_set_font(-1); draw_set_halign(fa_center); draw_text(320, 420, 'Renderer#comparison');
draw_text_transformed_color(500, 400, 'rot', 2, 2, 20, c_red, c_red, c_blue, c_blue, 1);
draw_set_halign(fa_left);
texture_set_repeat(true);
draw_primitive_begin_texture(pr_trianglestrip, sprite_get_texture(spr, 0));
draw_vertex_texture(400, 20, 0, 0); draw_vertex_texture(600, 20, 3, 0);
draw_vertex_texture(400, 120, 0, 2); draw_vertex_texture(600, 120, 3, 2);
draw_primitive_end();
texture_set_interpolation(true); draw_sprite_ext(spr, 0, 560, 330, 3.3, 2.7, 10, c_white, 1); texture_set_interpolation(false);
draw_healthbar(20, 440, 200, 460, 60, c_black, c_red, c_lime, 0, true, true);";

fn render(be: Option<Box<dyn Backend>>) -> Texture {
    let mut p = fmt::Project::default();
    p.settings.display_errors = true;
    p.sprites.push(Some(sprite()));
    p.objects.push(Some(fmt::Object {
        name: "o".into(),
        sprite: -1,
        visible: true,
        events: vec![
            fmt::Event { event_type: 0, number: 0, actions: vec![code("global.s = -1;")] },
            fmt::Event { event_type: 8, number: 0, actions: vec![code(DRAW)] },
        ],
        ..Default::default()
    }));
    let mut r = fmt::Room { name: "room0".into(), ..Default::default() };
    r.instances.push(fmt::Instance { x: 0, y: 0, object: 0, id: 100001, creation_code: String::new(), locked: false });
    p.last_instance_id = 101001;
    p.rooms.push(Some(r));
    p.room_order.push(0);
    let mut vm = Vm::new();
    let mut g = Game::new(p, &mut vm, true).expect("game");
    if let Some(b) = be {
        g.gfx.set_backend(b);
    }
    g.start(&mut vm).expect("start");
    g.step(&mut vm).expect("step");
    g.step(&mut vm).expect("step");
    g.frame_image()
}

#[test]
fn opengl_matches_software_2d() {
    let gl = match instance_runner::draw::gl::GlBackend::hidden() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("skipping: no OpenGL ({})", e);
            return;
        }
    };
    let soft = render(None);
    let glimg = render(Some(Box::new(gl)));
    let dir = std::env::var_os("GM_D3D_OUT").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let _ = std::fs::create_dir_all(&dir);
    soft.save_png(&dir.join("gm_2d_soft.png"), true).unwrap();
    glimg.save_png(&dir.join("gm_2d_gl.png"), true).unwrap();
    instance_runner::draw::dual::diff_image(&soft, &glimg, 8).save_png(&dir.join("gm_2d_diff.png"), true).unwrap();
    let (mx, n) = instance_runner::draw::dual::compare(&soft, &glimg, 8);
    eprintln!("2D soft vs GL: max diff {}, {} px", mx, n);
    assert!(n < 200, "{} pixels differ by more than 8", n);
}
