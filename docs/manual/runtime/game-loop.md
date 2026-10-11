# The game loop and event order

A running game is a loop. Many times per second Instance performs one **step**: it reads
the input, runs the events of all instances, moves them, checks collisions, and draws
a frame. Knowing the order in which this happens explains most "why does my code run
too late?" puzzles.

## Steps and room speed

The number of steps per second is the room's **speed**, available as
[`room_speed`](../reference/game-play.md#room_speed) (30 is a common value). You can
change it while the game runs; fractional values are allowed. Instance waits between
steps so the game never runs faster than `room_speed`. When a step takes too long the
game simply runs slower — it doesn't skip frames to catch up.
[`fps`](../reference/game-play.md#fps) holds the number of frames actually drawn in the
last second.

Everything in GML is measured in steps: speeds are in pixels per step, alarms count
steps, animation advances `image_speed` sub-images per step. A game designed for 30
steps per second runs twice as fast at 60.

## Order of events in a step

| # | What happens |
|---|---|
| 1 | Input is read. Special keys are handled (Esc, F4, F5/F6, F9; see [below](#special-keys)), and a click on the window's close button raises the **Close Button** event. |
| 2 | Every instance's `xprevious`, `yprevious` and `path_positionprevious` are set to the current values. |
| 3 | **Begin-step triggers**, then the **Begin Step** event. |
| 4 | **Time lines** advance and run their moments. |
| 5 | **Alarm** events. |
| 6 | **Keyboard** events (keys held), then **Key Press**, then **Key Release**. |
| 7 | **Mouse** events (including joystick events). |
| 8 | **Step triggers**, then the **Step** event. |
| 9 | Instances **move**: path following, friction, gravity, then `x += hspeed`, `y += vspeed`. |
| 10 | **Outside Room** / **Intersect Boundary** events, and their per-view variants. |
| 11 | **Collision** events. |
| 12 | **End-step triggers**, then the **End Step** event. |
| 13 | Destroyed instances are removed. Particle systems update. Views follow their objects. |
| 14 | The frame is **drawn**: backgrounds, then the **Draw** event (or the default sprite) of every visible instance, by depth, then foregrounds. |
| 15 | **Animation** advances: `image_index += image_speed`; an instance that passes its last sub-image gets **Animation End**. Backgrounds scroll. |
| 16 | A requested room change, restart, end or load is carried out. |

Some consequences:

- **Step sees the old position, End Step sees the new one.** Movement happens between
  them. Code that needs to react to where an instance *ended up* this step — a camera,
  a shadow that follows the player — belongs in End Step.
- **Draw sees the state after End Step**, but before the animation advances. A Draw
  event sees the same `image_index` that the default draw would use.
- **Collisions are checked after movement**, so in a Collision event the instances
  already overlap. `xprevious`/`yprevious` tell you where an instance came from.
- **Animation End fires after drawing**, so the frame where the last sub-image was shown
  has already been drawn.

### Order of instances within an event

For **Begin Step, Step and End Step** the instances are visited *object by object*: all
instances of the object with the lowest index first (in the order they were created),
then the next object, and so on. An instance created during these events runs in the
same phase only if its object's turn hasn't passed yet; otherwise it waits for the
next step.

Alarm events and keyboard events are handed out object by object in the same way
(alarm 0 for everyone first, then alarm 1, …; one key after another). Collision events
go object by object as well (see
[Movement and collisions](movement-and-collisions.md#collision-events)). Room Start,
Room End, Game Start, Game End and the mouse events go through the instances in
creation order.

The order of drawing is decided by depth, not by creation:
see [Drawing](drawing.md#depth-and-draw-order).

## Alarms

Each instance has twelve alarm clocks, `alarm[0]` … `alarm[11]`. Set one to a number of
steps and the matching Alarm event fires when it runs down:

```gml
// Create event: fire every second
alarm[0] = room_speed;
```

```gml
// Alarm 0 event
instance_create(x, y, obj_bullet);
alarm[0] = room_speed;    // re-arm
```

The exact rules:

- Alarms hold whole numbers; assignment rounds (halves to even, so `1.5` becomes 2 and
  `0.5` becomes 0).
- Each step, a positive alarm counts down by one. When it reaches 0 the event fires;
  inside the event the alarm reads 0. If it is still 0 at the next step, it becomes -1
  without firing again.
- Setting an alarm to 0 or less switches it off: `alarm[0] = 0` never fires.
- An alarm only counts down if the object has (or inherits from its parent) an event for
  that alarm. Otherwise the value just stays where it is.
- Alarms are processed in number order: every instance's Alarm 0, then every Alarm 1,
  and so on.

## Time lines

A time line is a list of moments, each holding actions. Assign it with
`timeline_index = tl_intro`, start it with `timeline_running = true` (or the
*Set Time Line* action), and the instance runs each moment when its step count
reaches it. Time lines advance right after Begin Step:

- `timeline_position` grows by `timeline_speed` each step (fractional and negative
  speeds work). All moments that the position passes over in a step run in order, so a
  speed of 2 never skips a moment.
- A time line started in the Create event runs moment 0 in the first step.
- At the end, a looping time line (`timeline_looping = true`) starts again from 0; a
  non-looping one simply has nothing more to run.

## Triggers

A trigger is a custom event with a condition you write in GML. The condition is a
piece of code that **returns** the result — `return place_meeting(x, y + 1, obj_water);`,
not just the expression. At the moment chosen for the trigger (begin step, step or end
step), the condition is evaluated for every instance whose object has that trigger
event, and the event runs where it returns true.

## Starting the game

When a game starts, Instance loads all resources, compiles the code, evaluates the
user-defined constants, runs the init functions of extension packages, and enters the
first room in the room order. Entering a room runs, in order: creation code and
**Create** for each placed instance, then **Game Start** (only for the first room of the
game), the room's creation code, and **Room Start**. Then a first frame is drawn, and
only after that does the first step begin. The details are in
[Rooms and views](rooms-and-views.md#what-happens-when-the-room-changes).

## Changing rooms, ending and restarting

[`room_goto()`](../reference/game-play.md#room_goto) and its relatives,
[`game_end()`](../reference/game-play.md#game_end),
[`game_restart()`](../reference/game-play.md#game_restart) and
[`game_load()`](../reference/game-play.md#game_load) don't stop your code mid-way:

- The current action finishes, including the statements after the call.
- The remaining actions of the same event are skipped.
- The rest of the step is abandoned: no further instances get the current event, and
  the later phases (End Step, Draw, …) don't happen.
- Then the change is carried out.

If several room changes are requested, the last one wins — but a requested game end,
restart or load is not overridden by a later room change.

**`game_end()`** sends **Game End** to every instance and closes the game (no Room End,
no Destroy events). **`game_restart()`** sends **Room End** and then **Game End** to every
instance, removes them all (persistent ones too) without Destroy events, and starts
again in the first room with Game Start and Room Start. A restart clears all
global variables and resets `score` to 0 and `lives` to -1; `health`, files, data
structures, particle systems and surfaces are kept. Sounds stop.

## Lives, health and score

`score`, `lives` and `health` are global built-in variables. They start at 0, -1 and
100. When `lives` goes from above 0 to 0 or below, every instance gets the
**No More Lives** event; the same goes for `health` and **No More Health**. Instance runs
these events right after the assignment — at the latest before the next function or
script call in the same code, or when the action ends.

The window caption can show them: with `show_score` on (the default), the caption reads
like `Cave Run Score: 120`; `show_lives` and `show_health` add `Lives: 3` and
`Health: 50`. `caption_score`, `caption_lives` and `caption_health` change the labels.

## Special keys

Depending on the Global Game Settings, a few keys are handled by Instance itself:
**Esc** ends the game, **F1** shows the game information, **F4** toggles fullscreen,
**F5** saves the game and **F6** loads it, and **F9** saves a screenshot
(`screenshot000.png`, `screenshot001.png`, … in the game's folder). If the window's
close button is set to act like Esc it does the same; otherwise every instance gets the
**Close Button** event, and nothing else happens unless your code reacts to it.

## When the loop pauses

Message boxes and other dialogs ([`show_message()`](../reference/splash-and-messages.md#show_message),
[`get_string()`](../reference/splash-and-messages.md#get_string), …) halt the game until
they are closed. So do [`sleep()`](../reference/game-play.md#sleep) and
[`keyboard_wait()`](../reference/user-interaction.md#keyboard_wait). With *Freeze the game
when the window loses focus* set in Global Game Settings, the game stops while another
window is active. A `while` loop that never ends hangs the game, since nothing else can
run until it returns.

See also: [Instances, objects and parenting](instances.md) ·
[Rooms and views](rooms-and-views.md) · [Movement and collisions](movement-and-collisions.md) ·
[Drawing](drawing.md)
