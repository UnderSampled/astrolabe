# Astrolabe support, interface 1

This directory is the authoritative source distributed with the Rust exporter.
It depends only on Godot 4.7.2. Generated assets use explicit resource paths.
No autoload, disc, Rete store, Rust process or checkout path is needed at runtime.

`rendering/sprite.gd` selects between persisted native billboard views using a
camera-facing dot product. Its thresholds, normal and child meshes are native
scene properties. Advanced OpenSpace sprite orientation/UV modes remain partial.

`scene/sector_boundary.gd` offers segment intersection with persisted triangle
boundaries. Its saved ArrayMesh is only a hidden inspection overlay; it does not
implement sector activation or sound propagation.

Source checkout support is maintained through version control. Adopted projects
update it explicitly with `astrolabe update-godot-support PROJECT`. Content
regeneration checks the interface version and validates the installed support;
it never replaces local support edits.

`character/character.gd` controls saved AnimationPlayer clips. `initial_state`,
`current_state`, `current_mechanics` and `autoplay` are exported properties.
`select_state(id)` chooses a native `state_ID` clip; `playing`, `seek_frame(frame)`,
`restore_configuration()` and `set_bounds_visible(bool)` are inspection/game
extension points. `cue_triggered(action, frame, column, configuration_slot)`
exposes a typed `cue.gd` Resource. Sound cues execute native event resources;
other actions currently emit the signal and a conversion diagnostic.

The player processes manually through `advance(seconds)`, also called by
`_process`. Godot evaluates native property/method tracks. The driver bounds
advances at native method keys and loop edges to retain cues across large ticks,
and handles state changes requested by cue callbacks after the mixer returns.
For independent deterministic control, disable processing on the character and
call `advance` yourself. Use native AnimationPlayer seeking or `seek_frame` for
side-effect-free inspection. The saved initial pose needs no script execution.
The `restored/` library and reset parts are generated ahead of time from the
canonical reset configuration. Editing Rete and regenerating updates them.

`character/mechanics.gd` exposes promoted mechanics parameters for inspection;
it does not simulate the original dynamics. Family bounds are saved hidden
native geometry. Morphs, interpolation, root motion and gameplay visibility
remain incomplete; inspect generated diagnostics for affected assets.

`audio/voice.gd` carries a native AudioStreamWAV and playback properties.
`audio/emitter.gd` owns native voices, selectors, weighted/nonrepeat choice,
random pitch and event-targeted stopping. Compiled event Resources provide
`execute(emitter)` and supported play events provide `select(emitter)`.
`voice_started(info)` exposes the selected stream, gain, pitch and source-choice
trace for testing/game integrations. A stopped nonabortable voice continues at
its captured position until it finishes or its owning scene is destroyed.
No source-shaped sound program or opcode interpreter is loaded at runtime.
