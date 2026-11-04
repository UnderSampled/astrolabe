# Placed actors as shared scene instances (2026-11-04)

Request: level trees owned every actor's internals; the Manor's three training
mannequins were editable as level nodes. Design: [Godot project
structure](../docs/astrolabe/godot-project.md#editing-and-regeneration).

## Before

- Each perso got its own fully copied scene (`persos/<name>__<digest>.tscn`),
  built from a shared rig function. The three mannequins (`level:/persos/world/0/152..154`)
  were 52-node copies differing only in identity metadata, ZDE/ZDR
  `source_collision_perso` metadata and Brain `dsg_0` (2/1/3).
- `expose_scene_tree` in `build.gd` wrote `[editable path=...]` for every nested
  instance (d8d0375), and `scene.rs` set Brain/SourceCamera fields inside actor
  instances through `set_editable_instance`.
- Godot 4.7.2 drops overrides on non-editable instance children (probed), so
  per-instance child data needs a root seam; a `_get_property_list` mirror would
  be lost on editor re-save because actor scripts are not `@tool`.

## Measured (`tools/scene_reuse.py`, fresh conversions from 8b2ad11 vs this change)

| | Manoir before | after | Brigand before | after |
|---|---|---|---|---|
| level nodes | 6716 | 1149 | 12912 (installed) | 1328 |
| level nodes inside instances | 5567 | 0 | 11584 | 0 |
| editable flags | 2718 | 0 | 7291 | 0 |
| distinct actor scenes / instances | 214 / 214 | 88 / 214 | 285 / 285 | 143 / 285 |
| level sub-resources | 810 | 432 | 877 | 376 |
| actor scene files | 200 | 74 | 227 | — |

Shared examples: Manoir dummies ×105, Biblio ×5, barrels ×4, teleporters ×4,
mannequins ×3; Brigand dummies ×46/×18, race markers ×36, bats ×6, thieves ×4,
wolves ×2.

## Evidence

- Flattened instantiated trees (`godot/tests/scene_flatten_dump.gd`) differ from
  the baseline only by the generated root scripts, the new forwarded root
  exports, and removed ZDE/ZDR `source_collision_perso` metadata (Manoir 70
  roots/29, Brigand 135/98). Every child value is unchanged. An earlier run
  caught 45 dummies inheriting `source_stock` from a template; root metadata keys
  are now part of the signature.
- Placed transforms: 176 Manoir world nodes are identical between the installed
  editor-saved level, the baseline and the new output.
- `godot/tests/actor_instancing_check.gd` (running level): mannequins, barrels,
  teleporters, dummies, Brigand bats, wolves and thieves share one scene per kind,
  own no level-owned internals, keep independent hit points and Brain values,
  resolve main-actor/actor-Brain hooks, and keep animations and collision shapes.
- `tests/actor_editor_check.gd` (real editor): the Manoir dock shows 1149 rows,
  each mannequin root with its 85 internals hidden and no editable children; the
  inspector lists `brain_dsg_0`; an editor save of `brain_dsg_0 = 9` on Epee
  keeps it and the other mannequins' values/hooks with 0 editable flags; the
  shared scene opens with all 52 nodes.

## Limits

- Actors whose source pose, configuration visibility, AI model or camera data
  differ remain separate scenes (Manoir: 49 structures, 73 content variants).
  Scenes split when roots differ in metadata keys (e.g. `source_stock`).
- AI object templates keep their own `persos/` scene for spawning.
- Forwarded child fields are applied from the root at runtime; in the editor the
  non-tool Brain node inside the shared scene shows the scene's value.
- Local-to-scene hit points are written per instance in the level.
- `tests/native_scene_check.gd` still fails at "ready constructed scene
  assets" at baseline too; its stale Fix-global skip is fixed here.
