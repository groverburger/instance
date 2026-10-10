//! Project folders: a lossless, diff-friendly directory form of a [`Project`].
//!
//! [`write_folder`] turns a project into ordinary files (GML in `.gml` files, images as
//! PNG, sounds as their original files, metadata as small TOML files) and
//! [`read_folder`] reads them back. For every project `p`,
//! `read_folder(write_folder(p)) == p` exactly — deleted slots, timestamps, the action
//! metadata copies, the resource tree, the room-editor state and every opaque field
//! included. [`read_project_any`] accepts either a `.gm81`/`.gmk` file or a folder.
//!
//! The format is documented for users in `docs/project-folder.md`. In short:
//!
//! ```text
//! project.toml              game id, guid, format version, [slots] (slot counts incl.
//!                           deleted ones), [settings], [game_information], constants,
//!                           room order, last ids, extension packages, library init code
//! game_information.rtf      the Game Information text
//! resource_tree.txt         the resource tree, one node per line, indented by 2 spaces
//! settings/icon.ico         the game icon; loading_bar_back.*, loading_bar_front.*,
//!                           load_image.* when present (bytes as stored)
//! sprites/<name>/sprite.toml + 0.png, 1.png, ...
//! backgrounds/<name>.toml + <name>.png
//! sounds/<name>.toml + <name>.<ext> (the embedded file as is)
//! paths/<name>.toml         points as one inline table per line
//! scripts/<name>.gml + <name>.toml
//! fonts/<name>.toml
//! timelines/<name>/timeline.toml + step_<n>.gml | step_<n>.actions.toml
//! objects/<name>/object.toml + <event>.gml | <event>.actions.toml
//! rooms/<name>/room.toml + creation.gml + instances/<id>.gml
//! triggers/<name>.toml + <name>.gml (the condition)
//! included_files/<file>.toml + <file> (the stored data)
//! ```
//!
//! * Every resource's metadata holds its slot `index` and `name`; slots without a
//!   resource are deleted slots (`[slots]` in `project.toml` keeps the total). A
//!   resource without `index` (e.g. a new `.gml` file in `scripts/`) gets the next free
//!   slot and a leaf in the resource tree.
//! * References between resources are names (`sprite = "spr_pod"`); `"#12"` is the
//!   fallback for slot 12 when a name is empty, duplicated, or the slot is deleted.
//! * Event files: `create`, `destroy`, `alarm_<n>`, `step`, `step_begin`, `step_end`,
//!   `collision_<object>`, `keyboard_<key>`, `keypress_<key>`, `keyrelease_<key>`
//!   (`vk_left`, `A`, `7`, `vk_f1`, `#N`), `mouse_<what>` (`left_button`,
//!   `global_left_pressed`, `wheel_up`, ...), `other_<what>` (`room_start`,
//!   `user_0`, `outside_view_2`, ...), `draw`, `trigger_<trigger>`, generic
//!   `event_<type>_<number>`; `~2` suffixes separate duplicates. The order of events is
//!   the `events` list in `object.toml`.
//! * An event or moment that is exactly one default "Execute Code" action (library 1,
//!   action 603) is a plain `.gml` file; anything else is `<event>.actions.toml`, with
//!   the code of code actions in sibling `<event>.<i>.gml` files. Action metadata is
//!   written only where it differs from the built-in action definition.
//! * Text is UTF-8 with LF line endings on disk; strings in the project are
//!   Windows-1252 with CR LF. Text whose line endings are not uniformly CR LF records
//!   its mode (`lf` = bare LF, `raw` = bytes as is) next to the file reference
//!   (`eol` keys), so packing reproduces the original exactly; new files pack as CR LF.
//!
//! [`write_folder`] owns the resource directories: files in them that are not part of
//! the project are removed, other files in the folder (e.g. `.git`) are left alone.

mod emit;
mod names;
mod read;
mod write;

use std::collections::HashMap;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;
use toml::{Table, Value};

