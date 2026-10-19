# Scripts and the code editor

A **script** is a named piece of GML code that you can call from anywhere: from
actions, from other scripts, from object events. Use scripts for anything you need in
more than one place, and to keep long pieces of code out of your objects.

Create one with **Resources > Create Script** or **Scripts > Create Script**. Open it
from the resource tree or from the list in the **Scripts** menu.

## The script form

At the top of the **Script Properties** form are:

| Field | What it does |
|---|---|
| Name | The script's name. You call the script by this name. |
| Load from file… | Replaces the code with the contents of a `.gml` or `.txt` file. |
| Save to file… | Saves the code to a `.gml` file. |

Below is the code editor. Press **OK** to apply your changes, or **Cancel** to discard
them.

## Calling a script

Call a script like a function, with its arguments in parentheses. Inside the script,
the arguments are `argument0`, `argument1`, … (or `argument[0]`, `argument[1]`, …), and
`return` sends back a result.

```gml
// scr_distance_to_player(x, y): how far a point is from obj_player
return point_distance(argument0, argument1, obj_player.x, obj_player.y);
```

```gml
if (scr_distance_to_player(x, y) < 64) {
    speed = 0;
}
```

A script runs on behalf of the instance that called it, so inside a script `x`,
`speed` and other variables are the caller's own.

> **Call a script with exactly the arguments it uses.** With the option *Throw an error
> when arguments aren't initialised correctly* ([Global game settings](game-settings.md#errors),
> Errors tab), a script that reads `argument0` and `argument1` must be called with
> exactly two arguments, or the game reports an error. Without it, missing arguments
> read as 0. Writing exact calls keeps your scripts working either way. See
> [Functions and scripts](../gml/scripts.md).

From actions, use **Execute Script** (control tab), which takes a script and up to five
arguments.

## The code editor

The same editor is used everywhere you write code: scripts, the **Execute Code**
action, room creation code, instance creation code and trigger conditions.

The **Execute Code** action and the room and instance creation code open in a
**code window** of their own. Drag its title bar to move it, drag an edge or corner to
resize it, or click the box in its title bar to fill the work area. It remembers its
place and size for the next time. While it is open, the object, time line or room form
it belongs to waits (it is greyed out); press **OK** to apply the code or **Cancel** to
discard it. The rest of the IDE stays usable, so you can look at other resources
while you write.

- **Colour coding** shows keywords, comments, strings, numbers, built-in functions,
  built-in variables, constants, your resource names and your scripts in different
  colours. Turn it on or off with **Use color coding** in
  [Preferences](main-window.md#preferences).
- **Line numbers** run down the left side (also a preference).
- **The status bar** below the code shows the cursor position as
  `line/total lines: column`. When the cursor is inside a function call, it also shows
  the function's arguments, for example `draw_text(x, y, string)`, so you don't have
  to remember their order. For your own scripts it shows which argument you are
  typing.
- **Text size** is set with **Code font size** in Preferences.

The usual editing keys work: arrow keys, Home/End, Shift to select, Ctrl+C/X/V to copy,
cut and paste, Ctrl+Z to undo, Ctrl+A to select everything, and Tab to insert a tab.

### Find and replace

Press **Ctrl+F** or click **Find/Replace** above the code to open the search bar.

| Control | What it does |
|---|---|
| Find | The text to look for. Press Enter to find the next match. |
| Aa | Match upper and lower case exactly. |
| Next, Prev | Jump to the next or previous match. |
| Replace | The replacement text. |
| Replace next | Replaces the next match. |
| Replace all | Replaces every match and tells you how many were replaced. |

### Labels for code actions

In an object or time line, an **Execute Code** action is listed as *Execute a piece of
code*. If the first line of the code is a comment starting with `///`, the rest of
that line is shown in the action list instead. That makes long event lists much
easier to read:

```gml
/// Fire a bullet when ready
if (can_fire) {
    instance_create(x, y, obj_bullet);
    can_fire = false;
    alarm[0] = 10;
}
```

## Errors in your code

The code editor doesn't check your code while you type. Mistakes are reported when the
game runs: syntax errors when the game starts, other errors when the faulty line runs.
The error message names the script or object, the event and the line. See
[Running and debugging games](running-and-debugging.md) and
[Errors and debugging](../runtime/errors.md).

> **Tip:** The `gmk-check` command-line tool checks all code in a project without
> running it, including the number of arguments passed to scripts. See
> [Project files](project-files.md#command-line-tools).

See also: [GML overview](../gml/overview.md) · [Functions and scripts](../gml/scripts.md) · [Common pitfalls](../gml/pitfalls.md)
