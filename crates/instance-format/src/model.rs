//! Format-independent project data model (specs/ide.md §2, specs/file-formats.md §1-§6).
//!
//! The model mirrors the v800 file layout closely so that every field survives a
//! read → write → read round trip. Per-record *format* version numbers are not stored:
//! writers emit the canonical numbers of the target format.
//!
//! Conventions:
//! * Resource lists are `Vec<Option<T>>`; the index is the resource id and `None` is a
//!   deleted slot. References between resources are `i32` slot indices, `-1` = none.
//! * Timestamps are Delphi `TDateTime` values ([`Timestamp`]).
//! * Colours are raw `u32` in `0x00BBGGRR` order; a non-zero high byte denotes a
//!   Windows system colour (spec §0.1).
//! * Booleans stored as `i32` in the file are `bool` here (any non-zero reads as true).

use serde::{Deserialize, Serialize};

/// Delphi `TDateTime`: days since 1899-12-30, fractional part = time of day.
pub type Timestamp = f64;

/// `parent` value of an object without parent.
pub const NO_PARENT: i32 = -100;
/// `applies_to` value meaning *self*.
pub const APPLIES_TO_SELF: i32 = -1;
/// `applies_to` value meaning *other*.
pub const APPLIES_TO_OTHER: i32 = -2;
/// First instance id handed out in rooms (spec §5.10).
pub const FIRST_INSTANCE_ID: i32 = 100000;
/// First tile id handed out in rooms (spec §5.10, [I]).
pub const FIRST_TILE_ID: i32 = 10000000;

/// RGBA8 image, rows top-down, `data.len() == width * height * 4`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl RgbaImage {
    /// A fully transparent image of the given size.
    pub fn new(width: u32, height: u32) -> Self {
        RgbaImage {
            width,
            height,
            data: vec![0; width as usize * height as usize * 4],
        }
    }

    /// RGBA of pixel (x, y). Panics when out of range.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        [
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        ]
    }

    /// Whether `data` has the length implied by width and height.
    pub fn is_consistent(&self) -> bool {
        self.data.len() == self.width as usize * self.height as usize * 4
    }
}

/// A complete project (.gmk / .gm81).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// "Unique game id" from the Loading tab.
    pub game_id: i32,
    /// Random project GUID (opaque).
    pub guid: [u8; 16],
    pub settings: Settings,
    /// Trigger slots; the slot index is the event number of trigger events (type 11).
    pub triggers: Vec<Option<Trigger>>,
    pub triggers_last_changed: Timestamp,
    /// User-defined constants in declaration order.
    pub constants: Vec<Constant>,
    pub constants_last_changed: Timestamp,
    pub sounds: Vec<Option<Sound>>,
    pub sprites: Vec<Option<Sprite>>,
    pub backgrounds: Vec<Option<Background>>,
    pub paths: Vec<Option<Path>>,
    pub scripts: Vec<Option<Script>>,
    pub fonts: Vec<Option<Font>>,
    pub timelines: Vec<Option<Timeline>>,
    pub objects: Vec<Option<Object>>,
    pub rooms: Vec<Option<Room>>,
    /// Last instance id allocated in the room editor.
    pub last_instance_id: i32,
    /// Last tile id allocated in the room editor.
    pub last_tile_id: i32,
    pub included_files: Vec<IncludedFile>,
    /// Names of the extension packages used by the game.
    pub extension_packages: Vec<String>,
    pub game_info: GameInfo,
    /// Initialisation code of each action library loaded when the file was saved.
    pub library_init_code: Vec<String>,
    /// Room indices in play order; the first is the start room.
    pub room_order: Vec<i32>,
    /// The 12 root nodes of the resource tree (see [`TreeNode::standard_roots`]).
    pub resource_tree: Vec<TreeNode>,
}

