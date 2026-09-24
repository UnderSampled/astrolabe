# Jump comparison checkpoint — 2026-09-24

The stationary and moving jump scenarios now pass independently in the
original game and Godot. The moving jump passes coarse functional comparison.
The stationary jump's action timing remains unaccepted.

## Original-game observation

Orrery loaded the supplied English save into `manoir`, waited ten seconds for
startup, then used physical Down for 800 ms and settled for 700 ms at a clear
grounded position. The executable SHA-256 was
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`;
the save SHA-256 was
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
Orrery run `run-1790239089721908440` ended cleanly.

Baseline action 47 was at source position
`[-74.37798, 25.42120, 4.55046]`. A physical `Control_L` pulse requested for
100 ms changed to action 2 and rose. One timed sequence produced these samples;
offsets include input and capture overhead, and inspection pauses the game for
a coherent read:

| Sequence offset | Action | Source Z | Observation |
| ---: | ---: | ---: | --- |
| 365 ms | 2 | 4.8432 | ascending |
| 464 ms | 2 | 5.5943 | ascending |
| 613 ms | 2 | 6.3852 | ascending |
| 811 ms | 2 | 6.9059 | airborne |
| 1059 ms | 4, secondary 3 | 6.1701 | descending |
| 1406 ms | 0 | 4.5481 | near the starting floor |

The key step returned at 304 ms despite a requested 100 ms press. The largest
sampled gain was 2.36 source units. The exact apex, input-consumption frame,
takeoff time and landing time remain unmeasured. The screenshot is in
the private Orrery run directory under
`reference`-independent local harness data; it is not a completed-update image.

The first attempted moving sample was inconclusive. Up first reached a wall;
further Up+Ctrl produced action 37 with no vertical gain. A later clear-route
probe confirmed 0.34 units of walking progress in 300 ms, but released Up
for inspection before sending a new Up+Ctrl chord. It again produced action
37 and negligible height gain, so it did not test uninterrupted walking. Both
owned runs stopped cleanly.

The `original_moving_jump_and_landing` Cucumber scenario now keeps Up held
from 200 to 700 ms and pulses Ctrl from 500 to 600 ms. The verified left-turn
setup faced the clear route without moving Hype. One original-game launch
passed five steps and 13 assertions in 26.69 seconds, with clean shutdown:
`TestResults/behavior-moving-jump-1790246743207529398.json`. At 450 ms Hype
was walking in action 78, 0.207 units forward and level. At 800 ms the
unarmed moving-jump action 7 had risen 1.721 units and moved a further 1.612
units along the walking direction. At 1100 ms action 70 was 2.328 units
above baseline. The 1900 ms and settled samples were idle 47 near the floor,
0.074 units above the initial floor and without late drift. Source sample
captures were pause compensated and physical input edges landed at their
planned active times.

The first Godot moving-jump run passed the takeoff and airborne Cucumber
steps but failed its settled state. Its floor sample had retained horizontal
external velocity; the actor slid 1.032 units and rose along a slope to a
0.434-unit height error by the last sample. That run used one launch and
cleaned up; report `TestResults/behavior-moving-jump-1790246842775712689.json`.
The native character now clears the airborne jump impulse on floor contact,
leaving subsequent walking to animation root motion. Updating only the
maintained support script in the same disposable project passed Godot's
headless editor scan. The moving-jump BDD then passed all five steps and
all assertions in 47.04 seconds with one launch and clean shutdown:
`TestResults/behavior-moving-jump-1790247007407800142.json`. At 800 ms
Hype used moving-jump action 7, had risen 1.973 source units and moved
1.516 units forward from the walking sample. It returned within 0.019
units of initial height, with native floor contact, zero external velocity
and no late drift. Comparing the independently passing reports with the
existing source-informed walk-entry/action-32 phase check passes all 36
functional checks:
`TestResults/behavior-moving-jump-comparison-20260924.json`. The largest
sampled relative-height difference is 0.461 source units, below the coarse
1.0-unit limit. Input edges and recording hashes match. This accepts the
functional moving jump at the stated tolerance, while exact input-consumption
timing and completed-render pairing remain open. Original-game samples use a
pause-compensated schedule, which does not establish source update time or
wall-time parity with Godot.

The new wall-jump scenario reuses the mapped manor corner. A physical Left
250 ms turn faces the wall, Up 3200 ms reaches it, and Down 700 ms backs Hype
to a grounded start. A diagnostic chord probe released Up between commands
and could not establish airborne wall contact; it is not BDD evidence. The
timed-input BDD keeps Up held from 200 to 700 ms and pulses Ctrl at 500–600 ms.
Its second original-game run passed all five steps and cleaned up after one
launch in 35.48 seconds:
`TestResults/original-wall-jump-luna-rerun-20260924/behavior-wall-jump-1790248541559707277.json`.
Hype began at `[-78.6239, 30.8277, 4.8536]`, walked 0.205 source units before
takeoff, rose in action 7, and reached the mapped wall while airborne. The
sampled minimum clearance was +0.022 units on the angled face and +0.0006
on the cross face; both stayed outside the established -0.08-unit penetration
limit. At the high sample action 7 had reached 7.4137 Z; descent used action
8. Hype returned to idle 47 near the wall without drift. The first run also
reached the wall and settled, but its 0.092-unit short pre-jump walk missed the
open-route scenario's 0.1-unit threshold. The wall case now has a 0.05-unit
minimum to assert real approach without conflating that short approach with
the unobstructed moving-jump path. The headless Godot wall-jump BDD also passed
all five steps and 21 assertions in one launch, with native contact against
`world/1/5/0` at the high and descent samples, native floor return, stable
idle and clean shutdown:
`TestResults/behavior-wall-jump-1790248631885494866.json`.
The independent-result comparison passes every relative position, height,
heading and input check. Its largest relative XY and height errors are 0.125
and 0.253 source units. The first comparison failed two exact action checks:
at the high sample the source was in action 7 and Godot in 70; at descent
the source was in 8 and Godot in 5
(`TestResults/behavior-wall-jump-comparison-20260924.json`). The Godot slope
query had treated a steep wall contact with up-normal component 0.262 as a
slope despite `is_on_floor() == false`. It now gates inclined-floor reporting
on native floor contact. A fresh Godot run passed the same wall-jump BDD with
high action 7 and descent action 8, matching the source. The final headless
run passed all five steps and 22 assertions, including direct observations
that both ground and slope queries are false at the high and descent samples
despite named native wall contact:
`TestResults/behavior-wall-jump-1790249314015594559.json`.
The corrected comparison passes 35 of 36 checks. Godot remains in action 5
at the 1900 ms floor sample while the source is already idle 47; both are
idle by the settled samples. Report:
`TestResults/behavior-wall-jump-predicate-comparison-20260924.json`.
No action comparator has been weakened, and sampled action-timing parity
remains unaccepted.

The original Hype condition handler at `0x00533d20` dispatches condition
selector 34 (`Cond_CollideWithSlope`) by testing bit `0x4` in a resolved
collision/category word at mechanics offset `+0xf8`. It does not inspect a
normal or apply an angle threshold itself. `FUN_00443f00` publishes the
aggregate, while `FUN_0059a504` can fill it from `FUN_00597be6`'s classified
contact response. That classifier compares the contact normal with a supplied
vector and configurable thresholds; the vector's exact meaning and the full
bit-producer set remain unresolved. Godot's native floor gate removes the
demonstrated steep-wall false positive, but its exact source equivalence is
still provisional.

The repeatable `original_stationary_jump_and_landing` Cucumber scenario passed
all five steps and ten assertions in 25.26 seconds. Its ignored local report
is `TestResults/behavior-stationary-jump-1790240969569836351.json`.

| Recording time | Action | Height above baseline |
| ---: | ---: | ---: |
| 0 ms | 47 | 0 |
| 550 ms | 2 | 1.653 |
| 800 ms | 2 | 2.343 |
| 1100 ms | 4 | 1.770 |
| 1600 ms | 0 | near 0 |
| 2100 and 2500 ms | 47 | near 0, stable |

Physical Down for 808 ms and 700 ms settling established a clear grounded
start. Ctrl pressed at 200 ms and released at 300 ms. Stopped-byte capture
does not prove completed-update timing; the report records capture intervals,
fixture and executable hashes, delivered input and clean original-game stop.
A separate bounded GDB trial saw one event 7 followed by repeated event 8
calls at the source mechanics callback `0x0058cc2d` during a Ctrl pulse. That
does not identify the effects of individual jump procedures. The trial's
game process was detached and stopped cleanly.

## Reached AI path and implementation boundary

The current normal export retains Hype's saved Brain and Global binding.
The source model is `fix:/ai/models/28`. Idle normal 4 rule 1 tests jump
just-pressed, `Cond_IsTypeOfGMTCollide(128)`, and ground/slope contact.
Its stationary branch enters normal 18. That schedule reads source time and
position, chooses the jump action, then requests `SetVectorSpeed`, `GoRelative`,
`SetImpulse` and `JumpAbsolute` in order before changing to normal 21. The
moving branch can reach normal 2 and uses `SetImpulse` with
`JumpWithoutAddingSpeed`. Reached arithmetic and normal 21 landing rules are
promoted to typed AI AST nodes and compiled into GDScript. The Godot side owns
focused native methods for source gameplay semantics; the exporter does not
replace the branch with a hardcoded jump or interpret the source AI tree at
runtime.

The first BDD assertion can check action 2 plus a measured rise and later
return near the original floor with stable idle. Godot can additionally report
native floor contact. Apex and airtime remain diagnostics until observation
windows are tight enough. SourceSession resolved two shared physical records
in `manoir`: gravity 17.0 and 9.81, both with vertical `(0, 0, 1)`. All 29
sectors list the gravity-17 ENV first; six list gravity-9.81 second. These are
48-byte allocations with no outgoing relocations. The second has density
1000 in its retained record. Hype's selected state mechanics ratio is 1.0
for jump action 2, 2.5 for action 3 and 1.8 for action 4. The source height is
consistent with a gravity-17, 9.5-upward-impulse ballistic estimate, but
active environment selection and impulse timing still need direct proof.
Dynamic selection of the second ENV is outside this dry-route checkpoint.
The collision material predicate and exact jump-call effects likewise need
resolution before asserting cross-engine jump physics parity.

## Current Godot checkpoint and open dependency

The headless Godot stationary-jump run reaches Hype's landing behavior
(`fix:/ai/models/28`, normal 102) with action 4 and native floor contact. Its
last observation places Hype at source
`[-74.37827, 25.62776, 4.52945]`, with zero external velocity and a floor
normal close to upright. The saved Brain then stops at source AI node 468:
`Action_ExecuteActionAndGenerateObject`. The run records no completed jump
samples because the behavior step checks for a live Brain before capture. See
the ignored local report
`TestResults/behavior-stationary-jump-1790242640787068544.json`; its one Godot
launch cleaned up. Earlier headless runs separately exposed and prompted typed
support for actor-field reads, elapsed-time checks, and particle-generator
switches on this landing route.

Node 468 has three source operands: Fix block `[5,0]` allocation 24656,
`fix:/families/1` state 0, and vector `(0,0,0)`. Earlier imports retained
the first as a generic source identity rather than an identified template.
An offline SourceSession lookup identifies allocation 24656 as an unrooted
Perso record with family index 7, object types `[7,9,65590]` and Brain model
Fix block `[17,0]` allocation 724. None of the 245 rooted `manoir` scene
nodes refers to this Perso. A pre-finish AI dependency scan now imports it as
`fix:/persos/5-0-24656` with family `fix:/families/7`, Brain model
`fix:/ai/models/724`, configuration `fix:/configurations/24658` and initial
state 2. The full current-schema `manoir` import completed in 21.78 seconds
with 4,312 assets. The source allocation identity stays at the OpenSpace
boundary; the useful AI call holds a Perso reference and distinct action and
vector values.
Family 1 state 0 resolves to animation `fix:/animations/148` and mechanics
`fix:/mechanics/13916`, but neither proves the object's type, spawn timing,
placement, or whether this invocation creates an object. Static disassembly
of the installed executable (same SHA as the passing original BDD run) shows
handler `0x0054a5d0` evaluates a current/requested-action eligibility gate at
`0x004356e0`. Its false branch exits before the apparent creation helper at
`0x0043eb60`; a separate helper at `0x0054a110` polls action status. A bounded
non-stop GDB trace of the original supplied-save jump reached the handler for
main Hype: the gate returned 1 at `0x0054a632`, and the creation helper returned
nonzero handle `0x03e0fe34` at `0x0054a72e`. After detach, Hype returned to
action 47. This proves that this landing invocation takes the generation path;
the handle's exact object identity, placement, lifetime and action timing
remain to be measured. A native Godot creation and action-wait implementation
is now under test; passing it alone will not establish the unmeasured lifecycle
details.

The first dependency scan finds 24 unique object-producing Perso templates
referenced from 176 rooted actors plus Global. One unrelated template uses
mechanics kind 11. The source's `DNM_fn_xIdentityCardCreate` case `0xb`
allocates a pointer-free 20-byte card; Astrolabe retains its 16-byte body as
an explicitly unresolved mechanics value and checks byte reconstruction. This
does not claim to implement kind 11 dynamics. The Godot action/object API
loads a converted native template, requests the specified action, instantiates
once and waits for native action completion. It currently supports only a zero
vector and provisionally places the generated scene at Hype's current world
transform. Those choices are implementation probes, not accepted source
placement or lifecycle semantics; a missing template or nonzero vector fails
explicitly.

A second bounded original-game trace confirmed a returned handle with record
kind word 4, linked at `+0x18` to Hype's rooted node and at `+0x1c` to the
creator's context pointer. The creator was passed selector 9, matching the
template's second object-type value, plus Hype's node field `+0x24`.
`FUN_0053f820` then received that handle and a vector; the handle's direct
header did not change, while the function's local decomp indicates a write
through a nested vector getter. The action status helper `0x0054a110` was
called 13 times with the same handle. This supports a native created object
with polled action completion, but it does not yet establish scene placement,
render visibility, lifetime, or the status return sequence. The source process
was stopped and no Wine/GDB residue remained.

A narrower trace at the status helper's actual return sites captured AL values
`1, 1, 1, 1, 1, 4` for the same created handle. The final value occurred
about 282 ms after creation in that run. A later read found the handle address
still readable but its owner and context fields cleared. This supports a
finite action wait; it does not yet establish whether the generated object was
freed or reused, or where it appeared. The supplied save and executable hashes
matched the passing original scenario, and the trace process stopped cleanly.

An initial export of that Rete import generated 5,913 Godot project files in
228.62 seconds, including 24 unrooted `Object_template` PackedScenes. The
generated Hype model 28 script contains a native path to the family-7
template, and Godot's headless editor import, script parse and direct
PackedScene load passed. The imported template is not added to the saved
level's initial scene tree.

A fresh export with native action wait and generation completed in 185.32
seconds. Reusing that project, the headless
`godot_stationary_jump_and_landing` Cucumber scenario passed five steps and
all 12 assertions in 46.35 seconds, with one launch and clean shutdown. The
report is `TestResults/behavior-stationary-jump-1790245135271193343.json`.
Godot rose 2.111 source units by the 550 ms sample and 2.560 units at the
800 ms sample, then returned within 0.0015 units of baseline and remained
idle, grounded and stationary. The native landing action generated exactly
one object. Hype's Brain remained running with no reported failure. The
exported project has 5,913 files; its parser/resource checks passed.

The first `compare-behavior` run on the two independently passing reports
produced `TestResults/behavior-stationary-jump-comparison-20260924.json` and failed three of 29
functional checks. At 800 ms the source action is 2 and Godot's is 3; at
1600 ms the source action is 0 and Godot's is already idle 47. At 1100 ms,
Godot is 1.323 units lower relative to its start than the source, above the
current 1.0-unit comparison limit. The other 26 checks passed, including
input delivery, horizontal motion, heading and the rise/high height samples.
Those differences need investigation before adjusting a limit or accepting
cross-engine parity. The two backends do not claim exact input-consumption
timing, and visual pairing is pending.

The original report's baseline, 550 ms and 800 ms observations took 68, 82
and 70 ms of wall time respectively; Orrery pauses the original process for
the typed actor capture, while Godot keeps processing during its observation
request. A bounded sparse-capture probe confirmed roughly 41–56 ms of
stopped-world time per original capture, but its input call overran the
requested duration by 226 ms and did not reproduce the jump schedule. It
therefore cannot establish how much of the cross-engine difference is
measurement overhead. Keep the comparison failing until the timing basis is
qualified with a controlled input schedule or source game time.

The stationary-jump Orrery runner now records the measured stopped interval
for each actor capture and schedules later inputs and samples against elapsed
game-running wall time. A fresh original scenario passed all ten assertions
in 25.45 seconds; its report is
`TestResults/behavior-stationary-jump-1790245784223175473.json`. Input edges
were delivered at active 200/300 ms, and each sample records a 63–77 ms
capture pause. At 550, 800 and 1100 active ms Hype was 1.951, 2.382 and
approximately zero units above baseline, with actions 2, 2 and 4. It was
idle 47 by 1600 ms. This timing basis is an explicit approximation: process
suspension is measured, but the source engine's update clock and any catch-up
after resume have not been read.

The comparison against the unchanged Godot report now passes 29 of 30
checks. The 1100 ms height error fell from 1.323 to 0.447 units and the
floor action matches. The remaining failure is at 800 ms: source action 2
versus Godot action 3, with relative height error 0.178 units. Report:
`TestResults/behavior-stationary-jump-comparison-active-time-20260924.json`.
The action transition window needs a direct source/Godot measurement before
deciding whether this is an acceptable timing difference or a native behavior
bug; the comparison remains failed.

An experimental stationary recording added 700 and 900 ms captures. Both
backends passed their own nine-sample BDD, but the added observations changed
the transition sequences. The original's 800 ms action became 4 instead of 2
(`TestResults/behavior-stationary-jump-1790247286894506192.json`); Godot's
became 2 instead of 3
(`TestResults/behavior-stationary-jump-1790247368410201831.json`). Those
runs diagnose sensitivity to sampling and are not replacement acceptance
recordings. The seven-sample scenario remains in the suite. The source
mechanics step clamps wall-time delta after an observation pause, so
subtracting stopped time from the runner schedule does not reconstruct an
unobserved trajectory. A follow-up should use separate trials with one
terminal capture each and fixed wall-time input edges before comparing the
transition window. In particular, no exact action-timing parity is claimed
from either the seven- or nine-sample results.

Other saved actors still report explicit unsupported AI calls during this
test. They have not stopped Hype's jump path; their gameplay effects remain
unverified. The generated object's identity, placement and lifetime have not
yet been compared with the original. Moving jump has coarse functional
cross-engine acceptance; stationary jump still has one failed action check.
