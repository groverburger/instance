# Open questions

Append-only. Each agent adds its own section.

## IDE (specs/ide.md)

Items the manual does not settle; resolve by black-box observation of the IDE and
of the files it writes.

- IDE-1: Keyboard shortcuts for every menu command (Create Sprite/Sound/... , Run,
  Debug, Save, etc.). Manual only says each create command has one and that 8.0 moved
  Ctrl+Alt combos to Shift+Ctrl.
- IDE-2: Default resource name numbering (`sprite<N>`): is N the new id, or the first
  unused number?
- IDE-3: Placement of a new resource created via Resources menu / toolbar (end of
  kind root, or inside the currently selected group?) and of Duplicate (after the
  source? at end?). Placement of Add Group when a group is selected.
- IDE-4: Sprite form "Precise collision checking" checkbox vs mask shape: does
  unchecking set shape=Rectangle, and how do the two interact in the saved file?
- IDE-5: Which pixel defines the background colour for "Remove Background" when
  loading sprites/backgrounds (manual only states bottom-left for the loading image).
- IDE-6: Path precision valid range (screenshot shows 4) and whether the path's
  backdrop room and snap values are stored in the project.
- IDE-7: Font "Letters" button exact range; whether Digits is 48-57.
- IDE-8: Event list display order and exact labels for each event (keyboard key
  names, mouse/joystick/view events), and keyboard key submenu grouping in the event
  selector.
- IDE-9: Action list: exact right-click menu items; whether Cancel on the argument
  dialog of a freshly dropped action removes it; how applies-to/relative/NOT are shown
  in the one-line description.
- IDE-10: How the Move Fixed direction-grid (and Move Free/other special widgets)
  map to stored argument strings — belongs to the action-library spec.
- IDE-11: Room editor "Show" dropdown items; status-bar content; exact right-click
  instance/tile menu items in 8.1.
- IDE-12: Global Game Settings: which options are hidden in simple mode; Resolution
  tab option lists (resolutions, frequencies, colour depths) and whether vsync is
  independent of "Set the resolution"; Errors tab — is there an 8.1-only extra option
  (e.g. about uninitialized arguments)?
- IDE-13: Run normally: are open forms with uncommitted changes included/committed
  before running? Working directory when the project was never saved? Is the IDE
  blocked while the game runs with "hide designer" off?
- IDE-14: Debug window: complete Run/Watch/Tools menu item list and watch-file format.
- IDE-15: Backup rotation order for `.gb0`..`.gb9`.
- IDE-16: `.gml` script library format (assumed `#define name` separators),
  `.gmres`, `.gmspr`, `.gmbck`, `.gmtrg` file formats (not documented in the manual).
- IDE-17: Is Save Sound hidden in simple mode? (Simple-mode sound form shows Load,
  Play and Save per the 1xx chapter — confirm.)
- IDE-18: Show Object Information / Show Information text format (cosmetic; we may
  define our own).
- IDE-19: Behaviour when a referenced resource (sprite of an object, object of an
  instance, trigger of an event) is deleted: how is it displayed and saved (-1)?

## runtime.md (runner behaviour) — needs black-box verification

- RT-01: Exact intra-step position of: xprevious/yprevious capture, timeline
  advance, particle auto-update, view-follow update, image_index advance +
  Animation End (before or after Draw?), background scrolling. (runtime.md §4.2)
- RT-02: After room_goto/game_end/etc. inside an action, do the *remaining actions
  of the same event* still run? Are remaining instances' events of the same phase
  skipped? (§3.1)
- RT-03: Is the room creation code re-run when re-entering a visited persistent
  room? Do restored instances get Room Start? (§3.2)
- RT-04: Does game_end() run Room End before Game End? What exactly does
  game_restart() run (Game End? Room End?) and reset (score/lives/health, globals,
  instance id counter)? (§3.3)
- RT-05: Outside Room / Intersect Boundary (and view variants): fired every step
  while the condition holds, or only on the transition? Behaviour for instances
  without sprite/mask; does Intersect Boundary also fire when completely outside?
  (§5.5)
- RT-06: Solid collision handling: is the put-back triggered by other.solid only,
  or by either instance being solid, and are both instances moved back/forward?
  Exact "move to new position" rule; order of pair iteration. (§8.6)
