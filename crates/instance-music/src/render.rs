//! Offline rendering through a SoundFont (rustysynth, the synthesiser the
//! runner uses for MIDI when no OS synth is available), including seamless
//! loop rendering.
//!
//! This module depends on rustysynth directly instead of on the runner's
//! sequencer: the compiled [`Song`] already has absolute event times, so
//! sequencing is a sort, and the runner crate would pull in a window system,
//! GL and audio-device dependencies for nothing.

use crate::compile::Song;
use crate::synth::Engine;
use crate::wav::Audio;
use std::sync::Arc;

pub use rustysynth::SoundFont;

/// Synthesiser block size: events take effect on block boundaries.
const BLOCK: usize = 64;

/// Finds a SoundFont: `explicit`, then `INSTANCE_SOUNDFONT` / `GM81_SOUNDFONT`,
/// then `soundfonts/default.sf2` next to the executable or in the working
/// directory, then common system locations.
pub fn find_soundfont(explicit: Option<&str>) -> Option<std::path::PathBuf> {
    let mut c: Vec<std::path::PathBuf> = Vec::new();
    if let Some(p) = explicit {
        return Some(p.into());
    }
    for v in ["INSTANCE_SOUNDFONT", "GM81_SOUNDFONT"] {
        if let Ok(p) = std::env::var(v) {
            if !p.is_empty() {
                c.push(p.into());
            }
        }
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        c.push(dir.join("soundfonts").join("default.sf2"));
    }
    c.push("soundfonts/default.sf2".into());
    c.push("web/soundfonts/default.sf2".into());
    for p in ["/usr/share/sounds/sf2/FluidR3_GM.sf2", "/usr/share/soundfonts/default.sf2", "/usr/share/soundfonts/FluidR3_GM.sf2"] {
        c.push(p.into());
    }
    c.into_iter().find(|p| p.is_file())
}

/// Loads a SoundFont file.
pub fn load_soundfont(path: &std::path::Path) -> Result<Arc<SoundFont>, String> {
    let f = std::fs::File::open(path).map_err(|e| format!("open SoundFont {}: {e}", path.display()))?;
    SoundFont::new(&mut std::io::BufReader::new(f)).map(Arc::new).map_err(|e| format!("load SoundFont {}: {e:?}", path.display()))
}

/// Render options.
#[derive(Clone, Debug)]
pub struct RenderOpts {
    pub rate: u32,
    pub mono: bool,
    /// Render the loop region as a seamless loop.
    pub looped: bool,
    /// Maximum release tail after the end (non-looped), seconds.
    pub tail: f64,
    /// Normalise the peak to this many dBFS (otherwise only clipping is prevented).
    pub normalize: Option<f64>,
}

impl Default for RenderOpts {
    fn default() -> Self {
        RenderOpts { rate: 44100, mono: false, looped: false, tail: 3.0, normalize: None }
    }
}

/// What a render did to the level.
#[derive(Clone, Debug)]
pub struct RenderInfo {
    /// Peak before any gain, linear.
    pub raw_peak: f32,
    /// Gain applied, dB.
    pub gain_db: f64,
}

/// Plays timed channel messages (seconds, bytes) through a synthesiser and
/// returns interleaved stereo audio covering `[0, until)` seconds.
/// When `until` is None it renders past the last event until silence or
/// `tail` seconds.
pub fn render_events(sf: &Arc<SoundFont>, rate: u32, events: Vec<(f64, Vec<u8>)>, until: Option<f64>, tail: f64) -> Result<Vec<f32>, String> {
    render_events_with(Some(sf), None, rate, events, until, tail)
}

