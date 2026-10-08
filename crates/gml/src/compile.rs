//! Compiler from AST to the resolved execution tree (spec §7).
//!
//! All identifiers are classified at compile time: `var` locals become slot numbers,
//! built-in variables/functions become ids, constants and resource names become
//! literal values, and user variables become interned [`VarId`]s.

use crate::ast::{AssignOp, Expr, ExprKind, Stmt, StmtKind, SwitchItem};
use crate::builtins::{self, FunctionId, VarScope, VariableId};
use crate::error::CompileError;
use crate::host::Host;
use crate::lexer::Span;
use crate::value::{binary, unary, BinOp, UnOp, Value};
use crate::vars::VarId;
use crate::vm::Interner;
use std::collections::HashMap;
use std::rc::Rc;

/// Compact source position stored in the execution tree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pos {
    pub line: u32,
    pub col: u32,
}

impl From<Span> for Pos {
    fn from(s: Span) -> Self {
        Pos {
            line: s.line,
            col: s.col,
        }
    }
}

/// Script-scope (argument family) variables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArgVar {
    Slot(u8),
    Array,
    Count,
    Relative,
}

/// Variable name after a `.`.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FieldName {
    User(VarId),
    Builtin(VariableId),
    Arg(ArgVar),
}

#[derive(Debug)]
pub(crate) enum Idx {
    None,
    /// Index expressions with their source positions (error positions).
    One(Ex, Pos),
    Two(Ex, Pos, Ex, Pos),
}

#[derive(Debug)]
pub(crate) enum PlaceKind {
    Local(u16),
    /// Plain user variable: global if declared `globalvar`, else on `self`.
    User(VarId),
    /// `self.name`: always the instance variable.
    SelfUser(VarId),
    /// `global.name`
    Global(VarId),
    /// Built-in variable with implicit owner (`self` / the game).
    Builtin(VariableId),
    Arg(ArgVar),
    /// `owner.name` with an owner known only at run time.
    Field(Ex, FieldName),
}

#[derive(Debug)]
pub(crate) struct Place {
    pub kind: PlaceKind,
    pub idx: Idx,
    pub pos: Pos,
}

#[derive(Debug)]
pub(crate) enum Ex {
    Const(Value),
    Local(u16, Pos),
    User(VarId, Pos),
    Builtin(VariableId, Pos),
    Arg(u8),
    Place(Box<Place>),
    Unary(UnOp, Box<Ex>, Pos),
    Binary(BinOp, Box<Ex>, Box<Ex>, Pos),
    Call(FunctionId, Vec<Ex>, Pos),
    Script(i32, Vec<Ex>, Pos),
    /// A condition (if/while/until/for) with the position for "Expression expected".
    At(Box<Ex>, Pos),
}

#[derive(Debug)]
pub(crate) struct Switch {
    pub value: Ex,
    pub cases: Vec<(Ex, usize)>,
    pub default: Option<usize>,
    pub body: Vec<St>,
}

#[derive(Debug)]
pub(crate) enum St {
    Nop,
    Block(Vec<St>),
    Assign(Box<Place>, Option<BinOp>, Ex),
    AssignLocal(u16, Option<BinOp>, Ex, Pos),
    Expr(Ex),
    If(Ex, Box<St>, Option<Box<St>>),
    Repeat(Ex, Box<St>, Pos),
    While(Ex, Box<St>),
    Do(Box<St>, Ex),
    For(Box<St>, Ex, Box<St>, Box<St>),
    Switch(Box<Switch>),
    With(Ex, Box<St>, Pos),
    GlobalVar(Vec<VarId>),
    Break,
    Continue,
    Exit,
    Return(Ex),
}

/// A compiled program (script, code action, expression, ...), ready to run with
/// [`Vm::run`](crate::Vm::run).
#[derive(Debug)]
pub struct Program {
    pub(crate) name: Rc<str>,
    pub(crate) source: Rc<[u8]>,
    pub(crate) body: Vec<St>,
    pub(crate) local_names: Vec<VarId>,
    /// 1 + the highest `argumentN` index used (0 if none): the argument count a
    /// script call must supply.
    pub(crate) arg_required: u8,
    /// Whether the `argument[i]` form is used (then more arguments are accepted).
    pub(crate) arg_array: bool,
}

