# Instance

Instance is a game development environment (IDE and game runner) written in Rust. It
opens, edits and runs GameMaker 8.1 project files (.gmk / .gm81) and behaves like the
GameMaker 8.1 runner; it is a clean-room implementation. Not affiliated with or endorsed
by YoYo Games. See CLEANROOM.md for how this was built.

- `crates/instance-format` – read/write .gmk (v800) / .gm81 (v810) project files
- `crates/instance-gmx` – convert GameMaker: Studio 1.x projects (.gmx folders, .gmz zip/7z) to .gm81 (`gmx2gm81`, specs/gmx-format.md)
- `crates/gml` – GML lexer, parser, compiler and interpreter
- `crates/instance-runner` – the game engine (`instance-runner game.gm81`)
- `crates/instance-ide` – the editor
- `crates/instance-net` – multiplayer (`mplay_*`) over Instance's own TCP/UDP protocol
- `crates/instance-dllcall`, `crates/instance-dllhost32` – DLL bridge (x64 in-process, x86 via a 32-bit helper)
- `crates/instance-check` – `gmk-check`, a static checker for projects (GML errors, script argument counts, broken references, ...)
- `specs/` – behavioural specifications the implementation is built from
- `docs/project-folder.md` – project folders: a project as plain files (`.gml`, PNG, TOML), lossless both ways

Making games with Instance (example games, music and sound tools, the game-making guide)
lives in a separate repository, `instance-gamekit`.

## Usage

```
cargo build --release
target/release/instance-ide [project.gm81]     # editor; F5 runs the game, F6 runs it in debug mode
target/release/instance-runner game.gm81            # play a .gmk (v701/v800) or .gm81 project
target/release/instance-runner game.gm81 --debug    # with the Debug Information window (watches, pause/step,
                                              # execute code, globals/locals/instances, messages)
target/release/instance-runner game.gm81 --headless --frames 300 --screenshot out.png
target/release/instance-runner game.gm81 --renderer software   # force the CPU renderer (default: OpenGL, falls back)
target/release/instance-runner game.gm81 --compare-renderers DIR --frames 300 --screenshot-every 50 DIR
                                              # render with both renderers and diff the frames
target/release/gmk-info game.gm81 [--tree]    # print a resource summary
target/release/gmk-unpack game.gm81 game/     # project file -> project folder (and gmk-pack back)
target/release/instance-runner game/          # run (or instance-ide: edit) a project folder directly
target/release/gmk-check game/ [--pedantic]   # static checks of a project file or folder
target/release/gmx2gm81 Game.gmx -o game.gm81  # convert a Studio project (folder, .project.gmx or .gmz)
cargo test --workspace                        # sample tests read $GM81_SAMPLES
target/release/instance-runner game.gm81 --headless --frames 600 --random-input 1 --max-seconds 60
target/release/instance-runner game.gm81 --headless --frames 600 --seed 1 --save-dir s --input 30:press:enter \
    --exec "120:global.cash = 1e5" --watch 60:global.cash --screenshot-at 200 shots --no-draw --audio-out a.wav
                                              # test harness: see --help
INSTANCE_PROFILE=1 target/release/instance-runner game.gm81 --headless   # per-phase/event/function timings
cargo test --release -p instance-runner --test corpus -- --ignored --nocapture   # runs every game in $GM81_CORPUS
```

On Windows the runner is a GUI program: a standalone game made with Create Executable
never opens a console, while runs of a project file print `show_debug_message` output,
errors and runner info to the calling console (or a new console window). `--no-console`
turns this off; the IDE uses it and pipes the output into its Output Log (Window >
Output Log), so games run from the IDE never show a console. Runtime errors appear in
a GameMaker-style error dialog (Ignore / Abort) as the game's error settings ask. Shells do not wait for
GUI programs, so scripts should redirect the output (`2>&1 | ...`) or use
`Start-Process -Wait`.

