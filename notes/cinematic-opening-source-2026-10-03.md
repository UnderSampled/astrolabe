# In-engine manor opening — source checkpoint, 2026-10-03

Current acceptance uses the actual authored main-menu New Game entry and the
full imported AI through the unskipped
[original endpoint](./manor-intro-reference-2026-10-03.md). The excerpt/component
results below remain diagnostic evidence. The prepared fourth-cut fixture is
deferred; it cannot substitute for the normal opening and UI integration.

## Original shot-command slice, bounded v9 acceptance

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
Syntax preflight precedes conversion; gameplay has a 25-second deadline. Full initialization,
remaining shots, dialogue/presentation and the approximately 209-second handoff
remain open. No original script-memory edits were needed for this slice.

V8 completed with 28/29 Rust cases passing in 134.938 seconds, unchanged source
and clean cleanup. The new shot harness failed its early syntax preflight in
0.355 seconds: Godot could not infer `forward_error` and `up_error` through the
dynamic camera reference. Neither conversion nor shot gameplay ran. Explicit
float declarations correct those two lines; v9 retained the same 29-case scope
and budgets. Existing v8 logs and source checkpoint remain unchanged.

V9 passed 29/29 Rust cases and all four Python modules in 184.421 seconds,
with unchanged source `f33cf17` and complete cleanup. The excerpt case took
119.308 seconds including fresh conversion. Native cuts select helpers33/18/19
at clocks0/3600/8100ms, then enter World behavior3. Maximum eye error is
0.00003282 units; maximum forward/up error is 0.00000198/0.00000183. All
three state guards pass, with 3,440 meshes retained. Sol inspected all three
640×480 PNGs: courtyard/tree, overhead statue-base and shield-close-up frames
are usable, while coloured/overlapping characters and absent fade/letterbox
remain presentation differences. Gogoud is held in this fixture. There were
114 nonfatal unbound InputMap warnings, no assigned-log GPU/device-loss/display
error markers, no timeout or leaked processes. Coredump enumeration remains
unverified because access to the system bus was denied. Evidence is
`TestResults/manor-source-shots-selection-v9-sol-20261003.log.json` and
`.log.d/manor-source-shot-excerpt-1791042072961288188/`, including source/fixture
hashes, compiled excerpt, logs, numeric report and PNGs. This accepts source
shot commands and waits in the declared excerpt; it does not accept full,
unchanged World initialization or the complete opening.

A read-only source inspection also identifies the fourth-cut producer:
Gogoud/model210 normal0 completes route6 and actions26/24, selects action23,
then assigns World.dsg_0 = 4. World/model154 behavior3 rule0 selects helper20,
waypoint67 and mode3. All three camera calls in that branch are now typed; the
World instance begins dsg_0 = 0, so a harness must not invent the trigger value.
The next integration hypothesis joins the existing actual Gogoud route/animation
sequence to this unchanged camera branch. Evidence is
`TestResults/manor-next-camera-source-sol-20261003.json` (1.364-second supervised
source-only import, clean cleanup; v8 digest preserved during execution).
Gogoud's following calls are dialogue position328, incremental actor turn23
and dialogue325. The separate UI worker owns dialogue and source fade/widescreen
calls317–323; camera/non-UI work must consume that interface rather than duplicate
presentation or independently choose cut timing.

The prepared `imported_gogoud_opening_ai_drives_fourth_camera_cut` case shares
the fresh source/Rete/export setup with the accepted excerpt case. It preserves
Gogoud/model210 and its actor byte-for-byte through Rete, checks the actual source
producer/consumer statements and records their hashes. World normal2 still uses
the same declared excerpt, while its normal3 and all Gogoud AI are unchanged.
The harness starts Gogoud normally and runs World's generated normal coroutine
on ordinary automatic Brain physics updates. World reflex stays held because
its unbound input expressions remain unsupported. No harness assignment supplies
World.dsg_0, camera eye, helper or mode. Assertions require original action
order, source-associated native contact, four cuts, the original fixed-eye/axis
values, and consumption of the generated trigger within 40 native milliseconds.
The case stops before Gogoud's subsequent 1200-ms wait reaches dialogue. Syntax
preflight owns both inherited harness files and its log before conversion;
gameplay is bounded at 25 seconds, the case at 180 seconds. Native execution
and four rendered images are pending Luna. This is integration preparation,
not a new original-game comparison or full-opening acceptance.

