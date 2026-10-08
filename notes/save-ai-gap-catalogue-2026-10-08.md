# Save-game and non-camera AI gap catalogue

## Integration reconciliation — 2026-10-08

This restores the authored research ledger from `01db3f8d416233957ae962687191525d0652bc14`
(27 preserved research commits). The source traces and negative results below
remain evidence. Their worker statuses, priorities and snapshot counts describe
that earlier branch, not the current main integration queue. Main was clean at
`edb7704b2597c26c78086acad2f58eaba1db7a78` when audited. The table below overrides
historical “merge-ready”, “pending” and “unmerged” labels. Integration is distinct
from complete source-game/native-game behavior acceptance.

| Rows / evidence | Current integration disposition | Remaining dependency or limit |
| --- | --- | --- |
| SG-BASE-001/002 | Already integrated semantic save baseline and saved sideways gate. | Native save is schema 1; preserve the documented fixture/revision limits. |
| SG-INV-001 | Semantic item-zero composition integrated as `00935a3` / `0b23982`; earlier `cb42a61` hardcoded join superseded. | Broader saved inventory ownership/member state needs its own save integration. |
| SG-INV-002/003, SG-MAGIC-001 | Source registration meanings retained; not blanket save implementation. | Remaining quantities/selected flags/default magic are not all composed by the active native save. |
| SG-PLAYER-HP-001, SG-PLAYER-WEAPON-001 | Negative evidence remains unresolved. | Do not equate a capacity/default field or arbitrary selected Boolean with saved current health/weapon identity. |
| SG-ACTOR-001/002 | Reader-side typed Boolean owner projection integrated as `2337318`, `8a344d5`, `1c5e5f5`. | General actor Brain helper `f21a7fc` lacked a production consumer/registration; a separate worker owns coherent integration. Projection alone is not restored gameplay. |
| SG-BRAIN-001 | Behavior/state continuation remains unresolved. | No arbitrary Brain/scheduler reconstruction is authorized. |
| SG-GLOBAL-001 | Optional canonical Global DSG[147] captured/restored as schema-1 Boolean by `be0b88e`. | Other Global fields remain separate gaps; DSG[147] does not prove key/story progression. |
| SG-DIALOGUE-001, SG-MAPSTAR-001 | Original bounded traces and unknowns retained. | No general dialogue/progression persistence claim. |
| AI-CTRL-001 | Existing finite Modulus support/guard already on main. | Exceptional arithmetic equivalence remains unproved. |
| AI-CTRL-002/003 | Distance/raw8 `d4e1a2f`; explicit actor/raw56 `3a92c29` integrated. | Designer-slot actor meanings in AI-CTRL-004 remain distinct unresolved operands. |
| AI-CTRL-005 | Raw207 actor behavior dispatch integrated as `d23c99c`. | Same-actor scheduler-status cases remain guarded; no scheduler reimplementation. |
| AI-SVC-001/002 | Source-backed AddHitPoints/Magic_AddMagicPoints integrated in first-visit work (`7bf7b08`). | Native helpers are not full saved health/magic ownership acceptance. |
| AI-AUD-001/002 | Voice/effects fade `54b1c2f` integrated. | Complete audio parity remains bounded by each test/trace. |
| AI-ACTOR-001/002/003 | Typed adjustment `f6d7d3b`, native travel `e67e55e`, activation/receiver work (`f3f5cda`, `60b3f16`, `7bf7b08`), waypoint containment `5cdc172` integrated. | Full original actor/lifecycle acceptance is broader than these finite paths. |
| AI-PRES-001 | Native animation frame query `8384dbc` integrated. | Frame query alone is not barrel AI acceptance. |
| AI-ACTOR-004/005 | Pickup/barrel candidate `1aa775e` plus audit `ed961c7` is preserved; assigned worker is completing actual consumers and WaitInteger `e793693`. | Await the reviewed coherent worker commit; do not register disconnected helpers as completed behavior. |
| AI-ACTOR-006 | Finite Gogoud/key/gate intro calls integrated (`8d68004` and following intro batch). | Full passive opening/eight-dialogue/player handoff remains unaccepted. |
| AI-AUD-003/004/005 | KillAllSounds, ChangeVolumeMusic, FadeObjectAndStop bounded source traces retained. | Unsupported paths remain guarded. |
| AI-AUD-006 | Receiver/nonblocking sound and fade-in integrated in `dcf30f3`. | Long ordered theme/mixer evidence remains separately scoped. |
| AI-PHYS-001, AI-COLL-001 | Partial wind/contact evidence retained. | Native feedback/qualified contact parity is not inferred from helpers; pickup worker owns reached collision completion. |

Camera shifts, visual diffuse mutation, finite intro calls and semantic named
receiver references discussed below are integrated (`41593ff`, `b1eb0fa`,
`fa26d73`, `71887e0`, `8d68004`, `9d93f17`). The latest root diagnostic still
identified first-visit selected armor and active camera mechanics callback
blockers; those are assigned separately. Title candidates `e9296e8` / `f7dc29d`
remain worker-owned until a reviewed complete title/style/state consumer lands.
The schema-3 fork checkpoint is a separate capability comparison assignment:
current schema-1 wiring does not dispose of useful preserved save functionality.

See the [complete disposition ledger](integration-disposition-2026-10-08.md) for
branch/worktree audit, superseded paths and missing evidence reconciliation.

## Preserved worker research snapshot

Snapshot: 2026-10-08, based on checkout `74338fb`. This is a finite source-led
backlog of source-reached gaps at that checkpoint and identified save-fixture
families, not a general engine API sweep. Camera is excluded. Source-unknown meanings stay
unknown; fixture values are regression expectations, not architecture.

## Historical statuses, owners, and branch

Statuses: **untraced** = candidate identified, identity/meaning unresolved;
**traced** = source path and gap located, implementation not underway;
**implementing** = peer claimed work; **verified** = focused acceptance passed
in isolation; **merged** = integrated and accepted. “Implementing” never means
the feature is complete. Owners are peer roles; “main to assign” is unclaimed.

Peers own separate save-global/inventory, saved actor-state, AI control/value,
AI actor-interaction, and AI presentation/audio scopes. Main owns merges and
shared schemas/compiler coordination. This catalogue started from `74338fb`;
main later integrated currency as `cb42a61`. At initial inventory, main reported
Rete 46/support 58 and a planned Rete 47 bump for `DesignerValue::Opaque`.
Rete versions are main-owned and the native save schema version is separate.
Existing camera, transition, menu, and loading-route owners keep their boundaries.

## Initial counts and priority

| Area | Bounded evidence snapshot |
| --- | --- |
| Supplied English save slot | 1,271 distinct identities / 1,534 player+named-level occurrences; 1,222 owned/typed; 508 meanings (503 source-verified, 5 runtime-corroborated); 763 meanings unresolved, 398 partly explained. |
| Initial source-reached non-camera AI snapshot | 14 unsupported operation IDs / 28 call mentions after adding the active Manor loader expression below, plus 2 partial behavior rows. Later traces add one separate imported Level call for function 184 and a reached model334 raw207 call. Category-1/raw-op56 has 49 imported calls; four were outside the baseline ActorKeyword path (one direct reference now merge-ready, three category-11 operands still unresolved; see AI-CTRL-003/004). The original 14/28 figure is the initial snapshot, not a recount of these later additions. |

The research checkpoint counted 37 rows: 14 save rows (2 merged baseline and 12 open:
2 merge-ready, 2 implementing, 5 traced, 3 untraced) and 23 non-camera AI rows
(7 verified and merge-ready, 0 implementing, 15 traced, 1 untraced; partial-
behavior, barrel/pickup, and Manor key/gate rows are included in the AI total).
“Traced” may still mean unresolved gameplay semantics or an unjoined save
identity; see each acceptance check.

Priority: close the finite merge-ready queue through main integration and
required Rete/support version coordination: save item-0 and actor Brain
projection, plus AI raw-op56, raw-op108, raw-op207, raw-op8, Global audio fades,
and function-184 frame query. Item-0's semantic importer passes the authentic
fixture, but main integration and native fresh-process persistence remain open;
actor Brain projection also needs importer and native consumer integration.
For the source-triggered Manor loader, semantic waypoint references are now
implemented in the waypoint batch; category5/op49 `Action_Camera_GoToWP` remains
camera-owned. Keep raw207's same-actor scheduler-status condition unresolved
and fail-closed. Resolve per-model raw56 designer-slot meanings or another
control-source blocker only when it is unowned and needed by the finite target
set. Continue the barrel generated-object and separate potion/gold pickup
traces without inferring explosion damage or equating their actors. The partial
Gogoud/key/gate source chain (`AI-ACTOR-006`) is deferred acceptance work and
does not block the first integrated playable checkpoint. Preserve unsupported
guards until behavior is proved.

## Historical checkpoint dependencies outside this catalogue

