//! The Debug Information window of debug mode (specs/ide.md §20.3): a second,
//! always-on-top winit window drawn with egui (its own OpenGL context on the
//! main thread). It talks to the game thread through a [`Link`]: it queues
//! [`Command`]s and shows the [`Snapshot`]s the game thread publishes between
//! steps (see [`service`]).

use gm_runner::debug::{Command, InstanceRow, Scope, Snapshot, Wants};
use gm_runner::Game;
use glutin::config::{ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, PossiblyCurrentGlContext, Version};
use glutin::display::{Display, DisplayApiPreference, GlDisplay};
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use web_time::Instant;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId, WindowLevel};

/// Shared between the game thread and the debug window.
#[derive(Default)]
pub struct Link {
    /// Window → game.
    pub commands: Vec<Command>,
    pub wants: Wants,
    /// Publish a snapshot now (the wanted lists changed).
    pub refresh: bool,
    /// Game → window.
    pub snap: Snapshot,
    pub fresh: bool,
    pub new_messages: Vec<String>,
}

pub type SharedLink = Arc<Mutex<Link>>;

/// Game-thread side, called once per loop iteration (also while paused):
/// carries out queued commands and publishes a snapshot about ten times a
/// second (at once after commands). Returns true when the window should
/// repaint.
pub fn service(g: &mut Game, vm: &mut gml::Vm, link: &SharedLink, last: &mut Option<Instant>, seen: &mut u64) -> bool {
    let (cmds, wants, refresh) = {
        let mut l = link.lock().unwrap();
        (std::mem::take(&mut l.commands), l.wants.clone(), std::mem::take(&mut l.refresh))
    };
    let acted = !cmds.is_empty();
    for c in cmds {
        g.debug_command(vm, c);
    }
    let due = last.is_none_or(|t| t.elapsed() >= Duration::from_millis(100));
    if !(acted || refresh || due) {
        return false;
    }
    *last = Some(Instant::now());
    let snap = g.debug_snapshot(vm, &wants);
    let msgs = match &g.debug {
        Some(d) => {
            let m = d.messages_since(*seen);
            *seen = d.message_count;
            m
        }
        None => Vec::new(),
    };
    let mut l = link.lock().unwrap();
    l.snap = snap;
    l.fresh = true;
    l.new_messages.extend(msgs);
    true
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Watches,
    Messages,
    Globals,
    Locals,
    Instances,
    Info,
}

enum Dialog {
    /// Add (None) or change (Some(index)) a watch.
    Watch(Option<usize>, String, String),
    Speed(String),
}

/// Window-side state.
struct State {
    tab: Tab,
    snap: Snapshot,
    messages: std::collections::VecDeque<String>,
    selected: Option<usize>,
    dialog: Option<Dialog>,
    exec_code: String,
    exec_scope: String,
    locals_scope: String,
    follow_messages: bool,
    /// Set when the user asked to end the game.
    status: String,
}

pub struct DebugWindow {
    // field order matters for drop: GL objects before the context, the context
    // before the display and the window
    egui: egui_glow::EguiGlow,
    gl: Arc<glow::Context>,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    _display: Display,
    window: Arc<Window>,
    link: SharedLink,
    st: State,
}

const W: u32 = 520;
const H: u32 = 520;

/// Classic light look, matching the IDE.
fn style(ctx: &egui::Context) {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut fonts = egui::FontDefinitions::default();
    #[cfg(windows)]
    {
        let dir = std::env::var_os("WINDIR").map(std::path::PathBuf::from).unwrap_or_else(|| "C:\\Windows".into());
        for (file, fam, key) in [("segoeui.ttf", egui::FontFamily::Proportional, "ui"), ("consola.ttf", egui::FontFamily::Monospace, "mono")] {
            if let Ok(b) = std::fs::read(dir.join("Fonts").join(file)) {
                fonts.font_data.insert(key.into(), Arc::new(egui::FontData::from_owned(b)));
                fonts.families.entry(fam).or_default().insert(0, key.into());
            }
        }
    }
    ctx.set_fonts(fonts);
    let mut v = egui::Visuals::light();
    v.panel_fill = egui::Color32::from_rgb(240, 240, 240);
    v.window_fill = egui::Color32::from_rgb(240, 240, 240);
    v.extreme_bg_color = egui::Color32::WHITE;
    v.faint_bg_color = egui::Color32::from_rgb(245, 245, 245);
    v.selection.bg_fill = egui::Color32::from_rgb(51, 153, 255);
    v.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.noninteractive] {
        w.corner_radius = egui::CornerRadius::same(2);
    }
    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(225, 225, 225);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(173, 173, 173));
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(229, 241, 251);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 120, 215));
    v.window_corner_radius = egui::CornerRadius::same(2);
    v.menu_corner_radius = egui::CornerRadius::same(2);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(6.0, 2.0);
    });
}

