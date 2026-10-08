//! Windowed frontend: winit event loop on the main thread, game logic on a
//! big-stack thread. Frames are presented either by OpenGL from the game
//! thread (the context is made current there on the winit window) or, as a
//! fallback, by copying software-rendered pixels to the window with softbuffer.

use crate::debug_window::{DebugWindow, SharedLink};
use gm_runner::draw::gl::{GlBackend, WindowHandles};

/// The window and its raw handles, sent to the game thread for OpenGL.
type WinMsg = (Arc<Window>, Result<WindowHandles, String>);
use gm_runner::draw::Backend;
use gm_runner::frontend::{self, mapping, Mapping, Pacer};
use gm_runner::{Game, InputEvent, UiHooks};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

/// Space between the game window and the debug window (physical pixels).
const DEBUG_GAP: u32 = 8;

/// Renderer choice (`--renderer`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Renderer {
    Gl,
    Software,
}

#[derive(Debug)]
enum UserEvent {
    Frame,
    Exit(i32),
    /// The game's own window icon (RGBA, width, height).
    Icon(Vec<u8>, u32, u32),
    /// New data for the debug window.
    Debug,
}

/// Window icon from straight RGBA pixels.
fn window_icon(rgba: Vec<u8>, w: u32, h: u32) -> Option<winit::window::Icon> {
    winit::window::Icon::from_rgba(rgba, w, h).ok()
}

/// The gm81-clean icon, used unless the game has an icon of its own.
fn default_icon() -> Option<winit::window::Icon> {
    window_icon(gm_brand::icon_rgba(48), 48, 48)
}

/// What the game thread publishes for the window (and the window for it).
#[derive(Default)]
struct Slot {
    /// Software frames (0RGB).
    pixels: Vec<u32>,
    /// Region (back buffer) size.
    w: u32,
    h: u32,
    caption: String,
    /// >0 fixed scale, 0 fill, <0 keep aspect.
    scale: f64,
    fullscreen: bool,
    cursor_visible: bool,
    /// Requested client size (0 = region × scale).
    req_w: i32,
    req_h: i32,
    resize: bool,
    /// Centre the window on its monitor at the next window update.
    recenter: bool,
    /// Explicit outer position requested by the game (window_set_position etc.).
    set_pos: Option<(i32, i32)>,
    set_mouse: Option<(f64, f64)>,
    /// Current client size of the window (set by the main thread).
    win_w: u32,
    win_h: u32,
    /// Colour outside the region (0RGB).
    outside: u32,
    /// Frames are presented by OpenGL from the game thread.
    gl: bool,
}

fn slot_mapping(s: &Slot) -> Mapping {
    mapping(s.win_w, s.win_h, s.w, s.h, s.scale, s.fullscreen)
}

struct App {
    slot: Arc<Mutex<Slot>>,
    input: Sender<InputEvent>,
    window: Option<Arc<Window>>,
    win_tx: Option<Sender<WinMsg>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    exit_code: i32,
    last_caption: String,
    fullscreen: bool,
    shown: bool,
    /// Debug mode: the link to the game thread, and the Debug Information window.
    debug_link: Option<SharedLink>,
    debug: Option<DebugWindow>,
}

/// Virtual key code of a winit key (its Debug name is the DOM `code` name).
fn vk(code: KeyCode) -> Option<u8> {
    gm_runner::input::vk_from_code(&format!("{:?}", code))
}

