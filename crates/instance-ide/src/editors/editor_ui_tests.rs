//! Headless UI tests of the sprite editor and image editor windows (egui_kittest).

use eframe::egui;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use instance_format::{ResourceKind, RgbaImage};

use super::image_editor::raster::{self, ColorMode, Mask, Px};
use super::ResKey;
use crate::project_ops as ops;

/// A small generated test sprite frame (a face; no external assets).
pub fn test_face(size: u32, hue_shift: u8) -> RgbaImage {
    let base = RgbaImage::new(size, size);
    let s = size as i32 - 1;
    let mut m = Mask::for_image(&base);
    m.ellipse((1, 1), (s - 1, s - 1), 1, true);
    let mut img = raster::composite(
        &base,
        &m,
        [255, 200u8.saturating_sub(hue_shift), 40, 255],
        ColorMode::Blend,
    );
    let mut o = Mask::for_image(&base);
    o.ellipse((1, 1), (s - 1, s - 1), 2, false);
    raster::composite_into(&mut img, &o, [90, 50, 10, 255], ColorMode::Blend);
    let mut f = Mask::for_image(&base);
    let e = size as i32 / 4;
    f.ellipse((e + 1, e + 1), (e + 3, e + 4), 1, true);
    f.ellipse((s - e - 3, e + 1), (s - e - 1, e + 4), 1, true);
    f.line((e + 1, s - e - 2), (s - e - 1, s - e - 2), 2);
    raster::composite_into(&mut img, &f, [40, 20, 0, 255], ColorMode::Blend);
    img
}

fn project_with_sprite(frames: Vec<RgbaImage>) -> instance_format::Project {
    let mut p = ops::new_project();
    ops::create_resource(&mut p, ResourceKind::Sprite, &ops::Placement::End);
    let s = p.sprites[0].as_mut().unwrap();
    s.frames = frames;
    p
}

fn click_at(h: &mut Harness<'_, crate::app::IdeApp>, pos: egui::Pos2, button: egui::PointerButton) {
    h.event(egui::Event::PointerMoved(pos));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Default::default(),
        });
        h.run_steps(1);
    }
}

/// Sprite form → Edit Sprite → Edit Frame → draw → close saving → OK commits the
/// change to the project; the image editor window keeps a stable size.
#[test]
fn ui_sprite_and_image_editor() {
    let mut h = Harness::builder()
        .with_size(egui::Vec2::new(1600.0, 1000.0))
        .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
    let frame0 = RgbaImage::new(16, 16);
    h.state_mut()
        .test_set_project(project_with_sprite(vec![frame0.clone(), frame0.clone()]));
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Sprite, 0));
    h.run_steps(3);
    h.get_by_label("Edit Sprite…").click();
    h.run_steps(3);
    h.get_by_label("✏ Edit Frame").click();
    h.run_steps(4);

    let r0 = h.get_by_label("image canvas").rect();
    h.run_steps(10);
    let r1 = h.get_by_label("image canvas").rect();
    assert_eq!(r0, r1, "image editor must not grow");
    assert!(r0.width() > 200.0 && r0.height() > 200.0, "canvas {r0:?}");

    // the image is centred and fitted: click the centre pixel with the pencil
    click_at(
        &mut h,
        r0.center() + egui::vec2(1.0, 1.0),
        egui::PointerButton::Primary,
    );
    h.run_steps(2);
    h.get_all_by_label("✔ Close Saving").last().unwrap().click();
    h.run_steps(3);
    // back in the sprite editor: add a frame, then close saving
    h.get_by_label("Add").click();
    h.run_steps(2);
    h.get_by_label("✔ Close Saving").click();
    h.run_steps(3);
    h.get_by_label("✔ OK").click();
    h.run_steps(3);
    let (p, modified, forms, msgs) = h.state().test_state();
    assert_eq!((forms, msgs), (0, 0));
    assert!(modified);
    let s = p.sprites[0].as_ref().unwrap();
    assert_eq!(s.frames.len(), 3);
    assert_eq!(
        s.frames[0].pixel(8, 8),
        [0, 0, 0, 255],
        "pencil drew in black"
    );
    assert_eq!(s.frames[1], frame0);
}

