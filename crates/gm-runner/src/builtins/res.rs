//! Chapters 408 (resource queries) and 409 (changing resources).

use super::{bo, r, st, zero, Reg, A};
use crate::actions::{ActKind, Act, ActionList, Node};
use crate::draw::Texture;
use crate::game::Game;
use crate::path::PathRes;
use crate::res::*;
use gml::{Error, Value, Vm};
use std::rc::Rc;

type R = Result<Value, Error>;

fn get<'a, T>(v: &'a [Option<T>], i: i32) -> Option<&'a T> {
    if i < 0 {
        return None;
    }
    v.get(i as usize).and_then(|x| x.as_ref())
}

fn get_mut<'a, T>(v: &'a mut [Option<T>], i: i32, what: &str) -> Result<&'a mut T, Error> {
    if i < 0 {
        return Err(Error::new(format!("{} does not exist: {}", what, i)));
    }
    v.get_mut(i as usize)
        .and_then(|x| x.as_mut())
        .ok_or_else(|| Error::new(format!("{} does not exist: {}", what, i)))
}

fn alloc<T>(v: &mut Vec<Option<T>>, t: T) -> i32 {
    v.push(Some(t));
    (v.len() - 1) as i32
}

/// Removes the background colour (bottom-left pixel) and optionally smooths edges.
fn post_load(t: &mut Texture, removeback: bool, smooth: bool) {
    if !removeback || t.w == 0 || t.h == 0 {
        return;
    }
    let key = t.get(0, t.h - 1);
    for p in t.px.iter_mut() {
        if p[0] == key[0] && p[1] == key[1] && p[2] == key[2] {
            p[3] = 0;
        }
    }
    if smooth {
        let src = t.clone();
        for y in 0..t.h as i32 {
            for x in 0..t.w as i32 {
                let i = (y as u32 * t.w + x as u32) as usize;
                if src.px[i][3] == 0 {
                    continue;
                }
                let mut n = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (xx, yy) = (x + dx, y + dy);
                        if xx >= 0 && yy >= 0 && xx < t.w as i32 && yy < t.h as i32 && src.px[(yy as u32 * t.w + xx as u32) as usize][3] == 0 {
                            n += 1;
                        }
                    }
                }
                t.px[i][3] = (t.px[i][3] as i32 - 32 * n).max(0) as u8;
            }
        }
    }
}

/// Loads an image file, splitting a horizontal strip into `n` frames.
fn load_frames(path: &std::path::Path, n: i32, removeback: bool, smooth: bool) -> Option<Vec<Texture>> {
    let img = Texture::load_file(path)?;
    let n = n.max(1) as u32;
    let fw = img.w / n;
    let mut out = Vec::new();
    for k in 0..n {
        let mut f = Texture::new(fw.max(1), img.h);
        f.blit(&img, (k * fw) as i32, 0, fw as i32, img.h as i32, 0, 0);
        post_load(&mut f, removeback, smooth);
        out.push(f);
    }
    Some(out)
}

fn screen_part(g: &mut Game, surf: Option<i32>, x: i32, y: i32, w: i32, h: i32) -> Result<Texture, Error> {
    let src = match surf {
        Some(s) => {
            g.gfx.surface_size(s).ok_or_else(|| Error::new(format!("Surface does not exist: {}", s)))?;
            crate::draw::Target::Surface(s)
        }
        None => crate::draw::Target::Back,
    };
    let mut t = g.gfx.be.read(src, x, y, w.max(1) as u32, h.max(1) as u32);
    if surf.is_none() {
        for p in t.px.iter_mut() {
            p[3] = 255;
        }
    }
    Ok(t)
}

fn code_list(vm: &mut Vm, g: &Game, code: &str, where_: &str) -> Result<ActionList, Error> {
    let prog = vm.compile(where_, code.as_bytes(), g).map_err(Error::from)?;
    Ok(ActionList {
        nodes: vec![Node::Act(Box::new(Act {
            idx: 0,
            kind: ActKind::Code(prog),
            applies: gml::special::SELF,
            relative: false,
            negate: false,
            question: false,
            args: Vec::new(),
        }))],
    })
}

fn append_list(old: Option<&Rc<ActionList>>, add: ActionList) -> Rc<ActionList> {
    // Lists are immutable once shared; build a new one holding the old nodes by
    // wrapping them in a block that re-executes the old list.
    match old {
        None => Rc::new(add),
        Some(o) => {
            let mut nodes: Vec<Node> = Vec::new();
            nodes.push(Node::Shared(o.clone()));
            nodes.extend(add.nodes);
            Rc::new(ActionList { nodes })
        }
    }
}

fn sprite_frames_gray(t: &Texture) -> Vec<u8> {
    t.px.iter().map(|p| ((p[0] as u32 + p[1] as u32 + p[2] as u32) / 3) as u8).collect()
}

