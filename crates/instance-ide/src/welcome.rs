//! The welcome dialog of the desktop IDE (File > Examples…, and the first start): the
//! example games, a tab each, to open (Edit) or play, as on the web (`web/ide.html`).
//! Only their screenshots are built in: Edit and Play download the game from the
//! website ([`SITE`]), keeping a copy in the user's data folder for when it can't be
//! reached.

use std::io::Read;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, pos2, vec2, Color32, Context, Id, Rect, RichText, Sense, Stroke, TextureHandle};

/// Where the example games are downloaded from (the web build's `examples/`).
const SITE: &str = "https://groverburger.github.io/instance/examples/";

/// One example game.
pub struct Example {
    pub title: &'static str,
    /// File name (on the website, and as opened or played).
    pub file: &'static str,
    blurb: &'static str,
    keys: &'static str,
    credit: &'static str,
    /// Licence link: text and address.
    license: (&'static str, &'static str),
    /// Screenshot (PNG or JPEG) and whether it is pixel art (scaled without smoothing).
    shot: &'static [u8],
    pixel: bool,
}

const LICENSES: &str = "https://github.com/groverburger/instance/blob/master/web/examples/";

pub static EXAMPLES: [Example; 3] = [
    Example {
        title: "Spelunky Classic",
        file: "spelunky.gm81",
        blurb: "Derek Yu's cave-diving platformer.",
        keys: "Arrows move, Z jumps, X whips.",
        credit: "© 2008, 2009 Derek Yu and Mossmouth, LLC, shared under the",
        license: ("Spelunky User License", "spelunky-COPYING.txt"),
        shot: include_bytes!("../../../web/examples/spelunky.png"),
        pixel: true,
    },
    Example {
        title: "Emberwood",
        file: "emberwood.gm81",
        blurb: "an autumn walk through a 3D forest.",
        keys: "WASD walk, the mouse looks, M shows the map.",
        credit: "© 2026 groverburger, shared under the",
        license: ("MIT License", "LICENSE-MIT.txt"),
        shot: include_bytes!("../../../web/examples/emberwood.jpg"),
        pixel: false,
    },
    Example {
        title: "Gravity Grade",
        file: "gravity_grade.gm81",
        blurb: "falling blocks, up to 20G.",
        keys: "Arrows move and drop, Z and X rotate.",
        credit: "© 2026 groverburger, shared under the",
        license: ("MIT License", "LICENSE-MIT.txt"),
        shot: include_bytes!("../../../web/examples/gravity_grade.png"),
        pixel: true,
    },
];

/// What the visitor picked; Edit and Play come with the downloaded game.
pub enum Choice {
    Edit(&'static Example, Vec<u8>),
    Play(&'static Example, Vec<u8>),
    New,
    Open,
}

/// A download's bytes so far, its expected bytes (0: unknown), and the result once done.
type Progress = Arc<Mutex<(u64, u64, Option<Result<Vec<u8>, String>>)>>;

/// A download in progress (on its own thread).
struct Download {
    ex: &'static Example,
    /// Play it (else edit it) once it is here.
    play: bool,
    state: Progress,
}

/// The dialog's state.
#[derive(Default)]
pub struct Welcome {
    pub open: bool,
    tab: usize,
    shots: Vec<Option<TextureHandle>>,
    download: Option<Download>,
    /// Why the last download failed.
    error: Option<String>,
}

/// The copy of `file` kept in the user's data folder.
fn cached(file: &str) -> Option<std::path::PathBuf> {
    Some(crate::platform::config_dir()?.join("examples").join(file))
}

/// Downloads an example game, reporting progress into `state`; on failure the copy
/// from an earlier download, if there is one.
fn fetch(file: &'static str, state: Progress) {
    let get = || -> Result<Vec<u8>, String> {
        let mut resp = ureq::get(&format!("{SITE}{file}")).call().map_err(|e| e.to_string())?;
        let want = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        state.lock().unwrap().1 = want;
        let mut r = resp.body_mut().as_reader();
        let mut out = Vec::with_capacity(want as usize);
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = r.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
            state.lock().unwrap().0 = out.len() as u64;
        }
        Ok(out)
    };
    let result = match get() {
        Ok(b) => {
            if let Some(p) = cached(file) {
                let _ = p.parent().map(std::fs::create_dir_all);
                let _ = std::fs::write(p, &b);
            }
            Ok(b)
        }
        Err(e) => cached(file)
            .and_then(|p| std::fs::read(p).ok())
            .ok_or(format!("Cannot download {file}: {e}")),
    };
    state.lock().unwrap().2 = Some(result);
}

/// Size of the screenshot area (16:9).
const STAGE: egui::Vec2 = vec2(608.0, 342.0);

impl Welcome {
    pub fn show(&mut self) {
        self.open = true;
        self.tab = 0;
        self.error = None;
    }

