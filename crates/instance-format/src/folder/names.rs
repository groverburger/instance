//! File names: sanitising resource names, event and moment file stems.

use std::collections::HashSet;

use crate::model::event_type as et;

/// Whether `s` can be used verbatim inside an event file name (`collision_<s>`):
/// identifier characters only.
pub fn ident_safe(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A file-system-safe version of `name` (Windows rules, which are the strictest).
pub fn sanitize(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '~' => '_',
            c if (c as u32) < 0x20 || (0x7F..=0x9F).contains(&(c as u32)) => '_',
            c => c,
        })
        .take(100)
        .collect();
    while s.ends_with('.') || s.ends_with(' ') {
        s.pop();
        s.push('_');
        if s.chars().all(|c| c == '_') {
            break;
        }
    }
    if s.starts_with(' ') || s.starts_with('.') {
        s.insert(0, '_');
    }
    if s.is_empty() {
        s.push('_');
    }
    let base = s.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((base.starts_with("COM") || base.starts_with("LPT"))
            && base.len() == 4
            && base.as_bytes()[3].is_ascii_digit());
    if reserved {
        s.insert(0, '_');
    }
    s
}

/// Hands out unique names inside one directory (case-insensitively, as on Windows).
#[derive(Default)]
pub struct Names {
    used: HashSet<String>,
}

impl Names {
    /// Reserve a fixed name (e.g. `sprite.toml`).
    pub fn reserve(&mut self, name: &str) {
        self.used.insert(name.to_lowercase());
    }

    /// A stem such that `stem + ext` is free for every extension in `exts`; the
    /// names are reserved. Collisions get a `~2`, `~3`, ... suffix.
    pub fn stem(&mut self, base: &str, exts: &[&str]) -> String {
        let mut n = 1;
        loop {
            let stem = if n == 1 {
                base.to_string()
            } else {
                format!("{base}~{n}")
            };
            if exts
                .iter()
                .all(|e| !self.used.contains(&format!("{stem}{e}").to_lowercase()))
            {
                for e in exts {
                    self.used.insert(format!("{stem}{e}").to_lowercase());
                }
                return stem;
            }
            n += 1;
        }
    }
}

/// Key names used in keyboard event file names (`vk_*` constants).
const KEYS: &[(i32, &str)] = &[
    (0, "vk_nokey"),
    (1, "vk_anykey"),
    (8, "vk_backspace"),
    (9, "vk_tab"),
    (13, "vk_enter"),
    (16, "vk_shift"),
    (17, "vk_control"),
    (18, "vk_alt"),
    (19, "vk_pause"),
    (27, "vk_escape"),
    (32, "vk_space"),
    (33, "vk_pageup"),
    (34, "vk_pagedown"),
    (35, "vk_end"),
    (36, "vk_home"),
    (37, "vk_left"),
    (38, "vk_up"),
    (39, "vk_right"),
    (40, "vk_down"),
    (44, "vk_printscreen"),
    (45, "vk_insert"),
    (46, "vk_delete"),
    (106, "vk_multiply"),
    (107, "vk_add"),
    (109, "vk_subtract"),
    (110, "vk_decimal"),
    (111, "vk_divide"),
    (160, "vk_lshift"),
    (161, "vk_rshift"),
    (162, "vk_lcontrol"),
    (163, "vk_rcontrol"),
    (164, "vk_lalt"),
    (165, "vk_ralt"),
];

fn key_name(k: i32) -> String {
    if let Some((_, n)) = KEYS.iter().find(|(c, _)| *c == k) {
        return n.to_string();
    }
    match k {
        48..=57 | 65..=90 => ((k as u8) as char).to_string(),
        96..=105 => format!("vk_numpad{}", k - 96),
        112..=123 => format!("vk_f{}", k - 111),
        _ => format!("#{k}"),
    }
}

fn parse_key(s: &str) -> Option<i32> {
    if let Some(n) = s.strip_prefix('#') {
        return n.parse().ok();
    }
    let l = s.to_ascii_lowercase();
    if let Some((c, _)) = KEYS.iter().find(|(_, n)| *n == l) {
        return Some(*c);
    }
    if s.len() == 1 {
        let b = s.as_bytes()[0].to_ascii_uppercase();
        if b.is_ascii_digit() || b.is_ascii_uppercase() {
            return Some(b as i32);
        }
    }
    if let Some(n) = l.strip_prefix("vk_numpad") {
        return n
            .parse::<i32>()
            .ok()
            .filter(|n| (0..=9).contains(n))
            .map(|n| n + 96);
    }
    if let Some(n) = l.strip_prefix("vk_f") {
        return n
            .parse::<i32>()
            .ok()
            .filter(|n| (1..=12).contains(n))
            .map(|n| n + 111);
    }
    None
}

