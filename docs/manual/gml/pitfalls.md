# Common pitfalls

These are the traps that catch GML programmers most often — beginners and people coming
from other languages alike. Each has a short explanation and a fix.

## Both sides of `&&` and `||` are always evaluated

GML has no short-circuit evaluation. This looks safe but fails when `target` is `noone`:

```gml
if (target != noone && target.hp > 0) {   // reads noone.hp: error
    target.hp -= 1;
}
```

Split the test:

```gml
if (target != noone) {
    if (target.hp > 0) {
        target.hp -= 1;
    }
}
```

The same applies to function calls with side effects: in `a() || b()`, both functions
always run. See [Logical operators](expressions.md#logical-operators-and-truth).

## `&&`, `||` and `^^` have the same precedence

They are evaluated left to right, so `a || b && c` means `(a || b) && c`, not
`a || (b && c)` as in C. Whenever you mix them, add parentheses:

```gml
if (dead || (lives == 0 && !continues_left)) game_over = true;
```

Also note that `&`, `|` and `^` bind **tighter** than comparisons: `flags & 4 == 4` is
`(flags & 4) == 4`. See [the precedence table](expressions.md#operators-and-precedence).

## `=` in a condition compares

Inside an expression `=` means `==`. `if (a = 5)` tests, it doesn't assign — handy, but
it also means:

```gml
a = b = 0;     // a becomes 1 or 0 (whether b equals 0); b is unchanged
```

Write two statements: `a = 0; b = 0;`.

## Numbers that look whole but aren't

All numbers are floating point. Results of divisions, `random()`, speeds and angles are
rarely whole:

- `string(x)` prints `"10.50"` for 10.5 and `"0.33"` for 1/3 — always two decimals for a
  non-whole number. Use `string(round(x))`, `string(floor(x))` or
  `string_format(x, 0, 1)` when you want something else.
- `x / 2` may leave an instance on a half pixel. Collision tests then round, and two
  instances that "should" touch may not. Keep positions whole with `round()` or
  `floor()` where it matters.
- Equality has a tiny built-in tolerance (10<sup>-13</sup>), which hides rounding noise
  such as `0.1 + 0.2 == 0.3`. It does not help for larger differences: after
  `x += 0.1` ten times, compare with `>=` rather than `==`.
- Where an integer is needed (array index, `repeat`, alarms, bitwise operators) the
  value is rounded with halves going to the even neighbour: `2.5` becomes 2, `3.5`
  becomes 4. Round explicitly if you need something else.

## Conditions: below 0.5 is false

A number is true when it is **0.5 or more**. Negative numbers are false, so is 0.4. This
matters when testing a value that may be fractional (`if (alpha)` is false for alpha
0.3). Compare explicitly: `if (alpha > 0)`.

## Strings and numbers don't mix

There is no automatic conversion. `"Score: " + score` is an error, and so is
`"5" == 5`. Convert with `string()` or `real()`. And there are no escape sequences in
strings: `"\n"` is a backslash and an `n`. For a new line in drawn text, use `#`.

## Using a variable before it has a value

*"Unknown variable"* means the variable was read before anything was assigned to it.
Common causes:

- the variable is set in a Step or Alarm event that hasn't run yet;
- it is set in only one branch of an `if` in the Create event;
- another instance reads it before that instance's Create event ran. When a room
  starts, each placed instance runs its Create event in placement order, so the first
  instance can't rely on the second one's variables yet. Do such set-up in Room Start
  or in the room's creation code.

Give every variable a starting value in the Create event. See
[Uninitialised variables](variables.md#uninitialised-variables).

## Scripts and argument counts

With the setting *Throw an error when arguments aren't initialised correctly* on, a
script must be called with exactly as many arguments as it uses. With it off, missing
arguments silently read 0 — so a forgotten argument doesn't cause an error, just a
wrong result. Check `argument_count` in scripts with optional arguments. See
[Checking the argument count](scripts.md#checking-the-argument-count).

A script with the same name as a built-in function replaces that function everywhere.
Prefix your scripts (`scr_`) to avoid it.

## Object or instance?

An object name stands for *all* its instances; an id for exactly one:

- `obj_enemy.hp` **reads the first** enemy's `hp` only.
- `obj_enemy.hp = 0` **writes all** enemies.
- `obj_ball.y += 8` reads the first ball and writes the result to all, so every ball
  ends up at the same height. Use `with (obj_ball) y += 8;`.
- `other` and `self` are the special values -2 and -1. `target = other;` stores -2,
  which later means a different instance. Store `other.id`.
- `other == obj_wall` is always false. Compare `other.object_index == obj_wall`.

See [Addressing other instances](instances.md).

## `other` is not what you expect

- Inside `with`, `other` is the instance that ran the `with`.
- In the Create event of an instance made by `instance_create()`, `other` is the new
  instance itself — **not** the creator. Set values after creating:
  `b = instance_create(x, y, obj_bullet); b.owner = id;`
- Outside collision events and `with`, `other` is usually just the instance itself.

## Destroying instances

- `instance_destroy()` does not stop the code: the statements after it still run.
  Add `exit` if they shouldn't.
- A destroyed instance disappears at once for `with`, `instance_exists()`,
  `instance_number()` and collisions, but stays in `instance_count` and `instance_id[]`
  until the end of the step. Loop with `with` rather than over `instance_id[]` when you
  destroy things.
- A stored id may point to an instance that no longer exists. Reading `id.hp` is then
  an error; check `instance_exists(id)` first.
- Instances removed by a room change get no Destroy event. Put clean-up that must always
  happen in Room End or Game End too.

## Changing rooms doesn't stop your code

After `room_goto()`, the rest of the current action still runs — but the remaining
actions of the event, and the rest of the step, are skipped. Don't put important code
after a room change in the same event; and don't expect `room` to change until the step
is over. See [The game loop](../runtime/game-loop.md#changing-rooms-ending-and-restarting).

## Persistent objects placed in rooms

A persistent instance follows you from room to room. If its object is also placed in
the room where you started, coming back to that room does not create a second copy of
that same placed instance. But a persistent object placed in *several* rooms gives you
one copy per room visited. Put such controllers in the first room only.

## Depth and drawing

- **Higher depth is further back.** The player at depth 0 is drawn over scenery at
  depth 100 and under a HUD at depth -1000.
- **A Draw event replaces the sprite.** Once an object has a Draw event, call
  `draw_self()` if you still want the sprite.
- **Drawing settings leak.** `draw_set_color`, `draw_set_alpha`, `draw_set_font`,
  alignment and blend mode stay set for everything drawn afterwards, by every instance.
  Set them at the start of each Draw event, or reset them at the end.
- **Draw events run once per view.** With two views, a HUD is drawn twice unless you
  check `view_current`.
- **Only draw in Draw events.** Drawing in a Step event is wiped before it is seen.

See [Drawing](../runtime/drawing.md).

## Alarms

- `alarm[0] = 0` switches the alarm off — it never fires. Use 1 for "next step".
- An alarm only counts down if the object has an event for it (or inherits one).
- Alarms hold whole numbers: `alarm[0] = room_speed / 4` rounds.

## Sounds

- `sound_play()` in a Step event while a key is held starts a new copy every step. Play
  sounds on key *press*, or check a flag.
- `sound_isplaying()` lags about a tenth of a second behind. `if (!sound_isplaying(m))
  sound_loop(m)` in a Step event starts several overlapping copies. See
  [Sound and music](../runtime/sound.md#sound_isplaying-lags-behind).

## Small syntax traps

- There is no `++`, `--`, `%` (use `mod`), `?:` or exponent notation (`1e3`).
- `var a = 1;` is a syntax error. Write `var a; a = 1;`.
- `var x;` is an error: locals can't reuse built-in names.
- If the code starts with `{`, nothing may follow the matching `}`.
- `ord("a")` is not the A key; key codes use capitals: `ord("A")`.
- `100032.speed` is a number with a decimal point; write `(100032).speed`.

## Solid instances

- Without a collision event between two objects, nothing happens when they meet — solid
  or not.
- With one, a solid collision puts the instance back but **keeps its speed**, so it
  keeps pushing into the wall every step. Set the speed (or call `move_contact_solid()`,
  `move_bounce_solid()`) in the collision event.

See [Movement and collisions](../runtime/movement-and-collisions.md#solid-instances).

## instance_change mid-code

After `instance_change()`, the rest of the running code still sees the instance as its
old object, and changes it makes to the instance are lost. Make `instance_change` the
last thing you do. See [Changing an instance's object](../runtime/instances.md#changing-an-instances-object).

## Saved games don't include everything

`game_save()` stores instances and variables, but not data structures, particle
systems, surfaces or playing sounds. Save those yourself if they matter. See
[Saving, files and settings](../runtime/files-and-saving.md#saving-and-loading-the-whole-game).

See also: [Expressions and operators](expressions.md) · [Addressing other instances](instances.md) ·
[Errors and debugging](../runtime/errors.md)
