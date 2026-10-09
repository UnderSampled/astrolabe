# Astrolabe support, interface 85

Interface85 adds the source continuation stencil as a native TextureButton.
Completed nonfinal dialogue pages show it at the source style anchor; clicking
uses the same selection path as Button_Hit. Finite final speech pages retain
their speech-holder completion contract. The native screen and compiled Hype
rules claim each physical Button_Hit press once, preventing a double advance.
Dialogue presentation and level assets
must be regenerated from Rete schema63; support updates alone lack the texture.
The exporter also compiles source vector subtraction (operator17), allowing the
entrance gate closing MoveLift to reach its authored endpoint and stop cue.
Cond123 retains its evaluated actor operand. Action waits and conditions use
the native completion flag together with current/target state identity;
automatic next states realign that target while retaining the numeric request.
Animation completion remains independent of action and speech completion.

Interface85 scopes STOP to its effects/voice/music module and publishes request
removal completion on the next native frame, including detached nonabortable
playback. `audio/mixer.gd` holds three source fade words independently of user
preference buses. Scripted source/object updates refresh dynamic players;
preference publication refreshes all active matching players, including current
static gain/pose. Raw effects overrides use low32 multiplication, signed SAR14
and final byte narrowing; exact127 remains the no-override sentinel. Emitters
expose the published byte and source/preference/object words in request signals.
The authored settings file and native bus names stay the same. Native byte gain
normalization, resource gain and distance attenuation still require external
driver evidence; this interface does not claim audible New Game mix parity.

Interface84 saves a level-owned `scene/sector_index.gd` component with ordered
sector, physical-visual and mesh NodePaths. Camera neighbor/history lookups use
these native bindings. Downward sector classification uses Godot TriangleMesh
BVHs shared by Mesh resources and cached on each sector boundary; it preserves
sector order, strict nearest-distance ties, activation/virtual-sector gates and
the existing incomplete-GEO diagnostics. Explicit publication reuses its first
point result when that point is selected. Level scenes must be regenerated;
support updates alone do not supply the new bindings. Query geometry follows
native Godot ray/vertex tolerances, with a small origin margin for surface hits.

Interface84 also suspends unselected configuration banks and hidden animation
slots through native process inheritance and top-level transforms. Physical
fallback areas use REMOVE disable mode; visibility restores local transforms
before physics participation. Independent ZDE/ZDR zones keep their own state
rules. Perso scenes must be regenerated with this support version to save the
new defaults, including initial actor activation and slot participation.

Interface83 retains byte-narrowed channel requests even without a matching rig
node. Actor-owned overrides combine with native animation default-visibility
keys, and disabled IDs are reapplied before native animation evaluation.
It also exposes read-only actor current-sector identity through saved named
references, including a known null holder value. Proc386 retains the raw effects
volume override, and Proc412 uses a bounded native effects fade followed by
effects-only stop/reset. Voice playback remains independent. Explicit camera
translation publication and changed boundary history update the actor's sector;
mode-zero publication captures the saved final sector-child identity before its
point lookup, selects the source second/third point, then updates membership
before translation.
native walk/mass motion maintains an existing sector. Ready-time null remains
null until an explicit lifecycle publication; queries do not assign containment.

Interface82 retains actor-owned activation/suppression and placed/generated
removal, preserves paused compiled Brains, permits direct Proc12 placement of
the bound camera and null-card key, and implements immediate Proc166 camera
flags. Source reset-on-kill and deferred camera flag groups remain unsupported.
Source primary-camera ownership remains independent of an authored presentation
camera. Initial registration and viewport context remain distinct. Proc166
evaluates its scalar operands before reloading the immediate camera binding.
Proc171 resets the camera actor's upper vertical limit while retaining its
lower limit. Primary Proc141 selection connects different camera owners through
the maintained framing reset, separate registration/context fields and native
actor contact-sector refresh. Export binds the ordered no-hit sector fallback
as a scene NodePath. Same-owner handback remains; pose-copy/optimal placement
and secondary viewports remain unsupported.
The authored game's optional training checkpoint retains settled logical state;
no source execution cursor is serialized. See
[`Manor training static handoff`](../../../notes/manor-training-static-2026-10-08.md).

This directory is the authoritative source distributed with the Rust exporter.
It depends only on Godot 4.7.2. Generated assets use explicit resource paths.
No addon autoload, disc, Rete store, Rust process or checkout path is needed at
runtime. The Godot starter project has a small `UserSettings` autoload for its
authored audio preferences.

`rendering/sprite.gd` selects between persisted native billboard views using a
camera-facing dot product. Its thresholds, normal and child meshes are native
scene properties. Advanced OpenSpace sprite orientation/UV modes remain partial.