fn mouse_name(n: i32) -> String {
    let b = ["left", "right", "middle"];
    let dir = ["left", "right", "up", "down"];
    match n {
        0..=2 => format!("{}_button", b[n as usize]),
        3 => "no_button".into(),
        4..=6 => format!("{}_pressed", b[(n - 4) as usize]),
        7..=9 => format!("{}_released", b[(n - 7) as usize]),
        10 => "enter".into(),
        11 => "leave".into(),
        16..=19 => format!("joystick1_{}", dir[(n - 16) as usize]),
        21..=28 => format!("joystick1_button{}", n - 20),
        31..=34 => format!("joystick2_{}", dir[(n - 31) as usize]),
        36..=43 => format!("joystick2_button{}", n - 35),
        50..=52 => format!("global_{}_button", b[(n - 50) as usize]),
        53..=55 => format!("global_{}_pressed", b[(n - 53) as usize]),
        56..=58 => format!("global_{}_released", b[(n - 56) as usize]),
        60 => "wheel_up".into(),
        61 => "wheel_down".into(),
        _ => format!("#{n}"),
    }
}

fn other_name(n: i32) -> String {
    match n {
        0 => "outside_room".into(),
        1 => "intersect_boundary".into(),
        2 => "game_start".into(),
        3 => "game_end".into(),
        4 => "room_start".into(),
        5 => "room_end".into(),
        6 => "no_more_lives".into(),
        7 => "animation_end".into(),
        8 => "end_of_path".into(),
        9 => "no_more_health".into(),
        10..=25 => format!("user_{}", n - 10),
        30 => "close_button".into(),
        40..=47 => format!("outside_view_{}", n - 40),
        50..=57 => format!("boundary_view_{}", n - 50),
        _ => format!("#{n}"),
    }
}

fn parse_table(s: &str, f: fn(i32) -> String, range: std::ops::RangeInclusive<i32>) -> Option<i32> {
    if let Some(n) = s.strip_prefix('#') {
        return n.parse().ok();
    }
    let l = s.to_ascii_lowercase();
    range.into_iter().find(|&n| f(n) == l)
}

fn num(n: i32) -> String {
    if n >= 0 {
        n.to_string()
    } else {
        format!("#{n}")
    }
}

fn parse_num(s: &str) -> Option<i32> {
    s.strip_prefix('#').unwrap_or(s).parse().ok()
}

/// File stem of an event. `object_ref` / `trigger_ref` give the reference text for
/// collision and trigger events (an identifier-safe name or `#N`).
pub fn event_stem(
    event_type: i32,
    number: i32,
    object_ref: impl Fn(i32) -> String,
    trigger_ref: impl Fn(i32) -> String,
) -> String {
    if needs_generic(event_type, number) {
        return format!("event_{event_type}_{number}");
    }
    match (event_type, number) {
        (et::CREATE, 0) => "create".into(),
        (et::DESTROY, 0) => "destroy".into(),
        (et::ALARM, n) => format!("alarm_{}", num(n)),
        (et::STEP, 0) => "step".into(),
        (et::STEP, 1) => "step_begin".into(),
        (et::STEP, 2) => "step_end".into(),
        (et::COLLISION, n) => format!("collision_{}", object_ref(n)),
        (et::KEYBOARD, n) => format!("keyboard_{}", key_name(n)),
        (et::MOUSE, n) => format!("mouse_{}", mouse_name(n)),
        (et::OTHER, n) => format!("other_{}", other_name(n)),
        (et::DRAW, 0) => "draw".into(),
        (et::KEY_PRESS, n) => format!("keypress_{}", key_name(n)),
        (et::KEY_RELEASE, n) => format!("keyrelease_{}", key_name(n)),
        (et::TRIGGER, n) => format!("trigger_{}", trigger_ref(n)),
        (t, n) => format!("event_{t}_{n}"),
    }
}

