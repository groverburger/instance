# Clean-room policy

This project is an independent, clean-room reimplementation of the GameMaker 8.1
runner and IDE, written in Rust. It is not affiliated with or endorsed by YoYo Games.

## Permitted inputs (spec phase)
- The user-facing manual (Game_Maker.chm, extracted to an out-of-tree reference dir).
- Plain data shipped with or produced by the program: project files (.gmk/.gm81),
  action libraries (.lib), extension packages (.ged), the `fnames` identifier list.
- Black-box observation of the program's behaviour and the files it reads/writes.

## Forbidden
- Disassembling, decompiling, or debugging any YoYo Games binary
  (Game_Maker.exe, GameMaker.exe, rundata, dxdata, DLLs).
- Copying manual prose verbatim into this repo. Specs paraphrase; only identifiers,
  signatures, constant names and numeric values are reproduced (needed for interop).
- Copying YoYo assets (sprites, sounds, backgrounds, examples) into this repo.

## Process
1. Spec writers read the permitted inputs and write behavioural specs in `specs/`.
2. Implementers work from `specs/` (and public, non-YoYo references such as Rust
   crate docs). They do not consult the reference directory.
3. Gaps in a spec are filed back to `specs/OPEN_QUESTIONS.md`, resolved by a spec writer.
