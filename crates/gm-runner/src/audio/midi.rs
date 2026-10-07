//! Standard MIDI File sequencer (specs/sound.md §4): parsing with `midly`,
//! a merged tempo-aware event list and a time-driven player with looping.
//! The player is pure (no I/O): it is advanced by elapsed seconds and emits
//! messages to a callback, which makes it testable and lets it drive either
//! the OS synthesiser thread or the SoundFont renderer.

use midly::{MetaMessage, Smf, Timing, TrackEventKind};

/// A sequenced event.
#[derive(Clone, Debug, PartialEq)]
pub enum MidiEv {
    /// A channel message: status + up to two data bytes, `len` = 2 or 3.
    Short([u8; 3], u8),
    /// A complete SysEx message including the leading 0xF0.
    SysEx(Vec<u8>),
    /// Tempo change, microseconds per quarter note.
    Tempo(u32),
}

/// Something the player wants sent.
#[derive(Clone, Debug, PartialEq)]
pub enum Out<'a> {
    Short([u8; 3], u8),
    SysEx(&'a [u8]),
    /// The loop point was reached: silence and reset all channels.
    LoopReset,
}

/// A parsed MIDI sequence.
#[derive(Clone, Debug)]
pub struct Sequence {
    /// Events sorted by tick (stable: track order kept for equal ticks).
    pub events: Vec<(u64, MidiEv)>,
    /// Ticks per quarter note (metrical) — or 0 when `tick_secs` is fixed.
    pub ppq: u32,
    /// Seconds per tick for SMPTE time-code files.
    pub tick_secs: Option<f64>,
    /// Tick of the end of the longest track.
    pub end_tick: u64,
}

/// Returns the SMF bytes inside RIFF RMID data, or the data itself.
fn unwrap_rmid(data: &[u8]) -> &[u8] {
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"RMID" {
        let mut p = 12;
        while p + 8 <= data.len() {
            let id = &data[p..p + 4];
            let len = u32::from_le_bytes([data[p + 4], data[p + 5], data[p + 6], data[p + 7]]) as usize;
            let body = p + 8;
            if id == b"data" {
                return &data[body..(body + len).min(data.len())];
            }
            p = body + len + (len & 1);
        }
    }
    data
}

/// Whether the bytes look like a MIDI file (SMF or RIFF RMID).
pub fn is_midi(data: &[u8]) -> bool {
    let d = unwrap_rmid(data);
    d.len() >= 4 && &d[0..4] == b"MThd"
}

impl Sequence {
    pub fn parse(data: &[u8]) -> Result<Sequence, String> {
        let smf = Smf::parse(unwrap_rmid(data)).map_err(|e| e.to_string())?;
        let (ppq, tick_secs) = match smf.header.timing {
            Timing::Metrical(t) => (t.as_int().max(1) as u32, None),
            Timing::Timecode(fps, sub) => (0, Some(1.0 / (fps.as_f32() as f64 * sub.max(1) as f64))),
        };
        let mut events: Vec<(u64, usize, usize, MidiEv)> = Vec::new();
        let mut end_tick = 0;
        let format_seq = matches!(smf.header.format, midly::Format::Sequential);
        let mut track_base = 0u64;
        for (ti, track) in smf.tracks.iter().enumerate() {
            let mut t = if format_seq { track_base } else { 0 };
            for (ei, e) in track.iter().enumerate() {
                t += e.delta.as_int() as u64;
                let ev = match e.kind {
                    TrackEventKind::Midi { .. } => {
                        let Some(live) = e.kind.as_live_event() else { continue };
                        let mut buf = Vec::with_capacity(3);
                        if live.write_std(&mut buf).is_err() || buf.is_empty() {
                            continue;
                        }
                        let mut m = [0u8; 3];
                        let n = buf.len().min(3);
                        m[..n].copy_from_slice(&buf[..n]);
                        MidiEv::Short(m, n as u8)
                    }
                    TrackEventKind::SysEx(d) => {
                        let mut v = Vec::with_capacity(d.len() + 1);
                        v.push(0xF0);
                        v.extend_from_slice(d);
                        if v.last() != Some(&0xF7) {
                            v.push(0xF7);
                        }
                        MidiEv::SysEx(v)
                    }
                    TrackEventKind::Meta(MetaMessage::Tempo(us)) => MidiEv::Tempo(us.as_int().max(1)),
                    _ => continue,
                };
                events.push((t, ti, ei, ev));
            }
            end_tick = end_tick.max(t);
            track_base = t;
        }
        events.sort_by_key(|a| (a.0, a.1, a.2));
        Ok(Sequence { events: events.into_iter().map(|(t, _, _, e)| (t, e)).collect(), ppq, tick_secs, end_tick })
    }

