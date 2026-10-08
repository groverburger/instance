//! Synthetic round-trip tests covering every record type, including ones never seen in
//! the sample files (triggers, constants, paths, fonts, timelines, tiles, includes,
//! extension packages, custom loading images).

use instance_format::gmk::{decode_v701, encode_v701};
use instance_format::*;

fn img(w: u32, h: u32, seed: u8) -> RgbaImage {
    let data = (0..w * h * 4)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect();
    RgbaImage {
        width: w,
        height: h,
        data,
    }
}

fn action(id: i32, args: &[&str]) -> Action {
    let mut a = builtin_action(1, id).expect("builtin").instantiate(1);
    for (i, v) in args.iter().enumerate() {
        a.args[i] = v.to_string();
    }
    a
}

fn sample_project() -> Project {
    let mut p = Project {
        game_id: 0x1234_5678,
        guid: *b"0123456789abcdef",
        ..Project::default()
    };
    let s = &mut p.settings;
    s.start_fullscreen = true;
    s.scaling = 150;
    s.color_outside_room = 0x00FF8040;
    s.vsync = 3;
    s.disable_screensaver = false;
    s.loading_bar = 2;
    s.loading_bar_back = Some(vec![1, 2, 3]);
    s.loading_bar_front = None;
    s.show_custom_load_image = true;
    s.custom_load_image = Some(vec![9; 17]);
    s.icon = vec![0, 0, 1, 0, 42];
    s.author = "Zoë \u{20AC}".into();
    s.information = "multi\r\nline".into();
    s.info_last_changed = 40069.4855;
    s.last_changed = 40070.25;
    s.version_build = 77;
    s.company = "ACME".into();

    p.triggers = vec![
        Some(Trigger {
            name: "low_health".into(),
            condition: "health < 10".into(),
            moment: 2,
            constant_name: "trig_low".into(),
        }),
        None,
        Some(Trigger::default()),
    ];
    p.triggers_last_changed = 40000.5;
    p.constants = vec![
        Constant {
            name: "MAX_SPEED".into(),
            value: "12".into(),
        },
        Constant {
            name: "TITLE".into(),
            value: "\"Hi\"".into(),
        },
    ];
    p.constants_last_changed = 40001.0;

    p.sounds = vec![
        Some(Sound {
            name: "snd".into(),
            last_changed: 1.5,
            kind: 1,
            file_type: ".wav".into(),
            file_name: "a.wav".into(),
            data: Some(b"RIFF....WAVE".to_vec()),
            effects: 5,
            volume: 0.75,
            pan: -0.5,
            preload: true,
        }),
        None,
        Some(Sound {
            name: "nodata".into(),
            ..Sound::default()
        }),
    ];
    p.sprites = vec![
        Some(Sprite {
            name: "spr".into(),
            last_changed: 2.0,
            origin_x: 3,
            origin_y: 4,
            frames: vec![img(5, 3, 1), img(5, 3, 2)],
            collision_shape: 2,
            alpha_tolerance: 128,
            separate_collision_masks: true,
            bbox_mode: 2,
            bbox_left: 1,
            bbox_right: 4,
            bbox_bottom: 2,
            bbox_top: 0,
        }),
        Some(Sprite {
            name: "empty".into(),
            ..Sprite::default()
        }),
    ];
    p.backgrounds = vec![
        None,
        Some(Background {
            name: "bg".into(),
            last_changed: 3.0,
            use_as_tileset: true,
            tile_width: 8,
            tile_height: 4,
            tile_h_offset: 1,
            tile_v_offset: 2,
            tile_h_sep: 3,
            tile_v_sep: 4,
            image: img(16, 8, 7),
        }),
        Some(Background {
            name: "noimage".into(),
            ..Background::default()
        }),
    ];
    p.paths = vec![Some(Path {
        name: "pth".into(),
        last_changed: 4.0,
        connection_kind: 1,
        closed: false,
        precision: 6,
        backdrop_room: 0,
        snap_x: 8,
        snap_y: 10,
        points: vec![
            PathPoint {
                x: 1.5,
                y: 2.5,
                speed: 100.0,
            },
            PathPoint {
                x: -3.0,
                y: 40.0,
                speed: 50.0,
            },
        ],
    })];
    p.scripts = vec![
        Some(Script {
            name: "scr".into(),
            last_changed: 5.0,
            code: "return argument0 + 1;".into(),
        }),
        None,
    ];
    p.fonts = vec![Some(Font {
        name: "fnt".into(),
        last_changed: 6.0,
        font_name: "Courier New".into(),
        size: 14,
        bold: true,
        italic: true,
        range_start: 32,
        range_end: 255,
        charset: 1,
        antialias: 3,
    })];
    p.timelines = vec![Some(Timeline {
        name: "tl".into(),
        last_changed: 7.0,
        moments: vec![
            Moment {
                step: 0,
                actions: vec![action(603, &["x += 1"])],
            },
            Moment {
                step: 30,
                actions: vec![],
            },
        ],
    })];
    let mut q = action(101, &["000000010", "4"]);
    q.relative = true;
    q.applies_to = APPLIES_TO_OTHER;
    let mut test = builtin_libraries()
        .iter()
        .flat_map(|l| l.actions.iter())
        .find(|a| a.is_question && !a.is_decoration())
        .unwrap()
        .instantiate(1);
    test.negate = true;
    p.objects = vec![
        Some(Object {
            name: "obj".into(),
            last_changed: 8.0,
            sprite: 0,
            solid: true,
            visible: false,
            depth: -10,
            persistent: true,
            parent: NO_PARENT,
            mask_sprite: 1,
            events: vec![
                Event {
                    event_type: event_type::CREATE,
                    number: 0,
                    actions: vec![q.clone(), test],
                },
                Event {
                    event_type: event_type::KEYBOARD,
                    number: 40,
                    actions: vec![q.clone()],
                },
                Event {
                    event_type: event_type::KEYBOARD,
                    number: 37,
                    actions: vec![],
                },
                Event {
                    event_type: event_type::TRIGGER,
                    number: 0,
                    actions: vec![action(603, &["show_message('t')"])],
                },
            ],
        }),
        None,
        Some(Object {
            name: "child".into(),
            parent: 0,
            ..Object::default()
        }),
    ];
    let mut room = Room {
        name: "rm".into(),
        last_changed: 9.0,
        caption: "Level 1".into(),
        width: 800,
        height: 600,
        speed: 60,
        persistent: true,
        isometric_grid: true,
        creation_code: "score = 0;".into(),
        views_enabled: true,
        ..Room::default()
    };
    room.backgrounds[0] = RoomBackground {
        visible: true,
        is_foreground: true,
        background: 1,
        x: 5,
        y: 6,
        tile_h: false,
        tile_v: true,
        hspeed: 2,
        vspeed: -1,
        stretch: true,
    };
    room.views[1].visible = true;
    room.views[1].follow_object = 0;
    room.instances = vec![
        Instance {
            x: 10,
            y: 20,
            object: 0,
            id: 100001,
            creation_code: "hp = 3;".into(),
            locked: false,
        },
        Instance {
            x: -5,
            y: 7,
            object: 2,
            id: 100002,
            creation_code: String::new(),
            locked: true,
        },
    ];
    room.tiles = vec![Tile {
        x: 32,
        y: 64,
        background: 1,
        tile_x: 8,
        tile_y: 4,
        width: 8,
        height: 4,
        depth: 1000000,
        id: 10000001,
        locked: true,
    }];
    room.editor.remember_room_editor_info = 1;
    room.editor.editor_width = 1024;
    room.editor.v_scroll = 33;
    p.rooms = vec![
        Some(room),
        None,
        Some(Room {
            name: "rm2".into(),
            ..Room::default()
        }),
    ];
    p.last_instance_id = 100002;
    p.last_tile_id = 10000001;
    p.included_files = vec![
        IncludedFile {
            last_changed: 10.0,
            file_name: "data.txt".into(),
            original_path: "C:\\x\\data.txt".into(),
            original_file_chosen: true,
            original_size: 4,
            data: Some(b"abcd".to_vec()),
            export_kind: 3,
            export_folder: "out".into(),
            overwrite_existing: true,
            free_memory_after_export: true,
            remove_at_game_end: false,
        },
        IncludedFile::default(),
    ];
    p.extension_packages = vec!["GM Room Transitions".into(), "Printing".into()];
    p.game_info.rtf = "{\\rtf1\\ansi hello}".into();
    p.game_info.show_in_separate_window = true;
    p.game_info.last_changed = 11.0;
    p.library_init_code = vec![String::new(); 6]
        .into_iter()
        .chain(["lib_init();".to_string()])
        .collect();
    p.room_order = vec![2, 0];
    let tree = &mut p.resource_tree;
    tree[0].children = vec![
        TreeNode::group(
            ResourceKind::Sprite,
            "grp",
            vec![TreeNode::leaf(ResourceKind::Sprite, 1, "empty")],
        ),
        TreeNode::leaf(ResourceKind::Sprite, 0, "spr"),
    ];
    tree[8].children = vec![
        TreeNode::leaf(ResourceKind::Room, 2, "rm2"),
        TreeNode::leaf(ResourceKind::Room, 0, "rm"),
    ];
    p
}

