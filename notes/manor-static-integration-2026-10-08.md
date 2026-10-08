# Isolated Manor static integration — 2026-10-08

This branch starts at accepted main
`78a1c861a41c3ec5ba2780ad3eef5e41ee060fc9` and lives at
`output/worktrees/integration-manor-static-20261008`, branch
`integration/manor-static-slices-20261008`. It is a lightweight Git worktree:
no local game assets, generated content, caches or reference submodules were
copied or initialized. Kip owns sequencing and acceptance. The published main
checkout and its support77/Rete58 generated Godot content remain untouched.

No build, compilation, lint, formatter, test, syntax check, Godot, original-game,
probe, export or runtime command was run during this integration. Statements
below describe source review, not executed verification. The tunnel worker's
earlier build/check report belongs to its isolated original revision only.

Current source union: **Rete60/support80**, including settled training,
actor-owned suppression, the primary source-camera owner, and the Proc166
evaluation-order correction. Earlier findings below are dated integration
checkpoints; the final follow-on section records which they resolve.

## Exact provenance and order

All contributions are cherry-picked with `-x`; their original commits and
worker checkouts are preserved.

| Slice | Original commit | Integration commit |
| --- | --- | --- |
| Training | `84a124449b4470194e4080a508ed190dcaa5a5bf` | `6656dbdd7529d6d6f5e1e14cbbfecd559878dcf8` |
| Tunnel | `76e397ab74bf8112acc3657f2317c96a1751c7df` | `f22b5325c44cd8622c1e6b3dc298e17788dcda17` |
| Presentation owner, speech and cadence | `333a7287b62c1d458902daccdac65d85f2821f3c` | `be274c2344ee46a51e0a78ed938a4e2a2a6e86fa` |
| Nullable dialogue text and deadline order | `e1267ec0bb5bc91c2e3aa64d8f7c083be8158297` | `0f4d5bc1db30d20441d99118f8c4692c3149d950` |
| Presentation corroboration note | `4c0d6ad9675b9cf16140b6c5469a680add75f86b` | `25abafcbdc1bf879edf5e8842a61638a9c8e2d70` |
| Courtyard source checkpoint pin | `8d858a5406315935d3f4ca038e1e8bd73b781011` | `f037a9dbfb2d56b24c2e8bac859621f517ae83e3` |
| Death/fall with Proc12 reconciliation | `d87fd6487cd0739e51a1718ba3cc812f8d771a81` | `0d550a1c07e44c2501c2deb82fcc8cd88c1996c1` |
| Final death source checkpoint | `1fd6b1ecc37db4f33d88ac73f10bd6f1d613ee70` | `34fdc545479f541217876ac9cad46706924a54c8` |
| Final courtyard query evidence | `458871fcb4ef2f0f9d38093f5efc2c4a27abbe00` | `20e894c2309a18958cf8a0cef975837b6bc413b5` |
| Object selection and native gate lift | `d9890dc12d0b2c3c7547b178826ed96c8923b2e1` | `135653e262b53342a03a20f6c604572894c726ad` |
| Key producer, custom bits, attachment and bounded camera | `d77f70b8ec072468029f086e121f126fe9226c1a` | `652048f353c4bbfe99db3d03475beda2fb168d9a` |
| Settled training and actor lifecycle | `60a2aec4263ae41129fa706e97254be216047260` | `ef08193142e21a804a1b4bca4c45841625dbc96b` |
| Primary source-camera viewport owner | `ee9c6f0872b6987b06b8e120a601126d2b2c5732` | `9981f113beb0c7493771023ef2bbe98fcce1da10` |
| Proc166 binding and operand order | `579e68e05785b9c0ed8cd90d7e3ec2ccaf385085` | `ff228c4921674be3a385b6271552eb2d8b18539d` |

