//! Project files: `.gmk` (v701, v800) and `.gm81` (v810) readers, v800/v810 writers
//! (specs/file-formats.md §1-§8).

use crate::error::{Error, Result};
use crate::image::{key_out_bottom_left, read_img701, read_img800, smooth_edges, write_img800};
use crate::io::{inflate, Reader, Writer};
use crate::model::*;

/// Magic number at the start of every project / export file.
pub const MAGIC: i32 = 1234321;

/// Project file format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatVersion {
    /// GameMaker 7 `.gmk` (obfuscated, read only).
    V701,
    /// GameMaker 8.0 `.gmk`.
    V800,
    /// GameMaker 8.1 `.gm81` (v800 layout with header version 810; spec addendum).
    V810,
}

impl FormatVersion {
    pub fn number(self) -> i32 {
        match self {
            FormatVersion::V701 => 701,
            FormatVersion::V800 => 800,
            FormatVersion::V810 => 810,
        }
    }
}

/// Inspect the header and return the file's format version.
pub fn detect_version(data: &[u8]) -> Result<FormatVersion> {
    let mut r = Reader::new(data, "file");
    let magic = r.i32()?;
    if magic != MAGIC {
        return Err(Error::BadMagic { found: magic });
    }
    let at = r.offset();
    match r.i32()? {
        701 => Ok(FormatVersion::V701),
        800 => Ok(FormatVersion::V800),
        810 => Ok(FormatVersion::V810),
        v => Err(Error::UnsupportedVersion {
            context: "file".into(),
            offset: at,
            what: "project",
            found: v,
        }),
    }
}

/// Read a project file of any supported version.
pub fn read_project(data: &[u8]) -> Result<Project> {
    Ok(parse(data, false)?.0)
}

/// Read a project file and also report its format version.
pub fn read_project_versioned(data: &[u8]) -> Result<(Project, FormatVersion)> {
    let (p, v, _) = parse(data, false)?;
    Ok((p, v))
}

/// Read a project from disk.
pub fn read_project_file(path: impl AsRef<std::path::Path>) -> Result<(Project, FormatVersion)> {
    read_project_versioned(&std::fs::read(path)?)
}

/// Canonical, compression-independent form of a project file.
///
/// For v800/v810 this is the file with every zlib block replaced by
/// `u32 inflated_length` followed by the inflated bytes; two files with equal canonical
/// bytes differ only in how zlib compressed their blocks. For v701 it is the
/// de-obfuscated stream (without the envelope).
pub fn canonical_bytes(data: &[u8]) -> Result<Vec<u8>> {
    Ok(parse(data, true)?.2.unwrap_or_default())
}

/// Write a project in the given format. Only [`FormatVersion::V800`] and
/// [`FormatVersion::V810`] can be written.
pub fn write_project(p: &Project, version: FormatVersion) -> Result<Vec<u8>> {
    match version {
        FormatVersion::V701 => Err(Error::InvalidModel(
            "writing the v701 format is not supported".into(),
        )),
        v => write_v800(p, v),
    }
}

/// Write a project to disk.
pub fn write_project_file(
    path: impl AsRef<std::path::Path>,
    p: &Project,
    version: FormatVersion,
) -> Result<()> {
    std::fs::write(path, write_project(p, version)?)?;
    Ok(())
}

// ------------------------------------------------------------------ v701 envelope

/// Build the seed-driven byte permutation `T` and its inverse `U` (spec §8.2).
pub fn make_table(seed: u32) -> ([u8; 256], [u8; 256]) {
    let a = 6 + seed % 250;
    let b = seed / 250;
    let mut t = [0u8; 256];
    for (i, v) in t.iter_mut().enumerate() {
        *v = i as u8;
    }
    for i in 1..=10000u32 {
        let j = ((i.wrapping_mul(a).wrapping_add(b)) % 254 + 1) as usize;
        t.swap(j, j + 1);
    }
    let mut inv = [0u8; 256];
    for (x, &v) in t.iter().enumerate() {
        inv[v as usize] = x as u8;
    }
    (t, inv)
}

/// De-obfuscate a v701 file. Returns the plain stream (starting at `game_id`) and the
/// absolute file offset at which it starts.
pub fn decode_v701(data: &[u8]) -> Result<(Vec<u8>, usize)> {
    let mut r = Reader::new(data, "v701 envelope");
    let m = r.i32()?;
    if m != MAGIC {
        return Err(Error::BadMagic { found: m });
    }
    r.expect(701, "project version")?;
    let n1 = r.count("junk")?;
    let n2 = r.count("junk")?;
    r.take(
        n1.checked_mul(4)
            .ok_or_else(|| r.invalid_at(8, "junk too large"))?,
    )?;
    let seed = r.u32()?;
    r.take(
        n2.checked_mul(4)
            .ok_or_else(|| r.invalid_at(12, "junk too large"))?,
    )?;
    let start = r.pos();
    let (_, inv) = make_table(seed);
    // FF-13: the first encoded byte is stored raw, as in the extension `.dat` format.
    // Evidence: treasure.gmk decoded this way has the same game id as treasure.gm81,
    // which the 8.1 IDE saved from it (see OPEN_QUESTIONS.md [IMPL-FMT]).
    let plain = data[start..]
        .iter()
        .enumerate()
        .map(|(k, &c)| {
            if k == 0 {
                c
            } else {
                inv[c as usize].wrapping_sub((start + k) as u8)
            }
        })
        .collect();
    Ok((plain, start))
}