impl DebugWindow {
    pub fn new(el: &ActiveEventLoop, link: SharedLink, near: Option<&Window>) -> Result<DebugWindow, String> {
        let mut attrs = Window::default_attributes()
            .with_title("Debug Information")
            .with_window_icon(winit::window::Icon::from_rgba(gm_brand::icon_rgba(32), 32, 32).ok())
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_inner_size(winit::dpi::LogicalSize::new(W, H));
        // to the right of the screen, away from the (centred) game window
        if let Some(mon) = near.and_then(|w| w.current_monitor()).or_else(|| el.primary_monitor()) {
            let (mp, ms) = (mon.position(), mon.size());
            let k = mon.scale_factor();
            let x = mp.x + ms.width as i32 - ((W as f64 + 30.0) * k) as i32;
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(x.max(mp.x), mp.y + (60.0 * k) as i32));
        }
        let window = Arc::new(el.create_window(attrs).map_err(|e| e.to_string())?);
        let (surface, context, display, gl) = create_gl(&window)?;
        let gl = Arc::new(gl);
        let egui = egui_glow::EguiGlow::new(el, gl.clone(), None, Some(window.scale_factor() as f32), false);
        style(&egui.egui_ctx);
        Ok(DebugWindow {
            egui,
            gl,
            surface,
            context,
            _display: display,
            window,
            link,
            st: State {
                tab: Tab::Watches,
                snap: Snapshot::default(),
                messages: Default::default(),
                selected: None,
                dialog: None,
                exec_code: String::new(),
                exec_scope: "global".into(),
                locals_scope: String::new(),
                follow_messages: true,
                status: String::new(),
            },
        })
    }

    pub fn outer_width(&self) -> u32 {
        self.window.outer_size().width
    }

    /// Moves the window to the right of `game` (kept on its monitor).
    pub fn place_beside(&self, game: &Window, gap: u32) {
        let Ok(p) = game.outer_position() else { return };
        let mut x = p.x + game.outer_size().width as i32 + gap as i32;
        let mut y = p.y;
        if let Some(mon) = game.current_monitor() {
            let (mp, ms, os) = (mon.position(), mon.size(), self.window.outer_size());
            x = x.min(mp.x + ms.width as i32 - os.width as i32).max(mp.x);
            // keep it above the task bar (about 48 logical pixels)
            let bottom = mp.y + ms.height as i32 - (48.0 * mon.scale_factor()) as i32;
            y = y.min(bottom - os.height as i32).max(mp.y);
        }
        self.window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    /// New data from the game thread.
    pub fn wake(&self) {
        self.window.request_redraw();
    }

    pub fn event(&mut self, ev: &WindowEvent) {
        match ev {
            WindowEvent::RedrawRequested => {
                self.redraw();
                return;
            }
            WindowEvent::CloseRequested => {
                // closing the debugger ends the game
                self.link.lock().unwrap().commands.push(Command::End);
                return;
            }
            WindowEvent::Resized(s) => {
                self.surface.resize(&self.context, NonZeroU32::new(s.width.max(1)).unwrap(), NonZeroU32::new(s.height.max(1)).unwrap());
            }
            _ => {}
        }
        if self.egui.on_window_event(&self.window, ev).repaint {
            self.window.request_redraw();
        }
    }

    fn redraw(&mut self) {
        {
            let mut l = self.link.lock().unwrap();
            if std::mem::take(&mut l.fresh) {
                self.st.snap = l.snap.clone();
            }
            for m in l.new_messages.drain(..) {
                if self.st.messages.len() >= gm_runner::debug::MAX_MESSAGES {
                    self.st.messages.pop_front();
                }
                self.st.messages.push_back(m);
            }
        }
        if !self.context.is_current() {
            let _ = self.context.make_current(&self.surface);
        }
        let mut cmds = Vec::new();
        let st = &mut self.st;
        self.egui.run(&self.window, |ui| debug_ui(ui, st, &mut cmds));
        let wants = Wants {
            globals: st.tab == Tab::Globals,
            instances: st.tab == Tab::Instances,
            locals: (st.tab == Tab::Locals && !st.locals_scope.trim().is_empty()).then(|| Scope::parse(&st.locals_scope)),
        };
        {
            let mut l = self.link.lock().unwrap();
            if l.wants != wants {
                l.wants = wants;
                l.refresh = true;
            }
            if !cmds.is_empty() {
                l.commands.extend(cmds);
            }
        }
        use glow::HasContext;
        // SAFETY: the context is current on this thread.
        unsafe {
            self.gl.clear_color(0.94, 0.94, 0.94, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        self.egui.paint(&self.window);
        let _ = self.surface.swap_buffers(&self.context);
        if self.egui.egui_ctx.has_requested_repaint() {
            self.window.request_redraw();
        }
    }
}

impl Drop for DebugWindow {
    fn drop(&mut self) {
        if !self.context.is_current() {
            let _ = self.context.make_current(&self.surface);
        }
        self.egui.destroy();
    }
}

/// An OpenGL 3.3 context for the window, current on this (the main) thread,
/// without vsync (the main thread also serves the game window).
fn create_gl(win: &Window) -> Result<(Surface<WindowSurface>, PossiblyCurrentContext, Display, glow::Context), String> {
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
    let rwh = win.window_handle().map_err(|e| e.to_string())?.as_raw();
    let rdh = win.display_handle().map_err(|e| e.to_string())?.as_raw();
    #[cfg(windows)]
    let pref = DisplayApiPreference::Wgl(Some(rwh));
    #[cfg(target_os = "macos")]
    let pref = DisplayApiPreference::Cgl;
    #[cfg(all(unix, not(target_os = "macos")))]
    let pref = DisplayApiPreference::Egl;
    let size = win.inner_size();
    let e = |e: glutin::error::Error| e.to_string();
    // SAFETY: the handles stay valid while the window lives; DebugWindow drops
    // the surface and context before the window.
    unsafe {
        let display = Display::new(rdh, pref).map_err(e)?;
        let tmpl = ConfigTemplateBuilder::new().compatible_with_native_window(rwh).with_alpha_size(8).build();
        let config = display
            .find_configs(tmpl)
            .map_err(e)?
            .reduce(|a, b| if b.num_samples() < a.num_samples() { b } else { a })
            .ok_or("no suitable OpenGL pixel format")?;
        let attrs = ContextAttributesBuilder::new().with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3)))).build(Some(rwh));
        let ctx = display.create_context(&config, &attrs).map_err(e)?;
        let sattrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            rwh,
            NonZeroU32::new(size.width.max(1)).unwrap(),
            NonZeroU32::new(size.height.max(1)).unwrap(),
        );
        let surface = display.create_window_surface(&config, &sattrs).map_err(e)?;
        let context = ctx.make_current(&surface).map_err(e)?;
        let _ = surface.set_swap_interval(&context, SwapInterval::DontWait);
        let gl = glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s));
        Ok((surface, context, display, gl))
    }
}

