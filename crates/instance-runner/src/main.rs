//! `instance-runner <project.gmk|.gm81|project folder> [--headless] [--frames N] [--screenshot out.png]
//! [--screenshot-every K DIR] [--input SCRIPT] [--renderer gl|software]
//! [--compare-renderers DIR] [--coverage]`
//!
//! Without `--headless` the game opens in a window rendered with OpenGL (falling
//! back to the software renderer if OpenGL cannot be initialised; `--renderer
//! software` forces it). Headless runs use the software renderer unless
//! `--renderer gl` is given (OpenGL on a hidden window). `--compare-renderers DIR`
//! runs headless with both renderers fed the same draw calls and compares their
//! frames (every `--screenshot-every K` steps, or the last frame), writing
//! `soft`, `gl` and `diff` PNGs to DIR.
//! `--input` takes a comma-separated scripted input queue of
//! `STEP:down:KEY`, `STEP:up:KEY`, `STEP:press:KEY` (down, then up two steps
//! later), `STEP:move:X:Y`, `STEP:click:X:Y`, `STEP:char:C`. `--random-input SEED`
//! adds a deterministic pseudo-random key/mouse script; `--max-seconds S` aborts a
//! headless run that takes longer (exit code 4).
//! Joystick items: `STEP:joy1:button:N:down|up`, `STEP:joy1:axis:x|y|z|r|u|v:VALUE`,
//! `STEP:joy1:pov:ANGLE` (−1 centred), `STEP:joy1:connect|disconnect` (also `joy2`).
//!
//! Test-harness options (headless only; docs/AGENT_GUIDE.md §10):
//! * `--exec STEP:CODE` (repeatable), `--exec-file STEP:FILE.gml`: GML run at the
//!   start of step STEP, before its events (STEP 0: after the first room's
//!   Create / Game Start / Room Start, before the first step), compiled and run
//!   as `execute_string`. `self` = `other` = the first live instance of the room
//!   (instance order), or the global pseudo-instance when there is none (see
//!   `instance_runner::harness`). Errors print as `ERROR in --exec at step N:`
//!   and do not stop the run.
//! * `--exec-room ROOM`: go to ROOM (name or number) before the first step: the
//!   first room is entered as usual (Create, Game Start, Room Start events; no
//!   step), then ROOM as by `room_goto`, so globals and persistent controllers
//!   the first room sets up exist. `--exec-room-direct ROOM` enters ROOM instead
//!   of the first room (Game Start events run there; the first room's set-up is
//!   missing).
//! * `--watch K:EXPR` (repeatable): after every K-th step prints
//!   `watch step N: EXPR = value` on stderr (same context as `--exec`); an error
//!   prints once (again only after a success).
//! * `--save-dir DIR`: `working_directory` becomes DIR (created if missing): files
//!   the game writes with relative names (saves, ini, text files, included files
//!   exported to the working directory) land there. `program_directory` stays the
//!   game's folder. A relative name missing from DIR is copied there from the
//!   game's folder the first time the game uses it (bundled data still loads;
//!   the original is never written); `--no-read-fallback` turns that off for a
//!   clean start (e.g. ignore a save file next to the game).
//! * `--screenshot-at LIST DIR`: frames after the listed steps (`60,200,450`) as
//!   `DIR/frame00060.png` (as `--screenshot-every`; both can be combined).
//! * `--no-draw`: frames that are not screenshotted are not rendered: the Draw
//!   events still run (games keep any logic there), animation advances and
//!   Animation End fires as usual, but backgrounds, tiles, default sprite draws,
//!   particles and all drawing into the back buffer are skipped (drawing into
//!   surfaces still happens). Reading the screen back (`draw_getpixel`,
//!   `screen_save`, `surface_copy` from the screen) sees an older frame.
//! * `--random-input SEED[@START]` (monkey input from step START, default 3),
//!   `--random-keys LIST` (key names as in `--input`), `--random-no-mouse`; the
//!   random mouse positions cover the first room's window (region) size.
//! * `--audio-out FILE.wav`: the game's audio mixed offline (44.1 kHz 16-bit stereo,
//!   `1 / room_speed` s per step, deterministic) with MIDI through the SoundFont
//!   (`INSTANCE_SOUNDFONT`); a loudness summary is printed at the end.
//! * `--log FILE`: everything written to stderr is also written to FILE.
//! * `--seed N`: `randomize()` (also the one at game start) seeds from N instead
//!   of the clock, so `random*` functions repeat from run to run.
//!
//! `--debug` runs in debug mode with the Debug Information window
//! (specs/ide.md §20.3). Console (Windows): runs of a project file print to the
//! parent's console or a new console window (`--new-console` forces a new one,
//! `--no-console` suppresses it: the IDE uses it and reads the piped output); a
//! standalone game never shows one (see `console.rs`). Errors are shown in the
//! error dialog (`error_dialog.rs`) and also written to stderr. On Windows the runner is a GUI program, so a shell does not
//! wait for it: use `Start-Process -Wait` or redirect its output in scripts.
//! (On `wasm32` this binary is empty; the browser runner is `instance_runner::web`.)

