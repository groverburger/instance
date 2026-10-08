//! Headless tests against the sample projects (opaque inputs, read from `GM81_SAMPLES`
//! or the default Examples directory; skipped when absent).

use std::path::PathBuf;

use gm_format::{canonical_bytes, read_project, FormatVersion, ResourceKind};

use crate::editors::{open_editor, ResKey};
use crate::project_ops::{self as ops, LIST_KINDS};

const SAMPLES: [&str; 5] = [
    "tutorial1.gmk",
    "treasure.gmk",
    "street racing.gmk",
    "treasure.gm81",
    "street racing.gm81",
];

fn sample_path(name: &str) -> Option<PathBuf> {
    let dir = std::env::var_os("GM81_SAMPLES")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Grob\Documents\gamemaker\GameMaker\Examples"));
    let p = dir.join(name);
    if p.is_file() {
        Some(p)
    } else {
        eprintln!("skipping: sample {} not found", p.display());
        None
    }
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("gm-ide-tests-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.join(name)
}

/// Open → save (as .gm81) without edits gives an equal project; for .gm81 inputs the
/// file content is identical modulo compression.
#[test]
fn open_save_is_lossless() {
    for name in SAMPLES {
        let Some(path) = sample_path(name) else {
            continue;
        };
        let original = std::fs::read(&path).unwrap();
        let p = ops::load_project(&path).unwrap();
        let out = tmp(&format!("{}.gm81", name.replace(' ', "_")));
        ops::save_project(&p, &out).unwrap();
        let saved = std::fs::read(&out).unwrap();
        let q = read_project(&saved).unwrap();
        assert!(p == q, "{name}: project changed by save");
        if gm_format::detect_version(&original).unwrap() == FormatVersion::V810 {
            assert!(
                canonical_bytes(&original).unwrap() == canonical_bytes(&saved).unwrap(),
                "{name}: saved file differs from the original"
            );
        }
    }
}

/// Opening every form on a sample and pressing OK without edits changes nothing.
#[test]
fn forms_open_clean() {
    for name in SAMPLES {
        let Some(path) = sample_path(name) else {
            continue;
        };
        let mut p = ops::load_project(&path).unwrap();
        let before = p.clone();
        let mut keys = vec![
            ResKey::GameInfo,
            ResKey::Settings,
            ResKey::Extensions,
            ResKey::Constants,
            ResKey::Triggers,
            ResKey::IncludedFiles,
        ];
        for (k, _, _) in LIST_KINDS {
            for i in 0..ops::slot_count(&p, k) {
                if ops::res_exists(&p, k, i) {
                    keys.push(ResKey::Res(k, i));
                }
            }
        }
        for key in keys {
            let mut ed =
                open_editor(key, &p, 1).unwrap_or_else(|| panic!("{name}: cannot open {key:?}"));
            assert!(
                !ed.is_dirty(&p),
                "{name}: {key:?} dirty right after opening"
            );
            assert!(
                !ed.commit(&mut p),
                "{name}: {key:?} commit changed the project"
            );
        }
        assert!(p == before, "{name}: project changed");
    }
}

/// The room order of the samples equals the room tree order (so recomputing it after
/// tree edits matches what the original IDE stores).
#[test]
fn room_order_matches_tree() {
    for name in SAMPLES {
        let Some(path) = sample_path(name) else {
            continue;
        };
        let p = ops::load_project(&path).unwrap();
        let mut q = p.clone();
        ops::rebuild_room_order(&mut q);
        assert_eq!(p.room_order, q.room_order, "{name}");
    }
}

/// Editing a sample through the project operations and saving gives a readable file.
#[test]
fn edit_and_save_sample() {
    let Some(path) = sample_path("treasure.gm81") else {
        return;
    };
    let mut p = ops::load_project(&path).unwrap();
    let (s, _) = ops::create_resource(&mut p, ResourceKind::Script, &ops::Placement::End);
    p.scripts[s].as_mut().unwrap().code = "show_message(\"hi\");".into();
    let r = ops::root_index(&p, ResourceKind::Object).unwrap();
    ops::add_group(&mut p, &[r], "enemies").unwrap();
    let out = tmp("edited.gm81");
    ops::save_project(&p, &out).unwrap();
    let q = ops::load_project(&out).unwrap();
    assert!(p == q);
    assert_eq!(q.scripts[s].as_ref().unwrap().name, format!("script{s}"));
}

/// The run helper writes a loadable temporary game.
#[test]
fn temp_game_file() {
    let mut p = ops::new_project();
    ops::create_resource(&mut p, ResourceKind::Room, &ops::Placement::End);
    let f = crate::run::write_temp_game(&p).unwrap();
    let q = ops::load_project(&f).unwrap();
    assert!(p == q);
    crate::run::cleanup();
}

/// Render the whole IDE headlessly with every form of a sample open: no panics, and
/// nothing is modified by merely displaying forms.
#[test]
fn ui_smoke_all_forms() {
    use egui_kittest::Harness;
    for name in ["treasure.gm81", "street racing.gm81", "tutorial1.gmk"] {
        let Some(path) = sample_path(name) else {
            continue;
        };
        let proj = ops::load_project(&path).unwrap();
        let mut h = Harness::builder()
            .with_size(eframe::egui::Vec2::new(1600.0, 1000.0))
            .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
        h.state_mut().test_set_project(proj.clone());
        h.run_steps(2);
        let mut keys = vec![
            ResKey::GameInfo,
            ResKey::Settings,
            ResKey::Extensions,
            ResKey::Constants,
            ResKey::Triggers,
            ResKey::IncludedFiles,
        ];
        for (k, _, _) in LIST_KINDS {
            for i in 0..ops::slot_count(&proj, k) {
                if ops::res_exists(&proj, k, i) {
                    keys.push(ResKey::Res(k, i));
                }
            }
        }
        let n = keys.len();
        for k in keys {
            h.state_mut().open_form(k);
            h.run_steps(2);
        }
        h.run_steps(3);
        let (p, modified, forms, msgs) = h.state().test_state();
        assert_eq!(forms, n, "{name}");
        assert_eq!(msgs, 0, "{name}");
        assert!(!modified, "{name}");
        assert!(*p == proj, "{name}: project changed by displaying forms");
    }
}

/// Drive the menus like a user: create one resource of each kind.
#[test]
fn ui_create_via_menu() {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    let mut h = Harness::builder()
        .with_size(eframe::egui::Vec2::new(1600.0, 1000.0))
        .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
    h.state_mut().test_set_project(ops::new_project());
    h.run_steps(2);
    for (k, label, _) in LIST_KINDS {
        h.get_by_label("Resources").click();
        h.run_steps(2);
        h.get_by_label(&format!("Create {label}")).click();
        h.run_steps(3);
        let (p, modified, _, _) = h.state().test_state();
        assert!(modified);
        assert_eq!(ops::slot_count(p, k), 1, "{label}");
    }
    let (_, _, forms, _) = h.state().test_state();
    assert_eq!(forms, LIST_KINDS.len());
    // OK on every form commits without error
    for _ in 0..20 {
        if h.state().test_state().2 == 0 {
            break;
        }
        // The most recently opened form is on top.
        h.get_all_by_label("✔ OK").last().unwrap().click();
        h.run_steps(3);
    }
    assert_eq!(h.state().test_state().2, 0, "OK did not close all forms");
}

fn harness() -> egui_kittest::Harness<'static, crate::app::IdeApp> {
    let mut h = egui_kittest::Harness::builder()
        .with_size(eframe::egui::Vec2::new(1600.0, 1000.0))
        .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
    h.state_mut().test_set_project(ops::new_project());
    h.run_steps(2);
    h
}

