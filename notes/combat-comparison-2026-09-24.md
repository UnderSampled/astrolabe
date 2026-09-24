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
crossbow's nonzero generated-object vector and projectile contact API remain
explicitly unsupported until their source behavior is measured and implemented.
