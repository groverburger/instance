//! Objective audio analysis for authors who cannot listen: levels, spectrum,
//! envelope, a one-line description, loop-seam checks, loudness over time
//! and a similarity score between sounds.

use crate::theory::freq_note_name;
use crate::wav::Audio;
use serde::Serialize;

/// In-place iterative radix-2 complex FFT (`re.len()` must be a power of two).
pub fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * std::f64::consts::PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f64, 0.0f64);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
        len <<= 1;
    }
}

fn db(x: f64) -> f64 {
    if x <= 1e-10 {
        -200.0
    } else {
        20.0 * x.log10()
    }
}

/// Band edges in Hz for the five reported bands.
pub const BANDS: [(&str, f64, f64); 5] = [("sub", 0.0, 80.0), ("low", 80.0, 300.0), ("mid", 300.0, 2000.0), ("high", 2000.0, 8000.0), ("air", 8000.0, 1e9)];

/// Spectral summary of a mono signal.
#[derive(Clone, Debug, Default)]
struct Spectrum {
    /// Average power per bin.
    power: Vec<f64>,
    bin_hz: f64,
    centroid: f64,
    flatness: f64,
    /// Per-frame centroid of non-silent frames.
    frame_centroids: Vec<f64>,
}

fn spectrum(x: &[f32], rate: u32) -> Spectrum {
    let n = if rate >= 32000 { 2048 } else { 1024 };
    let hop = n / 2;
    let win: Vec<f64> = (0..n).map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos()).collect();
    let bin_hz = rate as f64 / n as f64;
    let mut power = vec![0.0; n / 2];
    let mut frames = 0usize;
    let mut flat_sum = 0.0;
    let mut frame_centroids = Vec::new();
    let mut pos = 0usize;
    let (mut re, mut im) = (vec![0.0; n], vec![0.0; n]);
    let lo_bin = (50.0 / bin_hz).ceil() as usize;
    let hi_bin = ((16000.0f64.min(rate as f64 / 2.0 - bin_hz)) / bin_hz) as usize;
    loop {
        // a short sound still gets one (zero-padded) frame
        if pos >= x.len() && frames > 0 || pos > x.len() {
            break;
        }
        let mut e = 0.0;
        for i in 0..n {
            let s = x.get(pos + i).copied().unwrap_or(0.0) as f64;
            e += s * s;
            re[i] = s * win[i];
            im[i] = 0.0;
        }
        pos += hop;
        let rms = (e / n as f64).sqrt();
        if rms < 1e-4 && !(pos >= x.len() && frames == 0) {
            if pos >= x.len() {
                break;
            }
            continue;
        }
        fft(&mut re, &mut im);
        let (mut num, mut den) = (0.0, 0.0);
        let (mut log_sum, mut lin_sum, mut cnt) = (0.0, 0.0, 0usize);
        for k in 0..n / 2 {
            let p = re[k] * re[k] + im[k] * im[k];
            power[k] += p;
            num += k as f64 * bin_hz * p;
            den += p;
            if k >= lo_bin && k <= hi_bin {
                log_sum += (p + 1e-20).ln();
                lin_sum += p;
                cnt += 1;
            }
        }
        if den > 0.0 {
            frame_centroids.push(num / den);
        }
        if cnt > 0 && lin_sum > 0.0 {
            flat_sum += (log_sum / cnt as f64).exp() / (lin_sum / cnt as f64);
        }
        frames += 1;
        if pos >= x.len() {
            break;
        }
    }
    if frames == 0 {
        return Spectrum { power, bin_hz, ..Spectrum::default() };
    }
    power.iter_mut().for_each(|p| *p /= frames as f64);
    // power-weighted: the noise floor of many quiet high bins must not pull it up
    let (num, den) = power.iter().enumerate().fold((0.0, 0.0), |(a, b), (k, p)| (a + k as f64 * bin_hz * p, b + p));
    Spectrum { power, bin_hz, centroid: if den > 0.0 { num / den } else { 0.0 }, flatness: flat_sum / frames as f64, frame_centroids }
}