/// [`render_events`] with the built-in synthesiser: messages for its
/// channels go to `engine`, the rest to the SoundFont (which may be absent
/// when the engine plays every channel).
pub fn render_events_with(sf: Option<&Arc<SoundFont>>, mut engine: Option<&mut Engine>, rate: u32, mut events: Vec<(f64, Vec<u8>)>, until: Option<f64>, tail: f64) -> Result<Vec<f32>, String> {
    let mut synth = match sf {
        Some(sf) => {
            let mut settings = rustysynth::SynthesizerSettings::new(rate as i32);
            settings.maximum_polyphony = 128;
            settings.block_size = BLOCK;
            Some(rustysynth::Synthesizer::new(sf, &settings).map_err(|e| format!("synth: {e:?}"))?)
        }
        None => None,
    };
    events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let last = events.last().map(|e| e.0).unwrap_or(0.0);
    let stop_at = until.map(|u| (u * rate as f64).round() as usize);
    let max = stop_at.unwrap_or(((last + tail) * rate as f64) as usize + 1);
    let mut out: Vec<f32> = Vec::with_capacity(max * 2);
    let (mut l, mut r) = (vec![0f32; 256], vec![0f32; 256]);
    let mut pos = 0usize;
    let mut i = 0;
    let last_sample = (last * rate as f64).round() as usize;
    loop {
        while i < events.len() && (events[i].0 * rate as f64).round() as usize <= pos {
            let m = &events[i].1;
            if !m.is_empty() && m[0] >= 0x80 && m[0] < 0xF0 {
                let d1 = m.get(1).copied().unwrap_or(0);
                let d2 = m.get(2).copied().unwrap_or(0);
                let mine = engine.as_mut().map(|e| e.message(m[0] & 0x0F, m[0] & 0xF0, d1, d2)).unwrap_or(false);
                if !mine {
                    if let Some(s) = synth.as_mut() {
                        s.process_midi_message((m[0] & 0x0F) as i32, (m[0] & 0xF0) as i32, d1 as i32, d2 as i32);
                    }
                }
            }
            i += 1;
        }
        if pos >= max {
            break;
        }
        let next_ev = events.get(i).map(|e| (e.0 * rate as f64).round() as usize).unwrap_or(usize::MAX);
        let n = 256.min(max - pos).min(next_ev.saturating_sub(pos).max(1));
        match synth.as_mut() {
            Some(s) => s.render(&mut l[..n], &mut r[..n]),
            None => {
                l[..n].iter_mut().for_each(|x| *x = 0.0);
                r[..n].iter_mut().for_each(|x| *x = 0.0);
            }
        }
        if let Some(e) = engine.as_mut() {
            e.render(&mut l[..n], &mut r[..n]);
        }
        for k in 0..n {
            out.push(l[k]);
            out.push(r[k]);
        }
        pos += n;
        if stop_at.is_none() && pos > last_sample + rate as usize / 10 {
            // stop once the tail has fallen silent
            let peak = l[..n].iter().chain(r[..n].iter()).fold(0f32, |m, x| m.max(x.abs()));
            if peak < 3e-5 {
                break;
            }
        }
    }
    Ok(out)
}

fn song_events(song: &Song, filter: impl Fn(u64) -> bool, shift_secs: f64) -> Vec<(f64, Vec<u8>)> {
    let mut v = Vec::new();
    for tr in &song.tracks {
        // group by note: note-offs follow their note-on's filter decision
        let evs = song.track_events(tr);
        let mut on_keep = [[false; 128]; 16];
        for (t, m) in evs {
            let st = m[0] & 0xF0;
            let ch = (m[0] & 0x0F) as usize;
            let keep = if st == 0x80 {
                on_keep[ch][m[1] as usize & 127]
            } else {
                let k = filter(t);
                if st == 0x90 {
                    on_keep[ch][m[1] as usize & 127] = k;
                }
                k
            };
            if keep {
                v.push((song.secs_at(t) + shift_secs, m));
            }
        }
    }
    v
}

/// Renders a compiled song. With `opts.looped` the result is exactly one
/// pass of the loop region, rendered as the second of two consecutive
/// passes so the release tail of the end is already sounding at the start:
/// played in a loop it has no gap and no click.
pub fn render_song(song: &Song, sf: &Arc<SoundFont>, opts: &RenderOpts) -> Result<(Audio, RenderInfo), String> {
    render_song_with(song, Some(sf), opts)
}

/// Whether rendering `song` needs a SoundFont (some track is not a `synth=` track).
pub fn needs_soundfont(song: &Song) -> bool {
    song.tracks.iter().any(|t| t.def.synth.is_none())
}

