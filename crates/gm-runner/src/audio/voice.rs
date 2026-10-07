//! Per-sound shared control state (volume/fade/pan/effects/3D, specs/sound.md
//! §3, §5, §6) and the rodio [`Voice`] source that renders one playback.

use super::dsp::{EffectSettings, FxChain};
use super::midi::{Out, SeqPlayer};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Engine-wide values read by voices and the MIDI thread.
pub struct Globals {
    volume: AtomicU32,
    tempo: AtomicU64,
}

impl Default for Globals {
    fn default() -> Self {
        Globals { volume: AtomicU32::new(1f32.to_bits()), tempo: AtomicU64::new(1f64.to_bits()) }
    }
}

impl Globals {
    pub fn volume(&self) -> f64 {
        f32::from_bits(self.volume.load(Ordering::Relaxed)) as f64
    }
    pub fn set_volume(&self, v: f64) {
        self.volume.store((v.clamp(0.0, 1.0) as f32).to_bits(), Ordering::Relaxed)
    }
    pub fn tempo(&self) -> f64 {
        f64::from_bits(self.tempo.load(Ordering::Relaxed))
    }
    pub fn set_tempo(&self, t: f64) {
        let t = if t.is_nan() { 1.0 } else { t.clamp(0.01, 100.0) };
        self.tempo.store(t.to_bits(), Ordering::Relaxed)
    }
}

/// A linear volume ramp in real time (`sound_fade`).
#[derive(Clone, Copy, Debug)]
pub struct Fade {
    pub from: f64,
    pub to: f64,
    pub start: Instant,
    /// Seconds; 0 = constant `to`.
    pub dur: f64,
}

impl Fade {
    pub fn constant(v: f64) -> Self {
        Fade { from: v, to: v, start: Instant::now(), dur: 0.0 }
    }
    pub fn value_at(&self, now: Instant) -> f64 {
        if self.dur <= 0.0 {
            return self.to;
        }
        let t = now.saturating_duration_since(self.start).as_secs_f64() / self.dur;
        if t >= 1.0 {
            self.to
        } else {
            self.from + (self.to - self.from) * t
        }
    }
}

/// 3D settings of a kind-2 sound (§6).
#[derive(Clone, Debug, PartialEq)]
pub struct Sound3d {
    pub pos: [f64; 3],
    pub vel: [f64; 3],
    pub mindist: f64,
    pub maxdist: f64,
    pub cone_axis: [f64; 3],
    pub angle_in: f64,
    pub angle_out: f64,
    /// Hundredths of a dB (≤ 0).
    pub vol_outside: f64,
}

impl Default for Sound3d {
    fn default() -> Self {
        Sound3d { pos: [0.0; 3], vel: [0.0; 3], mindist: 1.0, maxdist: 1e9, cone_axis: [0.0, 0.0, 1.0], angle_in: 360.0, angle_out: 360.0, vol_outside: 0.0 }
    }
}

fn len3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// The result of the 3D model: distance × cone gain, pan −1..1, doppler pitch factor.
pub fn compute_3d(s: &Sound3d) -> (f64, f64, f64) {
    let d = len3(s.pos);
    let mindist = s.mindist.max(1e-6);
    let maxdist = s.maxdist.max(mindist);
    let dist_gain = mindist / d.clamp(mindist, maxdist);
    let mut cone_gain = 1.0;
    let al = len3(s.cone_axis);
    if d > 1e-9 && al > 1e-9 && s.angle_out < 360.0 {
        // angle between the cone axis and the direction source → listener (−p)
        let dot = -(s.cone_axis[0] * s.pos[0] + s.cone_axis[1] * s.pos[1] + s.cone_axis[2] * s.pos[2]) / (al * d);
        let theta = dot.clamp(-1.0, 1.0).acos().to_degrees();
        let half_in = s.angle_in.clamp(0.0, 360.0) / 2.0;
        let half_out = s.angle_out.clamp(0.0, 360.0).max(s.angle_in) / 2.0;
        let outside_db = s.vol_outside.min(0.0) / 100.0;
        let db = if theta <= half_in {
            0.0
        } else if theta >= half_out {
            outside_db
        } else {
            outside_db * (theta - half_in) / (half_out - half_in).max(1e-9)
        };
        cone_gain = 10f64.powf(db / 20.0);
    }
    let pan = if d > 1e-9 { (s.pos[0] / d).clamp(-1.0, 1.0) } else { 0.0 };
    let doppler = if d > 1e-9 {
        let radial = (s.vel[0] * s.pos[0] + s.vel[1] * s.pos[1] + s.vel[2] * s.pos[2]) / d;
        (343.0 / (343.0 + radial).max(1e-3)).clamp(0.25, 4.0)
    } else {
        1.0
    };
    (dist_gain * cone_gain, pan, doppler)
}

