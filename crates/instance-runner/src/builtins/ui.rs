//! Chapter 407: messages, questions, splash screens, highscores.
//!
//! Native dialogs come from the frontend's [`UiHooks`](crate::game::UiHooks);
//! text input and the highscore table use an in-game overlay drawn over the
//! last frame (windowed mode) and answer with defaults in headless mode.

use super::{bo, r, st, zero, Reg, A};
use crate::draw::painter::Xf;
use crate::draw::rgba;
use crate::game::{Game, HighscoreEntry};
use crate::input::{vk, InputEvent};
use gml::{Error, Value};

/// Highscore table settings.
#[derive(Clone)]
pub struct HighscoreStyle {
    pub background: i32,
    pub border: bool,
    pub font: String,
    pub size: i32,
    pub back_col: i32,
    pub new_col: i32,
    pub other_col: i32,
    pub caption: String,
    pub nobody: String,
    pub escape: String,
}

impl Default for HighscoreStyle {
    fn default() -> Self {
        HighscoreStyle {
            background: -1,
            border: true,
            font: "Times New Roman".into(),
            size: 10,
            back_col: 0xFFFFFF,
            new_col: 0x0000FF,
            other_col: 0x000000,
            caption: "Top Ten Players".into(),
            nobody: "<nobody>".into(),
            escape: "press <Escape> to close".into(),
        }
    }
}

/// Very small RTF → plain text conversion for the game information.
pub fn rtf_to_text(rtf: &str) -> String {
    if !rtf.starts_with("{\\rtf") {
        return rtf.to_string();
    }
    let mut out = String::new();
    let mut depth = 0i32;
    let mut skip_depth: Option<i32> = None;
    let chars: Vec<char> = rtf.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '{' => {
                depth += 1;
                if i + 2 < chars.len() && chars[i + 1] == '\\' && chars[i + 2] == '*' && skip_depth.is_none() {
                    skip_depth = Some(depth);
                }
            }
            '}' => {
                if skip_depth == Some(depth) {
                    skip_depth = None;
                }
                depth -= 1;
            }
            '\\' => {
                let mut j = i + 1;
                let mut word = String::new();
                while j < chars.len() && chars[j].is_ascii_alphabetic() {
                    word.push(chars[j]);
                    j += 1;
                }
                if word.is_empty() && j < chars.len() {
                    if chars[j] == '\'' && j + 2 < chars.len() {
                        let h: String = chars[j + 1..j + 3].iter().collect();
                        if let Ok(b) = u8::from_str_radix(&h, 16) {
                            if skip_depth.is_none() {
                                out.push_str(&instance_format::text::decode_cp1252(&[b]));
                            }
                        }
                        i = j + 3;
                        continue;
                    }
                    if skip_depth.is_none() && matches!(chars[j], '\\' | '{' | '}') {
                        out.push(chars[j]);
                    }
                    i = j + 1;
                    continue;
                }
                while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j] == ' ' {
                    j += 1;
                }
                if matches!(word.as_str(), "fonttbl" | "colortbl" | "stylesheet" | "info" | "pict") {
                    skip_depth = Some(depth);
                }
                if skip_depth.is_none() && (word == "par" || word == "line") {
                    out.push('\n');
                }
                if skip_depth.is_none() && word == "tab" {
                    out.push('\t');
                }
                i = j;
                continue;
            }
            '\r' | '\n' => {}
            _ => {
                if skip_depth.is_none() && depth >= 1 {
                    out.push(c);
                }
            }
        }
        i += 1;
    }
    out.trim().to_string()
}

fn hs_path(g: &Game) -> std::path::PathBuf {
    crate::vfs::config_dir().join(format!("highscore_{}.txt", g.game_id))
}

impl Game {
    pub fn load_highscores(&mut self) {
        if self.hs_loaded {
            return;
        }
        self.hs_loaded = true;
        if let Ok(t) = crate::vfs::read_to_string(&hs_path(self)) {
            for line in t.lines() {
                if let Some((s, n)) = line.split_once('\t') {
                    if let Ok(v) = s.parse::<f64>() {
                        self.highscores.push(HighscoreEntry {
                            name: n.to_string(),
                            score: v,
                        });
                    }
                }
            }
        }
        self.highscores.truncate(10);
    }

    pub fn save_highscores(&self) {
        let p = hs_path(self);
        if let Some(d) = p.parent() {
            let _ = crate::vfs::create_dir_all(d);
        }
        let s: String = self.highscores.iter().map(|e| format!("{}\t{}\n", e.score, e.name)).collect();
        let _ = crate::vfs::write(&p, s.as_bytes());
    }