/// Object form: add a Create event, drop a "Move Fixed" action via the palette, set an
/// argument in the dialog, commit.
#[test]
fn ui_object_events_and_actions() {
    use egui_kittest::kittest::Queryable;
    let mut h = harness();
    let mut p = ops::new_project();
    ops::create_resource(&mut p, ResourceKind::Object, &ops::Placement::End);
    h.state_mut().test_set_project(p);
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Object, 0));
    h.run_steps(3);
    h.get_by_label("Add Event").click();
    h.run_steps(3);
    h.get_by_label("Create").click();
    h.run_steps(3);
    h.get_by_label("Move Fixed").click();
    h.run_steps(3);
    // the argument dialog is open: pick the "up" direction and confirm
    h.get_by_label("↑").click();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    h.get_by_label("✔ OK").click();
    h.run_steps(3);
    let (p, modified, forms, msgs) = h.state().test_state();
    assert_eq!((forms, msgs), (0, 0));
    assert!(modified);
    let o = p.objects[0].as_ref().unwrap();
    assert_eq!(o.events.len(), 1);
    assert_eq!((o.events[0].event_type, o.events[0].number), (0, 0));
    let a = &o.events[0].actions[0];
    assert_eq!((a.library_id, a.action_id), (1, 101));
    assert_eq!(a.args[0], "000000010");
}

