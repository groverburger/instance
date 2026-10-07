//! Reading the GMX documents into the GM 8.1 model (code still in Studio syntax; see
//! [`crate::convert`] for the translation pass).

use crate::source::Source;
use crate::xml::{self, attr, attr_bool, attr_float, attr_int, boolean, child, children, elements, float, int, text, text_of};
use crate::Warning;
use gm_format::model::*;
use std::collections::HashMap;

/// Resource kinds of a GMX project, in GM 8.1 resource-tree order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Sprite,
    Sound,
    Background,
    Path,
    Script,
    Shader,
    Font,
    Timeline,
    Object,
    Room,
}

impl Kind {
    pub const ALL: [Kind; 10] = [
        Kind::Sprite,
        Kind::Sound,
        Kind::Background,
        Kind::Path,
        Kind::Script,
        Kind::Shader,
        Kind::Font,
        Kind::Timeline,
        Kind::Object,
        Kind::Room,
    ];
    /// (group tag, leaf tag) in project.gmx.
    pub fn tags(self) -> (&'static str, &'static str) {
        match self {
            Kind::Sprite => ("sprites", "sprite"),
            Kind::Sound => ("sounds", "sound"),
            Kind::Background => ("backgrounds", "background"),
            Kind::Path => ("paths", "path"),
            Kind::Script => ("scripts", "script"),
            Kind::Shader => ("shaders", "shader"),
            Kind::Font => ("fonts", "font"),
            Kind::Timeline => ("timelines", "timeline"),
            Kind::Object => ("objects", "object"),
            Kind::Room => ("rooms", "room"),
        }
    }
    pub fn tree_kind(self) -> Option<ResourceKind> {
        Some(match self {
            Kind::Sprite => ResourceKind::Sprite,
            Kind::Sound => ResourceKind::Sound,
            Kind::Background => ResourceKind::Background,
            Kind::Path => ResourceKind::Path,
            Kind::Script => ResourceKind::Script,
            Kind::Font => ResourceKind::Font,
            Kind::Timeline => ResourceKind::Timeline,
            Kind::Object => ResourceKind::Object,
            Kind::Room => ResourceKind::Room,
            Kind::Shader => return None,
        })
    }
}

/// A node of a project.gmx resource section.
#[derive(Debug, Clone)]
pub enum Node {
    Group(String, Vec<Node>),
    /// Path as written (e.g. `sprites\sPlayer`, `scripts\foo.gml`).
    Leaf(String),
}

/// Resource name from its project path.
pub fn leaf_name(path: &str) -> String {
    let last = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let last = last.strip_suffix(".gml").unwrap_or(last);
    let last = last.strip_suffix(".shader").unwrap_or(last);
    last.to_string()
}

fn parse_section(n: roxmltree::Node, kind: Kind) -> Vec<Node> {
    let (gt, lt) = kind.tags();
    let mut out = Vec::new();
    for c in elements(n) {
        let tag = c.tag_name().name();
        if tag == gt {
            out.push(Node::Group(attr(c, "name").to_string(), parse_section(c, kind)));
        } else if tag == lt {
            let t = text_of(c).trim().to_string();
            if !t.is_empty() {
                out.push(Node::Leaf(t));
            }
        }
    }
    out
}

pub fn flatten(nodes: &[Node], out: &mut Vec<String>) {
    for n in nodes {
        match n {
            Node::Group(_, c) => flatten(c, out),
            Node::Leaf(p) => out.push(p.clone()),
        }
    }
}

/// Everything read from the GMX files, before GML translation.
pub struct Loaded {
    pub project: Project,
    /// Resource tree sections per kind.
    pub tree: HashMap<Kind, Vec<Node>>,
    /// Resource name → index per kind (index in the 8.1 lists; shaders get their own).
    pub names: HashMap<Kind, HashMap<String, i32>>,
    /// Shader names (dropped).
    pub shaders: Vec<String>,
    /// Named room instances (`inst_XXXXXXXX`) → id.
    pub instance_names: HashMap<String, i32>,
    /// Studio-only object events: object index → [(event type, number, actions)].
    pub extra_events: Vec<(usize, i32, i32, Vec<Action>)>,
    /// Number of scripts each script file produced (`#define` splitting).
    pub split_counts: HashMap<String, usize>,
    pub warnings: Vec<Warning>,
}

pub struct Ctx<'a> {
    pub src: &'a Source,
    /// Directory of project.gmx inside the source ("" or "dir/").
    pub base: String,
    pub names: HashMap<Kind, HashMap<String, i32>>,
    pub warnings: Vec<Warning>,
}

impl Ctx<'_> {
    fn path(&self, rel: &str) -> String {
        format!("{}{}", self.base, rel.replace('\\', "/"))
    }
    fn read(&self, rel: &str) -> Option<Vec<u8>> {
        self.src.read(&self.path(rel))
    }
    fn read_string(&self, rel: &str) -> Option<String> {
        self.src.read_string(&self.path(rel))
    }
    fn warn(&mut self, ctx: impl Into<String>, msg: impl Into<String>) {
        self.warnings.push(Warning {
            context: ctx.into(),
            message: msg.into(),
        });
    }
    pub fn index(&self, kind: Kind, name: &str) -> i32 {
        if xml::is_none_name(name) {
            return -1;
        }
        self.names
            .get(&kind)
            .and_then(|m| m.get(name))
            .copied()
            .unwrap_or(-1)
    }
}

fn dir_of(rel: &str) -> &str {
    match rel.rfind(['\\', '/']) {
        Some(i) => &rel[..=i],
        None => "",
    }
}

