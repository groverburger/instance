# Running and debugging games

## Running a game

Press **F5**, click the green **Run** button, or choose **Run > Run normally**. The
game opens in its own window. Close it, or press Escape (if that key is enabled in
the [game settings](game-settings.md#other)), to come back to the IDE.

What happens when you run a game:

1. Instance checks that the game has at least one room.
2. It writes the project, as it is in memory, to a temporary file. You don't need to
   save first.
3. It starts the runner program with that file. The game's working folder is the
   folder of your project file, so files the game reads or writes (save games, level
   files, `game_errors.log`) are found next to the project. An unsaved project uses a
   temporary folder.

While the game runs, you can keep working in the IDE, but you can't start a second
run. The status bar shows *Game running…*, and *Game finished* when it ends.

> **Apply your forms first.** The game is made from the project, not from open forms.
> Changes in a form that you haven't confirmed with **OK** are not part of the run.

## The Output Log

Everything the game prints goes to the **Output Log**: messages from your
`show_debug_message` calls, error messages, and information from the runner. Open it
with **Window > Output Log** or the **Output log** toolbar button.

```gml
show_debug_message("player hit, lives = " + string(lives));
```

The log starts empty for each run, marked with lines such as
`--- Running catch.gm81 ---` and `--- Game finished ---`. Its toolbar has buttons to
**clear** the log, **copy** it to the clipboard and **save** it to a file, and the
**Scroll to new output** checkbox keeps the newest lines in view.

When a run reports errors you haven't seen yet, the log's title shows the number of
errors, the toolbar button gets a red badge, and the status bar shows a
**new errors – view log** button.

## When an error happens

If the game hits an error, for example code that reads a variable that doesn't exist,
the game shows an error window with the message. It tells you:

- the object and event (or script) where the error happened,
- the action number in that event,
- for code, the line and position, and the line itself.

Choose **Ignore** to skip the faulty action and continue, or **Abort** to end the game.
Some errors are fatal and only offer Abort. After an abort the status bar says *Game
aborted after an error*.

The [Errors tab](game-settings.md#errors) of the game settings decides whether errors
are shown, written to `game_errors.log`, or always end the game. See
[Errors and debugging](../runtime/errors.md) for common errors and what they mean.

## Debug mode

Press **F6**, click the red **Run in debug mode** button, or choose
**Run > Run in Debug mode**. The game runs as usual, and a second window,
**Debug Information**, stays on top of it. Use it to look inside the running game.

### Controlling the game

The **Run** menu and the first toolbar buttons control the game:

| Command | What it does |
|---|---|
| Run / Continue | Continues a paused game. |
| Pause | Pauses the game. |
| Step | Runs exactly one step while paused. |
| Restart game | Starts the game over. |
| End game | Ends the game. |

The status bar shows whether the game is running, paused or ended, the frame rate
(FPS), the number of instances, and the id of the instance under the mouse.

### Tabs

| Tab | What it shows |
|---|---|
| Watches | Expressions you want to keep an eye on, with their values updated every step. |
| Messages | The output of `show_debug_message`. *Scroll to new messages* follows new lines; **Clear** empties the list. |
| Globals | All global variables and their values. |
| Locals | The variables of one instance. Type an instance id or an object name (or pick an instance on the Instances tab). |
| Instances | All instances, with their id, object, position, depth and sprite. |
| Room info | The current room, room speed, number of instances, step number, mouse position and the instance under the mouse. |

The **Tools** menu opens the same tabs (Show global variables, Show local variables,
Show instances, Show messages, Show room info).

### Watch expressions

A watch is any GML expression, evaluated every step: `score`, `obj_player.x`,
`instance_number(obj_enemy)`, `global.level * 10`. Each watch is evaluated **in** a
scope: `global`, an instance id, or an object name.

Use the **Watch** menu or the toolbar to **Add**, **Change**, **Delete** or **Clear**
watches (Delete also works with the Delete key). **Load watches…** and **Save
watches…** keep a set of watches in a text file for the next session.

### Executing code

The **Execute code** bar runs a line of GML in the running game, in a scope you choose
(`global`, an instance id, or an object name). Use it to test ideas or to cheat while
testing: `score = 1000`, `room_goto(rm_boss)`, `with (obj_enemy) instance_destroy()`.
Results and errors appear on the Messages tab.

### Changing the game speed

**Tools > Set game speed…** changes the room speed while the game runs, to slow
things down and watch them closely, or speed them up.

> **Note:** In the [browser version](web.md), debug mode isn't available; F6 runs the
> game normally.

## Running games outside the IDE

You can also run a project directly with the runner, for example from a terminal:

```sh
instance-runner mygame.gm81
instance-runner mygame.gm81 --debug
```

See also: [Errors and debugging](../runtime/errors.md) · [Creating executables](executables.md) · [Keyboard shortcuts](shortcuts.md)
