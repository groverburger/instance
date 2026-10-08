//! Studio GML → GM 8.1 GML translation.
//!
//! The code is tokenized and parsed into an AST ([`ast`]); nodes that use Studio-only
//! syntax are re-emitted in 8.1 form, everything else is copied verbatim from the
//! source (including comments and formatting). See specs/gmx-format.md §GML.
//!
//! Rewrites:
//! * `x++` / `--x` statements → `x += 1` / `x -= 1`; inside expressions the
//!   increment is hoisted into statements before the containing statement
//!   (using a `var __gmx_tN` temporary for the postfix value).
//! * `c ? a : b` → hoisted `if` that assigns a temporary.
//! * Accessors: `l[| i]` → `ds_list_find_value(l, i)` (assignment: `ds_list_replace`),
//!   `m[? k]` → `ds_map_find_value` (assignment: compat `__gmx_ds_map_set`),
//!   `g[# x, y]` → `ds_grid_get` (assignment: `ds_grid_set` / `ds_grid_add`),
//!   `a[@ i]` → `a[i]`.
//! * `0xFF` → `$FF`; `var a = 1, b;` → `var a, b; a = 1`; `return;` → `exit`;
//!   `%=`, `<<=`, `>>=` → `a = a mod (b)` etc.
//! * `enum` declarations are removed and `Enum.member` replaced by its value.
//! * Optional (GMS2-style) string escapes.

pub mod ast;
pub mod lexer;
pub mod parser;

use ast::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Identifiers and calls seen in a piece of code.
#[derive(Debug, Default, Clone)]
pub struct Usage {
    /// Names called as functions.
    pub calls: BTreeSet<String>,
    /// Plain identifiers read (not after '.').
    pub idents: BTreeSet<String>,
    /// Plain identifiers assigned (or declared with var/globalvar), and fields assigned.
    pub assigned: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct Translated {
    pub code: String,
    pub usage: Usage,
    /// Set when the code could not be parsed; `code` is then the input unchanged.
    pub error: Option<String>,
    /// Whether anything was rewritten.
    pub changed: bool,
    /// Non-fatal translation notes.
    pub notes: Vec<String>,
}

/// Translation settings shared by all code of one project.
#[derive(Debug, Default, Clone)]
pub struct Translator {
    /// Enum name → member → value text.
    pub enums: HashMap<String, BTreeMap<String, String>>,
    /// Identifier (read) → replacement expression text (Studio-only built-in variables).
    pub var_renames: HashMap<String, String>,
    /// Function name → replacement name.
    pub call_renames: HashMap<String, String>,
    /// Treat `\` in strings as an escape character (GMS2 behaviour; off for GMS1).
    pub string_escapes: bool,
    /// Emulate Studio's short-circuit `&&` / `||` where the right operand could fail
    /// when evaluated unconditionally (it reads a field of another instance).
    pub short_circuit: bool,
    /// Rewrite `array_length_1d(v)` & co. to `__gmx_array_length_1d(owner, "v")`.
    pub array_funcs: bool,
}

/// Whether evaluating `e` may raise a runtime error depending on state: it reads a
/// variable of another instance (`inst.x`, `obj.x`, `f().x`).
fn risky(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Field(o, _) => {
            !matches!(&o.kind, ExprKind::Ident(n) if n == "global" || n == "self" || n == "other")
                || risky(o)
        }
        ExprKind::Num(_) | ExprKind::Hex(..) | ExprKind::Str(_) | ExprKind::Ident(_) => false,
        ExprKind::Paren(x) | ExprKind::Unary(_, x) => risky(x),
        ExprKind::Binary(_, a, b) => risky(a) || risky(b),
        ExprKind::Call(_, args) => args.iter().any(risky),
        ExprKind::Index(o, idx) | ExprKind::Access(_, o, idx) => risky(o) || idx.iter().any(risky),
        ExprKind::Ternary(a, b, c) => risky(a) || risky(b) || risky(c),
        ExprKind::IncDec { target, .. } => risky(target),
    }
}

/// For `array_length_1d(v)`, `array_length_2d(v, n)`, `array_height_2d(v)` whose first
/// argument names a variable: (owner expression or None for self, variable name).
fn array_arg<'e>(name: &str, args: &'e [Expr]) -> Option<(Option<&'e Expr>, &'e str)> {
    if !matches!(name, "array_length_1d" | "array_length_2d" | "array_height_2d") {
        return None;
    }
    match &args.first()?.kind {
        ExprKind::Ident(v) if v != "argument" && !v.starts_with("argument") => Some((None, v)),
        ExprKind::Field(o, v) => Some((Some(o), v)),
        _ => None,
    }
}

