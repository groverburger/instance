# Runtime (runner) behavioural specification — GameMaker 8.1 clean-room

Status: draft v1. Sources: the GM 8.1 user manual only (pages are cited by their
file number, e.g. `[111]` = events page, `[403_04]` = collision checking). Nothing
in this document was derived from YoYo binaries.

Tagging convention used throughout:

* No tag: behaviour described by the manual (paraphrased).
* **UNSPECIFIED – recommended behaviour: …**: the manual is silent or ambiguous;
  the recommendation reflects widely known, black-box-observable GM8 behaviour and
  should be confirmed by black-box testing before being treated as final. Items
  with low confidence also appear in `OPEN_QUESTIONS.md` (section "runtime.md").

Other specs are expected to cover: the GML language/interpreter, the file format,
the full function catalogue, and the drag-and-drop action library. This document
covers *when* things happen and *how runtime state evolves*.

---

## 1. Core runtime data model

### 1.1 Global game state
| Item | Notes |
|---|---|
| Resource tables | sprites, sounds, backgrounds, paths, scripts, fonts, timelines, objects, rooms, triggers. Indices are not necessarily contiguous (`[403_08]`: never do arithmetic on room indices). |
| Room order | an ordered list of room indices defined in the IDE; `room_first`/`room_last` are its ends; `room_next(r)` / `room_previous(r)` walk it and return -1 at the ends. Rooms created with `room_add()` are *not* in the order `[409_09]`. |
| Current room | `room` (assignable; assignment behaves as `room_goto`). |
| Instance list | all live instances of the current room in *creation order* (this order matters for event iteration and same-depth drawing). |
| Tile list | tiles of the current room (id, background, left/top/width/height, x, y, depth, visible, xscale, yscale, blend, alpha) `[405_08]`. |
| Globals | `score`, `lives`, `health`, `show_score`, `show_lives`, `show_health`, `caption_score`, `caption_lives`, `caption_health` `[403_09]`; `room_speed`, `room_caption`, `room_persistent`, `room_width`*, `room_height*`; background_* arrays; view_* arrays; `transition_kind`, `transition_steps`; `keyboard_*`, `mouse_*`; `cursor_sprite`; `fps*`, `current_time*`, date variables; `error_occurred`, `error_last`; `debug_mode*`; `gamemaker_version*`, `gamemaker_pro*`, `gamemaker_registered*`; `temp_directory`, `working_directory`; user `global.*` variables. |
| Persistent room snapshots | for each persistent room that has been left: the full saved room state (instances with all variables, tiles, backgrounds, views, room vars). |
| Particle systems | global, survive room changes and game restarts `[412_02]`. |
| Surfaces, data structures, files, mp grids | global; untouched by room changes. |

* A trailing `*` on a variable name means read-only, as in the manual.
* **UNSPECIFIED – recommended initial values:** `score = 0`, `lives = -1`,
  `health = 100`, `caption_score = "Score: "`, `caption_lives = "Lives: "`,
  `caption_health = "Health: "`, `transition_kind = 0`, `transition_steps = 80`
  (80 is stated by `[403_08]`), `cursor_sprite = -1` (stated by `[404_02]`).
  Defaults for `show_score/show_lives/show_health` are an open question (see §20.1).

### 1.2 Instance record
Every instance holds (built-ins, see §7 for coupling rules):

`id*`, `object_index*`, `x`, `y`, `xprevious`, `yprevious`, `xstart`, `ystart`,
`hspeed`, `vspeed`, `speed`, `direction`, `friction`, `gravity`,
`gravity_direction`, `solid`, `visible`, `persistent`, `depth`, `sprite_index`,
`mask_index`, `image_index`, `image_speed`, `image_number*`, `image_xscale`,
`image_yscale`, `image_angle`, `image_alpha`, `image_blend`, `sprite_width*`,
`sprite_height*`, `sprite_xoffset*`, `sprite_yoffset*`, `bbox_left*`,
`bbox_right*`, `bbox_top*`, `bbox_bottom*`, `alarm[0..11]`, `timeline_index`,
`timeline_position`, `timeline_speed`, `timeline_running`, `timeline_looping`,
`path_index*`, `path_position`, `path_positionprevious`, `path_speed`,
`path_orientation`, `path_scale`, `path_endaction`, plus a map of user-defined
instance variables (incl. arrays), and runtime flags: `active` (see §18),
`marked_for_destroy`, `mouse_was_over` (for enter/leave).

Defaults on creation (from the object resource unless stated):
`sprite_index` = object sprite; `mask_index` = object mask (-1 = use sprite);
`visible`, `solid`, `persistent`, `depth` from the object; `x`, `y` = creation
position; `xstart = xprevious = x`, `ystart = yprevious = y`.

**UNSPECIFIED – recommended defaults for the rest:** all speeds/friction/gravity 0,
`gravity_direction = 270`, `direction = 0`, `image_index = 0`, `image_speed = 1`,
`image_xscale = image_yscale = 1`, `image_angle = 0`, `image_alpha = 1`,
`image_blend = c_white`, every `alarm[i] = -1`, `timeline_index = -1`,
`timeline_position = 0`, `timeline_speed = 1`, `timeline_running = false`,
`timeline_looping = false`, `path_index = -1`, `path_position = 0`,
`path_positionprevious = 0`, `path_speed = 0`, `path_scale = 1`,
`path_orientation = 0`, `path_endaction = 0`.

### 1.3 Instance and tile ids
* Instance ids are unique and ≥ 100000 `[403_05]`. Ids of instances placed in the
  room editor are stored in the game file and reused when the room is (re)built.
* **UNSPECIFIED – recommended:** runtime-created instances take
  `next_instance_id`, a global counter initialised to (highest id stored in any room
  of the game file) + 1, incremented on each creation, never reused (also not after
  restart). Tile ids follow the same scheme with their own counter (editor tile ids
  are ≥ 10000000 in GM8 files; the file-format spec should confirm).

### 1.4 Special instance selectors
`self`, `other`, `all`, `noone`, `global` are special values `[401_06]`.
**UNSPECIFIED – recommended numeric values** (needed when they are stored in
variables): `self = -1`, `other = -2`, `all = -3`, `noone = -4`, `global = -5`.
Wherever an "object" argument is accepted it means: that object *and all its
descendants* `[205]`; an instance id means that one instance; `all` means every
(active) instance.

---

## 2. Startup sequence

1. **Process start.** Read the embedded game data. While loading, show the loading
   image (if configured; optionally colour-keyed by its bottom-left pixel and with a
   window alpha 0–255) and the progress bar (none / default / custom back+front
   images, front scaled or clipped) `[302_04]`.
2. **Apply settings** `[302_01..05]`: priority (`set_program_priority` range −3..3),
   resolution/colour depth/refresh change if "set the resolution" is checked (restore
   on exit), vsync, cursor visibility, window border/caption icons/stay-on-top/
   sizeable, freeze-on-focus-loss, screensaver suppression, error options, the
   special-key flags (Esc, close-as-Esc, F1, F4, F5/F6, F9), interpolation, outside
   colour, scaling mode (fixed %, keep aspect, full stretch).
3. **Export included files** that are set to auto-export, into the game folder,
   the temp folder (`temp_directory`), or a custom folder; respect "overwrite
   existing" and "free memory after export" `[209]`.
4. **Load resources** (sprites/backgrounds into textures, sounds into the audio
   engine, fonts, paths, scripts compiled, objects' event tables built, triggers,
   timelines, rooms). Build per-object *effective event tables* including
   inherited events (§6).
5. **Initialise globals** (§1.1), random seed (**UNSPECIFIED – recommended:**
   seed from the clock).
6. **Create the window**: windowed or fullscreen per settings; client/region size
   derived from the first room (§12.1). Set caption to the first room's caption.
