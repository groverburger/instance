//! Built-in synthesiser for WAV renders. A track declared with
//! `synth=PRESET` is played by this engine instead of the SoundFont: band-
//! limited oscillators with unison detune, a resonant 4-pole ladder filter
//! with its own envelope, FM voices, a drum machine, per-track stereo chorus,
//! a tempo-synced ping-pong delay, a hall reverb and kick-triggered sidechain
//! ducking. MIDI output is unaffected (the track's `inst=` program, or a GM
//! stand-in for the preset, is written instead).
//!
//! The engine takes the same channel messages as the SoundFont synthesiser,
//! so inline controls drive it too: `cutoff=` (CC74), `reso=` (CC71),
//! `attack=` (CC73), `release=` (CC72), `delay=` (CC94), `mod=` (vibrato),
//! `vol` `expr` `pan` `reverb` `chorus`, pitch bend and the sustain pedal.

use crate::compile::Song;
use std::f64::consts::{PI, TAU};

/// (name, description, GM program used for MIDI output)
pub const PRESETS: &[(&str, &str, u8)] = &[
    ("supersaw", "7 detuned saws, wide and bright: anthem leads, big chords", 81),
    ("lead", "3 saws, mono with glide and delayed vibrato: the classic synthwave lead", 81),
    ("square", "pulse-width-modulated square, slight detune: retro lead / counter-melody", 80),
    ("pluck", "saw + square with a snappy filter envelope: arpeggios", 84),
    ("pad", "4 detuned saws, slow attack, drifting filter, chorus: warm pads", 89),
    ("strings", "string machine: soft saws through a lush chorus", 50),
    ("brass", "2 saws with a swelling filter: synth-brass stabs", 62),
    ("bass", "mono saw + sub square, punchy filter: octave and pumping bass", 38),
    ("reese", "2 detuned saws, low-passed and driven: dark sustained bass", 39),
    ("sub", "pure sine sub bass", 38),
    ("bell", "2-operator FM bell: glassy, long", 98),
    ("epiano", "FM electric piano (DX-style tine)", 4),
    ("riser", "noise swept upward by the filter: risers, swooshes", 122),
    ("drums", "drum machine: 808/Linn-style kick, gated-reverb snare, clap, hats, toms, cymbals (drum tracks only)", 24),
];

/// Track options read by the engine (besides the shared controls), as
/// (name, range, meaning) for messages and the README.
pub const OPTIONS: &[(&str, &str)] = &[
    ("cutoff", "0..127 filter cutoff, 20 Hz * 1000^(v/127): 64 = 650 Hz, 90 = 2.7 kHz, 110 = 8 kHz"),
    ("reso", "0..127 filter resonance (127 = on the edge of self-oscillation)"),
    ("envamt", "-8..8 filter envelope amount, octaves"),
    ("attack", "0..127 amp attack, 1 ms * 10000^(v/127): 0 = 1 ms, 64 = 100 ms, 100 = 1.4 s"),
    ("decay", "0..127 amp + filter decay (same curve)"),
    ("sustain", "0..127 amp sustain level"),
    ("release", "0..127 amp release (same curve)"),
    ("detune", "0..100 unison spread, cents"),
    ("unison", "1..8 oscillators per note"),
    ("width", "0..100 stereo spread of the unison"),
    ("glide", "0..1000 portamento, ms (mono presets glide between overlapping or adjacent notes)"),
    ("drive", "0..100 filter drive / saturation"),
    ("duck", "0..100 sidechain: how far the kick pushes this track down"),
    ("delay", "0..127 send to the ping-pong delay (dotted eighths)"),
    ("vib", "0..100 vibrato depth, cents"),
];

pub fn preset_names() -> String {
    PRESETS.iter().map(|p| p.0).collect::<Vec<_>>().join(" ")
}