/// Linear pan law of §3: (left, right) factors.
pub fn linear_pan(pan: f64) -> (f64, f64) {
    let p = pan.clamp(-1.0, 1.0);
    (1.0 - p.max(0.0), 1.0 + p.min(0.0))
}

/// Equal-power pan normalised to unity at the centre, never amplifying.
pub fn equal_power_pan(pan: f64) -> (f64, f64) {
    let a = (pan.clamp(-1.0, 1.0) + 1.0) * std::f64::consts::FRAC_PI_4;
    ((a.cos() * std::f64::consts::SQRT_2).min(1.0), (a.sin() * std::f64::consts::SQRT_2).min(1.0))
}

/// Mutable per-sound parameters.
#[derive(Clone, Debug)]
pub struct SoundParams {
    pub kind: i32,
    pub volume: Fade,
    pub pan: f64,
    pub effects: EffectSettings,
    pub s3d: Sound3d,
}

/// Per-sound control block shared by the game thread and all voices of the sound.
pub struct SoundCtl {
    pub p: Mutex<SoundParams>,
    pub fx_version: AtomicU64,
}

/// The mixing decision for one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mix {
    pub left: f64,
    pub right: f64,
    /// Overall gain (for the MIDI CC7 path).
    pub gain: f64,
    /// Pan (for the MIDI CC10 path).
    pub pan: f64,
    pub doppler: f64,
}

impl SoundCtl {
    pub fn new(kind: i32, volume: f64, pan: f64, effects: i32) -> Self {
        SoundCtl {
            p: Mutex::new(SoundParams {
                kind,
                volume: Fade::constant(volume.clamp(0.0, 1.0)),
                pan: pan.clamp(-1.0, 1.0),
                effects: EffectSettings::with_flags(effects & 31),
                s3d: Sound3d::default(),
            }),
            fx_version: AtomicU64::new(0),
        }
    }

    pub fn volume_now(&self) -> f64 {
        self.p.lock().unwrap().volume.value_at(Instant::now())
    }

    pub fn set_volume(&self, v: f64) {
        self.p.lock().unwrap().volume = Fade::constant(v.clamp(0.0, 1.0));
    }

    pub fn fade(&self, v: f64, ms: f64) {
        let now = Instant::now();
        let mut p = self.p.lock().unwrap();
        let cur = p.volume.value_at(now);
        p.volume = Fade { from: cur, to: v.clamp(0.0, 1.0), start: now, dur: (ms / 1000.0).max(0.0) };
    }

    pub fn effects(&self) -> EffectSettings {
        self.p.lock().unwrap().effects.clone()
    }