7. **Go to the first room** using the room-change algorithm of §3 with
   `is_game_start = true`.
8. Enter the main loop (§4).

---

## 3. Room changes, game end, restart, save/load

### 3.1 Requesting a change
`room_goto(r)`, `room_goto_next()`, `room_goto_previous()`, `room_restart()`,
assignment to `room`, `game_end()`, `game_restart()`, `game_load(f)` do **not**
take effect immediately `[403_08]`, `[112_03]`:

* The currently executing action/script runs to completion (including code after
  the call and any calling scripts).
* After that no further *events* are executed in the current step; the pending
  change is then performed.
* **UNSPECIFIED – recommended:** the remaining actions of the *same* event action
  list are still executed (the check happens between events, not between actions);
  the pending request is checked after every event dispatch and the step is
  aborted at that point (skip remaining event phases and the draw). If several
  requests occur, the last one wins. Low confidence: see OPEN_QUESTIONS.
* `room_goto_previous()` in the first room / `room_goto_next()` in the last room
  is an error `[112_02]` (non-fatal runtime error; no change).
* `game_load` is likewise applied at the end of the current step `[403_08]`.

### 3.2 Room transition algorithm (`goto(new_room)`)
`[210_01]` gives the order; numbered steps are normative.

1. If a current room exists: every existing (active) instance receives the
   **Room End** event (`ev_other`/`ev_room_end`).
2. Capture the "old room" image if a transition is pending (§3.4).
3. Non-persistent instances are removed **without** a Destroy event. Persistent
   *active* instances are kept aside. Deactivated instances are never carried over,
   even if persistent `[403_06]`.
   * If the room being left is persistent, its state (remaining non-persistent
     instances incl. deactivated ones, tiles, backgrounds, views, room variables)
     is stored as its snapshot before removal.
4. Set `room = new_room`; load room properties: width, height, caption, speed
   (`room_speed`), background colour/show-colour, the 8 backgrounds, views enabled
   and the 8 views, tiles, persistent flag. If the new room has a snapshot
   (persistent & visited), restore all of that from the snapshot instead.
5. Add the carried-over persistent instances to the instance list (they keep all
   their variables and their position).
6. If the room has **no** snapshot: for each instance placed in the room, in the
   room's stored instance order: create the instance, run its **instance creation
   code** `[210_01]` and then its **Create** event. If a snapshot was restored,
   restored instances get **no** Create event.
   * **UNSPECIFIED – recommended:** each placed instance is created and fully
     initialised (creation code + Create) before the next one is created; an
     instance whose object is persistent and which already exists among the carried
     over instances is still created again (GM8 duplicates persistent objects placed
     in a room that is re-entered — a common user pitfall).
7. If `is_game_start` (first room after start or restart): every instance receives
   **Game Start**.
8. Run the **room creation code**. **UNSPECIFIED – recommended:** skip it when the
   room was restored from a snapshot (low confidence; see OPEN_QUESTIONS).
9. Every instance receives **Room Start** `[111]`, `[210_01]`.
10. Recompute window/region size from the room/views if needed (§12.1), update the
    caption, reset view-follow, play the transition (§3.4).

Room start applies to all instances present (including carried-over persistent
ones). Game Start happens after Create and before Room Start `[111]`.

### 3.3 Game end and restart
* `game_end()`: every instance receives **Game End** `[111]`; then included files
  flagged "remove at game end" are deleted, the temp folder is removed, the display
  mode is restored, and the process exits.
  **UNSPECIFIED – recommended:** do not run Room End before Game End. (Open question.)
* Pressing Esc with "Let <Esc> end the game" checked behaves as `game_end()`.
* `game_restart()`: **UNSPECIFIED – recommended:** run Game End for all instances,
  then remove *all* instances (persistent included, no Destroy events), discard all
  persistent-room snapshots, reset `score/lives/health` to defaults, stop all sounds,
  then perform §3.2 into `room_first` with `is_game_start = true`. User `global.*`
  variables, particle systems, surfaces, data structures and dynamically changed
  resources are **not** reset (`[412_02]` confirms for particle systems).

### 3.4 Transitions
* `transition_kind` (0–21 built-ins listed in `[403_08]`; 0 = none) applies to the
  next room change only and is then reset to 0. `transition_steps` (default 80) is
  the number of frames the effect lasts.
* Built-in kinds: 1–5 create from left/right/top/bottom/center; 6–9 shift from
  left/right/top/bottom; 10–13 interlaced from left/right/top/bottom; 14–17 push
  from left/right/top/bottom; 18/19 rotate left/right; 20 blend; 21 fade out and in.
* Implementation: render the old room's last frame to surface A; after §3.2 render
  the new room once (full draw incl. draw events) to surface B; then for
  `k = 1..transition_steps` draw the mix with fraction `k/transition_steps` and
  present, pacing at `room_speed`. No events run during the transition.
* `transition_define(kind, script_name)` registers a user script called with
  `(surfA, surfB, width, height, fraction)` per frame; `transition_exists(kind)`.
* Transitions are disabled in 3D mode `[403_08]`.

### 3.5 Save and load
`game_save(fname)` / `game_load(fname)`, F5/F6 `[403_08]`, `[302_03]`. Saved: the
"basic game data" — current room and all instance state, persistent room
snapshots, globals, views/backgrounds/tiles. Not saved: playing sounds/music
positions, changed resources, data structures, particles, surfaces, multiplayer
state. Loading a file from another game/version is an error. Load is deferred to
the end of the step. **UNSPECIFIED – recommended:** the file format is
implementation-defined (we do not need GM8 save-file compatibility); after a load,
no Create/Room Start events run; the default F5/F6 file name is
`<working_directory>\_save.gmd` (low confidence).

---

## 4. The main loop

### 4.1 Pacing
* The game aims to run `room_speed` steps per second (room property; changeable
  at runtime, fractional values allowed `[403_07]`).
* `fps*` = number of frames actually drawn during the last second. **UNSPECIFIED –
  recommended:** update `fps` once per second; never exceed `room_speed` (sleep
  until the next frame deadline; if late, do not try to catch up by skipping
  draws).
* With "use synchronization" (vsync) the present waits for vertical blank `[302_02]`;
  `set_synchronization(b)` and `screen_wait_vsync()` `[405_12]`.
* If "freeze when the form loses focus" is set, the loop pauses (no steps, no
  draws) while the window is unfocused `[302_01]`. On focus loss, input state is
  cleared (`[new]`: 8.0 change "io events like key presses are now cleared").

### 4.2 Order of one step
The manual fixes this macro order `[111]`, `[403_10]`:
begin step → alarms → keyboard/key press/key release → mouse → step → (move
instances) → collisions → end step → draw. Triggers run just before the begin
step, step and end step events respectively `[208]`.

The full recommended sequence (bold items are fixed by the manual; the rest is
**UNSPECIFIED – recommended behaviour** and should be verified):

1. Pump OS messages and update input state (§11): compute this step's
   pressed/released sets, held set, mouse position, wheel.
2. Handle built-in special keys if enabled: Esc (end game), F1 (show info),
   F4 (toggle fullscreen), F5 (save), F6 (load), F9 (screenshot to
   `screenshotNNN.png`, NNN increasing) `[302_03]`. Window close button: if "treat
   close button as Esc" → act as an Esc key press (generate the Esc keyboard /
   key-press event, and end the game if Esc-ends-game is set); otherwise every
   instance gets **Close Button** (`ev_close_button`) `[111]`, `[302_03]`.
3. For every instance: `xprevious = x`, `yprevious = y`,
   `path_positionprevious = path_position`.
