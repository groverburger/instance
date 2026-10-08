//! The error dialog (specs/runtime.md §19): a modal window over the game showing
//! GameMaker's error message, with Ignore and Abort (just Abort for fatal errors).
//! It is drawn with egui in the shared GameMaker style of `instance-ui` — borderless,
//! with the classic raised frame and gradient caption painted by egui — on the main
//! thread, while the game thread waits for the answer.

use crate::egui_window::EguiWindow;
use instance_ui::chrome;
use instance_ui::icons::{self, Glyph};
use instance_ui::theme;
use std::sync::mpsc::Sender;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId, WindowLevel};

/// What the user chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Ignore,
    Abort,
}

/// A request from the game thread: show `msg`, send true on the reply channel to
/// abort the game.
#[derive(Debug)]
pub struct Request {
    pub msg: String,
    pub fatal: bool,
    pub reply: Sender<bool>,
}

/// Logical size of the dialog.
const W: f64 = 540.0;
const H: f64 = 330.0;

pub struct ErrorDialog {
    win: EguiWindow,
    req: Request,
    title: String,
    /// The user chose (the dialog is about to be closed).
    done: bool,
}

impl ErrorDialog {
    /// Opens the dialog for `req`, centred over `owner` (the game window).
    pub fn new(el: &ActiveEventLoop, req: Request, title: &str, owner: Option<&Window>) -> Result<ErrorDialog, String> {
        let mut attrs = Window::default_attributes()
            .with_title(title)
            .with_window_icon(winit::window::Icon::from_rgba(instance_brand::icon_rgba(32), 32, 32).ok())
            .with_decorations(false)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_visible(false)
            .with_inner_size(winit::dpi::LogicalSize::new(W, H));
        #[cfg(windows)]
        if let Some(o) = owner {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            use winit::platform::windows::WindowAttributesExtWindows;
            if let Ok(RawWindowHandle::Win32(h)) = o.window_handle().map(|h| h.as_raw()) {
                attrs = attrs.with_owner_window(h.hwnd.get());
            }
        }
        // centred over the game window (or its monitor when it is not shown yet)
        if let Some(mon) = owner.and_then(|w| w.current_monitor()).or_else(|| el.primary_monitor()) {
            let k = mon.scale_factor();
            let (w, h) = ((W * k) as i32, (H * k) as i32);
            let (cx, cy) = match owner.and_then(|o| o.outer_position().ok().map(|p| (p, o.outer_size()))) {
                Some((p, s)) if owner.is_some_and(|o| o.is_visible() != Some(false)) => (p.x + s.width as i32 / 2, p.y + s.height as i32 / 2),
                _ => (mon.position().x + mon.size().width as i32 / 2, mon.position().y + mon.size().height as i32 / 2),
            };
            let (mp, ms) = (mon.position(), mon.size());
            let x = (cx - w / 2).clamp(mp.x, (mp.x + ms.width as i32 - w).max(mp.x));
            let y = (cy - h / 2).clamp(mp.y, (mp.y + ms.height as i32 - h).max(mp.y));
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(x, y));
        }
        let win = EguiWindow::new(el, attrs)?;
        win.window.set_visible(true);
        win.window.focus_window();
        win.window.request_redraw();
        Ok(ErrorDialog { win, req, title: title.to_string(), done: false })
    }

    pub fn id(&self) -> WindowId {
        self.win.id()
    }

    /// Brings the dialog to the front (the game window was clicked).
    pub fn focus(&self) {
        self.win.window.focus_window();
    }

    /// Answers the game thread; the dialog should be dropped after this.
    fn answer(&mut self, c: Choice) {
        if !self.done {
            self.done = true;
            let abort = c == Choice::Abort || self.req.fatal;
            eprintln!("instance-runner: {}", if abort { "the game was aborted after the error" } else { "the error was ignored" });
            let _ = self.req.reply.send(abort);
        }
    }

    /// Closes the dialog as if `Abort` was pressed (the game window was closed).
    pub fn abort(&mut self) {
        self.answer(Choice::Abort);
    }

    /// Handles an event of the dialog window; returns true when it is finished.
    pub fn event(&mut self, ev: &WindowEvent) -> bool {
        match ev {
            WindowEvent::RedrawRequested => {
                let (msg, fatal, title) = (self.req.msg.clone(), self.req.fatal, self.title.clone());
                let mut out = UiOut::default();
                self.win.redraw(|ui| out = error_ui(ui, &title, &msg, fatal));
                if out.drag {
                    let _ = self.win.window.drag_window();
                }
                if let Some(c) = out.choice {
                    self.answer(c);
                }
            }
            WindowEvent::CloseRequested => {
                self.answer(if self.req.fatal { Choice::Abort } else { Choice::Ignore });
            }
            _ => self.win.event(ev),
        }
        self.done
    }
}

impl Drop for ErrorDialog {
    fn drop(&mut self) {
        // never leave the game thread waiting
        self.answer(Choice::Abort);
    }
}

#[derive(Default)]
pub struct UiOut {
    pub choice: Option<Choice>,
    /// The caption was pressed: move the window with the mouse.
    pub drag: bool,
}

