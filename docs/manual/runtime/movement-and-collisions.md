# Movement and collisions

Every instance can move by itself: give it a speed and a direction, and Instance moves
it each step. This page explains how motion is computed, how instances follow paths,
and how collisions are detected and handled.

## Coordinates and directions

The room's origin (0, 0) is its top-left corner. `x` grows to the right and `y` grows
**downwards**. An instance's `x` and `y` are the position of its sprite's origin.

Directions are in degrees, counter-clockwise: 0 is right, 90 is **up**, 180 is left,
270 is down. [`point_direction()`](../reference/computing.md#point_direction),
[`lengthdir_x()`](../reference/computing.md#lengthdir_x) and
[`lengthdir_y()`](../reference/computing.md#lengthdir_y) use the same convention, so
`lengthdir_y(10, 90)` is -10.

## Speed and direction

An instance's motion is stored twice, and the two forms are kept in step:

- `speed` and `direction` — how fast and which way;
- `hspeed` and `vspeed` — the horizontal and vertical parts (pixels per step).

Setting either pair recomputes the other: after `speed = 4; direction = 90;`, `vspeed`
is -4 and `hspeed` is 0. Setting `hspeed = 3` recomputes `speed` and `direction`.

```gml
// Step event: steer with the arrow keys, asteroid style
if (keyboard_check(vk_left))  direction += 4;
if (keyboard_check(vk_right)) direction -= 4;
if (keyboard_check(vk_up))    motion_add(direction, 0.2);
speed = min(speed, 8);
```

[`motion_set(dir, speed)`](../reference/game-play.md#motion_set) sets both at once;
[`motion_add(dir, speed)`](../reference/game-play.md#motion_add) adds a push to the
current motion (vector addition).
[`move_towards_point(x, y, speed)`](../reference/game-play.md#move_towards_point) aims at
a point — it doesn't stop there, so check the distance yourself.

## Friction and gravity

- `friction` is subtracted from the speed every step until the instance stands still.
- `gravity` is added to the motion every step in the direction `gravity_direction`
  (270, straight down, by default).

```gml
// Create event of a platform-game player
gravity_direction = 270;
gravity = 0.5;
```

## What happens each step

After the Step event, every instance that is not on a path:

1. loses `friction` from its speed (stopping at 0);
2. gains `gravity` in `gravity_direction`;
3. moves: `x += hspeed; y += vspeed`.

An instance on a path moves along the path instead and ignores speed, friction and
gravity. Before any of this, at the start of the step, `xprevious`/`yprevious` were
set to the position at that moment.

You don't have to use the built-in motion at all: changing `x` and `y` yourself in the
Step event works fine. The built-in motion just saves code and plays well with
collision handling.

## Paths

A path is a route drawn in the path editor.
[`path_start(path, speed, endaction, absolute)`](../reference/game-play.md#path_start)
sets an instance moving along it:

- `speed` is in pixels per step; a negative speed runs the path backwards.
- `absolute` true uses the path's own coordinates (the instance jumps to its start);
  false moves the path so it starts at the instance's current position.
- `endaction` says what happens at the end: 0 stop, 1 restart from the beginning,
  2 continue from the current position (the path is laid out again from there),
  3 reverse direction.

Each time the end is reached the **End of Path** event fires.
[`path_end()`](../reference/game-play.md#path_end) stops following without an event.
While on a path, `path_position` (0 at the start, 1 at the end), `path_speed`,
`path_scale` and `path_orientation` can be read and changed, and `direction` points
along the path.

## Masks and bounding boxes

Collisions use the instance's **mask**: the sprite in `mask_index`, or its own sprite
when `mask_index` is -1. An instance with neither never collides with anything.

- A sprite with *precise collision checking* collides only where its pixels are
  non-transparent (above the sprite's alpha tolerance).
- Otherwise its whole bounding box counts (or the ellipse or diamond inside it, if the
  sprite uses those shapes).

The variables `bbox_left`, `bbox_top`, `bbox_right` and `bbox_bottom` give the
instance's bounding box in room coordinates, as whole pixels (inclusive). Scaling and
rotation (`image_xscale`, `image_yscale`, `image_angle`) are included. Positions are
rounded to whole pixels when collisions are tested, so an instance at `x = 15.5` and
one at `x = 16` may or may not touch depending on the rounding — keep colliding
things on whole coordinates where it matters.

Visibility and drawing don't matter for collisions: invisible instances collide, and a
Draw event that draws something else doesn't change the mask.

## Collision events

When two instances overlap after moving, each gets its **Collision event** for the
other one's object — if it has one. `other` in the event is the partner. A collision
event for an object also fires for instances of its child objects.

```gml
// obj_player, collision with obj_coin
score += 10;
with (other) instance_destroy();
```

A pair is handled once per step: the first instance's event runs, and immediately
after it the partner's event (if it has one). The order goes object by object: objects
in index order, each object's collision events in the order of their target object,
and the instances in creation order.

### Solid instances

Collision events get special treatment when **either** instance is `solid`:

1. Both instances are put back where they were at the start of the step
   (`xprevious`, `yprevious`) — so in the event they no longer overlap.
2. The collision events run.
3. Both are moved by their speed again. If they still collide, they are put back once
   more.

The speed is *not* changed by this, so an instance that keeps pushing against a wall
simply stays at its last free position each step. That's why a collision event with a
solid wall usually sets the speed itself:

```gml
// obj_ball, collision with obj_wall (solid)
move_bounce_solid(true);
```

```gml
// obj_player, collision with obj_floor (solid): land
if (vspeed > 0) {
    move_contact_solid(270, 12);
    vspeed = 0;
}
```

Without a collision event between two objects, nothing happens — even solid
instances pass straight through each other.

## Checking for collisions in code

Often it is simpler to look before moving. These functions test a position without
moving the instance or raising events:

| Function | True / returns |
|---|---|
| [`place_free(x, y)`](../reference/game-play.md#place_free) | no **solid** instance would collide with this one at (x, y) |
| [`place_empty(x, y)`](../reference/game-play.md#place_empty) | no instance at all would collide |
| [`place_meeting(x, y, obj)`](../reference/game-play.md#place_meeting) | an instance of `obj` (or that id, or `all`) would collide |
| [`instance_place(x, y, obj)`](../reference/game-play.md#instance_place) | the id of such an instance, or `noone` |
| [`position_meeting(x, y, obj)`](../reference/game-play.md#position_meeting), [`position_empty(x, y)`](../reference/game-play.md#position_empty), [`instance_position(x, y, obj)`](../reference/game-play.md#instance_position) | the same for a single point |
| [`collision_point()`](../reference/game-play.md#collision_point), [`collision_rectangle()`](../reference/game-play.md#collision_rectangle), [`collision_circle()`](../reference/game-play.md#collision_circle), [`collision_ellipse()`](../reference/game-play.md#collision_ellipse), [`collision_line()`](../reference/game-play.md#collision_line) | an instance touching the shape, or `noone`; `prec` asks for pixel-precise tests, `notme` skips the caller |

```gml
// Step event: walk, but don't walk into walls
var dx;
dx = (keyboard_check(vk_right) - keyboard_check(vk_left)) * 4;
if (place_free(x + dx, y)) x += dx;
```

## Moving with helpers

- [`move_contact_solid(dir, maxdist)`](../reference/game-play.md#move_contact_solid) moves
  step by step in a direction until just before touching a solid instance;
  [`move_contact_all()`](../reference/game-play.md#move_contact_all) stops at any instance.
- [`move_outside_solid(dir, maxdist)`](../reference/game-play.md#move_outside_solid) moves
  out of a solid instance the instance is stuck in.
- [`move_bounce_solid(adv)`](../reference/game-play.md#move_bounce_solid) and
  [`move_bounce_all(adv)`](../reference/game-play.md#move_bounce_all) reverse the motion
  against what was hit; `adv` true makes the bounce follow slopes better.
- [`move_snap(h, v)`](../reference/game-play.md#move_snap) rounds the position to a grid;
  [`place_snapped(h, v)`](../reference/game-play.md#place_snapped) tests whether it is on one.
- [`move_wrap(hor, vert, margin)`](../reference/game-play.md#move_wrap) wraps an instance
  that left the room around to the other side.
- [`distance_to_object(obj)`](../reference/game-play.md#distance_to_object) and
  [`distance_to_point(x, y)`](../reference/game-play.md#distance_to_point) measure from the
  bounding box.
- For finding a way around obstacles there are the motion-planning functions:
  [`mp_linear_step()`](../reference/game-play.md#mp_linear_step),
  [`mp_potential_step()`](../reference/game-play.md#mp_potential_step) and the grid-based
  [`mp_grid_path()`](../reference/game-play.md#mp_grid_path).

## Leaving the room

Two events watch the room's edges, using the bounding box:

- **Outside Room** fires every step while the instance's box is completely outside the
  room (at least one pixel beyond the edge).
- **Intersect Boundary** fires every step while the box is not completely inside the
  room — so also while it is completely outside.

When both apply in the same step, Outside Room comes first. With views enabled there
are matching **Outside View** and **Intersect View Boundary** events for each view.

```gml
// obj_bullet, Outside Room
instance_destroy();
```

See also: [The game loop](game-loop.md) · [Instances, objects and parenting](instances.md) ·
[Rooms and views](rooms-and-views.md) · [Game play functions](../reference/game-play.md)