    pub fn update_effects(&self, f: impl FnOnce(&mut EffectSettings) -> bool) {
        let changed = f(&mut self.p.lock().unwrap().effects);
        if changed {
            self.fx_version.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Computes the gains of a sound at `now`.
pub fn mix_of(p: &SoundParams, global: f64, now: Instant) -> Mix {
    if p.kind == 3 {
        // multimedia sounds ignore volume, pan and fades
        return Mix { left: 1.0, right: 1.0, gain: 1.0, pan: 0.0, doppler: 1.0 };
    }
    let base = p.volume.value_at(now).clamp(0.0, 1.0) * global.clamp(0.0, 1.0);
    if p.kind == 2 {
        let (g3, pan, doppler) = compute_3d(&p.s3d);
        let gain = base * g3;
        let (l, r) = equal_power_pan(pan);
        Mix { left: gain * l, right: gain * r, gain, pan, doppler }
    } else {
        let (l, r) = linear_pan(p.pan);
        Mix { left: base * l, right: base * r, gain: base, pan: p.pan, doppler: 1.0 }
    }
}

/// Liveness of one playback, shared between the engine and the renderer.
#[derive(Default)]
pub struct VoiceState {
    pub stop: AtomicBool,
    pub done: AtomicBool,
}

impl VoiceState {
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release)
    }
    pub fn stop_requested(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }
    pub fn finish(&self) {
        self.done.store(true, Ordering::Release)
    }
}

/// Decoded PCM, interleaved stereo f32.
pub struct Pcm {
    pub rate: u32,
    pub data: Vec<f32>,
}

impl Pcm {
    pub fn frames(&self) -> usize {
        self.data.len() / 2
    }
    pub fn duration(&self) -> f64 {
        self.frames() as f64 / self.rate.max(1) as f64
    }
}

/// Decodes any format rodio/symphonia understands into stereo f32.
pub fn decode(data: Arc<[u8]>) -> Option<Pcm> {
    use rodio::Source;
    let dec = rodio::Decoder::new(std::io::Cursor::new(data)).ok()?;
    let ch = dec.channels().get() as usize;
    let rate = dec.sample_rate().get();
    let mut out = Vec::new();
    let mut frame = Vec::with_capacity(ch);
    for s in dec {
        frame.push(s);
        if frame.len() == ch {
            if ch == 1 {
                out.push(frame[0]);
                out.push(frame[0]);
            } else {
                out.push(frame[0]);
                out.push(frame[1]);
            }
            frame.clear();
        }
    }
    Some(Pcm { rate, data: out })
}

/// A sample generator producing stereo frames at a fixed rate.
pub trait Gen: Send {
    fn rate(&self) -> u32;
    /// Next frame; `pitch` > 1 plays faster (doppler). None = finished.
    fn next(&mut self, pitch: f64) -> Option<(f32, f32)>;
}

/// Plays a decoded buffer, optionally looping.
pub struct PcmGen {
    pub pcm: Arc<Pcm>,
    pub pos: f64,
    pub looped: bool,
}

impl Gen for PcmGen {
    fn rate(&self) -> u32 {
        self.pcm.rate.max(1)
    }
    fn next(&mut self, pitch: f64) -> Option<(f32, f32)> {
        let n = self.pcm.frames();
        if n == 0 {
            return None;
        }
        if self.pos >= n as f64 {
            if self.looped {
                self.pos = self.pos.rem_euclid(n as f64);
            } else {
                return None;
            }
        }
        let i = self.pos as usize;
        let d = &self.pcm.data;
        let out = if pitch == 1.0 {
            (d[2 * i], d[2 * i + 1])
        } else {
            let f = (self.pos - i as f64) as f32;
            let j = if i + 1 < n { i + 1 } else if self.looped { 0 } else { i };
            (d[2 * i] + (d[2 * j] - d[2 * i]) * f, d[2 * i + 1] + (d[2 * j + 1] - d[2 * i + 1]) * f)
        };
        self.pos += pitch;
        Some(out)
    }
}

/// Streams through a decoder (multimedia sounds: no full decode in memory).
pub struct StreamGen {
    data: Arc<[u8]>,
    dec: Option<rodio::Decoder<std::io::Cursor<Arc<[u8]>>>>,
    ch: usize,
    rate: u32,
    looped: bool,
    produced: bool,
}

impl StreamGen {
    pub fn new(data: Arc<[u8]>, looped: bool) -> Option<Self> {
        use rodio::Source;
        let dec = rodio::Decoder::new(std::io::Cursor::new(data.clone())).ok()?;
        let ch = dec.channels().get() as usize;
        let rate = dec.sample_rate().get();
        Some(StreamGen { data, dec: Some(dec), ch, rate, looped, produced: false })
    }
}

impl Gen for StreamGen {
    fn rate(&self) -> u32 {
        self.rate.max(1)
    }
    fn next(&mut self, _pitch: f64) -> Option<(f32, f32)> {
        for _attempt in 0..2 {
            if let Some(dec) = self.dec.as_mut() {
                let a = dec.next();
                if let Some(a) = a {
                    let b = if self.ch >= 2 { dec.next().unwrap_or(a) } else { a };
                    for _ in 2..self.ch {
                        dec.next();
                    }
                    self.produced = true;
                    return Some((a, b));
                }
            }
            if !self.looped || !self.produced {
                return None;
            }
            self.dec = rodio::Decoder::new(std::io::Cursor::new(self.data.clone())).ok();
        }
        None
    }
}

/// Renders a MIDI sequence through a SoundFont synthesiser (fallback path).
pub struct SynthGen {
    pub player: SeqPlayer,
    pub synth: rustysynth::Synthesizer,
    pub globals: Arc<Globals>,
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    pos: usize,
    rate: u32,
    tail: usize,
}

impl SynthGen {
    pub const BLOCK: usize = 64;
    pub fn new(player: SeqPlayer, synth: rustysynth::Synthesizer, globals: Arc<Globals>, rate: u32) -> Self {
        SynthGen { player, synth, globals, buf_l: vec![0.0; Self::BLOCK], buf_r: vec![0.0; Self::BLOCK], pos: Self::BLOCK, rate, tail: 0 }
    }
    fn send(synth: &mut rustysynth::Synthesizer, o: Out) {
        match o {
            Out::Short(m, _) => synth.process_midi_message((m[0] & 0x0F) as i32, (m[0] & 0xF0) as i32, m[1] as i32, m[2] as i32),
            Out::SysEx(_) => {}
            Out::LoopReset => {
                synth.note_off_all(true);
                synth.reset_all_controllers();
            }
        }
    }
}

impl Gen for SynthGen {
    fn rate(&self) -> u32 {
        self.rate
    }
    fn next(&mut self, _pitch: f64) -> Option<(f32, f32)> {
        if self.pos >= Self::BLOCK {
            if self.player.finished {
                // let releases ring for up to 2 s, then stop
                self.tail += Self::BLOCK;
                if self.tail > self.rate as usize * 2 {
                    return None;
                }
            } else {
                let dt = Self::BLOCK as f64 / self.rate as f64 * self.globals.tempo();
                let synth = &mut self.synth;
                self.player.advance(dt, &mut |o| SynthGen::send(synth, o));
                if self.player.finished {
                    self.synth.note_off_all(false);
                }
            }
            self.synth.render(&mut self.buf_l, &mut self.buf_r);
            self.pos = 0;
            if self.player.finished && self.tail > 0 {
                let peak = self.buf_l.iter().chain(self.buf_r.iter()).fold(0f32, |m, x| m.max(x.abs()));
                if peak < 3.2e-5 {
                    return None;
                }
            }
        }
        let i = self.pos;
        self.pos += 1;
        Some((self.buf_l[i], self.buf_r[i]))
    }
}

/// One playing voice as a rodio source (interleaved stereo).
pub struct Voice {
    gen: Box<dyn Gen>,
    ctl: Arc<SoundCtl>,
    globals: Arc<Globals>,
    state: Arc<VoiceState>,
    chain: FxChain,
    fx_version: u64,
    rate: u32,
    mono: bool,
    /// Current and target gains (smoothed per block).
    gl: f32,
    gr: f32,
    tl: f32,
    tr: f32,
    doppler: f64,
    block_left: usize,
    pending_right: Option<f32>,
    /// Frames rendered since the input ended (effect tail).
    tail: Option<usize>,
    quiet: usize,
    ended: bool,
}

const VOICE_BLOCK: usize = 64;

impl Voice {
    pub fn new(gen: Box<dyn Gen>, ctl: Arc<SoundCtl>, globals: Arc<Globals>, state: Arc<VoiceState>) -> Self {
        let rate = gen.rate();
        let (fx, kind) = {
            let p = ctl.p.lock().unwrap();
            (if p.kind == 3 { EffectSettings::default() } else { p.effects.clone() }, p.kind)
        };
        let fx_version = ctl.fx_version.load(Ordering::Relaxed);
        let mut v = Voice {
            gen,
            chain: FxChain::new(&fx, rate),
            fx_version,
            ctl,
            globals,
            state,
            rate,
            mono: kind == 2,
            gl: 0.0,
            gr: 0.0,
            tl: 0.0,
            tr: 0.0,
            doppler: 1.0,
            block_left: 0,
            pending_right: None,
            tail: None,
            quiet: 0,
            ended: false,
        };
        v.refresh();
        v.gl = v.tl;
        v.gr = v.tr;
        v
    }

