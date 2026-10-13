//! Export a game as one self-contained web page: the web runner (the wasm-bindgen
//! glue and the WebAssembly module built by `scripts/build-web.*`), the game and,
//! when the game plays MIDI, a SoundFont, all in a single `.html` file that runs
//! from disk or any web server. Each binary payload is gzip-compressed and base64
//! encoded; the page unpacks them with the browser's `DecompressionStream`.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::Project;

/// File names of the web runner inside its folder (`dist/web/pkg`).
pub const RUNNER_GLUE: &str = "instance_runner.js";
pub const RUNNER_WASM: &str = "instance_runner_bg.wasm";

/// The web runner: its JavaScript glue and WebAssembly module.
pub struct WebRunner {
    pub glue: String,
    pub wasm: Vec<u8>,
}

impl WebRunner {
    /// Reads the runner from a folder holding [`RUNNER_GLUE`] and [`RUNNER_WASM`].
    pub fn read(dir: &Path) -> Result<Self, String> {
        let glue = dir.join(RUNNER_GLUE);
        let wasm = dir.join(RUNNER_WASM);
        Ok(WebRunner {
            glue: std::fs::read_to_string(&glue).map_err(|e| format!("{}: {e}", glue.display()))?,
            wasm: std::fs::read(&wasm).map_err(|e| format!("{}: {e}", wasm.display()))?,
        })
    }
}

/// Folders above the running executable, nearest first (the executable's own
/// folder, then its parents: covers `target/release` in a checkout).
fn exe_ancestors() -> Vec<PathBuf> {
    let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) else {
        return Vec::new();
    };
    dir.ancestors().take(4).map(Path::to_path_buf).collect()
}

/// Finds the web runner folder: `explicit` if given, else `$INSTANCE_WEB_RUNNER`,
/// else `web/pkg`, `pkg` or `dist/web/pkg` next to the executable or above it.
pub fn find_runner(explicit: Option<&Path>) -> Option<PathBuf> {
    let has = |d: &Path| d.join(RUNNER_GLUE).is_file() && d.join(RUNNER_WASM).is_file();
    if let Some(d) = explicit {
        return Some(d.to_path_buf());
    }
    if let Some(d) = std::env::var_os("INSTANCE_WEB_RUNNER") {
        return Some(PathBuf::from(d));
    }
    exe_ancestors()
        .into_iter()
        .flat_map(|a| [a.join("web").join("pkg"), a.join("pkg"), a.join("dist").join("web").join("pkg")])
        .find(|d| has(d))
}

/// Finds a SoundFont for MIDI music: `$INSTANCE_SOUNDFONT` (or `$GM81_SOUNDFONT`),
/// then `soundfonts/default.sf2` beside the web runner folder, next to the
/// executable or above it (`web/soundfonts` in a checkout).
pub fn find_soundfont(runner_dir: Option<&Path>) -> Option<PathBuf> {
    for v in ["INSTANCE_SOUNDFONT", "GM81_SOUNDFONT"] {
        if let Some(p) = std::env::var_os(v) {
            return Some(PathBuf::from(p));
        }
    }
    let sf = |d: &Path| d.join("soundfonts").join("default.sf2");
    let mut cands = Vec::new();
    if let Some(parent) = runner_dir.and_then(Path::parent) {
        cands.push(sf(parent));
    }
    for a in exe_ancestors() {
        cands.push(sf(&a));
        cands.push(sf(&a.join("web")));
    }
    cands.into_iter().find(|p| p.is_file())
}

/// Whether the game has MIDI sounds (they need a SoundFont to be heard).
pub fn uses_midi(p: &Project) -> bool {
    p.sounds.iter().flatten().any(|s| {
        let t = s.file_type.trim_start_matches('.').to_ascii_lowercase();
        (t == "mid" || t == "midi" || t == "rmi") && s.data.as_ref().is_some_and(|d| !d.is_empty())
    })
}

fn gzip_base64(data: &[u8]) -> String {
    let mut z = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    z.write_all(data).expect("writing to memory");
    base64(&z.finish().expect("writing to memory"))
}

fn base64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        s.push(A[(n >> 18) as usize & 63] as char);
        s.push(A[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if c.len() > 2 { A[n as usize & 63] as char } else { '=' });
    }
    s
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The page for `game` (the bytes of a `.gm81`), titled `title`.
pub fn page(title: &str, runner: &WebRunner, game: &[u8], soundfont: Option<&[u8]>) -> String {
    // the glue sits inside a <script>: keep the HTML parser from ending it early
    let glue = runner.glue.replace("</script", "<\\/script").replace("<!--", "<\\!--");
    let title = html_escape(if title.trim().is_empty() { "Game" } else { title });
    let fill = |name: &str| -> String {
        match name {
            "TITLE" => title.clone(),
            "GLUE" => glue.clone(),
            "WASM" => gzip_base64(&runner.wasm),
            "GAME" => gzip_base64(game),
            "SOUNDFONT" => soundfont.map_or_else(|| "null".into(), |sf| format!("\"{}\"", gzip_base64(sf))),
            _ => format!("{{{{{name}}}}}"),
        }
    };
    // one pass over the template, so nothing filled in is itself searched for markers
    let tpl = include_str!("web_page.html");
    let mut out = String::with_capacity(tpl.len() + glue.len() + runner.wasm.len() / 2 + game.len());
    let mut rest = tpl;
    while let Some(i) = rest.find("{{") {
        let Some(j) = rest[i..].find("}}") else { break };
        out.push_str(&rest[..i]);
        out.push_str(&fill(&rest[i + 2..i + j]));
        rest = &rest[i + j + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648() {
        for (i, o) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(i.as_bytes()), o);
        }
    }

    #[test]
    fn page_fills_every_marker() {
        let r = WebRunner {
            glue: "export function start_game() {} // </script> <!--".into(),
            wasm: vec![0, 97, 115, 109],
        };
        let html = page("A <b> game", &r, b"GM81", None);
        assert!(!html.contains("{{"), "unfilled marker");
        assert!(html.contains("<title>A &lt;b&gt; game</title>"));
        assert!(html.contains("<\\/script>") && !html.contains("// </script>"));
        assert!(html.contains("const SOUNDFONT = null;"));
        assert_eq!(html.matches("</script>").count(), 1);
    }
}
