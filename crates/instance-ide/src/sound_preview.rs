//! Playing a sound from its form (the Play / Stop buttons), with the game's own sound
//! engine (`instance-audio`), so the preview has the kind, volume, pan and effects
//! set in the form, saved or not. One sound plays at a time.

use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;

use instance_audio::{Audio, SoundRes};
use instance_format::Sound;

/// The engine slot all previews play in.
const SLOT: i32 = 0;

#[derive(Default)]
struct Preview {
    audio: Option<Audio>,
    /// The sound slot whose form started the current preview.
    owner: Option<usize>,
    /// The data last played: kept so replaying unchanged data reuses the decoded
    /// audio (the engine's cache is keyed by this allocation).
    data: Option<Arc<[u8]>>,
    /// A MIDI preview waiting for the SoundFont (web).
    #[cfg(target_arch = "wasm32")]
    queued: Option<(usize, SoundRes)>,
}

thread_local! {
    static PREVIEW: RefCell<Preview> = RefCell::new(Preview::default());
}

#[cfg(target_arch = "wasm32")]
fn is_midi(d: &[u8]) -> bool {
    d.starts_with(b"MThd") || (d.starts_with(b"RIFF") && d.get(8..12) == Some(b"RMID"))
}

impl Preview {
    fn res(&mut self, s: &Sound) -> Option<SoundRes> {
        let bytes = s.data.as_deref().filter(|d| !d.is_empty())?;
        let data = match &self.data {
            Some(d) if **d == *bytes => d.clone(),
            _ => Arc::from(bytes),
        };
        self.data = Some(data.clone());
        Some(SoundRes {
            name: s.name.clone(),
            kind: s.kind,
            file_type: s.file_type.clone(),
            file_name: s.file_name.clone(),
            data: Some(data),
            effects: s.effects,
            volume: s.volume,
            pan: s.pan,
            preload: false,
        })
    }

    fn start(&mut self, owner: usize, res: &SoundRes) {
        let audio = self.audio.get_or_insert_with(|| {
            let mut a = Audio::new(false);
            a.set_status_delay(Duration::ZERO);
            a
        });
        audio.stop(SLOT);
        audio.set_volume(SLOT, res, res.volume);
        audio.set_pan(SLOT, res, res.pan);
        audio.effect_set(SLOT, res, res.effects);
        audio.play(SLOT, res, false, &[]);
        self.owner = Some(owner);
    }
}

/// Plays the sound of the form for slot `owner`, stopping any other preview.
pub fn play(owner: usize, s: &Sound) {
    PREVIEW.with(|p| {
        let mut p = p.borrow_mut();
        let Some(res) = p.res(s) else { return };
        #[cfg(target_arch = "wasm32")]
        if is_midi(res.data.as_deref().unwrap_or_default()) && !crate::web::soundfont_ready() {
            // the engine picks its MIDI synthesiser once: load the SoundFont first
            if let Some(a) = &mut p.audio {
                a.stop(SLOT);
            }
            p.queued = Some((owner, res));
            p.owner = Some(owner);
            crate::web::load_soundfont();
            return;
        }
        p.start(owner, &res);
    });
}

/// Stops the preview if the form for slot `owner` started it.
pub fn stop(owner: usize) {
    PREVIEW.with(|p| {
        let mut p = p.borrow_mut();
        if p.owner == Some(owner) {
            if let Some(a) = &mut p.audio {
                a.stop(SLOT);
            }
            p.owner = None;
            #[cfg(target_arch = "wasm32")]
            {
                p.queued = None;
            }
        }
    });
}

/// Whether the form for slot `owner` has a preview playing (or about to play).
pub fn is_playing(owner: usize) -> bool {
    PREVIEW.with(|p| {
        let mut p = p.borrow_mut();
        if p.owner != Some(owner) {
            return false;
        }
        #[cfg(target_arch = "wasm32")]
        if p.queued.is_some() {
            if !crate::web::soundfont_ready() {
                return true;
            }
            let (o, res) = p.queued.take().unwrap();
            p.start(o, &res);
        }
        let playing = p.audio.as_mut().is_some_and(|a| a.is_playing(SLOT));
        if !playing {
            p.owner = None;
        }
        playing
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mono 16-bit WAV of `secs` seconds of silence.
    fn wav(secs: f64) -> Vec<u8> {
        let rate = 22050u32;
        let n = (rate as f64 * secs) as u32;
        let mut w = Vec::new();
        w.extend(b"RIFF");
        w.extend((36 + n * 2).to_le_bytes());
        w.extend(b"WAVEfmt ");
        w.extend(16u32.to_le_bytes());
        w.extend(1u16.to_le_bytes());
        w.extend(1u16.to_le_bytes());
        w.extend(rate.to_le_bytes());
        w.extend((rate * 2).to_le_bytes());
        w.extend(2u16.to_le_bytes());
        w.extend(16u16.to_le_bytes());
        w.extend(b"data");
        w.extend((n * 2).to_le_bytes());
        w.resize(w.len() + n as usize * 2, 0);
        w
    }

    #[test]
    fn play_stop_and_owner() {
        let s = Sound {
            name: "snd".into(),
            file_type: ".wav".into(),
            data: Some(wav(5.0)),
            volume: 1.0,
            ..Default::default()
        };
        play(3, &s);
        assert!(is_playing(3), "playing after Play");
        assert!(!is_playing(4), "another form's sound is not playing");
        stop(4);
        assert!(is_playing(3), "another form's Stop leaves it playing");
        // replaying unchanged data reuses the decoded audio
        let before = PREVIEW.with(|p| p.borrow().data.clone()).unwrap();
        play(3, &s);
        let after = PREVIEW.with(|p| p.borrow().data.clone()).unwrap();
        assert!(Arc::ptr_eq(&before, &after));
        stop(3);
        assert!(!is_playing(3), "stopped");
        // a sound without data does nothing
        play(5, &Sound::default());
        assert!(!is_playing(5));
    }
}