/// Obfuscate a plain v701 stream (inverse of [`decode_v701`]); used by tests.
pub fn encode_v701(plain: &[u8], seed: u32, n1: usize, n2: usize) -> Vec<u8> {
    let mut w = Writer::new();
    w.i32(MAGIC);
    w.i32(701);
    w.i32(n1 as i32);
    w.i32(n2 as i32);
    for i in 0..n1 {
        w.i32((i * 7 % 3000) as i32);
    }
    w.u32(seed);
    for i in 0..n2 {
        w.i32((i * 13 % 3000) as i32);
    }
    let start = w.buf.len();
    let (t, _) = make_table(seed);
    for (k, &b) in plain.iter().enumerate() {
        if k == 0 {
            w.buf.push(b);
        } else {
            w.buf.push(t[b.wrapping_add((start + k) as u8) as usize]);
        }
    }
    w.buf
}

// ------------------------------------------------------------------ reading

fn parse(data: &[u8], record: bool) -> Result<(Project, FormatVersion, Option<Vec<u8>>)> {
    let version = detect_version(data)?;
    match version {
        FormatVersion::V701 => {
            let (plain, start) = decode_v701(data)?;
            let mut r = Reader::new(&plain, "decoded v701 stream").with_base(start);
            let p = read_body(&mut r, false)?;
            r.end()?;
            let rec = record.then_some(plain.clone());
            Ok((p, version, rec))
        }
        _ => {
            let mut r = Reader::new(data, "file");
            if record {
                r.record = Some(Vec::new());
            }
            r.take(8)?; // magic + version
            let p = read_body(&mut r, true)?;
            r.end()?;
            Ok((p, version, r.record.take()))
        }
    }
}

fn read_body(r: &mut Reader<'_>, v8: bool) -> Result<Project> {
    let mut p = Project {
        game_id: r.i32()?,
        ..Project::default()
    };
    p.guid.copy_from_slice(r.take(16)?);
    if v8 {
        r.version(&[800, 810], "settings")?;
        p.settings = r.zblock("settings", |b| Ok(read_settings(b, true)?.0))?;
        r.version(&[800, 810], "triggers")?;
        let n = r.count("trigger")?;
        p.triggers = (0..n)
            .map(|i| r.zblock(&format!("triggers[{i}]"), read_trigger))
            .collect::<Result<_>>()?;
        p.triggers_last_changed = r.f64()?;
        r.version(&[800, 810], "constants")?;
        p.constants = read_constants(r)?;
        p.constants_last_changed = r.f64()?;
    } else {
        r.version(&[700, 701, 702], "settings")?;
        let (s, c) = read_settings(r, false)?;
        p.settings = s;
        p.constants = c;
    }
    p.sounds = read_list(r, v8, "sounds", |b, h| read_sound(b, h, v8))?;
    p.sprites = read_list(r, v8, "sprites", |b, h| read_sprite(b, h, v8))?;
    p.backgrounds = read_list(r, v8, "backgrounds", |b, h| read_background(b, h, v8))?;
    p.paths = read_list(r, v8, "paths", read_path)?;
    p.scripts = read_list(r, v8, "scripts", |b, h| {
        Ok(Script {
            name: h.0,
            last_changed: h.1,
            code: b.string()?,
        })
    })?;
    p.fonts = read_list(r, v8, "fonts", read_font)?;
    p.timelines = read_list(r, v8, "timelines", read_timeline)?;
    p.objects = read_list(r, v8, "objects", read_object)?;
    p.rooms = read_list(r, v8, "rooms", read_room)?;
    p.last_instance_id = r.i32()?;
    p.last_tile_id = r.i32()?;
    // Included files.
    r.version(&[620, 800, 810], "included files")?;
    let n = r.count("included file")?;
    p.included_files = (0..n)
        .map(|i| {
            if v8 {
                r.zblock(&format!("included_files[{i}]"), |b| read_include(b, true))
            } else {
                read_include(r, false)
            }
        })
        .collect::<Result<_>>()?;
    // Extension packages.
    r.version(&[700], "extension packages")?;
    let n = r.count("extension package")?;
    p.extension_packages = (0..n).map(|_| r.string()).collect::<Result<_>>()?;
    // Game information.
    if v8 {
        r.version(&[800, 810], "game information")?;
        p.game_info = r.zblock("game information", |b| read_game_info(b, true))?;
    } else {
        r.version(&[600, 620], "game information")?;
        p.game_info = read_game_info(r, false)?;
    }
    r.version(&[500], "library creation code")?;
    let n = r.count("library")?;
    p.library_init_code = (0..n).map(|_| r.string()).collect::<Result<_>>()?;
    r.version(&[700], "room order")?;
    let n = r.count("room order")?;
    p.room_order = (0..n).map(|_| r.i32()).collect::<Result<_>>()?;
    p.resource_tree = (0..12)
        .map(|_| read_tree_node(r, 0))
        .collect::<Result<_>>()?;
    Ok(p)
}

type Header = (String, Timestamp);