Training comes from `/tmp/astrolabe-manor-training-static-oct08`; tunnel from
`/tmp/astrolabe-manor-tunnel-static-20261008`; presentation from
`output/worktrees/manor-camera-dialogue-static-20261008`; courtyard notes from
`/tmp/astrolabe-courtyard-static-20261008`. Their source files are not edited.
Death comes from `output/worktrees/death-fall-static-sol-20261008`.
Key/gate comes from `/tmp/astrolabe-gogoud-key-gate-static-20261008`.
No new worker is launched. The lead approved the three follow-ons above;
later presentation/GameOver/recovery commits remain outside this exact set.

## Conflict reconciliation and contracts

The tunnel cherry-pick conflicts in four files:
`src/model/ai.rs`, `src/openspace/ai.rs`, `src/godot/ai.rs`, and the support
README. The competing hunks insert neighboring declarations. Both training's
Pursuit/ActivateActorAtPosition and tunnel's CameraGoToWaypoint remain in the
typed model, validator, exhaustive walker, source parser/writer and compiled
native paths. The Reflex-capable ChangeActorBehavior validator retains the
training `actor, ..` pattern rather than restoring the old Normal-only rule.
Both coroutine capture paths and both completion paths remain distinct; the
camera request captures camera then waypoint, while pursuit captures actor
then radius and uses existing native point-request cancellation.

Git maps tunnel's `demo.gd` edits onto `godot/game/game_session.gd`. The old
production filenames are not restored. Starter inventories, script UIDs,
inherited New Game references and existing `DemoCamera`/`demo_camera` APIs
retain the accepted filename cleanup. The native initial camera framing
Resource and UID are registered once in maintained support. Reset reads
parameter set0 while set1 remains the active framing block; it preserves the
selected target and the session-owned controller Resource.

Before the key follow-on, the unpublished union uses **Rete59/support78**,
incrementing the accepted baseline58/77 once. `src/rete/mod.rs`, `src/godot/support.rs`, support
`version.json` and existing affected assertion literals agree. Nullable
`DialogueSpan.text` now preserves absent versus allocated-empty meaning; the
reader keeps the optional value, validation checks only present text, and native
span Resources expose semantic `text_present`. No original pointer is exported.
The sibling stamps are provisional source states, not interchangeable persisted
packages; a future authorized conversion must import source using this union.
The published root remains on its accepted earlier interface and content.

Proc13 is reviewed through the reader, writer and compiler: first=map,
second=Hype entry, third=camera entry. No operand swap is introduced. The
transition service removes one trailing `~` independently, preserves ordered
native calls and a case-insensitive same-map continuation, and binds semantic
named markers before destination actor startup. Both named poses and marker
sector links remain native NodePath/transform relationships. Destination
inventory/life/armor preflight uses the same component snapshot APIs as restore;
inventory and armor explicitly clone their owned definitions, and the armor
facade remains bound to its catalogue. The main and Fix Brain transfer excludes
live actor/waypoint references and requires matching model identity. This is
in-session transfer, not an extension of the schema4 checkpoint envelope.

Presentation keeps one shared dispatch owner, validates before replacement,
binds speech/completion to the calling Brain's actor, and routes speech through
the Voice bus independently of object Effects gain. The captured generation
guards stale player-finished clearing. Cancellation and exit clear the owned
speech record and presentation, and input/completion remain owned through close.
The new source draw subset keeps first-span text presence, line-order deadline
replacement, strict playback comparison and per-draw float32 alpha steps.
Inner rendering/reveal, driver clock and x87 equivalence remain the documented
external limits; code review does not establish timing or full dialogue parity.

The accepted mip-input/import/publication fixes and all20 sector-neighbor
`.assign(...)` emissions remain in the branch. Nothing invokes them here.
No source scheduler, pool, AST/opcode interpreter, address lookup or relocation
reconstruction is added to the native target.

## Initial static lifecycle findings for the follow-on owners

These findings describe the first integrated slices. The approved settled
training follow-on below replaces both removal paths and their stopped-Brain
behavior; presentation completion remains independently owned.

