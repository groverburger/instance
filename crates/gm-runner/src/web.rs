//! Browser frontend (`wasm32-unknown-unknown`, see web/play.html).
//!
//! JavaScript fetches or receives the game file and calls [`start_game`] after a
//! user gesture (so audio may start). The game loop is driven by
//! `requestAnimationFrame`: each frame runs the steps that are due at
//! `room_speed` ([`crate::frontend::Pacer`]) and presents once. Rendering uses
//! the OpenGL backend on WebGL2 (falling back to the software renderer and a
//! 2D canvas), input comes from DOM events on the canvas and the document,
//! dialogs use `alert`/`confirm`/`prompt`, and files live in [`crate::vfs`]
//! (persisted in `localStorage`).

use crate::draw::Backend;
use crate::frontend::{self, mapping, Pacer};
use crate::{Game, InputEvent, UiHooks};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, KeyboardEvent, MouseEvent, WheelEvent};

/// Working/program directory of the game in the virtual file system.
const GAME_DIR: &str = "/game";

fn window() -> web_sys::Window {
    web_sys::window().expect("no window")
}

fn log(msg: &str) {
    web_sys::console::log_1(&msg.into());
}

/// Offers text to the system clipboard (best effort, asynchronous).
pub fn copy_text(text: &str) {
    let clip = window().navigator().clipboard();
    let _ = clip.write_text(text);
}

/// Supplies a SoundFont (.sf2) for MIDI music. Call before [`start_game`].
#[wasm_bindgen]
pub fn set_soundfont(data: Vec<u8>) {
    crate::audio::set_soundfont_bytes(data);
}

/// Dialogs through the browser's blocking `alert` / `confirm` / `prompt`.
struct WebUi {
    title: Rc<RefCell<String>>,
}

impl WebUi {
    fn titled(&self, msg: &str) -> String {
        let t = self.title.borrow();
        if t.is_empty() {
            msg.to_string()
        } else {
            format!("{}\n\n{}", t, msg)
        }
    }
}

impl UiHooks for WebUi {
    fn error(&mut self, msg: &str, fatal: bool) -> bool {
        web_sys::console::error_1(&msg.into());
        if fatal {
            let _ = window().alert_with_message(msg);
            return true;
        }
        !window()
            .confirm_with_message(&format!("{}\n\nPress OK to ignore the error, Cancel to abort the game.", msg))
            .unwrap_or(true)
    }
    fn message(&mut self, msg: &str) {
        let _ = window().alert_with_message(&self.titled(msg));
    }
    fn question(&mut self, msg: &str) -> bool {
        window().confirm_with_message(&self.titled(msg)).unwrap_or(false)
    }
    fn get_string(&mut self, msg: &str, default: &str) -> String {
        match window().prompt_with_message_and_default(msg, default) {
            Ok(Some(s)) => s,
            _ => default.to_string(),
        }
    }
    fn has_input(&self) -> bool {
        true
    }
    fn buttons(&mut self, msg: &str, buttons: &[String]) -> i32 {
        let b: Vec<&String> = buttons.iter().filter(|b| !b.is_empty()).collect();
        match b.len() {
            0 | 1 => {
                let _ = window().alert_with_message(&self.titled(msg));
                b.len() as i32
            }
            2 => {
                let q = format!("{}\n\nOK = {}, Cancel = {}", msg, b[0], b[1]);
                if window().confirm_with_message(&q).unwrap_or(false) {
                    1
                } else {
                    2
                }
            }
            _ => {
                let list: Vec<String> = b.iter().enumerate().map(|(i, s)| format!("{} = {}", i + 1, s)).collect();
                let q = format!("{}\n\nType the number of your choice: {}", msg, list.join(", "));
                match window().prompt_with_message_and_default(&q, "1") {
                    Ok(Some(s)) => s.trim().parse::<i32>().ok().filter(|n| (1..=b.len() as i32).contains(n)).unwrap_or(1),
                    _ => 0,
                }
            }
        }
    }
    fn menu(&mut self, items: &[String]) -> Option<usize> {
        let list: Vec<String> = items.iter().enumerate().map(|(i, s)| format!("{} = {}", i + 1, s)).collect();
        let q = format!("Choose: {}", list.join(", "));
        let s = window().prompt_with_message_and_default(&q, "1").ok().flatten()?;
        s.trim().parse::<usize>().ok().filter(|n| (1..=items.len()).contains(n)).map(|n| n - 1)
    }
    fn file_dialog(&mut self, save: bool, _filter: &str, name: &str) -> String {
        let q = if save { "Save as (file name):" } else { "Open (file name):" };
        match window().prompt_with_message_and_default(q, name) {
            Ok(Some(s)) if !s.trim().is_empty() => format!("{}/{}", GAME_DIR, s.trim()),
            _ => String::new(),
        }
    }
}

