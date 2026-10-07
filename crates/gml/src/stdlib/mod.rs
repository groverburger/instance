//! Built-in functions implemented by the interpreter itself ("natives"): the pure
//! computing functions of chapter 402, data structures (411), variables by name,
//! and code execution (`script_execute`, `execute_string`, `execute_file`).
//! Every other built-in function is dispatched to [`Host::call_function`].

use crate::builtins::{self, FunctionId, FUNCTIONS};
use crate::error::Error;
use crate::host::{Ctx, Host};
use crate::value::{to_int, Value};
use crate::vm::Vm;
use std::sync::OnceLock;

pub(crate) mod date;
mod ds_fns;
mod math;
mod misc;
mod string;

/// Result of a native function.
pub(crate) type NResult = Result<Value, Error>;

/// Signature of a native function.
pub(crate) type NativeFn = fn(&mut Vm, &mut dyn Host, &Ctx, Args) -> NResult;

/// Arguments passed to a native, with typed accessors that produce GML errors.
#[derive(Clone, Copy)]
pub(crate) struct Args<'a> {
    pub vals: &'a [Value],
    pub func: FunctionId,
}

impl<'a> Args<'a> {
    pub fn len(&self) -> usize {
        self.vals.len()
    }
    pub fn val(&self, i: usize) -> &'a Value {
        const ZERO: &Value = &Value::Real(0.0);
        self.vals.get(i).unwrap_or(ZERO)
    }
    pub fn type_err(&self) -> Error {
        Error::new(format!(
            "Wrong type of arguments to function {}.",
            self.func.name()
        ))
    }
    pub fn invalid(&self) -> Error {
        Error::new(format!(
            "Invalid argument to function {}.",
            self.func.name()
        ))
    }
    pub fn real(&self, i: usize) -> Result<f64, Error> {
        match self.val(i) {
            Value::Real(x) => Ok(*x),
            Value::Str(_) => Err(self.type_err()),
        }
    }
    pub fn int(&self, i: usize) -> Result<i64, Error> {
        Ok(to_int(self.real(i)?))
    }
    pub fn str(&self, i: usize) -> Result<&'a [u8], Error> {
        match self.val(i) {
            Value::Str(s) => Ok(s.as_bytes()),
            Value::Real(_) => Err(self.type_err()),
        }
    }
}

pub(crate) fn n(name: &'static str, f: NativeFn) -> (&'static str, NativeFn) {
    (name, f)
}

fn table() -> &'static Vec<Option<NativeFn>> {
    static T: OnceLock<Vec<Option<NativeFn>>> = OnceLock::new();
    T.get_or_init(|| {
        let mut list = Vec::new();
        math::register(&mut list);
        string::register(&mut list);
        date::register(&mut list);
        ds_fns::register(&mut list);
        misc::register(&mut list);
        let mut t: Vec<Option<NativeFn>> = vec![None; FUNCTIONS.len()];
        for (name, f) in list {
            let id = builtins::function_id(name)
                .unwrap_or_else(|| panic!("native {} is not in builtins.json", name));
            t[id.0 as usize] = Some(f);
        }
        t
    })
}

/// The native implementation of a function, if the interpreter has one.
pub(crate) fn native(f: FunctionId) -> Option<NativeFn> {
    table()[f.0 as usize]
}

/// Whether the interpreter implements built-in function `f` itself. Functions for
/// which this is `false` are sent to [`Host::call_function`].
pub fn is_native(f: FunctionId) -> bool {
    native(f).is_some()
}
