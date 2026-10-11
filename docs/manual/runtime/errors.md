# Errors and debugging

Sooner or later a game does something unexpected. This page explains the kinds of
errors Instance reports, what happens to the game after an error, and the tools for
finding out what your code is doing.

## Three kinds of errors

**Compile errors** are mistakes in the text of the code: a missing bracket, an unknown
function, `var a = 1`. Instance compiles all code when the game starts and reports
every compile error at that moment, saying where it is (for example *"COMPILATION ERROR
in code action (action 2 of Step event for object obj_player)"*, followed by the line
and the problem). The broken piece of code never runs;
each time it would have run, its error is reported again. The code editor in the IDE
and the `gmk-check` tool find these errors before you even run the game.

**Errors in your code** happen while code runs: reading a variable that doesn't exist,
adding a string to a number, dividing by zero, using an index that is out of range.
Such an error **abandons the rest of the action**: the statements after it don't run.
The next action of the event does run. If the error happens inside a script, both the
script and the action that called it are abandoned.

**Errors in built-in functions** — `sqrt(-1)`, `real("abc")`, `sound_play()` of a sound
that doesn't exist, `ln(0)` — are milder. The error is reported, the function returns
0, and your code **continues** with the next statement.

## What an error message looks like

```text
ERROR in
action number 1
of Step Event
for object obj_player:

Error in code at line 2:
   hp = hp - 'one';
            ^
at position 11: Wrong type of arguments to -.
```

It tells you the object, the event, which action of the event (counting from 1), the
line in the code, a caret under the place where the problem was noticed, and the
message. For an error inside a script, a line `In script scr_name:` comes before the
code part. Errors raised by built-in functions show only their message, such as
*"Cannot apply sqrt to negative number."*

## What happens after an error

With *Display error messages* on (the default), the game pauses and shows the message
in a dialog with two buttons:

- **Ignore** continues the game. The failing action is abandoned (see above) and
  everything else goes on as normal.
- **Abort** ends the game immediately, without Game End events.

Some errors are fatal and offer only Abort.

## Error settings

The **Errors** tab of Global Game Settings controls error handling:

| Option | Effect |
|---|---|
| Display error messages | Show errors in a dialog. When off, errors are not shown and the game carries on as if Ignore had been pressed. |
| Write error messages to file game_errors.log | Append each error to `game_errors.log` in the game's folder. |
| Abort on all error messages | Treat every error as fatal: the game ends at the first one. |
| Treat uninitialized variables as value 0 | Reading a variable that was never set gives 0 instead of an error. |
| Throw an error when arguments aren't initialised correctly | Check that scripts get exactly the arguments they use (see [Functions and scripts](../gml/scripts.md#checking-the-argument-count)). |

When you run a game from the IDE, errors also appear in the IDE's output log.

## Checking for errors in code

Every error — shown or not — sets two global variables:

- [`error_occurred`](../reference/game-play.md#error_occurred) becomes true;
- [`error_last`](../reference/game-play.md#error_last) holds the full message.

Both can be assigned, so you can reset `error_occurred = false`, try something, and look
again. This is most useful around built-in functions and
[`execute_string()`](../reference/resources.md#execute_string), which keep running after an
error:

```gml
error_occurred = false;
n = real(get_string("How many?", "3"));
if (error_occurred) {
    show_message("That is not a number.");
    n = 3;
}
```

[`show_error(message, abort)`](../reference/splash-and-messages.md#show_error) raises an
error of your own through the same mechanism; with `abort` true it ends the game.

## Common messages

| Message | Usual cause |
|---|---|
| Unknown variable *name* | Reading a variable before it was set — often a typo, a variable set in another object's Create event, or a Create event that hasn't run yet. Also reading `obj.name` when no instance of `obj` exists, or through an instance that was destroyed. |
| Unknown variable *a* or array index out of bounds | Reading an array element that was never written. |
| Wrong type of arguments to + (or -, \*, …) | Mixing a string and a number. Convert with `string()` or `real()`. |
| Cannot compare arguments. | Comparing a string with a number. |
| Expression expected | A string used as an `if`/`while` condition. |
| Division by 0. / Modulo by 0. | The right side of `/`, `div` or `mod` was 0. |
| Cannot assign to the variable | Writing a read-only built-in variable (`id`, `fps`, …), or writing through `noone`. |
| Negative array index / Array index >= 32000 | An array index out of the allowed range. |
| Illegal argument count calling script … | A script got a different number of arguments than it uses (only with the argument check on). |
| Too many nested script calls (recursion too deep). | A script that calls itself without end. |
| Unknown function or script: *name* | A typo, or a script that doesn't exist (compile error). |
| Wrong number of arguments to function or script. | A built-in function called with too few or too many arguments (compile error). |
| Cannot redeclare a builtin variable. | `var x` or `globalvar speed` — choose another name (compile error). |
| Program ends before end of the code. | The code starts with `{` and has more after the matching `}` (compile error). |

## Debugging

**Debug mode.** Run the game in debug mode (from the IDE, or with
`instance-runner game.gm81 --debug`) to get the Debug Information window next to the
game. It shows the frame rate and the instances, lets you watch expressions while the
game runs, pause and single-step it, and execute a piece of code in the game. See
[Running and debugging games](../ide/running-and-debugging.md).

**Messages.** [`show_debug_message(text)`](../reference/game-play.md#show_debug_message)
writes a line to the debug window's log and to the console or the IDE's output log —
much less intrusive than a message box:

```gml
show_debug_message("jump: vspeed=" + string(vspeed) + " y=" + string(y));
```

**Drawing values.** Drawing a variable on screen in a Draw event is often the quickest
way to watch it:

```gml
draw_set_color(c_white);
draw_text(view_xview[0] + 8, view_yview[0] + 8, "state: " + string(state));
```

**Tips.**

- Read the message: object, event, action number and line usually lead straight to
  the problem.
- *Unknown variable* in a Step event often means the variable is set in a different
  event, or only in some branch of the Create event. Give every variable a value in
  Create.
- When something happens "one step late", check the [event order](game-loop.md#order-of-events-in-a-step).
- Many puzzling bugs come from the traps listed in [Common pitfalls](../gml/pitfalls.md).

See also: [Common pitfalls](../gml/pitfalls.md) · [Running and debugging games](../ide/running-and-debugging.md) ·
[The game loop](game-loop.md)