Multiplayer: `mplay_*` over TCP/IP only (Instance protocol, specs/multiplayer.md
Part B; not compatible with GameMaker's DirectPlay sessions). Demo with two headless
runners: `cargo build -p instance-runner && cargo run -p instance-runner --example mplay_demo`.

Extensions and DLLs: packages are taken from the executable's bundle or from
`extensions/` next to the runner, `$INSTANCE_EXTENSIONS`, `%APPDATA%/Instance/extensions`
or the project's `extensions/` folder. 32-bit DLLs (all GM 8.1-era DLLs) need the helper:
`rustup target add i686-pc-windows-msvc && cargo build --release -p instance-dllhost32
--target i686-pc-windows-msvc`, then put `instance-dllhost32.exe` next to `instance-runner.exe`
(or set `INSTANCE_DLLHOST32`; cargo target directories are also searched).

Sound: MIDI music plays through the Windows synthesiser (midiOut). Elsewhere (macOS,
Linux), or
with `INSTANCE_MIDI=soundfont`, it is rendered with a SoundFont looked up from
`INSTANCE_SOUNDFONT`, then `<runner dir>/soundfonts/default.sf2`, then common system
locations (none is in the repository: `scripts/fetch-soundfont.sh` / `.ps1` downloads
GeneralUser GS, a freely redistributable General MIDI SoundFont, to `web/soundfonts/`;
copy `default.sf2` from there next to the runner or point `INSTANCE_SOUNDFONT` at it). `INSTANCE_AUDIO_DEBUG=1` logs MIDI activity.
Joysticks are read with gilrs (`INSTANCE_JOYSTICK=0` disables, `INSTANCE_JOY_DPAD_AS_STICK=1`,
`INSTANCE_JOY_RAW=1`); `--input` also accepts `STEP:joy1:button:N:down|up`,
`STEP:joy1:axis:x|y|z|r|u|v:VALUE`, `STEP:joy1:pov:ANGLE`, `STEP:joy1:connect|disconnect`.

Environment variables: `INSTANCE_EXTENSIONS`, `INSTANCE_DLLHOST32`, `INSTANCE_DEBUG_DLL`,
`INSTANCE_MIDI`, `INSTANCE_SOUNDFONT`, `INSTANCE_AUDIO_DEBUG`, `INSTANCE_JOYSTICK`,
`INSTANCE_JOY_DPAD_AS_STICK`, `INSTANCE_JOY_RAW`, `INSTANCE_MPLAY_PORT` and
`INSTANCE_PROFILE`. The names used by earlier builds (`GM81_*` instead of `INSTANCE_*`,
and `GM_PROFILE`) still work as fallbacks; an `INSTANCE_*` variable wins when both are
set. (`GM81_SAMPLES`, `GM81_CORPUS*` and `GM81_BLACKBOX_DIR` name folders of GameMaker 8.1
test material for the test suites and keep their names.)

## Platforms

| | Windows | macOS | Linux | Browser (wasm32) |
|---|---|---|---|---|
| IDE | yes | yes¹ | yes¹ | yes (`ide.html`) |
| Runner | OpenGL / software | OpenGL (CGL) / software² | OpenGL (EGL) / software¹ | WebGL2 / software (`play.html`) |
| MIDI | Windows synthesiser, else SoundFont | SoundFont | SoundFont | SoundFont fetched by the page |
| DLLs (`external_*`) | x64 in-process, x86 via helper | native `.dylib` only | native `.so` only | no (GML extensions work) |
| Multiplayer (`mplay_*`) | yes | yes | yes | no (`mplay_init_tcpip` fails) |

¹ Built and tested by CI (`.github/workflows/ci.yml`) and type-checked from Windows with
`scripts\cross-check.ps1`; not run interactively by the developers.
² The window surface is created on the main thread (AppKit requires it) and the context
is made current on the game thread; tested on Apple Silicon (OpenGL 4.1 Metal).
`--renderer software` forces the CPU renderer.

Per-user data of games (the emulated `registry_*` keys as `registry_<game id>.ini`, and
highscores) lives with the IDE's settings (`ide.json`, recent files) and installed
extensions in `%APPDATA%\Instance`, `~/Library/Application Support/Instance` or
`$XDG_CONFIG_HOME/Instance` (`~/.config/Instance`). Earlier builds used the folders
`gm81-clean` (IDE) and `gm81-runner` (runner) in the same place: on first start their
contents are copied into `Instance` (files already there are kept; the old folders are not
deleted, and a `.migrated-from-<old name>` marker stops the copy from repeating). In the
browser, the per-game `/config/gm81-runner` folder and the IDE's settings and autosave are
carried over the same way. Fonts are looked up in the
system font folders; when a font is missing the bundled Liberation Sans
(`crates/instance-runner/fonts`, SIL Open Font License) is used.
The IDE and the runner's own windows use Segoe UI (or Tahoma) when the system has it,
otherwise the bundled Roboto (`crates/instance-ui/fonts`, Apache License 2.0; e.g. in the
browser, on macOS and Linux); `INSTANCE_UI_FONT=roboto` forces it.

macOS app bundles: `scripts/macos-bundle.sh` (on a Mac) builds `Instance.app` (the IDE, with
the runner inside, so Run works) and `Instance Runner.app` in `dist/macos`, with the
icon generated by `cargo run -p instance-brand --example write_icons -- DIR`. CI uploads them
as artifacts. The runner app takes the game on the command line
(`open -a "Instance Runner" --args /path/game.gm81`); Finder "open with" events are
not handled yet.

Linux builds need the ALSA, udev, X11/Wayland and GL development packages (see the
`apt-get` line in the CI workflow).

## Web (WebAssembly)

**Try it online:** https://groverburger.github.io/instance/ opens the IDE in the browser
(`play.html` there is the player). The site is rebuilt from `master` by
`.github/workflows/pages.yml` on every push.

```
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129     # = the wasm-bindgen version in Cargo.lock
scripts\build-web.ps1          # or: scripts/build-web.sh     → static site in dist/web
python -m http.server -d dist/web 8000                # then open http://localhost:8000/
```

`dist/web` holds `index.html`, `ide.html` (the IDE) and `play.html` (the player) plus
`pkg/` (wasm + JS glue); any static web server works (the pages must be served over
HTTP, not opened as files).

- **Player** (`play.html`): `?game=URL` fetches a `.gm81`/`.gmk` from the server, or drop a
  file onto the page; a page can also embed a game (`<script id="instance-game" data-src=URL>`
  or `window.INSTANCE_GAME_BASE64`). The game starts after a click (browsers only allow audio
  after a user gesture). `?renderer=software` forces the CPU renderer; `?autostart`
  skips the button. The game window is scaled to fit the page (keeping its aspect ratio,
  by whole factors when it fits, letterboxed) and refitted when the page or the game's
  window size changes. Rendering uses WebGL2; input comes from the canvas and keyboard.
  `show_message`/`show_question`/`get_string` use the browser's `alert`/`confirm`/`prompt`.
  Files written by the game (`file_*`, `ini_*`, saves, highscores, registry) live in a
  virtual file system stored in `localStorage` per game id.
- **MIDI**: run `scripts/fetch-soundfont.sh` (or `.ps1`) to download GeneralUser GS
  (S. Christian Collins, ~32 MB, free to redistribute; checksum-verified) to
  `web/soundfonts/default.sf2` (ignored by git), or put another General MIDI SoundFont there, or
  set `INSTANCE_SOUNDFONT=/path/file.sf2` when running the build script; it is copied to
  `dist/web/soundfonts/default.sf2`. `?soundfont=URL` overrides it. Without one, MIDI music
  is silent and the console says so.
- **IDE** (`ide.html`): File > Open uses the browser's file picker (or drop a file);
  Save / Save As download the `.gm81`; the project is autosaved to IndexedDB and offered
  again on the next visit; `?project=URL` opens a project from the server. Run (F5) stores
  the game in IndexedDB and plays it in an overlay (`play.html?game=idb:run/game.gm81`).
  Create Executable is not available in the browser.
- **Limits in the browser**: no DLLs, no multiplayer, no room transitions; `sleep` delays
  the next step instead of blocking, `keyboard_wait`/`mouse_wait` return at once, and the
  in-game modal boxes (highscore name entry, splash images) are skipped. GML recursion is
  limited to 512 nested script calls (the browser's own call stack is the limit).

Standalone executables made by Create Executable carry the game after the runner with a
footer (specs/exe-format.md); executables made by earlier builds (old footer magic) still run.

Not supported yet: loading .exe files compiled by YoYo's GameMaker.
Open spec questions and implementation choices: `specs/OPEN_QUESTIONS.md`.

## Releases

Publishing a GitHub release runs `.github/workflows/release.yml`, which builds and
attaches:

- `Instance-<tag>-windows-x64.zip`: instance-ide, instance-runner, gmx2gm81,
  gmk-info, the 32-bit DLL helper and the MIDI SoundFont
- `Instance-<tag>-macos-arm64.zip` / `-macos-x64.zip` (and the matching
  `Instance-Runner-…` zips): ad-hoc signed `.app` bundles (right-click > Open on
  first launch, or `xattr -dr com.apple.quarantine Instance.app`)
- `Instance-<tag>-web.zip`: the static browser build; serve the folder with any
  static web server

The workflow can also be re-run by hand for an existing tag (Actions > Release >
Run workflow).
