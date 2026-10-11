# Instances, objects and parenting

An **object** describes a kind of thing: its sprite, whether it is solid or visible, its
depth, its parent, and the actions it performs in each event. An **instance** is one
such thing in the room. Ten coins on the screen are ten instances of one `obj_coin`
object, each with its own position and variables.

## Creating instances

Instances come into being in two ways:

- **Placed in the room editor.** They are created when the room is entered (see
  [Rooms and views](rooms-and-views.md#what-happens-when-the-room-changes)). Each runs its
  *instance creation code* from the room editor, then its Create event.
- **Created by code** with [`instance_create(x, y, obj)`](../reference/game-play.md#instance_create)
  or the *Create Instance* action. The new instance's **Create event runs immediately**,
  inside the call, and the function then returns the new instance's id.

```gml
var p;
p = instance_create(x, y - 8, obj_puff);
p.vspeed = -2;      // the Create event of obj_puff has already run
```

Every instance has a unique `id` of 100000 or more. An instance placed in the room
editor gets the id stored with it in the room; instances created by code get fresh
ids. `object_index` tells which object an instance belongs to.

A new instance takes its sprite, mask, visibility, solidity, persistence and depth from
its object. Everything else starts at a neutral value: speeds 0, `direction` 0,
`gravity_direction` 270 (down), `image_speed` 1, `image_xscale`/`image_yscale` 1,
`image_alpha` 1, `image_blend` `c_white`, alarms off (-1), no path, no time line.
`xstart`/`ystart` and `xprevious`/`yprevious` are the creation position.

## The built-in instance variables

| Group | Variables |
|---|---|
| Identity | `id`, `object_index` (read-only) |
| Position | `x`, `y`, `xprevious`, `yprevious`, `xstart`, `ystart` |
| Motion | `speed`, `direction`, `hspeed`, `vspeed`, `friction`, `gravity`, `gravity_direction` — see [Movement and collisions](movement-and-collisions.md) |
| Appearance | `sprite_index`, `image_index`, `image_speed`, `image_number` (read-only), `image_xscale`, `image_yscale`, `image_angle`, `image_alpha`, `image_blend`, `visible`, `depth` |
| Size | `sprite_width`, `sprite_height`, `sprite_xoffset`, `sprite_yoffset`, `bbox_left`, `bbox_right`, `bbox_top`, `bbox_bottom` (read-only) |
| Collisions | `solid`, `mask_index` |
| Timing | `alarm[0..11]`, `timeline_index`, `timeline_position`, `timeline_speed`, `timeline_running`, `timeline_looping` |
| Paths | `path_index` (read-only), `path_position`, `path_positionprevious`, `path_speed`, `path_scale`, `path_orientation`, `path_endaction` |
| Rooms | `persistent` |

A few details:

- `image_index` may be fractional; the drawn sub-image is the whole part, wrapped to the
  number of sub-images. Changing `sprite_index` does not reset `image_index`.
- `image_speed` is added to `image_index` each step, after drawing. When it passes the
  last sub-image it wraps around and the **Animation End** event fires. A negative
  `image_speed` plays backwards but never wraps and never raises Animation End.
- `sprite_width`/`sprite_height` include the scaling.

## Destroying instances

[`instance_destroy()`](../reference/game-play.md#instance_destroy) destroys the instance
running the code (use `with` to destroy others). What happens:

1. The **Destroy event** runs right away. The instance still exists during it.
2. The instance is marked as gone. From then on it gets no events, and `with`,
   `instance_exists()`, `instance_number()` and the collision functions no longer see it.
3. The rest of the code that called `instance_destroy()` still runs, and the instance
   can still read its own variables.
4. It is removed from the instance list at the end of the step. Until then it still
   counts in `instance_count` and `instance_id[]`.

Reading a variable of a destroyed instance from *another* instance (`enemy.hp`) is the
error *"Unknown variable hp"*, so check
[`instance_exists()`](../reference/game-play.md#instance_exists) when you keep ids around.
Calling `instance_destroy()` twice on the same instance in the same step runs its
Destroy event twice.

Instances removed because the room changes, or because the game restarts, get **no**
Destroy event.

## Changing an instance's object

[`instance_change(obj, perf)`](../reference/game-play.md#instance_change) turns the
instance into an instance of another object. It keeps its id, position, motion and
all variables, and takes the new object's sprite, mask, solidity, visibility,
persistence and depth. With `perf` true, the old object's Destroy event and the new
object's Create event run inside the call.

Internally the instance is replaced by a fresh copy, and this shows in one place: the
code that called `instance_change` keeps working on the *old* copy. It still sees the
old `object_index` and sprite, and anything it assigns after the call is lost. Make
`instance_change` the last thing the code does to the instance. Also, `with (id)` finds
nothing for the rest of that step; other code already sees the new object.

[`instance_copy(perf)`](../reference/game-play.md#instance_copy) creates a duplicate of
the current instance with all its variables and a new id, optionally running Create.

## Parents and inheritance

An object can have a **parent** object. The child then inherits:

- **Events.** Every event the child doesn't define itself runs the parent's actions (and
  the grandparent's, and so on). Defining the event in the child overrides it. To run the
  parent's version as well, call [`event_inherited()`](../reference/game-play.md#event_inherited)
  (or the *Call Parent Event* action) from the child's event.
- **Membership.** Anywhere an object stands for "all its instances" — `with (obj)`,
  `obj.var`, [`instance_number()`](../reference/game-play.md#instance_number),
  [`instance_find()`](../reference/game-play.md#instance_find),
  [`place_meeting()`](../reference/game-play.md#place_meeting), the collision functions,
  collision events — the instances of child objects are included.

Parents make shared behaviour easy: give `obj_enemy_parent` the damage and death logic,
make every enemy type its child, and a single Collision event with `obj_enemy_parent`
in the bullet object handles them all.

```gml
// Step event of obj_bat (child of obj_enemy_parent)
event_inherited();          // the shared enemy logic
y += sin(current_time / 200) * 2;   // plus bat-specific flutter
```

Alarms count down for children too when only the parent defines the alarm event.

## Calling events directly

- [`event_user(n)`](../reference/game-play.md#event_user) runs the instance's **User Event
  n** (0–15). User events never happen on their own, which makes them handy as named
  subroutines that children can override.
- [`event_perform(type, number)`](../reference/game-play.md#event_perform) runs any event
  of the instance's object, and
  [`event_perform_object(obj, type, number)`](../reference/game-play.md#event_perform_object)
  runs another object's event on this instance.

These run immediately and keep the caller's `other`. Inside any event, `event_type`,
`event_number`, `event_object` and `event_action` describe what is running.

## Persistent instances

An instance with `persistent` set survives room changes: when the room is left it moves
along to the next room, keeping all its variables and its position. It gets the Room End
and Room Start events but no new Create event. Typical uses are a game controller or a
player that walks from room to room.

If you place a persistent object in a room and come back to that room later, the
existing instance is kept and no second copy is created. A persistent instance is only
removed by destroying it, by [`game_restart()`](../reference/game-play.md#game_restart),
or by deactivating it before the room changes (deactivated instances are left behind).

## Deactivating instances

Deactivating an instance takes it out of the game without destroying it: it gets no
events, is not drawn, doesn't collide, and is ignored by `with` and every instance
function. Activating it brings it back exactly as it was.

```gml
// keep only the instances near the view active
instance_deactivate_region(view_xview[0] - 64, view_yview[0] - 64,
    view_wview[0] + 128, view_hview[0] + 128, false, true);
instance_activate_region(view_xview[0] - 64, view_yview[0] - 64,
    view_wview[0] + 128, view_hview[0] + 128, true);
```

See [`instance_deactivate_all()`](../reference/game-play.md#instance_deactivate_all),
[`instance_deactivate_object()`](../reference/game-play.md#instance_deactivate_object),
[`instance_deactivate_region()`](../reference/game-play.md#instance_deactivate_region) and
the matching `instance_activate_*` functions. Deactivated instances are lost when a
non-persistent room is left, and are kept with a persistent room.

## Finding and counting instances

| You want | Use |
|---|---|
| Is there any instance of an object, or does this id still exist? | [`instance_exists(obj or id)`](../reference/game-play.md#instance_exists) |
| How many instances of an object? | [`instance_number(obj)`](../reference/game-play.md#instance_number) |
| The n-th instance of an object | [`instance_find(obj, n)`](../reference/game-play.md#instance_find) |
| The nearest / furthest instance | [`instance_nearest(x, y, obj)`](../reference/game-play.md#instance_nearest), [`instance_furthest()`](../reference/game-play.md#instance_furthest) |
| The instance at a point / that would collide at a position | [`instance_position()`](../reference/game-play.md#instance_position), [`instance_place()`](../reference/game-play.md#instance_place) |
| All instances, by index | `instance_count` and `instance_id[0 .. instance_count-1]` |

These return `noone` when nothing is found. `instance_find` and `with` visit instances
in creation order.

See also: [Addressing other instances](../gml/instances.md) · [The game loop](game-loop.md) ·
[Objects](../ide/objects.md) · [Events](../ide/events.md)
