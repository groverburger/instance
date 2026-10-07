//! Variable storage: array-capable variable slots and variable tables.
//!
//! Every user variable (instance, global or `var` local) can be used as a scalar,
//! a 1-D array or a 2-D array (spec §9). The storage model used here:
//! the plain variable is element `[0,0]`, a 1-D access `a[i]` is `a[0,i]`, rows grow
//! independently, unwritten elements inside a row's extent read as real 0 and
//! anything outside the extent is undefined.

use crate::value::Value;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// Interned identifier of a user variable name. Ids are allocated by
/// [`Vm::intern`](crate::Vm::intern) and are stable for the lifetime of the `Vm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VarId(pub u32);

/// Maximum value (exclusive) of each array index.
pub const MAX_INDEX: usize = 32000;
/// Maximum number of elements in one array variable.
pub const MAX_ELEMENTS: usize = 1_000_000;

/// The contents of one variable.
#[derive(Clone, Debug)]
pub enum Var {
    /// A variable holding a single value (only element `[0,0]` exists).
    Scalar(Value),
    /// A variable that has been used as an array.
    Array(Box<Array2>),
}

/// A 2-D, row-ragged array.
#[derive(Clone, Debug, Default)]
pub struct Array2 {
    rows: Vec<Vec<Value>>,
    count: usize,
}

impl Array2 {
    /// The rows (row 0 holds 1-D elements).
    pub fn rows(&self) -> &[Vec<Value>] {
        &self.rows
    }
    /// Total number of allocated elements.
    pub fn len(&self) -> usize {
        self.count
    }
    /// `true` if no element was allocated.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl Var {
    /// Reads element `[i, j]`, or `None` if outside the variable's extent.
    #[inline]
    pub fn get(&self, i: usize, j: usize) -> Option<&Value> {
        match self {
            Var::Scalar(v) => (i == 0 && j == 0).then_some(v),
            Var::Array(a) => a.rows.get(i)?.get(j),
        }
    }

    /// Writes element `[i, j]`, growing the array as needed. Returns an error message
    /// if the array would exceed [`MAX_ELEMENTS`]. Indices must already be validated
    /// against [`MAX_INDEX`].
    pub fn set(&mut self, i: usize, j: usize, v: Value) -> Result<(), String> {
        if i == 0 && j == 0 {
            if let Var::Scalar(s) = self {
                *s = v;
                return Ok(());
            }
        }
        if let Var::Scalar(s) = self {
            let s = std::mem::take(s);
            *self = Var::Array(Box::new(Array2 {
                rows: vec![vec![s]],
                count: 1,
            }));
        }
        let Var::Array(a) = self else { unreachable!() };
        if i >= a.rows.len() {
            a.rows.resize_with(i + 1, Vec::new);
        }
        let row_len = a.rows[i].len();
        if j >= row_len {
            let extra = j + 1 - row_len;
            if a.count + extra > MAX_ELEMENTS {
                return Err("Array too large (more than 1000000 elements).".into());
            }
            a.count += extra;
            a.rows[i].resize(j + 1, Value::Real(0.0));
        }
        a.rows[i][j] = v;
        Ok(())
    }

    /// Creates a variable holding `[i, j] = v`.
    pub fn new_at(i: usize, j: usize, v: Value) -> Result<Var, String> {
        if i == 0 && j == 0 {
            return Ok(Var::Scalar(v));
        }
        let mut var = Var::Array(Box::default());
        var.set(i, j, v)?;
        Ok(var)
    }
}