/// Background form → Edit Background opens the image editor and saving commits.
#[test]
fn ui_background_image_editor() {
    let mut h = Harness::builder()
        .with_size(egui::Vec2::new(1600.0, 1000.0))
        .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
    let mut p = ops::new_project();
    ops::create_resource(&mut p, ResourceKind::Background, &ops::Placement::End);
    p.backgrounds[0].as_mut().unwrap().image = RgbaImage::new(40, 30);
    h.state_mut().test_set_project(p);
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Background, 0));
    h.run_steps(3);
    h.get_by_label("Edit Background…").click();
    h.run_steps(4);
    h.get_by_label("Fill").click();
    h.run_steps(2);
    let r = h.get_by_label("image canvas").rect();
    click_at(&mut h, r.center(), egui::PointerButton::Primary);
    h.get_by_label("✔ Close Saving").click();
    h.run_steps(3);
    h.get_by_label("✔ OK").click();
    h.run_steps(3);
    let (p, _, forms, _) = h.state().test_state();
    assert_eq!(forms, 0);
    let img = &p.backgrounds[0].as_ref().unwrap().image;
    assert!(
        img.data.chunks_exact(4).all(|px| px == [0, 0, 0, 255]),
        "filled black"
    );
    let _ = img.get(0, 0);
}

/// Create from Strip: the dialog cuts a strip into frames; Animation > Reverse works
/// through the menu.
#[test]
fn ui_strip_dialog_and_animation_menu() {
    use super::sprite_frames::SpriteFramesEditor;
    use crate::widgets::TextureCache;
    let mut src = RgbaImage::new(48, 16);
    for k in 0..3 {
        src.put(k * 16, 0, [k as u8 * 100, 0, 0, 255]);
    }
    let mut ed = SpriteFramesEditor::new(vec![RgbaImage::new(16, 16)], "s", 7);
    ed.open_strip_image(src, false);
    let mut h = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_ui_state(
            |ui, (ed, tex): &mut (SpriteFramesEditor, TextureCache)| {
                ed.show(ui.ctx(), tex, egui::Id::new("sfe"));
            },
            (ed, TextureCache::default()),
        );
    h.run_steps(3);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let frames = &h.state().0.frames;
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[2].pixel(0, 0), [200, 0, 0, 255]);
    h.get_by_label("Animation").click();
    h.run_steps(2);
    h.get_by_label("Reverse").click();
    h.run_steps(3);
    assert_eq!(h.state().0.frames[0].pixel(0, 0), [200, 0, 0, 255]);
    assert!(h.state().0.modified);
}

/// Render the IDE with the sprite editor and the image editor open and save a PNG
/// to `$GM_IDE_SCREENSHOT` (run with `--ignored`; needs a wgpu adapter).
#[test]
#[ignore]
fn screenshot_image_editor() {
    let Some(out) = std::env::var_os("GM_IDE_SCREENSHOT") else {
        eprintln!("set GM_IDE_SCREENSHOT=<dir> to write screenshots");
        return;
    };
    let out = std::path::PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let frames: Vec<RgbaImage> = (0..4).map(|k| test_face(32, k * 40)).collect();
    let mut h = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .with_theme(egui::Theme::Light)
        .wgpu()
        .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
    h.state_mut().test_set_project(project_with_sprite(frames));
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Sprite, 0));
    h.run_steps(3);
    h.get_by_label("Edit Sprite…").click();
    h.run_steps(4);
    let img = h.render().expect("render");
    img.save(out.join("sprite_editor.png")).unwrap();
    // open frame 1 in the image editor (onion skin of frame 0) and select a tool
    h.get_all_by_label("image 1").last().unwrap().click();
    h.run_steps(2);
    h.get_by_label("✏ Edit Frame").click();
    h.run_steps(4);
    h.get_by_label("Ellipse").click();
    h.run_steps(3);
    let r = h.get_by_label("image canvas").rect();
    h.event(egui::Event::PointerMoved(r.center()));
    h.run_steps(3);
    let img = h.render().expect("render");
    img.save(out.join("image_editor.png")).unwrap();
}
