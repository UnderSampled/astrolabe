# Mob drop vs dying mob collision (static, 2026-11-04)

Hypothesis (Stephen): a drop is not supposed to collide with the fading dying mob.
Verdict: the port already has NO Godot body collision between a drop and the corpse
(by layers). The source has no drop-vs-corpse exclusion either. Hypothesis is not
supported as a cause of anything in the port; a different cause must be sought.

## 1. Death chain, boar (ai/models/309.gd, world/0/4 and /6)
- Reflex rule 2 (309.gd:1595, 1763) -> change_normal(9).
- normal_9_schedule (1261-1316): roll dsg_2 (1..100) against dsg_1[0]=15 / dsg_1[1]=50 (normal_2:528)
  -> source_execute_action_and_generate_object(template0 | template1 | none, action 22) (1270, 1286);
  waits for action 22 to finish; dsg_3 = 255; request_action(41) (death anim); change_normal(10).
- normal_10_schedule (1331-1356): every 50 ms dsg_3 -= 8, source_set_transparency + transparent_display(true);
  when dsg_3 <= 5 -> source_kill() (1353). About 32 ticks = 1.6 s of fade, corpse stays an active actor.
- So the drop is generated at the START of death (action 22, before action 41 and the fade), and the
  corpse is killed ~1.6 s plus after. No source_set_object_active_state or collision call in boar death.
- Templates: Object_template__cd69592e26 (family 22, perso 5-0-27000) and __16b022289c (family 24,
  perso 5-0-27269): CharacterBody3D, collision_layer = 0, mask default 1, mechanics 19207.tres
  mode 4 (not 11), source_object_types 22/25 and 24/30. Owner/generator = the boar (character.gd:1437).
- Wolf 1203: same shape (drops 1020-1170 inside death normal, fade+kill 1364-1375).
  Thief 1060: drop in normal 11 (4261-4316), fade normal 13 (4382), set_object_active_state(false) when
  opacity < 1 (4392). Bat 774: source_kill only (515), no generate.

## 2. Port runtime
- character.gd:1345 source_execute_action_and_generate_object. Spawn transform = generator global
  transform (1405-ish `generated.transform = ...`); held invisible while the clip has a release cue.
- 1391-1396 card-less hold (n/a, 19207 has a card). 1397-1410 stand-in "item point contact": 0.1 sphere
  child on the drop for families without a movement zone (docs/astrolabe/stand-ins.md:72).
- 1418-1430: ONLY mechanics mode 11 gets mask |= 8 and add_collision_exception_with(self + owner bodies).
  Mode 4 drops get no exception and no extra mask bits.
- Layers: actors are layer 2 / mask 1, set in src/godot/character.rs:289 (boar scene
  actors/I_Wild_Boar__b69bd5dbb1.tscn:79); ZDE Area layer 4, ZDR StaticBody layer 8 (mask 0),
  PhysicalContact Area layer 16. Drop: layer 0, mask 1.
- Godot only collides A with B when A.mask & B.layer. Drop mask 1 vs corpse body layer 2, corpse ZDR 8:
  no contact. Corpse mask 1 vs drop layer 0: no contact. The drop's only possible contact is level
  geometry (layer 1). kill sets collision_layer = 0 (998) only for the generated killer, not for a
  placed boar; the placed boar keeps layer 2 during the whole fade (irrelevant to drops).
- contact_pickup.gd is pure spatial math (no physics layers). Cond 28 queries use mask 4|16 areas.

## 3. Source (MaiDFXvr_bleu.exe, reference/ghidra/functions)
- 00463df0 (mechanics contact query) -> 00464570 (candidate gather). Gate (00464570): mover sector
  activation &2, mover geometry sets non-empty, and mover dynamics byte +0x31 == 1 (FUN_0045f930).
  Then for every character in the mover's sector and active neighbouring sectors:
  - skip the mover itself;
  - if the mover has a brain (FUN_0043dbb0: type-4 SuperObject with perso+0x14 != 0), skip the character
    at (perso+0x14)+0x10 (FUN_00440fc0). Port comments call this the generator; unresolved whether it is
    the generator or a carried object. Not shown to be the mob that spawned a drop.
  - else 00464380(mover, set1 geom, set3 geom, other): other must be active (FUN_00438f40:
    StdGame+0x1b == 0 and +0x1c == 1) and have no dynamics or dynamics+0x31 != 0; then mover set3 geom vs
    other set3 geom (00464510) and, when other priority byte (+0x30, FUN_0045f910) <= mover's, mover set1
    geom vs the other's kind-0xb child collide sets (004644b0).
  - material flag 0x8000 on the hit material (00463df0 tail) turns a hit into "no collision".
- Bytes +0x30 (collision priority) and +0x31 (collide enabled) are read per object from the level
  dynamics chunk (loader 0045ece0-0045f3d0; setters 0045f920 / 0045f940). Generation sets +0x31 = 0 while
  the object is held and 1 on release (0043bcd0 cases 3/4: lines 213, 323, 398; 00403b90:59).
  The AI table setters at 005363c0 (set +0x31) / 005363f0 (get +0x31) exist but boar/wolf/thief death AI
  never call them (the only unsupported category 4 ops in 309 are 53 and 296 in normal 2/spawn, not death).