    fn start_download(&mut self, ex: &'static Example, play: bool) {
        let state = Arc::new(Mutex::new((0, 0, None)));
        let st = state.clone();
        std::thread::spawn(move || fetch(ex.file, st));
        self.download = Some(Download { ex, play, state });
        self.error = None;
    }

    /// Shows the dialog while it is open; the choice (which also closes it), if any.
    pub fn ui(&mut self, ctx: &Context) -> Option<Choice> {
        if !self.open {
            return None;
        }
        let mut choice = None;
        // a finished download becomes the choice
        if let Some(d) = &self.download {
            let done = d.state.lock().unwrap().2.take();
            match done {
                Some(Ok(bytes)) => {
                    choice = Some(if d.play { Choice::Play(d.ex, bytes) } else { Choice::Edit(d.ex, bytes) });
                    self.download = None;
                }
                Some(Err(e)) => {
                    self.error = Some(e);
                    self.download = None;
                }
                None => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
            }
        }
        let busy = self.download.is_some();
        let d = crate::mdi::dialog(ctx, Id::new("welcome"), "Welcome to Instance", |ui| {
            ui.set_width(STAGE.x + 22.0);
            if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                self.tab = (self.tab + EXAMPLES.len() - 1) % EXAMPLES.len();
            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                self.tab = (self.tab + 1) % EXAMPLES.len();
            }
            let sel = self.tabs(ui);
            // the tab's sheet: screenshot, text, Edit / Play
            let ex = &EXAMPLES[self.tab];
            let sheet = egui::Frame::NONE.inner_margin(10).show(ui, |ui| {
                self.stage(ui);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(STAGE.x - 190.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.label(RichText::new(ex.title).strong());
                            ui.label(format!(" — {}", ex.blurb));
                        });
                        ui.label(RichText::new(ex.keys).small().weak());
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.label(RichText::new(format!("{} ", ex.credit)).small().weak());
                            ui.hyperlink_to(RichText::new(ex.license.0).small(), format!("{LICENSES}{}", ex.license.1));
                            ui.label(RichText::new(".").small().weak());
                        });
                        if let Some(d) = &self.download {
                            let (got, want, _) = *d.state.lock().unwrap();
                            let mb = |n: u64| n as f64 / 1048576.0;
                            let text = if want > 0 {
                                format!("Downloading {}… {:.1} of {:.1} MB", d.ex.file, mb(got), mb(want))
                            } else {
                                format!("Downloading {}… {:.1} MB", d.ex.file, mb(got))
                            };
                            ui.label(RichText::new(text).small());
                        } else if let Some(e) = &self.error {
                            ui.label(RichText::new(e).small().color(Color32::from_rgb(160, 0, 0)));
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled_ui(!busy, |ui| {
                            if ui.add(egui::Button::new("Play").min_size(vec2(82.0, 24.0))).clicked() {
                                self.start_download(ex, true);
                            }
                            if ui.add(egui::Button::new("Edit").min_size(vec2(82.0, 24.0))).clicked() {
                                self.start_download(ex, false);
                            }
                        });
                    });
                });
            });
            // the sheet's raised edge, open below the selected tab
            let r = sheet.response.rect.shrink(0.5);
            let p = ui.painter();
            instance_ui::chrome::bevel(p, r, true);
            p.line_segment([pos2(sel.left() + 1.0, r.top()), pos2(sel.right() - 1.0, r.top())], Stroke::new(1.0, instance_ui::theme::PANEL));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("New project").min_size(vec2(82.0, 24.0))).clicked() {
                    choice = Some(Choice::New);
                }
                if ui.add(egui::Button::new("Open file…").min_size(vec2(82.0, 24.0))).clicked() {
                    choice = Some(Choice::Open);
                }
            });
            ui.add_space(4.0);
            ui.label(RichText::new("Not affiliated with or endorsed by YoYo Games.").small().weak());
        });
        if d.close {
            // (a download still running is dropped)
            self.download = None;
        }
        if d.close || choice.is_some() {
            self.open = false;
        }
        choice
    }

    /// The classic tabs, one per example; the selected tab's rectangle.
    fn tabs(&mut self, ui: &mut egui::Ui) -> Rect {
        let font = egui::FontId::proportional(13.0);
        let h = 22.0;
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
        let p = ui.painter().clone();
        let mut x = row.left() + 2.0;
        let mut sel_rect = None;
        for (i, ex) in EXAMPLES.iter().enumerate() {
            let w = ui.fonts_mut(|f| f.layout_no_wrap(ex.title.into(), font.clone(), Color32::BLACK).size().x) + 24.0;
            let r = Rect::from_min_size(pos2(x, row.top() + 2.0), vec2(w, h - 2.0));
            x += w - 1.0;
            let resp = ui.interact(r, ui.id().with(("welcome_tab", i)), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, self.tab == i, ex.title));
            if resp.clicked() {
                self.tab = i;
            }
            if self.tab == i {
                sel_rect = Some(r);
                continue;
            }
            let fill = if resp.hovered() { Color32::from_rgb(234, 242, 252) } else { Color32::from_gray(226) };
            paint_tab(&p, r, fill, ex.title, &font);
        }
        // the selected tab last, a little taller, over its neighbours
        let r = sel_rect.unwrap_or(row);
        let r = Rect::from_min_max(r.min - vec2(0.0, 2.0), r.max + vec2(0.0, 1.0));
        paint_tab(&p, r, instance_ui::theme::PANEL, EXAMPLES[self.tab].title, &font);
        r
    }

    /// The screenshot of the current example, fitted into a sunken black area.
    fn stage(&mut self, ui: &mut egui::Ui) {
        let (r, _) = ui.allocate_exact_size(STAGE, Sense::hover());
        let p = ui.painter();
        p.rect_filled(r, 0.0, Color32::BLACK);
        instance_ui::chrome::sunken(p, r);
        if self.shots.len() < EXAMPLES.len() {
            self.shots.resize_with(EXAMPLES.len(), || None);
        }
        let ex = &EXAMPLES[self.tab];
        let tex = self.shots[self.tab].get_or_insert_with(|| {
            let img = image::load_from_memory(ex.shot).map(|i| i.to_rgba8()).unwrap_or_else(|_| image::RgbaImage::new(1, 1));
            let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
            let opts = if ex.pixel { egui::TextureOptions::NEAREST } else { egui::TextureOptions::LINEAR };
            ui.ctx().load_texture(format!("welcome-{}", ex.file), ci, opts)
        });
        let s = tex.size_vec2();
        let k = (r.width() / s.x).min(r.height() / s.y);
        let shown = Rect::from_center_size(r.center(), s * k);
        p.image(tex.id(), shown, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
}

fn paint_tab(p: &egui::Painter, r: Rect, fill: Color32, label: &str, font: &egui::FontId) {
    p.rect_filled(r, 0.0, fill);
    let hi = Stroke::new(1.0, Color32::WHITE);
    let lo = Stroke::new(1.0, Color32::from_gray(105));
    let q = r.shrink(0.5);
    p.line_segment([q.left_bottom(), q.left_top() + vec2(0.0, 2.0)], hi);
    p.line_segment([q.left_top() + vec2(0.0, 2.0), q.left_top() + vec2(2.0, 0.0)], hi);
    p.line_segment([q.left_top() + vec2(2.0, 0.0), q.right_top() - vec2(2.0, 0.0)], hi);
    p.line_segment([q.right_top() + vec2(-1.0, 1.0), q.right_bottom()], lo);
    p.text(r.center(), egui::Align2::CENTER_CENTER, label, font.clone(), Color32::from_gray(10));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Downloads the smallest example from the website (needs the network:
    /// `cargo test -p instance-ide -- --ignored welcome`).
    #[test]
    #[ignore]
    fn welcome_downloads_an_example() {
        let state = Arc::new(Mutex::new((0, 0, None)));
        fetch("emberwood.gm81", state.clone());
        let (got, want, result) = std::mem::take(&mut *state.lock().unwrap());
        let bytes = result.expect("finished").expect("downloaded");
        assert!(bytes.len() > 1_000_000 && got == bytes.len() as u64, "{got} of {want}");
        assert!(instance_format::read_project_versioned(&bytes).is_ok());
    }
}
