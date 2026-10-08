//! Tokenizer for GameMaker: Studio 1.x GML (a superset of GM 8.1 GML).
//!
//! Tokens keep their byte span in the source so that unchanged code can be copied
//! verbatim; whitespace and comments are skipped (they are recovered from the gaps
//! between spans when re-emitting).

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    /// Decimal number, as written.
    Num(String),
    /// Hexadecimal number: digits only (from `$FF` or `0xFF`), and whether it was `0x`.
    Hex(String, bool),
    /// String literal including its delimiters, as written.
    Str(String),
    /// Operator or punctuation, e.g. `+=`, `[|`, `?`.
    Op(&'static str),
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub start: usize,
    pub end: usize,
}

// Longest first.
const OPS: &[&str] = &[
    "<<=", ">>=", "[|", "[?", "[#", "[@", "==", "!=", "<>", "<=", ">=", "&&", "||", "^^", "<<",
    ">>", "+=", "-=", "*=", "/=", "%=", "|=", "&=", "^=", ":=", "++", "--", "{", "}", "(", ")",
    "[", "]", ",", ";", ":", ".", "=", "<", ">", "!", "&", "|", "^", "~", "+", "-", "*", "/",
    "%", "?",
];

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub offset: usize,
    pub message: String,
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    lex_with(src, false)
}

/// Tokenize; with `escapes`, a backslash inside a string escapes the next character
/// (GMS2 behaviour). GMS1 and GM 8.1 have no escapes.
pub fn lex_with(src: &str, escapes: bool) -> Result<Vec<Token>, LexError> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c <= b' ' {
            i += 1;
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                i += 1;
            }
            i = (i + 2).min(b.len());
            continue;
        }
        let start = i;
        if c.is_ascii_alphabetic() || c == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            out.push(Token {
                tok: Tok::Ident(src[start..i].to_string()),
                start,
                end: i,
            });
            continue;
        }
        if c == b'0' && matches!(b.get(i + 1), Some(b'x' | b'X'))
            && b.get(i + 2).is_some_and(|d| d.is_ascii_hexdigit())
        {
            i += 2;
            let ds = i;
            while i < b.len() && b[i].is_ascii_hexdigit() {
                i += 1;
            }
            out.push(Token {
                tok: Tok::Hex(src[ds..i].to_string(), true),
                start,
                end: i,
            });
            continue;
        }
        if c == b'$' {
            i += 1;
            let ds = i;
            while i < b.len() && b[i].is_ascii_hexdigit() {
                i += 1;
            }
            if ds == i {
                return Err(LexError {
                    offset: start,
                    message: "lone '$'".into(),
                });
            }
            out.push(Token {
                tok: Tok::Hex(src[ds..i].to_string(), false),
                start,
                end: i,
            });
            continue;
        }
        if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i < b.len() && b[i] == b'.' {
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            out.push(Token {
                tok: Tok::Num(src[start..i].to_string()),
                start,
                end: i,
            });
            continue;
        }
        if c == b'"' || c == b'\'' {
            i += 1;
            while i < b.len() && b[i] != c {
                if escapes && b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i >= b.len() {
                return Err(LexError {
                    offset: start,
                    message: "unterminated string".into(),
                });
            }
            i += 1;
            out.push(Token {
                tok: Tok::Str(src[start..i].to_string()),
                start,
                end: i,
            });
            continue;
        }
        let rest = &src[i..];
        if let Some(op) = OPS.iter().find(|o| rest.starts_with(**o)) {
            i += op.len();
            out.push(Token {
                tok: Tok::Op(op),
                start,
                end: i,
            });
            continue;
        }
        // Skip characters that are not valid GML (e.g. stray non-ASCII); report.
        let ch = rest.chars().next().unwrap();
        return Err(LexError {
            offset: start,
            message: format!("unexpected character {ch:?}"),
        });
    }
    out.push(Token {
        tok: Tok::Eof,
        start: b.len(),
        end: b.len(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        lex(s).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn basics() {
        assert_eq!(
            toks("a[| 0x1F]++ // hi\n"),
            vec![
                Tok::Ident("a".into()),
                Tok::Op("[|"),
                Tok::Hex("1F".into(), true),
                Tok::Op("]"),
                Tok::Op("++"),
                Tok::Eof
            ]
        );
        assert_eq!(toks("'it\\'")[0], Tok::Str("'it\\'".into()));
        assert_eq!(toks("1.5 .5")[1], Tok::Num(".5".into()));
    }
}
