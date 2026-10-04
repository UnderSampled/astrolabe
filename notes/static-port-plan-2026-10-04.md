# Static port implementation plan — 2026-10-04

This is the working implementation ledger for the requested source-first port.
The initial pass deferred tests, builds, conversion runs and gameplay checks.
After reporting preparation failures, the user authorized rebuilding and retrying
`prepare-game` until it completes. Regression and gameplay tests remain deferred.
Static implementation/review and successful preparation are not gameplay
acceptance. Existing tests stay in place for that later pass.

## Survey and boundaries

The current Rust code has one canonical content model, OpenSpace and Rete
read/write adapters, and native Godot export. The authored Godot project is the
game. Keep those boundaries and compile source AI to ordinary GDScript; do not
introduce an engine interpreter, asset loader, no-op API catalog or guessed
gameplay semantics to reduce the unsupported count.

The following survey combines source inspection with the **previous** local
manor import report (`TestResults/review-import.json`), without executing a new
conversion. That report contained 4,413 assets, 2,075 unsupported-script
diagnostics across 54 AI models, and nine imported dialogues out of 67 entries.
Those are one level's dependency closure and diagnostic occurrences, not a
whole-game completion percentage or a count of distinct required APIs.

The change record below updates this initial survey as each dependency lands.

| Domain | Present at the initial survey | Gaps identified at the initial survey |
| --- | --- | --- |
| Startup/project ownership | Scoped conversion, staged native resource build, ownership receipts, authored settings preserved | One complete preparation command; clear missing-content entry; concise setup documentation |
| Media | CNT/GF, APM/BNM decoding, structured texture/audio assets, native PNG/WAV output | Restricted BNM/CSB writers, original encoding fidelity, video conversion/playback |
| Scenes/rendering | Root hierarchy, shared geometry/instances, collision, sectors, lights/fog, source materials | Additional LOD/geometry domains, source visibility/configuration changes, dynamic lighting and per-actor sector contact |
| Characters/actions | Native sampled animations, initial/reset configuration, action transitions, root motion, movement/jump/contact | Link-table object swaps, broader action/object lifecycle, remaining mechanics types |
| AI infrastructure | Typed trees, partial preservation, direct GDScript, explicit reached-unsupported failure | Exhaustive dependency traversal; generic actor/value receivers; remaining structured operators and calls |
| Player/combat | Movement, jump, sword/Fire/crossbow slices, inventory/magick Resources, life reads | Armor definitions/state, damage and target reactions, death, additional weapons/spells and interactions |
| Opening/camera | Routes, camera framing/aiming subset, normal New Game, first-dialogue checkpoint | Actor-directed initialization, armor selection, complete source camera commands and intro handoff |
| Dialogue/UI | Native main/pause menu, inventory presentation, HUD subset, localized text/voice/callback | Structure-based dialogue import, branching/channel/icon effects, remaining HUD/options/menu routes |
| Persistence/session | One session lifecycle; measured English start; native mana/inventory/one-world-flag save | General actor/world/progression persistence, source save interoperability, level transitions |
| Rete/reconstruction | Whole-asset MessagePack, identity validation, source-free reopening, supported fragments | New domains alongside their consumers; whole-level assembly and source-file/package fidelity |
| Comparison infrastructure | Orrery, separate Godot harness, external behavior suite, recording contracts | Later execution of updated regression/gameplay matrix; timing/render association and full routes |

The largest repeated unsupported operations in the retained manor report are
diffuse coefficient (330), link-table swap (255), actor behavior change (139),
WaitBoolean (61), inventory add (49), and armor-aware damage (48). Generic
actor/value support also blocks position, validity and distance calls across
many models. Count is not sufficient priority: dependencies and the real
New Game/player paths come first.

## Dependency-ordered work

Checkboxes mean statically implemented and reviewed only. Runtime acceptance is
deferred for every changed item. Independent tasks may proceed together with
exclusive file ownership; the parent integrates shared registrations/versions.

