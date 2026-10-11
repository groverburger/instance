# Input: keyboard, mouse and joysticks

There are two ways to react to the player: **events** (Keyboard, Key Press, Mouse, …),
which run actions when something happens, and **functions** you call from code to ask
about the current state. Both see the same input, which Instance reads once at the
start of each step.

## Keyboard events

Each object can have three kinds of keyboard event for any key:

| Event | Runs |
|---|---|
| **Keyboard** *key* | every step while the key is held down |
| **Key Press** *key* | once, in the step the key went down |
| **Key Release** *key* | once, in the step the key came up |

They run after the alarms and before the mouse events, all held-key events first, then
press, then release ([event order](game-loop.md#order-of-events-in-a-step)). The special
keys *any key* and *no key* match when any key, or no key at all, is held, pressed or
released. Keys on the numeric keypad only produce their events while Num Lock is on.

## Checking keys in code

| Function | True when |
|---|---|
| [`keyboard_check(key)`](../reference/user-interaction.md#keyboard_check) | the key is held down |
| [`keyboard_check_pressed(key)`](../reference/user-interaction.md#keyboard_check_pressed) | it went down since the previous step |
| [`keyboard_check_released(key)`](../reference/user-interaction.md#keyboard_check_released) | it came up since the previous step |
| [`keyboard_check_direct(key)`](../reference/user-interaction.md#keyboard_check_direct) | it is down on the physical keyboard right now, even if the game window doesn't have focus |

Keys are numbers. Use the `vk_` constants for special keys (`vk_left`, `vk_space`,
`vk_enter`, `vk_shift`, `vk_escape`, `vk_f1`, …, and `vk_anykey` / `vk_nokey`), and
`ord()` with a capital letter or digit for the others:

```gml
// Step event: walk with the arrow keys or WASD
var dx, dy;
dx = keyboard_check(vk_right) || keyboard_check(ord("D"));
dx -= keyboard_check(vk_left) || keyboard_check(ord("A"));
dy = keyboard_check(vk_down) || keyboard_check(ord("S"));
dy -= keyboard_check(vk_up) || keyboard_check(ord("W"));
x += dx * 3;
y += dy * 3;

if (keyboard_check_pressed(vk_space)) {
    instance_create(x, y, obj_bullet);
}
```

Note `ord("A")`, not `ord("a")`: key codes use the capital letter.

## Typed text

- [`keyboard_string`](../reference/user-interaction.md#keyboard_string) collects the
  characters the player types (up to the last 1024). Backspace removes the last
  character. You can assign to it, for example `keyboard_string = ""` to start a new
  input field.
- [`keyboard_lastchar`](../reference/user-interaction.md#keyboard_lastchar) is the last
  character typed, and
  [`keyboard_lastkey`](../reference/user-interaction.md#keyboard_lastkey) the code of the
  last key pressed. `keyboard_key` is the key currently down (0 if none).

```gml
// Draw event of a name-entry box
draw_text(x, y, "Name: " + keyboard_string + "_");
```

## Remapping and simulating keys

- [`keyboard_set_map(key1, key2)`](../reference/user-interaction.md#keyboard_set_map)
  makes `key1` behave as `key2` — handy for alternative controls.
  [`keyboard_unset_map()`](../reference/user-interaction.md#keyboard_unset_map) removes all
  mappings.
- [`keyboard_key_press(key)`](../reference/user-interaction.md#keyboard_key_press) and
  [`keyboard_key_release(key)`](../reference/user-interaction.md#keyboard_key_release)
  pretend the player pressed or released a key. The effect shows up in the **next**
  step: then `keyboard_check` is true, the Keyboard and Key Press events run, and a
  letter is added to `keyboard_string`.
- [`keyboard_clear(key)`](../reference/user-interaction.md#keyboard_clear) forgets the
  state of one key, and [`io_clear()`](../reference/user-interaction.md#io_clear) of all
  keys and mouse buttons — useful after a dialog so that the key that closed it doesn't
  also trigger something in the game.
- [`keyboard_wait()`](../reference/user-interaction.md#keyboard_wait) pauses the game until
  a key is pressed.

## Mouse events

| Event | Runs when |
|---|---|
| **Left / Right / Middle button** | every step the button is held with the cursor over the instance |
| **No button** | every step the cursor is over the instance and no button is held |
| **Left / Right / Middle pressed**, **released** | the button goes down / up over the instance |
| **Mouse enter**, **Mouse leave** | the cursor moves onto / off the instance |
| **Global left / right / middle button**, **pressed**, **released** | the same, anywhere in the window |
| **Mouse wheel up / down** | the wheel was turned |

"Over the instance" means the cursor is inside its mask — pixel-exact for sprites with
precise collision checking — so an instance without a sprite or mask never gets the
local mouse events. The global ones need no position.

## Mouse position and buttons

- `mouse_x` and `mouse_y` are the cursor's position **in the room**. With views they are
  translated through the view the cursor is over, so they can be compared directly with
  instance positions.
- [`mouse_check_button(b)`](../reference/user-interaction.md#mouse_check_button),
  [`mouse_check_button_pressed(b)`](../reference/user-interaction.md#mouse_check_button_pressed)
  and [`mouse_check_button_released(b)`](../reference/user-interaction.md#mouse_check_button_released)
  test `mb_left`, `mb_right`, `mb_middle`, `mb_any` or `mb_none`.
- [`mouse_wheel_up()`](../reference/user-interaction.md#mouse_wheel_up) and
  [`mouse_wheel_down()`](../reference/user-interaction.md#mouse_wheel_down) tell whether the
  wheel turned since the last step.
- `mouse_button` is the button currently held and `mouse_lastbutton` the last one
  pressed. [`mouse_clear(b)`](../reference/user-interaction.md#mouse_clear) ignores a button
  until it is released and pressed again.
- [`cursor_sprite`](../reference/user-interaction.md#cursor_sprite) draws a sprite as the
  mouse cursor (-1 for none); hide the system cursor in Global Game Settings or with
  [`window_set_cursor(cr_none)`](../reference/game-graphics.md#window_set_cursor).

```gml
// Step event of a turret: aim at the mouse, fire on click
image_angle = point_direction(x, y, mouse_x, mouse_y);
if (mouse_check_button_pressed(mb_left)) {
    var b;
    b = instance_create(x, y, obj_shell);
    b.direction = image_angle;
    b.speed = 10;
}
```

## Joysticks

Instance supports two joysticks (ids 1 and 2); modern game pads work as joysticks.

| Function | Gives |
|---|---|
| [`joystick_exists(id)`](../reference/user-interaction.md#joystick_exists) | whether the joystick is connected |
| [`joystick_xpos(id)`](../reference/user-interaction.md#joystick_xpos), [`joystick_ypos(id)`](../reference/user-interaction.md#joystick_ypos) | the stick position, -1 to 1 (left/up is negative) |
| [`joystick_zpos()`](../reference/user-interaction.md#joystick_zpos), [`joystick_rpos()`](../reference/user-interaction.md#joystick_rpos), [`joystick_upos()`](../reference/user-interaction.md#joystick_upos), [`joystick_vpos()`](../reference/user-interaction.md#joystick_vpos) | further axes (0 if the device lacks them) |
| [`joystick_direction(id)`](../reference/user-interaction.md#joystick_direction) | the stick direction as a numeric-keypad key code: `vk_numpad8` is up, `vk_numpad5` centred |
| [`joystick_check_button(id, n)`](../reference/user-interaction.md#joystick_check_button) | whether button `n` (1–32) is down |
| [`joystick_pov(id)`](../reference/user-interaction.md#joystick_pov) | the hat/d-pad angle in degrees (0 forward, 90 right), or -1 |
| [`joystick_name()`](../reference/user-interaction.md#joystick_name), [`joystick_axes()`](../reference/user-interaction.md#joystick_axes), [`joystick_buttons()`](../reference/user-interaction.md#joystick_buttons), [`joystick_has_pov()`](../reference/user-interaction.md#joystick_has_pov) | information about the device |

A stick counts as pushed in a direction when it is more than halfway out. The
**joystick events** — left, right, up, down and buttons 1–8 for each joystick — are found
among the mouse events. They run every step while the stick is pushed or the button is
held, for every instance that has them, and a diagonal fires both of its directions.

On a standard game pad the left stick is x/y, the right stick is u (horizontal) and r
(vertical), the triggers together are z, the d-pad is the point-of-view hat, and the
buttons are numbered A, B, X, Y, LB, RB, Back, Start, left stick, right stick.

See also: [The game loop](game-loop.md) · [User interaction functions](../reference/user-interaction.md) ·
[Events](../ide/events.md)
