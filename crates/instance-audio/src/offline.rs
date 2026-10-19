//! Offline mixing (`--audio-out`): the voices that would play on the rodio
//! mixer ([`super::voice::Voice`], with their effects, volume, pan and fades)
//! are rendered in game time into a 44.1 kHz 16-bit stereo WAV file. The
//! frontend advances the mix by exactly `1 / room_speed` seconds per step, so
//! the result is deterministic and independent of how fast the run goes.
//!
//! Voices at other sample rates are resampled linearly (the live mixer
//! resamples too). The sum is clipped to −1..1; clipped samples are counted.

use super::voice::{Globals, Voice};
use std::io::{Seek, SeekFrom, Write};
use std::sync::Arc;

/// Output sample rate.
pub const RATE: u32 = 44100;

/// One voice with a linear resampler to [`RATE`].
struct Resampled {
    v: Voice,
    step: f64,
    /// Position between `a` (source frame i) and `b` (frame i + 1), 0..1.
    pos: f64,
    a: (f32, f32),
    b: (f32, f32),
    /// `b` is a real frame (false: past the end of the voice, silence).
    b_valid: bool,
    started: bool,
    done: bool,
}

impl Resampled {
    fn new(v: Voice) -> Self {
        let step = v.rate() as f64 / RATE as f64;
        Resampled { v, step, pos: 0.0, a: (0.0, 0.0), b: (0.0, 0.0), b_valid: false, started: false, done: false }
    }

    /// Next output frame; None when finished.
    fn next(&mut self) -> Option<(f32, f32)> {
        if !self.started {
            self.started = true;
            match self.v.next_frame() {
                Some(f) => self.a = f,
                None => self.done = true,
            }
            if !self.done {
                if let Some(f) = self.v.next_frame() {
                    self.b = f;
                    self.b_valid = true;
                }
            }
        }
        if self.done {
            return None;
        }
        let f = self.pos as f32;
        let out = (self.a.0 + (self.b.0 - self.a.0) * f, self.a.1 + (self.b.1 - self.a.1) * f);
        self.pos += self.step;
        while self.pos >= 1.0 {
            self.pos -= 1.0;
            if !self.b_valid {
                self.done = true;
                break;
            }
            self.a = self.b;
            match self.v.next_frame() {
                Some(f) => self.b = f,
                None => {
                    self.b = (0.0, 0.0);
                    self.b_valid = false;
                }
            }
        }
        Some(out)
    }
}

/// Loudness statistics of the rendered mix.
#[derive(Default, Clone, Debug)]
pub struct Stats {
    pub frames: u64,
    pub peak: f32,
    pub sum_sq: f64,
    pub clipped: u64,
}

impl Stats {
    pub fn add(&mut self, l: f32, r: f32) {
        self.frames += 1;
        for x in [l, r] {
            let a = x.abs();
            if a > self.peak {
                self.peak = a;
            }
            if a >= 1.0 {
                self.clipped += 1;
            }
            self.sum_sq += (x as f64) * (x as f64);
        }
    }

    fn db(x: f64) -> String {
        if x <= 0.0 {
            "-inf".into()
        } else {
            format!("{:.1}", 20.0 * x.log10())
        }
    }

    /// One-line summary: duration, peak and RMS in dBFS, % clipped samples.
    pub fn summary(&self) -> String {
        let n = (self.frames * 2).max(1) as f64;
        format!(
            "{:.2} s, peak {} dBFS, RMS {} dBFS, {:.3}% clipped samples",
            self.frames as f64 / RATE as f64,
            Self::db(self.peak as f64),
            Self::db((self.sum_sq / n).sqrt()),
            100.0 * self.clipped as f64 / n
        )
    }
}

/// The offline mixer and WAV writer.
pub struct Offline {
    voices: Vec<Resampled>,
    out: Option<std::io::BufWriter<std::fs::File>>,
    /// Frames written (and the clock's position: `frames / RATE` seconds).
    frames: u64,
    /// Game time reached, seconds (the next step's audio starts here).
    time: f64,
    pub stats: Stats,
    last_patch: u64,
}