/// Room form: click on the canvas places an instance with a fresh id.
#[test]
fn ui_room_place_instance() {
    use egui_kittest::kittest::Queryable;
    let mut h = harness();
    let mut p = ops::new_project();
    ops::create_resource(&mut p, ResourceKind::Object, &ops::Placement::End);
    ops::create_resource(&mut p, ResourceKind::Room, &ops::Placement::End);
    h.state_mut().test_set_project(p);
    h.state_mut().open_form(ResKey::Res(ResourceKind::Room, 0));
    h.run_steps(3);
    let r = h.get_by_label("room canvas").rect();
    h.hover_at(r.min + eframe::egui::vec2(100.0, 60.0));
    h.run_steps(1);
    h.get_by_label("room canvas").hover();
    // click at an explicit position: press and release
    let pos = r.min + eframe::egui::vec2(100.0, 60.0);
    h.event(eframe::egui::Event::PointerMoved(pos));
    h.event(eframe::egui::Event::PointerButton {
        pos,
        button: eframe::egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    h.run_steps(1);
    h.event(eframe::egui::Event::PointerButton {
        pos,
        button: eframe::egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    h.run_steps(3);
    h.get_by_label("✔ OK").click();
    h.run_steps(3);
    let (p, _, forms, msgs) = h.state().test_state();
    assert_eq!((forms, msgs), (0, 0));
    let room = p.rooms[0].as_ref().unwrap();
    assert_eq!(room.instances.len(), 1, "instance placed");
    let i = &room.instances[0];
    assert_eq!(i.object, 0);
    assert_eq!(i.id, gm_format::FIRST_INSTANCE_ID + 1);
    assert_eq!((i.x % 16, i.y % 16), (0, 0), "snapped");
    assert!(p.last_instance_id >= i.id);
}

/// Create Executable: runner bytes + payload + footer; the footer reader finds the
/// project, and used extension packages are bundled (v2) or reported when missing.
#[test]
fn create_executable_round_trip() {
    let mut p = synthetic_project();
    let runner = tmp("fake-runner.exe");
    std::fs::write(&runner, b"MZ this is not really a runner").unwrap();
    // v1: no extension packages
    let out = tmp("game1.exe");
    let warnings = crate::exe::create_executable(&p, &runner, &out, None).unwrap();
    assert!(warnings.is_empty());
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"MZ this is not really a runner"));
    let e = gm_format::exe::read_embedded(&bytes).expect("footer");
    assert!(e.bundle.is_none());
    assert!(read_project(&e.payload).unwrap() == p);
    // v2: one installed package found in the project's extensions dir, one missing
    let proj_dir = tmp("exeproj");
    let ext = proj_dir.join("extensions");
    std::fs::create_dir_all(&ext).unwrap();
    std::fs::write(ext.join("Pkg A.ged"), b"ged").unwrap();
    std::fs::write(ext.join("Pkg A.dat"), b"dat").unwrap();
    p.extension_packages = vec!["Pkg A".into(), "Missing Pkg".into()];
    let out = tmp("game2.exe");
    let warnings =
        crate::exe::create_executable(&p, &runner, &out, Some(&proj_dir.join("game.gm81")))
            .unwrap();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("Missing Pkg"));
    let e = gm_format::exe::read_embedded(&std::fs::read(&out).unwrap()).unwrap();
    assert!(read_project(&e.payload).unwrap() == p);
    let pk = gm_format::exe::read_bundle(&e.bundle.unwrap()).unwrap();
    assert_eq!(pk.len(), 1);
    assert_eq!(
        (pk[0].name.as_str(), &pk[0].ged[..], &pk[0].dat[..]),
        ("Pkg A", &b"ged"[..], &b"dat"[..])
    );
}

fn drag(
    h: &mut egui_kittest::Harness<'_, crate::app::IdeApp>,
    from: eframe::egui::Pos2,
    by: eframe::egui::Vec2,
) {
    use eframe::egui::{Event, PointerButton};
    h.event(Event::PointerMoved(from));
    h.run_steps(1);
    h.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    h.run_steps(1);
    for i in 1..=5 {
        h.event(Event::PointerMoved(from + by * (i as f32 / 5.0)));
        h.run_steps(1);
    }
    h.event(Event::PointerButton {
        pos: from + by,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    h.run_steps(2);
}

/// MDI child windows: move by the caption, resize by the corner, maximise, minimise.
#[test]
fn ui_mdi_move_resize_maximise() {
    use eframe::egui::vec2;
    use egui_kittest::kittest::Queryable;
    let mut h = harness();
    h.state_mut().test_set_project(synthetic_project());
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Script, 0));
    h.run_steps(4);
    let id = h.state().test_form_ids()[0];
    let rect = |h: &egui_kittest::Harness<'_, crate::app::IdeApp>| {
        h.ctx.memory(|m| m.area_rect(id)).unwrap()
    };
    let r0 = rect(&h);
    drag(&mut h, r0.min + vec2(60.0, 12.0), vec2(50.0, 30.0));
    let r1 = rect(&h);
    assert!(
        (r1.min - r0.min - vec2(50.0, 30.0)).length() < 1.5,
        "{r0:?} -> {r1:?}"
    );
    assert!((r1.size() - r0.size()).length() < 0.5);
    drag(&mut h, r1.max - vec2(2.0, 2.0), vec2(-60.0, -40.0));
    let r2 = rect(&h);
    assert!(
        (r2.size() - r1.size() - vec2(-60.0, -40.0)).length() < 1.5,
        "{r1:?} -> {r2:?}"
    );
    h.get_by_label("Maximise").click();
    h.run_steps(3);
    let r3 = rect(&h);
    assert!(r3.width() > r2.width() + 100.0 && r3.height() > r2.height() + 100.0);
    h.get_by_label("Restore").click();
    h.run_steps(3);
    assert!((rect(&h).size() - r2.size()).length() < 1.5);
    h.get_by_label("Minimise").click();
    h.run_steps(3);
    assert!(rect(&h).height() < 40.0);
    assert_eq!(
        h.state().test_state().2,
        1,
        "minimising keeps the form open"
    );
}