Source-only preparation passed locked offline release build, all-target strict
Clippy, formatting and the expanded original source/Rete case (1.31-second body).
The supervised four-stage check took 92.176 seconds, with unchanged source and
clean cleanup: `TestResults/manor-gogoud-fourth-shot-v10-preflight-sol-20261003.json`
and its four logs. Stephen's subsequent request for a New Game menu preview
takes priority; this fourth-cut native assignment is deferred.

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

### Hype presentation: non-null particle component and truthful call diagnostics

The later actual New Game V10 checkpoint reached Gogoud's first dialogue with
all 69 rooted actor Brains automatic; full opening handoff remains unaccepted.
The user observed colored Hype breathing over the grey statue. Hype still stops
in Fix model 28, normal-0 schedule node 14, at procedure 194
(`SetParticleGenerator`). Its native character starts the initial action even
though the Brain stops; this explains how animation can remain visible, but
does not prove the full cause of the overlapping pose.

The new source corpus check proves Hype has a non-null 16-byte particle component
(`Fix/[5,0]/23321`), initially disabled with no selected generator. Therefore
the original API's null-component skip cannot legitimately unblock this call.
Node 15 is category 37 and binds generator `Fix/[5,0]/22092`; node 16 disables
emission. The generator's typed 44-byte definition resolves source, system and
environment records of 132, 40 and 68 bytes. The system has capacity 200, zero
active particles and kind 4 (sprite particles). Unknown fields remain explicit
byte arrays; the test reconstructs the interpreted component/definition bytes.
At that earlier checkpoint this was source research, before the useful particle
asset/native emitter slice described below. No new no-op was added.

