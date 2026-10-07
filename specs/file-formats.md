# GameMaker project, library and extension file formats

Clean-room spec. Everything here was derived by byte-level analysis of the sample
data files (and nothing else), plus field naming taken (paraphrased) from the user manual.
No YoYo executable was disassembled, decompiled or debugged.

Samples used:

| File | Format | Result |
|---|---|---|
| `Examples/tutorial1.gmk` | GMK v800 (GM 8.0, zlib blocks) | parsed byte-exactly to EOF |
| `Examples/street racing.gmk` | GMK v701 (GM 7, obfuscated) | decrypted, parsed byte-exactly to EOF |
| `Examples/treasure.gmk` | GMK v701 (GM 7, obfuscated) | decrypted, parsed byte-exactly to EOF |
| `Sprites/Stone1.gmspr` | exported sprite v800 | parsed to EOF |
| `lib/*.lib` (7 files) | action library v520 | all parsed to EOF |
| `extensions/*.ged` (3 files) | extension package v700 | all parsed to EOF |
| `extensions/*.dat` (3 files) | extension payload | de-obfuscated, all chunks inflate |

Reference decoder: `specs/tools/gmk_dump.py` (written from this document).
Decoded action table: `specs/actions.json`.

## 0. Legend and conventions

Confidence markers used throughout:

* **[V]** verified: the field was present in a sample with non-trivial content and the
  whole enclosing structure was consumed exactly (zlib block or file EOF).
* **[S]** structurally verified: the layout/size was consumed exactly, but every sample
  value was 0 / default, so the *meaning* (name, order among same-typed fields) is inferred
  from the IDE's UI order and the manual.
* **[I]** inferred: not present in any sample (e.g. a count was 0). Layout given by analogy
  with neighbouring formats. Implementations must treat it as provisional and fail loudly
  (exact-consumption check) when it does not fit. Each [I] item has an entry in
  `OPEN_QUESTIONS.md`.

### 0.1 Primitive types

All multi-byte values are little-endian.

| Name | Encoding |
|---|---|
| `i32` | 4-byte signed integer |
| `u32` | 4-byte unsigned integer |
| `bool` | `i32`, 0 = false, 1 = true (readers: any non-zero = true) |
| `f64` | 8-byte IEEE-754 double |
| `str` | `u32 n` followed by `n` bytes. No terminator. Bytes are ANSI (Windows code page, in practice Windows-1252); treat as opaque bytes and decode as Windows-1252 by default. Empty string = `00 00 00 00`. |
| `blob` | `u32 n` followed by `n` raw bytes |
| `zblock` | `u32 n` followed by `n` bytes of a zlib stream (RFC 1950, header `78 9C` observed). The inflated bytes form a sub-stream that is parsed with the same primitives and **must be consumed exactly**. |
| `time` | `f64` timestamp in Delphi `TDateTime` convention: days since 1899-12-30 00:00, fraction = time of day (e.g. 40069.4855 = 2009-09-12 11:39). |
| `color` | `i32` in `0x00BBGGRR` order (red in the low byte) e.g. `0xC0C0C0`. Values with the high byte set denote a Windows *system colour*: the observed game-information colour `0xFF000018` (v800) / `0x80000018` (v701) both mean system colour index `0x18` = 24 (info-tip background). Readers should map `(v & 0xFF)` through the system palette when `(v >> 24) != 0`. [V] values, [I] interpretation |

Resource references (sprite, object, ... in actions, rooms, objects) are `i32` *slot indices*
into the corresponding resource list, **including deleted/empty slots**. `-1` = none.

### 0.2 Image pixel formats

* **v800 image** ("img800"): `i32 version (=800)`, `i32 width`, `i32 height`, then if
  `width*height > 0`: `blob pixels` with `len == width*height*4`. Pixels are top-down rows,
  left to right, 4 bytes each in **B, G, R, A** order (verified: wood texture
  bytes `63 AD DE FF` = RGB(222,173,99)). Alpha is real per-pixel alpha. [V]
  (The `width*height == 0 → no blob` rule is [I].)
* **v701 image** ("img701"): `i32 marker`; if `marker == -1` no image [I]; if `marker == 10` [V]:
  `blob` holding a zlib stream whose inflated content is a complete Windows BMP file
  (`BM` header, 24-bit observed, bottom-up as usual for BMP). Transparency in v701 is not stored per pixel;
  see the `transparent` flag of the owning resource (the colour of the bottom-left pixel is the
  transparent colour [I], per manual).

