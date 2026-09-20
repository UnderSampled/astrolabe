# Astrolabe support, interface 17

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
`_process` for stationary inspection. Godot evaluates native tracks. The driver bounds
advances at native method keys, half-clip intervals and loop edges to retain cues and motion across large ticks,
and handles state changes requested by cue callbacks after the mixer returns.
For independent deterministic control, disable processing on the character and
call `advance` yourself. Use native AnimationPlayer seeking or `seek_frame` for
side-effect-free inspection. The saved initial pose needs no script execution.
The `restored/` library and reset parts are generated ahead of time from the
canonical reset configuration. Editing Rete and regenerating updates them.

The character root is a `CharacterBody3D`. `motion_enabled` opts into physics
playback, applying native root translation once through `move_and_slide`.
`gravity` is an explicit world-space acceleration; `external_velocity` retains
force/impulse motion independently of the current clip. Neither native seeking
nor `advance()` alone moves the body's world position. Exported movement
spheres/boxes and their compiled action selections provide contact geometry.
Other collision purposes and moving point/triangle shapes remain diagnosed.

`request_action(id, restart)` selects compiled family transitions, prohibited
destinations, repeat counts and next states. A repeated request preserves the
action unless `restart` is true. `requested_action_state` differs from
`current_state` while a transition is playing. `animation_complete` describes a
completed cycle; `action_complete` and `action_finished(id)` describe the end of
the action chain. `select_state()` is still a direct, looping inspection override.

`character/mechanics.gd` exposes promoted mechanics parameters for inspection;
it does not simulate the original dynamics. Family bounds are saved hidden
native geometry. Morphs, interpolation, root rotation and gameplay visibility
remain incomplete; inspect generated diagnostics for affected assets.

`audio/voice.gd` carries a native AudioStreamWAV and playback properties.
`audio/emitter.gd` owns native voices, selectors, weighted/nonrepeat choice,
random pitch and event-targeted stopping. Compiled event Resources provide
`execute(emitter)` and supported play events provide `select(emitter)`.
`voice_started(info)` exposes the selected stream, gain, pitch and source-choice
trace for testing/game integrations. A stopped nonabortable voice continues at
its captured position until it finishes or its owning scene is destroyed.
No source-shaped sound program or opcode interpreter is loaded at runtime.

`behavior/behavior.gd` provides timing, cancellation and bindings for generated
normal/reflex behavior coroutines. Each saved `Brain` node has typed `dsg_N` instance properties;
`main_actor_path` is a level-owned NodePath and `sound_emitter_path` resolves the
perso's native emitter. A standalone perso must bind `main_actor` before calling
`start()`. `automatic` uses native physics ticks; disable it and call
`advance(seconds)` for deterministic stepping. `stop()` cancels suspended work,
and `start()` resets the clock while preserving designer values. A wait captures
an integer millisecond duration or a frame count; the generated coroutine resumes
after that wait. A completed schedule starts again on its next allowed tick.
`sound_requested(identity)`
reports the executed native event. No source script data is loaded at runtime.

Interface 16 adds ordered rules, reflex priority and behavior transitions.
Generated branches and native coroutine stacks retain a suspended rule's place;
a blocking rule restarts only its schedule's pending action. A reflex meta-action
owns its completion tick too, without restarting the normal group's wait.
`change_normal` and `change_reflex` invalidate the old group's coroutines; a
cross-group request leaves its caller active. Compiled source yield/request
pairs defer entry until the next allowed tick. No generic tasks, instruction
cursors or source return flags are part of this interface.
The Astrolabe torch schedule, focused native flow/action/input variations and
Hype's complete idle rule 0 are checked. Full player control and original frame
cadence remain unfinished.
See `docs/ai-gdscript-slice.md` and `docs/hype-control-progress.md` in the Astrolabe
repository for evidence and limits. Interface numbers 3–12 and Rete schemas
11–20 were used by the archived experiment; this implementation skips them so
incompatible preserved local output cannot accidentally pass a version check.

Compiled action calls use the character's `request_action`, `is_in_action`, and
completion properties from generated coroutines. Designer action arrays become native integer arrays
after validating every member against the receiving family. Actor fields bind
as ordinary exported node references in the saved scene. These compiler helpers
do not yet import or run Hype's full player behavior.

`adopt_input(action, physical_key)` adds a missing InputMap action and default
physical key; existing entries, including empty disabled controls, remain owned
by the project. The compiler emits defaults for Hype's arrows, Shift, Ctrl,
Space, X, Z and Enter. Other logical controls warn when they lack bindings.
Define the same names in Project Settings > Input Map to override them. Runtime
adoption does not save or rewrite `project.godot`. Automatic Brain startup waits
until the parent's initial pose has been selected; `stop()` also cancels that
pending start. Action-array element assignments are ordinary native array writes.