/// GM program that stands in for a preset in MIDI files.
pub fn gm_fallback(name: &str) -> u8 {
    PRESETS.iter().find(|p| p.0 == name).map(|p| p.2).unwrap_or(80)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wave {
    Saw,
    Square,
    Tri,
    Sine,
    Fm,
    Noise,
    Drums,
}

/// A synth patch (times in seconds).
#[derive(Clone, Debug)]
pub struct Patch {
    pub wave: Wave,
    /// Second oscillator: (wave, level, semitones).
    pub wave2: Option<(Wave, f64, f64)>,
    pub unison: usize,
    pub detune: f64,
    pub width: f64,
    pub pw: f64,
    pub pwm: f64,
    pub pwm_rate: f64,
    pub sub: f64,
    pub noise: f64,
    pub fm_ratio: f64,
    pub fm_index: f64,
    pub fm_decay: f64,
    /// Tine: (ratio, index, decay).
    pub fm_tine: Option<(f64, f64, f64)>,
    pub cutoff: f64,
    pub reso: f64,
    pub envamt: f64,
    pub keytrack: f64,
    pub velfilt: f64,
    pub f_adsr: [f64; 4],
    pub a_adsr: [f64; 4],
    pub glide: f64,
    pub mono: bool,
    pub vib: f64,
    pub vib_rate: f64,
    pub vib_delay: f64,
    pub flfo: f64,
    pub flfo_rate: f64,
    pub drive: f64,
    pub duck: f64,
    pub delay: f64,
    pub chorus: f64,
    pub gain: f64,
}

impl Patch {
    fn base() -> Patch {
        Patch {
            wave: Wave::Saw,
            wave2: None,
            unison: 1,
            detune: 0.0,
            width: 0.5,
            pw: 0.5,
            pwm: 0.0,
            pwm_rate: 0.5,
            sub: 0.0,
            noise: 0.0,
            fm_ratio: 1.0,
            fm_index: 0.0,
            fm_decay: 1.0,
            fm_tine: None,
            cutoff: 100.0,
            reso: 10.0,
            envamt: 0.0,
            keytrack: 0.5,
            velfilt: 1.0,
            f_adsr: [0.005, 0.3, 0.0, 0.3],
            a_adsr: [0.005, 0.3, 1.0, 0.2],
            glide: 0.0,
            mono: false,
            vib: 0.0,
            vib_rate: 5.5,
            vib_delay: 0.3,
            flfo: 0.0,
            flfo_rate: 0.2,
            drive: 0.2,
            duck: 0.0,
            delay: 0.0,
            chorus: 0.0,
            gain: 1.0,
        }
    }

    pub fn preset(name: &str) -> Option<Patch> {
        let b = Patch::base();
        Some(match name {
            "supersaw" => Patch { unison: 7, detune: 28.0, width: 0.9, cutoff: 106.0, reso: 12.0, envamt: 1.5, f_adsr: [0.003, 0.5, 0.3, 0.4], a_adsr: [0.004, 0.4, 0.85, 0.35], delay: 0.12, gain: 0.85, ..b },
            "lead" => Patch {
                unison: 3,
                detune: 12.0,
                width: 0.45,
                wave2: Some((Wave::Square, 0.35, -12.0)),
                cutoff: 102.0,
                reso: 22.0,
                envamt: 1.6,
                f_adsr: [0.003, 0.35, 0.35, 0.3],
                a_adsr: [0.006, 0.4, 0.9, 0.25],
                mono: true,
                glide: 0.045,
                vib: 14.0,
                vib_delay: 0.25,
                delay: 0.28,
                gain: 1.0,
                ..b
            },
            "square" => Patch { wave: Wave::Square, unison: 2, detune: 8.0, width: 0.6, pw: 0.42, pwm: 0.12, pwm_rate: 0.7, cutoff: 96.0, reso: 15.0, envamt: 1.0, a_adsr: [0.004, 0.3, 0.8, 0.2], vib: 8.0, delay: 0.25, gain: 0.8, ..b },
            "pluck" => Patch {
                unison: 2,
                detune: 9.0,
                width: 0.7,
                wave2: Some((Wave::Square, 0.5, 0.0)),
                cutoff: 52.0,
                reso: 40.0,
                envamt: 4.5,
                f_adsr: [0.001, 0.16, 0.0, 0.15],
                a_adsr: [0.001, 0.38, 0.0, 0.18],
                delay: 0.32,
                gain: 1.1,
                ..b
            },
            "pad" => Patch {
                unison: 4,
                detune: 16.0,
                width: 0.85,
                cutoff: 70.0,
                reso: 18.0,
                envamt: 1.2,
                f_adsr: [0.9, 1.5, 0.4, 1.2],
                a_adsr: [0.55, 1.0, 0.9, 1.4],
                flfo: 0.6,
                flfo_rate: 0.13,
                chorus: 0.6,
                keytrack: 0.3,
                velfilt: 0.4,
                gain: 0.75,
                ..b
            },
            "strings" => Patch { unison: 3, detune: 10.0, width: 0.8, cutoff: 80.0, reso: 5.0, a_adsr: [0.25, 0.5, 0.95, 0.9], chorus: 0.9, velfilt: 0.5, gain: 0.7, ..b },
            "brass" => Patch {
                unison: 2,
                detune: 10.0,
                width: 0.5,
                cutoff: 62.0,
                reso: 15.0,
                envamt: 3.0,
                f_adsr: [0.07, 0.4, 0.35, 0.2],
                a_adsr: [0.012, 0.3, 0.85, 0.18],
                drive: 0.4,
                gain: 0.85,
                ..b
            },
            "bass" => Patch {
                wave2: Some((Wave::Square, 0.0, 0.0)),
                sub: 0.55,
                cutoff: 50.0,
                reso: 30.0,
                envamt: 3.2,
                keytrack: 0.4,
                f_adsr: [0.001, 0.14, 0.15, 0.08],
                a_adsr: [0.001, 0.25, 0.75, 0.05],
                mono: true,
                drive: 0.5,
                width: 0.0,
                gain: 1.2,
                ..b
            },
            "reese" => Patch { unison: 2, detune: 22.0, width: 0.3, sub: 0.4, cutoff: 56.0, reso: 20.0, envamt: 0.8, f_adsr: [0.01, 0.6, 0.6, 0.2], a_adsr: [0.004, 0.3, 0.9, 0.1], mono: true, glide: 0.03, drive: 0.7, gain: 1.0, ..b },
            "sub" => Patch { wave: Wave::Sine, cutoff: 127.0, reso: 0.0, a_adsr: [0.003, 0.3, 1.0, 0.06], mono: true, drive: 0.0, width: 0.0, gain: 1.3, ..b },
            "bell" => Patch { wave: Wave::Fm, fm_ratio: 3.5, fm_index: 2.5, fm_decay: 0.5, cutoff: 127.0, reso: 0.0, a_adsr: [0.001, 2.5, 0.0, 1.2], delay: 0.3, gain: 0.7, drive: 0.0, ..b },
            "epiano" => Patch {
                wave: Wave::Fm,
                fm_ratio: 1.0,
                fm_index: 1.4,
                fm_decay: 0.6,
                fm_tine: Some((14.0, 0.9, 0.04)),
                cutoff: 115.0,
                reso: 0.0,
                a_adsr: [0.001, 1.6, 0.25, 0.4],
                chorus: 0.5,
                gain: 0.8,
                drive: 0.0,
                ..b
            },
            "riser" => Patch { wave: Wave::Noise, cutoff: 30.0, reso: 60.0, envamt: 7.0, keytrack: 0.0, velfilt: 0.0, f_adsr: [1.6, 0.2, 1.0, 0.3], a_adsr: [1.4, 0.2, 1.0, 0.3], width: 0.0, chorus: 0.4, gain: 0.7, drive: 0.0, ..b },
            "drums" => Patch { wave: Wave::Drums, gain: 1.0, ..b },
            _ => return None,
        })
    }

    /// Applies a track option (see [`OPTIONS`]). Returns an error message for unknown names.
    pub fn apply(&mut self, k: &str, v: f64) -> Result<(), String> {
        match k {
            "cutoff" => self.cutoff = v.clamp(0.0, 127.0),
            "reso" => self.reso = v.clamp(0.0, 127.0),
            "envamt" => self.envamt = v.clamp(-8.0, 8.0),
            "attack" => {
                self.a_adsr[0] = cc_time(v);
            }
            "decay" => {
                self.a_adsr[1] = cc_time(v);
                self.f_adsr[1] = cc_time(v);
            }
            "sustain" => self.a_adsr[2] = (v / 127.0).clamp(0.0, 1.0),
            "release" => self.a_adsr[3] = cc_time(v),
            "detune" => self.detune = v.clamp(0.0, 100.0),
            "unison" => self.unison = (v.round() as usize).clamp(1, 8),
            "width" => self.width = (v / 100.0).clamp(0.0, 1.0),
            "glide" => self.glide = (v / 1000.0).clamp(0.0, 1.0),
            "drive" => self.drive = (v / 100.0).clamp(0.0, 1.0),
            "duck" => self.duck = (v / 100.0).clamp(0.0, 1.0),
            "delay" => self.delay = (v / 127.0).clamp(0.0, 1.0),
            "vib" => self.vib = v.clamp(0.0, 100.0),
            _ => return Err(format!("unknown synth option '{k}'")),
        }
        Ok(())
    }
}

/// Builds a track's patch: preset plus options.
pub fn patch_for(name: &str, opts: &[(String, f64)]) -> Result<Patch, String> {
    let mut p = Patch::preset(name).ok_or_else(|| format!("unknown synth preset '{name}' (known: {})", preset_names()))?;
    for (k, v) in opts {
        p.apply(k, *v)?;
    }
    Ok(p)
}

/// Whether a track option name belongs to the synth.
pub fn is_option(k: &str) -> bool {
    OPTIONS.iter().any(|o| o.0 == k)
}

/// CC value (0..127) to seconds for envelope times.
pub fn cc_time(v: f64) -> f64 {
    0.001 * 10000f64.powf(v.clamp(0.0, 127.0) / 127.0)
}

/// CC value (0..127) to cutoff frequency, Hz.
pub fn cc_cutoff(v: f64) -> f64 {
    20.0 * 1000f64.powf(v / 127.0)
}

// ---------------------------------------------------------------------------
// DSP building blocks

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f64 / u32::MAX as f64) * 2.0 - 1.0
    }
}

