# Actions

Actions are the building blocks of behaviour. You drag them from the action palette
into an event of an [object](objects.md) or a moment of a [time line](timelines.md),
and they run in order whenever that event happens.

This page lists every action, tab by tab, in the order of the palette. For how to add,
edit and arrange actions, see [Objects](objects.md#the-action-list).

## Common settings

Many actions share some settings, shown in their window:

- **Applies to** chooses which instance the action works on: **Self** (the instance
  running the event), **Other** (the other instance in a collision), or **Object**
  (every instance of an object you pick). Actions marked *A* in the tables below have
  it.
- **Relative** (marked *R*) adds the value to the current one instead of replacing
  it. For actions that take a position, Relative means "relative to the instance's
  own position": a Create Instance at (0, -16) relative creates the new instance just
  above this one.
- **Questions** (marked *Q*) test something. If the answer is yes, the next action
  runs; if no, it is skipped, or the **Else** part runs. The **NOT** checkbox reverses
  the answer. To make a question control several actions, put them between
  **Start Block** and **End Block**.

Every number field accepts a GML expression, such as `x + 16`, `random(100)` or
`global.difficulty * 2`.

> **In GML:** Each action has a GML equivalent. The descriptions below mention the
> closest function where it helps; see the [Function reference](../reference/README.md).

---

## move

### Move

| Action | Settings | What it does | |
|---|---|---|---|
| Move Fixed | directions, speed | Starts moving in one of the chosen directions (8 arrows, or the centre square for stop) at the given speed. If several directions are on, one is picked at random. | A R |
| Move Free | direction, speed | Starts moving at a speed in a direction given in degrees (0 = right, 90 = up, 180 = left, 270 = down). | A R |
| Move Towards | x, y, speed | Starts moving towards a point. | A R |
| Speed Horizontal | hor. speed | Sets the horizontal speed (`hspeed`). Positive is to the right. | A R |
| Speed Vertical | vert. speed | Sets the vertical speed (`vspeed`). Positive is downwards. | A R |
| Set Gravity | direction, gravity | Sets a constant pull in a direction, added to the speed every step. Use 270 for downwards. | A R |
| Reverse Horizontal | | Reverses the horizontal speed. | A |
| Reverse Vertical | | Reverses the vertical speed. | A |
| Set Friction | friction | Sets how much the speed decreases every step. | A R |

### Jump

| Action | Settings | What it does | |
|---|---|---|---|
| Jump to Position | x, y | Places the instance at a position. | A R |
| Jump to Start | | Places the instance back where it was created. | A |
| Jump to Random | snap hor, snap vert | Places the instance at a random position where it doesn't collide with anything solid, optionally on a grid. | A |
| Align to Grid | snap hor, snap vert | Rounds the position to a grid of the given cell size. | A |
| Wrap Screen | direction | When the instance has left the room, makes it reappear on the opposite side: horizontally, vertically or in both directions. Put it in the Outside Room event. | A |
| Move to Contact | direction, maximum, against | Moves the instance in a direction until it touches a solid (or any) instance, but at most the maximum distance (-1 for no limit). | A |
| Bounce | precise, against | Reverses the movement against solid (or all) instances, so the instance bounces off them. Put it in a collision event. *Precisely* bounces off slopes at the correct angle. | A |

### Paths

| Action | Settings | What it does | |
|---|---|---|---|
| Set Path | path, speed, at end, relative | Starts following a [path](paths.md). *At end* is stop, continue from start, continue from here, or reverse. *Relative* starts the path at the instance's position; *absolute* uses the path's own coordinates. | A |
| End Path | | Stops following the path. | A |
| Path Position | position (0-1) | Jumps to a point along the path: 0 is the start, 1 the end. | A R |
| Path Speed | speed | Changes the speed along the path. Negative speeds move backwards. | A R |

### Steps

| Action | Settings | What it does | |
|---|---|---|---|
| Step Towards | x, y, speed, stop at | Takes one step towards a point, stopping if the way is blocked by solid (or any) instances. Put it in the Step event to walk there step by step. | A R |
| Step Avoiding | x, y, speed, avoid | Takes one step towards a point, trying to steer around solid (or any) instances in the way. | A R |

---

## main1

### Objects

| Action | Settings | What it does | |
|---|---|---|---|
| Create Instance | object, x, y | Creates an instance of an object at a position. | A R |
| Create Moving | object, x, y, speed, direction | Creates an instance and sets it moving. | A R |
| Create Random | object 1–4, x, y | Creates an instance of one of up to four objects, picked at random. | A R |
| Change Instance | change into, perform events | Turns the instance into an instance of another object. With *perform events* set to *yes*, the Destroy event of the old object and the Create event of the new one run. | A |
| Destroy Instance | | Destroys the instance (its Destroy event runs). | A |
| Destroy at Position | x, y | Destroys all instances whose bounding box contains the position. | A R |

### Sprite

| Action | Settings | What it does | |
|---|---|---|---|
| Change Sprite | sprite, subimage, speed | Changes the instance's sprite. *subimage* is the frame to show (-1 keeps the current frame); *speed* is the animation speed (1 = one frame per step, 0 = no animation). | A |
| Transform Sprite | xscale, yscale, angle, mirror | Scales and rotates how the sprite is drawn, and optionally mirrors or flips it. | A |
| Color Sprite | color, alpha | Tints the sprite with a colour (white = no tint) and sets its opacity (0 to 1). | A |

### Sounds

| Action | Settings | What it does | |
|---|---|---|---|
| Play Sound | sound, loop | Plays a sound once, or over and over. | |
| Stop Sound | sound | Stops a sound. | |
| Check Sound | sound | Is the sound playing? | Q |

### Rooms

| Action | Settings | What it does | |
|---|---|---|---|
| Previous Room | transition | Goes to the previous room in the room order. | |
| Next Room | transition | Goes to the next room in the room order. | |
| Restart Room | transition | Starts the current room again. | |
| Different Room | new room, transition | Goes to a given room. | |
| Check Previous | | Is there a room before this one? | Q |
| Check Next | | Is there a room after this one? | Q |

The *transition* is a visual effect between the rooms: create, shift, interlace or
push from a side, rotate, blend or fade. The room change happens after the current
event has finished.

---

## main2

### Timing

| Action | Settings | What it does | |
|---|---|---|---|
| Set Alarm | number of steps, in alarm no | Sets one of the twelve alarm clocks. When it runs out, the Alarm event happens. | A R |
| Sleep | milliseconds, redraw | Pauses the whole game for a while, optionally redrawing the screen first. | |
| Set Time Line | time line, position, start, loop | Gives the instance a [time line](timelines.md) to run, from a position, starting now or later, looping or not. | A |
| Time Line Position | position | Jumps to a position in the time line. | A R |
| Time Line Speed | speed | Sets how fast the time line advances (1 = one step per step). | A R |
| Start Time Line | | Starts or continues the time line. | A |
| Pause Time Line | | Pauses the time line. | A |
| Stop Time Line | | Stops the time line and goes back to its start. | A |

### Info

| Action | Settings | What it does | |
|---|---|---|---|
| Display Message | message | Shows a message box and waits until the player closes it. Use `#` for a new line. | |
| Show Info | | Shows the [game information](game-information.md). | |
| Splash Text | file name | Shows the contents of a text file. | |
| Splash Image | file name | Shows an image file. | |
| Splash Webpage | webpage, open in | Meant to show a web page. Not supported in Instance: does nothing. | |
| Splash Video | file name, loop | Meant to play a video file. Not supported in Instance: does nothing. | |
| Splash Settings | caption, open in, close button, on escape, on mouse | Settings for the splash windows. Not supported in Instance: does nothing. | |

### Game

| Action | Settings | What it does | |
|---|---|---|---|
| Restart Game | | Starts the game again from the first room. | |
| End Game | | Ends the game. | |
| Save Game | file name | Saves the state of the game to a file. | |
| Load Game | file name | Loads a saved game. The load happens at the end of the step. | |

### Resources

| Action | Settings | What it does | |
|---|---|---|---|
| Replace Sprite | sprite, filename, images | Replaces a sprite's images with an image file, cut into the given number of frames. | |
| Replace Sound | sound, filename | Replaces a sound with a sound file. | |
| Replace Background | background, filename | Replaces a background's image with an image file. | |

---

## control

### Questions

| Action | Settings | What it does | |
|---|---|---|---|
| Check Empty | x, y, objects | Would the instance be free of collisions at this position? *objects* is only solid or all. | Q A R |
| Check Collision | x, y, objects | Would the instance collide at this position? The opposite of Check Empty. | Q A R |
| Check Object | object, x, y | Would the instance touch an instance of the object at this position? | Q A R |
| Test Instance Count | object, number, operation | Is the number of instances of the object equal to, smaller than, or larger than a number? | Q |
| Test Chance | sides | Throws a die with this many sides; yes if it comes up 1. | Q |
| Check Question | question | Asks the player a yes/no question. | Q |
| Test Expression | expression | Is the expression true? Any GML expression, such as `x > room_width / 2 && lives > 0`. | Q A |
| Check Mouse | button | Is this mouse button (or no button) held down? | Q |
| Check Grid | snap hor, snap vert | Is the instance exactly on a grid of this size? | Q A |

### Other

| Action | Settings | What it does | |
|---|---|---|---|
| Start Block | | Starts a block. The actions up to the matching End Block are treated as one, so a question or Repeat applies to all of them. | |
| Else | | The next action (or block) runs only when the previous question's answer was no. | |
| Exit Event | | Stops running the rest of the event's actions. | |
| End Block | | Ends a block. | |
| Repeat | times | Runs the next action (or block) a number of times. | |
| Call Parent Event | | Runs the same event of the parent object. | |

### Code

| Action | Settings | What it does | |
|---|---|---|---|
| Execute Code | code | Runs a piece of GML code, written in the [code editor](scripts.md#the-code-editor). | A |
| Execute Script | script, argument0–argument4 | Runs a script with up to five arguments. | A |
| Comment | comment | Does nothing. Shown in the list in italics, so you can explain what the actions do. | |

### Variables

| Action | Settings | What it does | |
|---|---|---|---|
| Set Variable | variable, value | Sets a variable to a value. With Relative, adds the value to it. Use `global.` in front of the name for a global variable. | A R |
| Test Variable | variable, value, operation | Is the variable equal to, smaller than or larger than the value? | Q A |
| Draw Variable | variable, x, y | Draws the value of a variable. Use it in a Draw event. | A R |

---

## score

The game keeps three global values: `score`, `lives` and `health` (0–100). By default
the score is shown in the window caption.

### Score

| Action | Settings | What it does | |
|---|---|---|---|
| Set Score | new score | Sets the score. With Relative, adds to it. | R |
| Test Score | value, operation | Is the score equal to, smaller than or larger than a value? | Q |
| Draw Score | x, y, caption | Draws the score with a caption in front. Use it in a Draw event. | R |
| Show Highscore | background, border, new color, other color, font | Shows the highscore table. If the current score is high enough, the player first types a name for it. | |
| Clear Highscore | | Empties the highscore table. | |

### Lives

| Action | Settings | What it does | |
|---|---|---|---|
| Set Lives | new lives | Sets the number of lives. When it drops to 0 or below, every instance gets the No More Lives event. | R |
| Test Lives | value, operation | Compares the number of lives with a value. | Q |
| Draw Lives | x, y, caption | Draws the number of lives with a caption. | R |
| Draw Life Images | x, y, image | Draws one copy of a sprite for each life. | R |

### Health

| Action | Settings | What it does | |
|---|---|---|---|
| Set Health | value (0-100) | Sets the health. When it drops to 0 or below, every instance gets the No More Health event. | R |
| Test Health | value, operation | Compares the health with a value. | Q |
| Draw Health | x1, y1, x2, y2, back color, bar color | Draws a health bar in a rectangle. | R |
| Score Caption | show score, score caption, show lives, lives caption, show health, health caption | Chooses which of score, lives and health appear in the window caption, and the text before each. | |

---

## extra

### Particles

These actions create particle effects such as smoke, sparks and rain. First create the
particle system, then define particle types, then create emitters that burst or
stream particles of a type. Up to 16 particle types and 8 emitters are available.
See [Particles](../reference/particles.md) for the functions behind them, which offer
more control.

| Action | Settings | What it does |
|---|---|---|
| Create Part System | depth | Creates the particle system, drawn at the given depth. |
| Destroy Part System | | Destroys the particle system. |
| Clear Part System | | Removes all particles. |
| Create Particle | type id, shape, sprite, min size, max size, size increment | Defines a particle type with a built-in shape (pixel, disk, square, line, star, circle, ring, sphere, flare, spark, explosion, cloud, smoke, snow) or a sprite, and its size. |
| Particle Color | type id, color mode, color1, color2, start alpha, end alpha | Sets a type's colours: *mixed* picks a colour between the two, *changing* changes from one to the other over its life. Also fades the opacity. |
| Particle Life | type id, min life, max life | Sets how many steps particles of a type live. |
| Particle Speed | type id, min speed, max speed, min dir, max dir, friction | Sets the speed and direction range of a type. |
| Particle Gravity | type id, amount, direction | Gives a type gravity. |
| Particle Secondary | type id, step type, step count, death type, death count | Makes particles of a type create other particles every step and when they die. |
| Create Emitter | emitter id, shape, xmin, xmax, ymin, ymax | Creates an emitter: an area (rectangle, ellipse, diamond or line) that particles come from. |
| Destroy Emitter | emitter id | Destroys an emitter. |
| Burst from Emitter | emitter id, particle type, number | Creates a number of particles at once. |
| Stream from Emitter | emitter id, particle type, number | Creates particles every step. A negative number means a chance: -5 creates one particle on average every five steps. |

### CD

**Play CD**, **Stop CD**, **Pause CD**, **Resume CD**, **Check CD** and **Check CD
Playing** controlled audio CDs. Instance doesn't support CD audio, so these actions
do nothing.

### Other

| Action | Settings | What it does |
|---|---|---|
| Set Cursor | sprite, cursor | Replaces the mouse cursor with a sprite, and shows or hides the normal cursor. |

---

## draw

Drawing actions only work in a **Draw** event. Remember that a Draw event replaces the
instance's normal drawing; add **Draw Sprite** with `sprite_index` and subimage -1 if
you still want to see it. With Relative, positions are relative to the instance.

### Drawing

| Action | Settings | What it does | |
|---|---|---|---|
| Draw Sprite | sprite, x, y, subimage | Draws a frame of a sprite (-1 = the instance's current frame). | A R |
| Draw Background | background, x, y, tiled | Draws a background image, optionally tiled over the whole room. | R |
| Draw Text | text, x, y | Draws text in the current font and colour. Use `#` for a new line. | A R |
| Draw Scaled Text | text, x, y, xscale, yscale, angle | Draws text scaled and rotated. | A R |
| Draw Rectangle | x1, y1, x2, y2, filled | Draws a filled or outlined rectangle in the current colour. | A R |
| Horizontal Gradient | x1, y1, x2, y2, color1, color2 | Draws a rectangle shading from left to right. | A R |
| Vertical Gradient | x1, y1, x2, y2, color1, color2 | Draws a rectangle shading from top to bottom. | A R |
| Draw Ellipse | x1, y1, x2, y2, filled | Draws a filled or outlined ellipse. | A R |
| Gradient Ellipse | x1, y1, x2, y2, color1, color2 | Draws an ellipse shading from the centre to the edge. | A R |
| Draw Line | x1, y1, x2, y2 | Draws a line. | A R |
| Draw Arrow | x1, y1, x2, y2, tip size | Draws an arrow. | A R |

### Settings

| Action | Settings | What it does |
|---|---|---|
| Set Color | color | Sets the colour for the drawing actions that follow. |
| Set Font | font, align | Sets the font and horizontal alignment for text. |
| Set Full Screen | action | Switches between window and full screen, or picks one. |

### Other

| Action | Settings | What it does | |
|---|---|---|---|
| Take Snapshot | filename | Saves a screenshot as an image file. | |
| Create Effect | type, x, y, size, color, where | Shows a ready-made effect (explosion, ring, ellipse, firework, smoke, smoke up, star, spark, flare, cloud, rain or snow) in a size and colour, below or above the instances. Unlike the other draw actions, it works in any event. | A R |

---

## Actions in older projects

Some actions are no longer in the palette but still work when a project uses them:
an older **Change Sprite** with a scale factor, an older **Set Time Line**, **Show
Video** (not supported), an older **Create Particle**, and **Open Webpage**. They
appear in action lists like any other action.

Actions supplied by [extension packages](extensions.md) are not shown in the palette;
use the package's functions from code instead.

See also: [Objects](objects.md) · [Events](events.md) · [GML overview](../gml/overview.md) · [Function reference](../reference/README.md)
