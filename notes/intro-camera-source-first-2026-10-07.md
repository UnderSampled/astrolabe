# Intro camera source-first pass — 2026-10-07

This pass starts from committed `main` and keeps camera work isolated in
`/tmp/astrolabe-intro-camera-source-pass`.

## Established bindings and fields

- Camera activation in `00440820` resolves the active `MainActor` and writes it
  through `00440fd0` into camera component `+0x10`. Procedure 160
  (`00548070`) writes its resolved actor to that field; procedure 163
  (`00548250`) restores the current `MainActor`. The scene exporter now gives
  each generated `SourceCamera` a `NodePath` to the exact `scene.main_actor`
  asset. Compiled procedure 163 retains its own typed AST opcode and calls the
  native reset API with the Brain's bound `main_actor`.
- `0043daf0` reads the active byte at parameter `+0x90` and viewport context
  at `+0x9c`. The model's source-order unknown tail puts these at indices 88
  and 96. Context 1 selects Godot's primary viewport; other contexts are not
  mapped. Parameter set 0 supplies this registration and the initial motion
  mode; set 1 still supplies the supported FOV/mode-3 framing.
- `00440f80` reads the signed target-child selector at parameter `+0xa0`, which
  is byte 100 in the preserved parameter tail. `-1` means the selected Perso
  itself. A nonnegative selector is retained and rejected explicitly until a
  source-child-to-Godot-node map exists; the exporter does not substitute an
  arbitrary Godot child. The manor camera record 0 has raw byte `0xff` at its
  `+0xa0` selector (the camera-record probe's allocation offset 160), so its
  startup target is the selected Perso itself.
- Camera activation (`00440820`) initializes component `+0x10` from the
  current MainActor; procedure 160 replaces it with its resolved Perso and
  procedure 163 resets it to the current MainActor. The exporter binds the
  manor camera's initial target to the exact `scene.main_actor` node, and the
  AI compiler retains procedure 163 as a typed reset call. Later procedure
  160 changes continue using the existing target API.
- `0051c8d0`, `0051d330`, and `0051d140` establish raw mode-0 target geometry
  from the selected actor's source basis and actor, target, and look offsets.
  The native camera exposes this unsmoothed geometry; it does not claim that
  it is the controller's current candidate eye.

## Collision and pose boundary

`0051f670` uses three segments: controller `+0x3c` to `+0x30`, controller
`+0x18` to `+0x30`, and controller `+0x18` to the selected actor's world
translation plus source-Y 1.8 (Godot Z -1.8). Its collision path goes through
`00463df0`, which updates sector context, searches active sector and
collision-neighbor lists, ignores material identifier bit `0x8000`, and
excludes the camera owner/selected target in the actor case. The helper now
requires all three actual controller points explicitly; it no longer
substitutes an unsmoothed actor anchor for `+0x3c`.

The exporter already marks static collision reachability, material IDs, and
sector links. It now also stores each sector's source node ID. The camera ray
provider maps Godot physics layers 1/2 to exported static/actor shapes and
filters with those sector, material, and actor exclusions. It reuses that
metadata, but it is not wired into the source update because the exact
controller points and dynamic source-sector boundary history are still needed.
The shared session Resource carries the source counters and the `005200a0`
teleport gate. The pose publisher mirrors `0051c7b0`'s branch: it tests the
source-resolved sector of its third position argument and selects that point
or its second argument for owner translation.

Mode 0 still reports unsupported through the AI API. The missing connection is
the controller-produced `+0x3c`, `+0x18`, and `+0x30` state and its candidate
result: `0051e620` obtains it through `0051db70`, including angle/distance state
in `0051d410`, `0051d6e0`, and `0051d7b0`. Sector updates in `00448280` also use
source boundary history; the current Godot sector metadata does not encode
that history. The collision provider therefore requires the actual controller
points, and the camera never treats uncomputed state as a clear ray or
successful pose update. No source runtime values or poses were substituted.

## Checks

`cargo fmt --all -- --check`, `git diff --check`, offline `cargo check`, and
the focused camera metadata, AI reset round-trip, and compiler tests passed.
Godot 4.7.2 `--check-only --script` parsed `source_camera.gd` and
`source_controller_state.gd`. The Godot-backed camera scene test could not
start because the sandboxed editor could not create its socket or write its
user editor settings; the exact focused command was sent to the parent task to
run outside this sandbox. No original-game run, debugger, new harness, or
install was used.