#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
mod console;
#[cfg(not(target_arch = "wasm32"))]
mod debug_window;
#[cfg(not(target_arch = "wasm32"))]
mod egui_window;
#[cfg(not(target_arch = "wasm32"))]
mod error_dialog;
#[cfg(not(target_arch = "wasm32"))]
mod standalone;
#[cfg(not(target_arch = "wasm32"))]
mod window;

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    desktop::run_main()
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
use instance_runner::draw::backend::Backend;
use instance_runner::{Game, InputEvent};
use std::path::PathBuf;
use crate::window::Renderer;
use crate::{standalone, window};

struct Opts {
    file: PathBuf,
    headless: bool,
    frames: u64,
    screenshot: Option<PathBuf>,
    every: Option<(u64, PathBuf)>,
    input: Vec<(u64, InputEvent)>,
    renderer: Option<Renderer>,
    compare: Option<PathBuf>,
    max_seconds: Option<f64>,
    random_input: Option<u64>,
    debug: bool,
    /// The game came from this executable (no console, ever).
    standalone: bool,
    no_console: bool,
    new_console: bool,
    execs: Vec<instance_runner::harness::Exec>,
    exec_room: Option<String>,
    exec_room_direct: bool,
    watches: Vec<(u64, String)>,
    save_dir: Option<PathBuf>,
    read_fallback: bool,
    shots_at: Vec<(u64, PathBuf)>,
    no_draw: bool,
    random_start: Option<u64>,
    random_keys: Option<Vec<u8>>,
    random_mouse: bool,
    audio_out: Option<PathBuf>,
    log: Option<PathBuf>,
    seed: Option<u32>,
}

/// What the command line asks for.
enum Parsed {
    Run(Box<Opts>),
    /// Print this and exit.
    Print(String),
}

/// The keys of the monkey script: arrows, space, enter, WASD, Z, X, shift.
const RANDOM_KEYS: [u8; 13] = [37, 38, 39, 40, 32, 13, b'W', b'A', b'S', b'D', b'Z', b'X', 16];

/// Settings of the monkey script.
struct Monkey<'a> {
    seed: u64,
    /// First step with random events (3 by default).
    start: u64,
    keys: &'a [u8],
    mouse: bool,
    /// Mouse area (window size).
    w: u32,
    h: u32,
}

/// A deterministic "monkey" input script up to step `frames`: held keys (by
/// default arrows/WASD/space/enter/Z/X/shift), mouse moves and left/right clicks
/// inside the `w`x`h` window. Escape and function keys are never pressed by
/// default. With the default settings (start 3, default keys, mouse on) the
/// sequence of events depends only on the seed (positions scale with the size).
fn random_input(m: &Monkey, frames: u64) -> Vec<(u64, InputEvent)> {
    let seed = m.seed;
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) | 1;
    let mut rnd = move |n: u64| -> u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % n.max(1)
    };
    let keys = if m.keys.is_empty() { &RANDOM_KEYS[..] } else { m.keys };
    let (w, h) = (m.w.max(1) as u64, m.h.max(1) as u64);
    let mut out = Vec::new();
    let mut f = m.start;
    while f < frames {
        let r = rnd(10);
        match if m.mouse { r } else { 0 } {
            0..=4 => {
                let k = keys[rnd(keys.len() as u64) as usize];
                let hold = 1 + rnd(20);
                out.push((f, InputEvent::KeyDown(k)));
                out.push((f + hold, InputEvent::KeyUp(k)));
            }
            5..=7 => {
                let (x, y) = (rnd(w) as f64, rnd(h) as f64);
                let b = if rnd(4) == 0 { 2 } else { 1 };
                out.push((f, InputEvent::MouseMove(x, y)));
                out.push((f, InputEvent::MouseDown(b)));
                out.push((f + 1 + rnd(4), InputEvent::MouseUp(b)));
            }
            _ => {
                let x = rnd(w) as f64;
                out.push((f, InputEvent::MouseMove(x, rnd(h) as f64)))
            }
        }
        f += 2 + rnd(8);
    }
    out
}

