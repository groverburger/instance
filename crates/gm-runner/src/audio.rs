//! Sound playback (runtime.md §15) through rodio. MIDI files are not
//! synthesised: they are tracked as "playing" while looped (see OPEN_QUESTIONS).

use crate::res::SoundRes;
use std::collections::HashMap;

struct Playing {
    player: Option<rodio::Player>,
    looped: bool,
    /// Simulated sounds (no device / MIDI): stays "playing" only while looped.
    simulated: bool,
}

/// The audio engine.
pub struct Audio {
    headless: bool,
    sink: Option<rodio::MixerDeviceSink>,
    tried: bool,
    playing: HashMap<i32, Vec<Playing>>,
    pub volumes: HashMap<i32, f64>,
    pub global_volume: f64,
}

impl Audio {
    pub fn new(headless: bool) -> Self {
        Audio {
            headless,
            sink: None,
            tried: false,
            playing: HashMap::new(),
            volumes: HashMap::new(),
            global_volume: 1.0,
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

    /// Plays sound `idx` once or looped. Background music (kind 1) stops other
    /// background music first.
    pub fn play(&mut self, idx: i32, res: &SoundRes, looped: bool, all: &[Option<SoundRes>]) {
        if res.kind == 1 {
            let others: Vec<i32> = self
                .playing
                .keys()
                .copied()
                .filter(|&k| k != idx && all.get(k as usize).and_then(|s| s.as_ref()).map(|s| s.kind == 1).unwrap_or(false))
                .collect();
            for o in others {
                self.stop(o);
            }
        }
        // multimedia / background: one instance at a time
        if res.kind != 0 {
            self.stop(idx);
        }
        self.cleanup();
        let ext = res.file_type.to_ascii_lowercase();
        let is_midi = ext.contains("mid") || ext.contains("rmi");
        let vol = (self.volumes.get(&idx).copied().unwrap_or(res.volume.clamp(0.0, 1.0)) * self.global_volume) as f32;
        let mut entry = Playing {
            player: None,
            looped,
            simulated: true,
        };
        if !is_midi {
            if let Some(data) = res.data.clone() {
                if let Some(dev) = self.device() {
                    let player = rodio::Player::connect_new(dev.mixer());
                    let cursor = std::io::Cursor::new(data);
                    let ok = if looped {
                        match rodio::Decoder::new_looped(cursor) {
                            Ok(d) => {
                                player.append(d);
                                true
                            }
                            Err(_) => false,
                        }
                    } else {
                        match rodio::Decoder::new(cursor) {
                            Ok(d) => {
                                player.append(d);
                                true
                            }
                            Err(_) => false,
                        }
                    };
                    if ok {
                        player.set_volume(vol);
                        entry.player = Some(player);
                        entry.simulated = false;
                    }
                }
            }
        }
        self.playing.entry(idx).or_default().push(entry);
    }

    fn cleanup(&mut self) {
        for v in self.playing.values_mut() {
            v.retain(|p| match &p.player {
                Some(pl) => !pl.empty(),
                None => p.looped,
            });
        }
        self.playing.retain(|_, v| !v.is_empty());
    }

    pub fn stop(&mut self, idx: i32) {
        if let Some(v) = self.playing.remove(&idx) {
            for p in v {
                if let Some(pl) = p.player {
                    pl.stop();
                }
            }
        }
    }

    pub fn stop_all(&mut self) {
        let keys: Vec<i32> = self.playing.keys().copied().collect();
        for k in keys {
            self.stop(k);
        }
    }

    pub fn is_playing(&mut self, idx: i32) -> bool {
        self.cleanup();
        self.playing
            .get(&idx)
            .map(|v| v.iter().any(|p| p.player.as_ref().map(|pl| !pl.empty()).unwrap_or(p.looped && p.simulated)))
            .unwrap_or(false)
    }

    pub fn set_volume(&mut self, idx: i32, v: f64) {
        self.volumes.insert(idx, v.clamp(0.0, 1.0));
        let g = self.global_volume;
        if let Some(list) = self.playing.get(&idx) {
            for p in list {
                if let Some(pl) = &p.player {
                    pl.set_volume((v.clamp(0.0, 1.0) * g) as f32);
                }
            }
        }
    }

    pub fn set_global_volume(&mut self, v: f64) {
        self.global_volume = v.clamp(0.0, 1.0);
        let vols = self.volumes.clone();
        for (idx, list) in &self.playing {
            let v = vols.get(idx).copied().unwrap_or(1.0);
            for p in list {
                if let Some(pl) = &p.player {
                    pl.set_volume((v * self.global_volume) as f32);
                }
            }
        }
    }
}
