//! The help viewer (Help > Contents, F1): the user manual of `docs/manual/`, embedded
//! into the binary at compile time (see `build.rs`), shown in a child window with
//! the contents tree from `docs/manual/README.md`, a search box (page titles, page
//! text and built-in names), back/forward buttons and the rendered page.
//!
//! Context help: [`request_topic`] (the code editor's F1 and "Help on …") opens the
//! reference entry of a built-in name, found through `reference/index.md`.

pub mod markdown;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use eframe::egui::{self, pos2, vec2, Color32, Id, Rect, Sense, Ui, Vec2};
use instance_ui::icons::Glyph;

use markdown::{Block, Doc, RenderCx};

include!(concat!(env!("OUT_DIR"), "/manual_files.rs"));

/// The contents page of the manual.
pub const HOME: &str = "README.md";
/// The A–Z index of the function reference (built-in name → entry).
pub const NAME_INDEX: &str = "reference/index.md";

/// The bytes of a manual file (path relative to `docs/manual`, `/`-separated).
pub fn file(path: &str) -> Option<&'static [u8]> {
    MANUAL_FILES
        .iter()
        .find(|(p, _)| *p == path)
        .map(|(_, b)| *b)
}

/// The text of a manual page.
pub fn page_text(path: &str) -> Option<&'static str> {
    file(path).and_then(|b| std::str::from_utf8(b).ok())
}

/// A place in the manual.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loc {
    pub page: String,
    pub anchor: Option<String>,
}

impl Loc {
    pub fn page(p: &str) -> Self {
        Loc {
            page: p.to_string(),
            anchor: None,
        }
    }
}

/// Resolves a link `target` found on page `from`. `None` for external links.
pub fn resolve(from: &str, target: &str) -> Option<Loc> {
    if target.contains("://") || target.starts_with("mailto:") {
        return None;
    }
    let (path, anchor) = match target.split_once('#') {
        Some((p, a)) => (p, Some(a.to_string()).filter(|a| !a.is_empty())),
        None => (target, None),
    };
    if path.is_empty() {
        return Some(Loc {
            page: from.to_string(),
            anchor,
        });
    }
    let mut parts: Vec<&str> = if path.starts_with('/') {
        Vec::new()
    } else {
        let mut v: Vec<&str> = from.split('/').collect();
        v.pop();
        v
    };
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    Some(Loc {
        page: parts.join("/"),
        anchor,
    })
}

// ---------------------------------------------------------------------------
// contents, index, search

/// A section of the contents (a `##` heading of `README.md` and its links).
#[derive(Clone, Debug)]
pub struct TocSection {
    pub title: String,
    pub pages: Vec<(String, String)>,
}

