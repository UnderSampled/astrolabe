# Hype control implementation

Work toward the ordered [gameplay plan](./godot-gameplay-plan.md), begun
2026-09-14. This record distinguishes component checks from a playable player.
The full control sequence is not yet implemented.
Current development output uses Rete schema 25 and Godot support interface 17;
the intervening identifiers belong to the archived experiment and are skipped
to prevent falsely accepting incompatible preserved local assets.

## Player Brain export gap — 2026-09-23 audit

The native exporter has a real saved `Brain` node path. The two Astrolabe torch
persos exercise it, but the normal `manoir` conversion does not attach that node
to Hype. In the captured `TestResults/hype-controls-manoir-port.log`, source
diagnostics identify the controlled actor as `level:/persos/world/0/0` and report
its `fix:/ai/models/28` as unsupported. The current importer scans every script
and designer type in an AI model, then skips the whole model if any item is
unsupported. This leaves `Perso.brain` absent; the Godot exporter returns without
creating a Brain node for that perso. This audit used the captured conversion
report and current code; it did not regenerate a `manoir` project.

The all-or-nothing gate is an implementation defect for the current development
rule: an unsupported script or designer field elsewhere in the model should not
prevent supported behavior and the owning Brain from being represented.

The source turn/walk and input-boundary probes demonstrate useful component
work, not a saved player Brain. They compile selected real rules to `rules.gd`,
instantiate Hype's exported character, and add that script as a test-created
Brain with fixture state. The normal exported `manoir` scene therefore has no
player AI path to run or compare. A passing input-consumer probe establishes
that selected source rules can operate on native character resources after test
setup; it does not establish that ordinary conversion wires those rules into
the saved scene.