impl App {
    fn create_window(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("")
            .with_window_icon(default_icon())
            .with_visible(false)
            .with_inner_size(winit::dpi::PhysicalSize::new(640, 480));
        let win = match el.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("gm-runner: cannot create window: {}", e);
                el.exit();
                return;
            }
        };
        {
            let size = win.inner_size();
            let mut s = self.slot.lock().unwrap();
            s.win_w = size.width;
            s.win_h = size.height;
        }
        if let Some(tx) = self.win_tx.take() {
            let _ = tx.send((win.clone(), WindowHandles::of(&win)));
        }
        if let Some(link) = self.debug_link.clone() {
            match DebugWindow::new(el, link, Some(&win)) {
                Ok(d) => self.debug = Some(d),
                Err(e) => eprintln!("gm-runner: cannot open the debug window: {}", e),
            }
        }
        self.window = Some(win);
    }

    /// Software presentation through softbuffer.
    fn redraw(&mut self) {
        let Some(win) = self.window.clone() else { return };
        if self.slot.lock().unwrap().gl {
            return;
        }
        if self.surface.is_none() {
            let ctx = match softbuffer::Context::new(win.clone()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("gm-runner: softbuffer: {}", e);
                    return;
                }
            };
            match softbuffer::Surface::new(&ctx, win.clone()) {
                Ok(s) => self.surface = Some(s),
                Err(e) => {
                    eprintln!("gm-runner: softbuffer: {}", e);
                    return;
                }
            }
        }
        let surface = self.surface.as_mut().unwrap();
        let size = win.inner_size();
        let (ww, wh) = (size.width.max(1), size.height.max(1));
        if surface.resize(NonZeroU32::new(ww).unwrap(), NonZeroU32::new(wh).unwrap()).is_err() {
            return;
        }
        let s = self.slot.lock().unwrap();
        let Ok(mut buf) = surface.buffer_mut() else { return };
        let (rw, rh) = (s.w.max(1) as f64, s.h.max(1) as f64);
        let m = mapping(ww, wh, s.w, s.h, s.scale, s.fullscreen);
        let outside = s.outside;
        for y in 0..wh {
            let ry = ((y as f64 - m.oy) / m.sy).floor();
            let row = (y * ww) as usize;
            if ry < 0.0 || ry >= rh || s.pixels.len() < (s.w * s.h) as usize || s.pixels.is_empty() {
                buf[row..row + ww as usize].fill(outside);
                continue;
            }
            let src_row = ry as usize * s.w as usize;
            for x in 0..ww {
                let rx = ((x as f64 - m.ox) / m.sx).floor();
                buf[row + x as usize] = if rx < 0.0 || rx >= rw { outside } else { s.pixels[src_row + rx as usize] };
            }
        }
        drop(s);
        let _ = buf.present();
    }

    fn apply_window_state(&mut self) {
        let Some(win) = self.window.clone() else { return };
        let mut s = self.slot.lock().unwrap();
        if s.caption != self.last_caption {
            win.set_title(&s.caption);
            self.last_caption = s.caption.clone();
        }
        if s.fullscreen != self.fullscreen {
            self.fullscreen = s.fullscreen;
            win.set_fullscreen(if s.fullscreen { Some(Fullscreen::Borderless(None)) } else { None });
        }
        if std::mem::take(&mut s.resize) && !self.fullscreen {
            let (w, h) = client_size(&s);
            if let Some(size) = win.request_inner_size(winit::dpi::LogicalSize::new(w, h)) {
                s.win_w = size.width;
                s.win_h = size.height;
            }
        }
        let recenter = std::mem::take(&mut s.recenter);
        if let Some((x, y)) = s.set_pos.take() {
            if !self.fullscreen {
                win.set_outer_position(PhysicalPosition::new(x, y));
            }
        } else if recenter && !self.fullscreen {
            // in debug mode the game and the debug window are centred side by side
            let extra = self.debug.as_ref().map(|d| d.outer_width() + DEBUG_GAP).unwrap_or(0);
            center_window(&win, extra);
            if let Some(d) = &self.debug {
                d.place_beside(&win, DEBUG_GAP);
            }
        }
        win.set_cursor_visible(s.cursor_visible);
        if let Some((x, y)) = s.set_mouse.take() {
            let m = slot_mapping(&s);
            let _ = win.set_cursor_position(PhysicalPosition::new(m.ox + x * m.sx, m.oy + y * m.sy));
        }
        drop(s);
        if !self.shown {
            self.shown = true;
            win.set_visible(true);
            // above a console window opened at start-up
            win.focus_window();
        }
    }
}

