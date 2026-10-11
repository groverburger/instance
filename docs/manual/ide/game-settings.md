# Global game settings

**Global Game Settings** control how the game as a whole looks and behaves: the
window, the special keys, version information, the loading screen and error handling.
Open them by double-clicking **Global Game Settings** in the resource tree, with
**Resources > Change Global Game Settings**, or with the toolbar button.

The settings are divided over six tabs. Press **OK** to apply your changes, or
**Cancel** to discard them.

Some settings are stored with the project, so they survive when the project is opened
in other tools, but the Instance runner doesn't act on them (yet). They are marked
*not used by Instance* below.

## Graphics

| Setting | What it does |
|---|---|
| Start in fullscreen mode | The game starts in full screen instead of in a window. Players can switch with F4 if that key is enabled. |
| Scaling: Fixed scale (%) | Draws the game at this percentage of its normal size. 100 means no scaling. |
| Scaling: Keep aspect ratio | Scales the game as large as fits, keeping its proportions. Bars fill the rest. This is the default. |
| Scaling: Full scale | Stretches the game to fill the screen or window completely. |
| Interpolate colors between pixels | Smooths scaled graphics. Leave it off for crisp pixel art. |
| Color outside the room region | The colour of the area around the game when it doesn't fill the screen or window. |
| Allow the player to resize the game window | The game window can be resized by dragging its edges. |
| Let the game window always stay on top | Keeps the game window above other windows. |
| Don't draw a border in windowed mode | A window without border or title bar. |
| Don't show the buttons in the window caption | Hides the minimise, maximise and close buttons. |
| Display the cursor | Shows the mouse pointer over the game. Untick it if the game draws its own cursor. |
| Freeze the game when the form loses focus | Pauses the game while another window is active. *Not used by Instance.* |
| Disable screensavers and power saving actions | Keeps the screen awake while the game runs. *Not used by Instance.* |

## Resolution

| Setting | What it does |
|---|---|
| Set the resolution of the screen | Changes the screen's colour depth, resolution and refresh rate while the game runs. *Not used by Instance.* |
| Color depth | No change, 16-bit or 32-bit. |
| Resolution (index, 0 = no change) | The screen resolution to use, as a number from a list of standard sizes. |
| Frequency (index, 0 = no change) | The refresh rate to use, as a number from a list. |
| Use synchronization to avoid tearing | Waits for the screen's refresh before showing each frame (vertical sync), for smoother movement. This one works independently of *Set the resolution*. |

## Other

**Default keys.** These keys are handled by the game itself, unless you switch them
off:

| Setting | What it does |
|---|---|
| Let \<Esc\> end the game | Pressing Escape ends the game. Switch it off if your game uses Escape, for example for a pause menu. |
| Treat the close button as \<Esc\> key | Clicking the window's close button acts like pressing Escape. When off, instances get the Close Button event instead and the game decides what to do. |
| Let \<F1\> show the game information | F1 shows the [game information](game-information.md). |
| Let \<F4\> switch between screen modes | F4 switches between window and full screen. |
| Let \<F5\> save the game and \<F6\> load a game | F5 saves the game to a file, F6 loads it again. |
| Let \<F9\> take a screenshot of the game | F9 saves a screenshot in the game's folder. |

**Game process priority:** Normal, High or Highest. *Not used by Instance.*

**Version information:** **Major**, **Minor**, **Release** and **Build** numbers, and
**Company**, **Product**, **Copyright** and **Description** texts (up to 64
characters each). These describe your game; they are stored with the project.

## Loading

| Setting | What it does |
|---|---|
| Show your own image while loading | Shows an image of your choice while the game loads. **Change image…** picks it; **Clear** removes it. |
| Make image partially transparent | Makes the colour of the image's bottom-left pixel transparent. |
| Translucency alpha | The opacity of the loading image (0–255). |
| Loading progress bar | No loading progress bar, the default bar, or your own bar. For your own bar, choose a **Back image…** and a **Front image…**; tick *Scale progress bar image* to stretch the front image (otherwise it is cut off). |
| Game icon | The icon of the game window. **Change icon…** loads an `.ico` file. |
| Game identifier | A number that identifies your game. Instance uses it to keep the game's highscores and registry values apart from those of other games. **Change** picks a new random number. |

The loading image and progress bar are *not used by Instance*, which loads games
quickly without a loading screen.

## Errors

| Setting | What it does |
|---|---|
| Display error messages | Shows an error window when an error happens in the game. When off, errors are ignored silently (but still recorded in `error_last`). |
| Write error messages to file game_errors.log | Also appends every error message to `game_errors.log` in the game's folder. |
| Abort on all error messages | Every error ends the game, instead of offering to continue. |
| Treat uninitialized variables as value 0 | Reading a variable that was never set gives 0 instead of an error. Handy for old projects, but it hides typing mistakes in variable names. |
| Throw an error when arguments aren't initialised correctly. | Scripts must be called with exactly as many arguments as they use; otherwise the call is an error. When off, missing arguments read as 0. |

See [Errors and debugging](../runtime/errors.md) for what error messages look like
and how to read them.

## Info

| Setting | What it does |
|---|---|
| Author | Your name. |
| Version | Your own version text for the game. |
| Information | Any notes about the game. |

The Info tab is for you and your team. The game itself can't read it.

See also: [Game information](game-information.md) · [Running and debugging games](running-and-debugging.md) · [Rooms and views](../runtime/rooms-and-views.md)