impl Default for Project {
    /// An empty project as the IDE would create it.
    fn default() -> Self {
        Project {
            game_id: 0,
            guid: [0; 16],
            settings: Settings::default(),
            triggers: Vec::new(),
            triggers_last_changed: 0.0,
            constants: Vec::new(),
            constants_last_changed: 0.0,
            sounds: Vec::new(),
            sprites: Vec::new(),
            backgrounds: Vec::new(),
            paths: Vec::new(),
            scripts: Vec::new(),
            fonts: Vec::new(),
            timelines: Vec::new(),
            objects: Vec::new(),
            rooms: Vec::new(),
            last_instance_id: FIRST_INSTANCE_ID,
            last_tile_id: FIRST_TILE_ID,
            included_files: Vec::new(),
            extension_packages: Vec::new(),
            game_info: GameInfo::default(),
            library_init_code: Vec::new(),
            room_order: Vec::new(),
            resource_tree: TreeNode::standard_roots(),
        }
    }
}

/// Global Game Settings (spec §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub start_fullscreen: bool,
    pub interpolate_pixels: bool,
    pub dont_draw_border: bool,
    pub display_cursor: bool,
    /// `>0` fixed scale in percent, `-1` keep aspect ratio, `0` full scale ([I]).
    pub scaling: i32,
    pub allow_window_resize: bool,
    pub always_on_top: bool,
    pub color_outside_room: u32,
    pub set_resolution: bool,
    /// 0 no change, 1 16-bit, 2 32-bit ([I]).
    pub color_depth: i32,
    pub resolution: i32,
    pub frequency: i32,
    pub dont_show_buttons: bool,
    /// Stored as a full `i32` (may become a flag word in v810, spec §7).
    pub vsync: i32,
    /// Not present in v701 files (reads as `true` there).
    pub disable_screensaver: bool,
    pub let_f4_fullscreen: bool,
    pub let_f1_game_info: bool,
    pub let_esc_end_game: bool,
    pub let_f5_save_f6_load: bool,
    pub let_f9_screenshot: bool,
    pub treat_close_as_esc: bool,
    /// 0 normal, 1 high, 2 highest ([I]).
    pub process_priority: i32,
    pub freeze_on_lose_focus: bool,
    /// 0 none, 1 default, 2 custom.
    pub loading_bar: i32,
    /// Custom loading bar back image, opaque bytes; only stored when `loading_bar == 2` ([I]).
    pub loading_bar_back: Option<Vec<u8>>,
    /// Custom loading bar front image, opaque bytes; only stored when `loading_bar == 2` ([I]).
    pub loading_bar_front: Option<Vec<u8>>,
    pub show_custom_load_image: bool,
    /// Custom loading image, opaque bytes; only stored when `show_custom_load_image` ([I]).
    pub custom_load_image: Option<Vec<u8>>,
    pub load_image_transparent: bool,
    /// 0..255.
    pub load_image_alpha: i32,
    pub scale_progress_bar: bool,
    /// Complete Windows `.ico` file.
    pub icon: Vec<u8>,
    pub display_errors: bool,
    pub write_errors_to_log: bool,
    pub abort_on_all_errors: bool,
    /// Bit 0 of the stored "uninitialized" flags word (spec §2 addendum).
    pub treat_uninitialized_as_zero: bool,
    /// Bit 1 of that flags word: GM 8.1's "throw an error when arguments are not
    /// initialized correctly" option (VERIFIED black-box, bb_lang vs bb_args0: set →
    /// script argument counts are checked; cleared → any count, missing arguments
    /// read as 0). Stored by the 8.1 IDE in most projects.
    #[serde(default)]
    pub argument_errors: bool,
    /// The remaining bits (2 and up) of that flags word, preserved verbatim. 0 when
    /// absent.
    #[serde(default)]
    pub uninitialized_extra_bits: i32,
    pub author: String,
    /// Free-text version string.
    pub version_text: String,
    pub info_last_changed: Timestamp,
    pub information: String,
    pub version_major: i32,
    pub version_minor: i32,
    pub version_release: i32,
    pub version_build: i32,
    pub company: String,
    pub product: String,
    pub copyright: String,
    pub description: String,
    /// Not present in v701 files (reads as 0).
    pub last_changed: Timestamp,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            start_fullscreen: false,
            interpolate_pixels: false,
            dont_draw_border: false,
            display_cursor: true,
            scaling: -1,
            allow_window_resize: false,
            always_on_top: false,
            color_outside_room: 0,
            set_resolution: false,
            color_depth: 0,
            resolution: 0,
            frequency: 0,
            dont_show_buttons: false,
            vsync: 0,
            disable_screensaver: true,
            let_f4_fullscreen: true,
            let_f1_game_info: true,
            let_esc_end_game: true,
            let_f5_save_f6_load: true,
            let_f9_screenshot: true,
            treat_close_as_esc: true,
            process_priority: 0,
            freeze_on_lose_focus: false,
            loading_bar: 1,
            loading_bar_back: None,
            loading_bar_front: None,
            show_custom_load_image: false,
            custom_load_image: None,
            load_image_transparent: false,
            load_image_alpha: 255,
            scale_progress_bar: true,
            icon: Vec::new(),
            display_errors: true,
            write_errors_to_log: false,
            abort_on_all_errors: false,
            treat_uninitialized_as_zero: false,
            argument_errors: false,
            uninitialized_extra_bits: 0,
            author: String::new(),
            version_text: "100".into(),
            info_last_changed: 0.0,
            information: String::new(),
            version_major: 1,
            version_minor: 0,
            version_release: 0,
            version_build: 0,
            company: String::new(),
            product: String::new(),
            copyright: String::new(),
            description: String::new(),
            last_changed: 0.0,
        }
    }
}

