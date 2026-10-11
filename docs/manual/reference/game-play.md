# Game play

Moving instances around, paths, motion planning, collisions, creating and destroying instances, timing, rooms, score, and generating events.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [Moving around](#moving-around)
- [Paths](#paths)
- [Motion planning](#motion-planning)
- [Collision checking](#collision-checking)
- [Instances](#instances)
- [Deactivating instances](#deactivating-instances)
- [Timing](#timing)
- [Rooms](#rooms)
- [Score, lives and health](#score-lives-and-health)
- [Generating events](#generating-events)
- [Miscellaneous variables and functions](#miscellaneous-variables-and-functions)

## Moving around

The motion variables of an instance and functions that move it.

### x

*Instance variable.* Instance's x-position.

### y

*Instance variable.* Instance's y-position.

### xprevious

*Instance variable.* Instance's x-position before the latest move.

### yprevious

*Instance variable.* Instance's y-position before the latest move.

### xstart

*Instance variable.* x-position where the instance started in the room.

### ystart

*Instance variable.* y-position where the instance started in the room.

### hspeed

*Instance variable.* Horizontal component of the instance's speed.

### vspeed

*Instance variable.* Vertical component of the instance's speed.

### direction

*Instance variable.* Current direction of motion in degrees (0-360, counter-clockwise, 0 = right).

### speed

*Instance variable.* Current speed in pixels per step.

### friction

*Instance variable.* Friction applied to speed each step, in pixels per step.

### gravity

*Instance variable.* Gravity amount added to motion each step, in pixels per step.

### gravity_direction

*Instance variable.* Direction of gravity in degrees; 270 is downward.

### motion_set

`motion_set(direction, speed)`

Sets motion to the given speed in direction dir.

### motion_add

`motion_add(direction, speed)`

Adds a speed/direction vector to the current motion.

### place_free

`place_free(x, y)`

Returns whether the instance would be collision-free (with solids) at (x,y).

### place_empty

`place_empty(x, y)`

Returns whether the instance at (x,y) would meet no instance, solid or not.

### place_meeting

`place_meeting(x, y, obj)`

Returns whether the instance at (x,y) would meet obj (object, id, all or other).

### place_snapped

`place_snapped(hsnap, vsnap)`

Returns whether the instance is aligned to the given snap grid.

### move_random

`move_random(hsnap, vsnap)`

Moves the instance to a random free position snapped to the grid.

### move_snap

`move_snap(hsnap, vsnap)`

Snaps the instance position to the given grid.

### move_towards_point

`move_towards_point(x, y, sp)`

Sets the instance moving toward (x,y) with speed sp.

### move_contact_solid

`move_contact_solid(direction, maxdist)`

Moves in direction dir until touching a solid, up to maxdist (negative for unlimited).

### move_contact_all

`move_contact_all(direction, maxdist)`

Like move_contact_solid but stops at contact with any instance.

### move_outside_solid

`move_outside_solid(direction, maxdist)`

Moves in direction dir until no longer inside a solid, up to maxdist.

### move_outside_all

`move_outside_all(direction, maxdist)`

Like move_outside_solid but considers any instance, not just solids.

### move_bounce_solid

`move_bounce_solid(advanced)`

Bounces off solid instances; advanced also handles slanted walls.

### move_bounce_all

`move_bounce_all(advanced)`

Bounces off any instance, not just solids.

### move_wrap

`move_wrap(hor, vert, margin)`

Wraps the instance to the opposite room edge horizontally/vertically once beyond margin.

### distance_to_point

`distance_to_point(x, y)`

Returns distance from the instance's bounding box to the point (x,y).

### distance_to_object

`distance_to_object(obj)`

Returns distance to the nearest instance of object obj.

### position_empty

`position_empty(x, y)`

Returns whether nothing exists at position (x,y).

### position_meeting

`position_meeting(x, y, obj)`

Returns whether an instance of obj (object, id, self, other or all) covers (x,y).

## Paths

Letting an instance follow a path.

### path_start

`path_start(path, speed, endaction, absolute)`

Starts the current instance along a path with given speed (negative runs backward), end action and absolute flag.

### path_end

`path_end()`

Stops the current instance from following its path.

### path_index

*Instance variable, read-only.* Index of the path currently followed; read-only, change via path functions.

### path_position

*Instance variable.* Position along the path from 0 (start) to 1 (end).

### path_positionprevious

*Instance variable.* Previous path position, useful to step back in collision events.

### path_speed

*Instance variable.* Speed in pixels per step along the path; negative moves backward.

### path_scale

*Instance variable.* Scale of the path the instance follows (1 is the original size).

### path_orientation

*Instance variable.* Counter-clockwise rotation applied to the path; 0 is the original orientation.

### path_endaction

*Instance variable.* Action taken when the path end is reached.

## Motion planning

Finding a way towards a goal around obstacles, step by step or as a whole path.

### mp_linear_step

`mp_linear_step(x, y, speed, checkall)`

Takes one straight step toward (x,y); stops at obstacles; returns whether the goal was reached.

### mp_potential_step

`mp_potential_step(x, y, speed, checkall)`

Takes a step toward (x,y) steering around obstacles; returns whether the goal was reached.

### mp_linear_step_object

`mp_linear_step_object(x, y, speed, obj)`

Straight step toward (x,y), treating only instances of obj as obstacles.

### mp_potential_step_object

`mp_potential_step_object(x, y, speed, obj)`

Obstacle-avoiding step toward (x,y), treating only instances of obj as obstacles.

### mp_potential_settings

`mp_potential_settings(maxrot, rotstep, ahead, onspot)`

Tunes potential-field stepping: max rotation, rotation step, look-ahead distance and on-spot rotation.

### mp_linear_path

`mp_linear_path(path, xgoal, ygoal, stepsize, checkall)`

Computes a straight path to (xgoal,ygoal) into an existing path; returns whether it succeeded.

### mp_potential_path

`mp_potential_path(path, xgoal, ygoal, stepsize, factor, checkall)`

Computes an obstacle-avoiding path to (xgoal,ygoal) into an existing path, limited by a length factor; returns success.

### mp_linear_path_object

`mp_linear_path_object(path, xgoal, ygoal, stepsize, obj)`

Straight path computation considering only instances of obj as obstacles.

### mp_potential_path_object

`mp_potential_path_object(path, xgoal, ygoal, stepsize, factor, obj)`

Potential-field path computation considering only instances of obj as obstacles.

### mp_grid_create

`mp_grid_create(left, top, hcells, vcells, cellwidth, cellheight)`

Creates a motion-planning grid from top-left corner, cell counts and cell size; returns its index.

### mp_grid_destroy

`mp_grid_destroy(id)`

Destroys the grid and frees its memory.

### mp_grid_clear_all

`mp_grid_clear_all(id)`

Marks every grid cell as free.

### mp_grid_clear_cell

`mp_grid_clear_cell(id, h, v)`

Marks cell (h,v) as free; cell 0,0 is top-left.

### mp_grid_clear_rectangle

`mp_grid_clear_rectangle(id, left, top, right, bottom)`

Frees all cells intersecting the given room-coordinate rectangle.

### mp_grid_add_cell

`mp_grid_add_cell(id, h, v)`

Marks cell (h,v) as forbidden; cell 0,0 is top-left.

### mp_grid_add_rectangle

`mp_grid_add_rectangle(id, left, top, right, bottom)`

Marks all cells intersecting the rectangle as forbidden.

### mp_grid_add_instances

`mp_grid_add_instances(id, obj, precision)`

Marks cells overlapped by instances of obj (or one instance) as forbidden; precision selects precise checks.

### mp_grid_path

`mp_grid_path(id, path, xstart, ystart, xgoal, ygoal, allowdiag)`

Computes a path through the grid between start and goal into an existing path; returns success.

### mp_grid_draw

`mp_grid_draw(id)`

Draws the grid with free cells green and forbidden red; slow debug aid.

## Collision checking

Testing whether shapes collide with instances.

### collision_point

`collision_point(x, y, obj, precision, notme)`

Tests whether point (x,y) collides with instances of obj. Returns an instance id or noone.

### collision_rectangle

`collision_rectangle(x1, y1, x2, y2, obj, precision, notme)`

Tests whether the filled rectangle collides with instances of obj. Returns an instance id or noone.

### collision_circle

`collision_circle(x1, y1, radius, obj, precision, notme)`

Tests whether the filled circle at the centre with given radius collides with instances of obj. Returns an instance id or noone.

### collision_ellipse

`collision_ellipse(x1, y1, x2, y2, obj, precision, notme)`

Tests whether the filled ellipse in the bounding corners collides with instances of obj. Returns an instance id or noone.

### collision_line

`collision_line(x1, y1, x2, y2, obj, precision, notme)`

Tests whether the line segment collides with instances of obj; handy for line-of-sight checks. Returns an instance id or noone.

## Instances

Instance properties, and creating, finding, changing and destroying instances.

### object_index

*Instance variable, read-only.* Index of the object this instance belongs to.

### id

*Instance variable, read-only.* Unique instance identifier (at least 100000).

### solid

*Instance variable.* Whether the instance is solid; changeable during the game.

### persistent

*Instance variable.* Whether the instance persists across rooms.

### mask_index

*Instance variable.* Sprite used as collision mask; -1 means use the instance's own sprite.

### instance_count

*Global variable, read-only.* Number of instances currently in the room.

### instance_id

*Global variable, read-only, array, indices 0 to instance_count-1.* Array of instance ids, indexed 0 to instance_count-1.

### instance_find

`instance_find(obj, n)`

Returns id of the (n+1)th instance of obj, or noone; assignments change every step.

### instance_exists

`instance_exists(obj)`

Returns whether an instance of obj (object, id or all) exists.

### instance_number

`instance_number(obj)`

Returns the number of instances of obj (or all).

### instance_position

`instance_position(x, y, obj)`

Returns id of the instance of obj at (x,y), or noone.

### instance_nearest

`instance_nearest(x, y, obj)`

Returns id of the instance of obj nearest to (x,y).

### instance_furthest

`instance_furthest(x, y, obj)`

Returns id of the instance of obj furthest from (x,y).

### instance_place

`instance_place(x, y, obj)`

Returns id of the obj instance met if this instance were at (x,y), or noone.

### instance_create

`instance_create(x, y, obj)`

Creates an instance of obj at (x,y) and returns its id.

### instance_copy

`instance_copy(performevent)`

Duplicates the current instance, optionally running its create event; returns new id.

### instance_change

`instance_change(obj, performevents)`

Turns the instance into another object, optionally running destroy and create events.

### instance_destroy

`instance_destroy()`

Destroys the current instance.

### position_destroy

`position_destroy(x, y)`

Destroys all instances whose sprite covers (x,y).

### position_change

`position_change(x, y, obj, performevents)`

Changes all instances at (x,y) into obj, optionally running destroy/create events.

## Deactivating instances

Taking instances out of the game temporarily, and bringing them back.

### instance_deactivate_all

`instance_deactivate_all(notme)`

Deactivates all instances in the room, optionally sparing the caller.

### instance_deactivate_object

`instance_deactivate_object(obj)`

Deactivates all instances of obj (or all, or one instance id).

### instance_deactivate_region

`instance_deactivate_region(left, top, width, height, inside, notme)`

Deactivates instances overlapping a region (or outside it when inside is false).

### instance_activate_all

`instance_activate_all()`

Activates all instances in the room.

### instance_activate_object

`instance_activate_object(obj)`

Activates all instances of obj (or all, or one instance id).

### instance_activate_region

`instance_activate_region(left, top, width, height, inside)`

Activates instances in a region (or outside it when inside is false).

## Timing

The room speed, alarms, time lines and the system clock.

### room_speed

*Global variable.* Game speed of the current room in steps per second.

### fps

*Global variable, read-only.* Number of frames actually drawn per second.

### current_time

*Global variable, read-only.* Milliseconds elapsed since the system started.

### current_year

*Global variable, read-only.* The current year.

### current_month

*Global variable, read-only.* The current month.

### current_day

*Global variable, read-only.* The current day of the month.

### current_weekday

*Global variable, read-only.* The current weekday (1=sunday to 7=saturday).

### current_hour

*Global variable, read-only.* The current hour.

### current_minute

*Global variable, read-only.* The current minute.

### current_second

*Global variable, read-only.* The current second.

### sleep

`sleep(milliseconds)`

Pauses the game for the given number of milliseconds.

### alarm

*Instance variable, array, indices 0 to 11.* Alarm clock values, indices 0 to 11; only decrement if the alarm event has actions.

### timeline_index

*Instance variable.* Time line attached to the instance; -1 detaches it. Does not start it.

### timeline_position

*Instance variable.* Current position within the time line; can be changed to skip or repeat.

### timeline_speed

*Instance variable.* Amount the time line advances per step; fractions, larger values and negatives allowed.

### timeline_running

*Instance variable.* Whether the time line is running; set to start or pause it.

### timeline_loop

*Instance variable.* Whether the time line restarts from the beginning after reaching its end.

## Rooms

The current room, moving between rooms, and saving and loading the game.

### room

*Global variable.* Index of the current room; assigning changes room, though room functions are preferable.

### room_first

*Global variable, read-only.* Index of the first room in the game.

### room_last

*Global variable, read-only.* Index of the last room in the game.

### room_width

*Global variable, read-only.* Width of the room in pixels.

### room_height

*Global variable, read-only.* Height of the room in pixels.

### room_caption

*Global variable.* Window caption text shown for the room.

### room_persistent

*Global variable.* Whether the current room is persistent.

### room_goto

`room_goto(number)`

Goes to the room with the given index.

### room_goto_previous

`room_goto_previous()`

Goes to the previous room.

### room_goto_next

`room_goto_next()`

Goes to the next room.

### room_previous

`room_previous(number)`

Returns the index of the room before number, or -1, without going there.

### room_next

`room_next(number)`

Returns the index of the room after number, or -1.

### room_restart

`room_restart()`

Restarts the current room.

### game_end

`game_end()`

Ends the game.

### game_restart

`game_restart()`

Restarts the game.

### game_load

`game_load(filename)`

Loads a saved game from the named file.

### game_save

`game_save(filename)`

Saves the game to the named file.

### transition_kind

*Global variable.* Kind of transition used for the next room change.

### transition_steps

*Global variable.* Number of steps a transition lasts; default 80.

### transition_color

*Global variable.* Colour used by certain room transitions.

### transition_define

`transition_define(kind, name)`

Registers a script (given by name string) as a custom transition of the given kind.

### transition_exists

`transition_exists(kind)`

Returns whether a transition of the given kind exists.

## Score, lives and health

The built-in score, lives and health values and how they are shown.

### score

*Global variable.* The current score.

### lives

*Global variable.* Number of lives remaining.

### health

*Global variable.* Current health, 0 to 100.

### show_score

*Global variable.* Whether the score is shown in the window caption.

### show_lives

*Global variable.* Whether lives are shown in the window caption.

### show_health

*Global variable.* Whether health is shown in the window caption.

### caption_score

*Global variable.* Caption text used for the score.

### caption_lives

*Global variable.* Caption text used for lives.

### caption_health

*Global variable.* Caption text used for health.

## Generating events

Running events of instances from code, and the event type and number constants.

### event_perform

`event_perform(type, number)`

Runs event number of the given type for the current instance.

### event_user

`event_user(number)`

Runs user-defined event number (0 to 15).

### event_perform_object

`event_perform_object(obj, type, number)`

Runs an event defined in another object, applied to the current instance.

### event_inherited

`event_inherited()`

Runs the parent object's version of the current event; needs a parent.

### event_type

*Global variable, read-only.* Type of the event currently executing.

### event_number

*Global variable, read-only.* Number of the event currently executing.

### event_object

*Global variable, read-only.* Index of the object whose event is executing.

### event_action

*Global variable, read-only.* Index of the executing action within the event, starting at 0.

### Constants: generating events

| Constant | Value | Meaning |
| --- | --- | --- |
| `ev_create` | 0 | Event type constant for the create event, for event_perform. |
| `ev_destroy` | 1 | Event type constant for the destroy event, for event_perform. |
| `ev_step` | 3 | Event type constant for the step event, for event_perform. |
| `ev_alarm` | 2 | Event type constant for the alarm event, for event_perform. |
| `ev_keyboard` | 5 | Event type constant for the keyboard event, for event_perform. |
| `ev_mouse` | 6 | Event type constant for the mouse event, for event_perform. |
| `ev_collision` | 4 | Event type constant for the collision event, for event_perform. |
| `ev_other` | 7 | Event type constant for the other event, for event_perform. |
| `ev_draw` | 8 | Event type constant for the draw event, for event_perform. |
| `ev_keypress` | 9 | Event type constant for the keypress event, for event_perform. |
| `ev_keyrelease` | 10 | Event type constant for the keyrelease event, for event_perform. |
| `ev_trigger` | 11 | Event type constant for the trigger event, for event_perform. |
| `ev_left_button` | 0 | Mouse event number constant (left button) for event_perform. |
| `ev_right_button` | 1 | Mouse event number constant (right button) for event_perform. |
| `ev_middle_button` | 2 | Mouse event number constant (middle button) for event_perform. |
| `ev_no_button` | 3 | Mouse event number constant (no button) for event_perform. |
| `ev_left_press` | 4 | Mouse event number constant (left press) for event_perform. |
| `ev_right_press` | 5 | Mouse event number constant (right press) for event_perform. |
| `ev_middle_press` | 6 | Mouse event number constant (middle press) for event_perform. |
| `ev_left_release` | 7 | Mouse event number constant (left release) for event_perform. |
| `ev_right_release` | 8 | Mouse event number constant (right release) for event_perform. |
| `ev_middle_release` | 9 | Mouse event number constant (middle release) for event_perform. |
| `ev_mouse_enter` | 10 | Mouse event number constant (mouse enter) for event_perform. |
| `ev_mouse_leave` | 11 | Mouse event number constant (mouse leave) for event_perform. |
| `ev_mouse_wheel_up` | 60 | Mouse event number constant (mouse wheel up) for event_perform. |
| `ev_mouse_wheel_down` | 61 | Mouse event number constant (mouse wheel down) for event_perform. |
| `ev_global_left_button` | 50 | Global mouse event number constant (left button) for event_perform. |
| `ev_global_right_button` | 51 | Global mouse event number constant (right button) for event_perform. |
| `ev_global_middle_button` | 52 | Global mouse event number constant (middle button) for event_perform. |
| `ev_global_left_press` | 53 | Global mouse event number constant (left press) for event_perform. |
| `ev_global_right_press` | 54 | Global mouse event number constant (right press) for event_perform. |
| `ev_global_middle_press` | 55 | Global mouse event number constant (middle press) for event_perform. |
| `ev_global_left_release` | 56 | Global mouse event number constant (left release) for event_perform. |
| `ev_global_right_release` | 57 | Global mouse event number constant (right release) for event_perform. |
| `ev_global_middle_release` | 58 | Global mouse event number constant (middle release) for event_perform. |
| `ev_joystick1_left` | 16 | Joystick 1 event number constant (left) for event_perform. |
| `ev_joystick1_right` | 17 | Joystick 1 event number constant (right) for event_perform. |
| `ev_joystick1_up` | 18 | Joystick 1 event number constant (up) for event_perform. |
| `ev_joystick1_down` | 19 | Joystick 1 event number constant (down) for event_perform. |
| `ev_joystick1_button1` | 21 | Joystick 1 event number constant (button1) for event_perform. |
| `ev_joystick1_button2` | 22 | Joystick 1 event number constant (button2) for event_perform. |
| `ev_joystick1_button3` | 23 | Joystick 1 event number constant (button3) for event_perform. |
| `ev_joystick1_button4` | 24 | Joystick 1 event number constant (button4) for event_perform. |
| `ev_joystick1_button5` | 25 | Joystick 1 event number constant (button5) for event_perform. |
| `ev_joystick1_button6` | 26 | Joystick 1 event number constant (button6) for event_perform. |
| `ev_joystick1_button7` | 27 | Joystick 1 event number constant (button7) for event_perform. |
| `ev_joystick1_button8` | 28 | Joystick 1 event number constant (button8) for event_perform. |
| `ev_joystick2_left` | 31 | Joystick 2 event number constant (left) for event_perform. |
| `ev_joystick2_right` | 32 | Joystick 2 event number constant (right) for event_perform. |
| `ev_joystick2_up` | 33 | Joystick 2 event number constant (up) for event_perform. |
| `ev_joystick2_down` | 34 | Joystick 2 event number constant (down) for event_perform. |
| `ev_joystick2_button1` | 36 | Joystick 2 event number constant (button1) for event_perform. |
| `ev_joystick2_button2` | 37 | Joystick 2 event number constant (button2) for event_perform. |
| `ev_joystick2_button3` | 38 | Joystick 2 event number constant (button3) for event_perform. |
| `ev_joystick2_button4` | 39 | Joystick 2 event number constant (button4) for event_perform. |
| `ev_joystick2_button5` | 40 | Joystick 2 event number constant (button5) for event_perform. |
| `ev_joystick2_button6` | 41 | Joystick 2 event number constant (button6) for event_perform. |
| `ev_joystick2_button7` | 42 | Joystick 2 event number constant (button7) for event_perform. |
| `ev_joystick2_button8` | 43 | Joystick 2 event number constant (button8) for event_perform. |
| `ev_outside` | 0 | Other-event number constant for when the object leaves the room. |
| `ev_boundary` | 1 | Other-event number constant for when the object reaches a room boundary. |
| `ev_game_start` | 2 | Other-event number constant for when the game starts. |
| `ev_game_end` | 3 | Other-event number constant for when the game ends. |
| `ev_room_start` | 4 | Other-event number constant for when the room starts. |
| `ev_room_end` | 5 | Other-event number constant for when the room ends. |
| `ev_no_more_lives` | 6 | Other-event number constant for when the lives reach zero. |
| `ev_animation_end` | 7 | Other-event number constant for when the sprite animation ends. |
| `ev_end_of_path` | 8 | Other-event number constant for when the path end is reached. |
| `ev_no_more_health` | 9 | Other-event number constant for when the health reaches zero. |
| `ev_close_button` | 30 | Other-event number constant for when the window close button is pressed. |
| `ev_user0` | 10 | Event number constant for user-defined event 0. |
| `ev_user1` | 11 | Event number constant for user-defined event 1. |
| `ev_user2` | 12 | Event number constant for user-defined event 2. |
| `ev_user3` | 13 | Event number constant for user-defined event 3. |
| `ev_user4` | 14 | Event number constant for user-defined event 4. |
| `ev_user5` | 15 | Event number constant for user-defined event 5. |
| `ev_user6` | 16 | Event number constant for user-defined event 6. |
| `ev_user7` | 17 | Event number constant for user-defined event 7. |
| `ev_user8` | 18 | Event number constant for user-defined event 8. |
| `ev_user9` | 19 | Event number constant for user-defined event 9. |
| `ev_user10` | 20 | Event number constant for user-defined event 10. |
| `ev_user11` | 21 | Event number constant for user-defined event 11. |
| `ev_user12` | 22 | Event number constant for user-defined event 12. |
| `ev_user13` | 23 | Event number constant for user-defined event 13. |
| `ev_user14` | 24 | Event number constant for user-defined event 14. |
| `ev_user15` | 25 | Event number constant for user-defined event 15. |
| `ev_step_normal` | 0 | Step-event number constant for the normal step event. |
| `ev_step_begin` | 1 | Step-event number constant for the begin-step event. |
| `ev_step_end` | 2 | Step-event number constant for the end-step event. |

## Miscellaneous variables and functions

Error handling, debug output, accessing variables by name and other functions.

### gamemaker_pro

*Global variable, read-only.* Flag telling whether the game was made with the full (paid) edition of the original tool rather than the free one.

### gamemaker_registered

*Global variable, read-only.* Same value as gamemaker_pro.

### gamemaker_version

*Global variable, read-only.* Version number of the engine (810 to 819 for version 8.1); compare it against ranges rather than exact values.

### error_occurred

*Global variable.* Whether an error has occurred.

### error_last

*Global variable.* String holding the last error message.

### show_debug_message

`show_debug_message(str)`

Shows the string in the debug output.

### debug_mode

*Global variable, read-only.* Whether the game runs in debug mode.

### variable_global_exists

`variable_global_exists(name)`

Returns whether a global variable with that name string exists.

### variable_global_get

`variable_global_get(name)`

Returns the value of the named global variable.

### variable_global_array_get

`variable_global_array_get(name, index)`

Returns element index of the named global array.

### variable_global_array2_get

`variable_global_array2_get(name, ind1, ind2)`

Returns element ind1,ind2 of the named global 2D array.

### variable_global_set

`variable_global_set(name, value)`

Sets the named global variable to a value.

### variable_global_array_set

`variable_global_array_set(name, index, value)`

Sets element index of the named global array.

### variable_global_array2_set

`variable_global_array2_set(name, ind1, ind2, value)`

Sets element ind1,ind2 of the named global 2D array.

### variable_local_exists

`variable_local_exists(name)`

Returns whether the current instance has a local variable with that name.

### variable_local_get

`variable_local_get(name)`

Returns the value of the named local variable.

### variable_local_array_get

`variable_local_array_get(name, index)`

Returns element index of the named local array.

### variable_local_array2_get

`variable_local_array2_get(name, ind1, ind2)`

Returns element ind1,ind2 of the named local 2D array.

### variable_local_set

`variable_local_set(name, value)`

Sets the named local variable to a value.

### variable_local_array_set

`variable_local_array_set(name, index, value)`

Sets element index of the named local array.

### variable_local_array2_set

`variable_local_array2_set(name, ind1, ind2, value)`

Sets element ind1,ind2 of the named local 2D array.

### set_program_priority

`set_program_priority(priority)`

Sets process priority from -3 (idle only) through 0 (normal) to +3 (real-time).

### set_application_title

`set_application_title(title)`

Sets the application title shown e.g. in the task bar.