4. Timelines advance for every instance (§10.2).
5. **Begin-step triggers**, then **Begin Step** events.
6. **Alarm events** (§5.3).
7. **Keyboard events** (held keys), then **Key Press**, then **Key Release**
   (§11.3). (The manual lists these three together; the press/release relative
   order to held events is UNSPECIFIED – recommended as written.)
8. **Mouse events** (§11.5).
9. **Step triggers**, then **Step** (normal) events.
10. **Motion update** for every instance (§7.4): friction, gravity, path
    advance (with End of Path events), then `x += hspeed; y += vspeed` for
    non-path instances.
11. Boundary events for every instance: **Outside Room**, **Intersect Boundary**,
    and, when views are enabled, **Outside View n** / **Intersect View n Boundary**
    for each view n (§5.5).
12. **Collision events** (§8.6).
13. **End-step triggers**, then **End Step** events.
14. Remove instances marked as destroyed from the instance list (§5.6).
15. Update particle systems with automatic update on (§16).
16. Update views (follow object) (§12.2).
17. **Draw** (§13), unless `set_automatic_draw(false)`; present the frame.
18. Advance animation for every instance: `image_index += image_speed`; wrap and
    raise **Animation End** (§7.3).
19. Scroll backgrounds: `background_x[i] += background_hspeed[i]`,
    `background_y[i] += background_vspeed[i]`.
20. Clear per-step input edges (pressed, released, wheel).
21. If a room change / end / restart / load is pending: perform it (§3).

Rationale for 15–18: draw events see the state after end step; animation end is
observed at the end of the step in GM8. Positions of 3, 4, 15, 16, 18, 19 are the
least certain (see OPEN_QUESTIONS).

### 4.3 Iteration rules for an event phase
For an event phase "deliver event E to all instances":

* Instances are visited in instance-list (creation) order.
* Only instances whose object has an *effective* handler for E (own or inherited,
  §6) receive it. Others are skipped (no cost).
* Deactivated instances and instances already marked destroyed are skipped.
* **UNSPECIFIED – recommended:** instances created during the phase are appended
  to the list and *are* visited in the same phase when the iteration reaches them
  (iterate by index over a growing list). Instances destroyed during the phase are
  skipped from then on.
* After each handler returns, check for a pending room change (§3.1).

### 4.4 Event context
During any event, `self` = the receiving instance; `other` = the other instance in
collision events (and inside `with`, the previous `self` `[401_19]`); otherwise
**UNSPECIFIED – recommended:** `other` = `self`. Read-only `event_type`,
`event_number`, `event_object` (the object whose handler is running — may be an
ancestor), `event_action` (index of the running action) `[403_10]`.

---

## 5. Events

### 5.1 Event types and numbers
Type constants (`[403_10]` names; **UNSPECIFIED – recommended numeric values**,
which must match the game-file encoding — the file-format spec is authoritative):

| constant | value | sub-numbers |
|---|---|---|
| `ev_create` | 0 | 0 |
| `ev_destroy` | 1 | 0 |
| `ev_alarm` | 2 | 0–11 |
| `ev_step` | 3 | `ev_step_normal`=0, `ev_step_begin`=1, `ev_step_end`=2 |
| `ev_collision` | 4 | other object index |
| `ev_keyboard` | 5 | key code |
| `ev_mouse` | 6 | see below |
| `ev_other` | 7 | see below |
| `ev_draw` | 8 | 0 |
| `ev_keypress` | 9 | key code |
| `ev_keyrelease` | 10 | key code |
| `ev_trigger` | 11 | trigger index |

`ev_mouse` numbers: left/right/middle button 0/1/2, no button 3, left/right/middle
press 4/5/6, left/right/middle release 7/8/9, mouse enter 10, mouse leave 11,
joystick1 left/right/up/down 16/17/18/19, joystick1 button1..8 21..28, joystick2
left/right/up/down 31..34, joystick2 button1..8 36..43, global left/right/middle
button 50/51/52, global press 53/54/55, global release 56/57/58, wheel up 60,
wheel down 61.

`ev_other` numbers: outside 0, boundary 1, game start 2, game end 3, room start
4, room end 5, no more lives 6, animation end 7, end of path 8, no more health 9,
user0..user15 = 10..25, close button 30, outside view n = 40+n, boundary view n =
50+n (n = 0..7).

Key codes are Windows virtual-key codes (`vk_left` = 37 etc.); `vk_nokey` = 0,
`vk_anykey` = 1. Letters use `ord('A')`..; digits `ord('0')`.. `[404_01]`.

