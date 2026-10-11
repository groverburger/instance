//! bb_sound: when sound_isplaying turns true after sound_play/sound_loop and false
//! after sound_stop or the end of a sound, step by step (SND-8).

use crate::builder::{ev, Gen};
use crate::framework::*;

/// A 16-bit mono 22050 Hz .wav with a soft square-ish tone of `ms` milliseconds.
fn tone(ms: u32, hz: f64) -> Vec<u8> {
    let rate = 22050u32;
    let n = rate * ms / 1000;
    let mut data = Vec::with_capacity(n as usize * 2);
    for i in 0..n {
        let t = i as f64 / rate as f64;
        let v = (t * hz * std::f64::consts::TAU).sin() * 0.25;
        data.extend_from_slice(&((v * 32767.0) as i16).to_le_bytes());
    }
    let mut w = Vec::new();
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // mono
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(data.len() as u32).to_le_bytes());
    w.extend_from_slice(&data);
    w
}

/// The probe's Step event: a timetable keyed on the step counter `t` (room speed 60).
const STEP: &str = r#"var p;
t += 1;
// A: sound_loop on a normal sound, polled each step, then sound_stop
if (t == 10) { sound_loop(sb_loop); bb_log('snd.A.call', sound_isplaying(sb_loop)); a0 = current_time; }
if (t > 10 && t <= 40) {
    p = sound_isplaying(sb_loop);
    bb_log('snd.A.s' + bb_pad(t - 10), p);
    if (p && a_ms < 0) a_ms = current_time - a0;
}
if (t == 41) { bb_log('snd.A.first_true_ms', a_ms); sound_stop(sb_loop); bb_log('snd.A.stop.call', sound_isplaying(sb_loop)); }
if (t > 41 && t <= 46) bb_log('snd.A.stop.s' + bb_pad(t - 41), sound_isplaying(sb_loop));
// B: sound_play of a 250 ms sound: when does it turn true, and false again?
if (t == 60) { sound_play(sb_short); bb_log('snd.B.call', sound_isplaying(sb_short)); }
if (t > 60 && t <= 90) bb_log('snd.B.s' + bb_pad(t - 60), sound_isplaying(sb_short));
// C: the polling pattern "if (!sound_isplaying(s)) sound_loop(s)" every step for 1 s
if (t >= 100 && t < 160) { if (!sound_isplaying(sb_loop)) { sound_loop(sb_loop); c_calls += 1; } }
if (t == 160) { bb_log('snd.C.loop_calls', c_calls); sound_stop(sb_loop); }
if (t > 160 && t <= 165) bb_log('snd.C.stop.s' + bb_pad(t - 160), sound_isplaying(sb_loop));
// D: sound_loop twice (does one sound_stop silence both?)
if (t == 180) sound_loop(sb_loop);
if (t == 200) { sound_loop(sb_loop); bb_log('snd.D.second.call', sound_isplaying(sb_loop)); }
if (t == 220) { sound_stop(sb_loop); bb_log('snd.D.stop.call', sound_isplaying(sb_loop)); }
if (t > 220 && t <= 225) bb_log('snd.D.stop.s' + bb_pad(t - 220), sound_isplaying(sb_loop));
// E: sound_play right after sound_stop of the same sound in one step
if (t == 240) sound_loop(sb_loop);
if (t == 260) { sound_stop(sb_loop); sound_loop(sb_loop); bb_log('snd.E.restart.call', sound_isplaying(sb_loop)); }
if (t > 260 && t <= 265) bb_log('snd.E.restart.s' + bb_pad(t - 260), sound_isplaying(sb_loop));
if (t == 266) sound_stop(sb_loop);
// F: background-music kind
if (t == 280) { sound_loop(sb_music); bb_log('snd.F.call', sound_isplaying(sb_music)); }
if (t > 280 && t <= 290) bb_log('snd.F.s' + bb_pad(t - 280), sound_isplaying(sb_music));
if (t == 291) { sound_stop(sb_music); bb_log('snd.F.stop.call', sound_isplaying(sb_music)); }
if (t > 291 && t <= 295) bb_log('snd.F.stop.s' + bb_pad(t - 291), sound_isplaying(sb_music));
if (t == 300) global.bb_done = 1;
"#;

pub fn build() -> Spec {
    let mut g = Gen::new("bb_sound", 81040);
    let fw = install(&mut g);
    g.sound("sb_loop", 0, tone(1000, 220.0));
    g.sound("sb_short", 0, tone(250, 440.0));
    g.sound("sb_music", 1, tone(1000, 330.0));
    let probe = g.object("sb_probe");
    g.event(probe, ev::CREATE, &["t = 0; a0 = 0; a_ms = -1; c_calls = 0;"]);
    g.event(probe, ev::STEP, &[STEP]);
    let r = g.room("rm_sound", ROOM_W, ROOM_H);
    g.room_mut(r).speed = 60;
    g.place(r, fw.out_obj, 0, 0, "");
    g.place(r, probe, 0, 0, "");
    Spec {
        name: "bb_sound",
        title: "Sound: sound_isplaying right after sound_play/sound_loop/sound_stop",
        lite: "Lite-safe. Plays quiet test tones for about 5 seconds; leave the sound on.",
        frames: 330,
        keys: vec![
            k("snd.A.", "SND-8", "isplaying per step after sound_loop (normal sound), and after sound_stop"),
            k("snd.B.", "SND-8", "isplaying per step after sound_play of a 250 ms sound (until it ends)"),
            k("snd.C.", "SND-8", "how many times \"if (!sound_isplaying) sound_loop\" fires in 60 steps"),
            k("snd.D.", "SND-8", "sound_loop twice, then one sound_stop"),
            k("snd.E.", "SND-8", "sound_stop + sound_loop in the same step"),
            k("snd.F.", "SND-8", "background-music kind: isplaying after sound_loop / sound_stop"),
            ki("snd.A.first_true_ms", "SND-8", "milliseconds until isplaying first read true (timing)"),
        ],
        project: g.finish(),
    }
}
