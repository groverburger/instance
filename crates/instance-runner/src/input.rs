//! Keyboard and mouse state (runtime.md §11).

/// Windows virtual-key codes used by the runner.
pub mod vk {
    pub const NOKEY: u8 = 0;
    pub const ANYKEY: u8 = 1;
    pub const BACKSPACE: u8 = 8;
    pub const TAB: u8 = 9;
    pub const ENTER: u8 = 13;
    pub const SHIFT: u8 = 16;
    pub const CONTROL: u8 = 17;
    pub const ALT: u8 = 18;
    pub const PAUSE: u8 = 19;
    pub const ESCAPE: u8 = 27;
    pub const SPACE: u8 = 32;
    pub const PAGEUP: u8 = 33;
    pub const PAGEDOWN: u8 = 34;
    pub const END: u8 = 35;
    pub const HOME: u8 = 36;
    pub const LEFT: u8 = 37;
    pub const UP: u8 = 38;
    pub const RIGHT: u8 = 39;
    pub const DOWN: u8 = 40;
    pub const PRINTSCREEN: u8 = 44;
    pub const INSERT: u8 = 45;
    pub const DELETE: u8 = 46;
    pub const NUMPAD0: u8 = 96;
    pub const MULTIPLY: u8 = 106;
    pub const ADD: u8 = 107;
    pub const SUBTRACT: u8 = 109;
    pub const DECIMAL: u8 = 110;
    pub const DIVIDE: u8 = 111;
    pub const F1: u8 = 112;
    pub const F4: u8 = 115;
    pub const F5: u8 = 116;
    pub const F6: u8 = 117;
    pub const F9: u8 = 120;
    pub const LSHIFT: u8 = 160;
    pub const RSHIFT: u8 = 161;
    pub const LCONTROL: u8 = 162;
    pub const RCONTROL: u8 = 163;
    pub const LALT: u8 = 164;
    pub const RALT: u8 = 165;
}

/// An input event from the window or the scripted queue. Mouse positions are in
/// region (back-buffer) pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    KeyDown(u8),
    KeyUp(u8),
    Char(char),
    MouseMove(f64, f64),
    /// GM button numbers: 1 left, 2 right, 3 middle.
    MouseDown(u8),
    MouseUp(u8),
    /// +1 up, -1 down.
    Wheel(i32),
    Close,
    Focus(bool),
    /// Scripted joystick change (specs/input-joystick.md).
    Joy(crate::joystick::JoyInput),
}

/// Input state.
#[derive(Clone, Debug)]
pub struct Input {
    pub held: [bool; 256],
    pub pressed: [bool; 256],
    pub released: [bool; 256],
    /// Keys cleared with keyboard_clear: ignored until released.
    pub suppressed: [bool; 256],
    pub direct: [bool; 256],
    pub keymap: [u8; 256],
    pub mouse_x: f64,
    pub mouse_y: f64,
    /// Indexed by GM button number (1..3).
    pub mb_held: [bool; 4],
    pub mb_pressed: [bool; 4],
    pub mb_released: [bool; 4],
    pub mb_suppressed: [bool; 4],
    pub wheel_up: bool,
    pub wheel_down: bool,
    pub keyboard_key: i32,
    pub keyboard_lastkey: i32,
    pub keyboard_lastchar: String,
    pub keyboard_string: String,
    pub mouse_button: i32,
    pub mouse_lastbutton: i32,
    pub numlock: bool,
    /// Joystick slots 1 and 2.
    pub joy: crate::joystick::Joysticks,
}

impl Default for Input {
    fn default() -> Self {
        let mut keymap = [0u8; 256];
        for (i, k) in keymap.iter_mut().enumerate() {
            *k = i as u8;
        }
        Input {
            held: [false; 256],
            pressed: [false; 256],
            released: [false; 256],
            suppressed: [false; 256],
            direct: [false; 256],
            keymap,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mb_held: [false; 4],
            mb_pressed: [false; 4],
            mb_released: [false; 4],
            mb_suppressed: [false; 4],
            wheel_up: false,
            wheel_down: false,
            keyboard_key: 0,
            keyboard_lastkey: 0,
            keyboard_lastchar: String::new(),
            keyboard_string: String::new(),
            mouse_button: 0,
            mouse_lastbutton: 0,
            numlock: true,
            joy: Default::default(),
        }
    }
}