fn parse_input(s: &str) -> Result<Vec<(u64, InputEvent)>, String> {
    let mut out = Vec::new();
    for item in s.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
        let p: Vec<&str> = item.split(':').collect();
        let bad = || format!("bad input item '{}'", item);
        let step: u64 = p.first().and_then(|x| x.parse().ok()).ok_or_else(bad)?;
        let kind = *p.get(1).ok_or_else(bad)?;
        let key = || p.get(2).and_then(|k| instance_runner::input::parse_key(k)).ok_or_else(bad);
        let xy = || -> Result<(f64, f64), String> {
            Ok((p.get(2).and_then(|v| v.parse().ok()).ok_or_else(bad)?, p.get(3).and_then(|v| v.parse().ok()).ok_or_else(bad)?))
        };
        match kind {
            "down" => out.push((step, InputEvent::KeyDown(key()?))),
            "up" => out.push((step, InputEvent::KeyUp(key()?))),
            "press" => {
                let k = key()?;
                out.push((step, InputEvent::KeyDown(k)));
                out.push((step + 2, InputEvent::KeyUp(k)));
            }
            "move" => {
                let (x, y) = xy()?;
                out.push((step, InputEvent::MouseMove(x, y)));
            }
            "click" => {
                let (x, y) = xy()?;
                out.push((step, InputEvent::MouseMove(x, y)));
                out.push((step, InputEvent::MouseDown(1)));
                out.push((step + 1, InputEvent::MouseUp(1)));
            }
            "char" => out.push((step, InputEvent::Char(p.get(2).and_then(|c| c.chars().next()).ok_or_else(bad)?))),
            k if k.starts_with("joy") => out.push((step, InputEvent::Joy(instance_runner::joystick::parse_scripted(&p[1..]).ok_or_else(bad)?))),
            _ => return Err(bad()),
        }
    }
    Ok(out)
}

