//! GML (GameMaker Language, GameMaker 8.1 dialect) compiler and interpreter.
//!
//! Implements `specs/gml-language.md` and the pure parts of `specs/builtins.json`.
//!
//! # Pipeline
//! source bytes → [`lexer`] → [`parser`] ([`ast`]) → compiler (resolves every name:
//! `var` locals to slots, built-ins to ids, constants and resource names to values)
//! → [`Program`] → executed by the [`Vm`].
//!
//! # Embedding
//! The game world stays outside this crate. A runner implements [`Host`] to provide
//! instances and their variables, built-in variables, world functions and scripts:
//!
//! ```
//! use gml::{Vm, Host, Ctx, Value, VarTable, Error, FunctionId, VariableId};
//!
//! struct World { me: VarTable }
//! impl Host for World {
//!     fn instance_vars(&mut self, id: i32) -> Option<&mut VarTable> {
//!         (id == 100000).then_some(&mut self.me)
//!     }
//!     fn instance_exists(&mut self, id: i32) -> bool { id == 100000 }
//!     fn select_instances(&mut self, target: i32, out: &mut Vec<i32>) {
//!         if target == gml::special::ALL || target == 100000 { out.push(100000) }
//!     }
//!     fn get_builtin(&mut self, _: &Ctx, _: i32, v: VariableId, _: i32) -> Result<Value, Error> {
//!         Err(Error::new(format!("{} not supported", v.name())))
//!     }
//!     fn set_builtin(&mut self, _: &Ctx, _: i32, v: VariableId, _: i32, _: Value) -> Result<(), Error> {
//!         Err(Error::new(format!("{} not supported", v.name())))
//!     }
//!     fn call_function(&mut self, _: &mut Vm, _: &Ctx, f: FunctionId, _: &[Value]) -> Result<Value, Error> {
//!         Err(Error::new(format!("{} not supported", f.name())))
//!     }
//!     fn call_script(&mut self, _: &mut Vm, _: &Ctx, _: i32, _: &[Value]) -> Result<Value, Error> {
//!         Err(Error::new("no scripts"))
//!     }
//! }
//!
//! let mut world = World { me: VarTable::new() };
//! let mut vm = Vm::new();
//! let prog = vm.compile("example", b"var i; total = 0; for (i = 1; i <= 10; i += 1) total += i; return total", &world).unwrap();
//! let r = vm.run(&mut world, &prog, Ctx::new(100000, 100000), &[]).unwrap();
//! assert_eq!(r, Value::Real(55.0));
//! ```

#![warn(missing_docs)]

pub mod ast;
pub mod builtins;
pub mod clock;
mod compile;
pub mod ds;
mod error;
mod host;
mod interp;
pub mod lexer;
mod numfmt;
pub mod parser;
mod stdlib;
mod value;
mod vars;
mod vm;

pub use builtins::{FunctionId, VariableId};
pub use compile::Program;
pub use error::{code_error_text, source_line, CompileError, Error, ErrorKind, Location};
pub use host::{special, Ctx, Host};
pub use stdlib::is_native;
pub use numfmt::{format_gm, string_format};
pub use value::{
    binary, format_fixed, real_to_string, same_real, to_i64_x87, to_int, unary, wrap_i32, BinOp, GmlStr, UnOp, Value,
};
pub use vars::{Array2, IdBuildHasher, IdHasher, Var, VarId, VarTable, MAX_ELEMENTS, MAX_INDEX};
pub use vm::{Clock, Interner, Settings, Vm};
