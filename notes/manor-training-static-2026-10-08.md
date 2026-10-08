# Manor training static implementation — 2026-10-08

This isolated change starts at `d493ef24859d9b2a0b18ed68f5ebb845e7feeebb`
in `/tmp/astrolabe-manor-training-static-oct08`, branch
`sol-manor-training-static-oct08`. The main checkout, other worktrees, original
data, generated Godot files and reference repositories were not edited.
The user requested static work only. No tests, builds, formatter/compiler/lint
checks, conversion, original-game runs, Godot runs or probes were performed.

## Source findings

The existing imported AI audit under
`output/integration-resume-20261008/source-audit/ai.json` was read as source
evidence. Its unsupported diagnostics predate integrated Boolean waits,
actor-position fields and local-Z distances; they are not a current coverage
measurement. Original Ghidra bodies, static disassembly of the installed PE and
readable C in `/home/stephen/Projects/hype-decomp` were consulted. The installed
PE SHA256 is `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
Callable tables identify the installed build's operation IDs; addresses and
relocations are not part of the added useful AI or native camera resources.

The three training mannequins, actors `level:/persos/world/0/152`, `/153` and
`/154`, share model `level:/ai/models/939`, family `level:/families/39`.
Their armature motion is authored in the native action clips. Normal 0 executes
state 3; Normal 1 executes
`7,11,9,15,17,14,8,6,12,13,10,16`; Normal 2 executes
`23,21,27,29,26,20,18,24,25,22,19,28`. Each then requests idle state 0 and
Normal 3. Hit rules select those behaviors according to facing, hit-zone
selection, the current stage and player attack flags. A separate runtime spin,
module-rotation table or replacement animation would duplicate source content.
Static exporter review confirms that each channel's full transform is saved in
native `Channels/channel_*:transform` tracks. The separate `RootMotion` track
extracts translation only, so channel rotations remain in the visual armature.

Mannequin designer field 0 is the stage selector: 1 sword, 2 magick, 3 crossbow,
4 completed. Fields 1 and 2 record the required two side reactions; both are
required by the successful stage branch. Those three fields have source save
flag 512, independently for each actor. The transient real in field 3 supports
the facing calculation. Success selects Gogoud's Reflex 3 and Normal 4; the
wrong-action branch selects Reflex 3 and Normal 6. The missing external Reflex
request was stopping these existing compiled source paths.

Gogoud actor `level:/persos/world/0/155`, model `level:/ai/models/991`, owns the
training sequence. Its local stage field 0 has source save flag **zero**; it is
not a proven original-save field. Its Normal behaviors choose stage positions,
play dialogues, set/release MainActor control flags and inspect the separate
mannequin completion fields. Normal 4 advances a successful exercise; Normal 11
returns to Normal16's crossbow exercise. The earlier report incorrectly
attributed a Global219 write to Normal11. Normal18 sets Global152 and stage5,
then its final dialogue completes into Normal19, which produces the physical
key. Global219 gates startup `KillSelf`; the audited Manor writer is the World
actor's mode6 setup path, not a proved ordinary training-completion write.
No training stage or completion flag is forced by authored game code.

The barrel helper is different content: Fix model 671, actor
`level:/persos/world/0/76`. Its nearby-barrel branch writes MainActor field 119
to 10 and passes **requested damage 10** to armored damage. Armor can change
the effective HP reduction. The returned life selects Hype Normal 15 (hurt) or
24 (death). Model 939 has no such armored-damage call. Existing barrel support
is reused without modification.

## Connected implementation

| Source call | Useful/native implementation |
| --- | --- |
| Proc_ChangeComportReflex, 208 | External `ChangeActorBehavior` accepts Reflex references, validates the owning model, and calls that actor's native `Brain.change_reflex`. Equal selection remains a no-op. Existing unresolved same-actor procedure control semantics still fail explicitly. |
| Proc_TurnPerso, 217 | Typed immediate actor-facing statement, compiled to the character's native world heading request with f32 output. It is separate from incremental turning. |
| Action_Pursuit, Meta 2 | Captures actor and f32 radius, samples the actor's current world origin each movement update, and reuses native point motion, arrival, failure and cancellation. No source scheduler/event list is recreated. |
| Func_DistanceToPosition, 13 | Pure full origin-to-point distance with real output and the source zero-negation branch. Explicit Ultra receivers survive encoding, including explicit Me. The discarded Ultra distance call in Gogoud's stage transition is an evaluated scalar query, without placement side effects. |
| Proc_ActivateObjectOnPosition, 12 | Typed actor/position operands evaluated in order, then a focused native Character0 placement/contact/activation call. See its bounds below. |
| Camera_Reset, 136 | Typed bound camera reference, native initial framing resource exported from parameter set 0, and restoration of mapped camera parameters/progress. Set 1 is the active parameter block. Selected target and session-owned controller history survive. |

The model validator, exhaustive walker, OpenSpace parser/writer and Godot
compiler contain the new variants. Pure standalone distance statements are
encodable. Camera initial framing is a native Resource with named properties,
saved with the converted scene; it contains no source parameter byte image or
runtime table interpreter. `framing.gd` and its UID are in maintained support.
Rete 59 and support 78 are provisional sibling stamps. Existing schema/version
assertion literals were updated mechanically; no new tests were added or run.

The readable-C interaction worker independently reconstructed the reflex
request, immediate facing, position distance, pursuit continuation and exact
activation helper in `/tmp/hype-sol-key-training-20261008`. Its wider placement
wrappers remain documented dependencies. Presentation source commit `d9b1c9d`
provides the full camera reset body in
`src/camera/camera_initial_reset.c`; the Astrolabe presentation worker explicitly
reserved Proc136 and camera scripts for this training worker.

## Bounds and integration

Activation first rounds source Z plus 0.5, then rounds that value minus 0.5.
The first source placement snapshots the elevated pose; the second performs
contact resolution from that snapshot and clears its velocity packet only on
successful contact, then activation runs. Both placement helpers skip when
dynamics is missing, but activation still runs. The native Character0 path uses
`CharacterBody3D.move_and_collide` for the downward step and clears native
velocity only when it reports contact. It retains the actor's current rotation,
AI behavior and action clips. The follow-up admits the converted null card and
bound camera mode3 through this direct placement path. Other mechanics and actors
under moving character parents fail before placement. Native scene hierarchy remains
stable for supported Manor actors; the original moving-parent detachment is
not claimed. Exact original contact response, contact flags in the same tick,
velocity mapping and floating-point edge behavior require runtime verification.

Camera reset restores the mapped initial offsets, mode, FoV, distance/angle
limits, child selector and parameter flags, adds the first-update snap bit,
clears mapped local progress/displacement, and restores the exact finite bound
values. It does not reset the separate controller Resource's process globals,
replace the target, or force a viewport selection. Unpromoted component fields
and unported camera modes remain outside the native mapped slice.

Kip owns integration and later acceptance. Shared AI model/import/compiler,
character support, Rete/version literals and support inventory may overlap
sibling patches. The presentation sibling also provisionally uses Rete59 and
support78; these need one deliberate union version. This branch does not touch
the session filename rename. The follow-up below defines a bounded native
checkpoint for the independently owned mannequin fields and transient Gogoud
state. It does not serialize an original execution cursor or claim that
Gogoud's stage has an original DSG save flag.

Static review covered operand/evaluation order, tree reconstruction, traversal,
native API names, resource ownership, support staging and version literals.
It is not type checking, GDScript parsing or runtime acceptance. Content must
be regenerated after integration. Later verification must cover both spin
directions/armature channels, wrong-action dialogue, all three stage transitions,
Normal18/dialogue/Normal19 completion, moving pursuit targets, activation contact, camera
target preservation, and save/resume during training.

## Follow-up: training checkpoints, activation and death camera

Checkpoint `84a124449b4470194e4080a508ed190dcaa5a5bf` is preserved. The
follow-up remains in this isolated branch; main `78a1c86` and the integration
worktree are read-only. The integration review at
`output/worktrees/integration-manor-static-20261008/notes/manor-static-integration-2026-10-08.md`
identified the old unconditional self-deletion and stopped retained Brain.
No tests, builds, compiler/formatter/lint/syntax checks, conversion, probes,
game/Godot runs or replay harnesses were run for the follow-up.

The final dialogue `gogood_master_finimannequin` explicitly selects model991
Normal19. That behavior approaches MainActor, turns, sets its lever latch and
activates actor142 at MainActor position plus sighting multiplied by0.1. It
eventually restores camera mode/target/framing, releases Hype56 and enters
Normal23. `whatif05` instead completes into Normal17. The key is StockAction
item21, Object category, quantity-1: inventory admission proves ownership;
a nonnegative quantity test would reject this legitimate key. All seven Gogoud
variables have source save0/init0. Source `005162c0` selects initial behaviors
and resets the designer buffer; the World actor's stage assignments are guarded
setup paths, not evidence that a normal save load reconstructs completed stages.

The optional native schema4 `training` section has its own version1. It admits
only settled stages1..5, Normal14/15/16/22/23 with Reflex0 and action23, live
mannequins at Normal3/Reflex0/action0, and no claimed/suspended trainer rule.
It retains the trainer's stage, lever-event latch and world pose; each
mannequin's byte selector and two Boolean side flags; and Hype's three transient
attack permissions. Global152/219 are independently saved Boolean fields.
Stages2/3 also require default ammunition latches, avoiding a saved refill timer.
Stage0 uses source startup only while the three mannequin fields match the
converted seed (actors152/153/154 selectors2/1/3, side flagsfalse). Older slots
without the new section retain their existing bounded restore behavior.

Capture rejects stage5 until item21 is admitted and the key's placed owner is
inactive/suppressed. This is after model703 Normal2's presentation and KillSelf;
it does not bypass the source box5 dialogue. Its presentation follow-up remains
a separately owned integration dependency. Restore preflights every
semantic actor/model/field/action binding, activation policy, Global flags and
the private inventory snapshot before modifying any live component. It restores
the selected settled behaviors/actions before automatic Brain startup and
retains the key's owned removal. No coroutine, clock or engine scheduler is
persisted. Original-save overlays cannot contain this native section, and a
progressed trainer combined with Global219 is rejected rather than guessed.

Original `00437940` releases generated/pool actors and sends placed actors
through `00437860`. Its non-reset branch disables activity and sets the
NeverBack suppression latch. Return policy1 instead dispatches component reset
event5; that wider reset remains explicitly unsupported by native kill.
`004377e0` and the active setter guard the suppression latch; ordinary
`00437800` deactivation leaves it alone. None of ordinary activation,
deactivation or the non-reset placed kill resets Brain selections. These facts
now survive as `ActorActivation`, replacing the lossy `contact_eligible` field.
Native placed KillSelf/KillPerso retains identity and pauses the Brain;
generated instances release their own node. Initial inactive actors defer first
behavior entry, and ordinary reactivation resumes existing native awaits.
Generated component initialization clears suppression, and the authored release
cue enables its pending activity. Barrel checkpoint capture recognizes a
retained suppression tombstone as well as an already released owner.

The key worker independently read the source StandardGame records for actors142
and155: both have active1, suppression0 and return policy4, so these specific
owners use the non-reset removal contract. These initial values do not prove
activity after the World's startup events. Native inactive processing retains
awaits, but equivalence of elapsed-time waits across a deactivated interval is
still unverified; the existing actor-local clock does not claim a source global
clock implementation.

Actor deactivation calls emitter `kill_all` without synchronously clearing the
dialogue holder. The presentation owner independently proved that original
`004ecc00` does not call `0041e910`; later speech status polling clears it. That
worker's next-process-frame completion adaptation is a required sibling change,
separate from explicit immediate dialogue stop.

Proc12's common placement chain `0052ee40/0052edc0` guards the dynamics workspace,
not the selected identity card. `0057162f/00571df3` snapshot/contact helpers
likewise run their common contact path before the retained pose-mode handling.
The selected camera's type3 movement callback is not required to place it.
Native Proc12 therefore permits the bound mode3 SourceCamera in the actual
cliff path as well as Character0 and converted null-card key. It temporarily
permits the synchronous native contact query for an inactive body, restores its
process mode, then applies activation; suppressed actors still receive placement
but remain suppressed. This does not broaden generic type3 movement. Original
moving-parent reparenting, contact response/flags and exact pose-history/reset
equivalence remain unverified.

The installed PE callable catalogue maps Procedure166 `Camera_ChangeFlag`
(`PLLL`) to `00548690..005488f1`, with tables `005488f4/00548918`. It evaluates
camera, group, operation, selector in that order. Immediate group0 modifies
parameter flags; group1 modifies controller/action flags. Operation0 clears,
1 sets and-1 copies the selected bit from initial parameters. Selectors map
through the proved source bit lists; out-of-range masks arezero and other
immediate operations are ignored. Hype Normal24's exact call(camera,1,0,3)
clears controller bit4 before Proc136 resets framing. Typed model, validator,
walker, source writer and compiled native component call retain all operands.
Deferred groups2/3 require their own real consumers and fail explicitly.
Original-C presentation worker owns full handler reconstruction, including those
deferred groups. The native target has no source flag scheduler or opcode table.

The follow-up stamps are provisional Rete60/support79. Kip owns one aggregate
version and fresh content generation; no persisted package under these stamps
was created here. Adjacent key-worker custom-bit/model/export hunks need merging,
not duplication. Tunnel's named KillPerso compiler must consume the new native
Boolean removal result and fail visibly for reset-on-kill/unknown policy, while
using this canonical helper. Keep the accepted `game_session`/`session_camera`
filenames; this follow-up edits neither older session filename.

Key-worker source tracing fixes model703 Normal2's viewport restoration to
Proc141(1,1,0, camera actor2). Its bounded native implementation requires that
SourceCamera already owns the primary viewport; an authored free camera cannot
silently satisfy that contract. Kip must resolve this binding alongside the
style5 dialogue completion before claiming key presentation or control-release
acceptance. Preserve the key worker's Brain exit-tree guard for synchronous
child-channel reparenting when merging this branch's activity guard.

Remaining acceptance includes both mannequin clip directions, source transitions
and callback chains, save/resume at every admitted stage, key presentation,
placed/generated lifecycle, emission completion timing, activation from inactive
state, cliff contact/camera pose and death's flags/reset order. Static review
establishes the connected code and evidence, not runtime acceptance.
