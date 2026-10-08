//! Headless UI tests of the object form (egui_kittest): event icons, the action palette
//! and the action list text.

use eframe::egui;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use instance_format::{builtin_action, event_type as et, Event, ResourceKind, APPLIES_TO_OTHER};

use super::editor_ui_tests::test_face;
use super::ResKey;
use crate::project_ops as ops;

/// A project with a sprite, a script and two objects; object 0 has several events.
fn sample_object_project() -> instance_format::Project {
    let mut p = ops::new_project();
    ops::create_resource(&mut p, ResourceKind::Sprite, &ops::Placement::End);
    p.sprites[0].as_mut().unwrap().frames = vec![test_face(32, 0)];
    p.sprites[0].as_mut().unwrap().name = "spr_player".into();
    ops::create_resource(&mut p, ResourceKind::Script, &ops::Placement::End);
    p.scripts[0].as_mut().unwrap().name = "characterCreate".into();
    ops::create_resource(&mut p, ResourceKind::Object, &ops::Placement::End);
    ops::create_resource(&mut p, ResourceKind::Object, &ops::Placement::End);
    {
        let wall = p.objects[1].as_mut().unwrap();
        wall.name = "oWall".into();
        wall.sprite = 0;
    }
    let act = |id: i32| builtin_action(1, id).unwrap().instantiate(1);
    let mut script = act(601);
    script.args[0] = "0".into();
    let mut code = act(603);
    code.args[0] = "speed = 4;\ndirection = 90;".into();
    let mut var = act(611);
    var.args[0] = "hp".into();
    var.args[1] = "1".into();
    var.relative = true;
    let mut test = act(612);
    test.args[0] = "hp".into();
    test.args[1] = "0".into();
    test.args[2] = "2".into();
    test.negate = true;
    let mut other = act(203);
    other.applies_to = APPLIES_TO_OTHER;
    let ev = |t: i32, n: i32, actions| Event {
        event_type: t,
        number: n,
        actions,
    };
    let o = p.objects[0].as_mut().unwrap();
    o.name = "oMain".into();
    o.sprite = 0;
    o.events = vec![
        ev(et::CREATE, 0, vec![script, code, act(604)]),
        ev(et::ALARM, 0, vec![act(203)]),
        ev(
            et::STEP,
            0,
            vec![test, act(422), var, act(424), act(421), act(425)],
        ),
        ev(et::COLLISION, 1, vec![other, act(115)]),
        ev(et::KEYBOARD, 37, vec![act(103)]),
        ev(et::OTHER, 7, vec![act(223)]),
        ev(et::DRAW, 0, vec![act(514)]),
    ];
    p
}

fn harness_with(p: instance_format::Project, size: egui::Vec2) -> Harness<'static, crate::app::IdeApp> {
    let mut h = Harness::builder()
        .with_size(size)
        .with_theme(egui::Theme::Light)
        .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
    h.state_mut().test_set_project(p);
    h.run_steps(2);
    h
}

/// Number of painted non-text shapes whose bounds lie inside `r`.
fn shapes_in(h: &Harness<'_, crate::app::IdeApp>, r: egui::Rect) -> usize {
    h.output()
        .shapes
        .iter()
        .filter(|s| !matches!(s.shape, egui::Shape::Text(_) | egui::Shape::Noop))
        .filter(|s| {
            let b = s.shape.visual_bounding_rect();
            b.is_positive() && r.expand(1.0).contains_rect(b)
        })
        .count()
}

/// The event list shows one row per event, each with an icon painted left of its
/// label; collision rows show the other object's name.
#[test]
fn ui_object_event_list_icons() {
    let mut h = harness_with(sample_object_project(), egui::vec2(1500.0, 900.0));
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Object, 0));
    h.run_steps(4);
    for label in [
        "Create",
        "Alarm 0",
        "Step",
        "<Left>",
        "Animation End",
        "Draw",
    ] {
        let r = h.get_by_label(label).rect();
        assert!(r.height() >= 16.0, "{label}: {r:?}");
        let icon = egui::Rect::from_min_size(r.min, egui::vec2(22.0, r.height()));
        assert!(
            shapes_in(&h, icon) >= 1,
            "no icon painted for {label} at {icon:?}"
        );
    }
    let col = h.get_by_label("Collision with oWall").rect();
    let icons = egui::Rect::from_min_size(col.min, egui::vec2(42.0, col.height()));
    assert!(shapes_in(&h, icons) >= 2, "collision icon and sprite");
}

/// The action list describes actions in readable lines with NOT/relative/for other.
#[test]
fn ui_object_action_text_lines() {
    let mut h = harness_with(sample_object_project(), egui::vec2(1500.0, 900.0));
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Object, 0));
    h.run_steps(4);
    for l in [
        "Execute script: characterCreate",
        "Execute a piece of code",
        "Call the inherited event",
    ] {
        h.get_by_label(l);
    }
    h.get_by_label("Step").click();
    h.run_steps(3);
    for l in [
        "If hp is not larger than 0",
        "Start of a block",
        "Set variable hp relative to 1",
        "End of a block",
        "Else",
        "Exit this event",
    ] {
        h.get_by_label(l);
    }
    h.get_by_label("Collision with oWall").click();
    h.run_steps(3);
    h.get_by_label("for other object: Destroy the instance");
}

