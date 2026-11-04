# Brigand: Wellet's race chain, end to end

Static trace, 2026-11-04, worktree at main 02e0f42. Nothing was run. Generated paths are under
`godot/generated/hype/levels/brigand/` (G/ below: `ai/models/1800.gd` is `G/1800.gd:line`); runtime under
`godot/addons/astrolabe/` (R/). Dialogue mechanics are in dialogue-answers.md and not repeated. Original evidence:
`TestResults/stephen-linear-brigand-20261102/recording/events.jsonl` (2 Hz Hype samples) and frames (index ~ s-5).

## Facts that correct the brief

- The race kid is Wellet (178, model 1800). In 1800, `character()` is **Wellet**, not Hype. `source_set_absolute_position
  (214.28,-166.44,-9.279)` (G/1800.gd:201-208) moves **Wellet** to the finish area. **Hype** is placed at
  (-68.9165,-9.3348,-5.2099), sighting (-0.8069,-0.5906,0), by `source_actor_set_absolute_position_from_receiver` +
  `source_change_sighting` (G/1800.gd:186-197); camera 0/2 to (-66.22,-7.36,-2.38) (G/1800.gd:198-213).
  The take agrees: Hype jumps to exactly (-68.9,-9.3,-5.2) at 793.4 s.
- The time limit is `dsg_6 = 60000` ms, set in `persos/iKid_CourseBrigand__7f090a5a73.tscn:995`, tested in 1800 normal 6
  (G/1800.gd:2280) against `dsg_5 = source_game_time()` (G/1800.gd:1217). Hype stands still during `wellet_depart`
  (G/1800.gd:1180-1198, 4 s) and the clock starts when it ends.
- No competitor, checkpoint count or lap logic exists: the "race" is a 60 s time trial through 37 markers ending at
  marker 168. Wellet does not run the course; he is parked at the finish and runs to Hype only after the win.
- The 37 markers (`iCourse_Brigand1_*`, models 1445) are chained by `dsg_0` in level.tscn Brain nodes. Order by world id:
  169 > 154 > 161 > 163 > 159 > 155 > 157 > 152 > 150 > 148 > 136 > 146 > 144 > 142 > 140 > 138 > 135 > 137 > 167 > 165 > 139 >
  143 > 141 > 145 > 147 > 149 > 151 > 170 > 171 > 153 > 156 > 158 > 160 > 162 > 164 > 166 > 168 (no `dsg_0`: last).
  169 starts in normal 1 in the level data, all others in normal 0.

## Race timeline from the take (Hype 2 Hz samples; t = elapsed s)

| t | Evidence | Meaning |
| --- | --- | --- |
| 581.7-592 | Hype (192.4,-121.1,-15.3) idle, normals 48/28 (dialogue) | First talk. Frame 785 shows the 10-plastyk counter. "No" path (refus): no race. |
| 650.7-661 | Hype jumps to (112,-13,-20) and back to (249,-212) | Toras round trip, unrelated. |
| 782.7-792.4 | Hype (192.8,-124.1,-15.0), normals 48/28 | Second talk, ends with Yes on the explanation page. |
| 793.4 (between 792.4 and 794.4) | Hype at (-68.9,-9.3,-5.2), normal 48 | 1800 normal 1: 2 s fade out, placement. Frames 790/793: grass start, hourglass at top left, Wellet text "Good fortune... ON YOUR MARK...! SET...! GO...!!!". |
| 799.1 | first move, normal 6 (run action 345), sparkle trail (frame 810), hourglass drained a quarter | Race starts after the 4 s depart dialogue. |
| 799-838 | one continuous run, (-69,-9) to (211,-170), normal 6/92 (jumps at 804.7, 805.7, 826.1), 2 Hz gaps <= 0.72 s | About 39 s of the 60 s. No respawn, no teleport. Lowest z -16.03 at 816.23 (32.1,-86.9); -15.93 at 816.73; no warp followed. |
| 838.2 | (210.6,-169.8,-14.3) n=30 a=378, glow (frame 834) | Last marker (168) reached; win. |
| 839.2-841.7 | n=50 then 49 (walks to (214.9,-174.7), wp 17-1/104), then 48 | Hype 50/49 path driven by 1800 normal 6. |
| 845-851 | n=61, 78, 90 | Reward animation (red potion upgrade; frame 841 shows a held gold object). |
| 858-868 | n=28/48 at (214.9,-174.7); frame 855: "If you want, you can run the race again." Yes/No | `wellet_reussit`. Stephen chose No. |
| 869.2 | Hype jumps to (192.8,-124.0,-15.0), Wellet's dialogue spot | `wellet_refus2` > completion 1800 normal 2: reset, Hype put back at the saved dsg_2. |
| 870-893 | normal 4 idle at that spot | Back to normal gameplay. |

