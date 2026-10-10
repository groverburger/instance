//! Project → folder.

use std::collections::BTreeMap;
use std::path::Path;

use toml::{Table, Value};

use super::emit;
use super::names::{self, sanitize, Names};
use super::*;
use crate::model::*;

/// Relative path (with `/`) → file contents.
pub type FileMap = BTreeMap<String, Vec<u8>>;

/// Write `p` as a project folder (format version recorded as 8.1).
///
/// `dir` is created if needed. Files of a previous unpack that are no longer part of
/// the project are removed from the resource directories; anything else in `dir` is
/// left alone. Files whose contents did not change are not rewritten.
pub fn write_folder(p: &Project, dir: &Path) -> Result<()> {
    write_folder_versioned(p, dir, FormatVersion::V810)
}

/// [`write_folder`] recording `version` as the format to pack to by default.
pub fn write_folder_versioned(p: &Project, dir: &Path, version: FormatVersion) -> Result<()> {
    let files = folder_files(p, version)?;
    sync(&files, dir)
}

/// The files of the folder form of `p`, without touching the disk.
pub fn folder_files(p: &Project, version: FormatVersion) -> Result<FileMap> {
    let mut w = W {
        refs: Refs::of(p),
        files: FileMap::new(),
    };
    w.project(p, version)?;
    w.tree(p);
    w.settings_blobs(&p.settings);
    w.sprites(p)?;
    w.sounds(p);
    w.backgrounds(p)?;
    w.paths(p);
    w.scripts(p);
    w.fonts(p);
    w.timelines(p);
    w.objects(p);
    w.rooms(p);
    w.triggers(p);
    w.included_files(p);
    Ok(w.files)
}

/// Write `files` under `dir` and remove stale files from the resource directories.
fn sync(files: &FileMap, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    for (rel, data) in files {
        let path = dir.join(rel);
        if std::fs::read(&path).ok().as_deref() == Some(data.as_slice()) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, data)?;
    }
    let attrs = dir.join(".gitattributes");
    if !attrs.exists() {
        // Keep checkouts byte-exact (no line-ending conversion of .gml / .rtf files).
        std::fs::write(&attrs, "* -text\n")?;
    }
    for kind in KIND_DIRS {
        prune(&dir.join(kind), kind, files)?;
    }
    Ok(())
}

/// Remove files under `path` (relative name `rel`) not in `keep`, then empty dirs.
fn prune(path: &Path, rel: &str, keep: &FileMap) -> Result<bool> {
    let Ok(rd) = std::fs::read_dir(path) else {
        return Ok(true);
    };
    let mut empty = true;
    for e in rd {
        let e = e?;
        let name = e.file_name().to_string_lossy().to_string();
        let sub = format!("{rel}/{name}");
        if e.file_type()?.is_dir() {
            if prune(&e.path(), &sub, keep)? {
                std::fs::remove_dir(e.path())?;
            } else {
                empty = false;
            }
        } else if keep.contains_key(&sub) {
            empty = false;
        } else {
            std::fs::remove_file(e.path())?;
        }
    }
    Ok(empty)
}

struct W {
    refs: Refs,
    files: FileMap,
}

fn int(i: impl Into<i64>) -> Value {
    Value::Integer(i.into())
}

fn s(x: impl Into<String>) -> Value {
    Value::String(x.into())
}

impl W {
    fn put(&mut self, path: String, data: Vec<u8>) {
        self.files.insert(path, data);
    }

    fn put_toml(&mut self, path: String, t: &Table) {
        self.put(path, emit::document(t).into_bytes());
    }

