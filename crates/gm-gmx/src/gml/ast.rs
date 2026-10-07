//! AST for Studio GML. Every node records its source span so that unchanged parts can
//! be emitted verbatim.

pub type Span = (usize, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    /// `[| i]` ds_list
    List,
    /// `[? k]` ds_map
    Map,
    /// `[# x, y]` ds_grid
    Grid,
    /// `[@ i]` array by reference
    Array,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Num(String),
    /// Hex digits; `true` when written `0x..` (Studio only).
    Hex(String, bool),
    Str(String),
    Ident(String),
    Paren(Box<Expr>),
    Unary(&'static str, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Field(Box<Expr>, String),
    Index(Box<Expr>, Vec<Expr>),
    Access(AccessKind, Box<Expr>, Vec<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `++x` / `x--` etc.
    IncDec {
        prefix: bool,
        inc: bool,
        target: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Empty,
    Block(Vec<Stmt>),
    /// `var a = 1, b;` (initialisers are Studio only).
    Var(bool /* globalvar */, Vec<(String, Option<Expr>)>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    Repeat(Expr, Box<Stmt>),
    While(Expr, Box<Stmt>),
    DoUntil(Box<Stmt>, Expr),
    For(Option<Box<Stmt>>, Option<Expr>, Option<Box<Stmt>>, Box<Stmt>),
    Switch(Expr, Vec<Stmt>),
    Case(Expr),
    Default,
    With(Expr, Box<Stmt>),
    Break,
    Continue,
    Exit,
    Return(Option<Expr>),
    Assign(Expr, &'static str, Expr),
    /// A call, or a `++`/`--` used as a statement.
    ExprStmt(Expr),
    /// `enum Name { A, B = 3 }` (removed in output; members become literals).
    Enum(String, Vec<(String, Option<Expr>)>),
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}