/// RMS envelope with `win` samples per point.
fn envelope(x: &[f32], win: usize) -> Vec<f64> {
    x.chunks(win.max(1)).map(|c| (c.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / c.len() as f64).sqrt()).collect()
}

/// Mel-spaced band levels (dB, 24 bands 60 Hz..10 kHz) of an average power spectrum.
fn mel_bands(sp: &Spectrum, rate: u32) -> Vec<f64> {
    let mel = |f: f64| 2595.0 * (1.0 + f / 700.0).log10();
    let inv = |m: f64| 700.0 * (10f64.powf(m / 2595.0) - 1.0);
    let nb = 24;
    let (m0, m1) = (mel(60.0), mel(10000.0));
    let edges: Vec<f64> = (0..nb + 2).map(|i| inv(m0 + (m1 - m0) * i as f64 / (nb + 1) as f64)).collect();
    let nyq = rate as f64 / 2.0;
    let mut out = Vec::with_capacity(nb);
    for b in 0..nb {
        let (lo, mid, hi) = (edges[b], edges[b + 1], edges[b + 2]);
        let mut e = 0.0;
        for (k, p) in sp.power.iter().enumerate() {
            let f = k as f64 * sp.bin_hz;
            if f > lo && f < hi && f < nyq {
                let w = if f <= mid { (f - lo) / (mid - lo) } else { (hi - f) / (hi - mid) };
                e += w * p;
            }
        }
        out.push(10.0 * (e + 1e-12).log10());
    }
    // floor at 60 dB below the loudest band so empty bands (low sample
    // rates, silence) do not dominate comparisons
    let mx = out.iter().cloned().fold(f64::MIN, f64::max);
    out.iter_mut().for_each(|v| *v = v.max(mx - 60.0));
    out
}

/// Features used for similarity.
#[derive(Clone, Debug, Serialize)]
pub struct Fingerprint {
    /// Mel band levels in dB, mean removed (spectral shape).
    pub mel: Vec<f64>,
    /// RMS envelope over the audible part in 16 steps, peak = 1.
    pub env: Vec<f64>,
    pub duration: f64,
    pub looped: bool,
}

/// Seam check of a looping sound.
#[derive(Clone, Debug, Serialize)]
pub struct LoopCheck {
    /// Largest |last sample − first sample| over channels.
    pub wrap_jump: f64,
    /// RMS of the sample-to-sample difference within 10 ms of the wrap.
    pub typical_step: f64,
    /// Level of the last 20 ms minus the first 20 ms (200 ms for sounds over 5 s), dB.
    pub level_diff_db: f64,
    pub verdict: String,
}

/// Loudness of one window.
#[derive(Clone, Debug, Serialize)]
pub struct Window {
    pub start: f64,
    pub rms_db: f64,
    pub peak_db: f64,
    pub centroid_hz: f64,
}

/// Everything measured about one sound.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub duration: f64,
    pub rate: u32,
    pub channels: u16,
    pub format: String,
    pub peak_dbfs: f64,
    pub rms_dbfs: f64,
    pub clip_percent: f64,
    pub dc_offset: f64,
    pub silence_start_ms: f64,
    pub silence_end_ms: f64,
    pub centroid_hz: f64,
    /// Percent of power in sub/low/mid/high/air.
    pub bands: [f64; 5],
    pub flatness: f64,
    pub pitch_hz: Option<f64>,
    pub attack_ms: f64,
    pub description: String,
    pub loop_check: Option<LoopCheck>,
    pub fingerprint: Fingerprint,
}

/// Analyses a whole sound. `looped`: the sound is played looping (adds the
/// seam check). Sounds over 10 s are described as music.
pub fn analyse(a: &Audio, looped: bool) -> Report {
    analyse_inner(a, looped, false)
}

/// Like [`analyse`], but always describes the sound as music.
pub fn analyse_music(a: &Audio, looped: bool) -> Report {
    analyse_inner(a, looped, true)
}

