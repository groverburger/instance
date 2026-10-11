//! Tests of the Extension Packages dialog with synthetic packages written into
//! temporary folders (and, when installed locally, the packages shipped with GM 8.1,
//! read in place and never copied).

use std::path::{Path, PathBuf};

use egui_kittest::kittest::Queryable;
use instance_format::extpkg::{self, ExtConstant, ExtFile, ExtFunction, ExtPackage};

use super::*;
use crate::dialogs::Picked;
use crate::editors::ResKey;
use crate::project_ops as ops;

fn tmp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "instance-ide-ext-tests-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn package(name: &str, version: &str) -> ExtPackage {
    ExtPackage {
        name: name.into(),
        version_text: version.into(),
        author: "Test Author".into(),
        date: "2026-10-11".into(),
        license: "Free".into(),
        description: format!("The {name} test package."),
        files: vec![ExtFile {
            file_name: "pkg.gml".into(),
            kind: extpkg::FILE_GML,
            functions: vec![
                ExtFunction {
                    name: "pkg_show".into(),
                    argument_count: 2,
                    arg_types: {
                        let mut t = [extpkg::ARG_REAL; 17];
                        t[0] = extpkg::ARG_STRING;
                        t
                    },
                    return_type: extpkg::ARG_REAL,
                    ..Default::default()
                },
                ExtFunction {
                    name: "__pkg_hidden".into(),
                    hidden: true,
                    ..Default::default()
                },
            ],
            constants: vec![ExtConstant {
                name: "pkg_answer".into(),
                value: "42".into(),
                hidden: false,
            }],
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// Writes `<file>.ged` / `.dat` (and `.chm` when `chm`) for `p` into `dir`.
fn write_pkg(dir: &Path, file: &str, p: &ExtPackage, chm: bool) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(format!("{file}.ged")), extpkg::write_ged(p)).unwrap();
    let data: Vec<Vec<u8>> = p.files.iter().map(|_| b"#define pkg_show\nreturn 1".to_vec()).collect();
    std::fs::write(dir.join(format!("{file}.dat")), extpkg::write_dat(&data, 1234)).unwrap();
    if chm {
        std::fs::write(dir.join(format!("{file}.chm")), b"help").unwrap();
    }
}

#[test]
fn scan_first_hit_wins_and_sorts() {
    let root = tmp_dir("scan");
    let (a, b) = (root.join("a"), root.join("b"));
    write_pkg(&a, "Shared", &package("Shared", "1.0"), true);
    write_pkg(&a, "zeta", &package("zeta", "1"), false);
    write_pkg(&b, "Shared", &package("Shared", "2.0"), false);
    write_pkg(&b, "Alpha", &package("Alpha", "3"), false);
    std::fs::write(b.join("Broken.ged"), b"not a ged").unwrap();
    std::fs::write(b.join("readme.txt"), b"x").unwrap();
    let found = scan(&[a.clone(), root.join("missing"), b.clone()]);
    let names: Vec<&str> = found.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["Alpha", "Broken", "Shared", "zeta"]);
    let shared = &found[2];
    assert_eq!(shared.dir, a);
    assert_eq!(shared.pkg.as_ref().unwrap().version_text, "1.0");
    assert!(shared.has_dat);
    assert_eq!(shared.chm.as_deref(), Some(a.join("Shared.chm").as_path()));
    assert!(found[0].chm.is_none());
    assert!(found[1].pkg.is_err());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn install_and_uninstall() {
    let root = tmp_dir("install");
    let src = root.join("download");
    let dest = root.join("user").join("extensions");
    // the source files are named differently from the package
    write_pkg(&src, "mypkg_v2", &package("My Package", "2.0"), true);
    let ged = src.join("mypkg_v2.ged");
    let picked = vec![Picked {
        name: "mypkg_v2.ged".into(),
        bytes: std::fs::read(&ged).unwrap(),
        path: Some(ged.clone()),
    }];
    assert_eq!(install_picked(&picked, &dest).unwrap(), "My Package");
    for ext in ["ged", "dat", "chm"] {
        assert!(dest.join(format!("My Package.{ext}")).is_file(), "{ext}");
    }
    let found = scan(std::slice::from_ref(&dest));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "My Package");
    assert!(found[0].has_dat && found[0].chm.is_some());
    // picked together (the browser case): no path, the .dat comes from the selection
    let dest2 = root.join("user2");
    let both: Vec<Picked> = ["ged", "dat"]
        .iter()
        .map(|e| Picked {
            name: format!("mypkg_v2.{e}"),
            bytes: std::fs::read(src.join(format!("mypkg_v2.{e}"))).unwrap(),
            path: None,
        })
        .collect();
    assert_eq!(install_picked(&both, &dest2).unwrap(), "My Package");
    assert!(!dest2.join("My Package.chm").exists());
    // uninstall
    uninstall("My Package", &dest).unwrap();
    assert!(scan(std::slice::from_ref(&dest)).is_empty());
    assert!(!dest.join("My Package.dat").exists());
    assert!(uninstall("My Package", &dest).is_err());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn install_errors() {
    let root = tmp_dir("install-errors");
    let dest = root.join("dest");
    let gex = [Picked {
        name: "thing.gex".into(),
        bytes: vec![1, 2, 3],
        path: None,
    }];
    let e = install_picked(&gex, &dest).unwrap_err();
    assert!(e.contains(".gex"), "{e}");
    // .ged without its .dat
    let p = package("Lonely", "1");
    let only_ged = [Picked {
        name: "Lonely.ged".into(),
        bytes: extpkg::write_ged(&p),
        path: None,
    }];
    let e = install_picked(&only_ged, &dest).unwrap_err();
    assert!(e.contains("Lonely.dat"), "{e}");
    // damaged .dat
    let mut bad = only_ged.to_vec();
    bad.push(Picked {
        name: "Lonely.dat".into(),
        bytes: vec![0; 3],
        path: None,
    });
    assert!(install_picked(&bad, &dest).unwrap_err().contains("damaged"));
    // not a .ged
    let junk = [Picked {
        name: "junk.ged".into(),
        bytes: b"junk".to_vec(),
        path: None,
    }];
    assert!(install_picked(&junk, &dest).is_err());
    assert!(!dest.exists(), "nothing was installed");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn signatures() {
    let p = package("S", "1");
    let f = &p.files[0].functions[0];
    assert_eq!(signature(f), "pkg_show(string, real)");
    let v = ExtFunction {
        name: "va".into(),
        argument_count: -1,
        ..Default::default()
    };
    assert_eq!(signature(&v), "va(...)");
    assert!(valid_file_name("GM Printing"));
    assert!(!valid_file_name("a/b") && !valid_file_name("") && !valid_file_name("x."));
}

fn harness() -> egui_kittest::Harness<'static, crate::app::IdeApp> {
    egui_kittest::Harness::builder()
        .with_size(eframe::egui::Vec2::new(1280.0, 800.0))
        .with_step_dt(1.0 / 60.0)
        .build_eframe(|cc| crate::app::IdeApp::new(cc, None))
}

/// The dialog lists used packages (missing ones marked), the installed ones from the
/// project's extensions folder, moves packages between the lists and commits on OK.
#[test]
fn ui_extension_packages_dialog() {
    let root = tmp_dir("ui");
    let ext = root.join("extensions");
    write_pkg(&ext, "Instance Test Pkg A", &package("Instance Test Pkg A", "1.5"), true);
    write_pkg(&ext, "Instance Test Pkg B", &package("Instance Test Pkg B", "2"), false);
    let mut hidden = package("Instance Test Hidden", "1");
    hidden.hidden = true;
    write_pkg(&ext, "Instance Test Hidden", &hidden, false);
    let mut p = ops::new_project();
    p.extension_packages = vec!["Instance Test Gone".into(), "Instance Test Pkg B".into()];
    let mut h = harness();
    h.state_mut().test_set_project(p);
    h.state_mut().test_set_path(root.join("game.gm81"));
    h.state_mut().open_form(ResKey::Extensions);
    h.run_steps(4);
    // used: missing one marked, installed one plain; available: A only (B is used,
    // the hidden package is not offered)
    h.get_by_label("Instance Test Gone (not installed)");
    h.get_by_label("Instance Test Pkg B");
    assert!(h.query_by_label("Instance Test Hidden").is_none());
    // select A: its information is shown
    h.get_by_label("Instance Test Pkg A").click();
    h.run_steps(2);
    h.get_by_label("1.5");
    h.get_by_label("Test Author");
    h.get_by_label("pkg_show(string, real)");
    h.get_by_label("pkg_answer = 42");
    assert!(h.query_by_label("__pkg_hidden()").is_none());
    // the arrow moves it to the used list
    h.get_by_label("Use the package").click();
    // (wait, so the double-click below does not count as a triple click)
    h.run_steps(60);
    // double-clicking B in the used list moves it back to the available list
    let b = h.get_by_label("Instance Test Pkg B").rect().center();
    h.event(eframe::egui::Event::PointerMoved(b));
    h.run_steps(1);
    for _ in 0..2 {
        for pressed in [true, false] {
            h.event(eframe::egui::Event::PointerButton {
                pos: b,
                button: eframe::egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            h.run_steps(1);
        }
    }
    h.run_steps(2);
    assert_eq!(h.state().test_state().0.extension_packages.len(), 2, "not committed yet");
    h.get_by_label("✔ OK").click();
    h.run_steps(3);
    assert_eq!(
        h.state().test_state().0.extension_packages,
        ["Instance Test Gone", "Instance Test Pkg A"]
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The packages shipped with GM 8.1, when installed locally, are listed with their
/// information (read in place; skipped when absent).
#[test]
fn shipped_packages_listed() {
    let dir = std::env::var_os("GM81_EXTENSIONS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(r"C:\Users\Grob\Documents\gamemaker\GameMaker\extensions")
        });
    if !dir.is_dir() {
        eprintln!("shipped extensions not found; skipping");
        return;
    }
    let found = scan(std::slice::from_ref(&dir));
    assert!(!found.is_empty());
    for i in &found {
        let p = i.pkg.as_ref().unwrap_or_else(|e| panic!("{}: {e}", i.name));
        assert!(i.has_dat, "{}", i.name);
        let funcs: Vec<String> = p
            .files
            .iter()
            .flat_map(|f| &f.functions)
            .filter(|f| !f.hidden)
            .map(signature)
            .collect();
        eprintln!(
            "{} v{} by {} ({}), help {:?}: {} functions, e.g. {:?}",
            p.name,
            p.version_text,
            p.author,
            p.date,
            i.chm.as_ref().map(|c| c.file_name()),
            funcs.len(),
            funcs.first()
        );
        assert!(!funcs.is_empty(), "{}", i.name);
    }
}

/// Renders the dialog with a selected package to `form_extensions.png` in
/// `$GM_IDE_SCREENSHOT` (manual check of the layout; run with `--ignored`; needs a
/// wgpu adapter). Packages on the normal search path (e.g. `INSTANCE_EXTENSIONS`)
/// are listed too; `GM_IDE_SCREENSHOT_PKG` names the package to select.
#[test]
#[ignore]
fn screenshot_extensions() {
    let Some(out) = std::env::var_os("GM_IDE_SCREENSHOT") else {
        return;
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let root = tmp_dir("shot");
    write_pkg(&root.join("extensions"), "Sample Package", &package("Sample Package", "1.0"), true);
    let mut p = ops::new_project();
    p.extension_packages = vec!["Sample Package".into(), "Missing Package".into()];
    let mut h = egui_kittest::Harness::builder()
        .with_size(eframe::egui::Vec2::new(900.0, 700.0))
        .with_theme(eframe::egui::Theme::Light)
        .wgpu()
        .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
    h.state_mut().test_set_project(p);
    h.state_mut().test_set_path(root.join("game.gm81"));
    h.state_mut().open_form(ResKey::Extensions);
    h.run_steps(4);
    let pick = std::env::var("GM_IDE_SCREENSHOT_PKG").unwrap_or("Sample Package".into());
    h.get_by_label(&pick).click();
    h.run_steps(4);
    h.hover_at(eframe::egui::pos2(890.0, 690.0));
    h.run_steps(2);
    h.render()
        .unwrap()
        .save(out.join("form_extensions.png"))
        .unwrap();
    let _ = std::fs::remove_dir_all(&root);
}
