# gm81-clean

A clean-room reimplementation of the GameMaker 8.1 runner and IDE in Rust.
Not affiliated with YoYo Games. See CLEANROOM.md for how this was built.

- `crates/gm-format` – read/write .gmk (v800) / .gm81 (v810) project files
- `crates/gml` – GML lexer, parser, compiler and interpreter
- `crates/gm-runner` – the game engine (`gm-runner game.gm81`)
- `crates/gm-ide` – the editor
- `crates/gm-net` – multiplayer (`mplay_*`) over gm81-clean's own TCP/UDP protocol
- `crates/gm-dllcall`, `crates/gm81-dllhost32` – DLL bridge (x64 in-process, x86 via a 32-bit helper)
- `specs/` – behavioural specifications the implementation is built from

## Usage

```
cargo build --release
target/release/gm-ide [project.gm81]          # editor; F5 runs the game
target/release/gm-runner game.gm81            # play a .gmk (v701/v800) or .gm81 project
target/release/gm-runner game.gm81 --headless --frames 300 --screenshot out.png
target/release/gmk-info game.gm81 [--tree]    # print a resource summary
cargo test --workspace                        # sample tests read $GM81_SAMPLES
target/release/gm-runner game.gm81 --headless --frames 600 --random-input 1 --max-seconds 60
GM_PROFILE=1 target/release/gm-runner game.gm81 --headless   # per-phase/event/function timings
cargo test --release -p gm-runner --test corpus -- --ignored --nocapture   # runs every game in $GM81_CORPUS
```

Multiplayer: `mplay_*` over TCP/IP only (gm81-clean protocol, specs/multiplayer.md
Part B; not compatible with GameMaker's DirectPlay sessions). Demo with two headless
runners: `cargo build -p gm-runner && cargo run -p gm-runner --example mplay_demo`.

Extensions and DLLs: packages are taken from the executable's bundle or from
`extensions/` next to the runner, `$GM81_EXTENSIONS`, `%APPDATA%/gm81-clean/extensions`
or the project's `extensions/` folder. 32-bit DLLs (all GM 8.1-era DLLs) need the helper:
`rustup target add i686-pc-windows-msvc && cargo build --release -p gm81-dllhost32
--target i686-pc-windows-msvc`, then put `gm81-dllhost32.exe` next to `gm-runner.exe`
(or set `GM81_DLLHOST32`; cargo target directories are also searched).

Not supported: compiled game .exe files, MIDI music, D3D 3D mode.
Open spec questions and implementation choices: `specs/OPEN_QUESTIONS.md`.
