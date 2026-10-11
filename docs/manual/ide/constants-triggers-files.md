# Constants, triggers and included files

Three project-wide lists live in the **Resources** menu: user-defined constants,
triggers, and files to include with the game.

## Constants

A **constant** is a name for a fixed value, such as `MAX_ENEMIES = 20` or
`GRAVITY = 0.5`. Use the name anywhere in code and in action fields instead of the
number. When you want to change the value, you change it in one place.

Open the list with **Resources > Define Constants**. The **User-Defined Constants**
form shows a table with a **Name** and a **Value** column. Click a cell to type in it.

| Button | What it does |
|---|---|
| Insert | Inserts an empty row above the selected one. |
| Add | Adds an empty row at the end. |
| Delete | Deletes the selected row. |
| Clear | Deletes all rows. |
| Up, Down | Moves the selected row. |
| Sort | Sorts the rows by name. |
| Load… | Adds constants from a text file. |
| Save… | Saves the constants to a text file. |

The text file format is one constant per line, written as `NAME=VALUE`.

The value can be a number, a string in quotes, or an expression made of numbers,
other constants and resource names, such as `room_speed * 2` or `c_red`. Constants are
known everywhere in the game and are coloured in the code editor.

> **Tip:** Writing constant names in capitals (`TILE_SIZE`) makes them easy to tell
> apart from variables.

## Triggers

A **trigger** is a custom event with a condition. Every step, the condition is checked
for each instance whose object has an event for the trigger, and when it's true, the
event happens for that instance. Triggers are handy for situations that several
objects need to react to, such as "the player is near" or "all coins are collected".

Open the list with **Resources > Define Triggers**. The **Triggers** form has the list
of triggers on the left, with **Add**, **Delete** and **Clear** buttons, and the
selected trigger's settings on the right:

| Setting | What it does |
|---|---|
| Name | The name shown in the event selector and in event lists. |
| Condition | GML code that decides whether the event happens. It must **return** true or false. |
| Moment of checking | **Begin of step** (just before the Begin Step events), **Middle of step** (just before the Step events), or **End of step** (just before the End Step events). |
| Constant name | An optional constant name for this trigger, so code can refer to it, for example with `event_perform(ev_trigger, TRG_NEAR)`. |

The condition is code, not a bare expression, so write `return` in front:

```gml
return distance_to_object(obj_player) < 64;
```

The condition runs as the instance being checked, so `x`, `y` and its other variables
are available.

Once a trigger exists, objects can use it: **Add Event > Trigger** lists all triggers.

> **Note:** Deleting a trigger that objects still use leaves those events behind.
> They appear in the object as `<trigger N>` and never happen.

## Included files

**Included files** are files that travel inside your project and are written to disk
when the game starts: a level file, a text file with dialogue, a DLL, music in a
format the game loads itself.

Open the list with **Resources > Included Files**. Click **Add…** to choose a file;
select a file in the list to change its settings. **Delete** removes the selected
file and **Clear** removes them all.

| Setting | What it does |
|---|---|
| File name | The name the file gets when it's written out. You can change it. |
| Original file, Size | Where the file came from, and its size. |
| Load… | Replaces the file with another one. |
| Store in the editable gm81 file | Keeps the file's contents in the project. It's on for new files. **Leave it on**: only stored files are part of the game. |
| Free memory after export | Releases the file's memory in the game once it's written out. *Not used by Instance.* |
| Overwrite the file if it exists | Writes the file even if a file of that name is already there. When off, an existing file is left alone. |
| Remove at game end | Deletes the file again when the game ends. *Not used by Instance:* the file stays. |
| Export | When the game starts: **Don't export automatically**, **Export to the temporary folder**, **Export to the folder of the game**, or **Export to the following folder:** with a folder you type. New files are exported to the folder of the game. |

With *Don't export automatically*, the game writes the file out itself when needed,
with [`export_include_file`](../reference/files-and-system.md#export_include_file).
Files exported to the temporary folder can be found through the `temp_directory`
variable.

See also: [Events: Trigger](events.md#trigger) · [Constants in GML](../gml/constants.md) · [Saving, files and settings](../runtime/files-and-saving.md)
