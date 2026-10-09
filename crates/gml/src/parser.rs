//! Recursive-descent parser (spec §3, §4.1).

use crate::ast::*;
use crate::error::CompileError;
use crate::lexer::{tokenize, Span, Tok, Token};
use crate::value::{BinOp, UnOp};

const MAX_DEPTH: u32 = 160;

/// Parses a whole program (a statement list) from source bytes.
pub fn parse_program(src: &[u8]) -> Result<Vec<Stmt>, CompileError> {
    let mut p = Parser::new(src)?;
    let mut out = Vec::new();
    // Spec §3 note 1: code whose first token is `{` is the program's single outer
    // block; any token after its matching `}` is an error (VERIFIED black-box).
    let outer_block = p.peek() == &Tok::LBrace;
    while p.peek() != &Tok::Eof {
        out.push(p.statement()?);
        if outer_block && p.peek() != &Tok::Eof {
            return Err(CompileError::new(
                "Program ends before end of the code.",
                p.span(),
            ));
        }
    }
    Ok(out)
}

/// Parses a single expression (action argument fields, user constants). Trailing
/// tokens are an error.
pub fn parse_expression(src: &[u8]) -> Result<Expr, CompileError> {
    let mut p = Parser::new(src)?;
    let e = p.expr()?;
    if p.peek() != &Tok::Eof {
        return Err(p.unexpected());
    }
    Ok(e)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    depth: u32,
}