`scene/sector_boundary.gd` offers two-sided segment intersection and source-
directed crossing queries over persisted triangle boundaries. Its saved
ArrayMesh is only a hidden inspection overlay; sector activation and sound
propagation remain separate from this geometry helper.
Static contact conversion uses the physical sectors and their explicit collision
neighbors. Shapes under unreferenced virtual display sectors remain in the saved
scene with collision layer/mask zero; their visible meshes remain present. Virtual
sectors explicitly listed as collision neighbors still provide native contact.
This excludes geometry that no source physical-sector query visits. Actor
contact queries use saved native sector references, collision neighbors and
activation flags. Movement across a saved portal updates the actor's sector;
broader activation scheduling remains partial.

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
Generation finds the containing level even when an authored session owns the
current scene. It retains that level's MainActor and external Brain bindings,
and duplicates mutable stock for each spawned actor.
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

`source_begin_actor_pursuit(actor, radius)` captures a native actor and rounded
radius, then samples its current origin through the same point-motion component.
`source_turn_toward(actor)` applies an immediate horizontal heading request.
Training mannequin hit reactions use their compiled Normal/Reflex behaviors and
native family action clips; the support package adds no separate spin animation.
`source_activate_at_position(point)` supports Character0, the bound mode3 camera
and a converted null identity card: elevated placement, downward native contact,
then activation. Other mechanics and actors under moving character parents
fail before placement. Suppressed actors retain their activation policy while
receiving the supported placement. Exact original contact
response and training timing still require runtime comparison.

`camera/framing.gd` stores the converted initial camera preset as native
properties. `Camera_Reset` restores those mapped parameters, the first-update
snap flag, camera-local progress and finite vertical limits. It preserves the
selected target and the separate session-owned controller history.

`source_camera_go_to_waypoint(camera_actor, waypoint)` implements the compiled
`Camera_GoToWP` request over captured native references. Each tick samples the
waypoint's current world position and camera actor translation, returns 4 for
inclusive radius arrival, or supplies displacement to `SourceCamera` and returns
1. Invalid bindings return -1 and fail the compiled caller. The camera's ordinary
position update owns motion; the request does not install a tween. MainActor and
external actor designer-array writes mutate their bound Brain arrays directly.
`source_kill_perso()` and `source_kill()` share Boolean removal: supported placed
actors retain identity, selected Brain and suppression; generated owners release
their nodes. Ordinary deactivation pauses Brain progression and bulk-stops voice
requests, whose guarded completion runs on the next native process frame.
Unknown/reset-on-kill policies return false to the compiled caller.
These additions have source review only; see the dated Manor
tunnel note for required runtime acceptance and pool-lifecycle limits.

`request_action(id, restart)` selects compiled family transitions, prohibited
destinations, repeat counts and next states. A repeated request preserves the
action unless `restart` is true. `requested_action_state` differs from
`current_state` while a transition is playing. `animation_complete` describes a
completed cycle; `action_complete` and `action_finished(id)` describe the end of
the action chain. `select_state()` is still a direct, looping inspection override.
`action_request_traced` reports each native action request and its resolved
selection for gameplay diagnostics.