/// A trigger definition (spec §3.1, [I]).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Trigger {
    pub name: String,
    /// GML expression that must evaluate to a boolean.
    pub condition: String,
    /// 0 middle/normal step, 1 begin step, 2 end step.
    pub moment: i32,
    /// Name of the constant chosen by the user (may be empty).
    pub constant_name: String,
}

/// A user-defined constant (spec §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    /// GML expression text.
    pub value: String,
}

/// Sound resource (spec §5.1).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Sound {
    pub name: String,
    pub last_changed: Timestamp,
    /// 0 normal, 1 background music, 2 3D, 3 multimedia player.
    pub kind: i32,
    /// Extension including the dot, e.g. `.wav`.
    pub file_type: String,
    pub file_name: String,
    /// Raw (uncompressed) file bytes, if any.
    pub data: Option<Vec<u8>>,
    /// Bit flags: 1 chorus, 2 echo, 4 flanger, 8 gargle, 16 reverb ([I] order).
    pub effects: i32,
    pub volume: f64,
    pub pan: f64,
    pub preload: bool,
}

/// Sprite resource (spec §5.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sprite {
    pub name: String,
    pub last_changed: Timestamp,
    pub origin_x: i32,
    pub origin_y: i32,
    pub frames: Vec<RgbaImage>,
    /// 0 precise, 1 rectangle, 2 disk, 3 diamond. Stored even when there are no frames.
    pub collision_shape: i32,
    pub alpha_tolerance: i32,
    pub separate_collision_masks: bool,
    /// 0 automatic, 1 full image, 2 manual.
    pub bbox_mode: i32,
    pub bbox_left: i32,
    pub bbox_right: i32,
    pub bbox_bottom: i32,
    pub bbox_top: i32,
}

impl Default for Sprite {
    fn default() -> Self {
        Sprite {
            name: String::new(),
            last_changed: 0.0,
            origin_x: 0,
            origin_y: 0,
            frames: Vec::new(),
            collision_shape: 1,
            alpha_tolerance: 0,
            separate_collision_masks: false,
            bbox_mode: 0,
            bbox_left: 0,
            bbox_right: 0,
            bbox_bottom: 0,
            bbox_top: 0,
        }
    }
}

impl Sprite {
    /// Width of the sprite (of its first frame), 0 without frames.
    pub fn width(&self) -> u32 {
        self.frames.first().map_or(0, |f| f.width)
    }
    /// Height of the sprite (of its first frame), 0 without frames.
    pub fn height(&self) -> u32 {
        self.frames.first().map_or(0, |f| f.height)
    }
}

/// Background resource (spec §5.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Background {
    pub name: String,
    pub last_changed: Timestamp,
    pub use_as_tileset: bool,
    pub tile_width: i32,
    pub tile_height: i32,
    pub tile_h_offset: i32,
    pub tile_v_offset: i32,
    pub tile_h_sep: i32,
    pub tile_v_sep: i32,
    pub image: RgbaImage,
}