Take caveats: only Hype is sampled; 181's and 1800's normals are not observed. The 100 ms water test in 1892 normal 2 could
have been missed by 2 Hz sampling (z -16.03 at 816.2). The take cannot tell whether 181 was in normal 2 during the race.

## Chain, link by link

Legend: IMPL = implemented, STAND = stand-in, MISSING, UNVER = unverified.

1. **Offer.** 1800 normal 3 (initial) records `dsg_4 = own position`, then normal 4 (G/1800.gd:1997-2020). IMPL.
   Normal 4 (G/1800.gd:2036-2257): Button_Hit pressed, Hype dsg_19 == 0, dsg_14 == 0, dsg_56 == 0, 3D distance < 3, vertical
   distance < 1, facing angle < 45 deg; then Hype dsg_56 = 1, dsg_75 = 0, camera 0/2 `change_angle(0,0,1.3)`, dsg_48 = 1,
   dsg_135 = Wellet, actions 8, 1, incremental turn, action 2, `source_position_dialog(305,10)`, stores `dsg_2` (Hype position)
   and `dsg_3` (Hype sighting), then dialogue by flags: `dsg_1 == 1` > `wellet_recommencer`; else `dsg_0 == 0` > cash HUD shown +
   `wellet_intro`; else `wellet_echouer` (G/1800.gd:2201-2217). Hype dsg_30 = 1, waits dsg_30 == 0, then normal 5 (idle).
   IMPL. Runtime guards that can fail: `source_camera_change_angle` (needs camera 0/2 SourceCamera; character.gd:1744), `source_turn_angles`.
2. **Affordability.** `wellet_intro` answer 0: gold -100, next `wellet_explicationcourse`, shortfall code 0 >
   `wellet_plastyksinsuffisants` (completion 1800 normal 0). Answer 1 > `wellet_refus` (completion normal 0). R/ui/dialogue.gd:415-424
   spends gold through the Hype inventory. IMPL (dialogue-answers.md). Needs >= 100 gold in the test save.
3. **Yes callback.** `wellet_explicationcourse` answer 1: completion_normal 1 on model 1800, no next item; answer 0 loops to the
   same item. `screen.gd:_answer_selected` stores {1800, 1}; the Brain change fires after the close fade. IMPL.
   `refus2` carries completion normal 2 (reset), refus/plastyksinsuffisants/depart carry normal 0.
4. **1800 normal 1 (start)** G/1800.gd:121-1225. IMPL except where noted. In order:
   - Hype dsg_71 = 0, own dsg_0 = 1, music fade/`send_music`, actions 3, 7, 4 (:124-160).
   - STE_FADE texture 0, `source_execute_fade(0,255,2.0)` and wait on `source_fade_in_progress` (:162-182; R/ui/screen.gd:136-190 tween,
     real-time). IMPL.
   - cash HUD off; Hype to (-68.9,-9.3,-5.2) + sighting; camera 0/2 to (-66.2,-7.4,-2.4); Wellet to (214.28,-166.44,-9.279);
     `source_camera_reset(0/2)` (R/camera/source_camera.gd:446 returns false without `initial_framing`) (:183-215). IMPL.
   - Hype `ChangeComport` 104 (:216-228; fix/28.gd:26108, race outfit: no stand-in; 105 restores).
   - 37 markers set active (:229-376, `source_set_object_active_state(true)`, always true, character.gd:1808); 169 > normal 1,
     other 36 > normal 0 (:377-857; each needs Brain `source_model_path == level:/ai/models/1445`).
   - 53 other Brigand actors (0/3, 85, 48... 0/18, 67, 68) set inactive (:858-1070).
   - 190 (model 2001) > normal 1, **181 (1892) > normal 2**, 43 (742) > 3, 16 (509) > 4 (:1094-1133).
   - 400 ms, transparency 0, **STAND generic-4-350 x2 (:1152, :1154), generic-4-349 (:1156)** (GPI SetImage/SetDraw: the hourglass),
     fade in `(255,0,1.0)`, 1 s, `wellet_depart` (completion 0), 4 s, `source_end_dialog` (clears the pending completion),
     Hype dsg_84 = 1, dsg_56 = 0, **STAND generic-4-350 (:1215), generic-4-353 x2 (:1218, :1220)** (GPI MovingUV, hourglass
     sand), `dsg_5 = clock`, normal 6 (:1170-1222).
