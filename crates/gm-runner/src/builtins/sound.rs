//! Chapter 406: sound, effects, 3D sound, CD audio (specs/sound.md).
//! Calls on non-existent sounds are ignored silently (§2).

use super::{bo, r, st, zero, Reg, A};
use crate::audio::dsp;
use crate::game::Game;
use crate::res::SoundRes;
use gml::{Error, Value};

impl Game {
    pub fn sound_play(&mut self, idx: i32, looped: bool) -> Result<(), Error> {
        let Some(res) = self.sound_res(idx) else { return Ok(()) };
        self.audio.play(idx, &res, looped, &[]);
        Ok(())
    }

    /// A clone of the sound resource (cheap: the data is shared).
    pub fn sound_res(&self, idx: i32) -> Option<SoundRes> {
        if idx < 0 {
            return None;
        }
        self.sounds.get(idx as usize).and_then(|s| s.clone())
    }
}

/// Runs `f` with the sound's resource if it exists.
fn with_sound(g: &mut Game, a: &A, f: impl FnOnce(&mut crate::audio::Audio, i32, &SoundRes, &A) -> Result<(), Error>) -> Result<Value, Error> {
    let i = a.i(0)?;
    if let Some(res) = g.sound_res(i) {
        f(&mut g.audio, i, &res, a)?;
    }
    zero()
}

fn effect(g: &mut Game, a: &A, which: i32, n: usize) -> Result<Value, Error> {
    let mut args = Vec::with_capacity(n);
    for k in 0..n {
        args.push(a.r(k + 1)?);
    }
    with_sound(g, a, |au, i, res, _| {
        au.effect_params(i, res, which, &args);
        Ok(())
    })
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
        with_sound(g, a, |au, i, res, a| {
            au.set_volume(i, res, a.r(1)?);
            Ok(())
        })
    }));
    l.push(("sound_global_volume", |g, _, _, a| {
        g.audio.set_global_volume(a.r(0)?);
        zero()
    }));
    l.push(("sound_fade", |g, _, _, a| {
        with_sound(g, a, |au, i, res, a| {
            au.fade(i, res, a.r(1)?, a.r(2)?);
            Ok(())
        })
    }));
    l.push(("sound_pan", |g, _, _, a| {
        with_sound(g, a, |au, i, res, a| {
            au.set_pan(i, res, a.r(1)?);
            Ok(())
        })
    }));
    l.push(("sound_background_tempo", |g, _, _, a| {
        g.audio.set_tempo(a.r(0)?);
        zero()
    }));
    l.push(("sound_set_search_directory", |_, _, _, _| zero()));
    // ---------------------------------------------------------------- effects
    l.push(("sound_effect_set", |g, _, _, a| {
        let i = a.i(0)?;
        let flags = a.i(1)? & dsp::SE_ALL;
        if let Some(Some(s)) = g.sounds.get_mut(i.max(0) as usize).filter(|_| i >= 0) {
            s.effects = flags;
        }
        with_sound(g, a, |au, i, res, _| {
            au.effect_set(i, res, flags);
            Ok(())
        })
    }));
    l.push(("sound_effect_chorus", |g, _, _, a| effect(g, a, dsp::SE_CHORUS, 7)));
    l.push(("sound_effect_flanger", |g, _, _, a| effect(g, a, dsp::SE_FLANGER, 7)));
    l.push(("sound_effect_echo", |g, _, _, a| effect(g, a, dsp::SE_ECHO, 5)));
    l.push(("sound_effect_gargle", |g, _, _, a| effect(g, a, dsp::SE_GARGLE, 2)));
    l.push(("sound_effect_reverb", |g, _, _, a| effect(g, a, dsp::SE_REVERB, 4)));
    l.push(("sound_effect_compressor", |g, _, _, a| effect(g, a, dsp::SE_COMPRESSOR, 6)));
    l.push(("sound_effect_equalizer", |g, _, _, a| effect(g, a, dsp::SE_EQUALIZER, 3)));
    // ---------------------------------------------------------------- 3D
    fn finite(x: f64) -> f64 {
        if x.is_finite() {
            x
        } else {
            0.0
        }
    }
    l.push(("sound_3d_set_sound_position", |g, _, _, a| {
        let p = [finite(a.r(1)?), finite(a.r(2)?), finite(a.r(3)?)];
        with_sound(g, a, |au, i, res, _| {
            au.set_3d(i, res, |s| s.pos = p);
            Ok(())
        })
    }));
    l.push(("sound_3d_set_sound_velocity", |g, _, _, a| {
        let v = [finite(a.r(1)?), finite(a.r(2)?), finite(a.r(3)?)];
        with_sound(g, a, |au, i, res, _| {
            au.set_3d(i, res, |s| s.vel = v);
            Ok(())
        })
    }));
    l.push(("sound_3d_set_sound_distance", |g, _, _, a| {
        let (mn, mx) = (finite(a.r(1)?).max(0.0), finite(a.r(2)?).max(0.0));
        with_sound(g, a, |au, i, res, _| {
            au.set_3d(i, res, |s| {
                s.mindist = mn;
                s.maxdist = mx.max(mn);
            });
            Ok(())
        })
    }));
    l.push(("sound_3d_set_sound_cone", |g, _, _, a| {
        let axis = [finite(a.r(1)?), finite(a.r(2)?), finite(a.r(3)?)];
        let (ai, ao, vo) = (finite(a.r(4)?).clamp(0.0, 360.0), finite(a.r(5)?).clamp(0.0, 360.0), finite(a.r(6)?).clamp(-10000.0, 0.0));
        with_sound(g, a, |au, i, res, _| {
            au.set_3d(i, res, |s| {
                s.cone_axis = axis;
                s.angle_in = ai;
                s.angle_out = ao.max(ai);
                s.vol_outside = vo;
            });
            Ok(())
        })
    }));
    // ---------------------------------------------------------------- CD audio (no CD support, §8)
    for n in ["cd_init", "cd_play", "cd_stop", "cd_pause", "cd_resume", "cd_set_position", "cd_set_track_position", "cd_open_door", "cd_close_door"] {
        l.push((n, |_, _, _, _| zero()));
    }
    for n in ["cd_present", "cd_playing"] {
        l.push((n, |_, _, _, _| bo(false)));
    }
    for n in ["cd_number", "cd_track", "cd_length", "cd_track_length", "cd_position", "cd_track_position"] {
        l.push((n, |_, _, _, _| r(0.0)));
    }
    l.push(("cd_paused", |_, _, _, _| bo(true)));
    l.push(("MCI_command", |_, _, _, a| st(&crate::audio::midi_out::mci_command(&a.s(0)?))));
}