#[test]
fn round_trip_800_and_810() {
    let p = sample_project();
    for v in [FormatVersion::V800, FormatVersion::V810] {
        let bytes = write_project(&p, v).unwrap();
        assert_eq!(detect_version(&bytes).unwrap(), v);
        let (q, qv) = read_project_versioned(&bytes).unwrap();
        assert_eq!(qv, v);
        assert_eq!(q, p, "round trip through {v:?}");
        // Writing is deterministic.
        assert_eq!(write_project(&q, v).unwrap(), bytes);
    }
}

#[test]
fn v810_differs_only_in_header() {
    let p = sample_project();
    let a = canonical_bytes(&write_project(&p, FormatVersion::V800).unwrap()).unwrap();
    let b = canonical_bytes(&write_project(&p, FormatVersion::V810).unwrap()).unwrap();
    assert_eq!(a.len(), b.len());
    let diff: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
    assert!(diff.iter().all(|&i| (4..8).contains(&i)), "{diff:?}");
}

#[test]
fn default_project_round_trips() {
    let p = Project::default();
    let q = read_project(&write_project(&p, FormatVersion::V800).unwrap()).unwrap();
    assert_eq!(p, q);
}

#[test]
fn write_v701_is_rejected() {
    assert!(matches!(
        write_project(&Project::default(), FormatVersion::V701),
        Err(Error::InvalidModel(_))
    ));
}