The source KillPerso and current KillSelf paths are not equivalent. Proc226's
Me keyword still parses to `Statement::KillSelf`, compiling to
`character().source_kill()`. That method calls `source_kill_perso()` and then
unconditionally queues its node for deletion. Named/MainActor KillPerso instead
calls `source_kill_perso()`, which stops Brain and sound and disables the retained
native actor. The tunnel source evidence distinguishes retained placed non-pool
Persos from released generated actors. Current self-kill does not make that
ownership distinction. Generated actors already carry
`astrolabe_generated_template` metadata; source pool machinery is unnecessary
to distinguish those native owners.

Gogoud model991's Global219 startup KillSelf can consequently remove its native
identity/Brain before later persistence capture. Its stage0 has no proved
original-save flag; a native mid-training checkpoint must retain transient Brain
state independently of each mannequin's three saved completion fields. The
training/Gogoud persistence owner must resolve this lifecycle contract in its
follow-on. This integration does not change self-kill or replay old save fields.

Two adjacent retained-node consequences remain for those owners to assess:
`source_set_object_active_state(true)` reenables processing but does not restart
a Brain previously stopped by kill, and emitter `kill_all()` stops players
without clearing `source_dialogue_record` through the dialogue-specific stop
API. These are direct native-code observations; required source reactivation
and speech cleanup semantics need a deliberate lifecycle decision. A forced
stop does not use the natural-finish closure. No runtime symptom is claimed.

The requested direct relay to training task
`01a11d84-b0c3-733e-ba9e-fbfd898fb0cd` was attempted once and rejected by
automatic approval review. It classified delegated authorization as insufficient
for cross-thread disclosure and requires direct human approval. The message
was not sent and is not represented as completed. No workaround/retry was used.
Kip subsequently reported relaying these findings directly to the training and
presentation owners, and instructed this executor not to retry the rejected
transmission. The findings are part of this review deliverable; future
recovery/GameOver session behavior stays with its assigned workers.

## Death/fall union and connected limits

The death cherry-pick conflicts in `notes/README.md`, `src/openspace/ai.rs`
and `src/godot/ai.rs`. Both source evaluations (training Func13 and the new
Proc0), both guarded expressions (distance-to-position and life subtraction),
and both coroutine polling paths (Meta49 and Meta61) survive. Proc136 remains
typed. Git applies the older `demo.gd` session hunks to `game_session.gd`;
the accepted scene/script names and UIDs remain unchanged.

There is one Proc12 owner: training's `ActivateActorAtPosition` model,
validator/walker, reader/writer, compiler capture and
`source_activate_at_position` native method. The death sibling's duplicate
`ActivateActorOnPosition` path is removed throughout. Both source operands still
evaluate in caller context, with the actor captured before the vector. The
retained method preserves intermediate float32 raise/lower stores, contact-only
velocity clearing, the missing-dynamics activation branch, visibility/activity
and the explicit Character0/moving-parent support boundary. No weaker contact
implementation is retained as an alias. The death worker's Proc12 naming and
scope in its original report describe its standalone revision; this union is
the current implementation.

The useful AI shape now also includes a recursive expression for Func11's
waypoint, Proc415 economic placement, explicit-receiver sighting, Proc56/Func37
byte life subtraction, Proc0 terminal death and Meta61 fade waiting. The
waypoint expression is visited recursively for validation and reference
collection; reader/writer and guarded compiler retain its original caller and
selected receiver contexts. Func224 preserves a real operand's IEEE float32
payload low byte. Zero life does not initiate death implicitly. The shared
screen owns fade direction and Tween; the existing coroutine token/revision
guards own cancellation. These remain source/code review conclusions only.

At the death integration checkpoint, the native session additionally called
the dialogue stop API after stopping non-music emitters. Final training review
below removes that integration-owned synchronous clear: forced-stop completion
belongs to the presentation owner's later polling frame. The transient terminal
flag and explicit R/Escape routes
do not implement original phase8 or restore a disk checkpoint. The native save
schema4 envelope and admission policy are unchanged.

At the death checkpoint, changes are one unpublished **Rete59/support78** union. No package was
persisted under this evolving source schema. The new recursive waypoint shape
belongs to the same unpublished increment, rather than claiming compatibility
with any worker's standalone59 assets. Accepted main remains Rete58/support77.

