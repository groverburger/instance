//! The Output Log: what a game run from the IDE writes to stdout / stderr (runner
//! info, `show_debug_message` output, error messages). The runner is started with
//! both streams piped ([`attach`]); reader threads append the lines to a shared
//! [`OutputLog`], which keeps the last [`MAX_LINES`] lines of the current run and
//! is cleared when the next run starts. The IDE shows it in an MDI child window
//! ([`window`]) opened from Window > Output Log, the toolbar or the status bar,
//! which also signal errors that arrived since the log was last looked at.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, Id, Vec2};
use gm_ui::icons::Glyph;

/// Lines kept per run (older lines are dropped).
pub const MAX_LINES: usize = 20_000;

/// What a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// Runner information and anything else the game prints.
    Info,
    /// `show_debug_message` output.
    Debug,
    /// Part of an error message.
    Error,
    /// Written by the IDE (run started / finished).
    Ide,
}

#[derive(Clone, Debug)]
pub struct LogLine {
    pub kind: LineKind,
    pub text: String,
}

/// The lines of the current run.
#[derive(Default)]
pub struct OutputLog {
    lines: VecDeque<LogLine>,
    /// Lines dropped from the front (over [`MAX_LINES`]).
    dropped: usize,
    /// Error messages this run.
    errors: usize,
    /// Error messages not yet seen (the log window was not showing).
    unseen_errors: usize,
    /// The following stderr lines continue an error message.
    in_error: bool,
    /// Bumped on every change (repaint / scroll decisions).
    pub generation: u64,
}

/// Shared between the IDE and the pipe reader threads.
pub type SharedLog = Arc<Mutex<OutputLog>>;

/// Whether a runner line starts an error message (GameMaker's message headers and
/// the runner's own failures).
pub fn is_error_start(line: &str) -> bool {
    let l = line.trim_start();
    l.starts_with("ERROR")
        || l.starts_with("FATAL")
        || l.starts_with("COMPILATION ERROR")
        || l.starts_with("Error in code")
        || l.starts_with("thread '") && l.contains("panicked")
        || l.starts_with("gm-runner: ") && {
            let rest = &l["gm-runner: ".len()..];
            rest.starts_with("error")
                || rest.starts_with("cannot")
                || rest.starts_with("unknown option")
        }
}

/// Lines that end an error message block (the runner's own status lines and
/// debug messages).
fn ends_error(line: &str) -> bool {
    line.starts_with("gm-runner: ") || line.starts_with("[debug] ") || line.starts_with("[message] ")
}

impl OutputLog {
    pub fn lines(&self) -> &VecDeque<LogLine> {
        &self.lines
    }
    pub fn len(&self) -> usize {
        self.lines.len()
    }
    pub fn dropped(&self) -> usize {
        self.dropped
    }
    pub fn errors(&self) -> usize {
        self.errors
    }
    pub fn unseen_errors(&self) -> usize {
        self.unseen_errors
    }
    pub fn mark_seen(&mut self) {
        if self.unseen_errors != 0 {
            self.unseen_errors = 0;
            self.generation += 1;
        }
    }

    /// Starts a new run: forgets everything.
    pub fn clear(&mut self) {
        let g = self.generation;
        *self = OutputLog::default();
        self.generation = g + 1;
    }

    fn push(&mut self, kind: LineKind, text: String) {
        if self.lines.len() >= MAX_LINES {
            self.lines.pop_front();
            self.dropped += 1;
        }
        self.lines.push_back(LogLine { kind, text });
        self.generation += 1;
    }

    /// A line written by the IDE.
    pub fn ide(&mut self, text: impl Into<String>) {
        self.in_error = false;
        self.push(LineKind::Ide, text.into());
    }

