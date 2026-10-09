//! `cargo run --release -p instance-runner --example record_midi_reference -- IN.mid OUT.wav [--tail SECS]`
//! — Windows only: plays a MIDI file in real time on the OS synthesiser (the
//! MIDI mapper, normally "Microsoft GS Wavetable Synth", as GameMaker 8.1 does)
//! with the runner's sequencer, while recording the default output device through
//! WASAPI loopback (cpal: an input stream on an output device). Writes a 16-bit
//! WAV at the device's mix rate (first two channels). A silent output stream keeps
//! the audio engine running so the loopback timeline has no gaps. The recording
//! includes every other sound the system plays meanwhile, and the device volume.

#[cfg(windows)]
fn main() {
    use instance_runner::audio::midi::{Out, SeqPlayer, Sequence};
    use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

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
    let [mid, out] = &files[..] else {
        eprintln!("usage: record_midi_reference IN.mid OUT.wav [--tail SECS]");
        std::process::exit(2);
    };
    let seq = Sequence::parse(&std::fs::read(mid).expect("read MIDI")).expect("parse MIDI");

    let host = rodio::cpal::default_host();
    let dev = host.default_output_device().expect("no default output device");
    let sup = dev.default_output_config().expect("output config");
    let cfg = sup.config();
    let (rate, ch) = (cfg.sample_rate, cfg.channels as usize);
    eprintln!("loopback: {:?} {} Hz, {} channels, {:?}", dev.description().map(|d| d.name().to_string()).ok(), rate, ch, sup.sample_format());
    if sup.sample_format() != rodio::cpal::SampleFormat::F32 {
        eprintln!("unsupported mix format {:?} (expected f32)", sup.sample_format());
        std::process::exit(1);
    }
    let rec: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let rec2 = rec.clone();
    let keep = dev
        .build_output_stream(&cfg, |d: &mut [f32], _: &_| d.fill(0.0), |e| eprintln!("output stream: {e}"), None)
        .expect("silent output stream");
    keep.play().expect("play silent stream");
    let cap = dev
        .build_input_stream(
            &cfg,
            move |d: &[f32], _: &_| {
                let mut r = rec2.lock().unwrap();
                for f in d.chunks(ch) {
                    r.push(f[0]);
                    r.push(*f.get(1).unwrap_or(&f[0]));
                }
            },
            |e| eprintln!("loopback stream: {e}"),
            None,
        )
        .expect("loopback stream (WASAPI)");
    cap.play().expect("start loopback");

    let (mut midi, name) = instance_runner::audio::midi_out::open_device().expect("MIDI output");
    eprintln!("MIDI out: {name}");
    // SAFETY: plain winmm timer-resolution call.
    unsafe {
        windows::Win32::Media::timeBeginPeriod(1);
    }
    std::thread::sleep(Duration::from_millis(300));
    let mut player = SeqPlayer::new(Arc::new(seq), false);
    let t0 = Instant::now();
    let mut last = t0;
    while !player.finished {
        std::thread::sleep(Duration::from_millis(1));
        let now = Instant::now();
        let dt = (now - last).as_secs_f64();
        last = now;
        player.advance(dt, &mut |o| match o {
            Out::Short(m, n) => midi.short(m, n),
            Out::SysEx(d) => midi.sysex(d),
            Out::LoopReset => {}
        });
    }
    for c in 0..16u8 {
        midi.short([0xB0 | c, 123, 0], 3); // all notes off (releases ring)
    }
    eprintln!("played {:.2} s", t0.elapsed().as_secs_f64());
    std::thread::sleep(Duration::from_secs_f64(tail));
    drop(cap);
    drop(midi);
    drop(keep);
    let pcm = rec.lock().unwrap().clone();
    write_wav(out, rate, &pcm).expect("write WAV");
    let peak = pcm.iter().fold(0f32, |m, x| m.max(x.abs()));
    println!("{out}: {:.2} s at {} Hz, peak {:.3}", pcm.len() as f64 / 2.0 / rate as f64, rate, peak);
}

#[cfg(windows)]
fn write_wav(path: &str, rate: u32, samples: &[f32]) -> std::io::Result<()> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
    std::fs::write(path, b)
}

#[cfg(not(windows))]
fn main() {
    eprintln!("record_midi_reference: Windows only (winmm MIDI out + WASAPI loopback)");
}