The current integrated-intro trace after `5403166` narrows the camera blockers:
at about +2 seconds, the camera Brain stops at node20, category3/op59
`GetShiftAlpha` (handler `0053d300`). The camera owner is assigned this
implementation. Hype Fix model28, Normal behavior0, script `Fix/17-0/37`,
instance `world/0/0`, stops at node1078, category4/op237
`Proc_SetVisualGMTDiffuseCoef`, before the aperture toggle. Its operands are
category36 GMT `Fix/6-1@729200` and category11 DSG87; that live target resolves
to `fix:/materials/06-01/18475`. The reached material is used by geometry
`fix:/models/geometry/18464`, object `fix:/objects/18460`, default config30,
on Manor New Game. The source inventory contains 330 calls across 11 scripts,
all using a category36 GMT and category11 DSG pair, with 30 distinct target
offsets, all in Fix/6-1. The API table identifies procedure237 at
`00543230` ([row610](research/hype-ai-api.tsv)). In the patch PE, that handler's
selector3 at `00543386` calls `0052efc0` and then
`GLI_xSetMaterialDiffuseCoef` (`0048f1b0`). The helper clamps four packed bytes
to 0..100 and scales them by 0.01; the setter writes four floats at VisualMaterial
`+0x14`, `+0x18`, `+0x1c`, and `+0x20`. Before evaluating the packed RGBA
operand, the handler resolves the category36 GameMaterial through
`GMT_fn_hGetVisualMaterial` (`0049df30`); a null GameMaterial or null
VisualMaterial skips RHS evaluation. VisualMaterial base color at `+0x34` is
independent. The handler mutates the resolved VisualMaterial in memory, and
later draws reread its diffuse fields. Hype DSG87 is an i32 at offset2244 with
initial buffer bytes `[0,0,0,0]`. All 30 target materials use flags `0x6ddc07ff`
and have unique visual pointers within the target set. Disassembly provenance
is patch PE SHA-256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`; installed
English EXE SHA-256 is
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`. The
installed EXE and patch PE are byte-identical for `00543230+0xc0`,
`00543386+0x3b`, `0052efc0+0x90`, `0048f1b0+0x22`, `004c5110+0x120`,
`00480080+0x120`, `004948c0+0x180`, and `0049d150+0x80`; this verifies the
installed handler and lighting/RLI routines for those ranges. The separate
local ICD SHA-256 is
`d61e1b45a1cae4c8f1b15aec4d2ab410943a90eaad8eaab58e57dae47b1db40b` and remains
encrypted; it is not the evidence source. The presentation peer's clean commit
`e970638824744abe3ebecc8432e32ab890a9346e` on
`codex/manor-visual-gmt-diffuse-20261008`, based on checkpoint `3eec3c4`, adds
typed `SetVisualGMTDiffuseCoef { material: AssetRef<Material>, value:
Expression::Variable }`, raw4/op237 parser/writer binding, Rete round-trip,
and an explicit codegen guard. Three focused tests pass: material identity and
live DSG gate through parser/writer/Rete/codegen; unbound GMT fails closed; and
non-integer designer storage is rejected. Main's fork-first integration has
cherry-picked typed commit `c8432db` and is wiring the Hype intro call through
a scene-owned runtime service; compiler edits are parked to avoid duplication.
This remains merge-ready typed preservation with integration underway. No
schema/support version bump was made by the peer slice. Main's intro acceptance
and tests are pending; do not mark the full path done.

Runtime material behavior remains incomplete while main wires the scene-owned
service. The reached model instance has empty serialized radiosity. `004c5110` lazily
creates per-LOD RLI through `00480080` (`GLI_xComputeLightForObject`) from
current global lights and actor transform; `004948c0` and `0049d150` consume
that RLI at draw time. Current Godot mesh colors bake
`2 * (ambient + diffuse * RLI)` while material `.tres` uses source
`VisualMaterial.color` for albedo and omits diffuse, so direct albedo changes
or stale static vertex colors are not equivalent. Runtime needs a dynamic
actor vertex-lighting service and shared VisualMaterial identity/state across
mesh, sprite, and opacity variants. The handler proves in-memory mutation and
later draw visibility only; save/reload and map-replacement serialization or
reset semantics remain unresolved. Do not assume the material is or is not
saved. Test shared visibility and repeated draw updates only until that path is
traced. **Status: typed preservation merge-ready with main integration in
progress; full runtime path incomplete pending intro acceptance and tests.**
The exact camera and presentation dependencies remain outside the counted
non-camera gap rows.
Missing configuration-bank metadata also remains under integration review.
Keep this bounded to the reported intro/gate path.

## Source save composition and coverage

Hype SAI saves are flat filtered views of one central runtime registration list,
not actor-framed chunks. Record layout is composed from registrations and
save-time address resolution, not a parallel authored offset map.

- `FUN_004c68e0` registers each live target pointer with type/extent and flags;
  `FUN_004c66c0` normalizes flags and eligibility. Registrations may retain a
  snapshot as well as a live address.
- `FUN_004c6d60` walks the selected runtime list in order. It serializes each
  registered field address through `Mmg_fn_vWhereIs` (`004ae420`) as packed
  block/module plus offset from the block base; load resolves that tuple to a
  current allocation (`004c7300`, inverse base lookup `004ae4c0`).
  `FUN_004c68e0` stores field pointer/type/flags and an array snapshot.
  `FUN_004c6e30` writes snapshot values for `0x0100/0x0400` and live values
  for `0x0200/0x0800`.
- Player `LevelNNN.sav` uses mask `0x0300`; named-level `.sav` uses `0x0c00`.
  A record can qualify for both. General files are separate metadata/timer
  paths.
- `FUN_004c7250`/`004c7300` decode and join identities back to current
  allocations/registrations; `004c6430 → 004c6d00 → 004c6c20` applies typed
  values and resolves array/reference forms.

See [Hype PC save-games](../docs/openspace/save-games.md), “File roles,”
“Decoded SAI records,” and “Registration and restoration findings”; decomp
anchors: [`004c68e0`](../reference/decompilation/functions/004c68e0_FUN_004c68e0.c),
[`004c66c0`](../reference/decompilation/functions/004c66c0_FUN_004c66c0.c),
[`004c6d60`](../reference/decompilation/functions/004c6d60_FUN_004c6d60.c),
[`004c6e30`](../reference/decompilation/functions/004c6e30_FUN_004c6e30.c),
[`004c7250`](../reference/decompilation/functions/004c7250_FUN_004c7250.c),
[`004c7300`](../reference/decompilation/functions/004c7300_FUN_004c7300.c),
[`004c6430`](../reference/decompilation/functions/004c6430_FUN_004c6430.c),
[`004c6d00`](../reference/decompilation/functions/004c6d00_FUN_004c6d00.c),
[`004c6c20`](../reference/decompilation/functions/004c6c20_FUN_004c6c20.c).
The reference decompilation is present in the main checkout but may be absent
from this isolated worktree; source addresses and the tracked save-game note
remain the evidence anchors.

Coverage axes must stay distinct. The current source join has 959
source-object/field identities, 261 live-registration identities, and 51
bounded-allocation-only identities. The 261 lack a bounded static block but
have exact live registration owners (201 dialogue-list and 60 map-star
fields). Of the 51 allocation-only identities, 45 remain meaning-unknown, 4
are source-verified and 2 runtime-corroborated. Separately, 718 records have a
known source/live field owner but unknown gameplay meaning; another 45 unknown
meanings are allocation-only. Thus “no static allocation match,” “no field
registration owner,” and “field meaning unknown” are not interchangeable
statuses. The 763 unresolved meanings include 398 with partial byte/bit/role
evidence. These profile counts come from `tools/save_semantics.py` and its
local `output/savegame-semantic-map/{coverage,semantic-catalogue}.json` report;
the durable checkpoint is [save semantic progress](savegame-semantic-map-progress-20260928.md).

One captured playable-Manor runtime registry has 2,180 registrations: 2,176
map to source allocations and 4 do not. The registrations reduce to 1,916
distinct block/module/offset keys; 260 keys occur twice (maximum multiplicity
2) with differing raw flags. For example, `0:5:1340` appears with
`0x1000031` and `0x1000204`. This is a single capture, not a census. The
normalizer `004c66c0` maps `0x0100|0x0200` to player eligibility mask `0x4` and
`0x0400|0x0800` to level mask `0x8`; flags describe route/value timing, not
owner identity. Keep duplicate registration/eligibility facts distinct from
the semantic owner join.

Brain values belong to each exact Perso owner: a shared AI model shares code,
not its actors’ Brain buffers. Inventory registration example:
[`00417fe0`](../reference/decompilation/functions/00417fe0_FUN_00417fe0.c).
Generic joins for all actor-owned properties and some registry partitions
remain unresolved.

### Allocation provenance is not semantic ownership

The supplied Manor allocation-join report queried 1,271 records: 1,010 fall
inside exactly one bounded Level/Fix allocation and 261 have no bounded source
block; it reports no conflicting-occurrence entries. These are allocation
provenance counts, not owner-join success. Examples establish why the join must
follow registrations and rooted source owners:

| Record / allocation evidence | Source-derived semantic owner | What the evidence does not prove |
| --- | --- | --- |
| Currency `0:10:5790` is inside Fix `[5780,5804)`, allocation 234. | Inventory handle from `00417fe0` / `DAT_006107bc`; quantity is signed `+0x0a` read by `0041d400` → `004a19d0`. | Allocation containment alone names neither inventory item nor field. Capacity `0:10:5828` is separately in `[5808,5840)` and is `max_element_capacity`; do not merge it with balance. |
| Hype DSG11 `0:17:15586` is in Fix `[13260,15600)`. | `level:/persos/world/0/0`, Brain `DSG[11]`; code model is shared Fix `Fix/17-0/28`. | Fix allocation/module does not make this field Fix-owned. |
| Global Brain `0:17:4560` is in Fix `[4312,4724)`. | Canonical owner `fix:/persos/15`; `level:/persos/world/0/1` is the appearance alias. | Do not replace the shared Fix owner with its Level appearance. |
| Lever `1:17:187072` is in Level `[187068,187076)`. | `level:/persos/world/0/174`, Brain `DSG[1]`. | The allocation alone does not establish the phase meaning. |
| Default magic/current/max/selection records `0:5:4986210/12/14/16` lie in Fix `[4986176,4986220)` (44 bytes). | Fix Magick resource: `00439830` registers `DAT_00610b98 +0x22/+24/+26/+28` as four `i16` fields with `0x0200`; `004398f0` registers `MagicLevel` and allocates `0x2a` bytes. | The generic allocation join does not name fields. Keep default/current/max/selected semantics distinct; do not create a parallel offset map. |
| Transform `2:6:1463692` and camera `2:6:1464196` each have bounded Level allocations. | Transform is separately importer-joined; camera is excluded. | Containment alone does not establish identity or meaning. |

The actor matches are stronger than allocation containment: fixture record
`0:17:15586` (`Level000.sav`, kind `0x10`, flags `0x0204`, value `00`) uniquely
matches one-byte Hype `DSG[11]` by exact address, extent and save bits, with no
descriptor ambiguity. Record `1:17:187072` (same kind/flags, value `01`)
uniquely matches one-byte lever `DSG[1]` by the same criteria. Thus
`single_bounded_allocation` reports placement only; descriptor uniqueness is a
separate join fact. Same-model Brains have per-Perso current buffers: model
identity is behavior/type reuse, not shared actor state.