    /// Inserts an entry; returns its rank (0-based) if it made the table.
    pub fn highscore_insert(&mut self, name: &str, score: f64) -> Option<usize> {
        self.load_highscores();
        let pos = self.highscores.iter().position(|e| score > e.score).unwrap_or(self.highscores.len());
        if pos >= 10 {
            return None;
        }
        self.highscores.insert(
            pos,
            HighscoreEntry {
                name: name.to_string(),
                score,
            },
        );
        self.highscores.truncate(10);
        self.save_highscores();
        Some(pos)
    }

    fn qualifies(&mut self, score: f64) -> bool {
        self.load_highscores();
        self.highscores.len() < 10 || self.highscores.iter().any(|e| score > e.score)
    }

    /// Runs a modal overlay loop: `draw` paints over the saved frame; returns when
    /// `step` returns Some.
    fn modal<T>(&mut self, mut step: impl FnMut(&mut Game, &[InputEvent]) -> Option<T>, draw: impl Fn(&mut Game)) -> Option<T> {
        if self.headless || self.pump.is_none() || self.gfx.present.is_none() {
            return None;
        }
        let saved = self.gfx.read_back();
        let (sx, sc, st_) = (self.gfx.pass_xf, self.gfx.pass_clip, self.gfx.target);
        // overlays are plain 2D
        let saved3d = (self.gfx.d3d.on, self.gfx.d3d.world, self.gfx.d3d.proj);
        self.gfx.d3d.on = false;
        self.gfx.d3d.world = crate::draw::mat::IDENT;
        self.gfx.d3d.proj = crate::d3d::Proj::Default;
        self.gfx.pass_xf = Xf::IDENTITY;
        self.gfx.pass_clip = self.gfx.back_clip();
        self.gfx.target = -1;
        let result;
        loop {
            let evs = self.pump.as_mut().map(|p| p()).unwrap_or_default();
            if evs.iter().any(|e| matches!(e, InputEvent::Close)) {
                self.close_requested = true;
                result = None;
                break;
            }
            if let Some(v) = step(self, &evs) {
                result = Some(v);
                break;
            }
            if cfg!(target_arch = "wasm32") {
                // no input can arrive while a step runs in the browser: cancel
                result = None;
                break;
            }
            self.gfx.be.write(crate::draw::Target::Back, 0, 0, &saved);
            draw(self);
            self.gfx.present_now();
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        self.gfx.be.write(crate::draw::Target::Back, 0, 0, &saved);
        (self.gfx.d3d.on, self.gfx.d3d.world, self.gfx.d3d.proj) = saved3d;
        self.gfx.pass_xf = sx;
        self.gfx.pass_clip = sc;
        self.gfx.target = st_;
        self.input.clear_all();
        result
    }

    fn overlay_text(&mut self, x: f64, y: f64, text: &str, col: i32, size: i32, font: &str, halign: i32) {
        let saved = (self.gfx.font, self.gfx.halign, self.gfx.valign);
        // temporary font resource
        self.fonts.push(Some(crate::res::FontRes {
            name: "__overlay".into(),
            font_name: font.to_string(),
            size,
            bold: false,
            italic: false,
            first: 32,
            last: 127,
            sprite: None,
        }));
        self.gfx.font = self.fonts.len() as i32 - 1;
        self.gfx.halign = halign;
        self.gfx.valign = 0;
        let bytes = instance_format::text::encode_cp1252(text);
        self.draw_text_full(x, y, &bytes, -1.0, -1.0, 1.0, 1.0, 0.0, [col; 4], 1.0);
        self.fonts.pop();
        (self.gfx.font, self.gfx.halign, self.gfx.valign) = saved;
    }

    /// In-game text input box.
    pub fn input_box(&mut self, prompt: &str, default: &str) -> String {
        let text = std::rc::Rc::new(std::cell::RefCell::new(default.to_string()));
        let prompt = prompt.to_string();
        let t2 = text.clone();
        let r_ = self.modal(
            move |_, evs| {
                let mut text = t2.borrow_mut();
                for e in evs {
                    match e {
                        InputEvent::Char(c) if !c.is_control() => text.push(*c),
                        InputEvent::KeyDown(k) if *k == vk::BACKSPACE => {
                            text.pop();
                        }
                        InputEvent::KeyDown(k) if *k == vk::ENTER => return Some(text.clone()),
                        InputEvent::KeyDown(k) if *k == vk::ESCAPE => return Some(String::new()),
                        _ => {}
                    }
                }
                None
            },
            |g| {
                let (w, h) = (g.gfx.back_size().0 as f64, g.gfx.back_size().1 as f64);
                let (bw, bh) = (w.min(360.0), 110.0);
                let (x0, y0) = ((w - bw) / 2.0, (h - bh) / 2.0);
                let mut p = g.gfx.painter();
                p.rect(x0, y0, x0 + bw, y0 + bh, [rgba(0xE0E0E0, 1.0); 4], false);
                p.rect(x0, y0, x0 + bw, y0 + bh, [rgba(0x404040, 1.0); 4], true);
                p.rect(x0 + 10.0, y0 + 70.0, x0 + bw - 10.0, y0 + 95.0, [rgba(0xFFFFFF, 1.0); 4], false);
                g.overlay_text(x0 + 10.0, y0 + 10.0, &prompt, 0, 10, "Arial", 0);
                let t = format!("{}_", text.borrow());
                g.overlay_text(x0 + 14.0, y0 + 74.0, &t, 0, 10, "Arial", 0);
            },
        );
        r_.unwrap_or_else(|| default.to_string())
    }

    /// Draws the highscore table into a box.
    pub fn draw_highscore_table(&mut self, x1: f64, y1: f64, x2: f64, _y2: f64, style: &HighscoreStyle, new_rank: Option<usize>, use_draw_font: bool) {
        self.load_highscores();
        let entries = self.highscores.clone();
        let line_h = (style.size as f64 * 96.0 / 72.0 * 1.4).max(12.0);
        for k in 0..10 {
            let y = y1 + k as f64 * line_h;
            let (name, score) = match entries.get(k) {
                Some(e) => (e.name.clone(), gml::real_to_string(e.score)),
                None => (style.nobody.clone(), "0".into()),
            };
            let col = if Some(k) == new_rank { style.new_col } else { style.other_col };
            if use_draw_font {
                let c = [self.gfx.color; 4];
                let al = self.gfx.alpha;
                let saved = self.gfx.halign;
                self.gfx.halign = 0;
                self.draw_text_full(x1, y, name.as_bytes(), -1.0, -1.0, 1.0, 1.0, 0.0, c, al);
                self.gfx.halign = 2;
                self.draw_text_full(x2, y, score.as_bytes(), -1.0, -1.0, 1.0, 1.0, 0.0, c, al);
                self.gfx.halign = saved;
            } else {
                let font = style.font.clone();
                self.overlay_text(x1, y, &name, col, style.size, &font, 0);
                self.overlay_text(x2, y, &score, col, style.size, &font, 2);
            }
        }
    }

    /// `highscore_show`: optionally ask for a name, then show the table.
    pub fn highscore_show(&mut self, numb: f64, style: HighscoreStyle) {
        self.load_highscores();
        let mut rank = None;
        if numb >= 0.0 && self.qualifies(numb) {
            let name = if self.headless || self.pump.is_none() {
                "Player".to_string()
            } else {
                self.input_box("Congratulations! You made the highscore list.\nPlease enter your name:", "")
            };
            let name = if name.is_empty() { "Anonymous".to_string() } else { name };
            rank = self.highscore_insert(&name, numb);
        }
        let style2 = style.clone();
        self.modal(
            |_, evs| {
                if evs.iter().any(|e| matches!(e, InputEvent::KeyDown(_) | InputEvent::MouseDown(_))) {
                    Some(())
                } else {
                    None
                }
            },
            move |g| {
                let (w, h) = (g.gfx.back_size().0 as f64, g.gfx.back_size().1 as f64);
                let (bw, bh) = (360.0f64.min(w), 400.0f64.min(h));
                let (x0, y0) = ((w - bw) / 2.0, (h - bh) / 2.0);
                if let Some(bg) = g.background(style2.background).map(|b| b.tex.clone()) {
                    let (sx, sy) = (bw / bg.w.max(1) as f64, bh / bg.h.max(1) as f64);
                    g.gfx.draw_tex(&crate::draw::Tex::Img(bg.clone()), 0.0, 0.0, bg.w as f64, bg.h as f64, 0.0, 0.0, x0, y0, sx, sy, 0.0, [0xFFFFFF; 4], 1.0);
                } else {
                    g.gfx.painter().rect(x0, y0, x0 + bw, y0 + bh, [rgba(style2.back_col, 1.0); 4], false);
                }
                if style2.border {
                    g.gfx.painter().rect(x0, y0, x0 + bw, y0 + bh, [rgba(0, 1.0); 4], true);
                }
                let font = style2.font.clone();
                g.overlay_text(x0 + bw / 2.0, y0 + 10.0, &style2.caption, style2.other_col, style2.size + 4, &font, 1);
                g.draw_highscore_table(x0 + 30.0, y0 + 50.0, x0 + bw - 30.0, y0 + bh - 40.0, &style2, rank, false);
                g.overlay_text(x0 + bw / 2.0, y0 + bh - 25.0, &style2.escape, style2.other_col, 8, "Arial", 1);
            },
        );
    }

    /// Message box (with the frontend's dialog).
    pub fn message(&mut self, s: &str) {
        self.ui.message(s);
        self.input.clear_all();
    }
}

fn text_arg(a: &A, i: usize) -> String {
    let t = a.text(i);
    let s = instance_format::text::decode_cp1252(&t);
    // '#' is a line break in GM messages
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'#') {
            out.push('#');
            chars.next();
        } else if c == '#' {
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    out
}

pub fn register(l: &mut Reg) {
    l.push(("show_message", |g, _, _, a| {
        let s = text_arg(a, 0);
        g.message(&s);
        zero()
    }));
    l.push(("show_message_ext", |g, _, _, a| {
        let s = text_arg(a, 0);
        let buttons: Vec<String> = (1..4).map(|i| a.s(i).unwrap_or_default().replace('&', "")).collect();
        let n = g.ui.buttons(&s, &buttons);
        g.input.clear_all();
        r(n as f64)
    }));
    l.push(("show_question", |g, _, _, a| {
        let s = text_arg(a, 0);
        let b = g.ui.question(&s);
        g.input.clear_all();
        bo(b)
    }));
    l.push(("get_integer", |g, _, _, a| {
        let s = text_arg(a, 0);
        let def = match a.val(1) {
            Value::Real(x) => gml::real_to_string(x),
            Value::Str(s) => s.to_string_lossy(),
        };
        let res = if g.ui.has_input() { g.ui.get_string(&s, &def) } else { g.input_box(&s, &def) };
        g.input.clear_all();
        r(res.trim().parse::<f64>().unwrap_or(0.0))
    }));
    l.push(("get_string", |g, _, _, a| {
        let s = text_arg(a, 0);
        let def = a.s(1).unwrap_or_default();
        let res = if g.ui.has_input() { g.ui.get_string(&s, &def) } else { g.input_box(&s, &def) };
        g.input.clear_all();
        st(&res)
    }));
    l.push(("show_menu", |g, _, _, a| {
        let items: Vec<String> = a.s(0)?.split('|').map(|s| s.to_string()).collect();
        let def = a.i(1)?;
        r(g.ui.menu(&items).map(|i| i as i32).unwrap_or(def) as f64)
    }));
    l.push(("show_menu_pos", |g, _, _, a| {
        let items: Vec<String> = a.s(2)?.split('|').map(|s| s.to_string()).collect();
        let def = a.i(3)?;
        r(g.ui.menu(&items).map(|i| i as i32).unwrap_or(def) as f64)
    }));
    l.push(("get_color", |g, _, _, a| r(g.ui.color(a.i(0)?) as f64)));
    l.push(("get_open_filename", |g, _, _, a| {
        let f = g.ui.file_dialog(false, &a.s(0)?, &a.s(1)?);
        st(&f)
    }));
    l.push(("get_save_filename", |g, _, _, a| {
        let f = g.ui.file_dialog(true, &a.s(0)?, &a.s(1)?);
        st(&f)
    }));
    l.push(("get_directory", |g, _, _, a| st(&g.ui.directory(&a.s(0)?))));
    l.push(("get_directory_alt", |g, _, _, a| st(&g.ui.directory(&a.s(1)?))));
    l.push(("show_error", |g, _, _, a| {
        let s = text_arg(a, 0);
        let abort = a.b(1)?;
        g.error_occurred = true;
        g.error_last = s.clone();
        let user_abort = g.ui_error(&s, abort);
        if abort || user_abort {
            g.aborted = true;
            g.ended = true;
            return Err(Error::halt());
        }
        zero()
    }));
    l.push(("show_info", |g, _, _, _| {
        let t = rtf_to_text(&g.game_info.clone());
        g.message(&t);
        zero()
    }));
    l.push(("load_info", |g, _, _, a| {
        let p = g.resolve_path(&a.s(0)?);
        if let Ok(b) = crate::vfs::read(&p) {
            g.game_info = instance_format::text::decode_cp1252(&b);
        }
        zero()
    }));
    // splash screens: text and images are shown as messages / overlays
    l.push(("splash_show_text", |g, _, _, a| {
        let p = g.resolve_path(&a.s(0)?);
        if let Ok(b) = crate::vfs::read(&p) {
            let t = rtf_to_text(&instance_format::text::decode_cp1252(&b));
            g.message(&t);
        }
        zero()
    }));
    l.push(("splash_show_image", |g, _, _, a| {
        let p = g.resolve_path(&a.s(0)?);
        let delay = a.r(1)?;
        if let Some(t) = crate::draw::Texture::load_file(&p).map(std::rc::Rc::new) {
            let tt = crate::draw::Tex::Img(t.clone());
            let start = web_time::Instant::now();
            g.modal(
                |_, evs| {
                    let timed_out = delay > 0.0 && start.elapsed().as_secs_f64() * 1000.0 >= delay;
                    if timed_out || evs.iter().any(|e| matches!(e, InputEvent::KeyDown(_) | InputEvent::MouseDown(_))) {
                        Some(())
                    } else {
                        None
                    }
                },
                |g| {
                    let (w, h) = (g.gfx.back_size().0 as f64, g.gfx.back_size().1 as f64);
                    g.gfx.painter().clear([0, 0, 0, 255]);
                    let s = (w / t.w.max(1) as f64).min(h / t.h.max(1) as f64);
                    let (x0, y0) = ((w - t.w as f64 * s) / 2.0, (h - t.h as f64 * s) / 2.0);
                    g.gfx.draw_tex(&tt, 0.0, 0.0, t.w as f64, t.h as f64, 0.0, 0.0, x0, y0, s, s, 0.0, [0xFFFFFF; 4], 1.0);
                },
            );
        }
        zero()
    }));
    // highscores
    l.push(("highscore_show", |g, _, _, a| {
        let n = a.r(0)?;
        let style = g.hs_style.clone();
        g.highscore_show(n, style);
        zero()
    }));
    l.push(("highscore_show_ext", |g, _, _, a| {
        let mut style = g.hs_style.clone();
        style.background = a.i(1)?;
        style.border = a.b(2)?;
        style.new_col = a.i(3)?;
        style.other_col = a.i(4)?;
        style.font = a.s(5)?;
        style.size = a.i(6)?;
        g.highscore_show(a.r(0)?, style);
        zero()
    }));
    l.push(("highscore_set_background", |g, _, _, a| {
        g.hs_style.background = a.i(0)?;
        zero()
    }));
    l.push(("highscore_set_border", |g, _, _, a| {
        g.hs_style.border = a.b(0)?;
        zero()
    }));
    l.push(("highscore_set_font", |g, _, _, a| {
        g.hs_style.font = a.s(0)?;
        g.hs_style.size = a.i(1)?;
        zero()
    }));
    l.push(("highscore_set_strings", |g, _, _, a| {
        g.hs_style.caption = a.s(0)?;
        g.hs_style.nobody = a.s(1)?;
        g.hs_style.escape = a.s(2)?;
        zero()
    }));
    l.push(("highscore_set_colors", |g, _, _, a| {
        g.hs_style.back_col = a.i(0)?;
        g.hs_style.new_col = a.i(1)?;
        g.hs_style.other_col = a.i(2)?;
        zero()
    }));
    l.push(("highscore_clear", |g, _, _, _| {
        g.load_highscores();
        g.highscores.clear();
        g.save_highscores();
        zero()
    }));
    l.push(("highscore_add", |g, _, _, a| {
        g.highscore_insert(&a.s(0)?, a.r(1)?);
        zero()
    }));
    l.push(("highscore_add_current", |g, _, _, _| {
        let sc = g.score;
        if g.qualifies(sc) {
            let name = if g.headless { "Player".into() } else { g.input_box("Please enter your name:", "") };
            g.highscore_insert(&name, sc);
        }
        zero()
    }));
    l.push(("highscore_value", |g, _, _, a| {
        g.load_highscores();
        let k = a.i(0)? - 1;
        r(if k >= 0 { g.highscores.get(k as usize).map(|e| e.score).unwrap_or(0.0) } else { 0.0 })
    }));
    l.push(("highscore_name", |g, _, _, a| {
        g.load_highscores();
        let k = a.i(0)? - 1;
        let n = if k >= 0 { g.highscores.get(k as usize).map(|e| e.name.clone()) } else { None };
        st(&n.unwrap_or_else(|| g.hs_style.nobody.clone()))
    }));
    l.push(("draw_highscore", |g, _, _, a| {
        let style = g.hs_style.clone();
        g.draw_highscore_table(a.r(0)?, a.r(1)?, a.r(2)?, a.r(3)?, &style, None, true);
        zero()
    }));
}