impl Program {
    /// The name given at compile time (used in error locations).
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Number of `var` local slots.
    pub fn local_count(&self) -> usize {
        self.local_names.len()
    }
    /// The number of arguments a call of this program as a script must supply
    /// (1 + the highest `argumentN` index it uses).
    pub fn arg_required(&self) -> usize {
        self.arg_required as usize
    }
    /// Whether the program uses the `argument[i]` array form.
    pub fn uses_argument_array(&self) -> bool {
        self.arg_array
    }
}

pub(crate) struct Compiler<'a> {
    pub interner: &'a mut Interner,
    pub host: &'a dyn Host,
    locals: HashMap<String, u16>,
    local_names: Vec<VarId>,
    arg_required: u8,
    arg_array: bool,
}

const MAX_ARGS: usize = 16;

/// The span of an expression without its enclosing parentheses (GM 8.1 positions).
fn inner_span(e: &Expr) -> Span {
    match &e.kind {
        ExprKind::Paren(x) => inner_span(x),
        _ => e.span,
    }
}

fn err<T>(msg: impl Into<String>, span: Span) -> Result<T, CompileError> {
    Err(CompileError::new(msg, span))
}

impl<'a> Compiler<'a> {
    pub fn new(interner: &'a mut Interner, host: &'a dyn Host) -> Self {
        Compiler {
            interner,
            host,
            locals: HashMap::new(),
            local_names: Vec::new(),
            arg_required: 0,
            arg_array: false,
        }
    }

    /// Records the use of an argument variable (for the script argument check).
    fn note_arg(&mut self, a: ArgVar) -> ArgVar {
        match a {
            ArgVar::Slot(n) => self.arg_required = self.arg_required.max(n + 1),
            ArgVar::Array => self.arg_array = true,
            _ => {}
        }
        a
    }

    pub fn finish(self, name: &str, source: &[u8], body: Vec<St>) -> Program {
        Program {
            name: Rc::from(name),
            source: Rc::from(source),
            body,
            local_names: self.local_names,
            arg_required: self.arg_required,
            arg_array: self.arg_array,
        }
    }

    fn declare_local(&mut self, name: &str) -> u16 {
        if let Some(&s) = self.locals.get(name) {
            return s;
        }
        if let Some(&s) = self.locals.get(&format!("local.{}", name)) {
            self.locals.insert(name.to_string(), s);
            return s;
        }
        let slot = self.local_names.len() as u16;
        let id = self.interner.intern(name);
        self.local_names.push(id);
        self.locals.insert(name.to_string(), slot);
        slot
    }

    // ------------------------------------------------------------- statements

    pub fn block(&mut self, stmts: &[Stmt]) -> Result<Vec<St>, CompileError> {
        let mut out = Vec::with_capacity(stmts.len());
        for s in stmts {
            match self.stmt(s)? {
                St::Nop => {}
                st => out.push(st),
            }
        }
        Ok(out)
    }

    fn boxed(&mut self, s: &Stmt) -> Result<Box<St>, CompileError> {
        Ok(Box::new(self.stmt(s)?))
    }

