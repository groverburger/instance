//! GML values: reals and (byte) strings, with the language's operator semantics.

use std::cmp::Ordering;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// An immutable, reference-counted GML string.
///
/// GML strings are sequences of 8-bit characters in the game's ANSI code page, so the
/// content is stored as raw bytes. Converting to/from Rust strings copies bytes as-is
/// (ASCII is unaffected); decoding the code page for display is the host's concern.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct GmlStr(Rc<[u8]>);

impl GmlStr {
    /// Creates a string from raw bytes.
    pub fn from_bytes(b: &[u8]) -> Self {
        GmlStr(Rc::from(b))
    }
    /// The raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    /// Lossy conversion for display/debugging (bytes ≥ 0x80 are mapped as Latin-1).
    pub fn to_string_lossy(&self) -> String {
        if let Ok(s) = std::str::from_utf8(&self.0) {
            if s.is_ascii() {
                return s.to_owned();
            }
        }
        self.0.iter().map(|&b| b as char).collect()
    }
}

impl Deref for GmlStr {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl From<&str> for GmlStr {
    fn from(s: &str) -> Self {
        GmlStr::from_bytes(s.as_bytes())
    }
}
impl From<&[u8]> for GmlStr {
    fn from(s: &[u8]) -> Self {
        GmlStr::from_bytes(s)
    }
}
impl From<Vec<u8>> for GmlStr {
    fn from(s: Vec<u8>) -> Self {
        GmlStr(Rc::from(s))
    }
}
impl From<String> for GmlStr {
    fn from(s: String) -> Self {
        GmlStr::from(s.into_bytes())
    }
}

impl fmt::Debug for GmlStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_string_lossy())
    }
}
impl fmt::Display for GmlStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

/// A GML value: either a real (IEEE double) or a string.
#[derive(Clone, Debug)]
pub enum Value {
    /// A real number. Booleans, ids, colours, etc. are all reals.
    Real(f64),
    /// A string.
    Str(GmlStr),
}

impl Default for Value {
    fn default() -> Self {
        Value::Real(0.0)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Real(v)
    }
}
impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Real(v as f64)
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Real(if v { 1.0 } else { 0.0 })
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Str(v.into())
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Str(v.into())
    }
}
impl From<GmlStr> for Value {
    fn from(v: GmlStr) -> Self {
        Value::Str(v)
    }
}
impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Str(v.into())
    }
}

/// Structural equality: same type and same contents (reals compared with `==`).
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Real(a), Value::Real(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            _ => false,
        }
    }
}

/// Rounds to the nearest integer, ties to even, saturating to the `i64` range
/// (NaN → 0). This is the language's `to_int` conversion (spec §4.6).
#[inline]
pub fn to_int(v: f64) -> i64 {
    v.round_ties_even() as i64
}

pub use crate::numfmt::{format_fixed, real_to_string};

/// The 64-bit integer conversion of bitwise operands: round half to even like
/// [`to_int`], but values outside the `i64` range (and NaN) give `i64::MIN`, as
/// the x87 integer store does (spec §4.6).
#[inline]
pub fn to_i64_x87(v: f64) -> i64 {
    let r = v.round_ties_even();
    if (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&r) {
        r as i64
    } else {
        i64::MIN
    }
}

/// `Integer(Trunc(v))`: truncation to a 64-bit integer, wrapped to 32 bits. GM 8.1
/// converts this way in `floor`, `ceil` and `file_text_write_real` (VERIFIED
/// black-box: bb_random raw values >= 2^31 print negative, bb_optional
/// file.real_line4). Out-of-range values give 0.
#[inline]
pub fn wrap_i32(v: f64) -> i32 {
    let t = v.trunc();
    if (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&t) {
        t as i64 as i32
    } else {
        0
    }
}

impl Value {
    /// Creates a string value from bytes.
    pub fn bytes(b: &[u8]) -> Self {
        Value::Str(GmlStr::from_bytes(b))
    }

    /// Truthiness (spec §4.3): a real is true iff `>= 0.5`; NaN and strings are false.
    #[inline]
    pub fn is_true(&self) -> bool {
        match self {
            Value::Real(v) => *v >= 0.5,
            Value::Str(_) => false,
        }
    }

