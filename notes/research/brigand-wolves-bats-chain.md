# Brigand wolves and bats: static chain trace

Static research, 2026-11-04, worktree brigand-regressions-20261104 at 02e0f42. Generated GDScript is the
installed build under /home/stephen/Projects/astrolabe/godot/generated/hype/levels/brigand (G below).
Nothing was run. Game runs non-strict (godot/game/bootstrap.gd:33), so `fail_unsupported` only records a
"guard" stand-in and the generated `if strict_unsupported: return` does not return
(godot/addons/astrolabe/behavior/behavior.gd:165-181); `stand_in()` returns false and the call is skipped.

Coordinates: source (x,y,z) maps to Godot (x, z, -y) (character.gd:471-473, 1783). Source z is Godot up.

## Earlier bat attempt (git log)

No commit changes bat aggro or motion. Related: af8a532 ported Cond24-27 (`source_collide_perso_type_zde`)
because bat/boar/guard/thief damage gates read 0; 30c7e00 enemy bars; b6db6ce checklist. The checklist
(docs/astrolabe/brigand-checklist.md:53-54) records "bat 1 stays about 5.7 m away and above reach; its dive
is not reproduced. Open." and "Not compared". Nothing was done for 774/850 motion.

## Wolves (model 1203, family 41; world/0/119 Wolf2, world/0/120 Wolf1)

Level data (G/level.tscn:50743-50747, 52172-52190): initial `current_normal = 4`, `dsg_33 = self`,
`dsg_34 = partner wolf`, `dsg_35 = I_Brigand1_ArbreLoups (world/0/65)`, `dsg_27/28` = waypoints,
mechanics shared/13916 (mode 0 Character). Wolves are at about (-73,-5,45) and (-83,-5,52); Hype starts at (-84,-2,-17).

1. Brain runs while Hype is far: IMPLEMENTED. No AI-range/sector gate exists; `advance()` only checks
   `source_object_active` (behavior.gd:262-270), and nothing deactivates the wolves. Whether the original
   deactivates actors outside an AI range/sector is UNVERIFIED (no runtime link to check).
2. Normal 4 init (1203.gd:382-457): 5000 ms wait, dsg_6/7 setup, then `change_normal(7)`. IMPLEMENTED.
   `generic-4-53` Proc_SetHitPointsToInit at 1203.gd:398 is STAND-IN (skipped; hit points are not reset).
3. Normal 7 detect (1203.gd:926-985): turn toward Hype; Wait(Hype dsg_56 == 0) (bool, fine); zde 2/1 touch
   -> action 8 then normal 1; else `distance_to_main_actor() < 16` (3D, behavior.gd:279) OR partner in normal 1
   -> `request_action(1)`, `change_normal(1)`. IMPLEMENTED.
4. Normal 1 pursuit (1203.gd:204-255): `source_begin_actor_pursuit(main_actor, 7.5)` -> character.gd:594, point
   motion 770-825; speed comes from root-motion of action 1. IMPLEMENTED (checklist: wolf 1 reaches Hype and dies
   in route tests). Rule 0: `distance_xy(self, Hype) <= 8`: partner not in normal 0 -> `change_normal(0)`; partner
   in normal 0 -> `request_action(1)`, `change_normal(6)` (attack). The two `generic-5-1` calls (1203.gd:220, 223;
   cat 5 op 1) are STAND-IN (skipped; effect unknown, not in stand-ins.md priority table).
5. Normal 0 "wait for opening" (1203.gd:79-185): rule 0 (xy to Hype < 4 -> normal 5); **rule 1
   (1203.gd:116-123) is STAND-IN `generic-untyped`**: `partial_expr_0 = 0`, then `0 < 3.0` is true, so
   `change_normal(12)` fires every pass; rules 2 and 3 (waypoint dsg_27/28 within 8/7 or partner in normal 7
   -> normal 3, the approach/attack setup) never evaluate, and the schedule's TurnPerso never gets a turn.