fn blep(t: f64, dt: f64) -> f64 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

fn osc(w: Wave, ph: f64, dt: f64, pw: f64) -> f64 {
    match w {
        Wave::Saw => 2.0 * ph - 1.0 - blep(ph, dt),
        Wave::Square => {
            let mut s = if ph < pw { 1.0 } else { -1.0 };
            s += blep(ph, dt);
            s -= blep((ph + 1.0 - pw) % 1.0, dt);
            s
        }
        Wave::Tri => 1.0 - 4.0 * (ph - 0.5).abs(),
        _ => (ph * TAU).sin(),
    }
}

/// Zero-delay-feedback 4-pole ladder low-pass (Zavalishin), driven input.
#[derive(Clone, Default)]
struct Ladder {
    s: [f64; 4],
    g: f64,
    k: f64,
}

impl Ladder {
    fn set(&mut self, fc: f64, rate: f64, k: f64) {
        self.g = (PI * fc.clamp(20.0, rate * 0.45) / rate).tan();
        self.k = k;
    }
    fn run(&mut self, x: f64, drive: f64) -> f64 {
        let g = self.g;
        let gg = g / (1.0 + g);
        let s = &mut self.s;
        let sig = (s[0] * gg * gg * gg + s[1] * gg * gg + s[2] * gg + s[3]) / (1.0 + g);
        let g4 = gg * gg * gg * gg;
        let y4 = (g4 * x + sig) / (1.0 + self.k * g4);
        let mut u = x - self.k * y4;
        u = (u * drive).tanh() / drive;
        for st in s.iter_mut() {
            let v = (u - *st) * gg;
            let y = v + *st;
            *st = y + v;
            u = y;
        }
        u * (1.0 + self.k * 0.55)
    }
}

/// RBJ biquad.
#[derive(Clone, Default)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Biquad {
    fn new(kind: u8, f: f64, q: f64, rate: f64) -> Biquad {
        let w = TAU * f.min(rate * 0.45) / rate;
        let (sn, cs) = w.sin_cos();
        let al = sn / (2.0 * q);
        let (b0, b1, b2) = match kind {
            0 => ((1.0 - cs) / 2.0, 1.0 - cs, (1.0 - cs) / 2.0),    // low-pass
            1 => ((1.0 + cs) / 2.0, -(1.0 + cs), (1.0 + cs) / 2.0), // high-pass
            _ => (al, 0.0, -al),                                    // band-pass, 0 dB peak
        };
        let a0 = 1.0 + al;
        Biquad { b: [b0 / a0, b1 / a0, b2 / a0], a: [-2.0 * cs / a0, (1.0 - al) / a0], z: [0.0; 2] }
    }
    fn run(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

#[derive(Clone)]
struct Env {
    a: f64,
    s: f64,
    level: f64,
    stage: u8, // 0 attack 1 decay 2 sustain 3 release 4 done
    dc: f64,
    rc: f64,
}

impl Env {
    fn new(adsr: [f64; 4], rate: f64) -> Env {
        let c = |t: f64| (-4.6 / (t.max(0.0005) * rate)).exp();
        Env { a: 1.0 / (adsr[0].max(0.0005) * rate), s: adsr[2], level: 0.0, stage: 0, dc: c(adsr[1]), rc: c(adsr[3]) }
    }
    fn retrigger(&mut self) {
        self.stage = 0;
    }
    fn release(&mut self) {
        if self.stage < 3 {
            self.stage = 3;
        }
    }
    fn next(&mut self) -> f64 {
        match self.stage {
            0 => {
                self.level += self.a;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = 1;
                }
            }
            1 => {
                self.level = self.s + (self.level - self.s) * self.dc;
                if (self.level - self.s).abs() < 1e-4 {
                    self.stage = 2;
                }
            }
            2 => self.level = self.s,
            3 => {
                self.level *= self.rc;
                if self.level < 2e-5 {
                    self.level = 0.0;
                    self.stage = 4;
                }
            }
            _ => self.level = 0.0,
        }
        self.level
    }
}

// ---------------------------------------------------------------------------
// Voices

const MAX_UNI: usize = 8;

struct Voice {
    key: u8,
    vel: f64,
    held: bool,
    sustained: bool,
    t: f64,
    since_off: f64,
    freq: f64,
    target: f64,
    ph: [f64; MAX_UNI],
    det: [f64; MAX_UNI],
    gl: [f64; MAX_UNI],
    gr: [f64; MAX_UNI],
    ph2: f64,
    ph_sub: f64,
    ph_mod: f64,
    ph_tine: f64,
    aenv: Env,
    fenv: Env,
    fl: Ladder,
    fr: Ladder,
    rng: Rng,
    kill: f64,
}