impl Input {
    pub fn apply(&mut self, e: &InputEvent) {
        match *e {
            InputEvent::KeyDown(k) => {
                self.direct[k as usize] = true;
                let k = self.keymap[k as usize];
                let ku = k as usize;
                if self.suppressed[ku] {
                    return;
                }
                if !self.held[ku] {
                    self.pressed[ku] = true;
                }
                self.held[ku] = true;
                self.keyboard_key = k as i32;
                self.keyboard_lastkey = k as i32;
                if k == vk::BACKSPACE {
                    self.keyboard_string.pop();
                }
                // generic modifiers also set left/right variants for direct checks
            }
            InputEvent::KeyUp(k) => {
                self.direct[k as usize] = false;
                let k = self.keymap[k as usize];
                let ku = k as usize;
                if self.suppressed[ku] {
                    self.suppressed[ku] = false;
                    return;
                }
                if self.held[ku] {
                    self.released[ku] = true;
                }
                self.held[ku] = false;
                if self.keyboard_key == k as i32 {
                    self.keyboard_key = (2..256).rev().find(|&i| self.held[i]).unwrap_or(0) as i32;
                }
            }
            InputEvent::Char(c) => {
                if !c.is_control() {
                    let s: String = c.to_string();
                    self.keyboard_lastchar = s.clone();
                    self.keyboard_string.push(c);
                    if self.keyboard_string.chars().count() > 1024 {
                        let skip = self.keyboard_string.chars().count() - 1024;
                        self.keyboard_string = self.keyboard_string.chars().skip(skip).collect();
                    }
                }
            }
            InputEvent::MouseMove(x, y) => {
                self.mouse_x = x;
                self.mouse_y = y;
            }
            InputEvent::MouseDown(b) => {
                let b = (b as usize).min(3);
                if self.mb_suppressed[b] {
                    return;
                }
                if !self.mb_held[b] {
                    self.mb_pressed[b] = true;
                }
                self.mb_held[b] = true;
                self.mouse_button = b as i32;
                self.mouse_lastbutton = b as i32;
            }
            InputEvent::MouseUp(b) => {
                let b = (b as usize).min(3);
                if self.mb_suppressed[b] {
                    self.mb_suppressed[b] = false;
                    return;
                }
                if self.mb_held[b] {
                    self.mb_released[b] = true;
                }
                self.mb_held[b] = false;
                self.mouse_button = (1..4).find(|&i| self.mb_held[i]).unwrap_or(0) as i32;
            }
            InputEvent::Wheel(d) => {
                if d > 0 {
                    self.wheel_up = true;
                } else if d < 0 {
                    self.wheel_down = true;
                }
            }
            InputEvent::Joy(ref j) => self.joy.apply(j),
            InputEvent::Focus(false) => self.clear_all(),
            InputEvent::Focus(true) | InputEvent::Close => {}
        }
    }

    /// Clears per-step edges (§4.2 step 20).
    pub fn end_step(&mut self) {
        self.pressed = [false; 256];
        self.released = [false; 256];
        self.mb_pressed = [false; 4];
        self.mb_released = [false; 4];
        self.wheel_up = false;
        self.wheel_down = false;
    }

    /// `io_clear`.
    pub fn clear_all(&mut self) {
        for k in 0..256 {
            if self.held[k] {
                self.suppressed[k] = true;
            }
        }
        self.held = [false; 256];
        self.pressed = [false; 256];
        self.released = [false; 256];
        for b in 0..4 {
            if self.mb_held[b] {
                self.mb_suppressed[b] = true;
            }
        }
        self.mb_held = [false; 4];
        self.mb_pressed = [false; 4];
        self.mb_released = [false; 4];
        self.keyboard_key = 0;
        self.mouse_button = 0;
    }

    /// `keyboard_check` family helper for `vk_anykey`/`vk_nokey`.
    pub fn check(&self, set: &[bool; 256], k: i32) -> bool {
        match k {
            0 => !set.iter().skip(2).any(|&b| b),
            1 => set.iter().skip(2).any(|&b| b),
            16 => set[16] || set[160] || set[161],
            17 => set[17] || set[162] || set[163],
            18 => set[18] || set[164] || set[165],
            k if (0..256).contains(&k) => set[k as usize],
            _ => false,
        }
    }

    pub fn mouse_check(&self, set: &[bool; 4], b: i32) -> bool {
        match b {
            -1 => set[1] || set[2] || set[3],
            0 => !(set[1] || set[2] || set[3]),
            1..=3 => set[b as usize],
            _ => false,
        }
    }
}

/// Display name of a key code (error messages).
pub fn key_name(k: i32) -> String {
    match k {
        0 => "No Key".into(),
        1 => "Any Key".into(),
        37 => "Left".into(),
        38 => "Up".into(),
        39 => "Right".into(),
        40 => "Down".into(),
        32 => "Space".into(),
        13 => "Enter".into(),
        27 => "Escape".into(),
        k if (48..=57).contains(&k) || (65..=90).contains(&k) => ((k as u8) as char).to_string(),
        k => format!("key {}", k),
    }
}

