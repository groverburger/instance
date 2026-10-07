//! Registry of every built-in identifier listed in `specs/builtins.json`.
//!
//! The tables live in [`generated`] (produced by `crates/gml/tools/gen_builtins.py`).
//! Functions and variables are identified by small integer ids ([`FunctionId`],
//! [`VariableId`]); the [`functions`] and [`variables`] modules give each one a named
//! constant (upper-cased GML name) so a host can `match` on them, e.g.
//! `gml::builtins::functions::INSTANCE_CREATE`.

use std::collections::HashMap;
use std::sync::OnceLock;

pub mod generated;

pub use generated::{functions, variables, CONSTANTS, FUNCTIONS, VARIABLES};

/// Index of a built-in function in [`FUNCTIONS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FunctionId(pub u16);

/// Index of a built-in variable in [`VARIABLES`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VariableId(pub u16);

/// Where a built-in variable lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarScope {
    /// One copy per instance (`x`, `speed`, `alarm[]`, ...).
    Instance,
    /// One copy per game (`room`, `score`, `view_xview[]`, ...).
    Global,
    /// Per program invocation (the `argument` family). Handled by the interpreter.
    Script,
}

/// Static description of a built-in function.
#[derive(Clone, Copy, Debug)]
pub struct FunctionInfo {
    /// GML name.
    pub name: &'static str,
    /// Minimum number of arguments accepted.
    pub min_args: u8,
    /// Maximum number of arguments accepted (16 for variadic functions).
    pub max_args: u8,
    /// Whether the function takes a variable number of arguments.
    pub variadic: bool,
    /// Manual chapter the function belongs to (e.g. `"402"`), or `"yoyo"`.
    pub chapter: &'static str,
}

/// Static description of a built-in variable.
#[derive(Clone, Copy, Debug)]
pub struct VariableInfo {
    /// GML name.
    pub name: &'static str,
    /// Storage scope.
    pub scope: VarScope,
    /// Whether assignments are rejected at compile time.
    pub readonly: bool,
    /// For array variables, the inclusive index bounds (`i32::MAX` = dynamic upper
    /// bound, checked by the host).
    pub array: Option<(i32, i32)>,
}

/// Static description of a built-in constant.
#[derive(Clone, Copy, Debug)]
pub struct ConstantInfo {
    /// GML name.
    pub name: &'static str,
    /// Numeric value (all built-in constants are reals).
    pub value: f64,
    /// `true` when the value is not stated by the manual and must be verified.
    pub inferred: bool,
}

struct Maps {
    functions: HashMap<&'static str, FunctionId>,
    variables: HashMap<&'static str, VariableId>,
    constants: HashMap<&'static str, f64>,
}

fn maps() -> &'static Maps {
    static MAPS: OnceLock<Maps> = OnceLock::new();
    MAPS.get_or_init(|| Maps {
        functions: FUNCTIONS
            .iter()
            .enumerate()
            .map(|(i, f)| (f.name, FunctionId(i as u16)))
            .collect(),
        variables: VARIABLES
            .iter()
            .enumerate()
            .map(|(i, v)| (v.name, VariableId(i as u16)))
            .collect(),
        constants: CONSTANTS.iter().map(|c| (c.name, c.value)).collect(),
    })
}

/// Looks up a built-in function by name.
pub fn function_id(name: &str) -> Option<FunctionId> {
    maps().functions.get(name).copied()
}

/// Looks up a built-in variable by name.
pub fn variable_id(name: &str) -> Option<VariableId> {
    maps().variables.get(name).copied()
}

/// Looks up a built-in constant's value by name.
pub fn constant_value(name: &str) -> Option<f64> {
    maps().constants.get(name).copied()
}

impl FunctionId {
    /// Static information about this function.
    pub fn info(self) -> &'static FunctionInfo {
        &FUNCTIONS[self.0 as usize]
    }
    /// GML name of this function.
    pub fn name(self) -> &'static str {
        self.info().name
    }
}

impl VariableId {
    /// Static information about this variable.
    pub fn info(self) -> &'static VariableInfo {
        &VARIABLES[self.0 as usize]
    }
    /// GML name of this variable.
    pub fn name(self) -> &'static str {
        self.info().name
    }
}
