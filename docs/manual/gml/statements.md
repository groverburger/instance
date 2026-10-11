# Statements and control flow

A GML program is a list of statements, run one after the other. This page describes
each kind.

## Assignments and calls

The two simple statements are an assignment and a function (or script) call:

```gml
hp -= 1;
instance_create(x, y, obj_spark);
```

Nothing else stands alone as a statement. `5;`, `a == b;` or a bare variable name are
syntax errors. A call used as a statement throws its result away.

## Blocks

`{ ... }` (or `begin ... end`) groups statements so they count as one, for example as
the body of an `if`. A block does not create a new variable scope.

## if

```gml
if (hp <= 0) {
    instance_destroy();
} else if (hp < 3) {
    sprite_index = spr_enemy_hurt;
} else {
    sprite_index = spr_enemy;
}
```

The condition is true when it is 0.5 or more ([truth rules](expressions.md#logical-operators-and-truth)).
Parentheses are optional and so is the noise word `then` (`if hp <= 0 then instance_destroy()`).
A `;` before `else` is allowed. An `else` belongs to the nearest `if` that has none.

## repeat

`repeat (n)` runs its body `n` times:

```gml
repeat (5) {
    instance_create(random(room_width), random(room_height), obj_coin);
}
```

The count is evaluated **once**, before the first round, and rounded to a whole number
(halves to even: `repeat (2.5)` runs twice). Zero or a negative count runs nothing. A
string count is the error *"Repeat count must be a number"*.

## while

`while (condition) statement` tests the condition first and runs the body as long as it
holds:

```gml
while (!place_free(x, y)) {
    y -= 1;          // move up until we are out of the wall
}
```

There is no safety limit. A condition that never becomes false freezes the game,
because no other event, drawing or input can happen until the loop ends.

## do ... until

The body runs first and the condition is tested afterwards. The loop stops when the
condition becomes **true**:

```gml
do {
    tx = random(room_width);
    ty = random(room_height);
} until (point_distance(tx, ty, x, y) > 100)
```

The body always runs at least once.

## for

`for (init; condition; step) body` is shorthand for the usual counting loop:

```gml
var i;
for (i = 0; i < 10; i += 1) {
    draw_text(16, 16 + i * 20, string(i));
}
```

`init` runs once. Then, as long as `condition` holds, the body runs followed by `step`.
`init` and `step` are statements (usually assignments), and the condition is an
expression. There is no `i++`: write `i += 1`.

## switch

`switch` compares one value with a list of `case` labels:

```gml
switch (keyboard_lastkey) {
    case vk_left:
    case ord("A"):
        hspeed = -4;
        break;
    case vk_right:
    case ord("D"):
        hspeed = 4;
        break;
    default:
        hspeed = 0;
}
```

- The switch value is evaluated once. Case labels are tried in order; they can be any
  expression, not just constants.
- Matching uses the same rule as `==` (including the small number tolerance). A case
  of the other type — a string against a real — simply doesn't match.
- Execution continues from the first matching label **and falls through** into the
  following cases until a `break` or the end of the switch. That is how several labels
  share one piece of code, as above; forgetting `break` is a classic bug.
- If nothing matches, execution continues at `default:` if there is one, otherwise the
  switch does nothing. `default` may appear anywhere in the list.

## break, continue, exit

- **`break`** leaves the innermost `repeat`, `while`, `do`, `for`, `switch` or `with`.
  Outside all of those it ends the current code, like `exit`.
- **`continue`** skips to the next round of the innermost loop: in a `for` loop it runs
  the step statement next; in `do ... until` it jumps to the condition test; in a `with`
  it moves on to the next instance. Outside a loop it ends the code, like `exit`.
- **`exit`** ends the current code immediately — the current code action or the current
  script call. It does not end the event: the next action of the event still runs.
  Inside a `with`, `exit` leaves the whole program, not just the current instance.

```gml
if (!visible) exit;      // nothing to do while hidden
```

## return

`return value;` ends a script and hands `value` back to the caller:

```gml
// script scr_clamp(value, lo, hi)
return min(max(argument0, argument1), argument2);
```

A value is required: a bare `return;` is a syntax error (use `exit` instead). In a code
action, `return` simply ends the code. What a script returns when it finishes without
`return` is described in [Functions and scripts](scripts.md#return-values).

## with

`with (target) statement` runs the statement once for each instance selected by
`target`, with that instance as `self`:

```gml
with (obj_enemy) {
    hp -= 1;
}
```

It is the most important tool for working with other instances, and has its own page:
[Addressing other instances](instances.md#with).

## var and globalvar

`var a, b;` declares local variables and `globalvar a, b;` declares global names. Both
take a list of bare names without values. See
[Variables and scope](variables.md#local-variables-var).

## The empty statement

A lone `;` does nothing. Extra semicolons are therefore harmless.

See also: [Expressions and operators](expressions.md) · [Functions and scripts](scripts.md) ·
[Addressing other instances](instances.md) · [Common pitfalls](pitfalls.md)
