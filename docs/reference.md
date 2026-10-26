# Reference

The details that don't fit in the README: command-line options, environment
variables, platform notes and where things are stored.

## Command line

```sh
instance-ide [project.gm81]                    # the editor; F5 runs the game, F6 runs it in debug mode
instance-runner game.gm81                      # play a .gmk (v701/v800) or .gm81 project
instance-runner game.gm81 --debug              # with the Debug Information window
instance-runner game/                          # run a project folder directly (the IDE opens them too)
instance-runner game.gm81 --renderer software  # force the CPU renderer (default: OpenGL, with fallback)

gmk-info game.gm81 [--tree]                    # resource summary
gmk-unpack game.gm81 game/                     # project file -> project folder (gmk-pack goes back)
gmk-check game/ [--pedantic]                   # static checks: GML errors, argument counts, broken references
gmx2gm81 Game.gmx -o game.gm81                 # convert a .gmx project (folder, .project.gmx or .gmz)
```

Project folders store a project as plain files (`.gml`, PNG, TOML) and convert
losslessly in both directions; see [project-folder.md](project-folder.md).

### Headless runs and the test harness

The runner can play a game without a window, which is how the test suites and the
black-box comparisons drive it. `instance-runner --help` lists every option.

```sh
instance-runner game.gm81 --headless --frames 300 --screenshot out.png
instance-runner game.gm81 --headless --frames 600 --random-input 1 --max-seconds 60
instance-runner game.gm81 --headless --frames 600 --seed 1 --save-dir s --no-read-fallback \
    --input 30:press:enter --exec "120:global.cash = 1e5" --watch 60:global.cash \
    --screenshot-at 200 shots --no-draw --audio-out a.wav
instance-runner game.gm81 --compare-renderers DIR --frames 300 --screenshot-every 50 DIR
INSTANCE_PROFILE=1 instance-runner game.gm81 --headless   # per-phase, per-event and per-function timings
```

Always pass `--save-dir` (and `--no-read-fallback`) to headless runs, so they don't
read or write save files in the game's folder.

### Console output on Windows

The runner is a GUI program. A standalone game made with Create Executable never
opens a console. Running a project file prints `show_debug_message` output, errors and
runner messages to the calling console (or a new console window); `--no-console`
turns that off. The IDE uses `--no-console` and shows the output in its Output Log
(Window > Output Log). Runtime errors appear in an error dialog (Ignore /
Abort), following the game's error settings.

Shells don't wait for GUI programs, so redirect the output (`2>&1 | ...`) or use
`Start-Process -Wait` in scripts.

## Tests

```sh
cargo test --workspace
cargo test --release -p instance-runner --test corpus -- --ignored --nocapture
```

Some suites need folders of external test projects: `GM81_SAMPLES`,
`GM81_CORPUS*` and `GM81_BLACKBOX_DIR`. They are skipped when the folders are missing.
The black-box projects (`crates/instance-blackbox`, manifest in
[specs/blackbox-tests.md](../specs/blackbox-tests.md)) are run in the reference engine
and in Instance, and the results are compared key by key.

## Sound

MIDI music goes to the Windows synthesiser. On macOS and Linux, or with
`INSTANCE_MIDI=soundfont`, it is rendered with a SoundFont, looked up in this order:

1. `INSTANCE_SOUNDFONT`
2. `<runner folder>/soundfonts/default.sf2`
3. common system locations

No SoundFont is in the repository. `scripts/fetch-soundfont.sh` (or `.ps1`) downloads
GeneralUser GS (free to redistribute, checksum-verified) to `web/soundfonts/`. Copy
`default.sf2` next to the runner, or point `INSTANCE_SOUNDFONT` at it.

For compatibility, `sound_isplaying` reports about 105 ms late in both directions.
`--sound-status-delay MS` (or `INSTANCE_SOUND_STATUS_DELAY`) changes the delay; `0`
reports the current state.

## Joysticks

Joysticks are read with gilrs. `INSTANCE_JOYSTICK=0` disables them,
`INSTANCE_JOY_DPAD_AS_STICK=1` maps the d-pad to the stick, and `INSTANCE_JOY_RAW=1`
logs raw input. Scripted input (`--input`) also accepts `STEP:joy1:button:N:down|up`,
`STEP:joy1:axis:x|y|z|r|u|v:VALUE`, `STEP:joy1:pov:ANGLE` and
`STEP:joy1:connect|disconnect`.

## Multiplayer

The `mplay_*` functions work over TCP/IP with Instance's own protocol
([specs/multiplayer.md](../specs/multiplayer.md), part B). It is not compatible
with DirectPlay sessions. There is a demo with two headless runners:

```sh
cargo build -p instance-runner && cargo run -p instance-runner --example mplay_demo
```

## Extensions and DLLs

Extension packages come from the executable's bundle, or from these folders (the
first match wins):

