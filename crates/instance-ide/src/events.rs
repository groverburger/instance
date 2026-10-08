//! Event naming and the event selector's menus (specs/ide.md §13.1-13.2,
//! numbering from specs/runtime.md §5.1).

use instance_format::{event_type as et, Project};

/// Named key codes (Windows virtual-key codes, from the `vk_*` constants).
pub const NAMED_KEYS: &[(i32, &str)] = &[
    (0, "No key"),
    (1, "Any key"),
    (37, "Left"),
    (39, "Right"),
    (38, "Up"),
    (40, "Down"),
    (17, "Ctrl"),
    (18, "Alt"),
    (16, "Shift"),
    (32, "Space"),
    (13, "Enter"),
    (27, "Escape"),
    (8, "Backspace"),
    (9, "Tab"),
    (19, "Pause"),
    (44, "Print Screen"),
    (36, "Home"),
    (35, "End"),
    (46, "Delete"),
    (45, "Insert"),
    (33, "Page Up"),
    (34, "Page Down"),
    (111, "Keypad /"),
    (106, "Keypad *"),
    (109, "Keypad -"),
    (107, "Keypad +"),
    (110, "Keypad ."),
];

/// Display name of key code `k`.
pub fn key_name(k: i32) -> String {
    if let Some((_, n)) = NAMED_KEYS.iter().find(|(c, _)| *c == k) {
        return n.to_string();
    }
    match k {
        48..=57 | 65..=90 => ((k as u8) as char).to_string(),
        96..=105 => format!("Keypad {}", k - 96),
        112..=123 => format!("F{}", k - 111),
        _ => format!("Key {k}"),
    }
}

/// Key groups for the selector menus: (submenu title, codes).
pub fn key_groups() -> Vec<(&'static str, Vec<i32>)> {
    vec![
        ("Special", vec![0, 1]),
        ("Arrows", vec![37, 39, 38, 40]),
        ("Modifiers", vec![17, 18, 16]),
        ("Common", vec![32, 13, 27, 8, 9, 19, 44]),
        ("Navigation", vec![36, 35, 46, 45, 33, 34]),
        ("Letters", (65..=90).collect()),
        ("Digits", (48..=57).collect()),
        (
            "Keypad",
            (96..=105).chain([111, 106, 109, 107, 110]).collect(),
        ),
        ("Function keys", (112..=123).collect()),
    ]
}

/// Mouse sub-events: (number, label), grouped.
pub fn mouse_events() -> Vec<(&'static str, Vec<(i32, String)>)> {
    let local = vec![
        (0, "Left button".to_string()),
        (1, "Right button".into()),
        (2, "Middle button".into()),
        (3, "No button".into()),
        (4, "Left pressed".into()),
        (5, "Right pressed".into()),
        (6, "Middle pressed".into()),
        (7, "Left released".into()),
        (8, "Right released".into()),
        (9, "Middle released".into()),
        (10, "Mouse enter".into()),
        (11, "Mouse leave".into()),
        (60, "Mouse wheel up".into()),
        (61, "Mouse wheel down".into()),
    ];
    let global = (50..=58).map(|n| (n, mouse_name(n))).collect::<Vec<_>>();
    let joy = |base: i32, bbase: i32| {
        let mut v: Vec<(i32, String)> = ["left", "right", "up", "down"]
            .iter()
            .enumerate()
            .map(|(i, d)| (base + i as i32, format!("Joystick {d}")))
            .collect();
        v.extend((0..8).map(|b| (bbase + b, format!("Button {}", b + 1))));
        v
    };
    vec![
        ("", local),
        ("Global mouse", global),
        ("Joystick 1", joy(16, 21)),
        ("Joystick 2", joy(31, 36)),
    ]
}

