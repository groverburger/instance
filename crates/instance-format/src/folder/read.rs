//! Folder → project.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use toml::{Table, Value};

use super::names::{self, EventName};
use super::*;
use crate::model::Path as GmPath;
use crate::model::*;

/// Read a project folder written by [`super::write_folder`] (or edited by hand).
pub fn read_folder(dir: &Path) -> Result<Project> {
    Ok(read_folder_versioned(dir)?.0)
}

/// [`read_folder`], also returning the `format_version` recorded in `project.toml`.
pub fn read_folder_versioned(dir: &Path) -> Result<(Project, FormatVersion)> {
    if !dir.join("project.toml").is_file() {
        return Err(folder_err(
            dir.display().to_string(),
            "not a project folder (no project.toml)",
        ));
    }
    Rd { root: dir }.project()
}

struct Rd<'a> {
    root: &'a Path,
}

/// One resource found on disk, before references are resolved.
struct Entry {
    /// Metadata file (or directory) for messages.
    rel: String,
    /// Directory of a directory resource, or the file stem of a file resource.
    stem: String,
    t: Table,
    index: Option<i32>,
    name: String,
}

// ------------------------------------------------------------------ table access

fn take_str(t: &mut Table, k: &str, path: &str) -> Result<Option<String>> {
    match t.remove(k) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(v) => Err(folder_err(
            path,
            format!("key `{k}` must be a string, found {}", emit::inline(&v)),
        )),
    }
}

fn take_int(t: &mut Table, k: &str, path: &str) -> Result<Option<i32>> {
    match t.remove(k) {
        None => Ok(None),
        Some(Value::Integer(i)) => i32::try_from(i)
            .map(Some)
            .map_err(|_| folder_err(path, format!("key `{k}`: {i} is out of range"))),
        Some(v) => Err(folder_err(
            path,
            format!("key `{k}` must be an integer, found {}", emit::inline(&v)),
        )),
    }
}

fn take_f64(t: &mut Table, k: &str, path: &str) -> Result<Option<f64>> {
    match t.remove(k) {
        None => Ok(None),
        Some(Value::Float(f)) => Ok(Some(f)),
        Some(Value::Integer(i)) => Ok(Some(i as f64)),
        Some(v) => Err(folder_err(
            path,
            format!("key `{k}` must be a number, found {}", emit::inline(&v)),
        )),
    }
}

fn take_bool(t: &mut Table, k: &str, path: &str) -> Result<Option<bool>> {
    match t.remove(k) {
        None => Ok(None),
        Some(Value::Boolean(b)) => Ok(Some(b)),
        Some(v) => Err(folder_err(
            path,
            format!(
                "key `{k}` must be true or false, found {}",
                emit::inline(&v)
            ),
        )),
    }
}

fn take_table(t: &mut Table, k: &str, path: &str) -> Result<Option<Table>> {
    match t.remove(k) {
        None => Ok(None),
        Some(Value::Table(x)) => Ok(Some(x)),
        Some(v) => Err(folder_err(
            path,
            format!("key `{k}` must be a table, found {}", emit::inline(&v)),
        )),
    }
}

fn take_array(t: &mut Table, k: &str, path: &str) -> Result<Option<Vec<Value>>> {
    match t.remove(k) {
        None => Ok(None),
        Some(Value::Array(a)) => Ok(Some(a)),
        Some(v) => Err(folder_err(
            path,
            format!("key `{k}` must be an array, found {}", emit::inline(&v)),
        )),
    }
}

fn take_strings(t: &mut Table, k: &str, path: &str) -> Result<Option<Vec<String>>> {
    let Some(a) = take_array(t, k, path)? else {
        return Ok(None);
    };
    a.into_iter()
        .map(|v| match v {
            Value::String(s) => Ok(s),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Float(f) => Ok(f.to_string()),
            Value::Boolean(b) => Ok(if b { "1" } else { "0" }.to_string()),
            v => Err(folder_err(
                path,
                format!("key `{k}`: expected strings, found {}", emit::inline(&v)),
            )),
        })
        .collect::<Result<_>>()
        .map(Some)
}

fn take_tables(t: &mut Table, k: &str, path: &str) -> Result<Option<Vec<Table>>> {
    let Some(a) = take_array(t, k, path)? else {
        return Ok(None);
    };
    a.into_iter()
        .map(|v| match v {
            Value::Table(x) => Ok(x),
            v => Err(folder_err(
                path,
                format!("key `{k}`: expected tables, found {}", emit::inline(&v)),
            )),
        })
        .collect::<Result<_>>()
        .map(Some)
}

fn take_eol(t: &mut Table, k: &str, path: &str) -> Result<Eol> {
    match take_str(t, k, path)? {
        None => Ok(Eol::Crlf),
        Some(s) => Eol::parse(&s).ok_or_else(|| {
            folder_err(
                path,
                format!("key `{k}`: expected \"crlf\", \"lf\" or \"raw\", found {s:?}"),
            )
        }),
    }
}

/// Replace a reference string at `k` by its slot number (absent stays absent).
fn resolve_key(t: &mut Table, k: &str, refs: &RefNames, path: &str) -> Result<()> {
    if let Some(v) = t.remove(k) {
        let i = refs
            .resolve_value(&v)
            .map_err(|m| folder_err(path, format!("key `{k}`: {m}")))?;
        t.insert(k.into(), Value::Integer(i as i64));
    }
    Ok(())
}