    /// A line the runner wrote (`stderr`: on its error stream).
    pub fn runner_line(&mut self, text: &str, stderr: bool) {
        let text = text.trim_end_matches(['\r', '\n']).replace('\r', "\n");
        // a CR inside a line (GameMaker's message bodies) splits it
        for (i, part) in text.split('\n').enumerate() {
            let kind = if is_error_start(part) {
                // (a header inside a message, e.g. "Error in code at line", is not
                // a new error)
                let header = part.starts_with("ERROR in") || part.starts_with("FATAL");
                if !self.in_error || (header && i == 0) {
                    self.errors += 1;
                    self.unseen_errors += 1;
                }
                self.in_error = true;
                LineKind::Error
            } else if part.starts_with("[debug] ") {
                self.in_error = false;
                LineKind::Debug
            } else if self.in_error && stderr && !ends_error(part) {
                LineKind::Error
            } else {
                self.in_error = false;
                LineKind::Info
            };
            self.push(kind, part.to_string());
        }
    }

    /// The whole log as text (Copy / Save).
    pub fn text(&self) -> String {
        let mut s = String::new();
        for l in &self.lines {
            s.push_str(&l.text);
            s.push('\n');
        }
        s
    }
}

/// Starts threads that copy the child's piped stdout and stderr into `log`
/// (repainting `ctx` on new output).
// (also compiled for the web, where no process is ever started)
pub fn attach(child: &mut std::process::Child, log: &SharedLog, ctx: Option<egui::Context>) {
    fn pump(r: impl std::io::Read + Send + 'static, log: SharedLog, stderr: bool, ctx: Option<egui::Context>) {
        std::thread::Builder::new()
            .name(if stderr { "runner-stderr" } else { "runner-stdout" }.into())
            .spawn(move || {
                use std::io::BufRead;
                let mut r = std::io::BufReader::new(r);
                let mut buf = Vec::new();
                loop {
                    buf.clear();
                    match r.read_until(b'\n', &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            let line = String::from_utf8_lossy(&buf);
                            log.lock().unwrap().runner_line(&line, stderr);
                            if let Some(c) = &ctx {
                                c.request_repaint();
                            }
                        }
                    }
                }
            })
            .ok();
    }
    if let Some(o) = child.stdout.take() {
        pump(o, log.clone(), false, ctx.clone());
    }
    if let Some(e) = child.stderr.take() {
        pump(e, log.clone(), true, ctx);
    }
}

/// Window-side state of the Output Log.
pub struct LogView {
    pub open: bool,
    pub follow: bool,
    /// Bring the window to the front at the next frame.
    pub raise: bool,
}

impl Default for LogView {
    fn default() -> Self {
        LogView {
            open: false,
            follow: true,
            raise: false,
        }
    }
}

/// Id of the Output Log window.
pub fn window_id() -> Id {
    Id::new("output_log_window")
}

fn color(kind: LineKind) -> Color32 {
    match kind {
        LineKind::Info => Color32::from_gray(40),
        LineKind::Debug => Color32::from_rgb(0, 70, 160),
        LineKind::Error => gm_ui::theme::ERROR_TEXT,
        LineKind::Ide => Color32::from_rgb(0, 110, 40),
    }
}

/// What the user asked for in the window.
#[derive(Default)]
pub struct LogOut {
    pub save: Option<Vec<u8>>,
}

