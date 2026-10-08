//! Compile-time and run-time error types.

use crate::lexer::Span;
use std::fmt;
use std::rc::Rc;

/// A syntax or name-resolution error found while compiling GML.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileError {
    /// Message, e.g. `"Unexpected symbol."`.
    pub message: String,
    /// 1-based line.
    pub line: u32,
    /// 1-based column ("position" in GM's wording).
    pub col: u32,
}

impl CompileError {
    /// Creates an error at a span.
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        CompileError {
            message: message.into(),
            line: span.line,
            col: span.col,
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error in code at line {}: at position {}: {}",
            self.line, self.col, self.message
        )
    }
}

impl std::error::Error for CompileError {}

/// Classification of run-time errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// Ordinary, ignorable language/library error (spec §13). The current program
    /// is abandoned.
    Runtime,
    /// An error that must abort the game.
    Fatal,
    /// Not an error: the host asked the interpreter to unwind all GML currently
    /// executing (e.g. to end the game immediately). Propagated unchanged.
    Halt,
    /// A crash-like failure that abandons the rest of the current event without a
    /// message (GM 8.1: reading `argument[16]`, VERIFIED black-box bb_lang
    /// args.arr_index_16). Script argument arrays are not restored on the way out.
    AbortEvent,
}

/// Where a run-time error happened.
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    /// Name given to the program when it was compiled (script name, etc.).
    pub program: Rc<str>,
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub col: u32,
    /// The text of the source line (without line break), if known.
    pub source_line: Option<String>,
}

/// GM 8.1's text for an error in code (VERIFIED black-box, bb_lang *.msg), with
/// `\n` between its lines (GM 8.1 itself uses `\r`):
/// `Error in code at line 1:\n   <line>\n<P+1 spaces>^\nat position P: <msg>`.
pub fn code_error_text(line: u32, col: u32, source_line: &str, msg: &str) -> String {
    format!(
        "Error in code at line {}:\n   {}\n{}^\nat position {}: {}",
        line,
        source_line,
        " ".repeat(col as usize + 1),
        col,
        msg
    )
}

/// The 1-based line `line` of `src` (without its line break).
pub fn source_line(src: &[u8], line: u32) -> String {
    let l = src
        .split(|&b| b == b'\n')
        .nth(line.saturating_sub(1) as usize)
        .unwrap_or(b"");
    let l = l.strip_suffix(b"\r").unwrap_or(l);
    l.iter().map(|&b| b as char).collect()
}

/// A run-time error. Host callbacks return this; the interpreter fills in the
/// location of the innermost failing expression if the host did not.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    /// Kind of error.
    pub kind: ErrorKind,
    /// Message text (e.g. `"Unknown variable foo"`).
    pub message: String,
    /// Source location, once known.
    pub location: Option<Location>,
}

impl Error {
    /// An ordinary run-time error.
    pub fn new(message: impl Into<String>) -> Self {
        Error {
            kind: ErrorKind::Runtime,
            message: message.into(),
            location: None,
        }
    }
    /// A fatal error.
    pub fn fatal(message: impl Into<String>) -> Self {
        Error {
            kind: ErrorKind::Fatal,
            message: message.into(),
            location: None,
        }
    }
    /// An event abort (see [`ErrorKind::AbortEvent`]).
    pub fn abort_event() -> Self {
        Error {
            kind: ErrorKind::AbortEvent,
            message: String::new(),
            location: None,
        }
    }
    /// A halt request (not an error; unwinds the interpreter).
    pub fn halt() -> Self {
        Error {
            kind: ErrorKind::Halt,
            message: String::new(),
            location: None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.location {
            Some(l) => write!(
                f,
                "{} (in {} at line {}, position {})",
                self.message, l.program, l.line, l.col
            ),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for Error {}

impl From<CompileError> for Error {
    fn from(e: CompileError) -> Self {
        Error::new(e.to_string())
    }
}
