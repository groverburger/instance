//! GML code editor (specs/ide.md §11): syntax colouring from `specs/builtins.json`,
//! line-number gutter, find/replace, function argument hint in the status bar.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use eframe::egui::{self, text::CCursor, text::LayoutJob, Color32, FontId, TextFormat, Ui};
use instance_format::Project;
use serde::Deserialize;

use crate::project_ops::LIST_KINDS;

const BUILTINS_JSON: &str = include_str!("../../../../specs/builtins.json");

/// GML keywords (specs/gml-language.md §2.6).
pub const KEYWORDS: &[&str] = &[
    "if",
    "then",
    "else",
    "begin",
    "end",
    "repeat",
    "while",
    "do",
    "until",
    "for",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "exit",
    "return",
    "with",
    "var",
    "globalvar",
    "and",
    "or",
    "xor",
    "not",
    "div",
    "mod",
];

#[derive(Deserialize)]
struct JEntry {
    name: String,
    kind: String,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Deserialize)]
struct JFile {
    entries: Vec<JEntry>,
}

/// Built-in identifiers.
pub struct Builtins {
    /// Function name → argument names.
    pub functions: HashMap<String, Vec<String>>,
    pub variables: HashSet<String>,
    pub constants: HashSet<String>,
}

/// The built-in identifier table (decoded once).
pub fn builtins() -> &'static Builtins {
    static B: OnceLock<Builtins> = OnceLock::new();
    B.get_or_init(|| {
        let f: JFile = serde_json::from_str(BUILTINS_JSON).unwrap_or(JFile { entries: vec![] });
        let mut b = Builtins {
            functions: HashMap::new(),
            variables: HashSet::new(),
            constants: HashSet::new(),
        };
        for e in f.entries {
            match e.kind.as_str() {
                "function" => {
                    b.functions.insert(e.name, e.args);
                }
                "variable" => {
                    b.variables.insert(e.name);
                }
                _ => {
                    b.constants.insert(e.name);
                }
            }
        }
        b
    })
}

/// Project-dependent identifier sets (resource names, scripts, user constants).
#[derive(Default, Clone)]
pub struct ProjectNames {
    pub resources: HashSet<String>,
    pub scripts: HashSet<String>,
    pub constants: HashSet<String>,
}

impl ProjectNames {
    pub fn from_project(p: &Project) -> Self {
        let mut n = ProjectNames::default();
        for (k, _, _) in LIST_KINDS {
            for i in 0..crate::project_ops::slot_count(p, k) {
                if let Some(name) = crate::project_ops::res_name(p, k, i) {
                    if k == instance_format::ResourceKind::Script {
                        n.scripts.insert(name.to_string());
                    } else {
                        n.resources.insert(name.to_string());
                    }
                }
            }
        }
        for c in &p.constants {
            n.constants.insert(c.name.clone());
        }
        n
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tok {
    Plain,
    Keyword,
    Comment,
    Str,
    Number,
    Function,
    Variable,
    Constant,
    Resource,
    Script,
    UserConstant,
}

/// Split GML source into coloured spans (byte ranges).
pub fn tokenize(src: &str, names: &ProjectNames) -> Vec<(std::ops::Range<usize>, Tok)> {
    let b = src.as_bytes();
    let bi = builtins();
    let mut out = Vec::new();
    let mut i = 0;
    let mut plain_start = 0;
    let flush = |out: &mut Vec<_>, s: usize, e: usize| {
        if e > s {
            out.push((s..e, Tok::Plain));
        }
    };
    while i < b.len() {
        let c = b[i];
        let start = i;
        let tok = if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            Some(Tok::Comment)
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                i += 1;
            }
            i = (i + 2).min(b.len());
            Some(Tok::Comment)
        } else if c == b'"' || c == b'\'' {
            i += 1;
            while i < b.len() && b[i] != c {
                i += 1;
            }
            i = (i + 1).min(b.len());
            Some(Tok::Str)
        } else if c.is_ascii_digit()
            || (c == b'$' && b.get(i + 1).is_some_and(|d| d.is_ascii_hexdigit()))
        {
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'.') {
                i += 1;
            }
            Some(Tok::Number)
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let w = &src[start..i];
            Some(if KEYWORDS.contains(&w) {
                Tok::Keyword
            } else if bi.functions.contains_key(w) {
                Tok::Function
            } else if bi.variables.contains(w) {
                Tok::Variable
            } else if bi.constants.contains(w) {
                Tok::Constant
            } else if names.scripts.contains(w) {
                Tok::Script
            } else if names.resources.contains(w) {
                Tok::Resource
            } else if names.constants.contains(w) {
                Tok::UserConstant
            } else {
                Tok::Plain
            })
        } else {
            // advance one UTF-8 char
            i += 1;
            while i < b.len() && (b[i] & 0xC0) == 0x80 {
                i += 1;
            }
            None
        };
        if let Some(t) = tok {
            flush(&mut out, plain_start, start);
            out.push((start..i, t));
            plain_start = i;
        }
    }
    flush(&mut out, plain_start, b.len());
    out
}