fn parse_args() -> Result<Parsed, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut o = Opts {
        file: PathBuf::new(),
        headless: false,
        frames: 300,
        screenshot: None,
        every: None,
        input: Vec::new(),
        renderer: None,
        compare: None,
        max_seconds: None,
        random_input: None,
        debug: false,
        standalone: false,
        no_console: false,
        new_console: false,
        execs: Vec::new(),
        exec_room: None,
        exec_room_direct: false,
        watches: Vec::new(),
        save_dir: None,
        read_fallback: true,
        shots_at: Vec::new(),
        no_draw: false,
        random_start: None,
        random_keys: None,
        random_mouse: true,
        audio_out: None,
        log: None,
        seed: None,
    };
    let mut harness_opt: Option<&'static str> = None;
    let mut file = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let mut next = || {
            i += 1;
            args.get(i).cloned().ok_or_else(|| format!("{} needs a value", a))
        };
        match a {
            "--headless" => o.headless = true,
            "--frames" => o.frames = next()?.parse().map_err(|_| "bad --frames")?,
            "--screenshot" => o.screenshot = Some(PathBuf::from(next()?)),
            "--screenshot-every" => {
                let k: u64 = next()?.parse().map_err(|_| "bad --screenshot-every")?;
                o.every = Some((k.max(1), PathBuf::from(next()?)));
            }
            "--input" => o.input = parse_input(&next()?)?,
            "--renderer" => {
                o.renderer = Some(match next()?.as_str() {
                    "gl" | "opengl" => Renderer::Gl,
                    "software" | "soft" => Renderer::Software,
                    r => return Err(format!("unknown renderer '{}' (gl or software)", r)),
                })
            }
            "--compare-renderers" => {
                o.compare = Some(PathBuf::from(next()?));
                o.headless = true;
            }
            "--random-input" => {
                let v = next()?;
                let (seed, start) = match v.split_once('@') {
                    Some((s, t)) => (s, Some(t.parse::<u64>().map_err(|_| "bad --random-input start step")?)),
                    None => (v.as_str(), None),
                };
                o.random_input = Some(seed.parse().map_err(|_| "bad --random-input")?);
                o.random_start = start;
            }
            "--random-keys" => {
                let v = next()?;
                let mut keys = Vec::new();
                for k in v.split(',').map(|k| k.trim()).filter(|k| !k.is_empty()) {
                    keys.push(instance_runner::input::parse_key(k).ok_or_else(|| format!("bad key '{}' in --random-keys", k))?);
                }
                if keys.is_empty() {
                    return Err("--random-keys needs at least one key".into());
                }
                o.random_keys = Some(keys);
            }
            "--random-no-mouse" => o.random_mouse = false,
            "--exec" => {
                let v = next()?;
                let (step, code) = instance_runner::harness::split_step(&v).ok_or("--exec needs STEP:CODE")?;
                o.execs.push(instance_runner::harness::Exec { step, origin: "--exec".into(), code: code.to_string() });
                harness_opt = Some("--exec");
            }
            "--exec-file" => {
                let v = next()?;
                let (step, path) = instance_runner::harness::split_step(&v).ok_or("--exec-file needs STEP:FILE")?;
                let code = std::fs::read(path).map_err(|e| format!("--exec-file {}: {}", path, e))?;
                let code = instance_format::text::decode_cp1252(&code);
                o.execs.push(instance_runner::harness::Exec { step, origin: format!("--exec-file {}", path), code });
                harness_opt = Some("--exec-file");
            }
            "--exec-room" | "--exec-room-direct" => {
                o.exec_room = Some(next()?);
                o.exec_room_direct = a == "--exec-room-direct";
                harness_opt = Some("--exec-room");
            }
            "--watch" => {
                let v = next()?;
                let (k, expr) = instance_runner::harness::split_step(&v).ok_or("--watch needs K:EXPR")?;
                if expr.trim().is_empty() {
                    return Err("--watch needs K:EXPR".into());
                }
                o.watches.push((k.max(1), expr.to_string()));
                harness_opt = Some("--watch");
            }
            "--save-dir" => {
                o.save_dir = Some(PathBuf::from(next()?));
                harness_opt = Some("--save-dir");
            }
            "--no-read-fallback" => o.read_fallback = false,
            "--screenshot-at" => {
                let list = next()?;
                let dir = PathBuf::from(next()?);
                for n in list.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
                    let n: u64 = n.parse().map_err(|_| format!("bad step '{}' in --screenshot-at", n))?;
                    o.shots_at.push((n, dir.clone()));
                }
                harness_opt = Some("--screenshot-at");
            }
            "--no-draw" => {
                o.no_draw = true;
                harness_opt = Some("--no-draw");
            }
            "--audio-out" => {
                o.audio_out = Some(PathBuf::from(next()?));
                harness_opt = Some("--audio-out");
            }
            "--log" => o.log = Some(PathBuf::from(next()?)),
            "--seed" => {
                o.seed = Some(next()?.parse::<i64>().map_err(|_| "bad --seed")? as u32);
                harness_opt = Some("--seed");
            }
            "--max-seconds" => o.max_seconds = Some(next()?.parse().map_err(|_| "bad --max-seconds")?),
            "--debug" => o.debug = true,
            "--no-console" => o.no_console = true,
            "--new-console" => o.new_console = true,
            "--coverage" => {
                use std::fmt::Write;
                let (imp, stub, native, total) = instance_runner::builtins::coverage();
                let mut t = format!("builtin functions: {} total; {} native (gml crate), {} implemented by runner, {} stubbed, {} missing
", total, native, imp, stub, total - native - imp - stub);
                for m in instance_runner::builtins::missing() {
                    let _ = writeln!(t, "  missing: {}", m);
                }
                let _ = writeln!(t, "action functions implemented: {}", instance_runner::action_lib::names().len());
                return Ok(Parsed::Print(t));
            }
            "--help" | "-h" => {
                return Ok(Parsed::Print(HELP.into()));
            }
            _ if a.starts_with("--") => return Err(format!("unknown option {}", a)),
            _ => {
                if file.is_none() {
                    file = Some(PathBuf::from(a));
                }
                // further arguments are game parameters
            }
        }
        i += 1;
    }
    // no file: a standalone executable carries its game (specs/exe-format.md)
    o.file = match file {
        Some(f) => f,
        None => {
            let f = standalone::extract_own_game().ok_or("no game file given")?;
            o.standalone = true;
            f
        }
    };
    if let Some(opt) = harness_opt {
        if !o.headless {
            return Err(format!("{} needs --headless", opt));
        }
    }
    if o.compare.is_some() && o.no_draw {
        eprintln!("instance-runner: --no-draw is ignored with --compare-renderers");
        o.no_draw = false;
    }
    // windowed runs: the monkey script uses a 640x480 area (headless runs make
    // it once the first room is known, see run_headless)
    if let (Some(seed), false) = (o.random_input, o.headless) {
        let keys = o.random_keys.clone().unwrap_or_default();
        let m = Monkey { seed, start: o.random_start.unwrap_or(3), keys: &keys, mouse: o.random_mouse, w: 640, h: 480 };
        o.input.extend(random_input(&m, o.frames));
    }
    Ok(Parsed::Run(Box::new(o)))
}

