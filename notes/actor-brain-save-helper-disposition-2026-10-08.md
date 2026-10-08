# Actor Brain save helper disposition — 2026-10-08

Audit baseline: `edb7704b2597c26c78086acad2f58eaba1db7a78`. Work is isolated
on `codex/brain-snapshot-integration-20261008`; main and peer checkouts are
unchanged by this worker.

Follow-up: the [save fork capability audit](save-fork-capability-audit-2026-10-08.md)
identifies useful persistence lost from `294bd26`. Retiring the generic Brain
wrapper does not retire that fuller component save behavior. Schema/inventory
adoption is assigned to the pickup worker; this branch additionally fixes live
Boolean target preflight without changing slot schema or admitted fields.

## Decision and contribution accounting

Do not replay `f21a7fcb2976109682d6bf6ac5cc88e434ea58ec`'s
`character/actor_save_state.gd`. It is a generic wrapper around typed Brain
property capture/assignment, with no packaging, scene export or game consumer.
The supported slot already implements its useful storage operations in
`game/save_state.gd`. A new Node on every actor would introduce another storage
API without a demonstrated gameplay difference. The slot's existing narrow
field admission is the owning contract; a generic `dsg_N` patch is not an
instruction to admit every registered source field to gameplay persistence.

The two demonstration fields in the old test are **not** aliases of the slot's
existing flags. Their omission remains explicit supported-scope information,
not a claim of full Brain restoration. The old helper does not implement their
gameplay meanings, save admission or source AI continuation, so installing it
would not complete those features.

| Original file/change | Disposition |
| --- | --- |
| `godot/addons/astrolabe/character/actor_save_state.gd` | Superseded storage wrapper; deliberately not installed/exported. |
| `godot/tests/actor_world_save_state.gd` | Replaced with a regression of the production slot API and maintained native inventory/magick Resources. Retains repeated-restore, invalid-patch atomicity and fresh-process checks. |
| `crates/godot-harness/tests/actor_world_save_state.rs` | Replaced with packaged-project syntax/write/reload stages, isolated XDG directories and owned logs. No nested `OS.execute` launches. |
| `tools/test-manifest.json` row | Adopted under the original test identity, with a synthetic fixture and 25-second process-tree deadline. |

This closes the integration disposition of `f21a7fc`; it is not awaiting a
cherry-pick. The replacement tests exercise a real consumer instead of keeping
a disconnected helper alive. They contain synthetic values, not original save
bytes or generated game assets.

## Exact current equivalents

At the audited baseline:

- `save_state.gd::validate` admits exact owner/field keys and requires Boolean
  values for Global `dsg_149`, optional Global `dsg_147`, and manor dummy `dsg_8`.
  Unsupported actors/fields are rejected before mutation.
- `_source_brain` and `_dummy_brain` resolve semantic role URIs through the
  exported level's `source_actor_brains` NodePath map. The canonical Global owner
  is `fix:/persos/15`; the dummy owner is `level:/persos/world/0/27`.
- `capture` reads those typed Brain properties together with the player's
  native magick, inventory and transform. `write` validates that complete state
  before writing a temporary file and atomically replacing the slot.
- `apply` validates the whole envelope, checks required live components and
  optional field availability, and validates inventory on a private clone
  before assigning live values. Repeated assignment does not start/reset Brain
  execution. Omitted optional `dsg_147` preserves the scene-defined value.
- `game/demo.gd` reads the slot/original import, calls `SAVE_STATE.apply` before
  player startup and calls `SAVE_STATE.write` from `save_game`.
  `game/bootstrap.gd` reads the slot before offering Load Game.
  These are actual game consumers.
- `src/godot/support.rs::STARTER` already packages `game/save_state.gd`.
  `src/godot/ai.rs::component` exports typed Brain properties and current values;
  `src/godot/scene.rs` persists the semantic Brain/node indices. `variable_type`
  maps `DesignerValue::Boolean` to native `bool`. No actor snapshot component is
  required for ordinary typed-property assignment.

The old helper's property-list/type checks are now split between the concrete
slot schema and the generated typed Brain. This is equivalence for the
supported fields, not permission for arbitrary Brain patches. The regression
also exercises unavailable optional properties to ensure failure precedes any
live mutation.

The original helper's Hype `level:/persos/world/0/0` `dsg_11` demonstration is
only a partial inventory-info-code-44 latch trace. The lever's
`level:/persos/world/0/174` `dsg_1` is an alternating phase in model 1396;
physical up/down and passage state are not established by that Boolean. The
dummy's `dsg_8` is a separate supported world flag. See
[the source save evidence](../docs/openspace/save-games.md) and
[the durable semantic ledger](savegame-semantic-evidence-20260928.md).
The new regression asserts that the slot rejects both old demonstration fields
and leaves an unmapped Hype `dsg_11` property unchanged.

## Source and importer evidence retained

The later importer contributions are already integrated:
`2337318` (semantic Brain projection), `8a344d5` (Global gate projection test),
`1c5e5f5` (optional projection), and `be0b88e` (optional Global latch consumer).
`src/openspace/level/brain_save.rs` consumes SAI locations only at the reader
boundary and emits owner/component/field identities plus `DesignerValue`.
`brain_save_tests.rs` retains the Hype/lever Boolean projection examples,
canonical Fix owner versus level appearance, ambiguity rejection, malformed
Boolean rejection and address-free payload assertions. Those are reader/data
evidence, not proof of gameplay restoration of every projected field.

