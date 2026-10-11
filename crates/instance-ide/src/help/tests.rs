//! Tests of the manual and the help viewer's model (the UI tests are in `crate::tests`).

use std::path::{Path, PathBuf};

use super::*;

#[path = "../../examples/gen_manual_reference.rs"]
#[allow(dead_code)]
mod gen;

fn manual_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual")
}

fn md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            md_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            out.push(p);
        }
    }
}

/// The committed function reference equals the generator's output
/// (`cargo run -p instance-ide --example gen_manual_reference` regenerates it).
#[test]
fn reference_matches_generator() {
    let (json, dir) = gen::paths();
    let src = std::fs::read_to_string(json).unwrap();
    let files = gen::generate(&src);
    let mut stale = Vec::new();
    for (name, text) in &files {
        let on_disk = std::fs::read_to_string(dir.join(name)).unwrap_or_default();
        if on_disk.replace("\r\n", "\n") != *text {
            stale.push(name.clone());
        }
    }
    assert!(
        stale.is_empty(),
        "docs/manual/reference is out of date ({stale:?}); run `cargo run -p instance-ide --example gen_manual_reference`"
    );
    // every group of the manual's contents has a file
    for (_, page) in toc().iter().flat_map(|s| s.pages.iter()) {
        if let Some(name) = page.strip_prefix("reference/") {
            assert!(
                files.contains_key(name),
                "README links reference/{name}, which is not generated"
            );
        }
    }
}

/// No page of the manual names a third-party product or company (built-in
/// identifiers that contain such a word are allowed).
#[test]
fn manual_names_no_third_party_products() {
    let mut files = Vec::new();
    md_files(&manual_dir(), &mut files);
    assert!(!files.is_empty());
    // (identifiers also appear as lowercase anchors)
    let b = crate::widgets::code_editor::builtins();
    let ids: std::collections::HashSet<String> = b
        .functions
        .keys()
        .chain(b.variables.iter())
        .chain(b.constants.iter())
        .map(|n| n.to_lowercase())
        .collect();
    let allowed = |w: &str| ids.contains(&w.to_lowercase());
    let mut bad = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        for (n, line) in text.lines().enumerate() {
            if let Some((_, word)) = gen::forbidden_word(line, &allowed) {
                bad.push(format!("{}:{}: {word}", f.display(), n + 1));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "third-party product names in the manual:\n{}",
        bad.join("\n")
    );
}

/// The forbidden-word check itself.
#[test]
fn forbidden_word_check() {
    let allowed = |w: &str| w == "gamemaker_version";
    assert!(gen::forbidden_word("Use gamemaker_version here.", &allowed).is_none());
    assert!(gen::forbidden_word("Made with GameMaker.", &allowed).is_some());
    assert!(gen::forbidden_word("a game maker tool", &allowed).is_some());
    assert!(gen::forbidden_word("by YoYo", &allowed).is_some());
    assert!(gen::forbidden_word("YoYo_GetPlatform", &allowed).is_some());
}

/// Every file of `docs/manual` is embedded, and every name of the index points at an
/// existing heading of the reference.
#[test]
fn embedded_manual_and_index_anchors() {
    let mut files = Vec::new();
    md_files(&manual_dir(), &mut files);
    for f in &files {
        let rel = f
            .strip_prefix(manual_dir())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert!(file(&rel).is_some(), "{rel} is not embedded");
    }
    assert!(names().len() >= 1555, "{} names", names().len());
    let mut anchors: HashMap<String, Vec<String>> = HashMap::new();
    for (name, loc, _) in names() {
        let list = anchors.entry(loc.page.clone()).or_insert_with(|| {
            let doc = markdown::parse(
                page_text(&loc.page).unwrap_or_else(|| panic!("{name}: no page {}", loc.page)),
            );
            doc.blocks
                .iter()
                .filter_map(|b| match b {
                    Block::Heading { anchor, .. } => Some(anchor.clone()),
                    _ => None,
                })
                .collect()
        });
        let a = loc.anchor.as_ref().unwrap();
        assert!(list.contains(a), "{name}: no heading #{a} in {}", loc.page);
    }
    assert_eq!(
        lookup_name("draw_sprite"),
        Some(Loc {
            page: "reference/game-graphics.md".into(),
            anchor: Some("draw_sprite".into())
        })
    );
}

#[test]
fn contents_tree() {
    let t = toc();
    assert!(t.iter().any(|s| s.title == "Function reference"));
    let all: Vec<&str> = t
        .iter()
        .flat_map(|s| s.pages.iter().map(|p| p.1.as_str()))
        .collect();
    assert!(all.contains(&"intro/welcome.md"));
    assert!(all.contains(&"reference/game-graphics.md"));
}

#[test]
fn link_resolution() {
    assert_eq!(
        resolve("README.md", "intro/welcome.md"),
        Some(Loc::page("intro/welcome.md"))
    );
    assert_eq!(
        resolve("reference/index.md", "game-play.md#x"),
        Some(Loc {
            page: "reference/game-play.md".into(),
            anchor: Some("x".into())
        })
    );
    assert_eq!(
        resolve("gml/overview.md", "../reference/README.md"),
        Some(Loc::page("reference/README.md"))
    );
    assert_eq!(
        resolve("gml/overview.md", "#top"),
        Some(Loc {
            page: "gml/overview.md".into(),
            anchor: Some("top".into())
        })
    );
    assert_eq!(resolve("README.md", "https://example.com/"), None);
}

#[test]
fn search_finds_names_and_pages() {
    let hits = search("draw_sprite");
    assert_eq!(hits[0].label, "draw_sprite");
    assert_eq!(hits[0].loc.anchor.as_deref(), Some("draw_sprite"));
    assert!(hits.iter().any(|h| h.label == "draw_sprite_ext"));
    let hits = search("Data structures");
    assert!(
        hits.iter()
            .any(|h| h.loc == Loc::page("reference/data-structures.md")),
        "{hits:?}"
    );
    // text inside a section
    let hits = search("priority queue");
    assert!(
        hits.iter()
            .any(|h| h.loc.page == "reference/data-structures.md"),
        "{hits:?}"
    );
    assert!(search("   ").is_empty());
}

#[test]
fn history() {
    let mut v = HelpView::default();
    v.open_contents();
    assert_eq!(v.current(), Some(&Loc::page(HOME)));
    v.open_topic("draw_text");
    assert_eq!(v.current().unwrap().anchor.as_deref(), Some("draw_text"));
    assert!(v.can_back() && !v.can_forward());
    v.back();
    assert_eq!(v.current(), Some(&Loc::page(HOME)));
    v.forward();
    assert_eq!(v.current().unwrap().anchor.as_deref(), Some("draw_text"));
    v.back();
    v.navigate(Loc::page(NAME_INDEX));
    assert!(!v.can_forward());
    // unknown words are searched for
    v.open_topic("no_such_function_here");
    assert_eq!(v.search, "no_such_function_here");
}
