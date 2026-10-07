//! Sound engine (specs/sound.md, runtime.md §15).
//!
//! * PCM sounds (WAV, MP3, OGG, … via rodio/symphonia) are decoded once into a
//!   per-sound cache and played as [`voice::Voice`] sources on the rodio mixer;
//!   each voice applies the sound's effects ([`dsp`]), volume/fade/pan and 3D
//!   model, read from a shared per-sound control block.
//! * MIDI files are sequenced by our own player ([`midi`]). On Windows the
//!   events go to the OS synthesiser (winmm `midiOut`, "Microsoft GS Wavetable
//!   Synth") from a dedicated thread ([`midi_out`]); volume is applied through
//!   channel volume (CC7). When no OS synthesiser can be opened (or
//!   `GM81_MIDI=soundfont` is set) MIDI is rendered with `rustysynth` into the
//!   PCM mixer using a SoundFont found on this search path:
//!     1. the file named by `GM81_SOUNDFONT`,
//!     2. `<runner exe dir>/soundfonts/default.sf2`,
//!     3. platform locations (`/usr/share/sounds/sf2/FluidR3_GM.sf2`,
//!        `/usr/share/soundfonts/default.sf2`, `/usr/share/soundfonts/FluidR3_GM.sf2`,
//!        `~/Library/Audio/Sounds/Banks/default.sf2`).
//!
//!   No SoundFont ships with gm81-clean. Without one MIDI is silent but
//!   `sound_isplaying` follows the logical state.
//! * Without an audio device (or in headless runs) every call succeeds and
//!   `sound_isplaying` follows the logical state (sound length in real time).
//! * `GM81_AUDIO_DEBUG=1` logs backend choices and MIDI message counts.

pub mod dsp;
pub mod midi;
pub mod midi_out;
pub mod voice;

use crate::res::SoundRes;
use midi::Sequence;
use midi_out::{Cmd, MidiThread};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use voice::{Globals, Pcm, Sound3d, SoundCtl, VoiceState};

/// Maximum simultaneous PCM voices (§2).
pub const MAX_VOICES: usize = 128;

pub(crate) fn debug() -> bool {
    static D: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *D.get_or_init(|| std::env::var("GM81_AUDIO_DEBUG").map(|v| !v.is_empty() && v != "0").unwrap_or(false))
}

macro_rules! alog {
    ($($t:tt)*) => { if debug() { eprintln!("audio: {}", format!($($t)*)); } };
}

/// Sound kind clamped to 0..3 (§7: out of range → 0).
pub fn kind_of(res: &SoundRes) -> i32 {
    if (0..=3).contains(&res.kind) {
        res.kind
    } else {
        0
    }
}