/// Where the region goes in the canvas (shared with the present hook and the
/// mouse handlers).
#[derive(Clone, Copy)]
struct Placement {
    /// Canvas size in pixels.
    w: u32,
    h: u32,
    map: frontend::Mapping,
    /// Colour around the region (0RGB).
    outside: u32,
}

/// Everything the animation-frame callback needs.
struct Runner {
    g: Game,
    vm: gml::Vm,
    canvas: HtmlCanvasElement,
    pacer: Pacer,
    title: Rc<RefCell<String>>,
    /// Last applied view (canvas size / caption only change when it changes).
    last: Option<frontend::View>,
    place: Rc<RefCell<Placement>>,
    gl: bool,
    done: bool,
}

impl Runner {
    fn apply_view(&mut self) {
        let v = frontend::view(&self.g);
        if self.last.as_ref() == Some(&v) {
            return;
        }
        let doc = window().document().unwrap();
        if !v.caption.is_empty() {
            doc.set_title(&v.caption);
        }
        let (cw, ch) = if v.fullscreen {
            let w = window();
            let iw = w.inner_width().ok().and_then(|x| x.as_f64()).unwrap_or(800.0) as u32;
            let ih = w.inner_height().ok().and_then(|x| x.as_f64()).unwrap_or(600.0) as u32;
            (iw, ih)
        } else if self.gl {
            frontend::client_size(&v)
        } else {
            // software frames are put 1:1 and scaled by CSS
            (v.w.max(1), v.h.max(1))
        };
        if self.canvas.width() != cw || self.canvas.height() != ch {
            self.canvas.set_width(cw);
            self.canvas.set_height(ch);
        }
        let style = self.canvas.style();
        let (css_w, css_h) = if v.fullscreen { (cw, ch) } else { frontend::client_size(&v) };
        let _ = style.set_property("width", &format!("{}px", css_w));
        let _ = style.set_property("height", &format!("{}px", css_h));
        let _ = style.set_property("cursor", if v.cursor_visible { "default" } else { "none" });
        let o = v.outside;
        let _ = style.set_property("background", &format!("#{:06x}", o & 0xFFFFFF));
        let map = if self.gl {
            mapping(cw, ch, v.w, v.h, v.scale, v.fullscreen)
        } else {
            // CSS stretches the region over the element
            let (sx, sy) = (css_w as f64 / v.w.max(1) as f64, css_h as f64 / v.h.max(1) as f64);
            frontend::Mapping { ox: 0.0, oy: 0.0, sx, sy }
        };
        *self.place.borrow_mut() = Placement { w: cw, h: ch, map, outside: v.outside };
        self.last = Some(v);
    }

    /// Runs the steps that are due; returns false once the game has ended.
    fn frame(&mut self) -> bool {
        if self.done {
            return false;
        }
        let mut steps = 0;
        let now = web_time::Instant::now();
        while self.pacer.due(now) && steps < 4 && !self.g.ended {
            if let Err(e) = frontend::tick(&mut self.g, &mut self.vm) {
                web_sys::console::error_1(&format!("gm-runner: {}", e).into());
                self.g.ended = true;
                break;
            }
            *self.title.borrow_mut() = self.g.caption();
            self.apply_view();
            self.g.gfx.present_now();
            self.pacer.advance(self.g.rs.speed);
            self.pacer.delay(std::mem::take(&mut self.g.sleep_debt_ms));
            steps += 1;
        }
        crate::vfs::persist();
        if self.g.ended {
            self.done = true;
            crate::vfs::persist();
            log("gm-runner: game ended");
            let w = window();
            if let Ok(f) = js_sys::Reflect::get(&w, &"gm81GameEnded".into()) {
                if let Some(f) = f.dyn_ref::<js_sys::Function>() {
                    let _ = f.call0(&w);
                }
            }
            return false;
        }
        true
    }
}