1. `extensions/` next to the IDE or runner
2. the folders in `INSTANCE_EXTENSIONS`
3. `extensions/` in the per-user data folder (where the IDE installs packages)
4. `extensions/` next to the project

64-bit DLLs load in-process. 32-bit DLLs, which covers most DLLs written for older
games, need the helper:

```sh
rustup target add i686-pc-windows-msvc
cargo build --release -p instance-dllhost32 --target i686-pc-windows-msvc
```

Put `instance-dllhost32.exe` next to `instance-runner.exe`, or set `INSTANCE_DLLHOST32`.

## Platforms

| | Windows | macOS | Linux | Browser |
|---|---|---|---|---|
| IDE | yes | yes | yes | yes |
| Renderer | OpenGL, software | OpenGL, software | OpenGL (EGL), software | WebGL2, software |
| MIDI | Windows synthesiser or SoundFont | SoundFont | SoundFont | SoundFont |
| DLLs | x64 in-process, x86 via helper | `.dylib` only | `.so` only | no |
| Multiplayer | yes | yes | yes | no |

macOS and Linux are built and tested by CI, but see little interactive use.

- **Linux** builds need the ALSA, udev, X11/Wayland and GL development packages (see
  the `apt-get` line in `.github/workflows/ci.yml`).
- **macOS**: `scripts/macos-bundle.sh` builds `Instance.app` (the IDE, with the runner
  inside) and `Instance Runner.app` in `dist/macos`. The runner app takes the game on
  the command line (`open -a "Instance Runner" --args /path/game.gm81`); Finder's
  "Open With" isn't handled yet. The release builds are ad-hoc signed: right-click >
  Open on first launch, or run `xattr -dr com.apple.quarantine Instance.app`.