fn tok_color(t: Tok, dark: bool) -> Color32 {
    let (d, l) = match t {
        Tok::Plain => (Color32::from_gray(220), Color32::from_gray(20)),
        Tok::Keyword => (
            Color32::from_rgb(120, 170, 255),
            Color32::from_rgb(0, 0, 160),
        ),
        Tok::Comment => (
            Color32::from_rgb(110, 180, 110),
            Color32::from_rgb(0, 128, 0),
        ),
        Tok::Str => (
            Color32::from_rgb(230, 150, 150),
            Color32::from_rgb(0, 0, 255),
        ),
        Tok::Number => (
            Color32::from_rgb(230, 190, 120),
            Color32::from_rgb(0, 0, 255),
        ),
        Tok::Function => (
            Color32::from_rgb(200, 140, 220),
            Color32::from_rgb(128, 0, 0),
        ),
        Tok::Variable => (
            Color32::from_rgb(220, 120, 120),
            Color32::from_rgb(128, 0, 0),
        ),
        Tok::Constant => (
            Color32::from_rgb(220, 120, 220),
            Color32::from_rgb(128, 0, 128),
        ),
        Tok::Resource => (
            Color32::from_rgb(120, 210, 210),
            Color32::from_rgb(0, 128, 128),
        ),
        Tok::Script => (
            Color32::from_rgb(170, 200, 120),
            Color32::from_rgb(128, 128, 0),
        ),
        Tok::UserConstant => (
            Color32::from_rgb(220, 160, 220),
            Color32::from_rgb(128, 0, 128),
        ),
    };
    if dark {
        d
    } else {
        l
    }
}

fn layout_job(
    src: &str,
    names: &ProjectNames,
    colour: bool,
    dark: bool,
    font: FontId,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    if !colour {
        job.append(
            src,
            0.0,
            TextFormat::simple(font, tok_color(Tok::Plain, dark)),
        );
        return job;
    }
    for (r, t) in tokenize(src, names) {
        let mut f = TextFormat::simple(font.clone(), tok_color(t, dark));
        if t == Tok::Comment {
            f.italics = true;
        }
        job.append(&src[r], 0.0, f);
    }
    job
}

/// Find/replace panel state of one editor.
#[derive(Default, Clone)]
pub struct CodeEditorState {
    pub find_open: bool,
    pub find: String,
    pub replace: String,
    pub case_sensitive: bool,
    pub message: String,
    /// Line/column of the cursor, for the status bar.
    cursor: Option<usize>,
}

fn char_to_byte(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map_or(s.len(), |(b, _)| b)
}

fn byte_to_char(s: &str, b: usize) -> usize {
    s[..b.min(s.len())].chars().count()
}

