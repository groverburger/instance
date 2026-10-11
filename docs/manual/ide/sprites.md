# Sprites

A **sprite** is the image of an object. It can be a single picture or an animation
made of several images of the same size, called **subimages** or frames. Objects use
sprites for their look and, by default, for their collisions.

To create a sprite, choose **Resources > Create Sprite** (or the toolbar button, or
**Create Sprite** in the right-click menu of the Sprites folder). To edit one,
double-click it in the resource tree. The **Sprite Properties** form opens.

## The sprite form

The left column holds the settings; the right side shows a preview of the current
subimage and a strip of all subimages underneath.

| Field | What it does |
|---|---|
| Name | The name you use in actions and code. |
| Load Sprite… | Replaces all images with images from files (see below). |
| Edit Sprite… | Opens the [sprite editor](#the-sprite-editor) to work on the frames. |
| Save… | Saves the images as a PNG strip (all frames side by side), or as a `.gmspr` sprite file with the origin and collision settings. |
| Remove background | When loading, makes the colour of the bottom-left pixel transparent everywhere in the image. On by default for sprites. |
| Smooth edges | When loading with Remove background, softens the edges left by the removed colour. |
| Make opaque | When loading, drops any transparency in the file. |
| Width, Height, Subimages | Size of the frames and their number (read-only). |
| Origin X, Y | The point of the sprite that sits at the instance's position. |
| Center | Sets the origin to the middle of the image. |
| Precise collision checking | Uses the exact shape of the visible pixels for collisions, instead of the bounding rectangle. |
| Separate collision masks | Gives every subimage its own collision mask instead of one shared mask. |
| Mask properties | Opens the [mask settings](#collision-masks). |
| OK, Cancel | Apply or discard your changes and close the form. |

### Loading images

**Load Sprite…** accepts PNG, BMP, GIF and JPEG images, and `.gmspr` sprite files.

- Select **several files** at once to get one frame per file. Frames of different sizes
  are placed on a common canvas.
- An **animated GIF** gives one frame per GIF frame.
- A file whose name ends in **`_stripN`** before the extension, such as
  `explosion_strip16.png`, is cut horizontally into N frames.
- A `.gmspr` file replaces the whole sprite: images, origin and collision settings.

The Remove background, Smooth edges and Make opaque options apply to the images you
load next, so set them before clicking Load Sprite.

### The preview

Use the **◀** and **▶** buttons next to *Show* to step through the subimages, or click
a frame in the strip at the bottom. The **Zoom** slider enlarges the preview (0 fits
it to the window). The blue rectangle is the bounding box of the collision mask, and
the red cross marks the origin (you can turn the cross off in
[Preferences](main-window.md#preferences)).

**Click in the preview to set the origin** at that pixel.

Next to *Show* are some quick frame commands: **Delete frame**, **Move left**,
**Move right** and **Add from file…**, which adds frames from image files at the end
(scaled onto the sprite's canvas size).

## The origin

The origin is the sprite's anchor point. When an instance is at position (x, y), the
sprite is drawn so that its origin lands on (x, y). Rotation and scaling also happen
around the origin. A top-left origin (0, 0) is simple for tiles and walls; a centred
origin suits things that rotate or that you want to position by their middle. The
origin can lie outside the image.

## Collision masks

Collisions between instances are checked with their **masks**: the area of the
sprite that counts as solid. Open **Mask properties** to adjust it.

| Setting | What it does |
|---|---|
| Bounding box: Automatic | The smallest rectangle around all pixels that are more opaque than the alpha tolerance. |
| Bounding box: Full image | The whole image. |
| Bounding box: Manual | The rectangle you type in **Left**, **Right**, **Top** and **Bottom**. |
| Shape: Precise | The visible pixels inside the bounding box. Same as ticking Precise collision checking. |
| Shape: Rectangle | The whole bounding box. |
| Shape: Disk | An ellipse that fills the bounding box. |
| Shape: Diamond | A diamond that fills the bounding box. |
| Alpha tolerance | How opaque a pixel must be (0–255) to count as part of the mask. |

Precise masks are the most accurate but cost more time to check. For most objects a
rectangle, disk or diamond is accurate enough and faster.

> **Tip:** An object can use a different sprite as its mask with the **Mask** setting
> in the object form. That's handy for a character whose animation frames differ in
> shape but who should always collide the same way.

## The sprite editor

**Edit Sprite…** opens the sprite editor, a window for working on all frames of the
sprite at once. The frames are shown as thumbnails labelled `image 0`, `image 1`, …;
click one to select it, and double-click it to edit it in the
[image editor](image-editor.md). A preview panel on the left plays the animation
(**Show Preview**, **Speed**, a **Background** colour, and **Checker** for a
checkerboard behind transparent pixels).

The toolbar has **Close Saving**, **Undo**, **Redo**, and frame buttons: **Add** (an
empty frame at the end), **Insert** (an empty frame before the selected one),
**Duplicate**, **Delete**, and buttons to move the selected frame left or right.

### File menu

| Command | What it does |
|---|---|
| New… | Asks for a size and starts over with one empty frame. |
| Create from File… | Replaces all frames with images from files. |
| Add from File… | Adds images from files at the end. If their size differs from the sprite's, you choose to place them top-left, centre them, or stretch them. |
| Make opaque, Remove background, Smooth edges | Options for the two commands above. |
| Save as PNG File… | Saves all frames as one horizontal strip. |
| Create from Strip…, Add from Strip… | Cut frames out of a sprite sheet (see below). |
| Close Saving Changes | Applies the changes to the sprite form and closes. |
| Close Discarding Changes | Closes without applying. |

### Edit menu

Undo, Redo, Cut, Copy, Paste (whole frames), **Erase…** (fill the current frame or
all frames with one colour, which may be transparent), Delete, Move Left, Move Right,
**Add Empty**, **Insert Empty**, Duplicate, and **Edit…** (open the selected frame in
the image editor).

### Transform menu

Each command can apply to the current frame or to all frames.

| Command | What it does |
|---|---|
| Shift… | Moves the image by a number of pixels. |
| Mirror Horizontal, Flip Vertical | Mirrors left-right or flips upside down. |
| Rotate… | Rotates by an angle. |
| Scale… | Scales the image within the same canvas. |
| Skew… | Slants the image horizontally or vertically. |
| Resize Canvas… | Changes the canvas size, keeping the image at a chosen anchor. |
| Stretch… | Resizes the canvas and stretches the image to fit. |
| Crop… | Shrinks the canvas to the visible pixels, plus a border. |

Resize Canvas, Stretch and Crop always change every frame, because all frames of a
sprite share one size.

### Images menu

**Cycle Left** and **Cycle Right** rotate the order of the frames. The other commands
change the colours of the current frame or all frames: Black and White, Colorize,
Shift Hue, Colorize Partial, Intensity, Invert, Make Opaque, Erase a Color, Smooth
Edges, Opacity, Fade, Blur, Sharpen and Outline. They are the same as in the image
editor's [Image menu](image-editor.md#image-menu).

### Animation menu

These commands generate frames, to make simple animations quickly.

| Command | What it does |
|---|---|
| Set Length… | Repeats or cuts the animation to a number of frames. |
| Stretch… | Spreads the animation over a number of frames by repeating or dropping frames. |
| Reverse | Plays the frames backwards. |
| Add Reverse | Appends the frames in reverse order, for a back-and-forth loop. |
| Translation Sequence… | Frames that move the image by a total offset. |
| Rotation Sequence… | Frames that rotate the image, clockwise or counter-clockwise. |
| Colorize… | Frames that shift the colour. |
| Fade to Color… | Frames that fade to a colour. |
| Disappear… | Frames that fade the image out. |
| Shrink…, Grow… | Frames that shrink the image to nothing or grow it from nothing. |
| Flatten…, Raise… | Frames that squash the image flat or raise it up, horizontally or vertically. |
| Morph… | Frames that blend into the images of another file. |

### Strips (sprite sheets)

**Create from Strip…** and **Add from Strip…** first ask for an image, then show it
with the cells that will become frames outlined. Adjust:

| Setting | Meaning |
|---|---|
| Number of images | How many frames to take. |
| Images per row | How many cells there are in one row. |
| Image width, Image height | Size of one cell. |
| Horizontal / Vertical cell offset | Number of cells to skip at the start. |
| Horizontal / Vertical pixel offset | Where the first cell starts. You can also click the image to set it. |
| Horizontal / Vertical separation | Gap in pixels between cells. |

Frames are taken row by row, left to right.

See also: [The image editor](image-editor.md) · [Objects](objects.md) · [Movement and collisions](../runtime/movement-and-collisions.md)