## 1. GMK / GM81 container (v800, and v810 by inference)

```
i32  magic         = 1234321  (0x0012D591)              [V]
i32  version       = 800 (.gmk GM8.0), 810 (.gm81) [810: I]
i32  game_id       (the "unique game id" of the Loading tab) [V]
u8   guid[16]      (random project GUID; not interpreted)    [V]
--- settings ---
i32  settings_version = 800                                  [V]
zblock settings                                              (section 2)
--- triggers ---
i32  triggers_version = 800                                  [V]
i32  trigger_count                                           [V] (0 in sample)
zblock trigger[trigger_count]                                (section 3.1) [I]
time triggers_last_changed                                   [V]
--- constants ---
i32  constants_version = 800                                 [V]
i32  constant_count                                          [V] (0 in sample)
{ str name; str value } [constant_count]                     [I]
time constants_last_changed                                  [V]
--- resources (each: section 4 generic layout) ---
sounds       (list version 800)                              [V]
sprites      (800)                                           [V]
backgrounds  (800)                                           [V]
paths        (800)                                           [V] empty
scripts      (800)                                           [V] empty
fonts        (800)                                           [V] empty
timelines    (800)                                           [V] empty
objects      (800)                                           [V]
rooms        (800)                                           [V]
--- trailing sections (section 6) ---
i32  last_instance_id     (100080 in sample)                 [V]
i32  last_tile_id         (10000000 in sample)               [V]
included files                                               (6.1)
extension packages                                           (6.2)
game information                                             (6.3)
library creation code                                        (6.4)
room order                                                   (6.5)
resource tree                                                (6.6)
<EOF>  (tutorial1.gmk ends exactly here)                     [V]
```

## 2. Settings block (inside the settings `zblock`, version 800)

Byte offsets are within the inflated block. Names follow the Global Game Settings tabs.

| Off | Type | Field | Sample | Notes |
|---|---|---|---|---|
| 0x00 | bool | start_fullscreen | 0 | [S] |
| 0x04 | bool | interpolate_pixels | 0 | [S] |
| 0x08 | bool | dont_draw_border | 0 | [S] |
| 0x0C | bool | display_cursor | 1 | [V] (default on) |
| 0x10 | i32 | scaling | -1 | [V] value; meaning [I]: `>0` fixed scale in percent (100 = none), `-1` keep aspect ratio, `0` full scale |
| 0x14 | bool | allow_window_resize | 0 | [S] |
| 0x18 | bool | always_on_top | 0 | [S] |
| 0x1C | color | color_outside_room | 0 | [S] |
| 0x20 | bool | set_resolution | 0 | [S] |
| 0x24 | i32 | color_depth | 0 | [S] 0 = no change; 1 = 16-bit; 2 = 32-bit [I] |
| 0x28 | i32 | resolution | 0 | [S] 0 = no change; then increasing standard resolutions [I] |
| 0x2C | i32 | frequency | 0 | [S] 0 = no change; then 60,70,85,100,120 [I] |
| 0x30 | bool | dont_show_buttons | 0 | [S] |
| 0x34 | i32 | vsync | 0 | [S] (v810: possibly a flag word, see 7) |
| 0x38 | bool | disable_screensaver | 1 | [V] new in v800 (absent in v702) |
| 0x3C | bool | let_f4_fullscreen | 1 | [V] |
| 0x40 | bool | let_f1_game_info | 1 | [V] |
| 0x44 | bool | let_esc_end_game | 1 | [V] |
| 0x48 | bool | let_f5_save_f6_load | 1 | [V] |
| 0x4C | bool | let_f9_screenshot | 1 | [V] |
| 0x50 | bool | treat_close_as_esc | 1 | [V] |
| 0x54 | i32 | process_priority | 0 | [S] 0 normal, 1 high, 2 highest [I] |
| 0x58 | bool | freeze_on_lose_focus | 0 | [S] |
| 0x5C | i32 | loading_bar | 1 | [V] 0 none, 1 default, 2 custom [I for 0/2] |
| — | — | if loading_bar == 2: `bool has_back; [img] back; bool has_front; [img] front` | — | [I] image encoding assumed to be a `zblock` of image data; unverified |
| 0x60 | bool | show_custom_load_image | 0 | [S] |
| — | — | if show_custom_load_image: `bool exists; [zblock image]` | — | [I] |
| 0x64 | bool | load_image_transparent | 0 | [S] |
| 0x68 | i32 | load_image_alpha | 255 | [V] 0..255 |
| 0x6C | bool | scale_progress_bar | 1 | [V] |
| 0x70 | blob | icon | 22486 bytes | [V] a complete Windows `.ico` file (`00 00 01 00`, 6 images in sample) |
| +0 | bool | display_errors | 1 | [V] |
| +4 | bool | write_errors_to_log | 0 | [S] |
| +8 | bool | abort_on_all_errors | 0 | [S] |
| +12 | bool | treat_uninitialized_as_zero | 0 | [S] |
| | str | author | "" | [V] |
| | str | version (free text) | "100" | [V] |
| | time | info_last_changed | | [V] |
| | str | information | "" | [V] |
| | i32 ×4 | version_major, minor, release, build | 1,0,0,0 | [V] |
| | str ×4 | company, product, copyright, description | "" | [V] |
| | time | settings_last_changed | | [V] |

