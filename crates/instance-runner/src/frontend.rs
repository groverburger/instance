//! Pieces shared by the frontends that show a game: the desktop window
//! (`main.rs`/`window.rs`, game loop on its own thread) and the browser
//! (`web.rs`, game loop driven by `requestAnimationFrame`). Both call
//! [`tick`] once per game step, pace steps with a [`Pacer`] and map the region
//! into their surface with [`mapping`].

use crate::Game;
use std::time::Duration;
use web_time::Instant;

/// Runs one game step: applies pending input, then [`Game::step`]. In debug
/// mode a paused game does not step (returns `Ok(false)`); watches are
/// re-evaluated after each step.
pub fn tick(g: &mut Game, vm: &mut gml::Vm) -> Result<bool, gml::Error> {
    g.pump_input();
    if !g.debug_should_step() {
        return Ok(false);
    }
    g.step(vm)?;
    if g.debug.is_some() {
        g.debug_update_watches(vm);
    }
    Ok(true)
}

/// Fixed-rate step scheduling at `room_speed`.
pub struct Pacer {
    next: Instant,
}

impl Default for Pacer {
    fn default() -> Self {
        Pacer { next: Instant::now() }
    }
}

impl Pacer {
    /// When the next step is due.
    pub fn next(&self) -> Instant {
        self.next
    }

    /// Whether the next step is due at `now`.
    pub fn due(&self, now: Instant) -> bool {
        now >= self.next
    }

    /// Schedules the step after the one just run (at `speed` steps per second).
    /// Falls back to "now" when more than a quarter second behind, so a stall
    /// does not cause a burst of catch-up steps.
    pub fn advance(&mut self, speed: f64) {
        self.next += Duration::from_secs_f64(1.0 / speed.max(1.0));
        let now = Instant::now();
        if now > self.next && now - self.next > Duration::from_millis(250) {
            self.next = now;
        }
    }

    /// Delays the next step (e.g. by `sleep()` in the browser).
    pub fn delay(&mut self, ms: f64) {
        if ms > 0.0 {
            self.next += Duration::from_secs_f64(ms / 1000.0);
        }
    }
}

/// How the window should look after a step.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub caption: String,
    /// >0 fixed scale, 0 fill, <0 keep aspect.
    pub scale: f64,
    pub fullscreen: bool,
    /// Colour outside the region (0RGB).
    pub outside: u32,
    /// Region (back buffer) size.
    pub w: u32,
    pub h: u32,
    /// Requested client size (0 = region × scale).
    pub req_w: i32,
    pub req_h: i32,
    pub cursor_visible: bool,
}

/// GameMaker BGR colour → 0RGB.
pub fn rgb_to_0rgb(c: u32) -> u32 {
    ((c & 0xFF) << 16) | (c & 0xFF00) | ((c >> 16) & 0xFF)
}

/// The current window state of a game.
pub fn view(g: &Game) -> View {
    let (w, h) = g.gfx.back_size();
    View {
        caption: g.caption(),
        scale: g.win.region_scale,
        fullscreen: g.fullscreen,
        outside: rgb_to_0rgb(g.settings.color_outside_room),
        w,
        h,
        req_w: g.win.w,
        req_h: g.win.h,
        cursor_visible: g.cursor != -1 && g.cursor_sprite < 0 && (g.settings.display_cursor || g.cursor != 0),
    }
}

/// Client-area size for a view (requested size, else region × scale).
pub fn client_size(v: &View) -> (u32, u32) {
    if v.req_w > 0 && v.req_h > 0 {
        return (v.req_w as u32, v.req_h as u32);
    }
    let k = if v.scale > 0.0 { v.scale } else { 1.0 };
    (((v.w.max(1) as f64) * k) as u32, ((v.h.max(1) as f64) * k) as u32)
}

/// Placement of the region in a surface: offset and scale per axis.
#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    pub ox: f64,
    pub oy: f64,
    pub sx: f64,
    pub sy: f64,
}

impl Mapping {
    /// Surface position → region position.
    pub fn to_region(&self, x: f64, y: f64) -> (f64, f64) {
        ((x - self.ox) / self.sx.max(1e-9), (y - self.oy) / self.sy.max(1e-9))
    }
}

/// Placement of the region (rw × rh) in a ww × wh surface (runtime.md §12.1).
pub fn mapping(ww: u32, wh: u32, rw: u32, rh: u32, scale: f64, fullscreen: bool) -> Mapping {
    let (ww, wh) = (ww.max(1) as f64, wh.max(1) as f64);
    let (rw, rh) = (rw.max(1) as f64, rh.max(1) as f64);
    let (sx, sy) = if scale > 0.0 && !fullscreen {
        (scale, scale)
    } else if scale == 0.0 {
        (ww / rw, wh / rh)
    } else {
        let k = (ww / rw).min(wh / rh);
        (k, k)
    };
    Mapping { ox: ((ww - rw * sx) / 2.0).floor(), oy: ((wh - rh * sy) / 2.0).floor(), sx, sy }
}

/// Factor that fits a `w × h` window into an `aw × ah` area keeping its aspect
/// ratio: the largest whole factor (≥ 1) when it fits unscaled, otherwise the
/// fractional factor that shrinks it to fit (browser frontend letterboxing).
pub fn fit_scale(w: f64, h: f64, aw: f64, ah: f64) -> f64 {
    let (w, h) = (w.max(1.0), h.max(1.0));
    if aw < 1.0 || ah < 1.0 {
        return 1.0;
    }
    let k = (aw / w).min(ah / h);
    if k >= 1.0 {
        k.floor()
    } else {
        k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_scale_letterboxes() {
        // fits: whole factor
        assert_eq!(fit_scale(640.0, 480.0, 1920.0, 969.0), 2.0);
        assert_eq!(fit_scale(640.0, 480.0, 800.0, 600.0), 1.0);
        // too big: shrink keeping the aspect ratio
        let k = fit_scale(1024.0, 768.0, 1366.0, 657.0);
        assert!((768.0 * k - 657.0).abs() < 1e-9 && 1024.0 * k <= 1366.0);
        let k = fit_scale(1920.0, 1080.0, 800.0, 600.0);
        assert!((1920.0 * k - 800.0).abs() < 1e-9 && 1080.0 * k <= 600.0);
    }

    #[test]
    fn mapping_keeps_aspect() {
        let m = mapping(800, 600, 400, 200, -1.0, false);
        assert_eq!((m.sx, m.sy, m.ox, m.oy), (2.0, 2.0, 0.0, 100.0));
        assert_eq!(m.to_region(400.0, 300.0), (200.0, 100.0));
    }

    #[test]
    fn pacer_steps_at_room_speed() {
        let mut p = Pacer::default();
        let t0 = p.next();
        p.advance(50.0);
        assert!(p.next() - t0 >= Duration::from_millis(19));
    }
}
