//! Text tool rasterisation: system TrueType fonts (or the UI's built-in font) through
//! `fontdue`, rendered into a coverage mask.

use std::path::PathBuf;
use std::sync::Arc;

use super::raster::Mask;

/// A font choice of the text tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontSource {
    /// The proportional font bundled with the UI toolkit.
    Builtin,
    /// A font file on disk.
    File(PathBuf),
}

impl FontSource {
    pub fn label(&self) -> String {
        match self {
            FontSource::Builtin => "Default".into(),
            FontSource::File(p) => p.file_stem().map_or_else(
                || p.display().to_string(),
                |s| s.to_string_lossy().into_owned(),
            ),
        }
    }
}

/// Font files found in the usual system font directories (sorted by name).
pub fn system_fonts() -> Vec<FontSource> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(w) = std::env::var_os("WINDIR") {
        dirs.push(PathBuf::from(w).join("Fonts"));
    }
    dirs.push("/usr/share/fonts".into());
    dirs.push("/usr/local/share/fonts".into());
    dirs.push("/Library/Fonts".into());
    dirs.push("/System/Library/Fonts".into());
    let mut out = Vec::new();
    let mut stack = dirs;
    let mut budget = 4000usize;
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            budget = budget.saturating_sub(1);
            if budget == 0 {
                break;
            }
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x.eq_ignore_ascii_case("ttf") || x.eq_ignore_ascii_case("otf"))
            {
                out.push(FontSource::File(p));
            }
        }
    }
    out.sort_by_key(|f| f.label().to_lowercase());
    out.dedup();
    out
}

/// Bytes of the UI toolkit's default proportional font.
fn builtin_font_bytes() -> Option<Vec<u8>> {
    let defs = eframe::egui::FontDefinitions::default();
    let fam = defs
        .families
        .get(&eframe::egui::FontFamily::Proportional)?
        .first()?
        .clone();
    Some(defs.font_data.get(&fam)?.font.to_vec())
}

/// Load a font.
pub fn load_font(src: &FontSource) -> Result<Arc<fontdue::Font>, String> {
    let bytes = match src {
        FontSource::Builtin => builtin_font_bytes().ok_or("no built-in font")?,
        FontSource::File(p) => std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?,
    };
    fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

/// Horizontal alignment of text lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// Rasterise `text` (lines separated by `\n` or `#`) with its top-left (or top-centre /
/// top-right, depending on `align`) at (x, y) into `mask`.
#[allow(clippy::too_many_arguments)]
pub fn draw_text(
    mask: &mut Mask,
    font: &fontdue::Font,
    text: &str,
    px: f32,
    x: i32,
    y: i32,
    align: Align,
    antialias: bool,
) {
    let lm = font.horizontal_line_metrics(px);
    let (ascent, line_h) = lm.map_or((px * 0.8, px * 1.2), |m| (m.ascent, m.new_line_size));
    let text = text.replace('#', "\n");
    for (li, line) in text.lines().enumerate() {
        let width: f32 = line
            .chars()
            .map(|c| font.metrics(c, px).advance_width)
            .sum();
        let mut pen = match align {
            Align::Left => x as f32,
            Align::Center => x as f32 - width / 2.0,
            Align::Right => x as f32 - width,
        };
        let base = y as f32 + ascent + li as f32 * line_h;
        for ch in line.chars() {
            let (m, bmp) = font.rasterize(ch, px);
            let gx = (pen + m.xmin as f32).round() as i32;
            let gy = (base - m.ymin as f32 - m.height as f32).round() as i32;
            if antialias {
                mask.blit_coverage(gx, gy, m.width, &bmp);
            } else {
                let hard: Vec<u8> = bmp
                    .iter()
                    .map(|&v| if v >= 128 { 255 } else { 0 })
                    .collect();
                mask.blit_coverage(gx, gy, m.width, &hard);
            }
            pen += m.advance_width;
        }
    }
}
