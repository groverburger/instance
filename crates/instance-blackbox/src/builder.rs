//! Small builder over the instance-format model for generating test projects.

use instance_format::*;

/// Event numbers used by the generator.
pub mod ev {
    pub const CREATE: (i32, i32) = (0, 0);
    pub const DESTROY: (i32, i32) = (1, 0);
    pub const STEP: (i32, i32) = (3, 0);
    pub const BEGIN_STEP: (i32, i32) = (3, 1);
    pub const END_STEP: (i32, i32) = (3, 2);
    pub const DRAW: (i32, i32) = (8, 0);
    pub fn alarm(n: i32) -> (i32, i32) {
        (2, n)
    }
    pub fn collision(obj: i32) -> (i32, i32) {
        (4, obj)
    }
    pub fn keyboard(k: i32) -> (i32, i32) {
        (5, k)
    }
    pub fn key_press(k: i32) -> (i32, i32) {
        (9, k)
    }
    pub fn key_release(k: i32) -> (i32, i32) {
        (10, k)
    }
    pub fn trigger(n: i32) -> (i32, i32) {
        (11, n)
    }
    pub const OUTSIDE: (i32, i32) = (7, 0);
    pub const BOUNDARY: (i32, i32) = (7, 1);
    pub const GAME_START: (i32, i32) = (7, 2);
    pub const GAME_END: (i32, i32) = (7, 3);
    pub const ROOM_START: (i32, i32) = (7, 4);
    pub const ROOM_END: (i32, i32) = (7, 5);
    pub const NO_LIVES: (i32, i32) = (7, 6);
    pub const ANIM_END: (i32, i32) = (7, 7);
    pub const PATH_END: (i32, i32) = (7, 8);
    pub const NO_HEALTH: (i32, i32) = (7, 9);
    pub fn user(n: i32) -> (i32, i32) {
        (7, 10 + n)
    }
}

/// Fixed timestamp written into every resource (2025-12-08).
const STAMP: f64 = 46000.5;

pub struct Gen {
    pub p: Project,
    pub name: String,
}

/// GM 8.1 stores code with Windows line endings; LF-only code shows up run together
/// on one line in its error messages.
pub fn crlf(code: &str) -> String {
    code.replace("\r\n", "\n").replace('\n', "\r\n")
}

/// A "code" action (library 1, action 603) containing `code`.
pub fn code_action(code: &str) -> Action {
    let mut a = builtin_action(1, 603)
        .expect("built-in code action")
        .instantiate(1);
    a.args[0] = crlf(code);
    a
}

/// Pixel colour helper (r, g, b, a).
pub type Rgba = [u8; 4];

impl Gen {
    pub fn new(name: &str, game_id: i32) -> Gen {
        let mut p = Project {
            game_id,
            ..Project::default()
        };
        let mut guid = [0u8; 16];
        for (i, b) in guid.iter_mut().enumerate() {
            *b = (game_id as u32).wrapping_mul(2654435761).rotate_left(i as u32 * 3) as u8 ^ i as u8;
        }
        p.guid = guid;
        let s = &mut p.settings;
        // Errors are always written to game_errors.log next to the game (diagnostics).
        // bb_lang/bb_args0 raise errors on purpose and record them in
        // error_occurred/error_last, so they don't show dialogs; every other project
        // shows errors so an unexpected failure is visible instead of a silent exit.
        s.display_errors = !(name.starts_with("bb_lang") || name == "bb_args0");
        s.write_errors_to_log = true;
        s.abort_on_all_errors = false;
        s.treat_uninitialized_as_zero = false;
        s.argument_errors = true;
        s.icon = make_ico();
        s.author = "Instance black-box generator".into();
        s.information = format!("Black-box test project {name}");
        s.last_changed = STAMP;
        s.info_last_changed = STAMP;
        s.let_f5_save_f6_load = false;
        s.let_f1_game_info = false;
        p.triggers_last_changed = STAMP;
        p.constants_last_changed = STAMP;
        p.game_info.last_changed = STAMP;
        p.game_info.rtf = format!(
            "{{\\rtf1\\ansi\\ansicpg1252\\deff0{{\\fonttbl{{\\f0\\fnil Arial;}}}}\\f0\\fs24 Black-box test {name}. Results appear on screen; Left/Right pages.\\par }}"
        );
        Gen {
            p,
            name: name.to_string(),
        }
    }

    // ------------------------------------------------------------- resources