The compiled `Proc_ActivateChannel` and `Proc_DeactivateChannel` calls use
`source_set_channel_active(channel, active)`. The actor-owned
`character/channel_visibility.gd` component narrows the operand to a byte,
retains visibility overrides and ordered deactivated IDs, and sets visibility
on every exported parent whose logical `source_channel` matches. An ID absent
from the rig still changes that component state successfully, as original
`00541880` does; a missing `Channels` binding or malformed matching node still
fails. Native `channel.gd:sample_visibility` keys supply object-channel defaults
and consume redundant overrides; absent/hidden channels do not consume them.
Part-selection tracks stay independent of this visibility, including activation
of a source-hidden object. Action changes reuse the native parents and component.
Native animation updates reapply the persistent deactivated IDs. These are ordinary
properties, native Animation tracks and visibility; no source channel/frame
tables or OpenSpace module/linked-list machinery run in the port.
Compiled inventory functions read and update `character/inventory.gd`, a shared
native Resource holding the selected weapon and item quantities. Generated
Persos share the spawning character's inventory reference. The Resource
implements AI-requested inventory operations only; attack selection, arrow
creation, motion and lifetime must come from compiled source AI. The current
manor behavior fixture restores inventory values observed in the supplied
English save; exported content does not hardcode those values.
`character/contact_pickup.gd` evaluates picking-center distances in an explicit
native inverse frame. This is a deterministic adaptation of the original's
undefined scratch flag, with no claim of numeric parity. `Cond28` reads the
selected native ZDE, prioritizes target ZDE over physical model shapes, and
requires every specified low-16 material bit on the actual contact. Native
intersection queries are capped at 4096; their ordering remains an adaptation.
`ui/pickup_artwork.gd` contains converted Texture resources for the full item
catalogue, including nonselectable food/gold artwork. Compiled pickup feedback
calls the authored `game/inventory_host.gd`, which creates a native pausable
Control and alpha tween through `ui/pickup_feedback.gd`.
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
store the requested target reference. A source camera component adds that native
`Camera3D` to the saved actor scene, with the initial parameter set's horizontal
FoV, source renderer clip planes and a half-turn matching the source viewing
axis. Four framing parameter sets and their unknown scalar fields persist in
Rete (currently schema 54). The active mode and rotated actor/look offsets
export as native properties. Mode 3 aims the camera actor from its current eye position at the
selected actor plus those offsets, using `Node3D.look_at`. A zero look offset
uses the source default of [0,-0.05,0]. The compiled camera rule requests each
update; an authored native AnimationPlayer can move the eye independently.
Camera procedure 160 accepts source keyword and rooted actor targets. Procedure
157 supports the opening's 0/3 transitions, equal-state requests and -1 restoring
the exported initial preset; other transitions fail explicitly. A referenced
Ultra receiver's procedure 253 writes native global position, converting source
coordinates once. Fixed AI waypoint positions become native vector constants
during export, using the same meaningful waypoint assets as route curves.
Actor-relative waypoints fail when reached until their bindings are promoted.
Initial target binding, viewport selection, follow/obstruction and other reset
fields remain unsupported. Target assignment alone does not change the view.
The rooted `StdCamer` now keeps its original model-437 Brain. Its source
type-11 Perso list is saved as a typed, capacity-bounded designer value;
compiled AI performs the original membership and append calls. The native
append method retains duplicate entries and refuses to grow past capacity.
The supplied `manoir` list begins empty. Binding a populated source list into
saved Godot actor references remains unsupported and fails explicitly during
export. This support code supplies the called list operation; it does not
choose camera targets or alter the source script's branches.
The observed return-zero/camera-update pair compiles to the native camera
update API. It accepts mode-3 aiming with a valid target, rejects unsupported
modes and degenerate eye/target geometry, and does not select a viewport.
Attaching this Brain does not yet reproduce the intro or follow camera.
The compiled `Function_Camera_GetShiftAlpha` and `Function_Camera_GetShiftTeta`
expressions preserve their integer selector and Perso receiver. The mapped
selectors used by camera model 437 read raw parameter floats `+0x44` and `+0x4c`
from its `SourceCamera`; unobserved selector values fail explicitly.
The compiled `Stats_DisplayMainActor*` calls retain their requested Boolean
visibility in `source_hud_display` and emit `hud_visibility_requested`. An
authored HUD can bind that signal to its native `Control` nodes. The English
Fix HUD is attached to generated level screens. Its six GLI handles join through
Fix.ptx/Fix.rtt to named members in `Textures.cnt`; export creates editable
native `TextureRect` nodes and binds player life to actor state. Currency uses
the source `InventoryFont` atlas and dedicated item-0 inventory quantity. These
source joins and saved-scene composition are verified; complete rendered and
runtime parity is still open.

`character/mechanics.gd` exposes promoted mechanics parameters for inspection;
it does not simulate the original dynamics. Family bounds are saved hidden
native geometry. Morphs, interpolation, root rotation and gameplay visibility
remain incomplete; inspect generated diagnostics for affected assets.

`character/armor_catalog.gd` is the native resource graph for the Fix-owned
armor definitions. Each generated level clones it, so the level's Brain scripts
share mutable selection and definition state while separate level instances
remain independent. `character/armor_state.gd` keeps the current
`source_armor_state()` / `source_select_armor()` interface and forwards the
selected byte index to that catalog. The catalog's strict save-snapshot checks
use the staged `character/snapshot_values.gd` helper. Imported resources retain
Texture2D artwork, normalized pieces, damage factors, resistance ranks, source
order, and unknown bytes. The HUD still uses its existing placeholder; armor
rendering is not part of this slice.

`character/snapshot_values.gd` provides strict exact-key and integer-range
checks for native save-state Resources. It rejects extra/missing fields and
non-integer numeric values before any checkpoint state is applied.