impl Parser {
    fn new(src: &[u8]) -> Result<Self, CompileError> {
        Ok(Parser {
            toks: tokenize(src)?,
            pos: 0,
            depth: 0,
        })
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }
    fn peek_at(&self, k: usize) -> &Tok {
        &self.toks[(self.pos + k).min(self.toks.len() - 1)].tok
    }
    fn span(&self) -> Span {
        self.toks[self.pos].span
    }
    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == t {
            self.bump();
            true
        } else {
            false
        }
    }
    fn unexpected(&self) -> CompileError {
        let t = &self.toks[self.pos];
        let msg = match &t.tok {
            Tok::Eof => "Unexpected end of code.".to_string(),
            other => format!("Unexpected {}.", other.describe()),
        };
        CompileError::new(msg, t.span)
    }
    fn expect(&mut self, t: Tok) -> Result<(), CompileError> {
        if self.eat(&t) {
            Ok(())
        } else {
            Err(CompileError::new(
                format!("Symbol {} expected.", t.text()),
                self.span(),
            ))
        }
    }
    fn ident(&mut self) -> Result<(String, Span), CompileError> {
        let span = self.span();
        match self.peek().clone() {
            Tok::Ident(s) => {
                self.bump();
                Ok((s, span))
            }
            _ => Err(CompileError::new("Variable name expected.", span)),
        }
    }
    fn enter(&mut self) -> Result<(), CompileError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(CompileError::new("Code is nested too deeply.", self.span()));
        }
        Ok(())
    }

    // ---------------------------------------------------------------- statements

    fn statement(&mut self) -> Result<Stmt, CompileError> {
        self.enter()?;
        let r = self.statement_inner();
        self.depth -= 1;
        r
    }

    fn statement_inner(&mut self) -> Result<Stmt, CompileError> {
        let span = self.span();
        let kind = match self.peek().clone() {
            Tok::Semi => {
                self.bump();
                StmtKind::Empty
            }
            Tok::LBrace | Tok::Begin => StmtKind::Block(self.block()?),
            Tok::Var => {
                self.bump();
                StmtKind::Var(self.name_list()?)
            }
            Tok::GlobalVar => {
                self.bump();
                StmtKind::GlobalVar(self.name_list()?)
            }
            Tok::If => {
                self.bump();
                let c = self.expr()?;
                self.eat(&Tok::Then);
                let t = self.statement()?;
                if self.peek() == &Tok::Semi && self.peek_at(1) == &Tok::Else {
                    self.bump();
                }
                let e = if self.eat(&Tok::Else) {
                    Some(Box::new(self.statement()?))
                } else {
                    None
                };
                StmtKind::If(c, Box::new(t), e)
            }
            Tok::Repeat => {
                self.bump();
                let c = self.expr()?;
                StmtKind::Repeat(c, Box::new(self.statement()?))
            }
            Tok::While => {
                self.bump();
                let c = self.expr()?;
                StmtKind::While(c, Box::new(self.statement()?))
            }
            Tok::With => {
                self.bump();
                let c = self.expr()?;
                StmtKind::With(c, Box::new(self.statement()?))
            }
            Tok::Do => {
                self.bump();
                let body = self.statement()?;
                if self.peek() == &Tok::Semi && self.peek_at(1) == &Tok::Until {
                    self.bump();
                }
                self.expect(Tok::Until)?;
                StmtKind::DoUntil(Box::new(body), self.expr()?)
            }
            Tok::For => {
                self.bump();
                self.expect(Tok::LParen)?;
                let init = self.simple_statement()?;
                self.eat(&Tok::Semi);
                let cond = self.expr()?;
                self.eat(&Tok::Semi);
                let step = self.simple_statement()?;
                self.expect(Tok::RParen)?;
                let body = self.statement()?;
                StmtKind::For(Box::new(init), cond, Box::new(step), Box::new(body))
            }
            Tok::Switch => {
                self.bump();
                let v = self.expr()?;
                let close = match self.peek() {
                    Tok::LBrace => Tok::RBrace,
                    Tok::Begin => Tok::End,
                    _ => return Err(CompileError::new("Symbol { expected.", self.span())),
                };
                self.bump();
                let mut items = Vec::new();
                loop {
                    let t = self.peek().clone();
                    if t == Tok::RBrace || t == Tok::End {
                        if t != close {
                            return Err(self.unexpected());
                        }
                        self.bump();
                        break;
                    }
                    match t {
                        Tok::Eof => {
                            return Err(CompileError::new(
                                format!("Symbol {} expected.", close.text()),
                                self.span(),
                            ))
                        }
                        Tok::Case => {
                            self.bump();
                            let e = self.expr()?;
                            self.expect(Tok::Colon)?;
                            items.push(SwitchItem::Case(e));
                        }
                        Tok::Default => {
                            let s = self.span();
                            self.bump();
                            self.expect(Tok::Colon)?;
                            items.push(SwitchItem::Default(s));
                        }
                        _ => items.push(SwitchItem::Stmt(self.statement()?)),
                    }
                }
                StmtKind::Switch(v, items)
            }
            Tok::Break => {
                self.bump();
                StmtKind::Break
            }
            Tok::Continue => {
                self.bump();
                StmtKind::Continue
            }
            Tok::Exit => {
                self.bump();
                StmtKind::Exit
            }
            Tok::Return => {
                self.bump();
                match self.peek() {
                    Tok::Semi | Tok::RBrace | Tok::End | Tok::Eof => {
                        return Err(CompileError::new(
                            "Unexpected symbol in expression.",
                            self.span(),
                        ))
                    }
                    _ => StmtKind::Return(self.expr()?),
                }
            }
            Tok::Ident(_) | Tok::LParen => return self.assign_or_call(),
            Tok::Case | Tok::Default => {
                return Err(CompileError::new(
                    "case or default outside of a switch statement.",
                    span,
                ))
            }
            _ => return Err(self.unexpected()),
        };
        Ok(Stmt { kind, span })
    }

    fn simple_statement(&mut self) -> Result<Stmt, CompileError> {
        let span = self.span();
        match self.peek() {
            Tok::LBrace | Tok::Begin => Ok(Stmt {
                kind: StmtKind::Block(self.block()?),
                span,
            }),
            Tok::Var => {
                self.bump();
                Ok(Stmt {
                    kind: StmtKind::Var(self.name_list()?),
                    span,
                })
            }
            Tok::Ident(_) | Tok::LParen => self.assign_or_call(),
            _ => Err(CompileError::new(
                "Assignment or function call expected.",
                span,
            )),
        }
    }

    fn block(&mut self) -> Result<Vec<Stmt>, CompileError> {
        let close = if self.bump().tok == Tok::LBrace {
            Tok::RBrace
        } else {
            Tok::End
        };
        let mut out = Vec::new();
        loop {
            let t = self.peek();
            if *t == Tok::RBrace || *t == Tok::End {
                if *t != close {
                    return Err(self.unexpected());
                }
                self.bump();
                return Ok(out);
            }
            if *t == Tok::Eof {
                return Err(CompileError::new(
                    format!("Symbol {} expected.", close.text()),
                    self.span(),
                ));
            }
            out.push(self.statement()?);
        }
    }

    fn name_list(&mut self) -> Result<Vec<(String, Span)>, CompileError> {
        let mut v = vec![self.ident()?];
        while self.eat(&Tok::Comma) {
            v.push(self.ident()?);
        }
        Ok(v)
    }

    fn assign_or_call(&mut self) -> Result<Stmt, CompileError> {
        let span = self.span();
        let target = self.postfix()?;
        let op = match self.peek() {
            Tok::Assign | Tok::ColonAssign => AssignOp::Set,
            Tok::PlusAssign => AssignOp::Compound(BinOp::Add),
            Tok::MinusAssign => AssignOp::Compound(BinOp::Sub),
            Tok::StarAssign => AssignOp::Compound(BinOp::Mul),
            Tok::SlashAssign => AssignOp::Compound(BinOp::Div),
            Tok::OrAssign => AssignOp::Compound(BinOp::BitOr),
            Tok::AndAssign => AssignOp::Compound(BinOp::BitAnd),
            Tok::XorAssign => AssignOp::Compound(BinOp::BitXor),
            _ => {
                if matches!(target.kind, ExprKind::Call(..)) {
                    return Ok(Stmt {
                        kind: StmtKind::Call(target),
                        span,
                    });
                }
                return Err(CompileError::new(
                    "Assignment operator expected.",
                    self.span(),
                ));
            }
        };
        match target.kind {
            ExprKind::Ident(_) | ExprKind::Field(..) | ExprKind::Index(..) => {}
            ExprKind::Call(..) => {
                return Err(CompileError::new(
                    "Cannot assign to a function result.",
                    target.span,
                ))
            }
            _ => {
                return Err(CompileError::new(
                    "Cannot assign to this expression.",
                    target.span,
                ))
            }
        }
        self.bump();
        let value = self.expr()?;
        Ok(Stmt {
            kind: StmtKind::Assign(target, op, value),
            span,
        })
    }

    // ---------------------------------------------------------------- expressions

    /// `expr = logic`
    fn expr(&mut self) -> Result<Expr, CompileError> {
        self.enter()?;
        let r = self.binary_level(0);
        self.depth -= 1;
        r
    }

    fn level_op(level: u8, t: &Tok) -> Option<BinOp> {
        use Tok::*;
        Some(match (level, t) {
            (0, AndAnd | And) => BinOp::And,
            (0, OrOr | Or) => BinOp::Or,
            (0, XorXor | Xor) => BinOp::Xor,
            (1, Lt) => BinOp::Lt,
            (1, Le) => BinOp::Le,
            (1, EqEq | Assign | ColonAssign) => BinOp::Eq,
            (1, NotEq) => BinOp::Ne,
            (1, Ge) => BinOp::Ge,
            (1, Gt) => BinOp::Gt,
            (2, Pipe) => BinOp::BitOr,
            (2, Amp) => BinOp::BitAnd,
            (2, Caret) => BinOp::BitXor,
            (3, Shl) => BinOp::Shl,
            (3, Shr) => BinOp::Shr,
            (4, Plus) => BinOp::Add,
            (4, Minus) => BinOp::Sub,
            (5, Star) => BinOp::Mul,
            (5, Slash) => BinOp::Div,
            (5, Div) => BinOp::IntDiv,
            (5, Mod) => BinOp::Mod,
            _ => return None,
        })
    }

    /// Precedence climbing over the levels of [`Self::level_op`] (0 = lowest).
    fn binary_level(&mut self, min_level: u8) -> Result<Expr, CompileError> {
        let mut lhs = self.unary()?;
        loop {
            let Some((op, level)) =
                (min_level..6).find_map(|l| Self::level_op(l, self.peek()).map(|op| (op, l)))
            else {
                return Ok(lhs);
            };
            let span = self.bump().span;
            self.enter()?;
            let rhs = self.binary_level(level + 1);
            self.depth -= 1;
            lhs = Expr {
                kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs?)),
                span,
            };
        }
    }

    fn unary(&mut self) -> Result<Expr, CompileError> {
        let span = self.span();
        let op = match self.peek() {
            Tok::Bang | Tok::Not => UnOp::Not,
            Tok::Minus => UnOp::Neg,
            Tok::Plus => UnOp::Plus,
            Tok::Tilde => UnOp::BitNot,
            _ => return self.postfix(),
        };
        self.bump();
        self.enter()?;
        let inner = self.unary();
        self.depth -= 1;
        Ok(Expr {
            kind: ExprKind::Unary(op, Box::new(inner?)),
            span,
        })
    }

    fn postfix(&mut self) -> Result<Expr, CompileError> {
        let span = self.span();
        let mut e = match self.peek().clone() {
            Tok::Real(v) => {
                self.bump();
                Expr {
                    kind: ExprKind::Real(v),
                    span,
                }
            }
            Tok::Str(s) => {
                self.bump();
                Expr {
                    kind: ExprKind::Str(s),
                    span,
                }
            }
            Tok::Ident(name) => {
                self.bump();
                if self.peek() == &Tok::LParen {
                    self.bump();
                    let mut args = Vec::new();
                    if !self.eat(&Tok::RParen) {
                        loop {
                            args.push(self.expr()?);
                            if self.eat(&Tok::RParen) {
                                break;
                            }
                            if !self.eat(&Tok::Comma) {
                                return Err(CompileError::new(
                                    "Symbol , or ) expected.",
                                    self.span(),
                                ));
                            }
                        }
                    }
                    let call = Expr {
                        kind: ExprKind::Call(name, args),
                        span,
                    };
                    if self.peek() == &Tok::LBracket {
                        return Err(CompileError::new(
                            "A function result cannot be indexed.",
                            self.span(),
                        ));
                    }
                    // `f(...).var` is accepted (spec §3 note 11, verified black-box)
                    if self.peek() != &Tok::Dot {
                        return Ok(call);
                    }
                    call
                } else {
                    Expr {
                        kind: ExprKind::Ident(name),
                        span,
                    }
                }
            }
            Tok::LParen => {
                self.bump();
                let inner = self.expr()?;
                self.expect(Tok::RParen)?;
                Expr {
                    kind: ExprKind::Paren(Box::new(inner)),
                    span,
                }
            }
            _ => return Err(self.unexpected()),
        };
        loop {
            match self.peek() {
                Tok::Dot => {
                    if matches!(e.kind, ExprKind::Real(_) | ExprKind::Str(_)) {
                        return Err(self.unexpected());
                    }
                    self.bump();
                    let (name, nspan) = self.ident()?;
                    if self.peek() == &Tok::LParen {
                        return Err(CompileError::new(
                            "Unexpected symbol (: a variable name is required after a dot.",
                            self.span(),
                        ));
                    }
                    e = Expr {
                        kind: ExprKind::Field(Box::new(e), name, nspan),
                        span: nspan,
                    };
                }
                Tok::LBracket => {
                    if !matches!(e.kind, ExprKind::Ident(_) | ExprKind::Field(..)) {
                        return Err(CompileError::new(
                            "Only a variable can be indexed.",
                            self.span(),
                        ));
                    }
                    let bspan = self.bump().span;
                    let mut idx = vec![self.expr()?];
                    if self.eat(&Tok::Comma) {
                        idx.push(self.expr()?);
                    }
                    self.expect(Tok::RBracket)?;
                    e = Expr {
                        kind: ExprKind::Index(Box::new(e), idx),
                        span: bspan,
                    };
                }
                _ => return Ok(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Vec<Stmt> {
        parse_program(s.as_bytes()).unwrap_or_else(|e| panic!("{}: {}", s, e))
    }
    fn perr(s: &str) -> CompileError {
        parse_program(s.as_bytes()).expect_err(s)
    }

    /// Renders an expression in fully parenthesised prefix form for precedence tests.
    fn show(e: &Expr) -> String {
        match &e.kind {
            ExprKind::Real(v) => format!("{}", v),
            ExprKind::Str(s) => format!("{:?}", String::from_utf8_lossy(s)),
            ExprKind::Ident(s) => s.clone(),
            ExprKind::Field(o, n, _) => format!("{}.{}", show(o), n),
            ExprKind::Index(t, i) => format!(
                "{}[{}]",
                show(t),
                i.iter().map(show).collect::<Vec<_>>().join(",")
            ),
            ExprKind::Call(n, a) => format!(
                "{}({})",
                n,
                a.iter().map(show).collect::<Vec<_>>().join(",")
            ),
            ExprKind::Unary(op, x) => format!("({}{})", op.symbol(), show(x)),
            ExprKind::Binary(op, a, b) => format!("({} {} {})", show(a), op.symbol(), show(b)),
            ExprKind::Paren(x) => show(x),
        }
    }
    fn pe(s: &str) -> String {
        show(&parse_expression(s.as_bytes()).unwrap())
    }

    #[test]
    fn outer_block_must_be_whole_program() {
        // VERIFIED black-box (spec §3 note 1): reported at the first token after the block.
        let e = perr("{ var t; t = 1 }\r\nx = 2");
        assert_eq!(e.message, "Program ends before end of the code.");
        assert_eq!((e.line, e.col), (2, 1));
        let e = perr("{ a = 1 } ;");
        assert_eq!(e.message, "Program ends before end of the code.");
        assert_eq!((e.line, e.col), (1, 11));
        // A block that is not the first token, or a lone outer block, is fine.
        p("{ a = 1 }");
        p("{ a = 1 }\r\n// trailing comment\r\n");
        p("a = 0 { a = 1 } b = 2");
        p("; { a = 1 } b = 2");
        p("begin a = 1 end b = 2");
        // Errors inside the block still win.
        assert_ne!(perr("{ a = } b = 1").message, "Program ends before end of the code.");
    }

    #[test]
    fn precedence() {
        assert_eq!(pe("a & b == c"), "((a & b) == c)");
        assert_eq!(pe("a || b && c"), "((a || b) && c)");
        assert_eq!(pe("x == 1 or x = 2"), "((x == 1) || (x == 2))");
        assert_eq!(pe("a < b < c"), "((a < b) < c)");
        assert_eq!(pe("1 + 2 * 3 - 4"), "((1 + (2 * 3)) - 4)");
        assert_eq!(pe("1 << 2 + 3"), "(1 << (2 + 3))");
        assert_eq!(pe("a | b << 1"), "(a | (b << 1))");
        assert_eq!(pe("-a.x"), "(-a.x)");
        assert_eq!(pe("- - a"), "(-(-a))");
        assert_eq!(pe("!-a"), "(!(-a))");
        assert_eq!(pe("a mod b div c mod d"), "(((a mod b) div c) mod d)");
        // `%` is not an operator in GM 8.1 (VERIFIED black-box, bb_lang syn.percent)
        assert!(parse_expression(b"7 % 3").is_err());
        assert_eq!(pe("not a and b"), "((!a) && b)");
        assert_eq!(pe("a := b"), "(a == b)");
        assert_eq!(pe("--b"), "(-(-b))");
        assert_eq!(pe("obj[1].id.x"), "obj[1].id.x");
        assert_eq!(pe("(100032).speed"), "100032.speed");
        assert_eq!(pe("b[4, 6] + other.alarm[0]"), "(b[4,6] + other.alarm[0])");
        assert_eq!(pe("f(1, g(2), 3)"), "f(1,g(2),3)");
    }

    #[test]
    fn statements() {
        p("a = 1 b = 2 c = a + b");
        p("{ a = 1; ; ; }");
        p("begin a = 1 end");
        p("if a b = 1; else b = 2");
        p("if (x > 3) then y = 1 else { y = 2 }");
        p("if variable_global_exists('a') global.a += 1 else global.a = 0");
        p("repeat 5 x += 1");
        p("while x < 10 x += 1");
        p("do x += 1 until x > 10");
        p("do { x += 1 } until (x > 10)");
        p("for (i = 0; i < 10; i += 1) a[i] = i");
        p("for (i = 0 i < 10 i += 1) {}");
        p("for (var i; i < 1; {i += 1}) {}");
        p("switch (x) { case 1: case 2: y = 1; break; default: y = 0 }");
        p("switch x begin case 'a': y = 1 end");
        p("with (obj) { x = 1; continue; }");
        p("var a, b, c; globalvar g1, g2;");
        p("return a + 1");
        p("exit; break; continue");
        p("instance_destroy()");
        p("(100032).speed = 0");
        p("obj[0].alarm[4] = 12");
        p("obj[1].id.x = 12");
        p("a = b = c");
        p("x := 5");
        let s = p("if a if b c = 1 else c = 2");
        // dangling else binds to the inner if
        match &s[0].kind {
            StmtKind::If(_, inner, None) => {
                assert!(matches!(inner.kind, StmtKind::If(_, _, Some(_))))
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn errors() {
        let e = perr("a = 1\n5;");
        assert_eq!((e.line, e.col), (2, 1));
        perr("a == b;");
        perr("var a = 1;");
        perr("i++");
        perr("return;");
        perr("return }");
        perr("case 1: a = 1");
        perr("f(x).y");
        perr("a = f(x)[0]");
        perr("a = 1e5");
        perr("a = (b)[0]");
        perr("a[0][1] = 2");
        perr("{ a = 1");
        perr("{ a = 1 end");
        perr("x = 12.x");
        perr("f(1,2");
        perr("a.b(1)");
        perr("5 = 3");
        let deep = "a = ".to_string() + &"(".repeat(2000) + "1" + &")".repeat(2000);
        perr(&deep);
    }
}
