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
sets Global designer field 219 and finishes the final sequence. That completion
flag also gates the model's startup `KillSelf` path. No training stage or
completion flag is forced by authored game code in this change.

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
AI behavior and action clips. Other mechanics, missing cards and actors under
moving character parents fail before placement. Native scene hierarchy remains
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
the session filename rename. Actor snapshot integration must retain each
mannequin's three saved fields and its current behavior/action. Any authored
mid-training checkpoint must also retain Gogoud's transient Brain state; that
requirement must not be described as an original DSG save flag.

Static review covered operand/evaluation order, tree reconstruction, traversal,
native API names, resource ownership, support staging and version literals.
It is not type checking, GDScript parsing or runtime acceptance. Content must
be regenerated after integration. Later verification must cover both spin
directions/armature channels, wrong-action dialogue, all three stage transitions,
final Global219 completion, moving pursuit targets, activation contact, camera
target preservation, and save/resume during training.