fn webgl2(canvas: &HtmlCanvasElement) -> Option<web_sys::WebGl2RenderingContext> {
    let opts = js_sys::Object::new();
    for (k, v) in [("antialias", false), ("alpha", false), ("depth", false), ("premultipliedAlpha", false), ("preserveDrawingBuffer", false)] {
        let _ = js_sys::Reflect::set(&opts, &k.into(), &v.into());
    }
    canvas
        .get_context_with_context_options("webgl2", &opts)
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<web_sys::WebGl2RenderingContext>().ok())
}

/// Installs the presentation hook: WebGL2 blit, or software pixels into a 2D canvas.
fn install_present(g: &mut Game, canvas: &HtmlCanvasElement, place: Rc<RefCell<Placement>>, gl: bool) {
    if gl {
        g.gfx.present = Some(Box::new(move |be: &mut dyn Backend| {
            let p = *place.borrow();
            let o = p.outside;
            if let Some(gl) = be.as_gl() {
                let m = p.map;
                gl.present(p.w, p.h, m.ox, m.oy, m.sx, m.sy, [(o >> 16) as u8, (o >> 8) as u8, o as u8, 255], false);
            }
        }));
        return;
    }
    let ctx = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok());
    g.gfx.present = Some(Box::new(move |be: &mut dyn Backend| {
        let Some(ctx) = &ctx else { return };
        let img = be.read_back();
        let data = img.to_rgba_bytes();
        let opaque: Vec<u8> = data.chunks_exact(4).flat_map(|c| [c[0], c[1], c[2], 255]).collect();
        if let Ok(id) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&opaque), img.w.max(1), img.h.max(1)) {
            let _ = ctx.put_image_data(&id, 0.0, 0.0);
        }
    }));
}