// ---------------------------------------------------------------------- UI

fn debug_ui(ui: &mut egui::Ui, st: &mut State, cmds: &mut Vec<Command>) {
    let ctx = ui.ctx().clone();
    let chrome = egui::Frame::new().fill(egui::Color32::from_rgb(240, 240, 240)).inner_margin(egui::Margin::symmetric(4, 2));
    egui::Panel::top("dbg-menu").frame(chrome).show(ui, |ui| menu_bar(ui, st, cmds));
    egui::Panel::top("dbg-toolbar").frame(chrome.inner_margin(egui::Margin::symmetric(4, 3))).show(ui, |ui| toolbar(ui, st, cmds));
    egui::Panel::bottom("dbg-status").frame(chrome.inner_margin(egui::Margin::symmetric(6, 3))).show(ui, |ui| status_bar(ui, st));
    egui::Panel::bottom("dbg-exec").frame(chrome.inner_margin(egui::Margin::symmetric(6, 4))).show(ui, |ui| exec_bar(ui, st, cmds));
    egui::Panel::top("dbg-tabs").frame(chrome.inner_margin(egui::Margin { left: 4, right: 4, top: 2, bottom: 0 })).show(ui, |ui| {
        ui.horizontal(|ui| {
            for (t, label) in [
                (Tab::Watches, "Watches"),
                (Tab::Messages, "Messages"),
                (Tab::Globals, "Globals"),
                (Tab::Locals, "Locals"),
                (Tab::Instances, "Instances"),
                (Tab::Info, "Room info"),
            ] {
                if ui.selectable_label(st.tab == t, label).clicked() {
                    st.tab = t;
                }
            }
        });
    });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::WHITE).stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(130, 135, 144))).inner_margin(4))
        .show(ui, |ui| match st.tab {
            Tab::Watches => watches(ui, st, cmds),
            Tab::Messages => messages(ui, st),
            Tab::Globals => pairs(ui, "globals", &st.snap.globals),
            Tab::Locals => locals(ui, st),
            Tab::Instances => instances(ui, st),
            Tab::Info => info(ui, st),
        });
    dialog(&ctx, st, cmds);
}

