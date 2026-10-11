# Time lines

A **time line** is a schedule of actions. It lists **moments**, each a step number,
and each moment has a list of actions, just like an event. When an instance runs a
time line, the actions of each moment run on that instance when the time line
reaches that step.

Time lines are useful for scripted sequences: an enemy wave that sends ships at
steps 0, 60 and 120, a boss with a fixed attack pattern, a cut scene, a tutorial that
shows hints one after another.

Create one with **Resources > Create Time Line** and double-click it in the tree to
open the **Time Line Properties** form.

## The time line form

From left to right, the form has the name and the moment buttons, the **Moments**
list, the **Actions** list of the selected moment, and the action palette. The action
list and palette work exactly as in the [object form](objects.md#the-action-list):
drag or click actions from the palette, double-click to edit, right-click for a menu.

Most buttons ask for step numbers in a small window. They start with the step of the
selected moment filled in.

| Button | What it does |
|---|---|
| Add | Asks for a step and adds a moment there (or selects it if it already exists). |
| Change | Moves the selected moment to another step. |
| Delete | Deletes all moments from one step till another. |
| Clear | Deletes all moments. |
| Shift | Moves the moments in a range so that the first step of the range lands on a new step. |
| Duplicate | Like Shift, but copies the moments and leaves the originals in place. |
| Spread | Stretches or squeezes the moments in a range by a percentage, counted from the start of the range. 200 doubles the gaps, 50 halves them. |
| Merge | Combines all moments in a range into the first one. |

When moments land on a step that already has a moment (by Change, Shift, Duplicate,
Spread or Merge), their actions are added to the end of that moment's list.

> **Note:** Negative step numbers are allowed in the editor, but those moments never
> run. Instance warns you when you add one.

## Running a time line

A time line does nothing until an instance runs it. In the object that should run it,
use the **Set Time Line** action (main2 tab), usually in the Create event:

| Setting | Meaning |
|---|---|
| time line | The time line to run. |
| position | The step to start at, usually 0. |
| start | Start Immediately, or Don't Start (start it later with Start Time Line). |
| loop | Don't Loop, or Loop to start again after the last moment. |

Or in GML:

```gml
timeline_index = tl_wave;
timeline_position = 0;
timeline_running = true;
timeline_loop = false;
```

The time line advances by `timeline_speed` (1 by default) every step, and runs all the
moments it passes. Other actions on the main2 tab (Time Line Position, Time Line
Speed, Start, Pause and Stop Time Line) control it while it runs.

Each instance has its own position in its time line, so several instances can run the
same time line independently.

See also: [Actions: main2](actions.md#main2) · [The game loop](../runtime/game-loop.md) · [Objects](objects.md)
