# Drawing

Each step ends with a new frame. By default every visible instance simply draws its
sprite; give an object a **Draw** event and you decide what it draws — text, shapes,
health bars, several sprites, anything.

## When drawing happens

Once per step, after End Step, Instance draws the frame:

1. The window is cleared, and the room's background colour is filled in (if *draw
   background colour* is on).
2. The background layers that are not foregrounds are drawn.
3. Instances, tiles and particle systems are drawn in order of **depth**. Each visible
   instance runs its Draw event, or the default draw if it has none.
4. Foreground layers are drawn on top.
5. The frame is shown.

With views enabled, steps 1–4 happen once for each visible view, so **Draw events run
once per view** (`view_current` tells which one).

Only draw in the Draw event. Drawing in other events goes onto the next frame's canvas
without the view's offset, and is wiped when the frame is cleared.

Invisible instances (`visible = false`) get no Draw event at all, but they still run
their other events and still collide.

## Depth and draw order

`depth` decides the order: **higher depth is drawn first** (further back), lower and
negative depth later (in front). Typical values: background scenery 100, the player 0,
the HUD -1000. Tiles have a depth too (1000000 by default, far behind everything),
and so do particle systems.

Instances with the same depth keep the order they were drawn in on the previous frame;
new instances are added after them. So equal-depth instances are drawn in creation
order, and changing one instance's depth and back again doesn't reshuffle the others.
An instance created inside a Draw event is drawn in the same frame.

Changing `depth` takes effect at the next frame.

## The default draw and draw_self

An instance without a Draw event draws its sprite as

```gml
draw_sprite_ext(sprite_index, image_index, x, y, image_xscale, image_yscale,
    image_angle, image_blend, image_alpha);
```

