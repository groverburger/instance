//! `instance-runner <project.gmk|.gm81> [--headless] [--frames N] [--screenshot out.png]
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
}

/// What the command line asks for.
enum Parsed {
    Run(Opts),
    /// Print this and exit.
    Print(String),
}

/// A deterministic "monkey" input script for `frames` steps: held arrow/WASD/
/// space/enter/Z/X keys, mouse moves and left/right clicks inside a 640x480 area.
/// Escape and function keys are never pressed.
fn random_input(seed: u64, frames: u64) -> Vec<(u64, InputEvent)> {
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) | 1;
    let mut rnd = move |n: u64| -> u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % n.max(1)
    };
    const KEYS: [u8; 13] = [37, 38, 39, 40, 32, 13, b'W', b'A', b'S', b'D', b'Z', b'X', 16];
    let mut out = Vec::new();
    let mut f = 3;
    while f < frames {
        match rnd(10) {
            0..=4 => {
                let k = KEYS[rnd(KEYS.len() as u64) as usize];
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
    };
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
            "--random-input" => o.random_input = Some(next()?.parse().map_err(|_| "bad --random-input")?),
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
                return Ok(Parsed::Print("usage: instance-runner FILE.gmk|FILE.gm81 [--debug] [--headless] [--frames N] [--screenshot OUT.png] [--screenshot-every K DIR] [--input SCRIPT] [--renderer gl|software] [--compare-renderers DIR] [--coverage] [--max-seconds S] [--random-input SEED] [--no-console] [--new-console]
".into()));
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
    if let Some(seed) = o.random_input {
        o.input.extend(random_input(seed, o.frames));
    }
    Ok(Parsed::Run(o))
}

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
    let mut g: Game = match instance_runner::load_game(&o.file, &mut vm, true) {
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
    g.scripted_input = o.input;
    if let Some(limit) = o.max_seconds {
        // watchdog: a game that hangs (endless loop in GML) must not block forever
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f64(limit));
            eprintln!("TIMEOUT: gave up after {}s", limit);
            std::process::exit(4);
        });
    }
    if let Err(e) = g.start(&mut vm) {
        eprintln!("instance-runner: error starting game: {}", e);
        return 1;
    }
    if let Some((_, d)) = &o.every {
        let _ = std::fs::create_dir_all(d);
    }
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
        if let Err(e) = g.step(&mut vm) {
            eprintln!("FATAL: {}", e);
            break;
        }
        if sync {
            // wait for the GPU so timings are honest
            let t = web_time::Instant::now();
            let _ = g.gfx.be.pixel(instance_runner::draw::Target::Back, 0, 0);
            g.draw_time += t.elapsed().as_secs_f64();
        }
        let el = ts.elapsed().as_secs_f64();
        if el > slowest.0 {
            slowest = (el, f);
        }
        if let Some((k, d)) = &o.every {
            if (f + 1) % k == 0 {
                match &o.compare {
                    Some(cd) => compare_frames(&mut g, cd, f + 1, true, &mut worst),
                    None => {
                        let _ = g.frame_image().save_png(&d.join(format!("frame{:05}.png", f + 1)), true);
                    }
                }
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
        Ok(Parsed::Run(o)) => o,
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
    let code = if o.headless {
        let h = std::thread::Builder::new()
            .name("game".into())
            .stack_size(instance_runner::GAME_STACK_SIZE)
            .spawn(move || run_headless(o))
            .expect("spawn game thread");
        h.join().unwrap_or(3)
    } else {
        // On macOS the OpenGL context would be created off the main thread, which
        // AppKit does not allow: default to the software renderer there.
        let default = if cfg!(target_os = "macos") { Renderer::Software } else { Renderer::Gl };
        let r = o.renderer.unwrap_or(default);
        window::run(o.file, o.input, r, o.debug)
    };
    std::process::exit(code);
}
}
