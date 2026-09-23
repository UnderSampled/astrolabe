# Hype AI evidence retained after the rollback

These are historical source observations retained from the archived experiment
`9374ef5` on `codex/archive-openspace-runtime-20260914`. They are not newly run
comparisons or evidence that player control works in the restored Godot port.
The archive's `docs/hype-player-control.md` contains the full investigation and
its intermediate passing/failing checkpoints. Current direction is in the
[gameplay plan](../godot-gameplay-plan.md) and [API assessment](../ai-api-assessment.md).

## Verified executable catalogue

Raymap was consulted first; its Hype list has 126 conditions, 240 functions,
443 procedures and 69 meta-actions. The supported executable's count/accessor
functions establish 128, 242, 446 and 69 respectively: **885 callable entries**.
Later IDs in the older list are shifted. Do not bind by those names blindly.

| Category | Table base | Record bytes | Count function | Handler accessor |
| --- | --- | --- | --- | --- |
| Condition | `005fe168` | 56 | `00526f90` | `00526f70` |
| Function | `006002a8` | 56 | `005271c0` | `005271a0` |
| Procedure | `00603798` | 60 | `00527290` | `00527270` |
| Meta-action | `0060a170` | 60 | `005273b0` | `00527460`, `00527480` |

Research executable SHA-256:
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.
Harness executable SHA-256:
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
The copies differ at three file bytes (`0x4e32d..0x4e32f`); the four API tables
were checked byte-identical during that investigation.

[hype-ai-api.tsv](../research/hype-ai-api.tsv) preserves the archive's catalogue:
category, ID, name, original parameter signature, entry address, update address
and direct-actor-call flag. Addresses and unresolved signatures are research
evidence only. This file is not compiled into the current Rust model or shipped
as a Godot operation table. The all-entry enum/stub generator stays archived.

The shared Hype AI model contains 107 normal and eight reflex behaviors,
259 unique scripts and 122 distinct callable entries. This is a static inventory;
it does not establish the reachable path from one save, or account for every
referenced camera, projectile or other actor's program. The assessment classifies
these calls by gameplay contract, not executable calling convention.

## Original save and controls

Recorded source run: 2026-09-13,
`/home/stephen/Projects/hype-harness/local/runs/20260913-155146/`.
Save archive SHA-256:
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The supplied English SaveGame0 reached `manoir`, loading phase 9, with 30/50 mana
and selected spell 0. This run used input and reads, with no memory writes,
level navigation, cheats or saved-game writes. All four installed save files
were checked byte-for-byte against the archive afterward.

| Input | Observed behavior |
| --- | --- |
| Arrows; Shift | Movement; walk/run selected different animation states. Save-point/slope interference means a clean speed ratio was not established. |
| Ctrl | Jump after backing away from the save point; airborne pose and return to idle on landing. Sampled source heights were 4.54942, 6.54690, 4.54931; the middle sample is not a measured apex. |
| X, then Space | Sword attack. |
| Z, then Space | Visible crossbow shot; ammunition decreased from 8 to 7. |
| Enter | Visible fire effect; mana decreased from 30 to 20. |
| Space near save point | Save interaction, cancelled without saving. Space is not the jump key. |

Screenshot names within that run: jump `screen-1789329572085344417.png`, sword
`screen-1789329575867934273.png`, crossbow `screen-1789329622746528045.png`,
magick `screen-1789329624830009114.png`. Timing was best effort, not frame-exact.
These observations do not yet establish matched Godot movement, damage,
projectile collision or spell lifetime.

## Research worth revisiting during a gameplay slice

- Action motion: archived `src/godot/animation.rs` converts sampled translation
  to a cumulative native animation track. Manual-state probes measured motion,
  not input-driven movement or floor contact. Review physics ownership, units
  and loop behavior before reuse.
- Character collision: archived `src/openspace/level/character_collision*.rs`
  separates useful collision profiles from instance activation; source/native
  selection probes provide evidence. Restore the fields/shapes needed for a
  real gameplay path and keep native moving-body restrictions explicit.
- Action state: transition and repeat/lock research belongs to the action's
  observable contract. Recover that meaning without restoring generic execution
  plumbing. An action can coordinate animation, motion, parts and spawned objects.
- Scheduling: source reflex priority, blocking, behavior re-entry and
  `Proc_SetActionReturn` affect visible timing. The 309 Hype uses comprise 303
  before behavior changes and six before incremental turning. Investigate those
  patterns as generated control flow; the old scheduler is not the default design.

Consult the archive for exact intermediate code/tests and corroborate assumptions
against Raymap/the engine when using them. No experimental runtime subsystem is
accepted simply because its isolated test once passed.