fn analyse_inner(a: &Audio, looped: bool, music: bool) -> Report {
    let rate = a.rate.max(1);
    let x = a.mono();
    let ch = a.channels.max(1) as usize;
    let n = x.len().max(1);
    let peak = a.samples.iter().fold(0f32, |m, s| m.max(s.abs())) as f64;
    let rms = (a.samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / a.samples.len().max(1) as f64).sqrt();
    let clip = a.samples.iter().filter(|s| s.abs() >= 0.999).count() as f64 / a.samples.len().max(1) as f64 * 100.0;
    let dc = (0..ch)
        .map(|c| a.samples.iter().skip(c).step_by(ch).map(|s| *s as f64).sum::<f64>() / a.frames().max(1) as f64)
        .fold(0.0f64, |m, d| if d.abs() > m.abs() { d } else { m });
    let thr = 10f32.powf(-50.0 / 20.0);
    let first = x.iter().position(|s| s.abs() > thr);
    let last = x.iter().rposition(|s| s.abs() > thr);
    let (sil_start, sil_end) = match (first, last) {
        (Some(f), Some(l)) => (f as f64 / rate as f64 * 1000.0, (n - 1 - l) as f64 / rate as f64 * 1000.0),
        _ => (a.duration() * 1000.0, 0.0),
    };
    let audible: &[f32] = match (first, last) {
        (Some(f), Some(l)) => &x[f..=l],
        _ => &x[..0],
    };

    let sp = spectrum(audible, rate);
    let total: f64 = sp.power.iter().sum::<f64>().max(1e-30);
    let mut bands = [0.0; 5];
    for (k, p) in sp.power.iter().enumerate() {
        let f = k as f64 * sp.bin_hz;
        if let Some(b) = BANDS.iter().position(|(_, lo, hi)| f >= *lo && f < *hi) {
            bands[b] += p / total * 100.0;
        }
    }
    // pitch: strongest peak 40 Hz..5 kHz of tonal sounds, parabolic interpolation
    let pitch_hz = if sp.flatness < 0.12 && !sp.power.is_empty() {
        let lo = (40.0 / sp.bin_hz) as usize + 1;
        let hi = ((5000.0 / sp.bin_hz) as usize).min(sp.power.len() - 2);
        (lo..=hi).max_by(|&i, &j| sp.power[i].partial_cmp(&sp.power[j]).unwrap()).and_then(|k| {
            let (a, b, c) = (sp.power[k - 1].max(1e-30).ln(), sp.power[k].max(1e-30).ln(), sp.power[k + 1].max(1e-30).ln());
            let d = a - 2.0 * b + c;
            let off = if d.abs() > 1e-12 { 0.5 * (a - c) / d } else { 0.0 };
            (sp.power[k] > total * 0.05).then_some((k as f64 + off) * sp.bin_hz)
        })
    } else {
        None
    };

    // envelope: 5 ms windows
    let win = (rate as usize / 200).max(1);
    let env = envelope(audible, win);
    let env_max = env.iter().cloned().fold(0.0, f64::max);
    let attack_ms = if env_max > 0.0 {
        let start = env.iter().position(|e| *e > env_max * 0.01).unwrap_or(0);
        let top = env.iter().position(|e| *e >= env_max * 0.9).unwrap_or(0);
        (top.saturating_sub(start)) as f64 * win as f64 / rate as f64 * 1000.0
    } else {
        0.0
    };
    let env16: Vec<f64> = if env.is_empty() || env_max <= 0.0 {
        vec![0.0; 16]
    } else {
        (0..16)
            .map(|i| {
                let a = i * env.len() / 16;
                let b = ((i + 1) * env.len() / 16).max(a + 1).min(env.len());
                env[a.min(env.len() - 1)..b].iter().cloned().fold(0.0, f64::max) / env_max
            })
            .collect()
    };
    let mut mel = mel_bands(&sp, rate);
    let mm = mel.iter().sum::<f64>() / mel.len() as f64;
    mel.iter_mut().for_each(|v| *v -= mm);

    let loop_check = looped.then(|| {
        // per channel: the jump across the wrap against the RMS sample step
        // within 10 ms on both sides of it (so a drum hit on the downbeat,
        // which also has large steps right after the wrap, is not a click)
        let frames = a.frames();
        let w10 = (rate as usize / 100).clamp(2, (frames / 4).max(2));
        let (mut jump, mut step, mut worst) = (0.0f64, 0.0f64, -1.0f64);
        for c in 0..ch {
            let s = |i: usize| a.samples[i * ch + c] as f64;
            if frames < 4 {
                break;
            }
            let j = (s(0) - s(frames - 1)).abs();
            let mut acc = 0.0;
            let mut cnt = 0;
            for i in 1..w10.min(frames) {
                acc += (s(i) - s(i - 1)).powi(2) + (s(frames - i) - s(frames - i - 1)).powi(2);
                cnt += 2;
            }
            let st = (acc / cnt.max(1) as f64).sqrt();
            let ratio = j / st.max(1e-6);
            if ratio > worst {
                (jump, step, worst) = (j, st, ratio);
            }
        }
        // compare 20 ms at the seam (200 ms for music, whose beats make short windows noisy)
        let long = n as f64 / rate as f64 > 5.0;
        let w20 = (rate as usize / if long { 5 } else { 50 }).min(n / 4).max(1);
        let lvl = |s: &[f32]| (s.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / s.len().max(1) as f64).sqrt();
        let (end_l, start_l, all_l) = (lvl(&x[n - w20..]), lvl(&x[..w20]), lvl(&x));
        let level_diff_db = db(end_l) - db(start_l);
        // a downbeat right after the wrap is normal; a loop whose end (or start)
        // is far below its average level fades and restarts abruptly
        let (end_rel, start_rel) = (db(end_l) - db(all_l), db(start_l) - db(all_l));
        let fade = if long { 12.0 } else { 10.0 };
        let verdict = if sil_start + sil_end > 30.0 && first.is_some() {
            format!("GAP: {:.0} ms of silence at the wrap", sil_start + sil_end)
        } else if jump > 0.02 && jump > 4.0 * step {
            format!("CLICK likely: the wrap jumps {jump:.3} ({:.1}x the steps around it)", jump / step.max(1e-9))
        } else if end_rel < -fade && first.is_some() {
            format!("FADES OUT: the end is {:.0} dB below the average level, so the restart is abrupt", -end_rel)
        } else if start_rel < -fade && first.is_some() {
            format!("FADES IN: the start is {:.0} dB below the average level, so each pass dips", -start_rel)
        } else {
            "smooth".to_string()
        };
        LoopCheck { wrap_jump: jump, typical_step: step, level_diff_db, verdict }
    });

    let duration = a.duration();
    let audible_secs = audible.len() as f64 / rate as f64;
    let description = describe(audible_secs.max(if first.is_some() { 0.0 } else { duration }), attack_ms, &env, &sp, pitch_hz, first.is_none(), music);
    Report {
        duration,
        rate: a.rate,
        channels: a.channels,
        format: a.format.clone(),
        peak_dbfs: db(peak),
        rms_dbfs: db(rms),
        clip_percent: clip,
        dc_offset: dc,
        silence_start_ms: sil_start,
        silence_end_ms: sil_end,
        centroid_hz: sp.centroid,
        bands,
        flatness: sp.flatness,
        pitch_hz,
        attack_ms,
        description,
        loop_check,
        fingerprint: Fingerprint { mel, env: env16, duration, looped },
    }
}

