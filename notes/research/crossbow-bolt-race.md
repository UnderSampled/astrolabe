# Crossbow bolt 733 vs mannequin 939: hit-registration race (static read, 2026-11-03)

Static only. Nothing was run. "V" = read in code/decomp, "I" = inferred.

## 1. Contact and kill, as generated

Bolt = fix/733 (template manoir/persos/Object_template__8ccb49ea48.tscn, CharacterBody3D layer 2, mechanics 207919 mode 11).

- Normal0 (733.gd:38-206): first schedule tick sets `main_actor Brain dsg_58 = 1`, `dsg_17 = 1`, `dsg_120 += 1`, then request_action, waits 2 schedule ticks, `change_normal(1)`. No contact rule exists in Normal0.
- Normal1 rule 0 (733.gd:228-273):
  `contact_result_0 = character().source_collide_with_nothing()`; `partial_expr_1 = not contact_result_0`; if false, `partial_expr_1 = character().source_collide_perso_zde_no_with_type_zde(1, 1)`. On true: `dsg_120 += -1`; only `if dsg_120 == 0` set Hype `dsg_58 = 0` and `dsg_17 = 0`; then `character().source_kill()`.
- Normal1 rule 1: `dsg_0 > 150` ticks, same decrement/clear/kill.
- `source_collide_with_nothing` (character.gd:1437) is `get_slide_collision_count() == 0` from the previous `move_and_slide`. The bolt masks layer 1 (world) and 8 (ZDR). **Any** slide contact kills it, not only the mannequin.
- Bolt ZDE zone 1 (index 0): box 0.2x0.2x2.1 at local z -0.5, identifier **2**. Zone 2 (index 1) is id 4096 and disabled. Movement shapes are saved disabled; mannequin ZDR is one sphere r 0.5 at (0,2.4,0), layer 8, id 128.
- `source_kill_perso` (character.gd:853-871) for generated actors: `source_set_object_active_state(false)`, `source_return_suppressed = true`, `brain.stop()`, `collision_layer = 0`, `queue_free()`. Queries skip it at once: `_source_contact_candidate` requires `actor.source_object_active` (character.gd:1944), and own queries require `source_object_active` (1967).
- Mannequin 939.gd:445-: `reflex_0_rule_0` gates on Hype `dsg_17 == 1`, Gogoud 155 not in Normal5, then `source_collide_perso_zde_no_with_type_zde(3,2) or (4,2)` (clears dsg_17, Normal4), else `(1,2)` (clears dsg_17, Normal5/6). Mask 2 matches the bolt's ZDE id 2.
- The query (character.gd:1965-2005) is `direct_space_state.intersect_shape` of each enabled ZDE shape of the selected zone (mask 4|16, areas only) and keeps hits whose actor passes `_source_contact_candidate` (active, sector-active and same or neighbour sector, not the generator) and whose material matches.
- dsg_17 and dsg_58 are cleared only when `dsg_120` hits 0. Two bolts in flight keep them set until both die. dsg_17 is a single latch: whichever bolt overlaps first is consumed by the mannequin, and the second bolt cannot re-arm it, since it was set at that bolt's spawn.

## 2. Original engine order (decomp)

- Main: FUN_004049d0 runs the gameplay callback 0x44a1e0, then the graphics-end callback, then GLI_vEndOfTurnEngine, then input (TestResults/physics-clock-investigation-20260920.md).
- Actor traversal: FUN_00449f90 / 0044a0d0 / 00449d70 walk the SuperObject child lists and call FUN_004499b0 per actor. It is skipped when the actor's stamp (StandardGame+0x10) equals the global counter DAT_00754aa8, and requires `FUN_00438f40 == 1`.
- Per actor, FUN_004442e0 does AI then mechanics: related-actor AI, then FUN_00444090 (reflex FUN_005158e0; if it does not suppress, normal FUN_005158b0), then FUN_004440e0 (mechanics), then FUN_004434d0. **There is no global all-AI-then-all-physics barrier**; each actor does AI then its own mechanics, in hierarchy order. The AI reads the contact report left by that actor's previous mechanics step, so `collide_with_nothing` lags by one step, as in the port.
- Delta is `clamp(raw_ms,16,80)*0.001`, real-time and variable. The original is not a fixed-step simulation.
- Kill: FUN_004377e0/00437800 -> FUN_00437740 / FUN_00437800 write the active byte (+0x1c) = 0 (FUN_00438f60) **immediately**. Which one KillSelf reaches is not traced. Cond28 candidates need FUN_00438f40 == 1 (actor_zone_query.c:51-83), so a killed bolt is invisible to later queries in the same frame. Queries are exact geometry tests against current matrices, with no broadphase or lag.
- Whether a newly generated object is updated in the same frame depends on its list position relative to the current traversal, and that is I only (not traced).
- Consequence in the original (I): the bolt kills on its AI pass, reading state S before its own move. The mannequin examines S (or S plus one move) in its own pass. Either traversal order lets the mannequin see each overlap state at least once before the bolt's kill takes effect, except when the bolt's AI pass precedes the mannequin's pass in the same frame, which it cannot do for the S the bolt acts on, since the mannequin's frame-N pass in that case sees the post-move S. So the original has no order race at the contact boundary.