pub fn register(l: &mut Reg) {
    // ---------------------------------------------------------------- sprites
    l.push(("sprite_exists", |g, _, _, a| bo(g.sprite(a.i(0)?).is_some())));
    l.push(("sprite_get_name", |g, _, _, a| st(g.sprite(a.i(0)?).map(|s| s.name.as_str()).unwrap_or("<undefined>"))));
    l.push(("sprite_get_number", |g, _, _, a| r(g.sprite(a.i(0)?).map(|s| s.frames.len() as f64).unwrap_or(0.0))));
    l.push(("sprite_get_width", |g, _, _, a| r(g.sprite(a.i(0)?).map(|s| s.width as f64).unwrap_or(0.0))));
    l.push(("sprite_get_height", |g, _, _, a| r(g.sprite(a.i(0)?).map(|s| s.height as f64).unwrap_or(0.0))));
    l.push(("sprite_get_xoffset", |g, _, _, a| r(g.sprite(a.i(0)?).map(|s| s.origin_x as f64).unwrap_or(0.0))));
    l.push(("sprite_get_yoffset", |g, _, _, a| r(g.sprite(a.i(0)?).map(|s| s.origin_y as f64).unwrap_or(0.0))));
    l.push(("sprite_get_bbox_left", |g, _, _, a| r(g.sprite(a.i(0)?).map(|s| s.bbox_left as f64).unwrap_or(0.0))));
    l.push(("sprite_get_bbox_right", |g, _, _, a| r(g.sprite(a.i(0)?).map(|s| s.bbox_right as f64).unwrap_or(0.0))));
    l.push(("sprite_get_bbox_top", |g, _, _, a| r(g.sprite(a.i(0)?).map(|s| s.bbox_top as f64).unwrap_or(0.0))));
    l.push(("sprite_get_bbox_bottom", |g, _, _, a| r(g.sprite(a.i(0)?).map(|s| s.bbox_bottom as f64).unwrap_or(0.0))));
    l.push(("sprite_save", |g, _, _, a| {
        let s = g.sprite(a.i(0)?).ok_or_else(|| Error::new("Sprite does not exist"))?;
        let n = s.frames.len().max(1);
        let f = s.frames[(a.i(1)?.max(0) as usize) % n].clone();
        f.save_png(&g.resolve_path(&a.s(2)?), false).map_err(|e| Error::new(e.to_string()))?;
        zero()
    }));
    l.push(("sprite_save_strip", |g, _, _, a| {
        let s = g.sprite(a.i(0)?).ok_or_else(|| Error::new("Sprite does not exist"))?;
        let mut strip = Texture::new(s.width * s.frames.len().max(1) as u32, s.height);
        for (k, f) in s.frames.iter().enumerate() {
            strip.blit(f, 0, 0, f.w as i32, f.h as i32, (k as u32 * s.width) as i32, 0);
        }
        strip.save_png(&g.resolve_path(&a.s(1)?), false).map_err(|e| Error::new(e.to_string()))?;
        zero()
    }));
    // ---------------------------------------------------------------- sounds
    l.push(("sound_exists", |g, _, _, a| bo(get(&g.sounds, a.i(0)?).is_some())));
    l.push(("sound_get_name", |g, _, _, a| st(get(&g.sounds, a.i(0)?).map(|s| s.name.as_str()).unwrap_or(""))));
    l.push(("sound_get_kind", |g, _, _, a| r(get(&g.sounds, a.i(0)?).map(|s| s.kind as f64).unwrap_or(-1.0))));
    l.push(("sound_get_preload", |g, _, _, a| bo(get(&g.sounds, a.i(0)?).map(|s| s.preload).unwrap_or(false))));
    l.push(("sound_discard", |g, _, _, a| {
        g.audio.discard(a.i(0)?);
        zero()
    }));
    l.push(("sound_restore", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(s) = g.sound_res(i) {
            g.audio.restore(i, &s);
        }
        zero()
    }));
    // ---------------------------------------------------------------- backgrounds
    l.push(("background_exists", |g, _, _, a| bo(g.background(a.i(0)?).is_some())));
    l.push(("background_get_name", |g, _, _, a| st(g.background(a.i(0)?).map(|s| s.name.as_str()).unwrap_or("<undefined>"))));
    l.push(("background_get_width", |g, _, _, a| r(g.background(a.i(0)?).map(|s| s.tex.w as f64).unwrap_or(0.0))));
    l.push(("background_get_height", |g, _, _, a| r(g.background(a.i(0)?).map(|s| s.tex.h as f64).unwrap_or(0.0))));
    l.push(("background_save", |g, _, _, a| {
        let b = g.background(a.i(0)?).ok_or_else(|| Error::new("Background does not exist"))?;
        b.tex.save_png(&g.resolve_path(&a.s(1)?), false).map_err(|e| Error::new(e.to_string()))?;
        zero()
    }));
    // ---------------------------------------------------------------- fonts
    l.push(("font_exists", |g, _, _, a| bo(get(&g.fonts, a.i(0)?).is_some())));
    l.push(("font_get_name", |g, _, _, a| st(get(&g.fonts, a.i(0)?).map(|s| s.name.as_str()).unwrap_or("<undefined>"))));
    l.push(("font_get_fontname", |g, _, _, a| st(get(&g.fonts, a.i(0)?).map(|s| s.font_name.as_str()).unwrap_or(""))));
    l.push(("font_get_bold", |g, _, _, a| bo(get(&g.fonts, a.i(0)?).map(|s| s.bold).unwrap_or(false))));
    l.push(("font_get_italic", |g, _, _, a| bo(get(&g.fonts, a.i(0)?).map(|s| s.italic).unwrap_or(false))));
    l.push(("font_get_first", |g, _, _, a| r(get(&g.fonts, a.i(0)?).map(|s| s.first as f64).unwrap_or(0.0))));
    l.push(("font_get_last", |g, _, _, a| r(get(&g.fonts, a.i(0)?).map(|s| s.last as f64).unwrap_or(0.0))));
    // ---------------------------------------------------------------- paths
    l.push(("path_exists", |g, _, _, a| bo(get(&g.paths, a.i(0)?).is_some())));
    l.push(("path_get_name", |g, _, _, a| st(get(&g.paths, a.i(0)?).map(|s| s.name.as_str()).unwrap_or("<undefined>"))));
    l.push(("path_get_length", |g, _, _, a| r(get(&g.paths, a.i(0)?).map(|s| s.length).unwrap_or(0.0))));
    l.push(("path_get_kind", |g, _, _, a| r(get(&g.paths, a.i(0)?).map(|s| s.kind as f64).unwrap_or(0.0))));
    l.push(("path_get_closed", |g, _, _, a| bo(get(&g.paths, a.i(0)?).map(|s| s.closed).unwrap_or(false))));
    l.push(("path_get_precision", |g, _, _, a| r(get(&g.paths, a.i(0)?).map(|s| s.precision as f64).unwrap_or(0.0))));
    l.push(("path_get_number", |g, _, _, a| r(get(&g.paths, a.i(0)?).map(|s| s.points.len() as f64).unwrap_or(0.0))));
    fn pt(g: &Game, a: &A, k: usize) -> R {
        let p = get(&g.paths, a.i(0)?);
        let n = a.i(1)?;
        r(p.and_then(|p| p.points.get(n.max(0) as usize)).map(|q| [q.0, q.1, q.2][k]).unwrap_or(0.0))
    }
    l.push(("path_get_point_x", |g, _, _, a| pt(g, a, 0)));
    l.push(("path_get_point_y", |g, _, _, a| pt(g, a, 1)));
    l.push(("path_get_point_speed", |g, _, _, a| pt(g, a, 2)));
    l.push(("path_get_x", |g, _, _, a| r(get(&g.paths, a.i(0)?).map(|p| p.at(a.r(1).unwrap_or(0.0)).0).unwrap_or(0.0))));
    l.push(("path_get_y", |g, _, _, a| r(get(&g.paths, a.i(0)?).map(|p| p.at(a.r(1).unwrap_or(0.0)).1).unwrap_or(0.0))));
    l.push(("path_get_speed", |g, _, _, a| r(get(&g.paths, a.i(0)?).map(|p| p.at(a.r(1).unwrap_or(0.0)).2).unwrap_or(0.0))));
    // ---------------------------------------------------------------- scripts, timelines, objects, rooms
    l.push(("script_exists", |g, _, _, a| bo(get(&g.scripts, a.i(0)?).is_some())));
    l.push(("script_get_name", |g, _, _, a| st(get(&g.scripts, a.i(0)?).map(|s| s.name.as_str()).unwrap_or("<undefined>"))));
    l.push(("script_get_text", |g, _, _, a| st(get(&g.scripts, a.i(0)?).map(|s| s.code.as_str()).unwrap_or(""))));
    l.push(("timeline_exists", |g, _, _, a| bo(get(&g.timelines, a.i(0)?).is_some())));
    l.push(("timeline_get_name", |g, _, _, a| st(get(&g.timelines, a.i(0)?).map(|s| s.name.as_str()).unwrap_or("<undefined>"))));
    l.push(("object_exists", |g, _, _, a| bo(g.object(a.i(0)?).is_some())));
    l.push(("object_get_name", |g, _, _, a| st(g.object(a.i(0)?).map(|s| s.name.as_str()).unwrap_or("<undefined>"))));
    l.push(("object_get_sprite", |g, _, _, a| r(g.object(a.i(0)?).map(|s| s.sprite as f64).unwrap_or(-1.0))));
    l.push(("object_get_solid", |g, _, _, a| bo(g.object(a.i(0)?).map(|s| s.solid).unwrap_or(false))));
    l.push(("object_get_visible", |g, _, _, a| bo(g.object(a.i(0)?).map(|s| s.visible).unwrap_or(false))));
    l.push(("object_get_depth", |g, _, _, a| r(g.object(a.i(0)?).map(|s| s.depth as f64).unwrap_or(0.0))));
    l.push(("object_get_persistent", |g, _, _, a| bo(g.object(a.i(0)?).map(|s| s.persistent).unwrap_or(false))));
    l.push(("object_get_mask", |g, _, _, a| r(g.object(a.i(0)?).map(|s| s.mask as f64).unwrap_or(-1.0))));
    l.push(("object_get_parent", |g, _, _, a| r(g.object(a.i(0)?).map(|s| if s.parent < 0 { -1.0 } else { s.parent as f64 }).unwrap_or(-1.0))));
    l.push(("object_is_ancestor", |g, _, _, a| {
        let (o, anc) = (a.i(0)?, a.i(1)?);
        bo(o != anc && g.is_a(o, anc))
    }));
    l.push(("room_exists", |g, _, _, a| bo(g.room_exists(a.i(0)?))));
    l.push(("room_get_name", |g, _, _, a| st(get(&g.rooms, a.i(0)?).map(|s| s.data.name.as_str()).unwrap_or("<undefined>"))));

    // ---------------------------------------------------------------- 409: sprites
    l.push(("sprite_set_offset", |g, _, _, a| {
        let (x, y) = (a.i(1)?, a.i(2)?);
        let s = get_mut(&mut g.sprites, a.i(0)?, "Sprite")?;
        s.origin_x = x;
        s.origin_y = y;
        zero()
    }));
    l.push(("sprite_duplicate", |g, _, _, a| {
        let s = g.sprite(a.i(0)?).cloned().ok_or_else(|| Error::new("Sprite does not exist"))?;
        r(alloc(&mut g.sprites, s) as f64)
    }));
    l.push(("sprite_assign", |g, _, _, a| {
        let s = g.sprite(a.i(1)?).cloned().ok_or_else(|| Error::new("Sprite does not exist"))?;
        let d = get_mut(&mut g.sprites, a.i(0)?, "Sprite")?;
        let name = d.name.clone();
        *d = s;
        d.name = name;
        zero()
    }));
    l.push(("sprite_merge", |g, _, _, a| {
        let s2 = g.sprite(a.i(1)?).cloned().ok_or_else(|| Error::new("Sprite does not exist"))?;
        let d = get_mut(&mut g.sprites, a.i(0)?, "Sprite")?;
        for f in s2.frames {
            if f.w == d.width && f.h == d.height {
                d.frames.push(f);
            } else {
                // stretch to size (nearest)
                let mut t = Texture::new(d.width, d.height);
                for y in 0..d.height {
                    for x in 0..d.width {
                        let sx = x * f.w / d.width.max(1);
                        let sy = y * f.h / d.height.max(1);
                        t.px[(y * d.width + x) as usize] = f.get(sx.min(f.w - 1), sy.min(f.h - 1));
                    }
                }
                d.frames.push(Rc::new(t));
            }
        }
        d.rebuild_masks();
        zero()
    }));
    fn sprite_from_file(g: &mut Game, a: &A, base: usize) -> Result<Option<SpriteRes>, Error> {
        let path = g.resolve_path(&a.s(base)?);
        let (n, rb, sm, xo, yo) = (a.i(base + 1)?, a.b(base + 2)?, a.b(base + 3)?, a.i(base + 4)?, a.i(base + 5)?);
        Ok(load_frames(&path, n, rb, sm).map(|fr| {
            let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            SpriteRes::from_frames(name, fr, xo, yo, true)
        }))
    }
    l.push(("sprite_add", |g, _, _, a| match sprite_from_file(g, a, 0)? {
        Some(s) => r(alloc(&mut g.sprites, s) as f64),
        None => r(-1.0),
    }));
    l.push(("sprite_replace", |g, _, _, a| {
        let i = a.i(0)?;
        match sprite_from_file(g, a, 1)? {
            Some(mut s) => {
                let d = get_mut(&mut g.sprites, i, "Sprite")?;
                s.name = d.name.clone();
                *d = s;
                bo(true)
            }
            None => bo(false),
        }
    }));
    fn gmspr(g: &Game, a: &A, i: usize) -> Result<Option<SpriteRes>, Error> {
        let p = g.resolve_path(&a.s(i)?);
        let Ok(bytes) = crate::vfs::read(&p) else { return Ok(None) };
        Ok(gm_format::read_sprite_export(&bytes).ok().map(|s| SpriteRes::from_format(&s)))
    }
    l.push(("sprite_add_sprite", |g, _, _, a| match gmspr(g, a, 0)? {
        Some(s) => r(alloc(&mut g.sprites, s) as f64),
        None => r(-1.0),
    }));
    l.push(("sprite_replace_sprite", |g, _, _, a| match gmspr(g, a, 1)? {
        Some(s) => {
            *get_mut(&mut g.sprites, a.i(0)?, "Sprite")? = s;
            bo(true)
        }
        None => bo(false),
    }));
    l.push(("sprite_create_from_screen", |g, _, _, a| {
        let mut t = screen_part(g, None, a.i(0)?, a.i(1)?, a.i(2)?, a.i(3)?)?;
        post_load(&mut t, a.b(4)?, a.b(5)?);
        let s = SpriteRes::from_frames("screen".into(), vec![t], a.i(6)?, a.i(7)?, true);
        r(alloc(&mut g.sprites, s) as f64)
    }));
    l.push(("sprite_add_from_screen", |g, _, _, a| {
        let mut t = screen_part(g, None, a.i(1)?, a.i(2)?, a.i(3)?, a.i(4)?)?;
        post_load(&mut t, a.b(5)?, a.b(6)?);
        let s = get_mut(&mut g.sprites, a.i(0)?, "Sprite")?;
        s.frames.push(Rc::new(t));
        s.rebuild_masks();
        zero()
    }));
    l.push(("sprite_create_from_surface", |g, _, _, a| {
        let mut t = screen_part(g, Some(a.i(0)?), a.i(1)?, a.i(2)?, a.i(3)?, a.i(4)?)?;
        post_load(&mut t, a.b(5)?, a.b(6)?);
        let s = SpriteRes::from_frames("surface".into(), vec![t], a.i(7)?, a.i(8)?, true);
        r(alloc(&mut g.sprites, s) as f64)
    }));
    l.push(("sprite_add_from_surface", |g, _, _, a| {
        let mut t = screen_part(g, Some(a.i(1)?), a.i(2)?, a.i(3)?, a.i(4)?, a.i(5)?)?;
        post_load(&mut t, a.b(6)?, a.b(7)?);
        let s = get_mut(&mut g.sprites, a.i(0)?, "Sprite")?;
        s.frames.push(Rc::new(t));
        s.rebuild_masks();
        zero()
    }));
    l.push(("sprite_delete", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(s) = g.sprites.get_mut(i.max(0) as usize) {
            *s = None;
        }
        zero()
    }));
    l.push(("sprite_set_alpha_from_sprite", |g, _, _, a| {
        let src = g.sprite(a.i(1)?).cloned().ok_or_else(|| Error::new("Sprite does not exist"))?;
        let d = get_mut(&mut g.sprites, a.i(0)?, "Sprite")?;
        for (k, f) in d.frames.iter_mut().enumerate() {
            let sf = &src.frames[k % src.frames.len().max(1)];
            let gray = sprite_frames_gray(sf);
            let mut t = (**f).clone();
            for y in 0..t.h.min(sf.h) {
                for x in 0..t.w.min(sf.w) {
                    let i = (y * t.w + x) as usize;
                    let v = gray[(y * sf.w + x) as usize];
                    t.px[i][3] = ((t.px[i][3] as u32 * v as u32) / 255) as u8;
                }
            }
            *f = Rc::new(t);
        }
        d.rebuild_masks();
        zero()
    }));
    l.push(("sprite_collision_mask", |g, _, _, a| {
        let s = get_mut(&mut g.sprites, a.i(0)?, "Sprite")?;
        s.separate = a.b(1)?;
        s.bbox_mode = a.i(2)?;
        s.bbox_left = a.i(3)?;
        s.bbox_right = a.i(4)?;
        s.bbox_top = a.i(5)?;
        s.bbox_bottom = a.i(6)?;
        s.shape = a.i(7)?;
        s.tolerance = a.i(8)?;
        s.rebuild_masks();
        zero()
    }));
    // ---------------------------------------------------------------- sounds
    fn sound_from_file(g: &Game, a: &A, base: usize) -> Result<Option<SoundRes>, Error> {
        let p = g.resolve_path(&a.s(base)?);
        let Ok(data) = crate::vfs::read(&p) else { return Ok(None) };
        let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
        Ok(Some(SoundRes {
            name: p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
            kind: Some(a.i(base + 1)?).filter(|k| (0..=3).contains(k)).unwrap_or(0),
            file_type: ext,
            file_name: p.to_string_lossy().into_owned(),
            data: Some(std::sync::Arc::from(data.as_slice())),
            effects: 0,
            volume: 1.0,
            pan: 0.0,
            preload: a.b(base + 2)?,
        }))
    }
    l.push(("sound_add", |g, _, _, a| match sound_from_file(g, a, 0)? {
        Some(s) => {
            let i = alloc(&mut g.sounds, s);
            g.audio.forget(i);
            if let Some(Some(s)) = g.sounds.get_mut(i as usize) {
                s.name = format!("__newsound{}", i);
            }
            if let Some(s) = g.sound_res(i) {
                if s.preload {
                    g.audio.restore(i, &s);
                }
            }
            r(i as f64)
        }
        None => r(-1.0),
    }));
    l.push(("sound_replace", |g, _, _, a| {
        let i = a.i(0)?;
        match sound_from_file(g, a, 1)? {
            Some(s) => {
                *get_mut(&mut g.sounds, i, "Sound")? = s;
                g.audio.forget(i);
                bo(true)
            }
            None => bo(false),
        }
    }));
    l.push(("sound_delete", |g, _, _, a| {
        let i = a.i(0)?;
        g.audio.forget(i);
        if let Some(s) = g.sounds.get_mut(i.max(0) as usize).filter(|_| i >= 0) {
            *s = None;
        }
        zero()
    }));
    // ---------------------------------------------------------------- backgrounds
    fn bg_new(name: &str, t: Texture) -> BackgroundRes {
        BackgroundRes {
            name: name.into(),
            tex: Rc::new(t),
            tileset: false,
            tile_width: 16,
            tile_height: 16,
            tile_hoffset: 0,
            tile_voffset: 0,
            tile_hsep: 0,
            tile_vsep: 0,
            smooth: false,
            preload: true,
        }
    }
    l.push(("background_duplicate", |g, _, _, a| {
        let b = g.background(a.i(0)?).cloned().ok_or_else(|| Error::new("Background does not exist"))?;
        r(alloc(&mut g.backgrounds, b) as f64)
    }));
    l.push(("background_assign", |g, _, _, a| {
        let b = g.background(a.i(1)?).cloned().ok_or_else(|| Error::new("Background does not exist"))?;
        let d = get_mut(&mut g.backgrounds, a.i(0)?, "Background")?;
        let n = d.name.clone();
        *d = b;
        d.name = n;
        zero()
    }));
    l.push(("background_add", |g, _, _, a| {
        let p = g.resolve_path(&a.s(0)?);
        match load_frames(&p, 1, a.b(1)?, a.b(2)?) {
            Some(mut f) => {
                let name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                r(alloc(&mut g.backgrounds, bg_new(&name, f.remove(0))) as f64)
            }
            None => r(-1.0),
        }
    }));
    l.push(("background_replace", |g, _, _, a| {
        let p = g.resolve_path(&a.s(1)?);
        match load_frames(&p, 1, a.b(2)?, a.b(3)?) {
            Some(mut f) => {
                get_mut(&mut g.backgrounds, a.i(0)?, "Background")?.tex = Rc::new(f.remove(0));
                bo(true)
            }
            None => bo(false),
        }
    }));
    l.push(("background_add_background", |g, _, _, a| {
        let p = g.resolve_path(&a.s(0)?);
        match Texture::load_file(&p) {
            Some(t) => r(alloc(&mut g.backgrounds, bg_new("background", t)) as f64),
            None => r(-1.0),
        }
    }));
    l.push(("background_replace_background", |g, _, _, a| {
        let p = g.resolve_path(&a.s(1)?);
        match Texture::load_file(&p) {
            Some(t) => {
                get_mut(&mut g.backgrounds, a.i(0)?, "Background")?.tex = Rc::new(t);
                bo(true)
            }
            None => bo(false),
        }
    }));
    l.push(("background_create_color", |g, _, _, a| {
        let mut t = Texture::new(a.i(0)?.max(1) as u32, a.i(1)?.max(1) as u32);
        t.fill(crate::draw::rgba8(a.i(2)?, 1.0));
        r(alloc(&mut g.backgrounds, bg_new("color", t)) as f64)
    }));
    l.push(("background_create_gradient", |g, _, _, a| {
        let (w, h) = (a.i(0)?.max(1) as u32, a.i(1)?.max(1) as u32);
        let (c1, c2, kind) = (a.i(2)?, a.i(3)?, a.i(4)?);
        let mut t = Texture::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let fx = x as f64 / (w.max(2) - 1) as f64;
                let fy = y as f64 / (h.max(2) - 1) as f64;
                let (dx, dy) = ((fx - 0.5).abs() * 2.0, (fy - 0.5).abs() * 2.0);
                let f = match kind {
                    0 => fx,
                    1 => fy,
                    2 => dx.max(dy),
                    3 => (dx * dx + dy * dy).sqrt().min(1.0),
                    4 => dx,
                    _ => dy,
                };
                t.px[(y * w + x) as usize] = crate::draw::rgba8(crate::particles::mix(c1, c2, f), 1.0);
            }
        }
        r(alloc(&mut g.backgrounds, bg_new("gradient", t)) as f64)
    }));
    l.push(("background_create_from_screen", |g, _, _, a| {
        let mut t = screen_part(g, None, a.i(0)?, a.i(1)?, a.i(2)?, a.i(3)?)?;
        post_load(&mut t, a.b(4)?, a.b(5)?);
        r(alloc(&mut g.backgrounds, bg_new("screen", t)) as f64)
    }));
    l.push(("background_create_from_surface", |g, _, _, a| {
        let mut t = screen_part(g, Some(a.i(0)?), a.i(1)?, a.i(2)?, a.i(3)?, a.i(4)?)?;
        post_load(&mut t, a.b(5)?, a.b(6)?);
        r(alloc(&mut g.backgrounds, bg_new("surface", t)) as f64)
    }));
    l.push(("background_delete", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(s) = g.backgrounds.get_mut(i.max(0) as usize) {
            *s = None;
        }
        zero()
    }));
    l.push(("background_set_alpha_from_background", |g, _, _, a| {
        let src = g.background(a.i(1)?).map(|b| b.tex.clone()).ok_or_else(|| Error::new("Background does not exist"))?;
        let d = get_mut(&mut g.backgrounds, a.i(0)?, "Background")?;
        let gray = sprite_frames_gray(&src);
        let mut t = (*d.tex).clone();
        for y in 0..t.h.min(src.h) {
            for x in 0..t.w.min(src.w) {
                let i = (y * t.w + x) as usize;
                t.px[i][3] = ((t.px[i][3] as u32 * gray[(y * src.w + x) as usize] as u32) / 255) as u8;
            }
        }
        d.tex = Rc::new(t);
        zero()
    }));
    // ---------------------------------------------------------------- fonts
    fn font_new(a: &A, base: usize) -> Result<FontRes, Error> {
        Ok(FontRes {
            name: a.s(base)?,
            font_name: a.s(base)?,
            size: a.i(base + 1)?,
            bold: a.b(base + 2)?,
            italic: a.b(base + 3)?,
            first: a.i(base + 4)?,
            last: a.i(base + 5)?,
            sprite: None,
        })
    }
    l.push(("font_add", |g, _, _, a| {
        let f = font_new(a, 0)?;
        r(alloc(&mut g.fonts, f) as f64)
    }));
    l.push(("font_replace", |g, _, _, a| {
        let f = font_new(a, 1)?;
        *get_mut(&mut g.fonts, a.i(0)?, "Font")? = f;
        zero()
    }));
    l.push(("font_add_sprite", |g, _, _, a| {
        let f = FontRes {
            name: "sprite font".into(),
            font_name: "Arial".into(),
            size: 12,
            bold: false,
            italic: false,
            first: a.i(1)?,
            last: 255,
            sprite: Some((a.i(0)?, a.i(1)?, a.b(2)?, a.i(3)?)),
        };
        r(alloc(&mut g.fonts, f) as f64)
    }));
    l.push(("font_replace_sprite", |g, _, _, a| {
        let spr = (a.i(1)?, a.i(2)?, a.b(3)?, a.i(4)?);
        get_mut(&mut g.fonts, a.i(0)?, "Font")?.sprite = Some(spr);
        zero()
    }));
    l.push(("font_delete", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(s) = g.fonts.get_mut(i.max(0) as usize) {
            *s = None;
        }
        zero()
    }));
    // ---------------------------------------------------------------- paths
    fn path<'a>(g: &'a mut Game, a: &A) -> Result<&'a mut PathRes, Error> {
        get_mut(&mut g.paths, a.i(0)?, "Path")
    }
    l.push(("path_set_kind", |g, _, _, a| {
        let k = a.i(1)?;
        let p = path(g, a)?;
        p.kind = k;
        p.rebuild();
        zero()
    }));
    l.push(("path_set_closed", |g, _, _, a| {
        let k = a.b(1)?;
        let p = path(g, a)?;
        p.closed = k;
        p.rebuild();
        zero()
    }));
    l.push(("path_set_precision", |g, _, _, a| {
        let k = a.i(1)?.clamp(1, 8);
        let p = path(g, a)?;
        p.precision = k;
        p.rebuild();
        zero()
    }));
    l.push(("path_add", |g, _, _, _| r(alloc(&mut g.paths, PathRes::new_empty("path".into())) as f64)));
    l.push(("path_assign", |g, _, _, a| {
        let src = get(&g.paths, a.i(1)?).cloned().ok_or_else(|| Error::new("Path does not exist"))?;
        let d = path(g, a)?;
        let n = d.name.clone();
        *d = src;
        d.name = n;
        zero()
    }));
    l.push(("path_duplicate", |g, _, _, a| {
        let src = get(&g.paths, a.i(0)?).cloned().ok_or_else(|| Error::new("Path does not exist"))?;
        r(alloc(&mut g.paths, src) as f64)
    }));
    l.push(("path_append", |g, _, _, a| {
        let src = get(&g.paths, a.i(1)?).cloned().ok_or_else(|| Error::new("Path does not exist"))?;
        let d = path(g, a)?;
        d.points.extend(src.points);
        d.rebuild();
        zero()
    }));
    l.push(("path_delete", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(s) = g.paths.get_mut(i.max(0) as usize) {
            *s = None;
        }
        zero()
    }));
    l.push(("path_add_point", |g, _, _, a| {
        let q = (a.r(1)?, a.r(2)?, a.r(3)?);
        let p = path(g, a)?;
        p.points.push(q);
        p.rebuild();
        zero()
    }));
    l.push(("path_insert_point", |g, _, _, a| {
        let (n, q) = (a.i(1)?, (a.r(2)?, a.r(3)?, a.r(4)?));
        let p = path(g, a)?;
        let n = (n.max(0) as usize).min(p.points.len());
        p.points.insert(n, q);
        p.rebuild();
        zero()
    }));
    l.push(("path_change_point", |g, _, _, a| {
        let (n, q) = (a.i(1)?, (a.r(2)?, a.r(3)?, a.r(4)?));
        let p = path(g, a)?;
        if let Some(x) = p.points.get_mut(n.max(0) as usize) {
            *x = q;
        }
        p.rebuild();
        zero()
    }));
    l.push(("path_delete_point", |g, _, _, a| {
        let n = a.i(1)?;
        let p = path(g, a)?;
        if n >= 0 && (n as usize) < p.points.len() {
            p.points.remove(n as usize);
        }
        p.rebuild();
        zero()
    }));
    l.push(("path_clear_points", |g, _, _, a| {
        let p = path(g, a)?;
        p.points.clear();
        p.rebuild();
        zero()
    }));
    l.push(("path_reverse", |g, _, _, a| {
        let p = path(g, a)?;
        p.points.reverse();
        p.rebuild();
        zero()
    }));
    fn center(p: &PathRes) -> (f64, f64) {
        if p.points.is_empty() {
            return (0.0, 0.0);
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for q in &p.points {
            x0 = x0.min(q.0);
            y0 = y0.min(q.1);
            x1 = x1.max(q.0);
            y1 = y1.max(q.1);
        }
        ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
    }
    l.push(("path_mirror", |g, _, _, a| {
        let p = path(g, a)?;
        let (cx, _) = center(p);
        p.points.iter_mut().for_each(|q| q.0 = 2.0 * cx - q.0);
        p.rebuild();
        zero()
    }));
    l.push(("path_flip", |g, _, _, a| {
        let p = path(g, a)?;
        let (_, cy) = center(p);
        p.points.iter_mut().for_each(|q| q.1 = 2.0 * cy - q.1);
        p.rebuild();
        zero()
    }));
    l.push(("path_rotate", |g, _, _, a| {
        let ang = a.r(1)?.to_radians();
        let p = path(g, a)?;
        let (cx, cy) = center(p);
        let (c, s) = (ang.cos(), ang.sin());
        p.points.iter_mut().for_each(|q| {
            let (dx, dy) = (q.0 - cx, q.1 - cy);
            q.0 = cx + dx * c + dy * s;
            q.1 = cy - dx * s + dy * c;
        });
        p.rebuild();
        zero()
    }));
    l.push(("path_scale", |g, _, _, a| {
        let (xs, ys) = (a.r(1)?, a.r(2)?);
        let p = path(g, a)?;
        let (cx, cy) = center(p);
        p.points.iter_mut().for_each(|q| {
            q.0 = cx + (q.0 - cx) * xs;
            q.1 = cy + (q.1 - cy) * ys;
        });
        p.rebuild();
        zero()
    }));
    l.push(("path_shift", |g, _, _, a| {
        let (xs, ys) = (a.r(1)?, a.r(2)?);
        let p = path(g, a)?;
        p.points.iter_mut().for_each(|q| {
            q.0 += xs;
            q.1 += ys;
        });
        p.rebuild();
        zero()
    }));
    // ---------------------------------------------------------------- timelines
    l.push(("timeline_add", |g, _, _, _| {
        r(alloc(&mut g.timelines, TimelineRes { name: "timeline".into(), moments: Default::default() }) as f64)
    }));
    l.push(("timeline_delete", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(s) = g.timelines.get_mut(i.max(0) as usize) {
            *s = None;
        }
        zero()
    }));
    l.push(("timeline_clear", |g, _, _, a| {
        get_mut(&mut g.timelines, a.i(0)?, "Time line")?.moments.clear();
        zero()
    }));
    l.push(("timeline_moment_clear", |g, _, _, a| {
        let st_ = a.i(1)?;
        get_mut(&mut g.timelines, a.i(0)?, "Time line")?.moments.remove(&st_);
        zero()
    }));
    l.push(("timeline_moment_add", |g, vm, _, a| {
        let (ti, step, code) = (a.i(0)?, a.i(1)?, a.s(2)?);
        let list = code_list(vm, g, &code, &format!("time line {} moment {}", ti, step))?;
        let t = get_mut(&mut g.timelines, ti, "Time line")?;
        let new = append_list(t.moments.get(&step), list);
        t.moments.insert(step, new);
        zero()
    }));
    // ---------------------------------------------------------------- objects
    fn obj<'a>(g: &'a mut Game, a: &A) -> Result<&'a mut ObjectRes, Error> {
        get_mut(&mut g.objects, a.i(0)?, "Object")
    }
    l.push(("object_set_sprite", |g, _, _, a| {
        let v = a.i(1)?;
        obj(g, a)?.sprite = v;
        zero()
    }));
    l.push(("object_set_solid", |g, _, _, a| {
        let v = a.b(1)?;
        obj(g, a)?.solid = v;
        zero()
    }));
    l.push(("object_set_visible", |g, _, _, a| {
        let v = a.b(1)?;
        obj(g, a)?.visible = v;
        zero()
    }));
    l.push(("object_set_depth", |g, _, _, a| {
        let v = a.i(1)?;
        obj(g, a)?.depth = v;
        zero()
    }));
    l.push(("object_set_persistent", |g, _, _, a| {
        let v = a.b(1)?;
        obj(g, a)?.persistent = v;
        zero()
    }));
    l.push(("object_set_mask", |g, _, _, a| {
        let v = a.i(1)?;
        obj(g, a)?.mask = v;
        zero()
    }));
    l.push(("object_set_parent", |g, _, _, a| {
        let v = a.i(1)?;
        obj(g, a)?.parent = if v < 0 { -100 } else { v };
        g.build_event_tables();
        zero()
    }));
    l.push(("object_add", |g, _, _, _| {
        let o = ObjectRes {
            name: format!("object{}", g.objects.len()),
            sprite: -1,
            solid: false,
            visible: true,
            depth: 0,
            persistent: false,
            parent: -100,
            mask: -1,
            events: Default::default(),
        };
        let i = alloc(&mut g.objects, o);
        g.build_event_tables();
        r(i as f64)
    }));
    l.push(("object_delete", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(s) = g.objects.get_mut(i.max(0) as usize) {
            *s = None;
        }
        g.build_event_tables();
        zero()
    }));
    l.push(("object_event_clear", |g, _, _, a| {
        let (t, n) = (a.i(1)?, a.i(2)?);
        obj(g, a)?.events.remove(&(t, n));
        g.build_event_tables();
        zero()
    }));
    l.push(("object_event_add", |g, vm, _, a| {
        let (oi, t, n, code) = (a.i(0)?, a.i(1)?, a.i(2)?, a.s(3)?);
        let list = code_list(vm, g, &code, &format!("object {} event {}/{}", oi, t, n))?;
        let o = get_mut(&mut g.objects, oi, "Object")?;
        let new = append_list(o.events.get(&(t, n)), list);
        o.events.insert((t, n), new);
        g.build_event_tables();
        zero()
    }));
    // ---------------------------------------------------------------- rooms
    fn room<'a>(g: &'a mut Game, a: &A) -> Result<&'a mut RoomRes, Error> {
        get_mut(&mut g.rooms, a.i(0)?, "Room")
    }
    l.push(("room_set_width", |g, _, _, a| {
        let v = a.i(1)?;
        room(g, a)?.data.width = v;
        zero()
    }));
    l.push(("room_set_height", |g, _, _, a| {
        let v = a.i(1)?;
        room(g, a)?.data.height = v;
        zero()
    }));
    l.push(("room_set_caption", |g, _, _, a| {
        let v = a.s(1)?;
        room(g, a)?.data.caption = v;
        zero()
    }));
    l.push(("room_set_persistent", |g, _, _, a| {
        let v = a.b(1)?;
        room(g, a)?.data.persistent = v;
        zero()
    }));
    l.push(("room_set_code", |g, vm, _, a| {
        let (ri, code) = (a.i(0)?, a.s(1)?);
        let prog = vm.compile(&format!("creation code of room {}", ri), code.as_bytes(), &*g).map_err(Error::from)?;
        let rm = get_mut(&mut g.rooms, ri, "Room")?;
        rm.data.creation_code = code;
        rm.creation = Some(prog);
        zero()
    }));
    l.push(("room_set_background_color", |g, _, _, a| {
        let (c, s) = (a.i(1)?, a.b(2)?);
        let rm = room(g, a)?;
        rm.data.background_color = (c & 0xFFFFFF) as u32;
        rm.data.draw_background_color = s;
        zero()
    }));
    l.push(("room_set_background", |g, _, _, a| {
        let b = a.i(1)?.clamp(0, 7) as usize;
        let layer = gm_format::RoomBackground {
            visible: a.b(2)?,
            is_foreground: a.b(3)?,
            background: a.i(4)?,
            x: a.i(5)?,
            y: a.i(6)?,
            tile_h: a.b(7)?,
            tile_v: a.b(8)?,
            hspeed: a.i(9)?,
            vspeed: a.i(10)?,
            stretch: false,
        };
        let rm = room(g, a)?;
        if rm.data.backgrounds.len() < 8 {
            rm.data.backgrounds.resize(8, Default::default());
        }
        rm.data.backgrounds[b] = layer;
        zero()
    }));
    l.push(("room_set_view", |g, _, _, a| {
        let v = a.i(1)?.clamp(0, 7) as usize;
        let view = gm_format::View {
            visible: a.b(2)?,
            view_x: a.i(3)?,
            view_y: a.i(4)?,
            view_w: a.i(5)?,
            view_h: a.i(6)?,
            port_x: a.i(7)?,
            port_y: a.i(8)?,
            port_w: a.i(9)?,
            port_h: a.i(10)?,
            h_border: a.i(11)?,
            v_border: a.i(12)?,
            h_speed: a.i(13)?,
            v_speed: a.i(14)?,
            follow_object: a.i(15)?,
        };
        let rm = room(g, a)?;
        if rm.data.views.len() < 8 {
            rm.data.views.resize(8, Default::default());
        }
        rm.data.views[v] = view;
        zero()
    }));
    l.push(("room_set_view_enabled", |g, _, _, a| {
        let v = a.b(1)?;
        room(g, a)?.data.views_enabled = v;
        zero()
    }));
    l.push(("room_add", |g, _, _, _| {
        let data = gm_format::Room {
            name: format!("room{}", g.rooms.len()),
            ..Default::default()
        };
        r(alloc(&mut g.rooms, RoomRes { data, creation: None, inst_creation: Default::default() }) as f64)
    }));
    l.push(("room_duplicate", |g, _, _, a| {
        let src = get(&g.rooms, a.i(0)?).cloned().ok_or_else(|| Error::new("Room does not exist"))?;
        r(alloc(&mut g.rooms, src) as f64)
    }));
    l.push(("room_assign", |g, _, _, a| {
        let src = get(&g.rooms, a.i(1)?).cloned().ok_or_else(|| Error::new("Room does not exist"))?;
        let d = room(g, a)?;
        let n = d.data.name.clone();
        *d = src;
        d.data.name = n;
        zero()
    }));
    l.push(("room_instance_add", |g, _, _, a| {
        let id = g.alloc_id();
        let inst = gm_format::Instance {
            x: a.i(1)?,
            y: a.i(2)?,
            object: a.i(3)?,
            id,
            creation_code: String::new(),
            locked: false,
        };
        room(g, a)?.data.instances.push(inst);
        r(id as f64)
    }));
    l.push(("room_instance_clear", |g, _, _, a| {
        room(g, a)?.data.instances.clear();
        zero()
    }));
    fn tile_add(g: &mut Game, a: &A) -> R {
        let id = g.next_tile_id;
        g.next_tile_id += 1;
        let t = gm_format::Tile {
            background: a.i(1)?,
            tile_x: a.i(2)?,
            tile_y: a.i(3)?,
            width: a.i(4)?,
            height: a.i(5)?,
            x: a.i(6)?,
            y: a.i(7)?,
            depth: a.i(8)?,
            id,
            locked: false,
        };
        room(g, a)?.data.tiles.push(t);
        r(id as f64)
    }
    l.push(("room_tile_add", |g, _, _, a| tile_add(g, a)));
    l.push(("room_tile_add_ext", |g, _, _, a| tile_add(g, a)));
    l.push(("room_tile_clear", |g, _, _, a| {
        room(g, a)?.data.tiles.clear();
        zero()
    }));
}
