//! bb_constants: numeric value of every built-in constant in builtins.json.

use crate::builder::Gen;
use crate::framework::*;

/// (name, spec value as text) of every constant in specs/builtins.json.
pub fn spec_constants() -> Vec<(String, String)> {
    let text = include_str!("../../../../specs/builtins.json");
    let v: serde_json::Value = serde_json::from_str(text).expect("builtins.json");
    let mut out = Vec::new();
    for e in v["entries"].as_array().unwrap() {
        if e["kind"] == "constant" {
            let name = e["name"].as_str().unwrap().to_string();
            let val = match &e["value"] {
                serde_json::Value::Null => "null".to_string(),
                x => x.to_string(),
            };
            out.push((name, val));
        }
    }
    out
}

fn family(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.split('_').next() {
        Some(p) if lower.contains('_') => p.to_string(),
        _ => "misc".into(),
    }
}

pub fn build() -> Spec {
    let mut g = Gen::new("bb_constants", 81001);
    let fw = install(&mut g);
    let consts = spec_constants();
    // Group by family, chunks of 8 per test so an unknown constant (runtime error
    // "unknown variable") only loses its own chunk.
    let mut tests = Vec::new();
    let mut i = 0;
    while i < consts.len() {
        let fam = family(&consts[i].0);
        let mut j = i;
        while j < consts.len() && j - i < 8 && family(&consts[j].0) == fam {
            j += 1;
        }
        let items: Vec<(String, String)> = consts[i..j]
            .iter()
            .map(|(n, _)| (format!("c.{n}"), n.clone()))
            .collect();
        let refs: Vec<(&str, &str)> = items.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        tests.push(vals(&format!("c.group{}", tests.len()), &refs));
        i = j;
    }
    // Event sub-numbers checked through event_perform-free means: the ev_* values
    // above, plus the vk/mb families. Also a few derived values.
    tests.push(vals(
        "c.derived",
        &[
            ("c.make_color_rgb.1_2_3", "make_color_rgb(1, 2, 3)"),
            ("c.c_orange.r", "color_get_red(c_orange)"),
            ("c.c_orange.g", "color_get_green(c_orange)"),
            ("c.c_orange.b", "color_get_blue(c_orange)"),
        ],
    ));
    let frames = unit_project(&mut g, &fw, tests, 60);
    Spec {
        name: "bb_constants",
        title: "Every built-in constant's numeric value",
        lite: "Lite-safe (only reads constants; constants of Pro-only features are plain numbers). A constant unknown to GM 8.1 shows up as c.groupN.aborted or a missing key.",
        frames,
        keys: vec![
            k("c.ev_", "RT-18, FF-17, GML-19", "event type / sub-event constants"),
            k("c.vk_", "GML-19, RT-13", "virtual key codes"),
            k("c.mb_", "RT-18, GML-19", "mouse button constants"),
            k("c.self", "RT-18, GML-13", "special instance constants"),
            k("c.other", "RT-18", "special instance constants"),
            k("c.all", "RT-18", "special instance constants"),
            k("c.noone", "RT-18", "special instance constants"),
            k("c.global", "RT-18", "special instance constants"),
            k("c.local", "GML-13", "the undocumented local constant"),
            k("c.os_", "GML-19", "os_* (null in spec)"),
            k("c.device_", "GML-19", "device_* (null in spec)"),
            k("c.text_type", "GML-19", "null in spec"),
            k("c.button_type", "GML-19", "null in spec"),
            k("c.input_type", "GML-19", "null in spec"),
            k("c.", "GML-19, runtime.md §13.4 (c_*), builtins.json inferred values", "all other constants (c_, fa_, ef_, bm_, cr_, ps_, pt_, se_, pr_, ty_, dll_, charsets)"),
            k("c.group", "GML-19", "a group of constants raised an error (unknown constant)"),
        ],
        project: g.finish(),
    }
}
