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
