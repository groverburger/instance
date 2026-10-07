//! Reading and writing GameMaker project files and action libraries.
//!
//! Clean-room implementation of `specs/file-formats.md`.
//!
//! * [`read_project`] reads `.gmk` v701 (GameMaker 7, obfuscated), v800 (GameMaker 8.0)
//!   and `.gm81` v810 (GameMaker 8.1) files into a [`Project`].
//! * [`write_project`] writes v800 or v810 files.
//! * [`canonical_bytes`] gives a compression-independent view of a file for diffing.
//! * [`read_library`] / [`write_library`] handle `.lib` action libraries;
//!   [`builtin_libraries`] returns the bundled built-in action definitions.
//! * [`read_sprite_export`] / [`write_sprite_export`] handle `.gmspr` files.
//!
//! Strings are decoded as Windows-1252 (lossless; see [`text`]).

pub mod actionlib;
pub mod error;
pub mod exe;
pub mod export;
pub mod gmk;
pub mod image;
mod io;
pub mod model;
pub mod text;

pub use actionlib::{
    builtin_action, builtin_action_summary, builtin_libraries, read_library, read_library_file,
    write_library, ActionDef, ActionLibrary, ArgSlot,
};
pub use error::{Error, Result};
pub use export::{read_sprite_export, write_sprite_export};
pub use gmk::{
    canonical_bytes, detect_version, read_project, read_project_file, read_project_versioned,
    write_project, write_project_file, FormatVersion,
};
pub use model::*;
