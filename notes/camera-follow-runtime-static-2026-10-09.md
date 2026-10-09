# Camera follow and session handoff — 2026-10-09

Worker branch `camera/follow-runtime-20261009`, isolated worktree
`output/worktrees/follow-camera-runtime-20261009`, baseline
`12ad58ce49d60b10cdcbb80a29c9118a5e9feffa` (Rete62/support84).
This is a source-backed implementation and static handoff. No Godot process,
native import, compilation, gameplay probe or runtime test was launched here.
Kip/integration owns the runtime slot and publication to the actual `godot/` game.

## Verified diagnosis

Baseline Load Game unconditionally calls DemoCamera `follow()` and
`make_current()` after restoring the save. That is the authored fixed-distance
presentation camera. New Game instead resolves the named source actor
`level:/persos/world/0/2`, normally presents its `SourceCamera`, and relies on
compiled source camera AI for cinematic/target/mode transitions. Its missing
camera branch previously selected DemoCamera too. These are static findings;
neither route's current runtime acceptance has been established by this worker.

The source primary owner is a viewport weak reference independent of the
viewport's currently rendered Camera3D. Changing presentation must not assign a
new target, select a different source preset or invent a camera pose.

Baseline mode0 computed an eye request and collision candidates but returned
without the ordinary Type3 frame translation/basis application. Only its special
0051c7b0 teleport gate wrote translation. This omission predates the primitive
mesh BVH regression fixed by baseline12ad58ce; the indexed sector/BVH and inactive
bank improvements are preserved.

## Source evidence and native implementation

The original engine/decompilation is authoritative. Read-only sources in
`/home/stephen/Projects/hype-decomp/src/camera` and `src/dynamics` supplied
005200a0,0051db70,00528030,0057910c,00578111,005796ea and0057b17f.
The dedicated Hype source worker supplied full instruction-backed contracts for
004444d0,00443f00,00578588,0057ab7d and0051f670. Its source-body commit is
`9d9544da15c265ed5070bd7af38e891ea1e93f23`; documentation follow-up4052b7b
corrects the retry gate, and65cd521041528730247e7ded22f8adac29c4949f
documents00578852/0059d3bd/0059d7fa. Read-only handoffs are
`research/worker-deltas/camera-motion-task2-handoff.md` and
`camera-motion-task2-focus-radius-contract.md` in its isolated
`/home/stephen/Documents/Codex/2026-10-09/task-2/hype-camera-motion` worktree.
No Hype source or reference submodule was edited here.

Camera scalar getters expose the retained +68/+6c speed limits,+70 linear
divisor,+74 response distance,+78 angular rate,+7c angular divisor,+80 angular
response,+84 angle rate and +50/+54/+58 view angles. Camera+ca is a separate
signed16 retry count. No persisted model/schema fields or virtual addresses
were added. Export uses parameter1 for active framing/registration and parameter0
for the reset resource and linear divisor; `_ready()` preserves the active mode.
Both resources explicitly mark their converted follow controls present.

Actor history now publishes eight newest-first positions, motion vectors and
seconds weights through named mechanics properties. Walk0/4 and Mass9 motion is
post-contact native velocity in units/sec. Type3 camera motion is committed frame
displacement. Neither is synthesized from position differences or divided by
time. A null target motion binding gives the source's zero motion result.
The first weighted query initializes zero-weight history with the source .03
weights, zero motion and target translation; raw inspection remains separate.
Existing native unsupported-publisher and publisher-change history boundaries
remain explicit; mixed mechanics-history parity is not accepted here.

Mode0 now distinguishes ordinary active-window motion from zero-window
observation candidates. The collision controller retains the most recently
successful ray geometry across clear queries. Its late guard overwrites the
observation candidates but preserves the motion already computed. The follow
consumer applies weighted target speed, distance/acceleration limits, horizontal
target-radius correction, angular response and view-angle offsets to the owning
actor's native global transform. The child half-turn converts actor+Z viewing to
Camera3D-Z viewing. Actual committed displacement supplies both Type3 history and
the next linear acceleration bound. Rejected rotation inputs cannot advance the
follow history; temporary recovery/input action flags cannot leak from a failed
bound update into the next frame.

The retry gate remains parameter94 mask0x0008, not0x0800. At the source default
retry total60, signed count40 already passes the second x87 2/3 test. Native
speed boosts preserve that boundary. Translation0x4, contact-query0x10,
angular0x400 and direct angular cap0x800 remain distinct.

## Session integration

The camera-only `game_session.gd` commit removes the Load Game DemoCamera
target/follow/current calls. `restore_source_camera_presentation()` obtains the
existing source owner in O(1), validates level ancestry and registration/context1,
makes it current and verifies the rendered viewport camera. It does not assign
target, mode or pose. Missing source ownership fails readiness explicitly.
New Game requires its named opening camera and the same registered owner rather
than selecting a fixed-distance substitute. Crossbow ray projection uses the
currently rendered Camera3D.

Compose this with save worker `bc2d8750971294fd43f97448f9f4d451f04190c7`:
keep its all-Brain hold, fixture/explicit selected save restore and two contact
frames. Immediately after `stage("physics_ready",...)` and before
`startup.start_brains()`, call
`if not restore_source_camera_presentation(false): return`.
Immediately after the awaited process frame and tree guard, call
`if not restore_source_camera_presentation(): return`.
The first call may defer absent registration; the final call requires the actual
selected source owner. Save selection, restoration and cinematic transition
logic remain with their respective workers. Shared character edits are limited
to camera history/publishers; loading's bank/configuration/material lifecycle
edits must be retained when integrating.

## Validation and acceptance still required

Owned Rust files pass `rustfmt --edition 2024 --check`; `git diff --check` passes.
The whole-tree formatter reports pre-existing differences outside this scope.
A separate Sol worker performed a read-only review of offsets, GDScript,
session ownership and prepared tests. Its frame-action/history findings were
fixed. These static checks are not compilation or gameplay acceptance.

Prepared checks in `tests/cameras.rs`, `check_camera_projection.gd` and
`check_camera_native_semantics.gd` cover active/reset distinction, exported
controls, motion units, weighted sampling, source null motion, follow response,
radius correction, failed-frame retry and concrete ordinary actor translation
plus rendered basis. They have not been run. Integration should run parser
preflight before conversion and these bounded probes within its runtime slot.
The support interface must be bumped as part of the combined integration, then
normal camera/mechanics scene regeneration must populate the new properties.
Do not run this support against stale generated cameras lacking controls.

Required actual-game observations for both Load Game and New Game after the
cinematic: rendered Camera3D path, semantic source owner path, selected target,
camera mode, camera Brain running/failure, actor global transform before/after
player movement and moving-target samples. Check that Hype is by the lectern in
the selected post-gate save, and retain the sideways gate behavior. These
observations belong to the single canonical reproduction; this worker has not
claimed that either route is playable.

Remaining conditional gaps fail explicitly: Type3 multi-contact slide response,
the curved-history/environment translation branches, target-roll parameter200
and0051ec70 offset recovery. Native basis arithmetic is not an x87 bit-for-bit
reproduction; near-vertical source side-axis continuity and mixed mechanics
history need separate acceptance. No push, source-disc/save/settings mutation,
generated asset tree copy or system change was made.