impl Default for Background {
    fn default() -> Self {
        Background {
            name: String::new(),
            last_changed: 0.0,
            use_as_tileset: false,
            tile_width: 16,
            tile_height: 16,
            tile_h_offset: 0,
            tile_v_offset: 0,
            tile_h_sep: 0,
            tile_v_sep: 0,
            image: RgbaImage::default(),
        }
    }
}

/// One path point.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct PathPoint {
    pub x: f64,
    pub y: f64,
    /// Speed in percent (100 = normal).
    pub speed: f64,
}

/// Path resource (spec §5.4, [I]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path {
    pub name: String,
    pub last_changed: Timestamp,
    /// 0 straight lines, 1 smooth curve.
    pub connection_kind: i32,
    pub closed: bool,
    /// 1..8.
    pub precision: i32,
    /// Room shown behind the path in the editor, -1 none.
    pub backdrop_room: i32,
    pub snap_x: i32,
    pub snap_y: i32,
    pub points: Vec<PathPoint>,
}

impl Default for Path {
    fn default() -> Self {
        Path {
            name: String::new(),
            last_changed: 0.0,
            connection_kind: 0,
            closed: true,
            precision: 4,
            backdrop_room: -1,
            snap_x: 16,
            snap_y: 16,
            points: Vec::new(),
        }
    }
}

/// Script resource (spec §5.5).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Script {
    pub name: String,
    pub last_changed: Timestamp,
    pub code: String,
}

/// Font resource (spec §5.6).
///
/// The file's `range_start` word is split: low 16 bits = [`Font::range_start`], bits
/// 16..23 = [`Font::charset`], bits 24..31 = [`Font::antialias`] ([I]); writers
/// recombine them, so the raw value always round-trips.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Font {
    pub name: String,
    pub last_changed: Timestamp,
    pub font_name: String,
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
    pub range_start: u16,
    pub range_end: i32,
    pub charset: u8,
    pub antialias: u8,
}

impl Default for Font {
    fn default() -> Self {
        Font {
            name: String::new(),
            last_changed: 0.0,
            font_name: "Arial".into(),
            size: 12,
            bold: false,
            italic: false,
            range_start: 32,
            range_end: 127,
            charset: 0,
            antialias: 0,
        }
    }
}

impl Font {
    /// The raw `range_start` word as stored in the file.
    pub fn packed_range_start(&self) -> i32 {
        (self.range_start as u32 | (self.charset as u32) << 16 | (self.antialias as u32) << 24)
            as i32
    }

    /// Set the three fields from a raw `range_start` word.
    pub fn set_packed_range_start(&mut self, raw: i32) {
        let raw = raw as u32;
        self.range_start = (raw & 0xFFFF) as u16;
        self.charset = (raw >> 16) as u8;
        self.antialias = (raw >> 24) as u8;
    }
}

/// One moment of a time line.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Moment {
    pub step: i32,
    pub actions: Vec<Action>,
}

/// Time line resource (spec §5.7, [I]).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Timeline {
    pub name: String,
    pub last_changed: Timestamp,
    pub moments: Vec<Moment>,
}

/// Event type numbers (spec §5.8).
pub mod event_type {
    pub const CREATE: i32 = 0;
    pub const DESTROY: i32 = 1;
    pub const ALARM: i32 = 2;
    pub const STEP: i32 = 3;
    pub const COLLISION: i32 = 4;
    pub const KEYBOARD: i32 = 5;
    pub const MOUSE: i32 = 6;
    pub const OTHER: i32 = 7;
    pub const DRAW: i32 = 8;
    pub const KEY_PRESS: i32 = 9;
    pub const KEY_RELEASE: i32 = 10;
    pub const TRIGGER: i32 = 11;
    /// Highest event type of the v800 format.
    pub const MAX: i32 = 11;
}

/// An event of an object together with its actions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Event {
    /// Event type (see [`event_type`]).
    pub event_type: i32,
    /// Sub-event number (alarm index, key code, collision object, trigger index, ...).
    pub number: i32,
    pub actions: Vec<Action>,
}

