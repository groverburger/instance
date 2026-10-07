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

## [IMPL-IDE] crates/gm-ide implementation choices and spec gaps (2026-10-07)

Choices made where specs/ide.md is silent; please confirm or correct.

- IMPL-IDE-1 (action list text): `actions.json` omits list/hint templates, so action
  rows show "palette title: arg, arg" (resource args by name, menu args by caption,
  booleans true/false), prefixed "for other:" / "for all <obj>:" and "NOT ", suffixed
  "(relative)". The code action shows its `///` label; comment (lib 1, id 605) is
  italic. Need the template token rules (`@0`, `@r`, `@w`, ...) plus template text, or
  a paraphrased template per action, to match the original rows.
- IMPL-IDE-2 (Move Fixed grid): arg0 edited as a 3x3 toggle grid using the
  keypad-order mask from actions.json (rows top→bottom = indices 6-8, 3-5, 0-2).
- IMPL-IDE-3 (smooth path drawing): the editor draws smooth paths as a quadratic
  B-spline through edge midpoints (open paths start/end at the end points),
  2^precision subdivisions per segment. The real curve formula is not in the spec
  and matters for the runner (path_* positions) too.
- IMPL-IDE-4 (timestamps): `last_changed` values are written as UTC Delphi
  TDateTime (unix/86400 + 25569). Is the original local time?
- IMPL-IDE-5 (empty events): events without actions are dropped on OK only if they
  were added in this form session; pre-existing empty events in a loaded file are
  kept so that open→OK is lossless. Spec §13.1 says "removed when committed".
- IMPL-IDE-6 (automatic bbox): with bbox mode Automatic / Full image the stored
  bbox_left/top/right/bottom are recomputed only when frames, tolerance or mode
  change in the form (tight box of alpha > tolerance over all frames; empty image →
  full image). Is the stored bbox for automatic mode the union over frames?
- IMPL-IDE-7 (new resource placement / naming): Resources menu creates into the
  selected group of that kind, else at the end of the root; Duplicate inserts right
  after the source and names the copy `<name>_copy`; default names use N = new id,
  bumped while the name is taken (IDE-2/IDE-3 still open).
- IMPL-IDE-8 (room_order): recomputed from the room tree order whenever rooms are
  created/deleted/moved; all samples have room_order == tree order. Unchanged
  otherwise, so open→save stays lossless.
- IMPL-IDE-9 (trigger slots): a new trigger reuses the first empty slot (slot index =
  event number); deleting trims trailing empty slots. Confirm the IDE's behaviour.
- IMPL-IDE-10 (game information): edited as plain text (P0 stopgap). On change the
  RTF prefix up to the first body text/\par (font table, colour table, initial
  formatting words) is kept and the body is regenerated (`\par` per line, cp1252
  `\'hh` escapes). Unchanged text keeps the RTF byte-identical.
- IMPL-IDE-11 (instance/tile ids on duplicate): duplicating a room renumbers its
  instances/tiles from the project allocators so ids stay unique (spec §15.4).
  Whether the original IDE does the same is unverified.
- IMPL-IDE-12 (resolution/frequency lists): the Resolution tab exposes `resolution`
  and `frequency` as raw indices because the option lists are not specified.
- IMPL-IDE-13 (runner invocation): Run writes `<temp>/gm81-clean-ide/<pid>/game.gm81`
  and starts `gm-runner <file>` with the project folder as working directory (temp
  folder for unsaved projects). No secure-mode/debug flags are passed yet: the runner
  CLI for those needs to be specified.
- IMPL-IDE-14 (rendering backend, not a spec issue): eframe's default wgpu backend
  crashed with an access violation on the development machine; the IDE uses the glow
  (OpenGL) backend.

## [IMPL-RUN] crates/gm-runner implementation choices (2026-10-07)

Choices made where specs/runtime.md, actions.json or builtins.json are silent or
leave a recommendation open. Each is easy to change; please confirm or correct.

Loop / events
- R-1: Step order exactly as runtime.md §4.2 (recommended positions for xprevious,
  timelines, particles, view update, animation after draw). A pending room change is
  checked after every event dispatch and aborts the rest of the step (no draw).
  Events raised synchronously from code (No More Lives/Health, Game Start/Room Start
  during a room change) are delivered to everyone even if a change is pending.
- R-2: A room change requested after `game_end`/`game_restart`/`game_load` in the
  same step does NOT override it (street racing.gm81 calls Restart Game from the No
  More Lives event and then "restart current room" in the same action list; with
  "last request wins" the game would never end). Among room changes the last wins.
- R-3: No More Lives / No More Health raised by `lives = ...` in code are dispatched
  after the running action finishes (the Host setter has no Vm); the Set Lives /
  Set Health actions dispatch immediately.
- R-4: Drag-and-drop questions with "applies to" an object are true only if true for
  every instance (false when there is none). Function actions applied to other/object
  run with `other` = the original self, like `with`.
- R-5: Action argument kind 2 ("both"): text starting with `"` or `'` is compiled as
  an expression, anything else is a literal string (matches the samples:
  `'Score: ' + string(score)` vs `Street Racing`). Kinds 3–14 are parsed as numbers
  (falling back to expressions).
- R-6: Move Fixed relative adds the chosen vector (motion_add); the "stop" cell with
  relative does nothing. Wrap (action) uses the motion sign and the bounding box:
  moving left and bbox_right < 0 → x += room_width + bbox width (and symmetric).