const HELP: &str = "usage: instance-runner FILE.gmk|FILE.gm81|PROJECT_FOLDER [options] [game parameters]
  --debug                     debug mode with the Debug Information window
  --headless                  no window (software renderer unless --renderer gl)
  --frames N                  headless: steps to run (default 300)
  --screenshot OUT.png        headless: save the last frame
  --screenshot-every K DIR    headless: save every K-th frame as DIR/frameNNNNN.png
  --screenshot-at LIST DIR    headless: save the frames after the listed steps (60,200,450)
  --input SCRIPT              scripted input: STEP:down|up|press:KEY, STEP:move|click:X:Y,
                              STEP:char:C, STEP:joy1:... (comma-separated)
  --random-input SEED[@START] add a deterministic random key/mouse script from step START (3)
  --random-keys LIST          keys of the random script (names as in --input: left,right,1,r)
  --random-no-mouse           random script without mouse events
  --exec STEP:CODE            headless: run GML at the start of step STEP (0 = before the first
                              step); self = first instance (or global); repeatable
  --exec-file STEP:FILE.gml   headless: same, code from a file
  --exec-room ROOM            headless: enter the first room (no step), then ROOM (name or number)
  --exec-room-direct ROOM     headless: start in ROOM instead of the first room
  --watch K:EXPR              headless: print `watch step N: EXPR = value` every K steps
  --save-dir DIR              headless: working_directory = DIR (relative saves/ini/text files go
                              there); missing files are copied from the game folder on first use
  --no-read-fallback          with --save-dir: do not copy files from the game folder
  --no-draw                   headless: render only screenshotted frames (Draw events still run)
  --audio-out FILE.wav        headless: record the game audio offline (44.1 kHz stereo, game time)
  --log FILE                  also write everything printed on stderr to FILE
  --seed N                    headless: randomize() seeds from N (reproducible random numbers)
  --max-seconds S             headless: give up after S seconds (exit 4)
  --renderer gl|software      renderer
  --compare-renderers DIR     headless: compare the software and OpenGL renderers
  --coverage                  print built-in function coverage
  --no-console, --new-console console handling (Windows)
";

/// Compares the two renderers' back buffers; saves images when `save`.
fn compare_frames(g: &mut Game, dir: &std::path::Path, step: u64, save: bool, worst: &mut (u8, usize, u64)) {
    let a = g.gfx.be.read_back();
    let Some(b) = g.gfx.be.read_back_secondary() else { return };
    let tol = 8;
    let (mx, n) = instance_runner::draw::dual::compare(&a, &b, tol);
    let pct = 100.0 * n as f64 / a.px.len().max(1) as f64;
    eprintln!("compare step {:5}: max channel diff {:3}, {:6} px (> {}) = {:.3}%", step, mx, n, tol, pct);
    if n > worst.1 {
        *worst = (mx.max(worst.0), n, step);
    } else {
        worst.0 = worst.0.max(mx);
    }
    if save {
        let _ = a.save_png(&dir.join(format!("step{:05}_soft.png", step)), true);
        let _ = b.save_png(&dir.join(format!("step{:05}_gl.png", step)), true);
        let _ = instance_runner::draw::dual::diff_image(&a, &b, tol).save_png(&dir.join(format!("step{:05}_diff.png", step)), true);
    }
}