/// Centres the window on its current (else the primary) monitor, keeping the
/// title bar on screen.
/// `extra` (physical pixels) is reserved to the right of the window.
fn center_window(win: &Window, extra: u32) {
    let Some(mon) = win.current_monitor().or_else(|| win.primary_monitor()) else { return };
    let (mp, ms, os) = (mon.position(), mon.size(), win.outer_size());
    let x = mp.x + (ms.width as i32 - os.width as i32 - extra as i32) / 2;
    let y = mp.y + ((ms.height as i32 - os.height as i32) / 2).max(0);
    win.set_outer_position(PhysicalPosition::new(x.max(mp.x), y));
}

fn client_size(s: &Slot) -> (u32, u32) {
    if s.req_w > 0 && s.req_h > 0 {
        return (s.req_w as u32, s.req_h as u32);
    }
    let k = if s.scale > 0.0 { s.scale } else { 1.0 };
    (((s.w.max(1) as f64) * k) as u32, ((s.h.max(1) as f64) * k) as u32)
}

fn rgb_to_0rgb(c: u32) -> u32 {
    frontend::rgb_to_0rgb(c)
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        self.create_window(el);
    }

    fn user_event(&mut self, el: &ActiveEventLoop, ev: UserEvent) {
        match ev {
            UserEvent::Frame => {
                self.create_window(el);
                self.apply_window_state();
                if let Some(w) = &self.window {
                    if !self.slot.lock().unwrap().gl {
                        w.request_redraw();
                    }
                }
            }
            UserEvent::Exit(code) => {
                self.exit_code = code;
                el.exit();
            }
            UserEvent::Icon(rgba, w, h) => {
                if let Some(win) = &self.window {
                    win.set_window_icon(window_icon(rgba, w, h));
                }
            }
            UserEvent::Debug => {
                if let Some(d) = &self.debug {
                    d.wake();
                }
            }
        }
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(d) = &mut self.debug {
            if d.id() == id {
                d.event(&event);
                return;
            }
        }
        match event {
            WindowEvent::CloseRequested => {
                let _ = self.input.send(InputEvent::Close);
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Resized(size) => {
                let mut s = self.slot.lock().unwrap();
                s.win_w = size.width;
                s.win_h = size.height;
            }
            WindowEvent::Focused(f) => {
                let _ = self.input.send(InputEvent::Focus(f));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if let Some(k) = vk(code) {
                        let e = if event.state == ElementState::Pressed { InputEvent::KeyDown(k) } else { InputEvent::KeyUp(k) };
                        let _ = self.input.send(e);
                    }
                }
                if event.state == ElementState::Pressed {
                    if let Some(t) = &event.text {
                        for c in t.chars() {
                            let _ = self.input.send(InputEvent::Char(c));
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = slot_mapping(&self.slot.lock().unwrap()).to_region(position.x, position.y);
                let _ = self.input.send(InputEvent::MouseMove(x, y));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let b = match button {
                    MouseButton::Left => 1,
                    MouseButton::Right => 2,
                    MouseButton::Middle => 3,
                    _ => return,
                };
                let e = if state == ElementState::Pressed { InputEvent::MouseDown(b) } else { InputEvent::MouseUp(b) };
                let _ = self.input.send(e);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let d = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(p) => p.y,
                };
                if d != 0.0 {
                    let _ = self.input.send(InputEvent::Wheel(if d > 0.0 { 1 } else { -1 }));
                }
            }
            _ => {}
        }
    }
}


/// Native dialogs through rfd.
struct RfdUi {
    title: Arc<Mutex<String>>,
}

impl UiHooks for RfdUi {
    fn error(&mut self, msg: &str, fatal: bool) -> bool {
        eprintln!("{}", msg);
        if fatal {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Error")
                .set_description(msg)
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
            return true;
        }
        let r = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Error")
            .set_description(format!("{}\n\nPress OK to ignore the error, Cancel to abort the game.", msg))
            .set_buttons(rfd::MessageButtons::OkCancel)
            .show();
        !matches!(r, rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes)
    }
    fn message(&mut self, msg: &str) {
        let t = self.title.lock().unwrap().clone();
        rfd::MessageDialog::new().set_title(t).set_description(msg).set_buttons(rfd::MessageButtons::Ok).show();
    }
    fn question(&mut self, msg: &str) -> bool {
        let t = self.title.lock().unwrap().clone();
        let r = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title(t)
            .set_description(msg)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();
        matches!(r, rfd::MessageDialogResult::Yes | rfd::MessageDialogResult::Ok)
    }
    fn get_string(&mut self, _msg: &str, default: &str) -> String {
        default.to_string()
    }
    fn buttons(&mut self, msg: &str, buttons: &[String]) -> i32 {
        let b: Vec<&String> = buttons.iter().filter(|b| !b.is_empty()).collect();
        let t = self.title.lock().unwrap().clone();
        let d = rfd::MessageDialog::new().set_title(t).set_description(msg);
        match b.len() {
            0 => {
                d.set_buttons(rfd::MessageButtons::Ok).show();
                0
            }
            1 => {
                d.set_buttons(rfd::MessageButtons::Ok).show();
                1
            }
            2 => match d.set_buttons(rfd::MessageButtons::OkCancelCustom(b[0].clone(), b[1].clone())).show() {
                rfd::MessageDialogResult::Custom(s) if s == *b[1] => 2,
                rfd::MessageDialogResult::Cancel | rfd::MessageDialogResult::No => 2,
                _ => 1,
            },
            _ => match d.set_buttons(rfd::MessageButtons::YesNoCancelCustom(b[0].clone(), b[1].clone(), b[2].clone())).show() {
                rfd::MessageDialogResult::Custom(s) if s == *b[1] => 2,
                rfd::MessageDialogResult::Custom(s) if s == *b[2] => 3,
                rfd::MessageDialogResult::No => 2,
                rfd::MessageDialogResult::Cancel => 3,
                _ => 1,
            },
        }
    }
    fn file_dialog(&mut self, save: bool, filter: &str, name: &str) -> String {
        let mut d = rfd::FileDialog::new();
        let parts: Vec<&str> = filter.split('|').collect();
        for pair in parts.chunks(2) {
            if pair.len() == 2 {
                let exts: Vec<String> = pair[1]
                    .split(';')
                    .map(|m| m.trim().trim_start_matches("*.").to_string())
                    .filter(|e| !e.is_empty() && e != "*")
                    .collect();
                if !exts.is_empty() {
                    d = d.add_filter(pair[0], &exts);
                }
            }
        }
        if !name.is_empty() {
            d = d.set_file_name(name);
        }
        let r = if save { d.save_file() } else { d.pick_file() };
        r.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
    }
    fn directory(&mut self, start: &str) -> String {
        let mut d = rfd::FileDialog::new();
        if !start.is_empty() {
            d = d.set_directory(start);
        }
        d.pick_folder().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
    }
}


/// Installs the presentation hook for the chosen renderer.
fn install_present(g: &mut Game, slot: &Arc<Mutex<Slot>>, proxy: &EventLoopProxy<UserEvent>, gl: bool) {
    let slot = slot.clone();
    let proxy = proxy.clone();
    if gl {
        g.gfx.present = Some(Box::new(move |be: &mut dyn Backend| {
            let (w, h) = be.back_size();
            let s = slot.lock().unwrap();
            let m = mapping(s.win_w, s.win_h, w, h, s.scale, s.fullscreen);
            let (ww, wh, out) = (s.win_w, s.win_h, s.outside);
            drop(s);
            if let Some(gl) = be.as_gl() {
                let o = [(out >> 16) as u8, (out >> 8) as u8, out as u8, 255];
                gl.present(ww, wh, m.ox, m.oy, m.sx, m.sy, o, false);
            }
            let _ = proxy.send_event(UserEvent::Frame);
        }));
    } else {
        g.gfx.present = Some(Box::new(move |be: &mut dyn Backend| {
            let img = be.read_back();
            let mut s = slot.lock().unwrap();
            s.w = img.w;
            s.h = img.h;
            s.pixels.clear();
            s.pixels.extend(img.px.iter().map(|p| ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | p[2] as u32));
            drop(s);
            let _ = proxy.send_event(UserEvent::Frame);
        }));
    }
}

#[allow(clippy::too_many_arguments)]
fn game_thread(
    path: PathBuf,
    input: Vec<(u64, InputEvent)>,
    rx: Receiver<InputEvent>,
    slot: Arc<Mutex<Slot>>,
    proxy: EventLoopProxy<UserEvent>,
    renderer: Renderer,
    win_rx: Receiver<WinMsg>,
    debug: Option<SharedLink>,
) -> i32 {
    let mut vm = gml::Vm::new();
    let mut g = match gm_runner::load_game(&path, &mut vm, false) {
        Ok(g) => g,
        Err(e) => {
            rfd::MessageDialog::new().set_title("gm-runner").set_description(&e).show();
            eprintln!("gm-runner: {}", e);
            return 1;
        }
    };
    if debug.is_some() {
        g.enable_debug();
        eprintln!("gm-runner: debug mode");
    }
    let (mut debug_last, mut debug_seen) = (None, 0u64);
    let mut use_gl = false;
    if renderer == Renderer::Gl {
        match win_rx.recv() {
            Ok((win, Ok(h))) => match GlBackend::for_window(win, h, g.settings.vsync & 1 != 0) {
                Ok(gl) => {
                    g.gfx.set_backend(Box::new(gl));
                    use_gl = true;
                }
                Err(e) => eprintln!("gm-runner: OpenGL initialisation failed ({}); using the software renderer", e),
            },
            Ok((_, Err(e))) => eprintln!("gm-runner: no window handle for OpenGL ({}); using the software renderer", e),
            Err(_) => return 1,
        }
    }
    eprintln!("gm-runner: renderer: {}", g.gfx.be.name());
    // A game that kept GameMaker's stock icon shows ours (set at window creation).
    if !gm_brand::is_stock_icon(&g.settings.icon) {
        if let Some((rgba, w, h)) = gm_brand::decode_ico(&g.settings.icon, 48) {
            let _ = proxy.send_event(UserEvent::Icon(rgba, w, h));
        }
    }
    {
        let mut s = slot.lock().unwrap();
        s.gl = use_gl;
        s.outside = rgb_to_0rgb(g.settings.color_outside_room);
    }
    let title = Arc::new(Mutex::new(String::new()));
    g.ui = Box::new(RfdUi { title: title.clone() });
    g.scripted_input = input;
    install_present(&mut g, &slot, &proxy, use_gl);
    g.pump = Some(Box::new(move || rx.try_iter().collect()));
    if let Err(e) = g.start(&mut vm) {
        eprintln!("gm-runner: {}", e);
        return 1;
    }
    let mut pacer = Pacer::default();
    let mut last_region = (0, 0);
    let mut last_pos = (i32::MIN, i32::MIN);
    let mut last_center = true;
    let (mut frames, mut busy, mut t_report) = (0u64, 0.0f64, Instant::now());
    let report = std::env::var_os("GM_RUNNER_FPS").is_some();
    while !g.ended {
        let t0 = Instant::now();
        if let Err(e) = frontend::tick(&mut g, &mut vm) {
            eprintln!("gm-runner: {}", e);
            break;
        }
        if let Some(link) = &debug {
            if crate::debug_window::service(&mut g, &mut vm, link, &mut debug_last, &mut debug_seen) {
                let _ = proxy.send_event(UserEvent::Debug);
            }
            if g.ended {
                break;
            }
        }
        {
            let mut s = slot.lock().unwrap();
            let v = frontend::view(&g);
            *title.lock().unwrap() = v.caption.clone();
            s.caption = v.caption;
            s.scale = v.scale;
            s.fullscreen = v.fullscreen;
            s.outside = v.outside;
            s.w = v.w;
            s.h = v.h;
            s.cursor_visible = v.cursor_visible;
            if g.window_dirty || last_region != (g.region_w, g.region_h) {
                if last_region != (g.region_w, g.region_h) || g.win.w > 0 {
                    s.resize = true;
                }
                // centred unless the game placed the window itself (runtime.md §12.1)
                if g.win.center && (s.resize || !last_center) {
                    s.recenter = true;
                }
                if !g.win.center && (g.win.x, g.win.y) != last_pos {
                    s.set_pos = Some((g.win.x, g.win.y));
                    last_pos = (g.win.x, g.win.y);
                }
                if g.win.center {
                    last_pos = (i32::MIN, i32::MIN);
                }
                last_center = g.win.center;
                s.req_w = g.win.w;
                s.req_h = g.win.h;
                last_region = (g.region_w, g.region_h);
                g.window_dirty = false;
            }
            if let Some(m) = g.win.set_mouse.take() {
                s.set_mouse = Some(m);
            }
        }
        g.gfx.present_now();
        busy += t0.elapsed().as_secs_f64();
        frames += 1;
        if report && t_report.elapsed() >= Duration::from_secs(2) {
            eprintln!("gm-runner: {:.2} ms/frame ({} frames, {})", busy * 1000.0 / frames as f64, frames, g.gfx.be.name());
            frames = 0;
            busy = 0.0;
            t_report = Instant::now();
        }
        pacer.advance(g.rs.speed);
        let now = Instant::now();
        if pacer.next() > now {
            std::thread::sleep(pacer.next() - now);
        }
    }
    if g.aborted {
        2
    } else {
        0
    }
}

/// Runs the game in a window; returns the exit code.
pub fn run(path: PathBuf, input: Vec<(u64, InputEvent)>, renderer: Renderer, debug: bool) -> i32 {
    let event_loop = match EventLoop::<UserEvent>::with_user_event().build() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("gm-runner: cannot create event loop: {}", e);
            return 1;
        }
    };
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let slot = Arc::new(Mutex::new(Slot {
        scale: 1.0,
        cursor_visible: true,
        ..Default::default()
    }));
    let (tx, rx) = channel();
    let (win_tx, win_rx) = channel();
    let slot2 = slot.clone();
    let proxy2 = proxy.clone();
    let debug_link: Option<SharedLink> = debug.then(Default::default);
    let link2 = debug_link.clone();
    let handle = std::thread::Builder::new()
        .name("game".into())
        .stack_size(gm_runner::GAME_STACK_SIZE)
        .spawn(move || {
            let code = game_thread(path, input, rx, slot2, proxy2.clone(), renderer, win_rx, link2);
            let _ = proxy2.send_event(UserEvent::Exit(code));
            code
        })
        .expect("spawn game thread");
    let mut app = App {
        slot,
        input: tx,
        window: None,
        win_tx: Some(win_tx),
        surface: None,
        exit_code: 0,
        last_caption: String::new(),
        fullscreen: false,
        shown: false,
        debug_link,
        debug: None,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("gm-runner: event loop error: {}", e);
    }
    drop(app.input);
    let code = app.exit_code;
    // the game thread ends on its own once the window is gone (Close event or exit)
    let _ = handle;
    code
}