- Corpse stops being collidable only when StdGame +0x1c goes 0: kill (FUN_004377e0/00437800 writes
  the active byte immediately, crossbow-bolt-race.md:26) i.e. Proc_KillPerso 0053fc30 or SetObjectActive
  false. Fade (transparency) does not touch collision. So the corpse is collidable through the whole fade.
- Not found: any owner/father/ignore flag in generation (0043bcd0, 005711f7 edge 00403b90), any death-phase
  collision-off call, or a rule that dynamic objects skip persos' sets. Whether a drop actually hits a
  corpse depends on the per-object +0x30 priority and +0x31 bytes (not parsed by the converter) and on
  whether the corpse has a set-3 geometry; unverified.

## Classification
(a) loot excluded only vs own owner: not supported (only the brain+0x10 exclusion, identity unresolved).
(b) dynamic objects never collide with persos: not supported; 00464570 explicitly iterates sector persos,
    gated by priority/enabled bytes (data).
(c) corpse collision off at a phase: only at kill (active byte), not at fade/death anim.
Closest reading: source = generic pair test, data-gated; port = nothing collides (by layers).

## First port divergence
character.gd:1418-1430 + src/godot/character.rs:289: actor bodies sit on layer 2 and drops mask only 1, so
drop-vs-persos contact is absent entirely (source tests it unless +0x30/+0x31 gate it). That is the
opposite of the hypothesis. No file:line produces drop-vs-corpse collision in the port.

## Red-capable test idea
Boar world/0/4 killed with sword (normal 9). Poll each physics frame until the drop appears
(child of destination with meta astrolabe_generated_template, family 22/24): log drop.get_slide_collision_count,
colliders, drop.collision_mask, drop global_position vs boar position, boar visible/layer, until the boar is freed.
Hypothesis predicts a boar collider in the list (red); static analysis predicts none (test stays green,
refuting the hypothesis). If the symptom is the drop jumping or stuck, log the ItemPointContact collisions
(only layer 1 expected) and the drop's velocity.

## Smallest per-instance fix if ever needed (unsourced, so NOT recommended)
character.gd after 1430, generated branch only: generated.add_collision_exception_with(self), the same call
as the mode-11 branch. Per instance (no shared resource mutation). Only valid with source evidence of
+0x30/+0x31 or the 00464570 skip for those specific objects.

## Maliq reproduction status (owner, 2026-11-04, checkpoint)

Stephen's case is Maliq: "the drop sat atop him until he disappeared, only then did it fall".
Probe `maliq_kill` (brigand_actor_probe.gd):
- The real scene; Maliq becomes thief 88 (model 1060).
- Hype draws the sword (Button_GetSword), walks in on Button_Up and swings (Button_Hit).
- Generated bodies are logged every tick.

Results:
- Thief 88: normals 7, 14, 5. It then stands at Maliq's spot in action 90, reflex 1, motion off.
- Hype closes to 1.5 m and swings (normals 4 and 29 alternate), but the thief takes no damage. It
  never reaches its death (normal 11, which drops) or its fade (normal 13).
- The only generated body seen is the thief's thrown dagger (layer 2, mask 9). It is not loot.

Next step:
- Find why Hype's swing does not damage thief 88 here: the sword zone contact or the attack flag
  dsg_17, compare the checklist's earlier thief kill. Then rerun to reach normal 11 and log the drop
  against the corpse through the fade.
- Static facts so far: the drops are on layer 0 with mask 1 and no owner exception. The source corpse
  stays collidable until it is deactivated (normal 13, opacity < 1), which matches Stephen's timing.

## Maliq loot reproduced (owner, 2026-11-04)

- Prerequisite bug fixed: `source_set_absolute_position` (and placed test starts) did not re-research
  the sector (00447bf0). Generated swing objects (fix 708) then inherited Hype's stale sector, the
  engine-turn gate left them untreated, and Hype's attack flag dsg_17 never turned on.
  `source_research_sector` now runs at absolute placement.
- With that fix, `maliq_kill`:
  - Thief 88 goes 7, 14, 5, then is hit (normal 12, three times), dies (11) and fades (13).
  - At death it spawns loot fix 1138 (card type 4) at its own position, layer 0, mask 9, no
    exceptions.