fn is_and(op: &str) -> bool {
    op == "&&" || op == "and"
}
fn is_or(op: &str) -> bool {
    op == "||" || op == "or"
}

/// Split a script file into `(name, code)` pieces at `#define name` lines. The first
/// piece (before any `#define`) gets `default_name`; it is dropped when it is only
/// whitespace and a `#define` follows.
pub fn split_defines(src: &str, default_name: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![(default_name.to_string(), String::new())];
    for line in src.split_inclusive('\n') {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("#define") {
            if rest.starts_with(|c: char| c.is_whitespace()) {
                let name = rest.trim().to_string();
                out.push((name, String::new()));
                continue;
            }
        }
        out.last_mut().unwrap().1.push_str(line);
    }
    if out.len() > 1 && out[0].1.trim().is_empty() {
        out.remove(0);
    }
    out
}

/// Strip `#macro NAME value` lines (GMS2) and `#region` / `#endregion` lines. Returns
/// the remaining code (with those lines blanked to keep line numbers) and the macros.
pub fn strip_directives(src: &str) -> (String, Vec<(String, String)>) {
    let mut macros = Vec::new();
    let mut out = String::with_capacity(src.len());
    for line in src.split_inclusive('\n') {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("#macro") {
            let rest = rest.trim();
            let (name, val) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
            // `#macro Config:NAME value` → just NAME.
            let name = name.rsplit(':').next().unwrap_or(name);
            macros.push((name.to_string(), val.trim().to_string()));
            out.push_str(if line.ends_with('\n') { "\n" } else { "" });
        } else if t.starts_with("#region") || t.starts_with("#endregion") {
            out.push_str("//");
            out.push_str(line);
        } else {
            out.push_str(line);
        }
    }
    (out, macros)
}

/// Parse code into statements.
pub fn parse_program(code: &str, escapes: bool) -> Result<Vec<Stmt>, String> {
    let toks = lexer::lex_with(code, escapes)
        .map_err(|e| format!("lex error at {}: {}", line_col(code, e.offset), e.message))?;
    let mut p = parser::Parser::new(&toks);
    p.program()
        .map_err(|e| format!("parse error at {}: {}", line_col(code, e.offset), e.message))
}

fn line_col(src: &str, off: usize) -> String {
    let off = off.min(src.len());
    let before = &src[..off];
    let line = before.matches('\n').count() + 1;
    let col = before.len() - before.rfind('\n').map_or(0, |i| i + 1) + 1;
    format!("line {line}:{col}")
}

/// Collect `enum` declarations of a program into `out`.
pub fn collect_enums(stmts: &[Stmt], src: &str, out: &mut HashMap<String, BTreeMap<String, String>>) {
    fn walk(s: &Stmt, src: &str, out: &mut HashMap<String, BTreeMap<String, String>>) {
        match &s.kind {
            StmtKind::Enum(name, members) => {
                let mut map = BTreeMap::new();
                let mut next: Option<i64> = Some(0);
                for (m, v) in members {
                    let (text, val) = match v {
                        None => match next {
                            Some(n) => (n.to_string(), Some(n)),
                            None => ("0".to_string(), None),
                        },
                        Some(e) => match eval_const(e, name, &map, out) {
                            Some(n) => (n.to_string(), Some(n)),
                            None => (format!("({})", &src[e.span.0..e.span.1]), None),
                        },
                    };
                    next = val.map(|n| n + 1);
                    map.insert(m.clone(), text);
                }
                out.insert(name.clone(), map);
            }
            StmtKind::Block(v) | StmtKind::Switch(_, v) => v.iter().for_each(|s| walk(s, src, out)),
            StmtKind::If(_, a, b) => {
                walk(a, src, out);
                if let Some(b) = b {
                    walk(b, src, out);
                }
            }
            _ => {}
        }
    }
    for s in stmts {
        walk(s, src, out);
    }
}

