# Events

An **event** is a moment when an object gets the chance to act. You add events to an
object with **Add Event** in the [object form](objects.md#events), and give each one a
list of actions. This page lists every event and exactly when it happens.

Events only matter for objects that have them: if an object has no Step event, nothing
happens for its instances in that phase of the step. An object also inherits the
events of its [parent](objects.md#parents).

In code, the current event is available as `event_type` and `event_number`, using
constants such as `ev_step` and `ev_step_begin`.

## Create

Happens once for each new instance, right after it's created: when a room starts and
builds the instances placed in it, or when code or an action creates one
(`instance_create`, **Create Instance**, and so on). Use it to set up the instance's
variables and start its movement.

For instances placed in a room, the instance's **creation code** (set in the room
editor) runs just before its Create event.

## Destroy

Happens when the instance is destroyed with **Destroy Instance**, **Destroy at
Position**, `instance_destroy()` and similar. The instance still exists while the
event runs, so you can read its position, for example to create an explosion there.

Instances that disappear because the room ends or the game restarts do **not** get a
Destroy event.

## Alarm

Each instance has twelve alarm clocks, **Alarm 0** to **Alarm 11**. Set one to a
number of steps (with **Set Alarm** or `alarm[0] = 30`). Every step it counts down by
one, and when it reaches 0 the alarm event happens. An alarm counts down only if the
object has an event for it. Set an alarm again inside its own event to make something
repeat:

```gml
// Alarm 0: fire every second
instance_create(x, y, obj_bullet);
alarm[0] = room_speed;
```

## Step

Happens every step. There are three:

| Event | When |
|---|---|
| Begin Step | At the start of each step, before alarms and input. |
| Step | In the middle of the step, after input events and before instances move. |
| End Step | At the end of the step, after movement and collisions, just before drawing. |

Use Step for most continuous behaviour (checking keys, steering, timers). Use End Step
when you need the final positions after movement and collisions, for example to make
something follow another object exactly.

## Collision

**Collision with *object*** happens every step in which the instance overlaps an
instance of that object (or of one of its children). In the event, `other` is the
instance it collides with. Both instances get their own collision event, if they have
one.

Overlap is tested with the instances' collision masks (see
[Sprites](sprites.md#collision-masks)). If either instance is **solid**, both are first
put back to where they were before this step's movement, and then the event runs.

A collision event needs no actions to make solid objects block movement, but an
object with no collision event for a pair will pass straight through.

## Keyboard

Keyboard events come in three kinds. The menus group the keys into Special, Arrows,
Modifiers, Common, Navigation, Letters, Digits, Keypad and Function keys.

| Event | Listed as | When |
|---|---|---|
| Keyboard | `<Left>` | Every step while the key is held down. |
| Key Press | `press <Left>` | Once, in the step the key goes down. |
| Key Release | `release <Left>` | Once, in the step the key comes up. |

Two special keys exist for all three:

- **`<No key>`**: happens when no key is held (or, for press and release, when no key
  went down or up).
- **`<Any key>`**: happens when any key is held (or was pressed, or released).

Keypad keys only produce events with Num Lock on. Keyboard events happen for every
instance with the event, wherever it is.

## Mouse

Mouse events come from the **Mouse** button of the event selector.

**Mouse over the instance.** These happen only when the mouse pointer is over the
instance's collision mask:

| Event | When |
|---|---|
| Left button, Right button, Middle button | Every step while that button is held. |
| No button | Every step while no button is held. |
| Left pressed, Right pressed, Middle pressed | Once, when the button goes down. |
| Left released, Right released, Middle released | Once, when the button comes up. |
| Mouse enter | When the pointer moves onto the instance. |
| Mouse leave | When the pointer moves off the instance. |

**Mouse wheel.** **Mouse wheel up** and **Mouse wheel down** happen when the wheel is
turned.

**Global mouse** (submenu). Global Left/Right/Middle Button, Pressed and Released work
like the events above but happen wherever the mouse is, for every instance that has
them. Use them for things like a crosshair or a click anywhere on the screen.

**Joystick 1, Joystick 2** (submenus). **Joystick left**, **right**, **up** and
**down** happen every step while the stick is pushed that way (diagonals give two
events), and **Button 1** to **Button 8** while a button is held. They happen for every
instance that has them.

## Other

The **Other** button holds the remaining events:

| Event | When |
|---|---|
| Outside Room | Every step in which the instance is completely outside the room. Typical use: destroy bullets that leave the screen. |
| Intersect Boundary | Every step in which the instance touches the room's edge, that is, it's partly outside. Typical use: keep the player inside the room. |
| Game Start | Once, in the first room, after the Create events of its instances and before Room Start. Also after a game restart. |
| Game End | When the game ends, for every instance. |
| Room Start | When a room starts, for every instance in it (including persistent instances that came along), after their Create events and the room's creation code. |
| Room End | When the room ends because the game goes to another room (or restarts), for every instance. |
| No More Lives | When `lives` drops to 0 or below. |
| No More Health | When `health` drops to 0 or below. |
| Animation End | When the instance's sprite animation reaches its last frame and wraps around to the first. |
| End Of Path | When the instance reaches the end of the path it follows. |
| Close Button | When the player clicks the window's close button, if the *Treat the close button as \<Esc\> key* setting is off. |
| Views > Outside View 0–7 | Every step in which the instance is completely outside that view. |
| Views > Boundary View 0–7 | Every step in which the instance touches the edge of that view. |
| User defined > User Defined 0–15 | Never by itself: only when code calls `event_user(n)`. Use them as named routines of an object. |

The view events only happen in rooms with views enabled.

## Draw

Happens when the instance is drawn, once per step (once per visible view, if the room
uses views). Only visible instances get it.

Without a Draw event, an instance draws its sprite automatically. **With** a Draw
event, it doesn't: the event replaces the default drawing, and the instance draws only
what the event's actions or code draw. To keep the sprite and add something, draw the
sprite yourself:

```gml
draw_sprite_ext(sprite_index, image_index, x, y, image_xscale, image_yscale,
    image_angle, image_blend, image_alpha);
draw_text(x, y - 20, "Boss");
```

Drawing functions only work in the Draw event. See [Drawing](../runtime/drawing.md).

## Trigger

A **trigger** is an event you define yourself, with a condition written in GML. It
happens for an instance in every step in which the condition is true, at the moment of
the step the trigger chooses (begin, middle or end). Define triggers with
**Resources > Define Triggers**; see
[Triggers](constants-triggers-files.md#triggers).

## The order of events in a step

Within one step, events happen in this order:

1. Begin Step (and begin-step triggers)
2. Time lines advance
3. Alarms
4. Keyboard, Key Press, Key Release
5. Mouse
6. Step (and middle-of-step triggers)
7. Instances move (speed, friction, gravity, paths; End Of Path)
8. Outside Room, Intersect Boundary and the view events
9. Collisions
10. End Step (and end-of-step triggers)
11. Draw
12. Animation advances (Animation End)

[The game loop and event order](../runtime/game-loop.md) explains each phase in detail.

See also: [Objects](objects.md) · [Actions](actions.md) · [The game loop](../runtime/game-loop.md) · [Input](../runtime/input.md)
