//! `gm-runner <project.gmk|.gm81> [--headless] [--frames N] [--screenshot out.png]
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
//! later), `STEP:move:X:Y`, `STEP:click:X:Y`, `STEP:char:C`.

mod window;

use gm_runner::draw::backend::Backend;
use gm_runner::{Game, InputEvent};
use std::path::PathBuf;
use window::Renderer;

struct Opts {
    file: PathBuf,
    headless: bool,
    frames: u64,
    screenshot: Option<PathBuf>,
    every: Option<(u64, PathBuf)>,
    input: Vec<(u64, InputEvent)>,
    renderer: Option<Renderer>,
    compare: Option<PathBuf>,
}

fn parse_input(s: &str) -> Result<Vec<(u64, InputEvent)>, String> {
    let mut out = Vec::new();
    for item in s.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
        let p: Vec<&str> = item.split(':').collect();
        let bad = || format!("bad input item '{}'", item);
        let step: u64 = p.first().and_then(|x| x.parse().ok()).ok_or_else(bad)?;
        let kind = *p.get(1).ok_or_else(bad)?;
        let key = || p.get(2).and_then(|k| gm_runner::input::parse_key(k)).ok_or_else(bad);
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
            _ => return Err(bad()),
        }
    }
    Ok(out)
}

fn parse_args() -> Result<Option<Opts>, String> {
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
            "--coverage" => {
                let (imp, stub, native, total) = gm_runner::builtins::coverage();
                println!("builtin functions: {} total; {} native (gml crate), {} implemented by runner, {} stubbed, {} missing", total, native, imp, stub, total - native - imp - stub);
                for m in gm_runner::builtins::missing() {
                    println!("  missing: {}", m);
                }
                println!("action functions implemented: {}", gm_runner::action_lib::names().len());
                return Ok(None);
            }
            "--help" | "-h" => {
                println!("usage: gm-runner FILE.gmk|FILE.gm81 [--headless] [--frames N] [--screenshot OUT.png] [--screenshot-every K DIR] [--input SCRIPT] [--renderer gl|software] [--compare-renderers DIR] [--coverage]");
                return Ok(None);
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
    o.file = file.ok_or("no game file given")?;
    Ok(Some(o))
}

/// Compares the two renderers' back buffers; saves images when `save`.
fn compare_frames(g: &mut Game, dir: &std::path::Path, step: u64, save: bool, worst: &mut (u8, usize, u64)) {
    let a = g.gfx.be.read_back();
    let Some(b) = g.gfx.be.read_back_secondary() else { return };
    let tol = 8;
    let (mx, n) = gm_runner::draw::dual::compare(&a, &b, tol);
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
        let _ = gm_runner::draw::dual::diff_image(&a, &b, tol).save_png(&dir.join(format!("step{:05}_diff.png", step)), true);
    }
}

fn run_headless(o: Opts) -> i32 {
    let mut vm = gml::Vm::new();
    let mut g: Game = match gm_runner::load_game(&o.file, &mut vm, true) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("gm-runner: {}", e);
            return 1;
        }
    };
    if let Some(dir) = &o.compare {
        let _ = std::fs::create_dir_all(dir);
        match gm_runner::draw::gl::GlBackend::hidden() {
            Ok(gl) => {
                let soft: Box<dyn Backend> = Box::new(gm_runner::draw::soft::SoftBackend::default());
                g.gfx.set_backend(Box::new(gm_runner::draw::dual::DualBackend { a: soft, b: Box::new(gl) }));
            }
            Err(e) => {
                eprintln!("gm-runner: cannot create an OpenGL context: {}", e);
                return 1;
            }
        }
    } else if o.renderer == Some(Renderer::Gl) {
        match gm_runner::draw::gl::GlBackend::hidden() {
            Ok(gl) => g.gfx.set_backend(Box::new(gl)),
            Err(e) => eprintln!("gm-runner: OpenGL initialisation failed ({}); using the software renderer", e),
        }
    }
    eprintln!("gm-runner: renderer: {}", g.gfx.be.name());
    g.scripted_input = o.input;
    if let Err(e) = g.start(&mut vm) {
        eprintln!("gm-runner: error starting game: {}", e);
        return 1;
    }
    if let Some((_, d)) = &o.every {
        let _ = std::fs::create_dir_all(d);
    }
    let sync = g.gfx.be.name() != "software";
    let mut worst = (0u8, 0usize, 0u64);
    let t0 = std::time::Instant::now();
    for f in 0..o.frames {
        if g.ended {
            eprintln!("game ended at step {}", f);
            break;
        }
        if let Err(e) = g.step(&mut vm) {
            eprintln!("FATAL: {}", e);
            break;
        }
        if sync {
            // wait for the GPU so timings are honest
            let t = std::time::Instant::now();
            let _ = g.gfx.be.pixel(gm_runner::draw::Target::Back, 0, 0);
            g.draw_time += t.elapsed().as_secs_f64();
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

fn main() {
    let o = match parse_args() {
        Ok(Some(o)) => o,
        Ok(None) => return,
        Err(e) => {
            eprintln!("gm-runner: {}", e);
            std::process::exit(2);
        }
    };
    let code = if o.headless {
        let h = std::thread::Builder::new()
            .name("game".into())
            .stack_size(gm_runner::GAME_STACK_SIZE)
            .spawn(move || run_headless(o))
            .expect("spawn game thread");
        h.join().unwrap_or(3)
    } else {
        let r = o.renderer.unwrap_or(Renderer::Gl);
        window::run(o.file, o.input, r)
    };
    std::process::exit(code);
}
