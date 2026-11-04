# Brigand Maliq chain: missing cutscene camera and fade + teleport past the boars

Static trace, 2026-11-04, worktree at main 02e0f42. Generated paths are under `godot/generated/hype/levels/brigand/` (installed build; the worktree has no generated tree). Runtime paths are under `godot/addons/astrolabe/`. Nothing was run. The two symptoms are **two unrelated causes**.

## Identities and facts used

- Maliq is world/0/20, model 575 (`ai/models/575.gd`). The cutscene camera is world/0/21 Theif_Cam, model 535. Hype is `fix/28.gd`. IWorld is model 284. The "Trous" actor is world/0/181 iDummy_Brigand1_Trous, model 1892. Global is fix persos/15 (`fix/1.gd`).
- Brigand 575 names the camera actor as a **constant** `level:/persos/world/0/21` (575.gd:293, 313, 330, 345...). No designer variable is involved, so fc997dd (camera target from a designer actor variable, for 715) is not on this path. The 575 camera targets are also constants: world/0/52 and world/0/53 (Dummy_Cam_Theif2 and Theif1).

## Symptom 3: camera not selected (first divergence found)

Chain, from trigger to picture:

1. 575 normal 0 starts, `dsg_0` is false, so it hides Maliq and enters normal 3 (575.gd:~88-106, `change_normal(3)`). IMPLEMENTED.
2. 575 normal_3 trigger: Hype within 7.5 of wp 17-1/178, or XY distance to Maliq < 9, or within 5 of wp 179 (575.gd:225-262). IMPLEMENTED. The original stops Hype at 6.9 from wp178, so this fires.
3. Hype DSG75 = 1 and DSG56 = 1 (575.gd:263-276). IMPLEMENTED.
4. `generic-4-349` (Proc_GPI_SetDraw, HUD) is skipped, so nothing is drawn (575.gd:277). STAND-IN (generic-4-349). HUD only; it is not a fade and not a camera.
5. Widescreen toggle, display visible, Maliq action 17 (575.gd:~283-287). IMPLEMENTED (bars are expected; see frame evidence).
6. `camera_actor_7.SourceCamera.source_select_primary(1, 1, 1)` on world/0/21 (575.gd:288-294, args viewport 1, previous 1, placement 1). **STAND-IN-by-refusal, effectively MISSING.** `source_camera.gd:251-274`:
   - `reset_selected := not source_viewport_registered` (line 271). Theif_Cam's SourceCamera has no `initially_registered` in level.tscn; the default is false (source_camera.gd:14). Only StdCamer (world/0/2, level.tscn:15483-15485) is registered.
   - So `placement` stays 1. Line 274 `if placement in [1, 2]: return false` ("Pose copy and the optimal-placement solver have wider matrix contracts"). That path is not implemented.
   - The generated guard `fail_unsupported("... requires supported primary-camera selection")` then falls through in the game, because `strict_unsupported` is false (game/bootstrap.gd:33) and the emitter's `continue_past_guards` turns the `return` into `if strict_unsupported: return` (src/godot/ai.rs:343-358, documented at undocumented-adaptations.md item 5).
7. 575.gd:296-346 then still run: set camera actor 0/21 position to wp 180 or 153, prepare target, set target 0/52 or 0/53, `source_camera_change_state(0/21, 3)`, switch microphone. All IMPLEMENTED, but acting on a camera that never became the primary viewport owner. The picture stays on StdCamer following Hype. This is exactly "cutscene ran, camera did not".
8. Return to the gameplay camera, normal_0 `dsg_0` branch (575.gd:~110-125): `source_select_primary(1, 0, 1)` on StdCamer. StdCamer is the registered primary, so `previous == self` and it returns true (source_camera.gd:262-267). It works, which hides the bug.

Scope: the same `(1, 0|1, 1)` triple is emitted for brigand 509 (3 sites), 599, 544, 1029, 1186, 575. Every manoir site uses placement 0 (703.gd, 868.gd), which is why the manoir camera switches work. Any Brigand cutscene whose camera is not already registered fails the same way.

