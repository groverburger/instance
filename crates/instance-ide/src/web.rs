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
        /// Hides the page's "Loading" / "Opening" notice (`web/ide.js`): called once a
        /// project is open, or a question about it is shown.
        #[wasm_bindgen(js_namespace = instanceIde, js_name = loaded)]
        pub fn loaded();
        /// Shows the welcome screen with the example projects (`web/ide.js`).
        #[wasm_bindgen(js_namespace = instanceIde, js_name = showWelcome)]
        pub fn show_welcome();
    }
}

/// A project file given as bytes, with the resource whose editor opens after it
/// is loaded (`?open=NAME`).
pub struct Linked {
    pub name: String,
    pub bytes: Vec<u8>,
    pub open: Option<String>,
}

/// The previous session's autosave.
pub struct Autosave {
    pub name: String,
    pub bytes: Vec<u8>,
    /// When it was saved, in ms since the epoch.
    pub time: f64,
}

/// Something that arrived asynchronously for the app.
pub enum Incoming {
    /// A project to open, e.g. from `?project=URL`.
    Open(Linked),
    /// The previous session's autosave, with the `?project=` project when there was
    /// one: the app asks which to open, so following a link never silently replaces
    /// work kept only in the browser.
    Autosave(Autosave, Option<Linked>),
    /// A file dropped onto the page (name, bytes).
    Dropped(String, Vec<u8>),
    /// File > New (the welcome screen's "Start an empty project").
    New,
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

/// The file name at the end of a URL.
fn url_file_name(url: &str) -> String {
    url.split(['?', '#'])
        .next()
        .and_then(|u| u.rsplit('/').next())
        .filter(|n| !n.is_empty())
        .unwrap_or("project.gm81")
        .to_string()
}

/// Fetches the project at `url`.
async fn fetch_linked(url: &str, open: Option<String>) -> Result<Linked, String> {
    match wasm_bindgen_futures::JsFuture::from(js::fetch_bytes(url)).await {
        Ok(b) => Ok(Linked {
            name: url_file_name(url),
            bytes: bytes_of(&b),
            open,
        }),
        Err(e) => Err(format!(
            "Cannot load {url}: {}",
            e.as_string().unwrap_or_else(|| format!("{e:?}"))
        )),
    }
}

/// The autosave of the previous session, if there is one.
async fn autosave() -> Option<Autosave> {
    let v = wasm_bindgen_futures::JsFuture::from(js::load_autosave()).await.ok()?;
    if v.is_null() || v.is_undefined() {
        return None;
    }
    let get = |k: &str| js_sys::Reflect::get(&v, &k.into()).ok();
    let bytes = get("bytes").map(|b| bytes_of(&b)).unwrap_or_default();
    (!bytes.is_empty()).then(|| Autosave {
        name: get("name").and_then(|n| n.as_string()).unwrap_or_default(),
        time: get("time").and_then(|t| t.as_f64()).unwrap_or(0.0),
        bytes,
    })
}

/// Marks the startup work as done; returns whether it had been already.
fn mark_started() -> bool {
    STARTED.with(|s| std::mem::replace(&mut *s.borrow_mut(), true))
}

fn repaint() {
    if let Some(c) = crate::dialogs::context() {
        c.request_repaint();
    }
}

/// Kicks off the startup work once: `?project=URL` is fetched and opened (and
/// `?open=NAME` opens that resource's editor), and a previous autosave is offered.
pub fn startup(_ctx: &eframe::egui::Context) {
    if mark_started() {
        return;
    }
    wasm_bindgen_futures::spawn_local(async move {
        let open = js::query_param("open").filter(|n| !n.is_empty());
        let linked = match js::query_param("project").filter(|u| !u.is_empty()) {
            Some(url) => match fetch_linked(&url, open).await {
                Ok(l) => Some(l),
                Err(e) => {
                    push(Incoming::Error(e));
                    None
                }
            },
            None => None,
        };
        match (autosave().await, linked) {
            (Some(a), l) => push(Incoming::Autosave(a, l)),
            (None, Some(l)) => push(Incoming::Open(l)),
            // nothing to open: the IDE is ready (otherwise `IdeApp::web_frame` says so
            // once it has opened the project or asked about it)
            (None, None) => js::loaded(),
        }
        repaint();
    });
}

/// Opens the project at `url` (relative to the page), then the editor of the
/// resource named `open` (if given); called by the welcome screen of `web/ide.js`.
/// Before the IDE's first frame this replaces the startup work (`?project=`, the
/// autosave offer); later, unsaved changes are asked about first.
#[wasm_bindgen]
pub fn open_project_url(url: String, open: Option<String>) {
    mark_started();
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_linked(&url, open.filter(|n| !n.is_empty())).await {
            Ok(l) => push(Incoming::Open(l)),
            Err(e) => push(Incoming::Error(e)),
        }
        repaint();
    });
}

/// Starts an empty project (the welcome screen's "Start an empty project"); unsaved
/// changes are asked about first.
#[wasm_bindgen]
pub fn new_project() {
    mark_started();
    push(Incoming::New);
    repaint();
}

/// Opens a project file given as bytes (the welcome screen's "Open a file").
#[wasm_bindgen]
pub fn open_project_bytes(name: String, bytes: Vec<u8>) {
    mark_started();
    push(Incoming::Open(Linked { name, bytes, open: None }));
    repaint();
}

thread_local! {
    /// The SoundFont for MIDI previews: 0 not requested, 1 loading, 2 done (or failed).
    static SOUNDFONT: RefCell<u8> = const { RefCell::new(0) };
}

/// Whether the SoundFont for MIDI previews has been loaded (or could not be).
pub fn soundfont_ready() -> bool {
    SOUNDFONT.with(|s| *s.borrow() == 2)
}

/// Starts loading the SoundFont named by `config.js` for MIDI previews (once).
pub fn load_soundfont() {
    let requested = SOUNDFONT.with(|s| {
        let mut s = s.borrow_mut();
        let was = *s;
        *s = was.max(1);
        was
    });
    if requested > 0 {
        return;
    }
    let ctx = crate::dialogs::context();
    wasm_bindgen_futures::spawn_local(async move {
        let url = web_sys::window()
            .and_then(|w| js_sys::Reflect::get(&w, &"INSTANCE_SOUNDFONT".into()).ok())
            .and_then(|v| v.as_string());
        match url {
            Some(u) => match wasm_bindgen_futures::JsFuture::from(js::fetch_bytes(&u)).await {
                Ok(b) => instance_audio::set_soundfont_bytes(bytes_of(&b)),
                Err(e) => push(Incoming::Error(format!(
                    "Cannot load the SoundFont {u}: {}; MIDI sounds are silent",
                    e.as_string().unwrap_or_else(|| format!("{e:?}"))
                ))),
            },
            None => push(Incoming::Error(
                "No SoundFont is set up for this site; MIDI sounds are silent.".into(),
            )),
        }
        SOUNDFONT.with(|s| *s.borrow_mut() = 2);
        if let Some(c) = ctx {
            c.request_repaint();
        }
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
