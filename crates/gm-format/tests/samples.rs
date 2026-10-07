//! Integration tests against the sample project files (opaque test inputs).
//!
//! The samples are read from the directory in `GM81_SAMPLES` (default:
//! `C:\Users\Grob\Documents\gamemaker\GameMaker\Examples`). Tests are skipped when a
//! file is absent. Nothing from the samples is copied into the repository.

use std::path::PathBuf;
use std::process::Command;

use gm_format::*;
use serde_json::Value;

fn sample(name: &str) -> Option<Vec<u8>> {
    let dir = std::env::var_os("GM81_SAMPLES")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Grob\Documents\gamemaker\GameMaker\Examples"));
    let p = dir.join(name);
    match std::fs::read(&p) {
        Ok(d) => Some(d),
        Err(_) => {
            eprintln!("skipping: sample {} not found", p.display());
            None
        }
    }
}

const ALL: [&str; 5] = [
    "tutorial1.gmk",
    "treasure.gmk",
    "street racing.gmk",
    "treasure.gm81",
    "street racing.gm81",
];

#[test]
fn all_samples_read() {
    for name in ALL {
        let Some(d) = sample(name) else { continue };
        let (p, v) = read_project_versioned(&d).unwrap_or_else(|e| panic!("{name}: {e}"));
        let expect = match name {
            "treasure.gm81" | "street racing.gm81" => FormatVersion::V810,
            "tutorial1.gmk" => FormatVersion::V800,
            _ => FormatVersion::V701,
        };
        assert_eq!(v, expect, "{name}");
        assert_eq!(p.resource_tree.len(), 12);
        assert!(p.rooms.iter().flatten().count() > 0, "{name}");
    }
}

/// read → write → read gives an equal project; v800/v810 files are re-written with
/// identical inflated block contents (only zlib compression may differ).
#[test]
fn samples_round_trip() {
    for name in ALL {
        let Some(d) = sample(name) else { continue };
        let (p, v) = read_project_versioned(&d).unwrap();
        let out_v = if v == FormatVersion::V701 {
            FormatVersion::V800
        } else {
            v
        };
        let out = write_project(&p, out_v).unwrap();
        let q = read_project(&out).unwrap();
        assert!(p == q, "{name}: project changed in round trip");
        if v != FormatVersion::V701 {
            let a = canonical_bytes(&d).unwrap();
            let b = canonical_bytes(&out).unwrap();
            if a != b {
                let i = a
                    .iter()
                    .zip(&b)
                    .position(|(x, y)| x != y)
                    .unwrap_or(a.len().min(b.len()));
                panic!(
                    "{name}: canonical bytes differ at {i:#x} (len {} vs {})",
                    a.len(),
                    b.len()
                );
            }
        }
    }
}

