//! A small Markdown document model for the help viewer: parsed with `pulldown-cmark`
//! into blocks and inline spans, rendered with egui widgets (headings, paragraphs,
//! emphasis, inline code, code blocks with GML colouring, lists, tables, block quotes,
//! images and links).

use eframe::egui::{self, Color32, FontId, RichText, Ui, Vec2};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// Inline content.
#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Text {
        text: String,
        bold: bool,
        italic: bool,
        strike: bool,
        code: bool,
        link: Option<String>,
    },
    Image {
        src: String,
        alt: String,
        link: Option<String>,
    },
    Break,
}

/// Block content.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        spans: Vec<Inline>,
        /// GitHub-style anchor (unique within the page).
        anchor: String,
    },
    Paragraph(Vec<Inline>),
    Code {
        lang: String,
        text: String,
    },
    List {
        /// First number of an ordered list.
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    Quote(Vec<Block>),
    Table {
        head: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Rule,
}

/// A parsed page.
#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub blocks: Vec<Block>,
}

/// GitHub's heading anchor of `text`: lowercase, spaces become `-`, punctuation other
/// than `-` and `_` is removed.
pub fn slug(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| {
            if c == ' ' {
                Some('-')
            } else if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}

/// The plain text of inline content.
pub fn plain(spans: &[Inline]) -> String {
    let mut s = String::new();
    for i in spans {
        match i {
            Inline::Text { text, .. } => s.push_str(text),
            Inline::Image { alt, .. } => s.push_str(alt),
            Inline::Break => s.push(' '),
        }
    }
    s
}

enum Frame {
    Blocks(Vec<Block>),
    Quote(Vec<Block>),
    List(Option<u64>, Vec<Vec<Block>>),
    Item(Vec<Block>),
}

#[derive(Default)]
struct TableB {
    head: Vec<Vec<Inline>>,
    rows: Vec<Vec<Vec<Inline>>>,
    row: Vec<Vec<Inline>>,
}

struct Builder {
    stack: Vec<Frame>,
    inl: Vec<Inline>,
    bold: u32,
    italic: u32,
    strike: u32,
    links: Vec<String>,
    heading: Option<u8>,
    code: Option<(String, String)>,
    image: Option<(String, String)>,
    table: Option<TableB>,
    anchors: std::collections::HashMap<String, usize>,
}

impl Builder {
    fn blocks(&mut self) -> &mut Vec<Block> {
        match self.stack.last_mut().unwrap() {
            Frame::Blocks(b) | Frame::Quote(b) | Frame::Item(b) => b,
            Frame::List(..) => unreachable!("blocks inside a list without item"),
        }
    }

    /// Inline content collected outside a paragraph (tight list items) becomes a
    /// paragraph.
    fn flush(&mut self) {
        if !self.inl.is_empty() && self.table.is_none() && self.heading.is_none() {
            let spans = std::mem::take(&mut self.inl);
            if !matches!(self.stack.last(), Some(Frame::List(..))) {
                self.blocks().push(Block::Paragraph(spans));
            }
        }
    }

    fn text(&mut self, text: &str, code: bool) {
        if let Some((_, alt)) = &mut self.image {
            alt.push_str(text);
            return;
        }
        if let Some((_, buf)) = &mut self.code {
            buf.push_str(text);
            return;
        }
        let link = self.links.last().cloned();
        let (bold, italic, strike) = (self.bold > 0, self.italic > 0, self.strike > 0);
        // merge with the previous span of the same style
        if let Some(Inline::Text {
            text: t,
            bold: b,
            italic: i,
            strike: s,
            code: c,
            link: l,
        }) = self.inl.last_mut()
        {
            if *b == bold && *i == italic && *s == strike && *c == code && *l == link {
                t.push_str(text);
                return;
            }
        }
        self.inl.push(Inline::Text {
            text: text.to_string(),
            bold,
            italic,
            strike,
            code,
            link,
        });
    }

    fn anchor(&mut self, text: &str) -> String {
        let base = slug(text);
        let n = self.anchors.entry(base.clone()).or_insert(0);
        let a = if *n == 0 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        *n += 1;
        a
    }
}

/// Parses Markdown text.
pub fn parse(src: &str) -> Doc {
    let mut b = Builder {
        stack: vec![Frame::Blocks(Vec::new())],
        inl: Vec::new(),
        bold: 0,
        italic: 0,
        strike: 0,
        links: Vec::new(),
        heading: None,
        code: None,
        image: None,
        table: None,
        anchors: Default::default(),
    };
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    for ev in Parser::new_ext(src, opts) {
        match ev {
            Event::Start(t) => match t {
                Tag::Paragraph => b.flush(),
                Tag::Heading { level, .. } => {
                    b.flush();
                    b.heading = Some(level as u8);
                }
                Tag::CodeBlock(kind) => {
                    b.flush();
                    let lang = match kind {
                        CodeBlockKind::Fenced(l) => l
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_ascii_lowercase(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    b.code = Some((lang, String::new()));
                }
                Tag::List(start) => {
                    b.flush();
                    b.stack.push(Frame::List(start, Vec::new()));
                }
                Tag::Item => b.stack.push(Frame::Item(Vec::new())),
                Tag::BlockQuote(_) => {
                    b.flush();
                    b.stack.push(Frame::Quote(Vec::new()));
                }
                Tag::Table(_) => {
                    b.flush();
                    b.table = Some(TableB::default());
                }
                Tag::TableHead | Tag::TableRow => {
                    if let Some(t) = &mut b.table {
                        t.row.clear();
                    }
                }
                Tag::TableCell => b.inl.clear(),
                Tag::Emphasis => b.italic += 1,
                Tag::Strong => b.bold += 1,
                Tag::Strikethrough => b.strike += 1,
                Tag::Link { dest_url, .. } => b.links.push(dest_url.to_string()),
                Tag::Image { dest_url, .. } => {
                    b.image = Some((dest_url.to_string(), String::new()))
                }
                _ => {}
            },
            Event::End(t) => match t {
                TagEnd::Paragraph => {
                    let spans = std::mem::take(&mut b.inl);
                    b.blocks().push(Block::Paragraph(spans));
                }
                TagEnd::Heading(_) => {
                    let spans = std::mem::take(&mut b.inl);
                    let level = b.heading.take().unwrap_or(1);
                    let anchor = b.anchor(&plain(&spans));
                    b.blocks().push(Block::Heading {
                        level,
                        spans,
                        anchor,
                    });
                }
                TagEnd::CodeBlock => {
                    if let Some((lang, mut text)) = b.code.take() {
                        if text.ends_with('\n') {
                            text.pop();
                        }
                        b.blocks().push(Block::Code { lang, text });
                    }
                }
                TagEnd::Item => {
                    b.flush();
                    if let Some(Frame::Item(blocks)) = b.stack.pop() {
                        if let Some(Frame::List(_, items)) = b.stack.last_mut() {
                            items.push(blocks);
                        }
                    }
                }
                TagEnd::List(_) => {
                    if let Some(Frame::List(start, items)) = b.stack.pop() {
                        b.blocks().push(Block::List { start, items });
                    }
                }
                TagEnd::BlockQuote(_) => {
                    b.flush();
                    if let Some(Frame::Quote(blocks)) = b.stack.pop() {
                        b.blocks().push(Block::Quote(blocks));
                    }
                }
                TagEnd::TableCell => {
                    let cell = std::mem::take(&mut b.inl);
                    if let Some(t) = &mut b.table {
                        t.row.push(cell);
                    }
                }
                TagEnd::TableHead => {
                    if let Some(t) = &mut b.table {
                        t.head = std::mem::take(&mut t.row);
                    }
                }
                TagEnd::TableRow => {
                    if let Some(t) = &mut b.table {
                        let row = std::mem::take(&mut t.row);
                        t.rows.push(row);
                    }
                }
                TagEnd::Table => {
                    if let Some(t) = b.table.take() {
                        b.blocks().push(Block::Table {
                            head: t.head,
                            rows: t.rows,
                        });
                    }
                }
                TagEnd::Emphasis => b.italic = b.italic.saturating_sub(1),
                TagEnd::Strong => b.bold = b.bold.saturating_sub(1),
                TagEnd::Strikethrough => b.strike = b.strike.saturating_sub(1),
                TagEnd::Link => {
                    b.links.pop();
                }
                TagEnd::Image => {
                    if let Some((src, alt)) = b.image.take() {
                        let link = b.links.last().cloned();
                        b.inl.push(Inline::Image { src, alt, link });
                    }
                }
                _ => {}
            },
            Event::Text(t) => b.text(&t, false),
            Event::Code(t) => b.text(&t, true),
            Event::SoftBreak => b.text(" ", false),
            Event::HardBreak => b.inl.push(Inline::Break),
            Event::Rule => {
                b.flush();
                b.blocks().push(Block::Rule);
            }
            Event::InlineHtml(h) | Event::Html(h) => {
                let h = h.trim().to_ascii_lowercase();
                if h == "<br>" || h == "<br/>" || h == "<br />" {
                    b.inl.push(Inline::Break);
                }
            }
            _ => {}
        }
    }
    b.flush();
    let blocks = match b.stack.into_iter().next() {
        Some(Frame::Blocks(v)) => v,
        _ => Vec::new(),
    };
    Doc { blocks }
}

/// The plain text of a block tree (search).
pub fn block_text(blocks: &[Block], out: &mut String) {
    for b in blocks {
        match b {
            Block::Heading { spans, .. } | Block::Paragraph(spans) => {
                out.push_str(&plain(spans));
                out.push('\n');
            }
            Block::Code { text, .. } => {
                out.push_str(text);
                out.push('\n');
            }
            Block::List { items, .. } => {
                for i in items {
                    block_text(i, out);
                }
            }
            Block::Quote(bs) => block_text(bs, out),
            Block::Table { head, rows } => {
                for c in head.iter().chain(rows.iter().flatten()) {
                    out.push_str(&plain(c));
                    out.push(' ');
                }
                out.push('\n');
            }
            Block::Rule => {}
        }
    }
}

// ---------------------------------------------------------------------------
// rendering

/// Colours of the page (the IDE's light theme).
const TEXT: Color32 = Color32::from_gray(20);
const LINK: Color32 = Color32::from_rgb(0, 80, 190);
const CODE_BG: Color32 = Color32::from_rgb(244, 244, 246);
const CODE_FRAME: Color32 = Color32::from_rgb(210, 210, 216);
const RULE: Color32 = Color32::from_gray(200);

/// Looks up the texture (and its size) of an image link of the page.
pub type ImageFn<'a> = dyn FnMut(&egui::Context, &str) -> Option<(egui::TextureId, Vec2)> + 'a;

/// What the renderer needs from the viewer, and what it reports back.
pub struct RenderCx<'a> {
    /// Scroll to the heading with this anchor (taken when found).
    pub scroll_to: Option<String>,
    /// A link that was clicked.
    pub clicked: Option<String>,
    /// Texture of an image, by its link text in the page.
    pub image: &'a mut ImageFn<'a>,
    /// Body text size.
    pub size: f32,
    /// Counter for widget ids.
    pub n: usize,
}

fn heading_size(level: u8, body: f32) -> f32 {
    match level {
        1 => body + 9.0,
        2 => body + 5.0,
        3 => body + 2.0,
        _ => body + 1.0,
    }
}

/// Renders inline spans as wrapped text (`size`: text size; `strong`: all bold).
fn spans_ui(
    ui: &mut Ui,
    spans: &[Inline],
    cx: &mut RenderCx,
    size: f32,
    strong: bool,
) -> egui::Response {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
        // Consecutive text spans of the same link (or of no link) form one label, so
        // that text, bold, italic and code share their baseline and wrap as one text.
        let mut k = 0;
        while k < spans.len() {
            match &spans[k] {
                Inline::Text { link, .. } => {
                    let link = link.clone();
                    let mut job = egui::text::LayoutJob::default();
                    while let Some(Inline::Text {
                        text,
                        bold,
                        italic,
                        strike,
                        code,
                        link: l,
                    }) = spans.get(k)
                    {
                        if *l != link {
                            break;
                        }
                        let font = if *code {
                            FontId::monospace(size)
                        } else if *bold || strong {
                            FontId::new(size, instance_ui::theme::bold())
                        } else {
                            FontId::proportional(size)
                        };
                        let mut f = egui::TextFormat::simple(
                            font,
                            if link.is_some() { LINK } else { TEXT },
                        );
                        f.italics = *italic;
                        if *code {
                            f.background = CODE_BG;
                        }
                        if *strike {
                            f.strikethrough = egui::Stroke::new(1.0, TEXT);
                        }
                        if link.is_some() {
                            f.underline = egui::Stroke::new(1.0, LINK);
                        }
                        job.append(text, 0.0, f);
                        k += 1;
                    }
                    if let Some(l) = link {
                        let r = ui
                            .add(egui::Label::new(job).sense(egui::Sense::click()))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(l.as_str());
                        if r.clicked() {
                            cx.clicked = Some(l);
                        }
                    } else {
                        ui.add(egui::Label::new(job));
                    }
                }
                Inline::Image { src, alt, link } => {
                    image_ui(ui, src, alt, link.as_deref(), cx);
                    k += 1;
                }
                Inline::Break => {
                    ui.end_row();
                    k += 1;
                }
            }
        }
    })
    .response
}

