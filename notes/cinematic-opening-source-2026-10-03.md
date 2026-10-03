# In-engine manor opening — source checkpoint, 2026-10-03

## Original shot-command slice, v8 prepared; native validation pending

Validated aiming is checkpointed as `b436f7c14cf4b68965b5503c7c60924d5b283684`.
The next commands come from `IWorld_Manoir` model 154, its actual initial normal
behavior 2. In the first-visit branch (`Global.dsg_147 == 0`), source nodes
909–947 select helper 33/waypoint 86, wait 3600 ms, select helper 18/waypoint 84,
wait 4500 ms, then select helper 19/waypoint 82 and change to behavior 3.
Each cut calls procedure 160, Ultra(camera, procedure 253(function 3(waypoint)))
and procedure 157(camera, 3), in that order. Controller 334 is a later load
trigger gated on World behavior 0, not evidence of the first opening driver.

Support interface 44 implements these calls through native target references,
global position and scoped 0/3 mode transitions. Script-only waypoints now use
the same promoted 28-byte decoder and meaningful asset identity as route points.
Godot resolves fixed positions to vector constants during conversion; relative
bindings fail when reached. Source keyword target serialization remains byte
compatible, so Rete schema remains 37. The scoped mode handler also supports
equal-state requests and -1 restoring the initial preset, without viewport
activation or follow/obstruction/reset internals.

Source/Rete checks confirm all three waypoint positions match the original
captured eyes within 0.00005 units and that the full source model and waypoint
assets survive save/reload; the camera stays in its original mode 0. All 33
partial-AI codec checks pass, including exact reconstruction of target, Ultra
placement, waypoint and state nodes and unresolved-binding preservation. The
keyword camera compiler regression also passes. Owned preflight evidence is
`TestResults/manor-source-shots-v8-{codec,source,target}-preflight-sol-20261003`
(`.log` and `.json`). The codec reporting wrapper raised an AttributeError
after its supervised child passed; its saved supervisor result records exit 0,
41.191 seconds and clean cleanup. Later wrappers use the actual Result fields.

The next Luna case deliberately extracts the last 12 unchanged statements from
that source branch into the temporary World schedule. `fixture.json` records
both full-source and excerpt hashes and `ai_modified: true`; camera model 437
remains unchanged. Earlier initialization/branch guards and other Brains are
held. Native physics frames drive the existing Brain clock/coroutine, and the
original camera rule aims each selected shot. Assertions alone contain the
recorded eyes and helpers; the harness does not assign camera eye/target/mode.
Viewport activation remains setup. Three cuts at 0/3600/8100 ms and transition
to behavior 3 are the bounded hypothesis, not full original opening acceptance.
Syntax preflight precedes conversion; gameplay has a 25-second deadline. Native
syntax, behavior and rendered evidence are still pending. Full initialization,
remaining shots, dialogue/presentation and the approximately 209-second handoff
remain open. No original script-memory edits were needed for this slice.

## Native mode-3 aiming acceptance, v7

Support interface 43 exports the active mode and actor/look offsets into native
properties. Mode 3 aims the meaningful camera actor with `Node3D.look_at` from
its current eye toward the selected actor plus rotated offsets; zero look offset
uses source [0,-0.05,0]. The actual compiled camera rule requests each update.
Other motion modes and original shot commands remain unsupported.

Luna's v7 run passed all 25 Rust cases (21 mandatory safeguards and four camera
cases) and four Python modules in 240.348 seconds. The actual manor source is
imported, saved/reopened in Rete and exported normally. The full camera model 437
is unchanged (Rete hash `02d0dd557e54b068a088ad3f5e8a591ea57b143ff0d694876944d030f5141442`);
the harness invokes only its original `normal_0_rule_0`. Four fixed eye/helper
fixtures reproduce observed forward/up within 0.000006 normalized-vector error.
There are 45 native eye-animation observations, maximum aim error 0.00000121,
and 2.29133 units displacement. Guarded unsupported mode and absent, coincident
or vertical target calls leave the pose intact. No new OpenSpace AI is authored.

The file starts in mode 0; mode 3 is an explicit temporary Rete fixture edit.
Recorded eye placements, helper targets, viewport selection and subsequent native
AnimationPlayer motion are declared harness setup. Other Brains are held. This
does not prove source ChangeState, shot commands, timing, viewport activation,
follow/obstruction or the approximately 209-second intro handoff.