fn describe(dur: f64, attack_ms: f64, env: &[f64], sp: &Spectrum, pitch: Option<f64>, silent: bool, music: bool) -> String {
    if silent {
        return "silent".into();
    }
    let len = if dur < 0.1 {
        "very short"
    } else if dur < 0.4 {
        "short"
    } else if dur < 1.5 {
        "medium"
    } else if dur < 6.0 {
        "long"
    } else {
        "very long"
    };
    let reg = match sp.centroid {
        c if c < 200.0 => "deep",
        c if c < 500.0 => "low",
        c if c < 1500.0 => "mid",
        c if c < 4000.0 => "bright",
        _ => "hissy",
    };
    if dur > 10.0 || (music && dur > 1.0) {
        // music / ambience: loudness steadiness over 1 s windows (env has 5 ms steps)
        let secs: Vec<f64> = env.chunks(200).filter(|c| c.len() >= 100).map(|c| db((c.iter().map(|v| v * v).sum::<f64>() / c.len() as f64).sqrt())).collect();
        let mut sorted = secs.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p = |q: f64| sorted.get(((sorted.len() - 1) as f64 * q) as usize).copied().unwrap_or(0.0);
        let range = p(0.9) - p(0.1);
        let dynamics = if range < 6.0 {
            "steady loudness"
        } else if range < 15.0 {
            "varied loudness"
        } else {
            "wide dynamics"
        };
        let top = p(1.0);
        let gaps = secs.iter().take(secs.len().saturating_sub(1)).filter(|v| **v < top - 30.0).count();
        let tex = if sp.flatness > 0.3 { ", noisy" } else { "" };
        let mut s = format!("music/ambience, {reg} register{tex}, {dynamics} ({range:.0} dB between quiet and loud seconds)");
        if gaps > 0 {
            s += &format!(", {gaps} near-silent second(s)");
        }
        return s;
    }
    let mx = env.iter().cloned().fold(0.0, f64::max).max(1e-12);
    let n = env.len().max(1);
    let tail: f64 = env[n * 7 / 10..].iter().cloned().fold(0.0, f64::max) / mx;
    // sustain: 10th percentile of the middle 80 %, smoothed over 50 ms
    let sm: Vec<f64> = env.chunks(10).map(|c| c.iter().sum::<f64>() / c.len() as f64).collect();
    let mid = &sm[sm.len() / 10..(sm.len() * 9 / 10).max(sm.len() / 10 + 1).min(sm.len())];
    let mut sorted = mid.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let floor = sorted.get(sorted.len() / 10).copied().unwrap_or(0.0) / mx;
    let shape = if attack_ms > dur * 1000.0 * 0.35 && dur > 0.15 {
        "swelling"
    } else if floor > 0.3 && dur > 0.3 {
        "sustained"
    } else if attack_ms < 30.0 && tail < 0.2 {
        "percussive"
    } else {
        "decaying"
    };
    let tex = if sp.flatness > 0.3 {
        "noise"
    } else if sp.flatness < 0.06 {
        "tone"
    } else {
        "noisy tone"
    };
    let mut s = format!("{len} {shape} {reg} {tex}");
    if let Some(p) = pitch {
        s += &format!(" ~{p:.0} Hz ({})", freq_note_name(p));
    }
    let fc = &sp.frame_centroids;
    if fc.len() >= 6 {
        let k = fc.len() / 3;
        let a = fc[..k].iter().sum::<f64>() / k as f64;
        let b = fc[fc.len() - k..].iter().sum::<f64>() / k as f64;
        if b > a * 1.35 {
            s += ", rising";
        } else if b < a / 1.35 {
            s += ", falling";
        }
    }
    s
}