5. **Timer.** 1800 normal 6 (G/1800.gd:2278-2665). `source_time_elapsed(dsg_5, dsg_6)` = Brain `clock_milliseconds` difference
   (R/behavior/behavior.gd:275); the Brain clock only advances while the actor is active (advance(), :263). IMPL.
   HUD: the hourglass is shown in the original (frames 790, 810) and **MISSING** here (STAND generic-4-349/350/353; also
   :2289 generic-4-350 node 10 at timeout). Logic unaffected.
6. **Course progression** (1445). Normal 0: action 0 > normal 2 (idle). Normal 1: action 1 > normal 3. Normal 3
   (G/1445.gd:99-160): if `distance_xy(marker, Hype) < 2` **and** `Hype.z + 1 > marker.z`: if `dsg_0` (next marker) set, next marker
   > normal 1; else Global `dsg_83 += 1`; this marker > normal 4. Elsewhere, within 9 xy: sound 103 + 102 once (`dsg_1` latch).
   Normal 4: sounds 103 + 31, action 2, action 0, normal 2. Order rule: only the next marker is armed; nothing counts skipped markers.
   IMPL (`NativeContactPickup.distance_xy` character/contact_pickup.gd:14). UNVER: the marker family
   `characters/feucourse__b3617575/.../actions.gd` lists states 0 and 1 only; if `state_2` does not exist,
   `request_action(2)` fails at once and marker 168 stays in normal 4 for one tick (see item 7).
7. **Win detection** G/1800.gd:2350: `source_actor_is_in_behavior(0/168, normal, 4)` (R/character/character.gd:1652,
   behavior.gd:372). IMPL. Risk: if normal 4 lasts under one tick the poll can miss it (current_normal is set immediately in
   `change_normal`, so with a fixed Brain order the 1800 poll still sees it once; UNVER, see test).
   Win sequence (:2350-2665): Hype dsg_56 = 1; waits Hype dsg_14 == 0; **STAND generic-4-349 (:2383), generic-4-165 (:2388,
   Camera_SetPosition)**; camera 0/2 `change_state(3)` (R/camera/source_camera.gd:487: false unless mode in 0/3/6, falls through);
   music; Hype `dsg_157[0] = wp 17-1/104` (G/…/waypoints/17-1/104.res exists), `dsg_166[0] = 7`; 1 s.
   - First win (`dsg_1 == 0`): Hype dsg_123 = 1, Hype ChangeComport 50, wait Hype dsg_24 == 1, spawn 0/172 (red potion upgrade,
     model 1398) 1.5 in front of Hype, 1 s, 172 > normal 1, wait Hype dsg_33 == 1, `sound_kill_music`.
   - Repeat win (`dsg_1 == 1`): dsg_123 = 2, Hype 50, bottles 0/189 (model 862) activated, wait Hype dsg_49 == 1,
     `source_inventory_add_object(189)` (character.gd:520, inventory.gd:256), 189 inactive.
   Then action 5, normal 8. Hype 50 > 49 (walk to wp, sets dsg_24 at fix/28.gd:17091) > rules at :18046-18177: no stand-in, UNVER
   at runtime. 1398 has STAND generic-4-176 x5, generic-4-165, generic-untyped (Camera_SetTargetPosition/SetPosition and an
   Ultra op on MainActor, G/1398.gd:197-771): UNVER whether the reward camera and `dsg_33` still complete. 862 has 121
   fail_unsupported guards (custom flags, Hype dsg_97/133, Global dsg_21/23, fix/535 camera): the repeat-win bottle path is UNVER.
