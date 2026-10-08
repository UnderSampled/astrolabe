# Save fork capability audit — 2026-10-08

Compared `294bd26`'s actual `game/save_state.gd` and maintained components
against `edb7704`, independently of version numbers. The fuller fork is not
wholly superseded. Several real persistence capabilities were lost while newer
Global/import behavior was retained in main.

The subsequent delegation assigned this branch the independent missing life,
armor, checkpoint, identity and scalar behavior. Their concrete implementation
and schema/ownership evidence are now recorded in the
[native checkpoint restoration](native-checkpoint-restoration-2026-10-08.md).
The table below describes the original audit baseline, not that follow-on state.

## Behavior comparison and required reconciliation

| Capability | Fork `294bd26` | Baseline `edb7704` | Disposition |
| --- | --- | --- | --- |
| Native magick points and spell | Saved/restored | Saved/restored | Retained; keep current import/seeding. |
| Sparse quantities and weapon selection | Legacy schema 1/2 read | Schema 1 read/write | Retained, but insufficient for full admitted inventory. |
| Inventory member identities/order, admission slots, gold, capacities, object/magic selections | Component `capture_snapshot`/`validate_snapshot`/`apply_snapshot` | Component APIs remain, slot does not call them | Real lost publication/restore behavior; pickup worker owns adoption. Sparse ID dictionaries cannot represent distinct members with equal numeric IDs or distinguish unadmitted quantities from owned slots. |
| Armor selection, resistance, current damage piece, broken flags and strikes | Component snapshot persisted | Component APIs remain, no slot armor | Real lost persistence. Use current armor catalogue snapshot API and validate exact ordered source bindings on private clones before live mutation. Artwork/thresholds/factors remain generated definitions. |
| Life current/maximum/limit | Persisted for schema 2/3 | Native Resource remains, slot omits life | Real lost fresh-process persistence. Current transient level transfer of life is not an equivalent. Adopt explicit checkpoint life values; fork's alive/coherent admission limits are authored checkpoint policy, not universal source API invariants. |
| Placement | Full origin and nonsingular Basis | Optional schema-1 `transform` with origin and orthonormal Basis | Core placement retained in a different representation. Full finite scaled/sheared Basis acceptance is narrowed, not entirely absent. Keep current original-import transform compatibility if restoring schema-2/3 placement. |
| Player identity | `resolve_player` checks semantic URI map and actor metadata | Session selects a hardcoded node path; save accepts caller actor | Lost identity check. Session and persistence should use one semantic resolver, verifying `level:/persos/world/0/0`. |
| Save admission | Idle action/normal/reflex, running supported Brain, floor contact, settled movement, released controls, no projectiles | No checkpoint admission in `write` | Lost guard. Restore a declared safe checkpoint policy before publishing richer state; do not claim arbitrary active AI/coroutine restoration. |
| Supported dummy `dsg_8` | Persisted | Persisted | Retained. This is separate from the placed barrels owned by pickup work. |
| Global `dsg_149` and optional `dsg_147` | Absent | Persisted; optional omission preserved | Newer useful behavior; preserve during reconciliation. Never replace with unverified Hype/lever fields. |
| Original-save import | Absent | Read/validate/import route and saved transform present | Newer useful behavior; preserve reader-boundary projection and legacy admission. |
| Save publication | Temp file then atomic rename | Same | Retained; preserve rejected-write safety. |
| Snapshot scalar validation | Finite integral JSON numbers, range checks, canonical item keys | `TYPE_INT` only; weaker item-key guard | Lost useful helper semantics; restore the JSON-compatible integer guard for the pickup worker's snapshot publication path. |

Schema-2/3 slot acceptance itself was also lost: main rejects those envelopes.
The useful approach is to restore the missing semantic fields through the
current component APIs while retaining current Global/original-import behavior,
not to replace the current file wholesale with the fork. Both inventory and
armor Resources remain scene-owned mutable graphs with named catalogue/member
identities. No source-address, relocation, encoded-byte replay or AI scheduler
snapshot belongs in the native slot.