/// What an event stem names: either a fixed `(type, number)` or a reference that
/// still needs resolving against the object or trigger names.
#[derive(Debug, PartialEq)]
pub enum EventName {
    Fixed(i32, i32),
    Collision(String),
    Trigger(String),
}

/// Parse an event stem (without any `~N` suffix and extension).
pub fn parse_event_stem(s: &str) -> Option<EventName> {
    use EventName::Fixed;
    let fixed = match s {
        "create" => Some((et::CREATE, 0)),
        "destroy" => Some((et::DESTROY, 0)),
        "step" => Some((et::STEP, 0)),
        "step_begin" => Some((et::STEP, 1)),
        "step_end" => Some((et::STEP, 2)),
        "draw" => Some((et::DRAW, 0)),
        _ => None,
    };
    if let Some((t, n)) = fixed {
        return Some(Fixed(t, n));
    }
    let (head, rest) = s.split_once('_')?;
    match head {
        "alarm" => parse_num(rest).map(|n| Fixed(et::ALARM, n)),
        "collision" if !rest.is_empty() => Some(EventName::Collision(rest.into())),
        "trigger" if !rest.is_empty() => Some(EventName::Trigger(rest.into())),
        "keyboard" => parse_key(rest).map(|n| Fixed(et::KEYBOARD, n)),
        "keypress" => parse_key(rest).map(|n| Fixed(et::KEY_PRESS, n)),
        "keyrelease" => parse_key(rest).map(|n| Fixed(et::KEY_RELEASE, n)),
        "mouse" => parse_table(rest, mouse_name, 0..=61).map(|n| Fixed(et::MOUSE, n)),
        "other" => parse_table(rest, other_name, 0..=57).map(|n| Fixed(et::OTHER, n)),
        "event" => {
            let (t, n) = rest.split_once('_')?;
            Some(Fixed(t.parse().ok()?, n.parse().ok()?))
        }
        _ => None,
    }
}

/// Events whose number is not covered by the regular names (e.g. `draw` with
/// number 3) use the generic `event_<type>_<number>` form.
fn needs_generic(event_type: i32, number: i32) -> bool {
    matches!(
        (event_type, number),
        (et::CREATE | et::DESTROY | et::DRAW, n) if n != 0
    ) || (event_type == et::STEP && !(0..=2).contains(&number))
}

/// Time-line moment file stem.
pub fn moment_stem(step: i32) -> String {
    format!("step_{}", num(step))
}

/// Parse a moment stem.
pub fn parse_moment_stem(s: &str) -> Option<i32> {
    parse_num(s.strip_prefix("step_")?)
}

/// Split a `name~N` suffix off a stem.
pub fn strip_dup_suffix(s: &str) -> &str {
    match s.rsplit_once('~') {
        Some((a, b)) if !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit()) => a,
        _ => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_names_round_trip() {
        let obj = |n: i32| format!("#{n}");
        for t in 0..=13 {
            for n in [
                -3, 0, 1, 2, 5, 13, 32, 37, 48, 65, 90, 100, 113, 160, 255, 1000,
            ] {
                let stem = event_stem(t, n, obj, obj);
                let parsed = parse_event_stem(&stem).unwrap_or_else(|| panic!("{stem}"));
                let ok = match parsed {
                    EventName::Fixed(a, b) => (a, b) == (t, n),
                    EventName::Collision(r) => t == et::COLLISION && r == obj(n),
                    EventName::Trigger(r) => t == et::TRIGGER && r == obj(n),
                };
                assert!(ok, "{t} {n} -> {stem}");
            }
        }
        assert_eq!(event_stem(5, 37, obj, obj), "keyboard_vk_left");
        assert_eq!(event_stem(9, 65, obj, obj), "keypress_A");
        assert_eq!(event_stem(7, 14, obj, obj), "other_user_4");
    }

    #[test]
    fn sanitize_names() {
        assert_eq!(sanitize("spr_a"), "spr_a");
        assert_eq!(sanitize("a/b:c"), "a_b_c");
        assert_eq!(sanitize(""), "_");
        assert_eq!(sanitize("con"), "_con");
        assert_eq!(sanitize("x."), "x_");
        let mut n = Names::default();
        assert_eq!(n.stem("a", &[".toml", ".gml"]), "a");
        assert_eq!(n.stem("A", &[".gml"]), "A~2");
        assert_eq!(strip_dup_suffix("step~2"), "step");
    }
}
