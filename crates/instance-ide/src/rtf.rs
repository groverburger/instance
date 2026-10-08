//! Minimal RTF handling for the Game Information editor (specs/ide.md §16, P0 stopgap):
//! the document is edited as plain text; the original RTF header (font/colour tables
//! and the leading formatting words) is kept when the text is written back.

/// Destinations whose content is not document text.
const SKIP_DESTINATIONS: [&str; 9] = [
    "fonttbl",
    "colortbl",
    "stylesheet",
    "info",
    "pict",
    "header",
    "footer",
    "generator",
    "listtable",
];

fn cp1252_char(b: u8) -> char {
    instance_format::text::decode_cp1252(&[b])
        .chars()
        .next()
        .unwrap_or('?')
}

/// Extract the plain text of an RTF document (`\par` → newline).
pub fn rtf_to_plain(rtf: &str) -> String {
    if !rtf.trim_start().starts_with("{\\rtf") {
        return rtf.to_string();
    }
    let b: Vec<char> = rtf.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    // Stack of "skip" flags per group.
    let mut skip_stack: Vec<bool> = vec![false];
    let mut uc_skip = 0usize;
    while i < b.len() {
        let skipping = *skip_stack.last().unwrap_or(&false);
        let c = b[i];
        match c {
            '{' => {
                skip_stack.push(skipping);
                i += 1;
            }
            '}' => {
                skip_stack.pop();
                if skip_stack.is_empty() {
                    break;
                }
                i += 1;
            }
            '\\' => {
                i += 1;
                if i >= b.len() {
                    break;
                }
                let d = b[i];
                if d.is_ascii_alphabetic() {
                    let start = i;
                    while i < b.len() && b[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word: String = b[start..i].iter().collect();
                    let nstart = i;
                    if i < b.len() && (b[i] == '-' || b[i].is_ascii_digit()) {
                        i += 1;
                        while i < b.len() && b[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                    let num: Option<i32> = b[nstart..i].iter().collect::<String>().parse().ok();
                    if i < b.len() && b[i] == ' ' {
                        i += 1;
                    }
                    if SKIP_DESTINATIONS.contains(&word.as_str()) {
                        if let Some(s) = skip_stack.last_mut() {
                            *s = true;
                        }
                        continue;
                    }
                    if skipping {
                        continue;
                    }
                    match word.as_str() {
                        "par" | "line" => out.push('\n'),
                        "tab" => out.push('\t'),
                        "u" => {
                            if let Some(n) = num {
                                let cp = if n < 0 { n + 65536 } else { n } as u32;
                                out.push(char::from_u32(cp).unwrap_or('?'));
                                uc_skip = 1;
                            }
                        }
                        _ => {}
                    }
                } else if d == '\'' {
                    let hex: String = b
                        .get(i + 1..i + 3)
                        .map(|s| s.iter().collect())
                        .unwrap_or_default();
                    i += 3;
                    if skipping {
                        continue;
                    }
                    if uc_skip > 0 {
                        uc_skip -= 1;
                        continue;
                    }
                    if let Ok(v) = u8::from_str_radix(&hex, 16) {
                        out.push(cp1252_char(v));
                    }
                } else if d == '*' {
                    if let Some(s) = skip_stack.last_mut() {
                        *s = true;
                    }
                    i += 1;
                } else {
                    // Control symbol: \\, \{, \}, \~ etc.
                    i += 1;
                    if skipping {
                        continue;
                    }
                    match d {
                        '\\' | '{' | '}' => out.push(d),
                        '~' => out.push('\u{a0}'),
                        '\n' | '\r' => out.push('\n'),
                        _ => {}
                    }
                }
            }
            '\r' | '\n' => i += 1,
            _ => {
                i += 1;
                if skipping {
                    continue;
                }
                if uc_skip > 0 {
                    uc_skip -= 1;
                    continue;
                }
                out.push(c);
            }
        }
    }
    // RTF documents usually end with a final \par that is not user text.
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Byte offset where the document body (first text or `\par`) starts: everything before
/// it is header (font table, colour table, initial formatting).
fn body_start(rtf: &str) -> usize {
    let b = rtf.as_bytes();
    let mut i = 0;
    let mut depth = 0i32;
    let mut skip_depth: Option<i32> = None;
    while i < b.len() {
        match b[i] {
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                if skip_depth == Some(depth) {
                    skip_depth = None;
                }
                depth -= 1;
                i += 1;
            }
            b'\\' => {
                let start = i;
                i += 1;
                if i < b.len() && b[i].is_ascii_alphabetic() {
                    let ws = i;
                    while i < b.len() && b[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word = &rtf[ws..i];
                    while i < b.len() && (b[i] == b'-' || b[i].is_ascii_digit()) {
                        i += 1;
                    }
                    if i < b.len() && b[i] == b' ' {
                        i += 1;
                    }
                    if skip_depth.is_none() {
                        if SKIP_DESTINATIONS.contains(&word) {
                            skip_depth = Some(depth);
                        } else if depth == 1 && matches!(word, "par" | "line" | "tab" | "u") {
                            return start;
                        }
                    }
                } else if i < b.len() && b[i] == b'*' {
                    if skip_depth.is_none() {
                        skip_depth = Some(depth);
                    }
                    i += 1;
                } else {
                    if skip_depth.is_none() && depth == 1 {
                        return start;
                    }
                    i += 1;
                }
            }
            b'\r' | b'\n' => i += 1,
            _ => {
                if skip_depth.is_none() && depth == 1 {
                    return i;
                }
                i += 1;
            }
        }
    }
    rtf.len()
}

const DEFAULT_HEADER: &str =
    "{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl{\\f0\\fnil Arial;}}\r\n\\viewkind4\\uc1\\pard\\f0\\fs24 ";

/// Escape plain text as an RTF body.
fn escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\n' => out.push_str("\\par\r\n"),
            '\r' => {}
            '\t' => out.push_str("\\tab "),
            c if (c as u32) < 0x80 => out.push(c),
            c => {
                let enc = instance_format::text::encode_cp1252(&c.to_string());
                if enc.len() == 1 && enc[0] != b'?' {
                    out.push_str(&format!("\\'{:02x}", enc[0]));
                } else {
                    let v = c as u32;
                    if v <= 0xFFFF {
                        out.push_str(&format!("\\u{}?", v as u16 as i16));
                    } else {
                        out.push('?');
                    }
                }
            }
        }
    }
    out
}

/// Replace the text of `original` by `text`, keeping the original header.
/// Returns `original` unchanged when the text is unchanged.
pub fn plain_to_rtf(original: &str, text: &str) -> String {
    if rtf_to_plain(original) == text {
        return original.to_string();
    }
    let header = if original.trim_start().starts_with("{\\rtf") {
        &original[..body_start(original)]
    } else {
        DEFAULT_HEADER
    };
    format!("{header}{}\\par\r\n}}\r\n", escape(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033{\\fonttbl{\\f0\\fnil Arial;}{\\f1\\fnil\\fcharset0 Arial;}}\r\n{\\colortbl ;\\red0\\green0\\blue128;}\r\n{\\*\\generator Msftedit 5.41.15.1515;}\\viewkind4\\uc1\\pard\\cf1\\b\\f0\\fs24 Treasure\\b0\\par\r\nCaf\\'e9 \\{x\\}\\par\r\n}\r\n";

    #[test]
    fn extract() {
        assert_eq!(rtf_to_plain(DOC), "Treasure\nCafé {x}");
        assert_eq!(rtf_to_plain(""), "");
    }

    #[test]
    fn untouched_is_identical() {
        let t = rtf_to_plain(DOC);
        assert_eq!(plain_to_rtf(DOC, &t), DOC);
    }

    #[test]
    fn edit_keeps_header() {
        let new = plain_to_rtf(DOC, "Hello\nWörld \\ {");
        assert!(new.starts_with("{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033{\\fonttbl"));
        assert!(new.contains("{\\colortbl ;\\red0\\green0\\blue128;}"));
        assert_eq!(rtf_to_plain(&new), "Hello\nWörld \\ {");
        let fresh = plain_to_rtf("", "abc");
        assert_eq!(rtf_to_plain(&fresh), "abc");
    }
}