The session now has a different native loading/transition lifecycle. A save
reconciliation should change only its actor resolution and slot consumption;
the fork's old `LevelPreparation`, authored Crossbow input and gate-placement
workaround are superseded by current source-driven consumers and must not be
reintroduced with the save patch. Current load ordering already applies the
fixture and then slot/import before physics and player Brain startup; retain it.

## Concrete snapshot read counterexample

Static inspection found main's `snapshot_values.gd::integer` requires
`TYPE_INT`; the fork accepted finite integral `int` or `float` values. Godot
JSON parsing returns numeric scalars as floats. A source-free, isolated native
checkpoint draft using the current maintained inventory/armor Resources wrote
schema 3, but its own `read` rejected the slot with:

> Inventory snapshot version or catalogue is unsupported.

The saved inventory had the correct version 2 and
`fix:/inventory/gameplay` catalogue. Replacing **only the draft project's**
snapshot guard with the fork's guard let write and fresh-process reload pass.
The draft also checked life, capacities/gold, unadmitted object state, selections,
armor damage, legacy schema-1 transform and schema-2 life, optional Global
omission and atomic rejection of mismatched component bindings. This is narrow
native-component evidence, not gameplay or original-engine acceptance.

Ignored evidence lives in `output/schema3-save-preflight/`: `read-probe.log`,
`write-fork-guard.log`, `reload-fork-guard.log` and the synthetic user-data slot.
The complete unaccepted draft and source-free fixture are preserved under
`output/schema3-save-comparison/`. They are comparison artifacts, not an
integration candidate or a production schema decision.

## Ownership and independently completed improvement

The subsequent delegation assigns inventory snapshots, placed-barrel `dsg_8`
and game-slot schema ownership to the pickup worker. Accordingly this branch
does not add competing schema/inventory/barrel code or change main's stable
save documentation. Integration owns sequencing of the life, armor, checkpoint
and identity reconciliation listed above. Shared files are `game/save_state.gd`,
`game/demo.gd`, `character/snapshot_values.gd`, the save harnesses and manifest;
this branch's production edit is only the Boolean field preflight in
`game/save_state.gd`.

The helper audit did expose one independent real gap: baseline `apply` checks
existence of Global fields, but fails to validate the live dummy field's presence
or any mapped live field's Boolean type before changing magick/inventory.
The replacement regression fails against baseline on an integer-typed dummy
Brain with "Incompatible live Dummy Brain was accepted or partially changed
components." The focused fix adds `_has_boolean_field`, checks all mapped live
Boolean targets before mutation and rejects invalid captures. Valid slot shape,
field ownership, omission and schema remain unchanged. Its regression covers
missing/type-mismatched dummy, required Global and optional Global properties.

That production fix preserves f21a7fc's useful type/atomicity finding through
actual slot consumers without installing the redundant generic Node. See the
[helper disposition](actor-brain-save-helper-disposition-2026-10-08.md).

Final packaged production-slot execution passed in 0.43 seconds under a
25-second deadline. The pre-fix production slot fails the same regression
on the incompatible live dummy Brain, independently of the optional Global
case. Both executions completed with no leaked children or remaining processes;
evidence is in ignored `TestResults/brain-snapshot-typed-deadline-final/`.
Scoped clippy, formatting and whitespace checks also passed. This verifies the
independent Boolean fix, not adoption of the broader draft or current main's
reconciled schema.

Main integrates this regression and Boolean preflight as
`6f35ceb773289f93af13c6320d649b7f8369721b` and
`542c836e44ee748f396b4ccab6cc3c7b81fdc2c8`. The merged production-slot case
passes fresh-process/type/atomicity checks in 1.01s, within the 42-case focused
integration run. See [the integration disposition](integration-disposition-2026-10-08.md)
for merged evidence and retained follow-up ownership; this does not adopt the
broader life/armor/checkpoint/identity or inventory/barrel capabilities.
