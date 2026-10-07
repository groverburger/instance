//! String functions (chapter 402, "string" section). Strings are byte strings;
//! positions are 1-based byte positions.

use super::{n, Args, NResult, NativeFn};
use crate::value::{format_fixed, Value};

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

/// Upper/lower-case mapping for ASCII and the Latin-1 letter range.
fn to_upper(b: u8) -> u8 {
    match b {
        b'a'..=b'z' => b - 32,
        0xE0..=0xFE if b != 0xF7 => b - 32,
        _ => b,
    }
}
fn to_lower(b: u8) -> u8 {
    match b {
        b'A'..=b'Z' => b + 32,
        0xC0..=0xDE if b != 0xD7 => b + 32,
        _ => b,
    }
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
            Value::Str(s) => Ok(Value::Real(parse_real(s))),
        }),
        n("string", |_, _, _, a| {
            Ok(Value::Str(a.val(0).to_gml_string()))
        }),
        n("string_format", |_, _, _, a| {
            let (v, total, dec) = (a.real(0)?, a.int(1)?, a.int(2)?);
            let s = format_fixed(v, dec.clamp(0, 18) as usize);
            let total = total.clamp(0, 1 << 20) as usize;
            let pad = total.saturating_sub(s.len());
            Ok(Value::from(" ".repeat(pad) + &s))
        }),
        n("chr", |_, _, _, a| {
            Ok(Value::bytes(&[a.int(0)?.rem_euclid(256) as u8]))
        }),
        n("ansi_char", |_, _, _, a| {
            Ok(Value::bytes(&[a.int(0)?.rem_euclid(256) as u8]))
        }),
        n("ord", |_, _, _, a| {
            Ok(Value::Real(a.str(0)?.first().copied().unwrap_or(0) as f64))
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
            Ok(Value::from(
                a.str(0)?.iter().map(|&b| to_lower(b)).collect::<Vec<u8>>(),
            ))
        }),
        n("string_upper", |_, _, _, a| {
            Ok(Value::from(
                a.str(0)?.iter().map(|&b| to_upper(b)).collect::<Vec<u8>>(),
            ))
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
    use super::parse_real;

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