fn menu_bar(ui: &mut egui::Ui, st: &mut State, cmds: &mut Vec<Command>) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("Run", |ui| {
            for (label, c) in [
                ("Run / Continue", Command::Run),
                ("Pause", Command::Pause),
                ("Step", Command::Step),
                ("Restart game", Command::Restart),
                ("End game", Command::End),
            ] {
                if ui.button(label).clicked() {
                    cmds.push(c);
                    ui.close();
                }
            }
        });
        ui.menu_button("Watch", |ui| {
            if ui.button("Add expression…").clicked() {
                st.dialog = Some(Dialog::Watch(None, String::new(), "global".into()));
                ui.close();
            }
            if ui.add_enabled(st.selected.is_some(), egui::Button::new("Change expression…")).clicked() {
                change_selected(st);
                ui.close();
            }
            if ui.add_enabled(st.selected.is_some(), egui::Button::new("Delete expression")).clicked() {
                delete_selected(st, cmds);
                ui.close();
            }
            if ui.button("Clear all expressions").clicked() {
                cmds.push(Command::ClearWatches);
                st.selected = None;
                ui.close();
            }
            ui.separator();
            if ui.button("Load watches…").clicked() {
                load_watches(st, cmds);
                ui.close();
            }
            if ui.button("Save watches…").clicked() {
                save_watches(st);
                ui.close();
            }
        });
        ui.menu_button("Tools", |ui| {
            for (label, t) in [
                ("Show global variables", Tab::Globals),
                ("Show local variables", Tab::Locals),
                ("Show instances", Tab::Instances),
                ("Show messages", Tab::Messages),
                ("Show room info", Tab::Info),
            ] {
                if ui.button(label).clicked() {
                    st.tab = t;
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Set game speed…").clicked() {
                st.dialog = Some(Dialog::Speed(format!("{}", st.snap.info.room_speed)));
                ui.close();
            }
        });
    });
}

fn change_selected(st: &mut State) {
    if let Some((e, s, _)) = st.selected.and_then(|i| st.snap.watches.get(i)) {
        st.dialog = Some(Dialog::Watch(st.selected, e.clone(), s.clone()));
    }
}

fn delete_selected(st: &mut State, cmds: &mut Vec<Command>) {
    if let Some(i) = st.selected.take() {
        cmds.push(Command::DeleteWatch(i));
    }
}

fn toolbar(ui: &mut egui::Ui, st: &mut State, cmds: &mut Vec<Command>) {
    ui.horizontal(|ui| {
        let paused = st.snap.info.paused;
        let tb = |ui: &mut egui::Ui, text: &str, tip: &str, on: bool| ui.add_enabled(on, egui::Button::new(text).min_size(egui::vec2(28.0, 24.0))).on_hover_text(tip);
        let run = egui::RichText::new("▶").color(egui::Color32::from_rgb(0, 140, 0));
        if ui.add_enabled(paused, egui::Button::new(run).min_size(egui::vec2(28.0, 24.0))).on_hover_text("Run / continue the game").clicked() {
            cmds.push(Command::Run);
        }
        if tb(ui, "⏸", "Pause the game", !paused).clicked() {
            cmds.push(Command::Pause);
        }
        if tb(ui, "⏭", "Step (one step while paused)", true).clicked() {
            cmds.push(Command::Step);
        }
        if tb(ui, "⟲", "Restart the game", true).clicked() {
            cmds.push(Command::Restart);
        }
        let end = egui::RichText::new("■").color(egui::Color32::from_rgb(200, 0, 0));
        if ui.add(egui::Button::new(end).min_size(egui::vec2(28.0, 24.0))).on_hover_text("End the game").clicked() {
            cmds.push(Command::End);
        }
        ui.separator();
        if tb(ui, "+", "Add a watch expression", true).clicked() {
            st.dialog = Some(Dialog::Watch(None, String::new(), "global".into()));
        }
        if tb(ui, "✏", "Change the selected expression", st.selected.is_some()).clicked() {
            change_selected(st);
        }
        if tb(ui, "✖", "Delete the selected expression", st.selected.is_some()).clicked() {
            delete_selected(st, cmds);
        }
        if tb(ui, "🗑", "Clear all expressions", !st.snap.watches.is_empty()).clicked() {
            cmds.push(Command::ClearWatches);
            st.selected = None;
        }
    });
}