Connected gaps at the death integration checkpoint (the final follow-on
section resolves the Proc12 and immediate Proc166 gaps):

- **Cliff camera Proc12:** the imported `level:/persos/world/0/2` StdCamer has
  a motion binding and mechanics mode3. The preserved source audit records the
  binding; accepted native `shared/mechanics/196782.tres` and the StdCamer scene
  identify mode3. A future conversion of this union emits the binding and its
  single Proc12 owner rejects that mechanics mode. Training's source-backed
  Character0 contract does not establish a camera placement/contact contract.
  The cliff watch is consequently not end-to-end connected merely because its
  actor/vector AST now parses. No current published runtime symptom is claimed.
- **Hype24 Proc166:** Raymap names this `Procedure_Camera_ChangeFlag`;
  `notes/research/hype-ai-api.tsv` gives operand signature `PLLL`. The read-only
  decompilation coverage ledger lists the entry as absent from the raw export and
  `untouched_missing_raw`, requiring independent static recovery. The current
  reader still preserves it unsupported. Proc136 reset is already integrated
  and is not an equivalent shortcut. Complete flags/selector semantics remain a
  source/native camera dependency; no engine flag machine is introduced.
- **Game over:** Proc307 presentation and its display-before-menu contract remain
  with the presentation follow-on owner until the lead gives its exact commit.
  The death note's source call/ordering trace does not substitute an implementation.
- **Original death placement/reset:** Proc8/14 and phase8 remain unsupported
  lifecycle variants, separate from native checkpoint semantics. The bounded
  ordinary Manor scan found no Proc8/14 uses; this does not establish other maps.
- **Fade/lifecycle limits:** native immediate zero-duration fade completion,
  fade ownership across level release, contact/sector and original RNG/x87
  equivalence remain unverified. No test, interpreter or source scheduler is
  added to hide those boundaries.

Final evidence pins `1fd6b1e` and `458871f` are included after their respective
implementation notes. Their recovered geometry queries establish neither the
reported courtyard wall defect nor a collision fix. No collision implementation
is changed and no runtime/validation rerun follows those evidence-only updates.

## Key/gate union and source-semantic reconciliation

The lead subsequently approved `d9890dc` then `d77f70b`, both included above.
The first cherry-pick conflicts in the notes index, AI model, walker, reader
and compiler. The second conflicts in the reader/compiler and also auto-merges
a duplicate `ChangeActorSighting` declaration and duplicate writer/visitor arms.
Those auto-merged semantic duplicates are resolved explicitly, rather than
counting absence of Git markers as integration acceptance.

Training's **DistanceToPosition is the single Func13 owner**. The key sibling's
ActorPointDistance path is removed from the model, validation, walker, reader,
writer and compiler. Both expressions represent the same source function, but
the complete training source helper in `training_receivers.c` negates a result
when it is not greater than zero before the float store, including positive
zero. The retained compiled path preserves that branch and source-axis origin
subtraction; the sibling's ordinary positive native distance omits it. Both
caller-context point evaluation and captured Ultra receiver survive. Relative-Z
Func192 and actor-valued Meta23 turn targets are separately retained.

Death's **ChangeActorSighting with an Expression actor is the single Proc445
owner**. The broader expression validator, exhaustive actor/vector traversal,
reader/writer and guarded native receiver capture remain. Key's narrower
ActorTarget declaration, duplicate later Ultra branch and compiler/writer arms
are removed. Key's Func164 ActorSighting and death's dot-field ActorSightingField
remain distinct source encodings, sharing the same guarded native sighting read.
Proc415 economic placement, Proc136 reset, pursuit, dynamic waypoint distance,
byte damage/death, Meta49 and Meta61 are retained. The general actor-valued DSG
Ultra dispatch added by key is preserved alongside the discarded Func13 query.