/// The contents tree, parsed from `docs/manual/README.md`.
pub fn toc() -> &'static [TocSection] {
    static T: OnceLock<Vec<TocSection>> = OnceLock::new();
    T.get_or_init(|| {
        let mut out: Vec<TocSection> = Vec::new();
        let doc = markdown::parse(page_text(HOME).unwrap_or(""));
        for b in &doc.blocks {
            match b {
                Block::Heading {
                    level: 2, spans, ..
                } => out.push(TocSection {
                    title: markdown::plain(spans),
                    pages: Vec::new(),
                }),
                Block::List { items, .. } => {
                    let Some(sec) = out.last_mut() else { continue };
                    for item in items {
                        if let Some((title, link)) = first_link(item) {
                            if let Some(loc) = resolve(HOME, &link) {
                                sec.pages.push((title, loc.page));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        out
    })
}

fn first_link(blocks: &[Block]) -> Option<(String, String)> {
    for b in blocks {
        if let Block::Paragraph(spans) = b {
            let mut title = String::new();
            let mut link = None;
            for s in spans {
                if let markdown::Inline::Text {
                    text,
                    link: Some(l),
                    ..
                } = s
                {
                    if link.is_none() || link.as_deref() == Some(l.as_str()) {
                        link = Some(l.clone());
                        title.push_str(text);
                    }
                }
            }
            if let Some(l) = link {
                return Some((title, l));
            }
        }
    }
    None
}

/// Built-in name → its reference entry, from `reference/index.md` (lines
/// `- [name](file.md#anchor) (kind)`). A name listed twice keeps its first entry.
pub fn names() -> &'static [(String, Loc, String)] {
    static N: OnceLock<Vec<(String, Loc, String)>> = OnceLock::new();
    N.get_or_init(|| {
        let mut out: Vec<(String, Loc, String)> = Vec::new();
        for line in page_text(NAME_INDEX).unwrap_or("").lines() {
            let Some(rest) = line.strip_prefix("- [") else {
                continue;
            };
            let Some((name, rest)) = rest.split_once("](") else {
                continue;
            };
            let Some((target, rest)) = rest.split_once(')') else {
                continue;
            };
            let kind = rest
                .trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .to_string();
            if let Some(loc) = resolve(NAME_INDEX, target) {
                out.push((name.to_string(), loc, kind));
            }
        }
        out
    })
}

/// The reference entry of a built-in name.
pub fn lookup_name(name: &str) -> Option<Loc> {
    names()
        .iter()
        .find(|(n, _, _)| n == name)
        .map(|(_, l, _)| l.clone())
}

/// A searchable page: its title and the text of each section (anchor, heading, text).
struct SearchPage {
    path: String,
    title: String,
    sections: Vec<(Option<String>, String, String)>,
}

fn search_pages() -> &'static [SearchPage] {
    static S: OnceLock<Vec<SearchPage>> = OnceLock::new();
    S.get_or_init(|| {
        let mut out = Vec::new();
        for (path, bytes) in MANUAL_FILES {
            if !path.ends_with(".md") || *path == NAME_INDEX {
                continue;
            }
            let Ok(text) = std::str::from_utf8(bytes) else {
                continue;
            };
            let doc = markdown::parse(text);
            let mut title = String::new();
            let mut sections: Vec<(Option<String>, String, String)> =
                vec![(None, String::new(), String::new())];
            for b in &doc.blocks {
                if let Block::Heading {
                    level,
                    spans,
                    anchor,
                } = b
                {
                    let h = markdown::plain(spans);
                    if *level == 1 && title.is_empty() {
                        title = h.clone();
                    } else {
                        sections.push((Some(anchor.clone()), h.clone(), String::new()));
                    }
                }
                let mut t = String::new();
                markdown::block_text(std::slice::from_ref(b), &mut t);
                sections.last_mut().unwrap().2.push_str(&t.to_lowercase());
            }
            if title.is_empty() {
                title = path.to_string();
            }
            out.push(SearchPage {
                path: path.to_string(),
                title,
                sections,
            });
        }
        out
    })
}

/// The title of a page (its first `#` heading).
pub fn page_title(path: &str) -> Option<String> {
    search_pages()
        .iter()
        .find(|p| p.path == path)
        .map(|p| p.title.clone())
        .or_else(|| {
            (path == NAME_INDEX).then(|| "All functions, variables and constants (A–Z)".to_string())
        })
}

/// One search result.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub label: String,
    /// Smaller text shown after the label (kind, page).
    pub detail: String,
    pub loc: Loc,
}

/// Searches built-in names, page titles and page text for `query`
/// (case-insensitive): exact and leading name matches first.
pub fn search(query: &str) -> Vec<Hit> {
    const MAX: usize = 200;
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut names: Vec<(u8, &(String, Loc, String))> = names()
        .iter()
        .filter_map(|e| {
            let n = e.0.to_lowercase();
            if n == q {
                Some((0, e))
            } else if n.starts_with(&q) {
                Some((1, e))
            } else if n.contains(&q) {
                Some((2, e))
            } else {
                None
            }
        })
        .collect();
    names.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1 .0.len().cmp(&b.1 .0.len()))
            .then_with(|| a.1 .0.cmp(&b.1 .0))
    });
    for (_, (n, loc, kind)) in names.into_iter().take(MAX / 2) {
        out.push(Hit {
            label: n.clone(),
            detail: kind.clone(),
            loc: loc.clone(),
        });
    }
    let pages = search_pages();
    for p in pages {
        if p.title.to_lowercase().contains(&q) {
            out.push(Hit {
                label: p.title.clone(),
                detail: "page".into(),
                loc: Loc::page(&p.path),
            });
        }
    }
    for p in pages {
        if p.title.to_lowercase().contains(&q) {
            continue;
        }
        if let Some((anchor, heading, _)) = p.sections.iter().find(|s| s.2.contains(&q)) {
            let loc = Loc {
                page: p.path.clone(),
                anchor: anchor.clone(),
            };
            if out.iter().any(|h| h.loc == loc) {
                continue;
            }
            out.push(Hit {
                label: if heading.is_empty() {
                    p.title.clone()
                } else {
                    format!("{} › {heading}", p.title)
                },
                detail: "text".into(),
                loc,
            });
        }
    }
    out.truncate(MAX);
    out
}

