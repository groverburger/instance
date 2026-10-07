//! MIDI playback through the OS synthesiser on a dedicated thread
//! (specs/sound.md §4.2 item 2, §4.3). On Windows this is the multimedia MIDI
//! output (`midiOut*`, normally "Microsoft GS Wavetable Synth"); elsewhere the
//! thread cannot open a device and the engine falls back to the SoundFont path.

use super::midi::{reset_messages, ChannelMixer, Out, SeqPlayer, Sequence, GS_RESET};
use super::voice::{mix_of, Globals, SoundCtl, VoiceState};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Counters for diagnostics (`GM81_AUDIO_DEBUG`) and tests.
#[derive(Default)]
pub struct MidiStats {
    pub device: Mutex<String>,
    pub short_msgs: AtomicU64,
    pub note_ons: AtomicU64,
    pub sysex_msgs: AtomicU64,
    pub level_updates: AtomicU64,
    pub loops: AtomicU64,
    pub songs: AtomicU64,
}

impl MidiStats {
    pub fn summary(&self) -> String {
        format!(
            "device '{}': {} songs, {} short messages ({} note-ons), {} sysex, {} volume updates, {} loops",
            self.device.lock().unwrap(),
            self.songs.load(Ordering::Relaxed),
            self.short_msgs.load(Ordering::Relaxed),
            self.note_ons.load(Ordering::Relaxed),
            self.sysex_msgs.load(Ordering::Relaxed),
            self.level_updates.load(Ordering::Relaxed),
            self.loops.load(Ordering::Relaxed)
        )
    }
}

/// A MIDI output device.
pub trait MidiDevice: Send {
    fn short(&mut self, m: [u8; 3], n: u8);
    fn sysex(&mut self, data: &[u8]);
}

pub enum Cmd {
    Play { seq: Arc<Sequence>, looped: bool, ctl: Arc<SoundCtl>, state: Arc<VoiceState> },
    Stop,
    Quit,
}