/// Decode a PNG file into RGBA8.
pub fn decode_png(data: &[u8]) -> Result<RgbaImage, String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(data));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; r.output_buffer_size().ok_or("png too large")?];
    let info = r.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let (w, h) = (info.width, info.height);
    let px = &buf[..info.buffer_size()];
    let n = (w * h) as usize;
    let mut data = Vec::with_capacity(n * 4);
    match info.color_type {
        png::ColorType::Rgba => data.extend_from_slice(&px[..n * 4]),
        png::ColorType::Rgb => {
            for p in px.chunks_exact(3).take(n) {
                data.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for p in px.chunks_exact(2).take(n) {
                data.extend_from_slice(&[p[0], p[0], p[0], p[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for &g in px.iter().take(n) {
                data.extend_from_slice(&[g, g, g, 255]);
            }
        }
        png::ColorType::Indexed => return Err("unexpanded palette image".into()),
    }
    Ok(RgbaImage {
        width: w,
        height: h,
        data,
    })
}

fn load_image(ctx: &mut Ctx, rel: &str, what: &str) -> Option<RgbaImage> {
    let Some(bytes) = ctx.read(rel) else {
        ctx.warn(what, format!("missing image file {rel}"));
        return None;
    };
    match decode_png(&bytes) {
        Ok(i) => Some(i),
        Err(e) => {
            ctx.warn(what, format!("cannot decode image {rel}: {e}"));
            None
        }
    }
}

/// Fit an image to w×h (crop / pad with transparency) so all frames match.
fn fit(img: RgbaImage, w: u32, h: u32) -> RgbaImage {
    if img.width == w && img.height == h {
        return img;
    }
    let mut out = RgbaImage::new(w, h);
    for y in 0..h.min(img.height) {
        for x in 0..w.min(img.width) {
            let s = ((y * img.width + x) * 4) as usize;
            let d = ((y * w + x) * 4) as usize;
            out.data[d..d + 4].copy_from_slice(&img.data[s..s + 4]);
        }
    }
    out
}

fn load_sprite(ctx: &mut Ctx, rel: &str, name: &str) -> Sprite {
    let mut s = Sprite {
        name: name.to_string(),
        ..Sprite::default()
    };
    let file = format!("{rel}.sprite.gmx");
    let Some(text_src) = ctx.read_string(&file) else {
        ctx.warn(format!("sprite {name}"), format!("missing {file}"));
        return s;
    };
    let doc = match xml::parse(&text_src) {
        Ok(d) => d,
        Err(e) => {
            ctx.warn(format!("sprite {name}"), format!("bad XML: {e}"));
            return s;
        }
    };
    let r = doc.root_element();
    let ty = int(r, "type", 0);
    if ty != 0 {
        ctx.warn(
            format!("sprite {name}"),
            format!("sprite type {ty} (vector/skeletal) is not supported; left empty"),
        );
    }
    s.origin_x = int(r, "xorig", 0);
    s.origin_y = int(r, "yorigin", 0);
    s.collision_shape = int(r, "colkind", 1).clamp(0, 3);
    s.alpha_tolerance = int(r, "coltolerance", 0);
    s.separate_collision_masks = boolean(r, "sepmasks", false);
    s.bbox_mode = int(r, "bboxmode", 0).clamp(0, 2);
    s.bbox_left = int(r, "bbox_left", 0);
    s.bbox_right = int(r, "bbox_right", 0);
    s.bbox_top = int(r, "bbox_top", 0);
    s.bbox_bottom = int(r, "bbox_bottom", 0);
    let w = int(r, "width", 0).max(0) as u32;
    let h = int(r, "height", 0).max(0) as u32;
    let dir = dir_of(rel).to_string();
    if let Some(frames) = child(r, "frames") {
        let mut fl: Vec<(i32, String)> = children(frames, "frame")
            .enumerate()
            .map(|(i, f)| (xml::attr_int(f, "index", i as i32), text_of(f)))
            .collect();
        fl.sort_by_key(|f| f.0);
        for (_, f) in fl {
            if let Some(img) = load_image(ctx, &format!("{dir}{f}"), &format!("sprite {name}")) {
                let (fw, fh) = if w > 0 && h > 0 { (w, h) } else { (img.width, img.height) };
                s.frames.push(fit(img, fw, fh));
            }
        }
    }
    s
}

fn load_background(ctx: &mut Ctx, rel: &str, name: &str) -> Background {
    let mut b = Background {
        name: name.to_string(),
        ..Background::default()
    };
    let file = format!("{rel}.background.gmx");
    let Some(src) = ctx.read_string(&file) else {
        ctx.warn(format!("background {name}"), format!("missing {file}"));
        return b;
    };
    let Ok(doc) = xml::parse(&src) else {
        ctx.warn(format!("background {name}"), "bad XML");
        return b;
    };
    let r = doc.root_element();
    b.use_as_tileset = boolean(r, "istileset", false);
    b.tile_width = int(r, "tilewidth", 16);
    b.tile_height = int(r, "tileheight", 16);
    b.tile_h_offset = int(r, "tilexoff", 0);
    b.tile_v_offset = int(r, "tileyoff", 0);
    b.tile_h_sep = int(r, "tilehsep", 0);
    b.tile_v_sep = int(r, "tilevsep", 0);
    let data = text(r, "data");
    if !data.trim().is_empty() {
        let dir = dir_of(rel).to_string();
        if let Some(img) = load_image(ctx, &format!("{dir}{}", data.trim()), &format!("background {name}")) {
            b.image = img;
        }
    }
    b
}

fn load_sound(ctx: &mut Ctx, rel: &str, name: &str) -> Sound {
    let mut s = Sound {
        name: name.to_string(),
        volume: 1.0,
        preload: true,
        ..Sound::default()
    };
    let file = format!("{rel}.sound.gmx");
    let Some(src) = ctx.read_string(&file) else {
        ctx.warn(format!("sound {name}"), format!("missing {file}"));
        return s;
    };
    let Ok(doc) = xml::parse(&src) else {
        ctx.warn(format!("sound {name}"), "bad XML");
        return s;
    };
    let r = doc.root_element();
    s.kind = int(r, "kind", 0).clamp(0, 3);
    s.file_type = text(r, "extension");
    s.effects = int(r, "effects", 0);
    s.volume = child(r, "volume")
        .map(|v| xml::float(v, "volume", 1.0))
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    s.pan = float(r, "pan", 0.0);
    s.preload = boolean(r, "preload", true);
    let group = int(r, "audioGroup", 0);
    if group != 0 {
        ctx.warn(format!("sound {name}"), format!("audio group {group} ignored (GM 8.1 has no audio groups)"));
    }
    let data = text(r, "data");
    let data = data.trim();
    if !data.is_empty() {
        let dir = dir_of(rel).to_string();
        let path = format!("{dir}audio/{data}");
        match ctx.read(&path) {
            Some(b) => {
                s.file_name = data.to_string();
                if s.file_type.is_empty() {
                    s.file_type = data.rfind('.').map(|i| data[i..].to_string()).unwrap_or_default();
                }
                s.data = Some(b);
            }
            None => ctx.warn(format!("sound {name}"), format!("missing audio file {path}")),
        }
    }
    s
}

fn load_font(ctx: &mut Ctx, rel: &str, name: &str) -> Font {
    let mut f = Font {
        name: name.to_string(),
        ..Font::default()
    };
    let file = format!("{rel}.font.gmx");
    let Some(src) = ctx.read_string(&file) else {
        ctx.warn(format!("font {name}"), format!("missing {file}"));
        return f;
    };
    let Ok(doc) = xml::parse(&src) else {
        ctx.warn(format!("font {name}"), "bad XML");
        return f;
    };
    let r = doc.root_element();
    let fname = text(r, "name");
    // '@' prefix selects the vertical variant of a CJK font in Windows.
    f.font_name = fname.trim().trim_start_matches('@').to_string();
    f.size = int(r, "size", 12);
    f.bold = boolean(r, "bold", false);
    f.italic = boolean(r, "italic", false);
    f.charset = int(r, "charset", 0).clamp(0, 255) as u8;
    f.antialias = int(r, "aa", 0).clamp(0, 3) as u8;
    // Ranges: <ranges><range0>32,127</range0>...</ranges>; older files have <first>/<last>.
    let mut lo = i32::MAX;
    let mut hi = i32::MIN;
    if let Some(rs) = child(r, "ranges") {
        for rg in elements(rs) {
            let t = text_of(rg);
            if let Some((a, b)) = t.split_once(',') {
                if let (Ok(a), Ok(b)) = (a.trim().parse::<i32>(), b.trim().parse::<i32>()) {
                    lo = lo.min(a);
                    hi = hi.max(b);
                }
            }
        }
    }
    if lo == i32::MAX {
        lo = int(r, "first", 32);
        hi = int(r, "last", 127);
    }
    f.range_start = lo.clamp(0, 65535) as u16;
    f.range_end = hi.max(lo);
    if f.range_end > 255 {
        ctx.warn(format!("font {name}"), format!("character range {lo}..{hi} clipped to 255"));
        f.range_end = 255;
    }
    if boolean(r, "includeTTF", false) {
        ctx.warn(format!("font {name}"), "embedded TTF not supported; the font is regenerated from its name");
    }
    f
}

fn load_path(ctx: &mut Ctx, rel: &str, name: &str) -> Path {
    let mut p = Path {
        name: name.to_string(),
        ..Path::default()
    };
    let file = format!("{rel}.path.gmx");
    let Some(src) = ctx.read_string(&file) else {
        ctx.warn(format!("path {name}"), format!("missing {file}"));
        return p;
    };
    let Ok(doc) = xml::parse(&src) else {
        ctx.warn(format!("path {name}"), "bad XML");
        return p;
    };
    let r = doc.root_element();
    p.connection_kind = int(r, "kind", 0);
    p.closed = boolean(r, "closed", true);
    p.precision = int(r, "precision", 4);
    p.backdrop_room = ctx.index(Kind::Room, &text(r, "backroom"));
    p.snap_x = int(r, "hsnap", 16);
    p.snap_y = int(r, "vsnap", 16);
    if let Some(pts) = child(r, "points") {
        for pt in children(pts, "point") {
            let v: Vec<f64> = text_of(pt).split(',').filter_map(|s| s.trim().parse().ok()).collect();
            if v.len() >= 2 {
                p.points.push(PathPoint {
                    x: v[0],
                    y: v[1],
                    speed: v.get(2).copied().unwrap_or(100.0),
                });
            }
        }
    }
    p
}

/// Parse an `<action>` element.
pub fn parse_action(ctx: &mut Ctx, a: roxmltree::Node, owner: &str) -> Action {
    let mut act = Action {
        library_id: int(a, "libid", 1),
        action_id: int(a, "id", 0),
        kind: int(a, "kind", 0),
        may_be_relative: boolean(a, "userelative", false),
        is_question: boolean(a, "isquestion", false),
        has_applies_to: boolean(a, "useapplyto", false),
        exe_type: int(a, "exetype", 0),
        function_name: text(a, "functionname"),
        code: text(a, "codestring"),
        relative: boolean(a, "relative", false),
        negate: boolean(a, "isnot", false),
        applies_to: APPLIES_TO_SELF,
        ..Action::default()
    };
    let who = text(a, "whoName");
    act.applies_to = match who.trim() {
        "" | "self" => APPLIES_TO_SELF,
        "other" => APPLIES_TO_OTHER,
        n => {
            let i = ctx.index(Kind::Object, n);
            if i < 0 {
                ctx.warn(owner, format!("action applies to unknown object '{n}'; using self"));
                APPLIES_TO_SELF
            } else {
                i
            }
        }
    };
    if let Some(args) = child(a, "arguments") {
        for arg in children(args, "argument") {
            let k = int(arg, "kind", 0);
            let val_node = elements(arg).find(|c| c.tag_name().name() != "kind");
            let tag = val_node.map(|v| v.tag_name().name()).unwrap_or("string");
            let raw = val_node.map(text_of).unwrap_or_default();
            let res_kind = match (k, tag) {
                (arg_kind::SPRITE, _) | (_, "sprite") => Some(Kind::Sprite),
                (arg_kind::SOUND, _) | (_, "sound") => Some(Kind::Sound),
                (arg_kind::BACKGROUND, _) | (_, "background") => Some(Kind::Background),
                (arg_kind::PATH, _) | (_, "path") => Some(Kind::Path),
                (arg_kind::SCRIPT, _) | (_, "script") => Some(Kind::Script),
                (arg_kind::OBJECT, _) | (_, "object") => Some(Kind::Object),
                (arg_kind::ROOM, _) | (_, "room") => Some(Kind::Room),
                (arg_kind::FONT, _) | (_, "font") => Some(Kind::Font),
                (arg_kind::TIMELINE, _) | (_, "timeline") => Some(Kind::Timeline),
                _ => None,
            };
            let v = match res_kind {
                Some(rk) => {
                    let i = ctx.index(rk, raw.trim());
                    if i < 0 && !xml::is_none_name(&raw) {
                        ctx.warn(owner, format!("action argument names unknown resource '{}'", raw.trim()));
                    }
                    i.to_string()
                }
                None => raw,
            };
            act.arg_kinds.push(k);
            act.args.push(v);
        }
    }
    act.argument_count = act.args.len() as i32;
    while act.args.len() < 8 {
        act.args.push("0".into());
        act.arg_kinds.push(0);
    }
    act
}

fn load_object(ctx: &mut Ctx, rel: &str, name: &str, index: usize, extra: &mut Vec<(usize, i32, i32, Vec<Action>)>) -> Object {
    let mut o = Object {
        name: name.to_string(),
        ..Object::default()
    };
    let file = format!("{rel}.object.gmx");
    let Some(src) = ctx.read_string(&file) else {
        ctx.warn(format!("object {name}"), format!("missing {file}"));
        return o;
    };
    let doc = match xml::parse(&src) {
        Ok(d) => d,
        Err(e) => {
            ctx.warn(format!("object {name}"), format!("bad XML: {e}"));
            return o;
        }
    };
    let r = doc.root_element();
    let owner = format!("object {name}");
    let sprite = text(r, "spriteName");
    o.sprite = ctx.index(Kind::Sprite, &sprite);
    o.solid = boolean(r, "solid", false);
    o.visible = boolean(r, "visible", true);
    o.depth = float(r, "depth", 0.0) as i32;
    o.persistent = boolean(r, "persistent", false);
    let parent = text(r, "parentName");
    o.parent = if xml::is_none_name(&parent) {
        NO_PARENT
    } else {
        let i = ctx.index(Kind::Object, &parent);
        if i < 0 {
            ctx.warn(&owner, format!("unknown parent '{parent}'"));
            NO_PARENT
        } else {
            i
        }
    };
    o.mask_sprite = ctx.index(Kind::Sprite, &text(r, "maskName"));
    if boolean(r, "PhysicsObject", false) {
        ctx.warn(&owner, "uses Studio physics; physics properties are dropped");
    }
    if let Some(evs) = child(r, "events") {
        for ev in children(evs, "event") {
            let et = attr_int(ev, "eventtype", -1);
            let mut num = attr_int(ev, "enumb", 0);
            if et == event_type::COLLISION {
                let other = attr(ev, "ename");
                num = ctx.index(Kind::Object, other);
                if num < 0 {
                    ctx.warn(&owner, format!("collision with unknown object '{other}' dropped"));
                    continue;
                }
            }
            let actions: Vec<Action> = children(ev, "action").map(|a| parse_action(ctx, a, &owner)).collect();
            let studio_only = match et {
                event_type::DRAW => num != 0,
                event_type::OTHER => num >= 58 || (26..40).contains(&num),
                0..=11 => false,
                _ => true,
            };
            if studio_only {
                extra.push((index, et, num, actions));
            } else {
                o.events.push(Event {
                    event_type: et,
                    number: num,
                    actions,
                });
            }
        }
    }
    o.events.sort_by_key(|e| e.event_type);
    o
}

fn load_timeline(ctx: &mut Ctx, rel: &str, name: &str) -> Timeline {
    let mut t = Timeline {
        name: name.to_string(),
        ..Timeline::default()
    };
    let file = format!("{rel}.timeline.gmx");
    let Some(src) = ctx.read_string(&file) else {
        ctx.warn(format!("timeline {name}"), format!("missing {file}"));
        return t;
    };
    let Ok(doc) = xml::parse(&src) else {
        ctx.warn(format!("timeline {name}"), "bad XML");
        return t;
    };
    let r = doc.root_element();
    let owner = format!("timeline {name}");
    for e in children(r, "entry") {
        let step = int(e, "step", 0);
        let mut actions = Vec::new();
        for ev in children(e, "event") {
            for a in children(ev, "action") {
                actions.push(parse_action(ctx, a, &owner));
            }
        }
        t.moments.push(Moment { step, actions });
    }
    t.moments.sort_by_key(|m| m.step);
    t
}

/// Instance colour attribute (ABGR as unsigned) → (blend BGR, alpha 0..1).
fn split_colour(v: f64) -> (u32, f64) {
    let c = v as i64 as u32;
    (c & 0xFFFFFF, ((c >> 24) & 0xFF) as f64 / 255.0)
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn load_room(
    ctx: &mut Ctx,
    rel: &str,
    name: &str,
    next_inst: &mut i32,
    next_tile: &mut i32,
    inst_names: &mut HashMap<String, i32>,
) -> Room {
    let mut room = Room {
        name: name.to_string(),
        ..Room::default()
    };
    let file = format!("{rel}.room.gmx");
    let Some(src) = ctx.read_string(&file) else {
        ctx.warn(format!("room {name}"), format!("missing {file}"));
        return room;
    };
    let doc = match xml::parse(&src) {
        Ok(d) => d,
        Err(e) => {
            ctx.warn(format!("room {name}"), format!("bad XML: {e}"));
            return room;
        }
    };
    let r = doc.root_element();
    let owner = format!("room {name}");
    room.caption = text(r, "caption");
    room.width = int(r, "width", 640);
    room.height = int(r, "height", 480);
    room.snap_x = int(r, "hsnap", 16);
    room.snap_y = int(r, "vsnap", 16);
    room.isometric_grid = boolean(r, "isometric", false);
    room.speed = int(r, "speed", 30);
    room.persistent = boolean(r, "persistent", false);
    room.background_color = (float(r, "colour", 0.0) as i64 as u32) & 0xFFFFFF;
    room.draw_background_color = boolean(r, "showcolour", true);
    room.creation_code = text(r, "code");
    room.views_enabled = boolean(r, "enableViews", false);
    if let Some(ms) = child(r, "makerSettings") {
        room.editor = RoomEditorState {
            remember_room_editor_info: boolean(ms, "isSet", false) as i32,
            editor_width: int(ms, "w", 0),
            editor_height: int(ms, "h", 0),
            show_grid: boolean(ms, "showGrid", true) as i32,
            show_objects: boolean(ms, "showObjects", true) as i32,
            show_tiles: boolean(ms, "showTiles", true) as i32,
            show_backgrounds: boolean(ms, "showBackgrounds", true) as i32,
            show_foregrounds: boolean(ms, "showForegrounds", true) as i32,
            show_views: boolean(ms, "showViews", false) as i32,
            delete_underlying_objects: boolean(ms, "deleteUnderlyingObj", false) as i32,
            delete_underlying_tiles: boolean(ms, "deleteUnderlyingTiles", true) as i32,
            current_tab: int(ms, "page", 0),
            h_scroll: int(ms, "xoffset", 0),
            v_scroll: int(ms, "yoffset", 0),
        };
    }
    if let Some(bgs) = child(r, "backgrounds") {
        room.backgrounds = children(bgs, "background")
            .take(8)
            .map(|b| RoomBackground {
                visible: attr_bool(b, "visible", false),
                is_foreground: attr_bool(b, "foreground", false),
                background: ctx.index(Kind::Background, attr(b, "name")),
                x: attr_int(b, "x", 0),
                y: attr_int(b, "y", 0),
                tile_h: attr_bool(b, "htiled", true),
                tile_v: attr_bool(b, "vtiled", true),
                hspeed: attr_int(b, "hspeed", 0),
                vspeed: attr_int(b, "vspeed", 0),
                stretch: attr_bool(b, "stretch", false),
            })
            .collect();
        room.backgrounds.resize(8, RoomBackground::default());
    }
    if let Some(vs) = child(r, "views") {
        room.views = children(vs, "view")
            .take(8)
            .map(|v| View {
                visible: attr_bool(v, "visible", false),
                view_x: attr_int(v, "xview", 0),
                view_y: attr_int(v, "yview", 0),
                view_w: attr_int(v, "wview", 640),
                view_h: attr_int(v, "hview", 480),
                port_x: attr_int(v, "xport", 0),
                port_y: attr_int(v, "yport", 0),
                port_w: attr_int(v, "wport", 640),
                port_h: attr_int(v, "hport", 480),
                h_border: attr_int(v, "hborder", 32),
                v_border: attr_int(v, "vborder", 32),
                h_speed: attr_int(v, "hspeed", -1),
                v_speed: attr_int(v, "vspeed", -1),
                follow_object: ctx.index(Kind::Object, attr(v, "objName")),
            })
            .collect();
        room.views.resize(8, View::default());
    }
    if let Some(is) = child(r, "instances") {
        for i in children(is, "instance") {
            let oname = attr(i, "objName");
            let obj = ctx.index(Kind::Object, oname);
            if obj < 0 {
                ctx.warn(&owner, format!("instance of unknown object '{oname}' dropped"));
                continue;
            }
            *next_inst += 1;
            let id = *next_inst;
            let iname = attr(i, "name");
            if !iname.is_empty() {
                inst_names.insert(iname.to_string(), id);
            }
            // Studio per-instance transform → creation-code prefix.
            let mut pre = String::new();
            let sx = attr_float(i, "scaleX", 1.0);
            let sy = attr_float(i, "scaleY", 1.0);
            let rot = attr_float(i, "rotation", 0.0);
            let col = attr_float(i, "colour", 4294967295.0);
            if sx != 1.0 {
                pre.push_str(&format!("image_xscale = {};", fmt_num(sx)));
            }
            if sy != 1.0 {
                pre.push_str(&format!("image_yscale = {};", fmt_num(sy)));
            }
            if rot != 0.0 {
                pre.push_str(&format!("image_angle = {};", fmt_num(rot)));
            }
            let (blend, alpha) = split_colour(col);
            if blend != 0xFFFFFF {
                pre.push_str(&format!("image_blend = {blend};"));
            }
            if alpha < 1.0 {
                pre.push_str(&format!("image_alpha = {};", fmt_num((alpha * 1000.0).round() / 1000.0)));
            }
            let code = attr(i, "code").to_string();
            room.instances.push(Instance {
                x: attr_float(i, "x", 0.0).round() as i32,
                y: attr_float(i, "y", 0.0).round() as i32,
                object: obj,
                id,
                // The prefix is split off again by the convert pass (marker line).
                creation_code: if pre.is_empty() {
                    code
                } else {
                    format!("{}{pre}\n{code}", crate::convert::TRANSFORM_MARK)
                },
                locked: attr_bool(i, "locked", false),
            });
        }
    }
    if let Some(ts) = child(r, "tiles") {
        let mut tile_code = String::new();
        for t in children(ts, "tile") {
            let bg = ctx.index(Kind::Background, attr(t, "bgName"));
            *next_tile += 1;
            let id = *next_tile;
            let sx = attr_float(t, "scaleX", 1.0);
            let sy = attr_float(t, "scaleY", 1.0);
            let (blend, alpha) = split_colour(attr_float(t, "colour", 4294967295.0));
            if sx != 1.0 || sy != 1.0 {
                tile_code.push_str(&format!("tile_set_scale({id}, {}, {});\n", fmt_num(sx), fmt_num(sy)));
            }
            if blend != 0xFFFFFF {
                tile_code.push_str(&format!("tile_set_blend({id}, {blend});\n"));
            }
            if alpha < 1.0 {
                tile_code.push_str(&format!("tile_set_alpha({id}, {});\n", fmt_num(alpha)));
            }
            room.tiles.push(Tile {
                x: attr_float(t, "x", 0.0).round() as i32,
                y: attr_float(t, "y", 0.0).round() as i32,
                background: bg,
                tile_x: attr_int(t, "xo", 0),
                tile_y: attr_int(t, "yo", 0),
                width: attr_int(t, "w", 16),
                height: attr_int(t, "h", 16),
                depth: attr_int(t, "depth", 1000000),
                id,
                locked: attr_bool(t, "locked", false),
            });
        }
        if !tile_code.is_empty() {
            room.creation_code = format!(
                "{}// tile scale/colour (gmx2gm81)\n{tile_code}\n{}",
                crate::convert::TRANSFORM_MARK,
                room.creation_code
            );
        }
    }
    if boolean(r, "PhysicsWorld", false) {
        ctx.warn(&owner, "physics world settings dropped (GM 8.1 has no physics)");
    }
    room
}

fn parse_colour_name(s: &str) -> u32 {
    let s = s.trim();
    let named = [
        ("clBlack", 0x000000),
        ("clWhite", 0xFFFFFF),
        ("clMaroon", 0x000080),
        ("clGreen", 0x008000),
        ("clOlive", 0x008080),
        ("clNavy", 0x800000),
        ("clPurple", 0x800080),
        ("clTeal", 0x808000),
        ("clGray", 0x808080),
        ("clSilver", 0xC0C0C0),
        ("clRed", 0x0000FF),
        ("clLime", 0x00FF00),
        ("clYellow", 0x00FFFF),
        ("clBlue", 0xFF0000),
        ("clFuchsia", 0xFF00FF),
        ("clAqua", 0xFFFF00),
    ];
    if let Some((_, v)) = named.iter().find(|(n, _)| n.eq_ignore_ascii_case(s)) {
        return *v;
    }
    if let Some(h) = s.strip_prefix('#') {
        // #RRGGBB → BGR
        if let Ok(v) = u32::from_str_radix(h, 16) {
            return ((v & 0xFF) << 16) | (v & 0xFF00) | ((v >> 16) & 0xFF);
        }
    }
    xml::parse_num(s).map(|v| v as i64 as u32).unwrap_or(0)
}

fn parse_guid(s: &str) -> [u8; 16] {
    let hex: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let mut g = [0u8; 16];
    if hex.len() == 32 {
        for (i, b) in g.iter_mut().enumerate() {
            *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0);
        }
    }
    g
}

fn load_config(ctx: &mut Ctx, rel: &str, p: &mut Project) {
    let file = format!("{}.config.gmx", rel.replace('\\', "/"));
    let Some(src) = ctx.read_string(&file) else {
        ctx.warn("settings", format!("missing {file}; using defaults"));
        return;
    };
    let Ok(doc) = xml::parse(&src) else {
        ctx.warn("settings", "bad config XML");
        return;
    };
    let r = doc.root_element();
    let Some(o) = child(r, "Options") else { return };
    let s = &mut p.settings;
    let b = |n: &str, d: bool| boolean(o, n, d);
    s.start_fullscreen = b("option_fullscreen", false);
    s.interpolate_pixels = b("option_interpolate", false);
    s.dont_draw_border = b("option_noborder", false);
    s.display_cursor = b("option_showcursor", true);
    s.scaling = int(o, "option_scale", -1);
    s.allow_window_resize = b("option_sizeable", false);
    s.always_on_top = b("option_stayontop", false);
    s.color_outside_room = parse_colour_name(&text(o, "option_windowcolor"));
    s.set_resolution = b("option_changeresolution", false);
    s.color_depth = int(o, "option_colordepth", 0);
    s.resolution = int(o, "option_resolution", 0);
    s.frequency = int(o, "option_frequency", 0);
    s.dont_show_buttons = b("option_nobuttons", false);
    s.vsync = int(o, "option_sync_vertex", 0);
    s.disable_screensaver = b("option_noscreensaver", true);
    s.let_f4_fullscreen = b("option_screenkey", true);
    s.let_f1_game_info = b("option_helpkey", true);
    s.let_esc_end_game = b("option_quitkey", true);
    s.let_f5_save_f6_load = b("option_savekey", true);
    s.let_f9_screenshot = b("option_screenshotkey", true);
    s.treat_close_as_esc = b("option_closeesc", true);
    s.process_priority = int(o, "option_priority", 0);
    s.freeze_on_lose_focus = b("option_freeze", false);
    s.loading_bar = int(o, "option_showprogress", 1).clamp(0, 2);
    if s.loading_bar == 2 {
        ctx.warn("settings", "custom loading bar images not converted; using the default bar");
        s.loading_bar = 1;
    }
    if b("option_hasloadimage", false) {
        ctx.warn("settings", "custom loading image not converted");
    }
    s.load_image_transparent = b("option_loadtransparent", false);
    s.load_image_alpha = int(o, "option_loadalpha", 255);
    s.scale_progress_bar = b("option_scaleprogress", true);
    s.display_errors = b("option_displayerrors", true);
    s.write_errors_to_log = b("option_writeerrors", false);
    s.abort_on_all_errors = b("option_aborterrors", false);
    s.treat_uninitialized_as_zero = b("option_variableerrors", false);
    s.uninitialized_extra_bits = if b("option_argumenterrors", false) { 2 } else { 0 };
    s.author = text(o, "option_author");
    s.version_text = text(o, "option_version");
    s.information = text(o, "option_information");
    s.version_major = int(o, "option_version_major", 1);
    s.version_minor = int(o, "option_version_minor", 0);
    s.version_release = int(o, "option_version_release", 0);
    s.version_build = int(o, "option_version_build", 0);
    s.company = text(o, "option_version_company");
    s.product = text(o, "option_version_product");
    s.copyright = text(o, "option_version_copyright");
    s.description = text(o, "option_version_description");
    let icon = text(o, "option_windows_game_icon");
    if !icon.trim().is_empty() {
        if let Some(ico) = ctx.read(icon.trim()) {
            s.icon = ico;
        }
    }
    p.game_id = int(o, "option_gameid", 0);
    p.guid = parse_guid(&text(o, "option_gameguid"));
    if let Some(cc) = child(r, "ConfigConstants").and_then(|c| child(c, "constants")) {
        for c in children(cc, "constant") {
            p.constants.push(Constant {
                name: attr(c, "name").to_string(),
                value: text_of(c),
            });
        }
    }
}

fn load_datafiles(ctx: &mut Ctx, n: roxmltree::Node, folder: &str, out: &mut Vec<IncludedFile>) {
    for c in elements(n) {
        match c.tag_name().name() {
            "datafiles" => {
                let sub = format!("{folder}{}/", attr(c, "name"));
                load_datafiles(ctx, c, &sub, out);
            }
            "datafile" => {
                let fname = text(c, "filename");
                let fname = fname.trim();
                if fname.is_empty() {
                    continue;
                }
                let path = format!("datafiles/{folder}{fname}");
                let data = ctx.read(&path).or_else(|| ctx.read(&format!("datafiles/{fname}")));
                if data.is_none() {
                    ctx.warn("included files", format!("missing data file {path}"));
                }
                let export = int(c, "exportAction", 2);
                out.push(IncludedFile {
                    last_changed: 0.0,
                    file_name: fname.to_string(),
                    original_path: fname.to_string(),
                    original_file_chosen: true,
                    original_size: data.as_ref().map_or(0, |d| d.len() as i32),
                    data,
                    // Studio data files are always readable from the working directory.
                    export_kind: if export == 0 { 2 } else { export.clamp(1, 3) },
                    export_folder: text(c, "exportDir"),
                    overwrite_existing: boolean(c, "overwrite", true),
                    free_memory_after_export: boolean(c, "freeData", true),
                    remove_at_game_end: boolean(c, "removeEnd", false),
                });
            }
            _ => {}
        }
    }
}

/// Extensions: GML function files become scripts; DLLs and others are reported.
fn load_extensions(ctx: &mut Ctx, n: roxmltree::Node, scripts: &mut Vec<(String, String)>) {
    for e in children(n, "extension") {
        let path = text_of(e).trim().to_string();
        let file = format!("{path}.extension.gmx");
        let Some(src) = ctx.read_string(&file) else {
            ctx.warn(format!("extension {path}"), "missing extension file");
            continue;
        };
        let Ok(doc) = xml::parse(&src) else { continue };
        let r = doc.root_element();
        let ename = text(r, "name");
        let Some(files) = child(r, "files") else { continue };
        for f in children(files, "file") {
            let fname = text(f, "filename");
            let kind = int(f, "kind", 0);
            if kind == 2 || fname.to_ascii_lowercase().ends_with(".gml") {
                match ctx.read(&format!("{path}/{fname}")) {
                    Some(b) => {
                        let code = crate::source::decode_text(&b);
                        for (sn, sc) in crate::gml::split_defines(&code, &leaf_name(&fname).replace('.', "_")) {
                            scripts.push((sn, sc));
                        }
                    }
                    None => ctx.warn(format!("extension {ename}"), format!("missing GML file {fname}")),
                }
            } else {
                let funcs: Vec<String> = child(f, "functions")
                    .map(|fs| children(fs, "function").map(|x| text(x, "name")).collect())
                    .unwrap_or_default();
                ctx.warn(
                    format!("extension {ename}"),
                    format!("native file {fname} not supported (functions: {})", funcs.join(", ")),
                );
            }
        }
    }
}

/// Read the whole project (`proj_rel` = path of the .project.gmx inside `src`).
pub fn load(src: &Source, proj_rel: &str) -> Result<Loaded, String> {
    let base = dir_of(&proj_rel.replace('\\', "/")).to_string();
    let text_src = src
        .read_string(proj_rel)
        .ok_or_else(|| format!("cannot read {proj_rel}"))?;
    let doc = xml::parse(&text_src).map_err(|e| format!("{proj_rel}: {e}"))?;
    let root = doc.root_element();
    if root.tag_name().name() != "assets" {
        return Err(format!("{proj_rel}: not a GameMaker: Studio project (root <{}>)", root.tag_name().name()));
    }
    let mut ctx = Ctx {
        src,
        base,
        names: HashMap::new(),
        warnings: Vec::new(),
    };
    // Sections.
    let mut tree: HashMap<Kind, Vec<Node>> = HashMap::new();
    for k in Kind::ALL {
        let (gt, _) = k.tags();
        let mut nodes = Vec::new();
        for sec in children(root, gt) {
            nodes.extend(parse_section(sec, k));
        }
        tree.insert(k, nodes);
    }
    // Script files may hold several scripts (#define); split now so the names exist.
    let mut script_list: Vec<(String, String, String)> = Vec::new(); // (leaf path, name, code)
    let mut script_paths = Vec::new();
    let mut split_counts = HashMap::new();
    flatten(&tree[&Kind::Script], &mut script_paths);
    for p in &script_paths {
        let name = leaf_name(p);
        let code = match ctx.read_string(p) {
            Some(c) => c,
            None => {
                ctx.warn(format!("script {name}"), format!("missing {p}"));
                String::new()
            }
        };
        let parts = crate::gml::split_defines(&code, &name);
        split_counts.insert(p.clone(), parts.len());
        for (n, c) in parts {
            script_list.push((p.clone(), n, c));
        }
    }
    let mut ext_scripts = Vec::new();
    if let Some(ne) = child(root, "NewExtensions") {
        load_extensions(&mut ctx, ne, &mut ext_scripts);
    }
    // Indices.
    let mut names: HashMap<Kind, HashMap<String, i32>> = HashMap::new();
    for k in Kind::ALL {
        let mut m = HashMap::new();
        if k == Kind::Script {
            for (i, (_, n, _)) in script_list.iter().enumerate() {
                m.entry(n.clone()).or_insert(i as i32);
            }
            let base_n = script_list.len();
            for (i, (n, _)) in ext_scripts.iter().enumerate() {
                m.entry(n.clone()).or_insert((base_n + i) as i32);
            }
        } else {
            let mut paths = Vec::new();
            flatten(&tree[&k], &mut paths);
            for (i, p) in paths.iter().enumerate() {
                m.entry(leaf_name(p)).or_insert(i as i32);
            }
        }
        names.insert(k, m);
    }
    ctx.names = names;

    let mut p = Project::default();
    let leaves = |k: Kind| {
        let mut v = Vec::new();
        flatten(&tree[&k], &mut v);
        v
    };
    for path in leaves(Kind::Sprite) {
        let s = load_sprite(&mut ctx, &path, &leaf_name(&path));
        p.sprites.push(Some(s));
    }
    for path in leaves(Kind::Sound) {
        let s = load_sound(&mut ctx, &path, &leaf_name(&path));
        p.sounds.push(Some(s));
    }
    for path in leaves(Kind::Background) {
        let b = load_background(&mut ctx, &path, &leaf_name(&path));
        p.backgrounds.push(Some(b));
    }
    for path in leaves(Kind::Path) {
        let x = load_path(&mut ctx, &path, &leaf_name(&path));
        p.paths.push(Some(x));
    }
    for (_, n, c) in &script_list {
        p.scripts.push(Some(Script {
            name: n.clone(),
            last_changed: 0.0,
            code: c.clone(),
        }));
    }
    for (n, c) in &ext_scripts {
        p.scripts.push(Some(Script {
            name: n.clone(),
            last_changed: 0.0,
            code: c.clone(),
        }));
    }
    let shaders: Vec<String> = leaves(Kind::Shader).iter().map(|s| leaf_name(s)).collect();
    if !shaders.is_empty() {
        ctx.warn(
            "shaders",
            format!("{} shader(s) dropped (GM 8.1 has no shaders): {}", shaders.len(), shaders.join(", ")),
        );
    }
    for path in leaves(Kind::Font) {
        let f = load_font(&mut ctx, &path, &leaf_name(&path));
        p.fonts.push(Some(f));
    }
    for path in leaves(Kind::Timeline) {
        let t = load_timeline(&mut ctx, &path, &leaf_name(&path));
        p.timelines.push(Some(t));
    }
    let mut extra = Vec::new();
    for (i, path) in leaves(Kind::Object).iter().enumerate() {
        let o = load_object(&mut ctx, path, &leaf_name(path), i, &mut extra);
        p.objects.push(Some(o));
    }
    let mut next_inst = FIRST_INSTANCE_ID;
    let mut next_tile = FIRST_TILE_ID;
    let mut inst_names = HashMap::new();
    for path in leaves(Kind::Room) {
        let r = load_room(&mut ctx, &path, &leaf_name(&path), &mut next_inst, &mut next_tile, &mut inst_names);
        p.rooms.push(Some(r));
    }
    p.last_instance_id = next_inst;
    p.last_tile_id = next_tile;
    p.room_order = (0..p.rooms.len() as i32).collect();

    // Constants, settings, help, data files.
    if let Some(cs) = child(root, "constants") {
        for c in children(cs, "constant") {
            p.constants.push(Constant {
                name: attr(c, "name").to_string(),
                value: text_of(c),
            });
        }
    }
    if let Some(cfgs) = child(root, "Configs") {
        let cfg = children(cfgs, "Config")
            .map(text_of)
            .find(|c| c.trim().ends_with("Default"))
            .or_else(|| children(cfgs, "Config").map(text_of).next());
        if let Some(cfg) = cfg {
            load_config(&mut ctx, cfg.trim(), &mut p);
        }
    }
    if let Some(h) = child(root, "help") {
        let rtf = text(h, "rtf");
        if !rtf.trim().is_empty() {
            if let Some(t) = ctx.read_string(rtf.trim()) {
                p.game_info.rtf = t;
            }
        }
    }
    for df in children(root, "datafiles") {
        load_datafiles(&mut ctx, df, "", &mut p.included_files);
    }
    if let Some(tg) = child(root, "TutorialState") {
        let _ = tg;
    }
    // Rooms caption default: Studio leaves it empty, 8.1 shows the caption in the title.
    let mut names = std::mem::take(&mut ctx.names);
    let mut sh = HashMap::new();
    for (i, s) in shaders.iter().enumerate() {
        sh.insert(s.clone(), i as i32);
    }
    names.insert(Kind::Shader, sh);
    Ok(Loaded {
        project: p,
        tree,
        names,
        shaders,
        instance_names: inst_names,
        extra_events: extra,
        split_counts,
        warnings: ctx.warnings,
    })
}

fn res_name(p: &Project, k: Kind, i: usize) -> Option<String> {
    match k {
        Kind::Sprite => p.sprites.get(i)?.as_ref().map(|r| r.name.clone()),
        Kind::Sound => p.sounds.get(i)?.as_ref().map(|r| r.name.clone()),
        Kind::Background => p.backgrounds.get(i)?.as_ref().map(|r| r.name.clone()),
        Kind::Path => p.paths.get(i)?.as_ref().map(|r| r.name.clone()),
        Kind::Script => p.scripts.get(i)?.as_ref().map(|r| r.name.clone()),
        Kind::Font => p.fonts.get(i)?.as_ref().map(|r| r.name.clone()),
        Kind::Timeline => p.timelines.get(i)?.as_ref().map(|r| r.name.clone()),
        Kind::Object => p.objects.get(i)?.as_ref().map(|r| r.name.clone()),
        Kind::Room => p.rooms.get(i)?.as_ref().map(|r| r.name.clone()),
        Kind::Shader => None,
    }
}

fn res_count(p: &Project, k: Kind) -> usize {
    match k {
        Kind::Sprite => p.sprites.len(),
        Kind::Sound => p.sounds.len(),
        Kind::Background => p.backgrounds.len(),
        Kind::Path => p.paths.len(),
        Kind::Script => p.scripts.len(),
        Kind::Font => p.fonts.len(),
        Kind::Timeline => p.timelines.len(),
        Kind::Object => p.objects.len(),
        Kind::Room => p.rooms.len(),
        Kind::Shader => 0,
    }
}

/// Build the 8.1 resource tree from the GMX sections (groups are kept). Resources
/// were numbered in tree order, so leaves map to a running counter; a script file
/// that held several `#define`s expands to all of its scripts. Resources added by the
/// converter are appended to their root.
pub fn build_tree(l: &Loaded, p: &Project) -> Vec<TreeNode> {
    fn go(
        nodes: &[Node],
        k: Kind,
        rk: ResourceKind,
        p: &Project,
        l: &Loaded,
        next: &mut usize,
        used: &mut [bool],
    ) -> Vec<TreeNode> {
        let mut out = Vec::new();
        for n in nodes {
            match n {
                Node::Group(g, c) => {
                    out.push(TreeNode::group(rk, g.clone(), go(c, k, rk, p, l, next, used)))
                }
                Node::Leaf(path) => {
                    let count = if k == Kind::Script {
                        l.split_counts.get(path).copied().unwrap_or(1)
                    } else {
                        1
                    };
                    for _ in 0..count {
                        let i = *next;
                        *next += 1;
                        if i < used.len() {
                            if let Some(name) = res_name(p, k, i) {
                                used[i] = true;
                                out.push(TreeNode::leaf(rk, i as i32, name));
                            }
                        }
                    }
                }
            }
        }
        out
    }
    let mut roots = TreeNode::standard_roots();
    for k in Kind::ALL {
        let Some(rk) = k.tree_kind() else { continue };
        let mut used = vec![false; res_count(p, k)];
        let mut next = 0;
        let mut children = go(&l.tree[&k], k, rk, p, l, &mut next, &mut used);
        for (i, u) in used.iter().enumerate() {
            if !u {
                if let Some(n) = res_name(p, k, i) {
                    children.push(TreeNode::leaf(rk, i as i32, n));
                }
            }
        }
        roots.iter_mut().find(|r| r.kind == rk).unwrap().children = children;
    }
    roots
}

