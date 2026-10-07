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