fn no_extra_keys(t: &Table, path: &str) -> Result<()> {
    match t.keys().next() {
        Some(k) => Err(folder_err(path, format!("unknown key `{k}`"))),
        None => Ok(()),
    }
}

fn check_strings(v: &Value, path: &str) -> Result<()> {
    match v {
        Value::String(s) => check_cp1252(s, path),
        Value::Array(a) => a.iter().try_for_each(|x| check_strings(x, path)),
        Value::Table(t) => t.values().try_for_each(|x| check_strings(x, path)),
        _ => Ok(()),
    }
}

/// Split `name.ext` into `(name, ext)` at the first dot.
fn split_first_dot(name: &str) -> (&str, &str) {
    match name.find('.') {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    }
}

// ------------------------------------------------------------------ file access

impl Rd<'_> {
    fn exists(&self, rel: &str) -> bool {
        self.root.join(rel).is_file()
    }

    fn bytes(&self, rel: &str) -> Result<Vec<u8>> {
        std::fs::read(self.root.join(rel)).map_err(|e| folder_err(rel, format!("cannot read: {e}")))
    }

    fn toml(&self, rel: &str) -> Result<Table> {
        let b = self.bytes(rel)?;
        let s = std::str::from_utf8(&b)
            .map_err(|e| folder_err(rel, format!("not valid UTF-8 ({e})")))?;
        let s = s.strip_prefix('\u{FEFF}').unwrap_or(s);
        let t: Table = s
            .parse()
            .map_err(|e: toml::de::Error| folder_err(rel, format!("TOML syntax error: {e}")))?;
        t.values().try_for_each(|v| check_strings(v, rel))?;
        Ok(t)
    }

    fn text(&self, rel: &str, eol: Eol) -> Result<String> {
        decode_text(&self.bytes(rel)?, eol, rel)
    }

    /// Entries of a directory as (name, is_dir), sorted; a missing directory is empty.
    fn list(&self, rel: &str) -> Result<Vec<(String, bool)>> {
        let Ok(rd) = std::fs::read_dir(self.root.join(rel)) else {
            return Ok(Vec::new());
        };
        let mut v = Vec::new();
        for e in rd {
            let e = e.map_err(|e| folder_err(rel, e.to_string()))?;
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            v.push((name, is_dir));
        }
        v.sort();
        Ok(v)
    }

    // -------------------------------------------------------------- discovery

    fn entry(&self, rel: String, stem: String, default_name: &str, mut t: Table) -> Result<Entry> {
        let index = match t.remove("index") {
            None => None,
            Some(Value::Integer(i)) if (0..=i32::MAX as i64).contains(&i) => Some(i as i32),
            Some(v) => {
                return Err(folder_err(
                    &rel,
                    format!(
                        "key `index` must be a slot number >= 0, found {}",
                        emit::inline(&v)
                    ),
                ))
            }
        };
        let name = match t.get("name") {
            None => {
                t.insert("name".into(), Value::String(default_name.to_string()));
                default_name.to_string()
            }
            Some(Value::String(s)) => s.clone(),
            Some(v) => {
                return Err(folder_err(
                    &rel,
                    format!("key `name` must be a string, found {}", emit::inline(v)),
                ))
            }
        };
        Ok(Entry {
            rel,
            stem,
            t,
            index,
            name,
        })
    }

    /// Directory resources: `<kind>/<name>/<meta>` (the metadata file is optional).
    fn dir_entries(&self, kind: &str, meta: &str) -> Result<Vec<Entry>> {
        let mut out = Vec::new();
        for (name, is_dir) in self.list(kind)? {
            if !is_dir {
                continue;
            }
            let dir = format!("{kind}/{name}");
            let rel = format!("{dir}/{meta}");
            let t = if self.exists(&rel) {
                self.toml(&rel)?
            } else {
                Table::new()
            };
            out.push(self.entry(rel, dir, &name, t)?);
        }
        Ok(out)
    }

    /// File resources: `<kind>/<stem>.toml`, plus `<stem><new_ext>` files without
    /// metadata (new resources).
    fn file_entries(&self, kind: &str, new_ext: Option<&str>) -> Result<Vec<Entry>> {
        let files = self.list(kind)?;
        let names: HashSet<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        let mut out = Vec::new();
        for (name, is_dir) in &files {
            if *is_dir {
                continue;
            }
            if let Some(stem) = name.strip_suffix(".toml") {
                let rel = format!("{kind}/{name}");
                let t = self.toml(&rel)?;
                out.push(self.entry(rel, stem.to_string(), stem, t)?);
            } else if let Some(stem) = new_ext.and_then(|e| name.strip_suffix(e)) {
                if !names.contains(format!("{stem}.toml").as_str()) {
                    out.push(self.entry(
                        format!("{kind}/{name}"),
                        stem.into(),
                        stem,
                        Table::new(),
                    )?);
                }
            }
        }
        Ok(out)
    }

    // -------------------------------------------------------------- project

    fn project(&self) -> Result<(Project, FormatVersion)> {
        let pt = "project.toml";
        let mut t = self.toml(pt)?;
        let mut p = Project::default();
        let version = match take_int(&mut t, "format_version", pt)?.unwrap_or(810) {
            800 => FormatVersion::V800,
            810 => FormatVersion::V810,
            v => {
                return Err(folder_err(
                    pt,
                    format!("format_version {v} is not supported (800 or 810)"),
                ))
            }
        };
        if let Some(v) = take_int(&mut t, "game_id", pt)? {
            p.game_id = v;
        }
        if let Some(g) = take_str(&mut t, "guid", pt)? {
            let ok = g.len() == 32 && g.bytes().all(|b| b.is_ascii_hexdigit());
            if !ok {
                return Err(folder_err(pt, "key `guid` must be 32 hexadecimal digits"));
            }
            for (i, b) in p.guid.iter_mut().enumerate() {
                *b = u8::from_str_radix(&g[i * 2..i * 2 + 2], 16).unwrap_or(0);
            }
        }
        if let Some(v) = take_int(&mut t, "last_instance_id", pt)? {
            p.last_instance_id = v;
        }
        if let Some(v) = take_int(&mut t, "last_tile_id", pt)? {
            p.last_tile_id = v;
        }
        if let Some(v) = take_f64(&mut t, "triggers_last_changed", pt)? {
            p.triggers_last_changed = v;
        }
        if let Some(v) = take_f64(&mut t, "constants_last_changed", pt)? {
            p.constants_last_changed = v;
        }
        let room_order = take_array(&mut t, "room_order", pt)?;
        p.extension_packages = take_strings(&mut t, "extension_packages", pt)?.unwrap_or_default();
        p.library_init_code = take_strings(&mut t, "library_init_code", pt)?.unwrap_or_default();
        for c in take_tables(&mut t, "constants", pt)?.unwrap_or_default() {
            p.constants.push(from_table(
                c,
                &Constant::default(),
                &format!("{pt} [constants]"),
            )?);
        }
        let mut slots = take_table(&mut t, "slots", pt)?.unwrap_or_default();
        let sp = format!("{pt} [slots]");
        let mut slot = |k: &str| -> Result<usize> {
            Ok(take_int(&mut slots, k, &sp)?.unwrap_or(0).max(0) as usize)
        };
        let n_sounds = slot("sounds")?;
        let n_sprites = slot("sprites")?;
        let n_backgrounds = slot("backgrounds")?;
        let n_paths = slot("paths")?;
        let n_scripts = slot("scripts")?;
        let n_fonts = slot("fonts")?;
        let n_timelines = slot("timelines")?;
        let n_objects = slot("objects")?;
        let n_rooms = slot("rooms")?;
        let n_triggers = slot("triggers")?;
        no_extra_keys(&slots, &sp)?;
        let st = take_table(&mut t, "settings", pt)?.unwrap_or_default();
        p.settings = from_table(st, &Settings::default(), &format!("{pt} [settings]"))?;
        self.settings_blobs(&mut p.settings)?;
        let mut gi = take_table(&mut t, "game_information", pt)?.unwrap_or_default();
        let gp = format!("{pt} [game_information]");
        let rtf_eol = take_eol(&mut gi, "rtf_eol", &gp)?;
        p.game_info = from_table(gi, &GameInfo::default(), &gp)?;
        if self.exists("game_information.rtf") {
            p.game_info.rtf = self.text("game_information.rtf", rtf_eol)?;
        }
        no_extra_keys(&t, pt)?;

        // Discover resources and assign slots.
        let sounds = assign(self.file_entries("sounds", None)?, n_sounds)?;
        let sprites = assign(self.dir_entries("sprites", "sprite.toml")?, n_sprites)?;
        let backgrounds = assign(
            self.file_entries("backgrounds", Some(".png"))?,
            n_backgrounds,
        )?;
        let paths = assign(self.file_entries("paths", None)?, n_paths)?;
        let scripts = assign(self.file_entries("scripts", Some(".gml"))?, n_scripts)?;
        let fonts = assign(self.file_entries("fonts", None)?, n_fonts)?;
        let timelines = assign(self.dir_entries("timelines", "timeline.toml")?, n_timelines)?;
        let objects = assign(self.dir_entries("objects", "object.toml")?, n_objects)?;
        let rooms = assign(self.dir_entries("rooms", "room.toml")?, n_rooms)?;
        let triggers = assign(self.file_entries("triggers", Some(".gml"))?, n_triggers)?;
        let rn = |what: &'static str, s: &Slots| {
            RefNames::new(
                what,
                s.slots.iter().map(|e| e.as_ref().map(|e| e.name.as_str())),
            )
        };
        let refs = Refs {
            sprites: rn("sprite", &sprites),
            sounds: rn("sound", &sounds),
            backgrounds: rn("background", &backgrounds),
            paths: rn("path", &paths),
            scripts: rn("script", &scripts),
            fonts: rn("font", &fonts),
            timelines: rn("time line", &timelines),
            objects: rn("object", &objects),
            rooms: rn("room", &rooms),
            triggers: rn("trigger", &triggers),
        };

        let new = NewSlots {
            sprites: sprites.new.clone(),
            sounds: sounds.new.clone(),
            backgrounds: backgrounds.new.clone(),
            paths: paths.new.clone(),
            scripts: scripts.new.clone(),
            fonts: fonts.new.clone(),
            timelines: timelines.new.clone(),
            objects: objects.new.clone(),
            rooms: rooms.new.clone(),
        };
        p.sounds = convert(sounds, |e| self.sound(e))?;
        p.sprites = convert(sprites, |e| self.sprite(e))?;
        p.backgrounds = convert(backgrounds, |e| self.background(e))?;
        p.paths = convert(paths, |e| self.path(e, &refs))?;
        p.scripts = convert(scripts, |e| self.script(e))?;
        p.fonts = convert(fonts, |e| from_table(e.t, &Font::default(), &e.rel))?;
        p.timelines = convert(timelines, |e| self.timeline(e, &refs))?;
        p.objects = convert(objects, |e| self.object(e, &refs))?;
        p.rooms = convert(rooms, |e| self.room(e, &refs))?;
        p.triggers = convert(triggers, |e| self.trigger(e))?;
        p.included_files = self.included_files()?;

        p.room_order = match room_order {
            Some(a) => {
                let mut v = a
                    .iter()
                    .map(|x| {
                        refs.rooms
                            .resolve_value(x)
                            .map_err(|m| folder_err(pt, format!("key `room_order`: {m}")))
                    })
                    .collect::<Result<Vec<_>>>()?;
                v.extend(new.rooms.iter().copied());
                v
            }
            None => (0..p.rooms.len() as i32)
                .filter(|&i| p.rooms[i as usize].is_some())
                .collect(),
        };
        p.resource_tree = self.tree(&p, &refs)?;
        add_new_leaves(&mut p, &new);
        Ok((p, version))
    }

    fn settings_blobs(&self, st: &mut Settings) -> Result<()> {
        let files = self.list("settings")?;
        let find = |stem: &str| -> Result<Option<Vec<u8>>> {
            let hits: Vec<&String> = files
                .iter()
                .filter(|(n, d)| !d && split_first_dot(n).0 == stem)
                .map(|(n, _)| n)
                .collect();
            match hits.as_slice() {
                [] => Ok(None),
                [one] => self.bytes(&format!("settings/{one}")).map(Some),
                _ => Err(folder_err(
                    "settings",
                    format!("several files named {stem}.*; keep only one"),
                )),
            }
        };
        st.icon = find("icon")?.unwrap_or_default();
        st.loading_bar_back = find("loading_bar_back")?;
        st.loading_bar_front = find("loading_bar_front")?;
        st.custom_load_image = find("load_image")?;
        Ok(())
    }

    // -------------------------------------------------------------- resources

    fn sprite(&self, mut e: Entry) -> Result<Sprite> {
        let frames = take_int(&mut e.t, "frames", &e.rel)?;
        let mut empty = BTreeMap::new();
        for v in take_array(&mut e.t, "empty_frames", &e.rel)?.unwrap_or_default() {
            let ok = match &v {
                Value::Array(a) if a.len() == 3 => {
                    let n: Vec<i64> = a.iter().filter_map(|x| x.as_integer()).collect();
                    (n.len() == 3).then(|| (n[0], n[1] as u32, n[2] as u32))
                }
                _ => None,
            };
            let (k, w, h) = ok.ok_or_else(|| {
                folder_err(
                    &e.rel,
                    "key `empty_frames`: expected [[frame, width, height], ...]",
                )
            })?;
            empty.insert(k as usize, (w, h));
        }
        let mut s: Sprite = from_table(e.t, &Sprite::default(), &e.rel)?;
        let dir = &e.stem;
        let pngs: HashSet<usize> = self
            .list(dir)?
            .iter()
            .filter_map(|(n, d)| {
                if *d {
                    None
                } else {
                    n.strip_suffix(".png")?.parse().ok()
                }
            })
            .collect();
        let n = match frames {
            Some(n) => n.max(0) as usize,
            None => (0..)
                .find(|k| !pngs.contains(k) && !empty.contains_key(k))
                .unwrap_or(0),
        };
        if let Some(extra) = pngs.iter().filter(|&&k| k >= n).min() {
            return Err(folder_err(
                format!("{dir}/{extra}.png"),
                format!("not used: the sprite has {n} frames (`frames` in sprite.toml)"),
            ));
        }
        for k in 0..n {
            s.frames.push(match empty.get(&k) {
                Some(&(width, height)) => RgbaImage {
                    width,
                    height,
                    data: Vec::new(),
                },
                None => {
                    let rel = format!("{dir}/{k}.png");
                    if !pngs.contains(&k) {
                        return Err(folder_err(rel, "missing frame image"));
                    }
                    decode_png(&self.bytes(&rel)?, &rel)?
                }
            });
        }
        Ok(s)
    }

    fn background(&self, mut e: Entry) -> Result<Background> {
        let empty = take_array(&mut e.t, "empty_image", &e.rel)?;
        let mut b: Background = from_table(e.t, &Background::default(), &e.rel)?;
        let png = format!("backgrounds/{}.png", e.stem);
        if self.exists(&png) {
            b.image = decode_png(&self.bytes(&png)?, &png)?;
        } else if let Some(a) = empty {
            let n: Vec<i64> = a.iter().filter_map(|x| x.as_integer()).collect();
            if n.len() != 2 {
                return Err(folder_err(
                    &e.rel,
                    "key `empty_image`: expected [width, height]",
                ));
            }
            b.image = RgbaImage {
                width: n[0] as u32,
                height: n[1] as u32,
                data: Vec::new(),
            };
        }
        Ok(b)
    }

    fn sound(&self, mut e: Entry) -> Result<Sound> {
        let data = take_str(&mut e.t, "data", &e.rel)?;
        let mut s: Sound = from_table(e.t, &Sound::default(), &e.rel)?;
        if let Some(f) = data {
            s.data = Some(self.bytes(&format!("sounds/{f}"))?);
        }
        Ok(s)
    }

    fn path(&self, mut e: Entry, refs: &Refs) -> Result<GmPath> {
        resolve_key(&mut e.t, "backdrop_room", &refs.rooms, &e.rel)?;
        from_table(e.t, &GmPath::default(), &e.rel)
    }

    fn script(&self, mut e: Entry) -> Result<Script> {
        let eol = take_eol(&mut e.t, "eol", &e.rel)?;
        let mut s: Script = from_table(e.t, &Script::default(), &e.rel)?;
        let gml = format!("scripts/{}.gml", e.stem);
        if self.exists(&gml) {
            s.code = self.text(&gml, eol)?;
        }
        Ok(s)
    }

    fn trigger(&self, mut e: Entry) -> Result<Trigger> {
        let eol = take_eol(&mut e.t, "eol", &e.rel)?;
        let mut s: Trigger = from_table(e.t, &Trigger::default(), &e.rel)?;
        let gml = format!("triggers/{}.gml", e.stem);
        if self.exists(&gml) {
            s.condition = self.text(&gml, eol)?;
        }
        Ok(s)
    }

    fn included_files(&self) -> Result<Vec<IncludedFile>> {
        let mut v: Vec<(i32, IncludedFile)> = Vec::new();
        let mut next = 0;
        let entries = self.file_entries("included_files", None)?;
        let max = entries.iter().filter_map(|e| e.index).max().unwrap_or(-1);
        for mut e in entries {
            e.t.remove("name");
            let data = take_str(&mut e.t, "data", &e.rel)?;
            let mut f: IncludedFile = from_table(e.t, &IncludedFile::default(), &e.rel)?;
            if let Some(d) = data {
                f.data = Some(self.bytes(&format!("included_files/{d}"))?);
            }
            let i = e.index.unwrap_or_else(|| {
                next += 1;
                max + next
            });
            if v.iter().any(|(j, _)| *j == i) {
                return Err(folder_err(&e.rel, format!("index {i} is used twice")));
            }
            v.push((i, f));
        }
        v.sort_by_key(|(i, _)| *i);
        Ok(v.into_iter().map(|(_, f)| f).collect())
    }

    // -------------------------------------------------------------- actions

    /// Files of an event / moment directory that hold action lists, as
    /// (stem, plain .gml?) in directory order; auxiliary code files are skipped.
    fn action_files(&self, dir: &str, meta: &str) -> Result<Vec<(String, String)>> {
        let mut out = Vec::new();
        for (name, is_dir) in self.list(dir)? {
            if is_dir || name == meta {
                continue;
            }
            let (stem, ext) = split_first_dot(&name);
            if ext == ".gml" || ext == ".actions.toml" {
                out.push((stem.to_string(), name.clone()));
            } else if ext.ends_with(".gml") || ext.ends_with(".toml") {
                // `<stem>.<i>.gml` code files are referenced from .actions.toml.
                let aux = ext.ends_with(".gml")
                    && ext[1..ext.len() - 4].bytes().all(|b| b.is_ascii_digit());
                if !aux {
                    return Err(folder_err(
                        format!("{dir}/{name}"),
                        "unexpected file name (expected <event>.gml or <event>.actions.toml)",
                    ));
                }
            }
        }
        Ok(out)
    }

    /// The actions of `<dir>/<stem>`: a plain `.gml` file or an `.actions.toml`.
    fn action_list(
        &self,
        dir: &str,
        stem: &str,
        eols: &mut Table,
        refs: &Refs,
    ) -> Result<Vec<Action>> {
        let gml = format!("{dir}/{stem}.gml");
        let toml_rel = format!("{dir}/{stem}.actions.toml");
        let eol = take_eol(eols, &format!("{stem}.gml"), &format!("{dir} [eol]"))?;
        match (self.exists(&gml), self.exists(&toml_rel)) {
            (true, true) => Err(folder_err(
                gml,
                format!("both {stem}.gml and {stem}.actions.toml exist; keep one"),
            )),
            (true, false) => {
                let mut a = action_template(1, 603);
                a.args[0] = self.text(&gml, eol)?;
                Ok(vec![a])
            }
            (false, true) => {
                let mut t = self.toml(&toml_rel)?;
                let list = take_tables(&mut t, "actions", &toml_rel)?.unwrap_or_default();
                no_extra_keys(&t, &toml_rel)?;
                list.into_iter()
                    .enumerate()
                    .map(|(i, a)| {
                        self.action(dir, a, &format!("{toml_rel} action {}", i + 1), refs)
                    })
                    .collect()
            }
            (false, false) => Err(folder_err(
                gml,
                format!("missing: neither {stem}.gml nor {stem}.actions.toml exists"),
            )),
        }
    }

    fn action(&self, dir: &str, mut t: Table, path: &str, refs: &Refs) -> Result<Action> {
        let lib = take_int(&mut t, "lib", path)?
            .ok_or_else(|| folder_err(path, "missing `lib` (library id, 1 for built-ins)"))?;
        let id = take_int(&mut t, "id", path)?
            .ok_or_else(|| folder_err(path, "missing `id` (action id)"))?;
        let tmpl = action_template(lib, id);
        let applies_to = match t.remove("applies_to") {
            None => APPLIES_TO_SELF,
            Some(Value::String(s)) if s == "self" => APPLIES_TO_SELF,
            Some(Value::String(s)) if s == "other" => APPLIES_TO_OTHER,
            Some(v) => refs
                .objects
                .resolve_value(&v)
                .map_err(|m| folder_err(path, format!("key `applies_to`: {m}")))?,
        };
        let relative = take_bool(&mut t, "relative", path)?.unwrap_or(false);
        let negate = take_bool(&mut t, "negate", path)?.unwrap_or(false);
        let code_eol = take_eol(&mut t, "code_eol", path)?;
        let code_file = take_str(&mut t, "code_file", path)?;
        let args = take_strings(&mut t, "args", path)?.unwrap_or_default();
        let tail = take_strings(&mut t, "args_tail", path)?;
        if let Some(k) = t.keys().find(|k| !ACTION_META.contains(&k.as_str())) {
            return Err(folder_err(path, format!("unknown key `{k}`")));
        }
        let mut a: Action = from_table(t, &tmpl, path)?;
        a.applies_to = applies_to;
        a.relative = relative;
        a.negate = negate;
        a.args.clear();
        if let Some(f) = code_file {
            a.args.push(self.text(&format!("{dir}/{f}"), code_eol)?);
        }
        a.args.extend(args);
        let n = a.args.len();
        match tail {
            Some(tail) => a.args.extend(tail),
            None => a.args.extend(std::iter::repeat_n(
                "0".to_string(),
                tmpl.args.len().saturating_sub(n),
            )),
        }
        Ok(a)
    }

    /// Order the action files of a directory: listed stems first, then the rest by key.
    fn ordered_stems(
        &self,
        dir: &str,
        meta: &str,
        listed: Option<Vec<String>>,
        key: impl Fn(&str) -> Result<(i32, i32)>,
    ) -> Result<Vec<String>> {
        let found = self.action_files(dir, meta)?;
        let mut stems: Vec<String> = Vec::new();
        let mut seen = HashSet::new();
        for s in listed.unwrap_or_default() {
            if !seen.insert(s.to_lowercase()) {
                return Err(folder_err(
                    format!("{dir}/{meta}"),
                    format!("{s:?} is listed twice"),
                ));
            }
            stems.push(s);
        }
        let mut extra: Vec<((i32, i32), String)> = Vec::new();
        for (stem, _) in found {
            if seen.insert(stem.to_lowercase()) {
                extra.push((key(&stem)?, stem));
            }
        }
        extra.sort();
        stems.extend(extra.into_iter().map(|(_, s)| s));
        Ok(stems)
    }

    fn timeline(&self, mut e: Entry, refs: &Refs) -> Result<Timeline> {
        let listed = take_strings(&mut e.t, "moments", &e.rel)?;
        let mut eols = take_table(&mut e.t, "eol", &e.rel)?.unwrap_or_default();
        let mut tl: Timeline = from_table(e.t, &Timeline::default(), &e.rel)?;
        let dir = &e.stem;
        let step = |stem: &str| {
            names::parse_moment_stem(names::strip_dup_suffix(stem)).ok_or_else(|| {
                folder_err(
                    format!("{dir}/{stem}"),
                    "not a moment name (expected step_<number>)",
                )
            })
        };
        let stems = self.ordered_stems(dir, "timeline.toml", listed, |s| Ok((step(s)?, 0)))?;
        for stem in stems {
            tl.moments.push(Moment {
                step: step(&stem)?,
                actions: self.action_list(dir, &stem, &mut eols, refs)?,
            });
        }
        no_extra_keys(&eols, &format!("{} [eol]", e.rel))?;
        Ok(tl)
    }

    fn object(&self, mut e: Entry, refs: &Refs) -> Result<Object> {
        resolve_key(&mut e.t, "sprite", &refs.sprites, &e.rel)?;
        resolve_key(&mut e.t, "mask_sprite", &refs.sprites, &e.rel)?;
        resolve_key(&mut e.t, "parent", &refs.objects, &e.rel)?;
        let listed = take_strings(&mut e.t, "events", &e.rel)?;
        let mut eols = take_table(&mut e.t, "eol", &e.rel)?.unwrap_or_default();
        let mut o: Object = from_table(e.t, &Object::default(), &e.rel)?;
        let dir = &e.stem;
        let event = |stem: &str| -> Result<(i32, i32)> {
            let bad = |m: String| folder_err(format!("{dir}/{stem}"), m);
            let name = names::parse_event_stem(names::strip_dup_suffix(stem))
                .ok_or_else(|| bad("not an event name (see docs/project-folder.md)".into()))?;
            Ok(match name {
                EventName::Fixed(t, n) => (t, n),
                EventName::Collision(r) => (
                    event_type::COLLISION,
                    refs.objects.resolve(&r).map_err(bad)?,
                ),
                EventName::Trigger(r) => {
                    (event_type::TRIGGER, refs.triggers.resolve(&r).map_err(bad)?)
                }
            })
        };
        let stems = self.ordered_stems(dir, "object.toml", listed, event)?;
        for stem in stems {
            let (event_type, number) = event(&stem)?;
            o.events.push(Event {
                event_type,
                number,
                actions: self.action_list(dir, &stem, &mut eols, refs)?,
            });
        }
        no_extra_keys(&eols, &format!("{} [eol]", e.rel))?;
        Ok(o)
    }

    fn room(&self, mut e: Entry, refs: &Refs) -> Result<Room> {
        let rel = e.rel.clone();
        let creation_eol = take_eol(&mut e.t, "creation_eol", &rel)?;
        let bgs = take_tables(&mut e.t, "backgrounds", &rel)?;
        let views = take_tables(&mut e.t, "views", &rel)?;
        let insts = take_tables(&mut e.t, "instances", &rel)?.unwrap_or_default();
        let tiles = take_tables(&mut e.t, "tiles", &rel)?.unwrap_or_default();
        let mut r: Room = from_table(e.t, &Room::default(), &rel)?;
        let dir = &e.stem;
        let creation = format!("{dir}/creation.gml");
        if self.exists(&creation) {
            r.creation_code = self.text(&creation, creation_eol)?;
        }
        if let Some(bgs) = bgs {
            r.backgrounds = bgs
                .into_iter()
                .enumerate()
                .map(|(i, mut b)| {
                    let at = format!("{rel} backgrounds[{i}]");
                    resolve_key(&mut b, "background", &refs.backgrounds, &at)?;
                    from_table(b, &RoomBackground::default(), &at)
                })
                .collect::<Result<_>>()?;
        }
        if let Some(views) = views {
            r.views = views
                .into_iter()
                .enumerate()
                .map(|(i, mut v)| {
                    let at = format!("{rel} views[{i}]");
                    resolve_key(&mut v, "follow_object", &refs.objects, &at)?;
                    from_table(v, &View::default(), &at)
                })
                .collect::<Result<_>>()?;
        }
        for (i, mut x) in insts.into_iter().enumerate() {
            let at = format!("{rel} instances[{i}]");
            if !x.contains_key("object") {
                return Err(folder_err(at, "missing `object`"));
            }
            resolve_key(&mut x, "object", &refs.objects, &at)?;
            let code_eol = take_eol(&mut x, "code_eol", &at)?;
            let code = take_str(&mut x, "code", &at)?;
            let mut inst: Instance = from_table(x, &Instance::default(), &at)?;
            if let Some(f) = code {
                inst.creation_code = self.text(&format!("{dir}/{f}"), code_eol)?;
            }
            r.instances.push(inst);
        }
        for (i, mut x) in tiles.into_iter().enumerate() {
            let at = format!("{rel} tiles[{i}]");
            if !x.contains_key("background") {
                return Err(folder_err(at, "missing `background`"));
            }
            resolve_key(&mut x, "background", &refs.backgrounds, &at)?;
            r.tiles.push(from_table(x, &Tile::default(), &at)?);
        }
        Ok(r)
    }

    // -------------------------------------------------------------- resource tree

    fn tree(&self, p: &Project, refs: &Refs) -> Result<Vec<TreeNode>> {
        let rel = "resource_tree.txt";
        if !self.exists(rel) {
            return Ok(default_tree(p));
        }
        let text = decode_text(&self.bytes(rel)?, Eol::Lf, rel)?;
        // Stack of (depth, node) under construction.
        let mut roots: Vec<TreeNode> = Vec::new();
        let mut stack: Vec<TreeNode> = Vec::new();
        fn pop_to(stack: &mut Vec<TreeNode>, roots: &mut Vec<TreeNode>, depth: usize) {
            while stack.len() > depth {
                let n = stack.pop().expect("non-empty");
                match stack.last_mut() {
                    Some(parent) => parent.children.push(n),
                    None => roots.push(n),
                }
            }
        }
        for (ln, line) in text.lines().enumerate() {
            let at = format!("{rel} line {}", ln + 1);
            let body = line.trim_start_matches(' ').trim_end();
            if body.is_empty() || body.starts_with('#') {
                continue;
            }
            let indent = line.len() - body.len();
            if indent % 2 != 0 {
                return Err(folder_err(at, "indentation must be a multiple of 2 spaces"));
            }
            let depth = indent / 2;
            if depth > stack.len() {
                return Err(folder_err(
                    at,
                    "indented more than one level below its parent",
                ));
            }
            pop_to(&mut stack, &mut roots, depth);
            let parent_kind = stack.last().map(|n| n.kind);
            let node = if body.starts_with('{') {
                let t: Table = format!("n = {body}")
                    .parse::<Table>()
                    .ok()
                    .and_then(|mut t| match t.remove("n") {
                        Some(Value::Table(x)) => Some(x),
                        _ => None,
                    })
                    .ok_or_else(|| folder_err(&at, "bad `{ ... }` node"))?;
                let mut t = t;
                let status = take_int(&mut t, "status", &at)?.unwrap_or(3);
                let kind = match take_int(&mut t, "kind", &at)? {
                    Some(k) => ResourceKind::from_i32(k),
                    None => parent_kind.ok_or_else(|| folder_err(&at, "missing `kind`"))?,
                };
                let name = take_str(&mut t, "name", &at)?.unwrap_or_default();
                let index = match t.remove("index") {
                    None => 0,
                    Some(v) => match refs.kind(kind) {
                        Some(r) => r.resolve_value(&v).map_err(|m| folder_err(&at, m))?,
                        None => v
                            .as_integer()
                            .and_then(|i| i32::try_from(i).ok())
                            .ok_or_else(|| folder_err(&at, "key `index` must be an integer"))?,
                    },
                };
                no_extra_keys(&t, &at)?;
                TreeNode {
                    status: NodeStatus::from_i32(status),
                    kind,
                    index,
                    name,
                    children: Vec::new(),
                }
            } else {
                match parent_kind {
                    None => TreeNode::standard_roots()
                        .into_iter()
                        .find(|r| r.name == body)
                        .ok_or_else(|| {
                            folder_err(
                                &at,
                                format!("{body:?} is not a standard root (Sprites, Sounds, ...)"),
                            )
                        })?,
                    Some(kind) => {
                        if let Some(g) = body.strip_suffix('/') {
                            TreeNode::group(kind, g, Vec::new())
                        } else {
                            let r = refs.kind(kind).ok_or_else(|| {
                                folder_err(&at, "this part of the tree holds no resources")
                            })?;
                            let index = r.resolve(body).map_err(|m| folder_err(&at, m))?;
                            TreeNode::leaf(kind, index, body)
                        }
                    }
                }
            };
            stack.push(node);
        }
        pop_to(&mut stack, &mut roots, 0);
        Ok(roots)
    }
}

