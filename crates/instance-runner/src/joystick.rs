//! Joysticks (specs/input-joystick.md): two slots refreshed once per step from
//! `gilrs` (windowed runs) or from scripted input (`--input STEP:joy1:...`).
//!
//! Options (environment):
//! Environment variables (`INSTANCE_*`; the `GM81_*` names of earlier builds also work):
//! * `INSTANCE_JOYSTICK=0` disables the device layer (only scripted input).
//! * `INSTANCE_JOY_DPAD_AS_STICK=1` makes the D-pad drive `joystick_direction` and
//!   the direction events (off by default, §5.3).
//! * `INSTANCE_JOY_RAW=1` uses raw axes/buttons in device order instead of the
//!   gamepad mapping of §5.2.

/// Deflection beyond which a direction counts (§3, strict inequality).
pub const DIR_THRESHOLD: f64 = 0.5;

/// `vk_numpad5`, the direction of a centred or absent stick.
pub const NUMPAD5: i32 = 101;

/// Joystick event sub-numbers of `ev_mouse` (§4).
pub mod ev {
    pub const JOY1_LEFT: i32 = 16;
    pub const JOY1_BUTTON1: i32 = 21;
    pub const JOY2_LEFT: i32 = 31;
    pub const JOY2_BUTTON1: i32 = 36;
}

/// State of one joystick slot.
#[derive(Clone, Debug, PartialEq)]
pub struct JoyState {
    pub connected: bool,
    pub name: String,
    pub axes_count: u32,
    pub buttons_count: u32,
    pub has_pov: bool,
    /// x, y, z, r, u, v in −1..1.
    pub axes: [f64; 6],
    pub buttons: [bool; 32],
    /// Degrees or −1.
    pub pov: f64,
    /// Driven by scripted input (the device layer leaves it alone).
    pub scripted: bool,
}

impl Default for JoyState {
    fn default() -> Self {
        JoyState { connected: false, name: String::new(), axes_count: 0, buttons_count: 0, has_pov: false, axes: [0.0; 6], buttons: [false; 32], pov: -1.0, scripted: false }
    }
}

impl JoyState {
    fn scripted_device() -> Self {
        JoyState { connected: true, name: "Scripted Joystick".into(), axes_count: 6, buttons_count: 32, has_pov: true, scripted: true, ..Default::default() }
    }

    /// Stick (x, y) used for directions; the D-pad optionally replaces it.
    fn stick(&self, dpad_as_stick: bool) -> (f64, f64) {
        if dpad_as_stick && self.pov >= 0.0 {
            let a = self.pov.to_radians();
            (a.sin(), -a.cos())
        } else {
            (self.axes[0], self.axes[1])
        }
    }

    /// `joystick_direction`: numpad key code (§3).
    pub fn direction(&self, dpad_as_stick: bool) -> i32 {
        if !self.connected {
            return NUMPAD5;
        }
        let (x, y) = self.stick(dpad_as_stick);
        let col = if x < -DIR_THRESHOLD {
            0
        } else if x > DIR_THRESHOLD {
            2
        } else {
            1
        };
        let row = if y < -DIR_THRESHOLD {
            2 // up: numpad 7/8/9
        } else if y > DIR_THRESHOLD {
            0 // down: numpad 1/2/3
        } else {
            1
        };
        96 + row * 3 + col + 1
    }

    /// Level-triggered joystick sub-events of this slot (0 or 1) in number order.
    pub fn events(&self, slot: usize, dpad_as_stick: bool, out: &mut Vec<i32>) {
        if !self.connected {
            return;
        }
        let (left, b1) = if slot == 0 { (ev::JOY1_LEFT, ev::JOY1_BUTTON1) } else { (ev::JOY2_LEFT, ev::JOY2_BUTTON1) };
        let (x, y) = self.stick(dpad_as_stick);
        if x < -DIR_THRESHOLD {
            out.push(left);
        }
        if x > DIR_THRESHOLD {
            out.push(left + 1);
        }
        if y < -DIR_THRESHOLD {
            out.push(left + 2);
        }
        if y > DIR_THRESHOLD {
            out.push(left + 3);
        }
        for b in 0..8 {
            if self.buttons[b] {
                out.push(b1 + b as i32);
            }
        }
    }
}

/// A scripted joystick change.
#[derive(Clone, Debug, PartialEq)]
pub enum JoyInput {
    Connect(usize),
    Disconnect(usize),
    /// Slot, button 1..32, down.
    Button(usize, u8, bool),
    /// Slot, axis 0..5 (x y z r u v), value.
    Axis(usize, usize, f64),
    /// Slot, angle or −1.
    Pov(usize, f64),
}