fn data_key(d: &Arc<[u8]>) -> usize {
    Arc::as_ptr(d) as *const u8 as usize
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Format {
    Midi,
    /// DirectMusic segment: exists but plays nothing.
    Segment,
    Pcm,
}

fn detect(d: &[u8]) -> Format {
    if midi::is_midi(d) {
        Format::Midi
    } else if d.len() >= 12 && &d[0..4] == b"RIFF" && &d[8..12] == b"DMSG" {
        Format::Segment
    } else {
        Format::Pcm
    }
}

/// Searches the SoundFont path (see module docs).
pub fn find_soundfont() -> Option<std::path::PathBuf> {
    let mut cands: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("GM81_SOUNDFONT") {
        if !p.is_empty() {
            cands.push(p.into());
        }
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        cands.push(dir.join("soundfonts").join("default.sf2"));
    }
    for p in ["/usr/share/sounds/sf2/FluidR3_GM.sf2", "/usr/share/soundfonts/default.sf2", "/usr/share/soundfonts/FluidR3_GM.sf2"] {
        cands.push(p.into());
    }
    if let Ok(h) = std::env::var("HOME") {
        cands.push(std::path::Path::new(&h).join("Library/Audio/Sounds/Banks/default.sf2"));
    }
    cands.into_iter().find(|p| p.is_file())
}

enum MidiBackend {
    Untried,
    Os(MidiThread),
    SoundFont(Arc<rustysynth::SoundFont>),
    Silent,
}

struct VoiceHandle {
    state: Arc<VoiceState>,
    kind: i32,
    midi: bool,
    looped: bool,
    serial: u64,
    /// Simulated playback (no device / no synth): logical end time, None = forever.
    simulated: Option<Option<Instant>>,
}

impl VoiceHandle {
    fn alive(&self, now: Instant) -> bool {
        match self.simulated {
            Some(end) => !self.state.stop_requested() && end.map(|e| now < e).unwrap_or(true),
            None => !self.state.is_done(),
        }
    }
}

type PcmCache = Arc<Mutex<HashMap<i32, (usize, Arc<Pcm>)>>>;

/// The audio engine (owned by the game thread).
pub struct Audio {
    headless: bool,
    sink: Option<rodio::MixerDeviceSink>,
    tried: bool,
    globals: Arc<Globals>,
    ctls: HashMap<i32, Arc<SoundCtl>>,
    voices: HashMap<i32, Vec<VoiceHandle>>,
    serial: u64,
    cache: PcmCache,
    seqs: HashMap<i32, (usize, Arc<Sequence>)>,
    durations: HashMap<i32, (usize, f64)>,
    midi: MidiBackend,
}

impl Audio {
    pub fn new(headless: bool) -> Self {
        Audio {
            headless,
            sink: None,
            tried: false,
            globals: Arc::new(Globals::default()),
            ctls: HashMap::new(),
            voices: HashMap::new(),
            serial: 0,
            cache: Arc::new(Mutex::new(HashMap::new())),
            seqs: HashMap::new(),
            durations: HashMap::new(),
            midi: MidiBackend::Untried,
        }
    }

    fn device(&mut self) -> Option<&rodio::MixerDeviceSink> {
        if self.headless {
            return None;
        }
        if !self.tried {
            self.tried = true;
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut s) => {
                    s.log_on_drop(false);
                    self.sink = Some(s)
                }
                Err(e) => eprintln!("audio: no output device ({}); sounds are disabled", e),
            }
        }
        self.sink.as_ref()
    }

    // ------------------------------------------------------------------ resources

    /// Decodes preloaded sounds in the background (§7) and opens the MIDI
    /// backend early when the game has MIDI sounds.
    pub fn preload(&mut self, sounds: &[Option<SoundRes>]) {
        if self.headless {
            return;
        }
        let mut jobs = Vec::new();
        let mut has_midi = false;
        for (i, s) in sounds.iter().enumerate() {
            let Some(s) = s else { continue };
            let Some(d) = s.data.clone() else { continue };
            match detect(&d) {
                Format::Midi => {
                    has_midi = true;
                    let _ = self.sequence(i as i32, &d);
                }
                Format::Pcm if s.preload && kind_of(s) != 3 => jobs.push((i as i32, d)),
                _ => {}
            }
        }
        if has_midi {
            self.midi_backend();
        }
        if jobs.is_empty() || self.device().is_none() {
            return;
        }
        let cache = self.cache.clone();
        let _ = std::thread::Builder::new().name("sound-preload".into()).spawn(move || {
            for (i, d) in jobs {
                let key = data_key(&d);
                if cache.lock().unwrap().get(&i).map(|e| e.0 == key).unwrap_or(false) {
                    continue;
                }
                if let Some(p) = voice::decode(d) {
                    cache.lock().unwrap().insert(i, (key, Arc::new(p)));
                }
            }
        });
    }

    fn pcm(&mut self, idx: i32, d: &Arc<[u8]>) -> Option<Arc<Pcm>> {
        let key = data_key(d);
        if let Some((k, p)) = self.cache.lock().unwrap().get(&idx) {
            if *k == key {
                return Some(p.clone());
            }
        }
        let p = Arc::new(voice::decode(d.clone())?);
        self.cache.lock().unwrap().insert(idx, (key, p.clone()));
        Some(p)
    }

    fn sequence(&mut self, idx: i32, d: &Arc<[u8]>) -> Option<Arc<Sequence>> {
        let key = data_key(d);
        if let Some((k, s)) = self.seqs.get(&idx) {
            if *k == key {
                return Some(s.clone());
            }
        }
        match Sequence::parse(d) {
            Ok(s) => {
                let s = Arc::new(s);
                self.seqs.insert(idx, (key, s.clone()));
                Some(s)
            }
            Err(e) => {
                alog!("sound {}: bad MIDI file ({})", idx, e);
                None
            }
        }
    }

    /// Length in seconds of a PCM sound without keeping it decoded.
    fn duration(&mut self, idx: i32, d: &Arc<[u8]>) -> f64 {
        let key = data_key(d);
        if let Some((k, s)) = self.durations.get(&idx) {
            if *k == key {
                return *s;
            }
        }
        let secs = if let Some((k, p)) = self.cache.lock().unwrap().get(&idx) {
            if *k == key {
                Some(p.duration())
            } else {
                None
            }
        } else {
            None
        };
        let secs = secs.unwrap_or_else(|| {
            use rodio::Source;
            match rodio::Decoder::new(std::io::Cursor::new(d.clone())) {
                Ok(dec) => match dec.total_duration() {
                    Some(t) => t.as_secs_f64(),
                    None => voice::decode(d.clone()).map(|p| p.duration()).unwrap_or(0.0),
                },
                Err(_) => 0.0,
            }
        });
        self.durations.insert(idx, (key, secs));
        secs
    }

    /// `sound_discard`: frees decoded audio (and stops the sound).
    pub fn discard(&mut self, idx: i32) {
        self.stop(idx);
        self.cache.lock().unwrap().remove(&idx);
    }

    /// `sound_restore`: decodes the sound now.
    pub fn restore(&mut self, idx: i32, res: &SoundRes) {
        let Some(d) = res.data.clone() else { return };
        match detect(&d) {
            Format::Pcm if !self.headless && kind_of(res) != 3 => {
                let _ = self.pcm(idx, &d);
            }
            Format::Midi => {
                let _ = self.sequence(idx, &d);
            }
            _ => {}
        }
    }

    /// The sound was replaced or deleted: stop it and drop all cached state.
    pub fn forget(&mut self, idx: i32) {
        self.stop(idx);
        self.ctls.remove(&idx);
        self.cache.lock().unwrap().remove(&idx);
        self.seqs.remove(&idx);
        self.durations.remove(&idx);
    }

    // ------------------------------------------------------------------ control blocks

    fn ctl(&mut self, idx: i32, res: &SoundRes) -> Arc<SoundCtl> {
        let kind = kind_of(res);
        let c = self.ctls.entry(idx).or_insert_with(|| Arc::new(SoundCtl::new(kind, res.volume, res.pan, res.effects)));
        {
            let mut p = c.p.lock().unwrap();
            p.kind = kind;
        }
        c.clone()
    }

    pub fn set_volume(&mut self, idx: i32, res: &SoundRes, v: f64) {
        self.ctl(idx, res).set_volume(v);
    }

    /// Current (possibly fading) volume.
    pub fn volume(&mut self, idx: i32, res: &SoundRes) -> f64 {
        self.ctl(idx, res).volume_now()
    }

    pub fn fade(&mut self, idx: i32, res: &SoundRes, v: f64, ms: f64) {
        self.ctl(idx, res).fade(v, ms);
    }

    pub fn set_pan(&mut self, idx: i32, res: &SoundRes, pan: f64) {
        let c = self.ctl(idx, res);
        c.p.lock().unwrap().pan = if pan.is_nan() { 0.0 } else { pan.clamp(-1.0, 1.0) };
    }

    pub fn set_global_volume(&mut self, v: f64) {
        self.globals.set_volume(if v.is_nan() { 0.0 } else { v });
    }

    pub fn global_volume(&self) -> f64 {
        self.globals.volume()
    }

    /// `sound_background_tempo`.
    pub fn set_tempo(&mut self, t: f64) {
        self.globals.set_tempo(t);
    }

    /// `sound_effect_set`.
    pub fn effect_set(&mut self, idx: i32, res: &SoundRes, flags: i32) {
        self.ctl(idx, res).update_effects(|e| {
            e.set(flags);
            true
        });
    }

    /// `sound_effect_*` parameter setters.
    pub fn effect_params(&mut self, idx: i32, res: &SoundRes, effect: i32, args: &[f64]) {
        self.ctl(idx, res).update_effects(|e| e.set_params(effect, args));
    }

    /// `sound_3d_*`: ignored unless the sound is of kind 2.
    pub fn set_3d(&mut self, idx: i32, res: &SoundRes, f: impl FnOnce(&mut Sound3d)) {
        if kind_of(res) != 2 {
            return;
        }
        let c = self.ctl(idx, res);
        let mut p = c.p.lock().unwrap();
        f(&mut p.s3d);
    }

    // ------------------------------------------------------------------ MIDI backend

    fn midi_backend(&mut self) -> &MidiBackend {
        if let MidiBackend::Untried = self.midi {
            self.midi = MidiBackend::Silent;
            if !self.headless {
                let force_sf = std::env::var("GM81_MIDI").map(|v| v.eq_ignore_ascii_case("soundfont")).unwrap_or(false);
                if !force_sf {
                    match MidiThread::start(self.globals.clone()) {
                        Ok(t) => {
                            alog!("MIDI: OS synthesiser opened: {}", t.stats.device.lock().unwrap());
                            self.midi = MidiBackend::Os(t);
                        }
                        Err(e) => alog!("MIDI: OS synthesiser unavailable ({})", e),
                    }
                }
                if let MidiBackend::Silent = self.midi {
                    match find_soundfont() {
                        Some(p) => match std::fs::File::open(&p).map_err(|e| e.to_string()).and_then(|f| {
                            rustysynth::SoundFont::new(&mut std::io::BufReader::new(f)).map_err(|e| format!("{:?}", e))
                        }) {
                            Ok(sf) => {
                                alog!("MIDI: SoundFont {}", p.display());
                                self.midi = MidiBackend::SoundFont(Arc::new(sf));
                            }
                            Err(e) => eprintln!("audio: cannot load SoundFont {}: {}", p.display(), e),
                        },
                        None => eprintln!("audio: no MIDI synthesiser (set GM81_SOUNDFONT to a .sf2 file); MIDI music is silent"),
                    }
                }
            }
        }
        &self.midi
    }

    /// Diagnostic summary of the OS MIDI path, if active.
    pub fn midi_stats(&self) -> Option<String> {
        match &self.midi {
            MidiBackend::Os(t) => Some(t.stats.summary()),
            MidiBackend::SoundFont(_) => Some("SoundFont synthesiser".into()),
            _ => None,
        }
    }

    // ------------------------------------------------------------------ playback

    fn stop_where(&mut self, pred: impl Fn(&VoiceHandle) -> bool) {
        for list in self.voices.values_mut() {
            list.retain(|v| {
                if pred(v) {
                    v.state.request_stop();
                    false
                } else {
                    true
                }
            });
        }
        self.voices.retain(|_, l| !l.is_empty());
    }

    /// Plays sound `idx` once or looped (§2).
    pub fn play(&mut self, idx: i32, res: &SoundRes, looped: bool, _all: &[Option<SoundRes>]) {
        let kind = kind_of(res);
        let Some(data) = res.data.clone() else { return };
        let fmt = detect(&data);
        if fmt == Format::Segment {
            alog!("sound {}: DirectMusic segments are not supported", idx);
            return;
        }
        // exclusivity rules
        let is_midi = fmt == Format::Midi;
        self.stop_where(|v| (kind == 1 && v.kind == 1) || (kind == 3 && v.kind == 3) || (is_midi && v.midi));
        self.cleanup();
        let ctl = self.ctl(idx, res);
        let state = Arc::new(VoiceState::default());
        self.serial += 1;
        let mut h = VoiceHandle { state: state.clone(), kind, midi: is_midi, looped, serial: self.serial, simulated: None };
        let now = Instant::now();
        let logical = |secs: f64| if looped { None } else { Some(now + Duration::from_secs_f64(secs.max(0.0))) };
        if is_midi {
            let Some(seq) = self.sequence(idx, &data) else { return };
            let has_device = self.device().is_some();
            let globals = self.globals.clone();
            self.midi_backend();
            let sf = match &self.midi {
                MidiBackend::Os(t) => {
                    alog!("sound {}: MIDI on OS synth ({} events, {:.1} s{})", idx, seq.events.len(), seq.duration(), if looped { ", looped" } else { "" });
                    t.send(Cmd::Play { seq: seq.clone(), looped, ctl: ctl.clone(), state: state.clone() });
                    None
                }
                MidiBackend::SoundFont(sf) if has_device => Some(sf.clone()),
                _ => {
                    h.simulated = Some(logical(seq.duration() / globals.tempo()));
                    None
                }
            };
            if let Some(sf) = sf {
                match rustysynth::Synthesizer::new(&sf, &rustysynth::SynthesizerSettings::new(44100)) {
                    Ok(synth) => {
                        let gen = voice::SynthGen::new(midi::SeqPlayer::new(seq, looped), synth, globals.clone(), 44100);
                        let v = voice::Voice::new(Box::new(gen), ctl, globals, state);
                        if let Some(dev) = self.device() {
                            dev.mixer().add(v);
                        }
                    }
                    Err(e) => {
                        alog!("synth error {:?}", e);
                        h.simulated = Some(logical(seq.duration() / globals.tempo()));
                    }
                }
            }
        } else if self.device().is_some() {
            let gen: Option<Box<dyn voice::Gen>> = if kind == 3 {
                voice::StreamGen::new(data.clone(), looped).map(|g| Box::new(g) as Box<dyn voice::Gen>)
            } else {
                self.pcm(idx, &data).map(|p| Box::new(voice::PcmGen { pcm: p, pos: 0.0, looped }) as Box<dyn voice::Gen>)
            };
            match gen {
                Some(g) => {
                    let v = voice::Voice::new(g, ctl, self.globals.clone(), state);
                    if let Some(dev) = self.device() {
                        dev.mixer().add(v);
                    }
                }
                None => {
                    alog!("sound {}: cannot decode", idx);
                    return;
                }
            }
        } else {
            let secs = self.duration(idx, &data);
            h.simulated = Some(logical(secs));
        }
        self.voices.entry(idx).or_default().push(h);
        self.limit_voices();
    }

    fn limit_voices(&mut self) {
        let now = Instant::now();
        let mut pcm: Vec<(u64, bool, i32)> = Vec::new();
        for (k, l) in &self.voices {
            for v in l {
                if !v.midi && v.alive(now) {
                    pcm.push((v.serial, v.looped, *k));
                }
            }
        }
        if pcm.len() <= MAX_VOICES {
            return;
        }
        pcm.sort();
        let excess = pcm.len() - MAX_VOICES;
        let victims: Vec<u64> = pcm.iter().filter(|v| !v.1).take(excess).map(|v| v.0).collect();
        self.stop_where(|v| victims.contains(&v.serial));
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        for v in self.voices.values_mut() {
            v.retain(|p| p.alive(now));
        }
        self.voices.retain(|_, v| !v.is_empty());
    }

    pub fn stop(&mut self, idx: i32) {
        if let Some(v) = self.voices.remove(&idx) {
            for p in v {
                p.state.request_stop();
            }
        }
    }

    pub fn stop_all(&mut self) {
        let keys: Vec<i32> = self.voices.keys().copied().collect();
        for k in keys {
            self.stop(k);
        }
        if let MidiBackend::Os(t) = &self.midi {
            t.send(Cmd::Stop);
        }
    }

    pub fn is_playing(&mut self, idx: i32) -> bool {
        let now = Instant::now();
        self.voices.get(&idx).map(|v| v.iter().any(|p| p.alive(now))).unwrap_or(false)
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        self.stop_all();
        if let Some(s) = self.midi_stats() {
            alog!("MIDI summary: {}", s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(kind: i32, data: Vec<u8>) -> SoundRes {
        SoundRes {
            name: "s".into(),
            kind,
            file_type: ".wav".into(),
            file_name: String::new(),
            data: Some(Arc::from(data.as_slice())),
            effects: 0,
            volume: 1.0,
            pan: 0.0,
            preload: true,
        }
    }

    fn wav(secs: f64) -> Vec<u8> {
        let rate = 8000u32;
        let n = (secs * rate as f64) as u32;
        let mut v = b"RIFF".to_vec();
        v.extend((36 + n * 2).to_le_bytes());
        v.extend(b"WAVEfmt ");
        v.extend(16u32.to_le_bytes());
        v.extend(1u16.to_le_bytes());
        v.extend(1u16.to_le_bytes());
        v.extend(rate.to_le_bytes());
        v.extend((rate * 2).to_le_bytes());
        v.extend(2u16.to_le_bytes());
        v.extend(16u16.to_le_bytes());
        v.extend(b"data");
        v.extend((n * 2).to_le_bytes());
        for i in 0..n {
            v.extend((((i as f32 * 0.1).sin() * 8000.0) as i16).to_le_bytes());
        }
        v
    }

    #[test]
    fn logical_state_without_device() {
        let mut a = Audio::new(true);
        let s = res(0, wav(0.1));
        a.play(0, &s, false, &[]);
        a.play(0, &s, false, &[]);
        assert!(a.is_playing(0));
        assert_eq!(a.voices[&0].len(), 2, "normal sounds overlap");
        std::thread::sleep(Duration::from_millis(150));
        assert!(!a.is_playing(0));
        a.play(0, &s, true, &[]);
        std::thread::sleep(Duration::from_millis(150));
        assert!(a.is_playing(0), "looping stays playing");
        a.stop(0);
        assert!(!a.is_playing(0));
    }

    #[test]
    fn background_and_midi_exclusivity() {
        let mut a = Audio::new(true);
        let m1 = res(1, midi::tests::build_smf(0, 96, &[vec![(0, vec![0x90, 60, 100]), (960, vec![0x80, 60, 0])]]));
        let m2 = res(0, midi::tests::build_smf(0, 96, &[vec![(0, vec![0x90, 62, 100]), (960, vec![0x80, 62, 0])]]));
        let w = res(1, wav(1.0));
        a.play(1, &m1, true, &[]);
        assert!(a.is_playing(1));
        // a MIDI of another kind still replaces the MIDI player's song
        a.play(2, &m2, false, &[]);
        assert!(!a.is_playing(1) && a.is_playing(2));
        // background WAV stops nothing but other background music
        a.play(1, &m1, true, &[]);
        a.play(3, &w, true, &[]);
        assert!(!a.is_playing(1) && a.is_playing(3));
        a.stop_all();
        assert!(!a.is_playing(3));
    }

    #[test]
    fn voice_limit() {
        let mut a = Audio::new(true);
        let s = res(0, wav(5.0));
        for _ in 0..(MAX_VOICES + 5) {
            a.play(0, &s, false, &[]);
        }
        assert_eq!(a.voices[&0].len(), MAX_VOICES);
    }

    #[test]
    fn three_d_only_for_kind_2() {
        let mut a = Audio::new(true);
        let s0 = res(0, wav(0.1));
        let s2 = res(2, wav(0.1));
        a.set_3d(0, &s0, |s| s.pos = [5.0, 0.0, 0.0]);
        a.set_3d(1, &s2, |s| s.pos = [5.0, 0.0, 0.0]);
        assert_eq!(a.ctl(0, &s0).p.lock().unwrap().s3d.pos, [0.0; 3]);
        assert_eq!(a.ctl(1, &s2).p.lock().unwrap().s3d.pos, [5.0, 0.0, 0.0]);
    }

    #[test]
    fn segment_never_plays() {
        let mut a = Audio::new(true);
        let mut d = b"RIFF\0\0\0\0DMSG".to_vec();
        d.extend([0; 16]);
        a.play(0, &res(0, d), true, &[]);
        assert!(!a.is_playing(0));
    }
}
