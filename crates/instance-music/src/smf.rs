//! Standard MIDI File (type 1) writer, and note statistics of any MIDI file.

use crate::compile::Song;
use crate::song::PPQ;

fn vlq(mut v: u64, out: &mut Vec<u8>) {
    let mut stack = vec![(v & 0x7F) as u8];
    v >>= 7;
    while v > 0 {
        stack.push(((v & 0x7F) as u8) | 0x80);
        v >>= 7;
    }
    stack.reverse();
    out.extend(stack);
}

fn meta(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut v = vec![0xFF, kind];
    vlq(data.len() as u64, &mut v);
    v.extend_from_slice(data);
    v
}

fn chunk(events: &[(u64, Vec<u8>)], end: u64) -> Vec<u8> {
    let mut body = Vec::new();
    let mut last = 0u64;
    for (t, e) in events {
        vlq(t - last, &mut body);
        body.extend_from_slice(e);
        last = *t;
    }
    vlq(end.saturating_sub(last), &mut body);
    body.extend([0xFF, 0x2F, 0x00]);
    let mut c = b"MTrk".to_vec();
    c.extend((body.len() as u32).to_be_bytes());
    c.extend(body);
    c
}

/// Writes the song as a type-1 SMF: a conductor track (title, tempo, time
/// and key signatures, section markers, `loopStart`/`loopEnd` markers) and
/// one track per song track. Every track ends exactly at the song end so
/// players that loop the file (GameMaker) keep the bar grid.
pub fn write_smf(song: &Song) -> Vec<u8> {
    let mut f = b"MThd".to_vec();
    f.extend(6u32.to_be_bytes());
    f.extend(1u16.to_be_bytes());
    f.extend(((song.tracks.len() + 1) as u16).to_be_bytes());
    f.extend((PPQ as u16).to_be_bytes());

    let mut cond: Vec<(u64, Vec<u8>)> = Vec::new();
    if !song.title.is_empty() {
        cond.push((0, meta(0x03, song.title.as_bytes())));
    }
    for &(t, (n, d)) in &song.times {
        let dd = (d as f64).log2() as u8;
        cond.push((t, meta(0x58, &[n as u8, dd, 24, 8])));
    }
    if let Some(k) = &song.key {
        // key signature: sharps/flats count from the tonic (major/minor only)
        let minor = matches!(k.mode.as_str(), "minor" | "aeolian" | "harmonic_minor" | "melodic_minor" | "dorian" | "phrygian");
        let major_pc = if minor { (k.tonic + 3) % 12 } else { k.tonic };
        const SF: [i8; 12] = [0, -5, 2, -3, 4, -1, 6, 1, -4, 3, -2, 5];
        cond.push((0, meta(0x59, &[SF[major_pc as usize] as u8, minor as u8])));
    }
    for &(t, bpm) in &song.tempos {
        let us = (60_000_000.0 / bpm).round() as u32;
        cond.push((t, meta(0x51, &[(us >> 16) as u8, (us >> 8) as u8, us as u8])));
    }
    for (t, name) in &song.markers {
        cond.push((*t, meta(0x06, name.as_bytes())));
    }
    if song.loop_range != (0, song.end) {
        cond.push((song.loop_range.0, meta(0x06, b"loopStart")));
        cond.push((song.loop_range.1, meta(0x06, b"loopEnd")));
    }
    cond.sort_by_key(|e| e.0);
    f.extend(chunk(&cond, song.end));

    for tr in &song.tracks {
        let mut ev = vec![(0u64, meta(0x03, tr.def.name.as_bytes()))];
        ev.extend(song.track_events(tr));
        let end = ev.last().map(|e| e.0).unwrap_or(0).max(song.end);
        f.extend(chunk(&ev, end));
    }
    f
}

/// Note statistics of a MIDI file.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct MidiStats {
    pub tracks: usize,
    pub notes: usize,
    pub channels: Vec<u8>,
    /// (channel 1-based, GM program 1-based)
    pub programs: Vec<(u8, u8)>,
    /// Lowest/highest note outside channel 10 (drums).
    pub lowest: Option<u8>,
    pub highest: Option<u8>,
    pub duration: f64,
    pub max_polyphony: usize,
}

/// Parses any SMF and counts notes, channels and programs.
pub fn midi_stats(data: &[u8]) -> Result<MidiStats, String> {
    use midly::{MidiMessage, Smf, Timing, TrackEventKind};
    let smf = Smf::parse(data).map_err(|e| e.to_string())?;
    let ppq = match smf.header.timing {
        Timing::Metrical(t) => t.as_int().max(1) as f64,
        Timing::Timecode(..) => 480.0,
    };
    let mut st = MidiStats { tracks: smf.tracks.len(), ..MidiStats::default() };
    let mut evs: Vec<(u64, i32)> = Vec::new(); // (tick, +1 on / -1 off)
    let mut tempos: Vec<(u64, u32)> = Vec::new();
    let mut end = 0u64;
    for tr in &smf.tracks {
        let mut t = 0u64;
        for e in tr {
            t += e.delta.as_int() as u64;
            match e.kind {
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int() + 1;
                    if !st.channels.contains(&ch) {
                        st.channels.push(ch);
                    }
                    match message {
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                            st.notes += 1;
                            let k = key.as_int();
                            if ch != 10 {
                                st.lowest = Some(st.lowest.map_or(k, |l| l.min(k)));
                                st.highest = Some(st.highest.map_or(k, |h| h.max(k)));
                            }
                            evs.push((t, 1));
                        }
                        MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. } => evs.push((t, -1)),
                        MidiMessage::ProgramChange { program } => {
                            let p = (ch, program.as_int() + 1);
                            if !st.programs.contains(&p) {
                                st.programs.push(p);
                            }
                        }
                        _ => {}
                    }
                }
                TrackEventKind::Meta(midly::MetaMessage::Tempo(us)) => tempos.push((t, us.as_int())),
                _ => {}
            }
        }
        end = end.max(t);
    }
    st.channels.sort();
    evs.sort();
    let mut cur = 0i32;
    for (_, d) in &evs {
        cur = (cur + d).max(0);
        st.max_polyphony = st.max_polyphony.max(cur as usize);
    }
    tempos.sort();
    let mut secs = 0.0;
    let mut last = 0u64;
    let mut us = 500_000.0;
    for (t, u) in tempos {
        secs += (t - last) as f64 * us / 1e6 / ppq;
        last = t;
        us = u as f64;
    }
    st.duration = secs + (end.saturating_sub(last)) as f64 * us / 1e6 / ppq;
    Ok(st)
}