### 5.2 Event semantics summary
| Event | When |
|---|---|
| Create | immediately when an instance is created (room build, `instance_create`, `instance_copy(true)`, `instance_change(...,true)`, `room_instance_add`ed instances on room build). |
| Destroy | immediately when destroyed by `instance_destroy`, `position_destroy`, Destroy action, `instance_change(...,true)` (old object's destroy). Runs *before* removal; the instance still exists during it `[111]`. **Not** run when instances are removed by a room change `[210_01]` or restart. |
| Alarm n | §5.3. |
| Begin step / Step / End step | §4.2. |
| Keyboard / Key press / Key release | §11.3. |
| Mouse | §11.5. |
| Collision | §8.6. |
| Outside room / Intersect boundary / view variants | §5.5. |
| Game start / Game end / Room start / Room end | §3. |
| No more lives / No more health | §5.4. |
| Animation end | §7.3. |
| End of path | §10.1. |
| Close button | §4.2 step 2. |
| User 0–15 | only via `event_user(n)` / `event_perform(ev_other, ev_userN)` `[403_10]`. |
| Draw | §13; only for visible instances `[111]`; once per visible view per step `[403_10]`. |
| Trigger | condition (GML returning true/false) is evaluated per instance at the chosen moment (before begin step / step / end step); if true the trigger event runs for that instance `[208]`. **UNSPECIFIED – recommended:** evaluate the condition once per instance that has a handler for that trigger, with `self` = that instance. |

### 5.3 Alarms
* 12 alarms per instance, `alarm[0..11]` `[111]`.
* Each step, for each alarm i of an instance **whose object has an effective
  handler for alarm i** (the manual: alarms only tick when the event has actions
  `[403_07]`): if `alarm[i] > 0` then `alarm[i] -= 1`; if it thereby reaches `<= 0`,
  the alarm fires.
* Setting an alarm to ≤ 0 switches it off (no event) `[112_03]`.
* **UNSPECIFIED – recommended:** alarms are processed instance by instance, and
  within an instance in order 0..11; before running the event set `alarm[i] = -1`
  so the handler can re-arm it; fractional values are allowed (an alarm set to 1.5
  fires after 2 decrements).

### 5.4 Lives and health
* When `lives` changes from > 0 to ≤ 0, every instance gets **No More Lives**
  `[403_09]`. When `health` becomes ≤ 0, every instance gets **No More Health**
  `[111]`.
* **UNSPECIFIED – recommended:** check at assignment time (the setter compares old
  and new value) and dispatch synchronously after the assignment statement; for
  health use the same "from > 0 to ≤ 0" edge rule.

### 5.5 Boundary events
Using the instance's bounding box (§8.2) and the room rectangle
`[0, room_width) × [0, room_height)`:
* **Outside room**: bbox lies completely outside the room `[111]`.
* **Intersect boundary**: bbox intersects the room border, i.e. it is at least
  partly outside `[111]`.
* View variants test the same against view n's rectangle (`view_xview..`).
* **UNSPECIFIED – recommended:**
  * Evaluated once per step after motion (§4.2 step 10–11), and fired **every
    step** while the condition holds (low confidence: GM8 may fire only on the
    transition into the state — see OPEN_QUESTIONS).
  * An instance without sprite and mask uses the single point (x, y) as its box.
  * Intersect boundary fires when the box is not completely inside the room
    (including when completely outside? — recommended: only when partly inside and
    partly outside).

### 5.6 Destruction semantics
* `instance_destroy()` (or any destroy path): run Destroy event, then mark the
  instance destroyed. From that moment it receives no events, is invisible to
  `with`, `instance_exists`, `instance_number`, collision and position queries.
* It remains in `instance_id[]`/`instance_count` until the end of the step
  `[403_05]`; code must test `instance_exists`.
* Code after `instance_destroy()` in the same script still runs for that instance
  (its variables remain readable).
* **UNSPECIFIED – recommended:** destroying an already-destroyed instance is a
  no-op (no second Destroy event).

### 5.7 Creation, copy and change
* `instance_create(x,y,obj)`: new instance with next id, defaults from object,
  appended to the instance list, Create event executed immediately (inside the
  caller), id returned `[403_05]`.
* `instance_copy(performevent)`: duplicates all variables (built-in and user) of
  `self` except `id`; optionally runs Create `[403_05]`.
* `instance_change(obj, perf)`: if `perf`, run the old object's Destroy; then
  `object_index = obj`, and **UNSPECIFIED – recommended:** take `sprite_index`,
  `mask_index`, `solid`, `visible`, `persistent`, `depth` from the new object while
  keeping position, motion and all user variables `[112_02]`; if `perf`, run the
  new object's Create. The id is unchanged.
* `position_destroy(x,y)` destroys every instance whose sprite/mask contains the
  point; `position_change(x,y,obj,perf)` changes them `[403_05]`. (The Destroy-at
  -Position action uses the bounding box `[112_02]`.)

---

## 6. Parent inheritance and event dispatch

### 6.1 Effective handlers
* Each object may have one parent; parent chains are acyclic `[205]`.
* For event (type, number), the *effective handler* of object O is the handler of
  the nearest object in the chain O, parent(O), parent(parent(O)), … that defines
  that event with at least one action. A child's own definition overrides the
  parent's `[205]`.
* Events without actions are treated as undefined (the IDE also deletes them `[111]`).
* `event_inherited()` (and the "call inherited event" action) runs the effective
  handler of the *parent of the object whose handler is currently executing*
  (i.e. of `event_object`, not of `object_index`), for the same event type/number,
  on the same instance, with the same `other`. If there is no parent or no
  ancestor defines it, it does nothing `[403_10]`.
* `event_perform(type, numb)` runs the effective handler of `self`'s object for
  that event immediately; `event_perform_object(obj, type, numb)` runs `obj`'s
  handler (looked up with inheritance from obj) on `self` `[403_10]`.
  `event_user(n)` = `event_perform(ev_other, ev_user0+n)`.
* **UNSPECIFIED – recommended:** nested event execution saves/restores
  `event_type/event_number/event_object/event_action` and `other`.

### 6.2 Applying to objects means descendants
Actions applied to an object, `with(obj)`, `obj.var` access, `instance_number`,
`instance_find`, `instance_position`, etc., all include instances of descendant
objects `[205]`. Reading `obj.var` returns the value from the *first* matching
instance; writing assigns to all of them `[401_06]`.

### 6.3 Collision events and parents
* An object's collision event with target T fires for collisions with instances
  of T **and of T's descendants** `[205]`.
* Collision events are inherited by child objects like any other event.
* **UNSPECIFIED – recommended resolution** when several candidate handlers exist
  for a pair (self object A, other object B):
  1. Walk A's chain (A first, then its ancestors). For the first object in that
     chain that has any collision event whose target is B or an ancestor of B,
     choose among its collision events the one whose target is nearest to B in B's
     chain (B itself first).
  2. Exactly one handler runs per (instance, other instance, step).

---

## 7. Instance variables and coupling rules

### 7.1 Position and motion coupling
* `x, y`: room position of the sprite origin; (0,0) is the room's top-left;
  y grows downwards `[403_01]`.
* Two equivalent motion representations are kept consistent:
  * Setting `hspeed` or `vspeed` → recompute `speed = sqrt(hspeed² + vspeed²)` and
    `direction = point_direction(0,0,hspeed,vspeed)` (degrees, counter-clockwise,
    0 = right, i.e. `atan2(-vspeed, hspeed)` normalised to [0,360)).
  * Setting `speed` or `direction` → `hspeed = speed·cos(direction°)`,
    `vspeed = −speed·sin(direction°)`.
* **UNSPECIFIED – recommended:**
  * `direction` is normalised into [0, 360) on assignment.
  * When `hspeed = vspeed = 0` after setting them, keep the old `direction`.
  * A negative `speed` is stored as given (direction unchanged) and the
    components are computed from it; reading back `speed` returns the stored
    value. (The manual says speed should be non-negative `[403_01]`.)
  * Round components whose magnitude is < 1e-10 to 0 to avoid drift.
* `motion_set(dir, sp)`: `direction = dir; speed = sp`.
* `motion_add(dir, sp)`: vector addition: `hspeed += sp·cos(dir)`,
  `vspeed −= sp·sin(dir)` then recompute speed/direction `[403_01]`.

### 7.2 xprevious / yprevious / xstart / ystart
* `xprevious/yprevious` hold the position at the start of the current step (set in
  §4.2 step 3), used by solid collision handling.
* `xstart/ystart` = creation position; never changed by the runtime (user may
  assign). "Jump to start" moves there `[112_01]`.

### 7.3 Animation
* `image_number*` = subimage count of `sprite_index` (0 if no sprite).
* `image_index` may be fractional; the drawn subimage is
  `floor(image_index) mod image_number` `[405_01]`.
* Changing `sprite_index` does **not** reset `image_index` `[405_01]`.
* Each step (§4.2 step 18): `image_index += image_speed`. If it becomes
  `≥ image_number`, subtract `image_number` (wrap) and fire **Animation End**
  `[111]`. **UNSPECIFIED – recommended:** with negative `image_speed`, if it becomes
  `< 0`, add `image_number` and fire Animation End; with `image_number = 0` do not
  advance; the wrap subtracts once (not a modulo).

### 7.4 Per-step motion update
For every active instance, in instance order (**UNSPECIFIED – recommended**
details; the manual only defines friction and gravity qualitatively `[112_01]`):

1. If on a path (`path_index ≥ 0`): advance the path (§10.1); speed/friction/
   gravity are ignored for path motion `[212]`. Skip steps 2–4.
2. Friction: if `friction ≠ 0` and `speed ≠ 0`: reduce `|speed|` by `friction`
   (keeping its sign); if `|speed| ≤ friction` set `speed = 0` (keep direction).
3. Gravity: `hspeed += gravity·cos(gravity_direction)`,
   `vspeed −= gravity·sin(gravity_direction)` (270 = down).
4. `x += hspeed; y += vspeed`.

### 7.5 Sprite/size variables
* `sprite_width* = sprite width × image_xscale`, `sprite_height* = height ×
  image_yscale` (scaling changes the image width/height `[405_01]`).
  `sprite_xoffset/yoffset*` = the sprite origin. **UNSPECIFIED – recommended:**
  offsets are *not* scaled; for no sprite all are 0.
* `image_xscale/yscale` may be negative (mirror/flip) `[405_01]`.
* `image_angle` rotates counter-clockwise around the origin.
* `image_blend`, `image_alpha` affect the default draw only (and `draw_self`).
* `depth`: drawing order (§13.2); changing it at runtime takes effect at the next
  draw.
* `mask_index`: sprite used for collisions; −1 = use `sprite_index` `[403_05]`.

---

## 8. Collision model

### 8.1 Masks of sprites
Per sprite (`[202]`, `[409_01]`):
* `sepmasks`: one mask per subimage, or one combined mask (union over subimages).
* bbox mode: 0 automatic (tight box of mask pixels, honouring alpha tolerance; per
  subimage if separate), 1 full image, 2 user-defined (`bbleft,bbtop,bbright,
  bbbottom`; a single box for all subimages even with separate masks).
* kind: 0 precise (pixels with alpha > tolerance, restricted to the bbox),
  1 rectangle (whole bbox), 2 disk/ellipse inscribed in the bbox, 3 diamond inscribed
  in the bbox.
* tolerance 0..255: a pixel belongs to the mask iff its alpha > tolerance
  (**UNSPECIFIED – recommended** comparison: strictly greater).
* "Precise collision checking" unchecked = kind 1 (bounding box) `[202]`.
* `sprite_get_bbox_*` return the sprite-local box (inclusive pixel coordinates).

### 8.2 Instance bounding box
The collision sprite is `mask_index` if ≥ 0, else `sprite_index`. If neither
exists, the instance has no box and never collides.

**UNSPECIFIED – recommended computation** (well-known GM8 convention; integer,
inclusive edges):
* Without rotation: with `xs = image_xscale`, `ys = image_yscale`, sprite origin
  `(ox,oy)`, local box `[l..r]×[t..b]`:
  `x1 = x + xs·(l − ox)`, `x2 = x + xs·(r + 1 − ox)`; `bbox_left = floor(min(x1,x2))`
  (round to nearest), `bbox_right = ceil-ish(max(x1,x2)) − 1`; same vertically.
* With rotation: rotate the four corners of the scaled local box around the
  origin, take the min/max, convert to inclusive integer edges as above.
* `bbox_*` are recomputed lazily whenever x, y, sprite/mask, image_index (for
  separate masks), scale or angle change.

### 8.3 Instance–instance test
Two instances collide iff their boxes overlap and:
* if both masks are non-precise rectangles (kind 1) and there is no rotation: box
  overlap suffices;
* otherwise, for every integer room pixel (px,py) in the box intersection, map the
  pixel centre into each instance's sprite-local coordinates by the inverse of
  (translate origin → scale → rotate) and test that instance's mask (kind 0 =
  mask bit; kind 2/3 = geometric test inside the local box). Collision iff some pixel
  is inside both.