6. Normal 12 (1203.gd:1412-1495) is a sidestep (action 20, point turn about dsg_35, action 1) ending in
   `change_normal(0)`. With step 5 this is a permanent 0 -> 12 -> 0 loop: the wolf strafes, never approaches
   or attacks until Hype is within 4 (xy), which the pursuit stop radius of 7.5/8 prevents.
7. Normals 5/6/10 (bite, circle, attack): IMPLEMENTED for the harness case (checklist 15/10/5/0 hits).

Why rule 1 is untyped: the source operand is category 3 id 8 (Func_DistanceXZToPerso, handler 0x537aa0). The
parser accepts only a literal `Expression::Actor` for (3, 8) (src/openspace/ai.rs:1990-1999), unlike (3, 4)
which accepts `Expression::Variable` (ai.rs:1968-1976). A designer-variable operand therefore falls to
`Expression::Unsupported`; the same operand shape is already supported for the MainActor/Actor forms
(`ActorLocalSourceXYDistance`, ai.rs:1998, 3500; src/godot/ai.rs:1475; emitted `NativeContactPickup.distance_xy`).
The variable is dsg_34 (partner, as listed in notes/research/self-behavior-change.md:20) or dsg_35 (tree).
UNVERIFIED which; dsg_35 is only otherwise read in normal 12, so "within 3 of the tree -> sidestep" is the likely
source meaning. Either way the port's constant zero is wrong. Counts: docs/astrolabe/full-game-ai-sites.md:61
lists this as the one "category-3 ID 8 requires an actor operand" diagnostic.

Most probable first divergence: 1203.gd:116-123 (normal_0_rule_1), STAND-IN `generic-untyped`; emitter
src/openspace/ai.rs:1990-1999. Before that, pre-conditions (steps 3-4) are faithful: if Hype never gets within 16
of a wolf, the source also does not move it (Hype starts 60+ units away).

Red-capable assertion (isolated Godot): instantiate the Brigand wolf character with Brain 1203 in normal 0
(`current_normal = 0`, non-strict), Hype 6 units away in xy, dsg_35 tree at least 10 units away. Tick 120 physics
frames. Expect (source): the Brain stays out of normal 12 (observe `current_normal`, `previous_normal` history and
`Brain.stand_ins_reached` containing no key "generic-untyped ... source AI node 2") and reaches normal 3 (rule 2/3)
when dsg_27 is within 8 or the partner is in normal 7. Today: `current_normal` flips to 12 on the first tick and
alternates 12/0. A second assertion: with the tree at distance 2, normal 12 is entered.

