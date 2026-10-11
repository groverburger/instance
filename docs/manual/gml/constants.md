# Constants

A constant is a name with a fixed value. You can use it anywhere a value is expected,
but you can't assign to it. GML has three sources of constants: the built-in ones,
the names of your resources, and constants you define yourself.

## Built-in constants

| Constants | Examples | Used for |
|---|---|---|
| Truth values and π | `true` (1), `false` (0), `pi` | conditions, maths |
| Special instances | `self` (-1), `other` (-2), `all` (-3), `noone` (-4), `global` (-5) | [addressing instances](instances.md) |
| Colours | `c_red`, `c_white`, `c_black`, `c_aqua`, … | drawing; a colour is a number `red + 256*green + 65536*blue` |
| Keys and mouse buttons | `vk_left`, `vk_space`, `vk_anykey`, `mb_left`, … | [input](../runtime/input.md) |
| Events | `ev_step`, `ev_collision`, `ev_other`, `ev_user0`, … | `event_perform()` and friends |
| Text alignment, blend modes, primitives | `fa_center`, `bm_add`, `pr_trianglelist`, … | [drawing](../runtime/drawing.md) |
| Particles, effects, sound effects, cursors, DLL types | `pt_shape_flare`, `ef_explosion`, `se_echo`, `cr_default`, `ty_real`, … | their function groups |

The [A–Z index](../reference/index.md) lists every built-in constant with its value.
Letter and digit keys have no `vk_` constant; use `ord("A")` or `ord("1")`.

## Resource names

Every sprite, sound, background, path, script, font, time line, object and room is a
constant whose value is its index:

```gml
instance_create(x, y, obj_explosion);   // obj_explosion is an object index
sprite_index = spr_player_run;
room_goto(rm_level2);
```

Indices are just numbers, but don't do arithmetic on them: `rm_level1 + 1` is not
necessarily the next room. Use `room_next(room)` or
[`room_goto_next()`](../reference/game-play.md#room_goto_next) for that.

Don't give resources the names of variables you use, or of built-in things such as
`self`, `other`, `all` or `global`; the built-in meaning wins and the resource becomes
unreachable from code.

## User-defined constants

You can define your own constants in Global Game Settings (see
[Constants, triggers and included files](../ide/constants-triggers-files.md)). Each has a
name and a value:

```text
TILE      = 32
GRAVITY   = 0.4
MAX_HP    = 3
HUD_COLOR = c_yellow
HALF_TILE = TILE / 2
TITLE     = "Cave Run"
```

- The value is an expression: numbers, strings, operators, built-in constants, resource
  names, and constants defined **earlier in the list**.
- Constants are evaluated once, when the game starts, before anything else runs.
  They can't refer to variables, instances or scripts. A value that can't be evaluated
  becomes 0.
- Use them in code like any other constant: `if (hp > MAX_HP) hp = MAX_HP;`

Constants make tuning easy — change `GRAVITY` in one place instead of hunting for
every `0.4` in the code.

## When names clash

If a user constant has the same name as a resource, the **resource wins**. If it has
the same name as a built-in constant, **your constant wins** (defining `c_red` as 5
makes `c_red` mean 5 everywhere). Neither is a good idea; give constants their own
names, for example in capitals.

Constants can't appear after a dot (`global.MAX_HP` is the compile error *"Variable
name expected."*), and assigning to one is a compile error.

[Extension packages](../runtime/extensions-and-dlls.md) can add constants of their own;
they behave like built-in constants.

See also: [Variables and scope](variables.md) · [Expressions and operators](expressions.md) ·
[A–Z index](../reference/index.md)
