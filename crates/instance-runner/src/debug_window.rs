//! The Debug Information window of debug mode (specs/ide.md §20.3): a second,
//! always-on-top winit window drawn with egui (its own OpenGL context on the
//! main thread). It talks to the game thread through a [`Link`]: it queues
//! [`Command`]s and shows the [`Snapshot`]s the game thread publishes between
//! steps (see [`service`]).

use crate::egui_window::EguiWindow;
use instance_runner::debug::{Command, InstanceRow, Scope, Snapshot, Wants};
use instance_runner::Game;
use instance_ui::icons::{tool_button, tool_separator, Glyph};
use instance_ui::theme;
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
    win: EguiWindow,
    link: SharedLink,
    st: State,
}

const W: u32 = 560;
const H: u32 = 540;

impl State {
    fn new() -> State {
        State {
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
        }
    }
}

impl DebugWindow {
    pub fn new(el: &ActiveEventLoop, link: SharedLink, near: Option<&Window>) -> Result<DebugWindow, String> {
        let mut attrs = Window::default_attributes()
            .with_title("Debug Information")
            .with_window_icon(winit::window::Icon::from_rgba(instance_brand::icon_rgba(32), 32, 32).ok())
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_inner_size(winit::dpi::LogicalSize::new(W, H));
        // to the right of the screen, away from the (centred) game window
        if let Some(mon) = near.and_then(|w| w.current_monitor()).or_else(|| el.primary_monitor()) {
            let (mp, ms) = (mon.position(), mon.size());
            let k = mon.scale_factor();
            let x = mp.x + ms.width as i32 - ((W as f64 + 30.0) * k) as i32;
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(x.max(mp.x), mp.y + (60.0 * k) as i32));
        }
        let win = EguiWindow::new(el, attrs)?;
        Ok(DebugWindow { win, link, st: State::new() })
    }

    pub fn outer_width(&self) -> u32 {
        self.win.window.outer_size().width
    }

    /// Moves the window to the right of `game` (kept on its monitor).
    pub fn place_beside(&self, game: &Window, gap: u32) {
        let Ok(p) = game.outer_position() else { return };
        let mut x = p.x + game.outer_size().width as i32 + gap as i32;
        let mut y = p.y;
        if let Some(mon) = game.current_monitor() {
            let (mp, ms, os) = (mon.position(), mon.size(), self.win.window.outer_size());
            x = x.min(mp.x + ms.width as i32 - os.width as i32).max(mp.x);
            // keep it above the task bar (about 48 logical pixels)
            let bottom = mp.y + ms.height as i32 - (48.0 * mon.scale_factor()) as i32;
            y = y.min(bottom - os.height as i32).max(mp.y);
        }
        self.win.window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
    }

    pub fn id(&self) -> WindowId {
        self.win.id()
    }

    /// New data from the game thread.
    pub fn wake(&self) {
        self.win.window.request_redraw();
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
            _ => {}
        }
        self.win.event(ev);
    }

    fn redraw(&mut self) {
        {
            let mut l = self.link.lock().unwrap();
            if std::mem::take(&mut l.fresh) {
                self.st.snap = l.snap.clone();
            }
            for m in l.new_messages.drain(..) {
                if self.st.messages.len() >= instance_runner::debug::MAX_MESSAGES {
                    self.st.messages.pop_front();
                }
                self.st.messages.push_back(m);
            }
        }
        let mut cmds = Vec::new();
        let st = &mut self.st;
        self.win.redraw(|ui| debug_ui(ui, st, &mut cmds));
        let wants = Wants {
            globals: st.tab == Tab::Globals,
            instances: st.tab == Tab::Instances,
            locals: (st.tab == Tab::Locals && !st.locals_scope.trim().is_empty()).then(|| Scope::parse(&st.locals_scope)),
        };
        let mut l = self.link.lock().unwrap();
        if l.wants != wants {
            l.wants = wants;
            l.refresh = true;
        }
        if !cmds.is_empty() {
            l.commands.extend(cmds);
        }
    }
}

