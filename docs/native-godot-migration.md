# Native scene migration

Implementation decisions, 2026-09-13. This work projects the existing canonical
scene/character assets; it does not introduce a second model or change source
reconstruction. The supported local fixture remains English PC Hype/Astrolabe.

## Ownership checkpoint

`godot/` now contains the committed source-free starter and the single maintained
`addons/astrolabe/` support source. `init-godot` adopts settings/bootstrap once;
`update-godot-support` preflights a separate installed-file receipt and refuses
edited support. The source checkout uses Git to update support. Regeneration
validates the installed interface and actual support code without replacing it.
Support interface 1 targets Godot 4.7.2; script `.uid` files stay with their owners.

Generated manifest schema 2 tracks selected-level dependency closures. Shared
assets can have multiple owners. Regenerating A preserves B and its dependencies;
changing a shared resource still used by B reports a conflict. Only unchanged
orphaned generated files are deleted. `preview.tscn`, its script and diagnostic
`validation.json` are level-local. Old preview layouts require fresh generation.

Checks at this checkpoint: Rust support/update/ownership unit tests; actual
Godot 4.7.2 import and execution of the dehydrated bootstrap. The native scene migration and complete local-suite acceptance are recorded below.

## Native character projection

Raymap `PersoBehaviour.UpdateFrameMontreal` resolves per-frame parent columns and
local transforms; the engine evidence and semantic corrections are in
[scene-animation-research.md](scene-animation-research.md). Preserve those exact
values in Model/Rete. At Godot conversion, compose the source affine transforms
into character-relative transforms on stable logical channel nodes. Pre-instance
part alternatives and animate native transform, visibility and opacity properties.
Use discrete keys to retain full affine/shear values and the established sampled
semantics. Do not infer smooth interpolation, root motion, morphs or gameplay
visibility overrides. AnimationPlayer/AnimationLibrary own target playback and
seeking. Runtime support may choose native clips and implement cue/engine behavior;
it must not resolve source matrices, object tables or per-frame hierarchies.

Animation libraries are specialized to the owning family/configuration, while
geometry and materials retain their independent shared identity. Perso scenes
persist an initial pose and are instanced by the meaningful level hierarchy.
A reset configuration is converted ahead of time as another native library/part
selection. Bounds become saved debug geometry, not physical collision. Mechanics
cards become typed properties for inspection and future behavior, not a claimed
dynamics simulation. Sound-program consumers are audited with this migration.

## Native implementation and focused evidence

The supported projection now emits a saved `level.tscn`, named perso scenes,
family/configuration-specialized `AnimationLibrary` resources and native
`AnimationPlayer` property/method tracks. Geometry/materials keep independent
shared identity. All channel/part alternatives and the selected initial pose
are saved before any node enters the tree. The preview supplies inspection
controls and a camera; the level owns its environment/fog and content.

The focused Godot regression verifies changing hierarchy, logical identity when
columns move, repeated logical IDs, full affine parent composition, shear,
configuration scale, opacity, forward/backward native seeking, reset configuration,
state speed, zero-speed states, entry/loop cues and cue-driven state changes.
Repeated IDs use a per-frame occurrence ordinal in addition to the logical ID;
this preserves the auxiliary columns present in 107 fixture clips. Their unknown
payloads remain explicitly unsupported. Text `Transform3D` serialization uses
matrix rows, unlike the GDScript `Basis` constructor's column arguments; the shear
regression caught and corrected this distinction.

Godot 4.7.2 folds a large `AnimationPlayer.advance()` to one loop and cannot
safely switch clips recursively inside an immediate method callback. Maintained
character support advances the **native Animation** at its method-key/loop
boundaries and defers callback-requested clip selection until that native advance
returns. Godot still evaluates every value/method track. No source matrices,
samples, hierarchy or object tables are shipped to this driver. Listeners see the
cue's pose; a state change cancels the remainder of the old tick. Inspection seeks
suppress effects. Configuration restore switches to its preconverted native
library without replaying cues or changing time.

Sound programs are compiled into direct GDScript branches/functions and typed
native voice/event resources. Native AudioStreamWAV resources carry waveform and
loop values. Support owns selectors, weighted/nonrepeat choice, native player
creation, pitch and event-targeted stopping. There is no sound-program JSON parser
or opcode dispatch loop. The focused real-mixer test retains Rete loop/gain edits,
material/default choice, nonrepeat selection, random pitch, targeted stop and
nonabortable voice behavior. Non-sound animation actions remain timed typed
signals, with explicit diagnostics rather than invented gameplay behavior.