Sol inspected all five 640×480 rendered PNGs. Courtyard/tree framing, the shield
close-up and statue-base views are recognizable. The native initial scene has
coloured/overlapping characters where the original presents a grey statue;
lighting/fade and letterbox also differ. The original final opening screenshot
shows Gogoud in the foreground, which this held-Brain fixture does not reproduce.
The motion endpoint image changes the courtyard perspective without an empty
frame or displaced target. These observations support usable native camera
geometry, not pixel or opening-presentation parity.

Source remained unchanged and cleanup completed. No GPU/device-loss markers
appeared in assigned logs. External coredump status remains unverified because
system-bus access was unavailable. Evidence:
`TestResults/manor-camera-aim-selection-v7-sol-20261003.log.json` and
`.log.d/manor-native-camera-rule-1791035598475331705/` (`fixture.json`,
`report.json`, five PNGs and owned logs). Earlier source/framing checkpoints
below retain their historical limitations.

## Camera framing checkpoint after fresh v3 capture

Luna's fresh `TestResults/manor-intro-camera-source-v3-sol-20261003/`
capture passed (exit 0, 24.614 seconds, cleanup complete): 80 observations,
69 camera captures, zero camera errors, one Return at the verified New Game
menu, and `manoir` phase 9. No source/save edits or gameplay interventions were
used. The camera actor's position agrees with the active viewport within
0.00002 source units. Its negative local Y axis agrees with the corrected
viewport forward. Source clock/update span 670/3 to 13814/390; screenshots
remain independent captures, without a calibrated render/update association.

The four 164-byte parameter records are now useful canonical camera fields,
attached to the Perso with their pool-scoped identities. The 220-byte component
is Fix allocation 23339; its parameter records are allocations 23340–23343.
The demonstrated records have no relocated pointers. Unknown parameter scalar
bytes (104 per record) and component scalar bytes (196, excluding its six
declared pointer words) remain explicit MessagePack byte values. Parameters
are embedded because these demonstrated allocations belong to their camera;
independently shared presets would need asset references. This ownership is
provisional, not a general engine record catalog.

The older 224/168-byte diagnostic extents included the following allocator
header. Engine size getters 00440050/00440060 return 220/164, matching the
file pool prefix minus its four-byte header. The new codec and checked live
record reader exclude those allocator words; they are layout metadata, not
unknown camera content. Raw v3 evidence is preserved with its original extents.

The declaration and codec connect the named fields to getters: target/look
offsets at +0x0c/+0x18 (00440ac0/00440af0), shared actor-relative offset
at +0x24 (00440b20 and rotation/addition in 0051c8d0), distance limits +0x30/+0x34
(00440b50/00440b70), yaw/pitch +0x40/+0x48 (00440bd0/00440c10), mode +0x98
(00440f40) and horizontal FoV +0x8c (00440f10). Viewport publication 0043dbd0
gets FoV from parameter set 1 and passes it to GLI camera aspect adjustment.
GLI creation/setters 00485000/004852b0 identify clipping at +0x6c/+0x70;
the source renderer link supplies 0.25/10000. Its derived view matrix and
renderer handles are not imported as replayable camera layout.

The initial framing has distance [3.5,4], yaw 0, pitch 0.3, horizontal FoV 1.5
radians and mode 0. The v3 live initial parameter set instead has mode 3.
These modes are preserved, but neither mode's motion is accepted yet. The actor-relative shift is [0,0,1.8]. Together with pitch 0.3 and ranges
3.5 and 4, it explains the earlier saved-game idle and settled-walk offsets:
behind = distance × cos(0.3), above = 1.8 + distance × sin(0.3). This is a
source-grounded geometric explanation, not a dynamic follow/obstruction pass. The
file component's target points outside the selected rooted manor hierarchy
and does not decode as a valid Perso node; it cannot supply a portable initial
target binding. Source initialization 00440820 sets the current main actor,
while the v3 opening capture shows a different target locator from the sampled
Hype and Gogoud. The next checked Orrery capture includes that target actor's
identity and pose in the camera's existing pause. It uses the declared target
link and existing checked Perso/transform readers, with no guessed opaque walk.