/// Find `needle` starting at byte `from` (wrapping). Returns byte range.
pub fn find_in(
    hay: &str,
    needle: &str,
    from: usize,
    case: bool,
    backwards: bool,
) -> Option<std::ops::Range<usize>> {
    if needle.is_empty() {
        return None;
    }
    let (h, n) = if case {
        (hay.to_string(), needle.to_string())
    } else {
        (hay.to_ascii_lowercase(), needle.to_ascii_lowercase())
    };
    let hits: Vec<usize> = h.match_indices(&n).map(|(i, _)| i).collect();
    if hits.is_empty() {
        return None;
    }
    let pos = if backwards {
        hits.iter().rev().find(|&&i| i < from).or(hits.last())
    } else {
        hits.iter().find(|&&i| i >= from).or(hits.first())
    };
    pos.map(|&i| i..i + n.len())
}

/// Signature hint for the call enclosing byte offset `at`, e.g. `draw_text(x, y, string)`.
pub fn call_hint(src: &str, at: usize, names: &ProjectNames) -> Option<String> {
    let b = &src.as_bytes()[..at.min(src.len())];
    let mut depth = 0i32;
    let mut i = b.len();
    let mut argn = 0;
    while i > 0 {
        i -= 1;
        match b[i] {
            b')' => depth += 1,
            b'(' if depth > 0 => depth -= 1,
            b'(' => {
                let end = i;
                let mut s = end;
                while s > 0 && (b[s - 1].is_ascii_alphanumeric() || b[s - 1] == b'_') {
                    s -= 1;
                }
                let name = &src[s..end];
                if let Some(args) = builtins().functions.get(name) {
                    let parts: Vec<String> = args
                        .iter()
                        .enumerate()
                        .map(|(k, a)| {
                            if k == argn {
                                format!("[{a}]")
                            } else {
                                a.clone()
                            }
                        })
                        .collect();
                    return Some(format!("{name}({})", parts.join(", ")));
                }
                if names.scripts.contains(name) {
                    return Some(format!("{name}(argument0, ...)  [argument {argn}]"));
                }
                return None;
            }
            b',' if depth == 0 => argn += 1,
            b'\n' | b';' | b'{' | b'}' if depth == 0 => return None,
            _ => {}
        }
    }
    None
}