- RT-07: Collision-event selection with parents when an object has collision
  events with both T and T's ancestor, or inherits one from its parent. (§6.3)
- RT-08: Alarm semantics: value after firing (-1 or 0), fractional alarms,
  whether an alarm ticks if only a parent defines the event, order within an
  instance. (§5.3)
- RT-09: Are instances created during an event phase visited in the same phase?
  (§4.3)
- RT-10: Speed/direction coupling edge cases: negative speed, direction
  normalisation, direction preserved at zero speed; friction/gravity order. (§7.1, §7.4)
- RT-11: bbox_* integer rounding rules with scaling/rotation; alpha tolerance
  comparison (> vs >=). (§8.1, §8.2)
- RT-12: No More Lives / No More Health: checked at assignment or once per step?
  Health edge rule. Default values of lives, health, show_score/show_lives/
  show_health and the caption format. (§1.1, §5.4, §12.1)
- RT-13: Keyboard event dispatch nesting (key-major vs instance-major) and order of
  held / press / release events relative to each other. (§11.3)
- RT-14: mouse_x/mouse_y with overlapping or no views under the cursor; whether
  mouse-over tests for instance mouse events are precise. (§11.5, §12.3)
- RT-15: View follow algorithm details (uses instance x/y or bbox? clamped to room?
  update time). (§12.2)
- RT-16: Draw tie order at equal depth between tiles, instances and particle
  systems; where cursor_sprite is drawn. (§13.1, §13.2)
- RT-17: Path orientation/scale pivot; endaction 1/2 overshoot handling; timeline
  looping/end semantics and negative speed. (§10)
- RT-18: Numeric values of ev_* / ev_other / ev_mouse constants, self/other/all/
  noone/global and mb_* constants (need the constants spec or file-format
  observation to confirm). (§1.4, §5.1, §11.2)
- RT-19: Persistent object placed in a room that is (re)entered while an instance
  of it already exists: duplicated? (§3.2 step 6)
- RT-20: Which runtime errors are fatal vs ignorable; behaviour of Ignore
  (skip action vs skip event). Default F5/F6 save-file name. (§19, §3.5)
- RT-21: instance_change: which properties come from the new object (sprite,
  mask, depth, solid, visible, persistent)? (§5.7)

## [GML] Language (from specs/gml-language.md, 2026-10-07)
Each item already has a recommended choice in the spec. They need black-box confirmation.
1. Truthiness at exactly 0.5. The manual gives both ">= 0.5 is true" and "rounded value <= 0 is false". The spec uses `v >= 0.5`. How do strings behave in conditions? The spec treats them as false.
2. Mixed real/string comparison (`"a" == 1`, `"a" < 1`): is it a runtime error, or false? The spec makes it false with no error.
3. Real equality: is it exact, or is there an epsilon? The spec uses exact comparison.
4. Rounding mode of the integer conversion (`to_int`) used for repeat counts, array indices, bitwise operands and the left side of `.`: is it half-to-even, half-away, or truncation? Which integer width do bitwise ops use (32 or 64 bit)?
5. `string(x)` formatting for non-integers, negative zero and large magnitudes. Is it always 2 decimals?
6. Division by zero for `/`, `div` and `mod`: is it an error, and what is the result if the error is ignored?
7. Array storage model: is `a` the same as `a[0]` and `a[0,0]`? Is `a[i]` the same as `a[0,i]`? Do unwritten in-range elements read as 0, or as an uninitialised-variable error?
8. Do object references in `.` and `with` include instances of child objects?
9. `with`: is the iteration a snapshot? How are instances created or destroyed during the loop handled?
10. What is `other` in non-collision events, especially the Create event run by `instance_create`, and `event_perform`?
11. Unpassed script arguments (`argumentN` with N >= argument_count): do they read as 0, or raise an error? Is `argument_count` writable?
12. Is `var` scoping lexical (compile time) or does it start when the `var` statement executes? Does `var` shadow built-in variables?
13. What do the `local` constant (assumed -7) and `local.x` mean?
14. Are `%`, `:=` (inside expressions), a leading-dot number (`.5`), and `12.` (trailing dot) accepted?
15. Is `continue` outside a loop a compile error, or does it act like `exit`? Is a bare `return;` legal?
16. What happens when the wrong type is assigned to a built-in variable (e.g. `x = "a"`)? Does reading an instance-less `obj.var` raise an error?
17. Error recovery: after a non-fatal error is ignored, does the rest of the code action continue, or is it abandoned?
18. Exact runtime and compile error message texts.
19. The values in builtins.json with `value_source: "inferred"` all need confirming, e.g. by printing each one with `show_message(string(c))` in GM 8.1. Values are unknown (null) for: os_*, device_ios_*, text_type, button_type, input_type.
20. Resolution priority when a script name clashes with a built-in function, or a user constant with a resource name.