- [x] S0: Survey current adapters, gameplay, UI, references and previous coverage.
- [x] S1: Complete preparation command and three-step fresh-start guide.
  Owner: workflow Sol agent; CLI and bootstrap/docs only. Preserve independent
  scene/menu/inventory/pause ownership and disclose partial preparation failures.
- [x] S2: Canonical exhaustive AI traversal and dependency collection.
  Owner: AI Sol agent. Fix missing recursion through integer casts and sniper
  arguments, then keep new expressions/statements in the same traversal.
- [x] S3: General actor/value lowering and actor-directed presentation.
  Owner: AI Sol agent, with native character interfaces from gameplay agent.
  Preserve keyword binding and explicit missing/invalid receiver failures.
- [x] S4: Native character presentation/movement cleanup and documented wind.
  Owner: gameplay Sol agent. Separate focused native components when useful;
  keep action/gameplay decisions in compiled source AI.
- [x] S5: Recover armor source definitions and implement armor selection/state.
  Depends on verified definition layout and state ownership, then S3. Inspect
  original code statically; a catalog name or guessed resistance is insufficient.
- [x] S6: Replace named-dialogue allowlisting with structural eligibility.
  Owner: parent. Reuse existing semantic model/native renderer, retain bounded
  parsing, and diagnose unsupported branching/effects by item.
- [ ] S7: Complete configuration swaps, actor lifecycle and behavior changes.
  Depends on S2/S3 and verified action/configuration semantics.
  Shared slot swaps, literal/null configuration selection and external behavior
  changes are implemented; dynamic configuration expressions and broader actor
  lifecycle remain open.
- [ ] S8: Complete armor-aware health, target response and death.
  Depends on S5/S7, actual source fight handlers and contact identity.
- [ ] S9: Complete intro camera/presentation and transition to player control.
  Depends on S3/S5/S6/S7; retain full source AI, no authored shortcut triggers.
- [ ] S10: Generalize session/persistence/progression and level transitions.
  Depends on owned state from S5/S7/S8; preserve atomic save replacement and
  validate the whole payload before applying it.
- [ ] S11: Remaining UI/options, inventory effects, spells and actor interactions.
  Follow the shared services established above, with source evidence per API.
- [ ] S12: Broaden levels/locales and complete content/reconstruction domains.
  Preserve explicit unsupported states; optional Godot reverse import and exact
  historical compression remain separate from the playable port.
- [ ] S13: Run regression, conversion, editor, gameplay and comparison checks.
  Deferred by the user. Do not count static review as any of these passes.

## Source availability and unresolved evidence

The reference submodules began uninitialized. The exact pinned Raymap commit
`2df138379acaa5c9d74ae275c6690ba8e2e3e52e` was restored for read-only source use;
its gitlink is unchanged. The private reference worktrees remain uninitialized;
the pinned C# snapshot is readable from the local Git object database. The
former workstation's ignored decompilation/proof reports are absent. The documented Zeus
patch was recovered via its HTTP download after the HTTPS routes failed. Only
`MaiDFXvr_bleu.exe` was extracted under ignored `reference/hype_patch/`; its
SHA-256 is `ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`,
matching the tracked API catalog. Static disassembly now supplies fresh source
evidence; the executable has not been run. Retained documents and earlier
proofs remain historical evidence, with their stated bounds.

In particular, the previously recorded next opening failure was procedure 429
`Fight_ChangeArmor` at node987. The verified executable table disagrees with the
older Raymap opcode labels at that position. Armor definitions, selection and
player damage now have a source-supported implementation against that matching
executable; the opening has not been rerun. The visible
Hype/statue overlap is an unresolved result, not permission to hide an actor or
change its imported transform by hand.

## Change and review record

Implementation batches and exact remaining gaps are appended here as they are
integrated. Existing dated runtime reports remain historical evidence and are
not relabeled after this static pass.

- S1 now composes the four existing conversion scopes through `prepare-game`.
  The starter displays a setup page when required assets are absent, and the
  README has a three-command entry path using the actual local `disk/` layout.
