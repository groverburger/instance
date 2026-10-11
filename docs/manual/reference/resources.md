# Resources

Reading the properties of the game's resources, and creating, changing and deleting resources while the game runs.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [Sprite information](#sprite-information)
- [Sound information](#sound-information)
- [Background information](#background-information)
- [Font information](#font-information)
- [Path information](#path-information)
- [Script information](#script-information)
- [Time line information](#time-line-information)
- [Object information](#object-information)
- [Room information](#room-information)
- [Changing sprites](#changing-sprites)
- [Changing sounds](#changing-sounds)
- [Changing backgrounds](#changing-backgrounds)
- [Changing fonts](#changing-fonts)
- [Changing paths](#changing-paths)
- [Running code](#running-code)
- [Changing time lines](#changing-time-lines)
- [Changing objects](#changing-objects)
- [Changing rooms](#changing-rooms)

## Sprite information

Reading the properties of sprites.

### sprite_exists

`sprite_exists(sprite)`

Returns whether a sprite with this index exists.

### sprite_get_name

`sprite_get_name(sprite)`

Returns the name of the sprite.

### sprite_get_number

`sprite_get_number(sprite)`

Returns the number of subimages of the sprite.

### sprite_get_width

`sprite_get_width(sprite)`

Returns the width of the sprite.

### sprite_get_height

`sprite_get_height(sprite)`

Returns the height of the sprite.

### sprite_get_xoffset

`sprite_get_xoffset(sprite)`

Returns the sprite x origin offset.

### sprite_get_yoffset

`sprite_get_yoffset(sprite)`

Returns the sprite y origin offset.

### sprite_get_bbox_left

`sprite_get_bbox_left(sprite)`

Returns the left edge of the sprite bounding box.

### sprite_get_bbox_right

`sprite_get_bbox_right(sprite)`

Returns the right edge of the sprite bounding box.

### sprite_get_bbox_top

`sprite_get_bbox_top(sprite)`

Returns the top edge of the sprite bounding box.

### sprite_get_bbox_bottom

`sprite_get_bbox_bottom(sprite)`

Returns the bottom edge of the sprite bounding box.

### sprite_save

`sprite_save(sprite, subimg, filename)`

Saves one subimage of a sprite to a png file.

### sprite_save_strip

`sprite_save_strip(sprite, filename)`

Saves all subimages of a sprite as a horizontal strip in a png file.

## Sound information

Reading the properties of sounds.

### sound_exists

`sound_exists(sound)`

Returns whether a sound with this index exists.

### sound_get_name

`sound_get_name(sound)`

Returns the name of the sound.

### sound_get_kind

`sound_get_kind(sound)`

Returns the sound kind: 0 normal, 1 background, 2 3d, 3 mmplayer.

### sound_get_preload

`sound_get_preload(sound)`

Returns whether the sound is set to preload.

### sound_discard

`sound_discard(sound)`

Frees the audio memory used by the sound.

### sound_restore

`sound_restore(sound)`

Reloads the sound into audio memory so it can play immediately.

## Background information

Reading the properties of backgrounds.

### background_exists

`background_exists(background)`

Returns whether a background with this index exists.

### background_get_name

`background_get_name(background)`

Returns the name of the background.

### background_get_width

`background_get_width(background)`

Returns the width of the background.

### background_get_height

`background_get_height(background)`

Returns the height of the background.

### background_save

`background_save(background, filename)`

Saves the background to a png file.

## Font information

Reading the properties of fonts.

### font_exists

`font_exists(font)`

Returns whether a font with this index exists.

### font_get_name

`font_get_name(font)`

Returns the resource name of the font.

### font_get_fontname

`font_get_fontname(font)`

Returns the underlying typeface name of the font.

### font_get_bold

`font_get_bold(font)`

Returns whether the font is bold.

### font_get_italic

`font_get_italic(font)`

Returns whether the font is italic.

### font_get_first

`font_get_first(font)`

Returns the first character index in the font.

### font_get_last

`font_get_last(font)`

Returns the last character index in the font.

## Path information

Reading the properties and points of paths.

### path_exists

`path_exists(path)`

Returns whether a path with this index exists.

### path_get_name

`path_get_name(path)`

Returns the name of the path.

### path_get_length

`path_get_length(path)`

Returns the total length of the path.

### path_get_kind

`path_get_kind(path)`

Returns the connection kind of the path: 0 straight, 1 smooth.

### path_get_closed

`path_get_closed(path)`

Returns whether the path is closed.

### path_get_precision

`path_get_precision(path)`

Returns the precision used when smoothing the path.

### path_get_number

`path_get_number(path)`

Returns the number of defining points of the path.

### path_get_point_x

`path_get_point_x(path, n)`

Returns the x of the n-th defining point (0-based).

### path_get_point_y

`path_get_point_y(path, n)`

Returns the y of the n-th defining point (0-based).

### path_get_point_speed

`path_get_point_speed(path, n)`

Returns the speed factor of the n-th defining point (0-based).

### path_get_x

`path_get_x(path, pos)`

Returns the x coordinate at relative position pos (0 to 1) along the path.

### path_get_y

`path_get_y(path, pos)`

Returns the y coordinate at relative position pos (0 to 1) along the path.

### path_get_speed

`path_get_speed(path, pos)`

Returns the speed factor at relative position pos (0 to 1) along the path.

## Script information

Reading the properties of scripts.

### script_exists

`script_exists(script)`

Returns whether a script with this index exists.

### script_get_name

`script_get_name(script)`

Returns the name of the script.

### script_get_text

`script_get_text(script)`

Returns the code text of the script.

## Time line information

Reading the properties of time lines.

### timeline_exists

`timeline_exists(timeline)`

Returns whether a time line with this index exists.

### timeline_get_name

`timeline_get_name(timeline)`

Returns the name of the time line.

## Object information

Reading the properties of objects.

### object_exists

`object_exists(obj)`

Returns whether an object with this index exists.

### object_get_name

`object_get_name(obj)`

Returns the name of the object.

### object_get_sprite

`object_get_sprite(obj)`

Returns the object default sprite index.

### object_get_solid

`object_get_solid(obj)`

Returns whether the object is solid by default.

### object_get_visible

`object_get_visible(obj)`

Returns whether the object is visible by default.

### object_get_depth

`object_get_depth(obj)`

Returns the object default depth.

### object_get_persistent

`object_get_persistent(obj)`

Returns whether the object is persistent by default.

### object_get_mask

`object_get_mask(obj)`

Returns the object mask sprite index, -1 if none.

### object_get_parent

`object_get_parent(obj)`

Returns the parent object index, negative if none.

### object_is_ancestor

`object_is_ancestor(ind1, ind2)`

True when object ind2 is a parent, grandparent, etc. of object ind1.

## Room information

Reading the properties of rooms.

### room_exists

`room_exists(room)`

Returns whether a room with this index exists.

### room_get_name

`room_get_name(room)`

Returns the name of the room.

## Changing sprites

Creating, loading, changing and deleting sprites while the game runs.

### sprite_set_offset

`sprite_set_offset(sprite, xoff, yoff)`

Sets the origin offset of a sprite.

### sprite_duplicate

`sprite_duplicate(sprite)`

Copies a sprite and returns the new index, or -1 on error.

### sprite_assign

`sprite_assign(sprite, source)`

Makes the sprite a copy of another sprite.

### sprite_merge

`sprite_merge(ind1, ind2)`

Appends the images of the second sprite onto the first, stretching if sizes differ; second is kept.

### sprite_add

`sprite_add(filename, image_count, removeback, smooth, xorig, yorig)`

Loads an image file (optionally a strip of subimages) as a new sprite and returns its index, with options for transparency, smoothing and origin.

### sprite_replace

`sprite_replace(sprite, filename, image_count, removeback, smooth, xorig, yorig)`

Like sprite_add but replaces an existing sprite; returns success.

### sprite_add_sprite

`sprite_add_sprite(filename)`

Loads a .gmspr sprite file as a new sprite and returns its index, or -1 on error.

### sprite_replace_sprite

`sprite_replace_sprite(sprite, filename)`

Like sprite_add_sprite but replaces an existing sprite; returns success.

### sprite_create_from_screen

`sprite_create_from_screen(x, y, width, height, removeback, smooth, xorig, yorig)`

Creates a sprite by copying a screen (drawing region) area and returns its index.

### sprite_add_from_screen

`sprite_add_from_screen(sprite, x, y, width, height, removeback, smooth)`

Appends a screen area as another subimage of the sprite.

### sprite_create_from_surface

`sprite_create_from_surface(id, x, y, width, height, removeback, smooth, xorig, yorig)`

Creates a sprite from an area of a surface, preserving alpha, and returns its index.

### sprite_add_from_surface

`sprite_add_from_surface(sprite, id, x, y, width, height, removeback, smooth)`

Appends an area of a surface as another subimage of the sprite.

### sprite_delete

`sprite_delete(sprite)`

Removes the sprite from memory.

### sprite_set_alpha_from_sprite

`sprite_set_alpha_from_sprite(ind, spr)`

Overwrites a sprite alpha using the intensity of another sprite; irreversible.

### sprite_collision_mask

`sprite_collision_mask(sprite, sepmasks, bboxmode, bbleft, bbright, bbtop, bbbottom, kind, tolerance)`

Changes a sprite collision mask: separate masks, bbox mode, bbox edges, mask kind and alpha tolerance.

## Changing sounds

Loading, replacing and deleting sounds while the game runs.

### sound_add

`sound_add(filename, kind, preload)`

Adds a sound from a file with kind and preload options; returns its index or -1 on failure.

### sound_replace

`sound_replace(sound, filename, kind, preload)`

Replaces an existing sound with a file, freeing the old one; returns success.

### sound_delete

`sound_delete(sound)`

Deletes the sound and frees all its memory permanently.

## Changing backgrounds

Creating, loading, changing and deleting backgrounds while the game runs.

### background_duplicate

`background_duplicate(background)`

Copies a background and returns the new index, or -1 on error.

### background_assign

`background_assign(background, source)`

Makes the background a copy of another background.

### background_add

`background_add(filename, removeback, smooth)`

Loads an image file as a new background (alpha channel used) and returns its index, or -1 on error.

### background_replace

`background_replace(background, filename, removeback, smooth)`

Like background_add but replaces an existing background; returns success.

### background_add_background

`background_add_background(filename)`

Loads a .gmbck background file as a new background and returns its index, or -1 on error.

### background_replace_background

`background_replace_background(background, filename)`

Like background_add_background but replaces an existing background; returns success.

### background_create_color

`background_create_color(width, height, color)`

Creates a solid-color background of given size and returns its index, or -1.

### background_create_gradient

`background_create_gradient(width, height, col1, col2, kind)`

Creates a gradient background (kind 0 to 5: horizontal, vertical, rectangle, ellipse, double horizontal, double vertical) and returns its index.

### background_create_from_screen

`background_create_from_screen(x, y, width, height, removeback, smooth)`

Creates a background by copying a screen (drawing region) area and returns its index.

### background_create_from_surface

`background_create_from_surface(id, x, y, width, height, removeback, smooth)`

Creates a background from a surface area, preserving alpha, and returns its index.

### background_delete

`background_delete(background)`

Removes the background from memory.

### background_set_alpha_from_background

`background_set_alpha_from_background(ind, back)`

Overwrites a background alpha using the intensity of another background; irreversible.

## Changing fonts

Adding, replacing and deleting fonts while the game runs.

### font_add

`font_add(name, size, bold, italic, first, last)`

Creates a new font from typeface name, size, bold, italic and character range; returns its index.

### font_add_sprite

`font_add_sprite(sprite, first, prop, separation)`

Creates a font from a sprite with one subimage per character, given first char, proportional flag and separation; returns index.

### font_replace

`font_replace(font, name, size, bold, italic, first, last)`

Replaces an existing font with a newly specified typeface font.

### font_replace_sprite

`font_replace_sprite(font, sprite, first, prop, separation)`

Replaces an existing font with a sprite-based font.

### font_delete

`font_delete(font)`

Deletes the font and frees its memory.

## Changing paths

Creating paths and changing their points while the game runs.

### path_set_kind

`path_set_kind(path, kind)`

Sets path connection kind: 0 straight, 1 smooth.

### path_set_closed

`path_set_closed(path, closed)`

Sets whether the path is closed.

### path_set_precision

`path_set_precision(path, precision)`

Sets smoothing precision of the path, between 1 and 8.

### path_add

`path_add()`

Creates a new empty path and returns its index.

### path_assign

`path_assign(ind, path)`

Makes the path a copy of another path.

### path_duplicate

`path_duplicate(path)`

Copies a path and returns the index of the copy.

### path_append

`path_append(ind, path)`

Appends another path to the given path.

### path_delete

`path_delete(path)`

Deletes the path.

### path_add_point

`path_add_point(path, x, y, speed)`

Appends a point at (x,y) with a speed factor (100 means normal speed).

### path_insert_point

`path_insert_point(path, n, x, y, speed)`

Inserts a point before point n with position and speed factor.

### path_change_point

`path_change_point(path, n, x, y, speed)`

Changes point n to a new position and speed factor.

### path_delete_point

`path_delete_point(path, n)`

Deletes point n from the path.

### path_clear_points

`path_clear_points(path)`

Removes all points, leaving an empty path.

### path_reverse

`path_reverse(path)`

Reverses the order of the path.

### path_mirror

`path_mirror(path)`

Mirrors the path horizontally about its center.

### path_flip

`path_flip(path)`

Flips the path vertically about its center.

### path_rotate

`path_rotate(path, angle)`

Rotates the path counterclockwise by an angle in degrees about its center.

### path_scale

`path_scale(path, xscale, yscale)`

Scales the path by x and y factors about its center.

### path_shift

`path_shift(path, xshift, yshift)`

Moves the path by the given x and y amounts.

## Running code

Running scripts by index, and running code from strings and files.

### execute_string

`execute_string(str, arg0, arg1, ...)`

Runs the code in a string, passing the given arguments.

### execute_file

`execute_file(filename, arg0, arg1, ...)`

Runs the code in a file, passing the given arguments.

### script_execute

`script_execute(script, arg0, arg1, ...)`

Runs the script with the given index and arguments.

## Changing time lines

Creating time lines and changing their moments while the game runs.

### timeline_add

`timeline_add()`

Creates a new time line and returns its index.

### timeline_delete

`timeline_delete(timeline)`

Deletes the time line; ensure no instance uses it.

### timeline_clear

`timeline_clear(timeline)`

Removes all moments from the time line; ensure it is not in use.

### timeline_moment_clear

`timeline_moment_clear(timeline, step)`

Clears all actions at a given moment of a time line.

### timeline_moment_add

`timeline_moment_add(timeline, step, codestr)`

Adds a code string action at a moment, creating the moment if needed.

## Changing objects

Creating objects and changing their properties and events while the game runs.

### object_set_sprite

`object_set_sprite(obj, sprite)`

Sets the object sprite; -1 removes it.

### object_set_solid

`object_set_solid(obj, solid)`

Sets whether new instances are solid by default.

### object_set_visible

`object_set_visible(obj, visible)`

Sets whether new instances are visible by default.

### object_set_depth

`object_set_depth(obj, depth)`

Sets the default depth of new instances.

### object_set_persistent

`object_set_persistent(obj, persistent)`

Sets whether new instances are persistent by default.

### object_set_mask

`object_set_mask(obj, sprite)`

Sets the object mask sprite; -1 uses the object own sprite.

### object_set_parent

`object_set_parent(ind, obj)`

Sets the object parent; -1 means none; changes instance behavior.

### object_add

`object_add()`

Creates a new object and returns its index for further setup and instance creation.

### object_delete

`object_delete(obj)`

Deletes the object; ensure no instances exist in any room.

### object_event_clear

`object_event_clear(obj, event_type, event_number)`

Clears all actions of one event of the object.

### object_event_add

`object_event_add(obj, event_type, event_number, codestr)`

Adds a code string action to an object event given event type and number.

## Changing rooms

Creating rooms and changing their settings, instances and tiles while the game runs.

### room_set_width

`room_set_width(room, width)`

Sets the room width.

### room_set_height

`room_set_height(room, height)`

Sets the room height.

### room_set_caption

`room_set_caption(room, str)`

Sets the room window caption.

### room_set_persistent

`room_set_persistent(room, persistent)`

Sets whether the room is persistent.

### room_set_code

`room_set_code(room, codestr)`

Sets the room creation code string.

### room_set_background_color

`room_set_background_color(room, color, show)`

Sets the room background color and whether it is shown.

### room_set_background

`room_set_background(room, bind, visible, fore, background, x, y, htiled, vtiled, hspeed, vspeed, alpha)`

Configures background bind (0-7) of a room: visibility, foreground, image, position, tiling, speed and alpha.

### room_set_view

`room_set_view(room, vind, visible, xview, yview, wview, hview, xport, yport, wport, hport, hborder, vborder, hspeed, vspeed, obj)`

Configures view vind (0-7) of a room: visibility, view and port rectangles, follow borders, speeds and followed object.

### room_set_view_enabled

`room_set_view_enabled(room, value)`

Sets whether views are enabled for the room.

### room_add

`room_add()`

Creates a new room (not in the room order) and returns its index.

### room_duplicate

`room_duplicate(room)`

Copies a room and returns the new index.

### room_assign

`room_assign(room, source)`

Makes the room a copy of another room.

### room_instance_add

`room_instance_add(room, x, y, obj)`

Adds an instance of an object to the room at a position and returns its index.

### room_instance_clear

`room_instance_clear(room)`

Removes all instances from the room.

### room_tile_add

`room_tile_add(room, background, left, top, width, height, x, y, depth)`

Adds a tile from a background region to the room at a position and depth; returns tile index.

### room_tile_add_ext

`room_tile_add_ext(room, background, left, top, width, height, x, y, depth, xscale, yscale, alpha)`

Like room_tile_add with extra x/y scale and alpha.

### room_tile_clear

`room_tile_clear(room)`

Removes all tiles from the room.