    /// A sprite with `frames` frames of `w`x`h`, each filled with `colors[i % len]`.
    pub fn sprite(&mut self, name: &str, w: u32, h: u32, colors: &[Rgba], origin: (i32, i32)) -> i32 {
        let frames = colors
            .iter()
            .map(|c| {
                let mut img = RgbaImage::new(w, h);
                for px in img.data.chunks_mut(4) {
                    px.copy_from_slice(c);
                }
                img
            })
            .collect();
        self.sprite_frames(name, frames, origin, |_| {})
    }

    pub fn sprite_frames(
        &mut self,
        name: &str,
        frames: Vec<RgbaImage>,
        origin: (i32, i32),
        tweak: impl FnOnce(&mut Sprite),
    ) -> i32 {
        let w = frames.first().map_or(0, |f| f.width) as i32;
        let h = frames.first().map_or(0, |f| f.height) as i32;
        // Automatic bbox = tight box of non-transparent pixels over all frames.
        let (mut l, mut t, mut r, mut b) = (w, h, -1, -1);
        for f in &frames {
            for y in 0..f.height {
                for x in 0..f.width {
                    if f.pixel(x, y)[3] > 0 {
                        l = l.min(x as i32);
                        t = t.min(y as i32);
                        r = r.max(x as i32);
                        b = b.max(y as i32);
                    }
                }
            }
        }
        if r < 0 {
            (l, t, r, b) = (0, 0, w - 1, h - 1);
        }
        let mut s = Sprite {
            name: name.into(),
            last_changed: STAMP,
            origin_x: origin.0,
            origin_y: origin.1,
            frames,
            collision_shape: 1,
            alpha_tolerance: 0,
            separate_collision_masks: false,
            bbox_mode: 0,
            bbox_left: l,
            bbox_right: r,
            bbox_top: t,
            bbox_bottom: b,
        };
        tweak(&mut s);
        self.p.sprites.push(Some(s));
        (self.p.sprites.len() - 1) as i32
    }

    pub fn font(&mut self, name: &str, family: &str, size: i32) -> i32 {
        self.p.fonts.push(Some(Font {
            name: name.into(),
            last_changed: STAMP,
            font_name: family.into(),
            size,
            ..Font::default()
        }));
        (self.p.fonts.len() - 1) as i32
    }

    /// A sound resource holding `wav` (a complete .wav file).
    pub fn sound(&mut self, name: &str, kind: i32, wav: Vec<u8>) -> i32 {
        self.p.sounds.push(Some(Sound {
            name: name.into(),
            last_changed: STAMP,
            kind,
            file_type: ".wav".into(),
            file_name: format!("{name}.wav"),
            data: Some(wav),
            effects: 0,
            volume: 1.0,
            pan: 0.0,
            preload: true,
        }));
        (self.p.sounds.len() - 1) as i32
    }

    pub fn script(&mut self, name: &str, code: &str) -> i32 {
        self.p.scripts.push(Some(Script {
            name: name.into(),
            last_changed: STAMP,
            code: crlf(code),
        }));
        (self.p.scripts.len() - 1) as i32
    }

    pub fn path(&mut self, name: &str, closed: bool, smooth: bool, pts: &[(f64, f64, f64)]) -> i32 {
        self.p.paths.push(Some(Path {
            name: name.into(),
            last_changed: STAMP,
            connection_kind: smooth as i32,
            closed,
            points: pts
                .iter()
                .map(|&(x, y, speed)| PathPoint { x, y, speed })
                .collect(),
            ..Path::default()
        }));
        (self.p.paths.len() - 1) as i32
    }

    /// A time line; `moments` = (step, code).
    pub fn timeline(&mut self, name: &str, moments: &[(i32, String)]) -> i32 {
        self.p.timelines.push(Some(Timeline {
            name: name.into(),
            last_changed: STAMP,
            moments: moments
                .iter()
                .map(|(s, c)| Moment {
                    step: *s,
                    actions: vec![code_action(c)],
                })
                .collect(),
        }));
        (self.p.timelines.len() - 1) as i32
    }

    pub fn trigger(&mut self, name: &str, condition: &str, moment: i32) -> i32 {
        self.p.triggers.push(Some(Trigger {
            name: name.into(),
            condition: condition.into(),
            moment,
            constant_name: String::new(),
        }));
        (self.p.triggers.len() - 1) as i32
    }

    pub fn constant(&mut self, name: &str, value: &str) {
        self.p.constants.push(Constant {
            name: name.into(),
            value: value.into(),
        });
    }

    pub fn object(&mut self, name: &str) -> i32 {
        self.p.objects.push(Some(Object {
            name: name.into(),
            last_changed: STAMP,
            ..Object::default()
        }));
        (self.p.objects.len() - 1) as i32
    }