Original (frames in `TestResults/stephen-linear-brigand-20261102/recording/frames`, 1 fps; index n is at about n+5 s):

- 000459-000461 (464-466 s): normal follow camera.
- 000462 (467.3 s) to 000466 (471.3 s): letterbox bars, a fixed wide shot of Maliq on the path, subtitle "MALIQ : Aha! No-one ventures into our forest without our leave,". The framing is identical across these frames, so it is a fixed cinematic camera.
- 000470 (475.6 s): gameplay HUD and follow camera. 000474 (479.4 s): Hype walking on, Maliq on the ground.
- No black or faded frame anywhere in 000459-000484 (mean luma of the picture area 49 to 80, bars excluded).

Recommended fix: implement the placement-1 path of `source_select_primary` (Proc141 placement 1: copy the previous owner's pose to the selected camera; decomp `Proc141` and the viewport owner switch, see notes/gogoud-key-gate-static-2026-10-08.md:143-330 for Proc141 and clean-C commit refs). Fix location is godot/addons/astrolabe/camera/source_camera.gd:274. Do not turn the guard into a silent success. Alternatively, if the decomp shows that placement 1 on an unregistered camera is a no-op copy for a camera with no previous pose, implement exactly that.

New parser assumption needing live memory: that `(1, 1, 1)` and `(1, 0, 1)` decode as (viewport, reset-previous, placement). The names come from the Proc141 reader, in which manoir 703 uses placement 0. Check against live Proc141 arguments at Maliq's scene in the original (Orrery).

Red-capable assertion (isolated Godot test): load the Brigand level at 575 normal 3 with Hype placed within 7 of wp178 and assert that after the first scheduler pass `SourceCamera.source_primary_camera(viewport)` is the world/0/21 camera and `ASTROLABE_STRICT_BEHAVIOR=1` records no `source_select_primary` failure. It fails today at source_camera.gd:274.

## Symptom 4: fade to black and teleport past the boars

The source call is **not in Maliq's chain**. It is the "hole" respawn on world/0/181.

1. `ai/models/1892.gd` (Brain of iDummy_Brigand1_Trous, level.tscn:80552-80557, `current_normal = 0`, activation known and active). Normal 0 (1892.gd:36-95) waits 300 ms, reads Hype's source position into `dsg_1`, sets `dsg_0` from `x + |y| > 443`, and then **unconditionally** tests `dsg_1.z < -16.0` (line 65). IMPLEMENTED as written.
2. When z < -16: camera 0/2 state 3, Global `dsg_201 = 1`, Global (`fix/1.gd` normal 2) change, `dsg_34 = 0`, `change_normal(1)` (1892.gd:66-86).
3. `fix/1.gd:417-439`: `dsg_201 == 1` starts `source_execute_fade(0, 255, 0.8)` (a fade **to black**), then `dsg_34 = 1`. This is the fade. IMPLEMENTED.
4. 1892 normal_1 (1892.gd:~132-175): when Global `dsg_34 == 1` and `dsg_0 == 0` (x + |y| < 283), `source_set_absolute_position(137.93, -137.39, -14.635)` on **main_actor (Hype)** and on StdCamer (0/2), then a 3 s wait. IMPLEMENTED. Source (137.9, -137.4) is Godot (137.9, -14.6, 137.4), just past boars A (127.1, 97.2) and B (116.1, 116.9). This is the teleport. The actor receiving it is correct (the source does it to Hype).
   
So the port executes the source faithfully. The divergence is **whether 1892 runs at all**:

- The original recording shows Hype well below z = -16 on the way to Maliq with **no fade or teleport**: z = -16.9 at (47.4, -25.7) at 426.4 s, -16.68 on the save statue (56.5, -27.5) for 427.9-439 s, and -17.48 at 439.5 s (recording/events.jsonl actor events, source coordinates). Position then continues normally, and Hype reaches Maliq at z = -15.0. In the port the same trigger at z < -16 on that path fires 1892.
- The most probable reason is the known **sector-based AI activation gap**: the original leaves distant actors idle until nearby, and the port runs every Brain from level start (docs/astrolabe/manoir-checklist.md:108; notes/research/static-checklist-records.md:296-330 gives the original wake rule: FUN_00449cd0/FUN_00449c60, StandardGame +0x1b/+0x1c/+0x20/+0x24; the wake caller is not found). Actor 181 sits at Godot (170.4, -12.2, 136.3), about 130 m from Maliq, in sector world/1/18 (level.tscn:80518). The port's "run everything" model therefore evaluates a far-away race actor against Hype's local position. **UNVERIFIED:** nobody has observed 181 idle in the original. The alternative (the original evaluates it and uses a different z) is not excluded.
- This is not a self-ChangeComport, fall-through, wrong-typing or wrong-destination bug. The typed condition `dsg_1.z < -16.0` and the destination vectors are literal. Nothing from fc997dd, undocumented adaptation 1-6, or 575/535/566/1060 is involved: 575's only position writes are to actor 88 (575.gd:62-70) and camera 0/21, and 566/1060 write no Hype position.
- 284's two `if bool(false):` blocks (284.gd:304, 312) come from `Expression::AbsentButton` (src/godot/ai.rs:2384-2390, a debug button not in the Godot input map evaluates false). The second block would teleport Hype to (43.085, -69.546, -15.0), next to Maliq, and reset Maliq's `dsg_0`. It is dead code, not this symptom, but it is a button-absent assumption to confirm.

Other fade owners checked and ruled out (distance conditions far from Maliq or not Hype-teleporting): 742 (wp at (267, -218) and (253, -231), Hype to the kid course), 509 (wp (255, -220), level change to toras), 599 (Zatila capture, teleport to wp (-71, -53)), 1186 (Zatila), 284 normal_1 (spawn at (-71, -53) near wp 4535), Hype 28 normal_81 (save statue, fade without teleport), Hype 28 normal_57 (teleports to `dsg_170` only after 599 writes it).

Recommended fix location: the Brain activation model, not 1892 and not the fade. Implement the original's sector wake/idle rule for persos (godot/addons/astrolabe/behavior/behavior.gd:265-268 gate plus the sector activity-list wake). A previous attempt, "run only the sectors in Hype's activity list", was reverted because it stranded Hype (manoir-checklist.md:108); the rule in static-checklist-records.md:327 is the corrected one (only sectors that leave the set at a change are touched, and StandardGame fields exempt actors). It needs live evidence first.

Orrery live check (before any change): at Brigand start, read world/0/181's active byte and current normal, and Global dsg_201 and dsg_34, while Hype stands on the save statue (z = -16.68). Predicted original: 181 idle (or not in an active sector), dsg_201 stays 0.

Red-capable assertion (isolated Godot test, no race state): load Brigand from the post-save-statue state with Hype at source (56.5, -27.5, -16.68) and assert after 5 s that no `source_execute_fade` was started and Hype's source position is unchanged within 1 m. It fails today (fade starts about 0.3 s after 1892 reads z).

## Order of the first divergences

| Symptom | First divergence | Evidence |
| --- | --- | --- |
| 3 | `source_select_primary` returns false for placement 1 on an unregistered camera | source_camera.gd:271-274; 575.gd:288-294; default at source_camera.gd:14 |
| 4 | Brain 1892 (world/0/181) runs while idle in the original, so z < -16 fires the hole respawn | 1892.gd:65; fix/1.gd:417-439; recording z at 426-439 s; manoir-checklist.md:108 |

Not claimed: that the camera fix removes the teleport or the reverse. Symptom 4 depends on Hype's path crossing z < -16 (the recording does so at about (47, -26) and on the save statue), so Stephen's route may cross it before Maliq. Check that the teleport time in the port log is before the 575 trigger.