Read-only corroboration used Raymap's `AI/DsgVarValue.cs` Boolean reader, then
`hype-decomp/src/ai/designer_variable_storage.c` and its initialization paths.
The decompilation's `ai_designer_storage_size` type-0 case is one byte;
`ai_write_designer_byte` performs a byte store, and current/initial buffers are
distinct per instance. This supports typed value storage, not a new gameplay
name or reconstruction of the source memory machinery in Godot. Nothing was
built, tested or modified in hype-decomp. No original game process was launched.

## Verification and limits

The source-free production-slot regression checks:

1. Fresh processes call real `write`, `read` and `apply` using packaged files.
2. Explicit true/false Global/dummy values survive serialization and restore;
   native inventory/magick values restore alongside them.
3. Repeated restore/capture is idempotent (JSON numeric normalization is
   accounted for; parser numeric scalars are floats).
4. A valid earlier flag change plus an unknown/non-Boolean field is rejected
   atomically. Missing or type-mismatched live dummy, required Global and
   optional Global fields also fail before mutation and invalidate capture.
5. Omission preserves the optional scene value; rejected writes preserve the
   prior slot; the old Hype/lever demonstration fields remain outside admission.

Early Godot 4.7.2 syntax/write/reload checks passed in approximately 0.42 seconds
total, with explicit owned logs in ignored
`output/brain-snapshot-preflight/`. The packaged regression also passed under
the compiled Rust harness in 0.44 seconds. Its separately supervised execution
reported exit 0, no timeout, no leaked children and no remaining processes;
owned stage logs and the synthetic saved slot are in ignored
`TestResults/brain-snapshot-focused/actor-world-save-state/`.
These tests establish supported slot storage semantics;
they do not establish full original save parity, gate/lever animation behavior
or original-engine continuation.

After adding live Boolean preflight, the final packaged regression passed in
0.43 seconds under its 25-second supervisor deadline, with no timeout, leaked
children or remaining processes. Evidence is in ignored
`TestResults/brain-snapshot-typed-deadline-final/`. Its negative control uses
the pre-fix production slot from `9284244` with the same final test: writing
passes, then reload fails on the incompatible dummy Brain. The fixture restores
the valid Global Brain before each independent invalid-live-field case, so
missing optional state cannot mask a dummy-field failure.

There is a pre-existing baseline discrepancy outside this helper assignment:
the executable `game/save_state.gd` supports schema 1, while
`docs/astrolabe/godot-project.md` describes schema 3 and portions of
`godot/tests/save_persistence.gd` expect checkpoint/schema-3 functions.
That older harness's mock inventory also lacks the current `clone_for_scene`
and `restore_legacy` contracts. Do not count that documentation/harness as
executed baseline coverage. The new regression uses maintained Resources and
the currently packaged implementation; reconciliation of the broader save
contract remains owned by integration, not hidden by this helper disposition.

Additional checks:

- `cargo test --locked --offline --profile validation --lib brain_save::`:
  six passed, two local-source checks intentionally ignored. The latter were
  not claimed as source-corpus verification.
- `cargo test --locked --offline --profile validation --lib godot::save_import::`:
  six passed, including optional Boolean/absence and address-free payload cases.
- Pinned-nextest Python supervisor/selection/cleanup checks: 29 passed in
  6.209 seconds. The first manual attempt lacked the worktree's nextest path;
  setting `ASTROLABE_NEXTEST` to the already installed pin resolved it.
- `cargo fmt --all --check`, `git diff --check`, and
  `cargo clippy --locked --offline -p godot-harness --all-targets -- -D warnings`
  passed, including the final live-preflight edit; final clippy evidence is
  `TestResults/brain-snapshot-final-clippy.log`.
- The earlier supervised `cargo build --release --locked --offline` passed in
  90.32 seconds with no leaked children. Final validation after the small
  GDScript preflight change used packaged Godot execution and scoped clippy;
  it did not repeat the release build.
- Full supervised `python3 tools/test.py --area save --offline --timeout 900`
  failed at build/discovery after 105.94 seconds, before tests ran:
  `crates/orrery/tests/comparison.rs:37` initializes `Perso` without
  `microphone` and `motion_binding_present`. This defect is present in
  `edb7704`; the helper patch does not edit that file. The runner report is
  `TestResults/brain-snapshot-save-20261008.log.json`, with source unchanged and
  successful cleanup. Required whole-selection safeguards therefore remain
  unverified; the focused passes are not a green save-suite result.

Integration subsequently reported those Orrery fixture/lint repairs in main
`98a3d2e` and inventory/loading changes in
`c375693b5c8310ffc8450dd97ce3be0e910083a3`. This isolated branch remains on
the declared audit baseline, does not duplicate those repairs, and does not
claim validation of the newer combined main. Integration owns applying the
narrow shared-file preflight and adapting this schema-1 regression to the
reconciled slot schema; the generic helper itself has no pending integration.
