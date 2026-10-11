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
  **Sprite part resolved (verified from user data):** the 8 trailer fields are present
  also for 0 frames (file-formats.md addendum "0-frame sprites"). 0-size images and
  has_data = 0 sounds remain open.
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
- FF-18: Settings "uninitialized" flags word (file-formats.md §2 +12 and the user-corpus
  addendum): bit 0 = treat uninitialized variables as 0 [V]; bit 1 (value 2) is stored by
  the 8.1 IDE in most user projects — meaning unverified (presumably the 8.1 Errors-tab
  option about uninitialized arguments, cf. IDE-12). Any further bits unknown. instance-format
  keeps bit 0 as `treat_uninitialized_as_zero` and preserves the rest verbatim in
  `uninitialized_extra_bits`.
- FF-17: Event sub-numbers for mouse/other/step beyond the values observed (left
  pressed = 6:4, outside room = 7:0, game start = 7:2, no more lives = 7:6, end step
  3:2 presumed) — should be settled in the events/runtime spec (relates to RT-18).
- Partially answers IDE-10: Move Fixed arg0 is a 9-char 0/1 mask over the 3×3 grid in
  numeric-keypad order (index 0 = down-left, 4 = stop, 7 = up; verified in samples). IDE-16: `.gmspr` format
  is now in §9; `.gmbck` etc. presumed analogous.

## [IMPL-FMT] instance-format implementation notes (2026-10-07)

Observations made while implementing `crates/instance-format` against the sample files
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
- tutorial1.gmk and treasure.gm81 re-written by instance-format have byte-identical inflated
  block contents (only zlib compression differs), so every per-record version number in
  §12 for those records is confirmed.
- Not covered by any sample and therefore still per-spec [I]: trigger, constant, path,
  font (v800), timeline, tile and included-file records; loading-bar / custom load images
  (stored as opaque blobs; only written when loading_bar == 2 / show_custom_load_image);
  0-frame sprites (mask fields omitted) and 0-size images (no pixel blob).
  *Correction (user corpus):* 0-frame sprites DO store the mask fields; see FF-11.
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

## [IMPL-IDE] crates/instance-ide implementation choices and spec gaps (2026-10-07)

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
- IMPL-IDE-13 (runner invocation): Run writes `<temp>/instance-ide/<pid>/game.gm81`
  and starts `instance-runner <file>` with the project folder as working directory (temp
  folder for unsaved projects). No secure-mode/debug flags are passed yet: the runner
  CLI for those needs to be specified.
- IMPL-IDE-14 (rendering backend, not a spec issue): eframe's default wgpu backend
  crashed with an access violation on the development machine; the IDE uses the glow
  (OpenGL) backend.