fn listen<E: wasm_bindgen::convert::FromWasmAbi + 'static>(target: &web_sys::EventTarget, ty: &str, f: impl FnMut(E) + 'static) {
    let cb = Closure::<dyn FnMut(E)>::new(f);
    let _ = target.add_event_listener_with_callback(ty, cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Characters a key press types (like a native text event).
fn key_char(e: &KeyboardEvent) -> Option<char> {
    if e.ctrl_key() || e.meta_key() {
        return None;
    }
    let k = e.key();
    match k.as_str() {
        "Enter" => Some('\r'),
        "Backspace" => Some('\u{8}'),
        "Tab" => Some('\t'),
        "Escape" => Some('\u{1b}'),
        _ => {
            let mut it = k.chars();
            let c = it.next()?;
            it.next().is_none().then_some(c)
        }
    }
}

/// Starts `data` (a .gm81/.gmk file) on the canvas with id `canvas_id`.
/// `renderer` is "gl" (default) or "software".
#[wasm_bindgen]
pub fn start_game(canvas_id: &str, data: &[u8], renderer: Option<String>) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let doc = window().document().ok_or("no document")?;
    let canvas: HtmlCanvasElement = doc
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("no element #{}", canvas_id)))?
        .dyn_into()
        .map_err(|_| "not a canvas")?;

    let mut vm = gml::Vm::new();
    let mut g = crate::load_game_bytes(data, GAME_DIR, &mut vm, false).map_err(|e| JsValue::from_str(&e))?;

    let want_gl = renderer.as_deref() != Some("software");
    let mut gl = false;
    if want_gl {
        match webgl2(&canvas) {
            Some(ctx) => match crate::draw::gl::GlBackend::for_webgl2(ctx, canvas.width(), canvas.height()) {
                Ok(be) => {
                    g.gfx.set_backend(Box::new(be));
                    gl = true;
                }
                Err(e) => crate::warn(&format!("gm-runner: WebGL2 initialisation failed ({}); using the software renderer", e)),
            },
            None => crate::warn("gm-runner: WebGL2 is not available; using the software renderer"),
        }
    }
    log(&format!("gm-runner: renderer: {}", g.gfx.be.name()));

    let title = Rc::new(RefCell::new(String::new()));
    g.ui = Box::new(WebUi { title: title.clone() });
    let place = Rc::new(RefCell::new(Placement {
        w: canvas.width(),
        h: canvas.height(),
        map: mapping(canvas.width(), canvas.height(), 640, 480, 1.0, false),
        outside: 0,
    }));
    install_present(&mut g, &canvas, place.clone(), gl);

    // input → queue drained by the game at the start of each step
    let queue: Rc<RefCell<Vec<InputEvent>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let q = queue.clone();
        g.pump = Some(Box::new(move || std::mem::take(&mut *q.borrow_mut())));
    }
    let _ = canvas.set_attribute("tabindex", "0");
    let target: &web_sys::EventTarget = doc.as_ref();
    {
        let q = queue.clone();
        listen(target, "keydown", move |e: KeyboardEvent| {
            if let Some(k) = crate::input::vk_from_code(&e.code()) {
                // keep arrows/space/tab/function keys from scrolling or leaving the page
                if matches!(k, 8 | 9 | 32..=40 | 112..=123) {
                    e.prevent_default();
                }
                if !e.repeat() {
                    q.borrow_mut().push(InputEvent::KeyDown(k));
                }
            }
            if let Some(c) = key_char(&e) {
                q.borrow_mut().push(InputEvent::Char(c));
            }
        });
        let q = queue.clone();
        listen(target, "keyup", move |e: KeyboardEvent| {
            if let Some(k) = crate::input::vk_from_code(&e.code()) {
                q.borrow_mut().push(InputEvent::KeyUp(k));
            }
        });
    }
    let ct: &web_sys::EventTarget = canvas.as_ref();
    {
        let (q, m) = (queue.clone(), place.clone());
        listen(ct, "mousemove", move |e: MouseEvent| {
            let (x, y) = m.borrow().map.to_region(e.offset_x() as f64, e.offset_y() as f64);
            q.borrow_mut().push(InputEvent::MouseMove(x, y));
        });
        let button = |e: &MouseEvent| match e.button() {
            0 => Some(1),
            2 => Some(2),
            1 => Some(3),
            _ => None,
        };
        let (q, m, c2) = (queue.clone(), place.clone(), canvas.clone());
        listen(ct, "mousedown", move |e: MouseEvent| {
            let _ = c2.focus();
            let (x, y) = m.borrow().map.to_region(e.offset_x() as f64, e.offset_y() as f64);
            q.borrow_mut().push(InputEvent::MouseMove(x, y));
            if let Some(b) = button(&e) {
                q.borrow_mut().push(InputEvent::MouseDown(b));
            }
            e.prevent_default();
        });
        let q = queue.clone();
        let win = window();
        let wt: &web_sys::EventTarget = win.as_ref();
        listen(wt, "mouseup", move |e: MouseEvent| {
            if let Some(b) = button(&e) {
                q.borrow_mut().push(InputEvent::MouseUp(b));
            }
        });
        listen(ct, "contextmenu", move |e: MouseEvent| e.prevent_default());
        let q = queue.clone();
        listen(ct, "wheel", move |e: WheelEvent| {
            let d = e.delta_y();
            if d != 0.0 {
                q.borrow_mut().push(InputEvent::Wheel(if d < 0.0 { 1 } else { -1 }));
            }
            e.prevent_default();
        });
        let q = queue.clone();
        listen(wt, "blur", move |_: web_sys::FocusEvent| q.borrow_mut().push(InputEvent::Focus(false)));
        let q = queue.clone();
        listen(wt, "focus", move |_: web_sys::FocusEvent| q.borrow_mut().push(InputEvent::Focus(true)));
        let q = queue.clone();
        listen(wt, "pagehide", move |_: web_sys::Event| {
            q.borrow_mut().push(InputEvent::Close);
            crate::vfs::persist();
        });
    }

    g.start(&mut vm).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let _ = canvas.focus();

    let runner = Rc::new(RefCell::new(Runner {
        g,
        vm,
        canvas,
        pacer: Pacer::default(),
        title,
        last: None,
        place,
        gl,
        done: false,
    }));
    runner.borrow_mut().apply_view();

    // requestAnimationFrame loop
    let cb: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let cb2 = cb.clone();
    *cb.borrow_mut() = Some(Closure::new(move || {
        let keep = runner.borrow_mut().frame();
        if keep {
            if let Some(f) = cb2.borrow().as_ref() {
                let _ = window().request_animation_frame(f.as_ref().unchecked_ref());
            }
        }
    }));
    if let Some(f) = cb.borrow().as_ref() {
        window().request_animation_frame(f.as_ref().unchecked_ref())?;
    }
    Ok(())
}