fn run_headless(o: Opts) -> i32 {
    let mut vm = gml::Vm::new();
    let save = o.save_dir.as_ref().map(|d| instance_runner::SaveDir { dir: d.clone(), read_fallback: o.read_fallback });
    let mut g: Game = match instance_runner::load_game_with(&o.file, &mut vm, true, save.as_ref()) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("instance-runner: {}", e);
            return 1;
        }
    };
    if o.debug {
        g.enable_debug();
    }
    if let Some(dir) = &o.compare {
        let _ = std::fs::create_dir_all(dir);
        match instance_runner::draw::gl::GlBackend::hidden() {
            Ok(gl) => {
                let soft: Box<dyn Backend> = Box::new(instance_runner::draw::soft::SoftBackend::default());
                g.gfx.set_backend(Box::new(instance_runner::draw::dual::DualBackend { a: soft, b: Box::new(gl) }));
            }
            Err(e) => {
                eprintln!("instance-runner: cannot create an OpenGL context: {}", e);
                return 1;
            }
        }
    } else if o.renderer == Some(Renderer::Gl) {
        match instance_runner::draw::gl::GlBackend::hidden() {
            Ok(gl) => g.gfx.set_backend(Box::new(gl)),
            Err(e) => eprintln!("instance-runner: OpenGL initialisation failed ({}); using the software renderer", e),
        }
    }
    eprintln!("instance-runner: renderer: {}", g.gfx.be.name());
    if o.save_dir.is_some() {
        eprintln!("instance-runner: working_directory: {}", g.working_directory);
    }
    if o.no_draw {
        let inner = std::mem::replace(&mut g.gfx.be, Box::new(instance_runner::draw::soft::SoftBackend::default()));
        g.gfx.be = Box::new(instance_runner::draw::discard::DiscardBackend { inner, discard: false });
    }
    if let Some(p) = &o.audio_out {
        if let Err(e) = g.audio.start_offline(Some(p)) {
            eprintln!("instance-runner: cannot write {}: {}", p.display(), e);
            return 1;
        }
    }
    g.scripted_input = o.input;
    if let Some(limit) = o.max_seconds {
        // watchdog: a game that hangs (endless loop in GML) must not block forever
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f64(limit));
            eprintln!("TIMEOUT: gave up after {}s", limit);
            log::finish();
            std::process::exit(4);
        });
    }
    let start_room = match &o.exec_room {
        Some(r) => match g.room_by_name(r) {
            Some(k) => Some(k),
            None => {
                eprintln!("instance-runner: --exec-room: no room '{}'", r);
                return 1;
            }
        },
        None => None,
    };
    // frames saved after step n (n = steps done), and whether frame n is drawn
    let shot_dirs = |n: u64| -> Vec<&std::path::Path> {
        let mut v: Vec<&std::path::Path> = Vec::new();
        if let (Some((k, d)), None) = (&o.every, &o.compare) {
            if n.is_multiple_of(*k) {
                v.push(d);
            }
        }
        for (s, d) in &o.shots_at {
            if *s == n && !v.contains(&d.as_path()) {
                v.push(d);
            }
        }
        v
    };
    let rendered = |n: u64| -> bool {
        !o.no_draw || !shot_dirs(n).is_empty() || (o.screenshot.is_some() && n == o.frames) || o.every.as_ref().is_some_and(|(k, _)| o.compare.is_some() && n.is_multiple_of(*k))
    };
    g.skip_render = !rendered(0);
    vm.settings.randomize_seed = o.seed;
    if let Err(e) = g.start_in(&mut vm, start_room, o.exec_room_direct) {
        eprintln!("instance-runner: error starting game: {}", e);
        return 1;
    }
    if let Some(seed) = o.random_input {
        let keys = o.random_keys.clone().unwrap_or_default();
        let m = Monkey { seed, start: o.random_start.unwrap_or(3), keys: &keys, mouse: o.random_mouse, w: g.region_w, h: g.region_h };
        g.scripted_input.extend(random_input(&m, o.frames));
    }
    if let Some((_, d)) = &o.every {
        let _ = std::fs::create_dir_all(d);
    }
    for (_, d) in &o.shots_at {
        let _ = std::fs::create_dir_all(d);
    }
    let mut watches: Vec<instance_runner::harness::Watch> =
        o.watches.iter().map(|(k, e)| instance_runner::harness::Watch::new(*k, e)).collect();
    let sync = g.gfx.be.name() != "software";
    let mut worst = (0u8, 0usize, 0u64);
    let t0 = web_time::Instant::now();
    let mut slowest = (0.0f64, 0u64);
    for f in 0..o.frames {
        if g.ended {
            eprintln!("game ended at step {}", f);
            break;
        }
        let ts = web_time::Instant::now();
        if !o.execs.is_empty() {
            g.harness_run_execs(&mut vm, &o.execs);
            if g.ended {
                eprintln!("game ended at step {}", f);
                break;
            }
        }
        g.skip_render = !rendered(f + 1);
        if let Err(e) = g.step(&mut vm) {
            eprintln!("FATAL: {}", e);
            break;
        }
        if o.audio_out.is_some() {
            let dt = 1.0 / g.rs.speed.max(1.0);
            g.audio.offline_advance(dt, None);
        }
        if !watches.is_empty() {
            g.harness_watch(&mut vm, &mut watches);
        }
        if sync && !g.skip_render {
            // wait for the GPU so timings are honest
            let t = web_time::Instant::now();
            let _ = g.gfx.be.pixel(instance_runner::draw::Target::Back, 0, 0);
            g.draw_time += t.elapsed().as_secs_f64();
        }
        let el = ts.elapsed().as_secs_f64();
        if el > slowest.0 {
            slowest = (el, f);
        }
        if let (Some((k, _)), Some(cd)) = (&o.every, &o.compare) {
            if (f + 1) % k == 0 {
                compare_frames(&mut g, cd, f + 1, true, &mut worst);
            }
        }
        let dirs = shot_dirs(f + 1);
        if !dirs.is_empty() {
            let img = g.frame_image();
            for d in dirs {
                let _ = img.save_png(&d.join(format!("frame{:05}.png", f + 1)), true);
            }
        }
    }
    let steps = g.frame.max(1) as f64;
    eprintln!(
        "ran {} steps in {:.2}s (drawing {:.2}s = {:.3} ms/frame, total {:.3} ms/step); room {} ({}), {} instances, caption {:?}",
        g.frame,
        t0.elapsed().as_secs_f64(),
        g.draw_time,
        g.draw_time * 1000.0 / steps,
        t0.elapsed().as_secs_f64() * 1000.0 / steps,
        g.room,
        g.rooms.get(g.room.max(0) as usize).and_then(|r| r.as_ref()).map(|r| r.data.name.clone()).unwrap_or_default(),
        g.instances.len(),
        g.caption()
    );
    if let Some(s) = g.gfx.be.stats() {
        eprintln!("renderer stats: {}", s);
    }
    if let Some(cd) = &o.compare {
        if o.every.is_none() {
            let fr = g.frame;
            compare_frames(&mut g, cd, fr, true, &mut worst);
        }
        eprintln!("compare: worst step {} with {} differing pixels, max channel diff {}", worst.2, worst.1, worst.0);
    }
    eprintln!("slowest step: {:.4}s (step {})", slowest.0, slowest.1);
    if !g.unsupported_calls.is_empty() {
        let list: Vec<String> = g.unsupported_calls.iter().map(|(n, c)| format!("{} x{}", n, c)).collect();
        eprintln!("stubbed/unsupported functions called: {}", list.join(", "));
    }
    if std::env::var_os("GM_DUMP_INSTANCES").is_some() {
        for i in &g.instances {
            eprintln!(
                "  {} {} ({:.1},{:.1}) spr {} img {:.2} vis {} depth {} active {} solid {}",
                i.id,
                g.object(i.object_index).map(|o| o.name.as_str()).unwrap_or("?"),
                i.x,
                i.y,
                i.sprite_index,
                i.image_index,
                i.visible,
                i.depth,
                i.active,
                i.solid
            );
        }
        let v = &g.rs.views[0];
        eprintln!("  view0 visible {} view ({},{} {}x{}) port ({},{} {}x{})", v.visible, v.xview, v.yview, v.wview, v.hview, v.xport, v.yport, v.wport, v.hport);
    }
    g.profile_report();
    if let Some(p) = &o.audio_out {
        if let Some(s) = g.audio.offline_finish() {
            eprintln!("audio-out: {}: {}", p.display(), s);
        }
    }
    if let Some(p) = o.screenshot {
        if let Err(e) = g.frame_image().save_png(&p, true) {
            eprintln!("cannot write screenshot: {}", e);
            return 1;
        }
    }
    if g.aborted {
        2
    } else {
        0
    }
}

