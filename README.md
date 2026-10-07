# gm81-clean

A clean-room reimplementation of the GameMaker 8.1 runner and IDE in Rust.
Not affiliated with YoYo Games. See CLEANROOM.md for how this was built.

- `crates/gm-format` – read/write .gmk (v800) / .gm81 (v810) project files
- `crates/gml` – GML lexer, parser, compiler and interpreter
- `crates/gm-runner` – the game engine (`gm-runner game.gm81`)
- `crates/gm-ide` – the editor
- `specs/` – behavioural specifications the implementation is built from

## Usage

```
cargo build --release
target/release/gm-ide [project.gm81]          # editor; F5 runs the game
target/release/gm-runner game.gm81            # play a .gmk (v701/v800) or .gm81 project
target/release/gm-runner game.gm81 --headless --frames 300 --screenshot out.png
target/release/gmk-info game.gm81 [--tree]    # print a resource summary
cargo test --workspace                        # sample tests read $GM81_SAMPLES
```

Not supported: compiled game .exe files, MIDI music, D3D 3D mode, multiplayer, DLLs.
Open spec questions and implementation choices: `specs/OPEN_QUESTIONS.md`.
