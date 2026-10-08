//! String functions (chapter 402, "string" section). Strings are byte strings;
//! positions are 1-based byte positions.

use super::{n, Args, NResult, NativeFn};
use crate::numfmt::string_format;
use crate::value::Value;

/// Parses the longest numeric prefix of `s` (after leading whitespace):
/// `[+-] digits [. digits] [(e|E) [+-] digits]`. Returns 0 if there is none.
pub(crate) fn parse_real(s: &[u8]) -> f64 {
    let mut i = 0;
    while i < s.len() && s[i] <= b' ' {
        i += 1;
    }
    let start = i;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        i += 1;
    }
    let ds = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    let mut nd = i - ds;
    if i < s.len() && s[i] == b'.' {
        let f = i + 1;
        let mut k = f;
        while k < s.len() && s[k].is_ascii_digit() {
            k += 1;
        }
        nd += k - f;
        if nd > 0 {
            i = k;
        }
    }
    if nd == 0 {
        return 0.0;
    }
    let mantissa_end = i;
    if i < s.len() && (s[i] == b'e' || s[i] == b'E') {
        let mut k = i + 1;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            k += 1;
        }
        let es = k;
        while k < s.len() && s[k].is_ascii_digit() {
            k += 1;
        }
        if k > es {
            i = k;
        }
    }
    let text = std::str::from_utf8(&s[start..i]).unwrap_or("0");
    let text = text.strip_prefix('+').unwrap_or(text);
    let mut fixed = text.to_string();
    // Rust does not accept "5." or ".5" forms with exponent oddities; normalise.
    if fixed.contains(".e") || fixed.contains(".E") {
        fixed = fixed.replace(".e", ".0e").replace(".E", ".0E");
    }
    fixed.parse::<f64>().unwrap_or_else(|_| {
        std::str::from_utf8(&s[start..mantissa_end])
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(0.0)
    })
}

/// `real(str)`: VERIFIED black-box (bb_lang real.*). Surrounding whitespace is
/// ignored; the rest must be a complete number `[+-] digits [. digits]
/// [(e|E) [+-] digits]` ("", "." and "-.5" are fine, giving 0, 0, -0.5);
/// anything else ("abc", "12abc", "0x10", "$10", "1,5", "inf", "-") is an error.
pub(crate) fn real_strict(s: &[u8]) -> Option<f64> {
    let t = s.trim_ascii();
    if t.is_empty() {
        return Some(0.0);
    }
    let mut i = 0;
    let signed = t[0] == b'+' || t[0] == b'-';
    if signed {
        i += 1;
    }
    let mut nd = 0;
    while i < t.len() && t[i].is_ascii_digit() {
        i += 1;
        nd += 1;
    }
    if i < t.len() && t[i] == b'.' {
        i += 1;
        while i < t.len() && t[i].is_ascii_digit() {
            i += 1;
            nd += 1;
        }
    }
    if nd > 0 && i < t.len() && (t[i] == b'e' || t[i] == b'E') {
        i += 1;
        if i < t.len() && (t[i] == b'+' || t[i] == b'-') {
            i += 1;
        }
        let es = i;
        while i < t.len() && t[i].is_ascii_digit() {
            i += 1;
        }
        if i == es {
            return None;
        }
    }
    if i != t.len() || (signed && nd == 0) {
        return None;
    }
    if nd == 0 {
        return Some(0.0);
    }
    Some(parse_real(t))
}

/// Unicode code points of the Windows-1252 bytes 0x80-0x9F (the five undefined
/// bytes stay the C1 control code points of the same value).
const CP1252_HIGH: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
];

/// The character of string byte `b` (Windows-1252).
pub(crate) fn cp1252_char(b: u8) -> char {
    match b {
        0x80..=0x9F => char::from_u32(CP1252_HIGH[(b - 0x80) as usize] as u32).unwrap_or('?'),
        _ => b as char,
    }
}

/// The Windows-1252 byte of character `c`, if it has one.
fn cp1252_byte(c: char) -> Option<u8> {
    let u = c as u32;
    if u < 0x80 || (0xA0..=0xFF).contains(&u) {
        Some(u as u8)
    } else {
        CP1252_HIGH.iter().position(|&h| h as u32 == u).map(|i| 0x80 + i as u8)
    }
}

