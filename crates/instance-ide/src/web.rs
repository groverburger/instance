//! Browser entry point and the JavaScript helpers of `web/ide.js`
//! (`window.instanceIde`): file pickers, downloads, the IndexedDB autosave and
//! running the game in the web runner (an overlay iframe with `play.html`).

use std::cell::RefCell;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Bindings to `window.instanceIde` (defined by `web/ide.js`, loaded before the wasm).
pub mod js {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        /// Shows a file picker; resolves to an array of `{name, bytes}` or null.
        #[wasm_bindgen(js_namespace = instanceIde, js_name = pickFiles)]
        pub fn pick_files(accept: &str, multiple: bool) -> js_sys::Promise;
        /// Offers `bytes` as a download named `name`.
        #[wasm_bindgen(js_namespace = instanceIde)]
        pub fn download(name: &str, bytes: &[u8]);
        /// Stores the game in IndexedDB and opens `play.html` over the IDE.
        #[wasm_bindgen(js_namespace = instanceIde, js_name = runGame)]
        pub fn run_game(bytes: &[u8]);
        /// Stores the autosave (name, bytes) in IndexedDB.
        #[wasm_bindgen(js_namespace = instanceIde, js_name = saveAutosave)]
        pub fn save_autosave(name: &str, bytes: &[u8]) -> js_sys::Promise;
        /// Resolves to `{name, bytes, time}` or null.
        #[wasm_bindgen(js_namespace = instanceIde, js_name = loadAutosave)]
        pub fn load_autosave() -> js_sys::Promise;
        /// Removes the autosave.
        #[wasm_bindgen(js_namespace = instanceIde, js_name = clearAutosave)]
        pub fn clear_autosave() -> js_sys::Promise;
        /// Fetches a URL; resolves to a Uint8Array (rejects on HTTP errors).
        #[wasm_bindgen(js_namespace = instanceIde, js_name = fetchBytes)]
        pub fn fetch_bytes(url: &str) -> js_sys::Promise;
        /// A query parameter of the page URL.
        #[wasm_bindgen(js_namespace = instanceIde, js_name = queryParam)]
        pub fn query_param(name: &str) -> Option<String>;
        /// Hides the "loading" message of the page.
        #[wasm_bindgen(js_namespace = instanceIde, js_name = loaded)]
        pub fn loaded();
    }
}

/// Something that arrived asynchronously for the app.
pub enum Incoming {
    /// A project to open (name, bytes), e.g. from `?project=URL`.
    Open(String, Vec<u8>),
    /// The previous session's autosave (name, bytes, time in ms since the epoch).
    Autosave(String, Vec<u8>, f64),
    /// A file dropped onto the page (name, bytes).
    Dropped(String, Vec<u8>),
    Error(String),
}

thread_local! {
    static INCOMING: RefCell<Vec<Incoming>> = const { RefCell::new(Vec::new()) };
    static STARTED: RefCell<bool> = const { RefCell::new(false) };
}

fn push(i: Incoming) {
    INCOMING.with(|v| v.borrow_mut().push(i));
}

/// Drains what arrived since the last frame.
pub fn take_incoming() -> Vec<Incoming> {
    INCOMING.with(|v| std::mem::take(&mut *v.borrow_mut()))
}

fn bytes_of(v: &JsValue) -> Vec<u8> {
    js_sys::Uint8Array::new(v).to_vec()
}

/// Reads a dropped file in the background; it arrives as [`Incoming::Dropped`].
pub fn read_dropped(f: eframe::egui::DroppedFileHandle) {
    let ctx = crate::dialogs::context();
    wasm_bindgen_futures::spawn_local(async move {
        let name = f.path().display().to_string();
        match f.bytes_async().await {
            Ok(b) => push(Incoming::Dropped(name, b)),
            Err(e) => push(Incoming::Error(format!("{name}: {e}"))),
        }
        if let Some(c) = ctx {
            c.request_repaint();
        }
    });
}

/// Kicks off the startup work once: `?project=URL` is fetched and opened,
/// otherwise a previous autosave is offered.
pub fn startup(ctx: &eframe::egui::Context) {
    if STARTED.with(|s| std::mem::replace(&mut *s.borrow_mut(), true)) {
        return;
    }
    let ctx = ctx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(url) = js::query_param("project").filter(|u| !u.is_empty()) {
            let name = url
                .rsplit('/')
                .next()
                .and_then(|n| n.split(['?', '#']).next())
                .unwrap_or("project.gm81")
                .to_string();
            match wasm_bindgen_futures::JsFuture::from(js::fetch_bytes(&url)).await {
                Ok(b) => push(Incoming::Open(name, bytes_of(&b))),
                Err(e) => push(Incoming::Error(format!(
                    "Cannot load {url}: {}",
                    e.as_string().unwrap_or_else(|| format!("{e:?}"))
                ))),
            }
        } else if let Ok(v) = wasm_bindgen_futures::JsFuture::from(js::load_autosave()).await {
            if !v.is_null() && !v.is_undefined() {
                let get = |k: &str| js_sys::Reflect::get(&v, &k.into()).ok();
                let name = get("name").and_then(|n| n.as_string()).unwrap_or_default();
                let time = get("time").and_then(|t| t.as_f64()).unwrap_or(0.0);
                let bytes = get("bytes").map(|b| bytes_of(&b)).unwrap_or_default();
                if !bytes.is_empty() {
                    push(Incoming::Autosave(name, bytes, time));
                }
            }
        }
        js::loaded();
        ctx.request_repaint();
    });
}

/// File > Export Web Page in the browser: fetches the web runner served beside the
/// IDE (and the SoundFont of `config.js` when the game plays MIDI), builds the page
/// and offers it as a download named `name`.
pub fn export_page(name: String, title: String, game: Vec<u8>, midi: bool) {
    let ctx = crate::dialogs::context();
    wasm_bindgen_futures::spawn_local(async move {
        let fetch = |url: String| async move {
            wasm_bindgen_futures::JsFuture::from(js::fetch_bytes(&url))
                .await
                .map(|b| bytes_of(&b))
                .map_err(|e| format!("Cannot load {url}: {}", e.as_string().unwrap_or_else(|| format!("{e:?}"))))
        };
        let result = async {
            let glue = fetch(format!("pkg/{}", instance_format::web::RUNNER_GLUE)).await?;
            let wasm = fetch(format!("pkg/{}", instance_format::web::RUNNER_WASM)).await?;
            let runner = instance_format::web::WebRunner {
                glue: String::from_utf8(glue).map_err(|e| e.to_string())?,
                wasm,
            };
            let sf_url = web_sys::window()
                .and_then(|w| js_sys::Reflect::get(&w, &"INSTANCE_SOUNDFONT".into()).ok())
                .and_then(|v| v.as_string());
            let soundfont = match sf_url {
                Some(u) if midi => Some(fetch(u).await?),
                _ => None,
            };
            Ok::<_, String>(instance_format::web::page(&title, &runner, &game, soundfont.as_deref()))
        };
        match result.await {
            Ok(html) => js::download(&name, html.as_bytes()),
            Err(e) => push(Incoming::Error(e)),
        }
        if let Some(c) = ctx {
            c.request_repaint();
        }
    });
}

/// Starts the IDE on the canvas with id `canvas_id` (WebGL2 through glow).
#[wasm_bindgen]
pub async fn start_ide(canvas_id: String) -> Result<(), JsValue> {
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas = document
        .get_element_by_id(&canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("no element with id {canvas_id}")))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|cc| Ok(Box::new(crate::app::IdeApp::new(cc, None)))),
        )
        .await
}
