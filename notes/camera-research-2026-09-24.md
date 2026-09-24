# Hype manoir camera research

The visible BDD wrapper currently creates its own `Camera3D` with Godot's
75-degree default FoV, at seven units behind Hype and 3.5 units above him,
looking at an assumed 1.4-unit target height. The original and Godot walk
screenshots from the clean focused run show materially different framing:
Hype is much larger in the original view. The aspect ratios already match
(original 1024×768, Godot 960×720), so distance, orientation and projection
must be measured separately. Earlier source-camera captures in
`material-transparency-evidence.md` measured a 1.5-radian horizontal FoV and
69.8838-degree vertical FoV at 4:3, but their camera pose belongs to a
different viewpoint; it must not be reused as the `manoir` pose.

The source probe used the supplied English save (archive SHA-256
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`,
executable SHA-256
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`),
verified `manoir` at phase 9,
and read the active viewport camera through its live engine record. The viewport
is 1024×768, with horizontal FoV 1.5 radians, vertical FoV 69.8838 degrees,
near plane 0.25 and far plane 10000. At saved idle the camera was
`(-74.37785, 24.14221, 7.38520)` in source axes while Hype was
`(-74.37860, 27.48549, 4.55085)`. After a physical 500 ms Left turn, the
camera offset rotated with the actor and remained 3.3433 units behind and
2.8344 above. The recorded camera basis aims at about 1.816 units above the
actor. The source idle screenshot is
`/home/stephen/Projects/hype-harness/local/orrery-runs/run-1790230129874423799/screen-1790230162225988151.png`.

A short forward pulse showed more than a rigid orbit: camera range from Hype
was 5.118 units on release, 4.989 at +350 ms, and 4.848 at +1100 ms, then
stable. The latter pose is about 3.822 units behind and 2.982 units above Hype,
still aimed at roughly the same height. This may reflect an obstacle limit,
target-follow lag or a changed desired distance; the record alone does not
distinguish them. The time series is in the second Orrery run at
`/home/stephen/Projects/hype-harness/local/orrery-runs/run-1790230358883261554/`.
These are stopped-byte observations, and their screenshots were independent
captures, so exact frame pairing is not yet verified.

Orrery now has an optional typed active-viewport camera observation captured
in the same process pause as MainActor. A fresh supplied-save idle sample
(`TestResults/camera-runtime-2026-09-24.json`) measured Hype at
`[-74.37856, 27.48479, 4.55084]` and camera at
`[-74.37771, 24.14157, 7.38519]` in source axes, with offset about 3.34322
behind and 2.83435 above. Projection was 1.5 rad horizontal / 69.8838°
vertical, near 0.25, far 10000. The source update counter was 1354 and source
clock 69597 ms; capture took 56.2 ms. This is one coherent stopped-byte sample,
not a proven completed-update frame. The bounded walk/turn recapture did not
reach the save after a controller restart, so the earlier unsynchronized
distance series retains its stated uncertainty. The game and Orrery service
were stopped without modifying the installation or save.

A temporary Godot calibration used the measured 4:3 projection and stationary
pose. Its visible focused BDD rerun passed both movement scenarios in one
process with complete cleanup (`TestResults/behavior-suite-1790230476038639185.json`).
At ready, its camera was `(-74.378662, 7.365318, -24.143211)` in Godot axes,
within 0.020 units of the source camera after axis conversion. The source
turn image and temporary Godot turn image framed Hype at similar size; the
Godot walk image left him roughly 30 pixels taller than the source settled-walk
image because its fixed boom stayed at the initial distance. The temporary
images are `TestResults/behavior-runs/godot-turn-release-idle-1790230522872399522.png`
and `TestResults/behavior-runs/godot-walk-release-stop-1790230525577366245.png`.
Their source and Godot frames are not synchronized, and geometry and actor
rotations differ, so landmark/pixel parity is unverified.

The user decided camera behavior should be its own development after magick.
The temporary Godot calibration and camera assertions were removed from the
movement suite. That suite continues to test actor behavior with its existing
test-owned presentation camera. The later camera slice should determine how
camera objects, selection and follow logic map to native Godot nodes, then
investigate the walk-distance change before implementing collision or easing.

## Native camera slice boundary

Raymap's viewer locates a `StdCamer` Perso and copies its transform to its
own initial camera, but that is a viewer setup path rather than the original
follow controller. The original viewport holds a separate active camera
pointer. Its general-camera routines initialize controller state from a camera
Perso and compute a target offset through an actor-derived rotation. The
checked-in decompilation does not establish follow smoothing or obstruction
behavior. Raymap's 116-byte `CameraInfo` reader belongs to a separate ROM
format, so its unnamed fields must not be applied to this Hype allocation.

