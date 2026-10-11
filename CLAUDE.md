# Notes for agents

This repository is **Instance**: a clean-room reimplementation of GameMaker 8.1, with
the runner, the IDE, the `.gmk`/`.gm81` file format, project folders and the `gmk-check`
linter. Work here is engine work.

- **Making a game** with Instance (example games, music and sound tools, the game-making
  guide) happens in the separate `instance-gamekit` repository (`../instance-gamekit`),
  not here.
- **Clean-room rules (`CLEANROOM.md`) are absolute**: never disassemble, decompile or
  debug any YoYo Games binary. Do not copy manual prose or YoYo assets into the repo.
  Implement from `specs/`. File gaps in `specs/OPEN_QUESTIONS.md`.
- **Specs are the source of truth** for behaviour:
  - `specs/gml-language.md`, `specs/runtime.md`: GML and the runtime
  - `specs/file-formats.md`, `docs/project-folder.md`: file formats
  - `specs/lite-edition.md`: what the Lite edition lacks
  - `specs/blackbox-tests.md`: comparing Instance against the real GameMaker
- **Check and test**:
  - `cargo build --workspace --all-targets`
  - `cargo test --workspace` (CI runs both)
  - `cargo clippy --workspace`
  - The `GM81_SAMPLES` / `GM81_CORPUS` test suites need folders of GameMaker 8.1 test
    material.
- **Headless runs** (`instance-runner GAME --headless ...`) must use
  `--save-dir <scratch dir> --no-read-fallback`, so they never read or write save files
  in the game's folder.
