//! Typed errors for every reader and writer in this crate.
//!
//! Errors raised while parsing carry a `context` (which part of the file was being
//! read, e.g. `"file"`, `"sprites[3]"`) and a byte `offset`. For data at top level the
//! offset is an absolute file offset (for v701 files: the absolute offset in the
//! obfuscated file). Inside a zlib block the offset is relative to the start of the
//! *inflated* block, and the context names the block and its file offset.

use std::fmt;

/// Result alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Every error the crate can produce.
#[derive(Debug)]
pub enum Error {
    /// Underlying I/O error (only from the `*_file` convenience functions).
    Io(std::io::Error),
    /// A read ran past the end of the available data.
    UnexpectedEof {
        context: String,
        offset: usize,
        needed: usize,
        available: usize,
    },
    /// The file does not start with the GameMaker magic number `1234321`.
    BadMagic { found: i32 },
    /// A version field holds a value this crate does not understand.
    UnsupportedVersion {
        context: String,
        offset: usize,
        what: &'static str,
        found: i32,
    },
    /// A field that must hold a fixed value holds something else.
    UnexpectedValue {
        context: String,
        offset: usize,
        what: &'static str,
        expected: i64,
        found: i64,
    },
    /// A field holds a value that cannot be represented in the model
    /// (negative count, inconsistent blob size, bad image, ...).
    InvalidValue {
        context: String,
        offset: usize,
        message: String,
    },
    /// A zlib block failed to inflate.
    Zlib {
        context: String,
        offset: usize,
        message: String,
    },
    /// A self-delimiting structure (zlib block or whole file) was not consumed exactly.
    TrailingBytes {
        context: String,
        offset: usize,
        remaining: usize,
    },
    /// A model value cannot be encoded by a writer (e.g. image buffer of the wrong size).
    InvalidModel(String),
    /// The bundled built-in action table could not be decoded.
    BuiltinTable(String),
    /// A project folder (see [`crate::folder`]) is malformed: `path` names the file or
    /// directory, `message` what is wrong with it.
    Folder { path: String, message: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::UnexpectedEof {
                context,
                offset,
                needed,
                available,
            } => write!(
                f,
                "{context}: unexpected end of data at offset {offset:#x} (needed {needed} bytes, {available} available)"
            ),
            Error::BadMagic { found } => {
                write!(f, "not a GameMaker file (magic {found}, expected 1234321)")
            }
            Error::UnsupportedVersion {
                context,
                offset,
                what,
                found,
            } => write!(
                f,
                "{context}: unsupported {what} version {found} at offset {offset:#x}"
            ),
            Error::UnexpectedValue {
                context,
                offset,
                what,
                expected,
                found,
            } => write!(
                f,
                "{context}: {what}: expected {expected}, found {found} at offset {offset:#x}"
            ),
            Error::InvalidValue {
                context,
                offset,
                message,
            } => write!(f, "{context}: {message} at offset {offset:#x}"),
            Error::Zlib {
                context,
                offset,
                message,
            } => write!(f, "{context}: bad zlib block at offset {offset:#x}: {message}"),
            Error::TrailingBytes {
                context,
                offset,
                remaining,
            } => write!(
                f,
                "{context}: {remaining} unconsumed bytes at offset {offset:#x}"
            ),
            Error::InvalidModel(m) => write!(f, "cannot encode project: {m}"),
            Error::BuiltinTable(m) => write!(f, "built-in action table: {m}"),
            Error::Folder { path, message } => write!(f, "{path}: {message}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl Error {
    /// Byte offset associated with the error, if any.
    pub fn offset(&self) -> Option<usize> {
        match self {
            Error::UnexpectedEof { offset, .. }
            | Error::UnsupportedVersion { offset, .. }
            | Error::UnexpectedValue { offset, .. }
            | Error::InvalidValue { offset, .. }
            | Error::Zlib { offset, .. }
            | Error::TrailingBytes { offset, .. } => Some(*offset),
            Error::BadMagic { .. } => Some(0),
            _ => None,
        }
    }
}