/// A fast hasher for small integer keys.
#[derive(Default, Clone, Copy)]
pub struct IdHasher(u64);

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }
    fn write_u32(&mut self, n: u32) {
        self.0 = (n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    fn write_i32(&mut self, n: i32) {
        self.write_u32(n as u32)
    }
}

/// `BuildHasher` for [`IdHasher`].
pub type IdBuildHasher = BuildHasherDefault<IdHasher>;

/// A table of user variables (one per instance, plus one for globals).
///
/// Hosts embed one `VarTable` in each instance and hand it to the interpreter via
/// [`Host::instance_vars`](crate::Host::instance_vars).
#[derive(Clone, Debug, Default)]
pub struct VarTable {
    map: HashMap<VarId, Var, IdBuildHasher>,
}

impl VarTable {
    /// Creates an empty table.
    pub fn new() -> Self {
        Self::default()
    }
    /// Looks up a variable.
    #[inline]
    pub fn get(&self, id: VarId) -> Option<&Var> {
        self.map.get(&id)
    }
    /// Looks up a variable mutably.
    #[inline]
    pub fn get_mut(&mut self, id: VarId) -> Option<&mut Var> {
        self.map.get_mut(&id)
    }
    /// Reads element `[i, j]` of a variable.
    #[inline]
    pub fn get_elem(&self, id: VarId, i: usize, j: usize) -> Option<&Value> {
        self.map.get(&id)?.get(i, j)
    }
    /// Reads a variable as a scalar (element `[0,0]`).
    pub fn get_value(&self, id: VarId) -> Option<&Value> {
        self.get_elem(id, 0, 0)
    }
    /// Writes element `[i, j]`, creating the variable if needed.
    #[inline]
    pub fn set_elem(&mut self, id: VarId, i: usize, j: usize, v: Value) -> Result<(), String> {
        match self.map.get_mut(&id) {
            Some(var) => var.set(i, j, v),
            None => {
                self.map.insert(id, Var::new_at(i, j, v)?);
                Ok(())
            }
        }
    }
    /// Writes a scalar value (element `[0,0]`).
    pub fn set_value(&mut self, id: VarId, v: Value) {
        let _ = self.set_elem(id, 0, 0, v);
    }
    /// Whether the variable exists.
    pub fn contains(&self, id: VarId) -> bool {
        self.map.contains_key(&id)
    }
    /// Removes a variable.
    pub fn remove(&mut self, id: VarId) -> Option<Var> {
        self.map.remove(&id)
    }
    /// Removes all variables.
    pub fn clear(&mut self) {
        self.map.clear()
    }
    /// Number of variables.
    pub fn len(&self) -> usize {
        self.map.len()
    }
    /// `true` if there are no variables.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    /// Iterates over all variables (unspecified order).
    pub fn iter(&self) -> impl Iterator<Item = (VarId, &Var)> {
        self.map.iter().map(|(k, v)| (*k, v))
    }
    /// Inserts a whole variable.
    pub fn insert(&mut self, id: VarId, var: Var) {
        self.map.insert(id, var);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_model() {
        let mut t = VarTable::new();
        let a = VarId(1);
        t.set_value(a, Value::Real(5.0));
        assert_eq!(t.get_elem(a, 0, 0), Some(&Value::Real(5.0)));
        assert_eq!(t.get_elem(a, 0, 1), None);
        t.set_elem(a, 0, 3, Value::Real(7.0)).unwrap();
        assert_eq!(t.get_elem(a, 0, 0), Some(&Value::Real(5.0)));
        assert_eq!(t.get_elem(a, 0, 2), Some(&Value::Real(0.0)));
        assert_eq!(t.get_elem(a, 0, 4), None);
        let b = VarId(2);
        t.set_elem(b, 4, 6, Value::Real(1.0)).unwrap();
        assert_eq!(t.get_elem(b, 0, 0), None);
        assert_eq!(t.get_elem(b, 4, 0), Some(&Value::Real(0.0)));
        assert_eq!(t.get_elem(b, 3, 0), None);
        let c = VarId(3);
        let mut failed = false;
        for i in 0..40 {
            if t.set_elem(c, i, 31999, Value::Real(1.0)).is_err() {
                failed = true;
                assert_eq!(i, 31); // 31 * 32000 <= 1e6 < 32 * 32000
                break;
            }
        }
        assert!(failed);
    }
}