    fn refresh(&mut self) {
        let Ok(p) = self.ctl.p.try_lock() else { return };
        let m = mix_of(&p, self.globals.volume(), Instant::now());
        self.tl = m.left as f32;
        self.tr = m.right as f32;
        self.doppler = m.doppler;
        let ver = self.ctl.fx_version.load(Ordering::Relaxed);
        if ver != self.fx_version && p.kind != 3 {
            self.fx_version = ver;
            self.chain = FxChain::new(&p.effects, self.rate);
        }
    }

    fn end(&mut self) -> Option<f32> {
        self.ended = true;
        self.state.finish();
        None
    }

    /// Next stereo frame (after effects and gain), None when finished.
    pub fn next_frame(&mut self) -> Option<(f32, f32)> {
        if self.ended {
            return None;
        }
        if self.state.stop_requested() {
            self.end();
            return None;
        }
        if self.block_left == 0 {
            self.refresh();
            self.block_left = VOICE_BLOCK;
        }
        self.block_left -= 1;
        // gain smoothing towards the target (no zipper noise)
        const SMOOTH: f32 = 1.0 / 16.0;
        self.gl += (self.tl - self.gl) * SMOOTH;
        self.gr += (self.tr - self.gr) * SMOOTH;
        let input = if self.tail.is_none() {
            match self.gen.next(self.doppler) {
                Some(f) => Some(f),
                None => {
                    if !self.chain.has_tail() {
                        self.end();
                        return None;
                    }
                    self.tail = Some(0);
                    None
                }
            }
        } else {
            None
        };
        let (mut l, mut r) = input.unwrap_or((0.0, 0.0));
        if self.mono {
            let m = (l + r) * 0.5;
            l = m;
            r = m;
        }
        let (l, r) = self.chain.process(l, r);
        if let Some(t) = self.tail.as_mut() {
            *t += 1;
            if l.abs().max(r.abs()) < 3.2e-5 {
                self.quiet += 1;
            } else {
                self.quiet = 0;
            }
            // −90 dB for 50 ms, or a 3 s cap
            if self.quiet > self.rate as usize / 20 || *t > self.rate as usize * 3 {
                self.end();
                return None;
            }
        }
        Some((l * self.gl, r * self.gr))
    }
}

impl Iterator for Voice {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if let Some(r) = self.pending_right.take() {
            return Some(r);
        }
        match self.next_frame() {
            Some((l, r)) => {
                self.pending_right = Some(r);
                Some(l)
            }
            None => self.end(),
        }
    }
}

