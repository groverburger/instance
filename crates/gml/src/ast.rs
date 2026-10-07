//! Abstract syntax tree produced by the [`parser`](crate::parser).

use crate::lexer::Span;
use crate::value::{BinOp, UnOp};

/// An expression with its source position.
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    /// The expression.
    pub kind: ExprKind,
    /// Position of the expression's first token (for binary operators: the operator).
    pub span: Span,
}

/// Expression kinds.
#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    /// Numeric literal.
    Real(f64),
    /// String literal.
    Str(Vec<u8>),
    /// Bare identifier (variable, constant, resource name...).
    Ident(String),
    /// `owner.name`
    Field(Box<Expr>, String, Span),
    /// `var[i]` or `var[i, j]`; the target is an `Ident` or `Field`.
    Index(Box<Expr>, Vec<Expr>),
    /// `name(args...)`
    Call(String, Vec<Expr>),
    /// Prefix operator.
    Unary(UnOp, Box<Expr>),
    /// Binary operator.
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// Parenthesised expression (kept so `(a).b` and call-as-owner checks work).
    Paren(Box<Expr>),
}

/// Assignment operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    /// `=` or `:=`
    Set,
    /// `+=`, `-=`, ... : `a op= e` means `a = a op e`.
    Compound(BinOp),
}

/// A statement with its source position.
#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    /// The statement.
    pub kind: StmtKind,
    /// Position of the first token.
    pub span: Span,
}

/// An item of a `switch` body.
#[derive(Clone, Debug, PartialEq)]
pub enum SwitchItem {
    /// `case expr:`
    Case(Expr),
    /// `default:`
    Default(Span),
    /// Ordinary statement.
    Stmt(Stmt),
}

/// Statement kinds.
#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    /// `;`
    Empty,
    /// `{ ... }` / `begin ... end`
    Block(Vec<Stmt>),
    /// `var a, b`
    Var(Vec<(String, Span)>),
    /// `globalvar a, b`
    GlobalVar(Vec<(String, Span)>),
    /// `if c s [else s]`
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    /// `repeat n s`
    Repeat(Expr, Box<Stmt>),
    /// `while c s`
    While(Expr, Box<Stmt>),
    /// `do s until c`
    DoUntil(Box<Stmt>, Expr),
    /// `for (init; cond; step) body`
    For(Box<Stmt>, Expr, Box<Stmt>, Box<Stmt>),
    /// `switch v { ... }`
    Switch(Expr, Vec<SwitchItem>),
    /// `with o s`
    With(Expr, Box<Stmt>),
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `exit`
    Exit,
    /// `return e`
    Return(Expr),
    /// `lvalue op= expr`
    Assign(Expr, AssignOp, Expr),
    /// Function or script call as a statement (the expression is a `Call`).
    Call(Expr),
}
