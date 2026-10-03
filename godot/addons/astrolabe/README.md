# Astrolabe support, interface 41

This directory is the authoritative source distributed with the Rust exporter.
It depends only on Godot 4.7.2. Generated assets use explicit resource paths.
No autoload, disc, Rete store, Rust process or checkout path is needed at runtime.

`rendering/sprite.gd` selects between persisted native billboard views using a
camera-facing dot product. Its thresholds, normal and child meshes are native
scene properties. Advanced OpenSpace sprite orientation/UV modes remain partial.

`scene/sector_boundary.gd` offers segment intersection with persisted triangle
boundaries. Its saved ArrayMesh is only a hidden inspection overlay; it does not
implement sector activation or sound propagation.
Static contact conversion uses the physical sectors and their explicit collision
neighbors. Shapes under unreferenced virtual display sectors remain in the saved
scene with collision layer/mask zero; their visible meshes remain present. Virtual
sectors explicitly listed as collision neighbors still provide native contact.
This excludes geometry that no source physical-sector query visits; native
per-actor sector filtering and dynamic sector activation remain pending.

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
The source object-generation API copies the caller's transform and initializes
the spawned Perso's native motion from the AI vector operand. Ordinary Godot
physics integrates that initial velocity; the projectile's compiled AI decides
its aiming, contact response and lifetime. The source vector's final movement
frame and exact collision timing still need a live cross-engine comparison. The
initial conversion uses the spawning actor's basis, but the captured original
arrow displacement does not establish that basis choice because the compiled
arrow AI may change its sighting before the first published motion sample.
The exported level keeps a scene-local map from stable rooted and Global Perso
references to native Brain paths. Object generation binds each compiled
external actor reference to a live Brain before the generated actor starts; a
missing binding fails the call with its source identity. These references are
scene instances and expire when the level is freed.
`gravity` is an explicit world-space acceleration; `external_velocity` retains
force/impulse motion independently of the current clip. Ground contact clears
downward speed without retaining a sideways projection of gravity. Neither native seeking
nor `advance()` alone moves the body's world position. Exported movement
spheres/boxes and their compiled action selections provide contact geometry.
The saved level enables motion for its main actor. When no custom gravity is
supplied, active motion uses the Godot project's default gravity; that value
is provisional until the original manor trajectory is measured.
Other collision purposes and moving point/triangle shapes remain diagnosed.

`source_add_wind(Vector3)` is an **intentional no-op**, explicitly requested for
the opening implementation. The compiler still evaluates its typed vector and
preserves procedure 30 through OpenSpace and Rete. Hype's misleadingly named
`Proc_AddWind` assigns actor-local world-axis drift, which callback `00589bfa`
adds to calculated linear velocity; zero clears that drift. Neither nonzero
wind nor resetting prior drift has physics parity here. Follow-up: add a separate
actor drift property, map its units and consumers, compose it with native velocity
without accumulation, and test nonzero assignment/overwrite/zero reset with contact.

`character/route.gd` holds a saved native `Curve3D`, arrival radii and loop closure.
Shared source waypoints remain shared useful assets; conversion resolves their
positions into the native curve. `source_reset_route(slot)` and the compiled
`GoOverWay` coroutine use actor-local progress and cancellable native steering.
Animation supplies speed and action mechanics limits; `CharacterBody3D` resolves
contact. Fixed StaticLine routes with dynamic kind zero are supported. Curved,
dynamic and actor-relative routes are retained and fail explicitly if called.
No source samples, dynamics records or route interpreter run in the port.

`request_action(id, restart)` selects compiled family transitions, prohibited
destinations, repeat counts and next states. A repeated request preserves the
action unless `restart` is true. `requested_action_state` differs from
`current_state` while a transition is playing. `animation_complete` describes a
completed cycle; `action_complete` and `action_finished(id)` describe the end of
the action chain. `select_state()` is still a direct, looping inspection override.
`action_request_traced` reports each native action request and its resolved
selection for gameplay diagnostics.

The compiled `Proc_ActivateChannel` and `Proc_DeactivateChannel` calls use
`source_set_channel_active(channel, active)`. It sets visibility on every
exported channel parent whose `source_channel` metadata matches the script's
typed channel operand; native animation tracks continue to select child parts.
Compiled inventory functions read and update `character/inventory.gd`, a shared
native Resource holding the selected weapon and item quantities. Generated
Persos share the spawning character's inventory reference. The Resource
implements AI-requested inventory operations only; attack selection, arrow
creation, motion and lifetime must come from compiled source AI. The current
manor behavior fixture restores inventory values observed in the supplied
English save; exported content does not hardcode those values.
Compiled magick reads use `character/magick.gd`, another Resource shared with
generated Persos. It holds current/maximum points and the raw AI selected-spell
code. Its function-230 deduction follows the verified source dispatch: it
subtracts when enough points remain, clamps to the maximum, and returns `1`
for success or `0` for insufficient points. The compiled script supplies the
ten-point argument and decides when to invoke it; no spell cost is hardcoded
into support. The saved menu selection's live raw code remains unverified.
The compiled laser-pointer calls use `character/laser_pointer.gd`, a native
Resource shared with generated Persos. Hiding its presentation preserves the
last target, which the arrow's original AI reads for aiming. Target acquisition
is intentionally unset until a source-backed game/camera ray can supply it;
scripts fail explicitly when the target is unavailable. The native sighting
method rotates the actor from the source vector while preserving position and
scale. Its up/roll choice and zero-vector behavior remain fidelity questions.
`Cond_CollideWithNothing` currently reads whether the current movement step had
no native slide contacts. The pinned source handler for model-733 operation 33
returns true when its published mechanics contact word is zero. Slide-contact
count is a provisional native approximation of that source word; per-arrow
contact counts are retained in behavior traces for comparison. Source hit and
cleanup parity remain open.
The arrow's `Camera_ChangeTargetedPerso(StdCamer, MainActor)` call is compiled.
The level exposes source actor node paths; the native camera API uses the
compiled operand identity to find the saved actor's `SourceCamera` child and
store the requested target reference. A source camera link adds that native
`Camera3D` to the saved actor scene. Follow motion, viewport selection,
projection fields and the procedure's other reset fields remain unsupported;
target assignment alone does not change the view.
The rooted `StdCamer` now keeps its original model-437 Brain. Its source
type-11 Perso list is saved as a typed, capacity-bounded designer value;
compiled AI performs the original membership and append calls. The native
append method retains duplicate entries and refuses to grow past capacity.
The supplied `manoir` list begins empty. Binding a populated source list into
saved Godot actor references remains unsupported and fails explicitly during
export. This support code supplies the called list operation; it does not
choose camera targets or alter the source script's branches.
The observed return-zero/camera-update pair compiles to the native camera
update API. That API still reports unsupported until the source pose and
projection contract is mapped; attaching this Brain does not yet make the
view follow.
The compiled `Stats_DisplayMainActor*` calls retain their requested Boolean
visibility in `source_hud_display` and emit `hud_visibility_requested`. An
authored HUD can bind that signal to its native `Control` nodes. The current
manor gameplay test scene has no HUD composition, so the signal alone does not
establish visual HUD parity.

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
`sound_fade_music(target, duration_ms)` controls a separate native music bus,
using the source 0–127 level and millisecond duration confirmed in the Hype
executable. Converted positional sound events continue to use their emitter bus.

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
See `notes/ai-gdscript-slice.md` and `notes/hype-control-progress.md` in the Astrolabe
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