End of block (exactly consumed in sample). [V]

## 3. Triggers and constants (v800)

### 3.1 Trigger record [I]

Each trigger is its own `zblock`; inflated:

```
bool exists
if exists:
  i32  version (expected 800)
  str  name
  str  condition      (GML; must return a boolean)
  i32  moment         (0 = middle/normal step, 1 = begin step, 2 = end step)
  str  constant_name  (name of the ev_ constant the user chose, may be empty)
```

Trigger *slot index* = event number used by object event type 11 (section 5.8).

### 3.2 Constants [I layout, V framing]

`i32 count` then `count × { str name; str value }` (value is a GML expression string),
then a `time`. In v701 the constant list lives inside the settings block instead (section 8.3).

## 4. Generic resource section

```
i32 list_version   (800 for every list in v800)
i32 count          (number of slots, including deleted ones)
count × zblock:
    bool exists            (0 → deleted/empty slot; block ends here)
    str  name
    time last_changed
    i32  resource_version  (per type, below)
    ... type-specific body (section 5)
```

Every block is self-delimiting; a reader can check exact consumption per resource. [V]

## 5. Resource bodies (v800)

### 5.1 Sound (resource_version 800) [V]

| Type | Field | Sample / notes |
|---|---|---|
| i32 | kind | 0 normal sound, 1 background music, 2 3D sound, 3 use multimedia player ([V] 0 and 1; 2,3 [I] by UI order) |
| str | file_type | extension incl. dot: ".wav", ".mid" |
| str | file_name | original file name: "click.wav" |
| bool | has_data | 1 |
| blob | data | only if has_data. Raw file bytes (RIFF/WAVE, SMF "MThd", ...) — **not** compressed |
| i32 | effects | bit flags [I bit order by UI]: 1 chorus, 2 echo, 4 flanger, 8 gargle, 16 reverb |
| f64 | volume | 1.0 |
| f64 | pan | 0.0 (−1..1) |
| bool | preload | 1 |

### 5.2 Sprite (resource_version 800) [V]

| Type | Field |
|---|---|
| i32 | origin_x |
| i32 | origin_y |
| i32 | frame_count |
| img800 × frame_count | sub-images (0.2) |
| *only if frame_count > 0* [I for the =0 case]: | |
| i32 | collision_shape: 0 precise, 1 rectangle, 2 disk, 3 diamond ([V] 0 and 1; order of 2,3 [I] from UI) |
| i32 | alpha_tolerance (0..255) |
| bool | separate_collision_masks |
| i32 | bbox_mode: 0 automatic, 1 full image, 2 manual [S] |
| i32 | bbox_left [V] |
| i32 | bbox_right [V] |
| i32 | bbox_bottom [V] |
| i32 | bbox_top [V] |

(Order left, right, **bottom**, top is verified against sprite content: e.g. spr_apple 32×32 → 3, 28, 29, 2.)

### 5.3 Background (resource_version 710) [V]

Note the per-resource version is 710 although the list version is 800.

| Type | Field |
|---|---|
| bool | use_as_tileset |
| i32 | tile_width (16) |
| i32 | tile_height (16) |
| i32 | tile_h_offset |
| i32 | tile_v_offset |
| i32 | tile_h_sep |
| i32 | tile_v_sep |
| img800 | image |

### 5.4 Path [I] (list present, no records in any sample)