// ---------------------------------------------------------------------- UI

fn debug_ui(ui: &mut egui::Ui, st: &mut State, cmds: &mut Vec<Command>) {
    let ctx = ui.ctx().clone();
    // the same chrome as the IDE's main window: grey panels, an etched line under
    // the menu bar, a classic tab strip and a white, framed list area
    let chrome = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(2, 1));
    egui::Panel::top("dbg-menu").frame(chrome).show(ui, |ui| menu_bar(ui, st, cmds));
    egui::Panel::top("dbg-toolbar")
        .frame(chrome.inner_margin(egui::Margin { left: 4, right: 4, top: 2, bottom: 3 }))
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().hline(r.x_range(), r.top() - 1.5, egui::Stroke::new(1.0, theme::SHADOW));
            ui.painter().hline(r.x_range(), r.top() - 0.5, egui::Stroke::new(1.0, egui::Color32::WHITE));
            toolbar(ui, st, cmds)
        });
    egui::Panel::bottom("dbg-status").frame(chrome.inner_margin(egui::Margin::symmetric(2, 2))).show(ui, |ui| status_bar(ui, st));
    egui::Panel::bottom("dbg-exec").frame(chrome.inner_margin(egui::Margin::symmetric(6, 4))).show(ui, |ui| exec_bar(ui, st, cmds));
    egui::Panel::top("dbg-tabs").frame(chrome.inner_margin(egui::Margin { left: 6, right: 6, top: 4, bottom: 0 })).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (t, label) in [
                (Tab::Watches, "Watches"),
                (Tab::Messages, "Messages"),
                (Tab::Globals, "Globals"),
                (Tab::Locals, "Locals"),
                (Tab::Instances, "Instances"),
                (Tab::Info, "Room info"),
            ] {
                if tab(ui, st.tab == t, label).clicked() {
                    st.tab = t;
                }
            }
        });
    });
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, theme::LIST_FRAME))
                .inner_margin(4)
                .outer_margin(egui::Margin { left: 6, right: 6, top: 0, bottom: 2 }),
        )
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

/// A classic tab of the tab strip: fixed size from its text; the selected tab is
/// white, taller and joins the list area below. Hover only changes the fill.
fn tab(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, theme::INK);
    let size = egui::vec2(galley.size().x + 16.0, 22.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label));
    let p = ui.painter();
    let r = if selected { rect } else { rect.with_min_y(rect.min.y + 2.0) };
    let fill = if selected {
        egui::Color32::WHITE
    } else if resp.hovered() {
        theme::HOVER_FILL
    } else {
        egui::Color32::from_gray(228)
    };
    // extend 1px down so the selected tab covers the list frame below it
    let body = if selected { r.with_max_y(r.max.y + 1.0) } else { r };
    p.rect_filled(body, 0.0, fill);
    let edge = egui::Stroke::new(1.0, theme::LIST_FRAME);
    let x0 = r.min.x + 0.5;
    let x1 = r.max.x - 0.5;
    let y0 = r.min.y + 0.5;
    p.line_segment([egui::pos2(x0, body.max.y), egui::pos2(x0, y0)], edge);
    p.line_segment([egui::pos2(x0, y0), egui::pos2(x1, y0)], edge);
    p.line_segment([egui::pos2(x1, y0), egui::pos2(x1, body.max.y)], edge);
    let text_pos = egui::pos2(rect.center().x - galley.size().x / 2.0, r.center().y - galley.size().y / 2.0);
    p.galley(text_pos, galley, theme::INK);
    resp
}