// ---------------------------------------------------------------------------
// context help requests

fn request_id() -> Id {
    Id::new("help_topic_request")
}

/// Asks the IDE to show the help on `word` (a built-in name; other words are
/// searched for).
pub fn request_topic(ctx: &egui::Context, word: &str) {
    ctx.data_mut(|d| d.insert_temp(request_id(), word.to_string()));
}

/// The pending context help request.
pub fn take_request(ctx: &egui::Context) -> Option<String> {
    ctx.data_mut(|d| d.remove_temp::<String>(request_id()))
}

// ---------------------------------------------------------------------------
// the viewer

/// State of the help window.
#[derive(Default)]
pub struct HelpView {
    pub open: bool,
    raise: bool,
    history: Vec<Loc>,
    pos: usize,
    pub search: String,
    /// Results of `search` (recomputed when it changes).
    hits: Vec<Hit>,
    hits_for: String,
    /// Scroll to this anchor (or to the top) at the next frame.
    scroll: Option<Option<String>>,
    /// Open the contents section of the current page in the tree.
    reveal: bool,
    docs: HashMap<String, Arc<Doc>>,
    images: HashMap<String, Option<(egui::TextureHandle, Vec2)>>,
}

impl HelpView {
    /// Opens the window (in front) at the current page, or the contents page.
    pub fn open_contents(&mut self) {
        self.open = true;
        self.raise = true;
        if self.history.is_empty() {
            self.navigate(Loc::page(HOME));
        }
    }

    /// Opens the reference entry of a built-in name, or searches for the word.
    pub fn open_topic(&mut self, word: &str) {
        match lookup_name(word) {
            Some(loc) => {
                self.open = true;
                self.raise = true;
                self.navigate(loc);
            }
            None => {
                self.open_contents();
                self.search = word.to_string();
            }
        }
    }

    /// Shows `loc` (a new history entry).
    pub fn navigate(&mut self, loc: Loc) {
        if self.history.get(self.pos) == Some(&loc) {
            self.scroll = Some(loc.anchor.clone());
            return;
        }
        if !self.history.is_empty() {
            self.history.truncate(self.pos + 1);
        }
        self.scroll = Some(loc.anchor.clone());
        self.history.push(loc);
        self.pos = self.history.len() - 1;
        self.reveal = true;
    }

    pub fn current(&self) -> Option<&Loc> {
        self.history.get(self.pos)
    }
    pub fn can_back(&self) -> bool {
        self.pos > 0
    }
    pub fn can_forward(&self) -> bool {
        self.pos + 1 < self.history.len()
    }
    pub fn back(&mut self) {
        if self.can_back() {
            self.pos -= 1;
            self.scroll = Some(self.history[self.pos].anchor.clone());
            self.reveal = true;
        }
    }
    pub fn forward(&mut self) {
        if self.can_forward() {
            self.pos += 1;
            self.scroll = Some(self.history[self.pos].anchor.clone());
            self.reveal = true;
        }
    }