/// Parses the joystick part of a scripted input item, e.g. `joy1:button:3:down`,
/// `joy2:axis:x:-1`, `joy1:pov:90`, `joy1:connect`, `joy1:disconnect`.
pub fn parse_scripted(parts: &[&str]) -> Option<JoyInput> {
    let slot = match parts.first()?.to_ascii_lowercase().as_str() {
        "joy1" => 0,
        "joy2" => 1,
        _ => return None,
    };
    let what = parts.get(1)?.to_ascii_lowercase();
    Some(match what.as_str() {
        "connect" => JoyInput::Connect(slot),
        "disconnect" => JoyInput::Disconnect(slot),
        "button" => {
            let n: u8 = parts.get(2)?.parse().ok()?;
            if !(1..=32).contains(&n) {
                return None;
            }
            let down = match parts.get(3).map(|s| s.to_ascii_lowercase()).as_deref() {
                Some("down") | Some("1") | None => true,
                Some("up") | Some("0") => false,
                _ => return None,
            };
            JoyInput::Button(slot, n, down)
        }
        "axis" => {
            let a = match parts.get(2)?.to_ascii_lowercase().as_str() {
                "x" => 0,
                "y" => 1,
                "z" => 2,
                "r" => 3,
                "u" => 4,
                "v" => 5,
                _ => return None,
            };
            let v: f64 = parts.get(3)?.parse().ok()?;
            JoyInput::Axis(slot, a, v.clamp(-1.0, 1.0))
        }
        "pov" => {
            let v: f64 = parts.get(2)?.parse().ok()?;
            JoyInput::Pov(slot, if v < 0.0 { -1.0 } else { v.rem_euclid(360.0) })
        }
        _ => return None,
    })
}

/// The `gilrs` device layer (not cloned with the input state).
#[derive(Default)]
pub struct Device {
    gilrs: Option<gilrs::Gilrs>,
    tried: bool,
    assigned: [Option<gilrs::GamepadId>; 2],
}

impl Clone for Device {
    fn clone(&self) -> Self {
        Device::default()
    }
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Device({:?})", self.assigned)
    }
}

/// Both joystick slots.
#[derive(Clone, Debug, Default)]
pub struct Joysticks {
    pub slots: [JoyState; 2],
    pub dpad_as_stick: bool,
    device: Device,
}

