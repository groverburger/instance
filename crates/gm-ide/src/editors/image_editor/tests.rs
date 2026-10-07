//! Tests of the pure image operations and the editor model (no UI).

use gm_format::RgbaImage;

use super::effects as fx;
use super::history::History;
use super::ops::{apply_to_frames, AnimOp, ImageOp};
use super::raster::{self, ColorMode, Mask, Px};
use super::{ImageEditor, ShapeFill, Tool};

const R: [u8; 4] = [255, 0, 0, 255];
const G: [u8; 4] = [0, 255, 0, 255];
const B: [u8; 4] = [0, 0, 255, 255];
const T: [u8; 4] = [0, 0, 0, 0];

fn filled(w: u32, h: u32, c: [u8; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for px in img.data.chunks_exact_mut(4) {
        px.copy_from_slice(&c);
    }
    img
}

/// A 3×2 image with distinct pixels: row 0 = R G B, row 1 = T R G.
fn sample() -> RgbaImage {
    let mut img = RgbaImage::new(3, 2);
    for (i, c) in [R, G, B, T, R, G].into_iter().enumerate() {
        img.put(i as i32 % 3, i as i32 / 3, c);
    }
    img
}

fn count(img: &RgbaImage, c: [u8; 4]) -> usize {
    img.data.chunks_exact(4).filter(|p| *p == c).count()
}

// ------------------------------------------------------------------ raster

#[test]
fn bresenham_line() {
    let pts = raster::line_points((0, 0), (4, 2));
    assert_eq!(pts.first(), Some(&(0, 0)));
    assert_eq!(pts.last(), Some(&(4, 2)));
    assert_eq!(pts.len(), 5);
    // every step moves by at most one pixel
    for w in pts.windows(2) {
        assert!((w[1].0 - w[0].0).abs() <= 1 && (w[1].1 - w[0].1).abs() <= 1);
    }
    assert_eq!(raster::line_points((3, 3), (3, 3)), vec![(3, 3)]);
}

#[test]
fn line_mask_and_composite() {
    let base = RgbaImage::new(8, 8);
    let mut m = Mask::for_image(&base);
    m.line((0, 0), (7, 7), 1);
    let img = raster::composite(&base, &m, R, ColorMode::Blend);
    assert_eq!(count(&img, R), 8);
    for i in 0..8 {
        assert_eq!(img.get(i, i), Some(R));
    }
    // a 3-pixel brush is thicker
    let mut m3 = Mask::for_image(&base);
    m3.line((0, 4), (7, 4), 3);
    let img = raster::composite(&base, &m3, R, ColorMode::Replace);
    assert_eq!(count(&img, R), 24);
}

#[test]
fn constrain_helpers() {
    assert_eq!(raster::constrain_45((0, 0), (10, 1)), (10, 0));
    assert_eq!(raster::constrain_45((0, 0), (1, 10)), (0, 10));
    assert_eq!(raster::constrain_45((0, 0), (5, -4)), (5, -5));
    assert_eq!(raster::constrain_square((0, 0), (3, -7)), (7, -7));
}

#[test]
fn rect_and_ellipse_masks() {
    let mut m = Mask::new(10, 10);
    m.rect((1, 1), (5, 4), 1, false);
    let n = m.data.iter().filter(|&&v| v > 0).count();
    assert_eq!(n, 2 * 5 + 2 * 2); // outline of a 5×4 box
    m.clear();
    m.rect((5, 4), (1, 1), 1, true);
    assert_eq!(m.data.iter().filter(|&&v| v > 0).count(), 20);
    m.clear();
    m.ellipse((0, 0), (8, 8), 1, true);
    assert_eq!(m.get(4, 4), 255);
    assert_eq!(m.get(0, 0), 0, "corner outside the circle");
    assert_eq!(m.get(4, 0), 255, "top centre on the circle");
    let mut o = Mask::new(10, 10);
    o.ellipse((0, 0), (8, 8), 1, false);
    assert_eq!(o.get(4, 4), 0, "outline is hollow");
    assert_eq!(o.get(0, 4), 255);
}

#[test]
fn blending() {
    assert_eq!(raster::blend_over(R, [0, 0, 255, 0]), R);
    assert_eq!(raster::blend_over(R, B), B);
    let half = raster::blend_over(R, [0, 0, 255, 128]);
    assert_eq!(half[3], 255);
    assert!(half[0] > 120 && half[0] < 135 && half[2] > 120 && half[2] < 135);
    // over transparent keeps the source colour
    let s = raster::blend_over(T, [10, 20, 30, 100]);
    assert_eq!(s, [10, 20, 30, 100]);
    // replace mode copies alpha
    assert_eq!(
        raster::apply_color(R, [1, 2, 3, 4], 255, ColorMode::Replace),
        [1, 2, 3, 4]
    );
}

#[test]
fn flood_fill_respects_regions_and_tolerance() {
    // left half red, right half slightly different red, wall of blue in column 4
    let mut img = filled(8, 4, R);
    for y in 0..4 {
        img.put(4, y, B);
        for x in 5..8 {
            img.put(x, y, [250, 0, 0, 255]);
        }
    }
    let mut a = img.clone();
    assert!(raster::flood_fill(
        &mut a,
        0,
        0,
        G,
        0,
        true,
        ColorMode::Blend
    ));
    assert_eq!(count(&a, G), 16, "stops at the blue wall");
    assert_eq!(count(&a, B), 4);
    // fill the right side with tolerance: it is connected only to itself
    let mut b = img.clone();
    raster::flood_fill(&mut b, 6, 1, G, 10, true, ColorMode::Blend);
    assert_eq!(count(&b, G), 12);
    // out of bounds is a no-op
    assert!(!raster::flood_fill(
        &mut b,
        -1,
        0,
        G,
        0,
        true,
        ColorMode::Blend
    ));
    // a large tolerance crosses the wall
    let mut c = img.clone();
    raster::flood_fill(&mut c, 0, 0, G, 255, true, ColorMode::Blend);
    assert_eq!(count(&c, G), 32);
}

#[test]
fn replace_color_and_erase() {
    let mut img = sample();
    raster::replace_color(&mut img, R, B, 0, true);
    assert_eq!(count(&img, R), 0);
    assert_eq!(count(&img, B), 3);
    let mut m = Mask::for_image(&img);
    m.set_max(0, 0, 255);
    raster::erase_into(&mut img, &m, 255);
    assert_eq!(img.get(0, 0), Some(T));
}

#[test]
fn copy_clear_paste_rect() {
    let img = sample();
    let c = raster::copy_rect(&img, (1, 0, 2, 2));
    assert_eq!((c.width, c.height), (2, 2));
    assert_eq!(c.get(0, 0), Some(G));
    assert_eq!(c.get(1, 1), Some(G));
    let mut d = img.clone();
    raster::clear_rect(&mut d, (1, 0, 2, 2));
    assert_eq!(count(&d, T), 5);
    raster::paste(&mut d, &c, 1, 0, ColorMode::Replace);
    assert_eq!(d, img);
}

#[test]
fn spray_stays_in_radius() {
    let mut m = Mask::new(40, 40);
    let mut seed = 1;
    m.spray(20, 20, 10, 200, &mut seed);
    for y in 0..40 {
        for x in 0..40 {
            if m.get(x, y) > 0 {
                let d = (((x - 20) * (x - 20) + (y - 20) * (y - 20)) as f32).sqrt();
                assert!(d <= 5.8, "dot at distance {d}");
            }
        }
    }
    assert!(m.data.iter().filter(|&&v| v > 0).count() > 20);
}

// ------------------------------------------------------------------ transforms

#[test]
fn mirror_flip_rotate() {
    let img = sample();
    assert_eq!(fx::mirror(&fx::mirror(&img)), img);
    assert_eq!(fx::flip(&fx::flip(&img)), img);
    assert_eq!(fx::mirror(&img).get(0, 0), Some(B));
    assert_eq!(fx::flip(&img).get(0, 0), Some(T));
    let cw = fx::rotate90_cw(&img);
    assert_eq!((cw.width, cw.height), (2, 3));
    // top-left of the rotated image is the old bottom-left
    assert_eq!(cw.get(0, 0), Some(T));
    assert_eq!(cw.get(1, 0), Some(R));
    assert_eq!(fx::rotate90_ccw(&cw), img);
    let r4 = fx::rotate90_cw(&fx::rotate90_cw(&fx::rotate90_cw(&cw)));
    assert_eq!(r4, img);
    assert_eq!(fx::rotate180(&img), fx::rotate90_cw(&cw));
    assert_eq!(fx::rotate(&img, 360.0, true), img);
    assert_eq!(fx::rotate(&img, 180.0, false), fx::rotate180(&img));
}

#[test]
fn arbitrary_rotation_keeps_canvas_and_centre() {
    let mut img = RgbaImage::new(9, 9);
    img.put(4, 4, R);
    img.put(4, 0, G);
    let r = fx::rotate(&img, 90.0, false);
    assert_eq!(r.get(4, 4), Some(R));
    // square 90° is exact: CCW moves the top pixel to the left edge
    assert_eq!(r.get(0, 4), Some(G));
    let r = fx::rotate(&img, 45.0, false);
    assert_eq!((r.width, r.height), (9, 9));
    assert_eq!(r.get(4, 4), Some(R));
}

#[test]
fn shift_scale_stretch_resize_crop() {
    let img = sample();
    let s = fx::shift(&img, 1, 1);
    assert_eq!(s.get(1, 1), Some(R));
    assert_eq!(s.get(0, 0), Some(T));
    assert_eq!(count(&s, T), 4);

    let big = fx::stretch(&img, 6, 4, false);
    assert_eq!((big.width, big.height), (6, 4));
    assert_eq!(big.get(0, 0), Some(R));
    assert_eq!(big.get(1, 1), Some(R));
    assert_eq!(big.get(5, 0), Some(B));
    assert_eq!(fx::stretch(&big, 3, 2, false), img);

    let c = fx::resize_canvas(&img, 5, 4, (1, 1));
    assert_eq!((c.width, c.height), (5, 4));
    assert_eq!(c.get(1, 1), Some(R));
    let c = fx::resize_canvas(&img, 2, 1, (0, 0));
    assert_eq!(c.get(1, 0), Some(G));

    let mut e = RgbaImage::new(10, 10);
    e.put(3, 4, R);
    e.put(5, 6, R);
    let k = fx::crop(&e, 0);
    assert_eq!((k.width, k.height), (3, 3));
    let k = fx::crop(&e, 1);
    assert_eq!((k.width, k.height), (5, 5));
    assert_eq!(fx::crop(&RgbaImage::new(4, 4), 0), RgbaImage::new(4, 4));

    // scaling by 2 about the centre: the centre pixel spreads
    let mut d = RgbaImage::new(4, 4);
    d.put(1, 1, R);
    let sc = fx::scale(&d, 2.0, 2.0, false);
    assert_eq!((sc.width, sc.height), (4, 4));
    assert_eq!(count(&sc, R), 4);
    assert_eq!(fx::scale(&d, 1.0, 1.0, false), d);
}

#[test]
fn skew_zero_is_identity() {
    let img = sample();
    assert_eq!(fx::skew(&img, 0.0, 0.0, false), img);
}

// ------------------------------------------------------------------ effects

#[test]
fn colour_effects() {
    let img = sample();
    assert_eq!(fx::invert(&fx::invert(&img)), img);
    assert_eq!(fx::invert(&img).get(0, 0), Some([0, 255, 255, 255]));
    let bw = fx::black_and_white(&img);
    for p in bw.data.chunks_exact(4) {
        assert!(p[0] == p[1] && p[1] == p[2]);
    }
    assert_eq!(fx::make_opaque(&img).get(0, 1).unwrap()[3], 255);
    let er = fx::erase_color(&img, R, 0);
    assert_eq!(count(&er, R), 0);
    assert_eq!(count(&er, T), 3);
    let op = fx::opacity(&img, 50.0, true);
    assert_eq!(op.get(0, 0).unwrap()[3], 128);
    assert_eq!(op.get(0, 1), Some(T), "transparent stays transparent");
    let op = fx::opacity(&img, 10.0, false);
    assert_eq!(op.get(1, 0).unwrap()[3], 10);
    let fa = fx::fade(&img, [255, 255, 255, 255], 255);
    assert_eq!(fa.get(0, 0), Some([255, 255, 255, 255]));
    assert_eq!(fx::fade(&img, B, 0), img);
    // hue: red → green via colorize
    let c = fx::colorize(&filled(2, 2, R), 120.0, 1.0);
    assert_eq!(c.get(0, 0), Some(G));
    let h = fx::shift_hue(&filled(1, 1, R), 240.0);
    assert_eq!(h.get(0, 0), Some(B));
    let p = fx::colorize_partial(&img, 0.0, 10.0, 240.0);
    assert_eq!(count(&p, R), 0);
    assert_eq!(count(&p, G), 2, "green untouched");
    let i = fx::intensity(&filled(1, 1, R), -100.0, 0.0);
    let px = i.get(0, 0).unwrap();
    assert!(px[0] == px[1] && px[1] == px[2]);
}

#[test]
fn hsv_round_trip() {
    for c in [
        R,
        G,
        B,
        [12, 200, 77, 255],
        [128, 128, 128, 255],
        [0, 0, 0, 255],
    ] {
        let (h, s, v) = fx::rgb_to_hsv(c);
        assert_eq!(fx::hsv_to_rgb(h, s, v, 255), c);
    }
}

#[test]
fn blur_and_sharpen() {
    let flat = filled(5, 5, [100, 50, 25, 255]);
    assert_eq!(fx::blur(&flat, 2, true, true), flat);
    assert_eq!(fx::sharpen(&flat, 1.5, true, true), flat);
    let mut dot = filled(5, 5, [0, 0, 0, 255]);
    dot.put(2, 2, [255, 255, 255, 255]);
    let b = fx::blur(&dot, 1, true, false);
    let centre = b.get(2, 2).unwrap()[0];
    assert!(centre < 255 && centre > 0);
    assert!(b.get(1, 1).unwrap()[0] > 0, "spreads to neighbours");
    assert_eq!(b.get(4, 4).unwrap()[0], 0);
    // alpha-only blur keeps colours
    let mut a = RgbaImage::new(3, 1);
    a.put(1, 0, R);
    let ab = fx::blur(&a, 1, false, true);
    assert!(ab.get(0, 0).unwrap()[3] > 0);
    assert_eq!(ab.get(1, 0).unwrap()[0], 255);
}

#[test]
fn outline_effect() {
    let mut img = RgbaImage::new(5, 5);
    img.put(2, 2, R);
    let o = fx::outline(&img, B, 1);
    assert_eq!(o.get(2, 2), Some(R));
    assert_eq!(count(&o, B), 4);
}

// ------------------------------------------------------------------ ops / animation

#[test]
fn image_ops_on_frames() {
    let mut frames = vec![sample(), sample()];
    apply_to_frames(&ImageOp::Mirror, &mut frames, 1, false);
    assert_eq!(frames[0], sample());
    assert_eq!(frames[1], fx::mirror(&sample()));
    // size-changing ops apply to all frames
    apply_to_frames(
        &ImageOp::Rotate {
            deg: 90.0,
            smooth: false,
        },
        &mut frames,
        0,
        false,
    );
    assert!(frames.iter().all(|f| (f.width, f.height) == (2, 3)));
    // crop uses the common bounding box
    let mut a = RgbaImage::new(10, 10);
    a.put(1, 1, R);
    let mut b = RgbaImage::new(10, 10);
    b.put(5, 3, R);
    let mut fr = vec![a, b];
    apply_to_frames(&ImageOp::Crop { border: 0 }, &mut fr, 0, false);
    assert!(fr.iter().all(|f| (f.width, f.height) == (5, 3)));
    assert_eq!(fr[0].get(0, 0), Some(R));
    assert_eq!(fr[1].get(4, 2), Some(R));
}

#[test]
fn animation_sequences() {
    let a = filled(2, 2, R);
    let b = filled(2, 2, G);
    let c = filled(2, 2, B);
    let seq = vec![a.clone(), b.clone(), c.clone()];
    assert_eq!(fx::reverse(&seq), vec![c.clone(), b.clone(), a.clone()]);
    assert_eq!(
        fx::add_reverse(&seq),
        vec![a.clone(), b.clone(), c.clone(), b.clone()]
    );
    assert_eq!(fx::set_length(&seq, 5).len(), 5);
    assert_eq!(fx::set_length(&seq, 5)[4], b);
    let st = fx::stretch_length(&seq, 6);
    assert_eq!(
        st,
        vec![
            a.clone(),
            a.clone(),
            b.clone(),
            b.clone(),
            c.clone(),
            c.clone()
        ]
    );
    assert_eq!(fx::stretch_length(&seq, 1), vec![a.clone()]);

    let mut dot = RgbaImage::new(8, 8);
    dot.put(0, 0, R);
    let tr = fx::translation_sequence(&[dot.clone()], 5, 4, 0);
    assert_eq!(tr.len(), 5);
    assert_eq!(tr[0].get(0, 0), Some(R));
    assert_eq!(tr[4].get(4, 0), Some(R));
    assert_eq!(tr[2].get(2, 0), Some(R));

    let rot = fx::rotation_sequence(&[sample()], 4, 360.0, true);
    assert_eq!(rot.len(), 4);
    assert_eq!(rot[0], sample(), "full turn starts unrotated");

    let dis = fx::disappear_sequence(&[a.clone()], 3);
    assert_eq!(dis[0], a);
    assert_eq!(dis[2].get(0, 0).unwrap()[3], 0);

    let fade = fx::fade_sequence(&[a.clone()], 2, [0, 0, 0, 255]);
    assert_eq!(fade[1].get(0, 0), Some([0, 0, 0, 255]));

    let sh = fx::shrink_sequence(&[a.clone()], 3, false);
    assert_eq!(sh[0], a);
    assert_eq!(count(&sh[2], T), 4, "last frame shrunk to nothing");

    let morph = fx::morph_sequence(&[a.clone()], 3, &b);
    assert_eq!(morph[0], a);
    assert_eq!(morph[2], b);

    let mut cyc = seq.clone();
    fx::cycle_left(&mut cyc);
    assert_eq!(cyc[0], b);
    fx::cycle_right(&mut cyc);
    assert_eq!(cyc, seq);

    assert_eq!(AnimOp::Reverse.apply(&seq)[0], c);
    assert_eq!(AnimOp::Grow { n: 4 }.apply(&seq).len(), 4);
}

// ------------------------------------------------------------------ history / editor

#[test]
fn history_undo_redo_and_limit() {
    let mut h: History<i32> = History::new(3);
    let mut cur = 0;
    for v in 1..=5 {
        h.push(cur);
        cur = v;
    }
    // only 3 levels kept
    let mut undone = Vec::new();
    while let Some(p) = h.undo(cur) {
        cur = p;
        undone.push(cur);
    }
    assert_eq!(undone, vec![4, 3, 2]);
    assert_eq!(h.redo(cur), Some(3));
    cur = 3;
    h.push(cur);
    assert!(!h.can_redo(), "a new change clears redo");
}

#[test]
fn editor_pencil_stroke_and_undo() {
    let mut ed = ImageEditor::new(RgbaImage::new(16, 16), "t");
    ed.tool = Tool::Pencil;
    ed.left = R;
    ed.press((1, 1), 0, false, false);
    ed.drag((5, 1), false);
    // not committed until released
    assert_eq!(count(&ed.image, R), 0);
    assert_eq!(count(&ed.composed(), R), 5);
    ed.release();
    assert_eq!(count(&ed.image, R), 5);
    assert!(ed.modified);
    // right button uses the right colour
    ed.right = B;
    ed.press((1, 3), 1, false, false);
    ed.release();
    assert_eq!(ed.image.get(1, 3), Some(B));
    ed.undo();
    assert_eq!(ed.image.get(1, 3), Some(T));
    ed.undo();
    assert_eq!(ed.image, RgbaImage::new(16, 16));
    ed.redo();
    assert_eq!(count(&ed.image, R), 5);
}

#[test]
fn editor_shapes_fill_picker_eraser() {
    let mut ed = ImageEditor::new(RgbaImage::new(10, 10), "t");
    ed.left = R;
    ed.right = G;
    ed.tool = Tool::Rect;
    ed.shape_fill = ShapeFill::Both;
    ed.press((0, 0), 0, false, false);
    ed.drag((4, 4), false);
    ed.release();
    assert_eq!(ed.image.get(0, 0), Some(R), "outline in the drawing colour");
    assert_eq!(ed.image.get(2, 2), Some(G), "fill in the other colour");

    ed.tool = Tool::Fill;
    ed.press((8, 8), 0, false, false);
    ed.release();
    assert_eq!(ed.image.get(9, 9), Some(R));
    assert_eq!(
        ed.image.get(2, 2),
        Some(G),
        "fill does not cross the outline"
    );

    ed.tool = Tool::Picker;
    ed.press((2, 2), 0, false, false);
    assert_eq!(ed.left, G);

    ed.tool = Tool::Eraser;
    ed.brush = 1;
    ed.press((9, 9), 0, false, false);
    ed.release();
    assert_eq!(ed.image.get(9, 9), Some(T));

    ed.tool = Tool::Line;
    ed.left = B;
    ed.press((0, 9), 0, false, false);
    ed.drag((9, 8), true); // shift snaps to horizontal
    ed.release();
    assert_eq!(ed.image.get(9, 9), Some(B));
}

#[test]
fn editor_selection_move_copy_paste() {
    let mut ed = ImageEditor::new(RgbaImage::new(8, 8), "t");
    ed.image.put(1, 1, R);
    ed.tool = Tool::Select;
    ed.press((0, 0), 0, false, false);
    ed.drag((2, 2), false);
    ed.release();
    assert_eq!(ed.selection, Some((0, 0, 3, 3)));
    // move the selection by (+3, +2)
    ed.press((1, 1), 0, false, false);
    ed.drag((4, 3), false);
    ed.release();
    ed.commit_pending();
    assert_eq!(ed.image.get(1, 1), Some(T));
    assert_eq!(ed.image.get(4, 3), Some(R));
    ed.undo();
    assert_eq!(ed.image.get(1, 1), Some(R));
    assert_eq!(ed.image.get(4, 3), Some(T));

    // internal paste of a selection
    ed.selection = Some((0, 0, 3, 3));
    let sel = ed.selected_image();
    assert_eq!(sel.get(1, 1), Some(R));
    ed.selection = Some((5, 5, 1, 1));
    ed.paste_image(sel);
    ed.commit_pending();
    assert_eq!(ed.image.get(6, 6), Some(R));
    ed.delete_selection();
    assert_eq!(ed.image.get(5, 5), Some(T));
}

#[test]
fn editor_ops_and_text() {
    let mut ed = ImageEditor::new(sample(), "t");
    ed.apply_op(&ImageOp::Mirror);
    assert_eq!(ed.image, fx::mirror(&sample()));
    ed.undo();
    assert_eq!(ed.image, sample());
    // size-changing ops are refused for frames of a multi-frame sprite
    let mut fr = ImageEditor::for_frame(sample(), Some(sample()), 1, 3, "s");
    fr.apply_op(&ImageOp::Stretch {
        w: 9,
        h: 9,
        smooth: false,
    });
    assert_eq!((fr.image.width, fr.image.height), (3, 2));
    assert!(fr.onion.is_some());

    // text with the built-in font draws something in the left colour
    let mut t = ImageEditor::new(RgbaImage::new(64, 32), "t");
    t.tool = Tool::Text;
    t.left = R;
    t.text = "Hi".into();
    t.text_size = 20.0;
    t.press((2, 2), 0, false, false);
    assert_eq!(count(&t.image, T), 64 * 32, "pending until applied");
    t.commit_text();
    let painted = t.image.data.chunks_exact(4).filter(|p| p[3] > 0).count();
    assert!(painted > 20, "text rendered ({painted} px)");
}