Before the native camera slice, the imported `StdCamer` had a rooted scene
transform, but its Perso asset omitted both its source camera link and Brain.
The Brain was still unimported after the first camera slice: its raw link
reaches Fix AI model 437, which the importer then rejected because designer
value type 11 was unsupported. The list slice below resolves that gate.
Model 733 belongs to the arrow and is a separate AI dependency. An older local Rete fixture
places it and Hype at different world coordinates from the supplied save; its
relative initial placement is not proof of the saved runtime camera pose.
The live horizontal 1.5-radian FoV, near/far planes and measured follow offsets
are observation targets, not portable export constants. The Montreal Perso's
camera slot at `+20` has a real relocated link on rooted `StdCamer` and is
null on the ordinary actor checked. The linked allocation is pointer-rich
and its Hype field meanings remain unknown. The first slice preserves the
association and adds a native `Camera3D` bound to that actor. Importing the
camera Brain and implementing verified follow behavior were separate work.
Camera assertions stay separate from movement/contact scenarios.

The verified executable's procedure 160 stores its resolved target Perso
pointer in the camera actor's runtime data at `+0x10`. Procedure 163 restores
the main actor there. The former also writes 255.0 to `+0x3c` and clears
several other fields with unknown meanings; these reset effects must remain
explicitly unresolved if only target assignment is ported. This target state
does not by itself select the active viewport camera or prove a follow pose.

The first native slice now records whether the Montreal Perso `+20` camera
link is present. The `StdCamer` asset retains that association through Rete;
the exporter gives it a saved `Camera3D` child. Godot would otherwise make the
first camera current automatically, so the child clears that implicit selection
on ready until source viewport selection is implemented. The level
also indexes actor nodes by their source `AssetRef<Perso>` paths. Model 733's
compiled camera operand is exactly `level:/persos/world/0/2`, the rooted
StdCamer path, so procedure 160 can store MainActor on that camera component
without matching a display name. The original handler also resets fields
whose meaning is unknown; this support method implements only the target
assignment and does not yet claim full procedure fidelity. The linked camera
allocation remains unpromoted and is not preserved as a full editable camera
asset in Rete. That source-content loss and the unimplemented follow,
projection and reset effects are explicit camera slice debt.

A fresh local `manoir` export saved `SourceCamera` in the rooted `StdCamer`
scene and the exact `level:/persos/world/0/2` actor-to-node binding in the
level scene. Godot's headless editor scan loaded the generated project without
scene or GDScript parse errors. This validates the saved structure and target
lookup, but does not establish that this camera produces the source view.
The compiled AI remains responsible for calling the camera API; this slice
adds only the target-state surface and prevents Godot from selecting an
unfinished camera by default. No support-side camera follow, FOV correction or
gameplay decision is used to make another scenario pass. Any future temporary
support approximation must be identified as fidelity debt before use.

## Camera Brain and list gate

The rooted `StdCamer` Brain links Fix AI model 437. Import previously dropped
that Brain because its designer field 27 used unsupported type 11. Raymap identifies
Hype type 11 as a list, and the installed build's size function at `0x005236a0`
reports `4 + 4 × capacity` bytes. The model's field has offset 68, capacity
one, current length zero, two zero header bytes and one zero slot in default,
initial and current memory. The slot has no relocation in those buffers.
Model 437's original scripts call `Cond_IsPersoInList` on that field and
`Proc_AddPersoInList` with an actor operand. Its useful values are therefore
ordered Perso references with bounded capacity, not an integer standing in
for an unexplained pointer. Populated source-slot reconstruction beyond the
observed empty case remains a separate validation limit.

The installed handlers establish the reached list operations: condition 53
compares actor identities in the active prefix, while procedure 69 appends
without deduplication and reports source error `0x0e` when at capacity. Source
removal can leave stale inactive entries, so the full capacity-sized slot array
is retained through Rete. The `manoir` import now restores model 437 and the
rooted `StdCamer` Brain. A focused local test checked the empty capacity-one
field in default and current memory, typed membership/add AST nodes, and Rete
round trip. A fresh full manor export and Godot editor scan passed. The saved
`StdCamer` scene has `SourceCamera` and `Brain` children, and the generated
model-437 script uses native list membership and append calls. The current
runtime projection of a populated source list is an explicit gap.

