# Exporting a web page

To let people play your game in a web browser, export it with **File > Export Web
Page…** or the toolbar button next to *Create executable*. The result is **one `.html`
file** that holds everything: the Instance runner, your game and, if the game plays
MIDI music, a SoundFont to play it with.

1. Instance applies all open forms, so the page contains your latest changes.
2. It asks where to save the page, named after your project. In the
   [browser version](web.md) the page is downloaded instead.
3. It reports *Exported …* in the status bar.

The game needs at least one room.

## Sharing the page

The page works on its own. Players can:

- open it straight from disk by double-clicking it;
- get it as an e-mail attachment or a download;
- play it from any web server or file host that serves `.html` files, such as
  GitHub Pages or itch.io (upload it as an HTML game, with the file as `index.html`).

The game starts when the player clicks **Play**, because browsers only allow
sound after the player has clicked or pressed a key. It needs a current browser
(Chrome, Edge, Firefox or Safari from 2023 on).

## Size

The runner adds about 2.5 MB to every page, and your game's resources are compressed
on top of that. A game with MIDI sounds also carries the SoundFont, which is much
larger (the one shipped with Instance adds about 15 MB). If size matters, render the
music to `.ogg` or `.wav` sounds instead of using MIDI.

## In the browser

Games in a web page have the same limits as games in the
[browser version](web.md#running-games) of Instance: files the game writes,
such as save games and highscores, are kept in the browser's storage, and DLLs,
extension packages and multiplayer don't work.

## If it doesn't work

- *The web runner … was not found*: Instance keeps the web runner in the `web/pkg`
  folder next to the IDE. If you built Instance yourself, run
  `scripts/build-web.sh` (or `scripts\build-web.ps1`) once, or set the
  `INSTANCE_WEB_RUNNER` environment variable to the folder holding
  `instance_runner.js` and `instance_runner_bg.wasm`.
- *No SoundFont was found*: the page is made without one, and its MIDI music is
  silent. Keep `soundfonts/default.sf2` next to the IDE, or set `INSTANCE_SOUNDFONT`
  to a `.sf2` file.

From the command line, `gmk-web GAME OUT.html` makes the same page from a `.gm81`
file or a project folder.

See also: [Creating executables](executables.md) · [Instance in the browser](web.md) · [Project files](project-files.md)
