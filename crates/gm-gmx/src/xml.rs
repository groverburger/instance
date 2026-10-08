//! Small helpers over `roxmltree` for the GMX documents.

use roxmltree::Node;

/// First child element named `name`.
pub fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

/// Child elements named `name`.
pub fn children<'a, 'i: 'a>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == name)
}

/// All child elements.
pub fn elements<'a, 'i: 'a>(n: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children().filter(|c| c.is_element())
}

/// Text content of a node (concatenated text children).
pub fn text_of(n: Node) -> String {
    n.children()
        .filter(|c| c.is_text())
        .filter_map(|c| c.text())
        .collect()
}

/// Text of child `name`, empty when missing.
pub fn text(n: Node, name: &str) -> String {
    child(n, name).map(text_of).unwrap_or_default()
}

/// Parse a GMX number: decimal (possibly fractional), `$hex`.
pub fn parse_num(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix('$').or_else(|| s.strip_prefix("0x")) {
        return u64::from_str_radix(h, 16).ok().map(|v| v as f64);
    }
    s.parse::<f64>().ok()
}

/// Integer value of child `name` (`def` when missing/invalid). Fractions truncate.
pub fn int(n: Node, name: &str, def: i32) -> i32 {
    parse_num(&text(n, name)).map(|v| v as i64 as i32).unwrap_or(def)
}

pub fn float(n: Node, name: &str, def: f64) -> f64 {
    parse_num(&text(n, name)).unwrap_or(def)
}

/// Parse a GMX boolean: `-1`/`1`/`true`/`True` are true; `0`/`false` false.
pub fn parse_bool(s: &str) -> Option<bool> {
    let s = s.trim();
    match s.to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => parse_num(s).map(|v| v != 0.0),
    }
}

pub fn boolean(n: Node, name: &str, def: bool) -> bool {
    parse_bool(&text(n, name)).unwrap_or(def)
}

/// Attribute helpers.
pub fn attr<'a>(n: Node<'a, '_>, name: &str) -> &'a str {
    n.attribute(name).unwrap_or("")
}

pub fn attr_int(n: Node, name: &str, def: i32) -> i32 {
    parse_num(attr(n, name)).map(|v| v as i64 as i32).unwrap_or(def)
}

pub fn attr_float(n: Node, name: &str, def: f64) -> f64 {
    parse_num(attr(n, name)).unwrap_or(def)
}

pub fn attr_bool(n: Node, name: &str, def: bool) -> bool {
    parse_bool(attr(n, name)).unwrap_or(def)
}

/// Whether a resource-name field means "none".
pub fn is_none_name(s: &str) -> bool {
    let s = s.trim();
    s.is_empty() || s == "<undefined>" || s == "&lt;undefined&gt;" || s == "<none>"
}

/// Parse an XML document; GMX files start with a comment, which is fine.
pub fn parse(src: &str) -> Result<roxmltree::Document<'_>, String> {
    let opts = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    roxmltree::Document::parse_with_options(src, opts).map_err(|e| e.to_string())
}
