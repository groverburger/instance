//! Action libraries (`.lib`, spec §10) and the bundled built-in action table
//! (derived from `specs/actions.json`).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::io::{Reader, Writer};
use crate::model::{action_kind, Action, Timestamp, APPLIES_TO_SELF};

/// An action library (one palette tab).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ActionLibrary {
    pub tab_caption: String,
    /// 1 for all built-in libraries; ids < 1000 are reserved for built-ins.
    pub library_id: i32,
    pub author: String,
    pub version: i32,
    pub last_changed: Timestamp,
    pub info: String,
    /// GML run at game start.
    pub init_code: String,
    /// Whole tab only shown in advanced mode.
    pub advanced_only: bool,
    /// Opaque library-maker counter.
    pub editor_counter: i32,
    /// Actions in palette order, including palette decorations (kinds 8-10).
    pub actions: Vec<ActionDef>,
}

/// One argument slot of an action definition.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ArgSlot {
    pub caption: String,
    /// See [`crate::model::arg_kind`].
    pub kind: i32,
    pub default: String,
    /// Menu captions separated by `|` (only meaningful for kind 4).
    pub menu_items: String,
}

impl ArgSlot {
    /// Menu captions as a list.
    pub fn menu(&self) -> Vec<&str> {
        if self.menu_items.is_empty() {
            Vec::new()
        } else {
            self.menu_items.split('|').collect()
        }
    }
}

/// An action definition (spec §10.2).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ActionDef {
    /// Identifier-like name; for palette labels the label text.
    pub name: String,
    /// Unique within the library; 999 for palette decorations.
    pub action_id: i32,
    /// 32x32 BMP file (empty in the bundled built-in table).
    pub image: Vec<u8>,
    pub hidden: bool,
    pub advanced_only: bool,
    pub pro_only: bool,
    pub palette_title: String,
    /// List template (empty in the bundled built-in table).
    pub list_text: String,
    /// Hint template (empty in the bundled built-in table).
    pub hint_text: String,
    /// See [`crate::model::action_kind`].
    pub kind: i32,
    /// 0 normal dialog, 1 none, 2 arrow grid, 5 code editor.
    pub interface: i32,
    pub is_question: bool,
    pub show_apply_to: bool,
    pub show_relative: bool,
    pub argument_count: i32,
    /// Argument slots (8 in practice).
    pub args: Vec<ArgSlot>,
    /// See [`crate::model::exe_type`].
    pub exe_type: i32,
    pub function_name: String,
    pub code: String,
}

impl ActionDef {
    /// Whether this entry only lays out the palette (placeholder/separator/label).
    pub fn is_decoration(&self) -> bool {
        self.kind >= action_kind::PLACEHOLDER
    }

    /// Create an action instance with default argument values, copying the library
    /// metadata as the IDE does when an action is dropped into an event.
    pub fn instantiate(&self, library_id: i32) -> Action {
        let slots = self.args.len().max(8);
        let mut arg_kinds = vec![0; slots];
        let mut args = vec!["0".to_string(); slots];
        for (i, a) in self.args.iter().enumerate() {
            arg_kinds[i] = a.kind;
            if (i as i32) < self.argument_count {
                args[i] = a.default.clone();
            }
        }
        Action {
            library_id,
            action_id: self.action_id,
            kind: self.kind,
            may_be_relative: self.show_relative,
            is_question: self.is_question,
            has_applies_to: self.show_apply_to,
            exe_type: self.exe_type,
            function_name: self.function_name.clone(),
            code: self.code.clone(),
            argument_count: self.argument_count,
            arg_kinds,
            applies_to: APPLIES_TO_SELF,
            relative: false,
            args,
            negate: false,
        }
    }
}

impl ActionLibrary {
    /// Find a (non-decoration) action by id.
    pub fn action(&self, action_id: i32) -> Option<&ActionDef> {
        self.actions
            .iter()
            .find(|a| a.action_id == action_id && !a.is_decoration())
    }
}