/// The code editor widget. `rows` is the minimum visible height in lines.
pub fn code_editor(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    code: &mut String,
    st: &mut CodeEditorState,
    names: &ProjectNames,
    prefs: &crate::prefs::Prefs,
    rows: usize,
) -> bool {
    let id = ui.make_persistent_id(id_salt);
    let edit_id = id.with("text");
    let mut changed = false;
    let font = FontId::monospace(prefs.code_font_size);
    let dark = ui.visuals().dark_mode;

    if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
        st.find_open = true;
    }
    let mut jump: Option<std::ops::Range<usize>> = None;
    ui.horizontal(|ui| {
        ui.toggle_value(&mut st.find_open, "Find/Replace");
        if st.find_open {
            ui.label("Find:");
            let r = ui.add(egui::TextEdit::singleline(&mut st.find).desired_width(120.0));
            ui.checkbox(&mut st.case_sensitive, "Aa");
            let cur = st.cursor.unwrap_or(0);
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("Next").clicked() || enter {
                jump = find_in(
                    code,
                    &st.find,
                    (cur + 1).min(code.len()),
                    st.case_sensitive,
                    false,
                );
                st.message = if jump.is_none() {
                    "not found".into()
                } else {
                    String::new()
                };
            }
            if ui.button("Prev").clicked() {
                jump = find_in(code, &st.find, cur, st.case_sensitive, true);
                st.message = if jump.is_none() {
                    "not found".into()
                } else {
                    String::new()
                };
            }
            ui.label("Replace:");
            ui.add(egui::TextEdit::singleline(&mut st.replace).desired_width(100.0));
            if ui.button("Replace next").clicked() {
                if let Some(r) = find_in(code, &st.find, cur, st.case_sensitive, false) {
                    code.replace_range(r.clone(), &st.replace);
                    changed = true;
                    jump = Some(r.start..r.start + st.replace.len());
                }
            }
            if ui.button("Replace all").clicked() && !st.find.is_empty() {
                let mut n = 0;
                let mut from = 0;
                while let Some(r) = find_in(code, &st.find, from, st.case_sensitive, false) {
                    if r.start < from {
                        break;
                    }
                    code.replace_range(r.clone(), &st.replace);
                    from = r.start + st.replace.len();
                    n += 1;
                }
                if n > 0 {
                    changed = true;
                }
                st.message = format!("{n} replaced");
            }
            ui.label(&st.message);
        }
    });

    let line_count = code.lines().count().max(1) + usize::from(code.ends_with('\n'));
    let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, _wrap: f32| {
        let job = layout_job(
            buf.as_str(),
            names,
            prefs.syntax_colouring,
            dark,
            font.clone(),
        );
        ui.painter().layout_job(job)
    };
    let avail = ui.available_size();
    let status_h = 20.0;
    let mut out_cursor = None;
    egui::ScrollArea::both()
        .id_salt(id.with("scroll"))
        .max_height((avail.y - status_h).max(prefs.code_font_size * rows as f32))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                if prefs.show_line_numbers {
                    let nums: String = (1..=line_count).map(|n| format!("{n:>4}\n")).collect();
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(nums)
                                .font(font.clone())
                                .color(Color32::from_gray(128)),
                        )
                        .selectable(false),
                    );
                }
                let out = egui::TextEdit::multiline(code)
                    .id(edit_id)
                    .code_editor()
                    .lock_focus(true)
                    .desired_rows(rows)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter)
                    .show(ui);
                if out.response.response.changed() {
                    changed = true;
                }
                out_cursor = out.cursor_range;
            });
        });
    if let Some(r) = jump {
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), edit_id) {
            let a = CCursor::new(byte_to_char(code, r.start));
            let b = CCursor::new(byte_to_char(code, r.end));
            state
                .cursor
                .set_char_range(Some(egui::text_selection::CCursorRange::two(a, b)));
            state.store(ui.ctx(), edit_id);
            ui.memory_mut(|m| m.request_focus(edit_id));
        }
        st.cursor = Some(r.start);
    } else if let Some(cr) = out_cursor {
        let c = cr.primary.index.0;
        st.cursor = Some(char_to_byte(code, c));
    }
    // Status bar: line/total: column, argument hint.
    let pos = st.cursor.unwrap_or(0).min(code.len());
    let before = &code[..pos];
    let line = before.matches('\n').count() + 1;
    let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    ui.horizontal(|ui| {
        ui.label(format!("{line}/{line_count}: {col}"));
        if let Some(h) = call_hint(code, pos, names) {
            ui.separator();
            ui.monospace(h);
        }
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens() {
        let names = ProjectNames {
            scripts: ["scr_move".to_string()].into(),
            ..Default::default()
        };
        let src = "if (x > 3) { draw_text(x, y, \"hi\"); scr_move(); } // c\nvar a; a = c_red;";
        let toks = tokenize(src, &names);
        let get = |w: &str| {
            let at = src.find(w).unwrap();
            toks.iter().find(|(r, _)| r.start == at).map(|t| t.1)
        };
        assert_eq!(get("if"), Some(Tok::Keyword));
        assert_eq!(get("x >"), Some(Tok::Variable));
        assert_eq!(get("3"), Some(Tok::Number));
        assert_eq!(get("draw_text"), Some(Tok::Function));
        assert_eq!(get("\"hi\""), Some(Tok::Str));
        assert_eq!(get("scr_move"), Some(Tok::Script));
        assert_eq!(get("// c"), Some(Tok::Comment));
        assert_eq!(get("c_red"), Some(Tok::Constant));
        // spans cover the whole text
        let total: usize = toks.iter().map(|(r, _)| r.len()).sum();
        assert_eq!(total, src.len());
    }

    #[test]
    fn hints_and_find() {
        let n = ProjectNames::default();
        let src = "draw_text(10, ";
        let h = call_hint(src, src.len(), &n).unwrap();
        assert!(h.starts_with("draw_text("), "{h}");
        assert!(h.contains('['));
        assert_eq!(find_in("abcABC", "abc", 1, false, false), Some(3..6));
        assert_eq!(find_in("abcABC", "abc", 1, true, false), Some(0..3));
    }
}
