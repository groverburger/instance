# Project folders

A **project folder** is a GameMaker 8.1 project spread out as ordinary files: GML in
`.gml` files, images as PNG, sounds as the files they were imported from, and metadata
as small TOML files. It is meant for editing with any text editor, for version control
(small, stable diffs) and for tools and agents that generate or patch games.

The conversion is lossless in both directions: unpacking a `.gm81` and packing the
folder again gives a file with identical contents (`canonical_bytes`), including deleted
slots, timestamps, the action metadata copies, the resource tree, the room-editor
state and fields whose meaning is unknown.

```sh
cargo run -p instance-format --bin gmk-unpack -- game.gm81 game/          # file -> folder
cargo run -p instance-format --bin gmk-pack   -- game/ game.gm81          # folder -> file
cargo run -p instance-format --bin gmk-pack   -- game/ game.gmk --version 800
cargo run -p instance-runner -- game/                                     # run a folder directly
```

`gmk-unpack` refuses a non-empty target directory unless `--force` is given; with
`--force` it updates the folder in place: files are only rewritten when their contents
change, stale files inside the resource directories are deleted, and anything else
(`.git`, notes, ...) is left alone. `gmk-pack` writes the format recorded in
`project.toml` (`format_version`) unless `--version` says otherwise.

From Rust: `instance_format::folder::{write_folder, read_folder, read_project_any}`;
`read_project_any(path)` accepts a `.gm81` / `.gmk` file or a folder. The runner
(`instance-runner DIR`) and the IDE (`instance-ide DIR`, which also saves back into the
folder) open folders too.

## Layout

An unpacked small game (`bb_smoke4` from `instance-blackbox generate`):

```text
bb_smoke4/
  project.toml               game id, guid, settings, constants, room order, slot counts
  game_information.rtf       Game Information text
  resource_tree.txt          the resource tree
  .gitattributes             "* -text" (written once; keeps checkouts byte-exact)
  settings/icon.ico
  fonts/bb_font.toml
  scripts/bb_log.gml         script code
  scripts/bb_log.toml        its slot index, name, timestamp
  objects/bb_out/object.toml
  objects/bb_out/create.gml  an event holding one "Execute Code" action
  objects/bb_out/step_end.gml
  objects/bb_out/draw.gml
  objects/bb_save/create.actions.toml   an event with several actions...
  objects/bb_save/create.0.gml          ...and the code of its code actions
  objects/bb_save/create.1.gml
  rooms/rm_tests/room.toml
```

| Path | Contents |
|---|---|
| `project.toml` | `format_version` (800/810), `game_id`, `guid` (hex), `last_instance_id`, `last_tile_id`, `room_order` (room names), `constants` (in order), `extension_packages`, `library_init_code`, `triggers_last_changed`, `constants_last_changed`; tables `[slots]` (number of slots per kind, deleted ones included), `[settings]` (Global Game Settings except the images), `[game_information]` (window settings; the text is in `game_information.rtf`) |
| `resource_tree.txt` | see below |
| `settings/` | `icon.ico`; `loading_bar_back.*`, `loading_bar_front.*`, `load_image.*` when present (stored bytes, extension sniffed) |
| `sprites/<name>/` | `sprite.toml` (origin, collision shape, bbox, `frames` count) and one PNG per frame: `0.png`, `1.png`, ... |
| `backgrounds/<name>.toml` + `<name>.png` | tile settings + image |
| `sounds/<name>.toml` + `<name>.<ext>` | kind, effects, volume, pan, preload, original file name; `data` names the embedded file, stored as is |
| `paths/<name>.toml` | settings, `backdrop_room`, `points = [{ x, y, speed }, ...]` |
| `scripts/<name>.gml` + `<name>.toml` | code + slot index / timestamp |
| `fonts/<name>.toml` | font settings |
| `timelines/<name>/` | `timeline.toml` (`moments` in order) + `step_<n>.gml` or `step_<n>.actions.toml` per moment |
| `objects/<name>/` | `object.toml` (sprite, solid, visible, depth, persistent, parent, mask, `events` in order) + one file per event |
| `rooms/<name>/` | `room.toml` (settings, `backgrounds`, `views`, `instances`, `tiles`, `[editor]`), `creation.gml`, `instances/<id>.gml` (instance creation code) |
| `triggers/<name>.toml` + `<name>.gml` | moment, constant name + the condition |
| `included_files/<file>.toml` + `<file>` | export settings + the stored data |