/// Read a `.lib` file.
pub fn read_library(data: &[u8]) -> Result<ActionLibrary> {
    let mut r = Reader::new(data, "action library");
    r.version(&[520], "library format")?;
    let mut l = ActionLibrary {
        tab_caption: r.string()?,
        library_id: r.i32()?,
        author: r.string()?,
        version: r.i32()?,
        last_changed: r.f64()?,
        info: r.string()?,
        init_code: r.string()?,
        advanced_only: r.bool()?,
        editor_counter: r.i32()?,
        actions: Vec::new(),
    };
    let n = r.count("action")?;
    for _ in 0..n {
        r.version(&[520], "library action")?;
        let mut a = ActionDef {
            name: r.string()?,
            action_id: r.i32()?,
            image: r.blob()?,
            hidden: r.bool()?,
            advanced_only: r.bool()?,
            pro_only: r.bool()?,
            palette_title: r.string()?,
            list_text: r.string()?,
            hint_text: r.string()?,
            kind: r.i32()?,
            interface: r.i32()?,
            is_question: r.bool()?,
            show_apply_to: r.bool()?,
            show_relative: r.bool()?,
            argument_count: r.i32()?,
            ..ActionDef::default()
        };
        let k = r.count("argument slot")?;
        for _ in 0..k {
            a.args.push(ArgSlot {
                caption: r.string()?,
                kind: r.i32()?,
                default: r.string()?,
                menu_items: r.string()?,
            });
        }
        a.exe_type = r.i32()?;
        a.function_name = r.string()?;
        a.code = r.string()?;
        l.actions.push(a);
    }
    r.end()?;
    Ok(l)
}

/// Read a `.lib` file from disk.
pub fn read_library_file(path: impl AsRef<std::path::Path>) -> Result<ActionLibrary> {
    read_library(&std::fs::read(path)?)
}

/// Write a `.lib` file.
pub fn write_library(l: &ActionLibrary) -> Result<Vec<u8>> {
    let mut w = Writer::new();
    w.i32(520);
    w.string(&l.tab_caption);
    w.i32(l.library_id);
    w.string(&l.author);
    w.i32(l.version);
    w.f64(l.last_changed);
    w.string(&l.info);
    w.string(&l.init_code);
    w.bool(l.advanced_only);
    w.i32(l.editor_counter);
    w.count(l.actions.len())?;
    for a in &l.actions {
        w.i32(520);
        w.string(&a.name);
        w.i32(a.action_id);
        w.blob(&a.image);
        w.bool(a.hidden);
        w.bool(a.advanced_only);
        w.bool(a.pro_only);
        w.string(&a.palette_title);
        w.string(&a.list_text);
        w.string(&a.hint_text);
        w.i32(a.kind);
        w.i32(a.interface);
        w.bool(a.is_question);
        w.bool(a.show_apply_to);
        w.bool(a.show_relative);
        w.i32(a.argument_count);
        w.count(a.args.len())?;
        for s in &a.args {
            w.string(&s.caption);
            w.i32(s.kind);
            w.string(&s.default);
            w.string(&s.menu_items);
        }
        w.i32(a.exe_type);
        w.string(&a.function_name);
        w.string(&a.code);
    }
    Ok(w.buf)
}

// ------------------------------------------------------------------ built-in table

const ACTIONS_JSON: &str = include_str!("../../../specs/actions.json");

#[derive(Deserialize)]
struct JTable {
    libraries: Vec<JLib>,
}

#[derive(Deserialize)]
struct JLib {
    tab_caption: String,
    lib_id: i32,
    lib_version: i32,
    advanced_only: bool,
    palette: Vec<JPalette>,
    actions: Vec<JAction>,
}

#[derive(Deserialize)]
struct JPalette {
    action_id: Option<i32>,
    kind: Option<i32>,
    #[serde(default)]
    label: String,
    #[serde(default)]
    advanced_only: bool,
}

