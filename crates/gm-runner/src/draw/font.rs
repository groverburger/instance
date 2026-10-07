//! Text rendering with system TrueType fonts (fontdue), runtime.md §13.4 /
//! builtins 405_05.
//!
//! GameMaker fonts are identified by a family name, point size and bold/italic
//! flags. We look the family up in the Windows font directory and rasterise glyphs
//! at `size * 96 / 72` pixels per em.

use super::Texture;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// A rasterised glyph.
pub struct Glyph {
    pub tex: Rc<Texture>,
    /// Offset of the bitmap's left edge from the pen position.
    pub xmin: f32,
    /// Offset of the bitmap's top edge below the baseline (negative = above).
    pub ytop: f32,
    pub advance: f32,
}

/// A resolved font face at a size.
#[derive(Clone)]
pub struct Face {
    pub id: usize,
    pub px: f32,
    pub ascent: f32,
    pub line_height: f32,
    pub antialias: bool,
}

/// Font lookup and glyph cache.
pub struct FontSystem {
    dirs: Vec<PathBuf>,
    scanned: Option<HashMap<(String, bool, bool), (PathBuf, u32)>>,
    fonts: Vec<fontdue::Font>,
    by_file: HashMap<(PathBuf, u32), usize>,
    faces: HashMap<(String, i32, bool, bool, bool), Option<Face>>,
    glyphs: HashMap<(usize, u32, char, bool), Rc<Glyph>>,
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}

fn known_file(family: &str, bold: bool, italic: bool) -> Option<&'static str> {
    let f = family.to_ascii_lowercase();
    let sel = |r, b, i, bi| match (bold, italic) {
        (false, false) => r,
        (true, false) => b,
        (false, true) => i,
        (true, true) => bi,
    };
    Some(match f.as_str() {
        "arial" => sel("arial.ttf", "arialbd.ttf", "ariali.ttf", "arialbi.ttf"),
        "times new roman" | "times" => sel("times.ttf", "timesbd.ttf", "timesi.ttf", "timesbi.ttf"),
        "courier new" | "courier" => sel("cour.ttf", "courbd.ttf", "couri.ttf", "courbi.ttf"),
        "verdana" => sel("verdana.ttf", "verdanab.ttf", "verdanai.ttf", "verdanaz.ttf"),
        "tahoma" | "ms sans serif" | "microsoft sans serif" | "system" => {
            sel("tahoma.ttf", "tahomabd.ttf", "tahoma.ttf", "tahomabd.ttf")
        }
        "comic sans ms" => sel("comic.ttf", "comicbd.ttf", "comici.ttf", "comicz.ttf"),
        "georgia" => sel("georgia.ttf", "georgiab.ttf", "georgiai.ttf", "georgiaz.ttf"),
        "trebuchet ms" => sel("trebuc.ttf", "trebucbd.ttf", "trebucit.ttf", "trebucbi.ttf"),
        "impact" => "impact.ttf",
        "lucida console" => "lucon.ttf",
        "segoe ui" => sel("segoeui.ttf", "segoeuib.ttf", "segoeuii.ttf", "segoeuiz.ttf"),
        "calibri" => sel("calibri.ttf", "calibrib.ttf", "calibrii.ttf", "calibriz.ttf"),
        "consolas" => sel("consola.ttf", "consolab.ttf", "consolai.ttf", "consolaz.ttf"),
        _ => return None,
    })
}

impl FontSystem {
    pub fn new() -> Self {
        let mut dirs = Vec::new();
        if let Ok(w) = std::env::var("WINDIR") {
            dirs.push(PathBuf::from(w).join("Fonts"));
        }
        dirs.push(PathBuf::from("C:\\Windows\\Fonts"));
        if let Ok(l) = std::env::var("LOCALAPPDATA") {
            dirs.push(PathBuf::from(l).join("Microsoft\\Windows\\Fonts"));
        }
        dirs.push(PathBuf::from("/usr/share/fonts/truetype/dejavu"));
        dirs.push(PathBuf::from("/usr/share/fonts/TTF"));
        FontSystem {
            dirs,
            scanned: None,
            fonts: Vec::new(),
            by_file: HashMap::new(),
            faces: HashMap::new(),
            glyphs: HashMap::new(),
        }
    }

    fn scan(&mut self) -> &HashMap<(String, bool, bool), (PathBuf, u32)> {
        if self.scanned.is_none() {
            let mut map = HashMap::new();
            for d in &self.dirs {
                let Ok(rd) = std::fs::read_dir(d) else { continue };
                for e in rd.flatten() {
                    let p = e.path();
                    let ext = p
                        .extension()
                        .map(|e| e.to_string_lossy().to_ascii_lowercase())
                        .unwrap_or_default();
                    if ext != "ttf" && ext != "otf" && ext != "ttc" {
                        continue;
                    }
                    let Ok(data) = std::fs::read(&p) else { continue };
                    let n = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
                    for idx in 0..n {
                        let Ok(face) = ttf_parser::Face::parse(&data, idx) else { continue };
                        let mut family = None;
                        for name in face.names() {
                            if name.name_id == ttf_parser::name_id::FAMILY && name.is_unicode() {
                                family = name.to_string();
                                if family.is_some() {
                                    break;
                                }
                            }
                        }
                        if let Some(f) = family {
                            map.entry((f.to_ascii_lowercase(), face.is_bold(), face.is_italic()))
                                .or_insert((p.clone(), idx));
                        }
                    }
                }
            }
            self.scanned = Some(map);
        }
        self.scanned.as_ref().unwrap()
    }

