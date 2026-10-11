# Backgrounds

A **background** is a single image used behind (or in front of) the action in a
room: a sky, a starfield, a floor pattern. A background can also be a **tile set**, a
sheet of small images that you place one by one in a room to build a level.

Create one with **Resources > Create Background** and double-click it in the tree to
open the **Background Properties** form.

## The background form

| Field | What it does |
|---|---|
| Name | The name you use in actions and code. |
| Load Background… | Loads an image file (PNG, BMP, GIF or JPEG). |
| Edit Background… | Opens the [image editor](image-editor.md). An empty background starts as a 64×64 image. |
| Save… | Saves the image as a PNG file. |
| Remove background | When loading, makes the colour of the bottom-left pixel transparent. Off by default for backgrounds. |
| Smooth edges | When loading with Remove background, softens the edges of the removed colour. |
| Make opaque | When loading, drops any transparency in the file. |
| Width, Height | The image size (read-only). |
| Use as tile set | Marks the background as a tile set and enables the tile settings. |
| OK, Cancel | Apply or discard your changes and close the form. |

The right side shows the image at full size.

## Tile sets

Tick **Use as tile set** to describe how the image is divided into tiles. The preview
draws the tile grid over the image so you can check the numbers.

| Setting | Meaning |
|---|---|
| Tile width, Tile height | Size of one tile in pixels. |
| Horizontal offset, Vertical offset | Where the first tile starts, counted from the top-left corner. |
| Horizontal sep, Vertical sep | Gap in pixels between neighbouring tiles. |

You then place tiles in a room on the room editor's
[tiles tab](rooms.md#the-tiles-tab). Tiles are drawn very cheaply and never have
events, so they are the right choice for scenery that doesn't need to do anything.
Walls that block movement are usually invisible solid objects placed on top of the
tiles.

## Using backgrounds in rooms

Each room has eight background slots on its
[backgrounds tab](rooms.md#the-backgrounds-tab). There you choose the image, whether
it tiles across the room, scrolls, is stretched, or is drawn in front of the
instances (a foreground).

In code you can draw a background anywhere with
[`draw_background`](../reference/game-graphics.md#draw_background), or change a room's
backgrounds through variables such as `background_index[0]`.

See also: [Rooms](rooms.md) · [The image editor](image-editor.md) · [Drawing](../runtime/drawing.md)