## The web build

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # the wasm-bindgen version in Cargo.lock
scripts/build-web.sh                               # or scripts\build-web.ps1; output in dist/web
python -m http.server -d dist/web 8000
```

`dist/web` contains `index.html`, `ide.html` (the IDE), `play.html` (the player) and
`pkg/` (wasm and JS glue). Any static web server works, but the pages must be served
over HTTP, not opened as files. The GitHub Pages site is built by
`.github/workflows/pages.yml` and opens the IDE at its root.

- **IDE**: File > Open uses the browser's file picker, or drop a file. Save downloads
  the `.gm81`. The project is autosaved to IndexedDB and offered again on the next
  visit. `?project=URL` opens a project from the server, and `&open=NAME` then opens
  the editor of the resource NAME (maximised). If the browser holds an autosave, a
  `?project=` link asks first: open the linked project (its autosave replaces the old
  one once it is changed), download the old one and then open the link, or keep
  working on the old one. Run (F5) plays the game in an overlay. Create Executable
  isn't available; File > Export Web Page is (below).
- **Welcome screen**: on a first visit (no `?project=`, no autosave) the IDE shows
  the example projects of `web/examples/` (Edit opens one in the IDE, Play in
  `play.html`), "New project" and "Open file". File > Examples… shows it again. It is
  plain HTML in `web/ide.html` and `web/ide.js`, styled like the IDE's own dialogs. While a
  project downloads and opens, an "Opening NAME" box with a progress bar shows; the IDE
  closes it (`instanceIde.loaded`) once the project is open or a question about it is up.
- **Examples**: `web/examples/*.gm81` are committed, because the Pages workflow can't
  read the private `instance-gamekit` repository; the build scripts copy the folder.
  `scripts/pack-web-examples.sh` repacks Emberwood and Gravity Grade from
  `$INSTANCE_GAMEKIT` (default `../instance-gamekit`), transcoding their large WAVs to
  OGG Vorbis (ffmpeg, oggenc or Python with `soundfile`) so that the files are 2 and
  5 MB instead of 12 and 40 MB. `scripts/build-spelunky-mod.py` makes `spelunky.gm81`
  (Spelunky Classic, under its own license, `spelunky-COPYING.txt`); Emberwood and
  Gravity Grade are under the MIT License (`LICENSE-MIT.txt`). The thumbnails
  (`*.png`, `emberwood.jpg`) are headless-runner screenshots. The desktop IDE shows the
  same welcome dialog (`crates/instance-ide/src/welcome.rs`) on its first start and with
  File > Examples…; only the screenshots are built in. Edit and Play download the game
  from the GitHub Pages site's `examples/`, keeping a copy in the user's data folder
  (`examples/`) that is used when the site can't be reached.
- **Player**: `play.html?game=URL` loads a game, or drop a file onto the page. A page
  can also embed a game with `<script id="instance-game" data-src=URL>` or
  `window.INSTANCE_GAME_BASE64`. A start screen shows the game's name and a progress
  bar while the runner and the game download; the SoundFont follows only for a game
  with MIDI sounds (`game_uses_midi`). The game starts after a click on Play, because
  browsers only allow audio after user input (`?autostart` skips the button). Exported
  pages (`crates/instance-format/src/web_page.html`) share the start screen's look.
  The page is the game's window: the game is scaled to fit it by whole factors (keeping
  its aspect ratio), and fills it, stretched, with the "Full scale" setting (or as
  placed by the scaling setting in full screen). `?renderer=software` forces the CPU
  renderer, and `?soundfont=URL` picks a different SoundFont.
- **Files** a game writes (`file_*`, `ini_*`, saves, highscores, registry) live in a
  virtual file system in `localStorage`, per game.
- **Limits**: no DLLs, no multiplayer and no room transitions. `sleep` delays the next
  step instead of blocking, and `keyboard_wait` / `mouse_wait` return at once. Modal
  boxes inside the game (highscore name entry, splash images) are skipped, and
  `show_message`, `show_question` and `get_string` use the browser's dialogs. Scripts
  can nest at most 512 calls deep.
- **Moving the mouse**: a page cannot move the cursor. When a game moves the mouse
  (`display_mouse_set`, `window_mouse_set`, `window_view(s)_mouse_set`) while the cursor
  is hidden, as mouse look in a 3D game does, the game takes the pointer lock: the
  cursor stays put, and the game's mouse moves with the mouse and goes where the game
  sets it. Esc (in the browser) ends the lock; a click on the game takes it again, and
  showing the cursor ends it. With the cursor shown, the game sees the new position
  until the mouse moves.

## Where things are stored

Settings, recent files, installed extensions and per-game data (the emulated
`registry_*` keys and highscores) live in `%APPDATA%\Instance`,
`~/Library/Application Support/Instance` or `~/.config/Instance`.

Earlier builds used the folders `gm81-clean` and `gm81-runner`. Their contents are
copied over on first start; the old folders are left alone.

## Fonts

Game fonts are looked up in the system font folders. A missing font falls back to the
bundled Liberation Sans (`crates/instance-runner/fonts`, SIL Open Font License). The
IDE uses Segoe UI or Tahoma when the system has them, and otherwise the bundled Roboto
(`crates/instance-ui/fonts`, Apache License 2.0). `INSTANCE_UI_FONT=roboto` forces
Roboto.

## Environment variables

| Variable | Purpose |
|---|---|
| `INSTANCE_EXTENSIONS` | extra extension package folders |
| `INSTANCE_DLLHOST32`, `INSTANCE_DEBUG_DLL` | 32-bit DLL helper path; DLL call logging |
| `INSTANCE_MIDI`, `INSTANCE_SOUNDFONT`, `INSTANCE_AUDIO_DEBUG` | MIDI backend, SoundFont file, audio logging |
| `INSTANCE_SOUND_STATUS_DELAY` | `sound_isplaying` delay in ms (default 105) |
| `INSTANCE_JOYSTICK`, `INSTANCE_JOY_DPAD_AS_STICK`, `INSTANCE_JOY_RAW` | joystick input |
| `INSTANCE_MPLAY_PORT` | multiplayer port |
| `INSTANCE_PROFILE` | timing profile of a run |
| `INSTANCE_UI_FONT` | `roboto` forces the bundled UI font |

The older `GM81_*` names (and `GM_PROFILE`) still work as fallbacks.

## Web pages

File > Export Web Page (and `gmk-web GAME OUT.html`) writes a game as one
self-contained `.html` file (`crates/instance-format/src/web.rs`, template
`web_page.html`): the web runner's JS glue inline, and the runner's wasm, the `.gm81`
and, for games with MIDI sounds, a SoundFont, each gzipped and base64-encoded. The page
runs from `file://` too. The native IDE and `gmk-web` look for the web runner in
`web/pkg`, `pkg` or `dist/web/pkg` next to the executable or up to three folders above
it (a checkout's `dist/web/pkg` after `scripts/build-web.sh`), or in
`$INSTANCE_WEB_RUNNER`. The browser IDE fetches `pkg/` from its own server, and the
SoundFont named by `config.js`.

## Executables

Create Executable appends the game to a copy of the runner, followed by a footer
([specs/exe-format.md](../specs/exe-format.md)). Executables made by earlier builds
still run. Executables built by other tools can't be loaded.

## Releases

Publishing a GitHub release runs `.github/workflows/release.yml`, which attaches:

- `Instance-<tag>-windows-x64.zip`: the IDE, the runner, `gmx2gm81`, `gmk-info`,
  `gmk-web`, the 32-bit DLL helper, the web runner (`web/pkg`, for Export Web Page) and
  the MIDI SoundFont
- `Instance-<tag>-macos-arm64.zip` and `-macos-x64.zip`, plus the matching
  `Instance-Runner-…` zips: ad-hoc signed `.app` bundles
- `Instance-<tag>-web.zip`: the static browser build

It can also be run by hand for an existing tag (Actions > Release > Run workflow).