pub fn run_main() {
    let no_console = std::env::args().any(|a| a == "--no-console");
    let o = match parse_args() {
        Ok(Parsed::Run(o)) => *o,
        Ok(Parsed::Print(t)) => {
            if !no_console {
                crate::console::init(false);
            }
            print!("{}", t);
            return;
        }
        Err(e) => {
            if !no_console {
                crate::console::init(false);
            }
            eprintln!("instance-runner: {}", e);
            std::process::exit(2);
        }
    };
    if !o.standalone && !o.no_console {
        crate::console::init(o.new_console);
    }
    if let Some(p) = &o.log {
        if let Err(e) = log::start(p) {
            eprintln!("instance-runner: cannot write log {}: {}", p.display(), e);
            std::process::exit(2);
        }
    }
    let code = if o.headless {
        let h = std::thread::Builder::new()
            .name("game".into())
            .stack_size(instance_runner::GAME_STACK_SIZE)
            .spawn(move || run_headless(o))
            .expect("spawn game thread");
        h.join().unwrap_or(3)
    } else {
        let r = o.renderer.unwrap_or(Renderer::Gl);
        window::run(o.file, o.input, r, o.debug)
    };
    log::finish();
    std::process::exit(code);
}

/// `--log FILE`: the process's stderr goes through a pipe; a thread copies it
/// to the original stderr and to the file.
mod log {
    use std::io::{Read, Write};
    use std::sync::Mutex;

    struct Tee {
        restore: Box<dyn FnOnce() + Send>,
        thread: std::thread::JoinHandle<()>,
    }

    static TEE: Mutex<Option<Tee>> = Mutex::new(None);