fn toolbar(ui: &mut egui::Ui, st: &mut State, cmds: &mut Vec<Command>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        let paused = st.snap.info.paused;
        if tool_button(ui, Glyph::Play, "Run / continue the game", paused).clicked() {
            cmds.push(Command::Run);
        }
        if tool_button(ui, Glyph::Pause, "Pause the game", !paused).clicked() {
            cmds.push(Command::Pause);
        }
        if tool_button(ui, Glyph::Step, "Step (one step while paused)", true).clicked() {
            cmds.push(Command::Step);
        }
        if tool_button(ui, Glyph::Restart, "Restart the game", true).clicked() {
            cmds.push(Command::Restart);
        }
        if tool_button(ui, Glyph::Stop, "End the game", true).clicked() {
            cmds.push(Command::End);
        }
        tool_separator(ui);
        if tool_button(ui, Glyph::Add, "Add a watch expression", true).clicked() {
            st.dialog = Some(Dialog::Watch(None, String::new(), "global".into()));
        }
        if tool_button(ui, Glyph::Edit, "Change the selected expression", st.selected.is_some()).clicked() {
            change_selected(st);
        }
        if tool_button(ui, Glyph::Delete, "Delete the selected expression", st.selected.is_some()).clicked() {
            delete_selected(st, cmds);
        }
        if tool_button(ui, Glyph::Clear, "Clear all expressions", !st.snap.watches.is_empty()).clicked() {
            cmds.push(Command::ClearWatches);
            st.selected = None;
        }
    });
}

fn status_bar(ui: &mut egui::Ui, st: &State) {
    let i = &st.snap.info;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let state = if i.ended {
            "Ended"
        } else if i.paused {
            "Paused"
        } else {
            "Running"
        };
        let cell = instance_ui::chrome::status_cell;
        cell(ui, 70.0, state);
        cell(ui, 70.0, &format!("FPS: {}", i.fps.round()));
        cell(ui, 100.0, &format!("Instances: {}", i.instances));
        let rest = if st.status.is_empty() {
            format!("Under mouse: {}", if i.under_mouse < 0 { "none".to_string() } else { i.under_mouse.to_string() })
        } else {
            st.status.clone()
        };
        let w = ui.available_width().max(40.0);
        cell(ui, w, &rest);
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
    let close;
    match d {
        Dialog::Watch(idx, expr, scope) => {
            let title = if idx.is_some() { "Change Watch Expression" } else { "Add Watch Expression" };
            let d = instance_ui::chrome::dialog(ctx, egui::Id::new("watch-dlg"), title, |ui| {
                ui.set_width(360.0);
                ui.label("Expression:");
                let r = ui.add(egui::TextEdit::singleline(expr).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
                if !r.has_focus() && expr.is_empty() {
                    r.request_focus();
                }
                ui.label("Evaluate in (global, an instance id, or an object name):");
                ui.add(egui::TextEdit::singleline(scope).desired_width(f32::INFINITY));
                ui.add_space(6.0);
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let r = ok_cancel(ui);
                let ok = r == Some(true) || (r.is_none() && enter);
                if ok && !expr.trim().is_empty() {
                    let s = Scope::parse(scope);
                    cmds.push(match idx {
                        Some(i) => Command::ChangeWatch(*i, expr.clone(), s),
                        None => Command::AddWatch(expr.clone(), s),
                    });
                }
                ok || r == Some(false)
            });
            close = d.inner || d.close;
        }
        Dialog::Speed(text) => {
            let d = instance_ui::chrome::dialog(ctx, egui::Id::new("speed-dlg"), "Set Game Speed", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Room speed (steps per second):");
                    ui.add(egui::TextEdit::singleline(text).desired_width(60.0));
                });
                ui.add_space(6.0);
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let r = ok_cancel(ui);
                if r == Some(true) || (r.is_none() && enter) {
                    if let Ok(v) = text.trim().parse::<f64>() {
                        cmds.push(Command::SetSpeed(v));
                    }
                }
                r.is_some() || enter
            });
            close = d.inner || d.close;
        }
    }
    if close {
        st.dialog = None;
    }
}