impl Voice {
    fn new(p: &Patch, key: u8, vel: f64, rate: f64, seed: u32, start_freq: Option<f64>, attack: f64, release: f64) -> Voice {
        let mut rng = Rng(seed.max(1));
        let target = 440.0 * 2f64.powf((key as f64 - 69.0) / 12.0);
        let n = p.unison.clamp(1, MAX_UNI);
        let mut v = Voice {
            key,
            vel,
            held: true,
            sustained: false,
            t: 0.0,
            since_off: 0.0,
            freq: start_freq.unwrap_or(target),
            target,
            ph: [0.0; MAX_UNI],
            det: [1.0; MAX_UNI],
            gl: [0.0; MAX_UNI],
            gr: [0.0; MAX_UNI],
            ph2: 0.0,
            ph_sub: 0.0,
            ph_mod: 0.0,
            ph_tine: 0.0,
            aenv: Env::new([attack, p.a_adsr[1], p.a_adsr[2], release], rate),
            fenv: Env::new(p.f_adsr, rate),
            fl: Ladder::default(),
            fr: Ladder::default(),
            rng: Rng(0),
            kill: 1.0,
        };
        let norm = 1.0 / (n as f64).sqrt();
        for u in 0..n {
            let x = if n == 1 { 0.0 } else { -1.0 + 2.0 * u as f64 / (n - 1) as f64 };
            // a little analogue drift on top of the spread
            let cents = x * p.detune * 0.5 + rng.next() * 2.0;
            v.det[u] = 2f64.powf(cents / 1200.0);
            v.ph[u] = rng.next() * 0.5 + 0.5;
            let pan = (x * p.width).clamp(-1.0, 1.0);
            let a = (pan + 1.0) * PI / 4.0;
            v.gl[u] = a.cos() * std::f64::consts::SQRT_2 * norm;
            v.gr[u] = a.sin() * std::f64::consts::SQRT_2 * norm;
        }
        v.ph2 = rng.next() * 0.5 + 0.5;
        v.rng = rng;
        v
    }

    fn done(&self) -> bool {
        self.aenv.stage == 4 || self.kill <= 0.0
    }
}

struct DrumVoice {
    key: u8,
    vel: f64,
    t: f64,
    ph: [f64; 6],
    rng: Rng,
    f: [Biquad; 4],
    pan: f64,
    choke: f64,
    dead: bool,
}

/// 808 hi-hat oscillator bank.
const METAL: [f64; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];

impl DrumVoice {
    fn new(key: u8, vel: f64, rate: f64, seed: u32) -> DrumVoice {
        let bq = Biquad::new;
        let f = match key {
            38 | 40 => [bq(2, 1800.0, 0.6, rate), bq(0, 7000.0, 0.7, rate), bq(0, 5200.0, 0.7, rate), bq(0, 5200.0, 0.7, rate)],
            39 => [bq(2, 1150.0, 1.4, rate), bq(2, 1250.0, 1.4, rate), bq(0, 5000.0, 0.7, rate), bq(0, 5000.0, 0.7, rate)],
            37 => [bq(2, 1700.0, 4.0, rate), bq(0, 9000.0, 0.7, rate), bq(0, 9000.0, 0.7, rate), bq(0, 9000.0, 0.7, rate)],
            42 | 44 | 46 => [bq(1, 7000.0, 0.8, rate), bq(1, 7000.0, 0.8, rate), bq(2, 10000.0, 0.8, rate), bq(0, 9000.0, 0.7, rate)],
            49 | 51 | 52 | 53 | 55 | 57 | 59 => [bq(1, 4500.0, 0.7, rate), bq(1, 4500.0, 0.7, rate), bq(0, 12000.0, 0.7, rate), bq(0, 12000.0, 0.7, rate)],
            _ => [bq(1, 5500.0, 0.7, rate), bq(0, 9000.0, 0.7, rate), bq(0, 3000.0, 0.7, rate), bq(0, 3000.0, 0.7, rate)],
        };
        let pan = match key {
            42 | 44 | 46 => 0.25,
            41 | 43 => 0.3,
            45 | 47 => 0.0,
            48 | 50 => -0.3,
            49 | 57 => -0.2,
            51 | 59 => 0.3,
            _ => 0.0,
        };
        DrumVoice { key, vel, t: 0.0, ph: [0.0; 6], rng: Rng(seed.max(1)), f, pan, choke: 1.0, dead: false }
    }

    /// Runs `x` through filters 0..n in series.
    fn chain(&mut self, mut x: f64, n: usize) -> f64 {
        for f in self.f[..n].iter_mut() {
            x = f.run(x);
        }
        x
    }

    fn hat(&mut self, rate: f64) -> f64 {
        let mut s = 0.0;
        for (i, f) in METAL.iter().enumerate() {
            self.ph[i] = (self.ph[i] + f * 1.7 / rate) % 1.0;
            s += if self.ph[i] < 0.5 { 1.0 } else { -1.0 };
        }
        s / 6.0 + self.rng.next() * 0.35
    }