    /// Follows a link clicked on the current page.
    fn follow(&mut self, ctx: &egui::Context, target: &str) {
        let from = self
            .current()
            .map(|l| l.page.clone())
            .unwrap_or_else(|| HOME.to_string());
        match resolve(&from, target) {
            // (links to images and other files: nothing to show on their own)
            Some(loc) if loc.page.ends_with(".md") => self.navigate(loc),
            Some(_) => {}
            None => ctx.open_url(egui::OpenUrl::new_tab(target)),
        }
    }

    fn doc(&mut self, path: &str) -> Option<Arc<Doc>> {
        if let Some(d) = self.docs.get(path) {
            return Some(d.clone());
        }
        let d = Arc::new(markdown::parse(page_text(path)?));
        self.docs.insert(path.to_string(), d.clone());
        Some(d)
    }
}

/// Id of the help window.
pub fn window_id() -> Id {
    Id::new("help_window")
}

/// Width of the contents pane.
const PANE_W: f32 = 250.0;
const ROW_H: f32 = 18.0;

/// Shows the help window (when open).
pub fn window(ctx: &egui::Context, view: &mut HelpView) {
    if !view.open {
        return;
    }
    if std::mem::take(&mut view.raise) {
        crate::mdi::bring_to_front(ctx, window_id());
    }
    let title = view
        .current()
        .and_then(|l| page_title(&l.page))
        .map_or("Help".to_string(), |t| format!("Help - {t}"));
    let r = crate::mdi::floating(
        ctx,
        window_id(),
        &title,
        crate::icons::Icon::Help,
        Vec2::new(980.0, 680.0),
        |ui| contents(ui, view),
    );
    if r.close {
        view.open = false;
    }
}

fn contents(ui: &mut Ui, view: &mut HelpView) {
    // toolbar
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if crate::icons::glyph_button(ui, Glyph::Back, "Back", view.can_back()).clicked() {
            view.back();
        }
        if crate::icons::glyph_button(ui, Glyph::Forward, "Forward", view.can_forward()).clicked() {
            view.forward();
        }
        crate::icons::tool_separator(ui);
        if crate::icons::tool_button(ui, crate::icons::Icon::Help, "Contents page", true).clicked()
        {
            view.navigate(Loc::page(HOME));
        }
        if crate::icons::glyph_button(ui, Glyph::Log, "Index of all functions", true).clicked() {
            view.navigate(Loc::page(NAME_INDEX));
        }
    });
    ui.add_space(3.0);
    let area = ui.available_rect_before_wrap();
    let left = Rect::from_min_max(area.min, pos2(area.min.x + PANE_W, area.max.y));
    let right = Rect::from_min_max(pos2(left.max.x + 4.0, area.min.y), area.max);
    // left pane: search box, then results or the contents tree
    let mut lui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(left)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let mut go: Option<Loc> = None;
    let r = lui.add(
        egui::TextEdit::singleline(&mut view.search)
            .hint_text("Search")
            .desired_width(PANE_W - 2.0)
            .id(Id::new("help_search")),
    );
    r.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search the help")
    });
    if view.search != view.hits_for {
        view.hits = search(&view.search);
        view.hits_for = view.search.clone();
    }
    if r.lost_focus() && lui.input(|i| i.key_pressed(egui::Key::Enter)) {
        if let Some(h) = view.hits.first() {
            go = Some(h.loc.clone());
        }
    }
    lui.add_space(3.0);
    let list = lui.available_rect_before_wrap().shrink(1.0);
    lui.painter().rect_filled(list, 0.0, Color32::WHITE);
    instance_ui::chrome::sunken(lui.painter(), list);
    let mut inner = lui.new_child(
        egui::UiBuilder::new()
            .max_rect(list.shrink2(vec2(2.0, 2.0)))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    inner.set_clip_rect(list.shrink(1.0).intersect(ui.clip_rect()));
    egui::ScrollArea::vertical()
        .id_salt("help_tree")
        .auto_shrink(false)
        .show(&mut inner, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if view.search.trim().is_empty() {
                tree_ui(ui, view, &mut go);
            } else {
                if view.hits.is_empty() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("  Nothing found.").weak());
                }
                let cur = view.current().cloned();
                for (k, h) in view.hits.iter().enumerate() {
                    let sel = cur.as_ref() == Some(&h.loc);
                    if row(
                        ui,
                        Id::new(("help_hit", k)),
                        0,
                        None,
                        &h.label,
                        Some(&h.detail),
                        sel,
                        true,
                    )
                    .clicked()
                    {
                        go = Some(h.loc.clone());
                    }
                }
            }
        });
    // right pane: the page
    ui.painter()
        .rect_filled(right.shrink(1.0), 0.0, Color32::WHITE);
    instance_ui::chrome::sunken(ui.painter(), right.shrink(1.0));
    let mut pui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right.shrink(3.0))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    pui.set_clip_rect(right.shrink(2.0).intersect(ui.clip_rect()));
    let cur = view.current().cloned().unwrap_or_else(|| Loc::page(HOME));
    let scroll = view.scroll.take();
    let mut sa = egui::ScrollArea::vertical()
        .id_salt("help_page")
        .auto_shrink(false);
    if matches!(scroll, Some(None)) {
        sa = sa.vertical_scroll_offset(0.0);
    }
    let doc = view.doc(&cur.page);
    let mut clicked = None;
    let mut images = std::mem::take(&mut view.images);
    sa.show(&mut pui, |ui| {
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: 14,
                right: 18,
                top: 10,
                bottom: 24,
            })
            .show(ui, |ui| {
                ui.set_width((ui.available_width()).max(200.0));
                match &doc {
                    Some(doc) => {
                        let page = cur.page.clone();
                        let mut image_fn =
                            |ctx: &egui::Context, src: &str| image(ctx, &mut images, &page, src);
                        let mut cx = RenderCx {
                            scroll_to: scroll.clone().flatten(),
                            clicked: None,
                            image: &mut image_fn,
                            size: 14.0,
                            n: 0,
                        };
                        markdown::blocks_ui(ui, &doc.blocks, &mut cx);
                        clicked = cx.clicked;
                    }
                    None => missing_page(ui, &cur.page),
                }
            });
    });
    view.images = images;
    if let Some(l) = clicked {
        view.follow(ui.ctx(), &l);
    }
    if let Some(l) = go {
        view.navigate(l);
    }
    ui.allocate_rect(area, Sense::hover());
}