/// Clicking a palette action adds it after the selection; dragging one onto the list
/// inserts it; the palette tabs switch the visible actions.
#[test]
fn ui_object_palette_adds_actions() {
    let mut h = harness_with(sample_object_project(), egui::vec2(1500.0, 900.0));
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Object, 0));
    h.run_steps(4);
    h.get_by_label("Draw").click();
    h.run_steps(2);
    // control tab: "Exit Event" has no dialog and is added directly
    h.get_by_label("control").click();
    h.run_steps(2);
    h.get_by_label("Exit Event").click();
    h.run_steps(3);
    h.get_by_label("Exit this event");
    // drag "Call Parent Event" to the top of the list
    let from = h.get_by_label("Call Parent Event").rect().center();
    let draw_text = crate::widgets::actions::action_label(
        &builtin_action(1, 514).unwrap().instantiate(1),
        &h.state().test_state().0.clone(),
    )
    .0;
    let to = h.get_by_label(&draw_text).rect().center_top() + egui::vec2(0.0, 3.0);
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    h.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    h.run_steps(1);
    for k in 1..=10 {
        h.event(egui::Event::PointerMoved(
            from + (to - from) * (k as f32 / 10.0),
        ));
        h.run_steps(1);
    }
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    h.run_steps(3);
    h.get_by_label("✔ OK").click();
    h.run_steps(3);
    let (p, modified, forms, _) = h.state().test_state();
    assert_eq!(forms, 0);
    assert!(modified);
    let draw = &p.objects[0].as_ref().unwrap().events[6];
    let ids: Vec<i32> = draw.actions.iter().map(|a| a.action_id).collect();
    assert_eq!(ids, vec![604, 514, 425]);
}

/// "Show Information" lists the events and actions.
#[test]
fn ui_object_show_information() {
    let p = sample_object_project();
    let text = super::object::object_info(&p, p.objects[0].as_ref().unwrap());
    assert!(text.contains("Information about object: oMain"));
    assert!(text.contains("Collision with oWall"), "{text}");
    assert!(text.contains("    Set variable hp relative to 1"), "{text}");
    assert!(text.contains("direction = 90;"), "{text}");
    let mut h = harness_with(p, egui::vec2(1500.0, 900.0));
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Object, 0));
    h.run_steps(4);
    h.get_by_label("Show Information").click();
    h.run_steps(3);
    h.get_by_label("OK").click();
    h.run_steps(3);
}

/// Render the object form of a sample (or synthetic) project and every palette tab to
/// PNGs in `$GM_IDE_SCREENSHOT` (run with `--ignored`; needs a wgpu adapter).
#[test]
#[ignore]
fn screenshot_object_form() {
    let Some(out) = std::env::var_os("GM_IDE_SCREENSHOT") else {
        eprintln!("set GM_IDE_SCREENSHOT=<dir> to write screenshots");
        return;
    };
    let out = std::path::PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    // GM_IDE_OBJECT=<project file>|<object name> picks a real project
    let (p, idx) = match std::env::var("GM_IDE_OBJECT").ok().and_then(|s| {
        let (f, o) = s.split_once('|')?;
        let p = ops::load_project(std::path::Path::new(f)).ok()?;
        let names: Vec<&str> = p
            .objects
            .iter()
            .flatten()
            .map(|o| o.name.as_str())
            .collect();
        eprintln!("objects: {names:?}");
        let i = p
            .objects
            .iter()
            .position(|x| x.as_ref().is_some_and(|x| x.name == o))?;
        Some((p, i))
    }) {
        Some(x) => x,
        None => (sample_object_project(), 0),
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1240.0, 720.0))
        .with_theme(egui::Theme::Light)
        .wgpu()
        .build_eframe(|cc| crate::app::IdeApp::new(cc, None));
    h.state_mut().test_set_project(p);
    h.run_steps(2);
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Object, idx));
    h.run_steps(4);
    let n_events = h.state().test_state().0.objects[idx]
        .as_ref()
        .unwrap()
        .events
        .len();
    if n_events > 1 {
        // select the event with the most actions
        let label = {
            let p = h.state().test_state().0;
            let o = p.objects[idx].as_ref().unwrap();
            let e = o.events.iter().max_by_key(|e| e.actions.len()).unwrap();
            crate::events::event_name(p, e.event_type, e.number)
        };
        h.get_by_label(&label).click();
        h.run_steps(3);
    }
    let park = egui::Event::PointerMoved(egui::pos2(5.0, 715.0));
    h.event(park.clone());
    h.run_steps(2);
    h.render()
        .unwrap()
        .save(out.join("object_form.png"))
        .unwrap();
    let tabs: Vec<String> = instance_format::builtin_libraries()
        .iter()
        .map(|l| l.tab_caption.clone())
        .collect();
    for t in tabs {
        h.get_by_label(&t).click();
        h.run_steps(2);
        h.event(park.clone());
        h.run_steps(2);
        h.render()
            .unwrap()
            .save(out.join(format!("palette_{t}.png")))
            .unwrap();
    }
    // the event selector, and a time line form (shares the action widgets)
    h.get_by_label("Add Event").click();
    h.run_steps(2);
    h.event(park.clone());
    h.run_steps(2);
    h.render()
        .unwrap()
        .save(out.join("event_selector.png"))
        .unwrap();
    let mut p = sample_object_project();
    ops::create_resource(&mut p, ResourceKind::Timeline, &ops::Placement::End);
    p.timelines[0].as_mut().unwrap().moments = vec![instance_format::Moment {
        step: 0,
        actions: p.objects[0].as_ref().unwrap().events[2].actions.clone(),
    }];
    h.state_mut().test_close_forms();
    h.state_mut().test_set_project(p);
    h.run_steps(2);
    h.state_mut()
        .open_form(ResKey::Res(ResourceKind::Timeline, 0));
    h.run_steps(4);
    h.event(park);
    h.run_steps(2);
    h.render()
        .unwrap()
        .save(out.join("timeline_form.png"))
        .unwrap();
}
