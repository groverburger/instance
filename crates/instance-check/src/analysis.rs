//! Compiles every code piece and analyses the GML: compile errors, script argument
//! counts, variables read but never assigned, possibly uninitialised instance
//! variables, no-short-circuit hazards, Lite-only features, line endings.

use crate::pieces::{self, Form, Owner, Piece};
use crate::{Diagnostic, Options, Severity};
use gml::ast::{AssignOp, Expr, ExprKind, Stmt, StmtKind, SwitchItem};
use gml::builtins::{self, VarScope};
use gml::lexer::Span;
use gml::{BinOp, Ctx, Error, FunctionId, Host, Value, VarTable, VariableId, Vm};
use instance_format::{event_type, Project, NO_PARENT};
use std::collections::{BTreeSet, HashMap, HashSet};

// ====================================================================== host

/// Compile-time name resolution as the runner does it (`instance-runner/src/game.rs`):
/// resource names, then user constants (a resource wins), scripts for calls.
struct CheckHost {
    constants: HashMap<String, Value>,
    scripts: HashMap<String, i32>,
}

impl CheckHost {
    fn new(p: &Project, vm: &mut Vm) -> Self {
        let mut h = CheckHost {
            constants: HashMap::new(),
            scripts: HashMap::new(),
        };
        macro_rules! names {
            ($list:expr) => {
                for (i, r) in $list.iter().enumerate() {
                    if let Some(r) = r {
                        h.constants
                            .entry(r.name.clone())
                            .or_insert(Value::Real(i as f64));
                    }
                }
            };
        }
        names!(p.sprites);
        names!(p.sounds);
        names!(p.backgrounds);
        names!(p.paths);
        names!(p.fonts);
        names!(p.timelines);
        names!(p.objects);
        names!(p.rooms);
        for (i, s) in p.scripts.iter().enumerate() {
            if let Some(s) = s {
                h.constants
                    .entry(s.name.clone())
                    .or_insert(Value::Real(i as f64));
                h.scripts.insert(s.name.clone(), i as i32);
            }
        }
        // A trigger's constant name (manual: usable with event_perform(ev_trigger, name)).
        for (i, t) in p.triggers.iter().enumerate() {
            if let Some(t) = t {
                if crate::structure::is_identifier(&t.constant_name) {
                    h.constants
                        .entry(t.constant_name.clone())
                        .or_insert(Value::Real(i as f64));
                }
            }
        }
        // User constants, in order (each may use the earlier ones).
        for c in &p.constants {
            let src = if c.value.trim().is_empty() {
                "0"
            } else {
                c.value.as_str()
            };
            let v = match vm.compile_expression(&c.name, src.as_bytes(), &h) {
                Ok(prog) => {
                    let mut dummy = NoWorld;
                    vm.run(
                        &mut dummy,
                        &prog,
                        Ctx::new(gml::special::NOONE, gml::special::NOONE),
                        &[],
                    )
                    .unwrap_or(Value::Real(0.0))
                }
                Err(_) => Value::Real(0.0),
            };
            h.constants.entry(c.name.clone()).or_insert(v);
        }
        h
    }
}

impl Host for CheckHost {
    fn resolve_constant(&self, name: &str) -> Option<Value> {
        self.constants.get(name).cloned()
    }
    fn resolve_script(&self, name: &str) -> Option<i32> {
        self.scripts.get(name).copied()
    }
    fn instance_vars(&mut self, _: i32) -> Option<&mut VarTable> {
        None
    }
    fn instance_exists(&mut self, _: i32) -> bool {
        false
    }
    fn select_instances(&mut self, _: i32, _: &mut Vec<i32>) {}
    fn get_builtin(&mut self, _: &Ctx, _: i32, v: VariableId, _: i32) -> Result<Value, Error> {
        Err(Error::new(v.name()))
    }
    fn set_builtin(
        &mut self,
        _: &Ctx,
        _: i32,
        v: VariableId,
        _: i32,
        _: Value,
    ) -> Result<(), Error> {
        Err(Error::new(v.name()))
    }
    fn call_function(
        &mut self,
        _: &mut Vm,
        _: &Ctx,
        f: FunctionId,
        _: &[Value],
    ) -> Result<Value, Error> {
        Err(Error::new(f.name()))
    }
    fn call_script(&mut self, _: &mut Vm, _: &Ctx, _: i32, _: &[Value]) -> Result<Value, Error> {
        Err(Error::new("script"))
    }
}