Fix location: emitter (src/openspace/ai.rs (3,8) arm plus a `Variable` target, emitting
`NativeContactPickup.distance_xy(self, <dsg node>)`, with a null/invalid actor guard). New parser assumption needing an
original live-memory check (Orrery): handler 0x537aa0 reads its P operand like 0x537c20 (designer variable in
the owner's table, type 10 perso) and the result is the XY magnitude in the target's local frame; and which variable
index node 2 of model 1203 holds.

## Bats (model 774, family 37; world/0/45..50; controller I_Dummy_Brigand_Bats world/0/51, model 850)

Level data: bats Brain `current_normal = 2` (G/persos/I_Brigand_Bat_1__8cc8b9d013.tscn:394), mechanics
brigand/mechanics/141730.tres is **mode 6** (not Character0/Mass9), `motion_enabled` is not set (only the main
actor gets it, src/godot/scene.rs:766-772 and level.tscn:9429+). Dummy 51 is at (15.2,-12.2,24.9); bat 1 at
(19.8,-7.5,30.3). Model 850 (the dummy) has an empty `normal_0` (850.gd:17-22): only a position anchor.

1. Brain runs when Hype is far: IMPLEMENTED (as for wolves; step 1 above, UNVERIFIED for original range gating).
2. Normal 2 (774.gd:198-322) idle until `dist(dummy 51, Hype) < 25`, then a per-bat delay (bat 1 8450 ms, bat 2
   550, bat 3 2750, bat 4 3420, bat 5 1750, bat 6 10000), sets dsg_7 waypoint, action 6, action 4,
   `change_normal(0)`. IMPLEMENTED (keyed on `character() == source_actor_node("level:/persos/world/0/N")`).
3. Normal 0 hover/approach (774.gd:64-100): sets vector speed (0, -5, Hype.z - bat.z - 0.5) then
   `character().source_go_relative()` (74), `TurnPerso` Hype, if `dist(bat, dummy) > 25` -> normal 1 (patrol), else
   if `distance_to_main_actor() < 5` -> normal 4 (dive). The only way into the dive is for the bat to get within 5
   of Hype by its own flight. IMPLEMENTED as code; **motion MISSING**: see 5.
4. Normal 4 dive (774.gd:356-499): vector speed (0, -9, dz + 1), go_relative, turn, then
   `source_collide_perso_type_zde(self,0,Hype,0)` -> Hype dsg_32/dsg_119/dsg_4, armor damage 3 (IMPLEMENTED since
   af8a532), else back to normal 1 if farther than 25 from the dummy. Reachable only after step 3 succeeds.
5. Bat flight: `source_set_vector_speed` (character.gd:475) stores the vector; `source_go_relative`
   (character.gd:931-932) only sets `source_relative_motion = true`. The vector is consumed in `_physics_step`
   (character.gd:279-281), but `_physics_step` returns at line 256 when `motion_enabled` is false. Only
   `source_begin_*` point motion, `source_jump*` (977-1030) and the main actor enable it; bats use none of them.
   MISSING: the bat never leaves its spawn, never nears Hype to 5, never aggros; patrol (normal 1, 133) is equally
   inert. This matches the checklist: "bat 1 stays about 5.7 m away". Mode 6 behavior (what dynamics callback
   drives it, and whether gravity applies) has no handler anywhere (`mode) in [0,9]` at character.gd:816) UNVERIFIED.
6. Reflex 0 (774.gd:555-630) hit-by-sword and life subtraction: IMPLEMENTED, gated on Hype dsg_17 = 1 (checklist
   notes dsg_17 is not set on the guard case; not traced here).

Most probable first divergence: step 5, runtime. `source_go_relative`/`source_set_vector_speed` do nothing for an
actor with `motion_enabled == false` (character.gd:256, 931), and the bat scene never enables it. The generated bat
code at 774.gd:73-74 is correct as typed.

Red-capable assertion: instantiate bat 1 with Brain 774 at `current_normal = 0`, Hype 20 units away and the dummy
within 25 of the bat. Tick 2 s of physics. Expect (source): bat `global_position` moves along its local forward
at 5 source units/s (Godot local vector (0, dz-0.5, 5)), `distance_to_main_actor()` shrinks, and once below 5
`current_normal == 4`. Today: `motion_enabled` stays false, position and `current_normal` (0) never change. A
narrower assertion: after one `source_go_relative()` call, `source_relative_motion` is consumed and
`external_velocity` equals `_source_relative_vector(source_vector_speed)`.

Fix location: runtime (godot/addons/astrolabe/character/character.gd): relative vector speed (Proc go-relative after
Proc29) must enable the actor's motion for a mechanics card of mode 6, as the source dynamics do for any
persistent speed vector. Assumptions needing an original live-memory/decomp check before coding: (a) what
dynamics type 6 is (decomp of the mechanics-mode dispatch near 00443f00 and the type table), whether it ignores
gravity, collides with geometry, and whether the speed vector persists across frames; (b) whether Go-relative
re-applies every frame or once (the port consumes it once per call; the bat re-issues it every schedule tick, so
both readings give 5 u/s, but gravity/ratio differ); (c) the axis convention (y = -5 as forward). No arbitrary
radii or speeds are needed: 5, 9 and 25 come from the model.

## Summary of link status

| Link | Wolves | Bats |
|---|---|---|
| Brain active when far | IMPLEMENTED (range gate UNVERIFIED) | same |
| Start behavior | normal 4 -> 7, STAND-IN generic-4-53 | normal 2, IMPLEMENTED |
| Detect Hype | 3D < 16, IMPLEMENTED | dummy-to-Hype < 25, IMPLEMENTED |
| Move toward Hype | pursuit IMPLEMENTED; then normal 0/12 loop from STAND-IN generic-untyped | MISSING (vector-speed motion inert) |
| Attack range | xy < 4 normal 5, unreachable in the loop | < 5 normal 4, unreachable |

## Outcome, wolves (2026-11-04, owner run)

Repro (godot/tests/brigand_actor_probe.gd `wolves`, `wolves_bite`; New Game, the real
LevelTransition into Brigand, a debug start 12 m from wolf 1 that also opens the Gogoud map
pickup, paged with Button_Hit as in the take at about 341 s):

- Installed 02e0f42: both wolves 4, 7, 1 (pursue to 8 m), then the 0/12 loop. They strafe out
  to about 16 m and never attack. That is Stephen's "wolves did not move to attack".
- First divergence confirmed: rule 1 of normal 0 is `Func_DistanceXZToPerso(dsg_35) < 3`. dsg_35 is
  the wolves' tree I_Brigand1_ArbreLoups. Handler 0x537aa0 has the same operand path as
  00537c20 (0x522610 with the owner as caller, then 0x438e00, then 0x52ea00, selector 3 instead of 7;
  objdump of MaiDFXvr_bleu.exe). So a designer-variable operand is the owner's actor, as already
  accepted for (3,4). The decoder (3,8) arm now accepts `Variable`. No parser byte assumption is new: the
  node is already decoded as Variable, and only the accepted operand shape changed.
- Second link: the circle (normal 6) steers by incremental-turn steps while the run action
  translates. The pursuit's finished request kept the native steering hold (character.gd point hold),
  so velocity stayed 0 with non-zero root motion. Each positive step of 0054d030 queues dynamics
  event5 (travel-native audit), superseding that request. The new `source_incremental_turn_step`
  ends the hold (tests/check_travel_native_semantics.gd, red without it).
- Result: 4, 7, 1, 0/10/6 circling in to about 6 m, 3/15 retreat to the tree waypoints, re-detect.
  `wolves_bite`: Hype 3.5 m from the waiting wolf, wolf 119 enters 5 at 2.66 m, Hype 50 to 49,
  hit reaction 15.
- Remaining: generic-4-53 (SetHitPointsToInit) and generic-5-1 (Action_IncrementalTurn in a rule)
  are still skipped. The wolves' Walk event5 body itself is not recovered. Not compared frame by
  frame with the take's 366 s fight.

## Live check of the (3,8) designer operand (owner, 2026-11-04)

Private disposable original, manor save, `navigate_level brigand`. Raw reads used Astrolabe's verified
designer layout (source_view.rs designer_view, records.rs DesignerWire and InfoWire):
- Wolf 1 chain: perso+0x10 brain, then mind, mind+0 model, model+8 definition
  (40 fields, length 248), infos at definition+4, 16-byte rows {offset, kind, save, init}.
- mind+0x0C memory: +0 points back to the definition (checked), +8 holds the current values.

| field | offset | kind | live value |
| --- | --- | --- | --- |
| dsg_33 | 192 | 10 | wolf 1's own SuperObject |
| dsg_34 | 196 | 10 | the SuperObject whose perso has object types (41, 67, 149): wolf 2, the partner |
| dsg_35 | 200 | 10 | the SuperObject whose perso has object types (4, 52, 135): I_Brigand1_ArbreLoups |

- The port's exported dsg_35 (the tree, world/0/65) matches.
- Orrery's designer decoder fails on these fields because it reads the stored SuperObject as a perso.
- Handler 0x537aa0 pushes [[operand]+4] to 0x438e00, which returns [x+0xC] (objdump). So the
  evaluated operand is a perso (StandardGame +0x0C is the SuperObject; that layout was checked live).
- So Func_DistanceXZToPerso(dsg_35) measures to the tree, as the corrected decoder emits.
- Inferred, not read: the SuperObject-to-perso step for a kind-10 slot inside 0x522610.