/// Handle to the MIDI thread.
pub struct MidiThread {
    tx: Sender<Cmd>,
    pub stats: Arc<MidiStats>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl MidiThread {
    /// Starts the thread and opens the OS synthesiser; Err if no device opened.
    pub fn start(globals: Arc<Globals>) -> Result<MidiThread, String> {
        let (tx, rx) = channel();
        let (ready_tx, ready_rx) = channel();
        let stats = Arc::new(MidiStats::default());
        let st = stats.clone();
        let handle = std::thread::Builder::new()
            .name("midi".into())
            .spawn(move || match open_device() {
                Ok((dev, name)) => {
                    *st.device.lock().unwrap() = name.clone();
                    let _ = ready_tx.send(Ok(name));
                    run(dev, rx, globals, st);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| e.to_string())?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(_)) => Ok(MidiThread { tx, stats, handle: Some(handle) }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("MIDI device did not open in time".into()),
        }
    }

    pub fn send(&self, c: Cmd) {
        let _ = self.tx.send(c);
    }
}

impl Drop for MidiThread {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Quit);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

struct Current {
    player: SeqPlayer,
    ctl: Arc<SoundCtl>,
    state: Arc<VoiceState>,
    mixer: ChannelMixer,
    last_levels: Instant,
}

fn levels(dev: &mut dyn MidiDevice, mixer: &ChannelMixer, stats: &MidiStats) {
    for m in mixer.all_levels() {
        dev.short(m, 3);
    }
    stats.level_updates.fetch_add(1, Ordering::Relaxed);
}

fn silence(dev: &mut dyn MidiDevice) {
    for m in reset_messages() {
        dev.short(m, 3);
    }
}

/// The sequencer loop (device-independent; tests drive it with a fake device).
pub fn run(mut dev: Box<dyn MidiDevice>, rx: Receiver<Cmd>, globals: Arc<Globals>, stats: Arc<MidiStats>) {
    #[cfg(windows)]
    // SAFETY: plain winmm timer-resolution calls.
    unsafe {
        windows::Win32::Media::timeBeginPeriod(1);
    }
    let mut cur: Option<Current> = None;
    let mut last = Instant::now();
    let mut buf: Vec<Out<'static>> = Vec::new();
    let mut last_report = Instant::now();
    loop {
        if super::debug() && last_report.elapsed() >= Duration::from_secs(2) {
            last_report = Instant::now();
            eprintln!("audio: MIDI {}", stats.summary());
        }
        let wait = if cur.is_some() { Duration::from_millis(2) } else { Duration::from_millis(250) };
        match rx.recv_timeout(wait) {
            Ok(Cmd::Play { seq, looped, ctl, state }) => {
                if let Some(c) = cur.take() {
                    silence(dev.as_mut());
                    c.state.finish();
                }
                let mut mixer = ChannelMixer::default();
                {
                    let p = ctl.p.lock().unwrap();
                    let m = mix_of(&p, globals.volume(), Instant::now());
                    mixer.gain = m.gain;
                    mixer.pan = m.pan;
                }
                levels(dev.as_mut(), &mixer, &stats);
                stats.songs.fetch_add(1, Ordering::Relaxed);
                cur = Some(Current { player: SeqPlayer::new(seq, looped), ctl, state, mixer, last_levels: Instant::now() });
                last = Instant::now();
            }
            Ok(Cmd::Stop) => {
                if let Some(c) = cur.take() {
                    silence(dev.as_mut());
                    c.state.finish();
                }
            }
            Ok(Cmd::Quit) | Err(RecvTimeoutError::Disconnected) => {
                if let Some(c) = cur.take() {
                    c.state.finish();
                }
                silence(dev.as_mut());
                break;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        let Some(c) = cur.as_mut() else { continue };
        if c.state.stop_requested() {
            silence(dev.as_mut());
            c.state.finish();
            cur = None;
            continue;
        }
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f64();
        last = now;
        // volume / pan / fade: re-send channel levels at most every 20 ms
        if now.duration_since(c.last_levels) >= Duration::from_millis(20) {
            c.last_levels = now;
            if let Ok(p) = c.ctl.p.try_lock() {
                let m = mix_of(&p, globals.volume(), now);
                let old = (c.mixer.gain, c.mixer.pan);
                c.mixer.gain = m.gain;
                c.mixer.pan = m.pan;
                let changed = (0..16).any(|ch| {
                    let mut o = c.mixer.clone();
                    o.gain = old.0;
                    o.pan = old.1;
                    o.vol_for(ch) != c.mixer.vol_for(ch) || o.pan_for(ch) != c.mixer.pan_for(ch)
                });
                if changed {
                    levels(dev.as_mut(), &c.mixer, &stats);
                }
            }
        }
        buf.clear();
        let mut sysex: Vec<Vec<u8>> = Vec::new();
        c.player.advance(dt * globals.tempo(), &mut |o| match o {
            Out::Short(m, n) => buf.push(Out::Short(m, n)),
            Out::SysEx(d) => {
                sysex.push(d.to_vec());
                buf.push(Out::SysEx(&[]));
            }
            Out::LoopReset => buf.push(Out::LoopReset),
        });
        let mut sx = sysex.into_iter();
        for o in buf.drain(..) {
            match o {
                Out::Short(m, n) => {
                    let (m, n) = c.mixer.filter(m, n);
                    if m[0] & 0xF0 == 0x90 && m[2] > 0 {
                        stats.note_ons.fetch_add(1, Ordering::Relaxed);
                    }
                    stats.short_msgs.fetch_add(1, Ordering::Relaxed);
                    dev.short(m, n);
                }
                Out::SysEx(_) => {
                    if let Some(d) = sx.next() {
                        stats.sysex_msgs.fetch_add(1, Ordering::Relaxed);
                        dev.sysex(&d);
                    }
                }
                Out::LoopReset => {
                    stats.loops.fetch_add(1, Ordering::Relaxed);
                    silence(dev.as_mut());
                    c.mixer = ChannelMixer { gain: c.mixer.gain, pan: c.mixer.pan, ..Default::default() };
                    levels(dev.as_mut(), &c.mixer, &stats);
                }
            }
        }
        if c.player.finished {
            silence(dev.as_mut());
            c.state.finish();
            cur = None;
        }
    }
    #[cfg(windows)]
    // SAFETY: matches the timeBeginPeriod above.
    unsafe {
        windows::Win32::Media::timeEndPeriod(1);
    }
}

#[cfg(windows)]
mod win {
    use super::MidiDevice;
    use windows::Win32::Media::Audio::*;

    pub struct WinMidi {
        h: HMIDIOUT,
    }

    // SAFETY: an HMIDIOUT may be used from any thread; this one is only used by the MIDI thread.
    unsafe impl Send for WinMidi {}

    impl MidiDevice for WinMidi {
        fn short(&mut self, m: [u8; 3], n: u8) {
            let msg = m[0] as u32 | if n > 1 { (m[1] as u32) << 8 } else { 0 } | if n > 2 { (m[2] as u32) << 16 } else { 0 };
            // SAFETY: valid open handle.
            unsafe {
                midiOutShortMsg(self.h, msg);
            }
        }
        fn sysex(&mut self, data: &[u8]) {
            let mut buf = data.to_vec();
            let mut hdr = MIDIHDR {
                lpData: windows::core::PSTR(buf.as_mut_ptr()),
                dwBufferLength: buf.len() as u32,
                dwBytesRecorded: buf.len() as u32,
                ..Default::default()
            };
            let sz = std::mem::size_of::<MIDIHDR>() as u32;
            // SAFETY: the header and buffer outlive the call; we wait for completion before unpreparing.
            unsafe {
                if midiOutPrepareHeader(self.h, &mut hdr, sz) != 0 {
                    return;
                }
                if midiOutLongMsg(self.h, &hdr, sz) == 0 {
                    let t0 = std::time::Instant::now();
                    let done = |h: &mut MIDIHDR| std::ptr::addr_of!(std::hint::black_box(h).dwFlags).read_unaligned() & MHDR_DONE != 0;
                    while !done(&mut hdr) && t0.elapsed().as_millis() < 200 {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                let mut tries = 0;
                while midiOutUnprepareHeader(self.h, &mut hdr, sz) == MIDIERR_STILLPLAYING && tries < 100 {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    tries += 1;
                }
            }
        }
    }

    impl Drop for WinMidi {
        fn drop(&mut self) {
            // SAFETY: valid open handle, closed once.
            unsafe {
                midiOutReset(self.h);
                midiOutClose(self.h);
            }
        }
    }

    pub fn open() -> Result<(Box<dyn MidiDevice>, String), String> {
        // SAFETY: plain winmm queries / open with no callback.
        unsafe {
            let n = midiOutGetNumDevs();
            // MIDI mapper first, then each device
            let mut ids: Vec<u32> = vec![u32::MAX];
            ids.extend(0..n);
            for id in ids {
                let mut h = HMIDIOUT::default();
                if midiOutOpen(&mut h, id, None, None, CALLBACK_NULL) == 0 {
                    let mut caps = MIDIOUTCAPSW::default();
                    let name = if midiOutGetDevCapsW(id as usize, &mut caps, std::mem::size_of::<MIDIOUTCAPSW>() as u32) == 0 {
                        let pname = { caps.szPname };
                        let end = pname.iter().position(|&c| c == 0).unwrap_or(32);
                        String::from_utf16_lossy(&pname[..end])
                    } else {
                        format!("MIDI device {}", id)
                    };
                    let mut dev = WinMidi { h };
                    dev.sysex(&super::GS_RESET);
                    std::thread::sleep(std::time::Duration::from_millis(40));
                    return Ok((Box::new(dev), name));
                }
            }
            Err("no MIDI output device".into())
        }
    }
}

fn open_device() -> Result<(Box<dyn MidiDevice>, String), String> {
    #[cfg(windows)]
    {
        win::open()
    }
    #[cfg(not(windows))]
    {
        let _ = GS_RESET;
        Err("no OS MIDI output on this platform".into())
    }
}

/// `MCI_command`: passes a command string to the Windows MCI (§8).
pub fn mci_command(cmd: &str) -> String {
    #[cfg(windows)]
    {
        let mut c = cmd.as_bytes().to_vec();
        c.retain(|&b| b != 0);
        c.push(0);
        let mut ret = [0u8; 1024];
        // SAFETY: NUL-terminated command, valid return buffer.
        let r = unsafe { windows::Win32::Media::Multimedia::mciSendStringA(windows::core::PCSTR(c.as_ptr()), Some(&mut ret), None) };
        if r != 0 {
            return String::new();
        }
        let end = ret.iter().position(|&b| b == 0).unwrap_or(ret.len());
        String::from_utf8_lossy(&ret[..end]).into_owned()
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::midi::tests::build_smf;

    struct Fake(Arc<Mutex<Vec<[u8; 3]>>>);
    impl MidiDevice for Fake {
        fn short(&mut self, m: [u8; 3], _: u8) {
            self.0.lock().unwrap().push(m);
        }
        fn sysex(&mut self, _: &[u8]) {}
    }

    #[test]
    fn thread_plays_scales_volume_and_stops() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = channel();
        let globals = Arc::new(Globals::default());
        let stats = Arc::new(MidiStats::default());
        let (g2, s2, l2) = (globals.clone(), stats.clone(), log.clone());
        let h = std::thread::spawn(move || run(Box::new(Fake(l2)), rx, g2, s2));
        // ppq 100: a note every 0.05 s, CC7 = 120 at the start
        let tr = vec![(0, vec![0xB0, 7, 120]), (0, vec![0x90, 60, 100]), (10, vec![0x80, 60, 0]), (0, vec![0x90, 62, 100]), (10, vec![0x80, 62, 0])];
        let seq = Arc::new(Sequence::parse(&build_smf(0, 100, &[tr])).unwrap());
        let ctl = Arc::new(SoundCtl::new(1, 0.5, 0.0, 0));
        let state = Arc::new(VoiceState::default());
        tx.send(Cmd::Play { seq, looped: true, ctl: ctl.clone(), state: state.clone() }).unwrap();
        std::thread::sleep(Duration::from_millis(400));
        assert!(!state.is_done());
        assert!(stats.loops.load(Ordering::Relaxed) >= 2, "{}", stats.summary());
        assert!(stats.note_ons.load(Ordering::Relaxed) >= 4);
        // the file's CC7 120 is sent scaled by the sound volume 0.5
        assert!(log.lock().unwrap().contains(&[0xB0, 7, 60]));
        // a fade to 0 drives channel volume to 0
        ctl.fade(0.0, 50.0);
        std::thread::sleep(Duration::from_millis(150));
        assert!(log.lock().unwrap().iter().rev().take(64).any(|m| *m == [0xB0, 7, 0]));
        state.request_stop();
        std::thread::sleep(Duration::from_millis(50));
        assert!(state.is_done());
        tx.send(Cmd::Quit).unwrap();
        h.join().unwrap();
        // the last messages are all-notes-off / reset
        let l = log.lock().unwrap();
        assert_eq!(l.last().unwrap()[1], 121);
    }

    #[test]
    fn tempo_factor_speeds_up() {
        let (tx, rx) = channel();
        let globals = Arc::new(Globals::default());
        globals.set_tempo(4.0);
        let stats = Arc::new(MidiStats::default());
        let (g2, s2) = (globals.clone(), stats.clone());
        let h = std::thread::spawn(move || run(Box::new(Fake(Arc::new(Mutex::new(Vec::new())))), rx, g2, s2));
        // one note per 0.5 s at normal tempo → ~0.125 s at tempo 4
        let tr = vec![(0, vec![0x90, 60, 100]), (96, vec![0x80, 60, 0])];
        let seq = Arc::new(Sequence::parse(&build_smf(0, 96, &[tr])).unwrap());
        let state = Arc::new(VoiceState::default());
        tx.send(Cmd::Play { seq, looped: false, ctl: Arc::new(SoundCtl::new(1, 1.0, 0.0, 0)), state: state.clone() }).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(state.is_done(), "a 0.5 s song at tempo 4 ends within 0.3 s");
        tx.send(Cmd::Quit).unwrap();
        h.join().unwrap();
    }
}