/// A world without instances, for evaluating user constants.
struct NoWorld;

impl Host for NoWorld {
    fn instance_vars(&mut self, _: i32) -> Option<&mut VarTable> {
        None
    }
    fn instance_exists(&mut self, _: i32) -> bool {
        false
    }
    fn select_instances(&mut self, _: i32, _: &mut Vec<i32>) {}
    fn get_builtin(&mut self, _: &Ctx, _: i32, v: VariableId, _: i32) -> Result<Value, Error> {
        Err(Error::new(v.name()))
    }
    fn set_builtin(
        &mut self,
        _: &Ctx,
        _: i32,
        v: VariableId,
        _: i32,
        _: Value,
    ) -> Result<(), Error> {
        Err(Error::new(v.name()))
    }
    fn call_function(
        &mut self,
        _: &mut Vm,
        _: &Ctx,
        f: FunctionId,
        _: &[Value],
    ) -> Result<Value, Error> {
        Err(Error::new(f.name()))
    }
    fn call_script(&mut self, _: &mut Vm, _: &Ctx, _: i32, _: &[Value]) -> Result<Value, Error> {
        Err(Error::new("script"))
    }
}

// ====================================================================== facts

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Scope {
    /// `name` or `self.name`: the current instance (or a `globalvar`).
    Plain,
    /// `global.name`
    Global,
    /// `owner.name` with another owner.
    Field,
}

#[derive(Debug)]
struct Read {
    name: String,
    scope: Scope,
    span: Span,
    in_with: bool,
    /// An array element read (`a[i]`).
    indexed: bool,
}

#[derive(Debug)]
struct Assign {
    name: String,
    scope: Scope,
    in_with: bool,
}

#[derive(Debug)]
struct Call {
    name: String,
    argc: usize,
    span: Span,
    in_with: bool,
    via_execute: bool,
}

/// What the analysis learns from one code piece.
#[derive(Debug, Default)]
struct Facts {
    reads: Vec<Read>,
    /// `=` assignments (and the string forms `variable_*_set('name', ...)`).
    assigns: Vec<Assign>,
    globalvars: Vec<String>,
    calls: Vec<Call>,
    /// (position, message)
    hazards: Vec<(Span, String)>,
    lite: Vec<(Span, String)>,
}

struct Walker<'a> {
    host: &'a CheckHost,
    locals: HashSet<String>,
    with_depth: u32,
    /// Set while walking the target of an index expression.
    indexing: bool,
    f: Facts,
}

/// Lite-edition-only function families (specs/lite-edition.md), by prefix.
const LITE_PREFIXES: &[(&str, &str)] = &[
    ("ds_", "data structure"),
    ("surface_", "surface"),
    ("d3d_", "3D"),
    ("part_", "particle"),
    ("effect_", "effect"),
    ("external_", "DLL"),
    ("mplay_", "multiplayer"),
    ("draw_set_blend_mode", "blend mode"),
    ("sprite_add", "sprite loading"),
    ("sprite_replace", "sprite loading"),
    ("background_add", "background loading"),
    ("background_replace", "background loading"),
];
/// Lite-edition-only functions (rotated / blended drawing), by exact name.
const LITE_NAMES: &[&str] = &[
    "draw_sprite_ext",
    "draw_sprite_general",
    "draw_sprite_stretched_ext",
    "draw_sprite_part_ext",
    "draw_sprite_tiled_ext",
    "draw_background_ext",
    "draw_background_stretched_ext",
    "draw_background_part_ext",
    "draw_background_tiled_ext",
    "draw_background_general",
];

/// The Lite-edition message for a call of `name`, if it is Standard-only. Function
/// families share one message so a report lists each family once.
fn lite_function(name: &str) -> Option<String> {
    if name == "draw_self" {
        return Some("draw_self draws with image_blend/image_angle (Standard-only; its Lite status is not verified)".into());
    }
    if LITE_NAMES.contains(&name) {
        return Some(format!(
            "{} (blended/rotated drawing) is not available in the Lite edition",
            name
        ));
    }
    LITE_PREFIXES
        .iter()
        .find(|(p, _)| name.starts_with(p))
        .map(|(p, what)| {
            format!(
                "{}* ({} functions) are not available in the Lite edition",
                p, what
            )
        })
}