fn read_list<T>(
    r: &mut Reader<'_>,
    v8: bool,
    label: &str,
    body: impl Fn(&mut Reader<'_>, Header) -> Result<T>,
) -> Result<Vec<Option<T>>> {
    let _list_version = r.i32()?;
    let n = r.count(label)?;
    let one = |b: &mut Reader<'_>| -> Result<Option<T>> {
        if !b.bool()? {
            return Ok(None);
        }
        let name = b.string()?;
        let lc = if v8 { b.f64()? } else { 0.0 };
        let _resource_version = b.i32()?;
        Ok(Some(body(b, (name, lc))?))
    };
    (0..n)
        .map(|i| {
            if v8 {
                r.zblock(&format!("{label}[{i}]"), one)
            } else {
                one(r)
            }
        })
        .collect()
}

fn opt_blob(r: &mut Reader<'_>) -> Result<Option<Vec<u8>>> {
    Ok(if r.bool()? { Some(r.blob()?) } else { None })
}

fn read_settings(r: &mut Reader<'_>, v8: bool) -> Result<(Settings, Vec<Constant>)> {
    let mut s = Settings {
        start_fullscreen: r.bool()?,
        interpolate_pixels: r.bool()?,
        dont_draw_border: r.bool()?,
        display_cursor: r.bool()?,
        scaling: r.i32()?,
        allow_window_resize: r.bool()?,
        always_on_top: r.bool()?,
        color_outside_room: r.u32()?,
        set_resolution: r.bool()?,
        color_depth: r.i32()?,
        resolution: r.i32()?,
        frequency: r.i32()?,
        dont_show_buttons: r.bool()?,
        vsync: r.i32()?,
        ..Settings::default()
    };
    if v8 {
        s.disable_screensaver = r.bool()?;
    }
    s.let_f4_fullscreen = r.bool()?;
    s.let_f1_game_info = r.bool()?;
    s.let_esc_end_game = r.bool()?;
    s.let_f5_save_f6_load = r.bool()?;
    s.let_f9_screenshot = r.bool()?;
    s.treat_close_as_esc = r.bool()?;
    s.process_priority = r.i32()?;
    s.freeze_on_lose_focus = r.bool()?;
    s.loading_bar = r.i32()?;
    if s.loading_bar == 2 {
        s.loading_bar_back = opt_blob(r)?;
        s.loading_bar_front = opt_blob(r)?;
    }
    s.show_custom_load_image = r.bool()?;
    if s.show_custom_load_image {
        s.custom_load_image = opt_blob(r)?;
    }
    s.load_image_transparent = r.bool()?;
    s.load_image_alpha = r.i32()?;
    s.scale_progress_bar = r.bool()?;
    s.icon = r.blob()?;
    s.display_errors = r.bool()?;
    s.write_errors_to_log = r.bool()?;
    s.abort_on_all_errors = r.bool()?;
    // An i32 bit set, not a plain bool: GM 8.1 stores e.g. 2 here (spec §2 addendum).
    let flags = r.i32()?;
    s.treat_uninitialized_as_zero = flags & 1 != 0;
    s.uninitialized_extra_bits = flags & !1;
    s.author = r.string()?;
    s.version_text = r.string()?;
    s.info_last_changed = r.f64()?;
    s.information = r.string()?;
    let constants = if v8 { Vec::new() } else { read_constants(r)? };
    s.version_major = r.i32()?;
    s.version_minor = r.i32()?;
    s.version_release = r.i32()?;
    s.version_build = r.i32()?;
    s.company = r.string()?;
    s.product = r.string()?;
    s.copyright = r.string()?;
    s.description = r.string()?;
    s.last_changed = if v8 { r.f64()? } else { 0.0 };
    Ok((s, constants))
}

fn read_constants(r: &mut Reader<'_>) -> Result<Vec<Constant>> {
    let n = r.count("constant")?;
    (0..n)
        .map(|_| {
            Ok(Constant {
                name: r.string()?,
                value: r.string()?,
            })
        })
        .collect()
}

fn read_trigger(b: &mut Reader<'_>) -> Result<Option<Trigger>> {
    if !b.bool()? {
        return Ok(None);
    }
    let _version = b.i32()?;
    Ok(Some(Trigger {
        name: b.string()?,
        condition: b.string()?,
        moment: b.i32()?,
        constant_name: b.string()?,
    }))
}

fn read_sound(r: &mut Reader<'_>, h: Header, v8: bool) -> Result<Sound> {
    let kind = r.i32()?;
    let file_type = r.string()?;
    let file_name = r.string()?;
    let data = if r.bool()? {
        let at = r.offset();
        let raw = r.blob()?;
        if v8 {
            Some(raw)
        } else {
            Some(inflate(&raw).map_err(|message| Error::Zlib {
                context: r.context().to_string(),
                offset: at,
                message,
            })?)
        }
    } else {
        None
    };
    Ok(Sound {
        name: h.0,
        last_changed: h.1,
        kind,
        file_type,
        file_name,
        data,
        effects: r.i32()?,
        volume: r.f64()?,
        pan: r.f64()?,
        preload: r.bool()?,
    })
}