fn image_ui(ui: &mut Ui, src: &str, alt: &str, link: Option<&str>, cx: &mut RenderCx) {
    let ctx = ui.ctx().clone();
    match (cx.image)(&ctx, src) {
        Some((tex, size)) => {
            let max_w = ui.available_width().max(32.0);
            let s = if size.x > max_w { max_w / size.x } else { 1.0 };
            let r = ui.add(egui::Image::new((tex, size * s)).sense(if link.is_some() {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            }));
            if !alt.is_empty() {
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, alt));
            }
            if let Some(l) = link {
                if r.clicked() {
                    cx.clicked = Some(l.to_string());
                }
            }
        }
        None => {
            ui.label(
                RichText::new(format!(
                    "[image: {}]",
                    if alt.is_empty() { src } else { alt }
                ))
                .italics()
                .color(Color32::GRAY),
            );
        }
    }
}

/// Renders blocks.
pub fn blocks_ui(ui: &mut Ui, blocks: &[Block], cx: &mut RenderCx) {
    for (i, b) in blocks.iter().enumerate() {
        cx.n += 1;
        let gap = if i == 0 { 0.0 } else { 6.0 };
        match b {
            Block::Heading {
                level,
                spans,
                anchor,
            } => {
                ui.add_space(gap + if *level <= 2 { 8.0 } else { 4.0 });
                let r = spans_ui(ui, spans, cx, heading_size(*level, cx.size), true);
                if *level <= 2 {
                    let y = r.rect.bottom() + 2.0;
                    ui.painter()
                        .hline(ui.max_rect().x_range(), y, egui::Stroke::new(1.0, RULE));
                    ui.add_space(4.0);
                }
                if cx.scroll_to.as_deref() == Some(anchor.as_str()) {
                    cx.scroll_to = None;
                    ui.scroll_to_rect(r.rect.expand(4.0), Some(egui::Align::TOP));
                }
            }
            Block::Paragraph(spans) => {
                ui.add_space(gap);
                spans_ui(ui, spans, cx, cx.size, false);
            }
            Block::Code { lang, text } => {
                ui.add_space(gap);
                code_ui(ui, lang, text, cx);
            }
            Block::List { start, items } => {
                ui.add_space(gap);
                for (k, item) in items.iter().enumerate() {
                    let marker = match start {
                        Some(n) => format!("{}.", n + k as u64),
                        None => "•".to_string(),
                    };
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        let w = if start.is_some() { 26.0 } else { 18.0 };
                        let (r, _) = ui
                            .allocate_exact_size(Vec2::new(w, cx.size + 4.0), egui::Sense::hover());
                        ui.painter().text(
                            egui::pos2(r.right() - 6.0, r.top() + 1.0),
                            egui::Align2::RIGHT_TOP,
                            marker,
                            FontId::proportional(cx.size),
                            TEXT,
                        );
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            blocks_ui(ui, item, cx);
                        });
                    });
                }
            }
            Block::Quote(inner) => {
                ui.add_space(gap);
                let r = ui
                    .horizontal_top(|ui| {
                        ui.add_space(12.0);
                        ui.vertical(|ui| blocks_ui(ui, inner, cx));
                    })
                    .response;
                ui.painter().vline(
                    r.rect.left() + 3.0,
                    r.rect.y_range(),
                    egui::Stroke::new(3.0, Color32::from_gray(205)),
                );
            }
            Block::Table { head, rows } => {
                ui.add_space(gap);
                table_ui(ui, head, rows, cx);
            }
            Block::Rule => {
                ui.add_space(gap);
                ui.separator();
            }
        }
    }
}