which is exactly what [`draw_self()`](../reference/game-graphics.md#draw_self) does.
Nothing is drawn when `sprite_index` is -1. **As soon as an object has a Draw event, the
sprite is no longer drawn automatically** — call `draw_self()` in the event if you still
want it:

```gml
// Draw event of obj_enemy: sprite plus a small health bar
draw_self();
draw_healthbar(x - 16, y - 24, x + 16, y - 20, hp / max_hp * 100,
    c_black, c_red, c_lime, 0, true, true);
```

## Drawing state

Colour, alpha, font, alignment and blend mode are **global settings** that stay as you
left them — across instances, events and frames. If one object sets the colour to red
and forgets to set it back, everything drawn afterwards is red. Set what you need at
the start of each Draw event.

| Setting | Function | Starts as |
|---|---|---|
| Colour | [`draw_set_color(col)`](../reference/game-graphics.md#draw_set_color) | black |
| Transparency | [`draw_set_alpha(alpha)`](../reference/game-graphics.md#draw_set_alpha) | 1 (opaque) |
| Font | [`draw_set_font(font)`](../reference/game-graphics.md#draw_set_font) | -1, the default font (Arial 12) |
| Text alignment | [`draw_set_halign()`](../reference/game-graphics.md#draw_set_halign), [`draw_set_valign()`](../reference/game-graphics.md#draw_set_valign) | `fa_left`, `fa_top` |
| Blend mode | [`draw_set_blend_mode()`](../reference/game-graphics.md#draw_set_blend_mode) | `bm_normal` |
| Circle smoothness | [`draw_set_circle_precision()`](../reference/game-graphics.md#draw_set_circle_precision) | 24 segments |

The colour and alpha apply to shapes and text. Sprites and backgrounds drawn with the
plain functions (`draw_sprite`, `draw_background`) ignore them; the `_ext` variants take
their own colour and alpha arguments.

## Colours

A colour is a number: `red + 256 * green + 65536 * blue`, each part 0–255. Use the
`c_` constants (`c_white`, `c_red`, `c_lime`, `c_aqua`, …),
[`make_color_rgb()`](../reference/game-graphics.md#make_color_rgb),
[`make_color_hsv()`](../reference/game-graphics.md#make_color_hsv) or
[`merge_color()`](../reference/game-graphics.md#merge_color). In hexadecimal the order
is reversed compared to web colours: `$0000FF` is red.

## Text

[`draw_text(x, y, string)`](../reference/game-graphics.md#draw_text) draws text in the
current font, colour and alignment. Numbers must be converted first:

```gml
draw_set_color(c_white);
draw_set_font(fnt_hud);
draw_set_halign(fa_left);
draw_text(8, 8, "Score: " + string(score));
```

In drawn text, `#` starts a new line (so do real line breaks in the string). To draw an
actual `#`, write `\#`. [`draw_text_ext()`](../reference/game-graphics.md#draw_text_ext)
wraps text at a given width, and [`string_width()`](../reference/game-graphics.md#string_width)
/ [`string_height()`](../reference/game-graphics.md#string_height) measure it. Fonts are
resources made in the IDE; Instance looks up the font by name among the fonts installed
on the system and falls back to a bundled sans-serif font if it is missing.

## Sprites, backgrounds and shapes

The drawing functions in [Game graphics](../reference/game-graphics.md) cover:

- sprites — [`draw_sprite()`](../reference/game-graphics.md#draw_sprite),
  [`draw_sprite_ext()`](../reference/game-graphics.md#draw_sprite_ext) (scaled, rotated,
  tinted), [`draw_sprite_stretched()`](../reference/game-graphics.md#draw_sprite_stretched),
  [`draw_sprite_part()`](../reference/game-graphics.md#draw_sprite_part) and more;
- backgrounds — [`draw_background()`](../reference/game-graphics.md#draw_background) and its
  variants;
- shapes — [`draw_line()`](../reference/game-graphics.md#draw_line),
  [`draw_rectangle()`](../reference/game-graphics.md#draw_rectangle),
  [`draw_circle()`](../reference/game-graphics.md#draw_circle), triangles, ellipses, arrows,
  gradients, [`draw_healthbar()`](../reference/game-graphics.md#draw_healthbar), and free-form
  primitives with [`draw_primitive_begin()`](../reference/game-graphics.md#draw_primitive_begin).

The sub-image argument of the sprite functions wraps around the number of sub-images;
-1 means "the instance's current `image_index`".

## Blend modes

[`draw_set_blend_mode()`](../reference/game-graphics.md#draw_set_blend_mode) changes how
new pixels combine with what is already there: `bm_normal`, `bm_add` (brightens —
good for glows, fire and lasers), `bm_subtract` and `bm_max`. Like the other settings it
stays in effect until you change it back:

```gml
draw_set_blend_mode(bm_add);
draw_sprite(spr_glow, 0, x, y);
draw_set_blend_mode(bm_normal);
```

## Surfaces

A surface is an off-screen image you can draw onto and then draw like a sprite —
useful for effects, lighting, mini-maps or a scaled view of the game.

```gml
// Create event
surf = -1;
```

```gml
// Draw event
if (!surface_exists(surf)) {
    surf = surface_create(256, 256);
}
surface_set_target(surf);
draw_clear_alpha(c_black, 0);
draw_circle(128, 128, 64, false);
surface_reset_target();
draw_surface(surf, x, y);
```

- [`surface_create(w, h)`](../reference/game-graphics.md#surface_create) returns an id; the
  contents start undefined, so clear it first.
- Between [`surface_set_target()`](../reference/game-graphics.md#surface_set_target) and
  [`surface_reset_target()`](../reference/game-graphics.md#surface_reset_target), all drawing
  goes to the surface, in the surface's own coordinates.
- Surfaces can be lost (for example when the display mode changes), so check
  [`surface_exists()`](../reference/game-graphics.md#surface_exists) before using one, and
  free it with [`surface_free()`](../reference/game-graphics.md#surface_free) when done.
- Surfaces are not saved with `game_save()`.

## Taking over the screen

- [`set_automatic_draw(false)`](../reference/game-graphics.md#set_automatic_draw) stops the
  per-step frame. Then [`screen_redraw()`](../reference/game-graphics.md#screen_redraw) draws
  the room on demand and [`screen_refresh()`](../reference/game-graphics.md#screen_refresh)
  shows the current picture again.
- [`screen_save(fname)`](../reference/game-graphics.md#screen_save) saves the screen as an image.
- [`draw_getpixel(x, y)`](../reference/game-graphics.md#draw_getpixel) reads a pixel's colour.

## Particles

Particle systems draw lots of small moving images cheaply: smoke, sparks, rain. They
live outside rooms, have their own depth, and update and draw themselves every step.
For quick effects, [`effect_create_above()`](../reference/particles.md#effect_create_above)
and `effect_create_below()` need just one line:

```gml
effect_create_above(ef_explosion, x, y, 1, c_orange);
```

See [Particles](../reference/particles.md) for the full system.

## 3D

Instance can also draw in 3D, with a depth buffer, a camera and lighting. See
[3D graphics](3d.md).

See also: [Rooms and views](rooms-and-views.md) · [The game loop](game-loop.md) ·
[Game graphics functions](../reference/game-graphics.md)