```
i32  resource_version (expected 530)
i32  connection_kind  (0 straight lines, 1 smooth curve)
bool closed
i32  precision        (1..8)
i32  backdrop_room    (room index shown in the editor, -1 none)
i32  snap_x
i32  snap_y
i32  point_count
point_count × { f64 x; f64 y; f64 speed (percent, 100 = normal) }
```

### 5.5 Script [I] (v800); [V] in v701

```
i32 resource_version (expected 800; 400 observed in v701)
str code
```

### 5.6 Font [I] (v800); [V] core fields in v701

```
i32  resource_version (800 expected; 540 observed in v701)
str  font_name        ("Arial")
i32  size             (points)
bool bold
bool italic
i32  range_start      (32)
i32  range_end        (127)
```
[I] In GM8.x the high bytes of `range_start` are believed to carry extra data (character
set in bits 16..23, anti-aliasing level in bits 24..31); readers should mask
`range_start & 0xFFFF`. Unverified.

### 5.7 Time line [I]

```
i32 resource_version (expected 500)
i32 moment_count
moment_count × { i32 step; action_list }
```
`action_list` = section 5.9.

### 5.8 Object (resource_version 430) [V]

| Type | Field | Notes |
|---|---|---|
| i32 | sprite | index or -1 |
| bool | solid | |
| bool | visible | |
| i32 | depth | |
| bool | persistent | |
| i32 | parent | object index, **-100 = no parent** (observed for every parentless object), ≥0 index |
| i32 | mask_sprite | -1 = same as sprite |
| i32 | max_event_type | 11 in v800 (event types 0..11), 10 in v701 (0..10) |
| event lists | (max_event_type + 1) lists, for type = 0..max_event_type | |

Each event list:

```
repeat:
  i32 event_number      (-1 terminates this type's list)
  action_list           (5.9)
```

Event numbers within a type are written in **descending** order in v701 samples
(e.g. keyboard 40, 39, 38, 37); readers must not assume ordering.

Event types (index of the list) — [V] for 0,2,3,4,5,6,7 (confirmed by content), [I] for others:

| type | meaning | event_number meaning |
|---|---|---|
| 0 | create | 0 |
| 1 | destroy | 0 |
| 2 | alarm | alarm index 0..11 |
| 3 | step | 0 normal, 1 begin, 2 end |
| 4 | collision | object index of the other object |
| 5 | keyboard (held) | Windows virtual-key code (37..40 = arrows observed) |
| 6 | mouse | mouse sub-event (4 = left pressed observed); full numbering in the events spec |
| 7 | other | other sub-event (0 outside room, 2 game start, 6 no more lives observed); full numbering in events spec |
| 8 | draw | 0 |
| 9 | key press | virtual-key code |
| 10 | key release | virtual-key code |
| 11 | trigger | trigger slot index (v800 only) |

### 5.9 Action list and action record [V]

```
i32 action_list_version = 400
i32 action_count
action_count × action:
  i32  action_version = 440
  i32  library_id           (1 = built-in libraries)
  i32  action_id            (unique within library; see actions.json)
  i32  kind                 (copy of the library kind: 0 normal, 1 begin block, 2 end block,
                             3 else, 4 exit, 5 repeat, 6 variable, 7 code)
  bool may_be_relative      (copy of library show_relative)
  bool is_question          (copy of library question flag)
  bool has_applies_to       (copy of library show_apply)
  i32  exe_type             (0 none, 1 call function, 2 run code)
  str  function_name        (e.g. "action_move"; empty for exe_type 0/2)
  str  code                 (library code body; empty for all built-in actions)
  i32  argument_count       (number of meaningful arguments, 0..8)
  i32  arg_kind_count       (always 8 observed)
  i32  arg_kind[arg_kind_count]   (library argument kinds; unused slots 0)
  i32  applies_to           (-1 self, -2 other, >=0 object index)
  bool relative
  i32  arg_value_count      (always 8 observed)
  str  arg_value[arg_value_count] (unused slots contain "0" in v800 and v701 samples)
  bool not                  (negate a question)
```

Argument values are always strings as typed in the dialog; resource arguments store the
resource index in decimal (e.g. "6"), colours store a decimal `color`, booleans "0"/"1",
menus the item index, `font_string` kind stores `"Name,size,bold,italic,..."`
(e.g. "Arial,10,0,1,0,0,0"; the trailing fields are [I]).
Action 101 (move fixed) arg0 is a 9-character `0/1` mask over the 3×3 direction grid in
numeric-keypad order: index 0 = down-left, 1 = down, 2 = down-right, 3 = left, 4 = stop,
5 = right, 6 = up-left, 7 = up, 8 = up-right [V] (treasure.gmk: <Up> key event uses
`000000010` and also tests the position (0,-4); <Down> uses `010000000`; <Left> `000100000`).

