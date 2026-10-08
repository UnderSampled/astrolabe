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

Training comes from `/tmp/astrolabe-manor-training-static-oct08`; tunnel from
`/tmp/astrolabe-manor-tunnel-static-20261008`; presentation from
`output/worktrees/manor-camera-dialogue-static-20261008`; courtyard notes from
`/tmp/astrolabe-courtyard-static-20261008`. Their source files are not edited.
Death comes from `output/worktrees/death-fall-static-sol-20261008`.
No new worker is launched. Key commit `d9890dc` and later training/key/
GameOver/recovery follow-ons are excluded pending the lead's exact final set.

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

One unpublished union uses **Rete59/support78**, incrementing the accepted
baseline58/77 once. `src/rete/mod.rs`, `src/godot/support.rs`, support
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

## Static lifecycle findings for the follow-on owners

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

The native death session stops non-music emitters, then explicitly calls their
dialogue stop API to clear speech records after forced stops, before disabling
level Brains/body processing and gameplay inputs. This closes terminal death's
speech-holder leak without editing the presentation worker's emitter. The
separate general KillPerso/speech and retained-Brain findings above remain with
their follow-on owners. The transient terminal flag and explicit R/Escape routes
do not implement original phase8 or restore a disk checkpoint. The native save
schema4 envelope and admission policy are unchanged.

All changes are still one unpublished **Rete59/support78** union. No package was
persisted under this evolving source schema. The new recursive waypoint shape
belongs to the same unpublished increment, rather than claiming compatibility
with any worker's standalone59 assets. Accepted main remains Rete58/support77.

Connected gaps for the integration lead:

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

## Acceptance boundary

There is no unresolved Git conflict in this slice. Compilation, GDScript
parsing, reconstructed bytes, fresh Rete/native resources and runtime behavior
are unverified. No assets are published or main merged. Later lead-directed
work must compose the exact training/key/GameOver/recovery follow-ons, preserve
the model/component boundaries, resolve lifecycle findings, then separately
authorize any executable validation or content preparation. The user may open
the accepted main Godot project throughout this source-only integration.
