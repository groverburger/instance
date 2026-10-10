//! Human-friendly event names (AGENT_GUIDE §3.6, file-formats.md §5.8).

use instance_format::{event_type as ev, Project};

/// Name of a virtual-key code as GM's event list shows it.
pub fn key_name(k: i32) -> String {
    match k {
        0 => "No Key".into(),
        1 => "Any Key".into(),
        8 => "Backspace".into(),
        9 => "Tab".into(),
        13 => "Enter".into(),
        16 => "Shift".into(),
        17 => "Ctrl".into(),
        18 => "Alt".into(),
        27 => "Escape".into(),
        32 => "Space".into(),
        33 => "Page Up".into(),
        34 => "Page Down".into(),
        35 => "End".into(),
        36 => "Home".into(),
        37 => "Left".into(),
        38 => "Up".into(),
        39 => "Right".into(),
        40 => "Down".into(),
        45 => "Insert".into(),
        46 => "Delete".into(),
        48..=57 | 65..=90 => ((k as u8) as char).to_string(),
        96..=105 => format!("Keypad {}", k - 96),
        106 => "Keypad *".into(),
        107 => "Keypad +".into(),
        109 => "Keypad -".into(),
        110 => "Keypad .".into(),
        111 => "Keypad /".into(),
        112..=123 => format!("F{}", k - 111),
        _ => format!("key {}", k),
    }
}

fn mouse_name(n: i32) -> String {
    let joy = |base: i32, j: i32| -> String {
        match n - base {
            0 => format!("Joystick {} Left", j),
            1 => format!("Joystick {} Right", j),
            2 => format!("Joystick {} Up", j),
            3 => format!("Joystick {} Down", j),
            b => format!("Joystick {} Button {}", j, b - 4),
        }
    };
    match n {
        0 => "Left Button".into(),
        1 => "Right Button".into(),
        2 => "Middle Button".into(),
        3 => "No Button".into(),
        4 => "Left Pressed".into(),
        5 => "Right Pressed".into(),
        6 => "Middle Pressed".into(),
        7 => "Left Released".into(),
        8 => "Right Released".into(),
        9 => "Middle Released".into(),
        10 => "Mouse Enter".into(),
        11 => "Mouse Leave".into(),
        16..=19 | 21..=28 => joy(16, 1),
        31..=34 | 36..=43 => joy(31, 2),
        50 => "Global Left Button".into(),
        51 => "Global Right Button".into(),
        52 => "Global Middle Button".into(),
        53 => "Global Left Pressed".into(),
        54 => "Global Right Pressed".into(),
        55 => "Global Middle Pressed".into(),
        56 => "Global Left Released".into(),
        57 => "Global Right Released".into(),
        58 => "Global Middle Released".into(),
        60 => "Mouse Wheel Up".into(),
        61 => "Mouse Wheel Down".into(),
        _ => format!("{}", n),
    }
}

/// `Create`, `Alarm 3`, `Collision with obj_wall`, `Key Press <Space>`,
/// `Other: Room Start`, `User Defined 0`, `Trigger trg_name`, ...
pub fn event_name(p: &Project, t: i32, n: i32) -> String {
    match t {
        ev::CREATE => "Create".into(),
        ev::DESTROY => "Destroy".into(),
        ev::ALARM => format!("Alarm {}", n),
        ev::STEP => match n {
            1 => "Begin Step".into(),
            2 => "End Step".into(),
            _ => "Step".into(),
        },
        ev::COLLISION => match usize::try_from(n)
            .ok()
            .and_then(|i| p.objects.get(i))
            .and_then(|o| o.as_ref())
        {
            Some(o) => format!("Collision with {}", o.name),
            None => format!("Collision with object {}", n),
        },
        ev::KEYBOARD => format!("Keyboard <{}>", key_name(n)),
        ev::MOUSE => format!("Mouse {}", mouse_name(n)),
        ev::OTHER => match n {
            0 => "Other: Outside Room".into(),
            1 => "Other: Intersect Boundary".into(),
            2 => "Other: Game Start".into(),
            3 => "Other: Game End".into(),
            4 => "Other: Room Start".into(),
            5 => "Other: Room End".into(),
            6 => "Other: No More Lives".into(),
            7 => "Other: Animation End".into(),
            8 => "Other: End of Path".into(),
            9 => "Other: No More Health".into(),
            10..=25 => format!("User Defined {}", n - 10),
            30 => "Other: Close Button".into(),
            40..=47 => format!("Other: Outside View {}", n - 40),
            50..=57 => format!("Other: Boundary View {}", n - 50),
            _ => format!("Other {}", n),
        },
        ev::DRAW => "Draw".into(),
        ev::KEY_PRESS => format!("Key Press <{}>", key_name(n)),
        ev::KEY_RELEASE => format!("Key Release <{}>", key_name(n)),
        ev::TRIGGER => match usize::try_from(n)
            .ok()
            .and_then(|i| p.triggers.get(i))
            .and_then(|t| t.as_ref())
        {
            Some(tr) => format!("Trigger {}", tr.name),
            None => format!("Trigger {}", n),
        },
        _ => format!("Event {}/{}", t, n),
    }
}