/// A small synthetic project with one populated resource of each kind.
fn synthetic_project() -> gm_format::Project {
    let mut p = ops::new_project();
    for (k, _, _) in LIST_KINDS {
        ops::create_resource(&mut p, k, &ops::Placement::End);
    }
    p
}

/// Regression: forms must not grow by themselves while the mouse moves over them
/// (content sized from the window's own available space fed back into the window).
#[test]
fn ui_forms_do_not_grow() {
    use eframe::egui::{pos2, Event};
    let mut projects = vec![("synthetic".to_string(), synthetic_project())];
    for name in ["treasure.gm81", "tutorial1.gmk"] {
        if let Some(path) = sample_path(name) {
            projects.push((name.to_string(), ops::load_project(&path).unwrap()));
        }
    }
    for (name, proj) in projects {
        let mut h = harness();
        h.state_mut().test_set_project(proj.clone());
        h.run_steps(2);
        let mut keys = vec![
            ResKey::GameInfo,
            ResKey::Settings,
            ResKey::Extensions,
            ResKey::Constants,
            ResKey::Triggers,
            ResKey::IncludedFiles,
        ];
        for (k, _, _) in LIST_KINDS {
            if let Some(i) = (0..ops::slot_count(&proj, k)).find(|&i| ops::res_exists(&proj, k, i))
            {
                keys.push(ResKey::Res(k, i));
            }
        }
        for key in keys {
            h.state_mut().test_close_forms();
            h.state_mut().open_form(key);
            h.run_steps(4);
            let id = h.state().test_form_ids()[0];
            let rect_of = |h: &egui_kittest::Harness<'_, crate::app::IdeApp>| {
                h.ctx.memory(|m| m.area_rect(id)).expect("form window rect")
            };
            let start = rect_of(&h);
            for f in 0..60 {
                let t = f as f32 / 60.0;
                let pos = pos2(
                    start.min.x + 5.0 + (start.width() - 10.0) * t,
                    start.min.y + 30.0 + (start.height() - 40.0) * ((t * 7.0) % 1.0),
                );
                h.event(Event::PointerMoved(pos));
                h.run_steps(1);
            }
            let end = rect_of(&h);
            assert!(
                (end.width() - start.width()).abs() < 1.0
                    && (end.height() - start.height()).abs() < 1.0,
                "{name}: {key:?} window changed size from {start:?} to {end:?}"
            );
        }
    }
}

/// Rects of every button in the accessibility tree, in tree order.
fn button_rects(
    h: &egui_kittest::Harness<'_, crate::app::IdeApp>,
) -> Vec<(String, eframe::egui::Rect)> {
    use egui_kittest::kittest::{by, NodeT, Queryable};
    h.query_all(by().role(eframe::egui::accesskit::Role::Button))
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
        .collect()
}