The redundant copies of library metadata in each action (kind, flags, function name,
arg kinds) allow the game to run even without the .lib files; the runner should use the
stored values. [V]

### 5.10 Room (resource_version 541) [V]

| Type | Field | Notes |
|---|---|---|
| str | caption | |
| i32 | width | 640 |
| i32 | height | 480 |
| i32 | snap_x | [V] |
| i32 | snap_y | [V] |
| bool | isometric_grid | [S] |
| i32 | speed | room speed, steps/s [V] |
| bool | persistent | [S] |
| color | background_color | 0xC0C0C0 [V] |
| bool | draw_background_color | [V] |
| str | creation_code | GML |
| i32 | background_count | 8 |
| background_count × 10 × i32 | backgrounds | `visible, is_foreground, background_index, x, y, tile_h, tile_v, hspeed, vspeed, stretch` ([V] for visible/index/tile flags; others [S]) |
| bool | views_enabled | |
| i32 | view_count | 8 |
| view_count × 14 × i32 | views | `visible, view_x, view_y, view_w, view_h, port_x, port_y, port_w, port_h, h_border, v_border, h_speed, v_speed, follow_object` (defaults observed: 0,0,0,640,480,0,0,640,480,32,32,-1,-1,-1) |
| i32 | instance_count | |
| instance_count × | instances | `i32 x, i32 y, i32 object, i32 id, str creation_code, bool locked` [V] (ids start at 100001) |
| i32 | tile_count | 0 in all samples |
| tile_count × | tiles | `i32 x, i32 y, i32 background, i32 tile_x, i32 tile_y, i32 width, i32 height, i32 depth, i32 id, bool locked` [I] (ids start at 10000001 [I]) |
| 14 × i32 | editor_state | [S] `remember_room_editor_info, editor_width, editor_height, show_grid, show_objects, show_tiles, show_backgrounds, show_foregrounds, show_views, delete_underlying_objects, delete_underlying_tiles, current_tab, h_scroll, v_scroll` (all 0 in samples) |

## 6. Trailing sections (v800)

### 6.1 Included files [I record layout]

```
i32 version = 800   [V]
i32 count           [V] (0)
count × zblock:
    time last_changed
    i32  version (800)
    str  file_name
    str  original_path
    bool original_file_chosen
    i32  original_size
    bool store_in_editable
    [blob data]                if store_in_editable
    i32  export_kind           (0 don't export, 1 temp dir, 2 game dir, 3 custom folder)
    str  export_folder
    bool overwrite_existing
    bool free_memory_after_export
    bool remove_at_game_end
```
All fields inside the block are [I] (named after the "Include files" dialog).

### 6.2 Extension packages [V framing]

```
i32 version = 700
i32 count
count × str package_name     [I] (names of the .ged packages used, e.g. "GM Room Transitions")
```

### 6.3 Game information [V]

```
i32 version = 800
zblock:
  color  background_color   (0xFF000018 = system info-tip colour)
  bool   show_in_separate_window
  str    window_caption     ("Game Information")
  i32    left               (-1 = default)
  i32    top                (-1)
  i32    width              (600)
  i32    height             (400)
  bool   show_window_border
  bool   allow_resize
  bool   stay_on_top
  bool   pause_game
  time   last_changed
  str    text               (RTF document, "{\rtf1\ansi...")
```

### 6.4 Library creation code [V framing]

```
i32 version = 500
i32 count             (7 = number of libraries loaded when saved)
count × str code      (all empty in samples)
```

### 6.5 Room order [V]

```
i32 version = 700
i32 count
count × i32 room_index     (first entry = starting room)
```

### 6.6 Resource tree [V]

Exactly 12 root nodes follow, each serialised recursively:

```
node:
  i32 status        1 = root (primary) node, 2 = user group, 3 = resource leaf
  i32 group_kind    resource type of the subtree (table below)
  i32 index         resource slot index for leaves; 0 for roots and groups
  str name
  i32 child_count
  node child[child_count]
```

Root order and kinds (identical in v701 and v800 samples):

