//! `gm-runner <project.gmk|.gm81> [--headless] [--frames N] [--screenshot out.png]
//! [--screenshot-every K DIR] [--input SCRIPT] [--coverage]`
//!
//! Without `--headless` the game opens in a window (winit + softbuffer, software
//! rendering). `--input` takes a comma-separated scripted input queue of
//! `STEP:down:KEY`, `STEP:up:KEY`, `STEP:press:KEY` (down, then up two steps
//! later), `STEP:move:X:Y`, `STEP:click:X:Y`, `STEP:char:C`.

mod window;

use gm_runner::{Game, InputEvent};
use std::path::PathBuf;

struct Opts {
    file: PathBuf,
    headless: bool,
    frames: u64,
    screenshot: Option<PathBuf>,
    every: Option<(u64, PathBuf)>,
    input: Vec<(u64, InputEvent)>,
    max_seconds: Option<f64>,
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
        max_seconds: None,
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
            "--max-seconds" => o.max_seconds = Some(next()?.parse().map_err(|_| "bad --max-seconds")?),
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
                println!("usage: gm-runner FILE.gmk|FILE.gm81 [--headless] [--frames N] [--screenshot OUT.png] [--screenshot-every K DIR] [--input SCRIPT] [--coverage] [--max-seconds S]");
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

fn run_headless(o: Opts) -> i32 {
    let mut vm = gml::Vm::new();
    let mut g: Game = match gm_runner::load_game(&o.file, &mut vm, true) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("gm-runner: {}", e);
            return 1;
        }
    };
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
        eprintln!("gm-runner: error starting game: {}", e);
        return 1;
    }
    if let Some((_, d)) = &o.every {
        let _ = std::fs::create_dir_all(d);
    }
    let t0 = std::time::Instant::now();
    let mut slowest = (0.0f64, 0u64);
    for f in 0..o.frames {
        if g.ended {
            eprintln!("game ended at step {}", f);
            break;
        }
        let ts = std::time::Instant::now();
        if let Err(e) = g.step(&mut vm) {
            eprintln!("FATAL: {}", e);
            break;
        }
        let el = ts.elapsed().as_secs_f64();
        if el > slowest.0 {
            slowest = (el, f);
        }
        if let Some((k, d)) = &o.every {
            if (f + 1) % k == 0 {
                let _ = g.frame_image().save_png(&d.join(format!("frame{:05}.png", f + 1)), true);
            }
        }
    }
    eprintln!(
        "ran {} steps in {:.2}s (drawing {:.2}s); room {} ({}), {} instances, caption {:?}",
        g.frame,
        t0.elapsed().as_secs_f64(),
        g.draw_time,
        g.room,
        g.rooms.get(g.room.max(0) as usize).and_then(|r| r.as_ref()).map(|r| r.data.name.clone()).unwrap_or_default(),
        g.instances.len(),
        g.caption()
    );
    eprintln!("slowest step: {:.4}s (step {})", slowest.0, slowest.1);
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
        window::run(o.file, o.input)
    };
    std::process::exit(code);
}