    fn secs_per_tick(&self, us_per_qn: f64) -> f64 {
        match self.tick_secs {
            Some(s) => s,
            None => us_per_qn / 1_000_000.0 / self.ppq as f64,
        }
    }

    /// Length in seconds at normal tempo.
    pub fn duration(&self) -> f64 {
        let mut us = 500_000.0;
        let mut last = 0u64;
        let mut secs = 0.0;
        for (t, e) in &self.events {
            secs += (*t - last) as f64 * self.secs_per_tick(us);
            last = *t;
            if let MidiEv::Tempo(u) = e {
                us = *u as f64;
            }
        }
        secs + (self.end_tick.saturating_sub(last)) as f64 * self.secs_per_tick(us)
    }
}

/// Plays a [`Sequence`] against a clock.
#[derive(Clone, Debug)]
pub struct SeqPlayer {
    pub seq: std::sync::Arc<Sequence>,
    idx: usize,
    tick: f64,
    us_per_qn: f64,
    pub looped: bool,
    pub finished: bool,
    /// Number of completed loops.
    pub loops: u64,
}

impl SeqPlayer {
    pub fn new(seq: std::sync::Arc<Sequence>, looped: bool) -> Self {
        SeqPlayer { seq, idx: 0, tick: 0.0, us_per_qn: 500_000.0, looped, finished: false, loops: 0 }
    }

    /// Current position in ticks.
    pub fn tick(&self) -> f64 {
        self.tick
    }

    /// Advances by `dt` seconds of sequence time (real time × tempo factor),
    /// emitting every event that falls inside the interval.
    pub fn advance(&mut self, mut dt: f64, out: &mut dyn FnMut(Out)) {
        if self.finished || dt.is_nan() || dt <= 0.0 {
            return;
        }
        // guard against degenerate empty sequences looping forever
        if self.seq.end_tick == 0 && self.seq.events.iter().all(|(t, _)| *t == 0) {
            while self.idx < self.seq.events.len() {
                let i = self.idx;
                self.idx += 1;
                self.emit_from(i, out);
            }
            if !self.looped {
                self.finished = true;
            }
            return;
        }
        let mut guard = 0u32;
        loop {
            guard += 1;
            if guard > 1_000_000 {
                return;
            }
            let spt = self.seq.secs_per_tick(self.us_per_qn);
            let next_tick = match self.seq.events.get(self.idx) {
                Some((t, _)) => *t as f64,
                None => self.seq.end_tick as f64,
            };
            let need = (next_tick - self.tick).max(0.0) * spt;
            if need > dt {
                self.tick += dt / spt;
                return;
            }
            dt -= need;
            self.tick = next_tick;
            if self.idx < self.seq.events.len() {
                let i = self.idx;
                self.idx += 1;
                self.emit_from(i, out);
            } else {
                // end of sequence
                if self.looped {
                    out(Out::LoopReset);
                    self.idx = 0;
                    self.tick = 0.0;
                    self.us_per_qn = 500_000.0;
                    self.loops += 1;
                } else {
                    self.finished = true;
                    return;
                }
            }
        }
    }

    fn emit_from(&mut self, i: usize, out: &mut dyn FnMut(Out)) {
        let seq = self.seq.clone();
        let (_, e) = &seq.events[i];
        match e {
            MidiEv::Short(m, n) => out(Out::Short(*m, *n)),
            MidiEv::SysEx(v) => out(Out::SysEx(v)),
            MidiEv::Tempo(us) => self.us_per_qn = *us as f64,
        }
    }
}

/// Channel-volume bookkeeping for the OS-synth path (§4.3): the file's own
/// CC7/CC10 values are remembered and re-sent scaled by the sound's gain/pan.
#[derive(Clone, Debug)]
pub struct ChannelMixer {
    pub file_vol: [u8; 16],
    pub file_pan: [u8; 16],
    pub gain: f64,
    pub pan: f64,
}

impl Default for ChannelMixer {
    fn default() -> Self {
        ChannelMixer { file_vol: [100; 16], file_pan: [64; 16], gain: 1.0, pan: 0.0 }
    }
}

