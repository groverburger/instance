# Game graphics

Drawing sprites, backgrounds, shapes and text, blending, surfaces, tiles, the display, the game window and views.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [Sprites and images](#sprites-and-images)
- [Backgrounds](#backgrounds)
- [Drawing sprites and backgrounds](#drawing-sprites-and-backgrounds)
- [Drawing shapes](#drawing-shapes)
- [Fonts and text](#fonts-and-text)
- [Advanced drawing](#advanced-drawing)
- [Surfaces](#surfaces)
- [Tiles](#tiles)
- [The display](#the-display)
- [The window](#the-window)
- [Views](#views)
- [Repainting the screen](#repainting-the-screen)

## Sprites and images

The sprite of an instance and how it is drawn.

### visible

*Instance variable.* Whether the instance is drawn; invisible instances stay active and still cause collisions.

### sprite_index

*Instance variable.* Index of the sprite the instance uses; changing it keeps the current subimage index.

### sprite_width

*Instance variable, read-only.* Width of the instance's sprite.

### sprite_height

*Instance variable, read-only.* Height of the instance's sprite.

### sprite_xoffset

*Instance variable, read-only.* Horizontal origin offset of the sprite.

### sprite_yoffset

*Instance variable, read-only.* Vertical origin offset of the sprite.

### image_number

*Instance variable, read-only.* Count of subimages in the current sprite.

### image_index

*Instance variable.* Currently drawn subimage (from 0); fractional values are rounded down and animation continues from it.

### image_speed

*Instance variable.* Subimage animation speed; 1 advances one subimage per step, 0 freezes on image_index.

### depth

*Instance variable.* Draw-order depth (default 0); higher values are drawn behind lower ones.

### image_xscale

*Instance variable.* Horizontal scale factor of the sprite; 1 is normal, -1 mirrors it.

### image_yscale

*Instance variable.* Vertical scale factor of the sprite; 1 is normal, -1 flips it.

### image_angle

*Instance variable.* Sprite rotation in degrees counterclockwise; 0 is unrotated.

### image_alpha

*Instance variable.* Opacity when drawing the sprite, from 0 (transparent) to 1 (opaque).

### image_blend

*Instance variable.* Blend color applied to the sprite when drawn; c_white means no tinting.

### bbox_left

*Instance variable, read-only.* Left edge of the instance's bounding box in the room, scaling included.

### bbox_right

*Instance variable, read-only.* Right edge of the instance's bounding box in the room.

### bbox_top

*Instance variable, read-only.* Top edge of the instance's bounding box in the room.

### bbox_bottom

*Instance variable, read-only.* Bottom edge of the instance's bounding box in the room.

## Backgrounds

The background images of the current room.

### background_color

*Global variable.* Background color of the room.

### background_showcolor

*Global variable.* Whether the window is cleared with the background color.

### background_visible

*Global variable, array, indices 0 to 7.* Per-background (0-7) flag for whether the background is shown.

### background_foreground

*Global variable, array, indices 0 to 7.* Per-background (0-7) flag making the background draw as a foreground.

### background_index

*Global variable, array, indices 0 to 7.* Per-background (0-7) index of the background image used.

### background_x

*Global variable, array, indices 0 to 7.* Per-background (0-7) x position of the image.

### background_y

*Global variable, array, indices 0 to 7.* Per-background (0-7) y position of the image.

### background_width

*Global variable, read-only, array, indices 0 to 7.* Width of the background image in slot 0-7.

### background_height

*Global variable, read-only, array, indices 0 to 7.* Height of the background image in slot 0-7.

### background_htiled

*Global variable, array, indices 0 to 7.* Per-background (0-7) flag for horizontal tiling.

### background_vtiled

*Global variable, array, indices 0 to 7.* Per-background (0-7) flag for vertical tiling.

### background_xscale

*Global variable, array, indices 0 to 7.* Per-background (0-7) horizontal scale; must be positive.

### background_yscale

*Global variable, array, indices 0 to 7.* Per-background (0-7) vertical scale; must be positive.

### background_hspeed

*Global variable, array, indices 0 to 7.* Per-background (0-7) horizontal scroll speed in pixels per step.

### background_vspeed

*Global variable, array, indices 0 to 7.* Per-background (0-7) vertical scroll speed in pixels per step.

### background_blend

*Global variable, array, indices 0 to 7.* Per-background (0-7) blend color; c_white is the default.

### background_alpha

*Global variable, array, indices 0 to 7.* Per-background (0-7) opacity from 0 (transparent) to 1 (opaque).

## Drawing sprites and backgrounds

Drawing sprites and backgrounds in a draw event, plainly or scaled, rotated, blended and cut.

### draw_self

`draw_self()`

Draws the instance itself using its own variables (position, scale, angle, etc.), as default drawing would.

### draw_sprite

`draw_sprite(sprite, subimg, x, y)`

Renders frame subimg of a sprite (subimg -1 = instance's current frame) with its origin placed at (x,y); unscaled, unblended.

### draw_sprite_pos

`draw_sprite_pos(sprite, subimg, x1, y1, x2, y2, x3, y3, x4, y4, alpha)`

Draws a sprite subimage stretched onto four given corner points with an alpha value.

### draw_sprite_ext

`draw_sprite_ext(sprite, subimg, x, y, xscale, yscale, rotation, color, alpha)`

Draws a sprite with x/y scale, counterclockwise rotation in degrees, blend color and alpha.

### draw_sprite_stretched

`draw_sprite_stretched(sprite, subimg, x, y, width, height)`

Draws the sprite stretched to fill the rectangle at (x,y) of size width by height.

### draw_sprite_stretched_ext

`draw_sprite_stretched_ext(sprite, subimg, x, y, width, height, color, alpha)`

Like draw_sprite_stretched but with a blend color and alpha.

### draw_sprite_tiled

`draw_sprite_tiled(sprite, subimg, x, y)`

Tiles the sprite across the whole room, with one copy placed at (x,y).

### draw_sprite_tiled_ext

`draw_sprite_tiled_ext(sprite, subimg, x, y, xscale, yscale, color, alpha)`

Tiled sprite drawing with scale factors, blend color and alpha.

### draw_sprite_part

`draw_sprite_part(sprite, subimg, left, top, width, height, x, y)`

Draws a rectangular part of a sprite subimage with the part's top-left at (x,y).

### draw_sprite_part_ext

`draw_sprite_part_ext(sprite, subimg, left, top, width, height, x, y, xscale, yscale, color, alpha)`

Draws a part of a sprite subimage with scale factors, blend color and alpha.

### draw_sprite_general

`draw_sprite_general(sprite, subimg, left, top, width, height, x, y, xscale, yscale, rotation, c1, c2, c3, c4, alpha)`

Most general sprite draw: part, scale, rotation about the part's top-left, four vertex colors and alpha.

### draw_background

`draw_background(background, x, y)`

Draws the background at (x,y), without blending or alpha.

### draw_background_ext

`draw_background_ext(background, x, y, xscale, yscale, rotation, color, alpha)`

Draws the background scaled and rotated, with blend color and alpha (0-1).

### draw_background_stretched

`draw_background_stretched(background, x, y, width, height)`

Draws the background stretched to the given region.

### draw_background_stretched_ext

`draw_background_stretched_ext(background, x, y, width, height, color, alpha)`

Stretched background drawing with blend color and alpha.

### draw_background_tiled

`draw_background_tiled(background, x, y)`

Tiles the background to fill the whole room.

### draw_background_tiled_ext

`draw_background_tiled_ext(background, x, y, xscale, yscale, color, alpha)`

Tiled background drawing with scale factors, blend color and alpha.

### draw_background_part

`draw_background_part(background, left, top, width, height, x, y)`

Draws a rectangular part of the background with its top-left at (x,y).

### draw_background_part_ext

`draw_background_part_ext(background, left, top, width, height, x, y, xscale, yscale, color, alpha)`

Draws a background part with scale factors, blend color and alpha.

### draw_background_general

`draw_background_general(background, left, top, width, height, x, y, xscale, yscale, rotation, c1, c2, c3, c4, alpha)`

Most general background draw: part, scale, rotation, four vertex colors and alpha.

## Drawing shapes

Points, lines, rectangles, circles, arrows, health bars and other shapes, and the color constants.

### draw_clear

`draw_clear(color)`

Fills the whole room with the given color, without alpha blending.

### draw_clear_alpha

`draw_clear_alpha(color, alpha)`

Fills the whole room with a color and alpha value; handy for surfaces.

### draw_point

`draw_point(x, y)`

Plots a single pixel at (x,y) using the active drawing colour.

### draw_line

`draw_line(x1, y1, x2, y2)`

Draws a line between (x1,y1) and (x2,y2).

### draw_line_width

`draw_line_width(x1, y1, x2, y2, width)`

Draws a line between two points with width width.

### draw_rectangle

`draw_rectangle(x1, y1, x2, y2, outline)`

Draws a rectangle, outlined if outline is true, filled otherwise.

### draw_roundrect

`draw_roundrect(x1, y1, x2, y2, outline)`

Draws a rounded rectangle, outlined or filled depending on outline.

### draw_triangle

`draw_triangle(x1, y1, x2, y2, x3, y3, outline)`

Draws a triangle from three points, outlined or filled.

### draw_circle

`draw_circle(x, y, r, outline)`

Draws a circle at (x,y) with radius r, outlined or filled.

### draw_ellipse

`draw_ellipse(x1, y1, x2, y2, outline)`

Draws an ellipse in the given bounding box, outlined or filled.

### draw_set_circle_precision

`draw_set_circle_precision(precision)`

Sets the segment count for circles, ellipses and rounded rectangles; 4-64, multiple of 4.

### draw_arrow

`draw_arrow(x1, y1, x2, y2, size)`

Draws an arrow from (x1,y1) to (x2,y2) with arrowhead size in pixels.

### draw_button

`draw_button(x1, y1, x2, y2, up)`

Draws a button shape in the box, raised if up is 1, pressed if 0.

### draw_path

`draw_path(path, x, y, absolute)`

Draws a path at (x,y), or at its defined position if absolute is true.

### draw_healthbar

`draw_healthbar(x1, y1, x2, y2, amount, backcol, mincol, maxcol, direction, showback, showborder)`

Draws a bar filled to amount (0-100) with back color, min/max colors interpolated, direction 0-3 and optional background/border.

### draw_getpixel

`draw_getpixel(x, y)`

Returns the color of the room pixel at (x,y); slow, so use sparingly.

### draw_set_color

`draw_set_color(color)`

Sets the color used for subsequent primitive drawing.

### draw_set_alpha

`draw_set_alpha(alpha)`

Sets the alpha (0-1) used for subsequent primitive drawing.

### draw_get_color

`draw_get_color()`

Returns the current drawing color.

### draw_get_alpha

`draw_get_alpha()`

Returns the current drawing alpha.

### make_color_rgb

`make_color_rgb(red, green, blue)`

Builds a color from red, green and blue components, each 0-255.

### make_color_hsv

`make_color_hsv(hue, saturation, value)`

Builds a color from hue, saturation and value components, each 0-255.

### color_get_red

`color_get_red(color)`

Returns the red component of a color.

### color_get_green

`color_get_green(color)`

Returns the green component of a color.

### color_get_blue

`color_get_blue(color)`

Returns the blue component of a color.

### color_get_hue

`color_get_hue(color)`

Returns the hue component of a color.

### color_get_saturation

`color_get_saturation(color)`

Returns the saturation component of a color.

### color_get_value

`color_get_value(color)`

Returns the value (brightness) component of a color.

### merge_color

`merge_color(col1, col2, amount)`

Blends col1 and col2; amount 0 gives col1, 1 gives col2, between interpolates.

### screen_save

`screen_save(filename)`

Saves the screen as a png file with the given name.

### screen_save_part

`screen_save_part(filename, x, y, width, height)`

Saves a rectangular part of the screen as a png file.

### Constants: drawing shapes

| Constant | Value | Meaning |
| --- | --- | --- |
| `c_aqua` | 16776960 | Predefined color constant: aqua. |
| `c_black` | 0 | Predefined color constant: black. |
| `c_blue` | 16711680 | Predefined color constant: blue. |
| `c_dkgray` | 4210752 | Predefined color constant: dark gray. |
| `c_fuchsia` | 16711935 | Predefined color constant: fuchsia. |
| `c_gray` | 8421504 | Predefined color constant: gray. |
| `c_green` | 32768 | Predefined color constant: green. |
| `c_lime` | 65280 | Predefined color constant: lime. |
| `c_ltgray` | 12632256 | Predefined color constant: light gray. |
| `c_maroon` | 128 | Predefined color constant: maroon. |
| `c_navy` | 8388608 | Predefined color constant: navy. |
| `c_olive` | 32896 | Predefined color constant: olive. |
| `c_purple` | 8388736 | Predefined color constant: purple. |
| `c_red` | 255 | Predefined color constant: red. |
| `c_silver` | 12632256 | Predefined color constant: silver. |
| `c_teal` | 8421376 | Predefined color constant: teal. |
| `c_white` | 16777215 | Predefined color constant: white. |
| `c_yellow` | 65535 | Predefined color constant: yellow. |
| `c_orange` | 4235519 | Predefined color constant: orange. |

## Fonts and text

Choosing a font and alignment, drawing text and measuring it.

### draw_set_font

`draw_set_font(font)`

Sets the font for drawing text; -1 selects the default (Arial 12).

### draw_set_halign

`draw_set_halign(halign)`

Sets horizontal text alignment (fa_left, fa_center, fa_right).

### draw_set_valign

`draw_set_valign(valign)`

Sets vertical text alignment (fa_top, fa_middle, fa_bottom).

### draw_text

`draw_text(x, y, string)`

Draws a string at (x,y) in the current color and alpha; # or chr(10)/chr(13) start a new line, \\# is a literal #.

### draw_text_ext

`draw_text_ext(x, y, string, separation, width)`

Draws multi-line text with line separation sep (-1 default) and wrap width width in pixels (-1 no wrapping).

### string_width

`string_width(string)`

Returns the pixel width of the string as draw_text would render it in the current font.

### string_height

`string_height(string)`

Returns the pixel height of the string as draw_text would render it.

### string_width_ext

`string_width_ext(string, separation, width)`

Returns the width of the string as draw_text_ext would render it.

### string_height_ext

`string_height_ext(string, separation, width)`

Returns the height of the string as draw_text_ext would render it.

### draw_text_transformed

`draw_text_transformed(x, y, string, xscale, yscale, angle)`

Draws text scaled by xscale/yscale and rotated counterclockwise by angle degrees.

### draw_text_ext_transformed

`draw_text_ext_transformed(x, y, string, separation, width, xscale, yscale, angle)`

Combines draw_text_ext with scaling and rotation for multi-line text.

### draw_text_color

`draw_text_color(x, y, string, c1, c2, c3, c4, alpha)`

Draws text with four corner colors (top-left, top-right, bottom-right, bottom-left) and alpha 0-1.

### draw_text_ext_color

`draw_text_ext_color(x, y, string, separation, width, c1, c2, c3, c4, alpha)`

draw_text_ext with per-vertex colors and alpha.

### draw_text_transformed_color

`draw_text_transformed_color(x, y, string, xscale, yscale, angle, c1, c2, c3, c4, alpha)`

draw_text_transformed with per-vertex colors and alpha.

### draw_text_ext_transformed_color

`draw_text_ext_transformed_color(x, y, string, separation, width, xscale, yscale, angle, c1, c2, c3, c4, alpha)`

draw_text_ext_transformed with per-vertex colors and alpha.

### Constants: fonts and text

| Constant | Value | Meaning |
| --- | --- | --- |
| `fa_left` | 0 | Horizontal alignment constant: left. |
| `fa_center` | 1 | Horizontal alignment constant: center. |
| `fa_right` | 2 | Horizontal alignment constant: right. |
| `fa_top` | 0 | Vertical alignment constant: top. |
| `fa_middle` | 1 | Vertical alignment constant: middle. |
| `fa_bottom` | 2 | Vertical alignment constant: bottom. |

## Advanced drawing

Colored shapes, primitives, textures and blend modes.

### YoYo_GetPlatform

`YoYo_GetPlatform()`

Undocumented extension function that reports the platform the game runs on.

### YoYo_EnableAlphaBlend

`YoYo_EnableAlphaBlend(enable)`

Undocumented extension function that turns alpha blending on or off.

### draw_point_color

`draw_point_color(x, y, col1)`

Plots a single pixel at (x,y) using colour col1.

### draw_line_color

`draw_line_color(x1, y1, x2, y2, col1, col2)`

Draws a line with color interpolated from col1 to col2.

### draw_line_width_color

`draw_line_width_color(x1, y1, x2, y2, width, col1, col2)`

Draws a line of width width with color interpolated from col1 to col2.

### draw_rectangle_color

`draw_rectangle_color(x1, y1, x2, y2, col1, col2, col3, col4, outline)`

Draws a rectangle with a color at each corner (TL, TR, BR, BL), outlined or filled.

### draw_roundrect_color

`draw_roundrect_color(x1, y1, x2, y2, col1, col2, outline)`

Draws a rounded rectangle with col1 in the middle and col2 at the edge.

### draw_triangle_color

`draw_triangle_color(x1, y1, x2, y2, x3, y3, col1, col2, col3, outline)`

Draws a triangle with per-vertex colors interpolated across it.

### draw_circle_color

`draw_circle_color(x, y, r, col1, col2, outline)`

Draws a circle with col1 at the center and col2 at the boundary.

### draw_ellipse_color

`draw_ellipse_color(x1, y1, x2, y2, col1, col2, outline)`

Draws an ellipse with col1 at the center and col2 at the boundary.

### draw_primitive_begin

`draw_primitive_begin(kind)`

Starts describing a primitive of the given kind.

### draw_vertex

`draw_vertex(x, y)`

Adds a vertex at (x,y) using the current color and alpha.

### draw_vertex_color

`draw_vertex_color(x, y, color, alpha)`

Adds a vertex with its own color and alpha, enabling smooth gradients.

### draw_primitive_end

`draw_primitive_end()`

Ends the primitive description and draws it.

### sprite_get_texture

`sprite_get_texture(sprite, subimg)`

Returns the texture id for a subimage of the sprite.

### background_get_texture

`background_get_texture(background)`

Returns the texture id for the background.

### texture_preload

`texture_preload(texture)`

Loads the texture into video memory immediately.

### texture_set_priority

`texture_set_priority(texture, priority)`

Sets a texture's retention priority when video memory is short; default 0, use positive values.

### texture_get_width

`texture_get_width(texture)`

Returns the texture width, in the range 0-1.

### texture_get_height

`texture_get_height(texture)`

Returns the texture height, in the range 0-1.

### draw_primitive_begin_texture

`draw_primitive_begin_texture(kind, texture)`

Starts a primitive of the given kind that uses the given texture.

### draw_vertex_texture

`draw_vertex_texture(x, y, xtex, ytex)`

Adds a vertex at (x,y) with texture coordinates (xtex,ytex), blended with the current color/alpha.

### draw_vertex_texture_color

`draw_vertex_texture_color(x, y, xtex, ytex, color, alpha)`

Adds a textured vertex with its own color and alpha.

### texture_set_interpolation

`texture_set_interpolation(linear)`

Chooses linear interpolation (true) or nearest pixel (false, default) for textures, sprites and backgrounds.

### texture_set_blending

`texture_set_blending(blend)`

Enables or disables color/alpha blending (default true); also affects sprites and backgrounds.

### texture_set_repeat

`texture_set_repeat(repeat)`

Sets whether texture coordinates beyond 1 repeat the texture; reset after drawing sprites or backgrounds.

### draw_set_blend_mode

`draw_set_blend_mode(mode)`

Sets the blend mode (bm_normal, bm_add, bm_subtract, bm_max); remember to restore normal afterwards.

### draw_set_blend_mode_ext

`draw_set_blend_mode_ext(src, dest)`

Sets custom source and destination blend factors; the result is src\*factor plus dest\*factor.

### Constants: advanced drawing

| Constant | Value | Meaning |
| --- | --- | --- |
| `pr_pointlist` | 1 | Primitive kind: vertices are drawn as separate points. |
| `pr_linelist` | 2 | Primitive kind: each vertex pair forms a line segment, so an even vertex count is needed. |
| `pr_linestrip` | 3 | Primitive kind: vertices form a connected polyline that is not closed automatically. |
| `pr_trianglelist` | 4 | Primitive kind: each vertex triple forms a triangle; count must be a multiple of 3. |
| `pr_trianglestrip` | 5 | Primitive kind: each new vertex forms a triangle with the previous two. |
| `pr_trianglefan` | 6 | Primitive kind: triangles all share the first vertex, each new vertex adding one. |
| `bm_normal` | 0 | Blend mode constant: normal blending (src alpha, inverse src alpha). |
| `bm_add` | 1 | Blend mode constant: additive blending. |
| `bm_max` | 2 | Blend mode constant: maximum blending. |
| `bm_subtract` | 3 | Blend mode constant: subtractive blending. |
| `bm_zero` | 1 | Blend factor (0,0,0,0). |
| `bm_one` | 2 | Blend factor (1,1,1,1). |
| `bm_src_color` | 3 | Blend factor equal to the source color (Rs,Gs,Bs,As). |
| `bm_inv_src_color` | 4 | Blend factor equal to one minus the source color. |
| `bm_src_alpha` | 5 | Blend factor equal to the source alpha in all channels. |
| `bm_inv_src_alpha` | 6 | Blend factor equal to one minus the source alpha. |
| `bm_dest_alpha` | 7 | Blend factor equal to the destination alpha. |
| `bm_inv_dest_alpha` | 8 | Blend factor equal to one minus the destination alpha. |
| `bm_dest_color` | 9 | Blend factor equal to the destination color. |
| `bm_inv_dest_color` | 10 | Blend factor equal to one minus the destination color. |
| `bm_src_alpha_sat` | 11 | Blend factor (f,f,f,1) with f = min(As, 1-Ad). |

## Surfaces

Drawing into images in memory and using them afterwards.

### surface_create

`surface_create(width, height)`

Creates a width by height surface and returns its id; contents are not cleared.

### surface_free

`surface_free(id)`

Frees the memory used by the surface.

### surface_exists

`surface_exists(id)`

Returns whether the surface id exists; check before use since surfaces can be lost.

### surface_get_width

`surface_get_width(id)`

Returns the surface width.

### surface_get_height

`surface_get_height(id)`

Returns the surface height.

### surface_get_texture

`surface_get_texture(id)`

Returns the texture of the surface, usable for textured drawing.

### surface_set_target

`surface_set_target(id)`

Makes the surface the drawing target and resets projection to cover it.

### surface_reset_target

`surface_reset_target()`

Restores drawing to the normal screen.

### draw_surface

`draw_surface(id, x, y)`

Draws the surface at (x,y) without blending or alpha.

### draw_surface_stretched

`draw_surface_stretched(id, x, y, width, height)`

Draws the surface stretched to the given region.

### draw_surface_tiled

`draw_surface_tiled(id, x, y)`

Tiles the surface over the whole room.

### draw_surface_part

`draw_surface_part(id, left, top, width, height, x, y)`

Draws a rectangular part of the surface at (x,y).

### draw_surface_ext

`draw_surface_ext(id, x, y, xscale, yscale, rotation, color, alpha)`

Draws the surface scaled and rotated with blend color and alpha.

### draw_surface_stretched_ext

`draw_surface_stretched_ext(id, x, y, width, height, color, alpha)`

Stretched surface drawing with blend color and alpha.

### draw_surface_tiled_ext

`draw_surface_tiled_ext(id, x, y, xscale, yscale, color, alpha)`

Tiled surface drawing with scale factors, color and alpha.

### draw_surface_part_ext

`draw_surface_part_ext(id, left, top, width, height, x, y, xscale, yscale, color, alpha)`

Draws a surface part with scale factors, color and alpha.

### draw_surface_general

`draw_surface_general(id, left, top, width, height, x, y, xscale, yscale, rotation, c1, c2, c3, c4, alpha)`

Most general surface draw: part, scale, rotation, four vertex colors and alpha.

### surface_getpixel

`surface_getpixel(id, x, y)`

Returns the color of the surface pixel at (x,y); slow.

### surface_save

`surface_save(id, filename)`

Saves the surface as a png file.

### surface_save_part

`surface_save_part(id, filename, x, y, width, height)`

Saves part of the surface as a png file.

### surface_copy

`surface_copy(destination, x, y, source)`

Copies the source surface onto the destination at (x,y) without blending.

### surface_copy_part

`surface_copy_part(destination, x, y, source, xs, ys, ws, hs)`

Copies a part of the source surface into the destination at (x,y) without blending.

## Tiles

Adding, finding, changing and removing tiles.

### tile_add

`tile_add(background, left, top, width, height, x, y, depth)`

Adds a tile from a background region at (x,y) with the given depth; returns its id.

### tile_delete

`tile_delete(id)`

Deletes the tile with the given id.

### tile_exists

`tile_exists(id)`

Returns whether a tile with the id exists.

### tile_get_x

`tile_get_x(id)`

X coordinate of the tile identified by id.

### tile_get_y

`tile_get_y(id)`

Y coordinate of the tile identified by id.

### tile_get_left

`tile_get_left(id)`

Left edge (within its background) of the tile's source rectangle.

### tile_get_top

`tile_get_top(id)`

Top edge (within its background) of the tile's source rectangle.

### tile_get_width

`tile_get_width(id)`

Width of the tile's source rectangle.

### tile_get_height

`tile_get_height(id)`

Height of the tile's source rectangle.

### tile_get_depth

`tile_get_depth(id)`

Depth at which the tile is drawn.

### tile_get_visible

`tile_get_visible(id)`

Returns the visibility of the tile with the given id.

### tile_get_xscale

`tile_get_xscale(id)`

Returns the x scale of the tile with the given id.

### tile_get_yscale

`tile_get_yscale(id)`

Returns the y scale of the tile with the given id.

### tile_get_background

`tile_get_background(id)`

Background resource the tile takes its image from.

### tile_get_blend

`tile_get_blend(id)`

Returns the blend color of the tile with the given id.

### tile_get_alpha

`tile_get_alpha(id)`

Returns the alpha of the tile with the given id.

### tile_set_position

`tile_set_position(id, x, y)`

Sets the tile's position.

### tile_set_region

`tile_set_region(id, left, top, width, height)`

Sets the region of the tile within its background.

### tile_set_background

`tile_set_background(id, background)`

Sets the tile's background.

### tile_set_visible

`tile_set_visible(id, visible)`

Sets whether the tile is visible.

### tile_set_depth

`tile_set_depth(id, depth)`

Sets the tile's depth.

### tile_set_scale

`tile_set_scale(id, xscale, yscale)`

Sets the tile's x and y scaling.

### tile_set_blend

`tile_set_blend(id, color)`

Sets the tile's blend color.

### tile_set_alpha

`tile_set_alpha(id, alpha)`

Sets the tile's alpha.

### tile_layer_hide

`tile_layer_hide(depth)`

Hides all tiles at the given depth.

### tile_layer_show

`tile_layer_show(depth)`

Shows all tiles at the given depth.

### tile_layer_delete

`tile_layer_delete(depth)`

Deletes all tiles at the given depth.

### tile_layer_shift

`tile_layer_shift(depth, x, y)`

Shifts all tiles at the depth by (x,y), useful for scrolling layers.

### tile_layer_find

`tile_layer_find(depth, x, y)`

Returns the id of the first tile at that depth and position, or -1 if none.

### tile_layer_delete_at

`tile_layer_delete_at(depth, x, y)`

Deletes all tiles at that depth covering position (x,y).

### tile_layer_depth

`tile_layer_depth(depth, newdepth)`

Moves all tiles at one depth to a new depth.

## The display

The screen's size, color depth and refresh rate.

### display_get_width

`display_get_width()`

Returns the display width in pixels.

### display_get_height

`display_get_height()`

Returns the display height in pixels.

### display_get_colordepth

`display_get_colordepth()`

Returns the display color depth in bits.

### display_get_frequency

`display_get_frequency()`

Returns the display refresh frequency.

### display_set_size

`display_set_size(width, height)`

Sets the display size in pixels; returns success (only some sizes allowed).

### display_set_colordepth

`display_set_colordepth(coldepth)`

Sets the color depth (normally 16 or 32); returns success.

### display_set_frequency

`display_set_frequency(frequency)`

Sets the display refresh rate (e.g. 60); returns success.

### display_set_all

`display_set_all(width, height, frequency, coldepth)`

Sets size, frequency and color depth at once (-1 keeps a value); returns success.

### display_test_all

`display_test_all(width, height, frequency, coldepth)`

Tests whether the combination is allowed without applying it (-1 keeps a value).

### display_reset

`display_reset(aa_level)`

Restores startup display settings and sets full-screen anti-aliasing level (0, 2, 4 or 8).

### display_mouse_get_x

`display_mouse_get_x()`

Returns the mouse x-coordinate on the display.

### display_mouse_get_y

`display_mouse_get_y()`

Returns the mouse y-coordinate on the display.

### display_mouse_set

`display_mouse_set(x, y)`

Moves the mouse to (x,y) on the display.

### display_aa

*Global variable, read-only.* Bitmask of supported antialiasing levels (2 = 2x, 4 = 4x, 8 = 8x bits summed, e.g. 14 for all).

## The window

The game window: full screen, caption, size, position, cursor and scaling.

### window_set_visible

`window_set_visible(visible)`

Shows or hides the game window; no keyboard events arrive while hidden.

### window_get_visible

`window_get_visible()`

Returns whether the window is visible.

### window_set_fullscreen

`window_set_fullscreen(full)`

Sets full-screen mode on or off.

### window_get_fullscreen

`window_get_fullscreen()`

Returns whether the window is full screen.

### window_set_showborder

`window_set_showborder(show)`

Sets whether the window border is shown (never in full screen).

### window_get_showborder

`window_get_showborder()`

Returns whether the window border is shown.

### window_set_showicons

`window_set_showicons(show)`

Sets whether the border icons (minimize, maximize, close) are shown.

### window_get_showicons

`window_get_showicons()`

Returns whether the border icons are shown.

### window_set_stayontop

`window_set_stayontop(stay)`

Sets whether the window always stays above other windows.

### window_get_stayontop

`window_get_stayontop()`

Returns whether the window stays on top.

### window_set_sizeable

`window_set_sizeable(sizeable)`

Sets whether the player can resize the window (needs a border, not full screen).

### window_get_sizeable

`window_get_sizeable()`

Returns whether the player can resize the window.

### window_set_caption

`window_set_caption(caption)`

Sets the window caption, visible only with a border outside full screen.

### window_get_caption

`window_get_caption()`

Returns the window caption.

### window_set_cursor

`window_set_cursor(cursor)`

Sets the mouse cursor shape shown in the window using a cr_ constant.

### window_get_cursor

`window_get_cursor()`

Returns the cursor currently used in the window.

### window_set_color

`window_set_color(color)`

Sets the color of the window area not covered by the room.

### window_get_color

`window_get_color()`

Returns the window background color.

### window_set_region_scale

`window_set_region_scale(scale, adaptwindow)`

Scales the drawing region in the window: 1 none, 0 fill window, negative fit keeping aspect; adaptwindow may resize window.

### window_get_region_scale

`window_get_region_scale()`

Returns the drawing region scale factor.

### window_set_position

`window_set_position(x, y)`

Sets the position of the window's client area.

### window_set_size

`window_set_size(width, height)`

Sets the size of the window's client area, kept large enough for the drawing region.

### window_set_rectangle

`window_set_rectangle(x, y, width, height)`

Sets window position and size in one call.

### window_center

`window_center()`

Centers the window on the screen.

### window_default

`window_default()`

Resets the window to default size and centered position.

### window_get_x

`window_get_x()`

Returns the window's x-coordinate.

### window_get_y

`window_get_y()`

Returns the window's y-coordinate.

### window_get_width

`window_get_width()`

Returns the window width.

### window_get_height

`window_get_height()`

Returns the window height.

### window_mouse_get_x

`window_mouse_get_x()`

Returns the mouse x-coordinate within the window.

### window_mouse_get_y

`window_mouse_get_y()`

Returns the mouse y-coordinate within the window.

### window_mouse_set

`window_mouse_set(x, y)`

Moves the mouse to (x,y) within the window.

### Constants: the window

| Constant | Value | Meaning |
| --- | --- | --- |
| `cr_default` | 0 | Cursor constant for the default mouse cursor. |
| `cr_none` | -1 | Cursor constant for the invisible mouse cursor. |
| `cr_arrow` | -2 | Cursor constant for the arrow mouse cursor. |
| `cr_cross` | -3 | Cursor constant for the crosshair mouse cursor. |
| `cr_beam` | -4 | Cursor constant for the text beam mouse cursor. |
| `cr_size_nesw` | -6 | Cursor constant for the diagonal NE-SW resize mouse cursor. |
| `cr_size_ns` | -7 | Cursor constant for the vertical resize mouse cursor. |
| `cr_size_nwse` | -8 | Cursor constant for the diagonal NW-SE resize mouse cursor. |
| `cr_size_we` | -9 | Cursor constant for the horizontal resize mouse cursor. |
| `cr_uparrow` | -10 | Cursor constant for the up arrow mouse cursor. |
| `cr_hourglass` | -11 | Cursor constant for the hourglass mouse cursor. |
| `cr_drag` | -12 | Cursor constant for the drag mouse cursor. |
| `cr_nodrop` | -13 | Cursor constant for the no-drop mouse cursor. |
| `cr_hsplit` | -14 | Cursor constant for the horizontal split mouse cursor. |
| `cr_vsplit` | -15 | Cursor constant for the vertical split mouse cursor. |
| `cr_multidrag` | -16 | Cursor constant for the multi-drag mouse cursor. |
| `cr_sqlwait` | -17 | Cursor constant for the SQL wait mouse cursor. |
| `cr_no` | -18 | Cursor constant for the no/forbidden mouse cursor. |
| `cr_appstart` | -19 | Cursor constant for the app-starting mouse cursor. |
| `cr_help` | -20 | Cursor constant for the help mouse cursor. |
| `cr_handpoint` | -21 | Cursor constant for the pointing hand mouse cursor. |
| `cr_size_all` | -22 | Cursor constant for the four-way resize mouse cursor. |

## Views

The parts of the room that are shown on the screen.

### view_enabled

*Global variable.* Whether views are enabled.

### view_current

*Global variable, read-only.* Index (0-7) of the view being drawn; use in draw events only.

### view_visible

*Global variable, array, indices 0 to 7.* Per-view (0-7) flag for whether the view is shown.

### view_xview

*Global variable, array, indices 0 to 7.* Array over views 0-7: left room coordinate of the area each view shows.

### view_yview

*Global variable, array, indices 0 to 7.* Array over views 0-7: top room coordinate of the area each view shows.

### view_wview

*Global variable, array, indices 0 to 7.* Per-view (0-7) width of the view in the room.

### view_hview

*Global variable, array, indices 0 to 7.* Per-view (0-7) height of the view in the room.

### view_xport

*Global variable, array, indices 0 to 7.* Array over views 0-7: left screen/window coordinate where the view is drawn.

### view_yport

*Global variable, array, indices 0 to 7.* Array over views 0-7: top screen/window coordinate where the view is drawn.

### view_wport

*Global variable, array, indices 0 to 7.* Per-view (0-7) width of the viewport.

### view_hport

*Global variable, array, indices 0 to 7.* Per-view (0-7) height of the viewport.

### view_angle

*Global variable, array, indices 0 to 7.* Per-view (0-7) rotation in degrees counterclockwise.

### view_hborder

*Global variable, array, indices 0 to 7.* Per-view (0-7) horizontal border in pixels kept around the followed object.

### view_vborder

*Global variable, array, indices 0 to 7.* Per-view (0-7) vertical border in pixels kept around the followed object.

### view_hspeed

*Global variable, array, indices 0 to 7.* Per-view (0-7) maximum horizontal scroll speed.

### view_vspeed

*Global variable, array, indices 0 to 7.* Per-view (0-7) maximum vertical scroll speed.

### view_object

*Global variable, array, indices 0 to 7.* Per-view (0-7) object (or instance id) the view follows; first instance if several.

### window_set_region_size

`window_set_region_size(width, height, adaptwindow)`

Sets the drawing region size; adaptwindow decides whether the window resizes to fit.

### window_get_region_width

`window_get_region_width()`

Returns the drawing region width.

### window_get_region_height

`window_get_region_height()`

Returns the drawing region height.

### window_view_mouse_get_x

`window_view_mouse_get_x(id)`

Returns the mouse x-coordinate relative to view id.

### window_view_mouse_get_y

`window_view_mouse_get_y(id)`

Returns the mouse y-coordinate relative to view id.

### window_view_mouse_set

`window_view_mouse_set(id, x, y)`

Sets the mouse position relative to view id.

### window_views_mouse_get_x

`window_views_mouse_get_x()`

Returns the mouse x-coordinate relative to the view it is in (same as mouse_x).

### window_views_mouse_get_y

`window_views_mouse_get_y()`

Returns the mouse y-coordinate relative to the view it is in (same as mouse_y).

### window_views_mouse_set

`window_views_mouse_set(x, y)`

Sets the mouse position relative to the first visible view.

## Repainting the screen

Drawing and showing the screen from code.

### screen_redraw

`screen_redraw()`

Redraws the room by running all draw events.

### screen_refresh

`screen_refresh()`

Refreshes the screen from the current room image without running draw events.

### screen_wait_vsync

`screen_wait_vsync()`

Waits for the monitor's next vertical sync.

### set_automatic_draw

`set_automatic_draw(value)`

Sets whether the room is redrawn automatically (default true).

### set_synchronization

`set_synchronization(value)`

Sets whether drawing syncs with the monitor refresh rate.
