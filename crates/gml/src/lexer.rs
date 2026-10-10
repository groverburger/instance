//! Lexer: turns GML source bytes into tokens with positions (spec §2).

use crate::error::CompileError;

/// A source position: 1-based line and column (column counted in bytes), plus the
/// byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Span {
    /// Byte offset of the first character.
    pub offset: u32,
    /// 1-based line number.
    pub line: u32,
    /// 1-based column (byte position within the line).
    pub col: u32,
}

/// Token kinds. Keyword and punctuation variants are named after their source text
/// (see [`Tok::text`]).
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// Numeric literal.
    Real(f64),
    /// String literal (raw bytes, no escapes).
    Str(Vec<u8>),
    /// Identifier (non-keyword).
    Ident(String),
    // keywords
    If,
    Then,
    Else,
    Begin,
    End,
    Repeat,
    While,
    Do,
    Until,
    For,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Exit,
    Return,
    With,
    Var,
    GlobalVar,
    And,
    Or,
    Xor,
    Not,
    Div,
    Mod,
    // punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Colon,
    Dot,
    // assignment
    Assign,      // =
    ColonAssign, // :=
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    OrAssign,
    AndAssign,
    XorAssign,
    // comparison
    EqEq,
    NotEq, // != and <>
    Lt,
    Le,
    Gt,
    Ge,
    // logic
    AndAnd,
    OrOr,
    XorXor,
    Bang,
    // bitwise
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,
    // arithmetic
    Plus,
    Minus,
    Star,
    Slash,
    /// End of input.
    Eof,
}

impl Tok {
    /// Human-readable description used in error messages.
    pub fn describe(&self) -> String {
        match self {
            Tok::Real(v) => format!("number {}", v),
            Tok::Str(_) => "string".into(),
            Tok::Ident(s) => format!("identifier {}", s),
            Tok::Eof => "end of code".into(),
            other => format!("symbol {}", other.text()),
        }
    }

    /// Source text of keyword/punctuation tokens.
    pub fn text(&self) -> &'static str {
        use Tok::*;
        match self {
            If => "if",
            Then => "then",
            Else => "else",
            Begin => "begin",
            End => "end",
            Repeat => "repeat",
            While => "while",
            Do => "do",
            Until => "until",
            For => "for",
            Switch => "switch",
            Case => "case",
            Default => "default",
            Break => "break",
            Continue => "continue",
            Exit => "exit",
            Return => "return",
            With => "with",
            Var => "var",
            GlobalVar => "globalvar",
            And => "and",
            Or => "or",
            Xor => "xor",
            Not => "not",
            Div => "div",
            Mod => "mod",
            LBrace => "{",
            RBrace => "}",
            LParen => "(",
            RParen => ")",
            LBracket => "[",
            RBracket => "]",
            Comma => ",",
            Semi => ";",
            Colon => ":",
            Dot => ".",
            Assign => "=",
            ColonAssign => ":=",
            PlusAssign => "+=",
            MinusAssign => "-=",
            StarAssign => "*=",
            SlashAssign => "/=",
            OrAssign => "|=",
            AndAssign => "&=",
            XorAssign => "^=",
            EqEq => "==",
            NotEq => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            AndAnd => "&&",
            OrOr => "||",
            XorXor => "^^",
            Bang => "!",
            Amp => "&",
            Pipe => "|",
            Caret => "^",
            Tilde => "~",
            Shl => "<<",
            Shr => ">>",
            Plus => "+",
            Minus => "-",
            Star => "*",
            Slash => "/",
            Real(_) | Str(_) | Ident(_) | Eof => "",
        }
    }
}

/// Whether `name` is a GML keyword (`if`, `var`, `div`, ...) and so cannot name a
/// variable, script or resource usable from code.
pub fn is_keyword(name: &str) -> bool {
    keyword(name).is_some()
}

fn keyword(s: &str) -> Option<Tok> {
    use Tok::*;
    Some(match s {
        "if" => If,
        "then" => Then,
        "else" => Else,
        "begin" => Begin,
        "end" => End,
        "repeat" => Repeat,
        "while" => While,
        "do" => Do,
        "until" => Until,
        "for" => For,
        "switch" => Switch,
        "case" => Case,
        "default" => Default,
        "break" => Break,
        "continue" => Continue,
        "exit" => Exit,
        "return" => Return,
        "with" => With,
        "var" => Var,
        "globalvar" => GlobalVar,
        "and" => And,
        "or" => Or,
        "xor" => Xor,
        "not" => Not,
        "div" => Div,
        "mod" => Mod,
        _ => return None,
    })
}

/// A token with its starting position.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    /// The token.
    pub tok: Tok,
    /// Where it starts.
    pub span: Span,
}