fn eval_const(
    e: &Expr,
    this: &str,
    cur: &BTreeMap<String, String>,
    enums: &HashMap<String, BTreeMap<String, String>>,
) -> Option<i64> {
    let ev = |e: &Expr| eval_const(e, this, cur, enums);
    match &e.kind {
        ExprKind::Num(n) => n.parse::<f64>().ok().filter(|f| f.fract() == 0.0).map(|f| f as i64),
        ExprKind::Hex(h, _) => i64::from_str_radix(h, 16).ok(),
        ExprKind::Paren(x) => ev(x),
        ExprKind::Unary("-", x) => ev(x).map(|v| -v),
        ExprKind::Unary("~", x) => ev(x).map(|v| !v),
        ExprKind::Binary(op, a, b) => {
            let (a, b) = (ev(a)?, ev(b)?);
            Some(match *op {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "<<" => a << (b & 63),
                ">>" => a >> (b & 63),
                "|" => a | b,
                "&" => a & b,
                "^" => a ^ b,
                _ => return None,
            })
        }
        ExprKind::Field(o, m) => {
            if let ExprKind::Ident(en) = &o.kind {
                let v = if en == this { cur.get(m) } else { enums.get(en)?.get(m) }?;
                v.parse().ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

struct Ctx<'a> {
    src: &'a str,
    tr: &'a Translator,
    tmp: usize,
    pre: Vec<String>,
    notes: Vec<String>,
}

fn compound_binop(op: &str) -> Option<&'static str> {
    Some(match op {
        "+=" => "+",
        "-=" => "-",
        "*=" => "*",
        "/=" => "/",
        "|=" => "|",
        "&=" => "&",
        "^=" => "^",
        "%=" => "mod",
        "<<=" => "<<",
        ">>=" => ">>",
        _ => return None,
    })
}

impl Translator {
    /// Translate a code piece (script, code action, creation code).
    pub fn translate(&self, code: &str) -> Translated {
        let mut usage = Usage::default();
        match parse_program(code, self.string_escapes) {
            Err(e) => Translated {
                code: code.to_string(),
                usage: crude_usage(code),
                error: Some(e),
                changed: false,
                notes: Vec::new(),
            },
            Ok(stmts) => {
                for s in &stmts {
                    collect_stmt(s, &mut usage);
                }
                let mut ctx = Ctx {
                    src: code,
                    tr: self,
                    tmp: 0,
                    pre: Vec::new(),
                    notes: Vec::new(),
                };
                let changed = stmts.iter().any(|s| ctx.needs_stmt(s));
                if !changed {
                    return Translated {
                        code: code.to_string(),
                        usage,
                        error: None,
                        changed: false,
                        notes: Vec::new(),
                    };
                }
                let mut out = String::new();
                let mut last = 0;
                for s in &stmts {
                    out.push_str(&code[last..s.span.0]);
                    out.push_str(&ctx.stmt(s));
                    last = s.span.1;
                }
                out.push_str(&code[last..]);
                rename_calls_in_usage(&mut usage, self);
                Translated {
                    code: out,
                    usage,
                    error: None,
                    changed: true,
                    notes: ctx.notes,
                }
            }
        }
    }

    /// Translate a single expression (D&D action argument). Falls back to the input
    /// when it does not parse as one expression or would need hoisting.
    pub fn translate_expr(&self, code: &str) -> Translated {
        let unchanged = |usage: Usage, error: Option<String>| Translated {
            code: code.to_string(),
            usage,
            error,
            changed: false,
            notes: Vec::new(),
        };
        let toks = match lexer::lex_with(code, self.string_escapes) {
            Ok(t) => t,
            Err(_) => return unchanged(crude_usage(code), None),
        };
        let mut p = parser::Parser::new(&toks);
        let e = match p.expr() {
            Ok(e) if p.at_eof() => e,
            _ => return unchanged(crude_usage(code), None),
        };
        let mut usage = Usage::default();
        collect_expr(&e, &mut usage);
        let mut ctx = Ctx {
            src: code,
            tr: self,
            tmp: 0,
            pre: Vec::new(),
            notes: Vec::new(),
        };
        if !ctx.needs_expr(&e) {
            return unchanged(usage, None);
        }
        let text = ctx.expr(&e);
        if !ctx.pre.is_empty() {
            return unchanged(usage, None);
        }
        rename_calls_in_usage(&mut usage, self);
        Translated {
            code: format!("{}{}{}", &code[..e.span.0], text, &code[e.span.1..]),
            usage,
            error: None,
            changed: true,
            notes: Vec::new(),
        }
    }
}

fn rename_calls_in_usage(u: &mut Usage, tr: &Translator) {
    if tr.array_funcs {
        for f in ["array_length_1d", "array_length_2d", "array_height_2d"] {
            if u.calls.contains(f) {
                u.calls.insert(format!("__gmx_{f}"));
            }
        }
    }
    if tr.call_renames.is_empty() {
        return;
    }
    u.calls = u
        .calls
        .iter()
        .map(|c| tr.call_renames.get(c).cloned().unwrap_or_else(|| c.clone()))
        .collect();
}

/// Rough identifier scan used when code does not parse.
fn crude_usage(code: &str) -> Usage {
    let mut u = Usage::default();
    if let Ok(toks) = lexer::lex(code) {
        for (i, t) in toks.iter().enumerate() {
            if let lexer::Tok::Ident(s) = &t.tok {
                if parser::is_keyword(s) {
                    continue;
                }
                let after_dot = i > 0 && toks[i - 1].tok == lexer::Tok::Op(".");
                if toks.get(i + 1).map(|t| &t.tok) == Some(&lexer::Tok::Op("(")) {
                    u.calls.insert(s.clone());
                } else if !after_dot {
                    u.idents.insert(s.clone());
                }
            }
        }
    }
    u
}

fn collect_stmt(s: &Stmt, u: &mut Usage) {
    match &s.kind {
        StmtKind::Empty
        | StmtKind::Break
        | StmtKind::Continue
        | StmtKind::Exit
        | StmtKind::Default
        | StmtKind::Return(None) => {}
        StmtKind::Block(v) => v.iter().for_each(|s| collect_stmt(s, u)),
        StmtKind::Var(_, v) => {
            for (n, e) in v {
                u.assigned.insert(n.clone());
                if let Some(e) = e {
                    collect_expr(e, u);
                }
            }
        }
        StmtKind::If(c, a, b) => {
            collect_expr(c, u);
            collect_stmt(a, u);
            if let Some(b) = b {
                collect_stmt(b, u);
            }
        }
        StmtKind::Repeat(c, b) | StmtKind::While(c, b) | StmtKind::With(c, b) => {
            collect_expr(c, u);
            collect_stmt(b, u);
        }
        StmtKind::DoUntil(b, c) => {
            collect_stmt(b, u);
            collect_expr(c, u);
        }
        StmtKind::For(a, c, b, body) => {
            if let Some(a) = a {
                collect_stmt(a, u);
            }
            if let Some(c) = c {
                collect_expr(c, u);
            }
            if let Some(b) = b {
                collect_stmt(b, u);
            }
            collect_stmt(body, u);
        }
        StmtKind::Switch(c, v) => {
            collect_expr(c, u);
            v.iter().for_each(|s| collect_stmt(s, u));
        }
        StmtKind::Case(e) | StmtKind::Return(Some(e)) | StmtKind::ExprStmt(e) => {
            collect_expr(e, u)
        }
        StmtKind::Assign(l, _, r) => {
            assigned_target(l, u);
            collect_expr(l, u);
            collect_expr(r, u);
        }
        StmtKind::Enum(..) => {}
    }
}

fn assigned_target(l: &Expr, u: &mut Usage) {
    match &l.kind {
        ExprKind::Ident(n) => {
            u.assigned.insert(n.clone());
        }
        ExprKind::Field(_, n) => {
            u.assigned.insert(n.clone());
        }
        ExprKind::Index(b, _) => assigned_target(b, u),
        _ => {}
    }
}

fn collect_expr(e: &Expr, u: &mut Usage) {
    match &e.kind {
        ExprKind::Num(_) | ExprKind::Hex(..) | ExprKind::Str(_) => {}
        ExprKind::Ident(n) => {
            u.idents.insert(n.clone());
        }
        ExprKind::Paren(x) | ExprKind::Unary(_, x) => collect_expr(x, u),
        ExprKind::Binary(_, a, b) => {
            collect_expr(a, u);
            collect_expr(b, u);
        }
        ExprKind::Call(n, args) => {
            u.calls.insert(n.clone());
            args.iter().for_each(|a| collect_expr(a, u));
        }
        ExprKind::Field(o, _) => collect_expr(o, u),
        ExprKind::Index(o, idx) | ExprKind::Access(_, o, idx) => {
            collect_expr(o, u);
            idx.iter().for_each(|a| collect_expr(a, u));
        }
        ExprKind::Ternary(a, b, c) => {
            collect_expr(a, u);
            collect_expr(b, u);
            collect_expr(c, u);
        }
        ExprKind::IncDec { target, .. } => {
            assigned_target(target, u);
            collect_expr(target, u);
        }
    }
    // Accessor/ternary rewrites call these.
    match &e.kind {
        ExprKind::Access(AccessKind::List, ..) => {
            u.calls.insert("ds_list_find_value".into());
        }
        ExprKind::Access(AccessKind::Map, ..) => {
            u.calls.insert("ds_map_find_value".into());
        }
        ExprKind::Access(AccessKind::Grid, ..) => {
            u.calls.insert("ds_grid_get".into());
        }
        _ => {}
    }
}

/// Replace `(span, text)` pieces inside `span` of `src`.
fn splice(src: &str, span: Span, mut repl: Vec<(Span, String)>) -> String {
    repl.sort_by_key(|r| r.0 .0);
    let mut out = String::new();
    let mut last = span.0;
    let word = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    for ((a, b), t) in repl {
        out.push_str(&src[last..a]);
        // Keep tokens apart when a replacement starts/ends with a word character
        // (e.g. `if cond` → `if __gmx_t0`).
        if word(out.chars().last()) && word(t.chars().next()) {
            out.push(' ');
        }
        out.push_str(&t);
        if word(t.chars().last()) && word(src[b..].chars().next()) {
            out.push(' ');
        }
        last = b;
    }
    out.push_str(&src[last..span.1]);
    out
}

/// Decode GMS2-style escapes in a string literal and re-encode it for GM 8.1.
fn reencode_string(lit: &str) -> String {
    let q = lit.chars().next().unwrap_or('"');
    let inner = &lit[1..lit.len().saturating_sub(1)];
    let mut val = String::new();
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') | Some('r') => val.push('#'),
                Some('t') => val.push('\t'),
                Some('#') => val.push_str("\\#"),
                Some(o) => val.push(o),
                None => val.push('\\'),
            }
        } else {
            val.push(c);
        }
    }
    if !val.contains(q) {
        return format!("{q}{val}{q}");
    }
    let other = if q == '"' { '\'' } else { '"' };
    if !val.contains(other) {
        return format!("{other}{val}{other}");
    }
    // Both quote kinds: concatenate pieces with chr(34).
    let parts: Vec<String> = val.split('"').map(|p| format!("\"{p}\"")).collect();
    format!("({})", parts.join(" + chr(34) + "))
}