fn status_bar(ui: &mut egui::Ui, st: &State) {
    let i = &st.snap.info;
    ui.horizontal(|ui| {
        let state = if i.ended {
            "Ended"
        } else if i.paused {
            "Paused"
        } else {
            "Running"
        };
        ui.label(egui::RichText::new(state).strong());
        ui.separator();
        ui.label(format!("FPS: {}", i.fps.round()));
        ui.separator();
        ui.label(format!("Instances: {}", i.instances));
        ui.separator();
        ui.label(format!("Under mouse: {}", if i.under_mouse < 0 { "none".to_string() } else { i.under_mouse.to_string() }));
        if !st.status.is_empty() {
            ui.separator();
            ui.label(&st.status);
        }
    });
}

fn exec_bar(ui: &mut egui::Ui, st: &mut State, cmds: &mut Vec<Command>) {
    ui.label("Execute code:");
    ui.horizontal(|ui| {
        ui.label("in");
        ui.add(egui::TextEdit::singleline(&mut st.exec_scope).desired_width(70.0)).on_hover_text("global, an instance id, or an object name or index");
        let resp = ui.add(egui::TextEdit::singleline(&mut st.exec_code).font(egui::TextStyle::Monospace).desired_width(ui.available_width() - 70.0).hint_text("GML code"));
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if (ui.button("Execute").clicked() || enter) && !st.exec_code.trim().is_empty() {
            cmds.push(Command::Execute(st.exec_code.clone(), Scope::parse(&st.exec_scope)));
            st.tab = Tab::Messages;
            st.follow_messages = true;
            if enter {
                resp.request_focus();
            }
        }
    });
}

/// A left-aligned cell of width `w`.
fn cell(ui: &mut egui::Ui, w: f32, widget: impl egui::Widget) -> egui::Response {
    ui.allocate_ui_with_layout(egui::vec2(w, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_min_width(w);
        ui.add(widget)
    })
    .inner
}

/// A clickable, left-aligned list cell (highlighted when selected).
fn select_cell(ui: &mut egui::Ui, w: f32, selected: bool, text: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 18.0), egui::Sense::click());
    let v = ui.visuals();
    let (bg, fg) = if selected {
        (Some(v.selection.bg_fill), egui::Color32::WHITE)
    } else if resp.hovered() {
        (Some(v.widgets.hovered.weak_bg_fill), v.text_color())
    } else {
        (None, v.text_color())
    };
    if let Some(bg) = bg {
        ui.painter().rect_filled(rect, 2.0, bg);
    }
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let galley = ui.painter().layout(text.to_string(), font, fg, f32::INFINITY);
    let pos = egui::pos2(rect.left() + 3.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(rect.intersect(ui.clip_rect())).galley(pos, galley, fg);
    resp
}

fn header(ui: &mut egui::Ui, cols: &[(&str, f32)]) {
    ui.horizontal(|ui| {
        for (c, w) in cols {
            cell(ui, *w, egui::Label::new(egui::RichText::new(*c).strong()));
        }
    });
    ui.separator();
}