#[test]
fn inconsistent_image_is_rejected() {
    let mut p = Project::default();
    p.sprites.push(Some(Sprite {
        frames: vec![RgbaImage {
            width: 2,
            height: 2,
            data: vec![0; 3],
        }],
        ..Sprite::default()
    }));
    assert!(matches!(
        write_project(&p, FormatVersion::V800),
        Err(Error::InvalidModel(_))
    ));
}

#[test]
fn bad_event_type_is_rejected() {
    let mut p = Project::default();
    p.objects.push(Some(Object {
        events: vec![Event {
            event_type: 12,
            number: 0,
            actions: vec![],
        }],
        ..Object::default()
    }));
    assert!(write_project(&p, FormatVersion::V800).is_err());
}

#[test]
fn events_are_grouped_by_type_on_write() {
    let mut p = Project::default();
    let ev = |t, n| Event {
        event_type: t,
        number: n,
        actions: vec![],
    };
    p.objects.push(Some(Object {
        events: vec![ev(5, 40), ev(0, 0), ev(5, 37)],
        ..Object::default()
    }));
    let q = read_project(&write_project(&p, FormatVersion::V800).unwrap()).unwrap();
    let got: Vec<(i32, i32)> = q.objects[0]
        .as_ref()
        .unwrap()
        .events
        .iter()
        .map(|e| (e.event_type, e.number))
        .collect();
    assert_eq!(got, vec![(0, 0), (5, 40), (5, 37)]);
}

