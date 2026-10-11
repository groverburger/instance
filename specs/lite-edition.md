# GameMaker 8.1 Lite vs Standard edition

GM 8.1 came as a free **Lite** edition and a paid **Standard** ("Pro") edition. Instance
has no editions: everything works in it. What Lite lacks matters for black-box testing
in the real GameMaker (`specs/blackbox-tests.md`) and for `gmk-check --lite`, which warns
about Standard-only features in a project meant to run in the Lite IDE.

## Evidence

* `specs/actions.json` `pro_only` flags (drag-and-drop actions): Transform Sprite,
  Color Sprite, Show Video, all Splash actions, Replace Sprite/Sound/Background, all
  particle actions (systems, particles, emitters), all CD actions, Set Cursor, Open
  Webpage, Draw Scaled Text, Horizontal/Vertical Gradient, Gradient Ellipse, Take
  Snapshot, Create Effect.
* Black-box testing on GM 8.1 Lite (`crates/instance-blackbox`): the `ds_list`
  benchmark produced no result (data structures unavailable); the test plan treats
  3D (`d3d_*`), surfaces, particles, blend modes, data structures (`ds_*`),
  multiplayer (`mplay_*`) and DLLs (`external_*`) as Standard-only, and drawing with
  `image_blend` / `image_angle` (rotated / blended sprite drawing) as Standard-only.
  Extension packages are believed to be Standard-only too (not tested).
* Verified to **work in Lite**: the core GML language and runtime (events, alarms,
  rooms, collisions, motion, views, drawing), `image_xscale/yscale/angle` affecting
  bounding boxes, `file_text_*` functions, `draw_getpixel`, `execute_string`,
  `keyboard_key_press`, sounds, timelines, triggers, paths, `random*`.

## How Lite fails

Observed directly in GM 8.1 Lite: calling a Standard-only function is a run-time
ERROR (non-fatal) with the message `This function is not available in the Lite
Edition.` (seen for `ds_list_create`), and *writing* a Standard-only variable raises
`image_blend: This variable is not available in the Lite Edition.` every time it is
assigned (seen for `image_blend` in a collision event, so it floods the error log).
Assume every other Standard-only feature fails the same way.

## Checking a project

`gmk-check --lite PROJECT` reports these features. By hand: grep for `ds_`, `surface_`,
`d3d_`, `part_`, `effect_`, `draw_set_blend_mode`, `external_`, `mplay_`, `image_angle`,
`image_blend`, `draw_sprite_ext`, `draw_self`. Lite-friendly substitutes: arrays (global
or instance) instead of `ds_list`/`ds_grid`; several objects or a parent instead of
maps; frame animation instead of rotation; pre-coloured sprite frames instead of
`image_blend`.