impl ChannelMixer {
    pub fn vol_for(&self, ch: usize) -> u8 {
        (self.file_vol[ch] as f64 * self.gain.clamp(0.0, 1.0)).round().clamp(0.0, 127.0) as u8
    }
    pub fn pan_for(&self, ch: usize) -> u8 {
        (self.file_pan[ch] as f64 + self.pan.clamp(-1.0, 1.0) * 64.0).round().clamp(0.0, 127.0) as u8
    }
    /// Rewrites a message on its way out; CC7/CC10 are scaled.
    pub fn filter(&mut self, m: [u8; 3], n: u8) -> ([u8; 3], u8) {
        if n == 3 && m[0] & 0xF0 == 0xB0 {
            let ch = (m[0] & 0x0F) as usize;
            match m[1] {
                7 => {
                    self.file_vol[ch] = m[2].min(127);
                    return ([m[0], 7, self.vol_for(ch)], 3);
                }
                10 => {
                    self.file_pan[ch] = m[2].min(127);
                    return ([m[0], 10, self.pan_for(ch)], 3);
                }
                121 => {
                    // reset all controllers: the file's levels revert to defaults
                    self.file_pan[ch] = 64;
                }
                _ => {}
            }
        }
        (m, n)
    }
    /// The CC7 and CC10 messages for all 16 channels.
    pub fn all_levels(&self) -> Vec<[u8; 3]> {
        let mut v = Vec::with_capacity(32);
        for ch in 0..16 {
            v.push([0xB0 | ch as u8, 7, self.vol_for(ch)]);
            v.push([0xB0 | ch as u8, 10, self.pan_for(ch)]);
        }
        v
    }
}

/// Messages that silence and reset every channel (loop point / stop).
pub fn reset_messages() -> Vec<[u8; 3]> {
    let mut v = Vec::new();
    for ch in 0..16u8 {
        v.push([0xB0 | ch, 64, 0]); // sustain off
        v.push([0xB0 | ch, 123, 0]); // all notes off
        v.push([0xB0 | ch, 120, 0]); // all sound off
        v.push([0xB0 | ch, 121, 0]); // reset controllers
    }
    v
}