    /// `true` if this is a real.
    pub fn is_real(&self) -> bool {
        matches!(self, Value::Real(_))
    }

    /// `true` if this is a string.
    pub fn is_str(&self) -> bool {
        matches!(self, Value::Str(_))
    }

    /// The real value, if this is a real.
    pub fn as_real(&self) -> Option<f64> {
        match self {
            Value::Real(v) => Some(*v),
            Value::Str(_) => None,
        }
    }

    /// The string bytes, if this is a string.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Str(s) => Some(s.as_bytes()),
            Value::Real(_) => None,
        }
    }

    /// The real value, or 0 for strings.
    pub fn real_or_zero(&self) -> f64 {
        self.as_real().unwrap_or(0.0)
    }

    /// Converts to a display string the way `string()` does.
    pub fn to_gml_string(&self) -> GmlStr {
        match self {
            Value::Real(v) => real_to_string(*v).into(),
            Value::Str(s) => s.clone(),
        }
    }

    /// Ordering used by data structures and `min`/`max`: reals numerically, strings
    /// ordinally, every real sorts before every string.
    pub fn total_cmp(&self, other: &Value) -> Ordering {
        match (self, other) {
            (Value::Real(a), Value::Real(b)) => a.partial_cmp(b).unwrap_or_else(|| a.total_cmp(b)),
            (Value::Str(a), Value::Str(b)) => a.as_bytes().cmp(b.as_bytes()),
            (Value::Real(_), Value::Str(_)) => Ordering::Less,
            (Value::Str(_), Value::Real(_)) => Ordering::Greater,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Real(v) => f.write_str(&real_to_string(*v)),
            Value::Str(s) => fmt::Display::fmt(s, f),
        }
    }
}

/// Binary operators of the language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `div`
    IntDiv,
    /// `mod`, `%`
    Mod,
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `==`, `=`, `:=` (in expressions)
    Eq,
    /// `!=`, `<>`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`, `and`
    And,
    /// `||`, `or`
    Or,
    /// `^^`, `xor`
    Xor,
}

impl BinOp {
    /// Source symbol used in error messages.
    pub fn symbol(self) -> &'static str {
        use BinOp::*;
        match self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            IntDiv => "div",
            Mod => "mod",
            BitAnd => "&",
            BitOr => "|",
            BitXor => "^",
            Shl => "<<",
            Shr => ">>",
            Eq => "==",
            Ne => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            And => "&&",
            Or => "||",
            Xor => "^^",
        }
    }
}

/// Unary operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    /// `-`
    Neg,
    /// `+`
    Plus,
    /// `!`, `not`
    Not,
    /// `~`
    BitNot,
}

impl UnOp {
    /// Source symbol used in error messages.
    pub fn symbol(self) -> &'static str {
        match self {
            UnOp::Neg => "-",
            UnOp::Plus => "+",
            UnOp::Not => "!",
            UnOp::BitNot => "~",
        }
    }
}

#[inline]
fn b(v: bool) -> Value {
    Value::Real(if v { 1.0 } else { 0.0 })
}

/// `Int64 shl n` as GM 8.1 computes it (Delphi's 64-bit shift helper): the count is
/// the low byte of the integer, taken as signed; below 32 (including negative
/// counts) the shift is by `count & 31`, 32..63 shifts the low word into the high
/// word, 64 and more give 0. VERIFIED black-box (bb_lang bit.1_shl_m1 = 2^31,
/// bit.1_shl_63, bit.1_shl_64).
fn shl(a: i64, n: i64) -> i64 {
    let cl = n as u8 as i8;
    let (mut lo, mut hi) = (a as u32, (a as u64 >> 32) as u32);
    if cl < 32 {
        let c = (cl as u8 & 31) as u32;
        if c != 0 {
            hi = (hi << c) | (lo >> (32 - c));
            lo <<= c;
        }
    } else if cl < 64 {
        hi = lo << (cl as u32 & 31);
        lo = 0;
    } else {
        return 0;
    }
    (((hi as u64) << 32) | lo as u64) as i64
}

