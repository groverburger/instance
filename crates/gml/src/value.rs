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

/// Converts a real to a string per GML `string()` rules (spec §4.7): integral values
/// without decimals, other values with exactly two decimals (half away from zero).
pub fn real_to_string(v: f64) -> String {
    if v.is_nan() {
        return "nan".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if v.fract() == 0.0 {
        if v == 0.0 {
            return "0".into();
        }
        return format!("{:.0}", v);
    }
    format_fixed(v, 2)
}

/// Formats `v` with exactly `dec` decimals, rounding half away from zero (based on
/// the exact binary value, so `2.675` (really 2.67499999...) gives `"2.67"`).
/// A result that rounds to zero is printed without a minus sign.
pub fn format_fixed(v: f64, dec: usize) -> String {
    if !v.is_finite() {
        return real_to_string(v);
    }
    let dec = dec.min(18);
    let a = v.abs();
    let scale = 10f64.powi(dec as i32);
    let scaled = a * scale;
    let exact = a.mul_add(scale, -scaled) == 0.0;
    let mut s = if exact && scaled.fract() == 0.5 {
        // Exact tie: round away from zero.
        let digits = format!("{:.0}", scaled.floor() + 1.0);
        let digits = format!("{:0>width$}", digits, width = dec + 1);
        let (ip, fp) = digits.split_at(digits.len() - dec);
        if dec > 0 {
            format!("{}.{}", ip, fp)
        } else {
            ip.to_string()
        }
    } else {
        format!("{:.*}", dec, a)
    };
    if v < 0.0 && s.bytes().any(|b| (b'1'..=b'9').contains(&b)) {
        s.insert(0, '-');
    }
    s
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

fn shl(a: i64, n: i64) -> i64 {
    if (0..64).contains(&n) {
        a.wrapping_shl(n as u32)
    } else {
        0
    }
}

fn shr(a: i64, n: i64) -> i64 {
    if (0..64).contains(&n) {
        a >> n
    } else if a < 0 {
        -1
    } else {
        0
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
                        return Err("Division by 0.".into());
                    }
                    a % c
                }
                BitAnd => (to_int(a) & to_int(c)) as f64,
                BitOr => (to_int(a) | to_int(c)) as f64,
                BitXor => (to_int(a) ^ to_int(c)) as f64,
                Shl => shl(to_int(a), to_int(c)) as f64,
                Shr => shr(to_int(a), to_int(c)) as f64,
                Eq => return Ok(b(a == c)),
                Ne => return Ok(b(a != c)),
                Lt => return Ok(b(a < c)),
                Le => return Ok(b(a <= c)),
                Gt => return Ok(b(a > c)),
                Ge => return Ok(b(a >= c)),
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
            And | Or | Xor => Ok(logic(op, l, r)),
            _ => Err(type_err(op.symbol())),
        },
        _ => match op {
            Eq => Ok(b(false)),
            Ne => Ok(b(true)),
            Lt | Le | Gt | Ge => Ok(b(false)),
            And | Or | Xor => Ok(logic(op, l, r)),
            _ => Err(type_err(op.symbol())),
        },
    }
}

fn logic(op: BinOp, l: &Value, r: &Value) -> Value {
    let (a, c) = (l.is_true(), r.is_true());
    b(match op {
        BinOp::And => a && c,
        BinOp::Or => a || c,
        _ => a != c,
    })
}

fn type_err(sym: &str) -> String {
    format!("Wrong type of arguments to {}.", sym)
}

/// Applies a unary operator. On failure returns the error message.
#[inline]
pub fn unary(op: UnOp, v: &Value) -> Result<Value, String> {
    match (op, v) {
        (UnOp::Not, v) => Ok(b(!v.is_true())),
        (UnOp::Neg, Value::Real(a)) => Ok(Value::Real(-a)),
        (UnOp::Plus, Value::Real(a)) => Ok(Value::Real(*a)),
        (UnOp::BitNot, Value::Real(a)) => Ok(Value::Real(!to_int(*a) as f64)),
        (op, Value::Str(_)) => Err(format!(
            "Wrong type of argument to unary operator {}.",
            op.symbol()
        )),
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
        assert_eq!(real_to_string(1e20), "100000000000000000000");
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
        assert_eq!(binary(BinOp::Shr, &r(-8.0), &r(70.0)).unwrap(), r(-1.0));
        assert_eq!(binary(BinOp::BitAnd, &r(2.5), &r(3.0)).unwrap(), r(2.0)); // 2.5 -> 2 (ties even)
        let s = |x: &str| Value::from(x);
        assert_eq!(binary(BinOp::Add, &s("a"), &s("b")).unwrap(), s("ab"));
        assert!(binary(BinOp::Add, &s("a"), &r(1.0)).is_err());
        assert!(binary(BinOp::Sub, &s("a"), &s("b")).is_err());
        assert_eq!(binary(BinOp::Lt, &s("ab"), &s("abc")).unwrap(), r(1.0));
        assert_eq!(binary(BinOp::Eq, &s("1"), &r(1.0)).unwrap(), r(0.0));
        assert_eq!(binary(BinOp::Ne, &s("1"), &r(1.0)).unwrap(), r(1.0));
        assert_eq!(binary(BinOp::Lt, &s("1"), &r(1.0)).unwrap(), r(0.0));
        assert_eq!(unary(UnOp::Not, &s("x")).unwrap(), r(1.0));
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
