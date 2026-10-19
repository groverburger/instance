use std::path::PathBuf;

use super::*;
use crate::actionlib::builtin_action;
use crate::gmk::{canonical_bytes, write_project};
use crate::model::Path as GmPath;
use crate::model::*;

/// A fresh, empty scratch directory under the system temp dir.
fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("instance-folder-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn code(c: &str) -> Action {
    let mut a = builtin_action(1, 603).unwrap().instantiate(1);
    a.args[0] = c.into();
    a
}

fn img(w: u32, h: u32, seed: u8) -> RgbaImage {
    let mut i = RgbaImage::new(w, h);
    for (k, b) in i.data.iter_mut().enumerate() {
        *b = (k as u8).wrapping_mul(31).wrapping_add(seed);
    }
    i
}

/// A project that exercises every corner of the folder format.
fn rich() -> Project {
    let mut p = Project {
        game_id: 123456,
        guid: *b"0123456789abcdef",
        last_instance_id: 100005,
        last_tile_id: 10000003,
        triggers_last_changed: 45000.123456789,
        constants_last_changed: 1e-7,
        ..Project::default()
    };
    p.settings.icon = vec![0, 0, 1, 0, 9, 9];
    p.settings.loading_bar = 2;
    p.settings.loading_bar_back = Some(b"BM\x01\x02".to_vec());
    p.settings.loading_bar_front = Some(Vec::new());
    p.settings.author = "Zo\u{eb} \u{20ac}\u{81}".into();
    p.settings.information = "line1\r\nline2\n".into();
    p.settings.argument_errors = true;
    p.settings.uninitialized_extra_bits = 12;
    p.game_info.rtf = "{\\rtf1 hi\\par\r\n}\r\n".into();
    p.constants = vec![
        Constant {
            name: "A".into(),
            value: "1".into(),
        },
        Constant {
            name: "B C".into(),
            value: "\"x\"\r\n".into(),
        },
    ];
    p.triggers = vec![
        Some(Trigger {
            name: "trig one".into(),
            condition: "x > 1\n".into(),
            moment: 1,
            constant_name: "ev_trig".into(),
        }),
        None,
    ];
    p.sounds = vec![
        Some(Sound {
            name: "snd_a".into(),
            last_changed: 44000.5,
            kind: 1,
            file_type: ".wav".into(),
            file_name: "C:\\x\\a.wav".into(),
            data: Some(vec![1, 2, 3]),
            effects: 5,
            volume: 0.75,
            pan: -0.25,
            preload: true,
        }),
        None,
        Some(Sound {
            name: "snd_none".into(),
            file_type: "weird ext".into(),
            ..Sound::default()
        }),
    ];
    let mut spr = Sprite {
        name: "spr_a".into(),
        origin_x: 3,
        origin_y: -2,
        frames: vec![img(4, 3, 0), img(4, 3, 9), RgbaImage::new(0, 0)],
        bbox_right: 3,
        bbox_bottom: 2,
        ..Sprite::default()
    };
    spr.last_changed = 45123.000001;
    p.sprites = vec![
        Some(spr),
        None,
        Some(Sprite {
            name: "dup".into(),
            ..Sprite::default()
        }),
        Some(Sprite {
            name: "dup".into(),
            ..Sprite::default()
        }),
        Some(Sprite {
            name: "".into(),
            ..Sprite::default()
        }),
        Some(Sprite {
            name: "con".into(),
            ..Sprite::default()
        }),
    ];
    p.backgrounds = vec![
        Some(Background {
            name: "bg_tiles".into(),
            use_as_tileset: true,
            image: img(8, 8, 3),
            ..Background::default()
        }),
        Some(Background {
            name: "bg_empty".into(),
            ..Background::default()
        }),
        Some(Background {
            name: "bg_zero_w".into(),
            image: RgbaImage {
                width: 0,
                height: 7,
                data: Vec::new(),
            },
            ..Background::default()
        }),
    ];
    p.paths = vec![Some(GmPath {
        name: "pth".into(),
        backdrop_room: 0,
        points: vec![
            PathPoint {
                x: 1.5,
                y: 2.0,
                speed: 100.0,
            },
            PathPoint {
                x: -3.0,
                y: 1e10,
                speed: 50.0,
            },
        ],
        ..GmPath::default()
    })];
    p.scripts = vec![
        Some(Script {
            name: "scr_crlf".into(),
            code: "a = 1;\r\nb = 2;".into(),
            ..Script::default()
        }),
        Some(Script {
            name: "scr_lf".into(),
            code: "a = 1;\nb = 2;\n".into(),
            ..Script::default()
        }),
        Some(Script {
            name: "scr_raw".into(),
            code: "a\r\nb\nc\rd".into(),
            ..Script::default()
        }),
        None,
        Some(Script {
            name: "scr/odd:name".into(),
            code: String::new(),
            ..Script::default()
        }),
    ];
    p.fonts = vec![Some(Font {
        name: "fnt".into(),
        bold: true,
        charset: 1,
        antialias: 3,
        ..Font::default()
    })];
    let mut dnd = builtin_action(1, 101)
        .map(|d| d.instantiate(1))
        .unwrap_or_default();
    dnd.relative = true;
    dnd.applies_to = 1;
    let mut odd_code = code("x = 1;\r\n");
    odd_code.applies_to = APPLIES_TO_OTHER;
    let mut custom = Action {
        library_id: 1234,
        action_id: 7,
        kind: action_kind::NORMAL,
        exe_type: exe_type::FUNCTION,
        function_name: "my_fn".into(),
        argument_count: 2,
        arg_kinds: vec![0, 1, 0, 0, 0, 0, 0, 0],
        applies_to: APPLIES_TO_SELF,
        args: vec!["1".into(), "two".into(), "0".into()],
        negate: true,
        ..Action::default()
    };
    custom.is_question = true;
    let mut tailed = code("y");
    tailed.args[5] = "junk".into();
    p.timelines = vec![Some(Timeline {
        name: "tl".into(),
        moments: vec![
            Moment {
                step: 30,
                actions: vec![code("a();")],
            },
            Moment {
                step: 0,
                actions: vec![code("b();"), dnd.clone()],
            },
            Moment {
                step: 30,
                actions: vec![],
            },
        ],
        ..Timeline::default()
    })];
    p.objects = vec![
        Some(Object {
            name: "obj_a".into(),
            sprite: 0,
            mask_sprite: 3,
            parent: 1,
            depth: -5,
            events: vec![
                Event {
                    event_type: event_type::CREATE,
                    number: 0,
                    actions: vec![code("hp = 3;\r\n")],
                },
                Event {
                    event_type: event_type::ALARM,
                    number: 2,
                    actions: vec![odd_code],
                },
                Event {
                    event_type: event_type::STEP,
                    number: 1,
                    actions: vec![code("lf\n")],
                },
                Event {
                    event_type: event_type::STEP,
                    number: 0,
                    actions: vec![dnd, custom, tailed],
                },
                Event {
                    event_type: event_type::COLLISION,
                    number: 1,
                    actions: vec![code("c")],
                },
                Event {
                    event_type: event_type::COLLISION,
                    number: 2,
                    actions: vec![code("d")],
                },
                Event {
                    event_type: event_type::COLLISION,
                    number: 9,
                    actions: vec![code("e")],
                },
                Event {
                    event_type: event_type::KEYBOARD,
                    number: 37,
                    actions: vec![code("k")],
                },
                Event {
                    event_type: event_type::KEY_PRESS,
                    number: 65,
                    actions: vec![code("k")],
                },
                Event {
                    event_type: event_type::KEY_RELEASE,
                    number: 999,
                    actions: vec![code("k")],
                },
                Event {
                    event_type: event_type::MOUSE,
                    number: 54,
                    actions: vec![code("m")],
                },
                Event {
                    event_type: event_type::OTHER,
                    number: 14,
                    actions: vec![code("u")],
                },
                Event {
                    event_type: event_type::DRAW,
                    number: 0,
                    actions: vec![],
                },
                Event {
                    event_type: event_type::DRAW,
                    number: 0,
                    actions: vec![code("dup draw")],
                },
                Event {
                    event_type: event_type::DRAW,
                    number: 7,
                    actions: vec![code("odd draw")],
                },
                Event {
                    event_type: event_type::TRIGGER,
                    number: 0,
                    actions: vec![code("t")],
                },
            ],
            ..Object::default()
        }),
        Some(Object {
            name: "obj_parent".into(),
            ..Object::default()
        }),
        Some(Object {
            name: "has space".into(),
            ..Object::default()
        }),
        None,
    ];
    let mut room = Room {
        name: "rm_main".into(),
        caption: "Caption \u{e9}".into(),
        creation_code: "room_code();\r\n".into(),
        views_enabled: true,
        instances: vec![
            Instance {
                x: 1,
                y: 2,
                object: 0,
                id: 100001,
                creation_code: String::new(),
                locked: false,
            },
            Instance {
                x: -1,
                y: 5,
                object: 2,
                id: 100002,
                creation_code: "speed = 2;\r\n".into(),
                locked: true,
            },
            Instance {
                x: 0,
                y: 0,
                object: 3,
                id: 100002,
                creation_code: "lf\n".into(),
                locked: false,
            },
            Instance {
                x: 0,
                y: 0,
                object: 77,
                id: 100003,
                creation_code: String::new(),
                locked: false,
            },
        ],
        tiles: vec![Tile {
            x: 1,
            y: 2,
            background: 0,
            tile_x: 8,
            tile_y: 0,
            width: 8,
            height: 8,
            depth: 1000000,
            id: 10000001,
            locked: true,
        }],
        editor: RoomEditorState {
            editor_width: 800,
            show_grid: 1,
            h_scroll: 17,
            ..RoomEditorState::default()
        },
        ..Room::default()
    };
    room.backgrounds[0].visible = true;
    room.backgrounds[0].background = 0;
    room.backgrounds[2].background = 5;
    room.views[0].visible = true;
    room.views[0].follow_object = 0;
    room.views.pop();
    p.rooms = vec![
        None,
        Some(room),
        Some(Room {
            name: "rm_two".into(),
            ..Room::default()
        }),
    ];
    p.room_order = vec![2, 1];
    p.included_files = vec![
        IncludedFile {
            file_name: "data.txt".into(),
            data: Some(b"hello".to_vec()),
            export_kind: 2,
            ..IncludedFile::default()
        },
        IncludedFile {
            file_name: "data.txt".into(),
            original_path: "C:\\data.txt".into(),
            ..IncludedFile::default()
        },
    ];
    p.extension_packages = vec!["Ext One".into()];
    p.library_init_code = vec![String::new(), "init();\r\n".into()];
    p.resource_tree[0].children = vec![
        TreeNode::group(
            ResourceKind::Sprite,
            "Group/",
            vec![TreeNode::leaf(ResourceKind::Sprite, 0, "spr_a")],
        ),
        TreeNode::group(
            ResourceKind::Sprite,
            "Players",
            vec![
                TreeNode::leaf(ResourceKind::Sprite, 2, "dup"),
                TreeNode::leaf(ResourceKind::Sprite, 3, "dup"),
                TreeNode::leaf(ResourceKind::Sprite, 4, ""),
                TreeNode::leaf(ResourceKind::Sprite, 5, "con"),
                TreeNode::leaf(ResourceKind::Sprite, 1, "deleted"),
            ],
        ),
        TreeNode::leaf(ResourceKind::Object, 0, "wrong kind"),
    ];
    p.resource_tree[7].children = vec![
        TreeNode::leaf(ResourceKind::Object, 0, "obj_a"),
        TreeNode::leaf(ResourceKind::Object, 1, "renamed"),
        TreeNode::leaf(ResourceKind::Object, 2, "has space"),
    ];
    p.resource_tree[8].children = vec![
        TreeNode::leaf(ResourceKind::Room, 1, "rm_main"),
        TreeNode::leaf(ResourceKind::Room, 2, "rm_two"),
    ];
    p.resource_tree[4].children = vec![TreeNode {
        status: NodeStatus::Unknown(9),
        kind: ResourceKind::Unknown(77),
        index: 4,
        name: " spaced ".into(),
        children: vec![TreeNode::leaf(ResourceKind::Script, 0, "scr_crlf")],
    }];
    p
}