/// The dialog's contents, filling the whole window (frame and caption included).
pub fn error_ui(ui: &mut egui::Ui, title: &str, msg: &str, fatal: bool) -> UiOut {
    let mut out = UiOut::default();
    let full = ui.ctx().content_rect();
    let p = ui.painter().clone();
    chrome::paint_window_frame(&p, full);
    let bar = chrome::caption_rect(full);
    let icon = |p: &egui::Painter, r: egui::Rect| icons::paint(p, r, Glyph::Error, true);
    chrome::paint_caption(&p, bar, title, Some(&icon), true, 24.0);
    let cap = ui.interact(bar.with_max_x(bar.max.x - 22.0), egui::Id::new("caption"), egui::Sense::click_and_drag());
    if cap.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_pressed()) {
        out.drag = true;
    }
    if chrome::caption_button(ui, egui::Id::new("close"), chrome::close_button_rect(bar), chrome::CaptionButton::Close) {
        out.choice = Some(if fatal { Choice::Abort } else { Choice::Ignore });
    }
    let content = egui::Rect::from_min_max(
        egui::pos2(full.min.x + chrome::BORDER + 10.0, bar.max.y + 10.0),
        egui::pos2(full.max.x - chrome::BORDER - 10.0, full.max.y - chrome::BORDER - 10.0),
    );
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(content).layout(egui::Layout::top_down(egui::Align::Min)));
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
        icons::paint(ui.painter(), r, Glyph::Error, true);
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.add_space(1.0);
            ui.label(egui::RichText::new(if fatal { "A fatal error occurred in the game." } else { "An error occurred in the game." }).strong());
            ui.label(if fatal {
                "The game cannot continue. Press Abort to end it."
            } else {
                "Press Ignore to continue the game, or Abort to end it."
            });
        });
    });
    ui.add_space(8.0);
    let buttons_h = 30.0;
    let memo = egui::Rect::from_min_max(egui::pos2(content.min.x + 2.0, ui.cursor().min.y + 2.0), egui::pos2(content.max.x - 2.0, content.max.y - buttons_h - 2.0));
    ui.painter().rect_filled(memo, 0.0, egui::Color32::WHITE);
    chrome::sunken(ui.painter(), memo);
    let mut memo_ui = ui.new_child(egui::UiBuilder::new().max_rect(memo.shrink(1.0)).layout(egui::Layout::top_down(egui::Align::Min)));
    memo_ui.set_clip_rect(memo.shrink(1.0));
    egui::ScrollArea::both().auto_shrink(false).show(&mut memo_ui, |ui| {
        let mut text: &str = msg;
        ui.add(
            egui::TextEdit::multiline(&mut text)
                .font(egui::TextStyle::Monospace)
                .frame(egui::Frame::NONE)
                .text_color(theme::INK)
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(4, 3))
                .id_salt("error-text"),
        );
    });
    let row = egui::Rect::from_min_max(egui::pos2(content.min.x, content.max.y - buttons_h + 6.0), content.max);
    let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::right_to_left(egui::Align::Center)));
    row_ui.spacing_mut().item_spacing.x = 8.0;
    if chrome::dialog_button(&mut row_ui, "Abort", fatal).clicked() {
        out.choice = Some(Choice::Abort);
    }
    if !fatal && chrome::dialog_button(&mut row_ui, "Ignore", true).clicked() {
        out.choice = Some(Choice::Ignore);
    }
    // keyboard: Enter = the default button, Escape = Ignore (Abort when fatal)
    let (enter, esc) = row_ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
    if out.choice.is_none() && (enter || esc) {
        out.choice = Some(if fatal { Choice::Abort } else { Choice::Ignore });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::{by, NodeT, Queryable};

    const MSG: &str = "ERROR in\naction number 3\nof Create Event\nfor object t019:\n\nError in code at line 1:\n   x = 'a' + y_undefined\n            ^\nat position 13: Unknown variable y_undefined";

    fn harness(fatal: bool) -> egui_kittest::Harness<'static, Option<Choice>> {
        let mut first = true;
        egui_kittest::Harness::builder().with_size(egui::vec2(540.0, 330.0)).build_ui_state(
            move |ui, choice: &mut Option<Choice>| {
                if first {
                    instance_ui::theme::apply(ui.ctx());
                    first = false;
                }
                if let Some(c) = error_ui(ui, "Error", MSG, fatal).choice {
                    *choice = Some(c);
                }
            },
            None,
        )
    }

    #[test]
    fn buttons_answer() {
        let mut h = harness(false);
        h.run_steps(2);
        h.get_by_label("Ignore").click();
        h.run_steps(2);
        assert_eq!(*h.state(), Some(Choice::Ignore));
        let mut h = harness(false);
        h.run_steps(2);
        h.get_by_label("Abort").click();
        h.run_steps(2);
        assert_eq!(*h.state(), Some(Choice::Abort));
        // fatal: only Abort
        let mut h = harness(true);
        h.run_steps(2);
        assert!(h.query_by_label("Ignore").is_none());
        h.get_by_label("Abort").click();
        h.run_steps(2);
        assert_eq!(*h.state(), Some(Choice::Abort));
    }

    /// Hovering the buttons does not move anything.
    #[test]
    fn hover_keeps_layout() {
        let mut h = harness(false);
        h.run_steps(3);
        let rects = |h: &egui_kittest::Harness<'_, Option<Choice>>| -> Vec<(String, egui::Rect)> {
            h.query_all(by().recursive(true))
                .filter_map(|n| {
                    let l = n.accesskit_node().label()?;
                    (!l.is_empty()).then(|| (l, n.rect()))
                })
                .collect()
        };
        let before = rects(&h);
        assert!(before.iter().any(|(l, _)| l == "Ignore"), "{before:?}");
        for (label, r) in &before {
            h.hover_at(r.center());
            h.run_steps(2);
            for ((l0, r0), (_, r1)) in before.iter().zip(&rects(&h)) {
                assert!((r0.min - r1.min).length() < 0.01 && (r0.max - r1.max).length() < 0.01, "hovering {label:?} moved {l0:?} from {r0:?} to {r1:?}");
            }
        }
    }
}
