# The main window

The main window holds the menu bar, the toolbar, the resource tree on the left, the
workspace where editors open, and the status bar. This page describes each of them.
For a quick introduction, see [A tour of the IDE](../intro/tour.md).

## The File menu

| Command | What it does |
|---|---|
| New | Closes the current project and starts an empty one. Shortcut Ctrl+N. |
| Open… | Opens a `.gm81` or `.gmk` project file. Shortcut Ctrl+O. |
| Recent Files | Lists the last eight projects you opened or saved. |
| Save | Saves the project to its file. A project without a file, or one that was opened from a `.gmk` file, asks for a name first. Shortcut Ctrl+S. |
| Save As… | Saves the project under a new name. Projects are always saved in the `.gm81` format. |
| Create Executable… | Builds a stand-alone program from the game. See [Creating executables](executables.md). |
| Preferences… | Opens the preferences (see below). |
| Exit | Closes Instance. |

New, Open and Exit ask whether to save first when the project has unsaved changes,
including changes in forms that are still open. Choosing **Yes** applies all open
forms and saves.

## The Edit menu

The Edit menu works on the item selected in the resource tree.

| Command | What it does |
|---|---|
| Insert resource | Creates a new resource of the selected kind just above the selected one, and opens it. |
| Duplicate | Makes a copy of the selected resource, named `<name>_copy`, and opens it. |
| Delete | Deletes the selected resource or group, after asking. Deleting a group deletes everything in it. This can't be undone. |
| Rename | Lets you type a new name in the tree. |
| Properties | Opens the editor of the selected item. |
| Add Group | Asks for a name and creates a group (a folder) in the selected group or resource folder. |
| Expand Resource Tree | Opens every folder in the tree. |
| Collapse Resource Tree | Closes every folder in the tree. |

## The Resources menu

The first nine commands create a new resource and open its editor: **Create Sprite**,
**Create Sound**, **Create Background**, **Create Path**, **Create Script**,
**Create Font**, **Create Time Line**, **Create Object** and **Create Room**. The new
resource goes at the end of its folder, or into the selected group if a group of the
same kind is selected in the tree.

New resources get a default name such as `sprite0` or `object3`. Rename them to
something meaningful.

The remaining commands open the editors for the game as a whole:

| Command | Editor |
|---|---|
| Change Game Information | [Game information](game-information.md) |
| Change Global Game Settings | [Global game settings](game-settings.md) |
| Select Extension Packages | [Extension packages](extensions.md) |
| Define Constants | [Constants](constants-triggers-files.md#constants) |
| Define Triggers | [Triggers](constants-triggers-files.md#triggers) |
| Included Files | [Included files](constants-triggers-files.md#included-files) |

## The Scripts menu

**Create Script** adds a new script. Below it, the menu lists all scripts in the
project; click one to open it.

## The Run menu

| Command | What it does |
|---|---|
| Run normally (F5) | Plays the game. |
| Run in Debug mode (F6) | Plays the game with the Debug Information window. |

Both are disabled while a game is running. See
[Running and debugging games](running-and-debugging.md).

## The Window menu

| Command | What it does |
|---|---|
| Output Log | Shows or hides the output of the last game run. |
| Cascade | Stacks all forms diagonally at their default sizes. |
| Arrange Tiled | Divides the workspace between all forms. |
| Minimise All | Minimises every form to the bottom of the workspace. |
| Close All | Closes every form, asking about each one with unsaved changes. |

The open forms are listed at the bottom of the menu; click one to bring it to the
front.

## The Help menu

**About Instance…** shows the About box.

## The toolbar

The toolbar has buttons for the most used commands. Hover over a button to see its
name.

| Group | Buttons |
|---|---|
| Project | New, Open, Save |
| Distribution | Create executable; Publish (not available) |
| Running | Run, Run in debug mode, Output log |
| Resources | Create a sprite, sound, background, path, script, font, time line, object or room |
| Game | Game information, Global game settings, Extension packages |
| Help | About |

The Output log button shows a small red badge when the last game run reported errors
you haven't looked at yet.

## The resource tree

The tree lists the resources of your game in nine folders, followed by the Game
Information, Global Game Settings and Extension Packages items.

- **Select** an item by clicking it.
- **Open** a resource with a double-click, or with **Properties** in its right-click
  menu. If its form is already open, it's brought to the front.
- **Rename** a resource or group: select it and press **F2**, or choose **Rename**
  from the right-click menu. Press Enter to accept or Escape to cancel. Renaming in
  the tree also renames it in any open form.
- **Reorder** by dragging. Drop an item above or below another item, or onto a group
  to move it inside. Items can only move within their own folder: a sound can't go
  among the sprites.
- **Group** related resources: right-click a folder or group and choose **Add
  Group**. Groups can be nested. They are only for your own organisation; they don't
  affect the game, with one exception: the order of rooms.

> **Room order:** The game starts in the first room in the tree, and the room actions
> (Next Room, Previous Room) and functions such as `room_next` follow the tree order.
> Drag rooms to change it.

The right-click menu depends on what you click:

| Item | Menu commands |
|---|---|
| A resource | Properties, Insert *kind*, Duplicate *kind*, Rename, Delete |
| A resource folder | Create *kind*, Add Group |
| A group | Create *kind*, Add Group, Rename, Delete group |
| Game Information, Global Game Settings, Extension Packages | Properties |

Sprites and backgrounds show their image in the tree, and objects show the image of
their sprite.

## Dropping files on the window

You can drag files from your file manager onto the main window:

- A `.gm81` or `.gmk` file opens that project (asking to save the current one first).
- An image file (PNG, BMP, GIF or JPEG) becomes a new sprite, named after the file,
  and its form opens. A file name ending in `_stripN` (for example
  `coin_strip8.png`) is cut into N frames.

## The status bar

The status bar shows the last thing that happened (for example *Saved …* or *Game
finished*), the path of the project file (`<new game>` for an unsaved project) and the
number of open forms. When a game run reports errors, a red **new errors – view log**
button appears at the left; click it to open the Output Log.

## Preferences

**File > Preferences…** holds settings that belong to you rather than to a project.
They are stored in your settings folder (see
[Installing](../intro/installing.md#where-instance-keeps-its-settings)).

| Setting | What it does |
|---|---|
| Runner executable | Path of the runner program used to play games and build executables. Leave empty to find it automatically next to the IDE. |
| Code font size | Text size in the code editor (8–28). |
| Show line numbers | Shows line numbers in the code editor. |
| Use color coding | Colours keywords, strings, functions, resource names and so on in the code editor. |
| Show the origin in sprite images | Draws a red cross at the origin in the sprite form's preview. |

See also: [A tour of the IDE](../intro/tour.md) · [Keyboard shortcuts](shortcuts.md) · [Project files](project-files.md)