/// Object resource (spec §5.8).
///
/// `events` keeps file order: grouped by ascending `event_type`, and within a type in
/// the stored order. Writers emit events grouped by type (stable within a type).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub name: String,
    pub last_changed: Timestamp,
    pub sprite: i32,
    pub solid: bool,
    pub visible: bool,
    pub depth: i32,
    pub persistent: bool,
    /// Parent object index, [`NO_PARENT`] (-100) for none.
    pub parent: i32,
    /// Mask sprite, -1 = same as sprite.
    pub mask_sprite: i32,
    pub events: Vec<Event>,
}

impl Default for Object {
    fn default() -> Self {
        Object {
            name: String::new(),
            last_changed: 0.0,
            sprite: -1,
            solid: false,
            visible: true,
            depth: 0,
            persistent: false,
            parent: NO_PARENT,
            mask_sprite: -1,
            events: Vec::new(),
        }
    }
}

impl Object {
    /// The event `(event_type, number)` if the object defines it.
    pub fn event(&self, event_type: i32, number: i32) -> Option<&Event> {
        self.events
            .iter()
            .find(|e| e.event_type == event_type && e.number == number)
    }
}

/// Action kinds (spec §5.9 / §10.2).
pub mod action_kind {
    pub const NORMAL: i32 = 0;
    pub const BEGIN_GROUP: i32 = 1;
    pub const END_GROUP: i32 = 2;
    pub const ELSE: i32 = 3;
    pub const EXIT: i32 = 4;
    pub const REPEAT: i32 = 5;
    pub const VARIABLE: i32 = 6;
    pub const CODE: i32 = 7;
    /// Palette-only kinds (never in games).
    pub const PLACEHOLDER: i32 = 8;
    pub const SEPARATOR: i32 = 9;
    pub const LABEL: i32 = 10;
}

/// Action execution types.
pub mod exe_type {
    pub const NONE: i32 = 0;
    pub const FUNCTION: i32 = 1;
    pub const CODE: i32 = 2;
}

/// Action argument kinds (spec §10.2).
pub mod arg_kind {
    pub const EXPRESSION: i32 = 0;
    pub const STRING: i32 = 1;
    pub const BOTH: i32 = 2;
    pub const BOOLEAN: i32 = 3;
    pub const MENU: i32 = 4;
    pub const SPRITE: i32 = 5;
    pub const SOUND: i32 = 6;
    pub const BACKGROUND: i32 = 7;
    pub const PATH: i32 = 8;
    pub const SCRIPT: i32 = 9;
    pub const OBJECT: i32 = 10;
    pub const ROOM: i32 = 11;
    pub const FONT: i32 = 12;
    pub const COLOR: i32 = 13;
    pub const TIMELINE: i32 = 14;
    pub const FONT_STRING: i32 = 15;
}

/// An action placed in an event or time-line moment (spec §5.9).
///
/// The library metadata (`kind`, flags, `exe_type`, `function_name`, `code`, `arg_kinds`)
/// is a copy stored in the file so that games run without the `.lib` files.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Action {
    pub library_id: i32,
    pub action_id: i32,
    /// See [`action_kind`].
    pub kind: i32,
    pub may_be_relative: bool,
    pub is_question: bool,
    pub has_applies_to: bool,
    /// See [`exe_type`].
    pub exe_type: i32,
    pub function_name: String,
    pub code: String,
    /// Number of meaningful arguments.
    pub argument_count: i32,
    /// Library argument kinds (8 entries in practice; unused = 0). See [`arg_kind`].
    pub arg_kinds: Vec<i32>,
    /// [`APPLIES_TO_SELF`], [`APPLIES_TO_OTHER`] or an object index.
    pub applies_to: i32,
    pub relative: bool,
    /// Argument values as typed (8 entries in practice; unused = "0").
    pub args: Vec<String>,
    /// NOT flag of question actions.
    pub negate: bool,
}

/// A background layer of a room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomBackground {
    pub visible: bool,
    pub is_foreground: bool,
    pub background: i32,
    pub x: i32,
    pub y: i32,
    pub tile_h: bool,
    pub tile_v: bool,
    pub hspeed: i32,
    pub vspeed: i32,
    pub stretch: bool,
}

