# Functions and scripts

GML code calls two kinds of functions: the **built-in functions** of Instance (several
hundred of them, listed in the [function reference](../reference/README.md)) and
your own **scripts**. Both are called the same way:

```gml
d = point_distance(x, y, mouse_x, mouse_y);   // built-in
scr_spawn_wave(3);                            // script
```

## Calling built-in functions

- The parentheses are required, even without arguments: `instance_destroy()`.
- Most functions take a fixed number of arguments. Giving a different number is a
  compile error, *"Wrong number of arguments to function or script."* A few accept a
  variable number (up to 16): `choose`, `min`, `max`, `mean`, `median`,
  `script_execute`, `execute_string` and `external_call` among them.
- Functions that do something rather than compute something return 0 when you use them
  in an expression.
- **When a built-in function itself fails** — `sqrt(-1)`, `real("abc")`, playing a sound
  that doesn't exist — the error is reported, the function returns 0, and your code
  **carries on** with the next statement. This differs from errors in your own code
  (an unknown variable, a type mismatch), which stop the rest of the action. See
  [Errors and debugging](../runtime/errors.md).

## Writing a script

A script is a resource holding a GML program. Its name becomes a function you can
call from any code, and from the *Execute Script* action. Inside, the values passed
in are `argument0` … `argument15`:

```gml
// script scr_hurt(target, amount)
// Reduces target's hp and flashes it. Returns the remaining hp.
with (argument0) {
    hp -= argument1;
    image_blend = c_red;
    alarm[1] = 5;
}
return argument0.hp;
```

```gml
// in a collision event: hurt the instance we collided with
left = scr_hurt(other.id, 2);
```

- `argument[0]` … `argument[15]` are the same variables in array form, so the index can
  be computed: `argument[i]`.
- `argument_count` is the number of arguments actually passed. It is read-only.
- `argument_relative` is 1 when the script was started by an *Execute Script* action
  with *Relative* ticked, otherwise 0.
- Arguments are passed by value. Assigning to `argument0` inside the script doesn't
  change anything in the caller.
- A call can pass at most 16 arguments; more is a compile error. The *Execute Script*
  action passes up to 5.

### Same instance, fresh locals

A script runs as part of the code that called it. `self` and `other` are the caller's,
so a script that sets `x` moves the calling instance, and a script called inside
`with (obj_enemy)` works on each enemy in turn. Each call gets its own `var` locals
and its own arguments; a script cannot see the caller's locals. Inside a `with` in the
script, `argument0` etc. still refer to the script's arguments.

### Checking the argument count

How strictly the argument count is checked depends on the option *Throw an error when
arguments aren't initialised correctly* on the **Errors** tab of Global Game Settings
(off in new projects):

- **Off:** any number of arguments is accepted. Arguments that weren't passed read
  as 0, and `argument_count` tells you how many there were. Good for scripts with
  optional arguments.
- **On:** a script must get exactly as many arguments as it uses — one more than the
  highest `argumentN` that appears in its code. A script that uses no arguments must
  be called with none. A script that also reads `argument[i]` accepts more ("at least
  N"). A wrong count is an error before the script even starts:

  ```text
  Illegal argument count calling script "scr_hurt".
  Script requires 2 arguments, 1 have been supplied.
  ```

  With this option on, reading `argument[i]` for an argument that wasn't passed is the
  error *"Unknown variable argument or array index out of bounds"*.

## Return values

`return value;` ends the script and makes `value` the result of the call. A script
can return a real or a string — not an array.

A script that ends without `return` (or with `exit`) still produces a result: the value
of the **last assignment or function call statement** it executed. `if` conditions
don't count. Relying on this makes code hard to read; return explicitly instead.

```gml
// script scr_sign_word(n)
if (argument0 > 0) return "positive";
if (argument0 < 0) return "negative";
return "zero";
```

## Recursion

A script may call itself. Instance allows 4096 nested calls (512 in the browser);
deeper nesting stops with the error *"Too many nested script calls (recursion too
deep)."*

```gml
// script scr_factorial(n)
if (argument0 <= 1) return 1;
return argument0 * scr_factorial(argument0 - 1);
```

## Scripts as values: script_execute

A script name is a constant whose value is the script's index, so you can store it in a
variable and call it later with
[`script_execute(ind, args...)`](../reference/resources.md#script_execute):

```gml
// Create event: each enemy type sets its own behaviour
ai_script = scr_ai_patrol;
```

```gml
// Step event
script_execute(ai_script, obj_player);
```

When the argument check above is on and `script_execute` passes the wrong number of
arguments, the error is reported but the script runs anyway (missing arguments read
0).

## Name clashes

If a script has the same name as a built-in function, calls go to **your script**. This
lets a game replace a built-in function, but it is easy to do by accident; prefixing
script names with `scr_` avoids it.

## Running code from a string

[`execute_string(code, args...)`](../reference/resources.md#execute_string) compiles and
runs a piece of GML at run time, with the caller's `self` and `other`, a fresh set of
locals, and optional arguments in `argument0…`. It returns the code's result.
[`execute_file(fname, args...)`](../reference/resources.md#execute_file) does the same
with code read from a file.

```gml
result = execute_string("return argument0 * 2;", 21);   // 42
```

Errors inside the executed code — compile errors included — are reported, and
`execute_string` then returns 0 to a caller that keeps running. Compiling takes time,
so don't build new code strings every step when a script would do.

## Extension functions

Functions added by [extension packages](../runtime/extensions-and-dlls.md) are called
exactly like built-in functions. Those written in GML behave like scripts.

See also: [Statements and control flow](statements.md) · [Variables and scope](variables.md) ·
[Addressing other instances](instances.md) · [Errors and debugging](../runtime/errors.md)
