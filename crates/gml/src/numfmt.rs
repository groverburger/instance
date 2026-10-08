//! GM 8.1 number formatting (`string()`, `string_format`): an emulation of the
//! Delphi fixed-point formatter it uses, including its double-precision digit
//! generation. VERIFIED black-box (bb_lang str.*, strfmt.*, bit.*, syn.hex_12_digits,
//! div0.tiny); see specs/gml-language.md §4.7.

/// `a * 10^k` (or `a / 10^-k`), rounded to double after each operation as GM 8.1's
/// FPU (53-bit precision) does.
fn scale10(a: f64, k: i32) -> f64 {
    let mut x = a;
    let mut k = k;
    if k >= 0 {
        while k > 22 {
            x *= 1e22;
            k -= 22;
        }
        x * 10f64.powi(k)
    } else {
        k = -k;
        while k > 22 {
            x /= 1e22;
            k -= 22;
        }
        x / 10f64.powi(k)
    }
}

/// The 18 significant decimal digits of `a > 0` and its decimal exponent `e`
/// (`a ≈ 0.d1d2…d18 × 10^e`). GM 8.1 scales the value by a power of ten in double
/// precision and converts the result to an 18-digit integer (round half even), so
/// the last digits show double rounding (2^63 → 922337203685477632).
fn digits18(a: f64) -> ([u8; 18], i32) {
    let mut e = a.log10().floor() as i32 + 1;
    let mut n: u64 = 0;
    for _ in 0..6 {
        let r = scale10(a, 18 - e).round_ties_even();
        if r >= 1e18 {
            e += 1;
            continue;
        }
        if r < 1e17 {
            e -= 1;
            continue;
        }
        n = r as u64;
        break;
    }
    let mut d = [0u8; 18];
    for i in (0..18).rev() {
        d[i] = (n % 10) as u8;
        n /= 10;
    }
    (d, e)
}

/// Rounds the 18 digits to `n` significant digits (half up on the decimal digits).
/// Returns the digits (empty = zero) and the possibly increased exponent.
fn round_digits(d: &[u8; 18], e: i32, n: i32) -> (Vec<u8>, i32) {
    if n < 0 {
        return (Vec::new(), e);
    }
    let n = (n as usize).min(18);
    let mut v: Vec<u8> = d[..n].to_vec();
    let mut e = e;
    if n < 18 && d[n] >= 5 {
        let mut i = n;
        loop {
            if i == 0 {
                // carry out of the first digit: 99.6 -> 100
                v = vec![0; n.max(1)];
                v[0] = 1;
                e += 1;
                break;
            }
            i -= 1;
            if v[i] == 9 {
                v[i] = 0;
            } else {
                v[i] += 1;
                break;
            }
        }
    }
    if v.iter().all(|&x| x == 0) {
        v.clear();
    }
    (v, e)
}

/// Scientific form with `p` significant digits, trailing zeros removed and the
/// exponent without `+` or leading zeros: `1.23456789E11`, `1E20`.
fn general_form(neg: bool, d: &[u8; 18], e: i32, p: i32) -> String {
    let (mut v, e) = round_digits(d, e, p);
    while v.last() == Some(&0) {
        v.pop();
    }
    if v.is_empty() {
        return "0".into();
    }
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    s.push((b'0' + v[0]) as char);
    if v.len() > 1 {
        s.push('.');
        for &x in &v[1..] {
            s.push((b'0' + x) as char);
        }
    }
    s.push('E');
    s.push_str(&(e - 1).to_string());
    s
}

/// Fixed-point formatting: at most `p` significant digits, rounded half up (on the
/// decimal digits) to `round_dec` decimals (negative: to tens, hundreds, ...),
/// printed with `disp_dec` decimals (zero padded). A value with more than `p`
/// integer digits is printed in scientific form. A result whose digits are all
/// zero has no minus sign.
pub fn format_gm(v: f64, p: i32, disp_dec: usize, round_dec: i32) -> String {
    if v.is_nan() {
        return "NAN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "INF".into() } else { "-INF".into() };
    }
    let neg = v < 0.0;
    let (sig, e) = if v == 0.0 {
        (Vec::new(), 1)
    } else {
        let (d, e) = digits18(v.abs());
        if e > p {
            return general_form(neg, &d, e, p);
        }
        round_digits(&d, e, p.min(e.saturating_add(round_dec)))
    };
    let digit = |i: i32| -> u8 {
        if i < 0 {
            0
        } else {
            sig.get(i as usize).copied().unwrap_or(0)
        }
    };
    let mut s = String::new();
    if sig.is_empty() || e <= 0 {
        s.push('0');
    } else {
        for i in 0..e {
            s.push((b'0' + digit(i)) as char);
        }
    }
    if disp_dec > 0 {
        s.push('.');
        for j in 1..=disp_dec as i32 {
            s.push((b'0' + digit(e - 1 + j)) as char);
        }
    }
    if neg && s.bytes().any(|b| (b'1'..=b'9').contains(&b)) {
        s.insert(0, '-');
    }
    s
}