#[test]
fn treasure_701_and_810_are_equivalent() {
    let (Some(a), Some(b)) = (sample("treasure.gmk"), sample("treasure.gm81")) else {
        return;
    };
    let old = read_project(&a).unwrap();
    let new = read_project(&b).unwrap();
    // FF-13 evidence: same game id when the first v701 byte is taken raw.
    assert_eq!(old.game_id, new.game_id);
    assert_eq!(old.guid, new.guid);
    fn names<T>(v: &[Option<T>], f: impl Fn(&T) -> &str) -> Vec<Option<String>> {
        v.iter()
            .map(|x| x.as_ref().map(|x| f(x).to_string()))
            .collect()
    }
    assert_eq!(
        names(&old.sounds, |x| &x.name),
        names(&new.sounds, |x| &x.name)
    );
    assert_eq!(
        names(&old.sprites, |x| &x.name),
        names(&new.sprites, |x| &x.name)
    );
    assert_eq!(
        names(&old.backgrounds, |x| &x.name),
        names(&new.backgrounds, |x| &x.name)
    );
    assert_eq!(
        names(&old.scripts, |x| &x.name),
        names(&new.scripts, |x| &x.name)
    );
    assert_eq!(
        names(&old.objects, |x| &x.name),
        names(&new.objects, |x| &x.name)
    );
    assert_eq!(
        names(&old.rooms, |x| &x.name),
        names(&new.rooms, |x| &x.name)
    );
    for (x, y) in old.sounds.iter().zip(&new.sounds) {
        if let (Some(x), Some(y)) = (x, y) {
            assert_eq!(x.data, y.data, "sound {}", x.name);
            assert_eq!((x.kind, x.preload), (y.kind, y.preload));
        }
    }
    for (x, y) in old.scripts.iter().zip(&new.scripts) {
        if let (Some(x), Some(y)) = (x, y) {
            assert_eq!(x.code, y.code);
        }
    }
    // Sprites/backgrounds: v701 BMP + transparent/smooth-edge flags must convert to
    // exactly the RGBA the 8.1 IDE produced, and the mask settings must match.
    for (x, y) in old.sprites.iter().zip(&new.sprites) {
        if let (Some(x), Some(y)) = (x, y) {
            assert!(
                *x == Sprite {
                    last_changed: x.last_changed,
                    ..y.clone()
                },
                "sprite {}: {:?} vs {:?}",
                x.name,
                Sprite {
                    frames: vec![],
                    ..x.clone()
                },
                Sprite {
                    frames: vec![],
                    ..y.clone()
                }
            );
        }
    }
    for (x, y) in old.backgrounds.iter().zip(&new.backgrounds) {
        if let (Some(x), Some(y)) = (x, y) {
            assert!(
                *x == Background {
                    last_changed: x.last_changed,
                    ..y.clone()
                },
                "background {}",
                x.name
            );
        }
    }
    for (x, y) in old.rooms.iter().zip(&new.rooms) {
        if let (Some(x), Some(y)) = (x, y) {
            assert!(
                *x == Room {
                    last_changed: x.last_changed,
                    ..y.clone()
                },
                "room {}",
                x.name
            );
        }
    }
    assert_eq!(old.resource_tree, new.resource_tree);
    // Settings: identical except timestamps (v701 has no settings timestamp).
    assert_eq!(
        old.settings,
        Settings {
            info_last_changed: old.settings.info_last_changed,
            last_changed: 0.0,
            ..new.settings.clone()
        }
    );
    // Game information: the IDE re-emits the RTF text, so only the window fields match.
    assert_eq!(
        old.game_info,
        GameInfo {
            last_changed: 0.0,
            rtf: old.game_info.rtf.clone(),
            ..new.game_info.clone()
        }
    );
    assert_eq!(old.library_init_code, new.library_init_code);
    assert_eq!(old.extension_packages, new.extension_packages);
    for (x, y) in old.objects.iter().zip(&new.objects) {
        if let (Some(x), Some(y)) = (x, y) {
            let strip = |o: &Object| Object {
                last_changed: 0.0,
                ..o.clone()
            };
            assert_eq!(strip(x), strip(y), "object {}", x.name);
        }
    }
    assert_eq!(old.room_order, new.room_order);
    assert_eq!(old.constants, new.constants);
    assert_eq!(old.last_instance_id, new.last_instance_id);
}

// ------------------------------------------------------------------ reference decoder

fn reference_dump(name: &str) -> Option<Value> {
    let dir = std::env::var_os("GM81_SAMPLES")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Grob\Documents\gamemaker\GameMaker\Examples"));
    let file = dir.join(name);
    if !file.exists() {
        return None;
    }
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs/tools/gmk_dump.py");
    let out = ["python", "python3"].iter().find_map(|py| {
        Command::new(py)
            .arg("-I")
            .arg(&script)
            .arg(&file)
            .output()
            .ok()
            .filter(|o| o.status.success())
    });
    let Some(out) = out else {
        eprintln!("skipping: python reference decoder unavailable");
        return None;
    };
    Some(serde_json::from_slice(&out.stdout).expect("reference decoder JSON"))
}

fn ref_names(v: &Value, list: &str) -> Vec<Option<String>> {
    v[list]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect()
}

fn our_names<T>(v: &[Option<T>], f: impl Fn(&T) -> &str) -> Vec<Option<String>> {
    v.iter()
        .map(|x| x.as_ref().map(|x| f(x).to_string()))
        .collect()
}

fn check_tree(r: &Value, n: &TreeNode) {
    assert_eq!(r["name"].as_str().unwrap(), n.name);
    assert_eq!(r["status"].as_i64().unwrap() as i32, n.status.to_i32());
    assert_eq!(r["group_kind"].as_i64().unwrap() as i32, n.kind.to_i32());
    assert_eq!(r["index"].as_i64().unwrap() as i32, n.index);
    let rc = r["children"].as_array().unwrap();
    assert_eq!(rc.len(), n.children.len(), "children of {}", n.name);
    for (a, b) in rc.iter().zip(&n.children) {
        check_tree(a, b);
    }
}

