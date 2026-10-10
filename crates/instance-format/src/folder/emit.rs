//! Deterministic TOML output.
//!
//! The `toml` crate is only used for parsing; output is produced here so that the
//! layout is fixed: keys in a stable order ([`KEY_ORDER`] first, then alphabetical),
//! strings always on one line with escapes (no multi-line strings, so a checkout that
//! rewrites line endings cannot change a value), floats in Rust's shortest round-trip
//! form, arrays of tables one inline table per line.

use toml::{Table, Value};

/// Keys printed first (in this order) in every table; the rest follow alphabetically.
const KEY_ORDER: &[&str] = &[
    "index",
    "name",
    "lib",
    "id",
    "object",
    "background",
    "x",
    "y",
];

/// Arrays of tables printed as `[[key]]` sections instead of inline tables.
const SECTION_ARRAYS: &[&str] = &["actions"];

fn ordered_keys(t: &Table) -> Vec<&String> {
    let mut keys: Vec<&String> = t.keys().collect();
    keys.sort_by(|a, b| {
        let pa = KEY_ORDER.iter().position(|k| k == a).unwrap_or(usize::MAX);
        let pb = KEY_ORDER.iter().position(|k| k == b).unwrap_or(usize::MAX);
        pa.cmp(&pb).then_with(|| a.cmp(b))
    });
    keys
}

/// A TOML basic string literal.
pub fn string(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\u{8}' => o.push_str("\\b"),
            '\u{c}' => o.push_str("\\f"),
            c if (c as u32) < 0x20 || (0x7F..=0x9F).contains(&(c as u32)) => {
                o.push_str(&format!("\\u{:04X}", c as u32))
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn key(k: &str) -> String {
    if !k.is_empty()
        && k.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        k.to_string()
    } else {
        string(k)
    }
}

fn float(f: f64) -> String {
    if f.is_nan() {
        "nan".into()
    } else if f.is_infinite() {
        if f > 0.0 { "inf" } else { "-inf" }.into()
    } else {
        // Debug gives the shortest representation that reads back exactly and always
        // contains a '.' or an exponent ("1.0", "1e20", "1.5e-7").
        format!("{f:?}")
    }
}

fn is_table_array(v: &Value) -> bool {
    matches!(v, Value::Array(a) if !a.is_empty() && a.iter().all(|x| x.is_table()))
}

/// A value on one line.
pub fn inline(v: &Value) -> String {
    match v {
        Value::String(s) => string(s),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => float(*f),
        Value::Boolean(b) => b.to_string(),
        Value::Datetime(d) => d.to_string(),
        Value::Array(a) => {
            let items: Vec<String> = a.iter().map(inline).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Table(t) => {
            if t.is_empty() {
                return "{}".into();
            }
            let items: Vec<String> = ordered_keys(t)
                .into_iter()
                .map(|k| format!("{} = {}", key(k), inline(&t[k])))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
    }
}

/// A value after `key = `; arrays of strings or tables get one element per line.
fn value(v: &Value) -> String {
    match v {
        Value::Array(a)
            if a.len() > 1 || (a.len() == 1 && (is_table_array(v) || a[0].is_str())) =>
        {
            if a.iter().all(|x| x.is_table() || x.is_str()) {
                let mut s = String::from("[\n");
                for x in a {
                    s.push_str("    ");
                    s.push_str(&inline(x));
                    s.push_str(",\n");
                }
                s.push(']');
                s
            } else {
                inline(v)
            }
        }
        _ => inline(v),
    }
}

/// A whole document.
pub fn document(t: &Table) -> String {
    let mut out = String::new();
    body(&mut out, t, "");
    out
}

fn body(out: &mut String, t: &Table, prefix: &str) {
    let keys = ordered_keys(t);
    let section_array =
        |k: &str, v: &Value| prefix.is_empty() && SECTION_ARRAYS.contains(&k) && is_table_array(v);
    for k in &keys {
        let v = &t[k.as_str()];
        if v.is_table() || section_array(k, v) {
            continue;
        }
        out.push_str(&format!("{} = {}\n", key(k), value(v)));
    }
    for k in &keys {
        let v = &t[k.as_str()];
        let path = if prefix.is_empty() {
            key(k)
        } else {
            format!("{prefix}.{}", key(k))
        };
        if let Value::Table(sub) = v {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("[{path}]\n"));
            body(out, sub, &path);
        } else if section_array(k, v) {
            for x in v.as_array().into_iter().flatten() {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&format!("[[{path}]]\n"));
                if let Value::Table(sub) = x {
                    body(out, sub, &path);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_values() {
        let mut t = Table::new();
        t.insert(
            "name".into(),
            Value::String("a\"b\\c\r\n\t\u{1}\u{81}é€".into()),
        );
        for (i, f) in [
            0.0,
            -0.0,
            1.0,
            1e20,
            1.5e-7,
            45678.12345678901,
            f64::MAX,
            f64::MIN_POSITIVE,
        ]
        .into_iter()
        .enumerate()
        {
            t.insert(format!("f{i}"), Value::Float(f));
        }
        t.insert("inf".into(), Value::Float(f64::NEG_INFINITY));
        t.insert("odd key".into(), Value::Integer(-5));
        let mut sub = Table::new();
        sub.insert("x".into(), Value::Boolean(true));
        t.insert(
            "arr".into(),
            Value::Array(vec![Value::Table(sub.clone()); 2]),
        );
        t.insert(
            "actions".into(),
            Value::Array(vec![Value::Table(sub.clone()); 2]),
        );
        t.insert("strs".into(), Value::Array(vec![Value::String("x".into())]));
        t.insert("sub".into(), Value::Table(sub));
        let text = document(&t);
        let back: Table = text.parse().unwrap();
        assert_eq!(back, t, "{text}");
        assert_eq!(document(&back), text);
    }
}