#[derive(Deserialize)]
struct JAction {
    id: i32,
    name: String,
    palette_title: String,
    kind: i32,
    interface: i32,
    is_question: bool,
    show_apply_to: bool,
    show_relative: bool,
    hidden: bool,
    advanced_only: bool,
    pro_only: bool,
    exe_type: i32,
    #[serde(default)]
    function: String,
    argc: i32,
    args: Vec<JArg>,
    #[serde(default)]
    summary: String,
}

#[derive(Deserialize)]
struct JArg {
    caption: String,
    kind: i32,
    default: String,
    #[serde(default)]
    menu: Vec<String>,
}

/// The built-in libraries, decoded once from the bundled table.
struct Builtins {
    libs: Vec<ActionLibrary>,
    summaries: Vec<((i32, i32), String)>,
}

fn builtins() -> &'static Builtins {
    static B: OnceLock<Builtins> = OnceLock::new();
    B.get_or_init(|| decode_builtins().expect("bundled specs/actions.json is valid"))
}

fn decode_builtins() -> Result<Builtins> {
    let t: JTable =
        serde_json::from_str(ACTIONS_JSON).map_err(|e| Error::BuiltinTable(e.to_string()))?;
    let mut libs = Vec::new();
    let mut summaries = Vec::new();
    for jl in t.libraries {
        let mut l = ActionLibrary {
            tab_caption: jl.tab_caption,
            library_id: jl.lib_id,
            version: jl.lib_version,
            advanced_only: jl.advanced_only,
            ..ActionLibrary::default()
        };
        for p in &jl.palette {
            let def = match (p.action_id, p.kind) {
                (Some(id), _) => {
                    let a = jl.actions.iter().find(|a| a.id == id).ok_or_else(|| {
                        Error::BuiltinTable(format!("palette refers to unknown action {id}"))
                    })?;
                    summaries.push(((jl.lib_id, id), a.summary.clone()));
                    let mut args: Vec<ArgSlot> = a
                        .args
                        .iter()
                        .map(|x| ArgSlot {
                            caption: x.caption.clone(),
                            kind: x.kind,
                            default: x.default.clone(),
                            menu_items: x.menu.join("|"),
                        })
                        .collect();
                    args.resize(8.max(args.len()), ArgSlot::default());
                    ActionDef {
                        name: a.name.clone(),
                        action_id: a.id,
                        hidden: a.hidden,
                        advanced_only: a.advanced_only,
                        pro_only: a.pro_only,
                        palette_title: a.palette_title.clone(),
                        kind: a.kind,
                        interface: a.interface,
                        is_question: a.is_question,
                        show_apply_to: a.show_apply_to,
                        show_relative: a.show_relative,
                        argument_count: a.argc,
                        args,
                        exe_type: a.exe_type,
                        function_name: a.function.clone(),
                        ..ActionDef::default()
                    }
                }
                (None, Some(kind)) => ActionDef {
                    name: p.label.clone(),
                    action_id: 999,
                    advanced_only: p.advanced_only,
                    kind,
                    args: vec![ArgSlot::default(); 8],
                    ..ActionDef::default()
                },
                (None, None) => {
                    return Err(Error::BuiltinTable(
                        "palette entry without id or kind".into(),
                    ))
                }
            };
            l.actions.push(def);
        }
        libs.push(l);
    }
    Ok(Builtins { libs, summaries })
}

/// The seven built-in action libraries in tab order (move, main1, main2, control,
/// score, extra, draw). Images and list/hint templates are not bundled.
pub fn builtin_libraries() -> &'static [ActionLibrary] {
    &builtins().libs
}

/// Look up a built-in action definition by library id and action id.
pub fn builtin_action(library_id: i32, action_id: i32) -> Option<&'static ActionDef> {
    builtin_libraries()
        .iter()
        .filter(|l| l.library_id == library_id)
        .find_map(|l| l.action(action_id))
}

/// Short paraphrased description of a built-in action (from the spec table).
pub fn builtin_action_summary(library_id: i32, action_id: i32) -> Option<&'static str> {
    builtins()
        .summaries
        .iter()
        .find(|(k, _)| *k == (library_id, action_id))
        .map(|(_, s)| s.as_str())
}