Source update `0x005200a0` reaches target computation `0x0051d330`, which
reads selected actor translation and rotates a camera dynamics vector by
actor basis. Viewport camera selection is a distinct call at `0x00485380`.
Neither trace yet maps the linked 224-byte source allocation to projection,
offset or smoothing fields; measured idle FoV/pose cannot serve as a general
export rule. The ordinary model-437 update path uses a distinct
compiled-AI pattern: `normal_0_rule_0` reaches `Proc_SetActionReturn` (category 4,
ID 6) with scalar zero before zero-argument `Proc_Camera_UpdatePosition`
(category 4, ID 135); `normal_11_schedule` uses the same pair. The earlier
compiler recognized procedure 6 only as a locomotion yield paired with
behavior request 209/210. Raymap provides the operation names, but the checked-in
decompilation has no handler body at the cataloged `0x0053fc30` entry, so
the meaning of action-return zero remained unresolved there. Offline
disassembly of the installed executable with Orrery's pinned SHA-256 locates
procedure 6 at `0x0053fc30`: its ID-6 branch stores the scalar's low byte in
return-value global `0x00721b5c`, sets flag `0x00721b58`, then returns
normally. A consumer at `0x00522750` clears that flag and retains its incoming
status when the stored value is zero. A bounded Orrery-controlled live run
loaded the pinned English `manoir` save, then GDB observed hits at
`0x0053fc30` and `0x00545d80` in that order on the same thread, about 3.43 ms
apart, with the same immediate return address `0x0252c2a4`. This supports
normalizing the exact return-zero/camera-update pair to one typed camera call
while retaining both source nodes for reconstruction. It does not identify
the rule/frame boundary or prove the camera pose produced. The generated call
now reaches an explicit Godot-side unsupported camera-update stub; it is not a
working follow camera. The focused source-node round trip and local model-437
Brain/Rete test passed. A fresh release manor export completed in 86.14 s;
Godot's headless editor scan found no parse errors. Generated model 437 has
zero unsupported type-11 or action-return-policy diagnostics, while 133 other
unsupported-script diagnostics remain, so this is not full camera-AI coverage.
An import of model 437's actual AST found default normal behavior 0. Its first
rule is an always-true `0 == 0` branch that sets designer field 30 to zero and
requests `CameraUpdatePosition`; behavior 0 has no schedule. That rule and
rule 2 have no unsupported fragments. Rules 1 and 3–5 contain other
unsupported expressions/procedures, whose reachability needs separate
evidence. The update API stub is therefore the immediate blocker on the
default first-rule path. Normal behavior 11 has another update schedule,
distinct from behavior 0. This is a static imported model observation, not a
live assertion of the currently selected camera behavior.

The game and Orrery service stopped, and source save
hashes remained unchanged. That read-only run reached phase 9 / `manoir` under
owned game PID 3241812 and LWP 3241970. The executable and supplied-save hashes
were the pinned
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`
and `877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`;
the 2,847-byte installed `manoir.sav` retained SHA-256
`d87d6b743514e070983633385742ab16c6f8a189ad6a01ff80e5bb636c7df6c7`.
GDB breakpoint pauses perturb timing, and its stack unwinder could not prove
the enclosing rule. The camera allocation behind the source link also needs
useful-field mapping and explicit unknown preservation beyond the current
association flag.

A read-only source allocation probe (`TestResults/camera-payload-probe.json`)
followed the rooted `StdCamer` Perso's `+0x14` link from Fix allocation 23322
to Fix allocation 23339. It also followed the other linked manor actor at
`world/0/147` to Level allocation 7889. Both targets are 224 bytes and have
six relocation sites at `+0x00`, `+0x04`, `+0x08`, `+0x0c`, `+0x10` and
`+0x1c`; the first four target the same source block, while the last two
cross into other blocks. Their nonpointer payloads match: almost all zero,
with sentinel floats at `+0xa4/+0xa8`, word 103 at `+0xac`, and word 42 at
`+0xdc`. These bytes do not identify the active viewport's FoV or follow
distance. The source payload is a distinct allocation from the live viewport
record that Orrery samples. The current `has_camera` import preserves the
association but does not yet preserve this payload or its six links.

Offline disassembly of the pinned executable's full procedure-135 handler
(`0x00545d80` through `0x00546073`) shows it follows the calling actor's
`+0x14` camera link, dereferences related records at allocation `+0x04` and
`+0x0c`, builds temporary camera state through `CAM_fn_vInitCameraStructure`,
and updates flags/scalars on the linked records. It writes root-allocation
flags at `+0xce`, `+0xd0` and `+0xd2`, then sets a global update indicator via
`0x005223b0(1)`. The four initial pointers in Fix allocation 23339 address
the same source block at base, base plus `0xa8`, base plus `0x150` and base
plus `0x1f8`; the latter records' allocation boundaries and useful values
remain unmapped. The handler does not directly select a viewport, write
FoV/clip values, or publish the final camera pose. The separate `0x005200a0`
update path derives target points from actor transforms and camera vectors.
A native `source_update_position()` that merely returns success or copies the
observed idle offset would hide this missing contract, so it remains an
explicit stub.

A second read-only allocator probe (`TestResults/camera-records-probe.json`)
resolved the root's four pointers to separate 168-byte Fix allocations
23340–23343; the other manor camera has corresponding Level allocations
7890–7893. None of these eight records contains a relocation. StdCamer's
first two records share the same nonzero parameter-like values, including
1.8 at `+0x2c`, 3.5 at `+0x30`, 2.5 at `+0x3c` and 1.5 at `+0x8c`; its last
two records are nearly empty. The observed live horizontal FoV is also 1.5,
but equality alone does not identify `+0x8c` as FoV. An independent camera
actor has different values in several of these slots, useful for tracing
their effects. The native implementation still needs the corresponding
read/use paths and source-to-Godot unit conversion.
The verified executable initializes a related record's `+0x8c` field to
1.5, reads/writes it through `0x00440f10/0x00440f30`, and one consumer uses it
in angle-based vector interpolation. No traced path takes it to viewport
projection, so treating it as horizontal FoV would be unsupported despite the
same numeric value. Values at `+0x2c`, `+0x30` and `+0x3c` are also unnamed.