/// `string(v)` for a real: integral values as integers ("ERROR" outside the 64-bit
/// integer range), other values with two decimals and at most 10 significant
/// digits (`4294967296.5` → `"4294967297.00"`, `123456789012.5` →
/// `"1.23456789E11"`).
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
        if v.abs() >= 9_223_372_036_854_775_808.0 {
            return "ERROR".into();
        }
        return format!("{}", v as i64);
    }
    format_gm(v, 10, 2, 2)
}

/// `string_format(v, tot, dec)`: up to 18 significant digits; `dec` outside
/// 0..=18 displays 18 decimals but rounds with the signed `dec`
/// (`string_format(1.5, 0, -1)` = `"0.000000000000000000"`); padded with spaces on
/// the left to `tot` characters.
pub fn string_format(v: f64, tot: i64, dec: i64) -> String {
    let disp = if (0..=18).contains(&dec) { dec as usize } else { 18 };
    let rd = dec.clamp(-400, 400) as i32;
    let s = format_gm(v, 18, disp, rd);
    let tot = tot.clamp(0, 10_000) as usize;
    if s.len() < tot {
        format!("{}{}", " ".repeat(tot - s.len()), s)
    } else {
        s
    }
}

/// `string_format`-style fixed formatting with `dec` decimals (0..=18).
pub fn format_fixed(v: f64, dec: usize) -> String {
    let dec = dec.min(18);
    format_gm(v, 18, dec, dec as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GM's literal parsing (lexer): 1.23456 is one ulp below the nearest double.
    fn lit(s: &str) -> f64 {
        crate::lexer::tokenize(s.as_bytes())
            .map(|t| match t[0].tok {
                crate::lexer::Tok::Real(v) => v,
                _ => panic!(),
            })
            .unwrap()
    }

    #[test]
    fn string_of_reals() {
        // VERIFIED black-box (bb_lang str.*)
        for (src, want) in [
            ("0.5", "0.50"),
            ("1.005", "1.00"),
            ("2.675", "2.67"),
            ("0.125", "0.13"),
            ("0.015", "0.02"),
            ("0.995", "1.00"),
            ("0.005", "0.01"),
            ("9.999", "10.00"),
            ("12345.678", "12345.68"),
            ("4294967296.5", "4294967297.00"),
            ("123456789012.5", "1.23456789E11"),
            ("0.001", "0.00"),
        ] {
            assert_eq!(real_to_string(lit(src)), want, "{src}");
        }
        assert_eq!(real_to_string(-lit("0.005")), "-0.01");
        assert_eq!(real_to_string(-lit("0.004")), "0.00");
        assert_eq!(real_to_string(1e20), "ERROR");
        assert_eq!(real_to_string(1234567890123.0), "1234567890123");
        assert_eq!(real_to_string(-0.0), "0");
    }

    #[test]
    fn string_format_cases() {
        // VERIFIED black-box (bb_lang strfmt.*, bit.*, div0.tiny, syn.hex_12_digits)
        assert_eq!(string_format(2.5, 0, 0), "3");
        assert_eq!(string_format(-1.5, 0, 0), "-2");
        assert_eq!(string_format(1.0 / 3.0, 8, 3), "   0.333");
        assert_eq!(string_format(123456.0, 3, 2), "123456.00");
        assert_eq!(string_format(1e20, 0, 2), "1E20");
        assert_eq!(string_format(1.5, 0, -1), "0.000000000000000000");
        assert_eq!(string_format(lit("1.23456"), 0, 20), "1.234559999999999840");
        assert_eq!(string_format(1.5, -5, 1), "1.5");
        assert_eq!(string_format(9223372036854775808.0, 0, 10), "9.22337203685477632E18");
        assert_eq!(string_format(-9223372036854775808.0, 0, 10), "-9.22337203685477632E18");
        assert_eq!(string_format(281474976710655.0, 0, 10), "281474976710655.0080000000");
        assert_eq!(string_format(1.0 / lit("0.0000001"), 0, 10), "9999999.9999999987");
        assert_eq!(string_format(-0.001, 0, 2), "0.00");
    }
}