// ------------------------------------------------------------------ slots

struct Slots {
    slots: Vec<Option<Entry>>,
    /// Slots given to entries without an `index`.
    new: Vec<i32>,
}

fn assign(entries: Vec<Entry>, min_slots: usize) -> Result<Slots> {
    let mut slots: Vec<Option<Entry>> = Vec::new();
    slots.resize_with(min_slots, || None);
    let mut fresh = Vec::new();
    for e in entries {
        match e.index {
            Some(i) => {
                let i = i as usize;
                if i >= slots.len() {
                    slots.resize_with(i + 1, || None);
                }
                if let Some(other) = &slots[i] {
                    return Err(folder_err(
                        &e.rel,
                        format!("index {i} is also used by {}", other.rel),
                    ));
                }
                slots[i] = Some(e);
            }
            None => fresh.push(e),
        }
    }
    let mut new = Vec::new();
    for e in fresh {
        new.push(slots.len() as i32);
        slots.push(Some(e));
    }
    Ok(Slots { slots, new })
}

fn convert<T>(s: Slots, mut f: impl FnMut(Entry) -> Result<T>) -> Result<Vec<Option<T>>> {
    s.slots
        .into_iter()
        .map(|e| e.map(&mut f).transpose())
        .collect()
}

struct NewSlots {
    sprites: Vec<i32>,
    sounds: Vec<i32>,
    backgrounds: Vec<i32>,
    paths: Vec<i32>,
    scripts: Vec<i32>,
    fonts: Vec<i32>,
    timelines: Vec<i32>,
    objects: Vec<i32>,
    rooms: Vec<i32>,
}

