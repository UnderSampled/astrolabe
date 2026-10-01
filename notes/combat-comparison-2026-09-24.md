# Combat comparison, 2026-09-24

The first combat recording starts from the supplied English `SaveGame0` in
`manoir`. Orrery physically holds Down for 800 ms to reach clear ground, waits
700 ms, then presses X for 120 ms and Space for 100 ms. The recording samples
Hype before selection, during drawing, twice during the swing, and twice after
recovery. It uses the same Cucumber steps and recorded physical input tape for
OpenSpace and Godot. The Godot run is headless by default.

The first original-game run reached action IDs **47, 45, 134, 134, 48, 48** at
0, 1100, 1700, 2000, 2700, and 3200 ms. A repeat reached **47, 45, 134, 48,
48, 48**: the swing had finished by the 2000 ms sample. Hype's source position
did not change in either run. Reports are
`TestResults/behavior-sword-attack-1790250237804405770.json` and
`TestResults/behavior-sword-attack-1790250359524316642.json`. The first test
failed because its provisional selection checkpoint required action 48; the
observed draw action was 45. The second failed because its late checkpoint
required action 134; the source had already recovered to 48. Acceptance now
requires action 45 during drawing, 134 at the early swing checkpoint, either
134 or 48 in the observed transition window, and 48 after recovery. The source
rerun passes all five Cucumber steps in
`TestResults/behavior-sword-attack-1790250567229954228.json`. The Godot
gameplay result is pending.

The sword test establishes selection, one swing and recovery on clear ground.
It does not yet establish a hit, target damage, weapon attachment visibility,
inventory state, or crossbow projectile behavior. Those need separately
observed effects and assertions. The current exporter preserves unsupported AI
operations with explicit failures, including channel presentation calls in
Hype's sword path. Implement the confirmed calls as typed model operations
compiled to Godot support methods; keep unknown gameplay effects explicit.

The first headless Godot run reached the manor Brain and stopped before the X
input on source `Proc_DeactivateChannel` (operation 105, model 28 node 885):
`TestResults/behavior-sword-attack-1790250602075227445.json`. Static source
decode identified its child as category 18, channel 72. Other reached channel
requests name channels 28, 80, 116 and 136; all five have corresponding
exported channel nodes with matching `source_channel` metadata. The source
script's HUD display calls 422–426 each have one integer Boolean operand.
They now decode as typed operations and call Godot support methods. Godot
toggles the matching channel parents and retains HUD visibility requests for
an authored native HUD. Neither the original handler's full channel side
effects nor visible HUD parity have been established.

The next headless Godot run reached the sword swing and recovery with action
sequence **47, 47, 134, 48, 48, 48**. Its only assertion failure was the draw
checkpoint, where the source reported 45 and Godot had returned to 47:
`TestResults/behavior-sword-attack-1790250994974308200.json`. No unsupported
call stopped the Brain. A source rerun using the jump suite's stopped-capture
pause compensation still reported action 45 at that draw checkpoint, ruling
out that timing-accounting difference. It reported action 135 rather than 134
at the early swing checkpoint; those phases need source action-family review
before the assertion is broadened. The pause-compensated source report is
`TestResults/behavior-sword-attack-1790251426006205933.json`.

Static action-family review showed normal behavior 29 executes actions 135 and
134 consecutively before armed idle 48. The early swing checkpoint may sample
either phase; raw IDs remain in the reports. The draw branch executes action
88 (0.45 s), then 45 (0.70 s), then requests 48. Godot action 47 at the draw
checkpoint is an actual discrepancy, so its draw assertion remains strict.
The native action request trace now records request, transition, selected
state and relevant designer fields to locate the interruption.