Raymap's partial Perso record was consulted first. Relevant particle handler,
selection, disable and dispatch byte ranges were then checked identical between
the decompilation reference and the pinned installed executable (SHA-256
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`).
Proof: `TestResults/hype-particle-handler-static-byte-proof-sol-20261003.json`.
The corpus report records bound identities, bytes and the later presentation
calls at nodes 814/816/817: transparency 255, transparent display, then display.
Fixing the first particle call alone does not establish correct visibility or
placement.

World model 154 normal-2 node 22 was misleadingly described by the compiler as
a missing `SetPersoAbsolutePosition` receiver. It is actually an actor-directed
`NoDisplay` (procedure 266) targeting `world/0/174`, not Hype. Unsupported Ultra
diagnostics now name the actual child category/operation. A negative control in
the existing compiled-camera reconstruction test preserves all unsupported
source nodes and proves that call is still unsupported rather than silently
accepted. Generated manor AI was not regenerated merely to update a diagnostic.

The source assertion and corrected diagnostic test passed, along with the two
menu source checks, with clean supervised cleanup and unchanged source:
`TestResults/menu-opening-integration-source-final-sol-20261003/report.json`
and `opening-source.json`. Formatting and root-package all-target Clippy with
warnings denied passed. The new corpus assertion is inventoried; the source-only
results are separate from any actual presentation or full-opening acceptance.
Next source work must implement the needed particle asset/API slice and trace
the statue's field-0 Position and actor-directed initialization calls; no
hardcoded hiding or pose substitution is justified by the present evidence.

### Typed particle selection asset/API checkpoint, native acceptance pending

The next local change promotes the real generator into one useful
`ParticleGenerator` asset rather than skipping call194. The rooted manor/source
AI scan finds one component (Hype), one generator, and no independently shared
emission/system/environment definitions. Those settings therefore belong in
one asset, with the original Sprite as a separately shared dependency. Minimal
Fix/level block/allocation identity is retained; runtime counters, memory
addresses, buffer pointers, lazy initialization and derived bounds remain at
the OpenSpace boundary. Unknown non-pointer fields remain explicit. This
boundary is provisional for levels with independent sharing.

The typed source is emission mode1, number2, shape1 and coordinate mode2.
Shape1 samples two independent scalars in [0,1], so the three supplied points
span a parallelogram, not a triangle. Initial velocity is [0,-1,0.5], symmetric
random extent [1.4,0,1], lifetime mode0 (infinite). The relative environment is
a radius5 sphere with acceleration [-0.1,-0.1,-0.1]. These are source meanings,
not accepted Godot particle motion. Static bytes for nine promoted definition,
selection-boundary, generation and mechanics functions match the installed
executable and reference: `TestResults/hype-particle-definition-static-byte-proof-sol-20261003.json`.
This supplements the earlier five handler/callback ranges and the Raymap-first
research. Neither static proof invokes the original engine.

Rete schema40 persists this useful asset and the actor's optional component.
The parser binds call194/category37 to a typed asset reference, and the compiler
preloads a converted native emitter scene. The native actor component selects
that scene and performs lazy relative-boundary initialization; call193 disables
births only on that component. Repeated selection uses ordinary PackedScene
resource identity. Invalid scenes are rejected without changing selection or
leaving children. Actors genuinely without a source component retain the
original API's skip. Hype's actual non-null component must succeed.

The saved generator scene is `CPUParticles3D`, capacity200, disabled, with the
converted QuadMesh, actual Sprite material and texture. Emission is deliberately
an explicit unsupported API: call192 on a present component fails until source
random velocity, infinite lifetime and boundary removal are implemented and
validated. Initially selected/active component conversion also fails pending a
saved-state contract. Source/environment record reconstruction covers the
verified parallelogram profile and rebuilds its derived normal without replaying
an encoded record. Whole particle-package reconstruction remains unsupported.
This is the smallest source-backed selection/disabled initialization slice;
it does not implement the full particle simulator.

The source-only final check passed in53.930seconds, with unchanged source and
clean cleanup: formatting0.495s, root all-target Clippy with warnings denied
3.142s, library/new-test compilation49.527s, all36 partial AI codec cases0.006s,
and the exact manor corpus case0.744s. Commands and supervised results are in
`TestResults/hype-particle-source-final-r4-sol-20261003/report.json`. The corpus
checks useful import, MessagePack equality, exact132/68-byte reconstruction,
radius editing, bound194/37/193 reconstruction and rejection of unbound operands.
Workspace/all-target `cargo check --locked --offline --workspace --all-targets`
also passed during preparation. Earlier reporting attempts are retained: the
first codec filter ran zero tests; the second actually passed36 cases but failed
its stale33-case count guard. The third report passed; the fourth is the final combined pass after adding
stable script UIDs to prevent native editor scans from changing frozen source.

The new declared `astrolabe::particles` case reloads the actual generator,
Sprite/material/texture closure and a disclosed excerpt of original nodes14..16
from fresh Rete before native export. It executes that original select/disable
pair and checks the real native resource, relative initialization, repeat/invalid
selection and explicit emission rejection. It changes the fixture AI and accepts
only that component slice. The normal New Game case separately keeps all rooted
actor Brains automatic and full generated AI unchanged, captures Hype's real
selected generator and subsequent failure, and requires selection/disable
initialization to advance past194. It continues the existing Gogoud contact,
first dialogue, restart and private-save checks. The native checks have not run
on this checkpoint; no rendering, initialization or whole-opening acceptance is
claimed from the Rust tests.

Support interface49 avoids the inventory worker's reserved48. Integration must
combine additions to `src/model/content.rs`, `src/godot/support.rs` and
`tools/test-manifest.json`, retaining49 rather than reverting the version to48.
The inventory worker remains independent of these Brain/NewGame changes and
must retain the existing font cache-order fix. The published local generation
is still interface47:4822 owned files, manor4785 plus menu37. Sol has neither
regenerated it nor launched either engine.

The frozen Luna recipe is
`python3 -B TestResults/run-hype-particle-integration-v12-sol-20261003.py`.
Its pin guards current source, commit, exact changed GDScript/preload bytes,
valid published receipt and authored settings/save fixture. Five syntax checks
precede release build and normal manor port. It copies/hash-verifies every prior
owned generated file into `output/archived-manor-before-particles-v12-20261003/`
and requires all37 unselected menu files to remain unchanged. The bounded
selection declares25/287 cases,262 exclusions, all21 mandatory safeguards and
four Python check modules. It includes source evidence, the new native component,
actual New Game and crossbow free-flight cleanup regression because the shared
particle-disable API changed. The overall budget is780s, selection420s, port180s,
release build120s and parser checks10s each, with cleanup reserved; failure or
timeout stops the recipe and preserves evidence. No source/schema/receipt edit
is permitted to force a pass. Native result locations will be
`TestResults/hype-particle-integration-v12-sol-20261003/result.json` and
`TestResults/hype-particle-selection-v12-sol-20261003.log.json`.

After native confirmation, Hype's next anticipated initialization barrier is
procedure187 (transparency255) at node814, then185 (transparent display),267
(display) and an unsupported expression at820. This is a hypothesis from the
compiled source, not a current gameplay observation. A useful independent UI
worker task is a native rendering component for187/185/186, corroborating
handlers00543130/00543100, in new rendering/harness files; typed parser/compiler
integration should remain coordinated. Statue model294's field0 Position and
its actor-directed initialization calls are separate unresolved source leads.
Neither colored-Hype/statue overlap nor the complete original opening endpoint
is accepted or hidden by this particle slice.

### V12 runtime result, opacity continuation and inventory integration

Fixed Luna completed V12 in462.307s. Publication succeeded at interface49 with
4826 owned files (manor4789, menu37); the prior4822-file generation is archived.
Normal New Game proved the actual non-null Hype component selects
`fix:/particles/generators/5-0-22092`, initializes capacity200 and disables
emission, then stops at the predicted model28/node814/procedure187. Gogoud's
existing first-dialogue, contact/actions, all69 automatic rooted Brains and
private-save/restart/Escape checks passed. The selection passed24/25 cases,
including all21 mandatory safeguards and all four Python modules, with unchanged
source and clean cleanup. This is not a clean25-case pass.

The isolated particle excerpt produced its success marker and correct generator
state, but its actor omitted the expected AnimationPlayer. Character process
updates therefore emitted node-not-found/null-is_playing errors. The correction
adds the ordinary native AnimationPlayer to that fixture; production character
error handling was not weakened. Preserve V12 failure evidence:
`TestResults/hype-particle-integration-v12-sol-20261003/result.json`,
`TestResults/hype-particle-selection-v12-sol-20261003.log.json` and sibling
`.log.d/particle-selection/engine.log`. The actual opening evidence is sibling
`.log.d/new-game-opening/report.json`; its Hype initialization is independently
valid despite the separate fixture failure.

The next source-backed change compiles187/185/186 into opacity and
transparent/default display APIs. Raymap names were checked first; installed
and reference executable bytes match for handler00543130, recursive setter
00543100, display flag helper00443040 and rendering routines00499ed0/004948c0.
Proof: `TestResults/hype-opacity-handler-static-byte-proof-sol-20261003.json`.
The handler clamps the integer to0..255, recursively stores the float opacity,
and separately clears/sets draw flag8 for transparent/default display. The
renderer retains its ordinary material pipeline at opacity>=254 and requires
shaded-blending flag0x20000000 for actor opacity below that threshold.

Native support stores those two independent properties and uses owned copies
of StandardMaterial3D surface/override materials for partial opacity. It
preserves shared source materials, the source texture/material alpha and
original transparency mode on restoration; unsupported reached materials fail
explicitly. Hype's actual255 request retains full opacity. Godot's native
GeometryInstance3D.transparency is ignored by Compatibility/Mobile renderers,
so it cannot implement this API on the project's current renderer; see the
[official property documentation](https://docs.godotengine.org/en/stable/classes/class_geometryinstance3d.html#class-geometryinstance3d-property-transparency).
Animation opacity tracks, attached generated actors and full render sorting/
blend fidelity are separate unverified paths. This change does not fix those
existing animation tracks or hide/relocate Hype.

The source corpus now reconstructs the actual814..817 presentation nodes
byte-for-byte. The component fixture compiles disclosed original14..16 and
814..817 excerpts, adds its missing AnimationPlayer, and exercises128 opacity,
mode restoration, clamps and preservation of a shared native material. The
unchanged-AI New Game reporter requires the actual255/transparent display calls
to complete and records the subsequent failure. Their native execution remains
pending after the next normal manor regeneration; anticipated next barrier is
expression category3/op35/node820, not a completed opening.

At the parent's instruction the independently accepted inventory commits
b0658c1/896959b/a84cd60/aa14a8e were integrated locally as
8fe83cc/e4ec6f6/5a6d2fc/85b8a43, after the opacity checkpointc51f231. Support
interface50 gives the combined inventory/particle/opacity source one new
identity. Rete remains40; the shared registries include both content kinds and
the inventory case increases the declared matrix to288. The common bitmap-font
helper preserves advance-before-glyph/cache ordering and the main engine
normalizer is unchanged. Existing manor/menu content, generated font and user
settings/saves are not edited during source integration.

Combined source-only validation passed61.118s with unchanged source/clean
cleanup: format0.529s, root all-target strict Clippy7.093s, library/particle-test
compilation52.079s,36 AI codec cases0.004s, exact Hype corpus0.819s and exact
inventory import/fresh Rete/native-text conversion0.575s. Commands/results are
in `TestResults/hype-opacity-inventory-source-sol-20261003/report.json`. The
initial opacity Rust check caught an undefined match variable; the explicit
185/186 arms corrected it and later checks pass. No engine was run by Sol.

Inventory has its own accepted isolated native gate (13.100s, clean cleanup)
at `/home/stephen/Documents/Codex/2026-10-03/task-4/astrolabe-inventory/output/inventory-qa-aa14a8e-r1/run/results.json`.
The worker inspected four rendered PNGs: original art/labels/counters are clean;
the static wheel and Magic's enlarged rings remain presentation limits. This
gate uses a synthetic actor inventory Resource and does not establish actual
manor entry or original-game parity. No additional user-delivery gate is required.

The inventory-only publication recipe is
`python3 -B TestResults/run-inventory-publication-v13-sol-20261003.py`.
It parses all nine changed GDScripts/closure first, then builds and performs a
normal `--content inventory` port. It requires146 new inventory files and all
4826 existing manor/menu hashes to be preserved, plus authored settings/save
fixture hashes. Budget240s with cleanup reserved; parser10s each, build120s,
port90s. It does not run another gameplay gate, or rebuild manor to force the
opening forward. Publishing interface50 inventory leaves the earlier manor AI
unchanged at its documented187 stop. A separate small corrected-fixture/opening
native validation follows normal manor regeneration; no clean V13 native pass
or complete intro/overlap acceptance is claimed here.

### V13/V14 results and narrow continuation — 2026-10-04

V13 inventory publication passed in69.779s:146 inventory files were added,
all4826 prior owned files were preserved, and the interface50 receipt owns4972.
The corrected particle/opacity excerpt fixture passed in8.911s with unchanged
source/generation and clean cleanup. Their separate reports are
`TestResults/inventory-publication-v13-sol-20261003/result.json` and
`TestResults/particle-fixture-retry-v13-sol-20261003/result.json`; they do not
relabel the historical V12 selection failure.

V14 ran `python3 -B TestResults/run-actual-opening-v14-sol-20261003.py` once.
Normal manor publication passed105.593s, retaining all37 menu and146 inventory
files and the accepted font cache. All4972 prior files were hash-verified in
`output/archived-manor-before-opacity-v14-20261003/` before regeneration. The
actual opening case ran30.928s and produced its first-dialogue marker, saved
slot preservation and opacity255/transparent-display initialization. Hype's
next failure is now category3/function35/node820. However, the owning Rust test
failed on an unexpected native resource-path error for
`models/world/1/27/0/instance.mesh.tres::stable_8`; the overall182.961s result is
**failed**, with unchanged source and clean cleanup. Evidence, actual model28
compilation and two screenshots are under
`TestResults/actual-opening-v14-sol-20261003/`. Both screenshots were inspected;
Hype/statue overlap remains visible. Full intro completion remains pending.

The Tab correction was integrated as64d902e after that native run completed.
The final V14 result was written at23:51:45.548365UTC; the cherry-pick's committer
timestamp is23:53:13UTC,87.452s later. The preserved frozen audit therefore
remains valid even if an execution worker read HEAD later while composing its
summary. Additive timing evidence is
`TestResults/actual-opening-v14-source-audit-timing-sol-20261004.json`; no V14
result or pin was edited.

Function35 is Raymap's `Func_GetHitPoints`. Original Hype nodes818..821 assign
`GetHitPoints(Me)` to designer field128. The unchanged source StandardGame byte
at+0x18 is50. Static handler00538870 dispatches35 through005388d7 to00460a30,
then getter00438ec0; it reads that unsigned byte and returns an integer. The
existing useful `HitPoints` and native actor-local Resource already represent
this state. The narrow compiler addition reads its current value, guards missing
or invalid life state, and leaves other actor receivers explicitly unsupported.
The codec reconstructs the original assignment bytes; full Hype AI survives
fresh MessagePack decode. A disclosed isolated assignment checks the compiler;
the normal port/New Game case supplies complete-graph gameplay validation.
Proof: `TestResults/hype-hitpoints-handler-static-proof-sol-20261004.json`.

Static export inspection also found the collision script is installed after
life state was attached. Godot's replacement script resets inherited properties;
the V14 Hype scene consequently lacks `source_hit_points`. Attach life after
installing the final script, so the regenerated actor can execute its own query.
The actual opening assertion independently requires50 in both the life Resource
and field128. Combat damage and other hit-point functions remain outside scope.

The reported mesh has unique subresource IDs and is byte-identical to its V13
copy. Its duplicate registration occurs during initial loading, before actor
AI. The session now keeps dependent resource loading in one background worker
instead of enabling parallel dependency workers, while retaining native cache
reuse and a responsive menu. This is a focused mitigation of the concurrent-load
lead; V14 alone does not prove its root cause or that the mitigation passes.
Godot's [ResourceLoader contract](https://docs.godotengine.org/en/stable/classes/class_resourceloader.html#class-resourceloader-method-load-threaded-request)
defines that option. No generated IDs/receipts or test error filters were altered.

Support52 is reserved for this life-read addition, avoiding the separate pause
worker's reserved51. Rete remains40 in this slice; the new codec regression
brings the declared matrix to289. The first source check caught a test expecting
the strict writer to reconstruct an unsupported receiver; it remains preserved
as failed. The corrected r2 passed37 codec cases, the actual source corpus,
strict root Clippy and formatting in50.879s, before the final export-order change.
Final current-source r3 passed in49.456s with unchanged source and clean cleanup:
format0.543s, strict root Clippy3.173s, validation library build44.960s,
37 codec cases0.004s and the actual source corpus0.757s. Command:
`python3 -B TestResults/check-hype-hitpoints-opening-source-r3-sol-20261004.py`.
Its `TestResults/hype-hitpoints-opening-source-r3-sol-20261004/report.json`
includes exact commands and digests. The next bounded actual-opening recipe is
`python3 -B TestResults/run-actual-opening-v15-sol-20261004.py`, for the fixed
Luna native executor, with exact-current syntax before normal scene conversion.
It preserves menu/inventory/font/settings/saves, backs up the prior publication,
uses480s including cleanup reserve and runs only the existing actual New Game
case. It requires the life read50/field128=50 as well as prior opening assertions.
Native continuation is pending; no V15 pass or repaired visual overlap is claimed.

### V15 native checkpoint — 2026-10-04

Fixed Luna executed that frozen recipe once and it **passed in 248.115s**.
Interface 52 publication owns 4,972 files; all 37 menu/146 inventory files, font,
settings and save fixtures were preserved. Source stayed unchanged and all
stages/syntax cleanup were clean. The actual New Game case reached first
dialogue, preserved the private slot and observed Hype life 50/field128=50,
particle capacity 200 initialized/disabled, and opacity 255/transparent display.
There are no unexpected engine errors; the V14 `stable_8` error is absent in
this run. This verifies the bounded checkpoint with serialized dependency
loading, not a general proof of the engine race's cause.

Evidence: `TestResults/actual-opening-v15-sol-20261004/result.json` and sibling
`new-game-opening/report.json`, `engine.log`, actual generated models and
`new-game-opening-start.png`/`new-game-opening-walk-end.png`. Sol inspected both
images; the colored Hype and grey statue still overlap. That visible defect and
the complete intro endpoint remain unresolved.

Hype's new reached failure is procedure 429/node987. The verified installed
executable catalog calls this `Fight_ChangeArmor`; Raymap's older procedure
table is shifted here and calls 429 a different operation. Static 0053dd60
subtracts 429, dispatches that first arm to 0053ddc8 and calls 0043a9b0 with one
integer. That helper selects a linked armor definition by byte index from the
shared armor root; the reached original node988 is integer 0 (the alternate
node984/985 branch selects 2). This is not enemy damage. The definitions also
own source armor artwork/resistance state, so a dummy no-op would conceal the
next gap. Source-based armor selection needs its actual useful definition/state
and native consumer; the separately requested HUD-layout investigation can
inform that boundary without broadening this checkpoint into unrelated APIs.
The verified executable hash, dispatch table, call bytes and decompilation hashes
are recorded in `TestResults/hype-armor429-handler-static-proof-sol-20261004.json`.

### Pause gate review — 2026-10-04

The separate pause worker's native gate at
`/home/stephen/Documents/Codex/2026-10-03/task-4/astrolabe-pause/output/pause-qa-2047ea0-r1/run/results.json`
is **failed**, in 13.369s. Its five syntax checks, pause-only conversion and
synthetic modal probe exited successfully with clean owned-process cleanup.
However, the probe logged two leaked ObjectDB instances and one resource still
in use at engine shutdown, which the gate correctly rejects. The missing-assets
variant was never executed. Frozen bundle hashes still match; no result or source
was rewritten. The probe's internal report passes Resume/state preservation,
Esc/Tab, delayed-confirmation cancellation, routes and editable scene checks.
Those are component observations, not a passing gate or actual manor acceptance.

Sol inspected the 4:3 and wide Resume screenshots: original source glyph artwork,
selected appearance and aspect fitting are present over a synthetic flat-color
stand-in. The smallest next check is one bounded verbose reproduction by fixed
Luna to identify the leaked owners, followed by the pause worker's lifecycle fix
and complete gate including the missing-assets control. Main integration and
publication are pending that result. When integrated, bump support to 53 after
the already published interface 52; the worker's reserved 51 must not downgrade
the main tree. Preserve all 4,972 prior generated files and unrelated ownership.
Read-only audit: `TestResults/pause-gate-review-sol-20261004.json`.

### Pause integration and publication handoff — 2026-10-04

The corrected separate gate `pause-qa-2047ea0-r2/run/results.json` passed in
9.045s. Only fixture teardown changed; production2047ea0 stayed unchanged. All
404 frozen member hashes and173 reused native files match. Shutdown is clean,
and the missing-assets control now passes along with the modal/routes/save checks.
The failed r1 remains preserved. No extra synthetic/visual gate is requested.

Main integrated f6f811a as `b37a78c` and2047ea0 as `8125cb6`, skipping duplicate
Tab. Support53/Rete41 unions the pause additions with the life/particle APIs;
life/AI/Tab source bytes and serial background loading are preserved. Existing
opening/demo smoke assertions now exercise Esc pause and Main Menu instead of
direct Esc-to-bootstrap; opening also requires Resume preserve the actor. Those
updated runtime assertions await a future actual-opening check; they have no
new native behavior pass. The release inventory has290 declared cases. Formatting
and diff checks pass; no broad tests or armor implementation were started.

Fixed Luna's next command is
`python3 -B TestResults/run-pause-publication-v16-sol-20261004.py`.
Its frozen pin requires exact-current parser inputs before release build and
normal `port disc godot --level manoir --locale en --content pause-menu --godot godot`.
It backs up/preserves all4972 current files, font/settings/save fixture, and adds
the accepted27 disjoint pause paths at support53 (expected4999 total). Only
publication is requested; no actual/synthetic gameplay rerun. Until its result
passes, the live generated receipt remains52 and pause publication is pending.
After that bounded delivery, park development at the parent's quota stop. Next
opening work is source armor429/node987 and the still-visible statue overlap;
the existing static proof and separate HUD investigation own the research lead.