#[test]
fn errors_carry_offsets() {
    let bytes = write_project(&sample_project(), FormatVersion::V800).unwrap();
    // Bad magic.
    let mut b = bytes.clone();
    b[0] ^= 1;
    assert!(matches!(read_project(&b), Err(Error::BadMagic { .. })));
    // Unsupported version.
    let mut b = bytes.clone();
    b[4..8].copy_from_slice(&900i32.to_le_bytes());
    match read_project(&b) {
        Err(Error::UnsupportedVersion { offset, found, .. }) => {
            assert_eq!((offset, found), (4, 900))
        }
        other => panic!("{other:?}"),
    }
    // Truncation.
    let cut = bytes.len() - 3;
    match read_project(&bytes[..cut]) {
        Err(e @ Error::UnexpectedEof { .. }) => assert!(e.offset().unwrap() <= cut),
        other => panic!("{other:?}"),
    }
    // Trailing garbage at top level.
    let mut b = bytes.clone();
    b.push(0);
    match read_project(&b) {
        Err(Error::TrailingBytes {
            offset, remaining, ..
        }) => assert_eq!((offset, remaining), (bytes.len(), 1)),
        other => panic!("{other:?}"),
    }
    // Corrupt zlib block (the settings block starts at offset 32: len, then 78 9C ...).
    let mut b = bytes.clone();
    b[36] = 0;
    b[37] = 0;
    match read_project(&b) {
        Err(Error::Zlib { offset, .. }) => assert_eq!(offset, 32),
        other => panic!("{other:?}"),
    }
}

#[test]
fn zblock_must_be_consumed_exactly() {
    // A script block with one extra byte after the code string.
    use flate2::write::ZlibEncoder;
    use std::io::Write;
    let mut p = Project::default();
    p.scripts.push(Some(Script {
        name: "s".into(),
        last_changed: 0.0,
        code: "a".into(),
    }));
    let good = write_project(&p, FormatVersion::V800).unwrap();
    let mut plain = Vec::new();
    plain.extend_from_slice(&1i32.to_le_bytes());
    plain.extend_from_slice(&1u32.to_le_bytes());
    plain.push(b's');
    plain.extend_from_slice(&0f64.to_le_bytes());
    plain.extend_from_slice(&800i32.to_le_bytes());
    plain.extend_from_slice(&1u32.to_le_bytes());
    plain.push(b'a');
    let enc = |d: &[u8]| {
        let mut e = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(d).unwrap();
        e.finish().unwrap()
    };
    let ok_block = enc(&plain);
    let pos = good
        .windows(ok_block.len())
        .position(|w| w == ok_block.as_slice())
        .expect("script block found");
    plain.push(0);
    let bad_block = enc(&plain);
    let mut bad = good[..pos - 4].to_vec();
    bad.extend_from_slice(&(bad_block.len() as u32).to_le_bytes());
    bad.extend_from_slice(&bad_block);
    bad.extend_from_slice(&good[pos + ok_block.len()..]);
    match read_project(&bad) {
        Err(Error::TrailingBytes {
            context, remaining, ..
        }) => {
            assert!(context.starts_with("scripts[0]"), "{context}");
            assert_eq!(remaining, 1);
        }
        other => panic!("{other:?}"),
    }
}

// ------------------------------------------------------------------ v701 synthetic