Object selection is admission-aware and uses the actual inventory Resource in
the authored screen/host. Raw real selector payloads remain IEEE float32 words;
quantity -1 does not itself grant ownership. Training991 Normal19 activates
physical key142, whose model703 later adds its own StockAction Object21.
Gate272 retains that selected key, uses Global149 and its authored two lift
segments; no intro completion grant or runtime item-name join is introduced.
Lift points/factor are captured once, the native request owns cancellation and
completion, and its physics step receives float32 seconds. The source-supported
16..80ms interval quantization is not reconstructed as another scheduler.

Both contributed custom-bit fields are retained: `Perso.custom_bits` in the
useful model/Rete, and exported `source_custom_bits` on the native actor.
The reader currently obtains their source value from StandardSaveWire's legacy
`custom_bits_init` member. Complete Cond107/Proc273 source and the original
getter/setter corroborate direct access to that same word. Snapshot-before-
operand evaluation, wrapped one-based index, low-byte truth and raw real words
remain native operations. The adjacent original setter at StandardGame+0x30
links two actors (its callers pass their actor pointers); it does **not** prove
another initial custom-bit mask. An early review hypothesis to that effect was
retracted, and no guessed offset or duplicate source value was added. If the
lead's request for "both custombitfields" denotes two distinct **source words**
rather than the model/native field pair, the approved commits define only the
one mapped word and that second semantic field still requires an exact source
contract. Existing opaque wire prefix/suffix fields have not been reinterpreted.

Attachment keeps the child-owned real Godot hierarchy and source actor/Brain/
entry indices. The synchronous Brain hierarchy-move guard is retained; reparent
notifications do not unconditionally stop the ongoing key coroutine. World
parent and source contact-sector paths remain named native references. The
future training activation/pause/resume change must preserve this guard as well
as the custom-bit pair and component ownership. Broader detach sector migration
is not established by the bounded Manor channel20 path.

Proc141 primary reselection and Proc158 focal changes retain the worker's
explicit registration/policy bounds and early operand-skipping behavior. The
existing Proc136 framing Resource and reset stay registered once. Initial FOV
is exported as a semantic native value. **Func171 vector angle is implemented;
Proc171 Camera_ResetCameraZMax is not.** The latter has no typed reader/writer/
compiler/native implementation in either approved key commit. The source API
catalogue and read-only coverage ledger also distinguish the camera procedure's
shared missing-raw handler from vector-angle's complete source. The bounded old
AI audit did not produce a key703/training991 Proc171 occurrence with the exact
Fix model paths selected; it is not a new complete raw allocation scan and does
not prove absence. A requested camera171 contract must not be claimed complete
from Func171 or replaced by Proc136. This discrepancy is surfaced to the lead.

At the key checkpoint the union uses **Rete60/support79**. The key follow-on increments its useful
AI/Perso shape and native interfaces; this integration retains the new values
and fixes `godot/addons/astrolabe/version.json`, which the worker left at78.
All model, support constants, manifest and existing version assertion literals
now agree by source inspection. No schema60 package has been written and no
support update performed on main. Standalone worker packages remain provisional;
future conversion must import using the full accepted union.

At that checkpoint the actual key remains a linked slice: training's
Proc12 null-card correction/reactivation/Gogoud checkpoint follow-on and exact
box5 presentation are explicitly incoming. Key's stage4 dynamics binding has
no mechanics identity card, which the current single training Proc12 owner
rejects. The exact box5 completion into key Normal2 is needed to detach/release
controls; generic text or admission alone cannot establish that handoff. Prior
death Proc166 and cliff camera mode3 gaps remain, as do Proc307 presentation
and the original phase8 distinction. No camera reset shortcut, checkpoint
exception or invented key consumption resolves those dependencies.

## Settled training and source-camera follow-ons

The lead approved `60a2aec`, verified at the clean training worker checkout,
then `ee9c6f0` at the clean key worker checkout, and `579e68e` at the clean
training checkout. Their exact integration commits are in the table above.
Only the isolated integration worktree is edited.