/// The GS reset SysEx sent when the OS synthesiser is opened.
pub const GS_RESET: [u8; 11] = [0xF0, 0x41, 0x10, 0x42, 0x12, 0x40, 0x00, 0x7F, 0x00, 0x41, 0xF7];

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::Arc;

    fn vlq(mut v: u32, out: &mut Vec<u8>) {
        let mut stack = vec![(v & 0x7F) as u8];
        v >>= 7;
        while v > 0 {
            stack.push(((v & 0x7F) as u8) | 0x80);
            v >>= 7;
        }
        stack.reverse();
        out.extend(stack);
    }

    /// Builds an SMF from tracks of (delta, raw event bytes).
    pub fn build_smf(format: u16, ppq: u16, tracks: &[Vec<(u32, Vec<u8>)>]) -> Vec<u8> {
        let mut f = b"MThd".to_vec();
        f.extend(6u32.to_be_bytes());
        f.extend(format.to_be_bytes());
        f.extend((tracks.len() as u16).to_be_bytes());
        f.extend(ppq.to_be_bytes());
        for t in tracks {
            let mut body = Vec::new();
            for (d, e) in t {
                vlq(*d, &mut body);
                body.extend(e);
            }
            body.extend([0x00, 0xFF, 0x2F, 0x00]);
            f.extend(b"MTrk");
            f.extend((body.len() as u32).to_be_bytes());
            f.extend(body);
        }
        f
    }

    fn tempo(us: u32) -> Vec<u8> {
        vec![0xFF, 0x51, 0x03, (us >> 16) as u8, (us >> 8) as u8, us as u8]
    }

    /// Times (seconds) at which note-ons are emitted, sampled with step `dt`.
    fn note_times(p: &mut SeqPlayer, total: f64, dt: f64) -> Vec<(f64, u8)> {
        let mut t: f64 = 0.0;
        let mut out = Vec::new();
        while t < total {
            t += dt;
            p.advance(dt, &mut |o| {
                if let Out::Short(m, _) = o {
                    if m[0] & 0xF0 == 0x90 && m[2] > 0 {
                        out.push((t, m[1]));
                    }
                }
            });
        }
        out
    }

    #[test]
    fn tempo_changes() {
        // ppq 100; default tempo 0.5 s/qn → note at tick 100 = 0.5 s;
        // tempo → 0.25 s/qn at tick 100 → note at tick 200 = 0.75 s;
        // tempo → 1 s/qn at tick 200 → note at tick 300 = 1.75 s
        let tr = vec![
            (0, vec![0x90, 60, 100]),
            (100, tempo(250_000)),
            (0, vec![0x90, 61, 100]),
            (100, tempo(1_000_000)),
            (0, vec![0x90, 62, 100]),
            (100, vec![0x90, 63, 100]),
        ];
        let seq = Arc::new(Sequence::parse(&build_smf(0, 100, &[tr])).unwrap());
        assert!((seq.duration() - 1.75).abs() < 1e-9);
        let mut p = SeqPlayer::new(seq, false);
        let n = note_times(&mut p, 3.0, 0.001);
        let expect = [(0.0, 60), (0.5, 61), (0.75, 62), (1.75, 63)];
        assert_eq!(n.len(), 4);
        for ((t, k), (et, ek)) in n.iter().zip(expect) {
            assert_eq!(*k, ek);
            assert!((t - et).abs() < 0.0015, "note {} at {} expected {}", k, t, et);
        }
        assert!(p.finished);
    }

    #[test]
    fn format1_tracks_are_merged() {
        let t0 = vec![(0, tempo(1_000_000))];
        let t1 = vec![(48, vec![0x91, 40, 90]), (48, vec![0x81, 40, 0])];
        let t2 = vec![(24, vec![0x92, 50, 90]), (0, vec![0xC2, 5])];
        let seq = Sequence::parse(&build_smf(1, 48, &[t0, t1, t2])).unwrap();
        let ticks: Vec<u64> = seq.events.iter().map(|e| e.0).collect();
        assert_eq!(ticks, vec![0, 24, 24, 48, 96]);
        assert_eq!(seq.events[2].1, MidiEv::Short([0xC2, 5, 0], 2));
        assert!((seq.duration() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn running_status_parsed() {
        let tr = vec![(0, vec![0x90, 60, 100]), (10, vec![62, 100])];
        let seq = Sequence::parse(&build_smf(0, 10, &[tr])).unwrap();
        assert_eq!(seq.events[1].1, MidiEv::Short([0x90, 62, 100], 3));
    }

    #[test]
    fn looping_restarts_with_reset_and_tempo_reset() {
        // 1 qn long: note at 0, tempo change to double speed at tick 50 (end 100)
        let tr = vec![(0, vec![0x90, 60, 100]), (50, tempo(250_000)), (50, vec![0x80, 60, 0])];
        let seq = Arc::new(Sequence::parse(&build_smf(0, 100, &[tr])).unwrap());
        // 0.25 s + 0.125 s = 0.375 s per pass
        assert!((seq.duration() - 0.375).abs() < 1e-9);
        let mut p = SeqPlayer::new(seq, true);
        let mut resets: Vec<f64> = Vec::new();
        let mut notes = Vec::new();
        let mut t: f64 = 0.0;
        for _ in 0..1000 {
            t += 0.001;
            p.advance(0.001, &mut |o| match o {
                Out::LoopReset => resets.push(t),
                Out::Short(m, _) if m[0] == 0x90 => notes.push(t),
                _ => {}
            });
        }
        assert!(!p.finished);
        assert_eq!(resets.len(), 2, "{:?}", resets);
        assert!((resets[0] - 0.375).abs() < 0.0015);
        assert!((resets[1] - 0.75).abs() < 0.0015, "tempo must reset at loop: {:?}", resets);
        assert_eq!(notes.len(), 3);
        assert_eq!(p.loops, 2);
    }

    #[test]
    fn large_steps_cross_loop_points() {
        let tr = vec![(0, vec![0x90, 60, 100]), (100, vec![0x80, 60, 0])];
        let seq = Arc::new(Sequence::parse(&build_smf(0, 100, &[tr])).unwrap());
        let mut p = SeqPlayer::new(seq, true);
        let mut ons = 0;
        p.advance(2.25, &mut |o| {
            if let Out::Short(m, _) = o {
                if m[0] == 0x90 {
                    ons += 1;
                }
            }
        });
        assert_eq!(ons, 5); // starts at 0, 0.5, 1.0, 1.5, 2.0
    }

    #[test]
    fn rmid_and_sysex() {
        let tr = vec![(0, vec![0xF0, 0x03, 0x7E, 0x7F, 0xF7])];
        let smf = build_smf(0, 96, &[tr]);
        let mut riff = b"RIFF".to_vec();
        riff.extend(((smf.len() + 12) as u32).to_le_bytes());
        riff.extend(b"RMIDdata");
        riff.extend((smf.len() as u32).to_le_bytes());
        riff.extend(&smf);
        assert!(is_midi(&riff));
        let seq = Sequence::parse(&riff).unwrap();
        assert_eq!(seq.events[0].1, MidiEv::SysEx(vec![0xF0, 0x7E, 0x7F, 0xF7]));
    }

    #[test]
    fn channel_mixer_scales_cc7() {
        let mut m = ChannelMixer::default();
        m.gain = 0.5;
        assert_eq!(m.filter([0xB3, 7, 120], 3), ([0xB3, 7, 60], 3));
        assert_eq!(m.vol_for(3), 60);
        assert_eq!(m.vol_for(0), 50); // file default 100
        m.pan = -1.0;
        assert_eq!(m.pan_for(0), 0);
        assert_eq!(m.filter([0x90, 7, 120], 3), ([0x90, 7, 120], 3));
    }
}