/// Tokenises a whole source text.
pub fn tokenize(src: &[u8]) -> Result<Vec<Token>, CompileError> {
    let mut lx = Lexer {
        src,
        pos: 0,
        line: 1,
        line_start: 0,
    };
    let mut out = Vec::with_capacity(src.len() / 3 + 1);
    loop {
        let t = lx.next()?;
        let eof = t.tok == Tok::Eof;
        out.push(t);
        if eof {
            return Ok(out);
        }
    }
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
    line_start: usize,
}

impl<'a> Lexer<'a> {
    fn span_at(&self, pos: usize) -> Span {
        Span {
            offset: pos as u32,
            line: self.line,
            col: (pos - self.line_start + 1) as u32,
        }
    }

    fn peek(&self, k: usize) -> u8 {
        *self.src.get(self.pos + k).unwrap_or(&0)
    }

    fn newline(&mut self, at: usize) {
        self.line += 1;
        self.line_start = at + 1;
    }

    fn skip_trivia(&mut self) {
        loop {
            let c = self.peek(0);
            if self.pos >= self.src.len() {
                return;
            }
            if c <= 0x20 {
                if c == b'\n' {
                    self.newline(self.pos);
                }
                self.pos += 1;
            } else if c == b'/' && self.peek(1) == b'/' {
                while self.pos < self.src.len() && self.src[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else if c == b'/' && self.peek(1) == b'*' {
                self.pos += 2;
                loop {
                    if self.pos >= self.src.len() {
                        return; // unterminated: runs to end of input
                    }
                    if self.src[self.pos] == b'*' && self.peek(1) == b'/' {
                        self.pos += 2;
                        break;
                    }
                    if self.src[self.pos] == b'\n' {
                        self.newline(self.pos);
                    }
                    self.pos += 1;
                }
            } else {
                return;
            }
        }
    }

    fn next(&mut self) -> Result<Token, CompileError> {
        self.skip_trivia();
        let start = self.pos;
        let span = self.span_at(start);
        if start >= self.src.len() {
            return Ok(Token {
                tok: Tok::Eof,
                span,
            });
        }
        let c = self.src[start];
        let tok = if c.is_ascii_alphabetic() || c == b'_' {
            while self.pos < self.src.len()
                && (self.src[self.pos].is_ascii_alphanumeric() || self.src[self.pos] == b'_')
            {
                self.pos += 1;
            }
            let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
            keyword(s).unwrap_or_else(|| Tok::Ident(s.to_string()))
        } else if c.is_ascii_digit() || (c == b'.' && self.peek(1).is_ascii_digit()) {
            // VERIFIED black-box (bb_lang syn.double_dot, strfmt.1_23456_0_20,
            // div0.tiny): GM 8.1 accumulates the integer digits (v = v*10 + d), then
            // adds each fraction digit times p (p = 0.1, then p /= 10 per digit), in
            // double precision. Further dots after the first are skipped
            // (`1.2.3` = 1.23).
            let mut v = 0.0f64;
            while self.peek(0).is_ascii_digit() {
                v = v * 10.0 + (self.peek(0) - b'0') as f64;
                self.pos += 1;
            }
            if self.peek(0) == b'.' {
                self.pos += 1;
                let mut p = 0.1f64;
                loop {
                    let d = self.peek(0);
                    if d.is_ascii_digit() {
                        v += (d - b'0') as f64 * p;
                        p /= 10.0;
                    } else if d != b'.' {
                        break;
                    }
                    self.pos += 1;
                }
            }
            Tok::Real(v)
        } else if c == b'$' {
            self.pos += 1;
            let ds = self.pos;
            while self.peek(0).is_ascii_hexdigit() {
                self.pos += 1;
            }
            let digits = &self.src[ds..self.pos];
            if digits.is_empty() {
                return Err(CompileError::new(
                    "Hexadecimal digits expected after $.",
                    span,
                ));
            }
            if digits.len() > 16 {
                return Err(CompileError::new("Hexadecimal number too long.", span));
            }
            let v = u64::from_str_radix(std::str::from_utf8(digits).unwrap(), 16).unwrap();
            Tok::Real(v as f64)
        } else if c == b'"' || c == b'\'' {
            self.pos += 1;
            let ds = self.pos;
            loop {
                if self.pos >= self.src.len() {
                    return Err(CompileError::new("Unterminated string.", span));
                }
                let d = self.src[self.pos];
                if d == c {
                    break;
                }
                if d == b'\n' {
                    self.newline(self.pos);
                }
                self.pos += 1;
            }
            let s = self.src[ds..self.pos].to_vec();
            self.pos += 1;
            Tok::Str(s)
        } else {
            let (t, n) = self.punct(c);
            if n == 0 {
                return Err(CompileError::new("Unexpected symbol.", span));
            }
            self.pos += n;
            t
        };
        Ok(Token { tok, span })
    }

    fn punct(&self, c: u8) -> (Tok, usize) {
        use Tok::*;
        let n = self.peek(1);
        match (c, n) {
            (b':', b'=') => (ColonAssign, 2),
            (b'+', b'=') => (PlusAssign, 2),
            (b'-', b'=') => (MinusAssign, 2),
            (b'*', b'=') => (StarAssign, 2),
            (b'/', b'=') => (SlashAssign, 2),
            (b'|', b'=') => (OrAssign, 2),
            (b'&', b'=') => (AndAssign, 2),
            (b'^', b'=') => (XorAssign, 2),
            (b'=', b'=') => (EqEq, 2),
            (b'!', b'=') => (NotEq, 2),
            (b'<', b'>') => (NotEq, 2),
            (b'<', b'=') => (Le, 2),
            (b'>', b'=') => (Ge, 2),
            (b'<', b'<') => (Shl, 2),
            (b'>', b'>') => (Shr, 2),
            (b'&', b'&') => (AndAnd, 2),
            (b'|', b'|') => (OrOr, 2),
            (b'^', b'^') => (XorXor, 2),
            (b'{', _) => (LBrace, 1),
            (b'}', _) => (RBrace, 1),
            (b'(', _) => (LParen, 1),
            (b')', _) => (RParen, 1),
            (b'[', _) => (LBracket, 1),
            (b']', _) => (RBracket, 1),
            (b',', _) => (Comma, 1),
            (b';', _) => (Semi, 1),
            (b':', _) => (Colon, 1),
            (b'.', _) => (Dot, 1),
            (b'=', _) => (Assign, 1),
            (b'<', _) => (Lt, 1),
            (b'>', _) => (Gt, 1),
            (b'!', _) => (Bang, 1),
            (b'&', _) => (Amp, 1),
            (b'|', _) => (Pipe, 1),
            (b'^', _) => (Caret, 1),
            (b'~', _) => (Tilde, 1),
            (b'+', _) => (Plus, 1),
            (b'-', _) => (Minus, 1),
            (b'*', _) => (Star, 1),
            (b'/', _) => (Slash, 1),
            _ => (Eof, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        tokenize(s.as_bytes())
            .unwrap()
            .into_iter()
            .map(|t| t.tok)
            .collect()
    }

    #[test]
    fn numbers() {
        assert_eq!(
            toks("12 12.5 12. .5 $00FFAA $FFAA00"),
            vec![
                Tok::Real(12.0),
                Tok::Real(12.5),
                Tok::Real(12.0),
                Tok::Real(0.5),
                Tok::Real(65450.0),
                Tok::Real(16755200.0),
                Tok::Eof
            ]
        );
        assert_eq!(
            toks("1e5"),
            vec![Tok::Real(1.0), Tok::Ident("e5".into()), Tok::Eof]
        );
        assert_eq!(
            toks("12.x"),
            vec![Tok::Real(12.0), Tok::Ident("x".into()), Tok::Eof]
        );
        assert!(tokenize(b"$").is_err());
        assert!(tokenize(b"$12345678901234567").is_err());
    }

    #[test]
    fn strings_and_comments() {
        assert_eq!(
            toks("'a\"b' \"c'd\" \"x\\y\""),
            vec![
                Tok::Str(b"a\"b".to_vec()),
                Tok::Str(b"c'd".to_vec()),
                Tok::Str(b"x\\y".to_vec()),
                Tok::Eof
            ]
        );
        assert_eq!(
            toks("a // c\n b /* x \n y */ c /* open"),
            vec![
                Tok::Ident("a".into()),
                Tok::Ident("b".into()),
                Tok::Ident("c".into()),
                Tok::Eof
            ]
        );
        assert_eq!(
            toks("\"// not\""),
            vec![Tok::Str(b"// not".to_vec()), Tok::Eof]
        );
        let e = tokenize(b"a = \"abc").unwrap_err();
        assert_eq!((e.line, e.col), (1, 5));
    }

    #[test]
    fn operators() {
        use Tok::*;
        assert_eq!(
            toks("<> != := ++ -- <= << >= >> ^^ ^= && &= || |="),
            vec![
                NotEq,
                NotEq,
                ColonAssign,
                Plus,
                Plus,
                Minus,
                Minus,
                Le,
                Shl,
                Ge,
                Shr,
                XorXor,
                XorAssign,
                AndAnd,
                AndAssign,
                OrOr,
                OrAssign,
                Eof
            ]
        );
    }

    #[test]
    fn positions() {
        let t = tokenize(b"a\n  bb\r\n\tc").unwrap();
        assert_eq!((t[1].span.line, t[1].span.col), (2, 3));
        assert_eq!((t[2].span.line, t[2].span.col), (3, 2));
        assert!(tokenize("é".as_bytes()).is_err());
    }
}