/// `INSTANCE_<name>` (or `GM81_<name>`) as a flag.
fn env_flag(name: &str) -> Option<bool> {
    instance_brand::appdirs::env_var(name).map(|v| !(v.is_empty() || v == "0" || v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off")))
}

impl Joysticks {
    /// Slot of a GM joystick id (1 or 2) if connected.
    pub fn get(&self, id: i32) -> Option<&JoyState> {
        match id {
            1 | 2 => Some(&self.slots[id as usize - 1]).filter(|s| s.connected),
            _ => None,
        }
    }

    /// All joystick sub-events of this step, in number order.
    pub fn events(&self) -> Vec<i32> {
        let mut v = Vec::new();
        for (i, s) in self.slots.iter().enumerate() {
            s.events(i, self.dpad_as_stick, &mut v);
        }
        v
    }

    pub fn direction(&self, id: i32) -> i32 {
        self.get(id).map(|s| s.direction(self.dpad_as_stick)).unwrap_or(NUMPAD5)
    }

    /// Applies a scripted change; the slot becomes a scripted device.
    pub fn apply(&mut self, e: &JoyInput) {
        let slot = match *e {
            JoyInput::Connect(s) | JoyInput::Disconnect(s) | JoyInput::Button(s, ..) | JoyInput::Axis(s, ..) | JoyInput::Pov(s, _) => s.min(1),
        };
        if let JoyInput::Disconnect(_) = e {
            self.slots[slot] = JoyState { scripted: true, ..Default::default() };
            return;
        }
        if !(self.slots[slot].connected && self.slots[slot].scripted) {
            self.slots[slot] = JoyState::scripted_device();
        }
        let s = &mut self.slots[slot];
        match *e {
            JoyInput::Button(_, n, down) => s.buttons[(n as usize - 1).min(31)] = down,
            JoyInput::Axis(_, a, v) => s.axes[a.min(5)] = v,
            JoyInput::Pov(_, p) => s.pov = p,
            _ => {}
        }
    }

    /// Refreshes the device-backed slots (hot-plug, axes, buttons, POV).
    pub fn poll(&mut self) {
        let d = &mut self.device;
        if !d.tried {
            d.tried = true;
            if let Some(v) = env_flag("JOY_DPAD_AS_STICK") {
                self.dpad_as_stick = v;
            }
            if env_flag("JOYSTICK") == Some(false) {
                return;
            }
            match gilrs::GilrsBuilder::new().with_default_filters(false).build() {
                Ok(g) => {
                    let mut ids: Vec<gilrs::GamepadId> = g.gamepads().map(|(id, _)| id).collect();
                    ids.sort_by_key(|id| usize::from(*id));
                    d.gilrs = Some(g);
                    for id in ids {
                        assign(d, &mut self.slots, id);
                    }
                }
                Err(e) => eprintln!("joystick: unavailable ({})", e),
            }
        }
        let mut evs = Vec::new();
        {
            let Some(g) = d.gilrs.as_mut() else { return };
            while let Some(ev) = g.next_event() {
                evs.push(ev);
            }
        }
        for ev in evs {
            match ev.event {
                gilrs::EventType::Connected => assign(d, &mut self.slots, ev.id),
                gilrs::EventType::Disconnected => {
                    for k in 0..2 {
                        if d.assigned[k] == Some(ev.id) {
                            d.assigned[k] = None;
                            if !self.slots[k].scripted {
                                self.slots[k] = JoyState::default();
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let raw = env_flag("JOY_RAW").unwrap_or(false);
        let Some(g) = d.gilrs.as_ref() else { return };
        for k in 0..2 {
            let Some(id) = d.assigned[k] else { continue };
            if self.slots[k].scripted {
                continue;
            }
            let Some(pad) = g.connected_gamepad(id) else { continue };
            read_pad(&pad, &mut self.slots[k], raw);
        }
    }
}

fn assign(d: &mut Device, slots: &mut [JoyState; 2], id: gilrs::GamepadId) {
    if d.assigned.contains(&Some(id)) {
        return;
    }
    for k in 0..2 {
        if d.assigned[k].is_none() && !slots[k].scripted {
            d.assigned[k] = Some(id);
            if let Some(g) = d.gilrs.as_ref() {
                let pad = g.gamepad(id);
                slots[k] = JoyState { connected: true, name: pad.name().to_string(), axes_count: 5, buttons_count: 10, has_pov: true, ..Default::default() };
            }
            return;
        }
    }
}

/// Maps a gamepad onto the GM joystick model (§5.2).
fn read_pad(pad: &gilrs::Gamepad, s: &mut JoyState, raw: bool) {
    use gilrs::{Axis, Button};
    let mapped = !raw && pad.mapping_source() != gilrs::MappingSource::None;
    if mapped {
        let trig = |b: Button| pad.button_data(b).map(|d| d.value() as f64).unwrap_or(0.0);
        s.axes = [
            pad.value(Axis::LeftStickX) as f64,
            -(pad.value(Axis::LeftStickY) as f64),
            trig(Button::LeftTrigger2) - trig(Button::RightTrigger2),
            -(pad.value(Axis::RightStickY) as f64),
            pad.value(Axis::RightStickX) as f64,
            0.0,
        ];
        for a in s.axes.iter_mut() {
            *a = a.clamp(-1.0, 1.0);
        }
        const MAP: [Button; 11] = [
            Button::South,
            Button::East,
            Button::West,
            Button::North,
            Button::LeftTrigger,
            Button::RightTrigger,
            Button::Select,
            Button::Start,
            Button::LeftThumb,
            Button::RightThumb,
            Button::Mode,
        ];
        s.buttons = [false; 32];
        for (i, b) in MAP.iter().enumerate() {
            s.buttons[i] = pad.is_pressed(*b);
        }
        s.axes_count = 5;
        s.buttons_count = 10;
        s.has_pov = true;
        let dx = pad.is_pressed(Button::DPadRight) as i32 - pad.is_pressed(Button::DPadLeft) as i32;
        let dy = pad.is_pressed(Button::DPadUp) as i32 - pad.is_pressed(Button::DPadDown) as i32;
        s.pov = pov_angle(dx, dy);
    } else {
        let st = pad.state();
        let mut axes: Vec<(u32, f64)> = st.axes().map(|(c, d)| (c.into_u32(), d.value() as f64)).collect();
        axes.sort_by_key(|a| a.0);
        s.axes = [0.0; 6];
        for (i, (_, v)) in axes.iter().take(6).enumerate() {
            s.axes[i] = v.clamp(-1.0, 1.0);
        }
        let mut btns: Vec<(u32, bool)> = st.buttons().map(|(c, d)| (c.into_u32(), d.is_pressed())).collect();
        btns.sort_by_key(|b| b.0);
        s.buttons = [false; 32];
        for (i, (_, p)) in btns.iter().take(32).enumerate() {
            s.buttons[i] = *p;
        }
        s.axes_count = axes.len().min(6) as u32;
        s.buttons_count = btns.len().min(32) as u32;
        s.has_pov = false;
        s.pov = -1.0;
    }
}

/// POV angle from D-pad directions (dx right +, dy up +): 0 up, 90 right, …; −1 centred.
pub fn pov_angle(dx: i32, dy: i32) -> f64 {
    if dx == 0 && dy == 0 {
        return -1.0;
    }
    (dx as f64).atan2(dy as f64).to_degrees().rem_euclid(360.0).round()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(x: f64, y: f64) -> JoyState {
        let mut s = JoyState::scripted_device();
        s.axes[0] = x;
        s.axes[1] = y;
        s
    }

    #[test]
    fn direction_thresholding() {
        assert_eq!(pad(0.0, 0.0).direction(false), 101);
        assert_eq!(pad(-1.0, 0.0).direction(false), 100); // numpad4
        assert_eq!(pad(1.0, 0.0).direction(false), 102);
        assert_eq!(pad(0.0, -1.0).direction(false), 104); // up = numpad8
        assert_eq!(pad(0.0, 1.0).direction(false), 98); // down = numpad2
        assert_eq!(pad(-0.9, -0.9).direction(false), 103); // numpad7
        assert_eq!(pad(0.9, -0.9).direction(false), 105);
        assert_eq!(pad(-0.9, 0.9).direction(false), 97);
        assert_eq!(pad(0.9, 0.9).direction(false), 99);
        // strict inequality at the threshold
        assert_eq!(pad(0.5, -0.5).direction(false), 101);
        assert_eq!(pad(0.51, -0.49).direction(false), 102);
        assert_eq!(JoyState::default().direction(false), 101);
    }

    #[test]
    fn events_and_dpad_option() {
        let mut s = pad(-0.8, 0.8);
        s.buttons[0] = true;
        s.buttons[7] = true;
        s.buttons[8] = true; // button 9: no event
        let mut v = Vec::new();
        s.events(0, false, &mut v);
        assert_eq!(v, vec![16, 19, 21, 28]);
        v.clear();
        s.events(1, false, &mut v);
        assert_eq!(v, vec![31, 34, 36, 43]);
        // POV ignored unless the D-pad option is on
        let mut s = pad(0.0, 0.0);
        s.pov = 90.0;
        assert_eq!(s.direction(false), 101);
        assert_eq!(s.direction(true), 102);
        v.clear();
        s.events(0, true, &mut v);
        assert_eq!(v, vec![17]);
    }

    #[test]
    fn pov_from_dpad() {
        assert_eq!(pov_angle(0, 0), -1.0);
        assert_eq!(pov_angle(0, 1), 0.0);
        assert_eq!(pov_angle(1, 0), 90.0);
        assert_eq!(pov_angle(0, -1), 180.0);
        assert_eq!(pov_angle(-1, 0), 270.0);
        assert_eq!(pov_angle(1, 1), 45.0);
        assert_eq!(pov_angle(-1, 1), 315.0);
    }

    #[test]
    fn scripted_parse_and_apply() {
        let mut j = Joysticks::default();
        assert!(j.get(1).is_none());
        j.apply(&parse_scripted(&["joy1", "button", "3", "down"]).unwrap());
        assert!(j.get(1).unwrap().buttons[2]);
        j.apply(&parse_scripted(&["joy1", "axis", "x", "-1"]).unwrap());
        j.apply(&parse_scripted(&["joy1", "axis", "y", "-2"]).unwrap());
        assert_eq!(j.direction(1), 103);
        assert_eq!(j.events(), vec![16, 18, 23]);
        j.apply(&parse_scripted(&["joy1", "button", "3", "up"]).unwrap());
        assert_eq!(j.events(), vec![16, 18]);
        j.apply(&parse_scripted(&["joy2", "pov", "450"]).unwrap());
        assert_eq!(j.get(2).unwrap().pov, 90.0);
        j.apply(&parse_scripted(&["joy1", "disconnect"]).unwrap());
        assert!(j.get(1).is_none());
        assert_eq!(j.direction(1), 101);
        assert!(j.get(0).is_none() && j.get(3).is_none());
        assert!(parse_scripted(&["joy3", "connect"]).is_none());
        assert!(parse_scripted(&["joy1", "button", "33"]).is_none());
    }
}