fn read_sprite(r: &mut Reader<'_>, h: Header, v8: bool) -> Result<Sprite> {
    let mut s = Sprite {
        name: h.0,
        last_changed: h.1,
        ..Sprite::default()
    };
    if v8 {
        s.origin_x = r.i32()?;
        s.origin_y = r.i32()?;
        let n = r.count("frame")?;
        s.frames = (0..n).map(|_| read_img800(r)).collect::<Result<_>>()?;
        // The collision/bbox trailer is present even when there are no frames
        // (spec §5.2 addendum, verified on user data).
        s.collision_shape = r.i32()?;
        s.alpha_tolerance = r.i32()?;
        s.separate_collision_masks = r.bool()?;
        s.bbox_mode = r.i32()?;
        s.bbox_left = r.i32()?;
        s.bbox_right = r.i32()?;
        s.bbox_bottom = r.i32()?;
        s.bbox_top = r.i32()?;
    } else {
        let w = r.i32()?.max(0) as u32;
        let hgt = r.i32()?.max(0) as u32;
        s.bbox_left = r.i32()?;
        s.bbox_right = r.i32()?;
        s.bbox_bottom = r.i32()?;
        s.bbox_top = r.i32()?;
        let transparent = r.bool()?;
        let smooth = r.bool()?;
        let _preload = r.bool()?;
        s.bbox_mode = r.i32()?;
        let precise = r.bool()?;
        s.collision_shape = if precise { 0 } else { 1 };
        // v701 has no such option; the 8.1 IDE converts with separate masks on
        // (observed on treasure.gmk vs treasure.gm81).
        s.separate_collision_masks = true;
        s.origin_x = r.i32()?;
        s.origin_y = r.i32()?;
        let n = r.count("frame")?;
        for _ in 0..n {
            let mut img = read_img701(r)?.unwrap_or_else(|| RgbaImage::new(w, hgt));
            if transparent {
                key_out_bottom_left(&mut img);
                if smooth {
                    smooth_edges(&mut img);
                }
            }
            s.frames.push(img);
        }
    }
    Ok(s)
}

fn read_background(r: &mut Reader<'_>, h: Header, v8: bool) -> Result<Background> {
    let mut b = Background {
        name: h.0,
        last_changed: h.1,
        ..Background::default()
    };
    let mut legacy = None;
    if !v8 {
        let w = r.i32()?.max(0) as u32;
        let hgt = r.i32()?.max(0) as u32;
        let transparent = r.bool()?;
        let smooth = r.bool()?;
        let _preload = r.bool()?;
        legacy = Some((w, hgt, transparent, smooth));
    }
    b.use_as_tileset = r.bool()?;
    b.tile_width = r.i32()?;
    b.tile_height = r.i32()?;
    b.tile_h_offset = r.i32()?;
    b.tile_v_offset = r.i32()?;
    b.tile_h_sep = r.i32()?;
    b.tile_v_sep = r.i32()?;
    match legacy {
        None => b.image = read_img800(r)?,
        Some((w, hgt, transparent, smooth)) => {
            let img = if r.bool()? { read_img701(r)? } else { None };
            b.image = img.unwrap_or_else(|| RgbaImage::new(w, hgt));
            if transparent {
                key_out_bottom_left(&mut b.image);
                if smooth {
                    smooth_edges(&mut b.image);
                }
            }
        }
    }
    Ok(b)
}

fn read_path(r: &mut Reader<'_>, h: Header) -> Result<Path> {
    let mut p = Path {
        name: h.0,
        last_changed: h.1,
        connection_kind: r.i32()?,
        closed: r.bool()?,
        precision: r.i32()?,
        backdrop_room: r.i32()?,
        snap_x: r.i32()?,
        snap_y: r.i32()?,
        points: Vec::new(),
    };
    let n = r.count("path point")?;
    for _ in 0..n {
        p.points.push(PathPoint {
            x: r.f64()?,
            y: r.f64()?,
            speed: r.f64()?,
        });
    }
    Ok(p)
}

fn read_font(r: &mut Reader<'_>, h: Header) -> Result<Font> {
    let mut f = Font {
        name: h.0,
        last_changed: h.1,
        font_name: r.string()?,
        size: r.i32()?,
        bold: r.bool()?,
        italic: r.bool()?,
        ..Font::default()
    };
    f.set_packed_range_start(r.i32()?);
    f.range_end = r.i32()?;
    Ok(f)
}

fn read_timeline(r: &mut Reader<'_>, h: Header) -> Result<Timeline> {
    let n = r.count("moment")?;
    let moments = (0..n)
        .map(|_| {
            Ok(Moment {
                step: r.i32()?,
                actions: read_actions(r)?,
            })
        })
        .collect::<Result<_>>()?;
    Ok(Timeline {
        name: h.0,
        last_changed: h.1,
        moments,
    })
}