## [IMPL-RUN] crates/instance-runner implementation choices (2026-10-07)

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
  frame for separate masks. A user-defined box is clipped to the image (the mask is a
  per-pixel image of the sprite; nothing outside the image collides). Evidence:
  pokemonEngine15.gm81's player sprites are 16x20 with a manual box l0..r16, moved on
  a 16-px grid against 16x16 solids; with an unclipped 17-px box the player hits a
  solid one step before a grid cell, is put back off-grid and its place_snapped-driven
  logic never stops it (stuck bumping forever). The game only works with clipping.
  (Earlier recommendation, superseded: box outside the image counts as set.)
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
  %APPDATA%\Instance\highscore_<game_id>.txt / registry_<game_id>.txt (not the real
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

## [D3D-IMPL] Implementer notes on 3D / renderer (instance-runner, 2026-10-07)

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
## [D3D] 3D graphics (specs/d3d.md, 2026-10-07)

Resolve by black-box tests with GM 8.1 (screenshots of small test games).

- D3D-1: Orientation of the default 3D projection after d3d_start(). Manual says y is
  reversed (origin bottom-left) and also that the camera looks "along the negative z
  axis" while greater depth looks smaller. Spec recommends: camera on the negative-depth
  side at distance = view width, image vertically mirrored vs 2D. Verify, and verify
  d3d_set_projection_perspective has the same orientation and _ortho is y-down.
- D3D-2: Sign of d3d_transform_*rotation_x/_y/_z/_axis angles (spec: +angle about z turns
  +x toward -y, like GM's direction). Verify x/y/axis by drawing a marked primitive.
- D3D-3: Which screen winding is culled by d3d_set_culling(true) under each projection,
  and the winding/normal sign of d3d_draw_wall and d3d_draw_floor (which side is visible).
- D3D-4: Do d3d_draw_* shapes with hrepeat/vrepeat > 1 repeat without
  texture_set_repeat(true)? Is non-repeat clamp or "not drawn"?
- D3D-5: Texture orientation on shape side faces (spec: v=0 at z2) and the exact
  tessellation of cylinder/cone/ellipsoid (rings, seam, cap mapping); floor slope axis.
- D3D-6: Alpha test in 3D mode: do pixels with alpha 0 write depth (spec: yes)?
- D3D-7: Do surfaces have their own depth buffer in 3D mode, and is it cleared?
- D3D-8: Point light attenuation curve over `range` (spec: linear 1-d/range); flat
  shading colour source; is the ambient term multiplied by vertex colour?
- D3D-9: Colour of model vertices/shapes recorded without colour: creation-time or
  draw-time draw colour?
- D3D-10: d3d_model_save text format (spec gives the commonly observed "100 / count /
  kind + 10 values" layout, number formatting unknown) — confirm by saving models.
- D3D-11: Default fov/aspect for d3d_set_projection (non-ext) (spec: 45°, view aspect)
  and the depth range of d3d_set_projection_ortho (spec: -32000..32000).
- D3D-12: Transform stack capacity (spec: 32) and whether the world transform applies to
  2D drawing outside 3D mode (spec: yes); does d3d_start/d3d_end reset culling/fog/etc.?
- D3D-13: How room backgrounds/foregrounds and tiles are drawn in 3D mode (spec: backdrop
  without depth test at z=0; tiles at layer depth).

## [MP] Multiplayer (specs/multiplayer.md, 2026-10-07)

- MP-1: Instance uses its own protocol (Part B); interop with GM 8.1 DirectPlay
  games is out of scope. Confirm this is acceptable.
- MP-2: When the host leaves a non-migrating session, does mplay_session_status() on the
  clients become 0 in GM? Do players still appear in mplay_player_find?
- MP-3: Are messages to "all" (player 0) also delivered to the sender in GM? (spec: no)
  Is name matching case-sensitive? (spec: yes)
- MP-4: Is shared data cleared on session create/join/end, and does a joiner receive the
  current table? (spec: yes)
- MP-5: Blocking time of mplay_session_find in GM (spec: ~1 s LAN, 3 s addressed).

## [JOY] Joystick (specs/input-joystick.md, 2026-10-07)

- JOY-1: Threshold for joystick_direction and direction events (spec: 0.5) and whether
  the POV hat contributes (spec: no).
- JOY-2: Value of joystick_direction for a missing joystick (spec: vk_numpad5 = 101).
- JOY-3: Are joystick events dispatched before or after the other mouse sub-events?
- JOY-4: Axis mapping of an XInput pad as seen by GM 8.1 (z = triggers sign, r/u = right
  stick) — verify with a real controller.

## [SND] Sound (specs/sound.md, 2026-10-07)

- SND-1: Curve mapping sound_volume/sound_global_volume 0..1 to loudness (spec: linear
  amplitude; GM may map linearly onto DirectSound hundredths of dB). Measure.
- SND-2: Pan curve (spec: linear attenuation of the opposite channel).
- SND-3: Can two MIDI sounds play at once (kind 0 MIDI + kind 1 MIDI)? (spec: no, one
  MIDI player). Does sound_volume affect MIDI in GM?
- SND-4: 3D sound beyond maxdist: silent or constant? Rolloff curve.
- SND-5: Do effect changes apply to already-playing voices? Do sound_effect_* parameter
  calls for an unselected effect have any effect?
- SND-6: Does sound_play on a non-existent sound raise an error in GM 8.1?
- SND-7: Does an MP3 given kind 0..2 play at all in GM 8.1?
- SND-8: When does sound_isplaying turn true after sound_play/sound_loop (the manual warns
  it can lag), and false after sound_stop or the end of a sound? Polling
  "if (!sound_isplaying(s)) sound_loop(s)" each step stacks loops if it lags. bb_sound.
  RESOLVED 2026-10-11 (sound.md §2): about 100..116 ms lag both ways (6 steps at 60/s),
  background music too; the polling pattern fired sound_loop 6 times.

## [EXT] Extensions and DLLs (specs/extensions.md, 2026-10-07)

- EXT-1: Layout of the .gex package installer file and where GM 8.1 installs user
  packages (IDE extensions folder vs per-user folder).
- EXT-2: Standalone exe must embed used packages; proposed extension of exe-format.md
  (package bundle after the project payload). Needs an owner decision.
- EXT-3: Meaning of .ged calling-convention codes (12, 11, 2) and argument_count -1.
  Implementer observation (2026-10-07, black-box): calling the shipped `Printing.dll`
  export `pr_set_font_size` (code 12, one real) through a typed **stdcall** call
  crashes the 32-bit helper, while a typed **cdecl** call works — so code 12 appears
  to mean cdecl (and 11 presumably stdcall), the reverse of the [I] guess in
  extensions.md §3.3. Instance's x86 bridge now uses a convention-agnostic thunk
  (restores ESP after the call), so either reading works.
- EXT-4: Exact timing of extension init functions relative to Game Start/Create events
  and of final functions relative to Game End; are finals called on game_restart?
- EXT-5: Are extension constant values evaluated as GML expressions or literal?
- EXT-6: external_call with wrong argument types: error or conversion? external_define
  return id numbering and error message wording.
- EXT-7: Meaning of the package-level `hidden` flag of a `.ged` (0 in all shipped
  packages). Instance's Extension Packages form (ide.md §18.4) assumes it hides the
  package from the Available list (a helper package only pulled in through another
  package's `uses`). Also: does GM 8.1 add a package's `uses` dependencies to the
  project automatically when the package is selected? Instance does not.

## [BLACKBOX] Black-box test projects (2026-10-07)

`crates/instance-blackbox` generates GM 8.1 test projects (`instance-blackbox generate <dir>`) that
log `key=value` observations for most RT-*, GML-*, G-*, R-* items above plus every
built-in constant (GML-19). The key → question mapping is `specs/blackbox-tests.md`;
`instance-blackbox compare <ours> <gm>` lists where instance-runner differs from GM 8.1. Resolve the
items here once GM results are in.

## [RESOLVED-ROOMS] Room transitions verified in GM 8.1 (bb_rooms, 2026-10-08)

Evidence: `gm81_blackbox_tests/gm/results_bb_rooms.txt` (GM 8.1 Lite); rules written
up in runtime.md §3.2a. instance-runner now matches all 48 bb_rooms keys.
- RT-02: RESOLVED (runtime.md §3.2a rule 1). After `room_goto` the current action
  finishes, the remaining actions of the event are skipped, the rest of the step is
  abandoned (no later instances / phases / draw) and the transition runs at once.
  Observed `go.ST go.after_goto k.RE ...` with no `go.next_action` and no `gx.ST`.
  Only room changes were verified; instance-runner keeps game_end/game_restart/game_load
  as before (remaining actions run; rest of the step abandoned).
- RT-03: RESOLVED (§3.2a rule 2c/2e, observed s08). Re-entering a persistent room
  does not re-run the room creation code (no `r1.CC2`); restored instances do get
  Room Start (`pi.RS42`), restored instances first, then the carried ones.
- RT-19: RESOLVED (§3.2a rule 2b). A placed instance whose id already exists (a
  carried persistent instance) is not created again: no duplicate, no creation code,
  no Create (`pp.RS1` on every re-entry).
- R-1: RESOLVED / corrected (§3.2a rules 1, 3). The change is now also checked
  between the actions of an event list (event lists running when it was requested
  stop after their current action; lists started later, e.g. the Create of an
  instance_create after the call, run normally). A frame is drawn right after a
  room is entered (after Room Start, before the first Begin Step), also at game
  start.
- R-10: RESOLVED / corrected (§3.2a rule 2). All placed instances are added to the
  list before any creation code / Create runs (`instance_count` = all placed);
  carried persistent instances follow them; then per placed instance its creation
  code and Create; then Game Start (first room only), room creation code, Room Start.
- RT-12: PARTLY RESOLVED (§3.2a rule 4): defaults (score 0, lives -1, health 100,
  show_score 1, show_lives/health 0, captions) and the caption format
  `Cap Score: 12 Lives: 3 Health: 50` (single spaces; `window_get_caption` returns
  the shown caption). The No More Lives/Health timing remains open.
- R-28 (new implementation choice, unverified): a room change requested while a
  room is being entered (Create, creation code, Room Start) does not cut the entry
  short: the requesting event stops after its action, the entry completes without
  drawing its frame, then the new room is entered at once (also before the first
  step). The new room's entry events still see the input edges of the step in which
  the change was requested (they are cleared after the transition).

## [RESOLVED-LANG] GML semantics verified in GM 8.1 (bb_lang, bb_args0, bb_random, bb_restart, bb_optional file.*, bb_clash; 2026-10-08)

Evidence: `gm81_blackbox_tests/gm/results_*.txt`. The rules are written up in
gml-language.md ("Language and library details — VERIFIED") and in runtime.md
§3.3 and §19. instance-runner now matches every non-informational key of bb_lang
(except `ref.other_in_driver_create`, see GML-10), bb_args0, bb_random,
bb_restart, bb_clash and bb_optional `file.*`.
- GML-1: RESOLVED. Reals are true when >= 0.5. A string condition is the
  run-time error "Expression expected".
- GML-2: RESOLVED. Mixed string/real comparison is "Cannot compare arguments.";
  arithmetic, logical and bitwise operators on mixed operands are errors.
- GML-3: RESOLVED. Comparison uses `SameValue`-style tolerance (1e-12, relative
  for large values). Open detail: relative vs absolute above 1, which would need
  `1e6 == 1e6 + 1e-7`.
- GML-4: RESOLVED. Conversion rounds half to even. Bitwise operations are 64-bit;
  `>>` is logical; shift counts follow Delphi's rule. `floor`/`ceil` wrap to 32
  bits.
- GML-5 / G-11: RESOLVED. Non-integral values print with 2 decimals and at most
  10 significant digits, otherwise in scientific form. Integral values outside
  the int64 range print "ERROR". `string_format` uses 18 digits.
- GML-6: RESOLVED. `/` and `div` by 0 give "Division by 0."; `mod` gives
  "Modulo by 0.". The error aborts the action.
- GML-11 / G-2 / FF-18 / IDE-12: RESOLVED.
  - Settings flag bit 1 is the "Throw an error when arguments aren't initialised
    correctly." option. With it set, the argument count is checked; with it
    clear, missing arguments are 0.
  - `argument_count` is read-only, checked at run time.
  - Arguments live in a global array.
  - instance-format exposes the bit as `Settings::argument_errors`.
- GML-12: RESOLVED. `var` is lexical, and a built-in name can't be redeclared.
- GML-14: RESOLVED. `%` is not an operator; `:=` in an expression compares; `.5`,
  `12.` and `1.2.3` are accepted; `1e3` is not.
- GML-15: RESOLVED. `continue`/`break` outside a loop act like exit (the value
  stays as set); a bare `return;` is a compile error.
- GML-16: RESOLVED. Wrong-type assignments to built-ins store 0. Reads through a
  missing owner give "Unknown variable x"; writes through `noone` give "Cannot
  assign to the variable"; other writes with no instance are no-ops.
- GML-17: RESOLVED. Code errors abandon the action. Built-in function errors and
  errors inside `execute_string` don't.
- GML-18: RESOLVED (format). Positions match GM's in the observed cases, except
  compile-time argument-count errors (GM reports them at the end of the call).
- GML-20: RESOLVED. Script beats built-in function; resource name beats user
  constant; user constant beats built-in constant.
- GML-10: PARTLY. `other` in a Create event run by `instance_create` is the new
  instance itself (`ref.other_in_driver_create = t275`). This is handled with
  bb_instances / bb_events (runner).
- G-1: CORRECTED. `choose/min/max/mean/median` accept 0 arguments, result 0.
- G-20: CORRECTED. Function errors are non-aborting and read "Error in function
  f()." (`sqrt(-1)`: "Cannot apply sqrt to negative number.").
- G-21: CONFIRMED (`round` ties to even, `sign(0)` = 0, `frac(-3.25)` = -0.25).
- G-22: CORRECTED. `real()` is strict (see gml-language.md); a real argument is
  returned unchanged.
- G-23: CORRECTED. `string_delete` with an index < 1 deletes nothing. A real
  passed for a string is "".
- G-24: CORRECTED. `string_lower/upper` return Latin-1 letters UTF-8-encoded.
  Open: confirm with `string_length(string_lower(chr(192)))` (expected 2).
- G-25: CORRECTED. `min/max` skip arguments of the other type.
- RT-04: PARTLY (runtime.md §3.3). `game_restart` clears globals, resets
  score/lives and keeps health. Whether Game End runs, and the id counter, still
  need a test that carries the results through a file.
- R-26: CORRECTED. `file_text_write_real` wraps integers to 32 bits;
  `file_text_open_read` of a missing file returns -1.

## [RESOLVED-MOTION] Motion / collision / bbox verified in GM 8.1 (bb_motion, bb_optional, 2026-10-08)

Evidence: `gm81_blackbox_tests/gm/results_bb_motion.txt`, `results_bb_optional.txt`
(GM 8.1 Lite); rules written up in runtime.md §4.2 (5b), §4.3, §7.5, §8.2, §8.3,
§8.6, §10.1, §10.2. instance-runner now matches all 472 bb_motion keys and all 22
bb_optional `xform.*` keys.
- RT-01: PARTLY RESOLVED. Timelines advance after Begin Step (mo.tl.*: moment 0 of a
  timeline started in Create logs step 1, after the clock's Begin Step), not before
  it. Order relative to alarms/keyboard/mouse/Step still open. Animation advances
  after Draw (Draw sees the pre-advance image_index, mo.an*.dr) — unchanged.
- RT-06 / R-14: RESOLVED (§8.6). The put-back happens when either instance is solid
  (solid mover vs non-solid wall stops too: mo.sol.r3m stays at 112). A colliding
  pair is handled once per step, the partner's event right after the first one.
  Collision events are processed object-major (objects ascending, then the event's
  target object ascending, then instance order): `r4m r4w r1m r1w r5m r5w1 r3m ...`.
  Not verified: whether both instances are moved back (only static walls tested).
- RT-08: (alarms) not covered by bb_motion.
- RT-11 / R-11: RESOLVED (§8.2). bbox corners are `xs·(l−ox)` and `xs·(r+1−ox) − 1`
  (−1 after scaling, also for negative scales), rotated, min/max rounded half to
  even (27 cases incl. origin 8,8 at .5 positions, scales −1/0.3/0.5/1.5/2, angles
  30/45/90/180). The bbox is only a pre-filter: a pixel belongs to an instance only
  if its centre maps into the local mask box, also for rectangle masks
  (coll.place_meeting_xm15_5 = 0 although bbox_right = 0 touches column 0).
- RT-17: RESOLVED for open paths (§10.1). Relative paths are anchored at their first
  point even when a negative speed starts them at position 1; end action stop sets
  position 1 (also when running backwards); restart/continue carry the overshoot,
  reverse mirrors it; path_speed is never changed by stop. Timelines: moments run
  with the position already advanced; speed 2/0.5/−1, fractional start, looping
  (position 0 after passing the last moment) as recommended.
- New (VERIFIED, §4.3): step events (begin/normal/end) are delivered object-major
  (objects ascending, then instance order): mo.seq `r1m r4m r5m r2m r3m`. Room
  Start/End stay in instance order (bb_rooms). Alarm/keyboard/mouse order across
  objects remains instance order until bb_instances / bb_events results arrive
  (bb_instances `in.s*` already logs step order of objects created out of index
  order; bb_events logs timeline vs alarm vs step tags).
- Animation (§7.5): negative image_speed never wraps and fires no Animation
  End (mo.an3: −1, −2, −3, ...).

## [RESOLVED-FOLLOWUP] bb_lang2 / bb_restart2 / bb_instances follow-ups (2026-10-08)

Evidence: `gm81_blackbox_tests/gm/results_bb_lang2.txt`, `results_bb_restart2.txt`,
`results_bb_instances.txt`. instance-runner now matches every non-informational key of all
projects with GM results.
- GML-3: RESOLVED (bounds). Equality is not Delphi's `SameValue` with 1e-12: `0 == 1e-13`,
  `1 == 1 + 5e-13` and `1e6 == 1e6 + 1e-7` are false, while `sin(pi) == 0` is true.
  instance-runner uses `|a-b| <= max(min(|a|,|b|), 1) * 1e-14` (gml-language.md, Comparison).
  Still open: the exact constant and whether it is relative above 1 (proposed bb_lang3
  keys: `0 == 1e-14`, `0 == 5e-15`, `0 == 1e-15`, `100 == 100 + 1.5e-13`,
  `1e6 == 1e6 + 5e-9`).
- G-24: CORRECTED AGAIN. Case mapping keeps one byte per character; it is `ord()` that
  returns the first UTF-8 byte (`ord(chr(192))` = 195, `string_length(chr(192))` = 1).
  Open: whether Latin-1 letters are case-mapped (test: `string_lower(chr(192)) ==
  chr(224)`), and `ord(chr(128))` / `ord(chr(159))` (code page 1252).
- G-25: RESOLVED. `mean()` counts strings as 0 (`mean(1, 'a')` = 0.5, no error).
- GML-11: RESOLVED. The implicit result is the last assignment or call statement
  (`abs(5);` last gives 5); `if` conditions don't count. "Script requires at least N
  arguments" for scripts that also use `argument[i]`. `script_execute` with too many
  arguments reports a non-aborting error and runs the script anyway. Open: whether the
  script also runs when `script_execute` passes too few (the result is 0 either way).
- RT-04: RESOLVED (runtime.md §3.3). `game_restart`: the action finishes, the rest of the
  event is skipped, Room End then Game End for all instances, no Destroy events.
- RT-21: PARTLY RESOLVED (runtime.md §5.7). Without events, the calling code keeps the
  old object and properties; by the next action the instance is the new object, but
  `with (id)` finds nothing. Open (proposed follow-up): `instance_number(new)` and
  `with (new)` right after the call inside the same code; `x = 5` after the call, read
  in the next step; `with (id)` in the next step; the same with `perf = true`.
- GML-18: action numbers in error headers are not restored after a nested event
  (runtime.md, errors): `destroy.read_after` reports "action number 1".
- (The open points of GML-3, G-24, GML-11 and RT-21 above are settled by bb_lang3, next
  section.)

## [RESOLVED-LANG3] bb_lang3 follow-up (2026-10-08)

Evidence: `gm81_blackbox_tests/gm/results_bb_lang3.txt` (GM 8.1). instance-runner now matches
all 29 bb_lang3 keys and still every non-informational key of the other projects.
- GML-3: RESOLVED. Equality is `|a - b| < 1e-13`, absolute also for large values
  (gml-language.md, Comparison). `0 == 1e-14`, `1 == 1 + 5e-14` are true;
  `100 == 100 + 1.5e-13`, `1e6 == 1e6 + 5e-10` and `0 == 1e-13` are false. A relative
  tolerance cannot give both `1 == 1 + 5e-14` true and `1e6 == 1e6 + 5e-10` false.
- G-24: RESOLVED. Characters are Windows-1252: `ord()` is the first UTF-8 byte of the
  character's code point (`ord(chr(128))` = 226 for €, `ord(chr(159))` = 197 for Ÿ,
  `ord(chr(160))` = 194, `ord(chr(255))` = 195); `string_lower/upper` use Unicode case
  mapping (`string_lower(chr(192)) == chr(224)`, `string_lower(chr(215))` unchanged,
  `string_upper(chr(255))` is Ÿ, `ord` 197), one character per character.
- GML-11: RESOLVED. `script_execute` with too few arguments reports the non-aborting
  error and still runs the script, missing arguments 0 (args3.execute_few_runs "0/7").
- RT-21: RESOLVED (runtime.md §5.7). `instance_change` replaces the instance record:
  the running code keeps the old one (reads old values, its writes are lost); all other
  code sees the new record at once (instance_number, with (obj), instance_find, `id.var`
  reads), except `with (id)`, which finds nothing until the end of the step. The same
  with `perf = true`. Still unverified: the new record's place in the instance order
  during that step, and `id.var = ...` writes during that step.

## [RESOLVED-EVENTS] Event dispatch verified in GM 8.1 (bb_events, bb_instances, 2026-10-08)

Evidence: `gm81_blackbox_tests/gm/results_bb_events.txt`, `results_bb_instances.txt`
(GM 8.1 Lite); rules in runtime.md §4.3a. instance-runner now matches all 59 bb_events keys
and 72 of 75 bb_instances keys.
- RT-01: PARTLY RESOLVED (§4.3a rule 1): timelines run after Begin Step, before
  alarms; Animation End after drawing. The positions of the xprevious capture and of
  the particle / view updates remain unverified.
- RT-05: RESOLVED (§4.3a rule 6): Outside Room = box beyond the room by at least one
  pixel; Intersect Boundary every step while not completely inside (also when fully
  outside); both repeat every step. Instances without a sprite: unverified.
- RT-08: RESOLVED (§4.3a rule 4): integer alarms (round half even), fire at 0 and
  read 0 in the event, 0 becomes -1 next step without firing; alarms only tick when
  the object has (or inherits) that alarm event; alarm-number-major order.
- RT-09: RESOLVED (§4.3a rule 2): object-major dispatch; an instance created during
  a phase runs in it only if its object has not been processed yet.
- RT-12: No More Lives / Health timing RESOLVED (§4.3a rule 7): at the assignment.
- RT-13: PARTLY RESOLVED (§4.3a rule 8): simulated keys take effect the next step;
  held-key events before press events (ev.s06 "a.KB a.KBany a.KP"). Key-major vs
  instance/object-major nesting is still open.
- RT-16 / D3D-IMPL-2: equal-depth instance draw order RESOLVED (§4.3a rule 5, stable
  re-sort of the previous frame's order). Tiles / particle systems at equal depth
  remain open.
- GML-10: RESOLVED (§4.3a rule 9): `other` is the instance itself in Create (run by
  instance_create) and Destroy.
- runtime.md §5.6: double destroy and reads after destroy RESOLVED (§4.3a rule 10).
- Still open from these projects:
  - RT-21 (bb_instances change.props): right after `instance_change(in_ic2, false)`
    the same code still reads the OLD object_index, sprite, depth, solid, visible,
    persistent and mask; instance-runner switches all of them at once. New test (added to
    bb_instances, needs a GM run)
    `change.later.next_action` / `change.later.counts`: logs the same properties in
    the following action, plus `instance_number(in_ic1)`,
    `instance_number(in_ic2)` and the id, to learn whether GM changes the object later
    (deferred) or replaces the instance record.
  - Collision event order (in.s01+1 / in.s02+2: GM "cx.COcy1 cy1.COcx" although cy1
    was placed before cx; instance-runner fires cy1's event first at step 2) belongs with
    the bb_motion collision work (mo.seq). GM appears to process pairs starting from
    the lower object index and to fire both sides of a pair together.
  - Keyboard / mouse phase order: instance-runner assumes object-major as for Step.
    Proposed test: two objects with Keyboard <K> events whose instances are placed in
    reverse object order, key press simulated the step before.

## [SURF] Surface target across the automatic draw (runtime.md §14.1, 2026-10-09)

- SURF-1: When a surface is still the drawing target as the automatic draw begins
  (set in End Step, not reset), GM 7/8-era games (Spelunky 1.1, ChevyRay's view-scaling
  scripts) evidently get the room drawn into that surface with each view mapped onto the
  whole surface even though the port (e.g. 960x720) is larger than the surface (320x240).
  Recommended behaviour recorded in runtime.md §14.1. Open: exact mapping when the port
  fits inside the surface or only partly overlaps it, whether the back buffer is cleared,
  and what happens with several views. Pro-only (surfaces), so not testable with Lite.