| # | name | group_kind | status |
|---|---|---|---|
| 1 | Sprites | 2 | 1 |
| 2 | Sounds | 3 | 1 |
| 3 | Backgrounds | 6 | 1 |
| 4 | Paths | 8 | 1 |
| 5 | Scripts | 7 | 1 |
| 6 | Fonts | 9 | 1 |
| 7 | Time Lines | 12 | 1 |
| 8 | Objects | 1 | 1 |
| 9 | Rooms | 4 | 1 |
| 10 | Game Information | 10 | **3** |
| 11 | Global Game Settings | 11 | **3** |
| 12 | Extension Packages | 13 | **3** |

Kind 5 is unused. Groups (status 2) carry the kind of their root and index 0. Leaves of
deleted resources do not appear. File ends right after the 12th root.

## 7. Version 810 (.gm81) — inference only [I]

No .gm81 sample was available. Hypotheses to check as soon as a sample exists:

1. Header `version` = 810 and `settings_version` = 810; everything else as v800 unless noted.
2. The settings `vsync` word may become a bit field (bit 0 = vsync; a higher bit for the
   new 8.1 "hardware T&L / force software vertex processing" switch).
3. 8.1 added more global-setting checkboxes (the 8.1 manual's "what's new" lists room-editor
   and font-rendering changes only, which are IDE state); extra settings fields at the end of
   the settings block are possible. A reader must rely on exact consumption to detect them.
4. Font range_start high bytes (charset / anti-alias) as in 5.6.

The v800 reader in `gmk_dump.py` accepts 810 using the v800 layout and reports a
consumption error if the layout differs.

## 8. Version 701 (GM 7 `.gmk`) — obfuscation and layout

### 8.1 File envelope [V]

```
i32 magic   = 1234321
i32 version = 701
i32 n1                      (880 / 1714 in samples)
i32 n2                      (1803 / 1443)
i32 junk[n1]                (random values < 3000; ignored)
i32 seed                    (26469 / 16411)
i32 junk[n2]
u8  encoded[]               (to EOF)
```

### 8.2 Decoding [V]

1. Build a 256-byte permutation `T` from `seed`:
   ```
   a = 6 + (seed mod 250);  b = seed div 250
   T[i] = i for i in 0..255
   for i = 1 .. 10000:
       j = ((i*a + b) mod 254) + 1          # 1..254
       swap(T[j], T[j+1])
   ```
   (T[0] is never moved.) Let `U` be the inverse permutation (`U[T[x]] = x`).
2. For every encoded byte at **absolute file offset** `p`:
   `plain = (U[byte] - p) mod 256`.
   (Equivalently the encoder wrote `T[(plain + p) mod 256]`.) Offset base verified: the two
   samples start the encoded area at 0x2A00 and 0x3168 and both decode with the absolute offset.
3. The decoded stream starts with `game_id`. Open question: whether the very first encoded
   byte is stored raw (the extension `.dat` format stores its first payload byte raw, see
   10.3). Both choices give a plausible `game_id`; only its low byte is affected.

The table was first recovered statistically (assuming mostly-zero plaintext per position
mod 256), and then confirmed to match the seed-driven construction exactly in both samples.

### 8.3 Decoded stream (differences from v800)

Overall order is the same as v800 **without**: zlib blocks, `last_changed` timestamps,
the triggers and constants sections. Concretely:

```
i32 game_id; u8 guid[16]
i32 settings_version = 702
settings (uncompressed, inline):
    same as v800 table but WITHOUT disable_screensaver (22 i32 before loading_bar);
    ... icon blob, 4 error flags, author, version str, time info_last_changed, information,
    i32 constant_count; constant_count × {str name; str value}     [S] (count 0)
    4 × i32 version numbers, 4 × str (company..description)
    (no trailing time)
i32 400 sounds list:   per slot: bool exists; str name; i32 version=600; i32 kind; str file_type;
                       str file_name; bool has_data; blob data (zlib-compressed file bytes!);
                       i32 effects; f64 volume; f64 pan; bool preload
i32 400 sprites list:  per slot: bool exists; str name; i32 version=542;
                       i32 width, height, bbox_left, bbox_right, bbox_bottom, bbox_top;
                       bool transparent, smooth_edges, preload; i32 bbox_mode; bool precise;
                       i32 origin_x, origin_y; i32 frame_count; img701 × frame_count
i32 400 backgrounds:   per slot: bool exists; str name; i32 version=543; i32 width, height;
                       bool transparent, smooth_edges, preload, use_as_tileset;
                       i32 tile_w, tile_h, h_off, v_off, h_sep, v_sep;
                       bool has_image; img701 if has_image
i32 420 paths:         (count 0 in samples) [I]
i32 400 scripts:       per slot: bool exists; str name; i32 version=400; str code   [V]
i32 540 fonts:         per slot: bool exists; str name; i32 version=540; str font; i32 size;
                       bool bold, italic; i32 range_start, range_end               [V]
i32 500 timelines:     (count 0) [I]
i32 400 objects:       per slot: bool exists; str name; i32 version=430; body as 5.8
                       with max_event_type = 10                                    [V]
i32 400 rooms:         per slot: bool exists; str name; body as 5.10 (version 541) [V]
i32 last_instance_id; i32 last_tile_id
i32 620 included files; i32 count (0) [I record]
i32 700 extension packages; i32 count; str names
i32 600 game information (uncompressed): same fields as 6.3 but without last_changed
i32 500 library creation code (as 6.4)
i32 700 room order (as 6.5)
resource tree (as 6.6, 12 roots)
<EOF>   [V for both samples]
```

