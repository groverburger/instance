//! MIDI written by instance-music, played by the runner's own sequencer, and
//! seamless loop rendering (when a SoundFont is available).

use instance_music::analysis::analyse;
use instance_music::render::{find_soundfont, load_soundfont, render_song, RenderOpts};
use instance_music::{compile_text, smf};
use instance_runner::audio::midi::{Out, SeqPlayer, Sequence};
use std::sync::Arc;

const SONG: &str = r#"
title "Round trip"
tempo 90
key A minor
track pad inst="Pad 2 (warm)" oct=4 gate=100
track bass inst="Fingered Bass" oct=2
track drums drums
pattern walk = 1e 5 8 5
grid beat
  kick  x...x...x...x...
  hat   x.x.x.x.x.x.x.x.
section A
  pad:   *w over | Am | F | C | G |
  bass:  walk over | Am | F | C | G |
  drums: beat*4
section B tempo=120
  pad:   {Dm}w {E7}w
  bass:  D2h A2 E2 G#2
  drums: beat*2
arrange A B
"#;

#[test]
fn runner_sequencer_plays_our_midi() {
    let song = compile_text(SONG, None).unwrap();
    let bytes = smf::write_smf(&song);
    let seq = Arc::new(Sequence::parse(&bytes).expect("runner parses our SMF"));
    // (tempo is stored in whole microseconds per quarter note)
    assert!((seq.duration() - song.duration()).abs() < 1e-4, "{} vs {}", seq.duration(), song.duration());

    // every note-on comes out at the time the song says
    let sr = &song;
    let mut expected: Vec<(f64, u8, u8)> = song
        .tracks
        .iter()
        .flat_map(|t| t.notes.iter().map(move |n| (sr.secs_at(n.tick), t.channel, n.key)))
        .collect();
    expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut player = SeqPlayer::new(seq, false);
    let mut got: Vec<(f64, u8, u8)> = Vec::new();
    let dt = 0.0005;
    let mut t = 0.0;
    while !player.finished && t < 60.0 {
        player.advance(dt, &mut |o| {
            if let Out::Short(m, _) = o {
                if m[0] & 0xF0 == 0x90 && m[2] > 0 {
                    got.push((t, m[0] & 0x0F, m[1]));
                }
            }
        });
        t += dt;
    }
    assert_eq!(got.len(), expected.len());
    for (g, e) in got.iter().zip(&expected) {
        assert!((g.0 - e.0).abs() < 0.002, "note {:?} expected at {:?}", g, e);
    }
    let mut a: Vec<(u8, u8)> = got.iter().map(|g| (g.1, g.2)).collect();
    let mut b: Vec<(u8, u8)> = expected.iter().map(|e| (e.1, e.2)).collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
    // the tempo change to 120 BPM in section B is honoured: B starts at 4 bars × 4 beats × 60/90 s
    assert!(got.iter().any(|g| (g.0 - 16.0 * 60.0 / 90.0).abs() < 0.002));

    let st = smf::midi_stats(&bytes).unwrap();
    assert_eq!(st.notes, expected.len());
    assert_eq!(st.channels, vec![1, 2, 10]);
    assert!((st.duration - song.duration()).abs() < 1e-4);
}

#[test]
fn looped_render_is_the_steady_state_pass() {
    let Some(path) = find_soundfont(None) else {
        eprintln!("no SoundFont (set INSTANCE_SOUNDFONT): skipped");
        return;
    };
    let sf = load_soundfont(&path).unwrap();
    let one = "tempo 120\ntrack pad inst=\"Pad 2 (warm)\" oct=4 gate=100\ntrack drums drums\nsection A\n pad: {Am}w {F}w\n drums: [crash kick]q hat hat hat snare hat hat hat\narrange A";
    let three = one.replace("arrange A", "arrange A A A");
    // 4 s at 32 kHz is a whole number of 64-sample blocks, so no stretching
    let rate = 32000;
    let opts = RenderOpts { rate, mono: true, looped: true, tail: 0.0, normalize: None };
    let (lp, _) = render_song(&compile_text(one, None).unwrap(), &sf, &opts).unwrap();
    let (full, _) = render_song(&compile_text(&three, None).unwrap(), &sf, &RenderOpts { looped: false, tail: 0.5, ..opts.clone() }).unwrap();
    let n = lp.samples.len();
    assert_eq!(n, (4.0 * rate as f64) as usize);
    // the loop equals the second of three consecutive passes …
    let pass2 = &full.samples[n..2 * n];
    let err: f32 = lp.samples.iter().zip(pass2).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
    assert!(err < 1e-4, "loop differs from pass 2 by {err}");
    // … and its start continues its end the way pass 3 continues pass 2:
    // same level and spectrum (rustysynth starts notes on 64-sample block
    // boundaries, so onsets may differ by up to 64 samples — inaudible)
    let k = rate as usize / 5;
    let pass3 = &full.samples[2 * n..2 * n + k];
    let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
    let db = |a: f32, b: f32| 20.0 * (a / b).log10();
    assert!(db(rms(&lp.samples[..k]), rms(pass3)).abs() < 1.0);
    let seam = analyse(&lp, true).loop_check.unwrap();
    assert_eq!(seam.verdict, "smooth", "{seam:?}");
    // at 22.05 kHz the loop (88200 samples) is stretched to 88192 and stays seamless
    let (lp2, _) = render_song(&compile_text(one, None).unwrap(), &sf, &RenderOpts { rate: 22050, ..opts.clone() }).unwrap();
    assert_eq!(lp2.samples.len(), 88192);
    assert_eq!(analyse(&lp2, true).loop_check.unwrap().verdict, "smooth");
}
