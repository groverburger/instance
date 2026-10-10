# Agent guide: GameMaker 8.1 projects and games

This guide is for an AI agent asked to **read or write GameMaker 8.x project files**
(`.gmk` / `.gm81`) or to **make a game that runs in GameMaker 8.1** (and in Instance,
this repository's clean-room reimplementation). It is a condensed, practical digest of
the specs in `specs/`, which stay the authority. Every rule marked **VERIFIED** was
observed in the real GameMaker 8.1 (Lite edition) with the black-box projects of
`crates/instance-blackbox`; everything else comes from the specs' reading of the
manual or from file samples, and uncertain points say so.

Conventions: `GM` = GameMaker 8.1; "Instance" = this project's runner/IDE;
paths are relative to the repository root.

## Contents

1. [Top 20 gotchas](#1-top-20-gotchas)
2. [GameMaker 8.1 in brief: the project model](#2-gamemaker-81-in-brief-the-project-model)
3. [File format reference](#3-file-format-reference)
4. [Producing a project programmatically](#4-producing-a-project-programmatically)
5. [Validating a project](#5-validating-a-project)
6. [Writing GML that works in GM 8.1](#6-writing-gml-that-works-in-gm-81)
7. [Runtime rules that matter to game authors](#7-runtime-rules-that-matter-to-game-authors)
8. [Lite vs Standard edition restrictions](#8-lite-vs-standard-edition-restrictions)
9. [Game design tips for GM 8.1](#9-game-design-tips-for-gm-81)
10. [Testing headless with scripted input](#10-testing-headless-with-scripted-input)
11. [Converting GameMaker: Studio projects](#11-converting-gamemaker-studio-projects)
12. [Where everything lives](#12-where-everything-lives)

---

## 1. Top 20 gotchas

Read these first.

| # | Rule | Details |
|---|---|---|
| 1 | Code is stored with **CR LF** line endings. | LF-only code compiles, but GM's error messages show it run together on one line. §3.7 |
| 2 | A code piece whose **first token is `{`** must be exactly one block. | Anything after the matching `}` (even `;`) is the compile error "Program ends before end of the code." §6.1 |
| 3 | **Constant folding errors are compile errors**: `'a' + 1`, `1 / 0`. | A compile error in a script stops the game at startup; in a code action it is fatal when the action first runs. §6.1 |
| 4 | **No `++`, `--`, `?:`, `%`, `%=`, `<<=`, `>>=`, `1e3`, escapes in strings, array literals, `[\| ]` accessors.** | `i++;` is "Assignment operator expected.". Use `i += 1`, `mod`, `$FF`, `chr(34)`. §6.1 |
| 5 | **No short-circuit evaluation.** | `if (instance_exists(o) && o.hp > 0)` still evaluates `o.hp` and fails when `o` is gone. Nest the `if`s. §6.3 |
| 6 | `var` cannot use a built-in variable name (`x`, `y`, `speed`, …). | "Cannot redeclare a builtin variable." §6.4 |
| 7 | Reading a never-assigned variable is a runtime error (unless the setting "treat uninitialized variables as 0" is on). | Initialise everything in Create. §6.4 |
| 8 | `var` locals live for **one code action** only. | The next action of the same event cannot see them. §6.4 |
| 9 | A name after `.` must be a variable. | `global.score_obj` where an object `score_obj` exists fails: "Variable name expected." Prefix resource names (`obj_`, `spr_`, …). §6.5 |
| 10 | A script named like a built-in function **replaces** it. | Naming a script `abs` makes every `abs()` call your script. §6.5 |
| 11 | Script argument counts are checked when settings bit 1 is set. | Calling a script with a different count than `1 + highest argumentN used` is an error. §6.6 |
| 12 | Truthiness: a real is true iff **≥ 0.5**; a string condition is a runtime error. | `if (0.4)` is false; `if ('a')` errors "Expression expected". §6.3 |
| 13 | Real equality has an absolute tolerance: `\|a-b\| < 1e-13`. | `sin(pi) == 0` is true. §6.3 |
| 14 | `string(x)` prints non-integers with exactly 2 decimals. | `string(0.125)` = `"0.13"`; `string(1/3)` = `"0.33"`. §6.7 |
| 15 | `room_goto()` lets the current action finish, then **skips the rest of the event and the rest of the step**. | Code after `room_goto` in the same action still runs. §7.3 |
| 16 | Step/Begin/End Step run **object-major** (by object index), not by instance creation order. | Order objects in the file accordingly. §7.1 |
| 17 | Alarms only count down when the object has (or inherits) that alarm event; `alarm[i] = 0` never fires. | §7.2 |
| 18 | Collisions do nothing unless an object has a collision event; with a handler, if either instance is **solid** they are put back to their previous positions. | §7.4 |
| 19 | There is **no Draw GUI event** in 8.1. | Draw the HUD in Draw at `view_xview[view_current] + dx`. §7.6 |
| 20 | Lite edition: no data structures (`ds_*`), surfaces, particles, blend modes, 3D, DLLs, rotated/blended sprite drawing, … | If the game must run in GM 8.1 Lite, avoid them. Instance has no editions. §8 |

---

## 2. GameMaker 8.1 in brief: the project model

A GM 8.1 game is one project file holding everything:

| Resource | What it is | Notes |
|---|---|---|
| Sprite | animated image (frames), origin, collision mask settings | used by objects for drawing and collisions |
| Sound | embedded sound file bytes + kind (normal / background music / 3D / multimedia) | WAV, MIDI, MP3 |
| Background | one image; may be used as a tile set | rooms show up to 8 background/foreground layers |
| Path | point list for `path_start` | straight or smooth, open or closed |
| Script | named GML function | called like a function, 0–16 arguments |
| Font | system font name + size + character range | `draw_set_font` |
| Time line | list of (step, actions) moments | attached to instances |
| Object | sprite, solid, visible, depth, persistent, parent, mask + **events → action lists** | the behaviour |
| Room | size, speed, background colour, creation code, 8 backgrounds, 8 views, **placed instances**, tiles | the levels |
| Also | Global Game Settings, triggers, constants, included files, extension package names, game information (RTF), room order, resource tree | |

Key facts:

* **Resource ids are slot indices** in each list (0, 1, 2, …), *including deleted
  slots*. All references (object → sprite, action argument → object, …) are these
  indices; `-1` = none. In GML a resource name is a constant holding its index.
* **Objects** have events (Create, Destroy, Alarm 0–11, Begin/Step/End Step,
  Collision with object N, Keyboard/Key Press/Key Release for key K, Mouse, Other
  (room start, user events, …), Draw, Trigger). Each event holds an **ordered action
  list**. An action is a drag-and-drop action from a library (move, create instance,
  …) or **Execute Code** (library 1, action 603) holding GML. A game written by an
  agent should use Execute Code actions almost exclusively.
* **Instances** are runtime copies of objects. Instances placed in a room have fixed
  ids (> 100000) stored in the room; runtime-created ones get new ids.
* **Parents**: an object inherits every event it does not define from its parent
  chain; `with (parent)`, collision events and `instance_number(parent)` include
  descendants.
* **Rooms** are played in the **room order** list; its first entry is the start room.
* The **resource tree** (folders in the IDE) is purely cosmetic but must be present and
  have 12 fixed roots.

---

## 3. File format reference

Full byte-level spec: **`specs/file-formats.md`** (with confidence markers [V]/[S]/[I]);
reference decoder `specs/tools/gmk_dump.py`; Rust implementation
`crates/instance-format/src/gmk.rs`. This section is enough to implement a v800/v810
reader/writer; consult the spec for the rare fields.

### 3.1 Primitives (all little-endian)

| Type | Encoding |
|---|---|
| `i32` / `u32` | 4 bytes |
| `bool` | `i32`, 0/1 (read any non-zero as true — except the settings flag word, §3.4) |
| `f64` | IEEE double |
| `str` | `u32 len` + bytes, no terminator, **Windows-1252** |
| `blob` | `u32 len` + raw bytes |
| `zblock` | `u32 len` + a **zlib** stream (RFC 1950). The inflated bytes are parsed with the same primitives and must be consumed exactly. |
| `time` | `f64` Delphi `TDateTime` (days since 1899-12-30) |
| `color` | `i32` `0x00BBGGRR` (red in the low byte). High byte ≠ 0 = Windows system colour. |
| `img800` | `i32 800, i32 w, i32 h`, then if `w*h > 0`: `blob` of `w*h*4` bytes, rows top-down, **B, G, R, A** order |

### 3.2 Container (v800 `.gmk` / v810 `.gm81`)

```
i32 magic = 1234321                 (0x0012D591)
i32 version = 800 (.gmk GM 8.0) | 810 (.gm81 GM 8.1)   — the only difference between them
i32 game_id
u8  guid[16]
i32 800, zblock settings                                  §3.4
i32 800, i32 n, n × zblock trigger, time                  §3.5
i32 800, i32 n, n × {str name; str value}, time           constants (value = GML expression)
9 resource lists, in this order:
  sounds, sprites, backgrounds, paths, scripts, fonts, timelines, objects, rooms
i32 last_instance_id                (highest id handed out by the room editor)
i32 last_tile_id
included files, extension packages, game information,
library creation code, room order, resource tree          §3.8
EOF
```

`.gm81` (810) was verified on many GM 8.1-saved files: every record below the header
uses the 800 layout unchanged, settings version stays 800.

### 3.3 Resource lists

```
i32 list_version (800)
i32 slot_count
slot_count × zblock:
    bool exists            — 0 = deleted slot; the block ends here
    str  name
    time last_changed
    i32  record_version    — per type, see table
    ...body
```

| Type | record ver. | Body (v800) |
|---|---|---|
| Sound | 800 | `i32 kind` (0 normal, 1 background music, 2 3D, 3 multimedia), `str file_type` (".wav"), `str file_name`, `bool has_data`, [`blob data`: raw file bytes, uncompressed], `i32 effects` (bit flags), `f64 volume`, `f64 pan`, `bool preload` |
| Sprite | 800 | `i32 origin_x, origin_y, frame_count`, `img800 × frame_count`, then **always** (also with 0 frames): `i32 collision_shape` (0 precise, 1 rectangle, 2 disk, 3 diamond), `i32 alpha_tolerance`, `bool separate_masks`, `i32 bbox_mode` (0 auto, 1 full, 2 manual), `i32 bbox_left, bbox_right, bbox_bottom, bbox_top` (note the order) |
| Background | **710** | `bool use_as_tileset`, `i32 tile_w, tile_h, h_off, v_off, h_sep, v_sep`, `img800` |
| Path | 530 | `i32 kind` (0 straight, 1 smooth), `bool closed`, `i32 precision`, `i32 backdrop_room`, `i32 snap_x, snap_y`, `i32 n`, `n × {f64 x, y, speed%}` |
| Script | 800 | `str code` |
| Font | 800 | `str font_name`, `i32 size`, `bool bold, italic`, `i32 range_start` (low 16 bits; the high bytes carry extra data — keep them), `i32 range_end` |
| Time line | 500 | `i32 n`, `n × {i32 step; action_list}` |
| Object | **430** | `i32 sprite`, `bool solid, visible`, `i32 depth`, `bool persistent`, `i32 parent` (**-100 = none**), `i32 mask_sprite` (-1 = sprite), `i32 11`, then for event type 0..11: repeated `{i32 number; action_list}` terminated by `i32 -1` |
| Room | **541** | see below |

**Action list** (`i32 400`, `i32 count`, then per action):

```
i32 440                 action version
i32 library_id          1 = built-in libraries
i32 action_id           e.g. 603 Execute Code (see specs/actions.json)
i32 kind                0 normal, 1 begin block, 2 end block, 3 else, 4 exit, 5 repeat, 6 set variable, 7 code
bool may_be_relative, bool is_question, bool has_applies_to
i32 exe_type            0 none, 1 call function, 2 code
str function_name       e.g. "action_move" (empty for exe_type 0/2)
str code                (empty for built-in actions)
i32 argument_count
i32 8, i32 arg_kind[8]  (unused 0)
i32 applies_to          -1 self, -2 other, ≥0 object index
bool relative
i32 8, str arg[8]       (values as typed; unused slots "0")
bool not                (negates a question action)
```

The library metadata (kind, flags, function name, arg kinds) is a copy taken from
the library; GM runs the action from these copies. Copy them exactly from
`specs/actions.json` (or use `builtin_action(lib, id).instantiate(lib)` in Rust).

Useful built-in actions (library 1):

| id | Action | kind | args |
|---|---|---|---|
| 603 | Execute Code | 7 (code), exe_type 2 | arg0 = GML program |
| 611 | Set Variable | 6, exe_type 2 | arg0 variable name, arg1 value expression; `relative` adds |
| 601 | Execute Script | 0, `action_execute_script` | arg0 script index, arg1..5 arguments |
| 408 | Test Expression (question) | 0, `action_if` | arg0 expression; true when ≥ 0.5 |
| 422 / 424 | Start / End Block | 1 / 2 | – |
| 421 | Else | 3 | – |
| 425 | Exit Event | 4 | – |
| 604 | Call Parent Event | 0, `action_inherited` | – |
| 301 | Set Alarm | 0, `action_set_alarm` | arg0 steps, arg1 alarm number (menu index) |

**Room body:**

```
str caption; i32 width, height, snap_x, snap_y; bool isometric; i32 speed;
bool persistent; color background_color; bool draw_background_color; str creation_code;
i32 8, 8 × { bool visible, foreground; i32 background, x, y; bool tile_h, tile_v; i32 hspeed, vspeed; bool stretch }
bool views_enabled;
i32 8, 8 × { bool visible; i32 view_x, view_y, view_w, view_h, port_x, port_y, port_w, port_h,
             h_border, v_border, h_speed, v_speed, follow_object }
i32 n, n × { i32 x, y, object, id; str creation_code; bool locked }      instances
i32 n, n × { i32 x, y, background, tile_x, tile_y, width, height, depth, id; bool locked }   tiles
14 × i32 editor state (all 0 is fine)
```

Defaults: room 640×480, speed 30, colour `0xC0C0C0`; view defaults
`0,0,0,640,480,0,0,640,480,32,32,-1,-1,-1`. Instance ids start at 100001 and must be
unique across the whole project (`last_instance_id` ≥ the highest); tile ids start at
10000001.

### 3.4 Settings block (inflated, in order)

```
bool start_fullscreen, interpolate_pixels, dont_draw_border, display_cursor
i32  scaling (>0 percent, -1 keep aspect, 0 full)
bool allow_resize, always_on_top; color outside_room; bool set_resolution
i32  color_depth, resolution, frequency; bool dont_show_buttons; i32 vsync
bool disable_screensaver, f4_fullscreen, f1_info, esc_ends, f5_f6_save_load, f9_screenshot, close_as_esc
i32  priority; bool freeze_on_lose_focus
i32  loading_bar (0 none, 1 default, 2 custom; 2 adds image data [I])
bool show_custom_load_image (if set: image data [I]); bool load_image_transparent
i32  load_image_alpha; bool scale_progress_bar
blob icon                  — a complete Windows .ico file
bool display_errors, write_errors_to_log, abort_on_all_errors
i32  error FLAGS WORD      — bit 0: treat uninitialized variables as 0
                             bit 1: "throw an error when arguments aren't initialised correctly"
                             other bits: preserve verbatim
str  author, version_text; time; str information
i32  major, minor, release, build; str company, product, copyright, description
time last_changed
```

Gotchas:

* The flags word is **not a bool**: GM 8.1 typically stores `2` (bit 1 set, bit 0
  clear). A reader that treats any non-zero value as "uninitialized = 0" is wrong.
* Bit 1 (VERIFIED) turns on the exact script argument-count check (§6.6). The GM 8.1
  IDE stored it in 20 of 21 user projects examined; Instance's `Settings::argument_errors`.
* Give the icon a real `.ico` (the IDE's Change Icon requires a 32×32 image; the
  black-box projects store a 16×16 32-bit icon and load fine in GM). Whether an empty
  icon blob loads in GM is unverified — don't rely on it.
* With `display_errors` off, a runtime error is silent in GM (and a startup compile
  error just closes the game).

### 3.5 Triggers

Each trigger is its own `zblock`: `bool exists`, then `i32 800, str name,
str condition, i32 moment (0 step, 1 begin step, 2 end step), str constant_name`.
The slot index is the event number of object event type 11. The **condition is a GML
program, not an expression**: write `return global.phase == 3;` (a bare expression
fails to compile: "Assignment operator expected."; VERIFIED). Triggers written this
way load and run in GM 8.1 (bb_events); the IDE only shows them in advanced mode.

### 3.6 Event numbers

| type | event | number |
|---|---|---|
| 0 | Create | 0 |
| 1 | Destroy | 0 |
| 2 | Alarm | 0..11 |
| 3 | Step | 0 normal, 1 begin, 2 end |
| 4 | Collision | other object's index |
| 5 / 9 / 10 | Keyboard (held) / Key Press / Key Release | virtual-key code (37 left, 38 up, 39 right, 40 down, 32 space, 13 enter, 27 escape, 16 shift, 17 ctrl, `ord('A')`…, 0 no key, 1 any key) |
| 6 | Mouse | 0/1/2 left/right/middle button held, 3 no button, 4/5/6 pressed, 7/8/9 released, 10 enter, 11 leave, 16–19 joystick1 L/R/U/D, 21–28 joy1 buttons, 31–34, 36–43 joystick2, 50–52 global held, 53–55 global pressed, 56–58 global released, 60/61 wheel up/down |
| 7 | Other | 0 outside room, 1 intersect boundary, 2 game start, 3 game end, 4 room start, 5 room end, 6 no more lives, 7 animation end, 8 end of path, 9 no more health, 10–25 user 0–15, 30 close button, 40+n outside view n, 50+n boundary view n |
| 8 | Draw | 0 |
| 11 | Trigger | trigger slot |

(Mouse/other numbers are the spec's values from the manual's constant order; the
constants themselves were checked by `bb_constants`.)

### 3.7 Line endings and text

Code (scripts, code actions, creation code, constants) is stored as Windows-1252
bytes with **CR LF** line endings (VERIFIED: LF-only code compiles but error messages
lose their line structure). Write CR LF.

### 3.8 Trailing sections

```
included files:     i32 800, i32 n, n × zblock { time; i32 800; str file_name, original_path;
                    bool chosen; i32 size; bool store; [blob data]; i32 export_kind (0 none, 1 temp,
                    2 game dir, 3 folder); str folder; bool overwrite, free_after_export, remove_at_end }   [I]
extension packages: i32 700, i32 n, n × str package_name
game information:   i32 800, zblock { color bg (0xFF000018 default); bool separate_window; str caption;
                    i32 left, top, width, height; bool border, resizable, on_top, pause_game; time; str rtf }
library init code:  i32 500, i32 n, n × str                 (n = 0 is fine)
room order:         i32 700, i32 n, n × i32 room_index      (first = start room)
resource tree:      exactly 12 root nodes, each recursively:
                    i32 status (1 root, 2 group, 3 leaf), i32 kind, i32 index, str name, i32 n, n × node
```

Tree roots, in order (name, kind, status): Sprites 2/1, Sounds 3/1, Backgrounds 6/1,
Paths 8/1, Scripts 7/1, Fonts 9/1, Time Lines 12/1, Objects 1/1, Rooms 4/1,
Game Information 10/**3**, Global Game Settings 11/**3**, Extension Packages 13/**3**.
Leaves: status 3, the root's kind, the resource index, the resource name. Deleted
resources have no leaf.

### 3.9 Other versions and files

* **v701 (GM 7 `.gmk`)** is obfuscated: after the header come two junk arrays and a
  seed; a 256-byte substitution table built from the seed is applied with the
  absolute file offset (`plain = (U[byte] - offset) mod 256`; the first encoded byte
  is stored raw). The decoded stream has no zlib blocks, no timestamps, older record
  versions and BMP images (bottom-left pixel = transparent colour). See
  `specs/file-formats.md` §8 and its "701 → 8.1 conversion" addendum. Instance reads
  v701 but only writes v800/v810.
* **`.gmspr`** (exported sprite): `i32 1234321`, `zblock { i32 800, sprite body }`.
* **`.lib`** action libraries and **`.ged`/`.dat`** extension packages: §10–11 of the spec.
* **Instance standalone executables** (`specs/exe-format.md`): runner exe + the
  project as `.gm81` bytes + a 24-byte footer `"INSTNC\0\x01", u64 offset, u64 length`
  (40-byte `\x02` variant when extension packages are bundled). GM's own `.exe`
  format is not supported.

---

## 4. Producing a project programmatically

### 4.1 Recommended route: the `instance-format` crate (Rust)

Public API (`crates/instance-format/src/lib.rs`):

| Item | Use |
|---|---|
| `read_project(&[u8])`, `read_project_file(path) -> (Project, FormatVersion)` | read v701 / v800 / v810 |
| `write_project(&Project, FormatVersion::V810)`, `write_project_file` | write v800 / v810 |
| `detect_version`, `canonical_bytes` | sniff the version; zlib-independent bytes for diffing two files |
| `Project`, `Settings`, `Sprite`, `RgbaImage` (**RGBA**, converted to BGRA on write), `Sound`, `Background`, `Path`, `Script`, `Font`, `Timeline`, `Object`, `Event`, `Action`, `Room`, `Instance`, `View`, `RoomBackground`, `Tile`, `TreeNode`, `ResourceKind` | the model; lists are `Vec<Option<T>>` (`None` = deleted slot) |
| `event_type::*`, `action_kind::*`, `exe_type::*`, `arg_kind::*`, `NO_PARENT` (-100), `APPLIES_TO_SELF/OTHER`, `FIRST_INSTANCE_ID` | constants |
| `builtin_action(lib_id, action_id)` → `ActionDef`; `.instantiate(lib_id)` → `Action` | an action with the right metadata copies |
| `TreeNode::standard_roots()`, `TreeNode::leaf(kind, index, name)` | resource tree |
| `read_sprite_export` / `write_sprite_export`, `read_library` / `write_library` | `.gmspr`, `.lib` |
| `write_folder(&Project, dir)`, `read_folder(dir)`, `read_project_any(path)` | lossless project folder form (below); `read_project_any` takes a file or a folder |

`Project::default()` is an empty IDE-like project (12 tree roots, default settings,
`last_instance_id = 100000`). The model types implement serde `Serialize`/`Deserialize`,
so a few lines of Rust can turn JSON into a `.gm81` if you prefer to generate the
model in another language.

**Complete, compiling example:** `crates/instance-format/examples/make_game.rs`
(sprite + object with Create/Step/Draw code actions + room with one instance → `.gm81`).
The essential parts:

```rust
use instance_format::*;

fn crlf(code: &str) -> String { code.replace("\r\n", "\n").replace('\n', "\r\n") }

fn code_action(code: &str) -> Action {
    let mut a = builtin_action(1, 603).expect("Execute Code").instantiate(1);
    a.args[0] = crlf(code);
    a
}

let mut p = Project { game_id: 424242, guid: *b"agent-guide-demo", ..Project::default() };
p.settings.icon = make_ico();                      // a complete .ico file
p.settings.argument_errors = true;                 // bit 1: exact script argument counts
p.sprites.push(Some(Sprite { name: "spr_player".into(), origin_x: 16, origin_y: 16,
    frames: vec![img], collision_shape: 1, bbox_right: 31, bbox_bottom: 31, ..Sprite::default() }));
p.objects.push(Some(Object { name: "obj_player".into(), sprite: 0, events: vec![
    Event { event_type: event_type::CREATE, number: 0, actions: vec![code_action("steps = 0;")] },
    Event { event_type: event_type::STEP,   number: 0, actions: vec![code_action("steps += 1;")] },
    Event { event_type: event_type::DRAW,   number: 0, actions: vec![code_action(
        "draw_sprite(sprite_index, image_index, x, y);\ndraw_text(8, 8, string(steps));")] },
], ..Object::default() }));
p.last_instance_id += 1;
p.rooms.push(Some(Room { name: "rm_main".into(), width: 320, height: 240, instances: vec![
    Instance { x: 160, y: 120, object: 0, id: p.last_instance_id, creation_code: String::new(), locked: false },
], ..Room::default() }));
p.room_order = vec![0];
p.resource_tree = TreeNode::standard_roots();
p.resource_tree[0].children.push(TreeNode::leaf(ResourceKind::Sprite, 0, "spr_player"));
p.resource_tree[7].children.push(TreeNode::leaf(ResourceKind::Object, 0, "obj_player"));
p.resource_tree[8].children.push(TreeNode::leaf(ResourceKind::Room, 0, "rm_main"));
std::fs::write("game.gm81", write_project(&p, FormatVersion::V810)?)?;
```

Run it:

```sh
cargo run -p instance-format --example make_game -- game.gm81
```

Checklist for a generated project:

* Events of an object grouped by type in ascending order (the writer groups them).
* Every code string CR LF; sprites' `frames` RGBA with `width*height*4` bytes.
* Bbox fields set: store the tight box of opaque pixels for `bbox_mode` 0, as
  `crates/instance-blackbox/src/builder.rs` does (whether GM recomputes it at load is
  not verified, so don't leave it at 0,0,0,0).
* Unique instance ids above 100000; `last_instance_id` ≥ the highest.
* `room_order` lists every room you want reachable via `room_goto_next`; first = start.
* Resource tree with 12 roots and a leaf per resource (the writer requires the 12
  roots; leaves are what the IDE lists — the runner ignores the tree).
* Prefer `show_score = false` in a room creation code or controller Create if you do
  not want "Score: 0" appended to the window caption (`show_score` defaults to 1).

A larger, battle-tested builder (sprites from colours, scripts, paths, timelines,
triggers, constants, rooms, placement, tree) is `crates/instance-blackbox/src/builder.rs`;
the projects it generates were loaded and run in the real GM 8.1.

### 4.2 Project folders: editing a game as plain files

Often the easiest route for an agent is not to build a `Project` in code but to edit a
**project folder**: GML in `.gml` files, images as PNG, sounds as their original files,
metadata in small TOML files, references by name. The conversion is lossless both ways
(`canonical_bytes`-identical; verified on the sample games and a corpus of 35 real
GM 8.1 projects). Full description: `docs/project-folder.md`.

```sh
cargo run -p instance-format --bin gmk-unpack -- game.gm81 game/   # file -> folder
# edit game/objects/obj_player/step.gml, add game/scripts/scr_new.gml, ...
cargo run -p instance-format --bin gmk-pack -- game/ game.gm81     # folder -> file
cargo run -p instance-runner -- game/ --headless --frames 60       # or run the folder directly
```

* One directory per resource kind (`sprites/<name>/0.png`, `objects/<name>/step.gml`,
  `rooms/<name>/room.toml`, `scripts/<name>.gml`, ...). An event holding exactly one
  default Execute Code action is a plain `.gml`; anything else is
  `<event>.actions.toml`.
* Write `.gml` with LF: packing converts to CR LF. Text must be Windows-1252
  representable (packing reports the file and line otherwise).
* A new `.gml` in `scripts/` or a new directory in `objects/` / `sprites/` (no
  metadata file needed) becomes a new resource in the next free slot, with a tree leaf.
* Pack errors name the file and key, e.g. an unknown sprite referenced by an object.
* `gmk-unpack DIR --force` refreshes an existing folder in place (only the resource
  directories are pruned; `.git` etc. are kept).

### 4.3 Other languages

Implement §3 directly (zlib via any library; check exact consumption when reading).
`specs/tools/gmk_dump.py` is a complete Python reader of the same layout: run
`python -I specs/tools/gmk_dump.py game.gm81` on your output — it raises on any
leftover or missing byte, which catches most writer bugs.

---

## 5. Validating a project

| Tool | Command | What it shows |
|---|---|---|
| `gmk-info` | `cargo run -p instance-format --bin gmk-info -- game.gm81 --tree` | version, resources per list with slots, events/actions per object, rooms, room order, tree |
| `gmk_dump.py` | `python -I specs/tools/gmk_dump.py game.gm81` | full JSON dump; strict exact-consumption check |
| `gmk-check` | `cargo run -p instance-check --bin gmk-check -- game.gm81` | static check without running: GML compile errors (GM's messages, with location), script argument counts, broken references, duplicate instance ids, name clashes; warnings for variables never assigned, no-short-circuit hazards, LF-only code |
| `instance-runner` | `instance-runner game.gm81 --headless --frames 120 --screenshot out.png` | runs the game without a window (software renderer) and writes the last frame |
| Instance IDE | `instance-ide game.gm81` | editor; F5 runs |
| Real GM 8.1 | open the `.gm81`, F5 | the reference; the only way to be sure about Lite restrictions |

Run `gmk-check` first on every generated project; it compiles every code piece
(scripts, code actions, Set Variable / expression arguments, time-line moments,
room and instance creation code, trigger conditions, constants) and reports, e.g.:

```
error: obj_player / Step / action 1, line 12 col 2: Assignment operator expected. [compile]
    i++;
     ^
1 error, 0 warnings in 113 code pieces           (the last line is always the summary)
```

Flags: `--pedantic` (also "possibly uninitialised" instance variables: read in an
event but not set in the Create chain), `--lite` (Standard-only functions, variables
and actions, §8), `--strict` (warnings fail too), `--json` (array of `{severity, kind,
resource, event, action, line, col, message, source_line}`), `--quiet` (summary only).
Exit codes: 0 no errors, 1 errors (or warnings with `--strict`), 2 cannot load.
It does not find run-time errors (wrong types, missing instances) — run the game too.

Headless runner output (stderr): a summary line `ran N steps …; room R (name),
K instances, caption "…"`, every runtime error as `ERROR: …` / `FATAL ERROR: …`
(with GM's message layout), `show_message` text as `[message] …`, questions answered
"yes", `get_string` returning its default, and a list of stubbed/unsupported
functions that were called. Exit codes: 0 ok, 1 load/start failure, 2 the game was
aborted by a fatal error, 4 `--max-seconds` timeout. On Windows the runner is a GUI
program: redirect its output (`2>&1 | cat`) or use `Start-Process -Wait` so the shell
waits. Build it with `cargo build -p instance-runner` (`target/debug/instance-runner`).
Set `GM_DUMP_INSTANCES=1` to print every instance (id, object, position, sprite,
depth, …) after the run.

Instance is stricter than "anything goes" but not a perfect GM oracle: it matches every
non-informational key of the black-box suites, but areas without black-box coverage
(see `specs/OPEN_QUESTIONS.md`) may differ from GM. A game that runs in Instance and uses
only Lite features and the rules in §6–§7 is very likely to run in GM 8.1.

---

## 6. Writing GML that works in GM 8.1

Language spec: `specs/gml-language.md`. Its final sections ("VERIFIED black-box")
override earlier text. Function catalogue: `specs/builtins.json` (1555 functions,
variables and constants, with argument lists and read-only flags).

### 6.1 Syntax: what exists and what doesn't

| Exists | Does **not** exist (in 8.1) |
|---|---|
| `{ }` / `begin end`, optional `;`, `if … then … else`, `repeat (n)`, `while`, `do … until`, `for (init; cond; step)`, `switch`/`case`/`default` (fall-through; strings allowed), `break`, `continue`, `exit`, `return expr`, `with (expr)`, `var a, b;`, `globalvar a;` | `++` / `--` (statement `i++` → "Assignment operator expected."), `?:`, `%` ("Unexpected symbol."), `%=`, `<<=`, `>>=`, `mod=` |
| `= := += -= *= /= \|= &= ^=` | `var a = 1;` (no initialiser), `return;` without value (compile error — use `exit`) |
| `and or xor not div mod`, `&& \|\| ^^ !`, `& \| ^ ~ << >>`, `<>` = `!=` | exponent literals `1e3` (compile error), `0xFF` (use `$FF`) |
| strings in `'…'` or `"…"`, may span lines | escape sequences (`\n`, `\"` are literal text); use `chr(34)`, `chr(39)`, and `#` for a line break in `draw_text` (`\#` = literal `#`) |
| 1-D and 2-D arrays `a[i]`, `a[i, j]` | array literals, arrays as values, `[\| ]`, `[? ]`, `[# ]`, `[@ ]`, structs, methods, `enum`, `#macro`, `#define` in code |
| `a.b.c`, `(expr).var`, `f(x).var` (a call can be the owner of `.`) | indexing a call result (`f()[0]`); `100032.speed` (write `(100032).speed`) |

* Identifiers are case-sensitive. Keywords: `if then else begin end repeat while do
  until for switch case default break continue exit return with var globalvar and or
  xor not div mod`. `true false pi self other all noone global local` are constants.
* **`=` inside an expression is comparison**: `a = b = c` assigns `b == c` to `a`;
  `if (x = 3)` compares.
* **Constant folding**: operations on literals are evaluated at compile time, so
  `'a' + 1` ("Wrong type of arguments to +.") and `1 / 0` ("Division by 0.") are
  **compile errors** (VERIFIED). A compile error in a script makes the game fail at
  startup; in a code action it is fatal when the action first runs.
* **First token `{`** (VERIFIED): the block is the whole program; any token after its
  closing `}` is "Program ends before end of the code.". Either put everything inside
  one outer block or start the code with something else.
* Function calls need parentheses; built-ins have fixed arity (wrong count is a
  compile error) except variadic ones (`choose`, `min`, `max`, `mean`, `median`,
  `script_execute`, `execute_string`, …), max 16 arguments.

### 6.2 Precedence (lowest → highest; all left-associative)

1. `&& || ^^` (`and or xor`) — **one level**: `a || b && c` = `(a || b) && c`
2. `< <= == = != <> >= >` — one level: `3 > 2 > 1` is `(1) > 1` = 0
3. `| & ^` — one level: `1 | 2 ^ 3` = 0, `a & b == c` = `(a & b) == c`
4. `<< >>`
5. `+ -`
6. `* / div mod`
7. unary `! - + ~`
8. `.`, `[ ]`, calls

All VERIFIED (bb_lang `prec.*`). When in doubt, parenthesise.

### 6.3 Values, truthiness, comparison, no short-circuit

* Values are **reals** (doubles) or **strings**. Booleans are 1/0; ids, colours,
  resources are reals. No implicit conversion: use `string(v)` / `real(s)`.
* Mixing types is a runtime error: `'a' + 1` via variables → "Wrong type of arguments
  to +."; comparing a string with a real → "Cannot compare arguments."; a string
  condition → "Expression expected"; `repeat ('3')` → "Repeat count must be a number".
  `+=` on strings concatenates; `-=` etc. on strings are errors.
* **Truthiness** (VERIFIED): a real is true iff `>= 0.5` (`0.5` true, `0.4999999`
  false, negatives false). `true == 0.5` is false.
* **Equality** (VERIFIED): reals are equal when `|a - b| < 1e-13` — absolute, also for
  large numbers. `sin(pi) == 0`, `lengthdir_x(10, 90) == 0`, `0.1 + 0.2 == 0.3` are
  true. `<` means "less and not equal".
* **No short-circuit**: both operands of `&& || ^^` are always evaluated. Guard
  dependent reads with nested `if`s:
  ```gml
  if (instance_exists(target)) { if (target.hp > 0) { ... } }
  ```
* Integer conversions round **half to even** (VERIFIED): `repeat (2.5)` runs 2 times,
  `repeat (3.5)` 4; `a[1.5]` is `a[2]`; `round(2.5) = 2`; `alarm[0] = 1.5` stores 2.
  `floor`/`ceil` return a 32-bit integer (values beyond ±2^31 wrap). `div` truncates,
  `mod` keeps the dividend's sign and works on fractions; `/`, `div`, `mod` by 0 are
  errors. Bitwise ops use 64-bit integers; `>>` is a logical shift.

### 6.4 Variables

| Kind | Syntax | Lifetime / rules |
|---|---|---|
| instance | `hp = 3;` | created on first assignment, on `self`; not visible to other instances except via `inst.hp` |
| local | `var i, n;` | **one code action / script invocation**; lexical from the `var` onwards (even if the `var` line isn't executed); visible inside `with` bodies; **cannot be a built-in variable name** (`var x;` is "Cannot redeclare a builtin variable.") |
| global | `global.level = 1;` or `globalvar level;` | whole game; **cleared by `game_restart()`** (VERIFIED) |
| built-in | `x`, `speed`, `alarm[0]`, `room`, `score`, `view_xview[0]`, … | fixed types: assigning a string to a real built-in stores 0 silently; boolean built-ins are true when > 0.5; writing a read-only one (`id`, `argument_count`) is the runtime error "Cannot assign to the variable" |

* **Uninitialised reads** are runtime errors ("Unknown variable foo") unless the
  setting "Treat uninitialized variables as value 0" (flags bit 0) is on. Initialise
  every instance variable in Create and every global in the first room's controller.
  `variable_local_exists('name')` / `variable_global_exists('name')` test existence.
* Arrays (VERIFIED, bb_lang `arr.*`):
  * A plain variable and element `[0]` / `[0,0]` are the same storage: `a = 5` ⇒
    `a[0] == 5`; `a[0] = 7` ⇒ `a == 7`. Assigning `a = 7` later changes only `a[0]`;
    other elements survive.
  * 1-D `a[i]` is row 0 of a 2-D array: `a[3]` ≡ `a[0, 3]`.
  * Each row has its own length. Unwritten elements below the highest written index
    of that row read 0; anything beyond, or in a row never written, is an error
    ("Unknown variable a or array index out of bounds").
  * Index limits: `0 ≤ index < 32000` per dimension ("Array index >= 32000",
    "Negative array index"); a string index is an error.
  * Arrays are not values: `b = a` copies `a[0]` only; scripts cannot take or return
    arrays. Pass an instance id or use globals.

### 6.5 Names and resolution

* Name lookup in code: keyword → call (built-in function, **but a script with the same
  name wins**) → `var` local → built-in variable → constant (resource name > user
  constant > built-in constant) → instance/global user variable.
* A resource name is a constant: you cannot assign to it, and you cannot use it after
  `.` (`global.obj_player = 1` → "Variable name expected."). **Use prefixes** for
  every resource (`spr_`, `snd_`, `bg_`, `path_`, `scr_`, `fnt_`, `tl_`, `obj_`,
  `rm_`) and never name variables like them.
* Never name resources like built-ins, keywords or constants (`score`, `health`,
  `self`, `c_red`, `abs`).

### 6.6 Scripts and arguments

* Call `scr_name(a, b)`; up to 16 arguments; `argument0..argument15`,
  `argument[i]`, `argument_count`. Scripts run with the caller's `self`/`other`.
* With settings bit 1 set (VERIFIED), a call must pass exactly `1 + highest argumentN
  used` arguments ("Illegal argument count calling script …"); a script that also uses
  `argument[i]` accepts at least that many. With bit 1 clear, any count works and
  missing ones read 0. **Safest: always pass exactly the arguments the script uses.**
* `return expr;` returns a value. Without `return`, GM returns the value of the last
  assignment/call statement executed — don't rely on it; return explicitly.
* `exit` leaves the current script/code action (not the event). `break`/`continue`
  outside a loop act like `exit`.
* `script_execute(scr, args…)` with a wrong count reports an error but still runs.

### 6.7 Library behaviours that surprise

* `string(x)`: integers print as integers; other reals with **2 decimals**
  (`string(2.675) = "2.67"`, `string(0.995) = "1.00"`), more than 10 integer digits
  switch to `1.23456789E11`; integral values beyond 64-bit print `"ERROR"`. Use
  `string_format(x, total, decimals)` for controlled output.
* `real(s)` is strict: only `[+-]digits[.digits][e[+-]digits]` with surrounding
  whitespace; `"12abc"`, `"$10"` etc. give an error and 0.
* Errors in **built-in functions** (e.g. `sqrt(-1)`, `real('x')`, `sound_play(99)`)
  are reported, return 0 and the code continues. Errors in **code** (operators,
  unknown variables, bad instance, script calls) abandon the rest of the current
  action; the next action runs.
* Strings are one byte per character (Windows-1252). `chr(n)` for 0..255.
* `min/max/choose/mean/median()` with no arguments return 0.
* `random(n)`, `irandom(n)`, `choose(…)`; the generator is seeded randomly at start —
  call `random_set_seed(k)` for reproducible runs (GM's generator is reproduced
  exactly by Instance, VERIFIED on 191 values).
* `file_text_open_read` of a missing file returns -1 (no error); `file_text_write_real`
  writes a leading space.
* `show_message`, `get_string`, `show_question` are modal (the game pauses).
* `execute_string(code)` compiles at run time; its errors (compile or run) are
  reported and it returns 0 to a caller that keeps running. Slow-ish — avoid per step.

### 6.8 Instances, `.` and `with`

| Owner value | read `o.v` | write `o.v = …` |
|---|---|---|
| `self` (-1), `other` (-2) | that instance | that instance |
| `all` (-3) | first instance | all instances |
| `noone` (-4) | error "Unknown variable v" | error "Cannot assign to the variable" |
| `global` (-5) | global variable | global variable |
| object index | first instance of the object **or a descendant** | all of them (`obj.y += 8` reads the first, writes all) |
| instance id ≥ 100000 | that instance; error if gone | that instance; silent no-op if gone |

`with (expr) stmt` runs `stmt` once per target with `self` = target and `other` = the
code's previous `self`. VERIFIED details: the target list is a **snapshot** (instances
created inside are not visited); instances destroyed before their turn are skipped;
descendants are included; `with (noone)` does nothing; `break` ends the loop; `var`
locals stay visible inside. (Per the manual-based spec, `exit` inside `with` leaves
the whole program.)

`other` outside `with`: the partner in a collision event; **the instance itself** in
a Create event run by `instance_create` and in a Destroy event (VERIFIED);
`event_user`/`event_perform` keep the caller's `other`.

`instance_destroy()` runs the Destroy event immediately; afterwards
`instance_exists(id)` is false and `instance_number` excludes it, while the destroyed
instance's own running code can still read its variables (VERIFIED). Calling it twice
in the same code runs Destroy twice.

`instance_change(obj, perf)` (VERIFIED, surprising): it replaces the instance record.
The code that called it keeps seeing the **old** object and properties, and its writes
after the call are lost; other code sees the new object at once; `with (id)` finds the
instance only from the next step. Call it as the last statement, or avoid it (destroy +
create is clearer).

---

## 7. Runtime rules that matter to game authors

Spec: `specs/runtime.md` (VERIFIED sections §3.2a, §4.3a, §5.7, §8.2, §8.3, §8.6,
§10, §19).

### 7.1 Order of one step (VERIFIED)

1. Begin-step triggers, **Begin Step**
2. Time lines advance (moments run)
3. **Alarms** (alarm 0 for all objects, then alarm 1, …)
4. **Keyboard** (held), **Key Press**, **Key Release**
5. **Mouse** events
6. Step triggers, **Step**
7. Motion: friction, gravity, path, `x += hspeed; y += vspeed`
8. Boundary events (Outside Room, Intersect Boundary, view variants)
9. **Collision** events
10. End-step triggers, **End Step**
11. Destroyed instances removed
12. **Draw** (once per visible view), then image_index advances and **Animation End**
    fires

(The order follows runtime.md §4.3a, VERIFIED except that mouse events were not
exercised by the tests; step 11's position is the spec's
recommendation.)

Dispatch order: Begin Step / Step / End Step / alarms go **object by object in
ascending object index**, and within an object by instance order (VERIFIED). An
instance created during a phase runs in that phase only if its object has not been
processed yet. Room Start/End and other one-off broadcasts use instance (creation)
order. Consequence: if a controller must step before the player, give it a **lower
object index** (create it first).

Other VERIFIED points:

* Timelines run after Begin Step and before alarms; held-key Keyboard events come
  before Key Press events.
* (Not verified: the exact points where `xprevious/yprevious` are captured — at the
  start of the step per the spec — and where destroyed instances are purged.)
* Animation: `image_index += image_speed` after Draw; on reaching `image_number` it
  wraps (subtract once) and fires Animation End. Negative `image_speed` never wraps
  and never fires Animation End. Draw sees the pre-advance value.
* Outside Room fires **every step** while the bounding box is completely outside the
  room (by at least a pixel); Intersect Boundary fires every step while the box is not
  completely inside (also when fully outside). Destroy or wrap the instance to stop it.
* No More Lives / No More Health fire **at the assignment** that makes `lives`/`health`
  ≤ 0.
* `keyboard_key_press(k)` takes effect in the next step.

### 7.2 Alarms (VERIFIED)

* 12 per instance, `alarm[0..11]`, default -1. Values are integers (assignment rounds
  half to even).
* Each step, an alarm > 0 is decremented; when it reaches 0 the event fires (and
  reads 0 inside it; it becomes -1 a step later unless re-armed). `alarm[i] = 0`
  (or 0.5) never fires. So `alarm[0] = 1` fires in the next step; `alarm[0] =
  room_speed` fires after one second.
* **Alarms only tick if the object has (or inherits) that Alarm event.**
* Re-arm inside the event for repeating timers: `alarm[0] = 30;`.

### 7.3 Rooms (VERIFIED, runtime.md §3.2a)

* `room_goto(r)` / `room_goto_next()` / `room_restart()` / `room = r`: the **current
  action finishes**, the remaining actions of that event are skipped, and the rest of
  the step is abandoned (no later instances' events, no End Step, no Draw). Then Room
  End runs for every instance and the new room is entered. Put `room_goto` last.
* Entering a room: all placed instances are added to the instance list first
  (`instance_count` already counts all of them in the first Create); carried
  persistent instances come after them; then, per placed instance, its **instance
  creation code, then its Create event**; then **Game Start** (first room only); then
  the **room creation code**; then **Room Start** for everyone; then one frame is
  drawn before the first Begin Step.
* Persistent instances survive room changes; a placed instance whose id already
  exists (a persistent one that came back) is not created again. But a *different*
  placed instance of a persistent object in another room is a second copy — place
  persistent controllers in exactly one room that is visited once (e.g. a boot room).
* Persistent rooms keep their state; re-entering one does not re-run its creation code
  or Create events (restored instances get Room Start).
* `room_goto_next()` in the last room / `room_goto_previous()` in the first is an
  error (per the manual). Check with `room_next(room) != -1`.
* `game_restart()`: action finishes, rest of event skipped, Room End then Game End for
  all, all instances removed (no Destroy), **globals cleared**, `score` 0, `lives`
  -1, but `health` keeps its value.
* Defaults at game start: score 0, lives -1, health 100, `show_score` 1,
  `show_lives`/`show_health` 0, caption parts "Score: ", "Lives: ", "Health: ".
  Window caption = `room_caption` + shown parts (single spaces).

### 7.4 Collisions (VERIFIED)

* A collision event fires only for objects that **have** a collision event for the
  other object (or one of its ancestors). Without one, instances pass through each
  other, solid or not.
* Order: objects ascending; for each, its collision events by target object
  ascending; then instance order. Each colliding pair is handled once per step: the
  first instance's event, then immediately the partner's event if it has one.
* If **either** instance is solid: the instances are moved back to their
  `xprevious/yprevious` before the events, then moved to their new positions again; if
  they still collide they stay at the previous position. (Tested with static walls;
  whether a moving partner is also put back is not verified.) Speed is not changed — a blocked mover keeps
  trying every step. Set `speed`/`hspeed` to 0 or use `move_contact_solid` in the
  event for clean stops.
* Bounding boxes: corners `xscale·(left − xorigin)` and `xscale·(right + 1 − xorigin) − 1`
  (same for y), rotated by `image_angle`, translated, rounded half to even. A pixel
  belongs to an instance only if its centre maps inside the mask, so boxes touch
  only when they really overlap (16-px box at x = −15.5 does not touch one at 0).
* Invisible instances still collide; collisions use the sprite or `mask_index`, never
  what the Draw event draws. An instance without sprite and mask never collides.
* `place_free(x, y)` tests solids only; `place_meeting(x, y, obj)` any instance of
  obj; `instance_place` returns the first one.

### 7.5 Motion

`speed/direction` and `hspeed/vspeed` are coupled (direction in degrees, 0 = right,
counter-clockwise, **y grows downward**, so 90 = up). Friction reduces speed each step;
`gravity` + `gravity_direction` (270 = down) add to the velocity each step. Paths:
`path_start(path, speed, endaction, absolute)`; end actions 0 stop, 1 restart,
2 continue, 3 reverse; End of Path fires in every case (VERIFIED details in
runtime.md §10.1). Time lines: moments run when the position passes them; set
`timeline_index`, `timeline_position = 0`, `timeline_running = true` (assigning the
index alone does not start it).

### 7.6 Drawing and views

* Draw order: higher `depth` first (background), lower/negative last (front). Equal
  depth keeps the previous frame's order; new instances go last (VERIFIED).
* An object **with** a Draw event does not draw its sprite automatically — call
  `draw_sprite(sprite_index, image_index, x, y)` yourself (`draw_self()` also exists,
  but it draws with `image_blend`/`image_angle`; its Lite status is not verified, §8). Invisible instances get no Draw event (but still step and
  collide).
* Draw state (`draw_set_color`, `draw_set_font`, `draw_set_alpha`, alignment) is global
  and persists between events and frames; set it explicitly before drawing.
* Drawing outside the Draw event is wiped by the next frame.
* **Views:** with views enabled, Draw runs once per visible view, in room coordinates.
  There is no GUI layer: draw a HUD from a high-priority object (very low depth, e.g.
  -10000) at view-relative positions:
  ```gml
  var vx, vy;
  vx = view_xview[view_current]; vy = view_yview[view_current];
  draw_set_color(c_white);
  draw_text(vx + 8, vy + 8, 'Score: ' + string(score));
  ```
  (Check `view_current` if several views are visible and the HUD should appear once.)
* View following: set the view's follow object and borders in the room; a border of
  at least half the view keeps the target centred.
* Text: `#` is a line break in `draw_text`; `draw_set_halign(fa_center)` etc. The
  default font is Arial 12 when none is set; for consistent output add a Font resource.
* Colours: `make_color_rgb(r, g, b)` = `r + 256*g + 65536*b`; `c_white`, `c_black`, ….

### 7.7 Sound

Kinds: **0 normal** (effects; any number of overlapping copies), **1 background
music** (one at a time; starting one stops the current — usual for MIDI), **2 3D**,
**3 multimedia player** (MP3; one at a time; no effects/volume). `sound_play`,
`sound_loop`, `sound_stop`, `sound_stop_all`, `sound_isplaying`, `sound_volume`.
Sounds continue across room changes; `game_restart` stops them. Sound data is
stored uncompressed in the project — keep files small (MIDI music is tiny).

**Composing and checking sound without ears** (`crates/instance-music`, full
reference in its README):

* `gmk-music` compiles a compact text notation (tracks with GM instruments,
  notes like `E5q. D5e`, chord symbols `{Am7}w`, accompaniment patterns played
  `over | Am | F | C | G |`, drum grids, sections, `arrange |: A B :|`,
  per-area `variant`s) to a type-1 MIDI file (a few KB; add as a kind 1 sound,
  `sound_loop`) or, with a SoundFont (`INSTANCE_SOUNDFONT`), to a WAV;
  `--loop` renders a seamless loop. `gmk-music info SONG.txt` reports length,
  ranges, density and mistakes (failed `|` bar checks, flat dynamics, silent bars).
  Prefer this to hand-placing samples: 60–120 s loops in ~80 lines of text.
  GM patches sound like "MIDI"; for synthwave/electronic music give tracks
  `synth=PRESET` (supersaw, lead, pluck, pad, bass, drums, …; `gmk-music
  synths`): a built-in synth with filter sweeps, sidechain `duck=`, chorus,
  delay and reverb, rendered to WAV (no SoundFont needed).
* `gmk-sounds GAME.gm81` measures every sound: peak/RMS dBFS, clipping,
  silence, attack, spectral centroid and band energies, a one-line description
  ("short percussive bright noise"), seam checks for looped sounds (kind 1,
  `sound_loop` in code, "loop" in the name) and a similarity table that flags
  sound effects that sound alike. Run it after adding sounds and act on its
  `check`, `loop` and **THESE SOUND ALIKE** lines.

---

## 8. Lite vs Standard edition restrictions

GM 8.1 came as a free **Lite** edition and a paid **Standard** ("Pro") edition. A game
that must run in the Lite IDE must avoid Standard-only features. Our evidence:

* `specs/actions.json` `pro_only` flags (drag-and-drop actions): Transform Sprite,
  Color Sprite, Show Video, all Splash actions, Replace Sprite/Sound/Background, all
  particle actions (systems, particles, emitters), all CD actions, Set Cursor, Open
  Webpage, Draw Scaled Text, Horizontal/Vertical Gradient, Gradient Ellipse, Take
  Snapshot, Create Effect.
* Black-box testing on GM 8.1 Lite (`crates/instance-blackbox`): the `ds_list`
  benchmark produced no result (data structures unavailable); the test plan treats
  3D (`d3d_*`), surfaces, particles, blend modes, data structures (`ds_*`),
  multiplayer (`mplay_*`) and DLLs (`external_*`) as Standard-only, and drawing with
  `image_blend` / `image_angle` (rotated / blended sprite drawing) as Standard-only.
  Extension packages are believed to be Standard-only too (not tested).
* Verified to **work in Lite**: everything in §6–§7, `image_xscale/yscale/angle`
  affecting bounding boxes, `file_text_*` functions, `draw_getpixel`,
  `execute_string`, `keyboard_key_press`, sounds, timelines, triggers, paths, views,
  `random*`.

Observed directly in GM 8.1 Lite: calling a Standard-only function is a run-time
ERROR (non-fatal) with the message `This function is not available in the Lite
Edition.` (seen for `ds_list_create`), and *writing* a Standard-only variable raises
`image_blend: This variable is not available in the Lite Edition.` every time it is
assigned (seen for `image_blend` in a collision event — so it floods the error log).
Assume every other Standard-only feature fails the same way. **Instance has no editions** — everything works there, so
test Lite compatibility by reviewing the code (grep for `ds_`, `surface_`, `d3d_`,
`part_`, `effect_`, `draw_set_blend_mode`, `external_`, `mplay_`, `image_angle`,
`image_blend`, `draw_sprite_ext`, `draw_self`) or in the real GM.

Lite-friendly substitutes: arrays (global or instance) instead of `ds_list`/`ds_grid`;
several objects or a parent instead of maps; frame animation instead of rotation;
pre-coloured sprite frames instead of `image_blend`.

---

## 9. Game design tips for GM 8.1

**Structure**

* One **controller object** (`obj_game`, persistent, placed only in the first room,
  lowest object index so it steps first) owns globals, score, level flow, HUD and
  music. Initialise all globals in its Create.
* Use **parent objects** for shared behaviour: `obj_enemy_parent` with Step/collision
  logic, children override what differs and call `event_inherited()` to extend.
  Collision events against a parent catch all children: `obj_player` collides with
  `obj_solid_parent`.
* Keep code in **scripts** for reuse (`scr_player_move()`), and pass exactly the
  arguments they use (§6.6).
* User events (`event_user(0)`) are cheap named callbacks inside an object.
* Name everything with prefixes; avoid names of built-ins (§6.5).

**Rooms and views**

* Use one room per level plus a boot/title room. Room creation code is a good place
  for per-level parameters (`global.level = 3;`). It runs *after* every Create
  event, so read those globals in Room Start (or later), not in Create.
* **Instance creation code** configures one placed instance (a moving platform's
  `amp = 70; spd = 1.6;`). It runs *before* that instance's Create event, so the
  object's Create must read those variables and never assign them, or it overwrites
  the room's values. Give every placed instance of such an object creation code.
* Room speed 30 (default) or 60. Movement speeds are per step.
* For scrolling games enable view 0, set its size and port, follow the player with
  borders; draw the HUD view-relative (§7.6).

**Sprites and collisions**

* Put the **origin** where the object "stands" (centre for top-down, bottom-centre for
  platformers) — it is the rotation/scale pivot and the x/y point.
* Prefer **rectangle masks** (collision_shape 1) for walls and players: faster and
  predictable. Use a dedicated mask sprite (`mask_index` / object `mask_sprite`) so
  changing animation frames doesn't change the hitbox.
* Platformer movement: check `place_free(x + hspeed, y)` / `place_meeting` before
  moving, or use solid walls with `move_contact_solid(direction, speed)` in the
  collision event and then zero the speed.

**Timing**

* Alarms for delays and repeats (`alarm[0] = room_speed * 2`), time lines for
  scripted sequences, `current_time` (ms) for real-time measurements.

**Performance** (GM 8.1 Lite measurements, `bb_perf`, rough)

| Operation | Throughput |
|---|---|
| empty `for` loop iteration | ~4 million/s |
| simple arithmetic / variable access | ~1–2 million/s |
| script call | ~600 000/s |
| `string(real)` | ~180 000/s |
| `place_free` (500 walls) | ~140 000/s |
| `instance_nearest` | ~100 000/s |
| `instance_create` + destroy | ~33 000/s |
| `with (obj) x += 1` over 300 instances | ~6 million instance-iterations/s |
| 1000 moving, self-colliding instances | ~110 steps/s |

So: keep active instances in the hundreds, avoid building strings every step (cache
HUD text, update it when the value changes), avoid `execute_string` in loops, prefer
`with` over looping `instance_find`, deactivate far-away instances
(`instance_deactivate_region` + `instance_activate_region`) in big levels, use tiles
or backgrounds for static scenery instead of instances.

**Saving and files**

* `game_save(file)` / `game_load(file)` save the whole game state (not sounds,
  resources changed at run time, or data structures); F5/F6 do it if enabled in the
  settings (consider disabling `let_f5_save_f6_load` for real games).
* Text files: `file_text_open_write`, `file_text_write_string`, `file_text_writeln`,
  `file_text_close`; check `file_exists` before reading. INI functions (`ini_open`,
  `ini_read_real`, …) exist in the function list; their Lite status is not verified.
* Highscores: `highscore_add`, `highscore_show` (built in, per game id).

**Sound**

* Background music as MIDI with kind 1; effects as short WAV with kind 0. MP3 needs
  kind 3 (multimedia), one at a time.
* Write music with `gmk-music` (§7.7): 60–120 s loops with bass, harmony,
  a melody built on a motif, drums and section-to-section dynamics; derive
  per-area tracks from one theme with `variant`s. Then run `gmk-sounds` on the
  .gm81: no clipping, music RMS below the effects' RMS, smooth loop seams,
  and no two effects for different actions above the similarity threshold
  (players notice when e.g. a drill and a jetpack loop sound the same).

**Robustness**

* Keep "Display error messages" on during development; turn on "write to log" too.
* Turning on "treat uninitialized variables as 0" hides bugs; prefer initialising.
* Guard references: `if (instance_exists(obj_player)) { … }` before `obj_player.x`.

---

## 10. Testing headless with scripted input

```sh
instance-runner game.gm81 --headless --frames 300 --screenshot out.png
instance-runner game.gm81 --headless --frames 300 --screenshot-every 30 frames/
instance-runner game.gm81 --headless --frames 600 --input "10:down:right,40:up:right,45:press:space"
instance-runner game.gm81 --headless --frames 600 --random-input 1 --max-seconds 60
instance-runner game.gm81 --debug          # windowed, with the Debug Information window

# a typical automated test: through the title menu, cheat money in, watch it,
# fast (no rendering), reproducible, with its own save folder, sound recorded
instance-runner game.gm81 --headless --frames 600 --seed 1 --save-dir test_save \
    --input 30:press:enter --exec "120:global.cash = 100000" --watch 60:global.cash \
    --screenshot-at 200,600 shots --no-draw --audio-out a.wav --log run.log
```

**Input**

* `--input` takes comma-separated items keyed by step number: `STEP:down:KEY`,
  `STEP:up:KEY`, `STEP:press:KEY` (down, then up two steps later), `STEP:move:X:Y`,
  `STEP:click:X:Y` (left button down, up one step later), `STEP:char:C`; joystick
  items `STEP:joy1:button:N:down|up`, `STEP:joy1:axis:x:VALUE`, `STEP:joy1:pov:ANGLE`.
  KEY: `left right up down space enter escape shift ctrl alt backspace tab`, a single
  letter/digit (`3` is the digit key, code 51), `f1`…`f12`, or a virtual-key code of
  two or more digits (`37`).
* `--random-input SEED[@START]` adds a deterministic "monkey" script (held arrows,
  WASD, space, enter, Z, X, shift; mouse moves and clicks inside the window size of
  the room the game starts in; never Escape/function keys) from step START (default
  3) — give a START after your scripted `--input` prefix (e.g. getting through a
  title menu) so random keys do not interfere with it. `--random-keys LIST` replaces
  the key set (names as in `--input`: `--random-keys left,right,up,space,1,2`);
  `--random-no-mouse` leaves out mouse events. Good for smoke-testing that nothing
  crashes.

**Injecting code and watching values** (headless only)

* `--exec STEP:CODE` (repeatable) and `--exec-file STEP:FILE.gml` run GML at the
  start of step STEP, before that step's events and input (STEP 0 = after the first
  room's Create / Game Start / Room Start events, before the first step). The code
  is compiled and run as by `execute_string`. Context: `self` and `other` are the
  **first live instance of the current room** (creation order), or the global
  pseudo-instance when the room has none (then only `global.*`, `with`, functions
  and scripts work). Don't rely on which instance that is — name the target:
  `--exec "120:with (obj_player) { x = 100; }"`, `--exec "0:global.cash = 1e5"`,
  `--exec "300:room_goto(rm_level3)"`. Errors print as `ERROR in --exec at step N:`
  (with the code excerpt) and never stop the run.
* `--watch K:EXPR` (repeatable) prints `watch step N: EXPR = value` on stderr after
  every K-th step, same context; reals in full precision, strings quoted
  (`--watch 60:global.cash`, `--watch 30:obj_player.x`,
  `--watch "60:instance_number(obj_enemy)"`). A failing watch (e.g. the instance does
  not exist yet) prints its error once, and again only after it has succeeded in
  between. Watches never change `error_occurred`/`error_last`.
* `--exec-room ROOM` (name or number) goes to ROOM before the first step: the first
  room is entered as usual (its Create, Game Start and Room Start events run, no
  step is played), then ROOM as by `room_goto`, so globals and persistent
  controllers the first room sets up exist. `--exec-room-direct ROOM` replaces the
  first room instead (Game Start runs in ROOM; whatever the first room initialises
  is missing). State a menu sets up when you press Enter (e.g. "new game") is not
  there either way — script the menu with `--input` or call the set-up script with
  `--exec 0:...`.
* `--seed N` makes `randomize()` (including the one at game start) seed from N:
  `random`/`irandom`/`choose` repeat exactly from run to run, so two runs (or a run
  with and without `--no-draw`) can be compared. Real-time sources
  (`current_time`, dates, `fps`) still vary.

**Files** — where relative file names go

* A game's `working_directory` (and `program_directory`) is **the folder of the
  game file** (`game.gm81`), not the directory you start the runner from: `ini_open
  ("save.ini")`, `file_text_open_write("save.txt")`, `game_save("x.sav")`, the error
  log and F9 screenshots all land next to the game file. (Exception: a game file in
  the system temp folder — the IDE's test runs — uses the current directory.)
* `--save-dir DIR` makes DIR the `working_directory` (created if missing) so a test
  run never touches the real save next to the game: everything written with a
  relative name goes to DIR, and included files set to export to the working
  directory are exported there. `program_directory` still names the game's folder.
  Bundled files keep loading: the first time the game uses a relative name that is
  missing from DIR but exists next to the game, it is copied into DIR (copy on first
  use; the original is never written, a file the game deletes stays deleted). That
  also copies an existing save file — add `--no-read-fallback` for a clean start
  (only what is in DIR exists). Use a fresh DIR per test for reproducible runs.

**Frames, speed, sound, logs**

* `--screenshot-at LIST DIR` saves the frames after the listed steps
  (`--screenshot-at 60,200,450 shots/` → `shots/frame00060.png` …), with the same
  naming as `--screenshot-every K DIR`; both can be combined.
* `--no-draw` renders only the frames that are saved (`--screenshot`,
  `--screenshot-at`, `--screenshot-every`). On the other frames the game's **Draw
  events still run** (games with logic in Draw behave the same), `image_index`
  still advances and Animation End still fires, but backgrounds, tiles, default
  sprite draws, particles and the cursor are skipped and every drawing operation
  into the screen is discarded (drawing into surfaces still happens). Caveats: code
  that reads the screen back — `draw_getpixel`, `screen_save`, `surface_copy` /
  `background_create_from_screen` from the screen — sees the last
  rendered frame; if the game ends before the final step, `--screenshot` shows the
  last rendered frame. With `--seed`, runs with and without `--no-draw` produce the
  same game state. Speed-ups measured over 2000 steps: 7–20× (deepcore 16.6 →
  1.2 ms/step, skyhook 3.3 → 0.44, neon_swarm 1.3 → 0.43, lantern 2.6 → 0.13).
* `--audio-out FILE.wav` records the game's audio offline into a 44.1 kHz 16-bit
  stereo WAV: each step advances exactly `1/room_speed` seconds of audio, so the
  file is the same on every run (with `--seed`) and as long as the game time,
  however fast the run goes. PCM sounds go through the same voice code as live
  playback (volume, pan, fades, 3D, effects); MIDI is rendered with the SoundFont
  synthesiser (set `INSTANCE_SOUNDFONT=path/to/file.sf2`; without one MIDI is
  silent). `sound_isplaying` and fades follow this audio clock. At the end the
  runner prints `audio-out: FILE: <seconds> s, peak <dB> dBFS, RMS <dB> dBFS, <x>%
  clipped samples` — the peak is measured before clipping, so a peak above 0 dBFS
  or a non-zero clip percentage means the mix is too loud.
* `--log FILE` also writes everything printed on stderr (errors, `watch` lines,
  `show_debug_message` output, the summary) to FILE — useful on Windows, where the
  runner is a GUI program and a shell may not capture its output.
* `--max-seconds S` kills hung games (exit 4).
* Look at the screenshot (an image the agent can read) and at stderr for `ERROR`
  lines and the "stubbed/unsupported functions" list.
* A practical self-test pattern: let the game write its own assertions via
  `show_debug_message('ok: …')` (printed on stderr by Instance) or to a text file with
  `file_text_*`, then grep the output; or check state from outside with `--watch`.
* `INSTANCE_PROFILE=1` prints per-event/function timings.
* Headless runs use the software renderer, where every background layer that covers
  the view costs roughly 15 ms per frame (measured: about 11 ms/frame with no layer,
  25 with one, 42 with two, on a 480×640 view). Expect slow headless runs for games
  with background layers; the windowed (GL) renderer was not measured.
* Reaching late content quickly: when the game stores progress in a file, write that
  file before the run (e.g. "all levels unlocked") and select the level with scripted
  input. When the game is generated by a program, build a throwaway variant (player
  placed next to the goal, say) into a scratch file instead of adding debug code to
  the real game.
* `--screenshot-every` frames pasted into one contact-sheet image make a long run
  quick to review.
* Files the game writes with a relative name (`file_text_open_write('save.txt')`)
  land in the runner's working directory in Instance; delete them between test runs
  if they affect the game. (Where GM 8.1 writes them was not checked.)

---

## 11. Converting GameMaker: Studio projects

`gmx2gm81 <Game.gmx dir | Game.project.gmx | Game.gmz> -o game.gm81` (crate
`crates/instance-gmx`, spec `specs/gmx-format.md`). Library use:
`instance_gmx::load_gmx(path) -> (Project, Vec<Warning>)`.

What it does: parses every code piece and rewrites Studio-only syntax (`++`/`--`,
`?:`, accessors → `ds_*` calls, `0xFF` → `$FF`, `var a = 1`, `for (var …)`,
`return;` → `exit`, `enum` inlined, `#define` split, short-circuit `&&`/`||` emulated
when the right side reads another instance), moves Draw GUI into a generated
controller drawing with an orthographic projection, merges Draw Begin/End into Draw,
maps audio/gamepad/etc. functions onto 8.1 equivalents via injected compat scripts,
and stubs unknown functions (listed in the warnings).

Caveats:

* Converted projects default to **"treat uninitialized variables as 0" on** and the
  **argument-count check off** (Studio code relies on both); options `--uninit-error`,
  `--uninit-studio`.
* The output is meant for Instance; it is usually **not Lite-compatible** (accessors
  become `ds_*`, Draw GUI uses `d3d_set_projection_ortho`, and several compat scripts
  rely on data structures).
* Dropped: shaders, physics, audio/texture groups, async events, window-resize event,
  SWF/Spine sprites, native extensions, custom loading images; cross-instance Draw
  Begin/End ordering is lost; arrays-as-values functions return 0.
* Studio's game icon is not carried over (Instance's icon is used).
* `--dump-code DIR` writes every converted code piece for review; read the warnings.

---

## 12. Where everything lives

| Topic | Location |
|---|---|
| Clean-room policy (read before contributing) | `CLEANROOM.md` |
| Project / library / extension byte layouts | `specs/file-formats.md`; decoder `specs/tools/gmk_dump.py` |
| GML language + VERIFIED rules | `specs/gml-language.md` |
| Runtime: events, rooms, collisions, drawing + VERIFIED rules | `specs/runtime.md` |
| Built-in functions/variables/constants | `specs/builtins.json` (format in gml-language.md §15) |
| Drag-and-drop actions (ids, args, flags) | `specs/actions.json` |
| Sound, 3D, extensions, multiplayer, joystick, IDE | `specs/sound.md`, `specs/d3d.md`, `specs/extensions.md`, `specs/multiplayer.md`, `specs/input-joystick.md`, `specs/ide.md` |
| Instance's standalone `.exe` layout | `specs/exe-format.md` |
| Studio → 8.1 mapping | `specs/gmx-format.md` |
| What is unknown / resolved | `specs/OPEN_QUESTIONS.md` (`[RESOLVED-*]` sections list the verified results) |
| Black-box test projects and key manifest | `specs/blackbox-tests.md`, `crates/instance-blackbox` |
| Reader/writer crate + `gmk-info` | `crates/instance-format` (example: `examples/make_game.rs`) |
| Project folders (`gmk-unpack` / `gmk-pack`, `instance_format::folder`) | `docs/project-folder.md`, `crates/instance-format/src/folder/` |
| GML compiler/interpreter (+ verified regression tests) | `crates/gml` (`tests/verified.rs`) |
| Runner | `crates/instance-runner` |
| Music notation → MIDI/WAV (`gmk-music`), sound analysis (`gmk-sounds`) | `crates/instance-music` (README = notation reference, `examples/*.txt`) |

**Black-box harness** (to settle a question against real GM 8.1):

```sh
cargo run -p instance-blackbox -- generate out/        # writes bb_*.gm81 + README.txt
cargo run -p instance-blackbox -- run-ours out/        # runs them in instance-runner → out/ours
# a human runs each bb_*.gm81 in GM 8.1 (F5) and collects results_<name>.txt in gm/
cargo run -p instance-blackbox -- compare out/ours gm/ # lists mismatches with question ids
cargo run -p instance-blackbox -- manifest             # regenerates specs/blackbox-tests.md
```

Each project logs `key=value` lines; adding a test means adding a project in
`crates/instance-blackbox/src/projects/` with keys documented in the manifest and an
`OPEN_QUESTIONS.md` id it answers. Record verified results in the specs (paraphrased,
with the evidence key), never by inspecting GameMaker binaries.