/// Read an action list (spec §5.9).
pub(crate) fn read_actions(r: &mut Reader<'_>) -> Result<Vec<Action>> {
    r.expect(400, "action list version")?;
    let n = r.count("action")?;
    (0..n)
        .map(|_| {
            r.expect(440, "action version")?;
            let mut a = Action {
                library_id: r.i32()?,
                action_id: r.i32()?,
                kind: r.i32()?,
                may_be_relative: r.bool()?,
                is_question: r.bool()?,
                has_applies_to: r.bool()?,
                exe_type: r.i32()?,
                function_name: r.string()?,
                code: r.string()?,
                argument_count: r.i32()?,
                ..Action::default()
            };
            let k = r.count("argument kind")?;
            a.arg_kinds = (0..k).map(|_| r.i32()).collect::<Result<_>>()?;
            a.applies_to = r.i32()?;
            a.relative = r.bool()?;
            let k = r.count("argument")?;
            a.args = (0..k).map(|_| r.string()).collect::<Result<_>>()?;
            a.negate = r.bool()?;
            Ok(a)
        })
        .collect()
}

fn read_object(r: &mut Reader<'_>, h: Header) -> Result<Object> {
    let mut o = Object {
        name: h.0,
        last_changed: h.1,
        sprite: r.i32()?,
        solid: r.bool()?,
        visible: r.bool()?,
        depth: r.i32()?,
        persistent: r.bool()?,
        parent: r.i32()?,
        mask_sprite: r.i32()?,
        events: Vec::new(),
    };
    let at = r.offset();
    let max_type = r.i32()?;
    if !(-1..=255).contains(&max_type) {
        return Err(r.invalid_at(at, format!("implausible max event type {max_type}")));
    }
    for event_type in 0..=max_type {
        loop {
            let number = r.i32()?;
            if number == -1 {
                break;
            }
            o.events.push(Event {
                event_type,
                number,
                actions: read_actions(r)?,
            });
        }
    }
    Ok(o)
}

fn read_room(r: &mut Reader<'_>, h: Header) -> Result<Room> {
    let mut m = Room {
        name: h.0,
        last_changed: h.1,
        caption: r.string()?,
        width: r.i32()?,
        height: r.i32()?,
        snap_x: r.i32()?,
        snap_y: r.i32()?,
        isometric_grid: r.bool()?,
        speed: r.i32()?,
        persistent: r.bool()?,
        background_color: r.u32()?,
        draw_background_color: r.bool()?,
        creation_code: r.string()?,
        backgrounds: Vec::new(),
        views: Vec::new(),
        instances: Vec::new(),
        tiles: Vec::new(),
        ..Room::default()
    };
    let n = r.count("room background")?;
    for _ in 0..n {
        m.backgrounds.push(RoomBackground {
            visible: r.bool()?,
            is_foreground: r.bool()?,
            background: r.i32()?,
            x: r.i32()?,
            y: r.i32()?,
            tile_h: r.bool()?,
            tile_v: r.bool()?,
            hspeed: r.i32()?,
            vspeed: r.i32()?,
            stretch: r.bool()?,
        });
    }
    m.views_enabled = r.bool()?;
    let n = r.count("view")?;
    for _ in 0..n {
        m.views.push(View {
            visible: r.bool()?,
            view_x: r.i32()?,
            view_y: r.i32()?,
            view_w: r.i32()?,
            view_h: r.i32()?,
            port_x: r.i32()?,
            port_y: r.i32()?,
            port_w: r.i32()?,
            port_h: r.i32()?,
            h_border: r.i32()?,
            v_border: r.i32()?,
            h_speed: r.i32()?,
            v_speed: r.i32()?,
            follow_object: r.i32()?,
        });
    }
    let n = r.count("instance")?;
    for _ in 0..n {
        m.instances.push(Instance {
            x: r.i32()?,
            y: r.i32()?,
            object: r.i32()?,
            id: r.i32()?,
            creation_code: r.string()?,
            locked: r.bool()?,
        });
    }
    let n = r.count("tile")?;
    for _ in 0..n {
        m.tiles.push(Tile {
            x: r.i32()?,
            y: r.i32()?,
            background: r.i32()?,
            tile_x: r.i32()?,
            tile_y: r.i32()?,
            width: r.i32()?,
            height: r.i32()?,
            depth: r.i32()?,
            id: r.i32()?,
            locked: r.bool()?,
        });
    }
    m.editor = RoomEditorState {
        remember_room_editor_info: r.i32()?,
        editor_width: r.i32()?,
        editor_height: r.i32()?,
        show_grid: r.i32()?,
        show_objects: r.i32()?,
        show_tiles: r.i32()?,
        show_backgrounds: r.i32()?,
        show_foregrounds: r.i32()?,
        show_views: r.i32()?,
        delete_underlying_objects: r.i32()?,
        delete_underlying_tiles: r.i32()?,
        current_tab: r.i32()?,
        h_scroll: r.i32()?,
        v_scroll: r.i32()?,
    };
    Ok(m)
}

fn read_include(r: &mut Reader<'_>, v8: bool) -> Result<IncludedFile> {
    let last_changed = if v8 { r.f64()? } else { 0.0 };
    let _version = r.i32()?;
    let mut f = IncludedFile {
        last_changed,
        file_name: r.string()?,
        original_path: r.string()?,
        original_file_chosen: r.bool()?,
        original_size: r.i32()?,
        ..IncludedFile::default()
    };
    if r.bool()? {
        f.data = Some(r.blob()?);
    }
    f.export_kind = r.i32()?;
    f.export_folder = r.string()?;
    f.overwrite_existing = r.bool()?;
    f.free_memory_after_export = r.bool()?;
    f.remove_at_game_end = r.bool()?;
    Ok(f)
}