* The mask subimage used is `floor(image_index) mod image_number` when masks are
  separate, else the combined mask.
* An instance never collides with itself.
* Visibility does not matter: invisible instances collide `[110]`, `[405_01]`.
  What is drawn does not matter: collisions always use the sprite/mask `[111]`.

### 8.4 Shape queries
`collision_point/rectangle/circle/ellipse/line(..., obj, prec, notme)` return an id
of a colliding instance or a negative value (`noone`) `[403_04]`. `prec` requests
per-pixel testing, honoured only if the instance's mask is precise; otherwise its
bbox is used. `notme` excludes the caller. **UNSPECIFIED – recommended:** return
the first match in instance order.

### 8.5 Place / position queries
* `place_free(x,y)`: no **solid** instance collides with self moved to (x,y)
  `[403_01]`.
* `place_empty(x,y)`: no instance at all collides.
* `place_meeting(x,y,obj)`: some instance of obj (or id/all/other) collides.
* `instance_place(x,y,obj)`: the first such instance, else `noone`.
* `position_empty(x,y)`, `position_meeting(x,y,obj)`, `instance_position(x,y,obj)`:
  point tests against masks.
* All "move self to (x,y)" tests temporarily set x,y, test, and restore (no events).

### 8.6 Collision event phase
Manual rules `[111]`, `[212]`:
* A collision event fires when two instances overlap; each involved instance gets
  its own event (if it has one), with `other` = the partner.
* If no handler exists for a pair, nothing happens: instances keep moving even
  through solid instances.
* With a handler and a **solid** other: the instance is put back to its previous
  position, the event runs, then the instance is moved to its new position; if it
  still collides, it is kept at the previous position (effectively stops). Path
  followers additionally do not advance `path_position` in that case.
* With a non-solid other: no position change, the event just runs; no re-check.
* 8.0 fixed a bug where mutual collision events ran twice `[new]`: each pair runs
  each side's event once per step.

**UNSPECIFIED – recommended algorithm** (each ordered pair handled by the event
owner, matching "one event for each instance"):
```
for each active instance A in instance order:
    for each active instance B != A in instance order:
        if A or B destroyed/deactivated: continue
        h = effective collision handler of A's object for B's object   (§6.3)
        if h is None: continue
        if not collide(A, B): continue          # uses current positions
        if B.solid:                             # manual rule: "other is solid"
            A.x, A.y = A.xprevious, A.yprevious
        run h on A with other = B
        if B.solid and A still exists:
            A.x += A.hspeed; A.y += A.vspeed    # "moved to its new position"
            if collide(A, B):
                A.x, A.y = A.xprevious, A.yprevious
```
Notes / open points:
* The manual only names the *other* instance's solidity. GM8 is widely believed to
  also apply the put-back when the event owner itself is solid, and to move *both*
  instances back; recommended to start with the rule above and verify by black-box
  test (OPEN_QUESTIONS).
* "Moved to its new position" is interpreted as re-applying this step's
  hspeed/vspeed from the previous position (so a handler that reverses the motion
  makes the instance bounce without stopping `[111]`). For path followers the new
  position is the path position instead, and on a blocked result `path_position`
  is reset to `path_positionprevious` `[212]`.
* Positions changed by A's handler are visible to later pairs in the same phase.

---

## 9. Motion helper functions
(Behaviour from `[403_01]`, `[112_01]`; numeric details UNSPECIFIED.)

* `move_towards_point(x,y,sp)`: `direction = point_direction(self.x,self.y,x,y)`,
  `speed = sp`. It does not stop at the target `[112_01]`.
* `place_snapped(h,v)`, `move_snap(h,v)` (round x,y to multiples), `move_random(h,v)`
  (random free position w.r.t. solids, snapped if h/v > 0; **recommended:** try up
  to 100 random positions, give up silently).
* `move_wrap(hor,vert,margin)`: if the origin is more than `margin` outside the room
  on a side, move it to the opposite side by ± (room size + 2·margin?) —
  **recommended:** `if x < −margin: x += room_width + 2·margin` (and symmetric).
  The Wrap action uses the motion direction to decide `[112_01]`.
* `move_contact_solid(dir,maxdist)` / `move_contact_all`: if already colliding, do
  nothing; else step 1 pixel at a time in `dir` up to `maxdist` (negative =
  unlimited; **recommended:** 1000 px cap) and stop at the last free position.
* `move_outside_solid(dir,maxdist)` / `move_outside_all`: if not colliding, do
  nothing; else step in `dir` until free or maxdist reached.
* `move_bounce_solid(adv)` / `move_bounce_all(adv)`: reflect the motion against
  the obstacle. **Recommended:** non-advanced: if place at (x+hspeed, y) is blocked
  negate hspeed, if (x, y+vspeed) blocked negate vspeed, if only the diagonal is
  blocked negate both; advanced: probe directions around the motion to estimate the
  surface normal and reflect. Must not move the instance itself.
* `distance_to_point(x,y)`: distance from the bbox (0 if inside);
  `distance_to_object(obj)`: min bbox-to-bbox distance to instances of obj.