/// Name of slot `i` of a resource list.
fn slot_name<T>(v: &[Option<T>], i: i32, name: impl Fn(&T) -> &str) -> String {
    v.get(i as usize)
        .and_then(|x| x.as_ref())
        .map(|x| name(x).to_string())
        .unwrap_or_default()
}

/// Leaves for resources that came without an `index`, under their kind's root.
fn add_new_leaves(p: &mut Project, new: &NewSlots) {
    use ResourceKind as K;
    let mut leaves: Vec<(K, i32, String)> = Vec::new();
    for &i in &new.sprites {
        leaves.push((K::Sprite, i, slot_name(&p.sprites, i, |x| &x.name)));
    }
    for &i in &new.sounds {
        leaves.push((K::Sound, i, slot_name(&p.sounds, i, |x| &x.name)));
    }
    for &i in &new.backgrounds {
        leaves.push((K::Background, i, slot_name(&p.backgrounds, i, |x| &x.name)));
    }
    for &i in &new.paths {
        leaves.push((K::Path, i, slot_name(&p.paths, i, |x| &x.name)));
    }
    for &i in &new.scripts {
        leaves.push((K::Script, i, slot_name(&p.scripts, i, |x| &x.name)));
    }
    for &i in &new.fonts {
        leaves.push((K::Font, i, slot_name(&p.fonts, i, |x| &x.name)));
    }
    for &i in &new.timelines {
        leaves.push((K::Timeline, i, slot_name(&p.timelines, i, |x| &x.name)));
    }
    for &i in &new.objects {
        leaves.push((K::Object, i, slot_name(&p.objects, i, |x| &x.name)));
    }
    for &i in &new.rooms {
        leaves.push((K::Room, i, slot_name(&p.rooms, i, |x| &x.name)));
    }
    for (kind, i, name) in leaves {
        let root = p
            .resource_tree
            .iter_mut()
            .find(|r| r.kind == kind && r.status == NodeStatus::Root);
        if let Some(root) = root {
            let already = {
                let mut found = false;
                root.walk(&mut |n, _| found |= n.status == NodeStatus::Leaf && n.index == i);
                found
            };
            if !already {
                root.children.push(TreeNode::leaf(kind, i, name));
            }
        }
    }
}

/// The standard roots with a leaf for every resource (when there is no tree file).
fn default_tree(p: &Project) -> Vec<TreeNode> {
    use ResourceKind as K;
    let mut roots = TreeNode::standard_roots();
    fn fill<T>(root: &mut TreeNode, v: &[Option<T>], name: impl Fn(&T) -> &str) {
        for (i, x) in v.iter().enumerate() {
            if let Some(x) = x {
                root.children
                    .push(TreeNode::leaf(root.kind, i as i32, name(x)));
            }
        }
    }
    for root in &mut roots {
        match root.kind {
            K::Sprite => fill(root, &p.sprites, |x| &x.name),
            K::Sound => fill(root, &p.sounds, |x| &x.name),
            K::Background => fill(root, &p.backgrounds, |x| &x.name),
            K::Path => fill(root, &p.paths, |x| &x.name),
            K::Script => fill(root, &p.scripts, |x| &x.name),
            K::Font => fill(root, &p.fonts, |x| &x.name),
            K::Timeline => fill(root, &p.timelines, |x| &x.name),
            K::Object => fill(root, &p.objects, |x| &x.name),
            K::Room => fill(root, &p.rooms, |x| &x.name),
            _ => {}
        }
    }
    roots
}