### Slots, names and references

* Every resource's metadata has `index` (its slot = resource id) and `name`. Slots with
  no resource are deleted slots; `[slots]` in `project.toml` records the total so that
  trailing deleted slots survive.
* File and directory names come from the resource name with characters that are not
  allowed in file names replaced by `_`; a name that is used twice (Windows ignores
  case) gets `~2`, `~3`, ... The file name is cosmetic: `name` in the metadata is what
  counts.
* References are written as names: `sprite = "spr_player"`, `parent = "obj_enemy"`,
  `object = "obj_wall"` in room instances, `background = "bg_tiles"` in tiles and room
  backgrounds, `follow_object` in views, `backdrop_room` in paths, `applies_to` in
  actions, `room_order`. When a name cannot identify the slot (empty, used by several
  resources, starting with `#`, or the slot is deleted) the reference is `"#<slot>"`,
  e.g. `mask_sprite = "#3"`. A raw integer is accepted as well. Absent keys mean "none"
  (`-1`; for `parent`, no parent).
* Action *arguments* are kept exactly as typed (resource arguments are slot numbers,
  as in the file).

### Events

| Event | File stem |
|---|---|
| Create, Destroy, Draw | `create`, `destroy`, `draw` |
| Alarm n | `alarm_0` ... `alarm_11` |
| Step, Begin Step, End Step | `step`, `step_begin`, `step_end` |
| Collision with object | `collision_<object name>` (or `collision_#<slot>`) |
| Keyboard / Key press / Key release | `keyboard_<key>`, `keypress_<key>`, `keyrelease_<key>` |
| Mouse | `mouse_left_button`, `mouse_right_pressed`, `mouse_middle_released`, `mouse_no_button`, `mouse_enter`, `mouse_leave`, `mouse_global_left_button`, `mouse_global_left_pressed`, `mouse_wheel_up`, `mouse_joystick1_left`, `mouse_joystick2_button3`, ... |
| Other | `other_outside_room`, `other_intersect_boundary`, `other_game_start`, `other_game_end`, `other_room_start`, `other_room_end`, `other_no_more_lives`, `other_no_more_health`, `other_animation_end`, `other_end_of_path`, `other_close_button`, `other_user_0` ... `other_user_15`, `other_outside_view_0`, `other_boundary_view_0`, ... |
| Trigger | `trigger_<trigger name>` (or `trigger_#<slot>`) |
| Anything else | `event_<type>_<number>` |

Keys are `vk_*` names (`vk_left`, `vk_space`, `vk_enter`, `vk_numpad0`, `vk_f1`,
`vk_lshift`, ...), a letter or digit (`keyboard_A`, `keypress_7`) or `#<code>`.
Names are only used when they consist of letters, digits and `_`; otherwise the
`#<slot>` form is written. A second event with the same type and number (possible in
files) gets a `~2` suffix. The order of the events is the `events` list in
`object.toml`; event files that are not listed (new ones) are added after the listed
ones, ordered by event type and number.

### Actions

An event (or time-line moment) that is exactly one **Execute Code** action (library 1,
action 603) with default settings is a plain `.gml` file. Anything else — several
actions, drag-and-drop actions, "applies to" other/object, *relative*, *NOT* — is an
`<event>.actions.toml`:

```toml
# Variable: Assign (or add to, when relative) a variable.
[[actions]]
lib = 1
id = 611
args = [
    "score",
    "10",
]
relative = true

# Code: Execute a GML code fragment (argument 0 holds the code).
[[actions]]
lib = 1
id = 603
applies_to = "other"
code_file = "step.1.gml"
```