* Motion planning (`[403_03]`): `mp_linear_step[_object]` (step of `stepsize`
  straight toward goal, stop on obstacle, return reached), `mp_potential_step
  [_object]` + `mp_potential_settings(maxrot=30, rotstep=10, ahead=3, onspot=true)`
  (steer around obstacles by testing directions left/right of the goal direction in
  rotstep increments, max change maxrot per step, look-ahead `ahead` steps; on
  failure rotate on the spot if onspot), `mp_linear_path`, `mp_potential_path(...,
  factor)` (simulate the step functions, recording a path; fail if length exceeds
  factor × straight distance), grid A*: `mp_grid_create(left,top,hcells,vcells,
  cw,ch)`, `mp_grid_destroy`, `mp_grid_clear_all/clear_cell/clear_rectangle`,
  `mp_grid_add_cell/add_rectangle/add_instances(id,obj,prec)`, `mp_grid_path(id,
  path,xs,ys,xg,yg,allowdiag)` (shortest path through cell centres, diagonal moves
  optional; start/goal cells must be free), `mp_grid_draw` (green free, red blocked).
  A full algorithmic spec belongs to a separate document.

---

## 10. Paths and timelines

### 10.1 Path following
`path_start(path, speed, endaction, absolute)` `[403_02]`, `[212]`:
* Sets `path_index`, `path_speed = speed`, `path_endaction`, `path_position = 0`
  (or 1 when speed < 0). Sets `speed = 0` (normal motion is ignored on a path).
* absolute: the path's own coordinates are used, the instance jumps to the start
  (end if speed < 0). Relative: the path is translated so its start (end for
  negative speed) lies on the current position.
* Each path point has a speed factor (percent); the effective speed is
  interpolated between points.
* Each step (§4.2 step 10): `path_position += path_speed × factor(pos) /
  scaled_length`; x,y = point at `path_position` transformed by `path_scale` and
  rotated by `path_orientation` (counter-clockwise) — **UNSPECIFIED – recommended
  pivot:** the path's start point (absolute or relative anchor). `direction` is set
  to the direction of travel each step `[212]`.
* When the end is reached (`path_position ≥ 1`, or `≤ 0` for negative speed):
  * 0 stop: clamp, `path_index = -1` (end path);
  * 1 restart: jump back to the start position (for an unclosed path) and
    continue, carrying any overshoot;
  * 2 continue from current position: re-anchor the path at the current position
    (same as 1 for a closed path);
  * 3 reverse: `path_speed = −path_speed`.
  In every case an **End of Path** event fires (`ev_other`/`ev_end_of_path`).
* `path_end()` stops following (no event). Closed paths connect the last point to
  the first; smooth paths use a curve through the control points (curve definition
  belongs to the path-resource spec).
* Collisions while on a path behave like normal motion; on a blocked solid
  collision `path_position` is not advanced; `path_positionprevious` lets the user
  undo the advance `[212]`.

### 10.2 Timelines
Variables `[403_07]`: `timeline_index` (−1 = none; assignment does not start it),
`timeline_position`, `timeline_speed` (may be fractional or negative),
`timeline_running`, `timeline_looping`. Actions: set (with start position, start
immediately?, loop?), position, speed, start, pause, stop (= pause and position 0)
`[206_02]`.

**UNSPECIFIED – recommended per-step algorithm** (step 4 of §4.2):
```
if timeline_running and timeline_index valid:
    p0 = timeline_position; p1 = p0 + timeline_speed
    if speed > 0: run every moment m with p0 <= m < p1 in increasing order
    if speed < 0: run every moment m with p1 < m <= p0 in decreasing order
    timeline_position = p1
    if speed > 0 and p1 > last_moment:
        if looping: timeline_position = 0 (minus nothing; no overshoot carry)
        # else stays; running remains true
    if speed < 0 and p1 < 0 and looping: timeline_position = last_moment
```
Moment actions run on the instance as if they were an event (with
`event_type = ev_other`? recommended: a dedicated internal type). Negative moments
never run `[213]`. Several moments in one step all run in order `[403_07]`.

---

## 11. Input model

### 11.1 Keyboard state
Per key code (0..255) the runtime keeps `held`, `pressed` (went down since the
previous step) and `released` (went up since the previous step).
* `keyboard_check(k)` = held; `_pressed` / `_released` = the edges since the last
  step `[404_01]`. `vk_anykey` = any key held/pressed/released; `vk_nokey` = no key
  held (and for pressed/released: none).
* `keyboard_check_direct(k)` reads the physical keyboard regardless of focus;
  supports `vk_lshift, vk_lcontrol, vk_lalt, vk_rshift, vk_rcontrol, vk_ralt`.
* `keyboard_key` = current key (0 if none); `keyboard_lastkey` = last pressed key
  (assignable); `keyboard_lastchar` = last typed character (string);
  `keyboard_string` = the last ≤ 1024 printable characters typed, Backspace removes
  the last character; assignable.
* `keyboard_set_map(k1,k2)` makes key k1 act as k2; `keyboard_get_map`,
  `keyboard_unset_map()`. **Recommended:** apply the map when OS key messages are
  translated into state.
* `keyboard_clear(k)`: clear the key's state; it produces no keyboard events until
  it starts repeating (i.e. until the next OS key-down/auto-repeat message).
* `keyboard_key_press(k)` / `keyboard_key_release(k)`: simulate.
* `keyboard_get_numlock/set_numlock`. Numeric-keypad keys produce their events
  only with NumLock on `[111]`.
* `keyboard_wait()`: block until a key is pressed (pumps messages, no steps).
* `io_clear()`: clear all keyboard and mouse states; `io_handle()`: pump messages
  and update the state mid-step `[404_01]`.

### 11.2 Mouse state
`mouse_x*`, `mouse_y*` (room coordinates, through the view the cursor is in),
`mouse_button` (current; `mb_none/mb_left/mb_middle/mb_right/mb_any`; assignable),
`mouse_lastbutton`, `mouse_check_button[_pressed|_released](b)`,
`mouse_wheel_up()/down()` (since last step), `mouse_clear(b)` (no further events
until released and pressed again), `mouse_wait()`, `cursor_sprite` (sprite drawn
at the mouse position; −1 none) `[404_02]`.
**UNSPECIFIED – recommended:** `mb_any = -1`, `mb_none = 0`, `mb_left = 1`,
`mb_right = 2`, `mb_middle = 3`.

### 11.3 Keyboard event phase (§4.2 step 7)
For every instance (instance order) with relevant handlers:
* **Keyboard**: for each key code currently held (ascending code order, recommended),
  fire `ev_keyboard`/k; additionally `vk_anykey` if any key is held, `vk_nokey`
  if none `[111]`. Fires every step while held.
* **Key press**: for each key in the pressed set, fire `ev_keypress`/k (plus
  anykey/nokey analogues).
* **Key release**: likewise for the released set.
* **UNSPECIFIED – recommended loop nesting:** for each key, for each instance
  (key-major), as GM8 conceptually dispatches "an event per key to all instances".

### 11.4 Joystick
Up to two joysticks `[404_03]`: `joystick_exists, _name, _axes, _buttons,
_has_pov, _direction (vk_numpad1..9), _check_button(id,1..32), _xpos.._vpos
(−1..1), _pov (degrees or −1)`. Joystick events (directions, buttons 1–8 for
each stick; diagonals give both directions) are mouse-type events delivered to all
instances each step while active `[111]`.

### 11.5 Mouse event phase (§4.2 step 8)
For every instance:
* "Mouse over" = the point (`mouse_x`, `mouse_y`) lies inside the instance's
  sprite/mask (precise if the mask is precise) `[111]`.
* If over: left/right/middle **button** events every step while held; **no button**
  if none held; **press** events on the down edge; **release** on the up edge.
* **Mouse enter** / **mouse leave** on transitions of "over" since the previous step.
* **Global** button/press/release events fire regardless of position.
* **Wheel up/down** events when the wheel moved since last step
  (**UNSPECIFIED:** recommended delivered to all instances, independent of
  position).
* **UNSPECIFIED – recommended:** with views, the "mouse over" test uses
  `mouse_x/mouse_y` as computed for the view under the cursor.

---