    pub fn stmt(&mut self, s: &Stmt) -> Result<St, CompileError> {
        Ok(match &s.kind {
            StmtKind::Empty => St::Nop,
            StmtKind::Block(v) => {
                let mut b = self.block(v)?;
                match b.len() {
                    0 => St::Nop,
                    1 => b.pop().unwrap(),
                    _ => St::Block(b),
                }
            }
            StmtKind::Var(names) => {
                for (n, span) in names {
                    // GM 8.1: `var x` fails with "Cannot redeclare a builtin variable."
                    // (gml-language.md §3 note 7, verified black-box with bb_perf)
                    if builtins::variable_id(n).is_some() {
                        return err("Cannot redeclare a builtin variable.", *span);
                    }
                    self.declare_local(n);
                }
                St::Nop
            }
            StmtKind::GlobalVar(names) => {
                for (n, span) in names {
                    if builtins::variable_id(n).is_some() {
                        return err("Cannot redeclare a builtin variable.", *span);
                    }
                }
                St::GlobalVar(names.iter().map(|(n, _)| self.interner.intern(n)).collect())
            }
            StmtKind::If(c, t, e) => {
                let c = self.cond(c)?;
                let t = self.boxed(t)?;
                let e = match e {
                    Some(e) => Some(self.boxed(e)?),
                    None => None,
                };
                St::If(c, t, e)
            }
            StmtKind::Repeat(c, b) => {
                St::Repeat(self.expr(c)?, self.boxed(b)?, inner_span(c).into())
            }
            StmtKind::While(c, b) => St::While(self.cond(c)?, self.boxed(b)?),
            StmtKind::DoUntil(b, c) => {
                let b = self.boxed(b)?;
                St::Do(b, self.cond(c)?)
            }
            StmtKind::For(i, c, st, b) => {
                let i = self.boxed(i)?;
                let c = self.cond(c)?;
                let st = self.boxed(st)?;
                let b = self.boxed(b)?;
                St::For(i, c, st, b)
            }
            StmtKind::Switch(v, items) => {
                let value = self.expr(v)?;
                let mut sw = Switch {
                    value,
                    cases: Vec::new(),
                    default: None,
                    body: Vec::new(),
                };
                for it in items {
                    match it {
                        SwitchItem::Case(e) => {
                            let e = self.expr(e)?;
                            sw.cases.push((e, sw.body.len()));
                        }
                        SwitchItem::Default(_) => {
                            if sw.default.is_none() {
                                sw.default = Some(sw.body.len());
                            }
                        }
                        SwitchItem::Stmt(s) => match self.stmt(s)? {
                            St::Nop => {}
                            st => sw.body.push(st),
                        },
                    }
                }
                St::Switch(Box::new(sw))
            }
            StmtKind::With(o, b) => St::With(self.expr(o)?, self.boxed(b)?, o.span.into()),
            StmtKind::Break => St::Break,
            StmtKind::Continue => St::Continue,
            StmtKind::Exit => St::Exit,
            StmtKind::Return(e) => St::Return(self.expr(e)?),
            StmtKind::Call(e) => St::Expr(self.expr(e)?),
            StmtKind::Assign(target, op, value) => {
                let op = match op {
                    AssignOp::Set => None,
                    AssignOp::Compound(o) => Some(*o),
                };
                let place = self.lvalue(target)?;
                let value = self.expr(value)?;
                if let (PlaceKind::Local(slot), Idx::None) = (&place.kind, &place.idx) {
                    St::AssignLocal(*slot, op, value, place.pos)
                } else {
                    St::Assign(Box::new(place), op, value)
                }
            }
        })
    }

    /// A condition expression, tagged with its position (inside parentheses).
    fn cond(&mut self, c: &Expr) -> Result<Ex, CompileError> {
        Ok(Ex::At(Box::new(self.expr(c)?), inner_span(c).into()))
    }

    // ------------------------------------------------------------- places

    fn idx(&mut self, idx: &[Expr]) -> Result<Idx, CompileError> {
        Ok(match idx {
            [] => Idx::None,
            [a] => Idx::One(self.expr(a)?, a.span.into()),
            [a, b] => Idx::Two(self.expr(a)?, a.span.into(), self.expr(b)?, b.span.into()),
            _ => unreachable!("parser allows at most two indices"),
        })
    }

    fn arg_var(name: &str) -> Option<ArgVar> {
        Some(match name {
            "argument" => ArgVar::Array,
            "argument_count" => ArgVar::Count,
            "argument_relative" => ArgVar::Relative,
            _ => {
                let n: u8 = name.strip_prefix("argument")?.parse().ok()?;
                if n < 16 {
                    ArgVar::Slot(n)
                } else {
                    return None;
                }
            }
        })
    }

    fn field_name(&mut self, name: &str) -> FieldName {
        if let Some(id) = builtins::variable_id(name) {
            if id.info().scope == VarScope::Script {
                if let Some(a) = Self::arg_var(name) {
                    return FieldName::Arg(a);
                }
            }
            return FieldName::Builtin(id);
        }
        FieldName::User(self.interner.intern(name))
    }