    pub fn obj(&mut self, i: i32) -> &mut Object {
        self.p.objects[i as usize].as_mut().unwrap()
    }

    /// Appends code actions to an event (creating the event if needed).
    pub fn event(&mut self, obj: i32, e: (i32, i32), codes: &[&str]) {
        let o = self.obj(obj);
        let actions: Vec<Action> = codes.iter().map(|c| code_action(c)).collect();
        if let Some(ev) = o
            .events
            .iter_mut()
            .find(|x| x.event_type == e.0 && x.number == e.1)
        {
            ev.actions.extend(actions);
        } else {
            o.events.push(Event {
                event_type: e.0,
                number: e.1,
                actions,
            });
            // keep file order: by type, then number
            o.events.sort_by_key(|x| (x.event_type, x.number));
        }
    }

    pub fn room(&mut self, name: &str, w: i32, h: i32) -> i32 {
        self.p.rooms.push(Some(Room {
            name: name.into(),
            last_changed: STAMP,
            caption: String::new(),
            width: w,
            height: h,
            speed: 30,
            background_color: 0x202020,
            ..Room::default()
        }));
        (self.p.rooms.len() - 1) as i32
    }

    pub fn room_mut(&mut self, r: i32) -> &mut Room {
        self.p.rooms[r as usize].as_mut().unwrap()
    }

    /// Places an instance; returns its id.
    pub fn place(&mut self, room: i32, obj: i32, x: i32, y: i32, creation_code: &str) -> i32 {
        self.p.last_instance_id += 1;
        let id = self.p.last_instance_id;
        self.room_mut(room).instances.push(Instance {
            x,
            y,
            object: obj,
            id,
            creation_code: crlf(creation_code),
            locked: false,
        });
        id
    }

    /// Finishes the project: resource tree and room order.
    pub fn finish(mut self) -> Project {
        let p = &mut self.p;
        p.room_order = (0..p.rooms.len() as i32).collect();
        let mut roots = TreeNode::standard_roots();
        fn leaves<T>(v: &[Option<T>], kind: ResourceKind, name: impl Fn(&T) -> String) -> Vec<TreeNode> {
            v.iter()
                .enumerate()
                .filter_map(|(i, r)| r.as_ref().map(|r| TreeNode::leaf(kind, i as i32, name(r))))
                .collect()
        }
        roots[0].children = leaves(&p.sprites, ResourceKind::Sprite, |r| r.name.clone());
        roots[1].children = leaves(&p.sounds, ResourceKind::Sound, |r| r.name.clone());
        roots[2].children = leaves(&p.backgrounds, ResourceKind::Background, |r| r.name.clone());
        roots[3].children = leaves(&p.paths, ResourceKind::Path, |r| r.name.clone());
        roots[4].children = leaves(&p.scripts, ResourceKind::Script, |r| r.name.clone());
        roots[5].children = leaves(&p.fonts, ResourceKind::Font, |r| r.name.clone());
        roots[6].children = leaves(&p.timelines, ResourceKind::Timeline, |r| r.name.clone());
        roots[7].children = leaves(&p.objects, ResourceKind::Object, |r| r.name.clone());
        roots[8].children = leaves(&p.rooms, ResourceKind::Room, |r| r.name.clone());
        p.resource_tree = roots;
        self.p
    }
}

/// A minimal 16x16 32-bit Windows icon (blue square with a white frame).
pub fn make_ico() -> Vec<u8> {
    let (w, h) = (16u32, 16u32);
    let xor_len = w * h * 4;
    let and_len = 4 * h; // 16 bits padded to 32 per row
    let img_len = 40 + xor_len + and_len;
    let mut v = Vec::new();
    v.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    v.extend_from_slice(&[w as u8, h as u8, 0, 0, 1, 0, 32, 0]);
    v.extend_from_slice(&img_len.to_le_bytes());
    v.extend_from_slice(&22u32.to_le_bytes());
    // BITMAPINFOHEADER
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&(w as i32).to_le_bytes());
    v.extend_from_slice(&((h * 2) as i32).to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&32u16.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&(xor_len + and_len).to_le_bytes());
    v.extend_from_slice(&[0; 16]);
    for y in 0..h {
        for x in 0..w {
            let edge = x == 0 || y == 0 || x == w - 1 || y == h - 1;
            if edge {
                v.extend_from_slice(&[255, 255, 255, 255]);
            } else {
                v.extend_from_slice(&[200, 80, 20, 255]); // BGRA
            }
        }
    }
    v.extend(std::iter::repeat(0u8).take(and_len as usize));
    v
}