/// GML strings are byte strings whose characters are Windows-1252 (`chr(n)` is byte
/// `n`, `string_length` counts characters = bytes). `string_lower/upper` follow the
/// Unicode case mapping of those characters and keep one byte per character
/// (VERIFIED black-box, bb_lang2 case.*, bb_lang3 case3.*):
/// `string_lower(chr(192)) == chr(224)`, `string_upper(chr(233)) == chr(201)`,
/// `string_lower(chr(215)) == chr(215)` (×), `string_upper(chr(255))` is Ÿ (byte
/// 0x9F, `ord` 197), lengths unchanged. A character whose mapping is not a single
/// Windows-1252 character stays as it is.
fn case_map(s: &[u8], upper: bool) -> Vec<u8> {
    s.iter()
        .map(|&b| {
            if b < 0x80 {
                return if upper { b.to_ascii_uppercase() } else { b.to_ascii_lowercase() };
            }
            let c = cp1252_char(b);
            let mut m: Vec<char> = if upper { c.to_uppercase().collect() } else { c.to_lowercase().collect() };
            match (m.len(), m.pop().and_then(cp1252_byte)) {
                (1, Some(r)) => r,
                _ => b,
            }
        })
        .collect()
}

/// `ord` returns the first byte of the UTF-8 encoding of the first character, read
/// as Windows-1252 (VERIFIED black-box, bb_lang2 case.ord_192 = 195 with
/// `string_length(chr(192))` = 1; bb_lang3 case3.ord_*: `ord(chr(127))` = 127,
/// `ord(chr(128))` = 226 (€ U+20AC), `ord(chr(159))` = 197 (Ÿ U+0178),
/// `ord(chr(160))` = 194, `ord(chr(255))` = 195).
pub(crate) fn ord_byte(s: &[u8]) -> u8 {
    match s.first() {
        None => 0,
        Some(&b) if b < 0x80 => b,
        Some(&b) => {
            let mut buf = [0u8; 4];
            cp1252_char(b).encode_utf8(&mut buf);
            buf[0]
        }
    }
}

