# Training completion: sword and magic stages to the key (static, 2026-11-03)

Static read-only. Nothing built or run. V = verified in code, I = inferred. Paths are
under `godot/generated/hype/levels/manoir/ai/models/` (`G` = 991.gd Gogoud, `M` = 939.gd
mannequin, `H` = fix/28.gd Hype, `S` = fix/708.gd sword, `K` = 703.gd key) and
`godot/addons/astrolabe/character/character.gd` (`C`). Builds on
`magic-mannequin-chain.md` and `self-behavior-change.md`. The earlier "corrected stage
table" holds: all three mannequins run the same model M (the three persos/i_Mannequin_*.tscn
each load 939.gd, line 25), only `dsg_0` 1/2/3 differs.

## A. Sword stage (stage 1, mannequin /world/0/153)

**A1. The count is two arms, not three swings (V).** M reflex_0_rule_0 (M:445-722):
1. Gate: Hype `dsg_17==1` (M:454) and Gogoud not in Normal5 (M:459).
2. Zones 3 or 4 (head, body) are tested first (M:470): consume `dsg_17`, mannequin Normal4
   wobble, nothing counted.
3. Else zone 1 (left arm, M:483): consume `dsg_17`, then `dsg_152==0` (Global) and
   (`dsg_55==1` and own `dsg_0==1`) for the sword stage. Pass: `dsg_2=1`; if `dsg_1==1`
   already, success (M:546-574).
4. Else zone 2 (right arm, M:603): same, sets `dsg_1`, success if `dsg_2==1` (M:666-695).
5. Success = Gogoud `change_reflex(3)`, `change_normal(4)`, mannequin `dsg_0=4`.

So the sword stage needs a hit on each arm, in either order, one arm per swing (any zone
match consumes `dsg_17`). Head and body hits are a wobble only. `dsg_1`/`dsg_2` are never
reset anywhere in M (V, grep), and a swing that hits both arms is impossible.

