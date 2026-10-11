# User interaction

Reading the keyboard, the mouse and joysticks.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [The keyboard](#the-keyboard)
- [The mouse](#the-mouse)
- [The joystick](#the-joystick)

## The keyboard

Reading and simulating keys, and the virtual key constants.

### keyboard_key

*Global variable.* Keycode of the key currently held down, or 0 if none.

### keyboard_lastkey

*Global variable.* Keycode of the most recently pressed key; assignable, e.g. reset to 0 once handled.

### keyboard_lastchar

*Global variable.* The last typed character, as a string.

### keyboard_string

*Global variable.* Printable characters typed so far (at most 1024); backspace removes the last one.

### keyboard_set_map

`keyboard_set_map(key1, key2)`

Remaps the key with keycode key1 so it acts as key2.

### keyboard_get_map

`keyboard_get_map(key)`

Returns the keycode that the given key is currently mapped to.

### keyboard_unset_map

`keyboard_unset_map()`

Removes all key remappings so every key maps to itself.

### keyboard_check

`keyboard_check(key)`

Returns true if the key with the given keycode is currently held down.

### keyboard_check_pressed

`keyboard_check_pressed(key)`

Returns true if the key went down since the previous step.

### keyboard_check_released

`keyboard_check_released(key)`

Returns true if the key was let go since the previous step.

### keyboard_check_direct

`keyboard_check_direct(key)`

Polls the hardware for the key state regardless of window focus; also supports left/right shift, control and alt keycodes.

### keyboard_get_numlock

`keyboard_get_numlock()`

Returns whether numlock is currently on.

### keyboard_set_numlock

`keyboard_set_numlock(on)`

Turns numlock on (true) or off (false).

### keyboard_key_press

`keyboard_key_press(key)`

Simulates pressing the key with the given keycode.

### keyboard_key_release

`keyboard_key_release(key)`

Simulates releasing the key with the given keycode.

### keyboard_clear

`keyboard_clear(key)`

Clears the key's state so it stops generating keyboard events until it repeats.

### io_clear

`io_clear()`

Clears all keyboard and mouse states.

### io_handle

`io_handle()`

Processes user input, updating keyboard and mouse status.

### keyboard_wait

`keyboard_wait()`

Pauses until the user presses a key.

### Constants: the keyboard

| Constant | Value | Meaning |
| --- | --- | --- |
| `vk_nokey` | 0 | Keycode meaning no key is pressed. |
| `vk_anykey` | 1 | Keycode meaning any key is pressed. |
| `vk_enter` | 13 | Keycode of the enter key. |
| `vk_return` | 13 | Keycode of the return key (same as vk_enter). |
| `vk_shift` | 16 | Keycode of the shift key. |
| `vk_control` | 17 | Keycode of the control key. |
| `vk_alt` | 18 | Keycode of the alt key. |
| `vk_escape` | 27 | Keycode of the escape key. |
| `vk_space` | 32 | Keycode of the space bar. |
| `vk_backspace` | 8 | Keycode of the backspace key. |
| `vk_tab` | 9 | Keycode of the tab key. |
| `vk_pause` | 19 | Keycode of the pause/break key. |
| `vk_printscreen` | 44 | Keycode of the printscreen/sysrq key. |
| `vk_left` | 37 | Keycode of the left arrow key. |
| `vk_right` | 39 | Keycode of the right arrow key. |
| `vk_up` | 38 | Keycode of the up arrow key. |
| `vk_down` | 40 | Keycode of the down arrow key. |
| `vk_home` | 36 | Keycode of the home key. |
| `vk_end` | 35 | Keycode of the end key. |
| `vk_delete` | 46 | Keycode of the delete key. |
| `vk_insert` | 45 | Keycode of the insert key. |
| `vk_pageup` | 33 | Keycode of the page up key. |
| `vk_pagedown` | 34 | Keycode of the page down key. |
| `vk_f1` | 112 | Keycode of function key F1. |
| `vk_f2` | 113 | Keycode of function key F2. |
| `vk_f3` | 114 | Keycode of function key F3. |
| `vk_f4` | 115 | Keycode of function key F4. |
| `vk_f5` | 116 | Keycode of function key F5. |
| `vk_f6` | 117 | Keycode of function key F6. |
| `vk_f7` | 118 | Keycode of function key F7. |
| `vk_f8` | 119 | Keycode of function key F8. |
| `vk_f9` | 120 | Keycode of function key F9. |
| `vk_f10` | 121 | Keycode of function key F10. |
| `vk_f11` | 122 | Keycode of function key F11. |
| `vk_f12` | 123 | Keycode of function key F12. |
| `vk_numpad0` | 96 | Keycode of number key 0 on the numeric keypad. |
| `vk_numpad1` | 97 | Keycode of number key 1 on the numeric keypad. |
| `vk_numpad2` | 98 | Keycode of number key 2 on the numeric keypad. |
| `vk_numpad3` | 99 | Keycode of number key 3 on the numeric keypad. |
| `vk_numpad4` | 100 | Keycode of number key 4 on the numeric keypad. |
| `vk_numpad5` | 101 | Keycode of number key 5 on the numeric keypad. |
| `vk_numpad6` | 102 | Keycode of number key 6 on the numeric keypad. |
| `vk_numpad7` | 103 | Keycode of number key 7 on the numeric keypad. |
| `vk_numpad8` | 104 | Keycode of number key 8 on the numeric keypad. |
| `vk_numpad9` | 105 | Keycode of number key 9 on the numeric keypad. |
| `vk_divide` | 111 | Keycode of the divide key on the numeric keypad. |
| `vk_multiply` | 106 | Keycode of the multiply key on the numeric keypad. |
| `vk_subtract` | 109 | Keycode of the subtract key on the numeric keypad. |
| `vk_add` | 107 | Keycode of the add key on the numeric keypad. |
| `vk_decimal` | 110 | Keycode of the decimal point key on the numeric keypad. |
| `vk_lshift` | 160 | Keycode of the left shift key. |
| `vk_lcontrol` | 162 | Keycode of the left control key. |
| `vk_lalt` | 164 | Keycode of the left alt key. |
| `vk_rshift` | 161 | Keycode of the right shift key. |
| `vk_rcontrol` | 163 | Keycode of the right control key. |
| `vk_ralt` | 165 | Keycode of the right alt key. |

## The mouse

The mouse position, its buttons and the cursor.

### mouse_x

*Global variable, read-only.* X-coordinate of the mouse in the room.

### mouse_y

*Global variable, read-only.* Y-coordinate of the mouse in the room.

### mouse_button

*Global variable.* The currently pressed mouse button (mb_none, mb_any, mb_left, mb_middle or mb_right).

### mouse_lastbutton

*Global variable.* The mouse button that was pressed most recently.

### mouse_check_button

`mouse_check_button(button)`

Returns true if the given mouse button is currently held down.

### mouse_check_button_pressed

`mouse_check_button_pressed(button)`

Returns true if the button was pressed since the previous step.

### mouse_check_button_released

`mouse_check_button_released(button)`

Returns true if the button was released since the previous step.

### mouse_wheel_up

`mouse_wheel_up()`

Returns true if the wheel was scrolled up since the previous step.

### mouse_wheel_down

`mouse_wheel_down()`

Returns true if the wheel was scrolled down since the previous step.

### mouse_clear

`mouse_clear(button)`

Clears the button's state so it creates no mouse events until released and pressed again.

### mouse_wait

`mouse_wait()`

Pauses until the user presses a mouse button.

### cursor_sprite

*Global variable.* Sprite drawn automatically at the mouse position as a cursor; -1 (default) means none.

### Constants: the mouse

| Constant | Value | Meaning |
| --- | --- | --- |
| `mb_any` | -1 | Mouse button constant meaning any button. |
| `mb_none` | 0 | Mouse button constant meaning no button. |
| `mb_left` | 1 | Mouse button constant for the left button. |
| `mb_right` | 2 | Mouse button constant for the right button. |
| `mb_middle` | 3 | Mouse button constant for the middle button. |

## The joystick

Joystick axes, buttons and directions.

### joystick_exists

`joystick_exists(id)`

Returns whether joystick id (1 or 2) is present.

### joystick_direction

`joystick_direction(id)`

Returns the keycode (vk_numpad1 to vk_numpad9) matching the joystick's direction.

### joystick_name

`joystick_name(id)`

Returns the name of the joystick.

### joystick_axes

`joystick_axes(id)`

Returns the number of axes the joystick has.

### joystick_buttons

`joystick_buttons(id)`

Returns the number of buttons the joystick has.

### joystick_has_pov

`joystick_has_pov(id)`

Returns whether the joystick supports a point-of-view hat.

### joystick_check_button

`joystick_check_button(id, button)`

Returns whether the given joystick button (1-32) is pressed.

### joystick_xpos

`joystick_xpos(id)`

Returns the x-axis position of the joystick, from -1 to 1.

### joystick_ypos

`joystick_ypos(id)`

Returns the y-axis position of the joystick.

### joystick_zpos

`joystick_zpos(id)`

Returns the z-axis position of the joystick, if it has one.

### joystick_rpos

`joystick_rpos(id)`

Returns the rudder (fourth axis) position of the joystick.

### joystick_upos

`joystick_upos(id)`

Returns the u (fifth axis) position of the joystick.

### joystick_vpos

`joystick_vpos(id)`

Returns the v (sixth axis) position of the joystick.

### joystick_pov

`joystick_pov(id)`

Returns the point-of-view angle in degrees (0 forward, 90 right, 180 back, 270 left), or -1 if none pressed.
