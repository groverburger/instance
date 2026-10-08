//! Sound effects (specs/sound.md §5): chorus, echo, flanger, gargle, reverb,
//! compressor and equalizer as stereo DSP on the PCM pipeline. These are
//! approximations of the classic DirectX effects, not bit-exact copies.

use std::f64::consts::PI;

pub const SE_CHORUS: i32 = 1;
pub const SE_ECHO: i32 = 2;
pub const SE_FLANGER: i32 = 4;
pub const SE_GARGLE: i32 = 8;
pub const SE_REVERB: i32 = 16;
pub const SE_COMPRESSOR: i32 = 32;
pub const SE_EQUALIZER: i32 = 64;
pub const SE_ALL: i32 = 127;

/// Chorus / flanger parameters (§5.2).
#[derive(Clone, Debug, PartialEq)]
pub struct ModDelayParams {
    pub wetdry: f64,
    pub depth: f64,
    pub feedback: f64,
    pub frequency: f64,
    pub wave: i32,
    pub delay: f64,
    pub phase: i32,
}

impl Default for ModDelayParams {
    fn default() -> Self {
        ModDelayParams { wetdry: 50.0, depth: 25.0, feedback: 0.0, frequency: 0.0, wave: 1, delay: 0.0, phase: 2 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EchoParams {
    pub wetdry: f64,
    pub feedback: f64,
    pub left: f64,
    pub right: f64,
    pub pandelay: bool,
}

impl Default for EchoParams {
    fn default() -> Self {
        EchoParams { wetdry: 50.0, feedback: 0.0, left: 333.0, right: 333.0, pandelay: false }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GargleParams {
    pub rate: f64,
    pub wave: i32,
}

impl Default for GargleParams {
    fn default() -> Self {
        GargleParams { rate: 1.0, wave: 0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReverbParams {
    pub gain: f64,
    pub mix: f64,
    pub time: f64,
    pub ratio: f64,
}

impl Default for ReverbParams {
    fn default() -> Self {
        ReverbParams { gain: 0.0, mix: 0.0, time: 1000.0, ratio: 0.001 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompressorParams {
    pub gain: f64,
    pub attack: f64,
    pub release: f64,
    pub threshold: f64,
    pub ratio: f64,
    pub delay: f64,
}

impl Default for CompressorParams {
    fn default() -> Self {
        CompressorParams { gain: 0.0, attack: 0.01, release: 50.0, threshold: -10.0, ratio: 10.0, delay: 0.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EqualizerParams {
    pub center: f64,
    pub bandwidth: f64,
    pub gain: f64,
}

impl Default for EqualizerParams {
    fn default() -> Self {
        EqualizerParams { center: 8000.0, bandwidth: 12.0, gain: 0.0 }
    }
}

/// The effect combination of a sound plus every effect's parameters.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectSettings {
    pub flags: i32,
    pub chorus: ModDelayParams,
    pub echo: EchoParams,
    pub flanger: ModDelayParams,
    pub gargle: GargleParams,
    pub reverb: ReverbParams,
    pub compressor: CompressorParams,
    pub equalizer: EqualizerParams,
}

fn arg(a: &[f64], i: usize, lo: f64, hi: f64) -> f64 {
    let v = a.get(i).copied().unwrap_or(0.0);
    if v.is_nan() {
        lo
    } else {
        v.clamp(lo, hi)
    }
}

fn iarg(a: &[f64], i: usize, lo: i32, hi: i32) -> i32 {
    (arg(a, i, lo as f64, hi as f64).round() as i32).clamp(lo, hi)
}

impl EffectSettings {
    pub fn with_flags(flags: i32) -> Self {
        EffectSettings { flags: flags & SE_ALL, ..Default::default() }
    }

    /// `sound_effect_set`: new combination, all parameters back to defaults.
    pub fn set(&mut self, flags: i32) {
        *self = EffectSettings::with_flags(flags);
    }

    /// Sets one effect's parameters (clamped). Returns false (and changes nothing)
    /// when the effect is not selected (§5.1).
    pub fn set_params(&mut self, effect: i32, a: &[f64]) -> bool {
        if self.flags & effect == 0 {
            return false;
        }
        let md = |a: &[f64]| ModDelayParams {
            wetdry: arg(a, 0, 0.0, 100.0),
            depth: arg(a, 1, 0.0, 100.0),
            feedback: arg(a, 2, -99.0, 99.0),
            frequency: arg(a, 3, 0.0, 10.0),
            wave: iarg(a, 4, 0, 1),
            delay: arg(a, 5, 0.0, 20.0),
            phase: iarg(a, 6, 0, 4),
        };
        match effect {
            SE_CHORUS => self.chorus = md(a),
            SE_FLANGER => self.flanger = md(a),
            SE_ECHO => {
                self.echo = EchoParams {
                    wetdry: arg(a, 0, 0.0, 100.0),
                    feedback: arg(a, 1, 0.0, 100.0),
                    left: arg(a, 2, 1.0, 2000.0),
                    right: arg(a, 3, 1.0, 2000.0),
                    pandelay: iarg(a, 4, 0, 1) == 1,
                }
            }
            SE_GARGLE => self.gargle = GargleParams { rate: arg(a, 0, 1.0, 1000.0), wave: iarg(a, 1, 0, 1) },
            SE_REVERB => {
                self.reverb = ReverbParams {
                    gain: arg(a, 0, -96.0, 0.0),
                    mix: arg(a, 1, -96.0, 0.0),
                    time: arg(a, 2, 0.001, 3000.0),
                    ratio: arg(a, 3, 0.001, 0.999),
                }
            }
            SE_COMPRESSOR => {
                self.compressor = CompressorParams {
                    gain: arg(a, 0, -60.0, 60.0),
                    attack: arg(a, 1, 0.01, 500.0),
                    release: arg(a, 2, 50.0, 3000.0),
                    threshold: arg(a, 3, -60.0, 0.0),
                    ratio: arg(a, 4, 1.0, 100.0),
                    delay: arg(a, 5, 0.0, 4.0),
                }
            }
            SE_EQUALIZER => {
                self.equalizer = EqualizerParams {
                    center: arg(a, 0, 80.0, 16000.0),
                    bandwidth: arg(a, 1, 1.0, 36.0),
                    gain: arg(a, 2, -15.0, 15.0),
                }
            }
            _ => return false,
        }
        true
    }
}

// ------------------------------------------------------------------ building blocks

/// A ring-buffer delay line with fractional (linear-interpolated) reads.
struct DelayLine {
    buf: Vec<f32>,
    pos: usize,
}

impl DelayLine {
    fn new(len: usize) -> Self {
        DelayLine { buf: vec![0.0; len.max(3)], pos: 0 }
    }
    /// Sample written `d` samples ago (d ≥ 1).
    fn read(&self, d: f64) -> f32 {
        let n = self.buf.len();
        let d = d.clamp(1.0, (n - 2) as f64);
        let i = d.floor() as usize;
        let f = (d - i as f64) as f32;
        // `pos` holds the newest sample (delay 1 once the next write happens)
        let a = self.buf[(self.pos + n + 1 - i) % n];
        let b = self.buf[(self.pos + n - i) % n];
        a + (b - a) * f
    }
    fn write(&mut self, x: f32) {
        self.pos = (self.pos + 1) % self.buf.len();
        self.buf[self.pos] = x;
    }
}

fn lfo(wave: i32, phase: f64) -> f64 {
    // phase in cycles; output −1..1
    let p = phase.rem_euclid(1.0);
    if wave == 0 {
        1.0 - 4.0 * (p - 0.5).abs()
    } else {
        (2.0 * PI * p).sin()
    }
}

trait Fx: Send {
    fn process(&mut self, l: f32, r: f32) -> (f32, f32);
}

struct ModDelay {
    p: ModDelayParams,
    lines: [DelayLine; 2],
    phase: f64,
    inc: f64,
    base: f64,
    range: f64,
    phase_off: f64,
}

impl ModDelay {
    fn new(p: &ModDelayParams, rate: f64, flanger: bool) -> Self {
        let ms = rate / 1000.0;
        let range_ms = if flanger { 4.0 } else { 10.0 };
        let len = ((20.0 + range_ms + 2.0) * ms) as usize + 4;
        ModDelay {
            p: p.clone(),
            lines: [DelayLine::new(len), DelayLine::new(len)],
            phase: 0.0,
            inc: p.frequency / rate,
            base: (p.delay * ms).max(1.0),
            range: p.depth / 100.0 * range_ms * ms,
            phase_off: (p.phase as f64 - 2.0) * 0.25,
        }
    }
}

impl Fx for ModDelay {
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let fb = (self.p.feedback / 100.0).clamp(-0.98, 0.98) as f32;
        let w = (self.p.wetdry / 100.0) as f32;
        let mut out = [0f32; 2];
        let x = [l, r];
        for c in 0..2 {
            let m = lfo(self.p.wave, self.phase + if c == 1 { self.phase_off } else { 0.0 });
            let d = self.base + self.range * (m + 1.0) * 0.5;
            let y = self.lines[c].read(d);
            self.lines[c].write(x[c] + fb * y);
            out[c] = x[c] * (1.0 - w) + y * w;
        }
        self.phase = (self.phase + self.inc).rem_euclid(1.0);
        (out[0], out[1])
    }
}

struct Echo {
    p: EchoParams,
    lines: [DelayLine; 2],
    dl: f64,
    dr: f64,
}

impl Echo {
    fn new(p: &EchoParams, rate: f64) -> Self {
        let len = (2001.0 * rate / 1000.0) as usize + 4;
        Echo {
            p: p.clone(),
            lines: [DelayLine::new(len), DelayLine::new(len)],
            dl: p.left * rate / 1000.0,
            dr: p.right * rate / 1000.0,
        }
    }
}

impl Fx for Echo {
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let fb = (self.p.feedback / 100.0).min(0.98) as f32;
        let w = (self.p.wetdry / 100.0) as f32;
        let yl = self.lines[0].read(self.dl);
        let yr = self.lines[1].read(self.dr);
        if self.p.pandelay {
            self.lines[0].write(l + fb * yr);
            self.lines[1].write(r + fb * yl);
        } else {
            self.lines[0].write(l + fb * yl);
            self.lines[1].write(r + fb * yr);
        }
        (l * (1.0 - w) + yl * w, r * (1.0 - w) + yr * w)
    }
}

struct Gargle {
    p: GargleParams,
    phase: f64,
    inc: f64,
}

impl Fx for Gargle {
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let m = if self.p.wave == 1 {
            if self.phase < 0.5 {
                1.0
            } else {
                0.0
            }
        } else {
            1.0 - (2.0 * self.phase - 1.0).abs()
        } as f32;
        self.phase = (self.phase + self.inc).rem_euclid(1.0);
        (l * m, r * m)
    }
}

struct Comb {
    buf: Vec<f32>,
    pos: usize,
    fb: f32,
    damp: f32,
    store: f32,
}

impl Comb {
    fn process(&mut self, x: f32) -> f32 {
        let y = self.buf[self.pos];
        self.store = y * (1.0 - self.damp) + self.store * self.damp;
        self.buf[self.pos] = x + self.store * self.fb;
        self.pos = (self.pos + 1) % self.buf.len();
        y
    }
}

struct Allpass {
    buf: Vec<f32>,
    pos: usize,
}

impl Allpass {
    fn process(&mut self, x: f32) -> f32 {
        let b = self.buf[self.pos];
        let y = b - x;
        self.buf[self.pos] = x + b * 0.5;
        self.pos = (self.pos + 1) % self.buf.len();
        y
    }
}

/// Freeverb-style network: 8 damped combs + 4 allpasses per channel.
struct Reverb {
    combs: [Vec<Comb>; 2],
    alls: [Vec<Allpass>; 2],
    in_gain: f32,
    wet: f32,
}

impl Reverb {
    fn new(p: &ReverbParams, rate: f64) -> Self {
        const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        const ALLS: [usize; 4] = [556, 441, 341, 225];
        let scale = rate / 44100.0;
        let t = (p.time / 1000.0).max(0.001);
        // high-frequency decay ratio: small ratio = highs die much faster
        let damp = (0.45 * (1.0 - p.ratio)) as f32;
        let mk = |spread: usize| -> (Vec<Comb>, Vec<Allpass>) {
            let combs = COMBS
                .iter()
                .map(|&n| {
                    let len = (((n + spread) as f64) * scale) as usize;
                    let len = len.max(1);
                    // RT60: gain^(t/len) = 10^-3
                    let fb = 10f64.powf(-3.0 * (len as f64 / rate) / t).min(0.98) as f32;
                    Comb { buf: vec![0.0; len], pos: 0, fb, damp, store: 0.0 }
                })
                .collect();
            let alls = ALLS
                .iter()
                .map(|&n| Allpass { buf: vec![0.0; ((((n + spread) as f64) * scale) as usize).max(1)], pos: 0 })
                .collect();
            (combs, alls)
        };
        let (cl, al) = mk(0);
        let (cr, ar) = mk(23);
        Reverb {
            combs: [cl, cr],
            alls: [al, ar],
            in_gain: (10f64.powf(p.gain / 20.0) * 0.015) as f32,
            wet: (10f64.powf(p.mix / 20.0) * 0.6) as f32,
        }
    }
}

impl Fx for Reverb {
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let input = (l + r) * self.in_gain;
        let mut out = [0f32; 2];
        for c in 0..2 {
            let mut acc = 0.0;
            for cb in self.combs[c].iter_mut() {
                acc += cb.process(input);
            }
            for ap in self.alls[c].iter_mut() {
                acc = ap.process(acc);
            }
            out[c] = acc;
        }
        (l * 0.8 + out[0] * self.wet, r * 0.8 + out[1] * self.wet)
    }
}

struct Compressor {
    p: CompressorParams,
    env: f64,
    att: f64,
    rel: f64,
    look: [DelayLine; 2],
    look_d: f64,
}

impl Compressor {
    fn new(p: &CompressorParams, rate: f64) -> Self {
        let coef = |ms: f64| (-1.0 / (ms * rate / 1000.0).max(1e-3)).exp();
        let look_d = p.delay * rate / 1000.0;
        let len = look_d as usize + 4;
        Compressor { p: p.clone(), env: 0.0, att: coef(p.attack), rel: coef(p.release), look: [DelayLine::new(len), DelayLine::new(len)], look_d }
    }
}

impl Fx for Compressor {
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let x = l.abs().max(r.abs()) as f64;
        let c = if x > self.env { self.att } else { self.rel };
        self.env = c * self.env + (1.0 - c) * x;
        let env_db = 20.0 * (self.env + 1e-9).log10();
        let over = env_db - self.p.threshold;
        let gr = if over > 0.0 { over * (1.0 - 1.0 / self.p.ratio) } else { 0.0 };
        let g = 10f64.powf((self.p.gain - gr) / 20.0) as f32;
        let (dl, dr) = if self.look_d >= 1.0 {
            let a = self.look[0].read(self.look_d);
            let b = self.look[1].read(self.look_d);
            self.look[0].write(l);
            self.look[1].write(r);
            (a, b)
        } else {
            (l, r)
        };
        (dl * g, dr * g)
    }
}

/// RBJ peaking biquad.
struct Equalizer {
    b: [f64; 3],
    a: [f64; 2],
    z: [[f64; 2]; 2],
}

impl Equalizer {
    fn new(p: &EqualizerParams, rate: f64) -> Self {
        let f0 = p.center.min(rate * 0.45);
        let a_ = 10f64.powf(p.gain / 40.0);
        let w0 = 2.0 * PI * f0 / rate;
        let bw_oct = p.bandwidth / 12.0;
        let alpha = w0.sin() * ((2f64.ln() / 2.0) * bw_oct * w0 / w0.sin()).sinh();
        let a0 = 1.0 + alpha / a_;
        Equalizer {
            b: [(1.0 + alpha * a_) / a0, (-2.0 * w0.cos()) / a0, (1.0 - alpha * a_) / a0],
            a: [(-2.0 * w0.cos()) / a0, (1.0 - alpha / a_) / a0],
            z: [[0.0; 2]; 2],
        }
    }
}

impl Fx for Equalizer {
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let mut out = [0f32; 2];
        for (c, x) in [l as f64, r as f64].into_iter().enumerate() {
            // transposed direct form II
            let z = &mut self.z[c];
            let y = self.b[0] * x + z[0];
            z[0] = self.b[1] * x - self.a[0] * y + z[1];
            z[1] = self.b[2] * x - self.a[1] * y;
            out[c] = y as f32;
        }
        (out[0], out[1])
    }
}

/// Soft limiter applied after the effects: transparent below 0.9, bounded by 1.
#[inline]
pub fn soft_limit(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.9 {
        x
    } else {
        x.signum() * (0.9 + 0.1 * ((a - 0.9) / 0.1).tanh())
    }
}

/// An effect chain built from [`EffectSettings`], processing order chorus → echo
/// → flanger → gargle → reverb → compressor → equalizer.
pub struct FxChain {
    fx: Vec<Box<dyn Fx>>,
    settings: EffectSettings,
    rate: f64,
}

impl FxChain {
    pub fn new(s: &EffectSettings, rate: u32) -> Self {
        let rate = rate.max(1) as f64;
        let mut fx: Vec<Box<dyn Fx>> = Vec::new();
        let f = s.flags;
        if f & SE_CHORUS != 0 {
            fx.push(Box::new(ModDelay::new(&s.chorus, rate, false)));
        }
        if f & SE_ECHO != 0 {
            fx.push(Box::new(Echo::new(&s.echo, rate)));
        }
        if f & SE_FLANGER != 0 {
            fx.push(Box::new(ModDelay::new(&s.flanger, rate, true)));
        }
        if f & SE_GARGLE != 0 {
            fx.push(Box::new(Gargle { p: s.gargle.clone(), phase: 0.0, inc: s.gargle.rate / rate }));
        }
        if f & SE_REVERB != 0 {
            fx.push(Box::new(Reverb::new(&s.reverb, rate)));
        }
        if f & SE_COMPRESSOR != 0 {
            fx.push(Box::new(Compressor::new(&s.compressor, rate)));
        }
        if f & SE_EQUALIZER != 0 {
            fx.push(Box::new(Equalizer::new(&s.equalizer, rate)));
        }
        FxChain { fx, settings: s.clone(), rate }
    }

    pub fn is_empty(&self) -> bool {
        self.fx.is_empty()
    }

    /// Whether the chain has effects that ring on after the input stops.
    pub fn has_tail(&self) -> bool {
        self.settings.flags & (SE_CHORUS | SE_ECHO | SE_FLANGER | SE_REVERB | SE_COMPRESSOR | SE_EQUALIZER) != 0
    }

    /// Effects without the final limiter (tests).
    pub fn process_raw(&mut self, mut l: f32, mut r: f32) -> (f32, f32) {
        for f in self.fx.iter_mut() {
            (l, r) = f.process(l, r);
        }
        (l, r)
    }

    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        if self.fx.is_empty() {
            return (l, r);
        }
        let (l, r) = self.process_raw(l, r);
        if !l.is_finite() || !r.is_finite() {
            // never let a bad state persist
            *self = FxChain::new(&self.settings.clone(), self.rate as u32);
            return (0.0, 0.0);
        }
        (soft_limit(l), soft_limit(r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(chain: &mut FxChain, secs: f64, rate: u32, amp: f32, noise: bool) -> (f32, f32) {
        let mut seed: u32 = 12345;
        let mut peak = 0f32;
        let mut raw_peak = 0f32;
        let n = (secs * rate as f64) as usize;
        for i in 0..n {
            let x = if noise {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0) * amp
            } else {
                (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / rate as f32).sin() * amp
            };
            let (l, r) = chain.process(x, -x);
            assert!(l.is_finite() && r.is_finite(), "NaN/inf at sample {}", i);
            peak = peak.max(l.abs()).max(r.abs());
            let _ = &mut raw_peak;
        }
        (peak, raw_peak)
    }

    #[test]
    fn every_effect_finite_and_bounded() {
        for flags in 1..=SE_ALL {
            let mut s = EffectSettings::with_flags(flags);
            let mut c = FxChain::new(&s, 44100);
            let (peak, _) = run(&mut c, 0.5, 44100, 0.5, true);
            assert!(peak <= 1.0, "flags {} peak {}", flags, peak);
            // extreme parameters
            s.set_params(SE_CHORUS, &[100.0, 100.0, 99.0, 10.0, 0.0, 20.0, 4.0]);
            s.set_params(SE_FLANGER, &[100.0, 100.0, -99.0, 10.0, 1.0, 0.0, 0.0]);
            s.set_params(SE_ECHO, &[100.0, 100.0, 1.0, 2000.0, 1.0]);
            s.set_params(SE_GARGLE, &[1000.0, 1.0]);
            s.set_params(SE_REVERB, &[0.0, 0.0, 3000.0, 0.999]);
            s.set_params(SE_COMPRESSOR, &[60.0, 500.0, 3000.0, -60.0, 1.0, 4.0]);
            s.set_params(SE_EQUALIZER, &[16000.0, 36.0, 15.0]);
            let mut c = FxChain::new(&s, 22050);
            let (peak, _) = run(&mut c, 0.5, 22050, 1.0, false);
            assert!(peak <= 1.0, "extreme flags {} peak {}", flags, peak);
        }
    }

    #[test]
    fn defaults_do_not_clip_moderate_input() {
        // each single effect with default parameters on a 0.4 sine stays below
        // full scale before the safety limiter
        for bit in 0..7 {
            let s = EffectSettings::with_flags(1 << bit);
            let mut c = FxChain::new(&s, 44100);
            let mut peak = 0f32;
            for i in 0..44100 {
                let x = (i as f32 * 220.0 * 2.0 * std::f32::consts::PI / 44100.0).sin() * 0.4;
                let (l, r) = c.process_raw(x, x);
                assert!(l.is_finite() && r.is_finite());
                peak = peak.max(l.abs()).max(r.abs());
            }
            assert!(peak < 1.0, "effect {} peak {}", 1 << bit, peak);
            assert!(peak > 0.01, "effect {} silent", 1 << bit);
        }
    }

    #[test]
    fn params_only_for_selected_and_clamped() {
        let mut s = EffectSettings::with_flags(SE_ECHO);
        assert!(!s.set_params(SE_REVERB, &[-10.0, -10.0, 100.0, 0.5]));
        assert_eq!(s.reverb, ReverbParams::default());
        assert!(s.set_params(SE_ECHO, &[150.0, -5.0, 0.0, 5000.0, 3.0]));
        assert_eq!(s.echo, EchoParams { wetdry: 100.0, feedback: 0.0, left: 1.0, right: 2000.0, pandelay: true });
        s.set(SE_ECHO | SE_GARGLE);
        assert_eq!(s.echo, EchoParams::default());
    }

    #[test]
    fn echo_delays_signal() {
        let mut s = EffectSettings::with_flags(SE_ECHO);
        s.set_params(SE_ECHO, &[100.0, 0.0, 10.0, 10.0, 0.0]);
        let mut c = FxChain::new(&s, 1000);
        let mut out = Vec::new();
        for i in 0..20 {
            out.push(c.process(if i == 0 { 0.5 } else { 0.0 }, 0.0).0);
        }
        let k = out.iter().position(|&v| v.abs() > 0.1).unwrap();
        assert_eq!(k, 10);
    }

    #[test]
    fn reverb_has_tail_that_decays() {
        let mut s = EffectSettings::with_flags(SE_REVERB);
        s.set_params(SE_REVERB, &[0.0, 0.0, 200.0, 0.5]);
        let mut c = FxChain::new(&s, 44100);
        for i in 0..4410 {
            let x = (i as f32 * 0.1).sin() * 0.5;
            c.process(x, x);
        }
        let mut early = 0f32;
        for _ in 0..4410 {
            early = early.max(c.process(0.0, 0.0).0.abs());
        }
        let mut late = 0f32;
        for _ in 0..44100 {
            late = c.process(0.0, 0.0).0.abs();
        }
        assert!(early > 1e-3, "no tail");
        assert!(late < 1e-4, "tail does not decay: {}", late);
    }
}