* `lib`, `id`: library and action id. `applies_to`: `"self"` (default), `"other"` or an
  object name; `relative`, `negate`: default false.
* `args`: the meaningful arguments (the first `argument_count`). For code actions the
  first argument is the file named by `code_file` and `args` lists the rest.
  `args_tail` appears only when the unused argument slots are not the usual `"0"`s.
* The action's library metadata (`kind`, `exe_type`, `function_name`, `code`,
  `argument_count`, `arg_kinds`, `may_be_relative`, `is_question`, `has_applies_to`) is
  taken from the built-in action table and written only where it differs (always, for
  actions of libraries that are not built in).
* The comment above each action is informational; comments are ignored when packing.

### Text, line endings, encoding

Strings in a project are Windows-1252 with CR LF line breaks. On disk all text is UTF-8
with LF line breaks. Packing turns LF (or CR LF, if an editor or git wrote it) into
CR LF, so new and edited files follow GameMaker's convention. A text whose line breaks
were not uniformly CR LF in the project records its mode next to the reference so that
packing reproduces it exactly: `eol = "lf"` / `"raw"` in script and trigger metadata,
`[eol]` tables keyed by file name in `object.toml` / `timeline.toml`, `code_eol` on
actions and instances, `creation_eol` in `room.toml`, `rtf_eol` in
`[game_information]`. `lf` keeps bare LF; `raw` writes the text exactly as stored
(keep such files away from line-ending conversion; the `.gitattributes` written by
`gmk-unpack` does that for git). A UTF-8 byte order mark is ignored. Characters that
Windows-1252 cannot represent are an error when packing (with file and line).

Strings inside TOML files are always single-line with escapes (`"a\r\nb"`), so line
ending conversion of the TOML files themselves cannot change them.

### Resource tree

`resource_tree.txt` has one node per line, indented by two spaces per level:

```text
Sprites
  Enemies/
    spr_bat
    spr_slime
  spr_player
Objects
  obj_player
Game Information
```

A top-level line is one of the 12 standard roots, `Name/` is a group and any other
line is a resource of the root's kind, by name. A node that has no short form (an
unusual status or kind, a leaf whose label differs from the resource name, a name with
leading spaces, ...) is spelled out as an inline table:
`{ index = 4, kind = 2, name = "x", status = 3 }`. Lines starting with `#` are
comments.

### Floats, images, sounds

* Floats (timestamps, volumes, path points) are written in the shortest form that reads
  back exactly.
* Images are RGBA PNGs written by the packer; any PNG an editor produces (palette,
  grey, RGB, 16-bit, with or without alpha) is accepted and converted to RGBA8. A frame
  with a zero size cannot be a PNG and is listed in `empty_frames` (`empty_image` for
  backgrounds).
* Sounds and included files are stored byte for byte.

## Editing by hand

* **Add a script:** create `scripts/scr_new.gml`. Without a `.toml` it becomes a new
  script named after the file, in the next free slot, with a leaf in the resource tree.
* **Add an event:** create `objects/obj_x/step.gml` (or any event file name above).
* **Add a sprite / object / room / time line:** create its directory (`sprites/spr_new`
  with `0.png` ..., `objects/obj_new` with event files, ...); the metadata file is
  optional — missing keys take the IDE's defaults, and a resource without `index`
  gets the next free slot. The same holds for `.toml` files of the other kinds: keys
  that are left out take default values.
* **Delete a resource:** delete its files; its slot becomes a deleted slot. Remove its
  line from `resource_tree.txt` as well.
* **Rename:** change `name` in the metadata and every reference to the old name
  (the packer reports each dangling reference with file and key).

Packing reports problems with the file they are in, e.g.
`objects/obj_player/object.toml: key `sprite`: unknown sprite "spr_plyer"` or
`objects/obj_player/stpe.gml: not an event name`.