fn missing_page(ui: &mut Ui, path: &str) {
    let title = toc()
        .iter()
        .flat_map(|s| s.pages.iter())
        .find(|(_, p)| p == path)
        .map(|(t, _)| t.clone())
        .unwrap_or_else(|| path.to_string());
    ui.label(
        egui::RichText::new(title)
            .size(23.0)
            .family(instance_ui::theme::bold())
            .color(Color32::from_gray(20)),
    );
    ui.add_space(10.0);
    ui.label(
        egui::RichText::new("This page has not been written yet.")
            .size(14.0)
            .color(Color32::from_gray(80)),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Use the contents on the left or the search box to find other topics.")
            .size(14.0)
            .color(Color32::from_gray(80)),
    );
}

/// The texture of an image of `page`.
fn image(
    ctx: &egui::Context,
    cache: &mut HashMap<String, Option<(egui::TextureHandle, Vec2)>>,
    page: &str,
    src: &str,
) -> Option<(egui::TextureId, Vec2)> {
    let loc = resolve(page, src)?;
    let entry = cache.entry(loc.page.clone()).or_insert_with(|| {
        let bytes = file(&loc.page)?;
        let img = image::load_from_memory(bytes).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        let tex = ctx.load_texture(
            format!("help:{}", loc.page),
            ci,
            egui::TextureOptions::LINEAR,
        );
        Some((tex, vec2(size[0] as f32, size[1] as f32)))
    });
    entry.as_ref().map(|(t, s)| (t.id(), *s))
}