/// A comparable text for simple owner expressions (`target`, `other.target`, `(t)`).
fn key(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Ident(n) => Some(n.clone()),
        ExprKind::Field(o, n, _) => Some(format!("{}.{}", key(o)?, n)),
        ExprKind::Paren(x) => key(x),
        _ => None,
    }
}

/// Owners guarded in a condition: `instance_exists(E)` and `E != noone`.
fn guards(e: &Expr, out: &mut Vec<String>) {
    match &e.kind {
        ExprKind::Call(n, args) if n == "instance_exists" && args.len() == 1 => {
            if let Some(k) = key(&args[0]) {
                out.push(k);
            }
        }
        ExprKind::Binary(BinOp::Ne, a, b) | ExprKind::Binary(BinOp::Gt, a, b) => {
            let is_noone = |x: &Expr| matches!(&x.kind, ExprKind::Ident(n) if n == "noone");
            if is_noone(b) {
                if let Some(k) = key(a) {
                    out.push(k);
                }
            } else if is_noone(a) {
                if let Some(k) = key(b) {
                    out.push(k);
                }
            }
            guards(a, out);
            guards(b, out);
        }
        ExprKind::Binary(_, a, b) => {
            guards(a, out);
            guards(b, out);
        }
        ExprKind::Unary(_, x) | ExprKind::Paren(x) => guards(x, out),
        _ => {}
    }
}

/// The first `E.name` read in `e` whose owner `E` is one of `keys`.
fn guarded_read(e: &Expr, keys: &[String]) -> Option<(String, Span)> {
    match &e.kind {
        ExprKind::Field(o, n, sp) => {
            if let Some(k) = key(o) {
                if keys.contains(&k) {
                    return Some((format!("{}.{}", k, n), *sp));
                }
            }
            guarded_read(o, keys)
        }
        ExprKind::Index(t, idx) => {
            guarded_read(t, keys).or_else(|| idx.iter().find_map(|i| guarded_read(i, keys)))
        }
        ExprKind::Call(_, args) => args.iter().find_map(|a| guarded_read(a, keys)),
        ExprKind::Unary(_, x) | ExprKind::Paren(x) => guarded_read(x, keys),
        ExprKind::Binary(_, a, b) => guarded_read(a, keys).or_else(|| guarded_read(b, keys)),
        _ => None,
    }
}

/// How the compiler resolves a plain identifier.
enum Name {
    Local,
    Builtin,
    Constant,
    Function,
    User,
}

impl<'a> Walker<'a> {
    fn new(host: &'a CheckHost) -> Self {
        Walker {
            host,
            locals: HashSet::new(),
            with_depth: 0,
            indexing: false,
            f: Facts::default(),
        }
    }

    fn classify(&self, name: &str) -> Name {
        if self.locals.contains(name) {
            Name::Local
        } else if builtins::variable_id(name).is_some() {
            Name::Builtin
        } else if self.host.constants.contains_key(name) || builtins::constant_value(name).is_some()
        {
            Name::Constant
        } else if builtins::function_id(name).is_some() {
            Name::Function
        } else {
            Name::User
        }
    }

    fn in_with(&self) -> bool {
        self.with_depth > 0
    }

    fn stmts(&mut self, v: &[Stmt]) {
        for s in v {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Empty | StmtKind::Break | StmtKind::Continue | StmtKind::Exit => {}
            StmtKind::Block(v) => self.stmts(v),
            StmtKind::Var(names) => {
                for (n, _) in names {
                    self.locals.insert(n.clone());
                }
            }
            StmtKind::GlobalVar(names) => {
                for (n, _) in names {
                    self.f.globalvars.push(n.clone());
                }
            }
            StmtKind::If(c, t, e) => {
                self.expr(c);
                self.stmt(t);
                if let Some(e) = e {
                    self.stmt(e);
                }
            }
            StmtKind::Repeat(c, b) | StmtKind::While(c, b) => {
                self.expr(c);
                self.stmt(b);
            }
            StmtKind::DoUntil(b, c) => {
                self.stmt(b);
                self.expr(c);
            }
            StmtKind::For(i, c, st, b) => {
                self.stmt(i);
                self.expr(c);
                self.stmt(st);
                self.stmt(b);
            }
            StmtKind::Switch(v, items) => {
                self.expr(v);
                for it in items {
                    match it {
                        SwitchItem::Case(e) => self.expr(e),
                        SwitchItem::Default(_) => {}
                        SwitchItem::Stmt(s) => self.stmt(s),
                    }
                }
            }
            StmtKind::With(o, b) => {
                self.expr(o);
                self.with_depth += 1;
                self.stmt(b);
                self.with_depth -= 1;
            }
            StmtKind::Return(e) | StmtKind::Call(e) => self.expr(e),
            StmtKind::Assign(target, op, value) => {
                self.lvalue(target, *op);
                self.expr(value);
            }
        }
    }