fn mouse_name(n: i32) -> String {
    let b = ["Left", "Right", "Middle"];
    match n {
        0..=2 => format!("{} Button", b[n as usize]),
        3 => "No Button".into(),
        4..=6 => format!("{} Pressed", b[(n - 4) as usize]),
        7..=9 => format!("{} Released", b[(n - 7) as usize]),
        10 => "Mouse Enter".into(),
        11 => "Mouse Leave".into(),
        16..=19 => format!(
            "Joystick 1 {}",
            ["Left", "Right", "Up", "Down"][(n - 16) as usize]
        ),
        21..=28 => format!("Joystick 1 Button {}", n - 20),
        31..=34 => format!(
            "Joystick 2 {}",
            ["Left", "Right", "Up", "Down"][(n - 31) as usize]
        ),
        36..=43 => format!("Joystick 2 Button {}", n - 35),
        50..=52 => format!("Global {} Button", b[(n - 50) as usize]),
        53..=55 => format!("Global {} Pressed", b[(n - 53) as usize]),
        56..=58 => format!("Global {} Released", b[(n - 56) as usize]),
        60 => "Mouse Wheel Up".into(),
        61 => "Mouse Wheel Down".into(),
        _ => format!("Mouse {n}"),
    }
}

/// Other sub-events (number, label) in menu order; views and user events are submenus.
/// Labelled sub-events.
pub type EventItems = Vec<(i32, String)>;

/// (main items, view items, user-defined items).
pub fn other_events() -> (EventItems, EventItems, EventItems) {
    let main = vec![
        (0, other_name(0)),
        (1, other_name(1)),
        (2, other_name(2)),
        (3, other_name(3)),
        (4, other_name(4)),
        (5, other_name(5)),
        (6, other_name(6)),
        (9, other_name(9)),
        (7, other_name(7)),
        (8, other_name(8)),
        (30, other_name(30)),
    ];
    let views = (40..48).chain(50..58).map(|n| (n, other_name(n))).collect();
    let user = (10..26).map(|n| (n, other_name(n))).collect();
    (main, views, user)
}

fn other_name(n: i32) -> String {
    match n {
        0 => "Outside Room".into(),
        1 => "Intersect Boundary".into(),
        2 => "Game Start".into(),
        3 => "Game End".into(),
        4 => "Room Start".into(),
        5 => "Room End".into(),
        6 => "No More Lives".into(),
        7 => "Animation End".into(),
        8 => "End Of Path".into(),
        9 => "No More Health".into(),
        10..=25 => format!("User Defined {}", n - 10),
        30 => "Close Button".into(),
        40..=47 => format!("Outside View {}", n - 40),
        50..=57 => format!("Boundary View {}", n - 50),
        _ => format!("Other {n}"),
    }
}

/// Display name of an event (spec §13.1).
pub fn event_name(p: &Project, event_type: i32, number: i32) -> String {
    match event_type {
        et::CREATE => "Create".into(),
        et::DESTROY => "Destroy".into(),
        et::ALARM => format!("Alarm {number}"),
        et::STEP => match number {
            1 => "Begin Step".into(),
            2 => "End Step".into(),
            _ => "Step".into(),
        },
        et::COLLISION => {
            let name = usize::try_from(number)
                .ok()
                .and_then(|i| p.objects.get(i))
                .and_then(|o| o.as_ref())
                .map_or_else(|| format!("<object {number}>"), |o| o.name.clone());
            format!("Collision with {name}")
        }
        et::KEYBOARD => format!("<{}>", key_name(number)),
        et::MOUSE => mouse_name(number),
        et::OTHER => other_name(number),
        et::DRAW => "Draw".into(),
        et::KEY_PRESS => format!("press <{}>", key_name(number)),
        et::KEY_RELEASE => format!("release <{}>", key_name(number)),
        et::TRIGGER => usize::try_from(number)
            .ok()
            .and_then(|i| p.triggers.get(i))
            .and_then(|t| t.as_ref())
            .map_or_else(|| format!("<trigger {number}>"), |t| t.name.clone()),
        _ => format!("Event {event_type}/{number}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        let p = Project::default();
        assert_eq!(event_name(&p, et::KEYBOARD, 37), "<Left>");
        assert_eq!(event_name(&p, et::KEY_PRESS, 65), "press <A>");
        assert_eq!(event_name(&p, et::OTHER, 12), "User Defined 2");
        assert_eq!(event_name(&p, et::MOUSE, 54), "Global Right Pressed");
        assert_eq!(event_name(&p, et::STEP, 1), "Begin Step");
        assert_eq!(key_name(113), "F2");
    }
}