8. **Wellet to Hype and reward dialogue.** Normal 8 (:3455-3510): turn toward 0/168, run (action 5) until within 3 of 168, then
   camera 0/2 `change_angle(0,0,0.7)`, Hype dsg_135 = Wellet, dsg_48 = 1, 5-step turn, normal 9. Normal 9 (:3521-3615): Hype
   within 2: actions 6, 1, 2, camera angle 1.3, `dsg_1 == 1` > `wellet_recommencer2` else **`dsg_1 = 1`**, `wellet_reussit`; Hype
   dsg_30 = 1, dsg_10 = 1, ChangeComport 105, normal 5. IMPL in logic; UNVER: Wellet runs by root motion (action 5) and needs the
   motion binding of the kid.
   `wellet_reussit`: Yes > explicationcourse (completion 1 = restart), No > refus2 (completion 2 = reset). Gold reward: none in
   the AI (the reward is the potion/bottles).
9. **Respawn / loss.** 1892 (G/1892.gd): normal 0 (100% awake in the port): every 300 ms Hype z < -16: camera `change_state(3)`,
   Global dsg_201 = 1, Global ChangeComport 2, dsg_34 = 0, normal 1; normal 1 waits Global dsg_34 == 1 then warps Hype to
   (137.93,-137.39,-14.635) or (200,-155.2,-14.259) by `dsg_0` (side) (:80-175, fix/1.gd:360-440 does the black fade). **During the
   race** 1800 normal 1 has put 181 in normal 2 (:1107): every 100 ms, Hype z < -16: same Global handshake, then (fade up, 2500 ms
   music fade) Hype to Wellet's `dsg_2/dsg_3`, Wellet to `dsg_4`, 500 ms, **1800 normal 7** (G/1892.gd:222-370). IMPL.
   Timeout (G/1800.gd:2280): Hype dsg_56 = 1, music fade, fade out, Hype to `dsg_2/dsg_3`, Wellet to `dsg_4`, camera reset,
   normal 7 (STAND generic-4-350 at :2289). IMPL.
   Normal 7 (:2679-3441): reactivates the 53 actors and deactivates the markers one per tick, 181 > normal 0, 190 > 0, 43 > 1,
   44 > 1, 16 > 2, Hype 105, dsg_56 = 1, camera, Hype turn toward Wellet, fade in 2 s, music, Wellet actions 8/1/2, dialogue
   `wellet_recommencer` (dsg_1 == 1) else `wellet_echouer`, Hype dsg_30 = 1, normal 5. STAND generic-4-349 (:2687). IMPL.
10. **Retry / quit and reset.** `recommencer*`/`echouer`/`reussit`: Yes > explicationcourse > completion normal 1 (a second race, no
    new payment: the -100 is only on `wellet_intro`); No > refus (normal 0) or refus2 (normal 2). Normal 2 (G/1800.gd:1238-1985):
    fade 2 s, Hype to dsg_2, Wellet to dsg_4, 38 actors on and 38 off, 181/190/43/44/16 restored, fade in, Hype dsg_84 = 0,
    dsg_56 = 0, normal 4 (:1972-1981). IMPL. Normal 0 (after refus) does action 3/7/4, camera reset, Hype dsg_84/56 = 0, normal 4. IMPL.

## Stand-ins, guards and weak points on the path (complete list)

- STAND: generic-4-349 (SetDraw) 1800:1156, 2383, 2687; generic-4-350 (SetImage) 1800:1152, 1154, 1215, 2289; generic-4-353
  (MovingUV) 1800:1218, 1220; generic-4-165 (Camera_SetPosition) 1800:2388; 1398: generic-4-176 x5, generic-4-165, generic-untyped.
  All HUD/camera cosmetics; none gates a state change.