    fn assign(&mut self, name: &str, scope: Scope, op: AssignOp, span: Span) {
        if op == AssignOp::Set {
            self.f.assigns.push(Assign {
                name: name.to_string(),
                scope,
                in_with: self.in_with(),
            });
        } else {
            // `a += 1` reads `a` first
            self.read(name, scope, span);
        }
    }

    fn read(&mut self, name: &str, scope: Scope, span: Span) {
        let indexed = std::mem::take(&mut self.indexing);
        self.f.reads.push(Read {
            name: name.to_string(),
            scope,
            span,
            in_with: self.in_with(),
            indexed,
        });
    }

    fn builtin_write(&mut self, name: &str, span: Span) {
        match name {
            "image_blend" => self.f.lite.push((span, "writing image_blend is not available in the Lite edition".into())),
            "image_angle" => self.f.lite.push((
                span,
                "image_angle: rotated sprite drawing is not available in the Lite edition (rotated collision masks do work)".into(),
            )),
            _ => {}
        }
    }

    /// The scope of `owner.name` for a user variable name, walking the owner.
    fn owner_scope(&mut self, owner: &Expr) -> Option<Scope> {
        let inner = match &owner.kind {
            ExprKind::Paren(x) => x,
            _ => owner,
        };
        if let ExprKind::Ident(n) = &inner.kind {
            match n.as_str() {
                "global" => return Some(Scope::Global),
                "self" => return Some(Scope::Plain),
                "local" => return None,
                _ => {}
            }
        }
        self.expr(owner);
        Some(Scope::Field)
    }

