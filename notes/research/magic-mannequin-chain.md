# Magic mannequin chain: Fire cast to Gogoud's key (static, 2026-11-03)

Static read-only research. Nothing was built or run, and no Godot or original
engine was used. Sources: generated GDScript under
`godot/generated/hype/levels/manoir/`, exported scenes there, the runtime in
`godot/addons/astrolabe/character/character.gd`, the hype-decomp C
(`src/ai/collision_zone_condition.c`, `src/collision/actor_zone_query.c`,
`src/collision/actor_zone_lookup.c`) and `research/hype-ai-api.tsv`. The Rete
dump in `output/research-manoir-rete` is stale: it still marks Cond 28 as
Unsupported. Line numbers are for the generated files as of `b72a54f`.

## Identities (corrects earlier notes)

The stage-to-weapon mapping in `notes/manor-training-static-2026-10-08.md`
("1 sword, 2 magick, 3 crossbow") is wrong for 2 and 3. The designer values and
Hype flags give:

| Gogoud dsg_0 / mannequin dsg_0 | Actor | Hype flag the mannequin needs | Writer of that flag |
|---|---|---|---|
| 1 | `/world/0/153` i_Mannequin_Epee | dsg_55 | sword swing (Hype 28.gd:11054; object fix/708) |
| 2 | `/world/0/152` i_Mannequin_Arbalete | dsg_58 | crossbow bolt fix/733 (laser-aimed, consumes item) |
| 3 | `/world/0/154` i_Mannequin_Magie | dsg_0 | Fire object fix/817 (also 28.gd:6256, a 50-point spell) |

The `persos/i_Mannequin_*.tscn` files set `dsg_0 = 1/2/3` at line 393. Magic is
therefore the **last** exercise, and its completion leads to Normal18,
finimannequin and the key.

## 1. The chain

1. **Cast** (Hype, fix/28.gd). Pressing Enter (Button_Magic) in Normal4 goes to
   Normal12, gated by dsg_10==0 and dsg_59==1 (needs the sword drawn, X, per
   the checklist). Normal12 (28.gd:~5845):
   - Reads the current spell (Fire raw 61 maps to 1).
   - Sets dsg_32, dsg_39 and dsg_19.
   - Needs 10 magic points or Global dsg_13.
   - Runs action 290.
   - `source_execute_action_and_generate_object(object_template_0_path, 271, Vector3.ZERO)`
     (28.gd:5909) spawns template `persos/Object_template__f49046045d.tscn`
     (`fix:/persos/5-0-25549`, model fix/817).
2. **Spawn and "flight"**. The Fire object copies Hype's global transform
   (character.gd:1279) and inherits Hype's contact sector, with
   `source_generator` = Hype (1303-1306). It has no mechanics card, so it does
   **not move** (character.gd:1256-1259, an explicit adaptation: with gravity
   it sank). Its only zone is `ZDE/ZDE_0_0`:
   - BoxShape3D 1.3 x 1.3 x 5.876, centred at local z +2.761.
   - So it covers about 0.12 to 5.64 m ahead of Hype's origin, ±0.65 m laterally
     and ±0.65 m vertically about Hype's origin.
   - Material identifier 2, `source_zone` 0, active only in **state 0**, on
     Area layer 4 (template :1109-1123).
   - If the action-271 clip carries `releases_generated_object`, the object
     stays `source_object_active=false` until the release cue
     (character.gd:1250-1255).
3. **Fire's flag window** (fix/817.gd normal_0_schedule):
   1. First tick: Hype `dsg_0 = 1`, `dsg_17 = 1` (:~130-134). It also removes 10
      magic points unless Global dsg_13.
   2. Action 0 runs to completion.
   3. Then, if `/world/0/76.dsg_2 == 0`, Hype `dsg_0 = dsg_17 = 0` (:202).
   4. Action 1, wait 1000 ms, `source_kill()`.
   5. Rule 0 also clears both flags and kills if Hype enters Normal94.

   Fire never resets on contact, unlike bolt 733.
   - **Hype dsg_17** means "an attack/projectile is live and not yet consumed".
     It is 1 on set and 0 on reset, and stores no spell id. Writers are 708,
     733, 757, 768, 782, 797, 817, 828, 842, 864, 882, 893, 908, 922, barrel
     488 and Hype 28. Resetters include 939 itself.
   - **Hype dsg_0** is the stage-3 discriminator.
4. **Zone semantics** (engine). Cond 28 `Cond_CollidePersoZDENoWithTypeZDE`
   (handler 005338a0; Cond 18-29 share it). Operands are self, a zone number N
   (u8) and a material mask (low 16 bits).
   - `actor_zone_query_0042f210` runs with capacity 1, on demand when the rule
     is evaluated. There is no persistent contact table.
   - Zone N is index N-1, matched by WORD id in the actor's category-2 (ZDE)
     list (`actor_zone_lookup_0042d830`).
   - Candidates are other characters in the querier's sector and
     collision-neighbour sectors with activation flag &2, and
     `FUN_00438f40 == 1` (meaning unresolved; probably active/alive).
   - The query zone is tested against the candidate's ZDEs, then its module
     collide sets of kind 0x0b/0x14.
   - The **target** material must carry **every** mask bit. The querying zone's
     material is not filtered. Spawned actors participate normally.
   - So "ZDE vs type ZDE" here means mannequin ZDE zone N against any other
     actor's ZDE with material bit 2. Fire's ZDE has identifier 2. ZDR/ZDM are
     not involved in the mannequin test. Its ZDR (layer 8, id 128) only matters
     to moving mechanics objects such as bolt 733.