- fail_unsupported guards that fall through in the game (`strict_unsupported` false): STE_FADE texture, camera reset/angle/state,
  transparency/transparent display, active state, ChangeComport model check, `source_run_dialog` refusal, Hype dsg_14/19/30/56/75/84/135.
  A strict stand-in run (`stand_in_report.gd`) stops at the first STAND above.
- Runtime methods that can return false: `source_camera_reset` (no initial_framing), `source_camera_change_state` (mode not 0/3/6),
  `source_camera_change_angle` (no SourceCamera on the actor), `source_run_dialog` (brain model mismatch, voice emitter missing).

## First divergences likely to block a full run (in order)

1. **181 (1892) awake from load.** The original never ran it at arrival (live read, brigand-maliq-chain.md:93-140). The port
   warps Hype on any z < -16 outside the race (Symptom 4, other worker) and, in the race, 1892 normal 2 can end the run
   (loss + `wellet_echouer`/`recommencer`) wherever Hype's z dips below -16. The take reaches z -16.03 at (32.1,-86.9) at 816.2
   s with no respawn. If the port reproduces that dip, the race fails here although the original did not (UNVER: poll could miss it).
2. **`wellet_depart` must stay open 4 s and not fire its completion.** If the port closes it early (no answers, untimed lines) the
   pending completion (1800 normal 0) cancels the race. The take shows it open at ~795-798 s.
3. **Win detection and Hype 50/49/dsg_24** (items 6-7). Needs the marker state 2 question answered and Hype normal 49 waypoint test.
4. **Wellet action 5 root-motion run** (item 8), else `wellet_reussit` never appears and Hype is stuck in 105/dsg_30.
5. Cosmetic: hourglass HUD (GPI), reward camera (1398 STANDs).

## Minimal targeted test plan (positioned starts; no teleport to the finish)

Fixture: save with >= 100 gold, Hype within 3 of Wellet (source ~ (192.4,-121.1,-15.3)) facing him. Log per tick: 1800/181/169/168 normals,
Hype source position, `Hype.dsg_56/14/30/123`, fade direction, pending dialogue completion, STAND/guard hits.
- T1 offer: Button_Hit > intro > Yes > explicationcourse > Yes. Assert gold -100 and 1800 normal 1 after the fade.
- T2 start: assert Hype at (-68.9,-9.3,-5.2), Wellet at (214.28,-166.44,-9.279), 169 in normal 3, 181 in normal 2, `wellet_depart` open
  4 s without completion, then 1800 normal 6 and Hype free (dsg_56 = 0).
- T3 progression (real movement, starts at the take's 799 s spawn, input-driven): markers advance in the chain order; Global `dsg_83`
  rises per marker (it is a counter only, nothing reads it here).
- T4 water: start Hype in the course at a point with z < -16 inside the race: expect 181 normal 2 > Hype to Wellet's `dsg_2` > 1800
  normal 7 > `wellet_recommencer`. Separately, walk the take's route through (32,-87) and log whether z < -16.
- T5 timeout: stand still 60 s of Brain clock > normal 7.
- T6 finish from a positioned start (3 markers before 168, Hype within 2 of 168 through 167 > 168 only by walking): expect 168 normal 4
  seen by the 1800 poll, Hype 50 > 49 > dsg_24, 172 spawned, Wellet runs to Hype, `wellet_reussit`; No > refus2 > normal 2 and
  Hype back at the saved dsg_2 with gameplay normal.

## Fix locations

- 1892 start rule: needs the missing original start rule for 181 (R/behavior/behavior.gd `advance` gate; brigand-maliq-chain.md).
  Do not weaken the 1892 logic itself.
- GPI hourglass: implement Proc_GPI_SetDraw/SetImage/MovingUV in R/ui/hud*.gd and the emitter (docs/astrolabe/full-game-ai-sites.md:115-120).
- Camera states used by the win and reward: `source_camera.gd:487` (`source_change_state`), 1398's Camera_SetTargetPosition/SetPosition.
- Marker action 2: family `characters/feucourse__b3617575/.../actions.gd` and its animation library (converter, not hand edits).