## [FF] File formats (specs/file-formats.md, specs/actions.json, 2026-10-07)

Samples were one v800 .gmk, two v701 .gmk, one .gmspr, 7 .lib, 3 .ged/.dat. Everything
below was **not** present in any sample (counts were 0, or no .gm81 existed). The fix in
every case is to make small sample projects in the IDE (black-box) and diff/parse them.

- FF-1: No .gm81 (v810) sample. Confirm header/settings version 810 and every per-record
  version; check whether settings v810 adds fields (a vsync/flags word, 8.1-only options)
  or whether any resource body changes (fonts, rooms). (spec §7)
- FF-2: Trigger record layout (v800) — trigger count was 0. (§3.1)
- FF-3: Constant list entry layout (name/value strings) in v800 top level and in v702
  settings — counts were 0; also confirm the constant count's position inside v702 settings. (§3.2, §8.3)
- FF-4: Path record (v800 version 530?, v701) — no paths in any sample. (§5.4)
- FF-5: Script record in v800 (only v701 seen) and font record in v800 (only v701 seen);
  font `range_start` high bytes (charset / anti-alias) in v800/v810. (§5.5, §5.6)
- FF-6: Time line record — none in samples. (§5.7)
- FF-7: Tile record in rooms (10 fields assumed) — no tiles in any sample. Room
  `editor_state` 14 ints: count verified, field order/meaning not (all zero). (§5.10)
- FF-8: Included-file record (v800 zblock / v701 version 620) — none in samples. (§6.1)
- FF-9: Extension-package list entries (assumed `str` names) — count 0 in samples. (§6.2)
- FF-10: Custom loading bar images (loading_bar == 2) and custom load image encoding
  inside settings — not present. (§2)
- FF-11: Sprite with 0 frames (are the 8 mask fields omitted?), img800 with width or
  height 0 (is the pixel blob omitted?), sound with has_data = 0 — not present. (§0.2, §5.1, §5.2)
- FF-12: Enum meanings verified only for observed values: sound kind 2/3, sound effect
  bit order, collision shapes 2/3, bbox_mode, scaling sign convention, colour depth /
  resolution / frequency / priority indices. (§2, §5)
- FF-13: v701 decoding: is the first encoded byte (low byte of game_id) stored raw, as in
  the extension .dat format, or decoded like the rest? (§8.2)
- FF-14: .ged call_convention values (11/12 on DLL vs GML files, 2 on one GML file):
  confirm 12 = stdcall, 11 = cdecl; file_kind 3/4; argument_count -1 for varargs. (§11.1)
- FF-15: Action library interface kinds 3 and 4 and argument kinds beyond 15 never occur
  in the built-in libs; meaning unknown (only matters for user libraries). (§10.2)
- FF-16: Action argument value encoding for kind 15 (font string) — only
  "Arial,10,0,1,0,0,0" seen; meaning of the trailing fields. (§5.9)
- FF-17: Event sub-numbers for mouse/other/step beyond the values observed (left
  pressed = 6:4, outside room = 7:0, game start = 7:2, no more lives = 7:6, end step
  3:2 presumed) — should be settled in the events/runtime spec (relates to RT-18).
- Partially answers IDE-10: Move Fixed arg0 is a 9-char 0/1 mask over the 3×3 grid in
  numeric-keypad order (index 0 = down-left, 4 = stop, 7 = up; verified in samples). IDE-16: `.gmspr` format
  is now in §9; `.gmbck` etc. presumed analogous.

## [IMPL-FMT] gm-format implementation notes (2026-10-07)

Observations made while implementing `crates/gm-format` against the sample files
(treasure.gmk v701 vs treasure.gm81, which the 8.1 IDE saved from it):