#[derive(Default)]
struct W(Vec<u8>);
impl W {
    fn i(&mut self, v: i32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn f(&mut self, v: f64) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn s(&mut self, v: &str) -> &mut Self {
        self.b(v.as_bytes())
    }
    fn b(&mut self, v: &[u8]) -> &mut Self {
        self.i(v.len() as i32);
        self.0.extend_from_slice(v);
        self
    }
    fn n(&mut self, k: usize, v: i32) -> &mut Self {
        for _ in 0..k {
            self.i(v);
        }
        self
    }
}

fn zlib(d: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(d).unwrap();
    e.finish().unwrap()
}

/// 2x2 24-bit BMP: bottom row (red, blue), top row (green, red).
fn bmp24() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&(54u32 + 16).to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&2i32.to_le_bytes());
    b.extend_from_slice(&2i32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&[0u8; 24]);
    b.extend_from_slice(&[0, 0, 255, 255, 0, 0, 0, 0]); // bottom: red, blue, pad
    b.extend_from_slice(&[0, 255, 0, 0, 0, 255, 0, 0]); // top: green, red, pad
    b
}

fn v701_plain() -> Vec<u8> {
    let mut w = W::default();
    w.i(4242);
    w.0.extend_from_slice(b"GUIDGUIDGUIDGUID");
    // Settings (702): 22 i32 before loading_bar ... per spec §8.3.
    w.i(702);
    w.n(4, 0).i(100).n(9, 0); // fullscreen..vsync (14 values)
    w.n(6, 1).i(0).i(0); // f4..close_as_esc, priority, freeze
    w.i(1); // loading_bar
    w.i(0).i(0).i(255).i(1); // custom image, transparent, alpha, scale
    w.b(&[0, 0, 1, 0]); // icon
    w.n(4, 0); // error flags
    w.s("me").s("1.0").f(39000.0).s("info");
    w.i(1).s("C1").s("5"); // constants
    w.i(2).i(3).i(4).i(5).s("co").s("pr").s("cr").s("de");
    // Sounds (400): slot 0 deleted, slot 1 with zlib data.
    w.i(400).i(2).i(0);
    w.i(1)
        .s("snd")
        .i(600)
        .i(0)
        .s(".wav")
        .s("x.wav")
        .i(1)
        .b(&zlib(b"WAVEDATA"));
    w.i(0).f(1.0).f(0.0).i(1);
    // Sprites (400): one transparent precise sprite with one BMP frame.
    w.i(400).i(1).i(1).s("spr").i(542).i(2).i(2);
    w.i(0).i(1).i(1).i(0); // bbox l r b t
    w.i(1).i(0).i(1); // transparent, smooth, preload
    w.i(0).i(1).i(1).i(1); // bbox_mode, precise, origin
    w.i(1).i(10).b(&zlib(&bmp24()));
    // Backgrounds (400): one without image.
    w.i(400).i(1).i(1).s("bg").i(543).i(3).i(1);
    w.i(0).i(0).i(0).i(1).i(16).i(16).n(4, 0).i(0);
    // Paths (420): one.
    w.i(420)
        .i(1)
        .i(1)
        .s("p")
        .i(530)
        .i(1)
        .i(1)
        .i(4)
        .i(-1)
        .i(16)
        .i(16);
    w.i(1).f(1.0).f(2.0).f(100.0);
    // Scripts.
    w.i(400).i(1).i(1).s("scr").i(400).s("x = 1;");
    // Fonts.
    w.i(540)
        .i(1)
        .i(1)
        .s("fnt")
        .i(540)
        .s("Arial")
        .i(12)
        .i(0)
        .i(1)
        .i(32)
        .i(127);
    // Timelines (500): one with one moment and one action.
    w.i(500).i(1).i(1).s("tl").i(500).i(1).i(5);
    w.i(400).i(1).i(440).i(1).i(603).i(7).i(0).i(0).i(1).i(2);
    w.s("")
        .s("")
        .i(1)
        .i(8)
        .i(1)
        .n(7, 0)
        .i(-1)
        .i(0)
        .i(8)
        .s("a = 1");
    for _ in 0..7 {
        w.s("0");
    }
    w.i(0);
    // Objects (400): one with keyboard events 40, 37 (descending).
    w.i(400)
        .i(1)
        .i(1)
        .s("obj")
        .i(430)
        .i(0)
        .i(1)
        .i(1)
        .i(0)
        .i(0)
        .i(-100)
        .i(-1)
        .i(10);
    for t in 0..=10 {
        if t == 5 {
            w.i(40).i(400).i(0).i(37).i(400).i(0);
        }
        w.i(-1);
    }
    // Rooms.
    w.i(400)
        .i(1)
        .i(1)
        .s("rm")
        .i(541)
        .s("cap")
        .i(640)
        .i(480)
        .i(16)
        .i(16)
        .i(0)
        .i(30)
        .i(0);
    w.i(0xC0C0C0).i(1).s("");
    w.i(1)
        .n(10, 0)
        .i(0)
        .i(1)
        .n(14, 0)
        .i(1)
        .i(1)
        .i(2)
        .i(0)
        .i(100001)
        .s("")
        .i(0);
    w.i(0).n(14, 0);
    w.i(100001).i(10000000);
    // Included files (620): one inline record.
    w.i(620)
        .i(1)
        .i(620)
        .s("f.txt")
        .s("C:\\f.txt")
        .i(1)
        .i(3)
        .i(1)
        .b(b"abc");
    w.i(2).s("").i(0).i(1).i(0);
    w.i(700).i(1).s("Pkg");
    w.i(600)
        .i(0x80000018u32 as i32)
        .i(0)
        .s("Game Information")
        .i(-1)
        .i(-1)
        .i(600)
        .i(400);
    w.i(1).i(1).i(0).i(1).s("{\\rtf1}");
    w.i(500).i(1).s("");
    w.i(700).i(1).i(0);
    for (i, (name, kind)) in [
        ("Sprites", 2),
        ("Sounds", 3),
        ("Backgrounds", 6),
        ("Paths", 8),
        ("Scripts", 7),
        ("Fonts", 9),
        ("Time Lines", 12),
        ("Objects", 1),
        ("Rooms", 4),
        ("Game Information", 10),
        ("Global Game Settings", 11),
        ("Extension Packages", 13),
    ]
    .iter()
    .enumerate()
    {
        w.i(if i < 9 { 1 } else { 3 }).i(*kind).i(0).s(name);
        if i == 7 {
            w.i(1).i(3).i(1).i(0).s("obj").i(0);
        } else {
            w.i(0);
        }
    }
    w.0
}