- R-7: Timeline moments run with event_type = -1 (no event) and event_object = the
  instance's object; errors report "action number N".
- R-8: Trigger conditions are compiled as `return (<condition>);`.
- R-9: Ignore on an error abandons only the failing action; the action list goes on
  (gml-language.md §13). Errors in headless mode go to stderr and are ignored unless
  fatal / "abort on all errors". Compile errors are reported once at load and the
  action raises its message each time it runs.
- R-10: Persistent objects placed in a room are created again on re-entry (RT-19
  recommendation). Placed instance ids are reused if free, else a new id is allocated.

Collision / geometry
- R-11: bbox edges: left = round(min), right = round(max) - 1 (never < left), using
  ties-to-even rounding; rotation of the scaled local box around the origin. Mask pixel
  test samples room pixel centres; point queries (position_*, mouse) test the point
  itself. Alpha tolerance comparison is strict (alpha > tolerance).
- R-12: Combined (non-separate) masks are the union of all frames; automatic bbox per
  frame for separate masks. A user box larger than the image counts as set outside
  the image for ellipse/diamond/rectangle shapes.
- R-13: collision_* with prec=false use the instance bbox; with prec=true the mask
  only if the sprite's shape is not rectangle. Shapes are rasterised by pixel centre
  (lines: centre within half a pixel of the segment).
- R-14: Solid collisions: the put-back is triggered by the other instance's solidity
  only; after the event the instance moves by hspeed/vspeed (path followers to their
  path position) and is put back if it still collides (RT-06 recommendation).

Drawing
- R-15: Software rasteriser, pixel-centre sampling with a top-left fill rule. Filled
  rectangles cover x1..x2 inclusive; lines are 1-px quads centred on pixel centres;
  ellipses/circles are triangle fans with draw_set_circle_precision segments.
- R-16: Blend modes (spec only defines normal): bm_add = (src_alpha, one), bm_max =
  (src_alpha, inv_src_color), bm_subtract = (zero, inv_src_color). The alpha channel
  is blended with the same factors (D3D default), which matters only for surfaces.
- R-17: Fonts: system TrueType files found by family name (Windows font folder,
  name-table scan as fallback), rasterised at size × 96/72 px per em; line height =
  ascent − descent of the face; bold/italic pick the matching face file (no synthesis).
  Sprite fonts (font_add_sprite) are drawn from the sprite frames.
- R-18: Without views the room is drawn 1:1 into a back buffer of the room size; the
  window scales it per the "scaling" setting (>0 fixed %, 0 stretch, <0 keep aspect).
  Window sizes are logical (DPI-scaled) pixels.
- R-19: Particle shapes are generated procedurally (64×64 at size 1). The built-in
  effects (effect_create_*) are approximations; their exact look is unspecified.
- R-20: Room transitions 1–17, 20, 21 are implemented; interlaced (10–13) and rotate
  (18, 19) fall back to "create from centre". Transitions are skipped in headless mode.

Sound / UI / files
- R-21: MIDI (and any undecodable sound) is not played; it counts as "playing" while
  looped so sound_isplaying works for background music. sound_fade sets the volume
  immediately; pan, tempo, effects, 3D sound and CD audio are accepted but ignored.
- R-22: get_string/get_integer and the highscore name entry use an in-game overlay
  (Enter confirms, Escape cancels → default/empty). show_message/show_question use
  native dialogs; show_message_ext maps to up to three native buttons. In headless
  mode questions answer "yes", inputs return their default, highscore names "Player".
- R-23: Highscores and registry_* values are stored in
  %APPDATA%\gm81-runner\highscore_<game_id>.txt / registry_<game_id>.txt (not the real
  registry). registry_set_root only namespaces the _ext keys.
- R-24: game_save/game_load use an implementation-defined binary format (magic
  "GM81RSV1"); F5/F6 use <working_directory>\_save.gmd.
- R-25: working_directory = the folder of the game file (canonicalised), or the
  process's current directory when the game file lives in the temp folder (IDE "Run").
- R-26: File ids for file_text_*/file_bin_* start at 1. file_text_write_real writes a
  leading space and integers without decimals, others with 6 decimals.
- R-27: Not supported (stubs that return 0/""/false): multiplayer (413), DLL calls
  (414), 3D mode (415; d3d_start returns false), joysticks (none present), CD audio,
  video/web splash screens, message_* dialog styling, clipboard (not yet).

## [D3D-IMPL] Implementer notes on 3D / renderer (gm-runner, 2026-10-07)

- D3D-IMPL-1: Culling of 2D-emitted geometry. d3d.md §3.2 says culling applies to
  2D-drawn triangles in 3D mode, but under `d3d_set_projection_ortho` (y down) every
  sprite/rectangle is clockwise on screen, so a HUD drawn with culling on would vanish.
  The runner culls only d3d primitives, shapes and models. Needs black-box check.
- D3D-IMPL-2: Equal-depth draw order vs. per-pass projection reset. Pokemon3d.gm81
  (all instances depth 0; the camera object created after most scenery) only looks
  right if the camera's Draw (which calls d3d_set_projection_ext) runs before the
  scenery — i.e. if GM draws equal-depth instances newest-first, or does not reset the
  projection each pass. runtime.md §13.2 says creation order; please verify which.
- D3D-IMPL-3: Shapes are generated with the front face on the side of the spec's
  normal (floor: +z, wall: (y2−y1, −(x2−x1), 0)); verify against GM.
- D3D-IMPL-4: R-27 is superseded for 415: 3D mode is implemented (software and OpenGL).