The source save writer `004c68e0` registers live targets and `004c6e30`
serializes those rows after `Mmg_fn_vWhereIs` resolves module and relative
offset at save time. For targets not found in a module, it logs and falls back
to raw address arithmetic on the module `-1` path; keep such targets unresolved
or runtime-only rather than guessing an asset owner. Registration is also
conditional: `00438c30` registers StandardGame `+0x18..+0x1c` only when policy
`00438fd0` returns 4. A possible field or bounded allocation is therefore not
enough; registration, owner, route and runtime policy all matter.

### Active weapon and health negative evidence

`00417fe0` registers each inventory handle's selected byte `+8` (`0x0200`)
and signed quantity `+0x0a`, but not the distinct current byte `+0x1c` in that
initializer. `INV_fn_bIsSelected` (`004a1920`) reads `+8`; `INV_fn_bIsCurrent`
(`004a1930`) reads `+0x1c`; `INV_fn_hGetCurrent` and
`INV_fn_hGetSelected` are separate scans. Inventory navigation (`00419060`,
`0041b500`, `0041b4c0`) drives current. In the supplied fixture both item 21
(Object, `0:10:4240`) and item 80 (Weapon, `0:10:212`) are selected. Neither
flag identifies the active weapon, and the current byte is not proven saved.
This is why `SG-PLAYER-WEAPON-001` remains unresolved rather than mapping one of
the selected flags.

The 84 `standard_hitpoint_candidates` bytes from 28 rooted StandardGame
allocations (78 Level / 6 Fix) are exposed by conditional policy 4 registration
at `00438c30`: `+0x18/+0x19/+0x1a` are adjacent to init names
`HitPointsInit`, `HitPointsMaxInit`, and `HitPointsMaxMax`. This does not prove
Hype's runtime current health, nor any Hype armor identity. Do not map these
init/max fields to player current HP; keep `SG-PLAYER-HP-001` unresolved.

Evidence source for this subsection: peer's source-only join report,
`output/savegame-semantic-map/manoir-allocation-joins.json`, the named
decompilation functions above, and the [save registration notes](../docs/openspace/save-games.md).

## Save semantic boundary and open component contract

The flat decoder in [`src/openspace/save_game.rs`](../src/openspace/save_game.rs)
can retain source record identity, headers, raw bytes and unknown tails for
OpenSpace-side research or lossless source handling. Those are reader/private
diagnostic details, not executable semantic state. The current
[`godot/game/save_state.gd`](../godot/game/save_state.gd) centralizes a small
Hype/Global/dummy capture/apply list; it reads native resources and source
Brain nodes but does not yet delegate capture/restore to owner components.

Astrolabe's established boundary applies to save imports as it does to asset
imports: resolve structure semantics and named links inside the OpenSpace
reader, then expose typed semantic values keyed by stable named owners/fields
and named object references. Source module/block/offsets, allocation layout,
address/relocation resolution and raw registration flags stay private to that
reader or its diagnostics. Do not expose an address-binding manifest as a save
API or pass those fields into the shared Rust model, Rete or Godot. Actors and
components own their native semantic values; their contracts consume names and
typed state only.

The reader must apply save-route and snapshot/live rules while interpreting the
source, rather than forwarding registration flags. Conditional registration
means absence is not a proven zero: resolve availability and meaning where
possible, and leave unknown or unmatched source data in private preservation /
diagnostic handling. Never represent raw unknown bytes as supported runtime
state. Existing [`AiModel.variables`](../src/model/ai.rs) save/init descriptors
are typed source-derived semantics; they are distinct from live registry
addresses and raw route bits. The canonical model already stores semantic
state on `Perso` and `Brain` ([`character.rs`](../src/model/character.rs),
[`ai.rs`](../src/model/ai.rs)), with named owner relationships.

The save-import peer proposes implementing the OpenSpace semantic projection
in a new file/type with focused tests, without editing the shared decoder/model
or Godot adapter. The native component interface, envelope, schema bump and
migration policy remain for the integrator to decide. Current
[`save_state.gd`](../godot/game/save_state.gd) validates native schema 1; no
schema-2 decision has been made. The coordinator should own envelope/version,
validation, named-reference resolution, ordering and apply, while component
contracts own typed semantic values. No persistent schema or
component-contract/version change is decided.

### Reader-side ownership resolution evidence

The source reader can use registration and allocation details internally to
resolve saved records. The Level/Fix source graph links
Perso→Brain→Mind→Model→designer definition; `designer_candidates` in
[`save_join.rs`](../src/openspace/level/save_join.rs) follows those structures
and matches the current Brain buffer against source records. Its internal
address key is discarded at the semantic boundary; resulting evidence is a
rooted owner, appearance alias where applicable, `DSG[index]`, declared type,
and typed value.

- Hype `0:17:15586` is semantically `level:/persos/world/0/0` `DSG[11]`,
  although its bytes lie in a Fix allocation and its code model is shared Fix
  Brain `Fix/17-0/28`. Each Perso has its own current Brain state.
- Global `0:17:4560` is canonically `fix:/persos/15`; the
  `level:/persos/world/0/1` path is an appearance alias, not its owner.
- Lever `1:17:187072` is `level:/persos/world/0/174` `DSG[1]`.
- Currency `0:10:5790` belongs to the separate Fix inventory resource, whose
  semantic consumer is inventory item 0. The 60 map-star entries are dynamic
  registry state, so actor ownership alone cannot name them.

These facts explain why source addresses are useful inside the reader and its
diagnostics, but unsuitable as portable model or save fields. Canonical state
should carry meanings and named references; Rete should persist that semantic
model. Current native schema is 1. Component interface, envelope, version and
migration details remain for the integrator to decide; no schema-2 or metadata
change is decided. The semantic boundary is established; only the concrete
component contract and version details remain open.

Acceptance for the reader/model boundary: use source joins to produce typed
Hype and lever states on distinct owners, retain the Global Fix owner while
resolving its Level appearance by name, join the inventory resource to item 0,
apply route/snapshot rules internally, and verify no address or raw save flag
is required by a Rete or native Godot consumer. Preserve unknown records only
in the private OpenSpace source/diagnostic layer. This follows the established
Astrolabe semantic boundary; it does not decide a component API or version.

## Verified save baseline

| ID | Current behavior / source | Acceptance | Status |
| --- | --- | --- | --- |
| `SG-BASE-001` | [Importer](../src/godot/save_import.rs) maps Hype transform `2:6:1463692`, mana/current/max/slot `0:5:4986212/4986214/4986216`, item quantities 21/54/61/74/80, Global DSG149 `0:17:4560`, and dummy DSG8 `0:17:297732`. See [save docs](../docs/openspace/save-games.md), [import tests](../godot/tests/original_save_import.gd), [persistence test](../godot/tests/save_persistence.gd). | Existing source/fixture and persistence coverage; keep as baseline. | **merged** |
| `SG-BASE-002` | Gate Boolean `0:17:4560` maps to Global DSG149 used by Manor gate model 272; compiled path changes gate position. See [importer](../src/godot/save_import.rs) and [AI regression](../src/godot/ai.rs). | Gate restoration is completed; do not reopen/count as a gap. | **merged** |

## Save rows at the research checkpoint

### Inventory and player progression

**`SG-INV-001` — item-0 current balance.** `0:10:5790` is type `0x02`, flags
`0x0204`, fixture value 20. `00417fe0` registers handle `+0x0a`; `0041d400`
reads signed 16-bit via `004a19d0`; `0041d420`/`0041d490` add/subtract;
`0040c950` displays it and `FUN_00419450` initializes it on reset. Separate
capacity is `0:10:5828=200` (`+0x14`, `004a19f0`); `0:10:5792` is absent. Peer
commit `35ae6e7` is integrated as `cb42a61`; its fixture CLI reads 20 and
synthetic tests preserve explicit zero/reject missing or invalid records. A
follow-up semantic binding is implemented in commit `8a3fed0` on isolated
branch `codex/save-currency-semantic-binding`: it follows the Fix HUD root →
sole item child → inventory handle → item definition, then joins quantity and
capacity into `{component: inventory_item, item_id: 0, quantity, capacity}`.
Source offsets/record identities remain in reader tests, docs, or diagnostic
data and do not enter gameplay JSON. Ambiguous, missing, wrong-item, wrong-type,
or wrong-flag joins fail closed; explicit zero is retained. Tests verify the
semantic projection is unchanged when block/module/offset placement changes.
Focused checks passed: 3 reader tests, 4 importer tests, and a Godot editor
scan. Authentic fixture verification is now complete: the imported
`Level000.sav` hash `6786fe4f781b04d1c674e33f04f40b95c7ec9f1f82e0cff5d56574b1cddb672f`
matches `unzip -p` from `/home/stephen/Projects/astrolabe/reference/Savefile_EN.zip`.
The archive was copied to
`output/save-currency-authentic-validation/original-fixture/SaveGame0` without
modifying the original. CLI assertions passed for semantic item-0 `{quantity:
20, capacity: 200}`, native `quantities["0"] == 20`, no source-save/address-shaped
gameplay fields, and source records isolated to the diagnostic sibling; the
bounded Godot import harness also passed. Follow-up commit `b53ebfe` corrects a
numeric Variant assertion. A native reload test exists but has not been run.
Acceptance: integrate the semantic binding and verify native fresh-process
round-trip, import 20, round-trip changed signed values including explicit
zero, and keep capacity distinct. **Owner: save-global/inventory. Status:
importer and authentic fixture verified; merge-ready, pending main integration
and native persistence validation.** Evidence: [semantic progress](savegame-semantic-map-progress-20260928.md),
[evidence ledger](savegame-semantic-evidence-20260928.md).

**`SG-INV-002` — remaining item quantities.** Registration `00417fe0` saves
paired main-list number/quantity records for 62 handles; only codes 21/54/61/74/80
are imported. Owner: save-global/inventory for source join. Acceptance: derive
exact record↔item bindings, preserve signed special values including `-1`, and
round-trip each supported quantity. **Status: traced.** Sources: [semantic
progress](savegame-semantic-map-progress-20260928.md), [importer](../src/godot/save_import.rs).

**`SG-INV-003` — per-item selected flags.** The same registration join identifies
62 Boolean flags, not restored by the current overlay. Fixture flags for items
21 and 80 are both true; they do not prove active weapon. Owner:
save-global/inventory. Acceptance: route each flag by exact item handle, then
fresh-process native persistence; do not infer active weapon. **Status: traced.**
Source: [semantic progress](savegame-semantic-map-progress-20260928.md).