Deleted slots are common in v701 samples (a slot is just `bool exists = 0`).

## 9. Exported resource files (`.gmspr`) [V, one sample]

```
i32 magic = 1234321
zblock: i32 version (800), then the v800 sprite body from 5.2 (origin_x onward)
        — no exists flag, no name, no timestamp.
```
By analogy other export types (.gmbck, .gmobj, .gmrom, ...) are presumed to follow the same
pattern with their v800 bodies [I].

## 10. Action libraries (`lib/*.lib`) [V, all 7 files]

### 10.1 File

```
i32  format_version = 520
str  tab_caption           ("move", "main1", ...)
i32  library_id            (1 for all built-in libs; ids <1000 reserved for built-ins)
str  author
i32  library_version       (600 / 710)
time last_changed
str  info                  (free text shown in the library maker)
str  init_code             (GML run at game start; empty in all built-in libs)
bool advanced_only         (whole tab only shown in advanced mode; 1 for "extra")
i32  editor_counter        (opaque library-maker counter; ignore)
i32  action_count
action_count × action
<EOF>
```

### 10.2 Action definition

```
i32  version = 520
str  name                  (identifier-like, e.g. "Set_Motion"; palette labels for kind 10)
i32  action_id             (999 for palette decorations)
blob image                 (always a 4150-byte 32×32 32bpp BMP file)
bool hidden                (legacy action kept only for loading old games)
bool advanced_only
bool pro_only              ("registered" edition only)
str  palette_title         (short tooltip title, e.g. "Move Fixed")
str  list_text             (template shown in the event's action list)
str  hint_text             (template for the long hint)
i32  kind                  (see below)
i32  interface             (0 normal dialog, 1 no dialog, 2 arrow grid (move fixed), 5 code editor;
                            3/4 not observed)
bool is_question
bool show_apply_to
bool show_relative
i32  argument_count        (0..8 used)
i32  arg_slot_count        (always 8)
arg_slot_count × { str caption; i32 kind; str default; str menu_items ("a|b|c") }
i32  exe_type              (0 none, 1 function, 2 code)
str  function_name
str  code
```

`kind` values: 0 normal, 1 begin block, 2 end block, 3 else, 4 exit, 5 repeat, 6 set
variable, 7 code, 8 palette placeholder (empty slot), 9 palette separator, 10 palette label.
Kinds 8–10 never appear in games (they only lay out the palette).

Argument kinds: 0 expression, 1 string, 2 string-or-expression ("both"), 3 boolean,
4 menu, 5 sprite, 6 sound, 7 background, 8 path, 9 script, 10 object, 11 room,
12 font, 13 color, 14 time line, 15 font description string.

Template placeholders in list/hint text (interop for the action-list UI):
`@0`..`@7` argument value; `@r` → "relative " text when relative is set;
`@w` → applies-to prefix ("for other: " / "for all X: ") when not self;
`@N` → "not " when negated; `@FI` / `@FB` → render the line italic / bold;
`#` → line break. Unused menu captions contain the editor placeholder "item 1|item 2"
and must be ignored unless kind == 4.

The full decoded table (137 real actions, palette layout, argument kinds/defaults/menus,
flags, function names and own-words summaries) is in `specs/actions.json`. No built-in action
has a GML body (`code` empty everywhere): `exe_type 1` actions call the named runtime function
`action_*` with the argument values; `exe_type 2` actions are 603 Code (arg0 is GML to execute)
and 611 Set Variable (assign arg1 to the variable named in arg0, or add when relative);
`exe_type 0` actions (blocks, else, exit, repeat, comment) are interpreted structurally.

