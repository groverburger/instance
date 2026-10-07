//! Chapter 406: sound (CD and effects are accepted but have no audible effect).

use super::{bo, r, zero, Reg};
use crate::game::Game;
use gml::Error;

impl Game {
    pub fn sound_play(&mut self, idx: i32, looped: bool) -> Result<(), Error> {
        let Some(Some(res)) = self.sounds.get(idx.max(0) as usize).cloned().filter(|_| idx >= 0) else {
            return Err(Error::new(format!("Sound does not exist: {}", idx)));
        };
        let all = std::mem::take(&mut self.sounds);
        self.audio.play(idx, &res, looped, &all);
        self.sounds = all;
        Ok(())
    }
}

pub fn register(l: &mut Reg) {
    l.push(("sound_play", |g, _, _, a| {
        g.sound_play(a.i(0)?, false)?;
        zero()
    }));
    l.push(("sound_loop", |g, _, _, a| {
        g.sound_play(a.i(0)?, true)?;
        zero()
    }));
    l.push(("sound_stop", |g, _, _, a| {
        g.audio.stop(a.i(0)?);
        zero()
    }));
    l.push(("sound_stop_all", |g, _, _, _| {
        g.audio.stop_all();
        zero()
    }));
    l.push(("sound_isplaying", |g, _, _, a| bo(g.audio.is_playing(a.i(0)?))));
    l.push(("sound_volume", |g, _, _, a| {
        g.audio.set_volume(a.i(0)?, a.r(1)?);
        zero()
    }));
    l.push(("sound_global_volume", |g, _, _, a| {
        g.audio.set_global_volume(a.r(0)?);
        zero()
    }));
    l.push(("sound_fade", |g, _, _, a| {
        // applied immediately (no gradual fade)
        g.audio.set_volume(a.i(0)?, a.r(1)?);
        zero()
    }));
    l.push(("sound_set_search_directory", |_, _, _, _| zero()));
    l.push(("sound_effect_set", |g, _, _, a| {
        let i = a.i(0)?;
        if let Some(Some(s)) = g.sounds.get_mut(i.max(0) as usize) {
            s.effects = a.i(1)?;
        }
        zero()
    }));
    let _ = r;
}
