# Instance in the browser

The whole IDE also runs in a web browser, compiled to WebAssembly. Nothing is
installed, and your project stays on your own computer: it isn't uploaded anywhere.
It works the same way as the desktop version, with a few differences around files.

## Opening and saving

- **File > Open…** uses the browser's file picker. You can also drag a `.gm81` or
  `.gmk` file onto the page. Dragging an image file onto the page makes a new sprite,
  as on the desktop.
- **File > Save** and **Save As…** *download* the project as a `.gm81` file, through
  the browser's normal download. Where it ends up depends on your browser's download
  settings.
- There is no **Recent Files** list and no **Exit** command.
- Buttons in the editors that load or save files (Load Sprite, Save Sound, Load from
  file, and so on) use the browser's file picker and downloads in the same way.

A web page can open a project from the server it's hosted on by adding
`?project=URL` to the IDE's address.

### Autosave

While the project has unsaved changes, the browser version saves a copy in the
browser's own storage every 15 seconds (*Autosaved in the browser* appears in the
status bar). If you close the tab without downloading the project, the next visit
offers to bring it back: choose **Restore** to continue where you left off, or
**Discard** to start fresh.

The autosave is a safety net, not a replacement for saving. It lives only in that
browser on that computer, and clearing the browser's data removes it. Download your
project regularly.

## Running games

**Run** (F5) plays the game in a window on top of the IDE. A progress bar shows the
player loading (the first time it downloads the player and the MIDI SoundFont); then
click **Start**. Click **✕** in the window's title bar to stop the game and return to
the IDE. Debug mode isn't available in the browser:
**Run in debug mode** (F6) plays the game normally.

Games running in the browser have some limits:

- Files a game writes (save games, INI files, highscores, registry values) are kept in
  the browser's storage, per game, not on disk.
- DLLs and multiplayer don't work, and there are no room transition effects.
- `sleep` delays the next step instead of freezing, and `keyboard_wait` and
  `mouse_wait` return immediately.
- Message boxes such as `show_message`, `show_question` and `get_string` use the
  browser's own dialogs; some other in-game windows (highscore name entry, splash
  screens) are skipped.
- Browsers only allow sound after the player has clicked or pressed a key.

## Not available in the browser

- **Create Executable**: use the desktop version, or [export a web page](web-pages.md),
  which works here too.
- **Installing extension packages**.
- **Preferences** that refer to programs on disk, such as the runner path, have no
  effect.

## Hosting it yourself

The web version can be put on any ordinary web server: it's a set of static files
(`index.html`, `ide.html` for the IDE, `play.html` for playing games, and a `pkg`
folder). The pages must be served over HTTP; opening them directly from disk doesn't
work. `play.html?game=URL` plays a game without the IDE, which is a simple way to put
your finished game on a website.

See also: [Installing](../intro/installing.md) · [Project files](project-files.md) · [Running and debugging games](running-and-debugging.md)
