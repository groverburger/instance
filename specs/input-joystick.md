# Joystick input — behavioural specification

Source: manual `404_03` (functions), `111_00` (joystick events), `403_10` (event
constants); event numbers from `runtime.md` §5.1. Paraphrased. Open items:
`OPEN_QUESTIONS.md` section `[JOY]`.

GM 8.1 sits on the Windows multimedia joystick interface; a gm81-clean runner maps
modern controllers onto the same model (§5).

---

## 1. Model

* Up to **two** joysticks, ids **1** and **2** `[404_03]`. Any other id: functions
  return the "absent" values of §2.
* Per joystick the runtime keeps, refreshed once per step (before the event
  phases, at the same time as the keyboard/mouse state — runtime.md §4.2):
  * up to 6 axes `x, y, z, r, u, v`, each a real in **−1..1** (centre 0);
  * up to 32 buttons, each pressed/not pressed;
  * a point-of-view hat: angle in degrees or −1 (centred).
* Axis orientation: `x` −1 = left, +1 = right; `y` −1 = up (forward), +1 = down
  (back) — the Windows joystick convention (**[I]**, consistent with
  `joystick_direction` returning numpad keys where "up" is `vk_numpad8`).
* Normalisation of a raw device range `[min,max]`:
  `pos = (raw − min) / (max − min) · 2 − 1`. **Recommended:** no dead zone is
  applied to the values returned by `joystick_*pos` (GM returns raw-normalised
  values); dead zones apply only to directions (§3).
* `io_handle()` refreshes joystick state too (**recommended**).

## 2. Functions `[404_03]`

| Function | Returns | Absent joystick |
|---|---|---|
| `joystick_exists(id)` | whether joystick `id` (1 or 2) is present | false |
| `joystick_name(id)` | device name string | `""` |
| `joystick_axes(id)` | number of axes | 0 |
| `joystick_buttons(id)` | number of buttons | 0 |
| `joystick_has_pov(id)` | whether it has a point-of-view hat | false |
| `joystick_direction(id)` | `vk_numpad1`..`vk_numpad9` for the stick direction (§3) | `vk_numpad5` (101) **recommended** |
| `joystick_check_button(id,numb)` | whether button `numb` (1–32) is down | false |
| `joystick_xpos(id)` | x axis −1..1 | 0 |
| `joystick_ypos(id)` | y axis | 0 |
| `joystick_zpos(id)` | z axis (if present) | 0 |
| `joystick_rpos(id)` | rudder / 4th axis | 0 |
| `joystick_upos(id)` | 5th axis (u) | 0 |
| `joystick_vpos(id)` | 6th axis (v) | 0 |
| `joystick_pov(id)` | POV angle 0..360 (0 forward, 90 right, 180 back, 270 left), −1 when not pressed | −1 |

* Axes the device does not have read as 0. Buttons beyond the device's count
  read false. `numb` outside 1..32 → false.
* POV values are continuous angles in degrees; an 8-way hat yields multiples of 45.
  Raw Windows POV values are hundredths of a degree (centre = 65535); divide by 100.

## 3. Direction thresholding

`joystick_direction(id)` combines x and y into a numpad-layout key code:

```
            x < −T     |x| ≤ T     x > T
y < −T      numpad7    numpad8     numpad9
|y| ≤ T     numpad4    numpad5     numpad6
y > T       numpad1    numpad2     numpad3
```
Key codes: `vk_numpad1`=97 … `vk_numpad9`=105 (Windows virtual-key codes).

**UNSPECIFIED – recommended threshold T = 0.5** (half deflection), strict
inequality. The same thresholds drive the direction events (§4). The POV hat does
**not** contribute to `joystick_direction` or to direction events (**recommended**;
GM documents only the stick).

## 4. Joystick events

Joystick events are sub-events of the **mouse** event type (`ev_mouse` = 6)
(runtime.md §5.1):

| Sub-event | Constant | Number |
|---|---|---|
| joystick 1 left / right / up / down | `ev_joystick1_left/right/up/down` | 16 / 17 / 18 / 19 |
| joystick 1 buttons 1..8 | `ev_joystick1_button1`..`8` | 21..28 |
| joystick 2 left / right / up / down | `ev_joystick2_left/right/up/down` | 31 / 32 / 33 / 34 |
| joystick 2 buttons 1..8 | `ev_joystick2_button1`..`8` | 36..43 |

(Values marked inferred in `builtins.json`; 20 and 35 are unused gaps.)

Semantics `[111_00]`:
* Direction events: **left** while x < −T, **right** while x > T, **up** while
  y < −T, **down** while y > T. A diagonal fires **both** of its directions
  `[111_00]`.
* Button events: buttons 1..8 only; fire while the button is held.
* They are *level-triggered*: delivered **every step** while the condition holds
  (like the keyboard "held" event), not on edges.
* Delivered to all instances that have a handler (not position-dependent),
  during the mouse event phase (runtime.md §11.5), in sub-event number order.
  **Recommended:** joystick events are dispatched before the other mouse
  sub-events of that step.
* A missing joystick produces no events.

## 5. Mapping modern controllers (recommendation)

Use the `gilrs` crate (cross-platform; XInput / Windows.Gaming.Input / evdev /
IOKit backends) as the device layer.

### 5.1 Slot assignment
* Slot 1 = the first connected gamepad (lowest gilrs id at startup), slot 2 = the
  second. A slot keeps its device until it disconnects; a newly connected device
  fills the lowest free slot. `joystick_exists` reflects the current state each
  step.
* `joystick_name`: the gilrs device name (e.g. "Xbox Controller").

### 5.2 Axis/button mapping (mimics what an Xbox-style pad looks like to the
Windows multimedia joystick API, so old games behave as they did with such pads
**[I]**)

| GM | Gamepad | Notes |
|---|---|---|
| x | left stick X | right = +1 |
| y | left stick Y | **down = +1** (invert gilrs, which reports up = +) |
| z | triggers combined | `z = LT − RT` (each 0..1), so −1..1 **[I]** |
| r | right stick Y | down = +1 |
| u | right stick X | right = +1 |
| v | — | 0 |
| POV | D-pad | up 0, right 90, down 180, left 270, diagonals 45/135/225/315, none −1 |
| button 1..10 | A, B, X, Y, LB, RB, Back/Select, Start, LS click, RS click | |
| button 11+ | Guide/Mode and extra buttons, in gilrs order | optional |

`joystick_axes` = 5, `joystick_buttons` = 10, `joystick_has_pov` = true for a
standard gamepad. For non-standard devices (flight sticks, wheels) use the raw
axes in gilrs order mapped to x, y, z, r, u, v and raw buttons in order.

### 5.3 Settings
Expose a runner option to swap/disable the mapping and to treat the D-pad as the
stick for `joystick_direction`/direction events (off by default for faithfulness;
many old GM games assume the stick).

## 6. Constants (values)

| Constant | Value | Source |
|---|---|---|
| `ev_mouse` | 6 | [I] runtime.md §5.1 |
| `ev_joystick1_left/right/up/down` | 16/17/18/19 | [I] |
| `ev_joystick1_button1..8` | 21..28 | [I] |
| `ev_joystick2_left/right/up/down` | 31/32/33/34 | [I] |
| `ev_joystick2_button1..8` | 36..43 | [I] |
| `vk_numpad1..vk_numpad9` | 97..105 | Windows VK codes (builtins.json) |