/// [`render_song`]; `synth=` tracks are played by the built-in synthesiser,
/// the others by the SoundFont (required only if there are such tracks).
pub fn render_song_with(song: &Song, sf: Option<&Arc<SoundFont>>, opts: &RenderOpts) -> Result<(Audio, RenderInfo), String> {
    let rate = opts.rate;
    if sf.is_none() && needs_soundfont(song) {
        return Err("this song has tracks without synth=: WAV output needs a SoundFont".into());
    }
    let mut engine = Engine::for_song(song, rate)?;
    let st = if opts.looped {
        let (ls, le) = song.loop_range;
        let (s0, s1) = (song.secs_at(ls), song.secs_at(le));
        // The synthesiser starts events on 64-sample block boundaries; unless
        // the loop is a whole number of blocks, sounds still ringing from the
        // previous pass are out of phase at the wrap (a faint click). So time
        // is stretched by at most 32 samples per pass to make it whole.
        let exact = (s1 - s0) * rate as f64;
        let n = ((exact / BLOCK as f64).round() as usize).max(1) * BLOCK;
        let f = n as f64 / exact;
        let mut ev = song_events(song, |t| t < le, 0.0);
        ev.extend(song_events(song, |t| t >= ls && t < le, s1 - s0));
        ev.iter_mut().for_each(|e| e.0 *= f);
        let start = (s1 * f * rate as f64).round() as usize;
        let all = render_events_with(sf, engine.as_mut(), rate, ev, Some((start + n) as f64 / rate as f64), 0.0)?;
        let mut v = all[(start * 2).min(all.len())..].to_vec();
        v.resize(n * 2, 0.0);
        v
    } else {
        let ev = song_events(song, |_| true, 0.0);
        let mut v = render_events_with(sf, engine.as_mut(), rate, ev, None, opts.tail)?;
        let min = (song.duration() * rate as f64).round() as usize * 2;
        if v.len() < min {
            v.resize(min, 0.0);
        }
        v
    };
    let (channels, mut st): (u16, Vec<f32>) = if opts.mono { (1, st.chunks(2).map(|c| (c[0] + c[1]) * 0.5).collect()) } else { (2, st) };
    let raw_peak = st.iter().fold(0f32, |m, x| m.max(x.abs()));
    let mut gain_db = 0.0;
    if let Some(db) = opts.normalize {
        if raw_peak > 0.0 {
            gain_db = db - 20.0 * (raw_peak as f64).log10();
        }
    } else if raw_peak > 0.989 {
        gain_db = -0.1 - 20.0 * (raw_peak as f64).log10();
    }
    if gain_db != 0.0 {
        let g = 10f32.powf(gain_db as f32 / 20.0);
        st.iter_mut().for_each(|x| *x *= g);
    }

    Ok((Audio { rate, channels, samples: st, format: "rendered".into() }, RenderInfo { raw_peak, gain_db }))
}

/// Renders a Standard MIDI File (any type) through the SoundFont.
pub fn render_midi_file(data: &[u8], sf: &Arc<SoundFont>, rate: u32) -> Result<Audio, String> {
    use midly::{MidiMessage, Smf, Timing, TrackEventKind};
    let smf = Smf::parse(data).map_err(|e| e.to_string())?;
    let (ppq, fixed) = match smf.header.timing {
        Timing::Metrical(t) => (t.as_int().max(1) as f64, None),
        Timing::Timecode(fps, sub) => (1.0, Some(1.0 / (fps.as_f32() as f64 * sub.max(1) as f64))),
    };
    let mut raw: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut tempos: Vec<(u64, f64)> = Vec::new();
    for tr in &smf.tracks {
        let mut t = 0u64;
        for e in tr {
            t += e.delta.as_int() as u64;
            match e.kind {
                TrackEventKind::Midi { channel, message } => {
                    let c = channel.as_int();
                    let m = match message {
                        MidiMessage::NoteOff { key, .. } => vec![0x80 | c, key.as_int(), 0],
                        MidiMessage::NoteOn { key, vel } => vec![0x90 | c, key.as_int(), vel.as_int()],
                        MidiMessage::Aftertouch { key, vel } => vec![0xA0 | c, key.as_int(), vel.as_int()],
                        MidiMessage::Controller { controller, value } => vec![0xB0 | c, controller.as_int(), value.as_int()],
                        MidiMessage::ProgramChange { program } => vec![0xC0 | c, program.as_int()],
                        MidiMessage::ChannelAftertouch { vel } => vec![0xD0 | c, vel.as_int()],
                        MidiMessage::PitchBend { bend } => {
                            let b = bend.0.as_int();
                            vec![0xE0 | c, (b & 0x7F) as u8, (b >> 7) as u8]
                        }
                    };
                    raw.push((t, m));
                }
                TrackEventKind::Meta(midly::MetaMessage::Tempo(us)) => tempos.push((t, us.as_int() as f64)),
                _ => {}
            }
        }
    }
    tempos.sort_by_key(|x| x.0);
    let secs = |tick: u64| -> f64 {
        if let Some(f) = fixed {
            return tick as f64 * f;
        }
        let mut s = 0.0;
        let mut last = 0;
        let mut us = 500_000.0;
        for &(t, u) in &tempos {
            if t >= tick {
                break;
            }
            s += (t - last) as f64 * us / 1e6 / ppq;
            last = t;
            us = u;
        }
        s + (tick - last) as f64 * us / 1e6 / ppq
    };
    let ev: Vec<(f64, Vec<u8>)> = raw.into_iter().map(|(t, m)| (secs(t), m)).collect();
    let samples = render_events(sf, rate, ev, None, 3.0)?;
    Ok(Audio { rate, channels: 2, samples, format: "MIDI (rendered)".into() })
}