- S2/S3 use one explicit-stack traversal for validation and dependency
  collection, and add source-proven actor values, full distance, validity,
  square/absolute value, directed presentation, Boolean waits and external
  behavior transitions. The [AI evidence](./ai-static-port-2026-10-04.md)
  identifies partial-axis distances and shared material lighting dependencies.
  The [material coefficient survey](./material-coefficient-source-2026-10-04.md)
  identifies visual-asset sharing and the source fourth lighting component as
  prerequisites for diffuse mutation; an RGB/albedo substitute is not admitted.
- S4 extracts actor rendering and session loading/crossbow input components,
  retaining native extension points. Source wind now replaces a persistent
  world-space drift term; it is composed with motion rather than accumulated.
- S5 adds canonical shared armor definitions/state, native Resources and
  selection/resistance/repair calls. S8's first dependency now includes
  source-defined player damage and remaining-life return values; contact,
  target-life presentation, reactions and death remain open. See the
  [armor evidence](./armor-source-2026-10-04.md).
- S6 checks each catalogue item's supported structure and emits per-item
  omission reasons. Unknown nonzero effect fields remain unsupported. No new
  import ran, so no increased dialogue count or visual coverage is claimed.
- S7 implements shared configuration slot permutations using saved native
  payload nodes and stable animation anchors. Reset restores the saved payload
  bank; swapping does not synthesize geometry, seek animation or replay cues.
  Growth is bounded at export. See the
  [configuration evidence](./configured-part-swap-source-2026-10-04.md).
  Resolved configuration selection now imports references outside family pools,
  precreates only reachable payload banks and supports a null current selection.
  Engine evidence corrected the prior reversed initial/current field labels;
  selecting a current table does not change the initial table or reset animation.
- S11 now has a separate canonical inventory gameplay catalogue and typed
  actor stock values, with native shared state for source-supported ordinary
  admission, capacity, gold, Weapon/Object selection and quantity removal.
  Arithmetic and return values follow the source's signed narrowing. The
  preparation follow-up below adds ordered replacement rules and per-kind
  Magic selection effects, with distinct catalogue members even when IDs repeat. See the
  [inventory contract](../docs/openspace/inventory-gameplay.md).
- S10 has a bounded native idle checkpoint preserving placement and life,
  the currently represented armor/inventory state, mana and one world flag.
  Native schema 3 binds equipment snapshots to catalogue and member identities
  and validates detached copies before changing live state. Older slots remain
  readable with their omitted fields seeded. AI continuation, general world
  progression and level transitions remain outside this checkpoint contract.
- Rete schema 42 and Godot support interface 54 identify this source batch.
  Existing packages/projects need regeneration or an explicit support update.
  The initial checks were source review, Rust formatting and whitespace
  inspection. The preparation follow-up below records subsequently authorized
  builds and conversion; regression and gameplay tests remain deferred.

## Next source implementation order

1. Preserve independently shared visual-material identity, then complete the
   traced lighting/alpha inputs before admitting diffuse mutation. This is the
   largest repeated unsupported API in the retained report and a prerequisite
   for consistent material variants.
2. Complete source camera modes, actor lifecycle and the remaining original
   opening path. Configuration and armor dependencies above are implemented,
   but no new opening success or resolved Hype/statue overlap is claimed.
3. Complete target damage/reactions/death, remaining inventory/Magic APIs,
   branching dialogue and the remaining UI routes against source evidence.
4. Expand persistent actor/world/progression state, level transitions and other
   level/localization content. Checkpoint schema 3 does not preserve arbitrary
   AI/animation continuation or configuration/world mutations.
5. Finish source-file/package assembly and remaining media domains, keeping
   exact historical compression fidelity separate from native Godot progress.

The final source pass found no missing maintained support registrations, Rust
formatting differences or whitespace errors. Existing tests were adjusted only
where structure, captured operands or checkpoint contracts had changed; none
were executed. Runtime cost, emitted Godot syntax/resources and actual gameplay
of this batch remain unverified until the deferred acceptance pass.