impl rodio::Source for Voice {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        rodio::ChannelCount::new(2).unwrap()
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        rodio::SampleRate::new(self.rate.max(1)).unwrap()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Drop for Voice {
    fn drop(&mut self) {
        self.state.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn fade_is_linear_in_real_time() {
        let t0 = Instant::now();
        let f = Fade { from: 1.0, to: 0.0, start: t0, dur: 2.0 };
        assert!((f.value_at(t0) - 1.0).abs() < 1e-9);
        assert!((f.value_at(t0 + Duration::from_millis(500)) - 0.75).abs() < 1e-9);
        assert!((f.value_at(t0 + Duration::from_millis(1000)) - 0.5).abs() < 1e-9);
        assert!((f.value_at(t0 + Duration::from_millis(2500)) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn fade_from_current_value_and_cancel() {
        let c = SoundCtl::new(0, 1.0, 0.0, 0);
        c.fade(0.0, 200.0);
        std::thread::sleep(Duration::from_millis(100));
        let v = c.volume_now();
        assert!(v > 0.25 && v < 0.75, "mid-fade volume {}", v);
        // a new fade starts from the current value
        c.fade(1.0, 10_000.0);
        let v2 = c.volume_now();
        assert!((v2 - v).abs() < 0.1, "{} vs {}", v2, v);
        // sound_volume cancels the fade
        c.set_volume(0.3);
        std::thread::sleep(Duration::from_millis(20));
        assert!((c.volume_now() - 0.3).abs() < 1e-9);
        // time ≤ 0: immediate
        c.fade(0.8, 0.0);
        assert!((c.volume_now() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn voice_gain_follows_fade() {
        // a constant 1.0 source; the rendered level must follow the fade in real time
        struct One;
        impl Gen for One {
            fn rate(&self) -> u32 {
                1000
            }
            fn next(&mut self, _: f64) -> Option<(f32, f32)> {
                Some((1.0, 1.0))
            }
        }
        let ctl = Arc::new(SoundCtl::new(0, 1.0, 0.0, 0));
        let g = Arc::new(Globals::default());
        let mut v = Voice::new(Box::new(One), ctl.clone(), g.clone(), Arc::new(VoiceState::default()));
        ctl.fade(0.0, 100.0);
        std::thread::sleep(Duration::from_millis(150));
        for _ in 0..1000 {
            v.next_frame();
        }
        let (l, r) = v.next_frame().unwrap();
        assert!(l.abs() < 1e-3 && r.abs() < 1e-3, "after fade: {} {}", l, r);
        g.set_volume(0.5);
        ctl.set_volume(1.0);
        for _ in 0..1000 {
            v.next_frame();
        }
        let (l, _) = v.next_frame().unwrap();
        assert!((l - 0.5).abs() < 1e-3, "global volume: {}", l);
    }

    #[test]
    fn attenuation_3d() {
        let mut s = Sound3d::default();
        s.mindist = 2.0;
        s.maxdist = 100.0;
        // inside mindist: full
        s.pos = [0.0, 0.0, 1.0];
        assert!((compute_3d(&s).0 - 1.0).abs() < 1e-9);
        // 1/d beyond mindist
        s.pos = [0.0, 0.0, 8.0];
        assert!((compute_3d(&s).0 - 0.25).abs() < 1e-9);
        // constant beyond maxdist
        s.pos = [0.0, 0.0, 1000.0];
        assert!((compute_3d(&s).0 - 0.02).abs() < 1e-9);
        // pan from x
        s.pos = [10.0, 0.0, 0.0];
        assert!((compute_3d(&s).1 - 1.0).abs() < 1e-9);
        s.pos = [-3.0, 0.0, 4.0];
        assert!((compute_3d(&s).1 + 0.6).abs() < 1e-9);
        // at the origin: centred, full
        s.pos = [0.0; 3];
        assert_eq!(compute_3d(&s), (1.0, 0.0, 1.0));
    }

    #[test]
    fn cone_3d() {
        let mut s = Sound3d { mindist: 100.0, ..Default::default() };
        // source at z=10 facing the listener (axis −z): inside the inner cone
        s.pos = [0.0, 0.0, 10.0];
        s.cone_axis = [0.0, 0.0, -1.0];
        s.angle_in = 90.0;
        s.angle_out = 180.0;
        s.vol_outside = -2000.0; // −20 dB
        assert!((compute_3d(&s).0 - 1.0).abs() < 1e-9);
        // facing away: outside the outer cone → −20 dB
        s.cone_axis = [0.0, 0.0, 1.0];
        assert!((compute_3d(&s).0 - 0.1).abs() < 1e-9);
        // sideways (90°): halfway between 45° and 90° half-angles → outer edge = −20 dB
        s.cone_axis = [1.0, 0.0, 0.0];
        assert!((compute_3d(&s).0 - 0.1).abs() < 1e-9);
        // 67.5°: −10 dB
        let a = 67.5f64.to_radians();
        s.cone_axis = [a.sin(), 0.0, -a.cos()];
        assert!((compute_3d(&s).0 - 10f64.powf(-0.5)).abs() < 1e-6);
    }

    #[test]
    fn doppler_and_pan_laws() {
        let s = Sound3d { pos: [0.0, 0.0, 10.0], vel: [0.0, 0.0, 34.3], ..Default::default() };
        let (_, _, d) = compute_3d(&s);
        assert!((d - 343.0 / 377.3).abs() < 1e-9 && d < 1.0, "receding lowers pitch");
        assert_eq!(linear_pan(0.0), (1.0, 1.0));
        assert_eq!(linear_pan(1.0), (0.0, 1.0));
        assert_eq!(linear_pan(-0.5), (1.0, 0.5));
        let (l, r) = equal_power_pan(0.0);
        assert!((l - 1.0).abs() < 1e-9 && (r - 1.0).abs() < 1e-9);
        let (l, r) = equal_power_pan(1.0);
        assert!(l.abs() < 1e-9 && (r - 1.0).abs() < 1e-9);
    }

    #[test]
    fn effect_tail_keeps_voice_alive() {
        let pcm = Arc::new(Pcm { rate: 1000, data: vec![0.5; 200] });
        let ctl = Arc::new(SoundCtl::new(0, 1.0, 0.0, super::super::dsp::SE_ECHO));
        ctl.update_effects(|e| e.set_params(super::super::dsp::SE_ECHO, &[50.0, 50.0, 100.0, 100.0, 0.0]));
        let st = Arc::new(VoiceState::default());
        let mut v = Voice::new(Box::new(PcmGen { pcm, pos: 0.0, looped: false }), ctl, Arc::new(Globals::default()), st.clone());
        let mut n = 0;
        while v.next_frame().is_some() {
            n += 1;
            assert!(n < 10_000);
        }
        assert!(n > 300, "echo tail rendered ({} frames)", n);
        assert!(st.is_done());
    }
}