impl Ctx<'_> {
    fn text(&self, sp: Span) -> &str {
        &self.src[sp.0..sp.1]
    }

    fn new_tmp(&mut self) -> String {
        let n = format!("__gmx_t{}", self.tmp);
        self.tmp += 1;
        n
    }

    fn needs_expr(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Num(_) => false,
            ExprKind::Hex(_, x) => *x,
            ExprKind::Str(s) => self.tr.string_escapes && s.contains('\\'),
            ExprKind::Ident(n) => self.tr.var_renames.contains_key(n),
            ExprKind::Paren(x) | ExprKind::Unary(_, x) => self.needs_expr(x),
            ExprKind::Binary(op, a, b) => {
                self.needs_expr(a)
                    || self.needs_expr(b)
                    || (self.tr.short_circuit && (is_and(op) || is_or(op)) && risky(b))
            }
            ExprKind::Call(n, args) => {
                self.tr.call_renames.contains_key(n)
                    || (self.tr.array_funcs && array_arg(n, args).is_some())
                    || args.iter().any(|a| self.needs_expr(a))
            }
            ExprKind::Field(o, m) => {
                if let ExprKind::Ident(en) = &o.kind {
                    if self.tr.enums.get(en).is_some_and(|m2| m2.contains_key(m)) {
                        return true;
                    }
                    // `obj.x` where obj is renamed: only the owner changes.
                    return self.tr.var_renames.contains_key(en);
                }
                self.needs_expr(o)
            }
            ExprKind::Index(o, idx) => {
                self.needs_expr(o) || idx.iter().any(|a| self.needs_expr(a))
            }
            ExprKind::Access(..) | ExprKind::Ternary(..) | ExprKind::IncDec { .. } => true,
        }
    }

    fn needs_stmt(&self, s: &Stmt) -> bool {
        match &s.kind {
            StmtKind::Empty
            | StmtKind::Break
            | StmtKind::Continue
            | StmtKind::Exit
            | StmtKind::Default => false,
            StmtKind::Return(None) | StmtKind::Enum(..) => true,
            StmtKind::Block(v) | StmtKind::Switch(_, v)
                if v.iter().any(|s| self.needs_stmt(s)) =>
            {
                true
            }
            StmtKind::Block(_) => false,
            StmtKind::Switch(c, _) => self.needs_expr(c),
            StmtKind::Var(_, v) => v.iter().any(|(_, e)| e.is_some()),
            StmtKind::If(c, a, b) => {
                self.needs_expr(c)
                    || self.needs_stmt(a)
                    || b.as_ref().is_some_and(|b| self.needs_stmt(b))
            }
            StmtKind::Repeat(c, b) | StmtKind::While(c, b) | StmtKind::With(c, b) => {
                self.needs_expr(c) || self.needs_stmt(b)
            }
            StmtKind::DoUntil(b, c) => self.needs_expr(c) || self.needs_stmt(b),
            StmtKind::For(a, c, b, body) => {
                a.as_ref().is_some_and(|a| self.needs_stmt(a))
                    || c.as_ref().is_some_and(|c| self.needs_expr(c))
                    || b.as_ref().is_some_and(|b| self.needs_stmt(b))
                    || self.needs_stmt(body)
            }
            StmtKind::Case(e) | StmtKind::Return(Some(e)) => self.needs_expr(e),
            StmtKind::ExprStmt(e) => self.needs_expr(e),
            StmtKind::Assign(l, op, r) => {
                matches!(*op, "%=" | "<<=" | ">>=")
                    || matches!(l.kind, ExprKind::Access(..))
                    || self.needs_expr(l)
                    || self.needs_expr(r)
            }
        }
    }

    /// Emit an expression; hoisted statements go to `self.pre`.
    fn expr(&mut self, e: &Expr) -> String {
        if !self.needs_expr(e) {
            return self.text(e.span).to_string();
        }
        match &e.kind {
            ExprKind::Hex(h, _) => format!("${h}"),
            ExprKind::Str(s) => reencode_string(s),
            ExprKind::Ident(n) => self.tr.var_renames[n].clone(),
            ExprKind::Field(o, m) => {
                if let ExprKind::Ident(en) = &o.kind {
                    if let Some(v) = self.tr.enums.get(en).and_then(|mm| mm.get(m)) {
                        return v.clone();
                    }
                }
                let ot = self.expr(o);
                splice(self.src, e.span, vec![(o.span, ot)])
            }
            ExprKind::Call(n, args) if self.tr.array_funcs && array_arg(n, args).is_some() => {
                // array_length_1d(a) etc.: pass owner and variable name instead of the
                // value (GM 8.1 arrays are not values).
                let (owner, var) = array_arg(n, args).unwrap();
                let owner = match owner {
                    None => "id".to_string(),
                    Some(o) => self.expr(o),
                };
                let mut out = format!("__gmx_{n}({owner}, \"{var}\"");
                for a in &args[1..] {
                    out.push_str(", ");
                    out.push_str(&self.expr(a));
                }
                out.push(')');
                out
            }
            ExprKind::Call(n, args) => {
                let mut repl: Vec<(Span, String)> =
                    args.iter().map(|a| (a.span, self.expr(a))).collect();
                if let Some(nn) = self.tr.call_renames.get(n) {
                    repl.push(((e.span.0, e.span.0 + n.len()), nn.clone()));
                }
                splice(self.src, e.span, repl)
            }
            ExprKind::Paren(x) | ExprKind::Unary(_, x) => {
                let t = self.expr(x);
                splice(self.src, e.span, vec![(x.span, t)])
            }
            ExprKind::Binary(op, a, b)
                if self.tr.short_circuit && (is_and(op) || is_or(op)) && risky(b) =>
            {
                // Studio short-circuits; GM 8.1 evaluates both operands.
                let at = self.expr(a);
                let t = self.new_tmp();
                let (bt, bpre) = self.sub(|s| s.expr(b));
                if is_and(op) {
                    self.pre.push(format!(
                        "var {t}; {t} = false; if ({at}) {{ {bpre}if ({bt}) {t} = true; }}"
                    ));
                } else {
                    self.pre.push(format!(
                        "var {t}; {t} = true; if (!({at})) {{ {bpre}if (!({bt})) {t} = false; }}"
                    ));
                }
                t
            }
            ExprKind::Binary(_, a, b) => {
                let ta = self.expr(a);
                let tb = self.expr(b);
                splice(self.src, e.span, vec![(a.span, ta), (b.span, tb)])
            }
            ExprKind::Index(o, idx) => {
                let mut repl = vec![(o.span, self.expr(o))];
                for a in idx {
                    let t = self.expr(a);
                    repl.push((a.span, t));
                }
                splice(self.src, e.span, repl)
            }
            ExprKind::Access(k, o, idx) => {
                let ot = self.expr(o);
                let it: Vec<String> = idx.iter().map(|a| self.expr(a)).collect();
                let args = it.join(", ");
                match k {
                    AccessKind::List => format!("ds_list_find_value({ot}, {args})"),
                    AccessKind::Map => format!("ds_map_find_value({ot}, {args})"),
                    AccessKind::Grid => format!("ds_grid_get({ot}, {args})"),
                    AccessKind::Array => format!("{ot}[{args}]"),
                }
            }
            ExprKind::Ternary(c, a, b) => {
                let ct = self.expr(c);
                let t = self.new_tmp();
                let (at, apre) = self.sub(|s| s.expr(a));
                let (bt, bpre) = self.sub(|s| s.expr(b));
                self.pre.push(format!(
                    "var {t}; if ({ct}) {{ {apre}{t} = {at}; }} else {{ {bpre}{t} = {bt}; }}"
                ));
                t
            }
            ExprKind::IncDec {
                prefix,
                inc,
                target,
            } => {
                let upd = self.incdec_stmt(*inc, target);
                if *prefix {
                    self.pre.push(format!("{upd};"));
                    self.lvalue_read(target)
                } else {
                    let t = self.new_tmp();
                    let cur = self.lvalue_read(target);
                    self.pre.push(format!("var {t}; {t} = {cur}; {upd};"));
                    t
                }
            }
            ExprKind::Num(_) => self.text(e.span).to_string(),
        }
    }

    /// Emit in a nested hoisting context; returns (text, hoisted statements).
    fn sub(&mut self, f: impl FnOnce(&mut Self) -> String) -> (String, String) {
        let saved = std::mem::take(&mut self.pre);
        let t = f(self);
        let pre = std::mem::replace(&mut self.pre, saved);
        let p = pre.iter().map(|p| format!("{p} ")).collect::<String>();
        (t, p)
    }

    /// Read the value of an lvalue (accessors become function calls).
    fn lvalue_read(&mut self, l: &Expr) -> String {
        self.expr(l)
    }

    fn incdec_stmt(&mut self, inc: bool, target: &Expr) -> String {
        let op = if inc { "+=" } else { "-=" };
        self.assign_text(target, op, "1")
    }

    /// Build an assignment statement text for `l op rhs`.
    fn assign_text(&mut self, l: &Expr, op: &str, rhs: &str) -> String {
        let bin = compound_binop(op);
        if let ExprKind::Access(k, o, idx) = &l.kind {
            let ot = self.expr(o);
            let it: Vec<String> = idx.iter().map(|a| self.expr(a)).collect();
            let args = it.join(", ");
            let cur = match k {
                AccessKind::List => format!("ds_list_find_value({ot}, {args})"),
                AccessKind::Map => format!("ds_map_find_value({ot}, {args})"),
                AccessKind::Grid => format!("ds_grid_get({ot}, {args})"),
                AccessKind::Array => format!("{ot}[{args}]"),
            };
            let val = match bin {
                Some(b) => format!("{cur} {b} ({rhs})"),
                None => rhs.to_string(),
            };
            return match k {
                AccessKind::List => format!("ds_list_replace({ot}, {args}, {val})"),
                AccessKind::Map => format!("__gmx_ds_map_set({ot}, {args}, {val})"),
                AccessKind::Grid if op == "+=" => format!("ds_grid_add({ot}, {args}, {rhs})"),
                AccessKind::Grid if op == "*=" => {
                    format!("ds_grid_multiply({ot}, {args}, {rhs})")
                }
                AccessKind::Grid => format!("ds_grid_set({ot}, {args}, {val})"),
                AccessKind::Array => match op {
                    "%=" | "<<=" | ">>=" => format!("{cur} = {val}"),
                    _ => format!("{cur} {op} {rhs}"),
                },
            };
        }
        let lt = self.expr(l);
        match op {
            "%=" | "<<=" | ">>=" => format!("{lt} = {lt} {} ({rhs})", bin.unwrap()),
            _ => format!("{lt} {op} {rhs}"),
        }
    }

    /// Emit a statement, wrapping hoisted statements in a block.
    fn stmt(&mut self, s: &Stmt) -> String {
        if !self.needs_stmt(s) {
            return self.text(s.span).to_string();
        }
        let saved = std::mem::take(&mut self.pre);
        let body = self.stmt_inner(s);
        let pre = std::mem::replace(&mut self.pre, saved);
        if pre.is_empty() {
            body
        } else {
            let p = pre.iter().map(|p| format!("{p} ")).collect::<String>();
            format!("{{ {p}{body} }}")
        }
    }

    /// Emit a condition expression; returns (text, hoisted statements) without
    /// adding the hoisted statements to the current context.
    fn cond(&mut self, e: &Expr) -> (String, String) {
        self.sub(|s| s.expr(e))
    }

    fn stmt_inner(&mut self, s: &Stmt) -> String {
        let src = self.src;
        match &s.kind {
            StmtKind::Empty | StmtKind::Break | StmtKind::Continue | StmtKind::Exit | StmtKind::Default => {
                self.text(s.span).to_string()
            }
            StmtKind::Return(None) => "exit".into(),
            StmtKind::Enum(name, _) => format!("/* enum {name} (inlined by gmx2gm81) */"),
            StmtKind::Block(v) | StmtKind::Switch(_, v) => {
                let mut repl: Vec<(Span, String)> = Vec::new();
                if let StmtKind::Switch(c, _) = &s.kind {
                    let t = self.expr(c);
                    repl.push((c.span, t));
                }
                for st in v {
                    let t = self.stmt(st);
                    repl.push((st.span, t));
                }
                splice(src, s.span, repl)
            }
            StmtKind::Var(global, v) => {
                let kw = if *global { "globalvar" } else { "var" };
                let names: Vec<&str> = v.iter().map(|(n, _)| n.as_str()).collect();
                let mut out = format!("{kw} {}", names.join(", "));
                for (n, e) in v {
                    if let Some(e) = e {
                        let t = self.expr(e);
                        out.push_str(&format!("; {n} = {t}"));
                    }
                }
                out
            }
            StmtKind::If(c, a, b) => {
                let ct = self.expr(c);
                let mut repl = vec![(c.span, ct)];
                repl.push((a.span, self.stmt(a)));
                if let Some(b) = b {
                    repl.push((b.span, self.stmt(b)));
                }
                splice(src, s.span, repl)
            }
            StmtKind::Repeat(c, b) | StmtKind::With(c, b) => {
                let ct = self.expr(c);
                let bt = self.stmt(b);
                splice(src, s.span, vec![(c.span, ct), (b.span, bt)])
            }
            StmtKind::While(c, b) => {
                let (ct, pre) = self.cond(c);
                let bt = self.stmt(b);
                if pre.is_empty() {
                    splice(src, s.span, vec![(c.span, ct), (b.span, bt)])
                } else {
                    format!("while (true) {{ {pre}if (!({ct})) break; {bt} }}")
                }
            }
            StmtKind::DoUntil(b, c) => {
                let bt = self.stmt(b);
                let (ct, pre) = self.cond(c);
                if pre.is_empty() {
                    splice(src, s.span, vec![(b.span, bt), (c.span, ct)])
                } else {
                    format!("do {{ {bt} {pre}}} until ({ct})")
                }
            }
            StmtKind::For(Some(a), c, st, body)
                if matches!(&a.kind, StmtKind::Var(_, v) if v.iter().any(|x| x.1.is_some())) =>
            {
                // `for (var i = 0; ...)`: declare before the loop, keep the last
                // initialiser in the header.
                let StmtKind::Var(global, v) = &a.kind else { unreachable!() };
                let kw = if *global { "globalvar" } else { "var" };
                let names: Vec<&str> = v.iter().map(|(n, _)| n.as_str()).collect();
                let mut pre = format!("{kw} {}; ", names.join(", "));
                let inits: Vec<(&String, &Expr)> =
                    v.iter().filter_map(|(n, e)| e.as_ref().map(|e| (n, e))).collect();
                let mut header = String::new();
                for (k, (n, e)) in inits.iter().enumerate() {
                    let t = self.expr(e);
                    if k + 1 == inits.len() {
                        header = format!("{n} = {t}");
                    } else {
                        pre.push_str(&format!("{n} = {t}; "));
                    }
                }
                let ct = c.as_ref().map(|c| (c.span, self.expr(c)));
                let stt = st.as_ref().map(|x| (x.span, self.stmt(x)));
                let bt = self.stmt(body);
                let mut repl = vec![(a.span, header), (body.span, bt)];
                repl.extend(ct);
                repl.extend(stt);
                format!("{{ {pre}{} }}", splice(src, s.span, repl))
            }
            StmtKind::For(a, c, st, body) => {
                let at = a.as_ref().map(|a| (a.span, self.stmt(a)));
                let ct = c.as_ref().map(|c| (c.span, self.cond(c)));
                let stt = st.as_ref().map(|x| (x.span, self.stmt(x)));
                let bt = self.stmt(body);
                let hoisted = ct.as_ref().is_some_and(|(_, (_, p))| !p.is_empty());
                if !hoisted {
                    let mut repl = vec![(body.span, bt)];
                    if let Some(x) = at {
                        repl.push(x);
                    }
                    if let Some((sp, (t, _))) = ct {
                        repl.push((sp, t));
                    }
                    if let Some(x) = stt {
                        repl.push(x);
                    }
                    splice(src, s.span, repl)
                } else {
                    self.notes.push(
                        "for-loop condition with ++/--/?: rewritten as while(true); `continue` skips the step".into(),
                    );
                    let (_, (cond, pre)) = ct.unwrap();
                    format!(
                        "{{ {}; while (true) {{ {pre}if (!({cond})) break; {bt} {}; }} }}",
                        at.map(|x| x.1).unwrap_or_default(),
                        stt.map(|x| x.1).unwrap_or_default()
                    )
                }
            }
            StmtKind::Case(e) => {
                let t = self.expr(e);
                if !self.pre.is_empty() {
                    self.notes.push("case label needs hoisting; dropped".into());
                    self.pre.clear();
                }
                splice(src, s.span, vec![(e.span, t)])
            }
            StmtKind::Return(Some(e)) => {
                let t = self.expr(e);
                splice(src, s.span, vec![(e.span, t)])
            }
            StmtKind::ExprStmt(e) => match &e.kind {
                ExprKind::IncDec { inc, target, .. } => self.incdec_stmt(*inc, target),
                _ => self.expr(e),
            },
            StmtKind::Assign(l, op, r) => {
                let rt = self.expr(r);
                if matches!(l.kind, ExprKind::Access(..)) || matches!(*op, "%=" | "<<=" | ">>=") {
                    self.assign_text(l, op, &rt)
                } else {
                    let lt = self.expr(l);
                    splice(src, s.span, vec![(l.span, lt), (r.span, rt)])
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