fn round_trip(p: &Project, tag: &str) -> PathBuf {
    let dir = scratch(tag);
    write_folder(p, &dir).unwrap();
    let q = match read_folder(&dir) {
        Ok(q) => q,
        Err(e) => panic!("{e}"),
    };
    if q != *p {
        let a = format!("{p:#?}");
        let b = format!("{q:#?}");
        for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
            assert_eq!(x, y, "first difference at debug line {i}");
        }
        panic!("projects differ");
    }
    let wa = write_project(p, FormatVersion::V810).unwrap();
    let wb = write_project(&q, FormatVersion::V810).unwrap();
    assert_eq!(canonical_bytes(&wa).unwrap(), canonical_bytes(&wb).unwrap());
    dir
}

#[test]
fn rich_project_round_trips() {
    let p = rich();
    let dir = round_trip(&p, "rich");
    assert!(read_project_any(&dir).unwrap() == p);
    let file = dir
        .join("..")
        .join(format!("rich-{}.gm81", std::process::id()));
    let bytes = write_project(&p, FormatVersion::V810).unwrap();
    std::fs::write(&file, &bytes).unwrap();
    assert!(read_project_any(&file).unwrap() == crate::gmk::read_project(&bytes).unwrap());
    let _ = std::fs::remove_file(&file);
    // Spot-check the layout.
    for f in [
        "project.toml",
        "game_information.rtf",
        "resource_tree.txt",
        "settings/icon.ico",
        "settings/loading_bar_back.bmp",
        "settings/loading_bar_front.bin",
        "sprites/spr_a/sprite.toml",
        "sprites/spr_a/1.png",
        "sprites/dup~2/sprite.toml",
        "sprites/_con/sprite.toml",
        "sounds/snd_a.wav",
        "sounds/snd_none.toml",
        "backgrounds/bg_tiles.png",
        "scripts/scr_crlf.gml",
        "scripts/scr_odd_name.gml",
        "objects/obj_a/create.gml",
        "objects/obj_a/alarm_2.actions.toml",
        "objects/obj_a/alarm_2.0.gml",
        "objects/obj_a/step_begin.gml",
        "objects/obj_a/step.actions.toml",
        "objects/obj_a/collision_obj_parent.gml",
        "objects/obj_a/collision_#2.gml",
        "objects/obj_a/collision_#9.gml",
        "objects/obj_a/keyboard_vk_left.gml",
        "objects/obj_a/keypress_A.gml",
        "objects/obj_a/keyrelease_#999.gml",
        "objects/obj_a/mouse_global_right_pressed.gml",
        "objects/obj_a/other_user_4.gml",
        "objects/obj_a/draw.actions.toml",
        "objects/obj_a/draw~2.gml",
        "objects/obj_a/event_8_7.gml",
        "objects/obj_a/trigger_#0.gml",
        "timelines/tl/step_30.gml",
        "timelines/tl/step_30~2.actions.toml",
        "rooms/rm_main/creation.gml",
        "rooms/rm_main/instances/100002.gml",
        "rooms/rm_main/instances/100002~2.gml",
        "triggers/trig one.gml",
        "included_files/data.txt",
        "included_files/data.txt~2.toml",
    ] {
        assert!(dir.join(f).is_file(), "missing {f}");
    }
    let lf = std::fs::read(dir.join("scripts/scr_crlf.gml")).unwrap();
    assert_eq!(lf, b"a = 1;\nb = 2;");
    let obj = std::fs::read_to_string(dir.join("objects/obj_a/object.toml")).unwrap();
    assert!(obj.contains("sprite = \"spr_a\""), "{obj}");
    assert!(obj.contains("mask_sprite = \"#3\""), "{obj}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn output_is_deterministic_and_prunes_stale_files() {
    let p = rich();
    let a = folder_files(&p, FormatVersion::V810).unwrap();
    let b = folder_files(&p, FormatVersion::V810).unwrap();
    assert_eq!(a, b);
    let dir = scratch("prune");
    write_folder(&p, &dir).unwrap();
    std::fs::write(dir.join("notes.txt"), "mine").unwrap();
    let mut q = p.clone();
    q.scripts[0] = None;
    q.objects[0].as_mut().unwrap().events.truncate(2);
    write_folder(&q, &dir).unwrap();
    assert!(!dir.join("scripts/scr_crlf.gml").exists());
    assert!(!dir.join("objects/obj_a/draw.actions.toml").exists());
    assert!(dir.join("notes.txt").exists());
    assert_eq!(read_folder(&dir).unwrap(), q);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn resources_missing_from_the_tree_are_found() {
    let mut p = Project::default();
    p.objects.push(Some(Object {
        name: "obj_a".into(),
        ..Object::default()
    }));
    p.rooms.push(Some(Room {
        name: "rm_a".into(),
        ..Room::default()
    }));
    p.room_order = vec![0];
    p.resource_tree[7]
        .children
        .push(TreeNode::leaf(ResourceKind::Object, 0, "obj_a"));
    p.resource_tree[8]
        .children
        .push(TreeNode::leaf(ResourceKind::Room, 0, "rm_a"));
    assert!(p.missing_from_tree().is_empty());
    let dir = scratch("tree-missing");
    write_folder(&p, &dir).unwrap();
    // The room keeps its `index` but its line is removed from the tree: it is still in
    // the game, and missing from the tree (gmk-pack refuses this).
    let tree = std::fs::read_to_string(dir.join("resource_tree.txt")).unwrap();
    assert!(tree.contains("  rm_a\n"), "{tree}");
    std::fs::write(dir.join("resource_tree.txt"), tree.replace("  rm_a\n", "")).unwrap();
    let q = read_folder(&dir).unwrap();
    assert_eq!(q.rooms[0].as_ref().unwrap().name, "rm_a");
    assert_eq!(
        q.missing_from_tree(),
        vec![(ResourceKind::Room, 0, "rm_a".to_string())]
    );
    // A resource without `index` gets a leaf of its own.
    std::fs::create_dir_all(dir.join("objects/obj_new")).unwrap();
    std::fs::write(dir.join("objects/obj_new/step.gml"), "x += 1;\n").unwrap();
    let q = read_folder(&dir).unwrap();
    assert_eq!(q.missing_from_tree().len(), 1);
    // A leaf of another kind does not count.
    let mut q = q;
    q.resource_tree[8]
        .children
        .push(TreeNode::leaf(ResourceKind::Object, 0, "rm_a"));
    assert_eq!(q.missing_from_tree().len(), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn hand_edits() {
    let mut p = Project::default();
    p.sprites.push(Some(Sprite {
        name: "spr".into(),
        frames: vec![img(2, 2, 1)],
        ..Sprite::default()
    }));
    p.objects.push(Some(Object {
        name: "obj".into(),
        sprite: 0,
        ..Object::default()
    }));
    p.resource_tree[0]
        .children
        .push(TreeNode::leaf(ResourceKind::Sprite, 0, "spr"));
    p.resource_tree[7]
        .children
        .push(TreeNode::leaf(ResourceKind::Object, 0, "obj"));
    let dir = scratch("edits");
    write_folder(&p, &dir).unwrap();
    // New script and new event written with LF (and a BOM): packed as CR LF.
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    std::fs::write(dir.join("scripts/scr_new.gml"), "\u{FEFF}a = 1;\nb = 2;\n").unwrap();
    std::fs::write(dir.join("objects/obj/step.gml"), "x += 1;\n").unwrap();
    let q = read_folder(&dir).unwrap();
    assert_eq!(q.scripts[0].as_ref().unwrap().name, "scr_new");
    assert_eq!(q.scripts[0].as_ref().unwrap().code, "a = 1;\r\nb = 2;\r\n");
    assert_eq!(q.resource_tree[4].children[0].name, "scr_new");
    let o = q.objects[0].as_ref().unwrap();
    assert_eq!(o.events[0].event_type, event_type::STEP);
    assert_eq!(o.events[0].actions[0].args[0], "x += 1;\r\n");
    // New sprite and object directories without metadata files.
    std::fs::create_dir_all(dir.join("sprites/spr_new")).unwrap();
    std::fs::write(
        dir.join("sprites/spr_new/0.png"),
        encode_png(&img(3, 2, 7), "t").unwrap(),
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("objects/obj_new")).unwrap();
    std::fs::write(dir.join("objects/obj_new/collision_obj.gml"), "hit();").unwrap();
    let q = read_folder(&dir).unwrap();
    let s = q.sprites[1].as_ref().unwrap();
    assert_eq!(
        (s.name.as_str(), s.frames.len(), s.width()),
        ("spr_new", 1, 3)
    );
    let o = q.objects[1].as_ref().unwrap();
    assert_eq!(o.name, "obj_new");
    assert_eq!(
        (o.events[0].event_type, o.events[0].number),
        (event_type::COLLISION, 0)
    );
    assert!(q.resource_tree[7]
        .children
        .iter()
        .any(|n| n.name == "obj_new" && n.index == 1));
    std::fs::remove_dir_all(dir.join("sprites/spr_new")).unwrap();
    std::fs::remove_dir_all(dir.join("objects/obj_new")).unwrap();
    // Unknown references are reported with the file.
    let meta = dir.join("objects/obj/object.toml");
    let text = std::fs::read_to_string(&meta).unwrap();
    std::fs::write(&meta, text.replace("\"spr\"", "\"spr_missing\"")).unwrap();
    let e = read_folder(&dir).unwrap_err().to_string();
    assert!(
        e.contains("objects/obj/object.toml") && e.contains("unknown sprite \"spr_missing\""),
        "{e}"
    );
    std::fs::write(&meta, text.replace("depth = 0", "depth = \"deep\"")).unwrap();
    let e = read_folder(&dir).unwrap_err().to_string();
    assert!(e.contains("depth"), "{e}");
    std::fs::write(&meta, text).unwrap();
    std::fs::write(dir.join("objects/obj/stpe.gml"), "").unwrap();
    let e = read_folder(&dir).unwrap_err().to_string();
    assert!(e.contains("stpe"), "{e}");
    std::fs::remove_file(dir.join("objects/obj/stpe.gml")).unwrap();
    std::fs::write(dir.join("scripts/scr_new.gml"), "s = \"\u{1F600}\";").unwrap();
    let e = read_folder(&dir).unwrap_err().to_string();
    assert!(e.contains("Windows-1252"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn png_variants_decode() {
    // A 2x1 palette PNG with transparency.
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_palette(vec![255, 0, 0, 0, 0, 255]);
        enc.set_trns(vec![255, 0]);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[0, 1]).unwrap();
    }
    let i = decode_png(&out, "x.png").unwrap();
    assert_eq!(i.data, vec![255, 0, 0, 255, 0, 0, 255, 0]);
    // 16-bit grey.
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 1, 1);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Sixteen);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[0x80, 0x00]).unwrap();
    }
    assert_eq!(
        decode_png(&out, "y.png").unwrap().data,
        vec![0x80, 0x80, 0x80, 255]
    );
}

#[test]
fn eol_modes() {
    for s in [
        "", "a", "a\r\nb", "a\nb\n", "a\r\nb\n", "a\rb", "\r", "\n\r",
    ] {
        let e = Eol::of(s);
        assert_eq!(e.restore(&e.to_disk(s)), s, "{s:?}");
    }
    assert_eq!(Eol::of("a\r\nb"), Eol::Crlf);
    assert_eq!(Eol::of("a\nb"), Eol::Lf);
    assert_eq!(Eol::of("a\r\nb\n"), Eol::Raw);
}

/// Round trip over real projects listed in `INSTANCE_FOLDER_SAMPLES` (`;`-separated
/// `.gm81` / `.gmk` files or directories to search; only read): the folder must read
/// back to an equal project, and packing it must give the original file's
/// [`canonical_bytes`] (v800 / v810 files).
#[test]
fn samples_round_trip() {
    fn walk(p: &std::path::Path, out: &mut Vec<PathBuf>) {
        if p.is_dir() {
            let mut v: Vec<PathBuf> = std::fs::read_dir(p)
                .unwrap()
                .flatten()
                .map(|e| e.path())
                .collect();
            v.sort();
            for x in v {
                walk(&x, out);
            }
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| x.eq_ignore_ascii_case("gm81") || x.eq_ignore_ascii_case("gmk"))
        {
            out.push(p.to_path_buf());
        }
    }
    fn count(d: &std::path::Path) -> usize {
        std::fs::read_dir(d)
            .unwrap()
            .flatten()
            .map(|e| {
                if e.path().is_dir() {
                    count(&e.path())
                } else {
                    1
                }
            })
            .sum()
    }
    let list = std::env::var("INSTANCE_FOLDER_SAMPLES").unwrap_or_default();
    let mut files = Vec::new();
    for x in list.split(';').filter(|s| !s.is_empty()) {
        walk(std::path::Path::new(x), &mut files);
    }
    for (k, path) in files.iter().enumerate() {
        let data = std::fs::read(path).unwrap();
        let (p, v) = crate::gmk::read_project_versioned(&data).unwrap();
        let dir = round_trip(&p, &format!("sample{k}"));
        if v != FormatVersion::V701 {
            let q = read_folder(&dir).unwrap();
            let packed = write_project(&q, v).unwrap();
            assert!(
                canonical_bytes(&packed).unwrap() == canonical_bytes(&data).unwrap(),
                "{}: packed folder differs from the original",
                path.display()
            );
        }
        eprintln!(
            "ok {} (v{}, {} files)",
            path.display(),
            v.number(),
            count(&dir)
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