/// Right-aligned OK / Cancel buttons: Some(true) = OK, Some(false) = Cancel.
fn ok_cancel(ui: &mut egui::Ui) -> Option<bool> {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let cancel = instance_ui::chrome::dialog_button(ui, "Cancel", false).clicked();
        let ok = instance_ui::chrome::dialog_button(ui, "OK", true).clicked();
        if ok {
            Some(true)
        } else if cancel {
            Some(false)
        } else {
            None
        }
    })
    .inner
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

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::{by, NodeT, Queryable};

    fn state(tab: Tab) -> State {
        let mut st = State::new();
        st.tab = tab;
        st.snap.watches = vec![
            ("x".into(), "global".into(), Ok("12".into())),
            ("score * 2".into(), "100001".into(), Err("unknown variable".into())),
        ];
        st.snap.globals = vec![("score".into(), "10".into()), ("lives".into(), "3".into())];
        st.snap.instances = (0..4)
            .map(|k| InstanceRow { id: 100001 + k, object: format!("obj{k}"), x: 1.0, y: 2.0, depth: 0.0, sprite: "spr".into() })
            .collect();
        st.messages.extend(["hello".to_string(), "world".to_string()]);
        st.snap.info.room = "room0".into();
        st
    }

    fn harness(tab: Tab) -> egui_kittest::Harness<'static, State> {
        let mut first = true;
        egui_kittest::Harness::builder().with_size(egui::vec2(W as f32, H as f32)).build_ui_state(
            move |ui, st: &mut State| {
                if first {
                    instance_ui::theme::apply(ui.ctx());
                    first = false;
                }
                let mut cmds = Vec::new();
                debug_ui(ui, st, &mut cmds);
            },
            state(tab),
        )
    }

    fn rects(h: &egui_kittest::Harness<'_, State>) -> Vec<(String, egui::Rect)> {
        h.query_all(by().recursive(true))
            .filter_map(|n| {
                let l = n.accesskit_node().label()?;
                (!l.is_empty()).then(|| (l, n.rect()))
            })
            .collect()
    }

    /// Hovering any widget of the debug window (toolbar buttons, tabs, list rows,
    /// buttons) never changes the size or position of any widget.
    #[test]
    fn hover_does_not_move_widgets() {
        for tab in [Tab::Watches, Tab::Messages, Tab::Globals, Tab::Instances, Tab::Info] {
            let mut h = harness(tab);
            h.run_steps(3);
            h.hover_at(egui::pos2(W as f32 - 3.0, H as f32 - 3.0));
            h.run_steps(2);
            let before = rects(&h);
            assert!(before.len() >= 15, "{before:?}");
            for (label, r) in &before {
                h.hover_at(r.center());
                h.run_steps(2);
                let after = rects(&h);
                assert_eq!(after.len(), before.len(), "hovering {label:?}");
                for ((l0, r0), (_, r1)) in before.iter().zip(&after) {
                    assert!(
                        (r0.min - r1.min).length() < 0.01 && (r0.max - r1.max).length() < 0.01,
                        "hovering {label:?} moved {l0:?} from {r0:?} to {r1:?}"
                    );
                }
            }
        }
    }

    /// The toolbar's run button continues a paused game; tabs switch.
    #[test]
    fn toolbar_and_tabs() {
        let mut h = harness(Tab::Watches);
        h.state_mut().snap.info.paused = true;
        h.run_steps(2);
        h.get_by_label("Globals").click();
        h.run_steps(2);
        assert!(h.state().tab == Tab::Globals);
        h.get_by_label("Add a watch expression").click();
        h.run_steps(2);
        assert!(matches!(h.state().dialog, Some(Dialog::Watch(None, _, _))));
        h.get_by_label("Cancel").click();
        h.run_steps(2);
        assert!(h.state().dialog.is_none());
    }
}
