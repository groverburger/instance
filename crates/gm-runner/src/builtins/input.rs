//! Chapter 404: keyboard, mouse and joystick.

use super::{bo, r, zero, Reg};
use crate::game::Game;
use crate::input::InputEvent;

impl Game {
    /// Applies all input events available from the frontend (io_handle).
    pub fn pump_input(&mut self) -> bool {
        let evs = match self.pump.as_mut() {
            Some(p) => p(),
            None => return false,
        };
        let any = !evs.is_empty();
        for e in evs {
            if let InputEvent::Close = e {
                self.close_requested = true;
            } else {
                self.input.apply(&e);
            }
        }
        any
    }

    /// Blocks until `pred` holds on the input state (keyboard_wait / mouse_wait).
    pub fn wait_input(&mut self, pred: fn(&crate::input::Input) -> bool) {
        if self.headless || self.pump.is_none() {
            return;
        }
        self.input.end_step();
        loop {
            self.pump_input();
            if pred(&self.input) || self.close_requested {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

pub fn register(l: &mut Reg) {
    l.push(("keyboard_set_map", |g, _, _, a| {
        let (k1, k2) = (a.i(0)?, a.i(1)?);
        if (0..256).contains(&k1) && (0..256).contains(&k2) {
            g.input.keymap[k1 as usize] = k2 as u8;
        }
        zero()
    }));
    l.push(("keyboard_get_map", |g, _, _, a| {
        let k = a.i(0)?;
        r(if (0..256).contains(&k) { g.input.keymap[k as usize] as f64 } else { k as f64 })
    }));
    l.push(("keyboard_unset_map", |g, _, _, _| {
        for (i, k) in g.input.keymap.iter_mut().enumerate() {
            *k = i as u8;
        }
        zero()
    }));
    l.push(("keyboard_check", |g, _, _, a| bo(g.input.check(&g.input.held, a.i(0)?))));
    l.push(("keyboard_check_pressed", |g, _, _, a| bo(g.input.check(&g.input.pressed, a.i(0)?))));
    l.push(("keyboard_check_released", |g, _, _, a| bo(g.input.check(&g.input.released, a.i(0)?))));
    l.push(("keyboard_check_direct", |g, _, _, a| {
        let k = a.i(0)?;
        bo(g.input.check(&g.input.direct, k) || g.input.check(&g.input.held, k))
    }));
    l.push(("keyboard_get_numlock", |g, _, _, _| bo(g.input.numlock)));
    l.push(("keyboard_set_numlock", |g, _, _, a| {
        g.input.numlock = a.b(0)?;
        zero()
    }));
    l.push(("keyboard_key_press", |g, _, _, a| {
        let k = a.i(0)?;
        if (0..256).contains(&k) {
            g.input.apply(&InputEvent::KeyDown(k as u8));
        }
        zero()
    }));
    l.push(("keyboard_key_release", |g, _, _, a| {
        let k = a.i(0)?;
        if (0..256).contains(&k) {
            g.input.apply(&InputEvent::KeyUp(k as u8));
        }
        zero()
    }));
    l.push(("keyboard_clear", |g, _, _, a| {
        let k = a.i(0)?;
        if (0..256).contains(&k) {
            let k = k as usize;
            if g.input.held[k] {
                g.input.suppressed[k] = true;
            }
            g.input.held[k] = false;
            g.input.pressed[k] = false;
            g.input.released[k] = false;
        }
        zero()
    }));
    l.push(("io_clear", |g, _, _, _| {
        g.input.clear_all();
        zero()
    }));
    l.push(("io_handle", |g, _, _, _| {
        g.pump_input();
        zero()
    }));
    l.push(("keyboard_wait", |g, _, _, _| {
        g.wait_input(|i| i.pressed.iter().skip(2).any(|&b| b));
        zero()
    }));
    l.push(("mouse_wait", |g, _, _, _| {
        g.wait_input(|i| i.mb_pressed.iter().any(|&b| b));
        zero()
    }));
    l.push(("mouse_check_button", |g, _, _, a| bo(g.input.mouse_check(&g.input.mb_held, a.i(0)?))));
    l.push(("mouse_check_button_pressed", |g, _, _, a| {
        let b = a.i(0)?;
        if b == 0 {
            return bo(false);
        }
        bo(g.input.mouse_check(&g.input.mb_pressed, b))
    }));
    l.push(("mouse_check_button_released", |g, _, _, a| {
        let b = a.i(0)?;
        if b == 0 {
            return bo(false);
        }
        bo(g.input.mouse_check(&g.input.mb_released, b))
    }));
    l.push(("mouse_wheel_up", |g, _, _, _| bo(g.input.wheel_up)));
    l.push(("mouse_wheel_down", |g, _, _, _| bo(g.input.wheel_down)));
    l.push(("mouse_clear", |g, _, _, a| {
        let b = a.i(0)?;
        let bs: Vec<usize> = if b < 0 { vec![1, 2, 3] } else if (1..=3).contains(&b) { vec![b as usize] } else { vec![] };
        for b in bs {
            if g.input.mb_held[b] {
                g.input.mb_suppressed[b] = true;
            }
            g.input.mb_held[b] = false;
            g.input.mb_pressed[b] = false;
            g.input.mb_released[b] = false;
        }
        g.input.mouse_button = 0;
        zero()
    }));
}