- FF-13 resolved (proposed): the **first encoded byte of a v701 stream is stored raw**,
  like the `.dat` payload. Decoding it raw gives game id 308668 for treasure.gmk, equal to
  the id stored in treasure.gm81; decoding it like the other bytes gives 308683.
  `gmk_dump.py` still decodes it (affects only the low byte of game_id).
  Spec §8.2 should be updated.
- v701 sprite/background conversion (spec §0.2 "[I]" transparency) confirmed and extended:
  with `transparent` set, every pixel whose RGB equals the bottom-left pixel becomes
  alpha 0 (RGB kept); with `smooth_edges` additionally set, every remaining opaque pixel
  loses 32 alpha per alpha-0 pixel among its 8 neighbours (out-of-image neighbours do not
  count). This reproduces the RGBA of all treasure.gm81 sprite frames exactly.
  `precise = 1` → collision_shape 0, else 1; the IDE sets `separate_collision_masks = 1`
  on converted sprites; alpha_tolerance 0; bbox values copied. Please add to spec §8.3.
- v701 settings lack `disable_screensaver`; the converted .gm81 has it = 1 (implementation
  uses true). All other settings/rooms/objects/actions/tree match field-for-field.
- Writers emit settings_version 800 for both v800 and v810 (per the spec addendum).
- tutorial1.gmk and treasure.gm81 re-written by gm-format have byte-identical inflated
  block contents (only zlib compression differs), so every per-record version number in
  §12 for those records is confirmed.
- Not covered by any sample and therefore still per-spec [I]: trigger, constant, path,
  font (v800), timeline, tile and included-file records; loading-bar / custom load images
  (stored as opaque blobs; only written when loading_bar == 2 / show_custom_load_image);
  0-frame sprites (mask fields omitted) and 0-size images (no pixel blob).
- Model choices worth reviewing: v701 `smooth_edges`/`preload` flags and the v701 image
  width/height are not kept in the model (converted as above); font `range_start` is split
  into range_start (low 16 bits) / charset (bits 16-23) / antialias (bits 24-31) and
  recombined on write.

## [IMPL-GML] crates/gml implementation choices (2026-10-07)

Choices made where specs/gml-language.md or builtins.json are silent. Each is easy to
change; please confirm or correct.

Language / interpreter
- G-1: Variadic minimum argument count = number of listed args minus 2 (fnames lists two
  illustrative trailing args): `choose/min/max/mean/median/execute_string/execute_file/
  script_execute/external_call` need >= 1, `external_define` >= 5. Max is 16.
- G-2: `argument_count` is read-only (spec recommendation), although builtins.json marks it
  writable. `argumentN[k]` with k > 0 is an "Unknown variable" error.
- G-3: `global.<name>` where `<name>` is an *instance* built-in (e.g. `global.x`) is a user
  global variable called `x`; `global.<global built-in>` is the built-in.
- G-4: Built-in array variables accept the 2-D form only with first index 0
  (`alarm[0,3]` = `alarm[3]`); a non-array built-in accepts only index 0.
- G-5: Assignment evaluation order: owner, indices, (current value for `op=`), then RHS.
  For `obj.v op= e` the owner/indices are evaluated once; the read uses the first
  instance and the result is written to all instances.
- G-6: `local.name` without a `var` declaration creates a hidden local slot for `name`;
  a run-time owner value of -7 (`o = local; o.x`) only finds such slots/declared locals.
- G-7: Nested program executions (script calls, execute_string, host re-entry) are
  limited to `Settings::max_call_depth` = 256 by default (not 10 000) because each level
  uses native stack; the runner may raise it when running on a big-stack thread.