**`SG-MAGIC-001` — configured default magic points.** `0:5:4986210` is
`DefaultMagicPoints`, fixture 20. Fresh-process source probe substituted 8/40
and observed those values while current/max mana stayed 30/50. Importer omits
it and now reports this explicitly. Owner: save-global/inventory source trace;
no implementation claimed. Acceptance: route separately from current/max mana
and verify native restore. Registration evidence confirms a four-field Fix
Magick resource at `0:5:4986210/12/14/16`; generic allocation containment alone
does not name the fields. **Status: traced.** Source: [save evidence](savegame-semantic-evidence-20260928.md)
and [registration evidence above](#allocation-provenance-is-not-semantic-ownership).

**`SG-PLAYER-HP-001` — Hype health/armor.** Corpus contains 84 source-verified
actor HP bytes, but exact Hype SAI identity is not joined and the native overlay
omits health/armor; importer now reports the omission. Owner: save-global/
inventory traces; main assigns native service boundary. Acceptance: join exact
Hype current/max HP/armor fields, trace reset/clamp, then restore and persist.
Current evidence proves only conditional StandardGame init/max candidates, not
Hype current HP or armor. **Status: traced; Hype identity unresolved.** Evidence:
[semantic progress](savegame-semantic-map-progress-20260928.md) and [negative ownership evidence](#active-weapon-and-health-negative-evidence).

**`SG-PLAYER-WEAPON-001` — active weapon identity.** Importer reports it as not
restored; two selected flags true (21, 80) are ambiguous, and no distinct save
identity is joined. Source distinguishes selected byte `+8` from current byte
`+0x1c`; the current byte is driven by navigation but not registered in traced
`00417fe0`. Owner: save-global/inventory traces; main assigns after identity
proof. Acceptance: identify a distinct persisted source field and verify its
fixture state independently of selected flags. **Status: traced; saved current
weapon identity unresolved.** Source: [importer](../src/godot/save_import.rs),
[inventory evidence](savegame-semantic-map-progress-20260928.md), and [negative ownership evidence](#active-weapon-and-health-negative-evidence).

### Actor and world state

**`SG-ACTOR-001` — Hype DSG11.** `0:17:15586` is a one-byte Boolean for
`level:/persos/world/0/0`; model28 has 11 reads/8 writes. It is a partial
Object-info-code-44/procedure-82 latch; native purpose remains unknown. Actor
peer commit `af5edd1` adds a generic typed Brain capture/apply helper and a
synthetic fresh-process proof (Godot 4.7.2 and focused test pass). Follow-up
commit `57f9092` on a branch rebased onto main `8ff522b` adds the crate-private
OpenSpace projection `project_actor_brain_save_fields(...)` in
`src/openspace/level/brain_save.rs`, with tests in
`src/openspace/level/brain_save_tests.rs`. It resolves the source registration
join to semantic actor identity / Brain field without exposing source address
or route metadata. Five focused projection tests passed; the ignored
source-backed Manor/Fix join also passed against `disc/` for the two fixture
records. This projection is not connected to `save_import`, a shared schema,
Godot components, or generated scenes. The peer also reports
`cargo test -p godot-harness --test actor_world_save_state`, `cargo check
--lib`, and rustfmt passing; Clippy still reports unrelated existing warnings
in `src/godot/character.rs`, `src/godot/sound.rs`, and `src/model/camera.rs`.
Acceptance: route the exact typed value through the agreed importer and owner
contract without naming its unknown gameplay meaning.
**Owner: saved actor-state. Status: implementing; projection/test evidence
ready, consumer integration pending.** Source: [semantic progress](savegame-semantic-map-progress-20260928.md).

**`SG-ACTOR-002` — Manor secret-lever phase.** `1:17:187072` is Boolean DSG1
for `level:/persos/world/0/174` (`iLevierSecretManoir`), read by Normal 7/8 to
alternate paired actions/poses; fixture changed 01→00 after use. This proves a
phase/toggle, not “unlocked” or physical up/down. It is separate from imported
dummy `/world/0/27` DSG8. The same crate-private projection commit `57f9092`
and source-backed test include the two identified actor-owned Brain records;
they establish semantic projection only. No importer/schema/Godot consumer is
integrated. Acceptance: persist the exact typed value under the agreed stable
owner contract without elevating phase into a broader gameplay claim.
**Status: implementing; projection/test evidence ready, consumer integration
pending.** Source: [semantic progress](savegame-semantic-map-progress-20260928.md).

**`SG-BRAIN-001` — active behavior/state.** `current_normal=4`, `current_reflex=3`,
`current_state=47`, channels [28,71,80] exist in a hand-authored fixture, but
source record identity/encoding is unjoined. Do not persist fixture-only values.
Owner: saved actor-state source-traces after shared contract. Acceptance: prove
record and owner/version rules before adding component state. **Status: untraced.**

### Bounded unresolved save families

**`SG-GLOBAL-001` — remaining Global DSG.** The initial source census left 194
of the 200-record `0:17` Global family semantically unresolved. A later
source-backed join identifies `0:17:4562` as Global `fix:/persos/15` Brain
`dsg_147`, a Boolean set by Gogoud model210's Normal12 action/state sequence.
The actor peer's semantic projection uniquely resolves it and preserves the
distinct Level appearance alias `/world/0/1`; six synthetic and one source-join
projection tests passed, with no source addresses in the payload. Main approved
an optional schema-1 Global `dsg_147: bool` contract: old v1 slots may omit it
and retain their default; capture it when the Brain has the field and apply it
before deferred AI. No version bump is needed. Importer/native capture and
apply are implemented in `f7d01209f7d206e5d0ac15296c26ef082f3d1559` on
`codex/save-global-dsg147`; focused Rust/native/GDScript checks passed, and
authentic-fixture projection resolves `fix:/persos/15.brain.dsg_147=true`.
**Status: merge-ready pending main integration and route acceptance, not done.**
Verify optional-field omission/default behavior and application before deferred
AI in the integrated route. This flag does not prove a completed quest or key
ownership. The remaining Global fields still need source ownership, type,
read/write and effect evidence; preserve unknowns. **Owner: saved actor-state
with main schema coordination.** Source: [semantic
progress](savegame-semantic-map-progress-20260928.md) and actor Brain source
projection.

**`SG-DIALOGUE-001` — 134 dialogue references.** 67 list counts and 67 member
slots resolve to dynamic answer objects, not authored answer keys, text or
effects. Owner: main assigns. Acceptance: join answer and effect before restoring
a choice; preserve unresolved references. **Status: untraced.** Source:
[evidence ledger](savegame-semantic-evidence-20260928.md).

**`SG-MAPSTAR-001` — 60 star-entry words.** `1:10:(1248+64*n)`, `n=0..59`,
maps to each entry `+0x38`; fixture has 43 ones/17 zeros, but no gameplay reader
or non-init writer. Do not call this collected-star state yet. Owner: main
assigns. Acceptance: join reader/writer and test actual star state, otherwise
preserve exact value. **Status: untraced.** Source: [progress](savegame-semantic-map-progress-20260928.md),
[evidence](savegame-semantic-evidence-20260928.md).

## Non-camera AI rows at the research checkpoint

AI peers share `src/model/ai.rs`, `src/godot/ai.rs`, `src/openspace/ai.rs`,
`src/openspace/ai_values.rs`, `src/openspace/level/ai.rs`, and `tests/ai.rs`;
main owns shared compiler/schema coordination. Unsupported source fragments
must retain explicit guards until semantics are proven.

### Current Manor loader and Hype shared Brain

**`AI-CTRL-001` — category 3/raw opcode 188, `MTH_Modulus`** (PE handler
`00539f60`, signature `XX`).** The hash-matched installed/reference PE table
resolves raw 188 as `MTH_Modulus`; raw 189 is
`RelativeXPositionOfPerso` (handler `00539fc0`, signature `LL`). The older
[`hype-ai-api.tsv`](research/hype-ai-api.tsv) and source transcription use a
shifted alias for these names; preserve that provenance, but key operation
identity by category/raw ID and verified dispatch. Commit `79e7e78` corrects
the guard fixture/catalogue ID from raw 189 to raw 188. The two expressions in
Hype `Fix/17-0/28`, schedule `Fix/17-0/54`, still require exact source-node raw
ID reconciliation; the decompiled `MTH_Modulus` body is absent. Keep raw 188
guarded and do not infer `%` from the name. Acceptance: prove call-node IDs and
handler semantics before implementation; test edge values after semantics are
established. Owner: AI control/value. **Status: traced.**

**`AI-CTRL-002` — category 3/raw opcode 8, Manor tunnel distance expression.**
Reachability is `Level/11/01`, model 334 `iDummy_Load_Manoir_MAD`, Normal 0,
schedule ordinal 3, actor `level:/persos/world/0/34`; serialized at
`Level:11:01+0xd400`, VM `0x0900d420`, with opcode bytes `[08 00 00 00]`,
child category-0/op5 `MainActor`, compared `< Real(6.0)`. The installed PE table
at `0x6002d4 + id*56` (resolver `0x5271a0`) identifies raw 8 as
`Func_DistanceXZToPerso` (embedded row `0x600494`, signature `P`, handler
`0x537aa0`); raw 7 is `Func_DistanceXYToPerso` (`0x537a20`). Despite the
embedded name, handler `0x537aa0` calls `0x52ea00` with selector 3, absolute-
values the result and returns Real. Selector 3 at `0x52e910` computes
`sqrt(x^2+y^2)` after transforming the explicit actor delta through its inverse
matrix: target-local source XY magnitude, corresponding to Godot XZ. Raymap and
older tables label XY at index 8; retain as alias provenance, not dispatch
semantics. The actor operand supplies the target-local frame; self/current
actor must also be positioned or the implementation fails closed. No gameplay
distance cap was found. Commit `fc2c7e5` implements raw 8. Peer checks passed:
`cargo test --locked --offline --test ai_distance_source --test ai_modulus_source_guard`
(distance includes Godot `--check-only`), `cargo fmt --all -- --check`, and
`cargo test --locked --offline --test ai --no-run`. It is merge-ready pending
main integration. Owner: AI control/value. Acceptance: cover actor resolution,
positioning/fail-closed cases and the 6.0 comparison. Do not claim this node
caused the separate scene-entry failure. **Status: verified; merge-ready,
pending main integration.**

**`AI-CTRL-003` — category 1/raw opcode 56 explicit actor-reference form.**
This is the second `Cond_And` child in the Manor model 334 loader trigger.
Imported Rete `levels/manoir/ai/models/334.rete` (`level:/ai/models/334`),
used by `iDummy_Load_Manoir_MAD`, Normal behavior 0, has a child at ordinal 6
with raw node bytes `[56,0,0,0]`, followed by a category-24 actor reference
`Source { role: Level, block: [5,1], allocation: 33 }` and a category-32
reference to `level:/ai/models/154`, Normal behavior 0. This follows the
category-3/raw-op8 `< Real(6.0)` child. The ordinal-6 VM location
`Level:11:01+0xd418` / `0x0900d438` is inferred from the known op8 location and
8-byte node stride; raw SNA bytes were not re-read for this continuation. The
current scene import's SNA relocation/rooted identity map resolves the actor
to `level:/persos/world/0/3`, whose Brain uses model 154.

Dispatch is now source-traced. Research PE SHA-256 is
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`; the
installed PE SHA-256 is
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`. The
exact handler ranges and resolver helpers for IDs 56/57 match byte-for-byte,
though the whole PE hashes differ. The API table maps 56 to
`Cond_IsInComport` / `0x005345a0`, and 57 to `Cond_IsInReflex` / `0x00534630`.
The condition-row accessor `FUN_00526f70` uses base `0x5fe194`, stride `0x38`;
the raw-56 row points to handler `0x5345a0`, signature `PQ`. Its inline text
incorrectly says `Cond_IsInReflex`; retain both names as provenance and use
raw ID plus handler behavior as identity. Handler 56 resolves the actor's
active Normal behavior pointer and compares it with the referenced behavior;
an empty Normal slot returns false. Handler 57 reads Reflex and is also false
for an empty slot. Neither operation changes the actor's behavior.

The imported scripts inventory has 49 `Cond_IsInComport` calls: 45 use `Me`;
the four others use one category-24 direct actor operand and category-11
variables 1 (two calls) and 2 (one call). The direct-actor call is
`Level/17-1/128`, behavior 0, script `Level/17-1/137`, model
`iRajoth2_Astrolabe`; it is tested under `Cond_Not` against behavior
`Level/17-1/38668` and then used in `Proc_ChangeComport`. The actor's stable
semantic URI is not supplied in this call-site report. The Manor model-334
operand above is now independently joined to `level:/persos/world/0/3`; its
relation to the Level/17-1/137 call in the 49-call inventory is not joined, so
do not merge or double-count these occurrences without the source-key join.

At main base `8ff522b`, the typed parser maps category-1 IDs 56/57 to
`Expression::IsInBehavior`, but only category-0 Self/Main become its
`ActorKeyword`; explicit category-24 actor references are `Unsupported` there.
Peer commit `acfe8e5` on `codex/ai-control-flow-20261008`, rebased onto main
`8ff522b`, adds a typed `ActorTarget` reference to `IsInBehavior`, raw56
category-24 parse/write, and guarded native Godot query helpers. Handler
semantics were refined: it compares only the referenced actor's active Normal
behavior ID with the target ID; it does not compare model paths, so equal IDs
match across different models. Empty Normal slot returns false and no state is
mutated. Focused generated-Godot coverage exercises Self/MainActor,
cross-model same-ID, and null-slot cases; local Manor import plus Rete
round-trip passed (36s); parser/compiler and behavior-flow tests, rustfmt, and
diff check passed. The isolated change does not bump schemas. Main owns Rete
47→48 and support 58→59; the feature is not yet integrated. Raw57 explicit
actor-reference handling remains unsupported and its call reachability has
not been joined; keep it separate from this raw56 result.

Acceptance: verify the rooted actor binding and source writer/Rete round-trip;
return true only when that actor's active Normal behavior ID matches the
referenced behavior ID, including cross-model matches; return false for a
different ID or empty Normal slot; avoid state mutation; and fail explicitly
for unresolved actor references. **Status: verified; merge-ready pending main
Rete/support version bumps and integration.** Do not include it in the initial
count of 14 unsupported operation IDs.

The guarded Normal0 branch is not otherwise statically complete. After raw8
and raw56 pass, the true path has ordered camera/DSG updates, then raw207
`Proc_ChangeComport(MainActor, fix:/ai/models/28 Normal50)`, then self
`ChangeBehavior(model334 Normal1)`. Its observed writes include DSG[56]=1,
DSG[51]=0, DSG[62]=0, and DSG[123]=3; category-29 Waypoint values from Level
block `[17,1]` allocations 56–58 are assigned into DSG[157][0..2], with arrays
157 and 166 in the branch. It also calls category5/raw49
`Action_Camera_GoToWP` (signature `PW`, handler `0x00522a50`, follow
`0x0054f2d0`) with Fix actor `[5,0]` allocation 23322 and Level waypoint
`[17,1]` allocation 55. The waypoint batch now resolves direct category-29
values to named Waypoint assets and supports the actor-relative query used by
raw108; its main integration remains pending (see `AI-ACTOR-003`). Raw49 belongs
to the camera owner and is excluded here. Raw207 is tracked separately below.

Test-only commit `71de8ca` on `codex/ai-control-flow-20261008` (based on
`acfe8e5`) adds an ignored source-backed import test for the actual Manor Level
11 schedule node at `+0xd400`. It asserts the 14-statement true branch and Rete
round-trip, including direct category-29 Waypoint source identities in
DSG[157][0..2], and preserves raw49 and raw207 as unsupported. The manually run
test passed (1 test, 48.64s); fmt and diff checks passed. This establishes
static import/serialization only; no original-game run or runtime acceptance
is claimed.

Sources: [Hype AI API table](research/hype-ai-api.tsv); imported source call
inventory `output/ai-research/scripts.txt` (including lines 36953, 39917,
40691, and 41063); imported Manor Rete model
`levels/manoir/ai/models/334.rete`.

**`AI-CTRL-004` — category 1/raw opcode 56 designer-variable actor operands.**
The same 49-call inventory identifies three non-Me calls whose actor operand
is category 11: variable 1 occurs in schedule `Level/17-1/439`, script 460
(three potion actors) and schedule `Level/17-1/527`, script 548 (two gold
actors); variable 2 occurs in schedule `Level/17-1/574`, script 595
(`iFleche2_Astrolabe`). Each checks a target Level behavior at offset 70000
after a 400 ms wait. Source inventory excerpts are
`output/ai-research/scripts.txt:36953,39917,40691,41063`.

Although `DesignerValue::Actor` exists in the shared model, these per-model
category-11 slots have not been joined to typed designer declarations or
proven actor defaults/identities. Do not infer that they mean Actor or map
them to a scene node yet. Owner: AI control/value; depends on per-model
designer-slot evidence and the raw56 semantic actor representation decision.
Acceptance: prove each slot's declared kind and target binding separately;
then preserve source identity through codec/Rete and test matching, different,
and empty active Normal behavior plus explicit failure for unresolved targets.
**Status: traced callsites; designer-slot meanings and bindings unresolved.**
Source: imported AI call inventory `output/ai-research/scripts.txt` at lines
36953, 39917, 40691, and 41063.

**`AI-CTRL-005` — category 4/raw opcode 207, `Proc_ChangeComport`.** The
Manor model334 Normal0 branch reaches this operation only after the verified
guard `category3/raw8 < Real(6.0)` AND `category1/raw56` is true for actor
`level:/persos/world/0/3` in model154 Normal0. The call passes MainActor and
`fix:/ai/models/28`, Normal50. The pinned reference PE table names signature
`PQ`, row `0x60681c`, handler `0x00542650`; its rows match the installed PE
for this operation. Static disassembly resolves the callback: evaluate the P
actor and Q Normal behavior; if the actor's current Normal behavior pointer is
already Q, return unchanged. Otherwise save the prior behavior, replace it,
reset active schedule/action state (`00525e40`), and select Q's first schedule
or clear the active schedule. Model334 therefore requests MainActor Normal50.
The helper may set scheduler status 8 for a changed self-target, gated by the
target descriptor byte at `+0x10`; that byte's meaning is not established.
The source-backed preservation test `71de8ca` retains the 14-statement Manor
branch and Rete round-trip. Implementation commit `90f4726` adds the bounded
typed statement and native lowering: it checks target model identity, then
changes that actor's Normal behavior through its Brain. Unbound behavior stays
Unsupported, Reflex is rejected, and a same-actor target fails closed while
the callback's scheduler-status condition (descriptor byte `+0x10`) remains
unnamed. The implementation remains commit `90f4726`; its branch head is now
`a301a33`, which fixes a Godot test fixture regression by disabling both
processing modes and asserting that script errors are absent. The peer reports
the raw207 suite passing (4/4), actor-query suite passing (3/3), and the local
Manor model334 source test passing. It needs main's Rete schema bump 48 to 49
and integration.
Acceptance: preserve the same-pointer no-op and external MainActor
change; cover reset/first-schedule behavior, missing first schedule, false
guard, and fail-closed handling of the unnamed self-target status case. Keep
the separate `Level/17-1/137` callsite independent until its source target is
joined. **Status: merge-ready; pending main Rete schema bump and integration,
not complete.** Source: [Hype AI API table](research/hype-ai-api.tsv), pinned-PE
procedure row `0x60681c`, and `output/ai-research/scripts.txt` model334 branch;
reference PE SHA-256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.

**`AI-SVC-001` — `Proc_AddHitPoints` (procedure 55, signature `PL`,
`00540930`).** One Self + integer 1 call in the same Hype model/schedule’s
regeneration branch. Callee behavior is not yet established by the snapshot.
Current AI dispatch lacks a typed path. Owner: main to assign; actor-interaction
peer declined health-service semantics. Acceptance: prove target, mutation,
clamping and return/timing before adding a native call, then cover boundaries.
**Status: traced.**

**`AI-SVC-002` — `Magic_AddMagicPoints` (function 229, signature `L`,
`00536550`).** One integer-1 call in the same schedule’s other regeneration
branch; no typed path or proven clamp/return semantics. Owner: main to assign.
Acceptance: prove target/mutation/return and distinguish current/default/max
mana in a focused test. **Status: traced.**

**`AI-AUD-001` — `Proc_SND_FadeVoice` (409) and `AI-AUD-002` —
`Proc_SND_FadeSound` (410).** Each is `LL`, handler `0053ec70`,
`direct_actor_call=0`; one call each in Global model 1
(`level:/persos/world/0/1`), Normal 2, `Fix/17-0/21`, with operands `(0,800)`.
The exact sequence is FadeMusic(0,800) → FadeVoice(0,800) → FadeSound(0,800) →
ExecuteFadeMS → Wait → KillMusic → FadeMusic(127,1). Static disassembly
matched the installed/reference executable hashes and resolved the dispatcher:
409→`0040ba50` (handle `75ec64`, record `754300`), 410→`0040ba90` (handle
`75ec70`, record `7529a0`). Both capture start level, target, duration and
current clock; the global tick linearly interpolates clamped 0..127 and
deactivates at completion. Their records/handles are independent. Current
compiler baseline handles 408/411/382, not 409/410. Commit `c4890a6` adds the
pair on top of `cb42a61`; focused source-byte, Rete, GDScript and runtime tests
pass. Owner: AI presentation/audio, only this reached pair. Acceptance: verify
typed operands, start/mid/end interpolation, clamping, independent channels,
overlap/interruption and operation order. Zero-duration behavior and
waveform-level equivalence remain unverified. **Status: verified; merge-ready,
pending main integration.** Source identity table: [API rows 409/410](research/hype-ai-api.tsv);
static disassembly details are in the audio peer's
`notes/ai-audio-fade-static-trace-2026-10-08.md` (not yet present in this
catalogue checkout). Dispatch trace is static PE evidence, not an original-game
run.

### Hype actor interactions and presentation follow-ups

**`AI-ACTOR-001` — `Action_AdjustPersoOnObject` (meta-action 17, `PXXAA`,
init `0054ac30`, update `0054ad00`).** Hype model `Fix/17-0/28` has category-5/
op17 calls in scripts 193 (behavior 35), 195 (36), 304 (75), 316 (81), and 356
(99). `Fix/17-0/140` is instead the Normal-25 schedule; it has a category-4/
op209 behavior transition, but no op17 node, and its relocation to a target
BehaviorRef is unresolved. The op's first operand is a runtime expression
involving designer 133 (`kind=10 Actor`, init 0, offset 0); it is null in all
three imported banks. Source codec supports `Actor`/`SourceActor`, but runtime
target binding is unresolved. Threshold pairs are 1.0/1.1 or 1.3/1.4, with
action operands 3/4. Init uses strict ordered hysteresis: if distance `< arg1`,
choose operand 4 (near); otherwise if distance `> arg2`, choose operand 3 (far);
equality enters neither branch. A rejected first action transition does not
fall through to try the other action; selection depends on `00435250`. Operand
4 selects mode 1, which moves away until distance `>= arg2`; operand 3 selects
mode 2, which moves toward until distance `<= arg1`. Mode 0 completes
immediately. Update uses `0054ae90`/`0054a960` and `0054a9f0`; it writes
normalized target delta to movement component
`+0x1c/+0x20/+0x24` and queues command `0x14` through `0052eeb0`. Completion
thresholds are inclusive, the timeout is 5000 ms, and final settling displaces
self by local `-Y * abs(arg2 - arg1)`, clears the movement component flag, then
updates hierarchy through `0052ec40`/`0052edc0`. Movement is routed through the
DNM callback table (`00443540`); command `0x14` fields go to type-specific
handlers, so mapping the state DNM handler is the native implementation
blocker. A native movement callback trace is still needed before runtime parity
can be claimed. Decompiled update:
[`0054ad00_FUN_0054ad00.c`](../reference/decompilation/functions/0054ad00_FUN_0054ad00.c).
Peer commit `8c79d4b` contains typed source decode/write and AST traversal (2
focused tests passed); this is preservation only, not native behavior. Generated
Godot still fails explicitly. Acceptance: resolve target binding and movement
callback, then test strict/equal thresholds, rejected transition, mode 0,
inclusive completion, timeout, settling transform, flag cleanup and hierarchy.
**Owner: AI actor-interaction. Status: traced; feature incomplete, typed
preservation committed on peer branch.** The model is reachable for Manor Hype;
activation in SaveGame0 is not asserted.

**`AI-ACTOR-002` — `Proc_ActivateObject` (procedure 3, `0053fc30`).** Four
fixed-package calls occur in scripts 136, 267, 348, and 37; the source
inventory also has Level calls in scripts 144 and 304. Their exact model routes
and runtime reachability are not yet joined. Current parser/compiler has no
typed path. Wrapper `FUN_0053fc00` exposes only indirect dispatch, so final
callback and lifecycle are unknown. Owner: actor-interaction, queued after the
reachable Hype collision trace. Acceptance: join reached models, trace callback,
then test activation/deactivation and repeated calls. **Status: untraced.** API:
[Hype AI table](research/hype-ai-api.tsv); wrapper: [decompilation](../reference/decompilation/functions/0053fc00_FUN_0053fc00.c).

**`AI-ACTOR-003` — category 1/raw condition 108, `Cond_CollisionWP`** (API row
108, signature `PW`, handler `005356c0`). Hype actor `level:/persos/world/0/0`
uses model `Fix/17-0/28`; Normal 49 lists rules `Fix/17-0/241` and 242 with
schedule 240. Script 241 node 1 is raw kind 1/op108 with Self and
`Operator_Array(Designer157, Designer113)`. Designer157 is a 20-entry
`SourceWaypointArray`; designer113 is `UnsignedByte` and currently reads 0 in
the actor buffer. `scripts.txt`/older `names.json` calls raw op108
`Cond_NullVector`, but that table is two IDs behind after camera conditions;
the current API table/address/signature identifies op108 as `Cond_CollisionWP`
(`Cond_NullVector` is op110). Key the item by raw ID and verified dispatch, and
retain the stale name as provenance only.

Operand representation was the implementation dependency: `Designer157`
imports as `DesignerValue::SourceWaypointArray`, containing only
`SourceObjectIdentity` entries; `Designer113` is `UnsignedByte`; source tree
`(2,26)` decodes as dynamic `Expression::ArrayElement`. Commit `365c4da` on
`codex/noncamera-ai-interactions-20261008` adds the typed raw108 path and source
identity-to-waypoint resolution. Manor import resolves the Hype model28
Normal49/script241 dynamic field157/113 selection, and model334 Normal0's
category29 values to `level:/waypoints/17-1/56..58`. This supplies the reached
array/index and direct waypoint cases without exposing source addresses to
native saves.

The condition resolves the selected actor and waypoint, computes full 3D
Euclidean distance from actor translation to waypoint world position, then
returns true exactly when `distance - waypoint_radius <= 0.0` (inclusive).
`0052e810` computes distance; `005173f0` computes waypoint location from local
XYZ transformed by its matrix at `+0x14` when present; `005169e0` reads radius
at `+0x10`; `004bb390` reads actor translation. This is a fresh synchronous
waypoint-sphere query per condition evaluation, not physical collision: no
contact manifold, type/contact mask, other-actor filter, cache, timer,
hysteresis or dwell. The source has no explicit null/invalid guard and assumes
valid actor/waypoint operands; unresolved inputs must remain explicit rather
than silently evaluating false. Owner: AI actor-interaction. Acceptance: array
index switch, transformed center, exact-radius/epsilon-outside boundary,
vertical 3D offset, zero/negative radius, inside/outside independent of physical
contact, movement between queries, and explicit invalid-input behavior.
The committed path preserves unresolved/null operands and fails explicitly.
Focused checks cover exact-radius inclusion, outside epsilon, vertical
separation, unresolved actor/waypoint, movement of a relative actor, and the
actual source fixture/Rete round-trip. The peer reports these tests passing.
Main owns shared AST/compiler integration and schema/support versioning; no
main integration is reported yet. **Status: merge-ready pending main
integration/version coordination, not complete.** Source: [Hype AI table]
(research/hype-ai-api.tsv), read-only trace
`output/research/adjust-perso-ghidra/collision_wp.ghidra.txt`, and source-backed
Manor import for model28/model334.

**`AI-PRES-001` — function/API ID 184, `Func_GetCurrectAnimFrame` alias
(handler `005398b0`).** Four reached uses across two shared-Fix script sites
and one Manor-Level site are joined:

- `Fix/17-0/28`, Normal 31, rule 0, script `Fix/17-0/177`: getter `< 6`
  gates ground/slope collision response; then `Action_WaitNbFrames(6 - getter)`
  (source nodes 7 and 98; `scripts.txt` lines 15561/15652 and verified API
  lines 15269/15360).
- The same model's Normal 80 schedule `Fix/17-0/314`: after
  `Cond_IsValidGMT(designer[133])`, node 6 checks getter `> 81` before a
  fade/combat/magic sequence (`scripts.txt` 24059; verified API 23767).
- A fourth call is separate imported-Level reachability, not another shared-
  Fix call: `Level/17-1/612`, Normal 2, rule 0, script `Level/17-1/627`, node
  17. Its model is referenced by `iBaril1_Astrolabe_MAD` and five
  `iBarrils_Astro_Item01/02/03/04/05` actors. Under designer[9]==0 and an
  `IsInAction` match against one of Level action refs 586860, 577956, or
  592896, it checks getter `> 5`; only then, when designer[7] matches the
  shared Global sequence field, does it update four Global fields
  (`scripts.txt` 41510–41563). This is a frame gate in a barrel state path;
  no fixed fuse duration or explosion meaning follows from the getter itself.
  The paired schedule626 contains the generated-object/drop and cleanup path
  tracked separately in `AI-ACTOR-004`.

The handler reads current state through `004428c0` (byte at state `+0x78`),
masks `& 0xff` and returns an integer: an unsigned 8-bit frame cursor, not
elapsed milliseconds. Animation frame count is at animation-data `+4`; update
rate comes from `00442e60`, with elapsed conversion `rate * elapsed_ms / 1000`,
not a fixed 60 Hz counter.

Boundary behavior depends on playback mode. `00433e90` loops modulo frame count
N (`N-1 → 0`, range `0..N-1`). `00434530` reads loop mode through `00442ab0`
(state `+0x14b`) and has a looping out-of-range reset to 0. Non-loop completion
can expose N (`00433c00` stores N after passing count), while another
`00434530` path clamps to `N-1` depending on action flags. Do not impose one
universal frame boundary. No-animation remains unresolved: the getter directly
dereferences state `+0x78` with no null/default branch; `00433c00` returns early
when current action or animation data is absent and shows no cursor reset. Do
not synthesize 0 or -1. Feature commit `3e77cb0` adds the generic typed
operation using existing `Character.frame_index` (floor of current animation
position times source FPS). Generated guards require a non-null native clip
and positive source frame rate before reading the index; no usable animation
state fails explicitly. The peer reports its focused test passing (1/1), with
formatting and whitespace checks passing.

Reference decompilation hash `ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`
differs from the installed PE, but the listed handler/update/helper ranges were
byte-compared and match, including `005398b0`, `004428c0`, `00433c00`,
`00433e90`, `00434530`, `004368f0`, `00442ab0`, `00442e60` and `0044a890`;
this is not a whole-binary match. No live game run proves these call sites
execute in a particular playthrough. Owner:
presentation peer. Acceptance: assert the returned byte, time/rate advance,
loop wrap, both mode-dependent terminal boundaries, exact caller thresholds and
wait argument; keep absent-animation/state behavior unresolved and fail closed.
**Status: merge-ready pending main integration/schema 49, not complete.** API:
[Hype table](research/hype-ai-api.tsv).

**`AI-ACTOR-004` — Manor barrel contact, generated-object/drop and lifetime
chain.** The six reachable actors `iBaril1_Astrolabe_MAD` and
`iBarrils_Astro_Item01/02/03/04/05` share `Level/17-1/612`
(`output/ai-research/scripts.txt:41386–42077`, actor instances in
`output/ai-research/inventory.json:20278,20540,20734,20928,21122,21316`).
The existing `AI-PRES-001` row owns only API function 184/frame-cursor semantics.
The barrel actor chain is actor-interaction scope:

- Reflex/set1 behavior1 script635 tests condition 28 with `Me`, zone 1 and
  type 2; script636 also contains MainActor-state/distance and type-4096
  contact alternatives. The branch records/selects a barrel and changes its
  Normal/Reflex behavior (`scripts.txt:42077+`). This overlaps the approximate
  contact primitive in `AI-COLL-001`; it is not a waypoint collision. The
  model612 scripts contain no `Proc_ActivateObject` call, so keep the unrelated
  candidate row `AI-ACTOR-002` separate.
- Normal behavior1 script623 handles ground contact and its time/counter/action
  branches (`41424–41509`). Normal behavior2 rule script627 checks its own
  action against the three Level action refs at source offsets 586860, 577956,
  and 592896, requires designer[9]==0, then requires
  `Func_GetCurrectAnimFrame > 5` before writing shared Fix Global fields
  (`41510–41563`; detail in `AI-PRES-001`). This establishes an animation-frame
  gate, not a timed fuse or an explosion by itself.
- The paired Normal behavior2 schedule script626 starts at inventory line
  41562. Its first node, ordinal 0, is category5/raw40
  `Action_WaitInteger` (signature `LLL`, init `0054dd00`, update `0054ddb0`);
  operands are `Dot(Fix block[5,0] source ref at relative offset 4,350,568,
  DSG[5])`, integer 5, and DSG[7] (`41563–41568`). The schedule then waits on
  Global/local fields, checks quantities/capacities for item IDs 10–14, 80 and
  81, and reads current/max HP
  while choosing a candidate (`41569–41901`).
  Its operand tree is node0 raw5/40; node1 category2/raw12 `Operator_Dot`,
  with node2 category24 source ref value `40067720` (Fix block[5,0], relative
  offset 4,350,568) and node3 category11 DSG[5]; node4 category12 literal 5;
  node5 category11 DSG[7]. Callback operand order is live left integer,
  selector, live right integer, so this call compares Fix-reference DSG[5]
  `<` MainActor DSG[7]. There is no direct DSG[8] or DSG[9] operand in this
  call. The meta-action registry has 69 entries; row40's callback
  `0054ddb0` uses signed comparisons with selectors 0 `!=`, 1 `==`, 2 `>=`,
  3 `<=`, 4 `>`, and 5 `<`. Callback status 1 completes and status 4 keeps
  waiting; selector values above 5 remain pending with status 4. Peer commit
  `e793693` adds serialized `Statement::WaitInteger` with traversal/validation,
  category5/op40 parsing and writing, and inline Godot polling codegen. Focused
  source decode/write, Rete round-trip, unresolved-reference guard, and native
  generated-Godot tests pass. The harness checks selectors 0–5 plus 6/255/256
  (`256` wraps to selector 0), signed 32-bit edges, live rereads of all three
  operands, and cancellation by whether the following statement executes. It
  does not inspect source callback status integers 1/4 directly; those meanings
  remain source-traced, and exact parity for invalid selectors remains pending.
  The slice is merge-ready pending Rete 48→49 integration; the Godot support
  interface stays at 59.
- It contains nine category5/raw15 `Action_ExecuteActionAndGenerateObject`
  nodes (signature `PAV`, init `0054a5d0`, update `0054a110`). Each takes a
  Fix block[5,0] category-24 template ref, DSG[9], and vector `(0,0,0)`; the
  zero-based source node ordinals and Fix block-relative offsets are
  `339: 4,966,032`, `352: 4,961,336`, `365: 4,968,816`, `378: 4,951,964`,
  `391: 4,958,560`, `404: 4,949,180`, `417: 4,942,108`, `430: 4,975,548`,
  and `438: 4,978,316` (`41902–42007`). These templates are not yet joined to
  named collectible assets. The fallback at node449 is category5/raw4
  `Action_ExecuteAction` (signature `A`, init `0054a020`, update `0054a110`)
  with DSG[9] (`42012–42013`). No `Sound` or fade call occurs in these model
  scripts. No explosion or damage operation is source-proven here; the HP
  query is part of candidate selection, not an HP mutation.
- The schedule changes actions, waits 2000 ms or calls `Proc_KillPerso(Me)`,
  then transitions to Normal behavior3 (`42008–42043`). Behavior3 script629
  counts transparency down and calls `Proc_KillPerso(Me)`; behavior4 script631
  waits 500 ms between Action changes (`42046–42073`). In this PC API table,
  procedure 226's dispatcher returns through its default path, and current
  Godot lowering preserves it as a no-op; neither call proves deletion/cleanup.

The generic raw15 object/action/vector node has typed parse/write and Godot
scene-instantiation support (`source_execute_action_and_generate_object`),
but these Manor templates/action resources have not been semantically joined
or tested. Condition 28 is typed with approximate native contact semantics
(`AI-COLL-001`). The raw40 slice is merge-ready but unintegrated and does not
make model612 end-to-end supported. Inventory
function 196 `Func_Inv_AddObject`, maximum-capacity function 210, and HP-max
function 38 still lack typed parser paths. The imported model descriptor declares actor field8 with save flags
`0x0a00`, while field9 (the rule's one-time latch) is non-save
(`inventory.json:20399–20409`). No fixture-record join exists for either field
on the `/17-1/612` barrel owner, and field9's initializer is untraced; neither
restoration nor a default value is established. Do not conflate this model
with the distinct saved-Manor barrel actors `/world/0/71`–`/74`: those have
DSG[8] records `1:17:73944`, `82360`, `83312`, and `84264`, each byte `00`,
flags `0x0a0c`, in both `SaveGame0/Level000.sav` and `SaveGame0/manoir.sav`.
No saved spawned-drop identity is established. Keep the existing item-quantity
save gap under `SG-INV-002`; do not add source addresses to native state or
assume these `/17-1/612` actors or their spawned drops persist. Owner:
AI actor-interaction. Acceptance: resolve all nine templates/action resources;
preserve raw40 source/Rete/codegen/harness coverage through integration and
resolve invalid-selector parity; then
verify grounded/contact trigger, frame and wait ordering, generated actor
identity/transform, eligible item/capacity selection, repeated-entry/consume
behavior, and cleanup. Keep “explosion” and damage blocked until an action
resource or handler proves them. **Status: traced; generic spawn path exists
and raw40 is merge-ready; Manor-specific semantics and integration incomplete.**

**`AI-ACTOR-005` — Manor potion/gold collision pickups and effects.** These are
separate imported reachable models, not yet proven targets of the barrel's
nine generation templates. The potion trio `iPotionMagique1/2/3_Astrolabe`
shares `Level/17-1/439`; the gold pair `iGold1/2_Astrolabe_MAD` shares
`Level/17-1/527`. Their set1 behavior0 scripts464 and 552 have a proximity
branch requiring MainActor distance-Z `<2.25`, distance-XY `<1.0`, user-event
and GiBlock checks, plus a separate valid-GMT/assigned-object branch. Potion
script464 zero-based node60 is category3/raw196 `Func_Inv_AddObject` (signature
`P`, API table handler `00539b60`, `scripts.txt:39990–39991`) with operand
`Me` node61.
The AST places the call on the right of an assignment whose destination is
MainActor DSG[97]: Operator_Affect node56 → Operator_Dot node57 → MainActor
node58 / DSG[97] node59 → function node60; it follows `Action_WaitBoolean` on
Fix Global field19. Behavior1
script466 branches on item-field values 12, 13 and 14, updates magic-display/
global timing state and issues a sound request, then follow-up script467 reaches
`Proc_KillPerso(Me)` (`40290–40316`). Gold script552 checks capacity:
category3/raw210 `Func_Inv_GetObjectMaximumCapacity(-1)` zero-based node58
compared with raw198 `Func_Inv_GetGold()` zero-based node60
(`scripts.txt:40762–40764`). Both table
entries use handler `00539b60`, with signatures `L` and empty respectively.
After its behavior transition, script556 zero-based node4 calls
category3/raw196 `Func_Inv_AddObject(Me)` (signature `P`, handler `00539b60`,
`scripts.txt:40830–40831`), on the right side of an assignment to MainActor
DSG[97], then sets transparency and calls `Stats_DisplayMainActorMagic` with type 99, while
script557 reaches `Proc_KillPerso(Me)` (`40824–40915`). These are source
observations for these potion/gold effects only; they do not prove every pickup
heals or that barrel drops use these models. As in `AI-ACTOR-004`, operation
226's pinned-PC default
dispatch and current no-op lowering do not establish actor deletion.

Primary decompilation establishes [`INV_fn_wAddElementToItem`](../reference/decompilation/functions/004a1e00_INV_fn_wAddElementToItem.c)
(`004a1e00`) adds across matching instantiated items, caps each at
[`INV_fn_wGetMaxElement`](../reference/decompilation/functions/004a19f0_INV_fn_wGetMaxElement.c)
(`004a19f0`, item `+0x14`), and returns the remainder.
[`INV_fn_wSubElementToItem`](../reference/decompilation/functions/004a1f60_INV_fn_wSubElementToItem.c)
(`004a1f60`) subtracts across instances and deletes an empty instance only
when `MustDisappearWhenEmpty` metadata is set; otherwise it changes display/
prohibition behavior. [`INV_fn_wItemGetMaxCapacity`](../reference/decompilation/functions/004a3850_INV_fn_wItemGetMaxCapacity.c)
(`004a3850`) reads maximum capacity directly. A separate patched-executable
registry/callback trace identifies function row196 as one Perso→integer at
`00539b60`: inventory-family switch case60 resolves its actor through
[`00522610`](../reference/decompilation/functions/00522610_FUN_00522610.c),
reads the linked inventory item at `+0x38`, passes it to
[`0041c720`](../reference/decompilation/functions/0041c720_FUN_0041c720.c),
and stores a signed-16-bit return. `0041c720` is stateful insertion/selection;
its branches can yield 0 or `0xffff`/item-info. The wrapper body itself is
absent from the local decomp function inventory, and source actor/item binding
plus the remaining inventory side effects still need explicit modeling. The
helper behavior at `004a1e00` alone does not establish the wrapper contract.
`Func_Inv_AddObject` and `Action_WaitBoolean` (meta-action 38) lack typed AI
paths, and no native exactly-once collect/effect/cleanup test covers these
models. Existing inventory quantities are persisted through the separate
inventory save work (`SG-INV-002`); these sources do not establish whether a
world pickup's presence/consumption is saved. Owner: AI actor-interaction, with
inventory mutation and quantity persistence coordinated through the save
owner. Acceptance: bind each source pickup to its named scene; test both
proximity and assigned-object entry, capacity/block branches, one inventory
addition per pickup, distinct potion/gold effects, and source-proven despawn
or repeat behavior. **Status: traced; not implemented or fixture-integrated.**
API identities: [Hype AI table](research/hype-ai-api.tsv), rows 28, 31, 35, 38,
40, 184, 196, 198, 208, 210, 226 and meta-action rows 4, 15, 38, 40.

**`AI-ACTOR-006` — Manor Gogoud/key/gate chain.** This is a partial source
chain, not proof of a key grant. In imported Manor AI model 210
(`iGogood_manoir`, actor `/world/0/5`), Normal12 runs action family27/state36
and then writes Global `fix:/persos/15` DSG147=1. Normal13 tests DSG147: the
true branch reaches category4/raw4 (Raymap alias `Proc_DesactivateObject`,
native callback/effect unverified); the false branch deactivates channel4,
reaches unsupported category4/raw220, changes to action0 and returns to
Normal0. There is no raw196 `Func_Inv_AddObject` call in model210.

The source/save peer separately joined `0:17:4562` to canonical Global
`fix:/persos/15` Brain `dsg_147=true`, retaining the Level `/world/0/1`
appearance alias. Main approved this as an optional schema-1 Boolean with no
version bump; omitted v1 fields keep their default, capture occurs when the
Brain has the field, and apply precedes deferred AI. Six synthetic projection
tests and one source-join test passed. Native save consumer integration remains
pending (`SG-GLOBAL-001`). This proves the flag's source owner/value, not its
relationship to item21.

The separately named key prop is `/world/0/14`, model265
`iDummy_InventoryObjects_Manoir_GogoudKey`, family4; its imported default
schedule has no grant operation. The inventory presentation labels item21
“Key to Gogoud’s manor,” but no source join connects that prop or Gogoud’s
family27/state36 action to item21 ownership or transfer. Do not infer award,
consumption, or item ownership from the names or DSG147 transition.

An adjacent imported IWorld model154 path in Normal0 compares category3/raw205
with integer21 before setting Global DSG149 and requesting the grille's
Normal1. The API table names raw205 `Func_Inv_GetObject` (empty signature,
shared dispatcher `00539b60`; `notes/research/hype-ai-api.tsv:336`), but the
local decompilation has no body. The item21 label and this comparison are an
association to investigate, not proof that the expression checks possession
or grants/consumes a key. The surrounding spatial/actor conditions and
raw205's returned value semantics remain unresolved.

The gate actor uses model272. Its Normal0 true branch checks Global DSG149,
calls category4/raw253 with `SelfPosition + Vector(2.6, 0, 0)`, then changes to
Normal3. A separate Normal1 sequence reaches category5/raw36 `Action_MoveLift`
before writing DSG149=1. Imported IWorld model154 requests Normal1 after its
raw205/DSG149 conditions; raw205 inventory-query semantics remain unresolved.
The raw36 callback's exact movement/animation effect is also unverified. In
the supplied English fixture DSG149 is 1; the user identifies it as the
post-key/open state, but a saved flag alone does not prove the causal route. The existing
`SG-BASE-002` gate restoration is merged baseline and stays closed as a gap:
restore its behavior without counting it again.

Acceptance when this deferred chain is picked up: prove the exact inventory
query and all eligibility conditions; identify whether/how Gogoud awards item
21 and whether the gate consumes it; resolve the raw4/raw220 branches and
raw36 gate movement; then exercise one handoff and gate opening across save,
fresh-process reload, and a second load without duplicate transfer or repeated
position displacement. Keep unknown guards explicit. **Owner: main to assign;
coordinate with the actor and Global/save owners. Status:
traced, partial and blocked on source semantics; not a first-playable
checkpoint blocker.** Sources: imported
[`model210.rete`](../output/research-manoir-rete/levels/manoir/ai/models/210.rete),
[`model272.rete`](../output/research-manoir-rete/levels/manoir/ai/models/272.rete),
[`model154.rete`](../output/research-manoir-rete/levels/manoir/ai/models/154.rete),
[`model265.rete`](../output/research-manoir-rete/levels/manoir/ai/models/265.rete),
inventory `fix/inventory/en.rete` item21, and [API row205](research/hype-ai-api.tsv).

### `Level/17-1` audio queue

Separate from the reached Global fade pair; package/model routes are exact, but
the catalogue does not infer a display map name.

**`AI-AUD-003` — `Proc_SND_KillAllSounds` (385).** One call in
`Level/17-1/78`, model 64 `IWorld_Astrolabe`; current compiler lacks it.
Acceptance: prove which channels “all” covers and test active fades/subsequent
playback. Owner: presentation future queue. **Status: traced.**

**`AI-AUD-004` — `Proc_SND_ChangeVolumeMusic` (392).** One call in
`Level/17-1/813`, model 802 `i_DummyAmbSon_Astrolabe`; no typed path. Acceptance:
trace range/state persistence, then test reset and overlapping music. Owner:
presentation future queue. **Status: traced.**

**`AI-AUD-005` — `Proc_SND_FadeObjectAndStop` (412).** Four calls in
`Level/17-1/893,895,896,897`, model 887 room emitters; no typed path. Acceptance:
prove emitter target, curve and stop/lifetime; test finish/cancel. Owner:
presentation future queue. **Status: traced.**

**`AI-AUD-006` — `Proc_SND_FadeInObject` (413).** Two calls in
`Level/17-1/891,899`, model 887 room emitters; no typed path. Acceptance: prove
initial level/curve/reuse, test from silence/mid-fade/after stop. Owner:
presentation future queue. **Status: traced.** API rows for these four are in
[Hype AI table](research/hype-ai-api.tsv); eight calls remain outside the
Global fade-pair implementation.

### Bounded partial behavior

**`AI-PHYS-001` — `Proc_AddWind` (procedure 30).** Reaches `source_add_wind`,
but maintained character service is an intentional no-op. Only zero-vector
input is checked; nonzero drift/reset are unresolved. Support callback
`00589bfa` assigns actor-local drift. Owner: main to assign. Acceptance: test
nonzero vector, integration, zero reset and actor isolation. **Status: traced.**
Sources: [support README](../godot/addons/astrolabe/README.md),
[`character.gd`](../godot/addons/astrolabe/character/character.gd).

**`AI-COLL-001` — `CollidePersoZDENoWithTypeZDE` (condition 28,
`005338a0`).** Typed paths exist, but contact semantics are approximate/incomplete;
not interchangeable with `CollideWithNothing`. Owner: main to assign after
actor-interaction claim. Acceptance: verify ZDE filter, contact timing and
boundary/overlap from source. **Status: traced.** Sources: [Hype AI table](research/hype-ai-api.tsv),
[combat comparison](combat-comparison-2026-09-24.md).

## Deferred acceptance beyond the first playable checkpoint

These are explicit future targets, not source-traced feature rows. They are not
included in the catalogue counts or the near-term integration gate; identify
the exact source actors/scripts and expected transitions before assigning an
API implementation.

- **FA-DUMMY-001 — training dummy hit/spin reactions.** Future acceptance:
  demonstrate the source-backed hit reaction, spin/animation sequence, and
  return or repeat behavior under player input. Script/action identities and
  timing remain untraced.
- **FA-PLAYER-DEATH-001 — dying after damage/falling off a cliff.** Future
  acceptance: trace the fall/damage trigger, health change, dying transition,
  and resulting save/reload behavior. No condition, damage callback, or death
  state is claimed here.

## Exclusions and handoff

- Camera API/records (including `2:6:1464196`, StdCamer) are excluded. Level
  transition, menu and loading-route owners retain their scopes.
- Existing wait/control-flow/behavior-transition AST/runtime paths are not gaps.
- Gate `0:17:4560` and dummy DSG8 are merged baseline. Currency `5790` is
  current balance; `5828` is separate capacity.
- Catalogue lead owns only this document/reconciliation. Main owns shared API,
  schema and integration. **Merged** requires implementation, focused
  acceptance, and main integration.