impl Default for RoomBackground {
    fn default() -> Self {
        RoomBackground {
            visible: false,
            is_foreground: false,
            background: -1,
            x: 0,
            y: 0,
            tile_h: true,
            tile_v: true,
            hspeed: 0,
            vspeed: 0,
            stretch: false,
        }
    }
}

/// A view of a room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub visible: bool,
    pub view_x: i32,
    pub view_y: i32,
    pub view_w: i32,
    pub view_h: i32,
    pub port_x: i32,
    pub port_y: i32,
    pub port_w: i32,
    pub port_h: i32,
    pub h_border: i32,
    pub v_border: i32,
    pub h_speed: i32,
    pub v_speed: i32,
    /// Object to follow, -1 none.
    pub follow_object: i32,
}

impl Default for View {
    fn default() -> Self {
        View {
            visible: false,
            view_x: 0,
            view_y: 0,
            view_w: 640,
            view_h: 480,
            port_x: 0,
            port_y: 0,
            port_w: 640,
            port_h: 480,
            h_border: 32,
            v_border: 32,
            h_speed: -1,
            v_speed: -1,
            follow_object: -1,
        }
    }
}

/// An object instance placed in a room.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Instance {
    pub x: i32,
    pub y: i32,
    pub object: i32,
    pub id: i32,
    pub creation_code: String,
    pub locked: bool,
}

/// A tile placed in a room ([I] layout).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
    pub background: i32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub width: i32,
    pub height: i32,
    pub depth: i32,
    pub id: i32,
    pub locked: bool,
}

/// Room-editor state saved with a room ([S]: 14 words, meaning inferred).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RoomEditorState {
    pub remember_room_editor_info: i32,
    pub editor_width: i32,
    pub editor_height: i32,
    pub show_grid: i32,
    pub show_objects: i32,
    pub show_tiles: i32,
    pub show_backgrounds: i32,
    pub show_foregrounds: i32,
    pub show_views: i32,
    pub delete_underlying_objects: i32,
    pub delete_underlying_tiles: i32,
    pub current_tab: i32,
    pub h_scroll: i32,
    pub v_scroll: i32,
}

/// Room resource (spec §5.10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub last_changed: Timestamp,
    pub caption: String,
    pub width: i32,
    pub height: i32,
    pub snap_x: i32,
    pub snap_y: i32,
    pub isometric_grid: bool,
    /// Steps per second.
    pub speed: i32,
    pub persistent: bool,
    pub background_color: u32,
    pub draw_background_color: bool,
    pub creation_code: String,
    /// 8 layers in practice.
    pub backgrounds: Vec<RoomBackground>,
    pub views_enabled: bool,
    /// 8 views in practice.
    pub views: Vec<View>,
    pub instances: Vec<Instance>,
    pub tiles: Vec<Tile>,
    pub editor: RoomEditorState,
}

impl Default for Room {
    fn default() -> Self {
        Room {
            name: String::new(),
            last_changed: 0.0,
            caption: String::new(),
            width: 640,
            height: 480,
            snap_x: 16,
            snap_y: 16,
            isometric_grid: false,
            speed: 30,
            persistent: false,
            background_color: 0xC0C0C0,
            draw_background_color: true,
            creation_code: String::new(),
            backgrounds: vec![RoomBackground::default(); 8],
            views_enabled: false,
            views: vec![View::default(); 8],
            instances: Vec::new(),
            tiles: Vec::new(),
            editor: RoomEditorState::default(),
        }
    }
}

/// An included file (spec §6.1, [I]).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IncludedFile {
    /// Not present in v701 files (reads as 0).
    pub last_changed: Timestamp,
    pub file_name: String,
    pub original_path: String,
    pub original_file_chosen: bool,
    pub original_size: i32,
    /// File contents when "store in editable" is set.
    pub data: Option<Vec<u8>>,
    /// 0 don't export, 1 temp dir, 2 game dir, 3 custom folder.
    pub export_kind: i32,
    pub export_folder: String,
    pub overwrite_existing: bool,
    pub free_memory_after_export: bool,
    pub remove_at_game_end: bool,
}