    /// One stereo sample.
    fn run(&mut self, rate: f64) -> (f64, f64) {
        let t = self.t;
        self.t += 1.0 / rate;
        let e = |tau: f64| (-t / tau).exp();
        let (mono, side) = match self.key {
            35 | 36 => {
                let f = 52.0 + 130.0 * e(0.024) + 40.0 * e(0.004);
                self.ph[0] = (self.ph[0] + f / rate) % 1.0;
                let body = (self.ph[0] * TAU).sin() * if t < 0.09 { 1.0 } else { (-(t - 0.09) / 0.075).exp() };
                let click = self.f[0].run(self.rng.next()) * e(0.003) * 0.8;
                (((body + click) * 1.6).tanh() * 0.95, 0.0)
            }
            38 | 40 => {
                self.ph[0] = (self.ph[0] + 185.0 * (1.0 + 0.4 * e(0.01)) / rate) % 1.0;
                self.ph[1] = (self.ph[1] + 330.0 / rate) % 1.0;
                let tone = (self.ph[0] * TAU).sin() * e(0.05) * 0.55 + (self.ph[1] * TAU).sin() * e(0.03) * 0.3;
                let n = self.rng.next();
                let snap = self.chain(n, 2) * e(0.11) * 2.3;
                // gated reverb: a dense burst held ~220 ms then cut
                let gate = if t < 0.004 { t / 0.004 } else if t < 0.22 { 1.0 - (t - 0.004) * 1.2 } else { (0.74 * (-(t - 0.22) / 0.012).exp()).max(0.0) };
                let nl = self.f[2].run(self.rng.next()) * gate * 0.75;
                let nr = self.f[3].run(self.rng.next()) * gate * 0.75;
                ((tone + snap) + (nl + nr) * 0.5, (nl - nr) * 0.5)
            }
            39 => {
                let n = self.rng.next();
                let burst = [0.0, 0.011, 0.022].iter().map(|o| if t >= *o { (-(t - o) / 0.005).exp() } else { 0.0 }).sum::<f64>();
                let tail = if t > 0.03 { (-(t - 0.03) / 0.13).exp() } else { 0.0 };
                let s = self.f[0].run(n) * (burst + tail * 0.8) * 2.2;
                let w = self.f[1].run(self.rng.next()) * tail * 0.9;
                (s, w * 0.6)
            }
            37 => {
                self.ph[0] = (self.ph[0] + 1700.0 / rate) % 1.0;
                ((self.ph[0] * TAU).sin() * e(0.012) * 0.6 + self.f[0].run(self.rng.next()) * e(0.02) * 1.5, 0.0)
            }
            42 | 44 | 46 => {
                let h = self.hat(rate);
                let d = match self.key {
                    42 => 0.035,
                    44 => 0.025,
                    _ => 0.33,
                };
                let s = self.chain(h, 3) * e(d) * self.choke * 2.6;
                (s, 0.0)
            }
            49 | 52 | 55 | 57 => {
                let h = self.hat(rate) * 0.7 + self.rng.next() * 0.5;
                let d = if self.key == 55 { 0.5 } else { 1.4 };
                let l = self.chain(h, 2) * e(d) * 1.3;
                let r = self.f[2].run(self.rng.next()) * e(d) * 0.35;
                (l, r)
            }
            51 | 53 | 59 => {
                self.ph[5] = (self.ph[5] + 3150.0 / rate) % 1.0;
                let bell = (self.ph[5] * TAU).sin() * e(0.3) * if self.key == 53 { 0.4 } else { 0.12 };
                let h = self.hat(rate);
                let h = self.chain(h, 2) * e(0.7) * 0.6;
                (h + bell, 0.0)
            }
            41 | 43 | 45 | 47 | 48 | 50 => {
                let base = match self.key {
                    41 => 72.0,
                    43 => 86.0,
                    45 => 104.0,
                    47 => 124.0,
                    48 => 148.0,
                    _ => 176.0,
                };
                let f = base * (1.0 + 0.7 * e(0.06));
                self.ph[0] = (self.ph[0] + f / rate) % 1.0;
                (((self.ph[0] * TAU).sin() * e(0.32) * 1.4).tanh() * 0.85 + self.f[0].run(self.rng.next()) * e(0.02) * 0.4, 0.0)
            }
            54 | 69 | 70 | 82 => (self.f[0].run(self.rng.next()) * e(0.06) * 1.2, 0.0),
            56 => {
                self.ph[0] = (self.ph[0] + 540.0 / rate) % 1.0;
                self.ph[1] = (self.ph[1] + 800.0 / rate) % 1.0;
                let s = (if self.ph[0] < 0.5 { 1.0 } else { -1.0 }) + (if self.ph[1] < 0.5 { 1.0 } else { -1.0 });
                (self.f[2].run(s) * e(0.12) * 0.45, 0.0)
            }
            k => {
                let f = 440.0 * 2f64.powf((k as f64 - 69.0) / 12.0);
                self.ph[0] = (self.ph[0] + f / rate) % 1.0;
                ((self.ph[0] * TAU).sin() * e(0.08) * 0.5, 0.0)
            }
        };
        if t > 3.0 {
            self.dead = true;
        }
        let g = self.vel;
        let a = (self.pan + 1.0) * PI / 4.0;
        let (pl, pr) = (a.cos() * std::f64::consts::SQRT_2, a.sin() * std::f64::consts::SQRT_2);
        ((mono * pl + side) * g, (mono * pr - side) * g)
    }
}

// ---------------------------------------------------------------------------
// Effects

struct Chorus {
    buf: Vec<[f64; 2]>,
    pos: usize,
    ph: f64,
}

impl Chorus {
    fn new(rate: f64) -> Chorus {
        Chorus { buf: vec![[0.0; 2]; (rate * 0.03) as usize + 4], pos: 0, ph: 0.0 }
    }
    fn read(&self, delay: f64, ch: usize) -> f64 {
        let n = self.buf.len();
        let p = self.pos as f64 - delay;
        let p = if p < 0.0 { p + n as f64 } else { p };
        let i = p.floor() as usize % n;
        let fr = p - p.floor();
        self.buf[i][ch] * (1.0 - fr) + self.buf[(i + 1) % n][ch] * fr
    }
    fn run(&mut self, l: f64, r: f64, mix: f64, rate: f64) -> (f64, f64) {
        self.buf[self.pos] = [l, r];
        self.ph = (self.ph + 0.55 / rate) % 1.0;
        let m = (self.ph * TAU).sin();
        let dl = (0.0075 + 0.0028 * m) * rate;
        let dr = (0.0075 - 0.0028 * m) * rate;
        let wl = self.read(dl, 1);
        let wr = self.read(dr, 0);
        self.pos = (self.pos + 1) % self.buf.len();
        let n = 1.0 / (1.0 + mix * 0.4);
        ((l + wl * mix) * n, (r + wr * mix) * n)
    }
}

struct Comb {
    buf: Vec<f64>,
    pos: usize,
    filt: f64,
}
struct Allpass {
    buf: Vec<f64>,
    pos: usize,
}

/// Freeverb-style stereo hall.
struct Reverb {
    combs: [Vec<Comb>; 2],
    aps: [Vec<Allpass>; 2],
    pre: Vec<f64>,
    pre_pos: usize,
    hp: f64,
    lp: f64,
}