    fn lvalue(&mut self, e: &Expr, op: AssignOp) {
        match &e.kind {
            ExprKind::Index(t, idx) => {
                for i in idx {
                    self.expr(i);
                }
                self.lvalue(t, op);
            }
            ExprKind::Ident(n) => match self.classify(n) {
                Name::User => self.assign(n, Scope::Plain, op, e.span),
                Name::Builtin => self.builtin_write(n, e.span),
                _ => {}
            },
            ExprKind::Field(owner, n, sp) => {
                let scope = self.owner_scope(owner);
                match builtins::variable_id(n) {
                    Some(id)
                        if !(scope == Some(Scope::Global)
                            && id.info().scope == VarScope::Instance) =>
                    {
                        if scope != Some(Scope::Global) {
                            self.builtin_write(n, *sp);
                        }
                    }
                    _ => {
                        if let Some(scope) = scope {
                            self.assign(n, scope, op, *sp);
                        }
                    }
                }
            }
            _ => self.expr(e),
        }
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Real(_) | ExprKind::Str(_) => {}
            ExprKind::Ident(n) => {
                if let Name::User = self.classify(n) {
                    self.read(n, Scope::Plain, e.span);
                }
            }
            ExprKind::Field(owner, n, sp) => {
                let ix = std::mem::take(&mut self.indexing);
                let scope = self.owner_scope(owner);
                self.indexing = ix;
                match builtins::variable_id(n) {
                    Some(id)
                        if !(scope == Some(Scope::Global)
                            && id.info().scope == VarScope::Instance) =>
                    {
                        self.indexing = false;
                    }
                    _ => {
                        if let Some(scope) = scope {
                            self.read(n, scope, *sp);
                        }
                    }
                }
            }
            ExprKind::Index(t, idx) => {
                for i in idx {
                    self.expr(i);
                }
                self.indexing = true;
                self.expr(t);
                self.indexing = false;
            }
            ExprKind::Call(name, args) => {
                for a in args {
                    self.expr(a);
                }
                self.call(name, args, e.span);
            }
            ExprKind::Unary(_, x) | ExprKind::Paren(x) => self.expr(x),
            ExprKind::Binary(op, a, b) => {
                self.expr(a);
                self.expr(b);
                if matches!(op, BinOp::And | BinOp::Or) {
                    let mut keys = Vec::new();
                    guards(a, &mut keys);
                    if !keys.is_empty() {
                        if let Some((what, sp)) = guarded_read(b, &keys) {
                            let word = if *op == BinOp::And { "&&" } else { "||" };
                            self.f.hazards.push((
                                sp,
                                format!(
                                    "{} is evaluated even when the left side of {} already decides the result (GM 8.1 has no short-circuit evaluation); nest the ifs",
                                    what, word
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }

    fn call(&mut self, name: &str, args: &[Expr], span: Span) {
        let is_script = self.host.scripts.contains_key(name);
        self.f.calls.push(Call {
            name: name.to_string(),
            argc: args.len(),
            span,
            in_with: self.in_with(),
            via_execute: false,
        });
        if is_script {
            return;
        }
        if let Some(m) = lite_function(name) {
            self.f.lite.push((span, m));
        }
        let lit = |i: usize| -> Option<String> {
            match &args.get(i)?.kind {
                ExprKind::Str(s) => Some(s.iter().map(|&b| b as char).collect()),
                _ => None,
            }
        };
        match name {
            "script_execute" => {
                if let Some(ExprKind::Ident(s)) = args.first().map(|a| &a.kind) {
                    if self.host.scripts.contains_key(s) {
                        self.f.calls.push(Call {
                            name: s.clone(),
                            argc: args.len() - 1,
                            span,
                            in_with: self.in_with(),
                            via_execute: true,
                        });
                    }
                }
            }
            "variable_local_set" | "variable_local_array_set" | "variable_local_array2_set" => {
                if let Some(n) = lit(0) {
                    self.f.assigns.push(Assign {
                        name: n,
                        scope: Scope::Plain,
                        in_with: self.in_with(),
                    });
                }
            }
            "variable_global_set" | "variable_global_array_set" | "variable_global_array2_set" => {
                if let Some(n) = lit(0) {
                    self.f.assigns.push(Assign {
                        name: n,
                        scope: Scope::Global,
                        in_with: self.in_with(),
                    });
                }
            }
            "execute_string" => {
                // assignments made by a literal code string count as assignments
                if let Some(code) = lit(0) {
                    let bytes: Vec<u8> = code.chars().map(|c| c as u32 as u8).collect();
                    if let Ok(ast) = gml::parser::parse_program(&bytes) {
                        let mut w = Walker::new(self.host);
                        w.with_depth = self.with_depth;
                        w.stmts(&ast);
                        self.f.assigns.extend(w.f.assigns);
                        self.f.globalvars.extend(w.f.globalvars);
                    }
                }
            }
            _ => {}
        }
    }
}

// ====================================================================== driver

struct Analysed {
    piece: Piece,
    facts: Facts,
}

fn diag(
    piece: &Piece,
    sev: Severity,
    kind: &'static str,
    span: Option<(u32, u32)>,
    message: String,
) -> Diagnostic {
    let event = match (&piece.event, &piece.note) {
        (Some(e), Some(n)) => Some(format!("{} ({})", e, n)),
        (Some(e), None) => Some(e.clone()),
        (None, Some(n)) => Some(n.clone()),
        (None, None) => None,
    };
    Diagnostic {
        severity: sev,
        kind,
        resource: piece.resource.clone(),
        event,
        action: piece.action,
        line: span.map(|s| s.0),
        col: span.map(|s| s.1),
        source_line: span.map(|s| gml::source_line(piece.src.as_bytes(), s.0)),
        message,
    }
}

fn at(s: Span) -> Option<(u32, u32)> {
    Some((s.line, s.col))
}

/// Whether code text has a line feed not preceded by a carriage return.
fn has_bare_lf(s: &str) -> bool {
    let b = s.as_bytes();
    b.iter()
        .enumerate()
        .any(|(i, &c)| c == b'\n' && (i == 0 || b[i - 1] != b'\r'))
}

/// Runs the code checks; returns the number of code pieces.
pub fn check(p: &Project, opts: &Options, out: &mut Vec<Diagnostic>) -> usize {
    let mut vm = Vm::new();
    let host = CheckHost::new(p, &mut vm);
    let all = pieces::collect(p);
    let count = all.len();

    // ---------------------------------------------------------------- compile
    let mut script_info: HashMap<usize, (usize, bool)> = HashMap::new();
    let mut ok: Vec<Analysed> = Vec::new();
    let mut lf_pieces: Vec<String> = Vec::new();
    for piece in all {
        if piece.note.is_none() && has_bare_lf(&piece.src) {
            lf_pieces.push(location(&piece));
        }
        let src = piece.src.as_bytes();
        let name = location(&piece);
        let compiled = match piece.form {
            Form::Program => vm.compile(&name, src, &host),
            Form::Expression => vm.compile_expression(&name, src, &host),
        };
        match compiled {
            Err(e) => {
                let mut sev = Severity::Error;
                let mut msg = e.message.clone();
                if !p.extension_packages.is_empty() && msg.starts_with("Unknown function or script")
                {
                    sev = Severity::Warning;
                    msg.push_str(" (may come from an extension package; not checked)");
                }
                out.push(diag(&piece, sev, "compile", Some((e.line, e.col)), msg));
            }
            Ok(prog) => {
                if let Owner::Script(i) = piece.owner {
                    script_info.insert(i, (prog.arg_required(), prog.uses_argument_array()));
                }
                let mut w = Walker::new(&host);
                match piece.form {
                    Form::Program => {
                        if let Ok(ast) = gml::parser::parse_program(src) {
                            w.stmts(&ast);
                        }
                    }
                    Form::Expression => {
                        if let Ok(ast) = gml::parser::parse_expression(src) {
                            w.expr(&ast);
                        }
                    }
                }
                ok.push(Analysed { piece, facts: w.f });
            }
        }
    }

    // ---------------------------------------------------------------- per-piece findings
    let mut lite: Vec<(Diagnostic, usize)> = Vec::new();
    let mut lite_seen: HashMap<String, usize> = HashMap::new();
    for a in &ok {
        let piece = &a.piece;
        if p.settings.argument_errors {
            for c in &a.facts.calls {
                let Some(&si) = host.scripts.get(&c.name) else {
                    continue;
                };
                let Some(&(need, array)) = script_info.get(&(si as usize)) else {
                    continue;
                };
                if c.argc < need || (c.argc > need && !array) {
                    let via = if c.via_execute {
                        " (through script_execute)"
                    } else {
                        ""
                    };
                    out.push(diag(
                        piece,
                        Severity::Error,
                        "script-args",
                        at(c.span),
                        format!(
                            "Illegal argument count calling script \"{}\"{}: the script requires {}{} arguments, {} supplied",
                            c.name,
                            via,
                            if array { "at least " } else { "" },
                            need,
                            c.argc
                        ),
                    ));
                }
            }
        }
        for (sp, m) in &a.facts.hazards {
            out.push(diag(
                piece,
                Severity::Warning,
                "short-circuit",
                at(*sp),
                m.clone(),
            ));
        }
        if opts.lite {
            for (sp, m) in &a.facts.lite {
                match lite_seen.get_mut(m) {
                    Some(k) => lite[*k].1 += 1,
                    None => {
                        lite_seen.insert(m.clone(), lite.len());
                        lite.push((
                            diag(piece, Severity::Warning, "lite", at(*sp), m.clone()),
                            1,
                        ));
                    }
                }
            }
        }
    }
    // Lite findings: the first use of each feature, with the number of uses
    for (mut d, n) in lite {
        if n > 1 {
            d.message.push_str(&format!(" (first of {} uses)", n));
        }
        out.push(d);
    }

    // ---------------------------------------------------------------- variables
    let globalvars: HashSet<&str> = ok
        .iter()
        .flat_map(|a| a.facts.globalvars.iter().map(|s| s.as_str()))
        .collect();
    let mut plain: HashSet<&str> = HashSet::new();
    let mut field: HashSet<&str> = HashSet::new();
    let mut global: HashSet<&str> = HashSet::new();
    for a in &ok {
        for s in &a.facts.assigns {
            match s.scope {
                Scope::Plain => {
                    plain.insert(&s.name);
                    if globalvars.contains(s.name.as_str()) {
                        global.insert(&s.name);
                    }
                }
                Scope::Field => {
                    field.insert(&s.name);
                }
                Scope::Global => {
                    global.insert(&s.name);
                }
            }
        }
    }
    let assigned = |r: &Read| -> bool {
        let n = r.name.as_str();
        match r.scope {
            Scope::Plain => {
                plain.contains(n)
                    || field.contains(n)
                    || (globalvars.contains(n) && global.contains(n))
            }
            Scope::Global => global.contains(n),
            Scope::Field => plain.contains(n) || field.contains(n) || global.contains(n),
        }
    };
    for a in &ok {
        let mut seen = BTreeSet::new();
        for r in &a.facts.reads {
            if assigned(r) || !seen.insert((r.scope, r.name.as_str())) {
                continue;
            }
            let what = match r.scope {
                Scope::Global => format!("global.{}", r.name),
                _ => r.name.clone(),
            };
            out.push(diag(
                &a.piece,
                Severity::Warning,
                "unassigned",
                at(r.span),
                format!(
                    "{} is read but never assigned anywhere in the project",
                    what
                ),
            ));
        }
    }

    if opts.pedantic && !p.settings.treat_uninitialized_as_zero {
        uninitialised(p, &host, &ok, &globalvars, &assigned, out);
    }

    if !lf_pieces.is_empty() {
        let mut list = lf_pieces
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        if lf_pieces.len() > 3 {
            list.push_str("; ...");
        }
        out.push(Diagnostic {
            severity: Severity::Warning,
            kind: "line-endings",
            resource: "project".into(),
            event: None,
            action: None,
            line: None,
            col: None,
            message: format!(
                "{} code piece(s) use LF-only line endings; GM 8.1 stores CR LF and shows LF-only code on one line in error messages ({})",
                lf_pieces.len(),
                list
            ),
            source_line: None,
        });
    }
    count
}

/// `obj / Step / action 2` style location of a piece.
pub fn location(p: &Piece) -> String {
    let mut s = p.resource.clone();
    if let Some(e) = &p.event {
        s.push_str(" / ");
        s.push_str(e);
    }
    if let Some(a) = p.action {
        s.push_str(&format!(" / action {}", a));
    }
    if let Some(n) = &p.note {
        s.push_str(&format!(" ({})", n));
    }
    s
}

/// Check 5 (`--pedantic`): an object reads an instance variable in an event other
/// than Create that neither it, its parents nor its children assign in their
/// initialisation code, and no other code assigns from outside.
fn uninitialised(
    p: &Project,
    host: &CheckHost,
    ok: &[Analysed],
    globalvars: &HashSet<&str>,
    assigned: &dyn Fn(&Read) -> bool,
    out: &mut Vec<Diagnostic>,
) {
    let nobj = p.objects.len();
    // assignments each script makes to its caller's instance (transitively)
    let mut script_assigns: HashMap<usize, HashSet<String>> = HashMap::new();
    let mut script_calls: HashMap<usize, Vec<usize>> = HashMap::new();
    for a in ok {
        if let Owner::Script(i) = a.piece.owner {
            let set = script_assigns.entry(i).or_default();
            for s in &a.facts.assigns {
                if s.scope == Scope::Plain && !s.in_with {
                    set.insert(s.name.clone());
                }
            }
            let calls = script_calls.entry(i).or_default();
            for c in &a.facts.calls {
                if !c.in_with {
                    if let Some(&si) = host.scripts.get(&c.name) {
                        calls.push(si as usize);
                    }
                }
            }
        }
    }
    loop {
        let mut changed = false;
        let keys: Vec<usize> = script_calls.keys().copied().collect();
        for s in keys {
            let callees = script_calls[&s].clone();
            for c in callees {
                let add: Vec<String> = script_assigns
                    .get(&c)
                    .map(|x| x.iter().cloned().collect())
                    .unwrap_or_default();
                let set = script_assigns.entry(s).or_default();
                for n in add {
                    changed |= set.insert(n);
                }
            }
        }
        if !changed {
            break;
        }
    }
    let script_set = |name: &str| -> Vec<String> {
        host.scripts
            .get(name)
            .and_then(|&i| script_assigns.get(&(i as usize)))
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    };

    // initialisation per object, and assignments made from outside an instance
    let mut init: Vec<HashSet<String>> = vec![HashSet::new(); nobj];
    // everything an object's own code assigns to itself, in any event
    let mut anywhere: Vec<HashSet<String>> = vec![HashSet::new(); nobj];
    let mut external: HashSet<String> = HashSet::new();
    for a in ok {
        if let Owner::Object { obj, .. } = a.piece.owner {
            for s in a
                .facts
                .assigns
                .iter()
                .filter(|s| s.scope == Scope::Plain && !s.in_with)
            {
                anywhere[obj].insert(s.name.clone());
            }
            for c in a.facts.calls.iter().filter(|c| !c.in_with) {
                anywhere[obj].extend(script_set(&c.name));
            }
        }
        let init_obj = match a.piece.owner {
            Owner::Object {
                obj,
                ev_type,
                ev_num,
            } => {
                let is_init = ev_type == event_type::CREATE
                    || (ev_type == event_type::OTHER && matches!(ev_num, 2 | 4 | 10..=25));
                is_init.then_some(obj)
            }
            Owner::Instance { object } => usize::try_from(object).ok().filter(|&o| o < nobj),
            _ => None,
        };
        for s in &a.facts.assigns {
            if s.in_with || s.scope == Scope::Field {
                external.insert(s.name.clone());
            } else if let (Some(o), Scope::Plain) = (init_obj, s.scope) {
                init[o].insert(s.name.clone());
            }
        }
        for c in &a.facts.calls {
            if c.in_with {
                external.extend(script_set(&c.name));
            } else if let Some(o) = init_obj {
                init[o].extend(script_set(&c.name));
            }
        }
    }

    // family = ancestors + self + descendants
    let parent = |o: usize| -> Option<usize> {
        let par = p.objects.get(o)?.as_ref()?.parent;
        if par == NO_PARENT || par < 0 {
            None
        } else {
            usize::try_from(par)
                .ok()
                .filter(|&x| x < nobj && p.objects[x].is_some())
        }
    };
    let ancestors = |o: usize| -> Vec<usize> {
        let mut v = vec![o];
        let mut cur = o;
        while let Some(par) = parent(cur) {
            if v.contains(&par) {
                break;
            }
            v.push(par);
            cur = par;
        }
        v
    };
    let mut reported: HashSet<(usize, String)> = HashSet::new();
    for a in ok {
        let Owner::Object { obj, ev_type, .. } = a.piece.owner else {
            continue;
        };
        if ev_type == event_type::CREATE {
            continue;
        }
        // what this piece assigns itself, directly or through the scripts it calls
        let mut own: HashSet<String> = a
            .facts
            .assigns
            .iter()
            .filter(|s| s.scope == Scope::Plain)
            .map(|s| s.name.clone())
            .collect();
        for c in a.facts.calls.iter().filter(|c| !c.in_with) {
            own.extend(script_set(&c.name));
        }
        let family: Vec<usize> = (0..nobj)
            .filter(|&o| {
                p.objects[o].is_some()
                    && (ancestors(o).contains(&obj) || ancestors(obj).contains(&o))
            })
            .collect();
        for r in &a.facts.reads {
            if r.scope != Scope::Plain || r.in_with || !assigned(r) {
                continue;
            }
            let n = r.name.as_str();
            if own.contains(n) || external.contains(n) || globalvars.contains(n) {
                continue;
            }
            if family.iter().any(|&o| init[o].contains(n)) {
                continue;
            }
            // arrays filled later (`a[n] = v` in another event) are usually guarded by
            // a count; don't report their element reads
            if r.indexed && family.iter().any(|&o| anywhere[o].contains(n)) {
                continue;
            }
            if !reported.insert((obj, r.name.clone())) {
                continue;
            }
            let oname = &p.objects[obj].as_ref().unwrap().name;
            out.push(diag(
                &a.piece,
                Severity::Warning,
                "uninitialised",
                at(r.span),
                format!("{} is possibly uninitialised: {} (and its parents/children) never assign it in Create or instance creation code", n, oname),
            ));
        }
    }
}
