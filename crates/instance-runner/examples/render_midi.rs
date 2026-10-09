//! `cargo run --release -p instance-runner --example render_midi -- IN.mid SOUNDFONT.sf2 OUT.wav [--tail SECS]`
//! — renders a MIDI file to a 44.1 kHz 16-bit stereo WAV through the same
//! SoundFont path the runner uses for MIDI (rustysynth with default settings,
//! the runner's sequencer), e.g. to compare candidate SoundFonts offline.
//! The release tail after the last event is `--tail` seconds (default 2), cut
//! short once the output falls silent.

use instance_runner::audio::midi::{Out, SeqPlayer, Sequence};
use std::sync::Arc;

const RATE: u32 = 44100;
const BLOCK: usize = 64;

/// Writes interleaved stereo f32 samples as a 16-bit PCM WAV file.
pub fn write_wav(path: &str, rate: u32, channels: u16, samples: &[f32]) -> std::io::Result<()> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    b.extend_from_slice(&(channels * 2).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
    std::fs::write(path, b)
}

fn main() {
    let mut tail = 2.0f64;
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--tail" {
            tail = args.next().and_then(|v| v.parse().ok()).expect("--tail SECS");
        } else {
            files.push(a);
        }
    }
    let [mid, sf2, out] = &files[..] else {
        eprintln!("usage: render_midi IN.mid SOUNDFONT.sf2 OUT.wav [--tail SECS]");
        std::process::exit(2);
    };
    let seq = Sequence::parse(&std::fs::read(mid).expect("read MIDI")).expect("parse MIDI");
    let sf = rustysynth::SoundFont::new(&mut std::io::BufReader::new(std::fs::File::open(sf2).expect("open SoundFont")))
        .unwrap_or_else(|e| panic!("load SoundFont: {e:?}"));
    let mut synth = rustysynth::Synthesizer::new(&Arc::new(sf), &rustysynth::SynthesizerSettings::new(RATE as i32))
        .unwrap_or_else(|e| panic!("synth: {e:?}"));
    let mut player = SeqPlayer::new(Arc::new(seq), false);
    let (mut l, mut r) = (vec![0f32; BLOCK], vec![0f32; BLOCK]);
    let mut pcm: Vec<f32> = Vec::new();
    let mut tail_left = (tail * RATE as f64) as usize;
    loop {
        if !player.finished {
            player.advance(BLOCK as f64 / RATE as f64, &mut |o| match o {
                Out::Short(m, _) => synth.process_midi_message((m[0] & 0x0F) as i32, (m[0] & 0xF0) as i32, m[1] as i32, m[2] as i32),
                Out::SysEx(_) => {}
                Out::LoopReset => {}
            });
            if player.finished {
                synth.note_off_all(false);
            }
        }
        synth.render(&mut l, &mut r);
        for i in 0..BLOCK {
            pcm.push(l[i]);
            pcm.push(r[i]);
        }
        if player.finished {
            let peak = l.iter().chain(r.iter()).fold(0f32, |m, x| m.max(x.abs()));
            if tail_left <= BLOCK || peak < 3.2e-5 {
                break;
            }
            tail_left -= BLOCK;
        }
    }
    let peak = pcm.iter().fold(0f32, |m, x| m.max(x.abs()));
    write_wav(out, RATE, 2, &pcm).expect("write WAV");
    println!("{out}: {:.2} s, peak {:.3}", pcm.len() as f64 / 2.0 / RATE as f64, peak);
}