## 11. Extension packages

### 11.1 `.ged` package description [V, all 3 files parsed to EOF]

```
i32  version = 700
bool editable
str  name                 ("GM Room Transitions")
str  folder               (install folder name, e.g. "GMTrans123")
str  version_text         ("1.1")
str  author
str  date
str  license
str  description
str  help_file            (.chm shipped alongside)
bool hidden
i32  uses_count; str uses[uses_count]       (dependencies; 0 observed)
i32  file_count
file_count × file:
   i32  version = 700
   str  file_name          ("Transitions.gml", "Printing.dll")
   str  original_path      (author's build path; ignore)
   i32  file_kind          (1 = DLL, 2 = GML [V]; 3 = action library, 4 = other [I])
   str  init_function      (called at game start, may be empty)
   str  final_function     (called at game end, may be empty)
   i32  function_count
   function_count × function:
       i32  version = 700
       str  name           (GML-visible name)
       str  external_name  (DLL export name; empty = same as name)
       i32  call_convention(observed 12 for DLL files, 11 and 2 for GML files; 12 presumed stdcall,
                            11 cdecl [I])
       str  help_line      (signature shown in code completion)
       bool hidden         (not shown/usable directly; helper)
       i32  argument_count (-1 = variable? [I]; 0..10 observed)
       i32  arg_type[17]   (1 = string, 2 = real; unused slots 2)
       i32  return_type    (1 = string, 2 = real)
   i32  constant_count
   constant_count × { i32 version=700; str name; str value; bool hidden }
<EOF>
```

### 11.2 `.dat` payload [V]

```
i32 seed
u8  encoded[]   (to EOF)
```
De-obfuscation: build `T`/`U` from `seed` exactly as 8.2, then
`plain[k] = U[encoded[k]]` with **no** position term, **except** `plain[0] = encoded[0]`
(the first payload byte is stored raw — verified on all three files; it is the low byte of the
first chunk length).

The plain payload is a concatenation of one chunk per `.ged` file entry, in order:
`{ u32 n; u8 zlib_stream[n] }`. Each inflates to the file's bytes (a DLL image or GML text).
GML files are a sequence of `#define name` sections, one per script/function.

## 12. Summary of per-record version numbers

| Record | v701 | v800 |
|---|---|---|
| header | 701 | 800 (810 for .gm81 [I]) |
| settings | 702 | 800 |
| triggers list / trigger | — | 800 / 800 [I] |
| constants | (in settings) | 800 |
| sounds list / sound | 400 / 600 | 800 / 800 |
| sprites list / sprite | 400 / 542 | 800 / 800 |
| image (frame) | marker 10 | 800 |
| backgrounds list / background | 400 / 543 | 800 / 710 |
| paths list / path | 420 / ? | 800 / 530 [I] |
| scripts list / script | 400 / 400 | 800 / 800 [I] |
| fonts list / font | 540 / 540 | 800 / 800 [I] |
| timelines list / timeline | 500 / ? | 800 / 500 [I] |
| objects list / object | 400 / 430 | 800 / 430 |
| action list / action | 400 / 440 | 400 / 440 |
| rooms list / room | 400 / 541 | 800 / 541 |
| included files | 620 | 800 |
| extension packages | 700 | 700 |
| game information | 600 | 800 |
| library creation code | 500 | 500 |
| room order | 700 | 700 |
| .lib file / action | 520 / 520 | — |
| .ged / file / function / constant | 700 | — |

## Addendum: verification against a real 810 sample

`treasure.gm81` (saved by the GM 8.1 IDE from the 701 `treasure.gmk`) was decoded
end-to-end by `specs/tools/gmk_dump.py` with exact consumption to EOF. Observed:

- Header version is 810; every section/record below it uses the 800 layout unchanged.
- Settings version 800; triggers and constants sections present (version 800, empty).
- Section versions: sounds/sprites/backgrounds/paths/scripts/fonts/timelines/objects/rooms
  = 800; included files 800; game information 800; extension packages 700;
  library creation code 500; room order 700.
- Record versions: sound 800, sprite 800, background 710, script 800, object 430,
  action 440, room 541.
- So the 810 differences previously marked [I] are confirmed as "none beyond the header
  version" for every section present in this sample. Paths, fonts, timelines, triggers,
  constants, tiles and included files remain unverified (no instances in this sample).