## Follow-up: reported preparation import failure

The user's `prepare-game disk godot` run stopped during scene import at
Fix6:1 offset 469152. Static source-byte inspection and an independent Sol
review traced this to the armor declaration offset: GPT18152 names the seventh
inventory pointer, while the complete reader starts at 18128. Counting the whole
reader from its seventh field placed the armor root 24 bytes late, on a diamond
gauge texture. The corrected formula selects GPT19140 for this corpus, whose
three linked armor records and referenced shield textures have the documented
allocation extents. The importer now checks those extents before traversing
fields and reports armor/inventory import context. See the corrected
[armor evidence](./armor-source-2026-10-04.md).

The same source review found that the separate inventory gold definition has a
null kind-string pointer. Ordinary catalogue items retain nonempty kinds; the
shared model now preserves the special resource's absent kind explicitly.
Neither change relaxes relocation checks or requires language hydration for
nonlocalized inventory kinds. No build, test, converter or runtime was executed
by the agents for this follow-up; complete preparation is still unverified.

## Follow-up: authorized preparation recovery

The user subsequently authorized release builds and normal preparation retries.
The inventory failure at Fix6:1 offset 4454836 exposed a second missing source
structure: the handle's trailing fields are one first/last/count list, rather
than three pointer fields that must be null. The importer now retains and
validates its ordered replacement relations. This corpus contains 21 relations
on 15 items and twelve distinct ID-50 placeholders. Native state and snapshots
retain member identity; source ID queries and UI ownership use separate views.
See the [inventory contract](../docs/openspace/inventory-gameplay.md).

The first release build passed. The first preparation retry imported the scene
and then rejected excessive native configured payload growth. A separate normal
OpenSpace-to-Rete import completed in 6.834 seconds with 4,418 assets. Its
diagnostics retain 1,386 unsupported-script occurrences and seven structurally
supported dialogues out of 67; this is not whole-game or gameplay acceptance.
Logs are under `TestResults/prepare-game-recovery-G7IFeh/`. Further preparation
results follow below.

Preparation subsequently exposed excessive configured-payload growth, two
GDScript/resource-file array syntax mixups, and quadratic scene finalization.
The exporter now keeps payload banks only on object-bearing channels, omits
explicitly absent payload nodes, and retains sampled anchors independently so
swaps into initially empty slots remain possible. Its 65,536-payload limit is
unchanged and counts actual emitted nodes. Editor visibility entries are scanned
and appended once. Native identity normalization uses streaming passes and
simultaneous resource-ID remapping; deterministic collision resolution preserves
otherwise unique editor node IDs. The generated manor contained 167,242 nodes
and seven collisions under the former raw 31-bit hash rule.

Attempt 4 reached the existing 360-second native-builder deadline before the
editor-visibility optimization. Attempt 5 completed native construction but was
stopped during the old quadratic Rust normalization after static inspection
confirmed its eventual node-ID collision rejection. Its staged source is retained
locally, and `prepare-05-stop.json` records the interruption. Neither attempt is
counted as successful preparation.

The final release build (`build-06.log`) passed in 42.28 seconds. The exact
`target/release/astrolabe prepare-game disk godot` command then completed all four
scopes (`prepare-06.log`, exit 0, status `complete`, no remaining scopes):

| Scope | Published files owned by scope | Seconds |
| --- | ---: | ---: |
| Scene | 5,954 | 143.971 |
| Main menu | 37 | 6.100 |
| Inventory | 146 | 6.555 |
| Pause menu | 27 | 5.849 |
| Total preparation | | 162.494 |

The normal command included staged native resource construction, import and
publication, then destination Godot import. Rust formatting and whitespace
checks passed. No regression suite or gameplay test was run. Successful
preparation does not validate the newly translated gameplay, complete the port,
or measure menu-to-play latency. The large native node count remains a loading
efficiency concern despite fixing the preparation blockers.