Training replaces the lossy contact Boolean with optional `ActorActivation`:
active state, return suppression and reset-on-kill policy. The useful
`Perso.custom_bits` and native `source_custom_bits` fields remain, including
the reader mapping, exporter emission and every affected Perso constructor.
Initial native contact eligibility is derived from active/suppressed state.
Ordinary deactivation pauses the existing Brain rather than stopping its
selection or suspended native awaits; inactive startup defers first entry.
The key's synchronous `source_begin_hierarchy_move`/`source_end_hierarchy_move`
guard remains in `_exit_tree`, independently of the activity guard in `advance`.

Self and named removal share a Boolean actor-owned contract. Placed actors
retain their identity and suppression; generated owners release their nodes.
Unknown and reset-on-kill placed policies fail before mutation. Both compiled
KillSelf and tunnel's named KillPerso consume the result and fail visibly for
unsupported removal. Ordinary activation never clears the suppression latch;
native checkpoint admission is a separate preflight contract. Generation
clears suppression before registration and held objects activate on release.

Native schema4 has an optional independently versioned training section for
settled stages1..5, retaining Gogoud's stage/lever latch/pose, each mannequin's
three fields, Hype's three permissions and Global152/219. Restore preflights
semantic actor/model/field/action references and a private inventory snapshot
before live mutation, and selects native idle actions/behaviors before automatic
Brain startup. Stage5 requires admitted Object21 and a retained inactive,
suppressed key owner, so pending box5 presentation and a loose ground key are
not bypassed. Source schema1 overlays cannot contain native training state.
Neither a source execution cursor nor a suspended coroutine is serialized.
Barrel capture accepts the retained suppression tombstone as well as a freed
owner. No checkpoint exception permits ordinary NeverBack reactivation.

The single training Proc12 owner now supports a converted absent identity
card and direct contact placement of the bound mode3 SourceCamera. It retains
the intermediate float32 stores, contact-only velocity clearing, placement
before activation and the suppressed-actor placement branch. Broader moving
parents and mechanics remain explicit unsupported contracts. The earlier
null-card key and cliff-camera source connection gaps are resolved in code;
their actual contact behavior remains unexecuted.

Proc166 is typed through model validation/traversal, reader/writer and native
compilation for immediate groups0/1. `579e68e` captures the early camera,
evaluates group/operation/selector in caller order, and reloads the immediate
component only afterward. Invalid selectors retain mask0, including the
operation-1 read of initial flags. Deferred groups remain unsupported.
Hype24's group1 clear and Proc136 reset remain distinct operations.

Proc141 uses a weak semantic primary-camera owner on the native viewport,
independent of an authored camera temporarily rendering there. Reselection
restores the previous target's opacity, makes the source Camera3D current and
returns before reset/placement. Switching away clears context while retaining
registration. Export carries registration and viewport context separately,
without a Manor-specific branch or original address. Proc158's unregistered
operand-skipping branch and initial FOV remain. Broader reset/unregistered
camera selection contracts continue to fail visibly.

Generic actor deactivation calls `kill_all` only. This review also removes the
integration-added terminal death `stop_dialogue_speech` call, preserving the
same no-synchronous-clear rule. Presentation's next-process-frame forced-stop
completion remains an incoming implementation; this review does not claim it
already exists. Exact box5 completion and Proc307/GameOver presentation remain
with that owner. Requested camera Proc171 remains absent from these approved
patches, distinct from implemented vector Func171. Original death phase8 and
Proc8/14 remain separate from native checkpoints.

The final source stamps are **Rete60/support80**. The viewport worker changes
the support constant and existing assertion to80; this review also updates the
maintained `version.json` and support README to80. No evolving schema60 package
has been written. No support or generated-content update occurs on main.

## Acceptance boundary

There is no unresolved Git conflict in this slice. Compilation, GDScript
parsing, reconstructed bytes, fresh Rete/native resources and runtime behavior
are unverified. No assets are published or main merged. Later lead-directed
work must compose the exact remaining presentation/GameOver/recovery follow-ons, preserve
the model/component boundaries, resolve lifecycle findings, then separately
authorize any executable validation or content preparation. The user may open
the accepted main Godot project throughout this source-only integration.