fn read_game_info(r: &mut Reader<'_>, v8: bool) -> Result<GameInfo> {
    Ok(GameInfo {
        background_color: r.u32()?,
        show_in_separate_window: r.bool()?,
        window_caption: r.string()?,
        left: r.i32()?,
        top: r.i32()?,
        width: r.i32()?,
        height: r.i32()?,
        show_window_border: r.bool()?,
        allow_resize: r.bool()?,
        stay_on_top: r.bool()?,
        pause_game: r.bool()?,
        last_changed: if v8 { r.f64()? } else { 0.0 },
        rtf: r.string()?,
    })
}

const MAX_TREE_DEPTH: usize = 256;

fn read_tree_node(r: &mut Reader<'_>, depth: usize) -> Result<TreeNode> {
    if depth > MAX_TREE_DEPTH {
        return Err(r.invalid_at(r.offset(), "resource tree too deep"));
    }
    let status = NodeStatus::from_i32(r.i32()?);
    let kind = ResourceKind::from_i32(r.i32()?);
    let index = r.i32()?;
    let name = r.string()?;
    let n = r.count("tree child")?;
    let children = (0..n)
        .map(|_| read_tree_node(r, depth + 1))
        .collect::<Result<_>>()?;
    Ok(TreeNode {
        status,
        kind,
        index,
        name,
        children,
    })
}

// ------------------------------------------------------------------ writing (v800 / v810)

fn write_v800(p: &Project, version: FormatVersion) -> Result<Vec<u8>> {
    let mut w = Writer::new();
    w.i32(MAGIC);
    w.i32(version.number());
    w.i32(p.game_id);
    w.raw(&p.guid);
    w.i32(800); // settings version: 800 in both v800 and v810 (spec addendum)
    w.zblock(|b| write_settings(b, &p.settings))?;
    w.i32(800);
    w.count(p.triggers.len())?;
    for t in &p.triggers {
        w.zblock(|b| {
            match t {
                None => b.bool(false),
                Some(t) => {
                    b.bool(true);
                    b.i32(800);
                    b.string(&t.name);
                    b.string(&t.condition);
                    b.i32(t.moment);
                    b.string(&t.constant_name);
                }
            }
            Ok(())
        })?;
    }
    w.f64(p.triggers_last_changed);
    w.i32(800);
    w.count(p.constants.len())?;
    for c in &p.constants {
        w.string(&c.name);
        w.string(&c.value);
    }
    w.f64(p.constants_last_changed);

    write_list(
        &mut w,
        &p.sounds,
        800,
        |s| (&s.name, s.last_changed),
        |b, s| {
            b.i32(s.kind);
            b.string(&s.file_type);
            b.string(&s.file_name);
            b.bool(s.data.is_some());
            if let Some(d) = &s.data {
                b.blob(d);
            }
            b.i32(s.effects);
            b.f64(s.volume);
            b.f64(s.pan);
            b.bool(s.preload);
            Ok(())
        },
    )?;
    write_list(
        &mut w,
        &p.sprites,
        800,
        |s| (&s.name, s.last_changed),
        write_sprite_body,
    )?;
    write_list(
        &mut w,
        &p.backgrounds,
        710,
        |s| (&s.name, s.last_changed),
        |b, g| {
            b.bool(g.use_as_tileset);
            b.i32(g.tile_width);
            b.i32(g.tile_height);
            b.i32(g.tile_h_offset);
            b.i32(g.tile_v_offset);
            b.i32(g.tile_h_sep);
            b.i32(g.tile_v_sep);
            write_img800(b, &g.image)
        },
    )?;
    write_list(
        &mut w,
        &p.paths,
        530,
        |s| (&s.name, s.last_changed),
        |b, t| {
            b.i32(t.connection_kind);
            b.bool(t.closed);
            b.i32(t.precision);
            b.i32(t.backdrop_room);
            b.i32(t.snap_x);
            b.i32(t.snap_y);
            b.count(t.points.len())?;
            for pt in &t.points {
                b.f64(pt.x);
                b.f64(pt.y);
                b.f64(pt.speed);
            }
            Ok(())
        },
    )?;
    write_list(
        &mut w,
        &p.scripts,
        800,
        |s| (&s.name, s.last_changed),
        |b, s| {
            b.string(&s.code);
            Ok(())
        },
    )?;
    write_list(
        &mut w,
        &p.fonts,
        800,
        |s| (&s.name, s.last_changed),
        |b, f| {
            b.string(&f.font_name);
            b.i32(f.size);
            b.bool(f.bold);
            b.bool(f.italic);
            b.i32(f.packed_range_start());
            b.i32(f.range_end);
            Ok(())
        },
    )?;
    write_list(
        &mut w,
        &p.timelines,
        500,
        |s| (&s.name, s.last_changed),
        |b, t| {
            b.count(t.moments.len())?;
            for m in &t.moments {
                b.i32(m.step);
                write_actions(b, &m.actions)?;
            }
            Ok(())
        },
    )?;
    write_list(
        &mut w,
        &p.objects,
        430,
        |s| (&s.name, s.last_changed),
        write_object,
    )?;
    write_list(
        &mut w,
        &p.rooms,
        541,
        |s| (&s.name, s.last_changed),
        write_room,
    )?;

    w.i32(p.last_instance_id);
    w.i32(p.last_tile_id);
    w.i32(800);
    w.count(p.included_files.len())?;
    for f in &p.included_files {
        w.zblock(|b| {
            b.f64(f.last_changed);
            b.i32(800);
            b.string(&f.file_name);
            b.string(&f.original_path);
            b.bool(f.original_file_chosen);
            b.i32(f.original_size);
            b.bool(f.data.is_some());
            if let Some(d) = &f.data {
                b.blob(d);
            }
            b.i32(f.export_kind);
            b.string(&f.export_folder);
            b.bool(f.overwrite_existing);
            b.bool(f.free_memory_after_export);
            b.bool(f.remove_at_game_end);
            Ok(())
        })?;
    }
    w.i32(700);
    w.count(p.extension_packages.len())?;
    for n in &p.extension_packages {
        w.string(n);
    }
    w.i32(800);
    let g = &p.game_info;
    w.zblock(|b| {
        b.u32(g.background_color);
        b.bool(g.show_in_separate_window);
        b.string(&g.window_caption);
        b.i32(g.left);
        b.i32(g.top);
        b.i32(g.width);
        b.i32(g.height);
        b.bool(g.show_window_border);
        b.bool(g.allow_resize);
        b.bool(g.stay_on_top);
        b.bool(g.pause_game);
        b.f64(g.last_changed);
        b.string(&g.rtf);
        Ok(())
    })?;
    w.i32(500);
    w.count(p.library_init_code.len())?;
    for c in &p.library_init_code {
        w.string(c);
    }
    w.i32(700);
    w.count(p.room_order.len())?;
    for &r in &p.room_order {
        w.i32(r);
    }
    if p.resource_tree.len() != 12 {
        return Err(Error::InvalidModel(format!(
            "resource tree must have 12 roots, has {}",
            p.resource_tree.len()
        )));
    }
    for n in &p.resource_tree {
        write_tree_node(&mut w, n)?;
    }
    Ok(w.buf)
}