fn watches(ui: &mut egui::Ui, st: &mut State, cmds: &mut Vec<Command>) {
    let w = ui.available_width();
    let mut open = None;
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        egui::Grid::new("watches").striped(true).num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
            cell(ui, w * 0.42, egui::Label::new(egui::RichText::new("Expression").strong()));
            cell(ui, w * 0.12, egui::Label::new(egui::RichText::new("In").strong()));
            ui.label(egui::RichText::new("Value").strong());
            ui.end_row();
            for (k, (e, s, v)) in st.snap.watches.iter().enumerate() {
                let sel = st.selected == Some(k);
                let r = select_cell(ui, w * 0.42, sel, e);
                if r.clicked() {
                    st.selected = Some(k);
                }
                if r.double_clicked() {
                    open = Some(k);
                }
                cell(ui, w * 0.12, egui::Label::new(s).truncate());
                let (text, color) = match v {
                    Ok(x) => (x.clone(), egui::Color32::BLACK),
                    Err(err) => (format!("error: {}", err), egui::Color32::from_rgb(190, 0, 0)),
                };
                ui.add(egui::Label::new(egui::RichText::new(text).monospace().color(color)).truncate());
                ui.end_row();
            }
        });
        if st.snap.watches.is_empty() {
            ui.add_space(6.0);
            ui.weak("No watch expressions. Use Watch > Add expression (or +) to add one.");
        }
    });
    if let Some(k) = open {
        st.selected = Some(k);
        change_selected(st);
    }
    if st.selected.is_some_and(|i| i >= st.snap.watches.len()) {
        st.selected = None;
    }
    if ui.input(|i| i.key_pressed(egui::Key::Delete)) && !ui.ctx().egui_wants_keyboard_input() {
        delete_selected(st, cmds);
    }
}

fn messages(ui: &mut egui::Ui, st: &mut State) {
    ui.horizontal(|ui| {
        ui.checkbox(&mut st.follow_messages, "Scroll to new messages");
        if ui.button("Clear").clicked() {
            st.messages.clear();
        }
        ui.weak(format!("{} messages", st.messages.len()));
    });
    ui.separator();
    let rows = st.messages.len();
    let h = ui.text_style_height(&egui::TextStyle::Monospace);
    egui::ScrollArea::both().auto_shrink(false).stick_to_bottom(st.follow_messages).show_rows(ui, h, rows, |ui, range| {
        for m in st.messages.range(range) {
            ui.label(egui::RichText::new(m).monospace());
        }
    });
}

fn pairs(ui: &mut egui::Ui, id: &str, list: &[(String, String)]) {
    let w = ui.available_width();
    header(ui, &[("Variable", w * 0.4), ("Value", w * 0.55)]);
    egui::ScrollArea::vertical().id_salt(id).auto_shrink(false).show(ui, |ui| {
        egui::Grid::new(id).striped(true).num_columns(2).min_col_width(w * 0.4).show(ui, |ui| {
            for (k, v) in list {
                ui.label(egui::RichText::new(k).monospace());
                ui.add(egui::Label::new(egui::RichText::new(v).monospace()).truncate());
                ui.end_row();
            }
        });
    });
}

fn locals(ui: &mut egui::Ui, st: &mut State) {
    ui.horizontal(|ui| {
        ui.label("Instance id or object name:");
        ui.add(egui::TextEdit::singleline(&mut st.locals_scope).desired_width(140.0));
    });
    ui.separator();
    match &st.snap.locals {
        _ if st.locals_scope.trim().is_empty() => {
            ui.weak("Enter an instance id or an object name (or pick an instance in the Instances tab).");
        }
        Some(Ok((id, vars))) => {
            ui.label(format!("Instance {}", id));
            pairs(ui, "locals", vars);
        }
        Some(Err(e)) => {
            ui.colored_label(egui::Color32::from_rgb(190, 0, 0), e);
        }
        None => {}
    }
}

fn instances(ui: &mut egui::Ui, st: &mut State) {
    let list: &[InstanceRow] = &st.snap.instances;
    ui.weak(format!("{} instances — click one to see its local variables", list.len()));
    let w = ui.available_width();
    let cols = [("Id", 0.14), ("Object", 0.3), ("x", 0.12), ("y", 0.12), ("Depth", 0.1), ("Sprite", 0.2)];
    let cols: Vec<(&str, f32)> = cols.iter().map(|(c, f)| (*c, w * f - 6.0)).collect();
    header(ui, &cols);
    let mut pick = None;
    let h = 18.0;
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, h, list.len(), |ui, range| {
        for r in &list[range] {
            let resp = ui.horizontal(|ui| {
                let cells = [
                    r.id.to_string(),
                    r.object.clone(),
                    format!("{:.1}", r.x),
                    format!("{:.1}", r.y),
                    format!("{}", r.depth),
                    r.sprite.clone(),
                ];
                let mut resp = ui.add_sized([cols[0].1, h], egui::Button::selectable(false, cells[0].clone()));
                for (k, c) in cells.iter().enumerate().skip(1) {
                    resp |= cell(ui, cols[k].1, egui::Label::new(c).truncate().sense(egui::Sense::click()));
                }
                resp
            });
            if resp.inner.clicked() {
                pick = Some(r.id);
            }
        }
    });
    if let Some(id) = pick {
        st.locals_scope = id.to_string();
        st.tab = Tab::Locals;
    }
}

