# Variables and scope

A variable is a named place that holds a value — a real or a string. In GML you don't
declare ordinary variables: assigning to a name creates it. *Where* it is created
depends on the kind of variable.

| Kind | Written as | Lives for | Seen by |
|---|---|---|---|
| Instance variable | `hp`, `self.hp`, `other.hp`, `inst.hp` | the lifetime of the instance | that instance (others reach it with `.`) |
| Global variable | `global.level`, or a `globalvar` name | the whole game | all code |
| Local variable | declared with `var` | one run of one piece of code | that piece of code only |
| Built-in variable | `x`, `speed`, `score`, `room`, ... | always exists | per instance or global, depending on the variable |

## Instance variables

A plain name you assign to becomes a variable of the instance that runs the code
(`self`):

```gml
// Create event of obj_enemy
hp = 3;
name = "Grunt";
```

Each `obj_enemy` instance now has its own `hp` and `name`. Changing one enemy's `hp`
does not touch the others. Another instance reads or writes them with the `.`
operator — `other.hp`, `obj_enemy.hp`, or `target.hp` where `target` holds an
instance id. See [Addressing other instances](instances.md).

An instance variable only exists on the instances that assigned it. It is common to
give every variable its starting value in the Create event, so the other events can
rely on it.

## Global variables

Prefix a name with `global.` to use one shared variable for the whole game:

```gml
global.level = 1;
global.player_name = "Ada";
```

`globalvar` saves you the prefix. After the declaration has *run*, the bare name means
the global variable everywhere — in every object and script:

```gml
// Game Start event of a controller object
globalvar coins;
coins = 0;
```

```gml
// any other code, later
coins += 1;              // the same as global.coins += 1
```

`globalvar` only declares the name; it doesn't give it a value. Because the effect
starts when the statement executes, put such declarations in code that runs early
(the first room's creation code or a Game Start event). Global variables survive room
changes. [`game_restart()`](../reference/game-play.md#game_restart) clears them.

## Local variables: `var`

`var` declares names that belong to the current piece of code only — one execution of
a code action, one call of a script, one `execute_string()`:

```gml
var i, total;
total = 0;
for (i = 0; i < 10; i += 1) {
    total += i;
}
score += total;
```

Locals are the right choice for loop counters and temporary values: they don't
clutter the instance, they vanish when the code ends, and two scripts can both use
`i` without disturbing each other.

The rules:

- `var` takes bare names. There is no initialiser: `var a = 1;` is a syntax error.
  Write `var a; a = 1;`.
- The declaration takes effect from that point to the **end of the code text**, no
  matter whether the `var` statement itself is reached at run time. Most people put
  all `var` lines at the top.
- A block `{ }` does not start a new scope. A `var` inside an `if` still covers the
  rest of the code.
- Locals stay visible inside `with` blocks in the same code, which makes them handy for
  passing values to other instances:

  ```gml
  var dx;
  dx = 16;
  with (obj_block) x += dx;   // dx is still our local
  ```

- Each script call gets its own locals; a script cannot see its caller's locals.
- A local can hold an [array](arrays.md): `var a; a[3] = 1;`.
- You can't declare a local (or a `globalvar`) with the name of a built-in variable:
  `var x;` fails with *"Cannot redeclare a builtin variable."*
- A local starts out *unset*, like any variable that was never assigned (see below).

## Built-in variables

Instance has many variables built in. Some belong to every instance — `x`, `y`,
`speed`, `direction`, `sprite_index`, `image_index`, `depth`, `alarm[0..11]`,
`visible` and many more. Others exist once for the whole game — `score`, `lives`,
`health`, `room`, `room_speed`, `mouse_x`, `keyboard_string`, `fps` and so on. The
[function reference](../reference/README.md) lists them all; the
[runtime chapter](../runtime/instances.md) explains the instance ones.

Some behave differently from your own variables:

- **Read-only variables** such as `id`, `object_index`, `instance_count` or `fps` can't
  be changed. The assignment compiles, but running it is the error *"Cannot assign to
  the variable"*.
- **They have a fixed type.** Assigning a string to a numeric built-in like `x` or
  `score` stores `0`, without an error.
- **Yes/no built-ins** (`visible`, `solid`, `persistent`, ...) store 1 when the value is
  above 0.5 and 0 otherwise.
- **Indexing a non-array built-in is ignored:** `x[1]` reads `x`.
- **Some assignments do things.** Setting `speed` updates `hspeed` and `vspeed`, setting
  `room` changes the room, setting `lives` to 0 or less triggers the No More Lives event.
  These are described in [How games run](../runtime/game-loop.md).

## Uninitialised variables

Reading a variable that was never assigned is a run-time error:

```gml
show_message(string(lives_left));   // ERROR: Unknown variable lives_left
```

For an array element that was never set you get *"Unknown variable a or array index
out of bounds"* instead.

The **Errors** tab of Global Game Settings has an option *Treat uninitialized variables
as value 0*. With it, such reads quietly give `0`. It does not create the variable, and
it hides typing mistakes (`scroe` instead of `score`), so leave it off while
developing if you can.

To check whether a variable exists, use
[`variable_local_exists()`](../reference/game-play.md#variable_local_exists) (for the
current instance) or
[`variable_global_exists()`](../reference/game-play.md#variable_global_exists). Their
companions read and write variables whose names are only known at run time:
[`variable_local_get()`](../reference/game-play.md#variable_local_get),
[`variable_local_set()`](../reference/game-play.md#variable_local_set),
[`variable_global_get()`](../reference/game-play.md#variable_global_get),
[`variable_global_set()`](../reference/game-play.md#variable_global_set) and their
`_array_` / `_array2_` forms.

```gml
if (!variable_global_exists("best_time")) {
    global.best_time = 9999;
}
```

## How a name is looked up

When the compiler meets a name, it decides what it means in this order:

1. A keyword (`if`, `with`, ...).
2. A name followed by `(`: a script or a built-in function. If a script has the same
   name as a built-in function, **the script wins**.
3. A name declared with `var` earlier in the same code: the local variable.
4. A built-in variable.
5. A constant: a resource name (sprite, object, room, ...), a user-defined constant or
   a built-in constant such as `c_red`. A resource name wins over a user constant of
   the same name, and a user constant wins over a built-in constant.
6. Anything else: a variable of the current instance — or a global variable, if a
   `globalvar` declaration for that name has run.

Constants and resource names can't be assigned to, and they can't be used after a dot:
if you have an object called `bb_out`, then `global.bb_out = 1` fails with *"Variable
name expected."* Give resources distinctive names (a prefix like `obj_`, `spr_`, `snd_`
helps) so they never collide with your variables.

See also: [Arrays](arrays.md) · [Addressing other instances](instances.md) ·
[Constants](constants.md) · [Common pitfalls](pitfalls.md)