5. **Mannequin** (939.gd reflex_0_rule_0, :445-715, the model's only hit
   path). The mannequin's ZDE zones are all identifier 1
   (`i_Mannequin_Magie__f9d3f5c2dd.tscn:329-373`):

   | Index | Zone number | Shape | Local position |
   |---|---|---|---|
   | 0 | 1 | arm, sphere r 0.6 | (-1.2, 1.4, 0) |
   | 1 | 2 | arm, sphere r 0.6 | (+1.2, 1.4, 0) |
   | 2 | 3 | head, sphere r 0.6 | (0, 2.2, 0) |
   | 3 | 4 | body, sphere r 0.5 | (0, 1.3, 0) |

   The rule runs these steps in order:
   1. Gate: Hype `dsg_17 == 1` (:453).
   2. Gate: Gogoud `/world/0/155` **not** in Normal5 (:459).
   3. `dsg_3` = facing angle to Hype in degrees.
   4. **Zones 3 or 4 first** (:470): `dsg_17 = 0`, Normal4 reaction if Hype is
      in front. Not counted.
   5. Else zone 1 (:483): `dsg_17 = 0`, reaction 5/6, then the stage test.
      - The stage test is Global dsg_152==0 and the flag matching own stage;
        for stage 3 that is Hype `dsg_0 == 1`.
      - Pass: `dsg_2 = 1`; if `dsg_1` is already 1, success.
      - Fail: Gogoud Reflex3 + Normal6 (the "mauvais" dialogue).
   6. Else zone 2 (:599): the same, setting `dsg_1`.

   So the count is **one hit on each arm, either order, one arm per cast**,
   because any zone match consumes dsg_17.
6. **Tell Gogoud**. Success requests Gogoud `change_reflex(3)` and
   `change_normal(4)`, then own `dsg_0 = 4` (:545-575 and the zone-2 twin).
7. **Gogoud** (991.gd). The magic exercise is entered by:
   - Normal10 (sets dsg_0 = 3, dialogue `gogood_master_mannequin03`, then
     Normal5);
   - the dialogue completing into Normal11 (`completion_normal = 11`);
   - Normal11 putting Hype in Normal50 and going to **Normal16**, the watch
     state.

   In Normal16, talking to Gogoud (Button_Hit, facing) runs whatif03, passes
   through Normal5 and completes into Normal17. So the mannequin's Normal5 gate
   is closed only during hint dialogues.

   Normal4 polls `dsg_0 == 3 && 154.dsg_0 == 4` (:~860). When it holds:
   1. Hype dsg_56 = 1.
   2. Kill `/world/0/160`, wait 1000 ms.
   3. Distance check, turn, widescreen, microphone and camera.
   4. Normal18 (:960).
8. **Normal18** (:3263-3591) sets dsg_0 = 5, Global dsg_152 = 1, actions 20/34/35,
   runs `gogood_master_finimannequin`, sets Hype fields and goes to Normal5. The
   dialogue resource has `completion_normal = 19`. Normal19 is reached only
   this way; nothing calls `change_normal(19)`.
9. **Normal19** (:3591-3811) runs actions 36 and 25, pursues Hype to radius 2.5
   and turns. It then:
   1. Sets dsg_5 = 1.
   2. Runs `Proc_ActivateObjectOnPosition` on key `/world/0/142` at
      `MainActor.position + MainActor.sighting * 0.1` (:3729-3741).
   3. Resets the camera, Reflex0, Hype dsg_56 = 0, Normal23.

   After that, the key's own Reflex0 admits item 21 (see
   `gogoud-key-gate-static-2026-10-08.md`).

## 2. Port status per link

| Link | Status |
|---|---|
| Cast gating and spawn (28 Normal12, generate object) | Implemented (compiled; native spawn). |
| Fire motion | **Stood in**: card-less object held at spawn (documented in code, not yet in stand-ins.md). |
| Fire ZDE export (box, id 2, state-0 activation, layer 4) | Implemented in template; per-state enable via `_set_zde_shape_active`. |
| Fire dsg_17/dsg_0 window, magic points, kill | Implemented (compiled). Depends on external Brains /76, /27, Global. |
| Cond 28 query (`character.gd:1958`) | Implemented with Godot shape queries. Adaptations: one hit per actor, native ordering, ZDE before physical, excludes own generator. Sector rule matches the source (&2 to `contact_sector_active`, neighbours). `FUN_00438f40` mapped to `source_contact_eligible`/`source_object_active` (unverified). |
| Mannequin rule, zone order, dsg_17 consume, stage test | Implemented faithfully (no stand-ins in 939.gd). |
| Cross-brain Reflex3/Normal4 to Gogoud | Implemented. |
| Gogoud Normal10/11/16/4/18, dialogue continuation to Normal19 | Implemented; continuation via dialogue resource `completion_normal`. |
| Key activation (Proc12, Func164 sighting, operator 2/18) | Implemented (checklist stage3_key.json). |