/// Game information window (spec §6.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameInfo {
    pub background_color: u32,
    pub show_in_separate_window: bool,
    pub window_caption: String,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub show_window_border: bool,
    pub allow_resize: bool,
    pub stay_on_top: bool,
    pub pause_game: bool,
    /// Not present in v701 files (reads as 0).
    pub last_changed: Timestamp,
    /// RTF document.
    pub rtf: String,
}

impl Default for GameInfo {
    fn default() -> Self {
        GameInfo {
            background_color: 0xFF000018,
            show_in_separate_window: false,
            window_caption: "Game Information".into(),
            left: -1,
            top: -1,
            width: 600,
            height: 400,
            show_window_border: true,
            allow_resize: true,
            stay_on_top: false,
            pause_game: true,
            last_changed: 0.0,
            rtf: String::new(),
        }
    }
}

/// Resource kinds as used by the resource tree (spec §6.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceKind {
    Object,
    Sprite,
    Sound,
    Room,
    Background,
    Script,
    Path,
    Font,
    GameInfo,
    Settings,
    Timeline,
    Extensions,
    /// A value not defined by the spec (kept for round-tripping).
    Unknown(i32),
}

impl ResourceKind {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Object,
            2 => Self::Sprite,
            3 => Self::Sound,
            4 => Self::Room,
            6 => Self::Background,
            7 => Self::Script,
            8 => Self::Path,
            9 => Self::Font,
            10 => Self::GameInfo,
            11 => Self::Settings,
            12 => Self::Timeline,
            13 => Self::Extensions,
            v => Self::Unknown(v),
        }
    }

    pub fn to_i32(self) -> i32 {
        match self {
            Self::Object => 1,
            Self::Sprite => 2,
            Self::Sound => 3,
            Self::Room => 4,
            Self::Background => 6,
            Self::Script => 7,
            Self::Path => 8,
            Self::Font => 9,
            Self::GameInfo => 10,
            Self::Settings => 11,
            Self::Timeline => 12,
            Self::Extensions => 13,
            Self::Unknown(v) => v,
        }
    }
}

/// Status of a resource-tree node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Primary (root) node.
    Root,
    /// User-created group.
    Group,
    /// Resource leaf (also used for the three single-item roots).
    Leaf,
    /// A value not defined by the spec (kept for round-tripping).
    Unknown(i32),
}

impl NodeStatus {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Root,
            2 => Self::Group,
            3 => Self::Leaf,
            v => Self::Unknown(v),
        }
    }

    pub fn to_i32(self) -> i32 {
        match self {
            Self::Root => 1,
            Self::Group => 2,
            Self::Leaf => 3,
            Self::Unknown(v) => v,
        }
    }
}

/// A node of the resource tree (spec §6.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeNode {
    pub status: NodeStatus,
    /// Resource kind of the subtree.
    pub kind: ResourceKind,
    /// Resource slot index for leaves; 0 for roots and groups.
    pub index: i32,
    pub name: String,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// The 12 empty root nodes in canonical order.
    pub fn standard_roots() -> Vec<TreeNode> {
        use ResourceKind::*;
        let roots: [(&str, ResourceKind, NodeStatus); 12] = [
            ("Sprites", Sprite, NodeStatus::Root),
            ("Sounds", Sound, NodeStatus::Root),
            ("Backgrounds", Background, NodeStatus::Root),
            ("Paths", Path, NodeStatus::Root),
            ("Scripts", Script, NodeStatus::Root),
            ("Fonts", Font, NodeStatus::Root),
            ("Time Lines", Timeline, NodeStatus::Root),
            ("Objects", Object, NodeStatus::Root),
            ("Rooms", Room, NodeStatus::Root),
            ("Game Information", GameInfo, NodeStatus::Leaf),
            ("Global Game Settings", Settings, NodeStatus::Leaf),
            ("Extension Packages", Extensions, NodeStatus::Leaf),
        ];
        roots
            .iter()
            .map(|&(name, kind, status)| TreeNode {
                status,
                kind,
                index: 0,
                name: name.into(),
                children: Vec::new(),
            })
            .collect()
    }

    /// A leaf for resource `index`.
    pub fn leaf(kind: ResourceKind, index: i32, name: impl Into<String>) -> Self {
        TreeNode {
            status: NodeStatus::Leaf,
            kind,
            index,
            name: name.into(),
            children: Vec::new(),
        }
    }

    /// A user group.
    pub fn group(kind: ResourceKind, name: impl Into<String>, children: Vec<TreeNode>) -> Self {
        TreeNode {
            status: NodeStatus::Group,
            kind,
            index: 0,
            name: name.into(),
            children,
        }
    }

    /// Visit every node depth-first (pre-order).
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a TreeNode, usize)) {
        fn go<'a>(n: &'a TreeNode, depth: usize, f: &mut impl FnMut(&'a TreeNode, usize)) {
            f(n, depth);
            for c in &n.children {
                go(c, depth + 1, f);
            }
        }
        go(self, 0, f);
    }
}