#[test]
fn v701_synthetic() {
    let plain = v701_plain();
    let file = encode_v701(&plain, 26469, 5, 3);
    assert_eq!(decode_v701(&file).unwrap().0, plain);
    let (p, v) = read_project_versioned(&file).unwrap();
    assert_eq!(v, FormatVersion::V701);
    assert_eq!(p.game_id, 4242);
    assert_eq!(p.settings.scaling, 100);
    assert!(p.settings.disable_screensaver);
    assert_eq!(p.settings.author, "me");
    assert_eq!(p.settings.version_build, 5);
    assert_eq!(
        p.constants,
        vec![Constant {
            name: "C1".into(),
            value: "5".into()
        }]
    );
    assert!(p.sounds[0].is_none());
    assert_eq!(
        p.sounds[1].as_ref().unwrap().data.as_deref(),
        Some(&b"WAVEDATA"[..])
    );
    let spr = p.sprites[0].as_ref().unwrap();
    assert_eq!(spr.collision_shape, 0);
    assert_eq!((spr.width(), spr.height()), (2, 2));
    let f = &spr.frames[0];
    // Bottom-left pixel (red) is the transparent colour.
    assert_eq!(f.pixel(0, 1), [255, 0, 0, 0]);
    assert_eq!(f.pixel(1, 0), [255, 0, 0, 0]);
    assert_eq!(f.pixel(0, 0), [0, 255, 0, 255]);
    assert_eq!(f.pixel(1, 1), [0, 0, 255, 255]);
    let bg = p.backgrounds[0].as_ref().unwrap();
    assert_eq!((bg.image.width, bg.image.height), (3, 1));
    assert!(bg.use_as_tileset);
    assert_eq!(p.paths[0].as_ref().unwrap().points.len(), 1);
    assert_eq!(p.scripts[0].as_ref().unwrap().code, "x = 1;");
    assert!(p.fonts[0].as_ref().unwrap().italic);
    let tl = p.timelines[0].as_ref().unwrap();
    assert_eq!(tl.moments[0].step, 5);
    assert_eq!(tl.moments[0].actions[0].args[0], "a = 1");
    let o = p.objects[0].as_ref().unwrap();
    let nums: Vec<_> = o.events.iter().map(|e| (e.event_type, e.number)).collect();
    assert_eq!(nums, vec![(5, 40), (5, 37)]);
    assert_eq!(p.rooms[0].as_ref().unwrap().instances[0].id, 100001);
    assert_eq!(p.included_files[0].data.as_deref(), Some(&b"abc"[..]));
    assert_eq!(p.included_files[0].export_kind, 2);
    assert_eq!(p.extension_packages, vec!["Pkg".to_string()]);
    assert_eq!(p.game_info.background_color, 0x80000018);
    assert_eq!(p.resource_tree[7].children[0].name, "obj");
    // Converting to v800 is lossless from then on.
    let q = read_project(&write_project(&p, FormatVersion::V800).unwrap()).unwrap();
    assert_eq!(p, q);
    // Errors inside the decoded stream report absolute file offsets.
    let cut = &file[..file.len() - 2];
    let e = read_project(cut).unwrap_err();
    assert!(e.offset().unwrap() >= file.len() - 8, "{e}");
}