The [suite framework](./realtime-comparison-plan.md#ordered-delivery-and-exit-evidence)
is the next overall implementation step. Its first gameplay case needs a verified
live Orrery original-game backend and should drive partial AI import on the Godot
side: retain the controlled actor's model and Brain identity,
selection and useful instance data through import and Rete, with stable
behavior/rule/field identities and explicit unsupported slots. A missing source rule,
an unsupported source rule and a supported rule must remain distinguishable.
Save a visible Brain node in the ordinary Hype perso scene and compile a bounded
executable dependency closure. Do not skip an unsupported rule that could affect
the chosen outcome; implement it or make the case fail visibly. Verify node/script
presence in the saved scene before runtime, then run a press–move–release–stop
case without test-time Brain insertion. Preserve unresolved source information
for investigation and report unsupported reconstruction separately. Keep the
remaining initialization, reflex, movement, equipment and service gaps explicit;
a structural node alone does not make Hype playable.

## Input boundary and focused validation — 2026-09-20

The first comparison-harness slice uses Hype's unchanged complete turn/walk rules
with its exported native character, animations and maintained support. The rule
fixture exports the transitive closure of its 14 referenced actions; it still
verifies all 389 source state identities, and complete scene tests retain full
native family coverage. This reduces a 252 MB unrelated animation setup without
removing any rule, action, pose, reconstruction or edited-value assertion.
The Godot 30-second probe and 60-second export deadlines are unchanged.

A post-character physics callback supplies physical events for the next native
physics tick. A separate compiled query observer verifies held/pressed/released
edges, while the actual source rule's rotation verifies consumption by gameplay
code. Four profiles exercise 48/60 Hz, each with ordinary execution or injected
80 ms main-loop/render stalls. These include consecutive one-tick pulses, release
edges, a Shift chord and multiple physics ticks between render loops. Thirty
neutral physics ticks establish readiness before the recording origin, excluding
asset-load catch-up from measured gameplay time. No gameplay clock or recorded
source delta is injected. Delayed-input and no-op rule faults must fail.

This is a **component boundary proof**, not full initialization/reflex, movement,
contact, real-time parity, or synchronized image acceptance. The 48 Hz comparison
candidate reflects the original investigation's ordinary 20/21 ms updates;
repeatable original native replay and timing-envelope qualification remain open.
The original English save was freshly loaded into `manoir` via the preserved MCP
on 2026-09-20, with the pinned executable hash, then the owned game was stopped.
Its setup evidence is in the external harness's
`local/runs/20260920-222522/events.jsonl`. The fresh running read showed raw/clamped
25/25 ms; it was not a completed-update snapshot.

Fresh evidence and the remaining runtime/coverage work are recorded in
[testing](../docs/astrolabe/testing.md) and the active [comparison plan](./realtime-comparison-plan.md).
The earlier failing logs below remain preserved.

## Paused implementation checkpoint — 2026-09-20

Gameplay expansion is paused while improved structured comparison/testing is
developed. This checkpoint preserves the current code and its unresolved tests;
it does not complete the input-driven walk/run milestone. Resume this work from
the checkpoint rather than the older archived runtime experiment.

Beyond the dated evidence below, the working tree includes complete source
idle-turn/walk rule probes, multiply/negate/subtract-assignment and native turn
support, plus typed character collision profiles/activation, Rete persistence,
source reconstruction and Godot sphere/box/point conversion. The source collision
probe covers 28 per-perso sets, 27 shared profiles and Hype's 389 movement
selections. Its live-shape expectations come from prior research, not a new
live-game run at this checkpoint.

Fresh validation is recorded in
`TestResults/realtime-harness-checkpoint-20260920.log.json`: **failed**, exit 124,
570.119 seconds. The log records 84 passing tests and one explicit failed test
before the existing whole-suite deadline stopped later coverage. Child and
temporary cleanup completed, with no leaked or remaining processes. The full
scene/direct/Rete/editor checks passed in 276.35 seconds before the cutoff.
This is not a complete passing suite or a performance benchmark; release-build
and Clippy checks also ran during part of this invocation. The new
`source_hype_turn_and_walk_rules_drive_native_character` test prints
`HYPE_NATIVE_RULES_OK`, but the Godot process then exceeds its 30-second deadline.
This is a failed test, including teardown; the success marker does not override
the result. Preserve and investigate the failure as part of test supervision
work. Do not increase the deadline or remove the test merely to obtain a pass.

`cargo fmt --check`, `cargo clippy --locked --offline --all-targets -- -D warnings`
and `git diff --check` pass. The locked offline release build passed in 37.63
seconds. Clippy initially found `chunks_exact_mut(8)` in the new probe; the
checkpoint makes only the mechanical `as_chunks_mut::<8>().0` substitution.
No gameplay implementation was changed during checkpointing. The full suite was
not rerun after that lint-only substitution.

Full initialization, remaining movement/reflex dependencies and native
environment/gravity semantics remain open. The rule probe explicitly supplies
initial state and exercises selected rules; it is not full player acceptance.
The historical sections below retain their observation dates and limitations.

## Movement boundary

The next request being implemented is animation-driven walk/run with native
floor/wall contact. Hype's converted decisions must select the actions; no
replacement hand-written player controller is introduced.

Raymap's `AnimFrameMontreal` identifies frame translation/rotation pointers but
does not interpret their motion. In the matching decompilation, `00435930`
integrates frame-local translations at the selected state's playback rate and
returns velocity; `004436c0` submits this to character mechanics. The useful
`AnimationFrame.motion_translation` already preserves these values through Rete.
The Godot exporter now bakes their integral into native position tracks. One
source unit remains one Godot unit; `(x,y,z)` maps to `(x,z,-y)`.

The maintained character is a `CharacterBody3D`. Its `AnimationPlayer` extracts
motion from a saved `RootMotion` node; only physics applies that displacement to
the body's world position, through `move_and_slide`. Force/impulse velocity stays
separate from animation displacement. Gravity is supplied explicitly by the
environment rather than assuming a value for the original game. Motion remains
opt-in until the real behavior and collider path is connected. Editor seeking
and ordinary pose inspection remain stationary.

The native motion test uses the actual exporter/support, direct and closed/reopened
Rete routes, and a changed source frame displacement. It checks fractional
intervals, whole loops without cue tracks, state playback rates, backward seeking,
rotated movement and floor/wall contact. Its synthetic collision sphere is now
exported from a useful collision profile along with state-dependent activation;
it does not establish Hype gameplay parity.
Early runs exposed a native full-loop extraction edge case: an advance from
time zero to a full loop wraps to the same time and yields zero displacement.
Bounding native advances to half a clip lets Godot observe both sides of the
wrap. This uses native resources, not source-frame evaluation at runtime.

## Original-game observations

Current comparison run:
`/home/stephen/Projects/hype-harness/local/runs/20260914-003635/`.
The persistent Hype MCP loaded English SaveGame0 normally into `manoir`.
Executable SHA-256:
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
Save archive SHA-256:
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The baseline used keyboard input and memory reads only, with no cheats,
navigation, memory writes or saves. Live actor, transform, Brain, and behavior
set references were resolved afresh from the controlled actor entry.

Initial controllable idle was normal behavior 4, reflex 3, with 30/50 mana and
spell 0, near the gate/save point. An 800 ms Down hold moved Hype backward from
source Y approximately 27.49 to 25.65. Key injection and observation overhead
are present; this is not a speed measurement.

A subsequent 35 ms Ctrl press produced stationary jump behavior 21. Sixteen
position samples at roughly 60 ms intervals show ascent and descent, with a
largest sampled height of 7.0991 source units versus a starting height about
4.5496. Landing was observed by approximately 1.2 seconds after the pre-input
clock read. These are sampled observations, not exact takeoff/apex/airtime
measurements. The original body remained at the same horizontal position until
the landing contact adjustment. The external run's `events.jsonl` retains the
input/read timings and screenshot paths.

The source's jump variants require more than interpreting their names.
`0058cc2d` rotates all three impulses through actor orientation. Events 6/7 clear
accumulated velocity; event 8 retains it. Event 6 also adds recent measured
motion (`004444d0`, a 0.1-second window). `JumpAbsolute` therefore does not mean
an unrotated world-space impulse. Sector DNM gravity and mechanics ratio govern
acceleration. These observations guide the later jump slice; jump helper and
trajectory parity are not yet demonstrated.

## Native level checkpoint

A fresh direct export to `output/hype-controls-20260914/` produced 4,908 files
in 76.985 seconds. The disposable probe loads the saved `manoir/level.tscn` and
its saved Hype instance, then supplies the measured save position and facing as
an explicit probe fixture. This is not full save import or player initialization.

Native states 47 (idle), 32 (walk) and 33 (run) render distinct poses. At 0.16
seconds, with their native 30 fps timing, local extracted motion was respectively
0, 0.428673 and 1.675961 Godot units along Z. These are phase-dependent samples,
not an average speed ratio. Screenshots `motion-state-47.png`,
`motion-state-32.png` and `motion-state-33.png` are retained in that disposable
project. The original 200 ms Up input selected normal behavior 5 and animation
state 78 (walk entry), changed position, then continued moving during the stop
transition. The forced native state probe does not reproduce that complete flow.

With the level collision bodies active, a two-second walk probe reached
`(-74.15075, 4.693386, -30.15221)` from
`(-74.378, 4.549549, -25.64676)` and remained grounded. A subsequent left-wall
approach ended at `(-77.74963, 4.551559, -27.43105)` with floor and wall contact.
The probe supplied downward acceleration 20 solely to exercise contact; this is
not a measured original gravity or jump-trajectory claim. The first contact
probe was invalid because disabling the entire scene removed its physics bodies;
the corrected probe stops script processing individually. Logs are in
`TestResults/hype-controls-manoir-motion-2.log` and
`TestResults/hype-controls-manoir-wall.log`.

## Validation in progress

The motion/collision/action component checkpoint passed 23 focused tests in
88.791 seconds (`TestResults/native-action-integration.log`). A complete local
run then exceeded its enforced shared deadline at 570.226 seconds, with no
assertion failure before termination; it is recorded as **failed**, with complete
child and temporary cleanup (`TestResults/20260914T050054.087534Z-1510522.log.json`).
Release compilation consumed 167 seconds. The test runner now uses a validation
profile with release semantics, optimization level 1 and no ThinLTO; production release builds keep
ThinLTO. The subsequent complete suite passed 90 tests in 468.895 seconds,
including a cold 47.38-second validation build, with complete child and temporary
cleanup (`TestResults/20260914T053629.872643Z-1763998.log.json`).

The compiler now supports named button expressions, family-checked action
references, native designer arrays/properties and action-completion coroutines.
Hype's full initialization, required rule/reflex flow, shared actor behavior,
turning and environment gravity remain unfinished. Neither
jump nor combat/magick is connected. Slice 1 of the gameplay plan remains open.

The compiled action/input checkpoint passed 24 focused tests in 103.560 seconds
(`TestResults/compiled-native-action-physics-edge.log`). Direct and reopened-Rete
native files agree; a changed Brain array index selects action 0 instead of 1,
and the native coroutine resumes after the transition, extra cycle and target
animation complete. The source checks reconstruct 164 numeric fields in each of
three Hype memory buffers and resolve 27 logical input names. This remains
component/codec coverage, not player-controller completion.

That check found an inherited-node serialization issue: Godot saved random
`parent_id_path` recovery hints alongside valid parent paths. Normalizing node
IDs left those hints stale. Export now omits the optional hints and preserves
explicit native parent paths. It also caught a test-input timing mistake:
Godot records an injected press for the following physics tick, which the
corrected test now observes. No custom input frame counter was introduced.

## Source initializer and native input

The 102 actual top-level action-array assignments in Hype's normal behavior 0
now decode into typed element assignments and compile to native GDScript writes.
The source fragments reconstruct byte-for-byte; all seven arrays retain family
ownership through Rete. A source fragment test runs the generated initializer
against the real imported 389-state family and verifies independent instance
arrays. This is explicitly a fragment of initialization, not a complete Brain.

A fresh English-save comparison in external harness run `20260914-014307`
resolved the controlled actor, family list and designer memory anew. All 102
initialized values agree, including action-array 164's idle 47, walk 32, run 33
and backward 36. The run used normal save loading and memory reads only; it has
the same executable/save hashes above and starts with 30/50 mana, spell 0.

Generated button queries adopt missing native InputMap controls: arrows,
Shift, Ctrl, Space, X, Z and Enter for the ten named controls in the gameplay
plan. Existing project entries, including empty disabled entries, are preserved.
Additional source logical controls still require authored bindings and warn if
missing. This adoption is in memory at startup; it does not rewrite project
settings. Developers can override the same action names in Project Settings.
Automatic Brain startup is deferred until after its character's initial pose,
with cancellation guarding a pending start.

The initializer test passed. The first physical-input test queried events still
in Godot's input buffer and failed. The corrected test delivers buffered events,
then verifies physical Up, an authored F attack binding, an intentionally empty
sword binding, and automatic initial action selection. It passed both direct
and reopened-Rete routes in 21.505 seconds
(`TestResults/native-input-delivery.log`). These are component checks: running
manoir still does not yet provide the real Hype input controller.

The behavior-flow work must preserve normal 4/5/6/7 rule priority while
interrupting their blocking action schedules. These movement rules contain no
meta-action waits; reflex 3 rule 7 contains a one-frame inventory wait, which
must remain explicit. Source `005227c0`, `00522840`, `00522930` and `005229d0`
confirm rule priority and cancellation of a schedule's suspended action.
This must become readable generated behavior flow using native callbacks and
coroutines, without restoring the archived generic scheduler or source cursors.

That native flow is now implemented. Each compiled behavior directly calls its
ordered rule coroutines and schedule. Native coroutine stacks retain a blocked
rule's place even if an earlier rule becomes true meanwhile. A blocking rule
invalidates only its schedule's current action; earlier assignments and branches
do not run again. Reflex meta-actions defer normal work on entry, continuation
and completion, without resetting a normal wait's captured deadline.

The codec resolves behavior operands to owning model/group/member references.
An adjacent `SetActionReturn(1)` and own-behavior request becomes one meaningful
yielding transition. Other return-policy patterns fail explicitly. The compiler
emits direct behavior requests and native awaits; no source return flag,
instruction cursor, generic task or operation dispatcher is retained. Requests
to the other behavior group leave the caller active. Same-state requests retain
the current behavior. A real change invalidates and releases the old coroutine
immediately, including when the destination has no schedule.

Matching source evidence is `005216e0`/`005426f0` for behavior replacement and
same-state handling, `005427c0` for own/cross-group requests, `00522584` and
`005225d1` for reflex meta-action priority, and `0054daa0` for frame-count waits.
`00542e60` plus the rule/schedule callers distinguishes rejected action requests:
a rule pass stops its lower rules but permits the schedule; a schedule retries
that action request. This is compiled flow around native action playback.

The focused native fixture covers ordered suspension, interruption, changed wait
duration, reflex completion priority, immediate/deferred entry, cross-group
requests and cancellation. A separate source fixture decodes and reconstructs
Hype normal 4's complete rule 0, round-trips it through Rete, and checks its
deferred transition to normal 41 in Godot. The fixture compacts variable 56 only
after byte reconstruction and uses an explicit destination marker; it does not
implement behavior 41 or substitute for the full player Brain.

The same fresh run also sampled physical movement after native keyboard input:

| Sequence | Source position Y | Normal behavior / animation state |
| --- | --- | --- |
| After backing away from the save point | 25.33804 | 4 / 47 |
| Up held 300 ms, immediately after the input call | 25.63409 | 5 / 32 |
| After the walk stop transition | 26.21962 | 4 / 47 |
| Shift+Up held 500 ms, immediately after the input call | 28.51692 | 6 / 33 |
| After the run stop transition | 28.52009 | 4 / 47 |

The pre/post input game clocks span 549 ms for the walk call and 684 ms for
the run call, including injection/observation overhead. These samples confirm
action choice, displacement and stopping behavior; they do not establish a
steady-state speed ratio. The run screenshot was captured after the animation
state read and can show a later transition pose.

A later idle-turn sample in the same still-owned process used Left for 250 ms,
then waited 600 ms after release. Source position stayed exactly
`(-74.3783646, 28.5200882, 4.5689011)` across all three transform reads. The
horizontal basis rotated by approximately 40.1600 degrees and stayed there;
normal behavior remained 4 and the settled animation was 47. A separate guarded
read confirmed unsigned-byte designer field 107 at its declared offset contains
160. Input-call overhead made the command take 463 ms; it is not 463 ms of held
input. Screenshot `screen-1789367202040171562.png` in the external run shows the
settled facing and pose. No memory intervention was used.

For the next turn helper, the actual idle rule 2 supplies field 107 (or its
negative), with a 1.5 multiplier for the fast turn branch. Source procedure 214
at `00542bb0` converts degrees to radians and submits turn request 3 through
`0052ef40`; `0058cc2d` supplies that angular request to mechanics. Procedure 215
also divides by the current frame duration and must not be conflated with 214.
The measured 40.16-degree response supports a 160 degrees/second ordinary turn.
The normal angular integration at `005a0642` multiplies the requested radians
by elapsed seconds before composing orientation. Native parity still needs
checking before that helper is claimed complete. Complete idle rule 2 is a
184-node source fixture needing only multiply, negate and `Turn` beyond the
currently supported calls; complete walk rule 1 is a 50-node fixture needing
subtract-assignment. Those are the next bounded conversion steps.

Concrete unresolved dependencies in the selected source path:

| Source behavior | Required conversion/API work | Why it matters |
| --- | --- | --- |
| Normal 0 initialization | Actor properties, vectors, visibility/channel configuration, shared actor behavior requests, inventory/HUD/audio/particle initialization and the one-frame wait | The action-array fragment alone cannot initialize the player. |
| Normal 4 idle schedule | Sighting vector/component expressions, increments and random idle choice | The original schedule updates facing/idle state before entering its blocking action. |
| Normal 4/5/6/7 rules; normal 30 stop | Turn request, arithmetic, floor/slope/material queries, shared actor properties and equipment services | Behavior transitions now compile; the remaining calls must select and resume the actual actions. |
| Reflex 3 | Time/position queries, shared actor decisions and inventory services | Native priority/wait cancellation is checked; the actual reflex calls must still preserve control locks and interrupts. |
| Sector environment and selected mechanics | Physical environment gravity/up direction and state gravity ratio | Constant probe gravity cannot establish the original movement/jump behavior. |

Raymap has no corresponding Hype sector mechanics-environment parser. Engine
`0059f0b8` allocates 48 bytes; `00577850` identifies gravity at +0, vertical at
+4/+8/+12, viscosity +16, density +20, liquid motion +28/+32/+36, volume kind
+40 and toxicity +44. +24 remains unresolved. `005a1948` multiplies environment
gravity by the selected mechanics ratio (+0x58) and applies it opposite the
vertical. These are findings for the next native physics binding, not a new
physics-environment codec or a claim that these semantics are implemented.

## Earlier validated integration checkpoint — 2026-09-14

The latest complete local corpus/Godot suite passed **94 tests in 451.614
seconds** (`TestResults/20260914T062627.071582Z-2088262.log.json`), including
27.54 seconds of compilation, all native scene/editor checks and complete
child/temporary cleanup. The new source idle rule and native flow/cancellation
checks pass alongside the action-array, input and contact regressions. The native
Scene dock exposes 2,759 preview rows and 2,758 level rows, including 150 source
nodes, 108 persos and 899 parts; native editor animation seeking also passes.
The preserved `godot/project.godot` still has the recovery copy's SHA-256
`1c4979a2209af1eb51dd5650ace5f03d9eff438eabd68ae8f3c780c57bbfaa9d`.
The locked offline release build passed in 37.62 seconds. Formatting, strict
all-target Clippy and diff whitespace checks also passed. The goal remains active:
the full initialized Hype controller, jump, weapons and magick are unfinished.

The preceding complete suite passed **91 tests in 469.441 seconds**
(`TestResults/20260914T055405.086137Z-1855899.log.json`), including compilation,
all native scene/editor checks and complete child/temporary cleanup. There were
no timeouts or leaked children. `cargo build --release --locked` passed in
37.89 seconds; `cargo fmt --check`, strict all-target Clippy and `git diff --check`
also passed. The goal remains active: these checks validate the implemented
components and source fragments, not a playable Hype controller or jump/combat.
