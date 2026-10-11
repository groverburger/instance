# Rooms and views

A game is a series of rooms: a title screen, levels, a game-over screen. Only one room
is active at a time. Views let a room be larger than the window and show parts of it,
following the player.

## Moving between rooms

The rooms form an ordered list (the order in the resource tree). The game starts in the
first one.

| Function | Goes to |
|---|---|
| [`room_goto(rm)`](../reference/game-play.md#room_goto), or `room = rm` | a specific room |
| [`room_goto_next()`](../reference/game-play.md#room_goto_next) | the next room in the order (an error in the last room) |
| [`room_goto_previous()`](../reference/game-play.md#room_goto_previous) | the previous room (an error in the first room) |
| [`room_restart()`](../reference/game-play.md#room_restart) | the current room, from scratch |

`room_first`, `room_last`, [`room_next(rm)`](../reference/game-play.md#room_next) and
[`room_previous(rm)`](../reference/game-play.md#room_previous) let you inspect the order;
the latter two return -1 at the ends.

The change does not happen in the middle of your code. The current action finishes, the
rest of the event and the rest of the step are skipped, and then the room changes (see
[The game loop](game-loop.md#changing-rooms-ending-and-restarting)).

## What happens when the room changes

1. Every instance gets the **Room End** event, in creation order.
2. Instances that are not persistent are removed, **without** Destroy events. If the room
   being left is persistent, its state is saved first (see below).
3. The new room's settings are loaded: size, speed, caption, background colour,
   backgrounds, views and tiles.
4. All instances placed in the new room are added — before any of their code runs.
   Persistent instances carried over from the previous room come after them in the
   instance order.
5. For each placed instance, in placement order: its **instance creation code**, then its
   **Create** event. Instances created during these are added at the end.
6. **Game Start**, but only when this is the first room of the game (or after a restart).
7. The room's **creation code**.
8. **Room Start** for every instance, in instance order (placed instances first, then the
   carried-over persistent ones).
9. A frame is drawn, and the first step in the new room begins.

During steps 4–8, `instance_count` already includes every placed instance, even those
whose Create event hasn't run yet. A placed instance should therefore not assume that
another placed instance has been initialised when its own Create event runs; do such
set-up in Room Start or in the room's creation code.

## Persistent rooms

A room marked **persistent** (in the room editor, or with
[`room_persistent`](../reference/game-play.md#room_persistent)` = true`) remembers its
state when you leave it: its instances with all their variables (deactivated ones
too), its tiles, backgrounds and views. When you return:

- the saved instances come back as they were — they get **no** Create event, and their
  instance creation code doesn't run again;
- the room's creation code does **not** run again;
- **Room Start** does run, for the restored instances and the carried-over persistent ones.

Persistent rooms suit places the player revisits, such as a town hub. They are forgotten
on `game_restart()`.

## Transitions

Set [`transition_kind`](../reference/game-play.md#transition_kind) before changing rooms
to get a visual effect between the old and the new room. It applies to the next room
change only and then goes back to 0 (none). `transition_steps` (default 80) sets the
number of frames it lasts.

| Kinds | Effect |
|---|---|
| 1–5 | the new room grows in from the left, right, top, bottom or centre |
| 6–9 | the new room shifts in from the left, right, top or bottom |
| 10–13 | interlaced from the left, right, top or bottom |
| 14–17 | the new room pushes the old one out to the left, right, top or bottom |
| 18, 19 | rotate left or right |
| 20 | blend |
| 21 | fade out and in |

In Instance, the interlaced (10–13) and rotating (18, 19) kinds are shown as kind 5
(grow from the centre). No events run while a transition plays. The browser version
skips transitions.

[`transition_define(kind, script)`](../reference/game-play.md#transition_define) adds a
transition of your own: the script is called once per frame of the transition with two
surfaces holding the old and the new room's picture, their width and height, and the
fraction of the transition done so far (0 to 1).

## Room properties at run time

The current room's values are global built-in variables: `room` (its index),
`room_width`, `room_height` (read-only), `room_speed`, `room_caption`,
`room_persistent`, `background_color`, `background_showcolor`, and the background and
view arrays below. Changing them affects the current room.

Rooms can also be built or edited by code: see the room functions in
[Resources](../reference/resources.md#room_add).

## Backgrounds and tiles

A room has eight background layers. Each is drawn behind the instances, or in front of
them if it is marked as a foreground. Their settings are arrays indexed 0–7:
`background_index[]`, `background_visible[]`, `background_foreground[]`,
`background_x[]`, `background_y[]`, `background_htiled[]`, `background_vtiled[]`,
`background_xscale[]`, `background_yscale[]`, `background_hspeed[]`,
`background_vspeed[]`, `background_blend[]` and `background_alpha[]`.

```gml
// scroll the far background slowly for a parallax effect
background_hspeed[0] = -0.5;
background_hspeed[1] = -2;
```

**Tiles** are pieces of background images placed in the room editor. They are drawn at
their own depth, between instances, and cost much less than instances. The
[`tile_*` functions](../reference/game-graphics.md#tile_add) add, move, hide and remove them
at run time.

## Views

Enable views in the room editor (or with `view_enabled = true`) and the room is shown
through up to eight views. Each view `n` (0–7) maps an area of the room onto an area of
the window:

| Variables | Meaning |
|---|---|
| `view_visible[n]` | whether the view is shown |
| `view_xview[n]`, `view_yview[n]`, `view_wview[n]`, `view_hview[n]` | the area of the **room** that is shown |
| `view_xport[n]`, `view_yport[n]`, `view_wport[n]`, `view_hport[n]` | where it is shown in the **window** |
| `view_angle[n]` | rotation of the view (counter-clockwise, in degrees) |
| `view_object[n]` | the object (its first instance) or instance the view follows |
| `view_hborder[n]`, `view_vborder[n]` | how close the followed instance may come to the view's edge |
| `view_hspeed[n]`, `view_vspeed[n]` | the maximum scroll speed per step, or -1 for no limit |

A view and its port may differ in size; the picture is then scaled. Several views make
split screens or a mini-map.

### Following an instance

Once per step, just before drawing, every view with a `view_object` moves so that the
instance stays at least the border distance away from the view's edges, limited by
the view speed and kept inside the room (when the room is larger than the view). If a
border is half the view size or more, the view keeps the instance centred on that axis.

To control the camera yourself, set `view_object[0] = -1` and assign `view_xview[0]`
and `view_yview[0]` — End Step is a good place, as the instances have moved by then.

```gml
// End Step of a camera object: smooth follow
view_xview[0] += (obj_player.x - view_wview[0] / 2 - view_xview[0]) * 0.1;
view_yview[0] += (obj_player.y - view_hview[0] / 2 - view_yview[0]) * 0.1;
```

### Views and coordinates

- Each visible view is drawn separately, so **Draw events run once per view**.
  `view_current` tells which view is being drawn — check it to draw a HUD only once.
- `mouse_x` and `mouse_y` are in room coordinates, translated through the view the
  mouse cursor is over.
- Things you draw are in room coordinates; to draw a HUD fixed on screen, add
  `view_xview[view_current]` and `view_yview[view_current]` to your positions.

## The window

The game's drawing area is the size of the room, or — with views — the box around the
ports of the views visible at room start. It is decided when the room starts. The
window shows it scaled according to the Global Game Settings (a fixed percentage,
keep aspect ratio, or fill the window); the window functions such as
[`window_set_fullscreen()`](../reference/game-graphics.md#window_set_fullscreen),
[`window_set_size()`](../reference/game-graphics.md#window_set_size) and
[`window_set_region_scale()`](../reference/game-graphics.md#window_set_region_scale)
change this while the game runs.

The window caption is `room_caption`, followed by the score, lives and health if those
are switched on ([Lives, health and score](game-loop.md#lives-health-and-score)).

See also: [The game loop](game-loop.md) · [Drawing](drawing.md) · [Rooms](../ide/rooms.md) ·
[Game graphics functions](../reference/game-graphics.md)