/// Shows the Output Log window (when open).
pub fn window(ctx: &egui::Context, log: &SharedLog, view: &mut LogView) -> LogOut {
    let mut out = LogOut::default();
    if !view.open {
        return out;
    }
    if std::mem::take(&mut view.raise) {
        crate::mdi::bring_to_front(ctx, window_id());
    }
    let mut l = log.lock().unwrap();
    l.mark_seen();
    let title = if l.errors() > 0 {
        format!(
            "Output Log - {} error{}",
            l.errors(),
            if l.errors() == 1 { "" } else { "s" }
        )
    } else {
        "Output Log".to_string()
    };
    let r = crate::mdi::floating(
        ctx,
        window_id(),
        &title,
        crate::icons::Icon::OutputLog,
        Vec2::new(780.0, 420.0),
        |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if crate::icons::glyph_button(ui, Glyph::Clear, "Clear the log", l.len() > 0)
                    .clicked()
                {
                    l.clear();
                }
                if crate::icons::glyph_button(ui, Glyph::Copy, "Copy the log to the clipboard", l.len() > 0)
                    .clicked()
                {
                    ui.ctx().copy_text(l.text());
                }
                if crate::icons::glyph_button(ui, Glyph::Save, "Save the log to a file", l.len() > 0)
                    .clicked()
                {
                    out.save = Some(l.text().into_bytes());
                }
                crate::icons::tool_separator(ui);
                ui.add_space(4.0);
                ui.checkbox(&mut view.follow, "Scroll to new output");
                ui.add_space(8.0);
                let mut info = format!("{} lines", l.len());
                if l.dropped() > 0 {
                    info.push_str(&format!(" ({} older lines dropped)", l.dropped()));
                }
                if l.errors() > 0 {
                    ui.add(egui::Label::new(
                        egui::RichText::new(format!(
                            "{} error{}",
                            l.errors(),
                            if l.errors() == 1 { "" } else { "s" }
                        ))
                        .color(gm_ui::theme::ERROR_TEXT),
                    ));
                    ui.add_space(6.0);
                }
                ui.weak(info);
            });
            ui.add_space(3.0);
            // white, sunken text area filling the rest
            let area = ui.available_rect_before_wrap().shrink(2.0);
            ui.painter().rect_filled(area, 0.0, Color32::WHITE);
            gm_ui::chrome::sunken(ui.painter(), area);
            let mut inner = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(area.shrink2(Vec2::new(4.0, 2.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            inner.set_clip_rect(area.shrink(1.0).intersect(ui.clip_rect()));
            inner.spacing_mut().item_spacing.y = 1.0;
            let row_h = inner.text_style_height(&egui::TextStyle::Monospace);
            let lines = l.lines();
            // (stuck to the end while it is scrolled to the end, also initially)
            let sa = egui::ScrollArea::both()
                .id_salt("output_log_rows")
                .auto_shrink(false)
                .stick_to_bottom(view.follow);
            sa.show_rows(&mut inner, row_h, lines.len(), |ui, range| {
                for line in lines.range(range) {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(if line.text.is_empty() { " " } else { &line.text })
                                .monospace()
                                .color(color(line.kind)),
                        )
                        .wrap_mode(egui::TextWrapMode::Extend),
                    );
                }
                if lines.is_empty() {
                    ui.weak("Output of games run from the IDE (runner information, show_debug_message, errors) appears here.");
                }
            });
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        },
    );
    if r.close {
        view.open = false;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_error_blocks() {
        let mut l = OutputLog::default();
        l.runner_line("gm-runner: renderer: gl\n", true);
        l.runner_line("[debug] hello\n", true);
        for s in [
            "ERROR in",
            "action number 3",
            "of Create Event",
            "for object t019:",
            "",
            "Error in code at line 1:",
            "   x = 'a' + y_undefined",
            "at position 9: Unknown variable y_undefined",
        ] {
            l.runner_line(s, true);
        }
        l.runner_line("gm-runner: the error was ignored", true);
        l.runner_line("[debug] after", true);
        let kinds: Vec<LineKind> = l.lines().iter().map(|x| x.kind).collect();
        assert_eq!(kinds[0], LineKind::Info);
        assert_eq!(kinds[1], LineKind::Debug);
        assert!(kinds[2..10].iter().all(|k| *k == LineKind::Error), "{kinds:?}");
        assert_eq!(kinds[10], LineKind::Info);
        assert_eq!(kinds[11], LineKind::Debug);
        assert_eq!(l.errors(), 1);
        assert_eq!(l.unseen_errors(), 1);
        l.mark_seen();
        assert_eq!(l.unseen_errors(), 0);
        // CR-separated message bodies are split into lines
        l.runner_line("FATAL ERROR in\rsomething\r\n", true);
        assert_eq!(l.errors(), 2);
        assert_eq!(l.lines().back().unwrap().text, "something");
        assert_eq!(l.lines().back().unwrap().kind, LineKind::Error);
    }

    #[test]
    fn keeps_the_last_lines() {
        let mut l = OutputLog::default();
        for i in 0..MAX_LINES + 10 {
            l.runner_line(&format!("line {i}"), false);
        }
        assert_eq!(l.len(), MAX_LINES);
        assert_eq!(l.dropped(), 10);
        assert_eq!(l.lines().front().unwrap().text, "line 10");
        l.clear();
        assert_eq!(l.len(), 0);
        assert_eq!(l.errors(), 0);
    }
}