## 12. Window, region and views

### 12.1 Region and window size
* The *drawing region* has size: if views are enabled, the bounding box of the
  ports of views visible at room start; otherwise the room size `[405_11]`.
  It is decided at room start and not automatically adapted afterwards;
  `window_set_region_size(w,h,adapt)` changes it `[405_11]`.
* The region is drawn in the window centred, scaled per the scaling setting /
  `window_set_region_scale(scale, adapt)`: positive = fixed factor, 0 = fill
  window, negative = largest aspect-preserving fit `[405_10]`, `[302_01]`. The
  area outside the region uses the outside colour (`window_set_color`).
* Window API (`[405_10]`): visible, fullscreen, showborder, showicons, stayontop,
  sizeable, caption, cursor (`cr_*` constants), colour, position/size/rectangle,
  center, default, mouse in window coordinates. In fullscreen, border and icons are
  never shown.
* Caption: `room_caption`, plus score/lives/health parts when `show_*` are true:
  **UNSPECIFIED – recommended format:** `room_caption + "  " + caption_score +
  string(score)` etc., parts separated by two spaces, lives/health only when
  enabled.
* `display_*` functions query/set the screen mode; `display_reset(aa)` resets the
  mode and anti-aliasing (0,2,4,8) and destroys all surfaces; `display_aa` bit set
  of supported levels `[405_09]`.

### 12.2 Views
Per view n (0–7) `[405_11]`: `view_visible`, `view_xview/yview/wview/hview` (room
area), `view_xport/yport/wport/hport` (region area), `view_angle`
(counter-clockwise), `view_hborder/vborder`, `view_hspeed/vspeed` (−1 =
unlimited), `view_object` (object → first instance of it; or a specific instance
id). Global `view_enabled`; `view_current*` (view being drawn, valid in Draw).

**Oversized borders (verified against user projects):** when `2*hborder >= wview`
(resp. `2*vborder >= hview`) the two edge rules contradict each other; applying them
against the previous view position makes the view jump between two positions every
step (visible flicker in pokemonEngine15, Amathyst, JetpackGame, which all ran fine
in GM 8.1). On such an axis the view is centred on the instance (`xview = cx - wview/2`),
which also matches the common "border = half the view keeps the player centred" idiom.

**UNSPECIFIED – recommended follow algorithm** (run once per step before draw):
```
inst = target instance (first in instance order) ; if none: skip
cx, cy = inst.x, inst.y
tx = xview ; if cx - hborder < xview: tx = cx - hborder
             if cx + hborder > xview + wview: tx = cx + hborder - wview
(same for y with vborder)
if hspeed >= 0: tx = xview + clamp(tx - xview, -hspeed, hspeed)
(same for y)
clamp tx to [0, room_width - wview], ty to [0, room_height - hview]
         (only if the room is larger than the view)
xview, yview = tx, ty
```

### 12.3 Mouse coordinates with views
`mouse_x/mouse_y` are relative to the view the cursor is in `[405_11]`.
**UNSPECIFIED – recommended:** search visible views from the highest index down
and take the first whose port contains the cursor; if none, use view 0 (or the
whole room if views are disabled). `window_view_mouse_get_x/y(id)`,
`window_view_mouse_set`, `window_views_mouse_get_x/y`, `window_views_mouse_set` as
in `[405_11]`.

---

## 13. Drawing model

### 13.1 Frame structure
Drawing happens into a back buffer, presented at the end of the step (double
buffering) `[405_12]`. Per frame:

1. Clear the whole window to the outside colour.
2. If views are disabled: one pass with the room rectangle mapped to the region.
   If enabled: one pass per *visible* view in index order 0..7, each mapping
   `(xview,yview,wview,hview,view_angle)` onto `(xport,yport,wport,hport)`;
   overlapping ports are painted in that order `[210_03]`. `view_current` is set.
3. Per pass:
   1. If `background_showcolor`, fill the pass area with `background_color`.
   2. Draw each visible background i = 0..7 that is *not* a foreground at
      (`background_x`, `background_y`), with `xscale/yscale`, tiling horizontally/
      vertically if set (tiled over the visible area), stretched over the room if
      the stretch flag was set in the editor, with `background_blend`/`alpha`.
   3. Draw the depth-sorted list (§13.2): instances (Draw event or default draw),
      tiles, particle systems with automatic draw.
   4. Draw visible foregrounds 0..7 the same way as backgrounds.
   5. **UNSPECIFIED – recommended:** draw `cursor_sprite` at the mouse position as
      the last thing in each pass (image advancing with a per-game counter).
4. Present. `screen_redraw()` performs 1–3 and presents; `screen_refresh()`
   presents the current back buffer only `[405_12]`.

`set_automatic_draw(false)` skips this per-step draw `[405_12]`. Drawing done in
non-draw events goes onto the back buffer in plain room coordinates (no view
transform) and is wiped by the next frame's clear unless refreshed `[405_12]`.

### 13.2 Depth ordering
* Higher depth is drawn first (further away); lower/negative depth later (in
  front) `[205]`.
* Instances with equal depth: in creation order `[205]`.
* Tiles live in layers by depth (editor default 1000000); particle systems have a
  depth; effects created with `effect_create_below` use depth 100000 and
  `effect_create_above` −100000 `[412_01_1explosions]`.
* **UNSPECIFIED – recommended tie rule** between different kinds at the same
  depth: tiles first, then instances, then particle systems.
* Only `visible` instances get Draw `[111]`. Invisible tiles/layers are skipped.

### 13.3 Default draw
If an instance's object has no effective Draw handler, the runtime draws its sprite
as `draw_sprite_ext(sprite_index, image_index, x, y, image_xscale, image_yscale,
image_angle, image_blend, image_alpha)` — exactly what `draw_self()` does
`[405_03]`. Nothing is drawn if `sprite_index < 0`. If a Draw handler exists, the
sprite is not drawn automatically `[111]`.

### 13.4 Draw state
Global, persistent between events and frames (**UNSPECIFIED – recommended initial
values** where not stated):
* colour `draw_set_color` (initial `c_black`), alpha `draw_set_alpha` (1);
  `draw_get_color/alpha` `[405_04]`.
* font `draw_set_font` (−1 = default Arial 12 `[405_05]`), `draw_set_halign`
  (`fa_left`), `draw_set_valign` (`fa_top`).
* `draw_set_circle_precision` (4..64, multiple of 4; initial 24).
* blend mode `draw_set_blend_mode(bm_normal|bm_add|bm_subtract|bm_max)`,
  `draw_set_blend_mode_ext(src,dest)` with factors `bm_zero, bm_one,
  bm_src_color, bm_inv_src_color, bm_src_alpha, bm_inv_src_alpha, bm_dest_alpha,
  bm_inv_dest_alpha, bm_dest_color, bm_inv_dest_color, bm_src_alpha_sat`; normal =
  (src_alpha, inv_src_alpha) `[405_06]`. Blend mode is *not* reset automatically.
* `texture_set_interpolation` (initial from game setting), `texture_set_blending`
  (true), `texture_set_repeat` (false; reset to false whenever a sprite/background
  is drawn) `[405_06]`.
* Colours are 24-bit integers with red in the low byte: `make_color_rgb(r,g,b) =
  r + 256·g + 65536·b` (**UNSPECIFIED**: format well known; constants `c_*` values
  belong to the constants spec).
* The non-`_ext` sprite/background/surface functions draw without blend colour and
  with alpha 1 `[405_03]`; `_ext/_general` take colour/alpha arguments; the
  `_general` rotation pivot is the top-left of the part.
* Text: `#`, CR and LF are newlines; `\#` is a literal `#` `[405_05]`.
* `draw_text*` uses the draw colour and alpha.
* `draw_clear(col)` / `draw_clear_alpha(col,a)` clear the whole target ignoring
  blending `[405_04]`.