fn code_ui(ui: &mut Ui, lang: &str, text: &str, cx: &mut RenderCx) {
    let font = FontId::monospace(cx.size - 1.0);
    let job = if lang == "gml" {
        crate::widgets::code_editor::layout_job(
            text,
            &crate::widgets::code_editor::ProjectNames::default(),
            true,
            false,
            font,
        )
    } else {
        egui::text::LayoutJob::simple(text.to_string(), font, TEXT, f32::INFINITY)
    };
    egui::Frame::new()
        .fill(CODE_BG)
        .stroke(egui::Stroke::new(1.0, CODE_FRAME))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::ScrollArea::horizontal()
                .id_salt(("help_code", cx.n))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Extend));
                });
        });
}

fn table_ui(ui: &mut Ui, head: &[Vec<Inline>], rows: &[Vec<Vec<Inline>>], cx: &mut RenderCx) {
    let ncols = head
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    if ncols == 0 {
        return;
    }
    // column widths: at least the longest word of each column, the rest shared in
    // proportion to how much more the column's longest text needs
    let total = ui.available_width() - 2.0;
    let pad = 6.0;
    let painter = ui.painter().clone();
    let measure = |text: &str, code: bool, bold: bool| -> f32 {
        let font = if code {
            FontId::monospace(cx.size)
        } else if bold {
            FontId::new(cx.size, instance_ui::theme::bold())
        } else {
            FontId::proportional(cx.size)
        };
        painter
            .layout_no_wrap(text.to_string(), font, TEXT)
            .size()
            .x
    };
    let mut min = vec![24.0f32; ncols];
    let mut natural = vec![24.0f32; ncols];
    let all_rows = std::iter::once((head, true)).chain(rows.iter().map(|r| (r.as_slice(), false)));
    for (row, header) in all_rows {
        for (c, cell) in row.iter().enumerate().take(ncols) {
            let mut full = 0.0;
            for s in cell {
                if let Inline::Text {
                    text, code, bold, ..
                } = s
                {
                    let b = *bold || header;
                    full += measure(text, *code, b);
                    for w in text.split_whitespace() {
                        min[c] = min[c].max(measure(w, *code, b) + 2.0 * pad + 2.0);
                    }
                }
            }
            natural[c] = natural[c].max(full + 2.0 * pad + 2.0);
        }
    }
    let nat_sum: f32 = natural.iter().sum();
    let mut widths: Vec<f32> = if nat_sum <= total {
        natural.clone()
    } else {
        let min_sum: f32 = min.iter().sum();
        let extra: f32 = natural
            .iter()
            .zip(&min)
            .map(|(n, m)| (n - m).max(0.0))
            .sum();
        let k = ((total - min_sum) / extra.max(1.0)).clamp(0.0, 1.0);
        natural
            .iter()
            .zip(&min)
            .map(|(n, m)| m + (n - m).max(0.0) * k)
            .collect()
    };
    let used: f32 = widths.iter().sum();
    if used < total {
        *widths.last_mut().unwrap() += total - used;
    }
    let widths: Vec<f32> = widths.iter().map(|w| w.floor()).collect();
    let line = egui::Stroke::new(1.0, CODE_FRAME);
    let row_ui =
        |ui: &mut Ui, cells: &[Vec<Inline>], header: bool, stripe: bool, cx: &mut RenderCx| {
            let bg = ui.painter().add(egui::Shape::Noop);
            let r = ui
                .horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (c, w) in widths.iter().enumerate() {
                        let cell = cells.get(c).map(|v| v.as_slice()).unwrap_or(&[]);
                        ui.allocate_ui_with_layout(
                            Vec2::new(*w, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(*w);
                                egui::Frame::new()
                                    .inner_margin(egui::Margin::symmetric(pad as i8, 3))
                                    .show(ui, |ui| {
                                        ui.set_width(*w - 2.0 * pad);
                                        spans_ui(ui, cell, cx, cx.size, header);
                                    });
                            },
                        );
                    }
                })
                .response;
            let rect = egui::Rect::from_min_size(
                r.rect.min,
                Vec2::new(widths.iter().sum(), r.rect.height()),
            );
            let fill = if header {
                Color32::from_rgb(232, 232, 236)
            } else if stripe {
                Color32::from_rgb(248, 248, 250)
            } else {
                Color32::WHITE
            };
            ui.painter()
                .set(bg, egui::Shape::rect_filled(rect, 0.0, fill));
            ui.painter()
                .rect_stroke(rect, 0.0, line, egui::StrokeKind::Inside);
        };
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if !head.is_empty() {
            row_ui(ui, head, true, false, cx);
        }
        for (k, row) in rows.iter().enumerate() {
            row_ui(ui, row, false, k % 2 == 1, cx);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_like_github() {
        assert_eq!(slug("draw_sprite"), "draw_sprite");
        assert_eq!(slug("Drawing shapes"), "drawing-shapes");
        assert_eq!(
            slug("Constants: drawing shapes"),
            "constants-drawing-shapes"
        );
        assert_eq!(
            slug("Addressing other instances: `with`, `other`"),
            "addressing-other-instances-with-other"
        );
        assert_eq!(slug("A – B"), "a--b");
    }

    #[test]
    fn parses_blocks() {
        let d = parse(
            "# Title\n\nSome *em* and **bold** `code` [link](a.md#x).\n\n- one\n- two\n\n1. a\n2. b\n\n```gml\nx = 1;\n```\n\n> quote\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n## Title\n",
        );
        let b = &d.blocks;
        assert!(matches!(&b[0], Block::Heading { level: 1, anchor, .. } if anchor == "title"));
        let Block::Paragraph(p) = &b[1] else {
            panic!("{:?}", b[1])
        };
        assert!(p
            .iter()
            .any(|i| matches!(i, Inline::Text { italic: true, text, .. } if text == "em")));
        assert!(p
            .iter()
            .any(|i| matches!(i, Inline::Text { bold: true, text, .. } if text == "bold")));
        assert!(p
            .iter()
            .any(|i| matches!(i, Inline::Text { code: true, text, .. } if text == "code")));
        assert!(p
            .iter()
            .any(|i| matches!(i, Inline::Text { link: Some(l), .. } if l == "a.md#x")));
        assert!(matches!(&b[2], Block::List { start: None, items } if items.len() == 2));
        assert!(matches!(&b[3], Block::List { start: Some(1), items } if items.len() == 2));
        assert!(matches!(&b[4], Block::Code { lang, text } if lang == "gml" && text == "x = 1;"));
        assert!(matches!(&b[5], Block::Quote(_)));
        assert!(matches!(&b[6], Block::Table { head, rows } if head.len() == 2 && rows.len() == 1));
        // the second heading with the same text gets a suffix
        assert!(matches!(&b[7], Block::Heading { anchor, .. } if anchor == "title-1"));
    }
}