    /// Compiles a variable reference (an `Ident`, `Field` or `Index` node).
    fn place(&mut self, e: &Expr, write: bool) -> Result<Place, CompileError> {
        let (target, idx) = match &e.kind {
            ExprKind::Index(t, i) => (&**t, self.idx(i)?),
            _ => (e, Idx::None),
        };
        let pos: Pos = target.span.into();
        let kind = match &target.kind {
            ExprKind::Ident(name) => {
                if let Some(&slot) = self.locals.get(name.as_str()) {
                    PlaceKind::Local(slot)
                } else if let Some(id) = builtins::variable_id(name) {
                    let _ = write;
                    match id.info().scope {
                        VarScope::Script => {
                            let a = Self::arg_var(name).expect("argument family");
                            PlaceKind::Arg(self.note_arg(a))
                        }
                        _ => PlaceKind::Builtin(id),
                    }
                } else if builtins::constant_value(name).is_some()
                    || self.host.resolve_constant(name).is_some()
                {
                    if write {
                        return err(
                            format!("Cannot assign to the constant {}.", name),
                            target.span,
                        );
                    }
                    return err(
                        format!("The constant {} cannot be used as an array.", name),
                        target.span,
                    );
                } else if builtins::function_id(name).is_some() {
                    return err(
                        format!("Function {} used without brackets.", name),
                        target.span,
                    );
                } else {
                    PlaceKind::User(self.interner.intern(name))
                }
            }
            ExprKind::Field(owner, name, nspan) => {
                let fname = self.field_name(name);
                // A name after `.` that is a constant (resource name, user or built-in
                // constant) is not a variable: GM 8.1 rejects `global.obj_name` with
                // "Variable name expected." (gml-language.md §3 note 12, verified black-box).
                if matches!(fname, FieldName::User(_))
                    && (builtins::constant_value(name).is_some()
                        || self.host.resolve_constant(name).is_some())
                {
                    return err("Variable name expected.", *nspan);
                }
                // (read-only built-ins are checked when the assignment runs: GM 8.1
                // reports "Cannot assign to the variable" at run time, VERIFIED
                // black-box bb_lang type.readonly_id, args.write_count)
                if let FieldName::Arg(a) = fname {
                    self.note_arg(a);
                }
                let owner_ex = self.expr(owner)?;
                let pos: Pos = (*nspan).into();
                let kind = match (&owner_ex, fname) {
                    (Ex::Const(Value::Real(o)), FieldName::User(v)) if *o == -5.0 => {
                        PlaceKind::Global(v)
                    }
                    (Ex::Const(Value::Real(o)), FieldName::User(v)) if *o == -1.0 => {
                        PlaceKind::SelfUser(v)
                    }
                    (Ex::Const(Value::Real(o)), FieldName::User(_)) if *o == -7.0 => {
                        PlaceKind::Local(self.declare_local_silent(name))
                    }
                    (Ex::Const(Value::Real(o)), FieldName::Builtin(b)) if *o == -1.0 => {
                        PlaceKind::Builtin(b)
                    }
                    (Ex::Const(Value::Real(o)), FieldName::Builtin(b))
                        if *o == -5.0 && b.info().scope == VarScope::Global =>
                    {
                        PlaceKind::Builtin(b)
                    }
                    (Ex::Const(Value::Real(o)), FieldName::Builtin(b)) if *o == -5.0 => {
                        // `global.x` where x is an instance built-in: a user global named x.
                        PlaceKind::Global(self.interner.intern(b.name()))
                    }
                    (_, FieldName::Arg(a)) => PlaceKind::Arg(a),
                    _ => PlaceKind::Field(owner_ex, fname),
                };
                return Ok(Place { kind, idx, pos });
            }
            ExprKind::Call(..) => {
                return err(
                    "A function result cannot be used as a variable.",
                    target.span,
                )
            }
            _ => return err("Variable expected.", target.span),
        };
        Ok(Place { kind, idx, pos })
    }

    /// `local.name` refers to the current program's locals without making the plain
    /// name local.
    fn declare_local_silent(&mut self, name: &str) -> u16 {
        if let Some(&s) = self.locals.get(name) {
            return s;
        }
        let key = format!("local.{}", name);
        if let Some(&s) = self.locals.get(&key) {
            return s;
        }
        let slot = self.local_names.len() as u16;
        let id = self.interner.intern(name);
        self.local_names.push(id);
        // Register under a name no identifier can have so later `local.name` finds it.
        self.locals.insert(key, slot);
        slot
    }

    fn lvalue(&mut self, e: &Expr) -> Result<Place, CompileError> {
        self.place(e, true)
    }

    // ------------------------------------------------------------- expressions