// ------------------------------------------------------------------ libraries / exports

#[test]
fn builtin_table() {
    let libs = builtin_libraries();
    assert_eq!(libs.len(), 7);
    assert_eq!(libs[0].tab_caption, "move");
    let real: usize = libs
        .iter()
        .map(|l| l.actions.iter().filter(|a| !a.is_decoration()).count())
        .sum();
    assert_eq!(real, 137);
    let mv = builtin_action(1, 101).unwrap();
    assert_eq!(mv.function_name, "action_move");
    assert_eq!(mv.args.len(), 8);
    assert_eq!(mv.args[0].default, "000000000");
    let a = mv.instantiate(1);
    assert_eq!(a.args.len(), 8);
    assert_eq!(a.arg_kinds.len(), 8);
    assert_eq!(a.args[0], "000000000");
    assert_eq!(a.args[7], "0");
    assert!(builtin_action_summary(1, 101).unwrap().contains("keypad"));
    assert!(builtin_action(1, 603).is_some());
    assert!(builtin_action(2, 101).is_none());
}

#[test]
fn library_round_trip() {
    let mut l = builtin_libraries()[2].clone();
    l.author = "someone".into();
    l.init_code = "global.x = 0;".into();
    l.last_changed = 39999.5;
    l.actions[1].image = vec![b'B', b'M', 1, 2, 3];
    l.actions[1].list_text = "Do @0#@r".into();
    let bytes = write_library(&l).unwrap();
    assert_eq!(read_library(&bytes).unwrap(), l);
    let mut b = bytes.clone();
    b.push(1);
    assert!(matches!(read_library(&b), Err(Error::TrailingBytes { .. })));
}

#[test]
fn sprite_export_round_trip() {
    let mut s = sample_project().sprites[0].clone().unwrap();
    let bytes = write_sprite_export(&s).unwrap();
    s.name.clear();
    s.last_changed = 0.0;
    assert_eq!(read_sprite_export(&bytes).unwrap(), s);
}

// ------------------------------------------------------------------ user-data regressions