    /// Write a text piece; returns its line-ending mode when it is not the default.
    fn put_text(&mut self, path: String, text: &str) -> Option<&'static str> {
        let eol = Eol::of(text);
        self.put(path, eol.to_disk(text).into_bytes());
        (eol != Eol::Crlf).then_some(eol.name())
    }

    // -------------------------------------------------------------- project.toml

    fn project(&mut self, p: &Project, version: FormatVersion) -> Result<()> {
        let mut t = Table::new();
        t.insert("format_version".into(), int(version_number(version)));
        t.insert("game_id".into(), int(p.game_id));
        let guid: String = p.guid.iter().map(|b| format!("{b:02x}")).collect();
        t.insert("guid".into(), s(guid));
        t.insert("last_instance_id".into(), int(p.last_instance_id));
        t.insert("last_tile_id".into(), int(p.last_tile_id));
        t.insert(
            "triggers_last_changed".into(),
            Value::Float(p.triggers_last_changed),
        );
        t.insert(
            "constants_last_changed".into(),
            Value::Float(p.constants_last_changed),
        );
        let order = p
            .room_order
            .iter()
            .map(|&r| s(self.refs.rooms.r(r)))
            .collect();
        t.insert("room_order".into(), Value::Array(order));
        t.insert(
            "extension_packages".into(),
            Value::Array(p.extension_packages.iter().map(s).collect()),
        );
        t.insert(
            "library_init_code".into(),
            Value::Array(p.library_init_code.iter().map(s).collect()),
        );
        let constants = p
            .constants
            .iter()
            .map(|c| Value::Table(to_table(c)))
            .collect();
        t.insert("constants".into(), Value::Array(constants));
        let mut slots = Table::new();
        for (name, count) in p.slot_counts() {
            slots.insert(name.into(), int(count.slots as i64));
        }
        slots.insert("triggers".into(), int(p.triggers.len() as i64));
        t.insert("slots".into(), Value::Table(slots));
        let mut st = p.settings.clone();
        st.icon.clear();
        st.loading_bar_back = None;
        st.loading_bar_front = None;
        st.custom_load_image = None;
        let mut st = to_table(&st);
        st.remove("icon");
        t.insert("settings".into(), Value::Table(st));
        let mut gi = to_table(&p.game_info);
        gi.remove("rtf");
        if let Some(eol) = self.put_text("game_information.rtf".into(), &p.game_info.rtf) {
            gi.insert("rtf_eol".into(), s(eol));
        }
        t.insert("game_information".into(), Value::Table(gi));
        self.put_toml("project.toml".into(), &t);
        Ok(())
    }

    fn settings_blobs(&mut self, st: &Settings) {
        if !st.icon.is_empty() {
            self.put("settings/icon.ico".into(), st.icon.clone());
        }
        for (stem, blob) in [
            ("loading_bar_back", &st.loading_bar_back),
            ("loading_bar_front", &st.loading_bar_front),
            ("load_image", &st.custom_load_image),
        ] {
            if let Some(b) = blob {
                self.put(format!("settings/{stem}{}", blob_ext(b)), b.clone());
            }
        }
    }

    // -------------------------------------------------------------- resource tree

    fn tree(&mut self, p: &Project) {
        let mut out = String::from(
            "# Resource tree: one node per line, 2 spaces per level. `Name/` is a group;\n\
             # `{ ... }` spells out a node that has no short form.\n",
        );
        for root in &p.resource_tree {
            self.tree_node(&mut out, root, None, 0);
        }
        self.put("resource_tree.txt".into(), out.into_bytes());
    }

    fn tree_node(
        &self,
        out: &mut String,
        n: &TreeNode,
        parent: Option<ResourceKind>,
        depth: usize,
    ) {
        out.push_str(&"  ".repeat(depth));
        out.push_str(&self.tree_line(n, parent));
        out.push('\n');
        for c in &n.children {
            self.tree_node(out, c, Some(n.kind), depth + 1);
        }
    }

    fn tree_line(&self, n: &TreeNode, parent: Option<ResourceKind>) -> String {
        let plain = tree_plain_safe(&n.name);
        match parent {
            None => {
                let std = TreeNode::standard_roots();
                let is_std = std.iter().any(|r| {
                    r.name == n.name && r.kind == n.kind && r.status == n.status && n.index == 0
                });
                if is_std && plain {
                    return n.name.clone();
                }
            }
            Some(pk) if n.kind == pk && plain => match n.status {
                NodeStatus::Group if n.index == 0 => return format!("{}/", n.name),
                NodeStatus::Leaf if n.children.is_empty() => {
                    let named = self
                        .refs
                        .kind(n.kind)
                        .and_then(|r| r.name_of(n.index))
                        .is_some_and(|x| x == n.name);
                    if named {
                        return n.name.clone();
                    }
                }
                _ => {}
            },
            _ => {}
        }
        let mut t = Table::new();
        t.insert("status".into(), int(n.status.to_i32()));
        t.insert("kind".into(), int(n.kind.to_i32()));
        t.insert("index".into(), int(n.index));
        t.insert("name".into(), s(n.name.clone()));
        emit::inline(&Value::Table(t))
    }

    // -------------------------------------------------------------- resources

    fn sprites(&mut self, p: &Project) -> Result<()> {
        let mut names = Names::default();
        for (i, sp) in p.sprites.iter().enumerate() {
            let Some(sp) = sp else { continue };
            let dir = format!("sprites/{}", names.stem(&sanitize(&sp.name), &[""]));
            let mut t = to_table(&Sprite {
                frames: Vec::new(),
                ..sp.clone()
            });
            t.remove("frames");
            t.insert("index".into(), int(i as i64));
            t.insert("frames".into(), int(sp.frames.len() as i64));
            let mut empty = Vec::new();
            for (k, f) in sp.frames.iter().enumerate() {
                if f.width == 0 || f.height == 0 {
                    empty.push(Value::Array(vec![
                        int(k as i64),
                        int(f.width),
                        int(f.height),
                    ]));
                } else {
                    let what = format!("sprite {} frame {k}", sp.name);
                    self.put(format!("{dir}/{k}.png"), encode_png(f, &what)?);
                }
            }
            if !empty.is_empty() {
                t.insert("empty_frames".into(), Value::Array(empty));
            }
            self.put_toml(format!("{dir}/sprite.toml"), &t);
        }
        Ok(())
    }

    fn sounds(&mut self, p: &Project) {
        let mut names = Names::default();
        for (i, sn) in p.sounds.iter().enumerate() {
            let Some(sn) = sn else { continue };
            let ft = &sn.file_type;
            let ext = if ft.len() >= 2
                && ft.len() <= 9
                && ft.starts_with('.')
                && ft[1..].bytes().all(|b| b.is_ascii_alphanumeric())
            {
                ft.clone()
            } else {
                ".bin".into()
            };
            let stem = names.stem(&sanitize(&sn.name), &[".toml", &ext]);
            let mut t = to_table(&Sound {
                data: None,
                ..sn.clone()
            });
            t.insert("index".into(), int(i as i64));
            if let Some(d) = &sn.data {
                let f = format!("{stem}{ext}");
                t.insert("data".into(), s(f.clone()));
                self.put(format!("sounds/{f}"), d.clone());
            }
            self.put_toml(format!("sounds/{stem}.toml"), &t);
        }
    }

    fn backgrounds(&mut self, p: &Project) -> Result<()> {
        let mut names = Names::default();
        for (i, b) in p.backgrounds.iter().enumerate() {
            let Some(b) = b else { continue };
            let stem = names.stem(&sanitize(&b.name), &[".toml", ".png"]);
            let mut t = to_table(&Background {
                image: RgbaImage::default(),
                ..b.clone()
            });
            t.remove("image");
            t.insert("index".into(), int(i as i64));
            if b.image.width == 0 || b.image.height == 0 {
                if b.image.width != 0 || b.image.height != 0 {
                    t.insert(
                        "empty_image".into(),
                        Value::Array(vec![int(b.image.width), int(b.image.height)]),
                    );
                }
            } else {
                let what = format!("background {}", b.name);
                self.put(
                    format!("backgrounds/{stem}.png"),
                    encode_png(&b.image, &what)?,
                );
            }
            self.put_toml(format!("backgrounds/{stem}.toml"), &t);
        }
        Ok(())
    }

    fn paths(&mut self, p: &Project) {
        let mut names = Names::default();
        for (i, x) in p.paths.iter().enumerate() {
            let Some(x) = x else { continue };
            let stem = names.stem(&sanitize(&x.name), &[".toml"]);
            let mut t = to_table(x);
            t.insert("index".into(), int(i as i64));
            t.remove("backdrop_room");
            if x.backdrop_room != -1 {
                t.insert(
                    "backdrop_room".into(),
                    s(self.refs.rooms.r(x.backdrop_room)),
                );
            }
            self.put_toml(format!("paths/{stem}.toml"), &t);
        }
    }

    fn scripts(&mut self, p: &Project) {
        let mut names = Names::default();
        for (i, x) in p.scripts.iter().enumerate() {
            let Some(x) = x else { continue };
            let stem = names.stem(&sanitize(&x.name), &[".toml", ".gml"]);
            let mut t = to_table(x);
            t.remove("code");
            t.insert("index".into(), int(i as i64));
            if let Some(eol) = self.put_text(format!("scripts/{stem}.gml"), &x.code) {
                t.insert("eol".into(), s(eol));
            }
            self.put_toml(format!("scripts/{stem}.toml"), &t);
        }
    }

    fn fonts(&mut self, p: &Project) {
        let mut names = Names::default();
        for (i, x) in p.fonts.iter().enumerate() {
            let Some(x) = x else { continue };
            let stem = names.stem(&sanitize(&x.name), &[".toml"]);
            let mut t = to_table(x);
            t.insert("index".into(), int(i as i64));
            self.put_toml(format!("fonts/{stem}.toml"), &t);
        }
    }

    fn triggers(&mut self, p: &Project) {
        let mut names = Names::default();
        for (i, x) in p.triggers.iter().enumerate() {
            let Some(x) = x else { continue };
            let stem = names.stem(&sanitize(&x.name), &[".toml", ".gml"]);
            let mut t = to_table(x);
            t.remove("condition");
            t.insert("index".into(), int(i as i64));
            if let Some(eol) = self.put_text(format!("triggers/{stem}.gml"), &x.condition) {
                t.insert("eol".into(), s(eol));
            }
            self.put_toml(format!("triggers/{stem}.toml"), &t);
        }
    }

    fn included_files(&mut self, p: &Project) {
        let mut names = Names::default();
        for (i, x) in p.included_files.iter().enumerate() {
            let stem = names.stem(&sanitize(&x.file_name), &[".toml", ""]);
            let mut t = to_table(&IncludedFile {
                data: None,
                ..x.clone()
            });
            t.insert("index".into(), int(i as i64));
            if let Some(d) = &x.data {
                t.insert("data".into(), s(stem.clone()));
                self.put(format!("included_files/{stem}"), d.clone());
            }
            self.put_toml(format!("included_files/{stem}.toml"), &t);
        }
    }

    // -------------------------------------------------------------- actions

    /// Write the actions of one event / moment as `<stem>.gml` or
    /// `<stem>.actions.toml` (+ code files) in `dir`; returns the line-ending mode of
    /// a plain `.gml` file when it is not the default.
    fn action_list(&mut self, dir: &str, stem: &str, actions: &[Action]) -> Option<&'static str> {
        if let Some(code) = plain_code(actions) {
            return self.put_text(format!("{dir}/{stem}.gml"), code);
        }
        let mut list = Vec::new();
        for (i, a) in actions.iter().enumerate() {
            let tmpl = action_template(a.library_id, a.action_id);
            let mut t = Table::new();
            t.insert("lib".into(), int(a.library_id));
            t.insert("id".into(), int(a.action_id));
            match a.applies_to {
                APPLIES_TO_SELF => {}
                APPLIES_TO_OTHER => {
                    t.insert("applies_to".into(), s("other"));
                }
                o => {
                    t.insert("applies_to".into(), s(self.refs.objects.r(o)));
                }
            }
            if a.relative {
                t.insert("relative".into(), Value::Boolean(true));
            }
            if a.negate {
                t.insert("negate".into(), Value::Boolean(true));
            }
            let n = (a.argument_count.max(0) as usize).min(a.args.len());
            let code_file = a.kind == action_kind::CODE && n >= 1;
            let start = if code_file {
                let f = format!("{stem}.{i}.gml");
                if let Some(eol) = self.put_text(format!("{dir}/{f}"), &a.args[0]) {
                    t.insert("code_eol".into(), s(eol));
                }
                t.insert("code_file".into(), s(f));
                1
            } else {
                0
            };
            if n > start {
                t.insert(
                    "args".into(),
                    Value::Array(a.args[start..n].iter().map(s).collect()),
                );
            }
            let default_tail = vec!["0".to_string(); tmpl.args.len().saturating_sub(n)];
            if a.args[n..] != default_tail[..] {
                t.insert(
                    "args_tail".into(),
                    Value::Array(a.args[n..].iter().map(s).collect()),
                );
            }
            let meta = to_table(a);
            let tmeta = to_table(&tmpl);
            for k in ACTION_META {
                if meta.get(*k) != tmeta.get(*k) {
                    t.insert((*k).into(), meta[*k].clone());
                }
            }
            list.push(Value::Table(t));
        }
        let mut doc = Table::new();
        doc.insert("actions".into(), Value::Array(list));
        // A comment naming each action (comments are ignored when reading).
        let mut text = String::new();
        let mut k = 0;
        for line in emit::document(&doc).lines() {
            if line == "[[actions]]" {
                if let Some(a) = actions.get(k) {
                    text.push_str(&action_comment(a));
                }
                k += 1;
            }
            text.push_str(line);
            text.push('\n');
        }
        self.put(format!("{dir}/{stem}.actions.toml"), text.into_bytes());
        None
    }

    fn timelines(&mut self, p: &Project) {
        let mut dirs = Names::default();
        for (i, x) in p.timelines.iter().enumerate() {
            let Some(x) = x else { continue };
            let dir = format!("timelines/{}", dirs.stem(&sanitize(&x.name), &[""]));
            let mut t = to_table(&Timeline {
                moments: Vec::new(),
                ..x.clone()
            });
            t.insert("index".into(), int(i as i64));
            let mut files = Names::default();
            files.reserve("timeline.toml");
            let mut stems = Vec::new();
            let mut eols = Table::new();
            for m in &x.moments {
                let stem = files.stem(&names::moment_stem(m.step), &[".gml", ".actions.toml"]);
                if let Some(eol) = self.action_list(&dir, &stem, &m.actions) {
                    eols.insert(format!("{stem}.gml"), s(eol));
                }
                stems.push(s(stem));
            }
            t.insert("moments".into(), Value::Array(stems));
            if !eols.is_empty() {
                t.insert("eol".into(), Value::Table(eols));
            }
            self.put_toml(format!("{dir}/timeline.toml"), &t);
        }
    }

    fn objects(&mut self, p: &Project) {
        let mut dirs = Names::default();
        for (i, o) in p.objects.iter().enumerate() {
            let Some(o) = o else { continue };
            let dir = format!("objects/{}", dirs.stem(&sanitize(&o.name), &[""]));
            let mut t = to_table(&Object {
                events: Vec::new(),
                ..o.clone()
            });
            t.insert("index".into(), int(i as i64));
            t.remove("sprite");
            t.remove("parent");
            t.remove("mask_sprite");
            if o.sprite != -1 {
                t.insert("sprite".into(), s(self.refs.sprites.r(o.sprite)));
            }
            if o.parent != NO_PARENT {
                t.insert("parent".into(), s(self.refs.objects.r(o.parent)));
            }
            if o.mask_sprite != -1 {
                t.insert("mask_sprite".into(), s(self.refs.sprites.r(o.mask_sprite)));
            }
            let mut files = Names::default();
            files.reserve("object.toml");
            let mut stems = Vec::new();
            let mut eols = Table::new();
            for e in &o.events {
                let base = names::event_stem(
                    e.event_type,
                    e.number,
                    |n| self.refs.objects.ident(n),
                    |n| self.refs.triggers.ident(n),
                );
                let stem = files.stem(&base, &[".gml", ".actions.toml"]);
                if let Some(eol) = self.action_list(&dir, &stem, &e.actions) {
                    eols.insert(format!("{stem}.gml"), s(eol));
                }
                stems.push(s(stem));
            }
            t.insert("events".into(), Value::Array(stems));
            if !eols.is_empty() {
                t.insert("eol".into(), Value::Table(eols));
            }
            self.put_toml(format!("{dir}/object.toml"), &t);
        }
    }

    fn rooms(&mut self, p: &Project) {
        let mut names = Names::default();
        for (i, r) in p.rooms.iter().enumerate() {
            let Some(r) = r else { continue };
            let dir = format!("rooms/{}", names.stem(&sanitize(&r.name), &[""]));
            let mut t = to_table(&Room {
                creation_code: String::new(),
                backgrounds: Vec::new(),
                views: Vec::new(),
                instances: Vec::new(),
                tiles: Vec::new(),
                ..r.clone()
            });
            for k in [
                "creation_code",
                "backgrounds",
                "views",
                "instances",
                "tiles",
            ] {
                t.remove(k);
            }
            t.insert("index".into(), int(i as i64));
            if !r.creation_code.is_empty() {
                if let Some(eol) = self.put_text(format!("{dir}/creation.gml"), &r.creation_code) {
                    t.insert("creation_eol".into(), s(eol));
                }
            }
            let dbg = to_table(&RoomBackground::default());
            let bgs = r
                .backgrounds
                .iter()
                .map(|b| {
                    let mut x = diff_table(to_table(b), &dbg);
                    if x.remove("background").is_some() {
                        x.insert(
                            "background".into(),
                            s(self.refs.backgrounds.r(b.background)),
                        );
                    }
                    Value::Table(x)
                })
                .collect();
            t.insert("backgrounds".into(), Value::Array(bgs));
            let dview = to_table(&View::default());
            let views = r
                .views
                .iter()
                .map(|v| {
                    let mut x = diff_table(to_table(v), &dview);
                    if x.remove("follow_object").is_some() {
                        x.insert(
                            "follow_object".into(),
                            s(self.refs.objects.r(v.follow_object)),
                        );
                    }
                    Value::Table(x)
                })
                .collect();
            t.insert("views".into(), Value::Array(views));
            let mut codes = Names::default();
            let mut insts = Vec::new();
            for inst in &r.instances {
                let mut x = Table::new();
                x.insert("id".into(), int(inst.id));
                x.insert("object".into(), s(self.refs.objects.r(inst.object)));
                x.insert("x".into(), int(inst.x));
                x.insert("y".into(), int(inst.y));
                if inst.locked {
                    x.insert("locked".into(), Value::Boolean(true));
                }
                if !inst.creation_code.is_empty() {
                    let f = format!(
                        "instances/{}.gml",
                        codes.stem(&sanitize(&inst.id.to_string()), &[".gml"])
                    );
                    if let Some(eol) = self.put_text(format!("{dir}/{f}"), &inst.creation_code) {
                        x.insert("code_eol".into(), s(eol));
                    }
                    x.insert("code".into(), s(f));
                }
                insts.push(Value::Table(x));
            }
            t.insert("instances".into(), Value::Array(insts));
            let tiles = r
                .tiles
                .iter()
                .map(|tl| {
                    let mut x = to_table(tl);
                    x.insert(
                        "background".into(),
                        s(self.refs.backgrounds.r(tl.background)),
                    );
                    if !tl.locked {
                        x.remove("locked");
                    }
                    Value::Table(x)
                })
                .collect();
            t.insert("tiles".into(), Value::Array(tiles));
            self.put_toml(format!("{dir}/room.toml"), &t);
        }
    }
}

/// `# name: summary` line for a built-in action (empty for others).
fn action_comment(a: &Action) -> String {
    let Some(def) = crate::actionlib::builtin_action(a.library_id, a.action_id) else {
        return String::new();
    };
    let mut c = def.name.clone();
    if let Some(sum) = crate::actionlib::builtin_action_summary(a.library_id, a.action_id) {
        c.push_str(": ");
        c.push_str(sum);
    }
    let c: String = c
        .chars()
        .map(|x| if x.is_control() { ' ' } else { x })
        .collect();
    format!("# {}\n", c.trim())
}

/// Whether a tree node name can be written in the short (bare) form.
pub(super) fn tree_plain_safe(name: &str) -> bool {
    !name.is_empty()
        && name.trim() == name
        && !name.starts_with('{')
        && !name.starts_with('#')
        && !name.ends_with('/')
        && !name.chars().any(|c| c.is_control())
}