    pub fn expr(&mut self, e: &Expr) -> Result<Ex, CompileError> {
        let pos: Pos = e.span.into();
        Ok(match &e.kind {
            ExprKind::Real(v) => Ex::Const(Value::Real(*v)),
            ExprKind::Str(s) => Ex::Const(Value::bytes(s)),
            ExprKind::Paren(inner) => self.expr(inner)?,
            ExprKind::Ident(name) => {
                if let Some(&slot) = self.locals.get(name.as_str()) {
                    Ex::Local(slot, pos)
                } else if let Some(id) = builtins::variable_id(name) {
                    match id.info().scope {
                        VarScope::Script => match self.note_arg(Self::arg_var(name).expect("argument family")) {
                            ArgVar::Slot(n) => Ex::Arg(n),
                            a => Ex::Place(Box::new(Place {
                                kind: PlaceKind::Arg(a),
                                idx: Idx::None,
                                pos,
                            })),
                        },
                        _ => Ex::Builtin(id, pos),
                    }
                } else if let Some(v) = self.host.resolve_constant(name) {
                    // user constants and resource names win over built-in constants
                    // (VERIFIED black-box, bb_clash clash.const_vs_builtin_const)
                    Ex::Const(v)
                } else if let Some(v) = builtins::constant_value(name) {
                    Ex::Const(Value::Real(v))
                } else if builtins::function_id(name).is_some() {
                    return err(format!("Function {} used without brackets.", name), e.span);
                } else {
                    Ex::User(self.interner.intern(name), pos)
                }
            }
            ExprKind::Field(..) | ExprKind::Index(..) => self.place_expr(e)?,
            ExprKind::Call(name, args) => {
                if args.len() > MAX_ARGS {
                    // GM 8.1 wording (bb_lang args.17_args.msg)
                    return err("Wrong number of arguments to function or script.", e.span);
                }
                let cargs = args
                    .iter()
                    .map(|a| self.expr(a))
                    .collect::<Result<Vec<_>, _>>()?;
                // a script named like a built-in function wins (VERIFIED black-box,
                // bb_clash clash.script_abs)
                if let Some(idx) = self.host.resolve_script(name) {
                    Ex::Script(idx, cargs, pos)
                } else if let Some(fid) = builtins::function_id(name) {
                    let info = fid.info();
                    if cargs.len() < info.min_args as usize || cargs.len() > info.max_args as usize
                    {
                        return err(
                            format!("Wrong number of arguments to function {}.", name),
                            e.span,
                        );
                    }
                    Ex::Call(fid, cargs, pos)
                } else {
                    return err(format!("Unknown function or script: {}", name), e.span);
                }
            }
            ExprKind::Unary(op, x) => {
                // GM 8.1 reports operator errors at the (right) operand (VERIFIED
                // black-box, bb_lang mix.*.msg, div0.*.msg)
                let operand_pos: Pos = x.span.into();
                let x = self.expr(x)?;
                if let Ex::Const(v) = &x {
                    match unary(*op, v) {
                        Ok(r) => return Ok(Ex::Const(r)),
                        Err(m) => return err(m, e.span),
                    }
                }
                Ex::Unary(*op, Box::new(x), operand_pos)
            }
            ExprKind::Binary(op, a, b) => {
                let right_pos: Pos = b.span.into();
                let a = self.expr(a)?;
                let b = self.expr(b)?;
                if let (Ex::Const(x), Ex::Const(y)) = (&a, &b) {
                    match binary(*op, x, y) {
                        Ok(r) => return Ok(Ex::Const(r)),
                        // GM 8.1 folds constants while compiling, so any error while
                        // folding (`'a' + 1`, `1 / 0`) is a compile error
                        // (gml-language.md §3 note 13, verified black-box).
                        Err(m) => return err(m, e.span),
                    }
                }
                Ex::Binary(*op, Box::new(a), Box::new(b), right_pos)
            }
        })
    }

    fn place_expr(&mut self, e: &Expr) -> Result<Ex, CompileError> {
        let p = self.place(e, false)?;
        Ok(match (&p.kind, &p.idx) {
            (PlaceKind::Local(s), Idx::None) => Ex::Local(*s, p.pos),
            (PlaceKind::User(v), Idx::None) => Ex::User(*v, p.pos),
            (PlaceKind::Builtin(b), Idx::None) => Ex::Builtin(*b, p.pos),
            (PlaceKind::Arg(ArgVar::Slot(n)), Idx::None) => Ex::Arg(*n),
            _ => Ex::Place(Box::new(p)),
        })
    }
}
