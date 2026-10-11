# Creating executables

To share your game with people who don't have Instance, turn it into a stand-alone
program with **File > Create Executable…** or the toolbar button.

1. Instance applies all open forms, so the executable contains your latest changes.
2. It asks where to save the program. On Windows it's an `.exe` file named after your
   project.
3. It builds the program and reports *Created …* in the status bar.

The game needs at least one room.

## What's inside

An executable is a copy of the Instance runner with your whole game attached to it.
Everything the game needs travels with it:

- all resources: sprites, sounds, backgrounds, fonts, scripts, objects, rooms;
- the [included files](constants-triggers-files.md#included-files) that are stored in
  the project;
- the [extension packages](extensions.md) the project uses. If a used package isn't
  installed on your computer, Instance warns you, and the executable is built without
  it.

The executable is a single file. Players just run it; there's nothing to install.

> **Tip:** Fonts are not included: the game uses the fonts installed on the player's
> computer, and falls back to a built-in font if one is missing. See [Fonts](fonts.md).

## Platforms

The executable runs on the same kind of system it was built on: an executable made on
Windows is a Windows program. To make a version for another system, create the
executable with Instance on that system. 32-bit DLLs used by a game additionally need
the `instance-dllhost32.exe` helper next to the executable; see
[Extensions and DLLs](../runtime/extensions-and-dlls.md).

## The game icon and version

The **game icon** from the [Loading tab](game-settings.md#loading) of the game
settings is used for the game window. The executable file itself shows the runner's
icon in the file manager.

## If it doesn't work

- *Runner executable … not found*: Instance needs the runner program to build the
  executable. Keep `instance-runner` next to the IDE, or set its path in
  **File > Preferences…**.
- *already contains a game*: the runner path in the preferences points at an
  executable you made earlier. Point it at the plain runner instead.
- Creating executables is not available in the [browser version](web.md). Save the
  project and use the desktop version.

Executables made with other tools can't be opened in Instance, and Instance's
executables can't be opened as projects. Always keep your `.gm81` project file.

See also: [Running and debugging games](running-and-debugging.md) · [Project files](project-files.md) · [Global game settings](game-settings.md)