## 3. Port ordering

- behavior.gd:50 sets `process_physics_priority = -10` for every Brain. character.gd `_physics_process` is at priority 0. Per physics tick the order is **all Brains in tree order, then all bodies' move_and_slide** (V). Brain.advance does reflex first, then normal.
- Generated objects are `add_child`-ed to the topmost ancestor under root (character.gd:1312), so they sit **last** in tree order, after the placed mannequin. So the mannequin's reflex normally runs **before** the bolt's kill in the same tick (V from code; the tree layout at runtime is I). New nodes first process on the next tick (I).
- Both the mannequin and the bolt therefore see the same end-of-previous-tick positions: shape queries run before this tick's moves, and Godot flushes moved Areas in the physics step between ticks (I; Godot internals not re-read). The comment at character.gd:1436 ("AI conditions run after _physics_process has moved the body") is wrong for priority -10: the contact result is the previous tick's.
- `queue_free` is deferred, but `collision_layer = 0` and `source_object_active = false` apply at once and hide the bolt from any Brain running later in that tick. This only matters if a Brain order inversion occurs.
- Net: if the bolt's ZDE overlaps an arm/head/body sphere at a tick boundary, the mannequin sees it first and consumes dsg_17. Brain order and query timing are not frame-phase dependent, since the step is fixed.

## 4. Where the nondeterminism most likely comes from (I; needs a trace to confirm)

1. **The bolt dies on a non-ZDE slide contact** (`collide_with_nothing` false) before its ZDE ever overlaps a mannequin zone. This matches "dsg_17 true, then false about 0.1 s later": Normal0 lasts about 3 ticks, then Normal1 rule 0 fires on any slide. Candidates are world geometry (layer 1), the mannequin's ZDR sphere above the head, or another actor's ZDR. This depends on spawn position and **pitch**. Yaw (within 0.02 rad) does not capture pitch: `dsg_2.z` comes from `source_laser_pointer_direction()` and is applied via `source_change_sighting` only when cos > 0.75. Pitch, spawn transform at the release cue and gravity ratio vary run to run.
2. Bolt sector eligibility: the mannequin's query rejects the bolt unless the bolt's `_contact_sector` is the mannequin's or a neighbour's (character.gd:1944-1960). The sector moves on boundary crossings, so a bolt grazing a boundary can be skipped (the original has the same sector filter).
3. Two-bolt latch and `dsg_120`: the second bolt cannot re-arm dsg_17 after the first is consumed.
4. Not likely: tunnelling. The ZDE box is 2.1 m long and the arm sphere 1.2 m across, so the bolt would need more than about 3 m per tick. Bolt speed was not measured.

## 5. Recommendation

- First add a per-tick trace: bolt position, `get_slide_collision_count()` and collider names, mannequin ZDE query result, and Hype dsg_17 / dsg_120. This splits cause 1 from the others.
- If cause 1: the question is whether the original type-11 mechanics report contact with world geometry and ZDR at all. `_collide_with_actor_zones` and `collision_mask |= 8` are the port's adaptation; check the 00464570/00463df0 query and which materials the type-11 contact test accepts.
- Make the order explicit rather than tree-dependent: give generated actors' Brains a priority of -9, after placed Brains. This keeps the mannequin's look before the bolt's kill, which matches the original's guarantee. It also keeps AI before motion.
- Optional stricter match: run each actor's Brain then its own move per actor, as FUN_004499b0 does. This is costly and not needed for this race.

Verified: 733/939 rules, kill semantics, Brain priority and add_child site, original per-actor AI-then-mechanics order, immediate active-flag clear. Not verified: runtime tree order, bolt speed, the actual cause, which kill procedure KillSelf maps to, same-frame update of generated objects in the original.