fn pos1(i: i64) -> usize {
    if i < 1 {
        0
    } else {
        (i - 1) as usize
    }
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

fn replace(s: &[u8], sub: &[u8], new: &[u8], all: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while let Some(p) = find(s, sub, i) {
        out.extend_from_slice(&s[i..p]);
        out.extend_from_slice(new);
        i = p + sub.len();
        if !all {
            break;
        }
    }
    out.extend_from_slice(&s[i..]);
    out
}

fn filter(a: &Args, keep: fn(u8) -> bool) -> NResult {
    Ok(Value::from(
        a.str(0)?
            .iter()
            .copied()
            .filter(|&b| keep(b))
            .collect::<Vec<u8>>(),
    ))
}

pub(super) fn register(t: &mut Vec<(&'static str, NativeFn)>) {
    t.extend([
        n("real", |_, _, _, a| match a.val(0) {
            Value::Real(x) => Ok(Value::Real(*x)),
            Value::Str(s) => match real_strict(s) {
                Some(v) => Ok(Value::Real(v)),
                None => Err(a.invalid()),
            },
        }),
        n("string", |_, _, _, a| {
            Ok(Value::Str(a.val(0).to_gml_string()))
        }),
        n("string_format", |_, _, _, a| {
            // VERIFIED black-box (bb_lang strfmt.string_arg): a string is returned
            // unchanged.
            if let Value::Str(s) = a.val(0) {
                return Ok(Value::Str(s.clone()));
            }
            let (v, total, dec) = (a.real(0)?, a.int(1)?, a.int(2)?);
            Ok(Value::from(string_format(v, total, dec)))
        }),
        n("chr", |_, _, _, a| {
            Ok(Value::bytes(&[a.int(0)?.rem_euclid(256) as u8]))
        }),
        n("ansi_char", |_, _, _, a| {
            Ok(Value::bytes(&[a.int(0)?.rem_euclid(256) as u8]))
        }),
        n("ord", |_, _, _, a| {
            Ok(Value::Real(ord_byte(a.str(0)?) as f64))
        }),
        n("string_length", |_, _, _, a| {
            Ok(Value::Real(a.str(0)?.len() as f64))
        }),
        n("string_pos", |_, _, _, a| {
            let (sub, s) = (a.str(0)?, a.str(1)?);
            Ok(Value::Real(
                find(s, sub, 0).map(|p| p + 1).unwrap_or(0) as f64
            ))
        }),
        n("string_copy", |_, _, _, a| {
            let (s, i, c) = (a.str(0)?, a.int(1)?, a.int(2)?);
            let start = pos1(i).min(s.len());
            let end = if c <= 0 {
                start
            } else {
                start.saturating_add(c as usize).min(s.len())
            };
            Ok(Value::bytes(&s[start..end]))
        }),
        n("string_char_at", |_, _, _, a| {
            let (s, i) = (a.str(0)?, a.int(1)?);
            if i < 1 || i as usize > s.len() {
                return Ok(Value::from(""));
            }
            Ok(Value::bytes(&s[i as usize - 1..i as usize]))
        }),
        n("string_delete", |_, _, _, a| {
            let (s, i, c) = (a.str(0)?, a.int(1)?, a.int(2)?);
            // VERIFIED black-box (bb_lang sf.delete_0_1, sf.delete_m1_2): an index
            // below 1 deletes nothing.
            if i < 1 {
                return Ok(Value::bytes(s));
            }
            let start = pos1(i).min(s.len());
            let end = if c <= 0 {
                start
            } else {
                start.saturating_add(c as usize).min(s.len())
            };
            let mut v = s[..start].to_vec();
            v.extend_from_slice(&s[end..]);
            Ok(Value::from(v))
        }),
        n("string_insert", |_, _, _, a| {
            let (sub, s, i) = (a.str(0)?, a.str(1)?, a.int(2)?);
            let at = pos1(i).min(s.len());
            let mut v = s[..at].to_vec();
            v.extend_from_slice(sub);
            v.extend_from_slice(&s[at..]);
            Ok(Value::from(v))
        }),
        n("string_lower", |_, _, _, a| {
            Ok(Value::from(case_map(a.str(0)?, false)))
        }),
        n("string_upper", |_, _, _, a| {
            Ok(Value::from(case_map(a.str(0)?, true)))
        }),
        n("string_repeat", |_, _, _, a| {
            let (s, c) = (a.str(0)?, a.int(1)?);
            if c <= 0 || s.is_empty() {
                return Ok(Value::from(""));
            }
            if (c as u128) * (s.len() as u128) > (1u128 << 31) {
                return Err(a.invalid());
            }
            Ok(Value::from(s.repeat(c as usize)))
        }),
        n("string_letters", |_, _, _, a| {
            filter(&a, |b| b.is_ascii_alphabetic())
        }),
        n("string_digits", |_, _, _, a| {
            filter(&a, |b| b.is_ascii_digit())
        }),
        n("string_lettersdigits", |_, _, _, a| {
            filter(&a, |b| b.is_ascii_alphanumeric())
        }),
        n("string_replace", |_, _, _, a| {
            Ok(Value::from(replace(a.str(0)?, a.str(1)?, a.str(2)?, false)))
        }),
        n("string_replace_all", |_, _, _, a| {
            Ok(Value::from(replace(a.str(0)?, a.str(1)?, a.str(2)?, true)))
        }),
        n("string_count", |_, _, _, a| {
            let (sub, s) = (a.str(0)?, a.str(1)?);
            let mut k = 0;
            let mut i = 0;
            while let Some(p) = find(s, sub, i) {
                k += 1;
                i = p + sub.len();
            }
            Ok(Value::Real(k as f64))
        }),
    ]);
}

#[cfg(test)]
mod tests {
    use super::{parse_real, real_strict};

    #[test]
    fn real_function() {
        // VERIFIED black-box (bb_lang real.*)
        assert_eq!(real_strict(b" 12"), Some(12.0));
        assert_eq!(real_strict(b"12 "), Some(12.0));
        assert_eq!(real_strict(b"1e3"), Some(1000.0));
        assert_eq!(real_strict(b"-.5"), Some(-0.5));
        assert_eq!(real_strict(b"+3"), Some(3.0));
        assert_eq!(real_strict(b""), Some(0.0));
        assert_eq!(real_strict(b"."), Some(0.0));
        for bad in [&b"abc"[..], b"12abc", b"0x10", b"$10", b"12.5.6", b"1,5", b"inf", b"-"] {
            assert_eq!(real_strict(bad), None, "{:?}", bad);
        }
    }

    #[test]
    fn real_parsing() {
        assert_eq!(parse_real(b"12"), 12.0);
        assert_eq!(parse_real(b"  -3.5abc"), -3.5);
        assert_eq!(parse_real(b"1e3"), 1000.0);
        assert_eq!(parse_real(b"2.5E-1"), 0.25);
        assert_eq!(parse_real(b"5."), 5.0);
        assert_eq!(parse_real(b".5"), 0.5);
        assert_eq!(parse_real(b"abc"), 0.0);
        assert_eq!(parse_real(b"-"), 0.0);
        assert_eq!(parse_real(b"7e"), 7.0);
        assert_eq!(parse_real(b"+4"), 4.0);
    }
}