impl Reverb {
    fn new(rate: f64) -> Reverb {
        let sc = rate / 44100.0;
        let mk = |spread: usize| -> (Vec<Comb>, Vec<Allpass>) {
            let c = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617].iter().map(|n| Comb { buf: vec![0.0; (((n + spread) as f64) * sc * 1.25) as usize], pos: 0, filt: 0.0 }).collect();
            let a = [556, 441, 341, 225].iter().map(|n| Allpass { buf: vec![0.0; (((n + spread) as f64) * sc) as usize], pos: 0 }).collect();
            (c, a)
        };
        let (cl, al) = mk(0);
        let (cr, ar) = mk(23);
        Reverb { combs: [cl, cr], aps: [al, ar], pre: vec![0.0; (rate * 0.03) as usize], pre_pos: 0, hp: 0.0, lp: 0.0 }
    }
    fn run(&mut self, x: f64) -> (f64, f64) {
        // pre-delay, then band-limit the input (no mud, no fizz)
        let d = self.pre[self.pre_pos];
        self.pre[self.pre_pos] = x;
        self.pre_pos = (self.pre_pos + 1) % self.pre.len();
        self.hp += (d - self.hp) * 0.03;
        let x = d - self.hp;
        self.lp += (x - self.lp) * 0.45;
        let inp = self.lp * 0.02;
        let mut out = [0.0; 2];
        for ch in 0..2 {
            let mut s = 0.0;
            for c in self.combs[ch].iter_mut() {
                let o = c.buf[c.pos];
                c.filt = o * 0.62 + c.filt * 0.38;
                c.buf[c.pos] = inp + c.filt * 0.89;
                c.pos = (c.pos + 1) % c.buf.len();
                s += o;
            }
            for a in self.aps[ch].iter_mut() {
                let b = a.buf[a.pos];
                let o = b - s;
                a.buf[a.pos] = s + b * 0.5;
                a.pos = (a.pos + 1) % a.buf.len();
                s = o;
            }
            out[ch] = s;
        }
        (out[0] * 3.0, out[1] * 3.0)
    }
}

/// Ping-pong delay with a darkening feedback path.
struct PingPong {
    l: Vec<f64>,
    r: Vec<f64>,
    pos: usize,
    lpl: f64,
    lpr: f64,
}

impl PingPong {
    fn new(secs: f64, rate: f64) -> PingPong {
        let n = ((secs * rate) as usize).max(1);
        PingPong { l: vec![0.0; n], r: vec![0.0; n], pos: 0, lpl: 0.0, lpr: 0.0 }
    }
    fn run(&mut self, x: f64) -> (f64, f64) {
        let ol = self.l[self.pos];
        let or = self.r[self.pos];
        self.lpl += (ol - self.lpl) * 0.35;
        self.lpr += (or - self.lpr) * 0.35;
        self.l[self.pos] = x + self.lpr * 0.42;
        self.r[self.pos] = self.lpl * 0.42;
        self.pos = (self.pos + 1) % self.l.len();
        (ol, or)
    }
}

// ---------------------------------------------------------------------------
// Engine

struct Chan {
    patch: Patch,
    voices: Vec<Voice>,
    drums: Vec<DrumVoice>,
    vol: f64,
    expr: f64,
    pan: f64,
    verb: f64,
    delay: f64,
    chorus_cc: f64,
    cutoff: f64,
    reso: f64,
    attack: f64,
    release: f64,
    modw: f64,
    bend: f64,
    bend_range: f64,
    rpn: (u8, u8),
    pedal: bool,
    chorus: Chorus,
    last_freq: Option<f64>,
    seed: u32,
}

/// The synthesiser for the `synth=` tracks of a song.
pub struct Engine {
    rate: f64,
    chans: Vec<Option<Chan>>,
    reverb: Reverb,
    delay: PingPong,
    duck: f64,
    duck_target: f64,
    duck_rel: f64,
    t: f64,
    /// DC blocker state (x1, y1) per side.
    dc: [(f64, f64); 2],
}

const MASTER: f64 = 0.16;
const MAX_VOICES: usize = 24;

impl Engine {
    /// An engine for the song's synth tracks, or None if it has none.
    pub fn for_song(song: &Song, rate: u32) -> Result<Option<Engine>, String> {
        let rate_f = rate as f64;
        let mut chans: Vec<Option<Chan>> = (0..16).map(|_| None).collect();
        let mut any = false;
        for tr in &song.tracks {
            let Some(name) = &tr.def.synth else { continue };
            any = true;
            let patch = patch_for(name, &tr.def.synth_opts)?;
            let (cutoff, reso, attack, release, delay) = (patch.cutoff, patch.reso, patch.a_adsr[0], patch.a_adsr[3], patch.delay);
            chans[tr.channel as usize & 15] = Some(Chan {
                patch,
                voices: Vec::new(),
                drums: Vec::new(),
                vol: 100.0 / 127.0,
                expr: 1.0,
                pan: 0.0,
                verb: 0.3,
                delay,
                chorus_cc: 0.0,
                cutoff,
                reso,
                attack,
                release,
                modw: 0.0,
                bend: 0.0,
                bend_range: 2.0,
                rpn: (127, 127),
                pedal: false,
                chorus: Chorus::new(rate_f),
                last_freq: None,
                seed: 0x9E37_79B9 ^ (tr.channel as u32 + 1).wrapping_mul(0x85EB_CA6B),
            });
        }
        if !any {
            return Ok(None);
        }
        let bpm = song.tempos.first().map(|t| t.1).unwrap_or(120.0);
        let beat = 60.0 / bpm;
        Ok(Some(Engine {
            rate: rate_f,
            chans,
            reverb: Reverb::new(rate_f),
            delay: PingPong::new(beat * 0.75, rate_f),
            duck: 0.0,
            duck_target: 0.0,
            duck_rel: (-4.6 / (beat * 0.55 * rate_f)).exp(),
            t: 0.0,
            dc: [(0.0, 0.0); 2],
        }))
    }