The checklist (`docs/astrolabe/manoir-checklist.md:62`) records
`stage2_magic.json` completing the whole magic chain to the finimannequin
dialogue. **The chain works in the port when Fire is placed right.** That route
stages Gogoud (`dsg_0 = 3`, `normal 0`), gives and draws the sword, and
teleports to two exact spots.

## Most likely break for the take-driven bot

Ranked. These are static inferences, not runtime-confirmed.

1. **Placement, not facing.** Fire does not travel; it is a fixed 1.3 m wide
   box from Hype's feet-origin to 5.6 m ahead.
   - In the source frame the mannequin is at (-54.90, 120.57), Godot y 24.93,
     yaw basis x = (-0.971, 0, -0.239). Its arm centres are at source
     **(-53.73, 120.28)** and **(-56.06, 120.86)**: exactly the route's `face`
     targets.
   - The passing route casts from (-53.7, 124.5) and (-56.0, 124.0), at
     z 26.0, about 4.2 m away.
   - From there the box axis is 1.2 m from the body centre, against a clearance
     of 0.65 + 0.5 = 1.15 m. That is only about **5 cm** of margin.
   - So to count, Hype must stand 0.6 to 2.4 m off the mannequin's centreline
     along its local X, under about 5.6 m away, facing roughly parallel to the
     line to an arm. Hype's origin must also sit about 0.2 to 2.6 m above the
     mannequin's origin; the route uses about +1.07 m.
   - Matching only the take's facing from a different spot does one of two
     things:
     - too far (over 5.6 m), too wide or off-height: no zone overlaps and
       nothing happens;
     - near the centreline: hits the head or body first, giving a Normal4
       wobble with dsg_17 consumed and nothing counted.
   - Note that Stephen's original take had a *moving* fireball. If the original
     Fire actually travels (or is parented to Hype's hand) and the port's hold
     is wrong, the take's cast spot is legitimately farther away than the port
     can reach. The hold adaptation is the first thing to check against the
     original. Its code comment cites only "with gravity it sank", not source
     evidence that the object is stationary.
2. **Activation and window.** The Fire ZDE counts only while:
   - the object is `source_object_active` (a release cue on Hype's action-271
     clip, if any, must have fired);
   - it is in state 0 (before action 1);
   - Hype dsg_17 is still 1 (until action 0 completes).

   If the bot's cast is interrupted (Hype leaves Normal12, enters Normal94, or
   action 0 completes at once), the window can be zero frames.
3. **Preconditions that silently skip the rule:**
   - Gogoud in Normal5, for example if the bot talks to him (whatif03) or a
     dialogue is still running;
   - Gogoud not yet at dsg_0 = 3 / Normal16. The mannequin still counts, but
     Normal4 will not advance;
   - sword not drawn, so no cast at all;
   - spell not Fire;
   - magic points under 10.
4. Unlikely: material or layer mismatch (verified: 2 vs mask 2, layer 4 in
   query mask 4|16); sector (Fire inherits Hype's; the mannequin's is `world/1/10`.
   That Hype's sector equals it or is a neighbour at the cast spot was not
   checked).

Quickest runtime discriminator (not run): during the bot's cast, sample on 154

- `source_collide_perso_zde_no_with_type_zde(1..4, 2)`;
- Hype dsg_17 and dsg_0;
- Gogoud `current_normal`;
- the Fire instance's `source_object_active`, `current_state`, `ZDE_0_0.disabled`
  and its global box versus the arm spheres.

`route_bot.gd:506` already samples zones 1/2.

## 3. Order and timing

- Fire sets dsg_17 in its first schedule tick, so a hit is seen on the
  mannequin's next reflex tick at the earliest. Both Brains run at physics
  priority -10, and their relative order is unspecified, which costs at most
  one frame. Fire does not clear dsg_17 on contact, so there is no same-frame
  race for Fire.
- Bolt 733 (crossbow stage) **does** race: its Normal1 rule 0 clears dsg_58 and
  dsg_17 and kills itself on its first contact. Its contact is either
  `collide_with_nothing` false (sliding into the mannequin's ZDR, layer 8) or
  its own ZDE zone 1 against material 1, which is the mannequin's ZDE. That is
  the same overlap the mannequin tests, so Brain order decides whether the
  mannequin sees the flags.
- Any zone match (including head and body) consumes dsg_17, so each cast counts
  at most one arm. Head and body are tested before the arms in the same rule.
- The mannequin writes Gogoud's Normal4 request before its own dsg_0 = 4.
  Gogoud's Normal4 reads 154.dsg_0 on a later tick, so the order is safe.
- Normal19 exists only as the finimannequin dialogue continuation. It needs the
  dialogue to complete; skipping or aborting it never spawns the key.