/// `Int64 shr n`: a logical (unsigned) shift with the same count rules as [`shl`].
/// VERIFIED black-box (bb_lang bit.m1_shr_1 = 2^63-1, bit.m8_shr_1).
fn shr(a: i64, n: i64) -> i64 {
    let cl = n as u8 as i8;
    let (mut lo, mut hi) = (a as u32, (a as u64 >> 32) as u32);
    if cl < 32 {
        let c = (cl as u8 & 31) as u32;
        if c != 0 {
            lo = (lo >> c) | (hi << (32 - c));
            hi >>= c;
        }
    } else if cl < 64 {
        lo = hi >> (cl as u32 & 31);
        hi = 0;
    } else {
        return 0;
    }
    (((hi as u64) << 32) | lo as u64) as i64
}

/// Tolerance of GM 8.1's real equality, see [`same_real`].
pub const REAL_EPS: f64 = 1e-14;

/// GM 8.1's real equality: `|a - b| <= max(min(|a|, |b|), 1) * 1e-14`.
///
/// VERIFIED black-box bounds (bb_lang eq.*, math.sin_pi_is0,
/// math.lengthdir_x_90_is0; bb_lang2 eq2.*): `sin(pi) == 0` (1.2e-16) and
/// `lengthdir_x(10, 90) == 0` (6.1e-16) are true; `0 == 1e-13`, `1 == 1 + 5e-13`,
/// `1 == 1.0000000001` and `1000000 == 1000000 + 1e-7` are false (so the tolerance
/// is below 1e-13 near 0 and 1, and a relative factor is below 1e-13). The
/// constant 1e-14 is chosen inside those bounds; the bb_lang3 eq3.* keys pin it.
#[inline]
pub fn same_real(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    let eps = a.abs().min(b.abs()).max(1.0) * REAL_EPS;
    (a - b).abs() <= eps
}

/// Real comparison with [`same_real`] equality.
#[inline]
pub(crate) fn cmp_real(op: BinOp, a: f64, c: f64) -> bool {
    let same = same_real(a, c);
    match op {
        BinOp::Eq => same,
        BinOp::Ne => !same,
        BinOp::Lt => a < c && !same,
        BinOp::Le => a < c || same,
        BinOp::Gt => a > c && !same,
        _ => a > c || same,
    }
}

/// Applies a binary operator. On failure returns the error message.
#[inline]
pub fn binary(op: BinOp, l: &Value, r: &Value) -> Result<Value, String> {
    use BinOp::*;
    match (l, r) {
        (Value::Real(a), Value::Real(c)) => {
            let (a, c) = (*a, *c);
            Ok(Value::Real(match op {
                Add => a + c,
                Sub => a - c,
                Mul => a * c,
                Div => {
                    if c == 0.0 {
                        return Err("Division by 0.".into());
                    }
                    a / c
                }
                IntDiv => {
                    if c == 0.0 {
                        return Err("Division by 0.".into());
                    }
                    (a / c).trunc()
                }
                Mod => {
                    if c == 0.0 {
                        return Err("Modulo by 0.".into());
                    }
                    a % c
                }
                BitAnd => (to_i64_x87(a) & to_i64_x87(c)) as f64,
                BitOr => (to_i64_x87(a) | to_i64_x87(c)) as f64,
                BitXor => (to_i64_x87(a) ^ to_i64_x87(c)) as f64,
                Shl => shl(to_i64_x87(a), to_i64_x87(c)) as f64,
                Shr => shr(to_i64_x87(a), to_i64_x87(c)) as f64,
                Eq | Ne | Lt | Le | Gt | Ge => return Ok(b(cmp_real(op, a, c))),
                And => return Ok(b(a >= 0.5 && c >= 0.5)),
                Or => return Ok(b(a >= 0.5 || c >= 0.5)),
                Xor => return Ok(b((a >= 0.5) != (c >= 0.5))),
            }))
        }
        (Value::Str(a), Value::Str(c)) => match op {
            Add => {
                let mut v = Vec::with_capacity(a.len() + c.len());
                v.extend_from_slice(a);
                v.extend_from_slice(c);
                Ok(Value::Str(v.into()))
            }
            Eq => Ok(b(a.as_bytes() == c.as_bytes())),
            Ne => Ok(b(a.as_bytes() != c.as_bytes())),
            Lt => Ok(b(a.as_bytes() < c.as_bytes())),
            Le => Ok(b(a.as_bytes() <= c.as_bytes())),
            Gt => Ok(b(a.as_bytes() > c.as_bytes())),
            Ge => Ok(b(a.as_bytes() >= c.as_bytes())),
            _ => Err(type_err(op)),
        },
        // VERIFIED black-box (bb_lang mix.*): comparing a string with a real is an
        // error, like arithmetic and logic on mixed operands.
        _ => match op {
            Eq | Ne | Lt | Le | Gt | Ge => Err("Cannot compare arguments.".into()),
            _ => Err(type_err(op)),
        },
    }
}