fn write_list<T>(
    w: &mut Writer,
    items: &[Option<T>],
    resource_version: i32,
    header: impl Fn(&T) -> (&String, Timestamp),
    body: impl Fn(&mut Writer, &T) -> Result<()>,
) -> Result<()> {
    w.i32(800);
    w.count(items.len())?;
    for it in items {
        w.zblock(|b| {
            match it {
                None => b.bool(false),
                Some(t) => {
                    b.bool(true);
                    let (name, lc) = header(t);
                    b.string(name);
                    b.f64(lc);
                    b.i32(resource_version);
                    body(b, t)?;
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn write_settings(b: &mut Writer, s: &Settings) -> Result<()> {
    b.bool(s.start_fullscreen);
    b.bool(s.interpolate_pixels);
    b.bool(s.dont_draw_border);
    b.bool(s.display_cursor);
    b.i32(s.scaling);
    b.bool(s.allow_window_resize);
    b.bool(s.always_on_top);
    b.u32(s.color_outside_room);
    b.bool(s.set_resolution);
    b.i32(s.color_depth);
    b.i32(s.resolution);
    b.i32(s.frequency);
    b.bool(s.dont_show_buttons);
    b.i32(s.vsync);
    b.bool(s.disable_screensaver);
    b.bool(s.let_f4_fullscreen);
    b.bool(s.let_f1_game_info);
    b.bool(s.let_esc_end_game);
    b.bool(s.let_f5_save_f6_load);
    b.bool(s.let_f9_screenshot);
    b.bool(s.treat_close_as_esc);
    b.i32(s.process_priority);
    b.bool(s.freeze_on_lose_focus);
    b.i32(s.loading_bar);
    let opt = |b: &mut Writer, v: &Option<Vec<u8>>| {
        b.bool(v.is_some());
        if let Some(d) = v {
            b.blob(d);
        }
    };
    if s.loading_bar == 2 {
        opt(b, &s.loading_bar_back);
        opt(b, &s.loading_bar_front);
    }
    b.bool(s.show_custom_load_image);
    if s.show_custom_load_image {
        opt(b, &s.custom_load_image);
    }
    b.bool(s.load_image_transparent);
    b.i32(s.load_image_alpha);
    b.bool(s.scale_progress_bar);
    b.blob(&s.icon);
    b.bool(s.display_errors);
    b.bool(s.write_errors_to_log);
    b.bool(s.abort_on_all_errors);
    b.i32(s.treat_uninitialized_as_zero as i32 | (s.uninitialized_extra_bits & !1));
    b.string(&s.author);
    b.string(&s.version_text);
    b.f64(s.info_last_changed);
    b.string(&s.information);
    b.i32(s.version_major);
    b.i32(s.version_minor);
    b.i32(s.version_release);
    b.i32(s.version_build);
    b.string(&s.company);
    b.string(&s.product);
    b.string(&s.copyright);
    b.string(&s.description);
    b.f64(s.last_changed);
    Ok(())
}

/// Sprite body (spec §5.2, from origin_x onward).
pub(crate) fn write_sprite_body(b: &mut Writer, s: &Sprite) -> Result<()> {
    b.i32(s.origin_x);
    b.i32(s.origin_y);
    b.count(s.frames.len())?;
    for f in &s.frames {
        write_img800(b, f)?;
    }
    // Always written, also for a sprite without frames (spec §5.2).
    b.i32(s.collision_shape);
    b.i32(s.alpha_tolerance);
    b.bool(s.separate_collision_masks);
    b.i32(s.bbox_mode);
    b.i32(s.bbox_left);
    b.i32(s.bbox_right);
    b.i32(s.bbox_bottom);
    b.i32(s.bbox_top);
    Ok(())
}

/// Read a v800 sprite body (spec §5.2, from origin_x onward).
pub(crate) fn read_sprite_body(r: &mut Reader<'_>) -> Result<Sprite> {
    read_sprite(r, (String::new(), 0.0), true)
}

/// Write an action list (spec §5.9).
pub(crate) fn write_actions(b: &mut Writer, actions: &[Action]) -> Result<()> {
    b.i32(400);
    b.count(actions.len())?;
    for a in actions {
        b.i32(440);
        b.i32(a.library_id);
        b.i32(a.action_id);
        b.i32(a.kind);
        b.bool(a.may_be_relative);
        b.bool(a.is_question);
        b.bool(a.has_applies_to);
        b.i32(a.exe_type);
        b.string(&a.function_name);
        b.string(&a.code);
        b.i32(a.argument_count);
        b.count(a.arg_kinds.len())?;
        for &k in &a.arg_kinds {
            b.i32(k);
        }
        b.i32(a.applies_to);
        b.bool(a.relative);
        b.count(a.args.len())?;
        for s in &a.args {
            b.string(s);
        }
        b.bool(a.negate);
    }
    Ok(())
}

fn write_object(b: &mut Writer, o: &Object) -> Result<()> {
    b.i32(o.sprite);
    b.bool(o.solid);
    b.bool(o.visible);
    b.i32(o.depth);
    b.bool(o.persistent);
    b.i32(o.parent);
    b.i32(o.mask_sprite);
    if let Some(e) = o
        .events
        .iter()
        .find(|e| !(0..=event_type::MAX).contains(&e.event_type) || e.number == -1)
    {
        return Err(Error::InvalidModel(format!(
            "object {}: event ({}, {}) cannot be stored",
            o.name, e.event_type, e.number
        )));
    }
    b.i32(event_type::MAX);
    for t in 0..=event_type::MAX {
        for e in o.events.iter().filter(|e| e.event_type == t) {
            b.i32(e.number);
            write_actions(b, &e.actions)?;
        }
        b.i32(-1);
    }
    Ok(())
}

fn write_room(b: &mut Writer, m: &Room) -> Result<()> {
    b.string(&m.caption);
    b.i32(m.width);
    b.i32(m.height);
    b.i32(m.snap_x);
    b.i32(m.snap_y);
    b.bool(m.isometric_grid);
    b.i32(m.speed);
    b.bool(m.persistent);
    b.u32(m.background_color);
    b.bool(m.draw_background_color);
    b.string(&m.creation_code);
    b.count(m.backgrounds.len())?;
    for g in &m.backgrounds {
        b.bool(g.visible);
        b.bool(g.is_foreground);
        b.i32(g.background);
        b.i32(g.x);
        b.i32(g.y);
        b.bool(g.tile_h);
        b.bool(g.tile_v);
        b.i32(g.hspeed);
        b.i32(g.vspeed);
        b.bool(g.stretch);
    }
    b.bool(m.views_enabled);
    b.count(m.views.len())?;
    for v in &m.views {
        b.bool(v.visible);
        for x in [
            v.view_x,
            v.view_y,
            v.view_w,
            v.view_h,
            v.port_x,
            v.port_y,
            v.port_w,
            v.port_h,
            v.h_border,
            v.v_border,
            v.h_speed,
            v.v_speed,
            v.follow_object,
        ] {
            b.i32(x);
        }
    }
    b.count(m.instances.len())?;
    for i in &m.instances {
        b.i32(i.x);
        b.i32(i.y);
        b.i32(i.object);
        b.i32(i.id);
        b.string(&i.creation_code);
        b.bool(i.locked);
    }
    b.count(m.tiles.len())?;
    for t in &m.tiles {
        for x in [
            t.x,
            t.y,
            t.background,
            t.tile_x,
            t.tile_y,
            t.width,
            t.height,
            t.depth,
            t.id,
        ] {
            b.i32(x);
        }
        b.bool(t.locked);
    }
    let e = &m.editor;
    for x in [
        e.remember_room_editor_info,
        e.editor_width,
        e.editor_height,
        e.show_grid,
        e.show_objects,
        e.show_tiles,
        e.show_backgrounds,
        e.show_foregrounds,
        e.show_views,
        e.delete_underlying_objects,
        e.delete_underlying_tiles,
        e.current_tab,
        e.h_scroll,
        e.v_scroll,
    ] {
        b.i32(x);
    }
    Ok(())
}

fn write_tree_node(w: &mut Writer, n: &TreeNode) -> Result<()> {
    w.i32(n.status.to_i32());
    w.i32(n.kind.to_i32());
    w.i32(n.index);
    w.string(&n.name);
    w.count(n.children.len())?;
    for c in &n.children {
        write_tree_node(w, c)?;
    }
    Ok(())
}