    /// Handles a channel message; returns true when the channel belongs to
    /// the engine (the SoundFont must not play it). Kicks on the drum channel
    /// trigger the sidechain whoever plays them.
    pub fn message(&mut self, ch: u8, cmd: u8, d1: u8, d2: u8) -> bool {
        if ch == 9 && cmd == 0x90 && d2 > 0 && (d1 == 35 || d1 == 36) {
            self.duck_target = (d2 as f64 / 110.0).min(1.0);
        }
        let rate = self.rate;
        let Some(c) = self.chans[ch as usize & 15].as_mut() else { return false };
        match cmd {
            0x90 if d2 > 0 => {
                c.seed = c.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let vel = d2 as f64 / 127.0;
                if c.patch.wave == Wave::Drums {
                    if d1 == 42 || d1 == 44 {
                        for v in c.drums.iter_mut().filter(|v| v.key == 46) {
                            v.choke = 0.0;
                        }
                    }
                    let g = vel.powf(1.3);
                    c.drums.retain(|v| !(v.key == d1 && v.t > 0.0 && d1 != 38 && d1 != 40 && d1 != 39));
                    c.drums.push(DrumVoice::new(d1, g, rate, c.seed));
                    if c.drums.len() > 32 {
                        c.drums.remove(0);
                    }
                    return true;
                }
                if c.patch.mono {
                    // legato / glide: reuse the sounding voice
                    if let Some(v) = c.voices.iter_mut().rev().find(|v| !v.done() && (v.held || v.since_off < 0.03)) {
                        v.key = d1;
                        v.target = 440.0 * 2f64.powf((d1 as f64 - 69.0) / 12.0);
                        if c.patch.glide <= 0.0 {
                            v.freq = v.target;
                        }
                        v.vel = vel;
                        if !v.held {
                            v.aenv.retrigger();
                            v.fenv.retrigger();
                        }
                        v.held = true;
                        v.since_off = 0.0;
                        c.last_freq = Some(v.target);
                        return true;
                    }
                    for v in c.voices.iter_mut() {
                        v.held = false;
                        v.aenv.release();
                    }
                } else {
                    for v in c.voices.iter_mut().filter(|v| v.key == d1 && v.held) {
                        v.held = false;
                        v.aenv.release();
                        v.fenv.release();
                    }
                }
                let start = if c.patch.glide > 0.0 { c.last_freq } else { None };
                let v = Voice::new(&c.patch, d1, vel, rate, c.seed, start, c.attack, c.release);
                c.last_freq = Some(v.target);
                c.voices.push(v);
                let live = c.voices.iter().filter(|v| !v.done()).count();
                if live > MAX_VOICES {
                    if let Some(v) = c.voices.iter_mut().find(|v| v.kill >= 1.0) {
                        v.kill = 0.999;
                    }
                }
            }
            0x80 | 0x90 => {
                for v in c.voices.iter_mut().filter(|v| v.key == d1 && v.held) {
                    v.held = false;
                    if c.pedal {
                        v.sustained = true;
                    } else {
                        v.aenv.release();
                        v.fenv.release();
                    }
                }
            }
            0xB0 => {
                let x = d2 as f64;
                match d1 {
                    1 => c.modw = x / 127.0,
                    6 if c.rpn == (0, 0) => c.bend_range = x,
                    7 => c.vol = x / 127.0,
                    10 => c.pan = ((x - 64.0) / 63.0).clamp(-1.0, 1.0),
                    11 => c.expr = x / 127.0,
                    64 => {
                        c.pedal = d2 >= 64;
                        if !c.pedal {
                            for v in c.voices.iter_mut().filter(|v| v.sustained) {
                                v.sustained = false;
                                v.aenv.release();
                                v.fenv.release();
                            }
                        }
                    }
                    71 => c.reso = x,
                    72 => c.release = cc_time(x),
                    73 => c.attack = cc_time(x),
                    74 => c.cutoff = x,
                    91 => c.verb = x / 127.0,
                    93 => c.chorus_cc = x / 127.0,
                    94 => c.delay = x / 127.0,
                    100 => c.rpn.1 = d2,
                    101 => c.rpn.0 = d2,
                    120 | 123 => {
                        for v in c.voices.iter_mut() {
                            v.held = false;
                            v.sustained = false;
                            v.aenv.release();
                        }
                    }
                    _ => {}
                }
            }
            0xE0 => {
                let v = (d1 as i32 | ((d2 as i32) << 7)) - 8192;
                c.bend = v as f64 / 8192.0 * c.bend_range;
            }
            _ => {}
        }
        true
    }

