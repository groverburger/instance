# gm81-clean

A clean-room reimplementation of the GameMaker 8.1 runner and IDE in Rust.
Not affiliated with YoYo Games. See CLEANROOM.md for how this was built.

- `crates/gm-format` – read/write .gmk (v800) / .gm81 (v810) project files
- `crates/gml` – GML lexer, parser, compiler and interpreter
- `crates/gm-runner` – the game engine (`gm-runner game.gm81`)
- `crates/gm-ide` – the editor
- `specs/` – behavioural specifications the implementation is built from