    fn pump(mut r: std::io::PipeReader, mut file: std::fs::File, mut orig: Option<std::fs::File>) {
        let mut buf = [0u8; 8192];
        loop {
            match r.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let _ = file.write_all(&buf[..n]);
                    let _ = file.flush();
                    if let Some(o) = orig.as_mut() {
                        if o.write_all(&buf[..n]).is_err() {
                            orig = None;
                        }
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    pub fn start(path: &std::path::Path) -> std::io::Result<()> {
        use std::os::windows::io::{AsRawHandle, FromRawHandle};
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::Console::{GetStdHandle, SetStdHandle, STD_ERROR_HANDLE};
        let file = std::fs::File::create(path)?;
        let (r, w) = std::io::pipe()?;
        // SAFETY: the original stderr handle stays open for the whole process;
        // the copy below is only written to, never closed (ManuallyDrop).
        unsafe {
            let orig = GetStdHandle(STD_ERROR_HANDLE).ok().filter(|h| !h.is_invalid() && !h.0.is_null());
            let orig_raw = orig.map(|h| h.0 as isize);
            let out = orig_raw.map(|h| std::mem::ManuallyDrop::new(std::fs::File::from_raw_handle(h as *mut std::ffi::c_void)));
            // a File that is never closed: clone the handle into an owned one
            let out = out.and_then(|f| f.try_clone().ok());
            SetStdHandle(STD_ERROR_HANDLE, HANDLE(w.as_raw_handle())).map_err(std::io::Error::other)?;
            let thread = std::thread::spawn(move || pump(r, file, out));
            let restore = move || {
                let h = orig_raw.map(|h| HANDLE(h as *mut std::ffi::c_void)).unwrap_or_default();
                let _ = SetStdHandle(STD_ERROR_HANDLE, h);
                drop(w);
            };
            *TEE.lock().unwrap() = Some(Tee { restore: Box::new(restore), thread });
        }
        Ok(())
    }

    #[cfg(unix)]
    pub fn start(path: &std::path::Path) -> std::io::Result<()> {
        use std::os::fd::{AsRawFd, FromRawFd};
        extern "C" {
            fn dup(fd: i32) -> i32;
            fn dup2(fd: i32, to: i32) -> i32;
        }
        let file = std::fs::File::create(path)?;
        let (r, w) = std::io::pipe()?;
        // SAFETY: plain descriptor duplication; `orig` is owned by the File below
        // and a second duplicate restores fd 2.
        unsafe {
            let orig = dup(2);
            let back = dup(2);
            if orig < 0 || back < 0 || dup2(w.as_raw_fd(), 2) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let out = Some(std::fs::File::from_raw_fd(orig));
            let thread = std::thread::spawn(move || pump(r, file, out));
            let restore = move || {
                dup2(back, 2);
                drop(w);
            };
            *TEE.lock().unwrap() = Some(Tee { restore: Box::new(restore), thread });
        }
        Ok(())
    }

    #[cfg(not(any(windows, unix)))]
    pub fn start(_: &std::path::Path) -> std::io::Result<()> {
        Err(std::io::Error::other("not supported on this platform"))
    }

    /// Restores stderr and waits until everything has been written.
    pub fn finish() {
        let t = TEE.lock().unwrap().take();
        if let Some(t) = t {
            let _ = std::io::stderr().flush();
            (t.restore)();
            let _ = t.thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The monkey script before the START/keys/area options (reference).
    fn old_random_input(seed: u64, frames: u64) -> Vec<(u64, InputEvent)> {
        let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) | 1;
        let mut rnd = move |n: u64| -> u64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % n.max(1)
        };
        let mut out = Vec::new();
        let mut f = 3;
        while f < frames {
            match rnd(10) {
                0..=4 => {
                    let k = RANDOM_KEYS[rnd(RANDOM_KEYS.len() as u64) as usize];
                    let hold = 1 + rnd(20);
                    out.push((f, InputEvent::KeyDown(k)));
                    out.push((f + hold, InputEvent::KeyUp(k)));
                }
                5..=7 => {
                    let (x, y) = (rnd(640) as f64, rnd(480) as f64);
                    let b = if rnd(4) == 0 { 2 } else { 1 };
                    out.push((f, InputEvent::MouseMove(x, y)));
                    out.push((f, InputEvent::MouseDown(b)));
                    out.push((f + 1 + rnd(4), InputEvent::MouseUp(b)));
                }
                _ => out.push((f, InputEvent::MouseMove(rnd(640) as f64, rnd(480) as f64))),
            }
            f += 2 + rnd(8);
        }
        out
    }

    #[test]
    fn random_input_options() {
        let m = |seed, start, keys: &'static [u8], mouse, w, h| Monkey { seed, start, keys, mouse, w, h };
        for seed in [0, 1, 7, 12345] {
            assert_eq!(random_input(&m(seed, 3, &[], true, 640, 480), 500), old_random_input(seed, 500), "default script unchanged");
        }
        let late = random_input(&m(1, 100, &[], true, 640, 480), 500);
        assert!(late.iter().all(|(s, _)| *s >= 100) && !late.is_empty());
        let keys = random_input(&m(1, 3, &[37, 39], false, 640, 480), 500);
        assert!(keys.iter().all(|(_, e)| matches!(e, InputEvent::KeyDown(37 | 39) | InputEvent::KeyUp(37 | 39))));
        let small = random_input(&m(1, 3, &[], true, 100, 50), 2000);
        assert!(small.iter().all(|(_, e)| !matches!(e, InputEvent::MouseMove(x, y) if *x >= 100.0 || *y >= 50.0)));
    }

    #[test]
    fn parse_harness_options() {
        let s = instance_runner::harness::split_step("120:global.cash = 1e5; a = b:c");
        assert_eq!(s, Some((120, "global.cash = 1e5; a = b:c")));
        assert_eq!(instance_runner::harness::split_step("x:1"), None);
    }
}
}