The trace in `TestResults/sword-debug-20260924/behavior-sword-attack-1790252367310834044.json`
showed the X path did run but requested action `-1` twice. The exported Hype
Brain's action arrays 162 and 164 defaulted to `-1` because the English-save
fixture resumed normal behavior 4 without restoring the arrays populated by
normal behavior 0. Orrery read both arrays from the original game's supplied
English save at startup and again after Down for 800 ms plus 700 ms settling;
both observations agreed. Array 162 was
`[88,45,200,201,192,194,216,220,235,236,218,221]`; array 164 was
`[48,48,47,31,31,32,92,92,33,27,27,36,63,111,82,62,112,85]`. The
original run was `run-1790252707092054741` under the local Orrery runtime
directory, using executable SHA-256 `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`
and save SHA-256 `877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The fixture now restores these values as typed Godot integer arrays and records
them in setup evidence. This is saved-state restoration, not an action-remapping
rule. The post-fix headless Godot run passed all nine assertions, with action
sequence **47, 45, 134, 48, 48, 48** and X/Space input edges at their planned
elapsed times. Its report is
`TestResults/behavior-sword-attack-1790252943191621702.json`.

The focused `python3 tools/test.py --area combat --local --offline` selection
passed **63/63** tests headless in 258.759 seconds, including its original and
Godot sword cases and mandatory safeguards. Its report is
`TestResults/20260924T123038.161616Z-development.log.json`. The fresh paired
backend reports are its `behavior-sword-attack-1790253271424542380.json`
(OpenSpace) and `behavior-sword-attack-1790253144917994468.json` (Godot).
Their recording hashes match and both sampled **47, 45, 134, 48, 48, 48**.
`TestResults/behavior-sword-attack-cross-backend-20260924.json` passes all 22
comparison checks: relative XY/heading errors and input-delivery deltas are
zero at the named checkpoints. Godot's starting height differs by roughly
0.068 source units. The comparison does not establish wall-clock or
input-consumption timing parity; Godot ran headless, so visual pairing remains
pending.

## Sword beam and weapon presentation follow-up

The visible Godot demo exposed two defects outside the first sword action
assertions: a beam lingered at the wrong origin without moving, and Hype's
crossbow appeared and disappeared during movement. The first sword assertion
set therefore remains an action-selection checkpoint, not an accepted combat
effect or visual comparison.

The generated beam is the Fix Perso `fix:/persos/5-0-24465`, object types
`[6, 8, 65589]`, with its own Brain model `fix:/ai/models/708`. Its original
normal behavior 0 writes Hype's designer fields 55, 17 and 114, requests
action 0, sets source vector speed according to Hype's designer state, then
enters behavior 1. Behavior 1 calls `GoRelative` while active and requests
`KillPerso(Me)` after 100 ms or when
`Operator_Ultra(MainActor, Cond_SeePerso(Me))` becomes false. The latter
evaluates visibility from Hype toward the beam; operation 76 is `SeePerso`,
not `SeePositionWithOffset` (operation 78). The script supplies motion and
lifetime, but does not specify the launch transform.

The importer previously left symbolic MainActor assignments unsupported, so
the compiled projectile Brain stopped before speed or motion. Typed AST,
OpenSpace codec and Godot compilation now cover these assignments, the reached
Ultra/SeePerso expression and self-kill. The spawning character binds the
projectile's Brain to Hype before it starts. `GoRelative` commits source vector
speed to native CharacterBody3D motion. These are shared AI API operations,
not a projectile-specific rule. Orrery has a checked scene-child actor
inspector so short-lived Persos can be observed within an input sequence.

Remaining fidelity work is explicit. The current native sight query uses first exported family bounds
centers and Godot's static collision layer. The original engine tests visual
geometry in selected sectors, so occlusion equivalence is unproven. The exact
`GoRelative` update timing and deletion timing also require a live comparison.
At the first beam checkpoint, the save fixture did not yet restore source
channel activation state; animation part visibility alone could not establish
that state. Static disassembly found
operations 104/105 setting/clearing bit 23 on a source channel record at
`+0x2c` while updating a per-actor byte-keyed list at `+0x158`; its rendering
interpretation needs a typed live read before the crossbow state can be fixed
with evidence. No combat hit or damage parity is claimed by this follow-up.

The next focused Godot run instrumented each generated Perso's exit in the
test harness. The same beam instance was observed running compiled normal
behavior 1, with source velocity about +35 along Y, and moving 2.33 source
units before `tree_exiting` at Brain clock 100 ms. The sight query was true in
the first captured frame and false at exit. The elapsed-time condition is
reached at exit, although the trace cannot establish when sight changed.
The report is `TestResults/behavior-sword-attack-1790256812405448354.json`.
The generator now initializes its transform before adding the
template to the scene, so its Brain cannot start while the actor is still at
the template transform. A fresh headless run passes new Godot-only assertions
for the compiled Brain, beam displacement and scripted lifetime; its report is
`TestResults/behavior-sword-attack-1790257007086710213.json`. That run
observed 1.75 source units of motion from its first frame until cleanup at
Brain clock 100 ms. These checks catch the former false green in which Hype's
action sequence passed while the beam's Brain stopped immediately.

A bounded original-game poll saw the dynamic scene child count increase from
176 to 177 during the Space attack, then return to 176 soon afterward. The
new node was not yet validated against the beam's Perso identity or sampled
for a transform during its short lifetime. Static disassembly of the exact
installed executable (SHA-256 above) does establish the generic creation
rule: handler `0x54a5d0` passes the caller SuperObject global-transform pointer
at `+0x24` to helper `0x43eb60`; helper `0x4425e0` copies the full matrix into
the new actor. For this Hype invocation, the copied transform is Hype's world
matrix. The vector operand is written into the generated actor's motion state
after creation, not added to its origin. Godot's current transform copy thus
matches the source API for this zero-vector case; the user's observed visual
launch discrepancy remains to be traced through exported beam geometry,
animation or timing. No sword-specific offset is warranted. Godot's current sight test uses
static collision geometry, while the source tests selected sector visuals, so
occlusion parity remains a native-support fidelity debt pending comparison.

Static inspection of the beam's two model parts found no ModelInstance
transform or action-0 animation offset. Their mesh vertices have asymmetric
local bounds, which can put the visible beam away from the actor origin even
when the source spawn copies Hype's transform exactly. This does not justify a
hand-specific offset in Godot support; a visual comparison of those converted
meshes and the source animation is still needed.

A later fresh Orrery run (`run-1790258146193747690`) established the saved
channel baseline before any input. At Hype Data3D `+0x158`, the persistent
deactivation list is the sorted byte IDs `[28, 71, 80]`. Each selected channel
record's `+0x2c` bit 23 agrees: 28, 71 and 80 are clear; 72, 116 and 136 are
set. After a short physical Up walk, the same IDs and bits remained. The source
build and save hashes match the fixture hashes above. The test fixture now
deactivates those three channel parents through the maintained channel API
after attaching the exported scene; original script calls can subsequently
toggle them. Movement and sword BDD checks assert this measured baseline and
its preservation during walking. This is save-state restoration, not an
exporter rule or a weapon-specific rendering workaround. The original reads
combined one stopped Hype actor capture with subsequent bounded memory reads;
they were not one atomic broad channel snapshot, so a typed Orrery channel
inspector remains useful for reusable cross-engine assertions.

The follow-up focused movement selection passed **69/69** tests in 371.84
seconds (`TestResults/20260924T142900.711323Z-development.log.json`). The
combat selection passed **67/67** in 252.67 seconds
(`TestResults/20260924T143534.289462Z-development.log.json`). Both included
their original-game and Godot BDD cases, the mandatory safeguards, source
integrity and process cleanup. A collision reconstruction probe initially
missed the newly imported sword-beam template's Fix collision profile and
geometry slot 1. Its source enumeration now includes rooted Persos, Global and
AI object templates; all imported collision profiles still reconstruct and
compare against source bytes. This corrected test discovery, not the collision
exporter or gameplay behavior.

## Native API fidelity debt

The current `SeePerso` implementation uses first exported family-bound centers
and a Godot static-collision ray. Source visibility uses selected sector visual
geometry, so the two can disagree near occluders. `GoRelative` commits speed on
a Godot physics step; its exact original-engine update timing and deletion
timing have not yet been compared. Channel activation maps the source request
to native channel-parent visibility, while the original also keeps a persistent
deactivation list and bit flag. The supplied save's measured baseline is now
restored, but rendered crossbow presentation still needs visual comparison.
These are native AI API fidelity items, not replacement gameplay rules. The
crossbow's projectile contact API remains explicitly unsupported until its
source behavior is measured and implemented.

## Crossbow AI and inventory boundary

Hype's existing model 28 selects the ranged path through its own action and
weapon conditions. Its generated arrow template uses model 733 for projectile
behavior. The current increment, decrement, vector-component assignment and
inventory-function work extends the typed AI AST, source codec and GDScript
compiler so those source rules can execute. The native `inventory.gd` Resource
only supplies inventory queries and mutation requested by compiled AI; it
contains no weapon selection, attack timing, projectile motion or display rule.
The English-save test fixture restores its measured selected weapon and item
quantities after scene attachment. This is a test starting state, not an export
constant. Orrery's typed combat inspector reads Hype and the active inventory
from one stopped source capture for a direct check of that state. The refreshed
local service verified that read in the original manor; its byte capture is
coherent while stopped but does not prove a completed engine update.
The run was `run-1790263016370512003` under the private Orrery run tree, using
executable SHA-256 `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`
and supplied-save archive SHA-256
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The original session and refreshed Orrery service were both stopped afterward.

The verified executable's functions 201, 208 and 209 traverse active inventory
handles: 201 returns the first active `Weapon` ID, 208 returns the first active
matching ID's quantity, and 209 returns the remaining quantity while only
writing it when nonnegative. The supplied save had weapon ID 80 selected and
active item IDs 21, 61 and 80, all with quantity zero. This does not establish
an ammunition-enabled crossbow shot. The cheat-controller's blue/red-arrow
branches call inventory addition, but their item operands and runtime effects
are not yet verified. In the live manor run, both `youngelf` and `oldelf` were
recognized bindings but left the active inventory unchanged through a further
two-second wait; no crossbow shot or ammunition consumption was claimed. Hype
model 28 requests arrow template
`fix:/persos/5-0-24728` with source vector `(0,-45,0)` in its firing paths;
the verified generation API's handling of this nonzero vector is still under
investigation. A source-game input and follow-up inventory capture are needed
before treating a shot or consumption as behavior evidence.

A separate bounded source run, `run-1790263609992610756`, used the same
verified executable and supplied save. The active `Weapon` ID 80 member was
resolved through the live inventory list; its 16-bit quantity was zero. A
guarded volatile-memory write changed only that member's quantity to three,
and the typed inspector confirmed it. This was a starting-state intervention,
not native ammunition acquisition or a modification of the supplied save.
After Z selection, one Space press reached Hype action 73 at 150 ms and
returned to action 48 by 400 ms. Designer field 95 became 80. Quantity stayed
three through captures at 150, 400 and 1,000 ms. The run therefore proves the
source AI entered a crossbow firing action from that test state; it does not
yet establish a spawned arrow or ammunition consumption. An independent
screenshot is not used as proof. The game and temporary Orrery service were
stopped, and the archive was left unchanged.

The verified generation handler links its new SuperObject under the
`DynamicWorld` root. The arrow template `fix:/persos/5-0-24728` has object
types `[8,10,65591]`. Orrery now exposes a bounded, checked tail of that
root's children, including each Perso's type triple and transform. The prior
combat inspector reported only Hype and inventory, so this adds the scene
evidence needed for a short-interval projectile comparison.

That follow-up ran as `run-1790264925891228728` with the same verified game
and save hashes. Before Z, DynamicWorld had 176 children; none of its last 16
matched the arrow template type. The laser point was
`[-73.77856,32.58957,6.05084]` and its presentation was inactive. Z made
presentation active without moving the point. One Space created child 176,
raising the count to 177. Its live type triple was `[8,10,65536]`, **different
from the imported arrow template's `[8,10,65591]`**. At release its transform
was still zero, consistent with a stopped snapshot before publication. At
about +30 and +80 ms it matched Hype's position
`[-74.37856,27.48474,4.55084]`; by +150 ms it had moved
`[+0.47266,+0.97266,+1.54688]`. It was gone by +300 ms, when the laser was
inactive at `[-73.77859,28.93899,6.05084]`, Hype action was 358, and weapon
80 quantity remained three. These captures establish a transient moving
DynamicWorld child but **do not identify it as the arrow**. The type change,
its AI/model identity, and the source frame/update timing need further checks
before projectile behavior can be accepted. The original game/service were
stopped and the save archive remained unchanged.

The bounded scene-tail read now also follows each Perso's Brain-to-model link
within that same stopped capture. It distinguishes a missing Brain, an
unreadable partial link, and a validated capture-local `live:*` model locator.
This prepares the next short-interval observation without reusing a process
pointer after resumption. A locator is not yet `fix:/ai/models/733`: the live
pointer-to-source-allocation address mapping must first be checked for an exact,
unique match against Fix allocation 733.

The arrow's compiled model 733 makes its own aiming decision. It reads the
global laser target direction, compares vector cosine to Hype's sighting,
optionally changes its orientation, and later checks contacts. Hype model 28
hides the pointer before creating the arrow and reactivates it afterward, so
visibility and target state must be separate. The typed AST/codec/compiler now
cover laser activation, direction, vector cosine and change-sighting. Godot
shares a native laser-target Resource with generated actors and retains its
point while hidden. No camera or viewport ray has been guessed as the source
target; the native target starts invalid, and compiled AI fails explicitly if
it is read before a target is supplied. Orrery's combat read now includes the
verified build's global laser point and visibility word. A bounded live capture
observed its target and visibility across the original firing action, but did
not establish how the engine chooses that target. Source inspection shows the
target query refreshes the stored point before checking the presentation flag,
so hiding the laser does not itself prevent a later query from updating the
target. Its ray inputs and collision rules still need translation. A fresh Rete import and
direct compilation of models 28 and 733 confirmed that model 28's firing
schedule has no unsupported source AI nodes. Model 733's later rule reaches
condition 33 (`CollideWithNothing`) and statement 160
(`Camera_ChangeTargetedPerso`). The native orientation
builder uses world up when constructing a basis; source roll and zero-vector
details are unresolved fidelity debt. Collision conditions remain unsupported.
The source target query is camera based and traverses several world object
kinds. Matching its camera ray and object filtering belongs to the later camera
work; until then, a controlled target can exercise the arrow AI in a test, but
normal Godot gameplay cannot acquire one.

Further static inspection of model 733's normal behavior 2 shows its
collision branch is `not Cond_CollideWithNothing()` with no operands. On
contact, the original AI calls `Camera_ChangeTargetedPerso` with a Fix actor
reference and the `MainActor` keyword, clears two MainActor designer fields if
field 120 is zero, then kills itself. A separate rule kills it after its lifetime counter exceeds
150. The verified executable's shared condition handler `0x533d20` reads the
actor contact result at offset `+0x20`; descriptor alignment and switch entry
for raw condition 33 require further validation before mapping native
contact. Native support should expose the
contact result needed by this compiled condition, rather than make an arrow
specific lifetime or steering decision. Camera procedure 160's remaining
reset effects and the Fix actor's role beyond selecting the rooted camera
remain unverified and belong with the later camera work. The raw
second operand is category 0, parameter 5: the `MainActor` keyword, not the
integer five. Condition 33 now has a typed source round trip and a compiled
Godot call guarded against an unknown native contact result. Its support API
returns unknown until the collision updater mapping is verified; a negated
condition cannot silently become true. A direct static Fix probe identified
the first camera operand, allocation 23322, as the valid `StdCamer` Perso:
object types `[2,2,2]`, family Fix allocation 23331 and Brain model Fix
allocation 437. The current in-memory import binds that source address to its
rooted level occurrence `level:/persos/world/0/2`, also named `StdCamer`; no
separate Fix Perso asset is expected. It decodes model 733's nested camera call
as a typed statement with this actor reference and `MainActor`. The Godot
camera API now resolves that identity to the saved `StdCamer` node and stores
the target on its native `SourceCamera`. It does not select the viewport or
move the camera. The rooted actor now retains its model-437 Brain and bounded
Perso-list operations. Its return-zero/camera-update pair now compiles to a
native API call, which still reports unsupported until the source pose update
contract is implemented.
The handler reads a mechanics contact-class word at runtime offset `+0xf8`.
The executable computes a jump-table index by subtracting 31 from its
selector; index 2 enters a Boolean mask-`0x2` test, while index 0 tests
mask `0x1`. The typed importer preserves model 733's on-disk operation 33,
so selector 33/index 2 is strongly supported, although the complete disk-to-
runtime selector copy has not been traced. The earlier bit-zero attribution
to condition 33 was a switch-alignment error. Collision processing snapshots
the previous `+0xf4` result to `+0xf0`, computes a current classified result
at `+0xf4`, and publishes `+0xf8 = +0xf4 | +0xf0`. An initialization path
clears the two inputs; the published value changes at the next aggregate
callback. The classifier uses contact-normal alignment with a caller-supplied
vector and possibly custom thresholds. Exact arrow inputs and ordering
relative to AI evaluation remain under investigation. This does not yet
justify mapping condition 33 to a Godot slide-collision count.
The selector-33 case at `0x00533dbf` specifically tests mask `0x2` (bit 1),
not mask `0x4` or bit 0. One classifier path produces class `0x2` after a
successful contact response when the normal's dot product with its reference
vector is at least 0.5 and below 0.8660254 under default thresholds. A
verified general mechanics caller supplies source `[0,0,1]` and those default
thresholds, but its use by the model-733 arrow is not yet established. An
original-game observation of that arrow's first condition evaluation is the
next check before choosing a native Godot contact mapping.

Any implementation that chooses a weapon or moves an arrow in Godot support
without a corresponding compiled AI request would be a replacement gameplay
rule and a material fidelity debt. Keep that boundary visible in review. Native
support approximations already listed above remain API fidelity debt; the
crossbow's laser target acquisition and contact calls are pending source-backed
implementation rather than a support-side attack script. The arrow's original
contact branch calls the camera procedure before clearing MainActor fields and
killing itself. Target storage now exists, but viewport selection, follow and
the procedure's other reset effects remain fidelity debt. Any test that
asserts arrow cleanup without those camera effects must report the exclusion.

The verified executable's object-generation handler copies the firing actor's
world transform into the new actor and separately writes the evaluated vector
to its movement state, caching its magnitude. The arrow's model 733 contains
no `GoRelative` call. A generic source mechanics updater reads that movement
state and integrates displacement through its ordinary collision path. The
Godot generation API now initializes the new native CharacterBody's motion
from the source vector immediately, so the compiled arrow AI can control its
subsequent aiming and lifetime. This resolves the former blanket rejection of
nonzero vectors; it does not establish arrow firing or motion parity. The
source updater does not rotate its velocity through the actor's basis in the
examined function, but the exact frame transfer from initial to active motion
and timestep still require live comparison. Those limits are native API
fidelity debt, not a weapon-specific gameplay rule.

In particular, `source_initialize_motion` currently multiplies a nonzero
generated vector by the spawning actor's basis. The verified generation
handler writes the evaluated vector directly to the new actor's motion state,
and the examined updater does not show that basis rotation. The conversion is
unverified for arrows and should be compared against a live moving arrow
before accepting it. The sword beam's generation vector is zero, so this
question does not explain its visible launch offset.

## Next crossbow behavior case

The next BDD case will use physical Z/Space from a declared quantity-three
starting state, then require a positively identified model-733 arrow and its
initial published displacement. The original backend must resolve the spawned
child's Brain/model identity; the prior child-count increase and differing
`[8,10,65536]` type triple are insufficient. Godot must show compiled model 28
requesting creation and compiled model 733 running during the claimed motion
interval. Projectile evidence is required for a pass, even if optional trace
fields are absent. The recorded source quantity remained three, so the case
will not invent ammunition use.

A controlled laser target may be supplied as an explicit test input for the
initial-motion case while normal camera-based target acquisition is
unsupported. Its exclusion must be visible in the comparison result. Contact
cleanup is a later case, because native `CollideWithNothing` and procedure
160's reset effects have not been mapped. No Godot support method should
select the weapon, schedule the shot or steer the arrow without the compiled
AI request. The current spawn-vector basis conversion is a specific native
API fidelity question to resolve before trajectory acceptance.

A further bounded original-game run used the verified English save and a
guarded current-process intervention to set the active Weapon 80 quantity
from zero to three. Orrery confirmed that starting value before and after
physical Z/Space input; it remained three. Hype was facing 180 degrees, which
would distinguish world-space from actor-relative initial velocity if an
identified arrow displacement were captured. One DynamicWorld child appeared
at ordinal 176 with type triple `[8,10,65536]`, live AI model locator
`live:0288e344`, and a partially published zero transform. A later read found
it at Hype's position, then it disappeared before any displacement away from
Hype was measured. The type differs from the imported arrow template
`[8,10,65591]`. An offline scan of verified Fix block `(17,0)` places model
733 at payload offset `0x4e324`; subtracting the corroborated live block base
`0x02840020` from the captured model locator `0x0288e344` yields exactly that
offset. The source model has three normal behaviors and no reflex set, matching
the live observation. This strongly supports model-733 identity, though this
run did not read the active block-table entry directly and the changed third
type word remains unexplained. No displacement sample establishes the arrow's
initial-motion frame or launch trajectory. The exact responses are in ignored
`TestResults/arrow-vector-live-20260924.jsonl`; the game and Orrery service
were stopped and the supplied save was unchanged. The offline mapping evidence
is in `TestResults/fix-block-17-0-ai-allocation-scan-20260924.json` and
`TestResults/fix-model-733-live-locator-crosscheck-20260924.json`. Do not
change the shared Godot vector conversion or mark the crossbow launch case
passing from these captures.

A subsequent single-request Orrery burst removed most MCP round-trip delay.
After the same guarded quantity-three setup and Z/Space input, 18 checked
scene-tail captures over about 69 ms all saw the same strongly identified
model-733 child, but every transform remained the unpublished zero matrix and
its integrated velocity was zero. The individual captures took roughly
3–4 ms, leaving too little resumed time for a verified source update between
them. This adds identity and sampling-cadence evidence, not a trajectory.
The next bounded sample needs deliberate resumed intervals across source
frames and at least two valid post-publication transforms; an unmodified
zero-transform series cannot justify a Godot vector-space change.

That delayed follow-up used 15 ms resumed intervals and captured a moving
model-733-correlated child. At source update stamp 3234 it was at
`[-73.86670,29.41665,6.08331]` with integrated velocity
`[1.38605,0,-0.51021]`; at stamp 3235 it was at
`[-73.77245,29.41665,6.04862]`. The displacement
`[+0.09425,0,-0.03469]` agrees directionally with that velocity. Earlier
stamp-zero or Hype-origin transforms were excluded as unpublished setup.
The child had live model locator `live:0288e344`, three normal behaviors and
the same unexplained runtime type triple `[8,10,65536]`. This is source
evidence of a moving arrow, not yet a clean test of whether the generation
vector `(0,-45,0)` is world-space or actor-relative: model 733 can reorient
the shot before this sample. Weapon 80 quantity remained three. The raw
capture is ignored `TestResults/arrow-vector-delayed-burst-20260924.json`;
the game/service were stopped and the installed saves still matched the
supplied archive. Leave the generic Godot vector conversion unchanged until
its frame is independently resolved.

A fresh disposable Godot export contains compiled Hype model 28 and arrow
model 733. In a bounded test with measured save inventory overlaid to weapon
80 quantity three, a declared valid laser target and a throwaway preselected
crossbow state, physical Space ran model 28's action-73 firing path and
generated one actor whose Brain identifies `fix:/ai/models/733`. The arrow
had nonzero initial velocity and a changed position by the 33.333 ms contact
check. At that check its Brain stopped on the explicit unknown result from
`source_collide_with_nothing()`. There were not two consecutive valid motion
samples before the failure, so this does not establish a passing crossbow BDD
case or normal playable firing. The normal BDD case still needs physical Z
selection, rather than the throwaway preselection used to isolate the arrow
path. The test-owned wrapper parsed and loaded the fresh exported scene;
Godot exited normally. The ignored result is
`TestResults/manoir-crossbow-control-corrected-20260924/result.json`.
Implement the contact condition only from the original collision-result
contract, using a narrow Godot API mapping. Do not insert projectile-specific
movement or return a fabricated no-contact value to advance the script.
The test wrapper's separate `crossbow_launch` starting-state overlay was
validated in the same running Godot process: it changed only the declared
weapon-80 quantity to three and set a valid retained laser target before
restarting Hype's Brain. Hype remained in action 47/normal behavior 4 with
designer field 59 false, so the future case still has to select the crossbow
through physical Z. A subsequent plain fixture reload restored quantity zero,
an invalid zero target and no overlay. The fixture file hash was unchanged;
the ignored validation report is
`TestResults/crossbow-overlay-reload-retry-20260924/result.json`.

## Repeatable source model identity

A static loader trace found the module-block table in the pinned patch/decomp
image. Module 17's pointer slot at `0x00880f64` leads to 16-byte block records;
block 0's first two words are the active payload base and inclusive capacity
end. Its loader applies the active-base minus SNA-base delta to addresses in
that block. This gives Orrery a route to compare a live child Brain model
pointer with model 733's source payload offset `0x4e324` without assuming a
fixed heap address. The installed executable has the same PE layout and
identical block initializer, lookup and loader bytes at those addresses as
the patch image, so the slot VA is valid for the hash-pinned live build. Its
value is runtime zero-fill data; the present trace still has not read the
active entry in a running installation. Verify its current base and bounds,
then resolve
the child pointer and block base within one checked capture. The earlier
subtraction against an inferred base remains corroboration, not a repeatable
BDD identity assertion.
Orrery's scene-tail observation now carries the checked active `(17,0)`
block-relative model offset when the model pointer falls inside that block.
Synthetic coverage verifies the table and range calculation; a live capture
must still confirm that model 733 reports `0x4e324` before the BDD case can
use it as a passing identity assertion.

A subsequent one-shot original-game probe verified the new Orrery
`prepare_crossbow_quantity` operation in the hash-pinned `manoir` session.
It resolved the active Weapon 80 quantity in one stopped capture, guarded its
old value zero, set only the current process to three and confirmed that value
through the typed inventory inspector. Physical Z/Space created a new
DynamicWorld child 176, with runtime types `[8,10,65536]` and a Brain model
locator matching the prior arrow candidate. Its captured update stamp was
zero, so neither published motion nor the new block-relative model identity
was checked in this sample. A debugger batch armed selector-33 breakpoints,
but Orrery's input path paused the process for its own observations; GDB saw
the attach-related `SIGSTOP` and detached before either breakpoint fired.
There is no live condition Boolean or mechanics contact word from this run.
The game, service and debugger were stopped; all installed save members still
match the archive. Details are in ignored
`TestResults/arrow-contact-debugger-20260924.md`. The next probe should read
the actor's mechanics contact fields through a checked stopped-byte Orrery
path while the input controller runs without a competing debugger.
The source mechanics helper resolves the checked scene child through
SuperObject `+0x04` to Perso, Perso `+0x08` to dynamics wrapper, and wrapper
`+0x10` to dynamics object. Its returned base is object `+0xd8`, so the
condition's `+0x20` word is object `+0xf8`. Orrery's existing scene-tail
dynamics observation now reads object `+0xf0/+0xf4/+0xf8` together as previous,
current and AI-visible contact-class words, along with the selector-33
mask-`0x2` result and whether the published word equals their union. A stopped
capture can land between stores; an inconsistent triple is recorded as such,
not silently discarded. This is a read-only observation of source state and
does not yet establish which value a prior AI evaluation consumed.

The first no-debugger capture using that observation verified the three-arrow
starting quantity, then pressed Z for 120 ms and Space for 100 ms without a
gap. Across eight 15 ms scene-tail samples, the child count stayed at 176;
no model-733 candidate appeared. The unrelated baseline child resolved to Fix
`(17,0)` offset 8024, so its contact words cannot be used as arrow evidence.
The run ended cleanly with the supplied save unchanged. Its ignored raw result
is `TestResults/arrow-contact-live-20260924.json`. Earlier successful shots
used Z for 55 ms, a 100 ms or longer gap, then Space for 55–80 ms. The
no-spawn result does not establish the condition's value or polarity.

A single cadence-matched retry used Z for 55 ms, a 100 ms gap and Space for
80 ms after the post-load settle. The new child 176's checked Brain model
resolved to Fix module 17, block 0, payload offset `0x4e324`: active base
`0x02840020` plus that offset equals its live model pointer `0x0288e344`.
The first published update (stamp 2298) had contact words `[0,0,0]` and
velocity `[1.3863,44.9757,-0.5103]` at
`[-73.8893,28.9979,6.0916]`. The next (stamp 2299) had `[4,4,4]`,
clear selector-33 mask `0x2`, vertical velocity zero and position
`[-73.8519,29.4167,6.0778]`; the child was absent at the following sample.
The two published samples establish the live model identity, displacement and
contact transition, but no breakpoint observed the condition's actual return
or the reason for removal. In particular, a zero mask in a post-update capture
cannot by itself prove which prior value the AI evaluated.
The raw ignored capture is
`TestResults/arrow-contact-retry-20260924-sequence.json`. Orrery and the game
were stopped and all supplied save members retained their original hashes.

The verified executable's generic mechanics callback `0x00585d51` looks up
an environment card with a downward query and passes that card's `+4` vector
to `0x0059a504`; that function calls the contact classifier `0x00597be6`
with the vector and a separate threshold descriptor. The generic defaults
include dot-product boundaries 0.8660254 and 0.5 for class `0x1`/`0x2`
paths; a later response can produce class `0x4`. The live arrow reports a
cached type-11 card, whose dispatcher callback is `0x0058a43e`, rather than
the examined generic callback. Its actual classifier vector, thresholds and
path to the published `0x4` word remain unresolved. This static trace does
not justify a Godot slide-count, floor or wall predicate for condition 33.

## Initial crossbow launch BDD (2026-09-25)

The new `Launching one crossbow arrow initially` Cucumber scenario uses a
declared three-arrow starting state in both backends. Physical Z selects the
crossbow; the recording observes armed action 48 before pressing Space. A
shorter first recording pressed Space while the original was still in draw
action 90 and correctly failed to find an arrow. The checked recording now
samples armed idle at 1,000 ms and presses Space at 1,100–1,180 ms. Orrery
captures a bounded, 25-stop scene-tail burst around the press and release;
Godot retains its generated actor frame trace. Each backend requires the
model-733 identity and two distinct positions of the same instance. The
source assertion also requires increasing published update stamps.

The original run passed all four assertions. Child 176 resolved to Fix module
17, block 0, payload offset `0x4e324`; published stamps 294/295 moved from
`[-73.82520,29.41665,6.06801]` to
`[-73.71429,29.41665,6.02718]`, a distance of 0.118 source units. The
supplied save remained unchanged and cleanup completed. Evidence:
`TestResults/original-crossbow-retry-20260925-evidence/behavior-crossbow-initial-launch-1790305617632821672.json`.

The Godot run passed the same recording. The generated model-733 Brain was
running in its first retained frame and the arrow moved 5.25 source units by
the next recorded frame. That next frame records the Brain stopping on the
explicit unsupported contact result. This is evidence for initial launch and
movement, not later AI behavior or trajectory parity. The Godot result is
`TestResults/crossbow-godot-revised-20260925-evidence/behavior-crossbow-initial-launch-1790305744520811514.json`.
The displacement magnitudes differ substantially over the sampled intervals;
those intervals are not paired frame by frame, so this is an open trajectory
fidelity issue, not a measured speed ratio.
The recording hashes match, and `orrery compare-behavior` passed the paired
initial-launch scope in `TestResults/crossbow-initial-comparison-20260925.json`.

Weapon 80 quantity remained three in the original run but became two in the
Godot run. The test deliberately has no cross-backend ammunition assertion.
The Godot start still supplies a declared laser target because camera-based
acquisition is unsupported. Contact evaluation and the arrow's subsequent
cleanup remain unresolved, as does the source frame for its generation
vector. The Godot test's first import took about 200 seconds; consider
reusing the generated project for compatible scenarios while keeping
fixture and runtime state explicit. None of these gaps should be hidden by
support-side firing or projectile scripts.

## Selector-33 debugger probe (2026-09-25)

Static disassembly of the pinned executable places selector 33's bit test at
`0x00533dbf`: it tests `EAX & 0x2` and stores the resulting Boolean through
the pointer at `[ESP+0x14]` before `0x00533dcf`. The handler calls a helper
that reaches the dynamics object's AI-visible contact word at `+0xf8`.
The handler's `ESI` operand was initially assumed to be a SuperObject, but a
later live trace put it inside the Fix block. Subsequent static review showed
that `ESI` is handler argument 2 and its selector is read from `[ESI-8]`;
it is not established as an actor pointer. The static record is ignored
`TestResults/arrow-selector33-static-20260924.txt`.

One bounded live run loaded the verified save, established quantity three,
selected the crossbow with physical Z and observed action 48 twice before
attaching GDB. It then sent physical Space for 80 ms. The exact case and
post-store breakpoints were armed, but no checked model-733 invocation or
Boolean was recorded. The JSONL contains only the arming record, so the run
cannot distinguish a missing projectile from an unobserved condition call or
a filtering/callback fault. No contact mapping is justified by this result.
The game was stopped, the installed saves retained their hashes, and no
screenshots were taken. Ignored evidence:
`TestResults/arrow-selector33-luna-20260925-033521.jsonl` and its `.gdb.log`.

The next probe needs per-breakpoint raw-hit and callback-error counters plus
an independent, statically verified arrow insertion witness. Only then can a
missing selector-33 result be assigned to spawn, branch reachability or the
identity filter. The existing BDD comparison remains limited to initial
launch and movement.

A subsequent bounded run added the post-insertion witness at `0x0054a72e`.
The returned SuperObject was a new child under the dynamic world root, and
its Brain model pointer resolved to Fix module 17/block 0, model 733 at
offset `0x4e324`. The witness succeeded once, while the selector-33 case and
post-store breakpoints had zero raw hits. This rules out a missing spawn in
that run, but gives no contact-condition Boolean. Evidence is ignored
`TestResults/arrow-selector33-spawn-case-luna-20260925-034937.jsonl`.

One more bounded run broke at handler entry `0x00533d5a`: 118 raw hits,
no case/post-store hits, and no callback or memory-read errors. Two logged
entry samples selected 31 and had `ESI=0x02880f04`, inside the live Fix
block. The existing filter compared `ESI` to the witnessed SuperObject and
rejected all 118 hits. That comparison is not established as a valid actor
identity test, so its rejection count does not show whether the arrow entered
the handler. The raw zero selector-33 hits show that this branch was not
observed during the capture window. A subsequent static argument review
confirmed the selector read from `[ESI-8]`. The third handler argument
resolves a mechanics pointer; at handler entry `EAX` holds its `+0xd8` base,
and `[EAX+0x20]` is the contact input. An actor-specific check can compare
that base with the witnessed arrow's dynamics object plus `0xd8`, then follow
selector 33 to the actual Boolean. The exact model-733 condition node is
another possible address filter once its ownership is verified. Evidence is
ignored
`TestResults/arrow-handler-entry-spawn-case-luna-20260925-035313.jsonl`.
The argument review is ignored
`TestResults/selector-handler-argument-static-luna-20260925.txt`.
Both runs detached and stopped cleanly, preserved SaveGame0 hashes, and used
no screenshots.

A read-only Fix decode located the `CollideWithNothing` node in model 733's
normal behavior 2, rule 0, at module 17/block 0 offset `0x4ea08` (wire bytes
`21 00 00 00 00 00 03 01`). The previously witnessed runtime block base
derives a node address of `0x0288ea28`; the selector handler should receive
`ESI=0x0288ea30`, since it reads the selector at `[ESI-8]`. This address has
not yet been observed at the breakpoint. Another operation-33 node exists in
normal behavior 1 at offset `0x4e7e8`. The ignored static evidence is
`TestResults/arrow-condition-node-static-20260925.json`.

A compact reread of the passing original crossbow trace found the spawned
model-733 child in normal behavior 0 at every observed sample. It initially
shared Hype's root transform, including translation
`[-74.378601, 27.485542, 4.550852]`; at the first observed movement its
update stamp was still zero. At stamps 294 and 295 it had moved away from
that origin, but normal behavior remained 0. The Godot projectile root also
began at Hype's root transform within about 0.02 source units. These samples
do not support adding a muzzle offset to the generation API. The visible
starting point could instead involve mesh-local placement or early motion;
compare root and mesh transforms at matched update stages before changing
spawn placement. The source/Godot displacement intervals are not paired, so
their magnitudes do not establish a speed ratio. The ignored extraction is
`TestResults/crossbow-launch-geometry-analysis-20260925.md`.

The generated arrow's retained velocity magnitude is 45 source units per
second; 5.25 units is seven 60 Hz physics intervals at that speed, matching
the reported physics-frame gap 708 to 715. This is a plausible timing
explanation for that Godot displacement, not proof that the source and Godot
trajectories match. The original sample at 1214 ms is already about 1.9 units
from insertion while its published update stamp remains zero.

The same paired report gives a more specific control-flow lead. Godot's
model-733 Brain is in normal behavior 0 at physics frame 708 with clock zero,
then switches to normal behavior 1 at its first 33.333 ms AI clock (frame
715) and stops on the explicit unsupported contact result. The original
child remains in normal behavior 0 at update stamps 294 and 295. This is a
plausible behavior-transition timing or compilation mismatch to resolve
before mapping condition 33. The current samples do not establish whether
the source would transition later, or whether the observed source and Godot
update phases are equivalent.

Read-only inspection of the generated model-733 script explains the Godot
transition: its normal-0 schedule executes `WaitFrames(2)`, lowered to two
`normal_schedule_tick` waits, then calls `change_normal(1, false)`. Normal-1
rule 0 immediately evaluates `CollideWithNothing` and exposes the existing
unsupported result. The shared support advances schedule ticks with native
physics frames, and the zero-time `start()` does not emit a wait tick. No
extra Godot tick is apparent in this code path. The original samples show
two published post-spawn update stamps but do not align source AI ticks to
Godot physics ticks; changing `WaitFrames` or contact semantics from these
traces would be premature. A source-backed WaitFrames cadence and a denser
first-update comparison are needed.

Static review of the pinned source executable identified
`Action_WaitNbFrames`' updater. With counter 2, its successive callbacks
decrement 2→1 and 1→0 while reporting continuation; a third callback sees
zero and reports completion. The disassembly does not establish whether the
first callback happens in the activation update or the following update, nor
when `ChangeBehavior` commits relative to that callback. Godot's two awaited
schedule ticks may therefore match the source or be one update early; the
source callback phase must be measured before changing the compiler. The
ignored evidence is `TestResults/waitframes-source-static-20260925.txt`.

An initial hardware-watchpoint attempt first guarded the live arrow's scene
list identity and incoming link. Its insertion breakpoint fired once, but
the candidate failed the kind/parent/model-733 identity gate, so no watchpoint
was armed and no unlink was observed. The callback omitted the rejected
candidate values; a read-only postmortem could narrow the rejection to that
gate but not identify its failing predicate. A retry must log each bounded
candidate and named predicate before deciding whether the arrow spawned.
This run used no screenshots, stopped the owned processes, and preserved
SaveGame0 hashes. Ignored evidence:
`TestResults/arrow-unlink-watch-luna-20260925-041507.jsonl`.

The instrumented retry logged the rejected candidate: SuperObject
`0x03e0fe34`, kind 4, under the expected dynamic root, with model pointer
`0x0288e344`. It is the same model-733 arrow identified by the earlier
checked capture. The probe's new identity helper mistakenly treated the
module-17 table slot as the block base, missing one pointer dereference;
it computed a false base and rejected the valid arrow before checking links.
No watchpoint was armed, so the removal path is still unknown. The recorded
Z-to-Space interval was 1586.9 ms, later than the passing BDD's 898 ms;
the arrow still spawned. Save hashes and cleanup passed, with no screenshots.
The ignored evidence is
`TestResults/arrow-unlink-watch-luna-20260925-042425.jsonl`. Reuse the
previously verified table lookup and test it against known addresses before
another debugger capture.

After correcting that lookup and passing a synthetic table-indirection
regression, one final bounded capture verified the model-733 SuperObject and
its incoming DynamicWorld sibling link. A hardware watchpoint saw the
predecessor's `next` slot at `0x03cb557c` change from the arrow
`0x03e0fe34` to zero at instruction `0x004fcd91`. This proves the arrow was
being detached from that sibling chain. The debugger callback attempted a
count-consistent traversal immediately after the pointer store, before the
routine decremented the parent's child count, so it failed at a transient
in-list state and did not serialize registers or a call stack. Static
disassembly places `0x004fcd91` inside a generic SuperObject reparent
routine: it removes the old-parent links and count, then links the object
under a caller-supplied new parent. This capture does not prove destruction.
The destination parent and caller are unknown without the missed register
and stack capture. The run stopped and cleaned up, retained all save hashes,
and used no screenshots. The cause of its disappearance from DynamicWorld
still needs a caller or later state observation. Ignored evidence:
`TestResults/arrow-unlink-watch-luna-20260925-042759.jsonl` and its
`.gdb.log`; static review is
`TestResults/arrow-unlink-pc-static-review-20260925.txt`.

Further static inspection of the same pinned routine established a safe
target for one diagnostic stop: at `0x004fcd91`, `EAX` is the moving
SuperObject, `[ESP+4]` is the caller return address, and `[ESP+0xc]` is the
required destination parent. Multiple callers use this routine with
different destinations, so the instruction address alone does not identify
why the arrow moved. Recording those values before any list traversal would
distinguish the destination and immediate caller. The static evidence is
ignored `TestResults/arrow-reparent-stack-static-20260925.txt`.

A reviewed callback then captured one complete mid-reparent stop without
traversing the inconsistent list. The verified model-733 arrow was
`0x03e0fe34`; the hardware watchpoint saw its predecessor `next` pointer
change to zero at `0x004fcd91`. `EAX` and the first argument matched the
arrow. The second argument at `[ESP+0xc]` was `0x03bd182c`, equal to the
arrow's **old DynamicWorld parent**; the return address was `0x0044a610`,
the DynamicWorld-root wrapper. Static caller review confirmed that this
wrapper explicitly passes that root to the generic reparent routine, which
unlinks and reinserts the actor at the destination list tail.
No register, stack, watchpoint or backtrace read failed, though GDB's
best-effort backtrace stopped at frame 0. This is a same-root remove/reinsert,
which does not explain its later
absence from the sampled scene tail. The capture stopped cleanly, preserved
SaveGame0 hashes, and used no screenshots. Z-to-Space was 1924.1 ms versus
the passing BDD's 898 ms, so timings are not paired. Ignored evidence:
`TestResults/arrow-unlink-watch-luna-20260925-043633.jsonl` and its
`.gdb.log`. These hexadecimal values were recalculated from the raw JSONL
integers; an initial verbal summary misconverted them. Corrected caller
evidence is ignored
`TestResults/arrow-unlink-root-reinsert-static-20260925.txt`.

## Shooting comparison leads (2026-09-25)

The paired initial-launch reports show a real Weapon 80 quantity difference:
the original remains three after the model-733 arrow appears and advances
through update stamps 294 and 295, while Godot changes three to two. The
compiled Hype model-28 firing path asks native inventory function 209 to
remove one from the selected weapon only if `fix:/persos/15` Global Brain
field 14 is zero. Both backends select Weapon 80, so checking a different
item does not explain the difference. The original behavior capture does
not read Global field 14; its live value and the branch taken remain unknown.
The ignored read-only analysis is
`TestResults/crossbow-quantity-discrepancy-analysis-20260925.md`. The Godot
behavior harness now records Global field 14 in snapshots and generated-arrow
frames as a diagnostic; compare it with a typed, source-identity-checked
Orrery read before changing inventory semantics or fixture state.
The focused headless Godot crossbow BDD passed 1/1 tests and all five steps
with cleanup complete after this diagnostic change. Global field 14 stayed
false in the baseline, selected, flight and all 15 retained arrow frames;
Weapon 80 changed from three to two only during flight. Its ignored evidence
is `TestResults/weapon-channel-field14-godot-20260925/`.

Static inspection of the source `WaitFrames(2)` updater proves two continuing
decrement callbacks and a third completion callback, but does not link its
action context to the spawned arrow or establish whether the first callback
occurs during activation. Two independent Luna static reviews stopped before
a live breakpoint because a matching countdown or shared model code would
not prove actor identity. The unresolved edge is the action-context owner
chain from the verified model-733 Brain. Ignored evidence:
`TestResults/waitframes-arrow-prereq-luna-20260925.txt`,
`TestResults/waitframes-arrow-identity-review-luna-20260925.txt` and
`TestResults/waitframes-action-context-trace-20260925.txt`. Godot's two
native schedule ticks may already match the source's observable wait;
do not add an extra tick without measuring invocation phase.

For weapon presentation, existing source evidence covers the saved
deactivated channels `[28,71,80]` and a short Up walk, while existing Godot
crossbow evidence shows channel 71 switching false→true at selection.
There is no paired source channel capture for selection, nor a verified
channel-to-rendered-weapon mapping. A sequential original/Godot
idle–walk–run–selection capture with channel IDs, active bits and owning
rendered nodes is needed to reproduce the user's observed crossbow flicker.
The ignored inventory is
`TestResults/weapon-channel-visibility-analysis-20260925.md`.

## Typed Crossbow Global observation (2026-10-01)

Orrery now identifies the active Fix Global by matching its exact Perso pointer
in ActualWorld, then follows its Brain/Mind/model references before reading
designer field 14 as a typed Boolean. In the paired combat run, the matching
actor was at scene path `[0,0]` (the actor at `[0,1]` was a different Perso).
The verified module-5/block-0 payload began at `0x314`, had a 20-word
allocation, and referenced the module-17/block-0 model at `0x1c`. Field 14 was
false at baseline, after selection, during flight, and across all 25 retained
arrow-burst captures. This agrees with Godot's field-14 value, so that field
does not explain the quantity difference: original Weapon 80 stayed at three;
Godot changed from three to two.

The lower-rate source AI snapshots show `normal.current` moving from 4 to 17
while the arrow was present and back to 4 afterward. Both backends reported
the selected weapon as 80, firing action `dsg95` as 80 during flight, and
`dsg130` as 0. This supports comparable firing-state entry, but does not prove
whether source native inventory function 209 ran or what it returned/wrote;
the exact MainActor Brain assignment target also remains unverified. Do not
change ammo semantics until that call or mutation is observed. The model-733
contact result and arrow lifetime also remain unresolved, so full Crossbow
acceptance is still open.

The 2026-10-01 focused combat run passed both original and Godot initial-launch
BDD cases, including the original displacement assertion. The suite result was
97 passed and one unrelated failure in
`openspace::level::ai::tests::source_ai_inventory_and_script_reconstruction`
(58 observed versus 57 expected). A later two-case diagnostic run passed the
Godot case; its original case spawned the arrow but failed the displacement
assertion while eight AI fields were being collected on every burst sample.
Burst sampling is now back to field 59 only to preserve the earlier passing
capture cadence; the richer fields remain on lower-rate snapshots. That final
sampling adjustment has not been runtime-verified.

An elevated, two-case rerun on the committed checkpoint passed the Godot
initial-launch case in 122.2 seconds. Its original case passed setup, arming,
spawn and model-733 identity, but failed the displacement assertion. In that
capture the new child was at Hype's root transform in burst observations 4–6;
its integrated velocity and contact words were all zero, and it was absent
from the sampled tail at observation 7. A follow-up after restoring the
lower-rate branch fields still passed the field-14 and owner-AI assertions,
but reproduced this no-displacement result. By contrast, the earlier passing
capture of the same input recording first showed movement at observation 8,
later recorded velocity `[1.3864, 0, -0.5103]` and contact class 4, and measured
1.8874 units of displacement. That class does not set selector-33's `0x2`
mask. The captures establish that launch and early movement vary between
source runs; neither one proves the AI contact branch ran or why the child
left the tail. Original quantity remained three. The reports are
`TestResults/crossbow-followup-20261001-elevated/behavior-crossbow-initial-launch-1790838161172678457.json`
and
`TestResults/crossbow-followup-20261001-final/behavior-crossbow-initial-launch-1790838244235252812.json`.

An Orrery diagnostic now searches the complete bounded ActualWorld hierarchy
for actors whose Brain/Mind chain points to the checked active Fix model-733
allocation. This distinguishes leaving the DynamicWorld tail from leaving the
live scene. In a fresh physical Z/Space run, the original arrow was found at
scene path `[0,176]` in two captures 59 ms apart, with the same transform
`[-74.3785, 27.4839, 4.5508]`; the full 317-node search found no model-733
instance in the next capture 10 ms later or in the remaining burst. Thus it
was removed from ActualWorld during that interval, rather than merely
reparented elsewhere. This run failed the displacement assertion, so it does
not establish normal launch motion. It also does not establish whether source
contact cleanup or explicit AI deletion removed the actor. The focused test
has passed with displacement in other runs, preserving the observed
run-to-run variation. Report:
`TestResults/crossbow-world-search-20261001/behavior-crossbow-initial-launch-1790839603661099883.json`.

The focused Godot trace then reached model 733 node 4 at about 33 ms and
stopped on condition category 1 operation 28. Raymap's Hype condition catalog
names operation 28 `Cond_CollidePersoZDENoWithTypeZDE`; the imported Rete nodes
are a keyword (4), constant (1), and mask (1). Disassembly of the pinned
patched executable (SHA-256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`) shows
the generic condition path reaching `FUN_00524420` and then
`FUN_0042f210`. That helper either resolves a collision set or selects one of
the actor's collision zones, then uses `FUN_0042f2a0` and
`FUN_0042f4e0` to collect geometric intersections with active characters.
This is not a synonym for `Cond_CollideWithNothing` or a count of the last
`CharacterBody3D` slide contacts.