fn check_actions(r: &Value, acts: &[Action]) {
    let ra = r.as_array().unwrap();
    assert_eq!(ra.len(), acts.len());
    for (x, a) in ra.iter().zip(acts) {
        assert_eq!(x["action_id"].as_i64().unwrap() as i32, a.action_id);
        assert_eq!(x["library_id"].as_i64().unwrap() as i32, a.library_id);
        assert_eq!(x["function_name"].as_str().unwrap(), a.function_name);
        assert_eq!(x["applies_to"].as_i64().unwrap() as i32, a.applies_to);
        assert_eq!(x["relative"].as_bool().unwrap(), a.relative);
        assert_eq!(x["not"].as_bool().unwrap(), a.negate);
        let args: Vec<&str> = x["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        assert_eq!(args, a.args);
    }
}

/// Our reader agrees with specs/tools/gmk_dump.py on structure (counts, names, events,
/// actions, instances, tree).
#[test]
fn matches_reference_decoder() {
    for name in ALL {
        let Some(d) = sample(name) else { continue };
        let Some(r) = reference_dump(name) else {
            continue;
        };
        let p = read_project(&d).unwrap();
        assert_eq!(
            ref_names(&r, "sounds"),
            our_names(&p.sounds, |x| &x.name),
            "{name}"
        );
        assert_eq!(ref_names(&r, "sprites"), our_names(&p.sprites, |x| &x.name));
        assert_eq!(
            ref_names(&r, "backgrounds"),
            our_names(&p.backgrounds, |x| &x.name)
        );
        assert_eq!(ref_names(&r, "paths"), our_names(&p.paths, |x| &x.name));
        assert_eq!(ref_names(&r, "scripts"), our_names(&p.scripts, |x| &x.name));
        assert_eq!(ref_names(&r, "fonts"), our_names(&p.fonts, |x| &x.name));
        assert_eq!(
            ref_names(&r, "timelines"),
            our_names(&p.timelines, |x| &x.name)
        );
        assert_eq!(ref_names(&r, "objects"), our_names(&p.objects, |x| &x.name));
        assert_eq!(ref_names(&r, "rooms"), our_names(&p.rooms, |x| &x.name));
        // Objects: events and actions.
        for (ro, o) in r["objects"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&p.objects)
        {
            let Some(o) = o else { continue };
            assert_eq!(ro["sprite"].as_i64().unwrap() as i32, o.sprite);
            assert_eq!(ro["parent"].as_i64().unwrap() as i32, o.parent);
            let re = ro["events"].as_array().unwrap();
            assert_eq!(re.len(), o.events.len(), "{name}: object {}", o.name);
            for (x, e) in re.iter().zip(&o.events) {
                assert_eq!(x["type"].as_i64().unwrap() as i32, e.event_type);
                assert_eq!(x["number"].as_i64().unwrap() as i32, e.number);
                check_actions(&x["actions"], &e.actions);
            }
        }
        // Rooms: instances.
        for (rr, m) in r["rooms"]["items"].as_array().unwrap().iter().zip(&p.rooms) {
            let Some(m) = m else { continue };
            assert_eq!(rr["width"].as_i64().unwrap() as i32, m.width);
            assert_eq!(rr["speed"].as_i64().unwrap() as i32, m.speed);
            let ri = rr["instances"].as_array().unwrap();
            assert_eq!(ri.len(), m.instances.len());
            for (x, i) in ri.iter().zip(&m.instances) {
                assert_eq!(x["id"].as_i64().unwrap() as i32, i.id);
                assert_eq!(x["object"].as_i64().unwrap() as i32, i.object);
                assert_eq!(
                    (
                        x["x"].as_i64().unwrap() as i32,
                        x["y"].as_i64().unwrap() as i32
                    ),
                    (i.x, i.y)
                );
            }
            assert_eq!(rr["tiles"].as_array().unwrap().len(), m.tiles.len());
        }
        // Sprites: frame counts (and sizes for v800+).
        for (rs, s) in r["sprites"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&p.sprites)
        {
            let Some(s) = s else { continue };
            let rf = rs["frames"].as_array().unwrap();
            assert_eq!(rf.len(), s.frames.len());
            if let Some(w) = rf.first().and_then(|f| f.get("width")) {
                assert_eq!(w.as_i64().unwrap() as u32, s.width());
                assert_eq!(rs["bbox_bottom"].as_i64().unwrap() as i32, s.bbox_bottom);
            }
        }
        // Trailing sections.
        let order: Vec<i32> = r["room_order"]["rooms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_i64().unwrap() as i32)
            .collect();
        assert_eq!(order, p.room_order);
        assert_eq!(
            r["last_instance_id"].as_i64().unwrap() as i32,
            p.last_instance_id
        );
        assert_eq!(
            r["settings"]["icon"]["len"].as_u64().unwrap() as usize,
            p.settings.icon.len()
        );
        assert_eq!(
            r["library_creation_code"]["code"].as_array().unwrap().len(),
            p.library_init_code.len()
        );
        let rt = r["resource_tree"].as_array().unwrap();
        for (a, b) in rt.iter().zip(&p.resource_tree) {
            check_tree(a, b);
        }
    }
}
