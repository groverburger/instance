//! Recursive-descent parser for Studio GML (precedence as in GM 8.1, see
//! specs/gml-language.md §4.1, plus `?:` at the lowest level, `++`/`--`, accessors).

use super::ast::*;
use super::lexer::{Tok, Token};

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

pub struct Parser<'a> {
    toks: &'a [Token],
    pos: usize,
}

type PResult<T> = Result<T, ParseError>;

const KEYWORDS: &[&str] = &[
    "if", "then", "else", "begin", "end", "repeat", "while", "do", "until", "for", "switch",
    "case", "default", "break", "continue", "exit", "return", "with", "var", "globalvar", "and",
    "or", "xor", "not", "div", "mod", "enum",
];

pub fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

impl<'a> Parser<'a> {
    pub fn new(toks: &'a [Token]) -> Self {
        Parser { toks, pos: 0 }
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }
    fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)].tok
    }
    fn start(&self) -> usize {
        self.toks[self.pos].start
    }
    fn prev_end(&self) -> usize {
        if self.pos == 0 {
            0
        } else {
            self.toks[self.pos - 1].end
        }
    }
    fn bump(&mut self) -> &Token {
        let t = &self.toks[self.pos];
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }
    fn is_op(&self, op: &str) -> bool {
        matches!(self.peek(), Tok::Op(o) if *o == op)
    }
    fn is_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Tok::Ident(s) if s == kw)
    }
    fn eat_op(&mut self, op: &str) -> bool {
        if self.is_op(op) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.is_kw(kw) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn err<T>(&self, msg: impl Into<String>) -> PResult<T> {
        Err(ParseError {
            offset: self.start(),
            message: msg.into(),
        })
    }
    fn expect_op(&mut self, op: &str) -> PResult<()> {
        if self.eat_op(op) {
            Ok(())
        } else {
            self.err(format!("expected '{op}', found {:?}", self.peek()))
        }
    }
    fn ident(&mut self) -> PResult<String> {
        match self.peek().clone() {
            Tok::Ident(s) if !is_keyword(&s) => {
                self.bump();
                Ok(s)
            }
            t => self.err(format!("expected identifier, found {t:?}")),
        }
    }

    pub fn at_eof(&self) -> bool {
        matches!(self.peek(), Tok::Eof)
    }

    /// Parse a whole program.
    pub fn program(&mut self) -> PResult<Vec<Stmt>> {
        let mut v = Vec::new();
        while !self.at_eof() {
            if self.eat_op(";") {
                continue;
            }
            v.push(self.statement()?);
        }
        Ok(v)
    }

    fn block_end(&self) -> bool {
        self.is_op("}") || self.is_kw("end")
    }

    pub fn statement(&mut self) -> PResult<Stmt> {
        let start = self.start();
        let kind = self.statement_kind()?;
        Ok(Stmt {
            kind,
            span: (start, self.prev_end()),
        })
    }

    fn body(&mut self) -> PResult<Box<Stmt>> {
        // A body may be a lone `;`.
        if self.is_op(";") {
            let s = self.start();
            self.bump();
            return Ok(Box::new(Stmt {
                kind: StmtKind::Empty,
                span: (s, self.prev_end()),
            }));
        }
        Ok(Box::new(self.statement()?))
    }

    fn statement_kind(&mut self) -> PResult<StmtKind> {
        if self.eat_op("{") || self.eat_kw("begin") {
            let mut v = Vec::new();
            while !self.block_end() {
                if self.at_eof() {
                    return self.err("unterminated block");
                }
                if self.eat_op(";") {
                    continue;
                }
                v.push(self.statement()?);
            }
            self.bump();
            return Ok(StmtKind::Block(v));
        }
        if self.is_op(";") {
            self.bump();
            return Ok(StmtKind::Empty);
        }
        let kw = match self.peek() {
            Tok::Ident(s) => s.clone(),
            _ => String::new(),
        };
        match kw.as_str() {
            "var" | "globalvar" => {
                self.bump();
                let mut names = Vec::new();
                loop {
                    let n = self.ident()?;
                    let init = if self.eat_op("=") || self.eat_op(":=") {
                        Some(self.expr()?)
                    } else {
                        None
                    };
                    names.push((n, init));
                    if !self.eat_op(",") {
                        break;
                    }
                }
                Ok(StmtKind::Var(kw == "globalvar", names))
            }
            "if" => {
                self.bump();
                let c = self.expr()?;
                self.eat_kw("then");
                let t = self.body()?;
                // `;` may precede else.
                if self.is_op(";") && matches!(self.peek_at(1), Tok::Ident(s) if s == "else") {
                    self.bump();
                }
                let e = if self.eat_kw("else") {
                    Some(self.body()?)
                } else {
                    None
                };
                Ok(StmtKind::If(c, t, e))
            }
            "repeat" => {
                self.bump();
                let c = self.expr()?;
                Ok(StmtKind::Repeat(c, self.body()?))
            }
            "while" => {
                self.bump();
                let c = self.expr()?;
                self.eat_kw("do");
                Ok(StmtKind::While(c, self.body()?))
            }
            "with" => {
                self.bump();
                let c = self.expr()?;
                self.eat_kw("do");
                Ok(StmtKind::With(c, self.body()?))
            }
            "do" => {
                self.bump();
                let b = self.body()?;
                if self.is_op(";") && matches!(self.peek_at(1), Tok::Ident(s) if s == "until") {
                    self.bump();
                }
                if !self.eat_kw("until") {
                    return self.err("expected 'until'");
                }
                let c = self.expr()?;
                Ok(StmtKind::DoUntil(b, c))
            }
            "for" => {
                self.bump();
                self.expect_op("(")?;
                let init = if self.is_op(";") {
                    None
                } else {
                    Some(Box::new(self.statement()?))
                };
                self.eat_op(";");
                let cond = if self.is_op(";") {
                    None
                } else {
                    Some(self.expr()?)
                };
                self.eat_op(";");
                let step = if self.is_op(")") {
                    None
                } else {
                    Some(Box::new(self.statement()?))
                };
                self.eat_op(";");
                self.expect_op(")")?;
                Ok(StmtKind::For(init, cond, step, self.body()?))
            }
            "switch" => {
                self.bump();
                let c = self.expr()?;
                if !(self.eat_op("{") || self.eat_kw("begin")) {
                    return self.err("expected '{' after switch");
                }
                let mut v = Vec::new();
                while !self.block_end() {
                    if self.at_eof() {
                        return self.err("unterminated switch");
                    }
                    if self.eat_op(";") {
                        continue;
                    }
                    let s = self.start();
                    if self.eat_kw("case") {
                        let e = self.expr()?;
                        self.expect_op(":")?;
                        v.push(Stmt {
                            kind: StmtKind::Case(e),
                            span: (s, self.prev_end()),
                        });
                    } else if self.eat_kw("default") {
                        self.expect_op(":")?;
                        v.push(Stmt {
                            kind: StmtKind::Default,
                            span: (s, self.prev_end()),
                        });
                    } else {
                        v.push(self.statement()?);
                    }
                }
                self.bump();
                Ok(StmtKind::Switch(c, v))
            }
            "break" => {
                self.bump();
                Ok(StmtKind::Break)
            }
            "continue" => {
                self.bump();
                Ok(StmtKind::Continue)
            }
            "exit" => {
                self.bump();
                Ok(StmtKind::Exit)
            }
            "return" => {
                self.bump();
                let none = match self.peek() {
                    Tok::Eof => true,
                    Tok::Op(o) => matches!(*o, ";" | "}"),
                    Tok::Ident(s) => matches!(s.as_str(), "end" | "case" | "default" | "else"),
                    _ => false,
                };
                if none {
                    Ok(StmtKind::Return(None))
                } else {
                    Ok(StmtKind::Return(Some(self.expr()?)))
                }
            }
            "enum" => {
                self.bump();
                let name = self.ident()?;
                self.expect_op("{")?;
                let mut members = Vec::new();
                while !self.is_op("}") {
                    let m = self.ident()?;
                    let v = if self.eat_op("=") {
                        Some(self.expr()?)
                    } else {
                        None
                    };
                    members.push((m, v));
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op("}")?;
                Ok(StmtKind::Enum(name, members))
            }
            _ => {
                let lhs = self.unary()?;
                let op = match self.peek() {
                    Tok::Op(o)
                        if matches!(
                            *o,
                            "=" | ":=" | "+=" | "-=" | "*=" | "/=" | "%=" | "|=" | "&=" | "^="
                                | "<<=" | ">>="
                        ) =>
                    {
                        Some(*o)
                    }
                    _ => None,
                };
                if let Some(op) = op {
                    if !is_lvalue(&lhs) {
                        return self.err("assignment to a non-variable");
                    }
                    self.bump();
                    let rhs = self.expr()?;
                    return Ok(StmtKind::Assign(lhs, op, rhs));
                }
                match &lhs.kind {
                    ExprKind::Call(..) | ExprKind::IncDec { .. } => Ok(StmtKind::ExprStmt(lhs)),
                    // A call used as the owner of '.', e.g. `f().x` without assignment.
                    _ => Err(ParseError {
                        offset: lhs.span.0,
                        message: "expected assignment or call".into(),
                    }),
                }
            }
        }
    }

    pub fn expr(&mut self) -> PResult<Expr> {
        let start = self.start();
        let c = self.logic()?;
        if self.eat_op("?") {
            let a = self.expr()?;
            self.expect_op(":")?;
            let b = self.expr()?;
            return Ok(Expr {
                kind: ExprKind::Ternary(Box::new(c), Box::new(a), Box::new(b)),
                span: (start, self.prev_end()),
            });
        }
        Ok(c)
    }

    fn binary_level(
        &mut self,
        ops: &[&'static str],
        kws: &[&'static str],
        next: fn(&mut Self) -> PResult<Expr>,
    ) -> PResult<Expr> {
        let start = self.start();
        let mut l = next(self)?;
        loop {
            let op = match self.peek() {
                Tok::Op(o) if ops.contains(o) => *o,
                Tok::Ident(s) => match kws.iter().find(|k| **k == s.as_str()) {
                    Some(k) => *k,
                    None => break,
                },
                _ => break,
            };
            self.bump();
            let r = next(self)?;
            l = Expr {
                kind: ExprKind::Binary(op, Box::new(l), Box::new(r)),
                span: (start, self.prev_end()),
            };
        }
        Ok(l)
    }

    fn logic(&mut self) -> PResult<Expr> {
        self.binary_level(&["&&", "||", "^^"], &["and", "or", "xor"], Self::compare)
    }
    fn compare(&mut self) -> PResult<Expr> {
        self.binary_level(
            &["<", "<=", "==", "=", ":=", "!=", "<>", ">=", ">"],
            &[],
            Self::bitwise,
        )
    }
    fn bitwise(&mut self) -> PResult<Expr> {
        self.binary_level(&["|", "&", "^"], &[], Self::shift)
    }
    fn shift(&mut self) -> PResult<Expr> {
        self.binary_level(&["<<", ">>"], &[], Self::additive)
    }
    fn additive(&mut self) -> PResult<Expr> {
        self.binary_level(&["+", "-"], &[], Self::mult)
    }
    fn mult(&mut self) -> PResult<Expr> {
        self.binary_level(&["*", "/", "%"], &["div", "mod"], Self::unary)
    }

    fn unary(&mut self) -> PResult<Expr> {
        let start = self.start();
        let op: Option<&'static str> = match self.peek() {
            Tok::Op(o) if matches!(*o, "!" | "-" | "+" | "~" | "++" | "--") => Some(o),
            Tok::Ident(s) if s == "not" => Some("not"),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let e = self.unary()?;
            let kind = match op {
                "++" | "--" => {
                    if !is_lvalue(&e) {
                        return Err(ParseError {
                            offset: start,
                            message: format!("'{op}' applied to a non-variable"),
                        });
                    }
                    ExprKind::IncDec {
                        prefix: true,
                        inc: op == "++",
                        target: Box::new(e),
                    }
                }
                _ => ExprKind::Unary(op, Box::new(e)),
            };
            return Ok(Expr {
                kind,
                span: (start, self.prev_end()),
            });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut e = self.primary()?;
        loop {
            if self.is_op(".") {
                self.bump();
                let n = self.ident()?;
                e = Expr {
                    kind: ExprKind::Field(Box::new(e), n),
                    span: (start, self.prev_end()),
                };
            } else if self.is_op("[") {
                self.bump();
                let idx = self.args_until("]")?;
                e = Expr {
                    kind: ExprKind::Index(Box::new(e), idx),
                    span: (start, self.prev_end()),
                };
            } else if let Tok::Op(o @ ("[|" | "[?" | "[#" | "[@")) = self.peek() {
                let k = match *o {
                    "[|" => AccessKind::List,
                    "[?" => AccessKind::Map,
                    "[#" => AccessKind::Grid,
                    _ => AccessKind::Array,
                };
                self.bump();
                let idx = self.args_until("]")?;
                e = Expr {
                    kind: ExprKind::Access(k, Box::new(e), idx),
                    span: (start, self.prev_end()),
                };
            } else if (self.is_op("++") || self.is_op("--")) && is_lvalue(&e) {
                let inc = self.is_op("++");
                self.bump();
                e = Expr {
                    kind: ExprKind::IncDec {
                        prefix: false,
                        inc,
                        target: Box::new(e),
                    },
                    span: (start, self.prev_end()),
                };
                break;
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn args_until(&mut self, close: &str) -> PResult<Vec<Expr>> {
        let mut v = Vec::new();
        if self.eat_op(close) {
            return Ok(v);
        }
        loop {
            v.push(self.expr()?);
            if self.eat_op(close) {
                return Ok(v);
            }
            self.expect_op(",")?;
        }
    }

    fn primary(&mut self) -> PResult<Expr> {
        let start = self.start();
        let kind = match self.peek().clone() {
            Tok::Num(n) => {
                self.bump();
                ExprKind::Num(n)
            }
            Tok::Hex(h, x) => {
                self.bump();
                ExprKind::Hex(h, x)
            }
            Tok::Str(s) => {
                self.bump();
                ExprKind::Str(s)
            }
            Tok::Ident(s) if !is_keyword(&s) => {
                self.bump();
                if self.is_op("(") {
                    self.bump();
                    let args = self.args_until(")")?;
                    ExprKind::Call(s, args)
                } else {
                    ExprKind::Ident(s)
                }
            }
            Tok::Op("(") => {
                self.bump();
                let e = self.expr()?;
                self.expect_op(")")?;
                ExprKind::Paren(Box::new(e))
            }
            t => return self.err(format!("unexpected {t:?} in expression")),
        };
        Ok(Expr {
            kind,
            span: (start, self.prev_end()),
        })
    }
}

/// Whether an expression can be assigned to.
pub fn is_lvalue(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Ident(_) | ExprKind::Field(..) | ExprKind::Index(..) | ExprKind::Access(..) => {
            true
        }
        _ => false,
    }
}