fn info(ui: &mut egui::Ui, st: &State) {
    let i = &st.snap.info;
    egui::Grid::new("info").num_columns(2).striped(true).spacing([24.0, 6.0]).show(ui, |ui| {
        let rows = [
            ("Room", format!("{} ({})", i.room, i.room_index)),
            ("Room speed", format!("{}", i.room_speed)),
            ("FPS", format!("{}", i.fps.round())),
            ("Instances", i.instances.to_string()),
            ("Step", i.step.to_string()),
            ("Mouse (room)", format!("{:.0}, {:.0}", i.mouse.0, i.mouse.1)),
            ("Instance under mouse", if i.under_mouse < 0 { "none".into() } else { i.under_mouse.to_string() }),
            ("State", if i.ended { "ended" } else if i.paused { "paused" } else { "running" }.into()),
        ];
        for (k, v) in rows {
            ui.label(egui::RichText::new(k).strong());
            ui.label(v);
            ui.end_row();
        }
    });
}

fn dialog(ctx: &egui::Context, st: &mut State, cmds: &mut Vec<Command>) {
    let Some(d) = &mut st.dialog else { return };
    let mut close = false;
    match d {
        Dialog::Watch(idx, expr, scope) => {
            let title = if idx.is_some() { "Change Watch Expression" } else { "Add Watch Expression" };
            egui::Modal::new(egui::Id::new("watch-dlg")).show(ctx, |ui| {
                ui.set_width(360.0);
                ui.heading(title);
                ui.label("Expression:");
                let r = ui.add(egui::TextEdit::singleline(expr).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
                if !r.has_focus() && expr.is_empty() {
                    r.request_focus();
                }
                ui.label("Evaluate in (global, an instance id, or an object name):");
                ui.add(egui::TextEdit::singleline(scope).desired_width(f32::INFINITY));
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() || enter {
                        if !expr.trim().is_empty() {
                            let s = Scope::parse(scope);
                            cmds.push(match idx {
                                Some(i) => Command::ChangeWatch(*i, expr.clone(), s),
                                None => Command::AddWatch(expr.clone(), s),
                            });
                        }
                        close = true;
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        close = true;
                    }
                });
            });
        }
        Dialog::Speed(text) => {
            egui::Modal::new(egui::Id::new("speed-dlg")).show(ctx, |ui| {
                ui.heading("Set Game Speed");
                ui.horizontal(|ui| {
                    ui.label("Room speed (steps per second):");
                    ui.add(egui::TextEdit::singleline(text).desired_width(60.0));
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if let Ok(v) = text.trim().parse::<f64>() {
                            cmds.push(Command::SetSpeed(v));
                        }
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        }
    }
    if close {
        st.dialog = None;
    }
}

/// Watch files: one watch per line, `scope<TAB>expression`.
fn load_watches(st: &mut State, cmds: &mut Vec<Command>) {
    let Some(p) = rfd::FileDialog::new().add_filter("Watch files", &["txt"]).pick_file() else { return };
    match std::fs::read_to_string(&p) {
        Ok(t) => {
            for line in t.lines().filter(|l| !l.trim().is_empty()) {
                let (s, e) = line.split_once('\t').unwrap_or(("global", line));
                cmds.push(Command::AddWatch(e.to_string(), Scope::parse(s)));
            }
        }
        Err(e) => st.status = format!("cannot read {}: {}", p.display(), e),
    }
}

fn save_watches(st: &mut State) {
    let Some(p) = rfd::FileDialog::new().add_filter("Watch files", &["txt"]).set_file_name("watches.txt").save_file() else { return };
    let t: String = st.snap.watches.iter().map(|(e, s, _)| format!("{}\t{}\n", s, e)).collect();
    if let Err(e) = std::fs::write(&p, t) {
        st.status = format!("cannot write {}: {}", p.display(), e);
    }
}