- Port cause of "sat atop him": mask bit 8 (actors' ZDR reaction zones). _collide_with_actor_zones()
  (d07a878, Nov 2: "actors stop against each other's reaction zones") applies to every
  motion-enabled body, generated loot included. So the loot rests on the dying thief's ZDR until the
  kill removes it.
- Source gate still to decode, then fix:
  - The original pair test is 00463df0, then 00464570, then 00464380. The other actor must be
    active, and must have no dynamics or have dynamics+0x31 != 0; priority bytes +0x30 are compared
    at 0045f910.
  - Generation holds the new object's +0x31 at 0 and sets it to 1 on release (0043bcd0).
  - Decode those bytes for the loot template and the thief, and port that rule. No blanket exception.

## Correction and decoded CollSet bytes (owner, 2026-11-04)

Correction: the earlier static claim "a drop's mask is 1, so it can only touch level geometry" was
wrong. The live drop has mask 9: _collide_with_actor_zones() adds layer 8, the actors' ZDR. The port
loot does collide with the dying thief's zones, as Stephen saw.

The test's D is the CollSet at perso+0x18, CharacterCollisionsWire, 52 bytes (constructor 0045f4f0).
Bytes +0x30 to +0x33 are preserved as `CharacterCollisions.unknown` and not yet exported. Stored
values from the Brigand level import:
- Hype (world/0/0): [127, 1, 0, 0], so priority 127 and +0x31 = 1.
- Thief 88, Maliq 20, boar 4, wolf 119 and 70 other level persos: [0, 1, 0, 0].
- Loot template fix:/persos/5-0-27269 (thief drop, model 1138), 12 other fix templates and level
  persos 55, 179, 180, 182, 189: [0, 0, 0, 0].

00464380, with esi = the other character and edi = the mover (objdump):
1. Skip unless the other is active (00438f40 == 1).
2. If the other has D, skip unless D+0x31 != 0 (0045f930).
3. Run the bounding test 0055dde0.
4. If the other has D and the gate 00463dd0(arg) holds, run the zone test 00464510. This one has no
   priority check.
5. Skip the rest if the mover's D+0x30 is below the other's (`cmp mover, other; jb`).
6. If 00463dd0(arg2) holds, test each of the other's zones with 004644b0.

For loot as mover against the thief: the thief is active with flag 1 and priority 0, and the loot
priority is 0. Neither priority nor flag excludes the pair. So by these bytes, the original also
tests loot against the corpse.

What is undecided is the mover side:
- which test kinds the loot's type-4 mechanics query requests (00463df0/00464570 arguments, the
  00463dd0 gates);
- whether the loot's own +0x31 (0 at generation, 1 on release in 0043bcd0) gates it;
- which zone type the thief's ZDR or ZDM presents.

The take's 549 to 568 s frames (boar fights) are too small to show loot.

Next:
- Read 00463df0, 00464570 and 00463dd0, plus the type-4 call site.
- Or kill thief 88 in the private original and read the loot's height and contacts during the fade.
- Then export CollSet +0x30/+0x31 per perso, which needs a model and schema field coordinated with
  the static owner's schema 73 work, and gate the port's actor-zone collision by that rule.

## Live original attempt (owner, 2026-11-04, second pass)

- Private disposable original, menu load, then `navigate_level brigand`.
- The memory hop toward Maliq's stop point (source 40.1, -71.0, -14.8) used the earlier hop13
  method, in 5 m steps with a 0.6 s settle.
- Hype sank under the world: z -261 to -312 by the last steps. The floor of unloaded sectors does
  not hold him, and the level restarted (phase 38/4).
- Loading the take's forest save did not select the slot reliably through the menu (it loaded the
  manor slot).
- A walking approach would need key steering over about 50 to 150 m, which is outside the
  no-route-bot rule. So the original's loot behavior was not observed live.
- Mover-side contact query: static trace in loot-mover-contact-query.md.

## Fix (owner, 2026-11-04)

Source, from loot-mover-contact-query.md:
- 00464570 tests characters only if the mover's CollSet +0x31 == 1.
- 00464380 gates both character stages on 00463dd0 (list non-empty) of the mover's ZDM list. They
  test the mover's ZDM against the other's ZDR.
- An item without ZDM geometry therefore meets scenery only. The thief loot template has no ZDM;
  that absence is why the port adds the "item point contact" stand-in sphere.

Port:
- _collide_with_actor_zones() now adds layer 8 (actors' ZDR) only when the body has ZDM geometry
  (exported Movement_* shapes, active or not).
- The loot keeps mask 1, collides with the world and is picked up by contact as before.

Probe `maliq_kill`, ASTROLABE_SEED=11 (Hype steps 4 m back when the thief enters normal 11, so
contact pickup does not take the loot at once):
- Old: loot mask 9. It rests on the thief's ZDR at y -13.07 (tick 850, thief in normal 11, about
  2.3 m up), and only later reaches the floor at -15.4.
- New: loot mask 1, no ZDR contact, floor at -15.4 (tick 866) while the thief is still fading
  (normal 13 active until tick 969).
- Seed 7: no drop in either run (random roll).

Counterexamples:
- Hype keeps mask 9.
- Wolves, bats and Maliq's scene still pass.
- The Manoir type-19 census is unchanged (0.00 m).

Remaining:
- The original's visible loot motion was not observed live (placement limits above).
- The loot's +0x31 release flag (0, then 1 on release) is not modelled. It only matters for movers
  that have a ZDM.
- Contact response 00599703 was not read.
