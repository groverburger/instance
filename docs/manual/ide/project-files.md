# Project files and project folders

## Project files

An Instance project is normally a single file with the extension **`.gm81`**. It holds
everything: all resources with their images and sounds, the settings, the game
information, constants, triggers and stored included files. Copy that one file and
you've copied the whole game.

| Format | Open | Save |
|---|---|---|
| `.gm81` | Yes | Yes. This is the format Instance saves in. |
| `.gmk` | Yes | No. Saving a project opened from a `.gmk` file asks for a new `.gm81` name, and the original file is left untouched. |
| project folder | Yes: **File > Open Folder…** | Yes: back into the same folder, or **File > Save As Folder…** |

Instance opens `.gmk` and `.gm81` projects made with older 2D game tools of the same
family, and keeps every part of them, including data it doesn't edit, when it saves
them again.

### Saving

**File > Save** (Ctrl+S) writes the project file. Instance first writes to a
temporary file next to it and then replaces the old file, so a crash or a full disk
during saving can't leave you with a half-written project.

Instance doesn't keep backup copies of older versions. Use version control, or make a
copy of the file from time to time, for important projects.

> **Tip:** Save often. Pressing OK in a form only changes the project in memory; the
> file on disk changes when you save. The main window's title shows a `*` while there
> are unsaved changes, and Instance asks before closing a project with unsaved changes.

## Project folders

A **project folder** stores a project as many small, ordinary files instead of one
file: code in `.gml` files, images as PNG files, sounds as the original sound files,
and settings in small text files (TOML). This is useful when:

- you keep your game in **version control** (such as git), where small text files give
  readable differences and easy merges;
- you want to edit code or images with **other tools**;
- a program or script generates or changes parts of a game.

Converting between a project file and a folder loses nothing, in either direction.

### Working with folders

Open a project folder with **File > Open Folder…**, by dropping the folder onto the
main window, or by giving it on the command line:

```sh
instance-ide mygame/
```

**File > Save** then writes back into the same folder. Only the files that belong to
the project are rewritten: other files in the folder, such as a `.git` folder or your
own notes and source art, are left alone.

To turn the project you have open into a folder, use **File > Save As Folder…** and
pick an empty folder (you can make a new one in the dialog). If you pick a folder that
already holds other files, Instance makes a new folder named after the project inside
it. From then on, Save writes to the folder. **File > Save As…** goes the other way and
saves a `.gm81` file again.

Project folders aren't available in the [browser version](web.md), which has no access
to folders on your computer.

The runner can also play a folder directly:

```sh
instance-runner mygame/
```

To convert without the IDE, use the command-line tools:

```sh
gmk-unpack mygame.gm81 mygame/     # project file to folder
gmk-pack mygame/ mygame.gm81       # folder to project file
```

### What's in a folder

| Path | Contents |
|---|---|
| `project.toml` | Game settings, constants, room order and other project-wide data |
| `game_information.rtf` | The game information text |
| `resource_tree.txt` | The layout of the resource tree, groups included |
| `sprites/<name>/` | `sprite.toml` and one PNG per frame |
| `backgrounds/<name>.png`, `.toml` | The image and its tile settings |
| `sounds/<name>.<ext>`, `.toml` | The sound file and its settings |
| `paths/<name>.toml` | The points and settings |
| `scripts/<name>.gml`, `.toml` | The code and its metadata |
| `fonts/<name>.toml` | The font settings |
| `timelines/<name>/` | The moments, with their code or actions |
| `objects/<name>/` | `object.toml` and one file per event |
| `rooms/<name>/` | `room.toml`, the room creation code and instance creation code |
| `triggers/`, `included_files/` | Triggers and included files |

An event or moment with a single **Execute Code** action is stored as a plain `.gml`
file, so most code can be edited directly.

## Command-line tools

Instance comes with a few small programs for working with projects outside the IDE.

| Tool | What it does |
|---|---|
| `gmk-info game.gm81 [--tree]` | Prints a summary of the resources in a project (with `--tree`, the resource tree). |
| `gmk-unpack game.gm81 dir/` | Converts a project file into a project folder. |
| `gmk-pack dir/ game.gm81` | Converts a project folder into a project file. |
| `gmk-check game.gm81` | Checks a project (file or folder) for problems without running it: GML errors with line and column, scripts called with the wrong number of arguments (when the game's argument check is on), references to resources that don't exist, clashing names, variables that are read but never set. `--pedantic` reports more possible problems; `--strict` makes warnings count as failures. |
| `gmx2gm81 Game.gmx -o game.gm81` | Converts a project from the later XML-based `.gmx` / `.gmz` format into a `.gm81` file. |

`gmk-info` and `gmx2gm81` are included in the Windows download; the others are built
from the source code.

See also: [The main window](main-window.md) · [Creating executables](executables.md) · [Instance in the browser](web.md)