fn header(data_bytes: u32) -> [u8; 44] {
    let mut h = [0u8; 44];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&(36u32.wrapping_add(data_bytes)).to_le_bytes());
    h[8..16].copy_from_slice(b"WAVEfmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..22].copy_from_slice(&1u16.to_le_bytes());
    h[22..24].copy_from_slice(&2u16.to_le_bytes());
    h[24..28].copy_from_slice(&RATE.to_le_bytes());
    h[28..32].copy_from_slice(&(RATE * 4).to_le_bytes());
    h[32..34].copy_from_slice(&4u16.to_le_bytes());
    h[34..36].copy_from_slice(&16u16.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data_bytes.to_le_bytes());
    h
}

impl Offline {
    /// Creates the WAV file (`None`: mix without writing, for tests).
    pub fn create(path: Option<&std::path::Path>) -> std::io::Result<Offline> {
        let out = match path {
            Some(p) => {
                if let Some(d) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
                    std::fs::create_dir_all(d)?;
                }
                let mut w = std::io::BufWriter::new(std::fs::File::create(p)?);
                w.write_all(&header(0))?;
                Some(w)
            }
            None => None,
        };
        Ok(Offline { voices: Vec::new(), out, frames: 0, time: 0.0, stats: Stats::default(), last_patch: 0 })
    }

    pub fn add(&mut self, v: Voice) {
        self.voices.push(Resampled::new(v));
    }

    /// Number of voices still mixed.
    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    /// Renders `secs` more seconds of game time, moving the audio clock along.
    pub fn advance(&mut self, secs: f64, globals: &Arc<Globals>, mut sink: Option<&mut Vec<(f32, f32)>>) {
        self.time += secs.max(0.0);
        let target = (self.time * RATE as f64).round() as u64;
        const BLOCK: u64 = 64;
        let mut buf: Vec<u8> = Vec::with_capacity(((target.saturating_sub(self.frames)) * 4) as usize);
        while self.frames < target {
            globals.set_virtual_time(self.frames as f64 / RATE as f64);
            let n = (target - self.frames).min(BLOCK);
            for _ in 0..n {
                let (mut l, mut r) = (0f32, 0f32);
                for v in self.voices.iter_mut() {
                    if let Some((a, b)) = v.next() {
                        l += a;
                        r += b;
                    }
                }
                self.stats.add(l, r);
                if let Some(s) = sink.as_deref_mut() {
                    s.push((l, r));
                }
                for x in [l, r] {
                    let q = (x.clamp(-1.0, 1.0) * 32767.0).round() as i16;
                    buf.extend_from_slice(&q.to_le_bytes());
                }
            }
            self.voices.retain(|v| !v.done);
            self.frames += n;
        }
        globals.set_virtual_time(self.time);
        if let Some(w) = self.out.as_mut() {
            let _ = w.write_all(&buf);
        }
        // keep the header valid now and then (a run killed by --max-seconds)
        if self.frames - self.last_patch >= RATE as u64 * 5 {
            self.last_patch = self.frames;
            self.patch_header();
        }
    }

    fn patch_header(&mut self) {
        let bytes = (self.frames * 4).min(u32::MAX as u64 - 36) as u32;
        if let Some(w) = self.out.as_mut() {
            let _ = w.flush();
            let f = w.get_mut();
            if let Ok(pos) = f.stream_position() {
                let _ = f.seek(SeekFrom::Start(0));
                let _ = f.write_all(&header(bytes));
                let _ = f.seek(SeekFrom::Start(pos));
            }
        }
    }

    /// Finalises the WAV file; returns the loudness summary.
    pub fn finish(&mut self) -> String {
        self.patch_header();
        if let Some(mut w) = self.out.take() {
            let _ = w.flush();
        }
        self.stats.summary()
    }
}