use crate::error::{Error, Result};
use crate::gmk::FormatVersion;
use crate::model::{Project, RgbaImage};
use crate::text::{decode_cp1252, encode_cp1252};

pub use read::{read_folder, read_folder_versioned};
pub use write::{folder_files, write_folder, write_folder_versioned, FileMap};

/// Read a project from a `.gm81` / `.gmk` file or from a project folder.
pub fn read_project_any(path: impl AsRef<Path>) -> Result<Project> {
    Ok(read_project_any_versioned(path)?.0)
}

/// Like [`read_project_any`], also returning the format version (of the file, or the
/// `format_version` recorded in a folder's `project.toml`).
pub fn read_project_any_versioned(path: impl AsRef<Path>) -> Result<(Project, FormatVersion)> {
    let path = path.as_ref();
    if path.is_dir() {
        read_folder_versioned(path)
    } else {
        crate::gmk::read_project_file(path)
    }
}

/// Whether `path` looks like a project folder (a directory with a `project.toml`).
pub fn is_project_folder(path: impl AsRef<Path>) -> bool {
    path.as_ref().join("project.toml").is_file()
}

/// Top-level resource directories owned by the folder writer.
const KIND_DIRS: &[&str] = &[
    "settings",
    "sprites",
    "sounds",
    "backgrounds",
    "paths",
    "scripts",
    "fonts",
    "timelines",
    "objects",
    "rooms",
    "triggers",
    "included_files",
];

fn folder_err(path: impl Into<String>, message: impl Into<String>) -> Error {
    Error::Folder {
        path: path.into(),
        message: message.into(),
    }
}

// ------------------------------------------------------------------ text

/// How the line endings of a text piece are stored (see module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Eol {
    /// Only CR LF line breaks (or none): LF on disk, CR LF in the project.
    Crlf,
    /// Only bare LF line breaks: kept as LF.
    Lf,
    /// Anything else: the text exactly as stored.
    Raw,
}

impl Eol {
    fn of(s: &str) -> Eol {
        let b = s.as_bytes();
        let mut lone_lf = false;
        let mut crlf = false;
        for (i, &c) in b.iter().enumerate() {
            match c {
                b'\r' if b.get(i + 1) == Some(&b'\n') => crlf = true,
                b'\r' => return Eol::Raw,
                b'\n' if i > 0 && b[i - 1] == b'\r' => {}
                b'\n' => lone_lf = true,
                _ => {}
            }
        }
        match (crlf, lone_lf) {
            (true, true) => Eol::Raw,
            (false, true) => Eol::Lf,
            _ => Eol::Crlf,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Eol::Crlf => "crlf",
            Eol::Lf => "lf",
            Eol::Raw => "raw",
        }
    }

    fn parse(s: &str) -> Option<Eol> {
        match s {
            "crlf" => Some(Eol::Crlf),
            "lf" => Some(Eol::Lf),
            "raw" => Some(Eol::Raw),
            _ => None,
        }
    }

    /// The text as written to disk.
    fn to_disk(self, s: &str) -> String {
        match self {
            Eol::Crlf => s.replace("\r\n", "\n"),
            Eol::Lf | Eol::Raw => s.to_string(),
        }
    }

    /// The project string for text read from disk.
    fn restore(self, s: &str) -> String {
        match self {
            Eol::Crlf => s.replace("\r\n", "\n").replace('\n', "\r\n"),
            Eol::Lf => s.replace("\r\n", "\n"),
            Eol::Raw => s.to_string(),
        }
    }
}