Rete schema is now 37, replacing the former camera-presence flag with the
component. Godot support interface 42 saves Camera3D with the source horizontal
FoV, clipping and a local Y half-turn: converted source camera -Y becomes the
actor's +Z, whereas Camera3D looks along -Z. `KEEP_WIDTH` preserves the horizontal
projection ([Godot 4.7 Camera3D documentation](https://docs.godotengine.org/en/4.7/classes/class_camera3d.html)).
The maintained native update still explicitly returns unsupported. No target
motion, smoothing, obstruction, viewport selection or intro handoff pass is
claimed. This does not reuse the saved-game demo's measured fixed boom as the
converted source camera.

Sol's source-only ownership/reconstruction test passes: all four records rebuild
byte-for-byte, deliberate target-offset/FoV edits rebuild changed values while
preserving all other bytes. A new native test loads the normally exported saved
scene after a Rete FoV edit, checking view direction, clipping and projection
matrix independently of preview bootstrap. Luna's v5 run accepted the source/Rete
camera cases (26/30 total), but four support/native cases failed on the stale
bundled support manifest (41 versus Rust interface 42). After synchronizing the
maintained manifest, v6 passed all 22 selected Rust cases and four Python modules
in 116.292 seconds, including all 21 mandatory safeguards and the saved native
camera projection. Source stayed unchanged and cleanup was complete. These are
component/regression passes; no Sol game run occurred. Evidence is
`TestResults/manor-camera-framing-selection-v6-sol-20261003.log.json`.

The v6 original capture passed in 24.039 seconds with 80 observations, 69 camera
captures and zero errors at manor phase 9. The checked targets are helper Persos
of family 4/type 39: instances 106, 92, 93 and 94, distinct from Hype and Gogoud.
Target world position plus its rotated actor offset [0,0,1.8] and rotated look
offset (zero defaults to [0,-0.05,0]) explains all sampled view directions with
residual below 0.00003 source units. This corroborates helpers 0051c8d0/0051d140
using same-pause target/viewport observations, rather than target origin. All
sampled targets have identity rotation; rotated-target behavior still needs
native/source-informed tests. The capture shows fixed eyes separated by cuts
near source clock 4.3, 9.3 and 12.4 seconds, not a continuously moving shot.
Mode 3 retains the eye while aiming at the selected helper; source 005200a0
case 3 only requests eye motion for explicit-position flag 0x40, and 0051e620
suppresses ordinary follow motion for mode 3. This supports a focused native
look-at component, not a port of controller scratch records or scheduler.

Raw evidence: `TestResults/manor-camera-target-source-v6-sol-20261003/report.json`.
Offset derivation: `TestResults/manor-camera-target-v6-offset-analysis-sol-20261003.json`.
No GPU/device-loss markers were reported in the assigned logs; that does not
prove absence of external coredumps. Save archive hash remained unchanged.

This checkpoint concerns the in-engine New Game opening. AVI playback is outside
this task. It establishes source route structure and an original opening-camera
baseline and a validated native Gogoud route/action component; it does **not**
establish a working complete Godot intro or exact route/camera timing parity.
Crossbow results remain in the separate October 2 notes and commits.

## Implemented source inspection

`openspace::level::inspect_actor_ways` provides bounded, typed transient
observations of a rooted actor's route slots. It uses declarative binrw records,
the existing source relocation/session layer and pool-scoped source identities.
It checks list ownership, previous/tail links, cycles, count bounds and finite
point geometry. Empty slots retain their index. Unknown waypoint/link bytes and
unpromoted link-geometry references remain explicit. StaticLine geometry is now
promoted separately below. The inspection remains a transient diagnostic; the
canonical asset graph owns persisted routes and shared waypoints.

Raymap's Hype AI catalog was the practical starting point. Its `MSWay` parser
is for another engine layout and is not used as the Hype contract. The engine
provides the relevant declarations:

- `00522040` evaluates AI node category 23 as an actor-local byte index.
- `0043e370`/`0043e340` resolve that index through Perso+0x20, the component's
  first pointer, and an actor-local intrusive route list.
- `00459270`/`00459290` define the 12-byte list; `004591a0`/`00459250` define
  the 20-byte binding, route pointer at +12 and four actor-local progress bytes.
- `005177e0` and `00518f30..00518fc0` define the 12-byte Way.
  `00517980` allocates **N** point references and **N-1** link references.
- `00516990` defines a 28-byte waypoint, including its final four bytes;
  Raymap reads only the first 24. `005173f0` resolves an optional actor-relative
  point through that actor's world transform.
- `005260c0` defines the 72-byte link. The 52-byte DynamicObject and non-StaticLine
  geometry remain unpromoted.

Gogoud is `level:/persos/world/0/5`, source name `iGogood_manoir`, family 27,
model `level:/ai/models/210`. Its seven route slots include empty slots 1–3.
Slot 0 closes a loop by repeating the same shared waypoint identity. Opening
slot **6** has two fixed points, both radius **1**:

| Point | Source X | Source Y | Source Z |
| --- | ---: | ---: | ---: |
| 0 | -28.26834297 | 148.91851807 | 33.17883301 |
| 1 | -18.16604805 | 149.11961365 | 32.88652039 |

The single link has connection 0 (StaticLine), dynamic kind 0 and length
10.10852337. This is source geometry, not a measured live trajectory.
Current typed evidence is
`TestResults/cinematic-route-inspection-sol-20261003/source-routes.json`.

The inventoried local test
`openspace::level::way::tests::source_gogoud_opening_route_and_records`
checks these meaningful values, empty/shared slots and reconstructed bytes for
all Gogoud route records. It also deliberately edits a waypoint X coordinate
and verifies the reconstructed value and preservation of the other bytes.

## Opening AI and wind

Gogoud normal 13 resets slot 0 with `Proc_ReInitWay` (procedure 220), selects
action 0, then changes to normal 0. Normal 0 starts with
`Action_GoOverWay(Way(6))` (meta-action 27), followed by native actor actions,
turning and dialogue. Engine entry `0054b260` selects the actor's route;
update `0054c8d0` delegates to `0054c800`, which obtains the next point/radius
and calls `0054a960`. `0054a9f0` supplies a normalized direction to actor
mechanics until the point is reached. This is a route-driven actor behavior,
not evidence for a separate Hype cinematic scheduler. Nonzero link speed is
not inferred: this update calculates and discards `WP_fnx_Way_GetCurrentSpeed`.

Hype model `fix:/ai/models/28`, normal 0 **schedule**, source node 9, calls
`Proc_AddWind(Vector(0,0,0))` (procedure 30). The pinned executable's handler
at `005404d0`, opcode-30 branch `00540782`, **assigns** the three components
at actor mechanics-control offsets +0x88/+0x8c/+0x90, despite the name AddWind.
`0052ec40` resolves this actor-local block through the actor's dynamics record;
it does not resolve a global weather setting. Consumer `00589bfa` adds the
stored vector to its calculated actor linear velocity, after its relative
motion calculation. Zero therefore clears this extra drift. This path shows
no direct particle or animation update. Other consumers, units and persistence
outside this mechanics callback are not established here. The following
`Proc_SetParticleGenerator` (194, node 14) is a separate unsupported call with
an unpromoted generator reference; it must not be treated as the wind effect.

The source `StdCamer`, `level:/persos/world/0/2`, uses `fix:/ai/models/437`.
Its first rule calls procedure 135, camera position update. Maintained
`source_camera.gd` explicitly returns unsupported for that update. Camera
geometry/update semantics and later dialogue remain unimplemented; bypassing
them would not demonstrate the intro. Route reset/GoOverWay implementation is
and its native component acceptance are described below.

## Original capture and execution limit

An earlier bounded isolated New Game probe reached `manoir`, phase **9**, and
retained 70 MainActor/active-camera observations. The prepared base's
`PlayVideo=0` was copied unchanged. It pressed Return at the main menu and a
second Return while entering the scene, so it is an entry diagnostic, not a
verified full intro input recording. The current example removes that second
press and adds typed Gogoud observations; that revised runtime path subsequently succeeded, as recorded below.

The earlier first observation had source clock 1130 ms/update 25, Hype action
47, and active camera position `[-16.69891357,155.74366760,33.20948029]`.
Projection was horizontal FoV 1.5 rad, vertical FoV 1.21970236 rad, near 0.25,
far 10000. Later observations show camera movement. Pose words are retained;
the derived forward vector is not accepted here as full orientation evidence.
Screenshots are independently captured, and these stopped-byte observations
do not assert synchronized completed-update/render association. Gogoud's live
AI/action/trajectory was not captured by that version.

Evidence: `TestResults/cinematic-entry-source-sol-20261003/report.json`, sibling
`.log`/`.supervisor.json` and the report's `runs/` screenshots. Execution took
21.526589 s, with no timeout, leaked child or remaining process, and 0.003653 s
cleanup. Executable SHA-256 was
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
The supplied save archive was unchanged before/after:
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.

Automatic approval review rejected the revised isolated New Game/actor capture,
first as outside Crossbow scope and again because it did not accept the
delegated opening-milestone instruction as trusted scope expansion. No rejected
run was executed or bypassed. A third reviewed retry was interrupted before
launch; a subsequent process check found no game/probe/bridge session and no
new runtime evidence directory. The user then directly approved the isolated
capture and specified **Luna** as its executor. Sol must not run that capture.
Its fixed supervised command is:

```sh
python3 TestResults/run-cinematic-entry-source-20261003.py
```

It has a 100-second supervisor budget, uses maintained Orrery and a copy-on-write
installation/save snapshot, and retains the prepared video policy. Luna must
use normal approval review with no concurrent original-game worker. The exact
assignment, unchanged approval evidence and fixed named-field extractor are in
`TestResults/cinematic-worker-opening-capture-20261003.md`. Luna's durable-thread
tools were unavailable here; the parent will relay it. No dispatch is claimed.

The initial source-route corpus check passed in 0.33 s; the final test adds a
deliberate point edit and passed the frozen 22-case safeguarded selection
executed by Luna. Current `cargo fmt --check`,
`cargo clippy --locked --offline --all-targets -- -D warnings` and the diagnostic
example build passed. Those prior compile checks precede the new route implementation.

Read-only source inspection can run independently and launches no engine:

```sh
cargo build --locked --offline --profile validation -p behavior-suite --example cinematic_entry_probe
target/validation/examples/cinematic_entry_probe TestResults/cinematic-route-inspection-sol-20261003 --source-only
```

## Original opening capture completed

Luna executed the revised one-Return recipe in the isolated installation. It
retained 80 frames, including 69 Hype and 69 Gogoud observations, with zero
inspection errors. Capture elapsed 22.872 s (cleanup 0.006 s), with no timeouts,
leaks or remaining processes; the supplied archive hash was unchanged. The
original-game session is released. The frozen source-inspector selection passed
22/22 in 95.339 s and offline release compilation passed in 44.24 s. These
results predate the native route changes below and do not validate them.

Evidence is `TestResults/cinematic-entry-source-actors-sol-20261003/`, including
`report.json`, `facts.json`, `compact-observations.json`, screenshots and retained
isolated files. Sol inspected both screenshots: the first shows the dark statue
shot, the last a bright Gogoud foreground shot beside the castle with the statue
behind him. They show a scripted in-engine sequence, not a player follow camera.

Gogoud changes from initial normal 13/action 18 to normal 0/action 0. His source
position advances from [-28.410318,148.635834,32.940781] at clock 666 ms to action
26 near clock 8587 ms, then action 24 near 11112 ms and action 23 near 12140 ms.
He settles at [-18.638809,149.097321,32.910141]. Hype remains action 47 and changes
from normal 0 to normal 48 by the next retained observation. Active camera
positions change during the sequence; the first remains
[-16.698914,155.743668,33.209480]. Source observations are separately stopped
captures, not a synchronized completed-update/render association.

## Implemented opening slice — verification pending

The new `model::route` API makes reusable Routes and independently shared
Waypoints ordinary assets. Actor bindings retain local slot order, null slots
and minimal progress bytes. Repeated/shared waypoint identities refer to one
asset; conflicting values fail import. Routes and waypoints have role URIs with
pool-scoped source identity, geometry and explicit unknown fields. No virtual
addresses, cached StaticLine world-position buffer or relocation layout is saved.
Rete schema **36** persists these values as MessagePack assets. This replaces the
provisional schema rather than maintaining backward compatibility.

Engine `00530e50` allocates the eight-byte StaticLine object: waypoint and
resolved world-position references. `00530ff0` derives its 12-byte position cache;
`00526cc0` reads that position. Declarative binrw now promotes this object before
following its pointers. Fixed endpoint caches are checked against their waypoint
and then discarded; geometry is rebuilt from edited useful values. Other
connection geometry and the 52-byte DynamicObject remain explicit preservation
placeholders. OpenSpace route/waypoint export composition and relocation parity
remain pending; the native slice makes no whole-package reconstruction claim.

Godot support interface **41** saves native `Curve3D` resources with arrival radii
and loop closure. No sampled link/dynamics tables ship as reconstruction code.
Typed ResetRoute/GoOverRoute map procedure 220/meta-action 27 to actor-local native
progress and cancellable compiled coroutines. Engine `004436c0` supplies animation velocity magnitude, not signed forward
speed. Gogoud's walk uses source negative Y/native positive Z; route facing
follows the same source convention. The current action mechanics limits linear/angular rates, and native
CharacterBody3D contact moves the actor. Engine arrival callback `0054a9a0`
compares 3D distance <= radius; zero radius additionally accepts 0.25 before
snapping (`0054a9f0`). Only fixed StaticLine routes with dynamic kind zero are
currently runnable. Curved/dynamic/actor-relative calls fail explicitly when
reached. Native timing, contact and the actual Gogoud sequence are not yet passed.

Stephen explicitly requested `Proc_AddWind` as a **no-op plus documented technical
debt**. Typed AddWind now round-trips procedure 30 through OpenSpace/Rete and emits
`source_add_wind(Vector3)`. It evaluates its operand and accepts finite vectors,
then changes no motion state. This lets the zero call advance without claiming
wind physics parity. Hype's actual semantics assign actor-local world drift and
add it to linear velocity; nonzero drift and resetting prior drift are deferred.
Follow-up must introduce a separate drift property, verify units/consumers and
exercise assignment, overwrite, zero reset and contact. The following source
particle-generator call at node 14 remains unsupported and visible.

New inventoried checks cover source-node reconstruction, an edited Rete waypoint
through native route/contact/cancellation/shared progress, and the actual exported
Gogoud sequence [0,26,24,23] in manor. The latter holds other Brains to isolate
Gogoud and compares his ending pose within 0.75 source units; it is explicitly a
component regression, not full intro acceptance. Camera code and the authored
Load Game bootstrap are unchanged. Camera update, Hype's remaining initialization
and later dialogue still block a complete New Game opening.

## First native result and collision correction

Luna's fixed 32-case selection passed **31/32** in 398.526 seconds, including all
21 mandatory safeguards, the synthetic route/contact/shared-progress regression,
source record/AI reconstruction and exported Crossbow expiry. All four Python
checks passed; cleanup completed with no timeout/leak and an unchanged source
digest. The actual Gogoud case failed after 129.568 seconds: actions remained
`[0]`, with no AI failure and route point 1 still active. It ended near
[-23.797615,149.416687,32.917122]. This is a failed opening gate, not acceptance.
Result: `TestResults/cinematic-worker-result-native-routes-20261003.md`.
Trace: `TestResults/cinematic-native-routes-selection-20261003.log.d/gogoud-native-opening-1790993016286310187.json`.

Sol compared that trace with the preserved source geometry. Initial native speed
is approximately 1.24 units/second, consistent with the original walk, before
contact near X -23.8. The blocking fence is `world/1/9/0`, under `sctjardinvirt`.
Its two-sided faces run near X -23.3, Z 32.90..33.52: a sphere of radius 0.5
stops near X -23.8. Static triangle evidence is retained in
`TestResults/gogoud-opening-stall-static-triangles-20261003.json`.

This sector is virtual, has no physical boundary and is absent from every source
collision-neighbor list. Gogoud starts in physical sector `world/1/15`; its
neighbors are `world/1/16`, `world/1/19`, `world/1/26`. Source `00448280` updates
the actor's current sector. `00464570` visits that sector and its explicit collision
neighbors; `00464150` traverses those subtrees for contact. The native exporter
had instead installed every stored geometry as a global blocker, including
unreachable display-sector geometry. A step-up workaround is inappropriate:
the original trace crosses this X coordinate without climbing the fence.

Conversion now marks shapes in unreferenced virtual sectors with collision
layer/mask zero, preserving their native resources and visible meshes. Physical
sectors and explicitly referenced virtual collision sectors remain contact bodies.
This is an offline conversion of already preserved relationships, with no source
sector scheduler or table interpreter. It removes globally unreachable blockers;
the current native contact world still uses the union of eligible sectors. Exact
per-actor neighbor filtering and dynamic sector activation remain pending.

The synthetic route regression now uses converted floor/wall assets, checks
blocking by a listed virtual sector, cancellation, and traversal through an
unlisted display-sector wall before arrival. The real Gogoud fixture checks the
retained fence and records source node/material, point and normal for every
sampled slide contact. Its action and ending-pose assertions are unchanged.

After this correction, `cargo fmt --check`,
`cargo check --locked --offline --all-targets` (2.25 seconds) and
`cargo clippy --locked --offline --all-targets -- -D warnings` (2.53 seconds)
pass. Runtime verification is pending the corrected 34-case assignment
`TestResults/cinematic-worker-native-routes-collision-fix-20261003.md`.
No corrected actual-actor pass or full intro acceptance is claimed. Hype
initialization, source opening camera and dialogue remain subsequent gates.

### Contact reporter parse correction

Luna's corrected collision selection passed **33/34** in 380.038 seconds. The
synthetic listed/unlisted-sector contact regression and native scene checks
passed, as did all mandatory safeguards and four Python checks. Cleanup completed
with no timeout/leak and unchanged source. The actual Gogoud case failed before
gameplay after 104.715 seconds: the test reporter called
`shape_find_owner(get_collider_shape())`, passing an Object to an integer API.
No native JSON was written; the route and action sequence were unrun.
Result: `TestResults/cinematic-worker-result-native-routes-collision-fix-20261003.md`.

Sol corrected the reporter to `get_collider_shape_index()`, matching the maintained
character contact API. The Rust case now runs Godot `--check-only --script` in a
minimal temporary project before importing/exporting manor, so a harness parse
error fails before costly setup. No production motion/contact code changed in
this correction. `cargo check --locked --offline --all-targets` (2.01 seconds),
Clippy (0.15 seconds) and formatting pass; Godot parsing is still assigned to Luna.

The next fixed assignment first proves syntax in a supervised parse-only stage
with a ten-second process deadline, thirty-second total budget and owned cleanup.
Only after that passes may Luna run **22/262** inventoried cases: the actual
Gogoud case and all 21 mandatory safeguards, plus all four Python checks, with
the unchanged case deadlines and a 360-second suite budget. The 33 previously
passing collision-fix cases remain prior regression evidence rather than being
repeated without a code change. Assignment:
`TestResults/cinematic-worker-gogoud-reporter-fix-20261003.md`.

### Owned syntax-log correction

That first syntax-only command failed in 0.184 seconds, with Godot exiting zero
but reporting two errors opening/rotating `user://logs/godot.log`. The nameless
minimal project selected Godot's default user-data location outside the permitted
write roots. This was the test wrapper's setup mistake, not a new GDScript error
or an Orrery launch failure. The error gate correctly stopped all later checks;
neither syntax acceptance nor real-actor gameplay was claimed.
Evidence: `TestResults/cinematic-worker-result-gogoud-reporter-fix-20261003.md`.

The Rust preflight now passes `--log-file godot-parser.log`, relative to its owned
temporary project, requires that file and bounds/checks its contents as well as
captured stderr/stdout. The separate supervised preflight writes an explicit
absolute log under `TestResults`, retains both logs, requires creation and rejects
every `ERROR:` in either stream. Godot documents the path override in its
[command-line reference](https://docs.godotengine.org/en/stable/tutorials/editor/command_line_tutorial.html).
No logging disablement, error suppression, home-directory mutation or permission
escalation is needed. Original failure evidence is retained. The corrected fixed
assignment is `TestResults/cinematic-worker-gogoud-owned-log-20261003.md`;
its syntax stage is still 10 seconds with a 30-second total budget, followed only
on success by the same 22 cases, four Python checks and 360-second suite budget.

### Native Gogoud component accepted

Luna's owned-log preflight passed in **0.188 seconds**: both exits zero, no errors,
an explicit 101-byte engine log, unchanged source and complete temporary cleanup.
The subsequent selection passed **22/262** inventoried cases (240 explicitly
excluded) in **173.636 seconds** of its 360-second budget. Build/discovery took
1.076 seconds, four Python check modules 6.031 seconds, execution 166.421 seconds
and final cleanup 0.039 seconds. All 21 mandatory safeguards passed; no stage
timed out, leaked children or left a process. The real Gogoud case took 118.035
seconds including conversion and gameplay. Source was unchanged throughout.

The actual exported scene produced 218 native samples, with no AI failure:

| First sampled action | Native clock ms | Source-axis position |
| --- | ---: | --- |
| 0 | 133.333 | [-28.273355,148.642273,32.921593] |
| 26 | 7483.333 | [-19.121235,149.073685,32.893291] |
| 24 | 9933.333 | [-18.689968,149.094009,32.890007] |
| 23 | 10983.333 | [-18.689968,149.094009,32.890007] |

Route progress completes before action 26; actions 24 and 23 remain at the final
pose. All sampled actions are on the floor. The finished pose is **0.0550781**
source units from original [-18.638809,149.097321,32.910141], well within the
component's declared 0.75-unit tolerance. Contact samples identify source nodes
`world/1/15/0`, `world/1/26/0` and `world/1/13/0`; the finish has one upward ground
contact on `world/1/13/0`, material `level:/models/world/1/5/0/materials/7` (shared
geometry identity). The display fence under sector `world/1/9` remains as one
saved shape with collision layer zero. It no longer stops the native walk.

Commands:

```sh
python3 TestResults/run-cinematic-gogoud-owned-log-preflight-20261003.py
cargo fmt --check
python3 TestResults/run-cinematic-gogoud-owned-log-selection-20261003.py
python3 TestResults/extract-cinematic-gogoud-owned-log-facts-20261003.py
```

Evidence: `TestResults/cinematic-worker-result-gogoud-owned-log-20261003.md`,
`TestResults/cinematic-gogoud-owned-log-selection-20261003.log.json` and
`TestResults/cinematic-gogoud-owned-log-selection-20261003.log.d/gogoud-native-opening-1790996387842631501.json`.
The original trace remains in
`TestResults/cinematic-entry-source-actors-sol-20261003/report.json`.
Earlier 33 passing collision regressions remain part of this checkpoint's
evidence; they were not silently replaced by the narrower final selection.

This validates meaningful native movement/contact and the observed action order,
with other Brains held. It does not validate exact elapsed timing (the original
samples lack a calibrated common start/update boundary), full New Game startup,
camera, Hype initialization, dialogue, per-actor collision-neighbor filtering or
wind physics. Wind is still the explicitly authorized no-op. No new original
launch or full release matrix was run in this final check.

### Next camera boundary: correct observation before native movement

The validated route/wind/collision slice was locally checkpointed as `f72344d`.
Hype's next initialization call remains procedure 194, selecting an unpromoted
particle generator. Its following call disables emission. Assignment/emitter
semantics need useful assets and native behavior; no additional no-op is added.
The source camera's initial normal 0 first rule independently reaches procedure
135, which still reports unsupported in native support.

The previous viewport observation's `forward_source` was wrong: it selected a
row of the stored basis. Source `POS_fn_vMulMatrixVertexWithoutScale` at
`004b98d0` multiplies the stored matrix columns; `GLI_xProjection` at `004935b0`
accepts positive camera-space Z. An isometric camera's inverse-view rotation
therefore maps [0,0,1] to the third column of the transposed stored matrix. This
also agrees with `POS_fn_vInvertIsoMatrix` at `004b9e30`; camera publication at
`0043dbd0` remaps source camera axes and then inverts the pose for GLI.

Sol verified the relevant reference-decompilation code against the pinned live
executable (SHA-256 `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`).
Despite differing whole-executable hashes, these byte ranges are identical to
the decompiled patch reference:

| Address | Bytes | SHA-256 |
| --- | ---: | --- |
| 0043dbd0 | 256 | a4f6651e98e7f6376943696fee7081e63db3500692cd09165421f41e05363890 |
| 004b98d0 | 214 | 6eaf57024f44c3e2d037c2e73c99da3c35bff1d6f0d487253a5d0f9918e39528 |
| 004935b0 | 214 | 8948db89c0a9bc0cf2aa9a257e2134b412873125617d27313d277f1ed3011d5b |
| 004b9e30 | 614 | 306ed95edf024a0f78d915a6fb0535282aa11d3641a874387de5a855119f6d50 |

The decoder now derives inverse-view positive Z; position, raw pose words and
projection values are unchanged. Its existing synthetic test additionally checks
that a world point one unit along decoded forward projects to [0,0,1] through
the source matrix. The earlier third-row value instead projected along view -Y.
This is an observation fix at the OpenSpace boundary, not an Orrery lifecycle
change. Native camera parameters, target calculation and cinematic behavior are
still unmapped. The previous raw captures are retained without rewriting them.

The existing opening probe adds optional `--camera-records`, using Orrery's
`inspect_scene_camera("manoir", [2,2,2], ...)` unique checked search. It captures
the camera actor/Brain, selected fields 24,26,28,29,30,31,38,39,40,41, four linked
records and the active viewport during one pause, alongside the separate Hype
and Gogoud samples. It rejects a phase-9 level other than manor and requires at
least one camera capture with no camera-inspection errors, retaining its report
on failure. No original memory writes, skip input or base setting changes occur.
The linked words/addresses remain transient debug evidence and do not enter Rete.

Native test/game execution stays assigned to Luna. The next fixed assignment is
`TestResults/cinematic-worker-camera-source-20261003.md`: 24/262 cases (three
camera checks plus 21 mandatory safeguards), four Python checks, followed only
on success by the owned build/isolated opening capture. The selection budget is
300 seconds; two builds have 80-second deadlines and capture has 100 seconds
under a 280-second overall job budget. No original camera-record capture or
corrected native test pass is claimed yet.

Sol's `cargo fmt --check`, `cargo check --locked --offline --all-targets`
(2.47 seconds), and `cargo clippy --locked --offline --all-targets -- -D warnings`
(2.95 seconds) pass. Explicit probe compilation with
`cargo check --locked --offline -p behavior-suite --example cinematic_entry_probe`
also passes (6.59 seconds). Broader probe-package Clippy failed in the unchanged
Orrery `observe_magick_behavior` method at line 673 (`too_many_arguments`), outside
this camera change; no Magic refactor or lint suppression was added.

Read-only derivation from the preserved viewport words is retained in
`TestResults/cinematic-camera-direction-analysis-20261003.json`. Corrected
forward is [0.786991,0.555323,0.268816] at original clock 666 ms and
[-0.082333,0.996541,0.011291] at 12309 ms. Projecting a point one unit along
each corrected direction gives view [0,0,1] within 0.000011 units. This is a
calculation on existing evidence, not a new game run or calibrated render match.