/// A sprite with 0 frames still stores the 8-field collision/bbox trailer (spec §5.2
/// addendum). Minimal reproduction of the layout seen in user projects: a hand-built
/// block is spliced into a written project and must read back; the writer must emit
/// exactly that block.
#[test]
fn zero_frame_sprite_keeps_trailer() {
    let mut p = Project::default();
    p.sprites.push(Some(Sprite {
        name: "sprite19".into(),
        last_changed: 40000.25,
        collision_shape: 0,
        ..Sprite::default()
    }));
    let good = write_project(&p, FormatVersion::V810).unwrap();

    let mut w = W::default();
    w.i(1).s("sprite19").f(40000.25).i(800); // exists, name, last_changed, version
    w.i(0).i(0).i(0); // origin_x, origin_y, frame_count = 0
    w.n(8, 0); // collision_shape .. bbox_top, present although there are no frames
    let block = zlib(&w.0);
    let pos = good
        .windows(block.len())
        .position(|x| x == block.as_slice())
        .expect("writer emits the trailer for a 0-frame sprite");

    // Splice a variant with non-zero trailer values and read it.
    let mut w = W::default();
    w.i(1).s("sprite19").f(40000.25).i(800);
    w.i(3).i(4).i(0);
    w.i(2).i(17).i(1).i(2).i(5).i(6).i(7).i(8);
    let new_block = zlib(&w.0);
    let mut d = good[..pos - 4].to_vec();
    d.extend_from_slice(&(new_block.len() as u32).to_le_bytes());
    d.extend_from_slice(&new_block);
    d.extend_from_slice(&good[pos + block.len()..]);
    let q = read_project(&d).unwrap();
    let s = q.sprites[0].as_ref().unwrap();
    assert!(s.frames.is_empty());
    assert_eq!((s.origin_x, s.origin_y), (3, 4));
    assert_eq!(
        (
            s.collision_shape,
            s.alpha_tolerance,
            s.separate_collision_masks,
            s.bbox_mode
        ),
        (2, 17, true, 2)
    );
    assert_eq!(
        (s.bbox_left, s.bbox_right, s.bbox_bottom, s.bbox_top),
        (5, 6, 7, 8)
    );
    let again = write_project(&q, FormatVersion::V810).unwrap();
    assert_eq!(
        canonical_bytes(&again).unwrap(),
        canonical_bytes(&d).unwrap()
    );

    // The old (frame-count-conditional) layout without the trailer is rejected.
    let mut w = W::default();
    w.i(1).s("sprite19").f(40000.25).i(800).i(0).i(0).i(0);
    let short = zlib(&w.0);
    let mut d = good[..pos - 4].to_vec();
    d.extend_from_slice(&(short.len() as u32).to_le_bytes());
    d.extend_from_slice(&short);
    d.extend_from_slice(&good[pos + block.len()..]);
    assert!(read_project(&d).is_err());
}

/// The settings word after `abort_on_all_errors` is a bit set; GM 8.1 projects store 2
/// there. Bit 0 alone means "treat uninitialized variables as 0"; other bits round-trip.
#[test]
fn uninitialized_flags_word_is_lossless() {
    for (treat, args, extra, raw) in [
        (false, true, 0, 2i32),
        (true, true, 0, 3),
        (true, false, 0, 1),
        (false, false, 0, 0),
        (false, true, 8, 10),
    ] {
        let mut p = Project::default();
        p.settings.icon = b"ICONICON".to_vec();
        p.settings.display_errors = true;
        p.settings.treat_uninitialized_as_zero = treat;
        p.settings.argument_errors = args;
        p.settings.uninitialized_extra_bits = extra;
        let out = write_project(&p, FormatVersion::V810).unwrap();
        let mut want = W::default();
        want.b(b"ICONICON").i(1).i(0).i(0).i(raw);
        let canon = canonical_bytes(&out).unwrap();
        assert!(
            canon.windows(want.0.len()).any(|x| x == want.0.as_slice()),
            "raw flags {raw} not written"
        );
        let q = read_project(&out).unwrap();
        assert_eq!(q.settings.treat_uninitialized_as_zero, treat);
        assert_eq!(q.settings.argument_errors, args);
        assert_eq!(q.settings.uninitialized_extra_bits, extra);
        assert_eq!(q, p);
    }
}