`audio/voice.gd` carries a native AudioStreamWAV and playback properties.
`audio/emitter.gd` owns native voices, selectors, weighted/nonrepeat choice,
random pitch and event-targeted stopping. Compiled event Resources provide
`execute(emitter)` and supported play events provide `select(emitter)`.
`voice_started(info)` exposes the selected stream, gain, pitch and source-choice
trace, source module and native bus for testing/game integrations. STOP selects
only the request's effects, voice or music module, including when modules share
an event identity. `execute_voice_event(event)` dispatches a compiled voice event
without changing the speech holder. Per-object volume changes refresh dynamic
effects; existing static effects retain their captured gain. Removed requests
publish `voice_completed`
on the next native frame; `completion_reason` distinguishes natural finish,
event stop, bulk kill and effects fade. A stopped nonabortable request completes
while its native player continues at its captured position until natural finish
or owning-scene destruction. Native finish frees that player without publishing
a second completion.
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


The maintained `ui/screen.tscn` composes letterboxes, dialogue and HUD on one
CanvasLayer. Generated levels instance it and provide converted native dialogue
resources and the main-actor binding. Authored scene compositions may adopt the
level through an already-ready `AstrolabeUI` parent; content regeneration does
not replace customized support scenes. While dialogue is active, the maintained
screen routes the configured `Button_Hit` action to `Proc_SelectDialog`. See
`docs/astrolabe/ui.md` in the source
repository for source API coverage, locale handling and remaining fidelity work.

`ui/menu/menu.gd` and `ui/menu/button.gd` provide native focus, texture states and
short audio feedback for generated editable menus. `action_selected(StringName)`
keeps application routing outside converter support. Inherit a generated menu
scene under `game/` and assign it to the bootstrap to preserve designer changes.
The source pause overlay deliberately has no backdrop; `background_required`
is false only for that generated composition. `focus_initial()` supports modal
reopening, and `cancel_activation()` prevents a closed confirmation timer from
dispatching an action later. Authored `game/pause_host.gd` owns pause and game
routes, without replacing current session state.

The particle selection API uses an actor-local `ParticleSource` and converted
`CPUParticles3D` scenes with the original sprite material/texture and capacity.
`Proc_SetParticleGenerator` selects and initializes that scene; the following
`Proc_DisableParticleGenerator` stops births on the selected component. Actors
without a source component follow the original API's skip. Hype has a real
component, so its initialization must select the real converted generator.
Active emission, source random velocity, infinite lifetime and boundary removal
remain unsupported: enabling a present component fails explicitly. Initially
selected/active source components also fail conversion pending a validated
saved-state contract. The first supported slice starts unselected and disabled.


`ui/inventory/` composes native category Buttons, six item Buttons, original
TextureRects, a bitmap FontFile and native item Resources. `screen.gd.bind_state`
reads an existing actor inventory Resource; quantities and selected weapon retain
their gameplay/save ownership. It emits `item_requested` without implementing
item effects. Layout, texture and font conversion happen ahead of runtime; no
source records or font tables are interpreted by the UI. The authored
`game/inventory_host.gd` owns modal pause and the weapon-selection adapter.
See [inventory ownership](../../game/inventory/README.md) for generation,
inherited scene overrides and the current limits.

Interface 73 registers `ui/location_title.tscn` under the shared Screen canvas.
Style4 uses the converted two-line untimed/unvoiced title resource and its native
font/stencils; the compiled source Brain owns its 200/3000ms waits and lifecycle.
Screen dispatches Position/Run/Select/End/IsOver through the active presentation.
End closes without a completion callback; level adoption/release cancels both
presentations immediately. The game session exposes `source_get_game_state()`
for compiled Func 195, returning selectors 0..8 or null for the source's
unrepresented combined-mask stale-result case. There is no title load trigger.

`ui/vignette.tscn` presents a converted Texture2D with a positive millisecond
hold measured after its first drawn frame. Compiled Proc307 queues that image
on the active game session, which freezes gameplay and returns through its
existing main-menu route after presentation teardown. Artwork is an AI content
dependency resolved at source import; runtime does not read the source CNT.
There is no added fade or zero-duration input-dismiss path for this procedure.

Actor emitter bulk termination reports voice completion on the next native
frame, through the same guarded dialogue callback as natural playback finish.
Explicit owner speech stop still clears its record immediately. These distinct
paths preserve completion ownership without treating bulk request kill as a
synchronous source holder clear.

The bounded style5 dialogue path uses the source center stencil without frame
edges. It accepts one unvoiced page of flags3 lines with no line layout or
answer side effects. Null-text rows retain their vertical/page positions;
normal and emphasis spans use the converted bitmap font, with four ordered
emphasis passes before the normal foreground. Bit0 bypasses letter
reveal, and bit1 permits selection only after the complete page has drawn.
The shared Screen retains the existing owner/completion callback and explicit
source selection. `keysico_gogood` has this contract; it has no icon or timed
auto-close. Other style5 contracts remain explicit import/runtime failures.