/// The contents tree: the manual's sections (books) and their pages.
fn tree_ui(ui: &mut Ui, view: &mut HelpView, go: &mut Option<Loc>) {
    let cur = view.current().map(|l| l.page.clone()).unwrap_or_default();
    let reveal = std::mem::take(&mut view.reveal);
    if row(
        ui,
        Id::new("help_toc_home"),
        0,
        Some(TreeIcon::Page),
        "Contents",
        None,
        cur == HOME,
        true,
    )
    .clicked()
    {
        *go = Some(Loc::page(HOME));
    }
    for (k, sec) in toc().iter().enumerate() {
        let id = Id::new(("help_toc", k));
        let mut open = ui.data_mut(|d| *d.get_persisted_mut_or(id, false));
        let has_cur = sec.pages.iter().any(|(_, p)| *p == cur);
        if reveal && has_cur && !open {
            open = true;
            ui.data_mut(|d| d.insert_persisted(id, true));
        }
        let r = row(
            ui,
            id.with("row"),
            0,
            Some(TreeIcon::Book(open)),
            &sec.title,
            None,
            false,
            true,
        );
        if r.clicked() {
            open = !open;
            ui.data_mut(|d| d.insert_persisted(id, open));
        }
        if open {
            for (t, p) in &sec.pages {
                let exists = file(p).is_some();
                let r = row(
                    ui,
                    id.with(p),
                    1,
                    Some(TreeIcon::Page),
                    t,
                    None,
                    *p == cur,
                    exists,
                );
                if r.clicked() {
                    *go = Some(Loc::page(p));
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum TreeIcon {
    Book(bool),
    Page,
}

/// A row of the contents tree or the result list: icon, label (highlighted when
/// selected; `exists = false` greys it out), optional detail text. The hover and
/// selection backgrounds are painted inside the fixed row, so nothing moves.
#[allow(clippy::too_many_arguments)]
fn row(
    ui: &mut Ui,
    id: Id,
    depth: usize,
    icon: Option<TreeIcon>,
    label: &str,
    detail: Option<&str>,
    selected: bool,
    exists: bool,
) -> egui::Response {
    let width = ui.available_width().max(60.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
    let mut x = rect.min.x + 3.0 + depth as f32 * 16.0;
    let cy = rect.center().y;
    let p = ui.painter().clone();
    if let Some(icon) = icon {
        let ir = Rect::from_min_size(pos2(x, cy - 8.0), vec2(16.0, 16.0));
        match icon {
            TreeIcon::Book(open) => crate::icons::paint(
                &p,
                ir,
                if open {
                    crate::icons::Icon::FolderOpen
                } else {
                    crate::icons::Icon::Folder
                },
                true,
            ),
            TreeIcon::Page => instance_ui::icons::paint(&p, ir.shrink(1.0), Glyph::Log, exists),
        }
        x = ir.max.x + 3.0;
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    let galley = p.layout_no_wrap(label.to_string(), font.clone(), Color32::BLACK);
    let label_r = Rect::from_min_size(
        pos2(x, rect.min.y + 1.0),
        vec2(galley.size().x + 6.0, ROW_H - 2.0),
    );
    let hit = Rect::from_min_max(pos2(rect.min.x, rect.min.y), pos2(rect.max.x, rect.max.y));
    let r = ui.interact(hit, id, Sense::click());
    r.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, label)
    });
    let color = if selected {
        p.rect_filled(label_r, 0.0, instance_ui::theme::SELECTION);
        Color32::WHITE
    } else {
        if r.hovered() {
            p.rect_filled(label_r, 0.0, Color32::from_rgb(229, 243, 255));
        }
        if exists {
            Color32::BLACK
        } else {
            Color32::from_gray(130)
        }
    };
    p.galley_with_override_text_color(pos2(x + 3.0, cy - galley.size().y / 2.0), galley, color);
    if let Some(d) = detail {
        let dx = label_r.max.x + 6.0;
        if dx < rect.max.x - 20.0 {
            p.text(
                pos2(dx, cy),
                egui::Align2::LEFT_CENTER,
                d,
                egui::FontId::proportional(11.0),
                Color32::from_gray(120),
            );
        }
    }
    r
}

#[cfg(test)]
mod tests;