    fn find_file(&mut self, family: &str, bold: bool, italic: bool) -> Option<(PathBuf, u32)> {
        if let Some(f) = known_file(family, bold, italic) {
            for d in &self.dirs {
                let p = d.join(f);
                if p.exists() {
                    return Some((p, 0));
                }
            }
        }
        let fam = family.to_ascii_lowercase();
        let map = self.scan();
        for (b, i) in [(bold, italic), (bold, false), (false, italic), (false, false)] {
            if let Some(r) = map.get(&(fam.clone(), b, i)) {
                return Some(r.clone());
            }
        }
        None
    }

    fn load(&mut self, path: PathBuf, idx: u32) -> Option<usize> {
        if let Some(&i) = self.by_file.get(&(path.clone(), idx)) {
            return Some(i);
        }
        let data = std::fs::read(&path).ok()?;
        let settings = fontdue::FontSettings {
            collection_index: idx,
            ..Default::default()
        };
        let font = fontdue::Font::from_bytes(data, settings).ok()?;
        self.fonts.push(font);
        let i = self.fonts.len() - 1;
        self.by_file.insert((path, idx), i);
        Some(i)
    }

    /// Resolves a font; falls back to Arial and then to any loadable font.
    pub fn face(&mut self, family: &str, size: i32, bold: bool, italic: bool, antialias: bool) -> Option<Face> {
        let key = (family.to_ascii_lowercase(), size, bold, italic, antialias);
        if let Some(f) = self.faces.get(&key) {
            return f.clone();
        }
        let mut file = self.find_file(family, bold, italic);
        if file.is_none() {
            file = self.find_file("Arial", bold, italic);
        }
        if file.is_none() {
            // any font at all
            let map = self.scan();
            file = map.values().next().cloned();
        }
        let face = file.and_then(|(p, i)| self.load(p, i)).map(|id| {
            let px = (size.max(1) as f32) * 96.0 / 72.0;
            let lm = self.fonts[id].horizontal_line_metrics(px);
            let (ascent, line_height) = match lm {
                Some(m) => (m.ascent, m.ascent - m.descent),
                None => (px * 0.9, px * 1.15),
            };
            Face {
                id,
                px,
                ascent: ascent.round(),
                line_height: line_height.round().max(1.0),
                antialias,
            }
        });
        self.faces.insert(key, face.clone());
        face
    }

    /// A rasterised glyph of a face.
    pub fn glyph(&mut self, face: &Face, ch: char) -> Rc<Glyph> {
        let key = (face.id, face.px.to_bits(), ch, face.antialias);
        if let Some(g) = self.glyphs.get(&key) {
            return g.clone();
        }
        let font = &self.fonts[face.id];
        let (m, cov) = font.rasterize(ch, face.px);
        let mut tex = Texture::new(m.width as u32, m.height as u32);
        for (i, c) in cov.iter().enumerate() {
            let a = if face.antialias {
                *c
            } else if *c >= 128 {
                255
            } else {
                0
            };
            tex.px[i] = [255, 255, 255, a];
        }
        let g = Rc::new(Glyph {
            tex: Rc::new(tex),
            xmin: m.xmin as f32,
            ytop: -(m.ymin as f32 + m.height as f32),
            advance: m.advance_width,
        });
        self.glyphs.insert(key, g.clone());
        g
    }

    /// Advance width of a character.
    pub fn advance(&mut self, face: &Face, ch: char) -> f32 {
        self.fonts[face.id].metrics(ch, face.px).advance_width
    }
}

/// Decodes a GML (Windows-1252) string into lines, applying the `#` newline rule
/// (`\#` is a literal `#`; CR, LF and CRLF also break lines).
pub fn split_lines(s: &[u8]) -> Vec<Vec<char>> {
    let text = gm_format::text::decode_cp1252(s);
    let mut lines = vec![Vec::new()];
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if chars.get(i + 1) == Some(&'#') => {
                lines.last_mut().unwrap().push('#');
                i += 1;
            }
            '#' | '\n' => lines.push(Vec::new()),
            '\r' => {
                if chars.get(i + 1) == Some(&'\n') {
                    i += 1;
                }
                lines.push(Vec::new());
            }
            _ => lines.last_mut().unwrap().push(c),
        }
        i += 1;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines() {
        let l = split_lines(b"a#b\\#c\r\nd");
        assert_eq!(l.len(), 3);
        assert_eq!(l[1], vec!['b', '#', 'c']);
    }
}