**A2. The Hype side (V).**
- `S` (708) is the sword object. Normal0 (S:30-110) sets Hype `dsg_55=1` and `dsg_17=1`,
  sets a vector speed (-29.6/-33.3/-35/-38.3/-43.9 depending on Hype's gait flags), then
  Normal1 (S:116-139) clears both flags and kills itself when 100 ms game time has elapsed
  **or `source_see_perso(Hype, sword)` is false** (a layer-1 world ray from Hype's first
  bound to the sword's; C:1360; actors never block it, C:1368).
- The template `persos/Object_template__d27b7216c2.tscn` has one live ZDE shape: a
  1.5 x 0.2 x 2.4 box at local z -0.9, identifier 2, zone 1, active in states 0-9
  (lines 111-130). Zone 0 (identifier 4096) is state 10 only and `disabled`. The box is 0.2 m
  thick vertically, so only a thin horizontal slab at the sword object's origin height
  counts. The object copies Hype's global transform at generation (C:1305), so that height is
  Hype's origin height.
- Hype writes `dsg_55/dsg_17` from H Normal29 (H:11054-11520) and Normal31
  (H:12799-13020) either via the sword object (Global `/persos/15 dsg_16==0`) or directly
  (`dsg_16==1`, flags cleared when the action completes). Swing entry needs
  `Button_Hit` just-pressed, `dsg_59==1`, `dsg_7==1`, `dsg_19==0` (e.g. H:1610-1628, and
  from Normal30 at H:12404). `dsg_56==1` sends Hype to Normal41 (H:1610) so Hype cannot swing
  while a Gogoud scene holds him.
- Lateral arithmetic (I, from shape data only): box half-width 0.75 against body r 0.5 and
  arm r 0.6 at x=+-1.2 gives an arm-only window of about 1.25 to 2.55 m between the box
  centreline and the mannequin axis. Anything nearer hits the body first. The slab must
  also be at height roughly 0.7 to 2.1 m above the mannequin origin. The sword moves along
  Hype's heading, so heading and lateral offset matter, the 0.1 s lifetime sweeps depth.

**A3. What Gogoud waits for (V).**
- Normal14 (G:2448-2623) is the stage-1 watch state. `Button_Hit` with Hype `dsg_19==0`,
  `dsg_14==0`, `dsg_56==0`, distance <2.5 m and mutual facing <25 deg starts
  `whatif01` and goes to Normal5. This is the only thing Normal14 does.
- Normal4 (G:688-963) polls three branches. Stage 1 branch (G:690-772): `dsg_0==1` and
  `/world/0/153 dsg_0==4`. When true:
  1. Turn toward Hype in 3 ticks.
  2. Hype `dsg_56=1`; camera retarget to perso 141 and set absolute position.
  3. **`dsg_1=255`** (G:746).
  4. Wait 1500 ms game clock.
  5. `change_reflex(1)` (start the fade, no wait).
  6. Request action 54 and wait for its completion.
  7. `change_normal(13)` (G:772).
- Normal4 is entered only from M's success path (V, grep: no other `change_normal(4)` on
  155). So **reaching Normal4 at stage 1 already means both arms counted** and
  `153.dsg_0==4`.
- Normal13 (G:1998-2446) with `152.dsg_0!=4` teleports Gogoud to (-49,134,25), `dsg_1=0`,
  reflex 2 (fade in), action 55, camera on mannequin 152, Hype to (-42,142,25.3), Hype Normal50
  (scripted walk, waits for Hype `dsg_24==1`), turns, then `change_normal(9)` (stage-2
  intro). The other branch (arbalete done) does the same for mannequin 154 and goes to
  Normal10 (stage 3).

**A4. Gogoud `dsg_1` is a fade-out opacity, not a timer (V).** Reflex1 (G:5087-5119):
`dsg_1<20` makes Gogoud transparent+hidden and returns to reflex 0; otherwise
`dsg_1 -= 10`, `source_set_transparency(dsg_1)`, then one reflex tick wait. From 255 it
takes 24 steps (255, 245, ... 25, 15 -> hidden). Reflex2 (G:5133) is the inverse fade-in
(+10 until >240) used by Normal13. The countdown starts only after Normal4's 1500 ms wait,
and it is independent of whether Normal4 later reaches action 54 and Normal13. It
does **not** gate the transition. A log of `255,245,235,225` therefore proves: both arms
counted, the 1.5 s wait elapsed, reflex 1 was running. It says nothing about training being
unfinished.

## B. Magic stage: arm hit versus wobble, and port departures

Count rule is the same M code: Fire (`fix/817`) sets Hype `dsg_0=1`, `dsg_17=1`; stage test
is Hype `dsg_0==1` and own `dsg_0==3` (M:540-544, 660-664). Arm = zone 1 or 2 (id 1 sphere
r 0.6 at (+-1.2,1.4,0)); head = zone 3 (0,2.2,0) r 0.6; body = zone 4 (0,1.3,0) r 0.5; head
and body win if the Fire box also touches them (V, M:470 before 483).

Arm-only window for Fire (box 1.3 x 1.3 x 5.876, centred z +2.761, half-width 0.65): box
axis between about 1.15 and 2.45 m from the mannequin axis laterally (I, geometry), and Hype
origin 0.2 to 2.6 m above mannequin origin. The inner edge is the tight one: 0.45-0.5 m of
slack from a take cast that has been aimed at an arm centre (1.2 + 0.4..0.5 off).

| Link | Status | Evidence |
|---|---|---|
| Cond28 zone ordering (3,4 before 1,2; one arm per cast) | Implemented, same as source | M:470-483, dsg_17 consumed at every branch |
| Zone matching by material (target must carry all mask bits) | Implemented | C:1965-2000, `contact_material_matches` |
| One-hit-per-actor in the query | Port adaptation, harmless here | C:1996-1999 (`selected_actors`). Fire has one shape, mannequin zones are separate queries |
| ZDE vs ZDE ordering | Port adaptation (unsourced ordering), harmless | C:1989-1991. Only matters when several actors overlap the zone |
| Shape sizes / vertical extent | Implemented from source shape data | `i_Mannequin_Magie__f9d3f5c2dd.tscn:335-373`, Fire template box |
| Mannequin ZDE activation per state | Implemented, not a factor | all 4 zones active in states 0-30 (`source_active_states`) |
| Fire ZDE active in state 0 only, layer 4 | Implemented | template, `_state_movement_collision` |
| Fire does not move | **Stand-in / unsourced hold**. Comment at C:1256-1259 cites "with gravity it sank"; the existing addendum shows the asset-level travel (0 to 2.7 m of channel motion) is covered by the static box, so equivalent if the source also lacks dynamics | `magic-mannequin-chain.md` addendum |
| Spawn transform captured at generation, not at release cue | **Possible departure (I)**. `generated.transform` copies Hype's transform when the action starts (C:1305). If Hype's heading changes before the release cue (`waits_for_release`, C:1250-1255) the box is oriented to the earlier heading. Source behaviour not checked | C:1250-1306 |
| Generator-contact skip | Implemented, **not a factor for the mannequin** (skips Hype only, `actor == source_generator`, C:1941) | C:1941 |
| Fire dsg_17 window | Implemented; `dsg_17` stays 1 until action 0 completes or Normal94 | fix/817.gd |
| Sector rule | Implemented; Hype and mannequin sectors must be equal or neighbours (C:1951-1962). Not checked for the bot's cast spots | I |

**Verdict (I): no port departure found that makes a source-counting cast register as a
wobble, other than the spawn-transform/heading timing.** The ZDE shapes, zone order,
mask, layer and activation all match the source data.

## C. Chain from completion to the key (V unless marked)

1. Gogoud Normal4 (G:858-963), magic branch: `dsg_0==3` and `154.dsg_0==4`.
   - Hype `dsg_56=1`, kill perso 160, 1000 ms wait.
   - **Distance check** (G:897-917): if Hype is within 4.5 m of waypoint 17-1/6, Hype is
     teleported to that waypoint.
   - Turn, widescreen, microphone and camera, then `change_normal(18)`.
2. Normal18 (G:3287-3605):
   - If Hype is within 3.0 m of waypoint 17-1/6 **or** waypoint 051ba63d (G:3322-3343 region), Hype is
     forced into Normal74, a 1000 ms wait follows, Hype is set up for Normal50 (a scripted walk
     to a waypoint) and **waits for Hype `dsg_24==1`** (open-ended).
   - Sets Global `dsg_152=1`, `dsg_0=5`, actions, `run_dialog("gogood_master_finimannequin")`,
     Hype `dsg_48=1`, `dsg_135=Gogoud`, `dsg_30=1`, `dsg_75=1`, Normal5.
3. Dialogue completion calls Normal19 through the resource's `completion_normal=19`
   (`ui/screen.gd:229`, requires Gogoud's Brain model to be 991). Nothing else enters it.
   Selecting a line needs `Button_Hit` (screen.gd:49) when `allow_skip`, or runs by voice
   time. See D3.
4. Normal19 (G:3618-3825): actions 36/25, pursue Hype, `dsg_5=1`, then
   `source_activate_at_position` on `/world/0/142` at Hype position + sighting*0.1, 500 ms
   wait, action, camera reset, reflex 0, Hype `dsg_56=0`, Normal23.
5. Key reflex (K:562-870). It fires when:
   - `distance_xy(key,Hype)<0.85` (or `<1.1` with Hype `dsg_8==1`);
   - `distance_z<1.5`, `relative_z<0.2`;
   - Hype `dsg_14==0`;
   - Hype faces the key within 80 deg.

   Then:
   - Key lifts +2 z if not in action 0; sets Hype `dsg_56=1`, `dsg_135=key`, `dsg_48=1`.
   - **Waits for Hype `dsg_48==0`**. Hype clears it only in Normal48 rule 3 (H:16984-17026)
     once it faces the key within +-10 deg (turning at `dsg_107` per tick).
   - Sets Hype `dsg_70=1`, **waits for `dsg_70==0`**. Hype sees `dsg_70==1` in
     Normal48 rule 4 (H:17029), Normal49 (H:17042-) or Normal41 (H:16280) and goes to
     **Normal74** (not 33; no `dsg_70` reference in Normal33, V grep). Normal74 rule 1
     (H:20515) clears `dsg_70` and returns to 48.
   - `source_inventory_add_object` (K:856), Hype `dsg_97` = item id (21).

   Stephen's "Gogoud gives the key" is the Normal19 activation placing the key at Hype.

Things a bot or the port can miss:
- Dialogue must complete (Normal19 exists only through it). Aborting or skipping the
  continuation never spawns the key.
- Hype must be in a normal that honours `dsg_56` (rule 0 -> Normal41) to reach 48/74. If
  Hype is in a normal without that rule (airborne, in a custom action) `dsg_48` never clears.
- `dsg_24` wait in Normal18 (only when Hype is within 3 m of the waypoints) depends on the
  scripted walk finishing.
- The key distance and the 80 deg facing check fix the pickup. The key is placed at
  Hype+0.1 sighting so distance is fine, but `relative_z<0.2` and `dsg_14==0` are not
  guaranteed.
- The bot must not press attack/inputs that restart Hype's normal during the sequence.
- Hype `dsg_56` hold: do not run position/heading scripts that fight the scripted turn.

## D. The observed failures

**D1. Magic, tr1/tr2: one arm counted, other casts wobble.**
Verdict: aim, not a port bug (I). The arm-only window (A/B above) is 1.15 to 2.45 m
off the mannequin axis. The take's casts 2 and 3 sat 0.4 and 0.52 m from an arm centre, so
their inner slack to the body limit is only about 0.45-0.5 m; a bot spot 0.3-0.5 m
off toward the centre crosses that edge and flips the cast to a body hit. Cast 1 of the
take itself computes as a body hit (0.9 vs 1.15 limit), which is consistent with the original
take using cast 1 as a wobble and casts 2/3 as the arms. All geometry, order, activation
and consume-on-first-zone rules match source; the only open port question is the heading used
for the spawned box (spawn-time transform, C:1305) versus the heading at the release cue.
Bot must: stand within about 0.2 m of the take's cast spots on the outer side (away from
the body) and with the box axis lateral offset >=1.25 m, face the arm centre at the
cast moment (not mid-turn), and not cast while Gogoud is in Normal5 or `dsg_0` is wrong.
Prefer a margin of 0.3 m outward. Check `fire.current_state`, `source_object_active` and
`zde_1/zone` overlap at the cast frame to confirm.

**D2. Sword, tr3: Normal4 reached, `dsg_1` 255->225, no advance.**
What must happen for N4 -> N13: after N4 starts, 1500 ms game clock, then action 54 must
complete (`source_action_completed`, C:1773), nothing may call another `change_normal` on
Gogoud, and Gogoud must stay alive in N4 (G:759-772). The countdown only proves N4 was in its
wait->reflex phase. Candidates for the stall, ordered:
1. **A further arm hit after success (V mechanism, I that it happened).** After success
   the mannequin has `dsg_0=4`. A later swing that touches an arm passes the `dsg_17` gate
   (the sword object sets it, M:454), consumes it, but now fails the stage test
   (`dsg_0==1` is false), so the **else branch** runs (M:575-601 / 696-722): Gogoud
   `change_reflex(3)` and `change_normal(6)` ("mauvais_mannequin" dialogue, G:984-1096,
   then Normal5). That kills Normal4's coroutine, swaps reflex 1 for reflex 3 (the fade stops
   at 225, matching the observed freeze) and the stage is **permanently deadlocked**:
   mannequin `dsg_0` stays 4, so no later hit can pass the `dsg_0==1` test and re-enter Normal4.
   The swing at take time 162.90 is exactly such a candidate when Normal4 has already begun
   (Hype `dsg_56=1` only takes effect after 3 ticks and only in normals with rule 0).
   Note the original take swung then because the original was still waiting for the 3rd
   hit; the bot swings at take times while the port already counted two arms earlier.
   Verify by logging Gogoud `current_normal` and `current_reflex` after the freeze (expect 6 or 5
   and reflex 3) and mannequin `dsg_0/dsg_1/dsg_2`.