/// With the IDE theme, every kind of button keeps its size when hovered (egui drops
/// the frame of unselected selectable buttons at rest, so the frame stroke must not
/// add to their size).
#[test]
fn theme_hover_keeps_widget_sizes() {
    use eframe::egui;
    use egui_kittest::kittest::{by, NodeT, Queryable};
    let mut flag = false;
    let mut b = false;
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(700.0, 120.0))
        .build_ui(move |ui| {
            if !flag {
                crate::theme::apply(ui.ctx());
                flag = true;
            }
            ui.horizontal(|ui| {
                let _ = ui.button("plain");
                ui.add(egui::Button::selectable(false, "sel-off"));
                ui.add(egui::Button::selectable(true, "sel-on"));
                let _ = ui.selectable_label(false, "label-off");
                ui.checkbox(&mut b, "check");
                ui.menu_button("menu", |ui| ui.label("x"));
                ui.add(egui::Button::new("frameless").frame(false));
                let _ = ui.button("last");
            });
        });
    h.run_steps(3);
    let rects = |h: &egui_kittest::Harness<'_>| -> Vec<(String, egui::Rect)> {
        h.query_all(by().recursive(true))
            .filter_map(|n| {
                let l = n.accesskit_node().label()?;
                (!l.is_empty()).then(|| (l, n.rect()))
            })
            .collect()
    };
    h.hover_at(egui::pos2(690.0, 110.0));
    h.run_steps(2);
    let before = rects(&h);
    assert!(before.len() >= 8, "{before:?}");
    for (label, r) in &before {
        h.hover_at(r.center());
        h.run_steps(2);
        for ((l0, r0), (_, r1)) in before.iter().zip(&rects(&h)) {
            assert!(
                (r0.min - r1.min).length() < 0.01 && (r0.max - r1.max).length() < 0.01,
                "hovering {label:?} moved {l0:?} from {r0:?} to {r1:?}"
            );
        }
    }
}

/// Hovering a button must not change its size or move any other widget (the hover
/// frame is drawn inside the button's rect).
#[test]
fn ui_hover_does_not_move_neighbours() {
    let mut h = harness();
    let mut p = ops::new_project();
    ops::create_resource(&mut p, ResourceKind::Sprite, &ops::Placement::End);
    ops::create_resource(&mut p, ResourceKind::Object, &ops::Placement::End);
    ops::create_resource(&mut p, ResourceKind::Room, &ops::Placement::End);
    h.state_mut().test_set_project(p);
    for k in [
        ResourceKind::Room,
        ResourceKind::Sprite,
        ResourceKind::Object,
    ] {
        h.state_mut().open_form(ResKey::Res(k, 0));
        h.run_steps(3);
    }
    // park the pointer on the empty workspace
    h.hover_at(eframe::egui::pos2(1500.0, 900.0));
    h.run_steps(3);
    let before = button_rects(&h);
    assert!(before.len() > 30, "only {} buttons found", before.len());
    for (label, r) in &before {
        h.hover_at(r.center());
        h.run_steps(2);
        let after = button_rects(&h);
        assert_eq!(after.len(), before.len(), "hovering {label:?}");
        for ((l0, r0), (_, r1)) in before.iter().zip(&after) {
            assert!(
                (r0.min - r1.min).length() < 0.01 && (r0.max - r1.max).length() < 0.01,
                "hovering {label:?} moved {l0:?} from {r0:?} to {r1:?}"
            );
        }
    }
}

/// Renders dialogs to PNGs in `$GM_IDE_SCREENSHOT` (manual check of the chrome; run
/// with `--ignored`; needs a wgpu adapter).
#[test]
#[ignore]
fn screenshot_dialogs() {
    use egui_kittest::kittest::Queryable;
    let Some(out) = std::env::var_os("GM_IDE_SCREENSHOT") else {
        return;
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let mut h = egui_kittest::Harness::builder()
        .with_size(eframe::egui::Vec2::new(1280.0, 800.0))
        .with_theme(eframe::egui::Theme::Light)
        .wgpu()
        .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
    let mut p = ops::new_project();
    ops::create_resource(&mut p, ResourceKind::Object, &ops::Placement::End);
    h.state_mut().test_set_project(p);
    h.run_steps(2);
    h.get_by_label("About gm81-clean").click();
    h.run_steps(3);
    h.render()
        .unwrap()
        .save(out.join("dialog_about.png"))
        .unwrap();
    h.get_by_label("OK").click();
    h.run_steps(3);
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Object, 0));
    h.run_steps(3);
    h.get_by_label("Add Event").click();
    h.run_steps(3);
    h.render()
        .unwrap()
        .save(out.join("dialog_event.png"))
        .unwrap();
    h.get_by_label("Create").click();
    h.run_steps(3);
    h.get_by_label("Move Fixed").click();
    h.run_steps(3);
    h.render()
        .unwrap()
        .save(out.join("dialog_action.png"))
        .unwrap();
}