/// "Wrong type of arguments to +." for arithmetic, "... for &&." for bitwise and
/// logical operators (GM 8.1 wording, bb_lang mix.*.msg).
fn type_err(op: BinOp) -> String {
    use BinOp::*;
    match op {
        Add | Sub | Mul | Div | IntDiv | Mod => format!("Wrong type of arguments to {}.", op.symbol()),
        _ => format!("Wrong type of arguments for {}.", op.symbol()),
    }
}

/// Applies a unary operator. On failure returns the error message.
#[inline]
pub fn unary(op: UnOp, v: &Value) -> Result<Value, String> {
    match (op, v) {
        (UnOp::Not, Value::Real(a)) => Ok(b(!(*a >= 0.5))),
        (UnOp::Neg, Value::Real(a)) => Ok(Value::Real(-a)),
        (UnOp::Plus, Value::Real(a)) => Ok(Value::Real(*a)),
        (UnOp::BitNot, Value::Real(a)) => Ok(Value::Real(!to_i64_x87(*a) as f64)),
        // VERIFIED black-box (bb_lang mix.not_s, mix.neg_s)
        (_, Value::Str(_)) => Err("Wrong type of arguments to unary operator.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_conversion() {
        assert_eq!(real_to_string(5.0), "5");
        assert_eq!(real_to_string(-0.0), "0");
        assert_eq!(real_to_string(0.5), "0.50");
        assert_eq!(real_to_string(1.0 / 3.0), "0.33");
        assert_eq!(real_to_string(-2.675), "-2.67"); // -2.67499999... in binary
        assert_eq!(real_to_string(0.125), "0.13"); // exact tie, away from zero
        assert_eq!(real_to_string(-0.001), "0.00");
        assert_eq!(real_to_string(1e20), "ERROR");
        assert_eq!(real_to_string(123456.789), "123456.79");
    }

    #[test]
    fn ops() {
        let r = |x: f64| Value::Real(x);
        assert_eq!(binary(BinOp::IntDiv, &r(7.5), &r(2.0)).unwrap(), r(3.0));
        assert_eq!(binary(BinOp::Mod, &r(7.5), &r(2.0)).unwrap(), r(1.5));
        assert_eq!(binary(BinOp::IntDiv, &r(-7.0), &r(2.0)).unwrap(), r(-3.0));
        assert_eq!(binary(BinOp::Mod, &r(-7.0), &r(2.0)).unwrap(), r(-1.0));
        assert!(binary(BinOp::Div, &r(1.0), &r(0.0)).is_err());
        assert_eq!(binary(BinOp::Shl, &r(1.0), &r(70.0)).unwrap(), r(0.0));
        assert_eq!(binary(BinOp::Shr, &r(-8.0), &r(70.0)).unwrap(), r(0.0));
        // Delphi 64-bit shifts (VERIFIED black-box bb_lang bit.*)
        assert_eq!(binary(BinOp::Shl, &r(1.0), &r(-1.0)).unwrap(), r(2147483648.0));
        assert_eq!(binary(BinOp::Shl, &r(1.0), &r(63.0)).unwrap(), r(-9223372036854775808.0));
        assert_eq!(binary(BinOp::Shl, &r(1.0), &r(64.0)).unwrap(), r(0.0));
        assert_eq!(binary(BinOp::Shr, &r(-1.0), &r(1.0)).unwrap(), r(9223372036854775807.0));
        assert_eq!(binary(BinOp::Shr, &r(1099511627776.0), &r(8.0)).unwrap(), r(4294967296.0));
        assert_eq!(binary(BinOp::BitOr, &r(-2147483649.0), &r(0.0)).unwrap(), r(-2147483649.0));
        assert_eq!(binary(BinOp::BitOr, &r(4294967295.5), &r(0.0)).unwrap(), r(4294967296.0));
        assert!(binary(BinOp::Mod, &r(5.0), &r(0.0)).unwrap_err().contains("Modulo by 0"));
        assert_eq!(binary(BinOp::BitAnd, &r(2.5), &r(3.0)).unwrap(), r(2.0)); // 2.5 -> 2 (ties even)
        let s = |x: &str| Value::from(x);
        assert_eq!(binary(BinOp::Add, &s("a"), &s("b")).unwrap(), s("ab"));
        assert!(binary(BinOp::Add, &s("a"), &r(1.0)).is_err());
        assert!(binary(BinOp::Sub, &s("a"), &s("b")).is_err());
        assert_eq!(binary(BinOp::Lt, &s("ab"), &s("abc")).unwrap(), r(1.0));
        // VERIFIED black-box (bb_lang mix.*): mixed comparisons and logic are errors
        assert_eq!(binary(BinOp::Eq, &s("1"), &r(1.0)).unwrap_err(), "Cannot compare arguments.");
        assert!(binary(BinOp::Ne, &s("1"), &r(1.0)).is_err());
        assert!(binary(BinOp::Lt, &s("1"), &r(1.0)).is_err());
        assert_eq!(binary(BinOp::And, &s("a"), &r(1.0)).unwrap_err(), "Wrong type of arguments for &&.");
        assert_eq!(binary(BinOp::BitOr, &s("a"), &r(1.0)).unwrap_err(), "Wrong type of arguments for |.");
        assert!(unary(UnOp::Not, &s("x")).is_err());
        // tolerant equality (Delphi SameValue)
        assert_eq!(binary(BinOp::Eq, &r(0.1 + 0.2), &r(0.3)).unwrap(), r(1.0));
        assert_eq!(binary(BinOp::Lt, &r(0.3), &r(0.1 + 0.2)).unwrap(), r(0.0));
        assert_eq!(binary(BinOp::Ge, &r(0.3), &r(0.1 + 0.2)).unwrap(), r(1.0));
        assert_eq!(binary(BinOp::Eq, &r(std::f64::consts::PI.sin()), &r(0.0)).unwrap(), r(1.0));
        assert_eq!(binary(BinOp::Eq, &r(1.0), &r(1.0000000001)).unwrap(), r(0.0));
        assert_eq!(binary(BinOp::Eq, &r(0.0), &r(1.0000000000000002e-12)).unwrap(), r(0.0));
        // VERIFIED black-box (bb_lang2 eq2.*): the tolerance is below 1e-13
        assert_eq!(binary(BinOp::Eq, &r(0.0), &r(1e-13)).unwrap(), r(0.0));
        assert_eq!(binary(BinOp::Eq, &r(0.0), &r(5e-13)).unwrap(), r(0.0));
        assert_eq!(binary(BinOp::Eq, &r(1.0), &r(1.0 + 5e-13)).unwrap(), r(0.0));
        assert_eq!(binary(BinOp::Eq, &r(1e6), &r(1e6 + 1e-7)).unwrap(), r(0.0));
        assert_eq!(binary(BinOp::Lt, &r(1e6), &r(1e6 + 1e-7)).unwrap(), r(1.0));
        assert_eq!(binary(BinOp::Eq, &r(10.0 * (std::f64::consts::PI / 2.0).cos()), &r(0.0)).unwrap(), r(1.0));
        assert!(unary(UnOp::Neg, &s("x")).is_err());
        assert_eq!(unary(UnOp::BitNot, &r(0.0)).unwrap(), r(-1.0));
    }

    #[test]
    fn truthiness() {
        assert!(Value::Real(0.5).is_true());
        assert!(!Value::Real(0.49).is_true());
        assert!(!Value::Real(-1.0).is_true());
        assert!(!Value::Real(f64::NAN).is_true());
        assert!(!Value::from("1").is_true());
    }
}