/// Counts of resource slots, for summaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SlotCount {
    /// Number of slots (including deleted ones).
    pub slots: usize,
    /// Number of existing resources.
    pub used: usize,
}

fn slot_count<T>(v: &[Option<T>]) -> SlotCount {
    SlotCount {
        slots: v.len(),
        used: v.iter().filter(|x| x.is_some()).count(),
    }
}

impl Project {
    /// Slot counts of the nine resource lists, in file order
    /// (sounds, sprites, backgrounds, paths, scripts, fonts, timelines, objects, rooms).
    pub fn slot_counts(&self) -> [(&'static str, SlotCount); 9] {
        [
            ("sounds", slot_count(&self.sounds)),
            ("sprites", slot_count(&self.sprites)),
            ("backgrounds", slot_count(&self.backgrounds)),
            ("paths", slot_count(&self.paths)),
            ("scripts", slot_count(&self.scripts)),
            ("fonts", slot_count(&self.fonts)),
            ("timelines", slot_count(&self.timelines)),
            ("objects", slot_count(&self.objects)),
            ("rooms", slot_count(&self.rooms)),
        ]
    }

    /// Names of the slots of the resource list of `kind` (`None` for deleted slots), or
    /// `None` when `kind` has no resource list (game information, settings, ...).
    pub fn slot_names(&self, kind: ResourceKind) -> Option<Vec<Option<&str>>> {
        use ResourceKind as K;
        fn names<T>(v: &[Option<T>], name: impl Fn(&T) -> &str) -> Vec<Option<&str>> {
            v.iter().map(|x| x.as_ref().map(&name)).collect()
        }
        Some(match kind {
            K::Sprite => names(&self.sprites, |x| &x.name),
            K::Sound => names(&self.sounds, |x| &x.name),
            K::Background => names(&self.backgrounds, |x| &x.name),
            K::Path => names(&self.paths, |x| &x.name),
            K::Script => names(&self.scripts, |x| &x.name),
            K::Font => names(&self.fonts, |x| &x.name),
            K::Timeline => names(&self.timelines, |x| &x.name),
            K::Object => names(&self.objects, |x| &x.name),
            K::Room => names(&self.rooms, |x| &x.name),
            _ => return None,
        })
    }

    /// Resources that have no leaf under their kind's root in the resource tree, as
    /// (kind, slot, name). The IDE shows only what is in the tree, so these would be in
    /// the game but invisible in the IDE.
    pub fn missing_from_tree(&self) -> Vec<(ResourceKind, i32, String)> {
        let mut out = Vec::new();
        for root in TreeNode::standard_roots() {
            let Some(names) = self.slot_names(root.kind) else {
                continue;
            };
            let mut listed = vec![false; names.len()];
            for r in self.resource_tree.iter().filter(|r| r.kind == root.kind) {
                r.walk(&mut |n, _| {
                    if n.status == NodeStatus::Leaf && n.kind == root.kind {
                        if let Some(l) = listed.get_mut(n.index as usize) {
                            *l = true;
                        }
                    }
                });
            }
            for (i, name) in names.iter().enumerate() {
                if let (Some(name), false) = (name, listed[i]) {
                    out.push((root.kind, i as i32, name.to_string()));
                }
            }
        }
        out
    }
}