- G-8: Parser nesting limit: 160 levels of statements/parentheses ("Code is nested too
  deeply."). `for` header semicolons are optional, as in the grammar.
- G-9: Errors: compile errors for a built-in function name used without brackets (read or
  assigned) and for indexing a constant. Owners are converted with to_int and saturated
  to the i32 range. Reading `obj.v` when the object has no instance, `noone.v`, or a
  variable of a non-existent instance id is an error; the corresponding writes are no-ops.
  Assigning a user variable through implicit `self` when self does not exist is an error.
- G-10: `with` skips snapshot targets (other than the first, and other than self/other)
  for which `Host::instance_exists` is false at their turn.
- G-11: `string()` of NaN/inf gives "nan"/"inf"/"-inf"; a non-integral value that rounds
  to zero prints without a minus sign ("0.00"); two-decimal rounding is half-away-from-zero
  on the exact binary value (2.675 -> "2.67", 0.125 -> "0.13").
- G-12: `execute_string` caches compiled code by source text (256 entries).
  `execute_file` reads the file through `Host::read_file` (default: std::fs, relative to
  the current directory).
- G-13: Built-in constants whose value is null in builtins.json get distinct values in
  listing order within their family: os_win32..os_android = 0..5,
  device_ios_iphone/_retina/ipad = 0..2, text_type/button_type/input_type = 0..2.

Library (chapter 402)
- G-20: Math functions raise "Invalid argument to function f." when finite inputs give a
  non-finite result (sqrt(-1), ln(0), log of <= 0, arcsin(2), power(0,-1), exp(1000)).
- G-21: `round` uses ties-to-even (like to_int). `sign(0)` = 0. `frac(-3.25)` = -0.25.
- G-22: `real(str)` parses the longest numeric prefix after leading whitespace
  (`[+-]digits[.digits][e[+-]digits]`), 0 if none; `real(real)` returns the value and
  `string(string)` returns the string. String functions given a real (and vice versa)
  raise "Wrong type of arguments to function f.".
- G-23: String positions are 1-based byte positions rounded with to_int. `string_copy`/
  `string_delete` clamp index < 1 to 1 and count to the end; `string_char_at` out of
  range gives ""; `string_insert` past the end appends; an empty substring is never
  found (`string_pos` 0, `string_count` 0, replace is a no-op); `string_count` counts
  non-overlapping matches. `chr(v)` = the single byte `v mod 256`; `ord("")` = 0.
- G-24: `string_lower/upper` map ASCII and the Latin-1 letter ranges (0xC0-0xDE <->
  0xE0-0xFE, except 0xD7/0xF7); `string_letters/digits/lettersdigits` keep ASCII only.
- G-25: `min/max` accept all-real or all-string arguments (ordinal string order); mixed is
  an error. `median` of an even count is the lower middle value.
- G-26: Random generator: SplitMix64, seed 0 at VM creation (the runner calls randomize()
  at game start per runtime.md §2). `random_get_seed` returns the last seed set.
  `irandom(x)` uses floor(x); `irandom_range` floors both bounds and accepts them in
  either order. Sequences are not GM-compatible.
- G-27: Dates: day 0 = 1899-12-30, fraction = time of day. `date_get_week` = ISO-8601
  week; `date_get_weekday` 1 = Sunday .. 7 = Saturday (matching current_weekday in
  runtime.md); spans are absolute, year = 365.25 days, month = 30.4375 days;
  `date_create_*` with invalid parts return 0; valid years 1..9999; the `*_string`
  functions use ISO formats "yyyy-mm-dd", "hh:mm:ss", "yyyy-mm-dd hh:mm:ss" instead of the
  system locale. Current time = OS local time (Windows), UTC elsewhere.
- G-28: Clipboard functions are not pure; they are delegated to the host.

Data structures (chapter 411)
- G-30: Each kind has its own id space starting at 0; the lowest free id is reused after a
  destroy. Using a non-existent id is a runtime error.
- G-31: `ds_set_precision` default 1e-7; it applies to real comparisons in list find,
  map keys, priority values and grid value searches.
- G-32: Maps are sorted multimaps (reals before strings, strings ordinal). `ds_map_add`
  with an existing key adds a duplicate; replace/delete/find act on the first match.
  Missing keys, empty pops/heads, `find_first/last/next/previous` with no answer and
  out-of-range list/grid reads return 0. `ds_list_sort` orders reals before strings.
- G-33: Out-of-range `ds_list_insert/replace/delete` and `ds_grid_set*` are ignored.
  Grid coordinates are rounded with to_int; regions are normalised (corners swapped)
  and clipped; a disk contains cells whose centre (integer coordinates) is within
  distance r of (xm, ym). `ds_grid_add` with mismatched types and `ds_grid_multiply` on
  strings leave the cell unchanged. Aggregates ignore string cells (0 if none).
  `ds_grid_value_x/y` return -1 when not found; search order is row by row.
- G-34: `ds_*_write` produce an implementation-defined hex text format (not compatible
  with files written by GM 8.1); `ds_*_read` of a malformed string is a runtime error.