/// Parses a key name for the scripted input syntax ("left", "a", "3", "37", ...).
/// A single character is that key (so "3" is the digit key, code 51); longer
/// numbers are virtual-key codes.
pub fn parse_key(s: &str) -> Option<u8> {
    let l = s.to_ascii_lowercase();
    if l.len() == 1 && l.as_bytes()[0].is_ascii_alphanumeric() {
        return Some(l.as_bytes()[0].to_ascii_uppercase());
    }
    Some(match l.as_str() {
        "left" => vk::LEFT,
        "right" => vk::RIGHT,
        "up" => vk::UP,
        "down" => vk::DOWN,
        "space" => vk::SPACE,
        "enter" | "return" => vk::ENTER,
        "escape" | "esc" => vk::ESCAPE,
        "shift" => vk::SHIFT,
        "control" | "ctrl" => vk::CONTROL,
        "alt" => vk::ALT,
        "backspace" => vk::BACKSPACE,
        "tab" => vk::TAB,
        _ => {
            if let Ok(n) = l.parse::<u8>() {
                n
            } else if l.len() == 1 {
                l.as_bytes()[0].to_ascii_uppercase()
            } else if let Some(f) = l.strip_prefix('f') {
                let n: u8 = f.parse().ok()?;
                111 + n
            } else {
                return None;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges() {
        let mut i = Input::default();
        i.apply(&InputEvent::KeyDown(37));
        assert!(i.pressed[37] && i.held[37]);
        i.end_step();
        i.apply(&InputEvent::KeyDown(37)); // auto-repeat
        assert!(!i.pressed[37]);
        i.apply(&InputEvent::KeyUp(37));
        assert!(i.released[37] && !i.held[37]);
        assert!(i.check(&i.held, 0));
    }

    #[test]
    fn key_names() {
        assert_eq!(parse_key("3"), Some(b'3')); // the digit key, not code 3
        assert_eq!(parse_key("a"), Some(b'A'));
        assert_eq!(parse_key("37"), Some(37));
        assert_eq!(parse_key("left"), Some(vk::LEFT));
        assert_eq!(parse_key("f5"), Some(116));
        assert_eq!(parse_key("nope"), None);
    }
}

/// Virtual key code (`vk_*`) of a physical key named like the DOM
/// `KeyboardEvent.code` / winit `KeyCode` ("KeyA", "ArrowLeft", "Numpad0", ...).
pub fn vk_from_code(code: &str) -> Option<u8> {
    Some(match code {
        "ArrowLeft" => 37,
        "ArrowUp" => 38,
        "ArrowRight" => 39,
        "ArrowDown" => 40,
        "Space" => 32,
        "Enter" | "NumpadEnter" => 13,
        "Escape" => 27,
        "Backspace" => 8,
        "Tab" => 9,
        "ShiftLeft" => 16,
        "ShiftRight" => 16,
        "ControlLeft" | "ControlRight" => 17,
        "AltLeft" | "AltRight" => 18,
        "Pause" => 19,
        "PageUp" => 33,
        "PageDown" => 34,
        "End" => 35,
        "Home" => 36,
        "PrintScreen" => 44,
        "Insert" => 45,
        "Delete" => 46,
        "Digit0" => 48,
        "Digit1" => 49,
        "Digit2" => 50,
        "Digit3" => 51,
        "Digit4" => 52,
        "Digit5" => 53,
        "Digit6" => 54,
        "Digit7" => 55,
        "Digit8" => 56,
        "Digit9" => 57,
        "KeyA" => 65,
        "KeyB" => 66,
        "KeyC" => 67,
        "KeyD" => 68,
        "KeyE" => 69,
        "KeyF" => 70,
        "KeyG" => 71,
        "KeyH" => 72,
        "KeyI" => 73,
        "KeyJ" => 74,
        "KeyK" => 75,
        "KeyL" => 76,
        "KeyM" => 77,
        "KeyN" => 78,
        "KeyO" => 79,
        "KeyP" => 80,
        "KeyQ" => 81,
        "KeyR" => 82,
        "KeyS" => 83,
        "KeyT" => 84,
        "KeyU" => 85,
        "KeyV" => 86,
        "KeyW" => 87,
        "KeyX" => 88,
        "KeyY" => 89,
        "KeyZ" => 90,
        "Numpad0" => 96,
        "Numpad1" => 97,
        "Numpad2" => 98,
        "Numpad3" => 99,
        "Numpad4" => 100,
        "Numpad5" => 101,
        "Numpad6" => 102,
        "Numpad7" => 103,
        "Numpad8" => 104,
        "Numpad9" => 105,
        "NumpadMultiply" => 106,
        "NumpadAdd" => 107,
        "NumpadSubtract" => 109,
        "NumpadDecimal" => 110,
        "NumpadDivide" => 111,
        "F1" => 112,
        "F2" => 113,
        "F3" => 114,
        "F4" => 115,
        "F5" => 116,
        "F6" => 117,
        "F7" => 118,
        "F8" => 119,
        "F9" => 120,
        "F10" => 121,
        "F11" => 122,
        "F12" => 123,
        "NumLock" => 144,
        "ScrollLock" => 145,
        "Semicolon" => 186,
        "Equal" => 187,
        "Comma" => 188,
        "Minus" => 189,
        "Period" => 190,
        "Slash" => 191,
        "Backquote" => 192,
        "BracketLeft" => 219,
        "Backslash" => 220,
        "BracketRight" => 221,
        "Quote" => 222,
        _ => return None,
    })
}