    /// Adds `l.len()` samples of the synth tracks and effects into l/r.
    pub fn render(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = l.len();
        let rate = self.rate;
        let dt = 1.0 / rate;
        // sidechain envelope for the block
        let mut duck = vec![0.0f64; n];
        for d in duck.iter_mut() {
            if self.duck_target > 0.0 {
                self.duck += (self.duck_target - self.duck) * 0.08;
                if self.duck > self.duck_target * 0.97 {
                    self.duck_target = 0.0;
                }
            } else {
                self.duck *= self.duck_rel;
            }
            *d = self.duck;
        }
        let mut send_v = vec![0.0f64; n];
        let mut send_d = vec![0.0f64; n];
        let mut sum_l = vec![0.0f64; n];
        let mut sum_r = vec![0.0f64; n];
        let mut bl = vec![0.0f64; n];
        let mut br = vec![0.0f64; n];
        let t0 = self.t;
        for c in self.chans.iter_mut().flatten() {
            let p = &c.patch;
            if c.voices.is_empty() && c.drums.is_empty() && c.chorus_cc + p.chorus <= 0.0 {
                continue;
            }
            bl.iter_mut().for_each(|x| *x = 0.0);
            br.iter_mut().for_each(|x| *x = 0.0);
            // drum machine
            for v in c.drums.iter_mut() {
                for i in 0..n {
                    let (a, b) = v.run(rate);
                    bl[i] += a;
                    br[i] += b;
                }
            }
            c.drums.retain(|v| !v.dead);
            // tonal voices
            let k = p.reso / 127.0 * 3.9;
            let drive = 1.0 + p.drive * 3.0;
            let bend = 2f64.powf(c.bend / 12.0);
            let glide_c = if p.glide > 0.0 { (-4.6 / (p.glide * rate)).exp() } else { 0.0 };
            let nuni = p.unison.clamp(1, MAX_UNI);
            for v in c.voices.iter_mut() {
                for i in 0..n {
                    if v.done() {
                        break;
                    }
                    let t = v.t;
                    v.t += dt;
                    if !v.held {
                        v.since_off += dt;
                    }
                    if v.kill < 1.0 {
                        v.kill -= 1.0 / (0.004 * rate);
                    }
                    v.freq = v.target * (v.freq / v.target).powf(glide_c);
                    let vib_depth = p.vib * ((t - p.vib_delay) / 0.4).clamp(0.0, 1.0) + c.modw * 40.0;
                    let vib = if vib_depth > 0.0 { 2f64.powf(vib_depth * ((t0 + i as f64 * dt) * TAU * p.vib_rate).sin() / 1200.0) } else { 1.0 };
                    let f = v.freq * bend * vib;
                    let fe = v.fenv.next();
                    let ae = v.aenv.next() * v.kill.max(0.0);
                    if i % 8 == 0 {
                        let oct = p.envamt * fe + p.keytrack * (v.key as f64 - 60.0) / 12.0 + p.velfilt * (v.vel - 0.7) + p.flfo * ((t0 + i as f64 * dt) * TAU * p.flfo_rate).sin();
                        let fc = cc_cutoff(c.cutoff) * 2f64.powf(oct);
                        v.fl.set(fc, rate, k);
                        v.fr.set(fc, rate, k);
                    }
                    let (mut sl, mut sr) = (0.0, 0.0);
                    match p.wave {
                        Wave::Fm => {
                            v.ph_mod = (v.ph_mod + f * p.fm_ratio * dt) % 1.0;
                            let idx = p.fm_index * ((-t / p.fm_decay).exp() * 0.85 + 0.15);
                            let mut m = idx * (v.ph_mod * TAU).sin();
                            if let Some((ratio, ti, td)) = p.fm_tine {
                                v.ph_tine = (v.ph_tine + f * ratio * dt) % 1.0;
                                m += ti * (-t / td).exp() * (v.ph_tine * TAU).sin();
                            }
                            v.ph[0] = (v.ph[0] + f * dt) % 1.0;
                            let s = (v.ph[0] * TAU + m).sin();
                            sl = s;
                            sr = s;
                        }
                        Wave::Noise => {
                            sl = v.rng.next();
                            sr = v.rng.next();
                        }
                        w => {
                            let pw = (p.pw + p.pwm * ((t0 + i as f64 * dt) * TAU * p.pwm_rate).sin()).clamp(0.05, 0.95);
                            for u in 0..nuni {
                                let d = f * v.det[u] * dt;
                                v.ph[u] += d;
                                if v.ph[u] >= 1.0 {
                                    v.ph[u] -= 1.0;
                                }
                                let s = osc(w, v.ph[u], d, pw);
                                sl += s * v.gl[u];
                                sr += s * v.gr[u];
                            }
                            if let Some((w2, lvl, semi)) = p.wave2 {
                                if lvl > 0.0 {
                                    let d = f * 2f64.powf(semi / 12.0) * dt;
                                    v.ph2 = (v.ph2 + d) % 1.0;
                                    let s = osc(w2, v.ph2, d, pw) * lvl;
                                    sl += s;
                                    sr += s;
                                }
                            }
                            if p.sub > 0.0 {
                                let d = f * 0.5 * dt;
                                v.ph_sub = (v.ph_sub + d) % 1.0;
                                let s = osc(Wave::Square, v.ph_sub, d, 0.5) * p.sub;
                                sl += s;
                                sr += s;
                            }
                            if p.noise > 0.0 {
                                let s = v.rng.next() * p.noise;
                                sl += s;
                                sr += s;
                            }
                        }
                    }
                    let g = ae * (0.25 + 0.75 * v.vel);
                    if p.cutoff >= 127.0 && c.cutoff >= 127.0 && p.reso <= 0.0 {
                        bl[i] += sl * g;
                        br[i] += sr * g;
                    } else {
                        bl[i] += v.fl.run(sl, drive) * g;
                        br[i] += v.fr.run(sr, drive) * g;
                    }
                }
            }
            c.voices.retain(|v| !v.done());
            // chorus, mix, sidechain, pan, sends
            let cmix = (p.chorus + c.chorus_cc).min(1.0);
            let gain = c.vol * c.vol * c.expr * c.expr * p.gain * MASTER;
            let a = (c.pan + 1.0) * PI / 4.0;
            let (pl, pr) = (a.cos() * std::f64::consts::SQRT_2, a.sin() * std::f64::consts::SQRT_2);
            let dsend = c.delay;
            for i in 0..n {
                let (mut x, mut y) = (bl[i], br[i]);
                if cmix > 0.0 {
                    (x, y) = c.chorus.run(x, y, cmix, rate);
                }
                let g = gain * (1.0 - p.duck * duck[i]);
                x *= g * pl;
                y *= g * pr;
                bl[i] = x;
                br[i] = y;
                let m = (x + y) * 0.5;
                send_v[i] += m * c.verb;
                send_d[i] += m * dsend;
            }
            for i in 0..n {
                sum_l[i] += bl[i];
                sum_r[i] += br[i];
            }
        }
        let k = 1.0 - TAU * 10.0 / rate;
        for i in 0..n {
            let (dl, dr) = self.delay.run(send_d[i]);
            let (vl, vr) = self.reverb.run(send_v[i] + (dl + dr) * 0.25);
            for (side, x) in [(0, sum_l[i] + dl + vl), (1, sum_r[i] + dr + vr)] {
                let (x1, y1) = self.dc[side];
                let y = x - x1 + k * y1;
                self.dc[side] = (x, y);
                if side == 0 {
                    l[i] += y as f32;
                } else {
                    r[i] += y as f32;
                }
            }
        }
        self.t += n as f64 * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_render_sound() {
        for (name, _, _) in PRESETS {
            let drums = *name == "drums";
            let text = if drums {
                "tempo 120\ntrack d drums synth=drums\nd: [kick hat]q snare clap [ohat tom1] | crash ride rim cowbell |\n".to_string()
            } else {
                format!("tempo 120\ntrack t synth={name} oct=4\nt: C4q E4 G4 [C4 E4 G4]q | C4w |\n")
            };
            let song = crate::compile_text(&text, None).unwrap();
            let mut e = Engine::for_song(&song, 22050).unwrap().unwrap();
            let mut l = vec![0f32; 22050 * 3];
            let mut r = vec![0f32; 22050 * 3];
            let mut evs: Vec<(f64, Vec<u8>)> = Vec::new();
            for tr in &song.tracks {
                for (t, m) in song.track_events(tr) {
                    evs.push((song.secs_at(t), m));
                }
            }
            evs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let mut pos = 0;
            for (t, m) in evs {
                let s = ((t * 22050.0) as usize).min(l.len());
                if s > pos {
                    e.render(&mut l[pos..s], &mut r[pos..s]);
                    pos = s;
                }
                e.message(m[0] & 15, m[0] & 0xF0, m.get(1).copied().unwrap_or(0), m.get(2).copied().unwrap_or(0));
            }
            let end = l.len();
            e.render(&mut l[pos..end], &mut r[pos..end]);
            let peak = l.iter().chain(r.iter()).fold(0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.01, "{name}: silent (peak {peak})");
            assert!(peak.is_finite() && peak < 4.0, "{name}: peak {peak}");
        }
    }
}
