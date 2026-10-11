# The image editor

The image editor is a small paint program for one image: a frame of a sprite or the
image of a background. Open it from:

- the [sprite editor](sprites.md#the-sprite-editor): double-click a frame, or choose
  **Edit > Edit…**;
- the [background form](backgrounds.md): click **Edit Background…**.

The window has a menu bar and toolbar at the top, the tool box on the left, the canvas
in the middle, the colour panel on the right and a status bar at the bottom. The
status bar shows a hint for the current tool, the pixel under the mouse, the image
size and the size of the selection.

When you are done, click **Close Saving** (or **File > Close Saving Changes**) to
keep your changes, or **File > Close Discarding Changes** to throw them away. Closing
the window otherwise asks whether to save the changes.

## Drawing with two colours

The editor always has two colours: the **left colour**, used when you draw with the
left mouse button, and the **right colour**, used with the right button. The colour
panel shows both as swatches; click a swatch to choose which one the colour field
below edits.

- **Palette:** left-click a colour to make it the left colour, right-click to make it
  the right colour.
- **Colour field:** pick any colour for the swatch being edited.
- **Opacity:** 0 (fully transparent) to 255 (fully opaque) for that colour.
- **Swap colours** exchanges the two.
- **Transparent as right colour** sets the right colour to full transparency, so the
  right mouse button erases.
- **Color mode:** in **Blend** mode, a semi-transparent colour is mixed with what's
  already there; in **Replace** mode it replaces the pixels, transparency included.

## Tools

| Tool | How to use it |
|---|---|
| Pencil | Draw freehand. Shift draws straight lines; Ctrl picks the colour under the mouse. |
| Spray | Spray dots of colour. **Density** sets how many. |
| Eraser | Erase to transparency. **Strength** sets how much is removed at a time. |
| Picker | Click to take the colour under the mouse as the left (or, with the right button, the right) colour. |
| Line | Drag to draw a line. Shift snaps to horizontal, vertical or 45°. |
| Rectangle | Drag to draw a rectangle. Shift makes a square. |
| Ellipse | Drag to draw an ellipse. Shift makes a circle. |
| Fill | Fill the connected area of similar colour. |
| Change Color | Replace every pixel of the clicked colour, anywhere in the image. |
| Text | Click to place text, type it in the options, then **Apply** (or Enter). |
| Select | Drag to select a rectangle. Drag inside the selection to move it; drag with the right button to move a copy. |

Options for the current tool appear below the tool box:

- **Size** (and quick size buttons) for the Pencil, Line, Eraser, Spray, Rectangle and
  Ellipse.
- **Outline**, **Filled**, or **Outline + fill** for rectangles and ellipses. The fill
  of *Outline + fill* uses the other colour.
- **Tolerance** for Fill and Change Color: how different a colour may be and still
  count as the same (0 = exact). **Compare alpha too** also compares transparency.
- For Text: the text (`#` starts a new line), **Size**, alignment (**L**, **C**,
  **R**) and **Anti-alias**.
- For Select: a **Select All** button. Delete removes the selection, Escape ends it,
  and Ctrl+C, Ctrl+X and Ctrl+V copy, cut and paste.

## Moving around

- Turn the **mouse wheel** over the canvas to zoom in or out at the mouse position.
- Drag with the **middle mouse button** to move the view.
- The toolbar has **Zoom Out**, **Zoom In**, **No Zoom** (1:1) and **Zoom to Fit**,
  and shows the zoom level.
- **Grid** shows a pixel grid when zoomed in.
- When you edit a frame of an animated sprite, **Onion skin** shows the previous frame
  faintly behind the current one, and **◀ Prev** / **Next ▶** move to the neighbouring
  frames.

## Menus

### File menu

| Command | What it does |
|---|---|
| New… | Starts a new, empty image of a size you choose (not for sprites with several frames). |
| Open… | Replaces the image with an image file (not for sprites with several frames). |
| Previous Image, Next Image | Move to the previous or next frame of the sprite. |
| Save as PNG File… | Saves the image as a PNG file. |
| Close Saving Changes, Close Discarding Changes | Close the editor, keeping or dropping the changes. |

### Edit menu

Undo (Ctrl+Z), Redo (Ctrl+Y), Cut, Copy, Paste, Delete, Select All (Ctrl+A),
**Erase to Left Color** (fill the whole image with the left colour), and **Paste from File…** (paste an image file as a movable selection).

### View menu

Zoom In, Zoom Out, No Zoom, Zoom to Fit, **Show Grid** and, for sprite frames,
**Show Previous Frame (onion skin)**.

### Transform menu

Shift, Mirror Horizontal, Flip Vertical, Rotate, Scale, Skew, Resize Canvas, Stretch
and Crop, as described for the [sprite editor](sprites.md#transform-menu). Resize
Canvas, Stretch and Crop are not available while editing one frame of a sprite with
several frames, because all frames must keep the same size; use the sprite editor's
Transform menu for those.

### Image menu

| Command | What it does |
|---|---|
| Black and White | Removes all colour. |
| Colorize… | Gives the image one hue, with a saturation. |
| Shift Hue… | Rotates all hues by an amount. |
| Colorize Partial… | Changes only colours near a source hue (within a tolerance) to a new hue. |
| Intensity… | Changes saturation and brightness. |
| Invert | Inverts the colours. |
| Make Opaque | Removes all transparency. |
| Erase a Color… | Makes a colour (within a tolerance) transparent. |
| Smooth Edges | Softens the edges between opaque and transparent pixels. |
| Opacity… | Changes transparency, relative (%) or absolute (0–255). Fully transparent pixels stay transparent. |
| Fade… | Fades the image towards a colour by an amount. |
| Blur… | Blurs a small, medium or large amount; choose whether to blur the colour, the transparency or both. |
| Sharpen… | Sharpens (subtle, strong or special), for colour and/or transparency. |
| Outline… | Draws an outline of a colour and thickness around the visible pixels. |
| Set Transparency from Left Color | Makes every pixel of the left colour transparent. |

Commands with `…` open a small window with their settings. Every command can be
undone.

See also: [Sprites](sprites.md) · [Backgrounds](backgrounds.md) · [Keyboard shortcuts](shortcuts.md)