/// Check that `s` survives Windows-1252 encoding (project strings are ANSI).
fn check_cp1252(s: &str, path: &str) -> Result<()> {
    if decode_cp1252(&encode_cp1252(s)) == s {
        return Ok(());
    }
    for (line, text) in s.split('\n').enumerate() {
        for c in text.chars() {
            let cs = c.to_string();
            if decode_cp1252(&encode_cp1252(&cs)) != cs {
                return Err(folder_err(
                    path,
                    format!(
                        "line {}: character {:?} (U+{:04X}) cannot be stored in a GameMaker 8.1 project (Windows-1252 text)",
                        line + 1,
                        c,
                        c as u32
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Decode a text file read from disk: UTF-8 (a BOM is dropped), Windows-1252
/// representable, line endings per `eol`.
fn decode_text(bytes: &[u8], eol: Eol, path: &str) -> Result<String> {
    let s = std::str::from_utf8(bytes).map_err(|e| {
        folder_err(
            path,
            format!("not valid UTF-8 text ({e}); save the file as UTF-8"),
        )
    })?;
    let s = s.strip_prefix('\u{FEFF}').unwrap_or(s);
    check_cp1252(s, path)?;
    Ok(eol.restore(s))
}

// ------------------------------------------------------------------ images

fn encode_png(img: &RgbaImage, what: &str) -> Result<Vec<u8>> {
    if !img.is_consistent() {
        return Err(Error::InvalidModel(format!(
            "{what}: image data does not match {}x{}",
            img.width, img.height
        )));
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc
            .write_header()
            .map_err(|e| Error::InvalidModel(format!("{what}: PNG: {e}")))?;
        w.write_image_data(&img.data)
            .map_err(|e| Error::InvalidModel(format!("{what}: PNG: {e}")))?;
    }
    Ok(out)
}

/// Decode any PNG (palette, grey, RGB, 16-bit, with or without alpha) into RGBA8.
fn decode_png(data: &[u8], path: &str) -> Result<RgbaImage> {
    let bad = |e: String| folder_err(path, format!("cannot read PNG: {e}"));
    let mut dec = png::Decoder::new(std::io::Cursor::new(data));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = dec.read_info().map_err(|e| bad(e.to_string()))?;
    let size = r
        .output_buffer_size()
        .ok_or_else(|| bad("image too large".into()))?;
    let mut buf = vec![0; size];
    let info = r.next_frame(&mut buf).map_err(|e| bad(e.to_string()))?;
    let (w, h) = (info.width as usize, info.height as usize);
    let line = info.line_size;
    let ch = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return Err(bad("unexpanded palette image".into())),
    };
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let row = &buf[y * line..y * line + w * ch];
        for p in row.chunks_exact(ch) {
            out.extend_from_slice(&match ch {
                4 => [p[0], p[1], p[2], p[3]],
                3 => [p[0], p[1], p[2], 255],
                2 => [p[0], p[0], p[0], p[1]],
                _ => [p[0], p[0], p[0], 255],
            });
        }
    }
    Ok(RgbaImage {
        width: w as u32,
        height: h as u32,
        data: out,
    })
}

/// File extension for an opaque blob, sniffed from its first bytes.
fn blob_ext(b: &[u8]) -> &'static str {
    if b.starts_with(b"BM") {
        ".bmp"
    } else if b.starts_with(b"\x89PNG") {
        ".png"
    } else if b.starts_with(b"GIF8") {
        ".gif"
    } else if b.starts_with(&[0xFF, 0xD8]) {
        ".jpg"
    } else {
        ".bin"
    }
}

// ------------------------------------------------------------------ tables

/// A model value as a TOML table (via its serde representation).
fn to_table<T: Serialize>(v: &T) -> Table {
    match Value::try_from(v) {
        Ok(Value::Table(t)) => t,
        _ => unreachable!("model types serialize to TOML tables"),
    }
}

/// Keys of `t` whose value differs from `default`'s.
fn diff_table(t: Table, default: &Table) -> Table {
    t.into_iter()
        .filter(|(k, v)| default.get(k) != Some(v))
        .collect()
}

/// Overlay `user` on the serde table of `default` and deserialize; unknown keys and
/// type errors are reported with the key name.
fn from_table<T: Serialize + DeserializeOwned>(user: Table, default: &T, path: &str) -> Result<T> {
    let base = to_table(default);
    if let Some(k) = user.keys().find(|k| !base.contains_key(*k)) {
        let mut known: Vec<&String> = base.keys().collect();
        known.sort();
        let known: Vec<&str> = known.iter().map(|s| s.as_str()).collect();
        return Err(folder_err(
            path,
            format!("unknown key `{k}` (known keys: {})", known.join(", ")),
        ));
    }
    let mut t = base.clone();
    for (k, v) in &user {
        t.insert(k.clone(), v.clone());
    }
    match Value::Table(t).try_into::<T>() {
        Ok(v) => Ok(v),
        Err(e) => {
            // Find the offending key for a precise message.
            for (k, v) in user {
                let mut one = base.clone();
                one.insert(k.clone(), v.clone());
                if let Err(e) = Value::Table(one).try_into::<T>() {
                    return Err(folder_err(
                        path,
                        format!(
                            "key `{k}` = {}: {}",
                            emit::inline(&v),
                            first_line(&e.to_string())
                        ),
                    ));
                }
            }
            Err(folder_err(path, first_line(&e.to_string())))
        }
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().to_string()
}

// ------------------------------------------------------------------ references

/// Name ↔ slot mapping of one resource kind.
#[derive(Default)]
struct RefNames {
    /// Display name of the kind ("sprite").
    what: &'static str,
    /// Name of every slot (`None` = deleted).
    names: Vec<Option<String>>,
    /// Names that are unique within the kind.
    unique: HashMap<String, i32>,
}

impl RefNames {
    fn new<'a>(what: &'static str, names: impl Iterator<Item = Option<&'a str>>) -> Self {
        let names: Vec<Option<String>> = names.map(|n| n.map(str::to_string)).collect();
        let mut count: HashMap<&str, (usize, i32)> = HashMap::new();
        for (i, n) in names.iter().enumerate() {
            if let Some(n) = n {
                let e = count.entry(n.as_str()).or_insert((0, i as i32));
                e.0 += 1;
            }
        }
        let unique = count
            .into_iter()
            .filter(|(_, (c, _))| *c == 1)
            .map(|(n, (_, i))| (n.to_string(), i))
            .collect();
        RefNames {
            what,
            names,
            unique,
        }
    }

    fn of<T>(what: &'static str, v: &[Option<T>], name: impl Fn(&T) -> &str) -> Self {
        Self::new(what, v.iter().map(|x| x.as_ref().map(&name)))
    }

    /// The name of slot `i` if it can stand for the slot unambiguously.
    fn name_of(&self, i: i32) -> Option<&str> {
        let n = self.names.get(usize::try_from(i).ok()?)?.as_deref()?;
        let usable = !n.is_empty()
            && !n.starts_with('#')
            && n != "self"
            && n != "other"
            && self.unique.get(n) == Some(&i);
        usable.then_some(n)
    }

    /// Reference text for slot `i`: its name, or `#i`.
    fn r(&self, i: i32) -> String {
        self.name_of(i)
            .map_or_else(|| format!("#{i}"), str::to_string)
    }

    /// Reference text usable in a file name (identifier characters only).
    fn ident(&self, i: i32) -> String {
        match self.name_of(i) {
            Some(n) if names::ident_safe(n) => n.to_string(),
            _ => format!("#{i}"),
        }
    }

    /// Resolve reference text (`name` or `#N`).
    fn resolve(&self, s: &str) -> std::result::Result<i32, String> {
        if let Some(n) = s.strip_prefix('#') {
            return n.parse().map_err(|_| {
                format!(
                    "bad {} reference {s:?} (expected #<slot number>)",
                    self.what
                )
            });
        }
        if let Some(&i) = self.unique.get(s) {
            return Ok(i);
        }
        if self.names.iter().flatten().any(|n| n == s) {
            Err(format!(
                "{} name {s:?} is used by several {}s; refer to it as \"#<slot>\"",
                self.what, self.what
            ))
        } else {
            Err(format!("unknown {} {s:?}", self.what))
        }
    }

    /// Resolve a TOML value: a reference string or a raw slot number.
    fn resolve_value(&self, v: &Value) -> std::result::Result<i32, String> {
        match v {
            Value::String(s) => self.resolve(s),
            Value::Integer(i) => i32::try_from(*i).map_err(|_| format!("slot {i} out of range")),
            v => Err(format!(
                "expected a {} name, found {}",
                self.what,
                emit::inline(v)
            )),
        }
    }
}

/// Name tables of every referable kind.
struct Refs {
    sprites: RefNames,
    sounds: RefNames,
    backgrounds: RefNames,
    paths: RefNames,
    scripts: RefNames,
    fonts: RefNames,
    timelines: RefNames,
    objects: RefNames,
    rooms: RefNames,
    triggers: RefNames,
}

impl Refs {
    fn of(p: &Project) -> Refs {
        Refs {
            sprites: RefNames::of("sprite", &p.sprites, |x| &x.name),
            sounds: RefNames::of("sound", &p.sounds, |x| &x.name),
            backgrounds: RefNames::of("background", &p.backgrounds, |x| &x.name),
            paths: RefNames::of("path", &p.paths, |x| &x.name),
            scripts: RefNames::of("script", &p.scripts, |x| &x.name),
            fonts: RefNames::of("font", &p.fonts, |x| &x.name),
            timelines: RefNames::of("time line", &p.timelines, |x| &x.name),
            objects: RefNames::of("object", &p.objects, |x| &x.name),
            rooms: RefNames::of("room", &p.rooms, |x| &x.name),
            triggers: RefNames::of("trigger", &p.triggers, |x| &x.name),
        }
    }

    fn kind(&self, k: crate::model::ResourceKind) -> Option<&RefNames> {
        use crate::model::ResourceKind as K;
        Some(match k {
            K::Sprite => &self.sprites,
            K::Sound => &self.sounds,
            K::Background => &self.backgrounds,
            K::Path => &self.paths,
            K::Script => &self.scripts,
            K::Font => &self.fonts,
            K::Timeline => &self.timelines,
            K::Object => &self.objects,
            K::Room => &self.rooms,
            _ => return None,
        })
    }
}

/// Whether `a` is exactly one built-in "Execute Code" action with default metadata.
fn plain_code(actions: &[crate::model::Action]) -> Option<&str> {
    let [a] = actions else {
        return None;
    };
    if a.library_id != 1 || a.action_id != 603 || a.args.is_empty() {
        return None;
    }
    let mut t = action_template(1, 603);
    t.args[0] = a.args[0].clone();
    (t == *a).then_some(a.args[0].as_str())
}

/// The action an IDE would create for `(library_id, action_id)`: the built-in
/// definition instantiated, or a blank action for unknown libraries.
fn action_template(library_id: i32, action_id: i32) -> crate::model::Action {
    match crate::actionlib::builtin_action(library_id, action_id) {
        Some(d) => d.instantiate(library_id),
        None => crate::model::Action {
            library_id,
            action_id,
            arg_kinds: vec![0; 8],
            args: vec!["0".into(); 8],
            applies_to: crate::model::APPLIES_TO_SELF,
            ..Default::default()
        },
    }
}

/// Action metadata keys that are written only when they differ from the template.
const ACTION_META: &[&str] = &[
    "kind",
    "may_be_relative",
    "is_question",
    "has_applies_to",
    "exe_type",
    "function_name",
    "code",
    "argument_count",
    "arg_kinds",
];

/// Format version number recorded in `project.toml`.
fn version_number(v: FormatVersion) -> i32 {
    match v {
        FormatVersion::V810 => 810,
        _ => 800,
    }
}

#[cfg(test)]
mod tests;
