# Saving, files and settings

Games often need to remember things between sessions: a saved game, the player's
options, a high-score table, unlocked levels. Instance offers several tools, from a
one-line save of the whole game to reading and writing your own files.

## Where files go

Relative file names are resolved against
[`working_directory`](../reference/files-and-system.md#working_directory), the folder the
game was started from — normally the folder that contains the game file or
executable. Absolute paths are used as given.

| Variable | Folder |
|---|---|
| `working_directory` | the game's folder (relative names land here) |
| `program_directory` | the folder of the running program |
| `temp_directory` | the system's folder for temporary files |

Not every game folder is writable (for example under *Program Files*), so for files that
must always be writable consider the INI or registry functions below, or a folder you
create yourself.

In the browser version there is no real file system: the files a game writes are kept
in the browser's local storage, separately for each game, and survive a page reload.

## Saving and loading the whole game

[`game_save(fname)`](../reference/game-play.md#game_save) writes the state of the game
to a file and [`game_load(fname)`](../reference/game-play.md#game_load) restores it. If
the game settings allow it, **F5** and **F6** do the same with the file `_save.gmd` in
the working directory.

```gml
// Key Press F2: quick save to slot 1
game_save("slot1.sav");
```

What is saved: the current room, every instance with all its variables, global
variables, `score`, `lives`, `health`, the room speed, and the room's backgrounds and
views. What is **not** saved: playing sounds, data structures, particle systems,
surfaces, resources changed at run time, and the remembered state of persistent rooms
(they start fresh after a load).

A few rules:

- `game_load` takes effect at the end of the current step, like a room change. After a
  load no Create or Room Start events run — the instances simply continue.
- The file format is Instance's own; it can't load saves made by other programs, and a
  save from a different game (or a different game id) is refused with an error.
- Because data structures are not saved, games that keep important state in lists or
  maps should save those separately (see `ds_*_write` in
  [Data structures](../reference/data-structures.md)).

## INI files

INI files are small text files of sections and keys — perfect for settings:

```text
[options]
volume=0.8
fullscreen=0
```

```gml
// load settings
ini_open("settings.ini");
global.volume = ini_read_real("options", "volume", 1);
global.fullscreen = ini_read_real("options", "fullscreen", 0);
ini_close();
```

```gml
// save settings
ini_open("settings.ini");
ini_write_real("options", "volume", global.volume);
ini_write_real("options", "fullscreen", global.fullscreen);
ini_close();
```

One INI file is open at a time. [`ini_open()`](../reference/files-and-system.md#ini_open)
reads it (a missing file is fine — it starts empty), the read functions take a default
to return when the key isn't there, and the changes are written to disk when you call
[`ini_close()`](../reference/files-and-system.md#ini_close) (or open another INI file).
Section and key names are not case-sensitive.

## Text files

```gml
// write
var f;
f = file_text_open_write("log.txt");
file_text_write_string(f, "Level reached:");
file_text_writeln(f);
file_text_write_real(f, global.level);
file_text_close(f);
```

```gml
// read
var f, title, lvl;
f = file_text_open_read("log.txt");
if (f >= 0) {
    title = file_text_read_string(f);
    file_text_readln(f);
    lvl = file_text_read_real(f);
    file_text_close(f);
}
```

- The open functions return a file id (starting at 1). Opening a missing file for
  reading returns -1 without an error, so test the result.
- [`file_text_write_real()`](../reference/files-and-system.md#file_text_write_real) writes a
  space before the number. Whole numbers are written without decimals (as 32-bit
  integers, so very large values wrap around), other numbers with 6 decimals.
- [`file_text_read_string()`](../reference/files-and-system.md#file_text_read_string) reads up
  to the end of the line; [`file_text_readln()`](../reference/files-and-system.md#file_text_readln)
  moves to the next line; [`file_text_eof()`](../reference/files-and-system.md#file_text_eof)
  tells when the file is used up.
- Always close files you open.

## Binary files

[`file_bin_open(fname, mode)`](../reference/files-and-system.md#file_bin_open) opens a file
for byte-by-byte access (mode 0 read, 1 write, 2 both), with
[`file_bin_read_byte()`](../reference/files-and-system.md#file_bin_read_byte),
[`file_bin_write_byte()`](../reference/files-and-system.md#file_bin_write_byte),
[`file_bin_seek()`](../reference/files-and-system.md#file_bin_seek) and
[`file_bin_size()`](../reference/files-and-system.md#file_bin_size).

## The registry

The `registry_*` functions
([`registry_write_real()`](../reference/files-and-system.md#registry_write_real),
[`registry_read_string()`](../reference/files-and-system.md#registry_read_string), …) store
named values per game. Instance doesn't touch the Windows registry: the values go to a
small file per game id in the user's Instance data folder (`%APPDATA%\Instance` on
Windows, `~/Library/Application Support/Instance` on macOS, `~/.config/Instance` on
Linux). They are a convenient place for options and unlock flags that should survive
even if the game folder is read-only.

## Highscores

The built-in high-score table keeps the ten best scores of a game, stored in the same
per-user data folder.
[`highscore_add(name, score)`](../reference/splash-and-messages.md#highscore_add) adds an
entry, [`highscore_show(score)`](../reference/splash-and-messages.md#highscore_show) shows the
table (asking for a name if the score qualifies), and
[`highscore_value(place)`](../reference/splash-and-messages.md#highscore_value) /
[`highscore_name(place)`](../reference/splash-and-messages.md#highscore_name) read it.

## Working with files and folders

[`file_exists()`](../reference/files-and-system.md#file_exists),
[`file_delete()`](../reference/files-and-system.md#file_delete),
[`file_rename()`](../reference/files-and-system.md#file_rename),
[`file_copy()`](../reference/files-and-system.md#file_copy),
[`directory_exists()`](../reference/files-and-system.md#directory_exists) and
[`directory_create()`](../reference/files-and-system.md#directory_create) do what their names
say. [`file_find_first(mask, attr)`](../reference/files-and-system.md#file_find_first) and
[`file_find_next()`](../reference/files-and-system.md#file_find_next) list files matching a
pattern such as `"saves\*.sav"`. The `filename_*` functions split paths into folder,
name and extension.

## Included files

Files added to the game in the IDE (see
[Constants, triggers and included files](../ide/constants-triggers-files.md)) are stored
inside the game and can be exported automatically when the game starts, or by code with
[`export_include_file()`](../reference/files-and-system.md#export_include_file) and
[`export_include_file_location()`](../reference/files-and-system.md#export_include_file_location).

## Parameters, environment and other programs

- [`parameter_count()`](../reference/files-and-system.md#parameter_count) and
  [`parameter_string(n)`](../reference/files-and-system.md#parameter_string) give the
  command-line arguments the game was started with.
- [`environment_get_variable(name)`](../reference/files-and-system.md#environment_get_variable)
  reads an environment variable.
- [`execute_program(prog, arg, wait)`](../reference/files-and-system.md#execute_program) and
  [`execute_shell(prog, arg)`](../reference/files-and-system.md#execute_shell) start other
  programs or open documents.

See also: [Files and system functions](../reference/files-and-system.md) ·
[Data structures](../reference/data-structures.md) · [The game loop](game-loop.md)