/// Loudness per window of `secs` seconds.
pub fn windows(a: &Audio, secs: f64) -> Vec<Window> {
    let x = a.mono();
    let w = ((a.rate as f64 * secs) as usize).max(1);
    x.chunks(w)
        .enumerate()
        .map(|(i, c)| {
            let rms = (c.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / c.len() as f64).sqrt();
            let peak = c.iter().fold(0f32, |m, s| m.max(s.abs())) as f64;
            Window { start: i as f64 * secs, rms_db: db(rms), peak_db: db(peak), centroid_hz: spectrum(c, a.rate).centroid }
        })
        .collect()
}

/// Similarity 0..1 of two sounds: spectral shape correlation (70 %) and
/// envelope shape (30 %, ignored between two loops, whose envelope is
/// their length), scaled down for very different durations of one-shots.
pub fn similarity(a: &Fingerprint, b: &Fingerprint) -> f64 {
    let dot: f64 = a.mel.iter().zip(&b.mel).map(|(x, y)| x * y).sum();
    let na = a.mel.iter().map(|x| x * x).sum::<f64>().sqrt();
    let nb = b.mel.iter().map(|x| x * x).sum::<f64>().sqrt();
    let spec = if na > 1e-9 && nb > 1e-9 { (dot / (na * nb)).max(0.0) } else { 0.0 };
    if a.looped && b.looped {
        return spec;
    }
    let env = 1.0 - a.env.iter().zip(&b.env).map(|(x, y)| (x - y).abs()).sum::<f64>() / a.env.len().max(1) as f64;
    let (d0, d1) = (a.duration.min(b.duration).max(0.01), a.duration.max(b.duration).max(0.01));
    let dur = (d0 / d1).sqrt().max(0.5);
    (0.7 * spec + 0.3 * env) * dur
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(f: f64, secs: f64, rate: u32, amp: f32) -> Audio {
        let n = (secs * rate as f64) as usize;
        let samples = (0..n).map(|i| (2.0 * std::f64::consts::PI * f * i as f64 / rate as f64).sin() as f32 * amp).collect();
        Audio { rate, channels: 1, samples, format: "test".into() }
    }

    fn noise(secs: f64, rate: u32, seed: u32) -> Audio {
        let mut s = seed;
        let n = (secs * rate as f64) as usize;
        let samples = (0..n)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                (s as f32 / u32::MAX as f32 - 0.5) * 0.8
            })
            .collect();
        Audio { rate, channels: 1, samples, format: "test".into() }
    }

    #[test]
    fn fft_of_impulse_is_flat() {
        let mut re = vec![0.0; 8];
        let mut im = vec![0.0; 8];
        re[0] = 1.0;
        fft(&mut re, &mut im);
        assert!(re.iter().all(|v| (v - 1.0).abs() < 1e-12) && im.iter().all(|v| v.abs() < 1e-12));
    }

    #[test]
    fn sine_centroid_pitch_levels() {
        let r = analyse(&sine(1000.0, 1.0, 44100, 0.5), false);
        assert!((r.centroid_hz - 1000.0).abs() < 60.0, "centroid {}", r.centroid_hz);
        assert!((r.pitch_hz.unwrap() - 1000.0).abs() < 5.0, "pitch {:?}", r.pitch_hz);
        assert!((r.peak_dbfs - -6.02).abs() < 0.1);
        assert!((r.rms_dbfs - -9.03).abs() < 0.1);
        assert!(r.bands[2] > 99.0);
        assert!(r.description.contains("tone") && r.description.contains("sustained"), "{}", r.description);
        let r = analyse(&sine(110.0, 1.0, 22050, 0.5), false);
        assert!((r.pitch_hz.unwrap() - 110.0).abs() < 3.0);
        assert!(r.bands[1] > 99.0);
    }

    #[test]
    fn noise_is_noise_and_loops_are_checked() {
        let r = analyse(&noise(1.0, 22050, 1), true);
        assert!(r.flatness > 0.3, "flatness {}", r.flatness);
        assert!(r.description.contains("noise"), "{}", r.description);
        assert!(r.centroid_hz > 3000.0);
        // a sine of a whole number of periods loops smoothly; a cut one clicks
        let ok = analyse(&sine(100.0, 1.0, 22050, 0.5), true);
        assert_eq!(ok.loop_check.unwrap().verdict, "smooth");
        let cut = analyse(&sine(100.0, 1.0025, 22050, 0.5), true);
        assert!(cut.loop_check.unwrap().verdict.starts_with("CLICK"));
    }

    #[test]
    fn percussive_envelope_and_similarity() {
        let mut hit = noise(0.3, 22050, 7);
        let n = hit.samples.len();
        for (i, s) in hit.samples.iter_mut().enumerate() {
            *s *= (-(i as f32) / n as f32 * 12.0).exp();
        }
        let r = analyse(&hit, false);
        assert!(r.description.contains("percussive"), "{}", r.description);
        let a = analyse(&noise(1.0, 22050, 3), true).fingerprint;
        let b = analyse(&noise(1.5, 22050, 9), true).fingerprint;
        let c = analyse(&sine(220.0, 1.0, 22050, 0.5), true).fingerprint;
        assert!(similarity(&a, &b) > 0.9, "{}", similarity(&a, &b));
        assert!(similarity(&a, &c) < 0.5, "{}", similarity(&a, &c));
    }
}