### 13.5 Interpolation and scaling
"Interpolate colours between pixels" toggles bilinear filtering for sprites,
backgrounds, tiles `[302_01]` (same flag as `texture_set_interpolation`).

---

## 14. Surfaces and 3D

### 14.1 Surfaces `[405_07]`
* `surface_create(w,h)` returns an id; contents are undefined (not cleared).
* `surface_set_target(id)` redirects all drawing and sets an orthographic
  projection covering exactly the surface; `surface_reset_target()` returns to the
  back buffer (restoring the projection of the current view pass).
* Surfaces may be lost (display reset, mode change, screensaver, Ctrl-Alt-Del);
  `surface_exists` must be checked. Not saved in save games.
* Changing the target inside Draw events is discouraged by the manual (projection
  problems); a clean-room implementation should nonetheless support it correctly by
  saving and restoring the projection.
* `surface_getpixel`, `surface_save[_part]` (PNG), `surface_get_texture`,
  `draw_surface*` (same family as sprites), `surface_copy[_part]` (no blending).

### 14.2 3D mode summary `[415]`
* `d3d_start()` / `d3d_end()`. Starting 3D enables a 24-bit depth buffer
  (`d3d_set_hidden`) and a perspective projection (`d3d_set_perspective`); the
  default camera sits above the room at a distance equal to the room/view width,
  looking down the negative z-axis to the room centre; depth acts as z; instances
  at depth 0 appear at normal size.
* Each frame (each view pass) the projection is reset to the default before
  drawing; user code sets it in Draw (`d3d_set_projection`, `_ext` with fov/aspect/
  znear/zfar defaults 1 and 32000, `_ortho`, `_perspective`).
* In 3D: backgrounds/foregrounds and tiles should not be used, mouse coordinates
  are not transformed, collisions remain 2D, room transitions do not work.
* Primitive drawing, shapes, transforms, fog, lighting and models are covered by a
  separate d3d spec.

---

## 15. Sound model `[406]`
* Sound kinds: normal (many simultaneous copies, effects allowed), background
  music (only one plays at a time; starting one stops the current), 3D (mono,
  positional), multimedia-player (e.g. MP3: one at a time, no effects/volume).
* `sound_play(i)` (once), `sound_loop(i)`, `sound_stop(i)` (all copies),
  `sound_stop_all()`, `sound_isplaying(i)` (true while actually audible; may lag
  behind play/stop calls).
* `sound_volume(i,v)` 0..1, `sound_global_volume(v)`, `sound_fade(i,v,ms)`,
  `sound_pan(i,p)` −1..1, `sound_background_tempo(f)` (MIDI, 0.01..100),
  `sound_set_search_directory(dir)`. Volume can only attenuate.
* Effects `sound_effect_set(snd, flags)` with `se_none, se_chorus, se_echo,
  se_flanger, se_gargle, se_reverb, se_compressor, se_equalizer` (combinable by
  addition) and per-effect parameter setters with documented ranges `[406_02]`.
* 3D sound functions take positions in metres relative to a listener at origin
  `[406_03]`; CD functions `[406_04]` may be stubbed.
* Sound state is independent of rooms (music keeps playing across room changes).
* If no sound device exists the game still runs (sounds are silently ignored)
  `[new]`.

---

## 16. Particles (runtime integration) `[412]`
* Particle systems are global, persist across rooms and restarts until destroyed.
* Each system has a depth (participates in §13.2), a draw position offset
  (`part_system_position`), draw order (old-to-new default), and flags
  `automatic_update` (default on) and `automatic_draw` (default on).
* Automatic update happens once per step (§4.2 step 15; position recommended);
  `part_system_update(ind)` does the same manually; `part_system_drawit(ind)` draws
  manually (inside a Draw event).
* `effect_create_below/above` use an internal system at depth 100000 / −100000;
  `effect_clear()`.
* The particle simulation itself is specified elsewhere.

---

## 17. Pop-ups, splash screens, highscores `[407]`
* `show_message`, `show_message_ext`, `show_question`, `get_integer`,
  `get_string`, `show_menu[_pos]`, `get_color`, `get_open_filename`,
  `get_save_filename`, `get_directory[_alt]`, `show_error(str,abort)`: modal; the
  game loop is suspended until they return; customisation via `message_*`.
* Splash screens (`splash_show_video/text/image/web`) interrupt the game by
  default; closing by Esc, mouse click, delay expiry, or close button; settings via
  `splash_set_*`. `show_info()` shows the game information window.
* Highscore table: 10 entries persisted per game id `[302_04]`; `highscore_show`,
  `_show_ext`, `_set_*`, `_clear`, `_add`, `_add_current`, `_value(1..10)`,
  `_name(1..10)`, `draw_highscore`.
* **UNSPECIFIED – recommended:** after any modal dialog returns, clear the input
  state (`io_clear`) and reset the frame timer so the game does not try to catch up.

---

## 18. Deactivation `[403_06]`
* A deactivated instance is effectively removed: no events (incl. Draw), not drawn,
  ignored by collisions, `with`, all instance functions and by destroy-all
  operations; persistent deactivated instances do not move to the next room.
* `instance_deactivate_all(notme)`, `instance_deactivate_object(obj)` (object, id or
  all), `instance_deactivate_region(l,t,w,h,inside,notme)` (inside = bbox partly
  inside region; not inside = completely outside), and the matching
  `instance_activate_*` functions (which do find deactivated instances).
* **UNSPECIFIED – recommended:** an instance that deactivates itself finishes the
  current code but receives no further events; deactivation takes effect
  immediately for queries; deactivated instances keep their position in the
  creation order; they are discarded when a non-persistent room is left and saved
  in the snapshot of a persistent room.

---

## 19. Error handling `[302_05]`, `[403_11]`
* Runtime errors (e.g. unknown variable, wrong argument type, nonexistent
  resource/instance, division by zero) carry a message identifying object, event,
  action number and code position (`[new]`).
* Settings: "display error messages" (show a dialog), "write to game_errors.log"
  (append to that file in the game folder), "abort on all error messages" (every
  error is fatal), "treat uninitialized variables as 0".
* Fatal vs non-fatal: some errors are fatal (game aborts); others can be ignored.
  **UNSPECIFIED – recommended:** fatal = errors during game loading or that leave
  the interpreter in an inconsistent state; all others non-fatal. The dialog offers
  Abort (end game, no Game End event) and Ignore (continue with the next action;
  the failing action is abandoned).
* If messages are not displayed: non-fatal errors are silently ignored (and
  logged if enabled); `error_occurred = true`, `error_last = message` are set
  either way.
* `show_error(str, abort)` raises a user error through the same path.
* `show_debug_message(str)` writes to the debug console (no-op without debug mode).

---

## 20. Miscellaneous

### 20.1 Score/lives/health and caption
See §1.1, §5.4, §12.1. Whether `show_score` etc. default to true is an open
question; recommended: `show_score = true`, `show_lives = false`,
`show_health = false`.

### 20.2 Timing variables
`current_time*` (ms since system start), `current_year/month/day/weekday(1=Sunday)/
hour/minute/second*`, `sleep(ms)` (blocks; no events, no redraw) `[403_07]`.

### 20.3 Variables-by-name
`variable_global_exists/get/set`, `variable_local_*` incl. array forms
`[403_11]` — interpreter spec.

### 20.4 Program/application
`set_program_priority(−3..3)`, `set_application_title(str)` `[403_11]`.

### 20.5 Executing actions vs code
A Test Expression / conditional is true when the value ≥ 0.5 `[112_04]`; Exit Event
stops the current action list; `event_action*` is the index of the running action.
