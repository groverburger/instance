//! `external_define` / `external_call` in the browser: native DLLs cannot be
//! loaded there, so every definition fails with an error message (GML-only
//! extension packages keep working).

use gml::{Error, Value};
use std::path::PathBuf;

pub const DLL_STDCALL: i32 = 1;
pub const TY_STRING: i32 = 1;

#[derive(Default)]
pub struct Dlls {
    pub search: Vec<PathBuf>,
    pub cwd: String,
    pub verbose: bool,
}

pub fn find_helper() -> Option<PathBuf> {
    None
}

impl Dlls {
    pub fn define(&mut self, dll: &str, name: &str, _calltype: i32, _restype: i32, _types: &[i32]) -> Result<usize, String> {
        Err(format!("cannot load {} (function {}): DLLs are not supported in the browser", dll, name))
    }
    pub fn arity(&self, _id: usize) -> Option<usize> {
        None
    }
    pub fn call(&mut self, id: usize, _args: &[Value]) -> Result<Value, Error> {
        Err(Error::new(format!("Unknown external function id {}.", id)))
    }
    pub fn free(&mut self, _dll: &str) {}
    pub fn shutdown(&mut self) {}
}