The current Godot character exporter only instantiates the ZDM movement group
(`src/godot/character.rs::movement_collision`); it does not expose the ZDE
geometry required by this condition. The node's three operands have not yet
been matched to the helper's zone/collision-set parameters, and target-type
filtering is not available on those exported shapes. A direct boolean mapping
would therefore make arrow AI proceed on an unverified result. The attempted
`get_slide_collision_count() == 0` mapping was removed; `CollideWithNothing`
again fails visibly while its source contact-class producer remains unknown.
Focused Godot evidence is
`TestResults/crossbow-contact-20261001/behavior-crossbow-initial-launch-1790840376462423854.json`:
the arrow launches and moves, then its Brain stops at this op28 gap. Ammo
parity remains three in the original and two in Godot. Crossbow use is not
accepted until the ZDE query and contact result are represented natively and
the live ammo discrepancy is traced to its source inventory call.

The decoded Fix Rete package does contain the arrow's source ZDE data:
`fix:/persos/5-0-24728` (family 8, AI model 733) uses
`fix:/collision-profiles/24780` as collision group 1 (ZDE). Its geometry slots
are `207949` and `207957`; state 0 selects zone 0, and state 2 selects zone 1.
Both zones are source boxes spanning `[-0.1, -0.55, -0.1]` to
`[0.1, 1.55, 0.1]`, with collision identifiers 2 and 4096 respectively.
Thus the missing data is specifically the Godot projection and query, not an
OpenSpace/Rete import gap. The model-733 expression's `Me`, constant `1`, and
mask `1` operands are consistent with selecting the arrow's first zone, but
their exact mapping to the helper's arguments remains unverified; do not
assume a zone-index convention from this asset alone.

### Operation 28 native query progress (2026-10-01)

The generic condition call resolves the operands more narrowly than the initial
trace established. In `FUN_00524420`, the node's actor and zone operands become
`param_1` and `param_3` to `FUN_0042f210`; the node's final mask operand becomes
its result-capacity argument. The helper passes the fixed `DAT_00721b60`
material filter, selects zone `param_3 - 1` from the actor's ZDE group, and
collects up to the requested number of intersecting characters. Its boolean
result is whether at least one match was collected. Thus this node's `Me`,
constant 1, and mask 1 are source actor, first zone, and one-result capacity.
The fixed source material-filter rule is still not represented in Godot.

The Rust parser now preserves this typed condition, the exporter projects ZDE
profiles into native `Area3D` shapes, and compiled AI calls a direct-space
intersection query. This is progress toward the condition, not acceptance:
the query does not yet apply the source material filter, and the focused live
Crossbow case still needs to show the arrow survives the condition, resolves
contact/lifetime correctly, and matches source ammunition behavior.
