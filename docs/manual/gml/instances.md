# Addressing other instances: `with`, `other` and object names

Code always runs *as* some instance — `self`. Plain variable names like `x` or `hp`
mean that instance's variables. This page shows how to read and change other
instances: through the `.` operator, with `with`, and with the special value `other`.

## Objects, instances and ids

An **object** is a template (`obj_enemy`); **instances** are the things in the room made
from it. Code refers to them with plain numbers:

| Value | Means |
|---|---|
| An object name, like `obj_enemy` | the object's index (0, 1, 2, …): stands for *all* its instances — and the instances of its child objects |
| An instance id (100000 or more) | exactly one instance. `id` holds an instance's own id; [`instance_create()`](../reference/game-play.md#instance_create), [`instance_nearest()`](../reference/game-play.md#instance_nearest), [`instance_place()`](../reference/game-play.md#instance_place) and friends return ids |
| `self` (-1) | the instance running the code |
| `other` (-2) | the "other" instance (see below) |
| `all` (-3) | every active instance |
| `noone` (-4) | no instance; functions return it when they find nothing |
| `global` (-5) | not an instance: the container of global variables |

Because these are ordinary numbers you can store them in variables, pass them to
scripts and compare them: `if (target == noone)`, `owner = id`.

## The `.` operator

`thing.name` reads or writes the variable `name` of whatever `thing` selects:

```gml
other.hp -= 1;                 // the other instance
global.level += 1;             // a global variable
obj_door.locked = false;       // every obj_door
boss = instance_find(obj_boss, 0);
boss.speed = 0;                // one instance, through its id
```

What happens depends on the left side:

| Left side | Reading `thing.v` | Writing `thing.v = …` |
|---|---|---|
| An instance id | that instance's `v`; error if it doesn't exist | that instance; nothing happens if it doesn't exist |
| An object | `v` of the **first** instance of the object | **all** instances of the object (and of its children) |
| `self`, `other` | that instance | that instance |
| `all` | the first instance | every instance |
| `noone` | error *"Unknown variable v"* | error *"Cannot assign to the variable"* |
| `global` | the global variable | the global variable |

Reading through an object that has no instances, or through an id whose instance is
gone, is the error *"Unknown variable v"*. Writing through an object without instances
silently does nothing.

Compound assignment through an object reads from the first instance and writes the
result to all of them. `obj_ball.y += 8` therefore sets **every** ball to *the first
ball's* `y` plus 8 — usually not what you want. Use `with (obj_ball) y += 8;` to move
each ball by 8.

### Numbers and calls on the left

A number followed directly by a dot is read as a decimal point: `100032.speed` is a
syntax error. Write `(100032).speed`.

A function call may stand on the left of a dot, so
`instance_nearest(x, y, obj_coin).image_blend = c_yellow` works — but be sure the
call can't return `noone`. A call result can't be indexed with `[ ]`.

Only variable names may follow the dot. Constants and resource names can't, so name
your resources so they don't clash with variable names (`global.bb_out` fails to
compile if an object is called `bb_out`).

## with

`with (target) statement` runs the statement once for every instance that `target`
selects, with `self` set to that instance:

```gml
// push every enemy within 64 pixels away from the bomb
var bx, by;
bx = x;
by = y;
with (obj_enemy) {
    if (point_distance(x, y, bx, by) < 64) {
        motion_add(point_direction(bx, by, x, y), 6);
    }
}
```

The target can be:

- an **object** — all its active instances, including instances of child objects;
- an **instance id** — that one instance, if it exists and is active (otherwise nothing);
- `self`, `other`, `all`, or `noone` (which runs nothing).

Inside the `with` block:

- `self` is the instance being visited and **`other` is the instance that ran the
  `with`**. So `other.x` is the outside instance's position and `other.score_gained += 1`
  counts something back to it.
- Local `var` variables of the surrounding code stay usable — the easiest way to hand
  values in.
- In a script, `argument0` etc. still mean the script's arguments.
- `break` stops visiting further instances; `continue` moves on to the next one;
  `exit` leaves the whole code.

With nested `with` blocks, `other` in the inner one is the instance currently visited
by the outer one. When the `with` finishes, `self` and `other` are restored.

### What `with` visits

The list of instances is taken when the `with` starts:

- Instances **created** inside the block are not visited by this `with`.
- Instances **destroyed** (or deactivated) before their turn are skipped.
- Destroying the visited instance itself is fine: `with (obj_bullet) instance_destroy();`
  destroys every bullet.

`with` also lets you call a script or function *as* another instance:

```gml
with (other) instance_destroy();          // destroy the instance we touched
with (instance_nearest(x, y, obj_enemy)) hp -= 5;
```

## What `other` means

| Where | `other` is |
|---|---|
| Inside a `with` | the instance that ran the `with` (the previous `self`) |
| Collision event | the instance being collided with |
| Create event of an instance made by `instance_create()` | **the new instance itself**, not the creator |
| Destroy event | the instance itself |
| `event_user()` / `event_perform()` | the caller's `other`, unchanged |
| Scripts | whatever it was in the code that called the script |
| Other events | the instance itself |

Since a Create event can't see its creator, pass information *after* creating:

```gml
var b;
b = instance_create(x, y, obj_bullet);
b.owner = id;
b.direction = image_angle;
b.speed = 12;
```

The Create event has already run by then, so give `owner` a default value there and
let the creator overwrite it.

## Instance or object? A checklist

- Need **one particular** instance? Keep its id (`target = instance_nearest(...)`) and
  use `target.v` or `with (target)`. Check `instance_exists(target)` first if it
  might be gone.
- Need **every** instance of a kind? Use `with (obj_kind)`.
- Reading `obj_kind.v` gives you the *first* instance's value only. That's fine for an
  object with exactly one instance (a controller, the player), and a source of bugs
  otherwise.
- `other` and `self` are the special values -2 and -1, not ids. `target = other;`
  stores -2, which later means whatever `other` is *then*. Store `other.id` (or `id`)
  to remember a particular instance.
- Comparing `other == obj_wall` is always false for the same reason. Compare
  `other.object_index == obj_wall`, and use
  [`object_is_ancestor()`](../reference/resources.md#object_is_ancestor) if child objects
  should count too.

See also: [Instances, objects and parenting](../runtime/instances.md) ·
[Variables and scope](variables.md) · [Common pitfalls](pitfalls.md)