2. Action 54 never completes: Gogoud `request_action(54)` requires `state_54` and finishing one
   clip cycle (C:1779-1806, `_state_next(54)` not defined so it ends). Unlikely if the
   passing run completed, but check `is_in_action(54)` and `action_complete`.
3. Time base: the passing run showed N4 163.13 -> N13 164.26 (1.13 s) although the code
   waits 1.5 s game clock, so take time and game time are not 1:1; do not use take time to
   predict Gogoud transitions.

The first swing during a run-skid (Normal30) is not a count issue: the swing is started
from Normal30 by `Button_Hit` (H:12404), the sword object spawns from H Normal29 schedule.
Position tolerance for the sword is tighter than for Fire: box 0.2 m thick (vertical), arm-only
window about 1.25 to 2.55 m lateral, lifetime 100 ms, killed early if the layer-1 ray
Hype->sword hits geometry. Matching position within 0.1 m and heading within 0.05-0.3 rad
can move the box 0.2-0.7 m laterally at 2-3 m, enough to flip arm to body. Bot rule: swing
only from a stationary stance (not the skid), aim the box at an arm centre, never swing again
once Gogoud has entered Normal4 for that stage (watch `153.dsg_0==4` or Gogoud normal 4).

**D3. Key not obtained with incomplete training: expected** (Normal19 never reached). The
~10 s longer dialogue (I): a line advances by `Button_Hit` select only when the panel is
opened (`modulate.a>=0.99`, `dialogue.gd:251-255`) and `allow_skip`, else by voice time
(`dialogue.gd:156-176`), and drawing waits while MainActor's speech holder is set
(`dialogue.gd:129`). A bot replaying the take's select presses at take times can land them
before the panel is open or during Hype's own speech, so they are dropped and the port falls
back to voice length. The take's 2.5 s means the player skipped. Not a Gogoud-script bug. Port
check: whether the select press was received (add a log in `Dialogue.select()`), and whether
the panel open time (0056d860 steps, `SOURCE_ALPHA_STEP`) matches the original.

## Open items
- Log, not run: Gogoud `current_normal/current_reflex`, 153 `dsg_0/1/2`, Hype `dsg_17/55`,
  and the swing frame for the tr3 freeze.
- Decomp not consulted for: whether `generated.transform` timing matches the source at the
  release cue, and whether card-less objects get dynamics.