Generated resources and script/import sidecars use deterministic path-derived
UIDs; saved node IDs use the owning scene and node path. File-local resource IDs
are normalized. The generated manifest owns `.gd.uid` and media `.import` files;
`.godot/` remains a rebuildable cache. The UID alphabet follows Godot's
[ResourceUID implementation](https://github.com/godotengine/godot/blob/master/core/io/resource_uid.cpp).
These provisional identities survive value edits; renaming source identity or
moving assets deliberately changes their identity and requires authored links to
be updated. Support `.uid` files remain committed with their authoritative source.

Conversion holds strong references to reused native resources while building;
otherwise repeated perso construction reparses libraries after temporary actors
are freed. Transform validation runs once per pool/clip, not once per sample.
Godot subprocesses retain bounded deadlines and abort excessive diagnostic output.
Complete-suite, editor and regeneration acceptance passed as recorded below.

## Original-engine comparison during migration

The available MCP harness ran the original English fixture in
`/home/stephen/Projects/hype-harness/local/runs/20260913-033203`, PID 3818473,
starttime 4697129. Executable SHA256:
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
The English SaveGame0 archive SHA256 is
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
It loaded to phase 9/manoir, then navigated to Astrolabe. Level navigation was
the only memory intervention; no asset data was patched.

Fresh read-only traversal resolved hero SO `0x03bfa678`, Perso `0x02110698`
and Perso3dData `0x021106e8`. A 700ms Up probe reached walking state 32:
state pointer `0x02134a3c`, animation `0x02134a80`, 29 frames, 30 FPS,
23 columns. Its cue array at `0x0213829c` has frame 0/column 22/slot 480 and
frame 14/column 22/slot 479, matching canonical `fix:/animations/941` and the
native walking regression. Runtime frame 13 had next-cue cursor 1. A later
1100ms hold changed state, so that sample does not establish the late walking
cursor; prior late-cursor evidence remains separately documented in
[scene-animation-research.md](scene-animation-research.md#original-walking-cue-comparison).
The harness screenshot is `screen-1789285255180620959.png` in that external run.
The game was stopped after comparison; controller/evidence remain outside this
repository. This validates source timing/cue structure, not generated-package
acceptance or waveform/pixel parity.

## Remaining semantic limits

The migration preserves the existing useful slice; milestones 6–7 remain partial.
Morph/related data, lip sync, engine interpolation, root motion, automatic gameplay
visibility, AI transitions, full physics, ambient routing/propagation, advanced
sprite/LOD behavior and full scene/package reconstruction remain open. Light and
reserved animation channels retain transforms but do not gain invented render
payloads. Family bounds are hidden native debug meshes, not physics. Mechanics
resources expose promoted parameters without claiming a dynamics simulation.
Native discrete sampling intentionally preserves established semantics. Original
engine acceptance of reconstructed packages and exact pre-compression package
parity remain later gates.

Actual editor probe: `tests/native_editor_check.gd` ran as a disposable
EditorPlugin under Godot 4.7.2 on an isolated X display. EditorInterface opened
Hype's saved scene, selected its AnimationPlayer, inspected `state_32`, sought
forward to frame 14 and backward to frame 0. It verified **634 persisted
descendants, 778 normal/reset clips and 22 moving channels**, with
`Engine.is_editor_hint() == true` and no scene reconstruction. Local log:
`output/native-editor-check-verbose.log`. The initial headless editor attempt
exited unsuccessfully during startup; the isolated-display retry passed.

A real 29-frame/30-FPS walking regression additionally exposed decimal/ULP
rounding at the native loop boundary. The support driver crosses the native loop
edge and snaps the native player to time zero without replaying methods, avoiding
both a stranded clock and accumulated drift. The synthetic suite now covers that
fractional case alongside whole loops and callback state changes.

The committed `godot/` project has now been rehydrated locally: **3,800 owned
files, 399 scenes and about 291 MB** of generated content. Source-derived assets
and caches remain ignored. `output/native-opening.png` renders the saved level
with the existing recorded camera/FoV, state 47/frame 0 and the separately
measured visibility overrides 28,71,80,136. Those overrides are comparison inputs,
not baked converter defaults. The native image was visually inspected; scene
geometry, materials, placement and the initial character are present. Compared
with the previous fog-enabled Godot capture, full-image normalized RMSE is
0.00682944 (about 1.74 eight-bit levels). Small raster differences remain; this
is not a claim of pixel-perfect or gameplay parity. The report and exact image
hashes are in `output/native-opening-comparison/report.json`.

`output/native-authored-variation.png` renders the committed inherited
`game/characters/luminous_hype.tscn`: the independently authored beacon mesh,
blue emission material, light and scale surround the actual native Hype scene.
The disposable renderer adds only a camera and lighting. No source visibility
overrides are used in this variation; gameplay-driven equipment visibility is
still a documented gap. The fixture-wide pre-ready check reports **150 source
nodes, 108 persos, 899 persisted part alternatives, 989 clips and 23 libraries**.


## Accepted checkpoint

The complete local suite passed **83 tests in 364.727 seconds**, including
compilation (warm build), all source corpus/reconstruction tests, native Godot
checks, direct/reopened-Rete conversion, deliberate regeneration and cleanup.
Report: `TestResults/20260913T081826.069430Z-4061873.log.json`.
Peak child RSS was 1,238,060 KiB, child CPU time 361.435 seconds; no processes
leaked and the single 570-second deadline was not exceeded. The supervisor now
collects all test-target failures with `--no-fail-fast`, without extending its
one deadline. Strict Clippy and formatting passed.

The scene test converts directly and from a reopened Rete store with identical
owned-file hashes. It retains the existing source reconstruction and deliberate
shared-geometry edit. Changing Hype's current Rete state from 47 to 32 then
regenerating updates the native initial pose and inherited authored variation,
while preserving authored scene bytes, project settings, resource/node identities
and native extension references. An explicit support update and removal/rebuild
of `.godot/` still pass the same scene check. The actual editor then opens the
same regenerated Hype scene and scrubs the native walking clip. Ownership tests
also retain unselected level closures and reject shared-resource conflicts,
authored collisions, modified generated resources and modified support updates.

The first full attempt failed audio-harness cleanup; stopped AudioServer voices
must retire before removing the temporary mixer bus. A later attempt exposed a
stale preview-wrapper path in the fog test. Both were fixed and are covered by
the accepted full run. An intermediate run was intentionally interrupted after
acceptance-test source changed; it is not counted as a passing suite.
