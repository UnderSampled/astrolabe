# Undocumented adaptations (read-only sweep, 2026-11-03)

Static sweep of the hand-written runtime (godot/addons, godot/game), the
converter (src/godot), the decoder (src/openspace) and the AI emitter
(src/godot/ai.rs, action.rs, stand_ins.rs) against docs/astrolabe/stand-ins.md.
Working copy at ee62a41 plus another agent's uncommitted edits to character.gd
and game_session.gd. Nothing was built or run, so line numbers can drift.

"Covered" means a stand-ins.md row or a concrete source citation (address,
note, record). "Comment only" means the departure is acknowledged in code but
has neither.

Overall: no special cases keyed to a model, family, actor or level name turned
up beyond the few listed below. Most magic constants carry addresses. The real
exposure is in four places:
- silently skipped AI (generic-* stand-ins and guards that fall through)
- physics replaced by a CharacterBody3D model with tuned knobs
- the death and level-exit cut-offs
- crossbow aiming

Severity: High means it changes progression or broad gameplay. Medium means a
visible or audible difference, or narrow gameplay. Low means cosmetic, an edge
case or a fallback.

## High

1. **Self behavior switches skipped.** src/godot/ai.rs:4300-4330.
   - What it does: ChangeActorBehavior aimed at the executing actor is emitted as `fail_unsupported` ("source control-status semantics are unresolved"). There are 291 sites in godot/generated, 59 of them in brigand model 1800.
   - Coverage: only the blanket `guard` row. There is no per-site id or table row.
   - Effect: in the game (non-strict), the guard records the call and continues without switching, so actors stay in their current comportement.

2. **Behavior requests with non-behavior operands dropped.** src/godot/ai.rs:1797-1815 and 4579-4600, fed by src/openspace/ai.rs:1219-1227.
   - What it does: emits `stand_in("generic-untyped")` and returns. There are about 35 sites, mostly brigand models 927 and 1060.
   - Coverage: the general "generic" sentence only.
   - Effect: those AI state machines lose transitions.

3. **29 `generic-<cat>-<op>` stand-ins have no table rows.**
   - What they do: they skip the call and read as zero.
   - Coverage: stand-ins.md describes a single `generic` id, but compiled code records ids such as:
     - `generic-3-220` ReduceIndexOfVectorArray, 18 sites
     - `generic-3-153` Func_GetNormalCollideVector, 18
     - `generic-5-25` Action_GoToWP, 9
     - `generic-1-30` Cond_CollideMovingZDM, 8
     - `generic-4-176` and `generic-4-165`, camera set position and set target, 5 and 2
     - `generic-4-217` Proc_TurnPerso and `generic-4-215` Proc_Turn2, 4 and 3
     - `generic-4-349`, `-350`, `-353`, `-356`, `-357`, the Proc_GPI_* HUD calls, 2 to 5 each
     - `generic-4-51` ReachTarget, `generic-4-53` SetHitPointsToInit, `generic-4-32` SetTarget, `generic-4-222` InitWayWithWp and `generic-1-68` IsValidWay, 2 each
     - one site each: `generic-3-42` ElapsedTime, the Reseau waypoint-network lookups (`3-134`, `3-137`, `3-141`), `generic-5-7` WaitEndOfActionPerso and `generic-4-218` ChangeAction (Ultra)
     - `generic-untyped`, 94 sites in total, covering unbound camera actors, unsupported distance operands and similar
   - Also: models 816 (the squirrel) and 1862 have whole scripts that could not be typed.
   - Effect: movement, turning and pathfinding calls do nothing on brigand-level AI, and a zero hit-point reset can make reachable logic diverge.
   - Gap in the checks: the unit test checks only table ids against the doc, never the generic-* ids.

4. **Death ends the run.** godot/game/game_session.gd:65-99.
   - What it does: `source_player_is_dead` freezes all Brains, emitters, the camera and the level, and shows "Press R to reload". The original's death and reset phase is not played.
   - Coverage: comment only ("until its level-reset lifecycle is supported").
   - Effect: every death is a hard stop.

5. **Guards fall through.** src/godot/ai.rs:343-358.
   - What it does: `continue_past_guards` turns the `return` after each `fail_unsupported` into `if strict_unsupported: return`. Execution then continues with uninitialised locals, null actors or `false`. There are about 17,000 guard sites, the largest groups being configured-part swaps (582), Brain field 56 (446), armor damage (420) and channel visibility.
   - Coverage: the `guard` row documents the policy. The per-site behavior, and AdjustPersoOnObject never running (ai.rs:4440, 10 sites), are not documented.
   - Effect: broad and unpredictable, usually a script error further down the coroutine.

6. **Level exits to unconverted levels end the game.** godot/game/level_transition.gd:~78, game_session.gd:1113.
   - What it does: shows "reached the end of the converted game" and returns to the menu.
   - Coverage: comment only.
   - Effect: progression stops at the converted edge. This is intended, but it is not a stand-in row.

## Medium

7. **Fire and every card-less generated object are frozen in place.** godot/addons/astrolabe/character/character.gd:1256-1261 (the model case).
   - What it does: `motion_enabled = current_mechanics != null`. The rule is keyed to the absence of a mechanics card, not to model 817, so it applies to every generated actor without a card.
   - Coverage: the comment was rewritten since the "with gravity it sank" version. It now argues "no card, no motion procs, zero root motion, travel is channel animation (about 0 to 2.7 m), covered by a 5.9 m ZDE box". That reasoning is plausible, but it cites no note or address, the 2.7 m and 5.9 m figures are measured or derived without a reference, and there is no stand-in row.
   - Effect: these objects never fall, slide or collide. Their contact zones stay at the spawn point.

8. **Step-down snap.** character.gd:205 and 1408.
   - What it does: `SOURCE_STEP_DOWN := 0.35` is used as `floor_snap_length` for every motion-enabled actor.
   - Coverage: justified by a symptom only ("or the fall reflex fires mid-step"). `mechanics.stairs_step_height` and `bring_down_height` exist on the card but are unused.
   - Effect: changes ground contact, `Cond_CollideWithGround`, the fall reflex, and stair and ledge behavior for Hype and every walker.

9. **Floor and wall classification tuned to symptoms.** character.gd:214-219.
   - What it does: `wall_min_slide_angle = 0` ("Godot's default stops Hype at the manoir wall end") and `floor_max_angle = acos(slope_limit)` ("Hype stands on corners at nz 0.52 to 0.58").
   - Coverage: none.
   - Effect: wall and floor contact semantics, and every AI condition built on them.

10. **Physics step is a move_and_slide substitute.** character.gd:253-300.
    - What it does: on landing it zeroes `external_velocity` ("retaining airborne horizontal velocity would slide an idle character") and slides along non-floor normals.
    - Coverage: comment only. It is described as a design choice, not as a source routine.
    - Effect: jump arcs, momentum and knockback.

11. **Collision conditions mapped to Godot body state.**
    - What it does:
      - CollideWithWall maps to `is_on_wall()` (ai.rs:2395).
      - CollideWithPerso fires when any slide collider is a CharacterBody3D (character.gd:~1403).
      - InTopOfJump is `airborne and not on_floor and v·up <= 0` (character.gd:~1157).
      - CollideWithSlope is a normal between 0 and 0.999 (character.gd:~1683). The source uses bit 0x4 of a collision word at 00533d20, and notes/jump-comparison-2026-09-24.md calls this "provisional".
    - Coverage: none.
    - Effect: AI jump, land and wall logic.

12. **Gravity fallbacks.**
    - What it does:
      - Runtime: Godot's 9.8 project gravity is used when an actor has no physical_environment (character.gd:226-228, "until ... measured").
      - Converter: the level gravity comes from the first sector's first environment. A level whose sectors differ exports none (src/godot/environment.rs:12-62).
    - Coverage: none.
    - Effect: wrong fall speed in mixed-environment levels.

13. **Crossbow aiming is invented.** game_session.gd:1004-1033 and the duplicate in crossbow_input.gd:67-83.
    - What it does:
      - The code is keyed to weapon 80.
      - Pitch moves at 45 deg/s and is clamped to ±35 deg.
      - The aim ray is 2000 units long with mask 1, cast from the screen centre.
      - When Hype is not aiming, the laser target is forced to `position + basis.z * 30` "so the bolt keeps his sighting".
      - The keys are remapped to BT_Camera_Look and the arrow keys.
    - Coverage: none for the constants. It interacts with the `generator-contact` row.
    - Effect: bolt direction and range.

14. **Dialogue and title admission.** ui/dialogue.gd:48-59 and ui/location_title.gd:28-36.
    - What it does: `can_run` rejects anything outside the admitted set, so screen.gd:~207 returns false to the AI. The admitted set is:
      - run_mode 1
      - boxes 1, 3 and 5 (plus 4 for titles)
      - no voice on box 5
      - no choice or answer modules
      - titles: at most two lines, no timing, no flags
    - Coverage: none.
    - Effect: some dialogues and titles never appear, and whatever AI waits on them depends on the script.

15. **Refused actions count as completed.** src/godot/ai.rs:2745-2765.
    - What it does: ExecuteAction and ExecuteActionAndGenerateObject finish their blocking wait when the request is rejected or `spawn_result == 0`.
    - Coverage: none, and nothing is recorded.
    - Effect: schedules advance past actions that never played.

16. **Scheduler differences.** godot/addons/astrolabe/behavior/behavior.gd.
    - What it does:
      - Same-target change_normal and change_reflex return false without restarting (355 and 395).
      - The clock is accumulated physics delta.
      - A cap of 64 behavior entries per tick calls stop().
      - Inactive actors skip advance().
      - The comment at line 157 is stale: it says rules retry next tick, which contradicts continue_past_guards.
    - Coverage: none.

17. **Action-return policy branches without a source.** src/openspace/ai.rs:1038-1075.
    - What it does: policy 1 before a meta-action is dropped. Policy 1 before a procedure other than 209 or 210 is forced to hold. Policy 0 is cited.
    - Coverage: none for the two policy-1 branches.
    - Effect: behavior sequencing timing.

18. **Camera fails closed instead of running the source paths.** godot/addons/astrolabe/camera/source_camera.gd:1180-1292, 1305-1327, 1788-1802, 1925-1928, 2001-2004 and 2069-2075.
    - What it does: these cases `push_error` and leave the pose unchanged that frame:
      - an obstructed follow ray (multi-contact Type3 slide)
      - the 0051ec70 interpolation with no candidate
      - action flags 2 and 0x40, and roll flag 0x200
      - modes outside {0, 3, 6}
      - placement words 1 and 2
      - sector index incomplete (types 1 and 11, or multi-LOD)
    - Coverage: the source routines are named, but the departure (not running them) has no row.
    - Effect: the camera freezes or lags near walls and in tight spaces.

19. **Converter drops collision zones and infers activations.** src/godot/character.rs:229-231, 354-367 and 458-470; collision.rs:73-85 and 102.
    - What it does: drops ZDE and ZDR triangle meshes, point collision and zero-radius spheres. When a profile has no activation table, a zone that is active at load is made active in every state (`fallback_active`).
    - Coverage: diagnostic only.
    - Effect: enemy hit and reaction zones may be missing or always on.

20. **Mechanics exported as inert numbers.** character.rs:1380-1450.
    - What it does: Unresolved card kinds become `mode = kind` with no values. Camera position and velocity sources exist only for modes 0 and 4, Mass, and kind 3. Only the main actor gets `motion_enabled` (scene.rs:767-775, "until their own behavior path is supported").
    - Coverage: none.
    - Effect: non-Hype actors are kinematic unless their AI moves them.

21. **Animation cues not executed.** character.rs:1356-1363.
    - What it does: mechanics, attachment_control, camera and unknown cue kinds are exported signal-only.
    - Coverage: none.

22. **Particles never emit.** character.rs:818-826, particle.rs:17-60 and particle_generator.gd:26-27.
    - What it does: `emitting = false`, only the first sprite view is used, and `enable_source_emission()` always returns false.
    - Coverage: comment only.
    - Effect: no particle effects anywhere.

23. **Lighting constants.** src/godot/light.rs:51-95.
    - What it does: `light_energy = 2.0`, `cull_mask = 2`, the spot angle is `angles[1] / 2` clamped to 0.01..89, and visibility is frozen at export. Attenuation, pulse, rotation, sector filtering and shadows are unmapped. The 2·color ambient term is backed by notes (level-rust-research.md:141), but the energy and cull mask are not.
    - Related: user_settings.gd:348-355 forces `shadow_enabled = true` on every non-negative light.
    - Coverage: none for the energy, cull mask, frozen visibility or forced shadows.

24. **Environment fallbacks.** environment.rs:56-62 and 144-150.
    - What it does: fog is disabled for the whole level when fogs differ. The ambient fallback is white at 0.7. The background colour is hardcoded to (0.08, 0.1, 0.14). Godot's smooth fog stands in for the quantized Glide table, which the code admits. Fog start and end cite 0049a460.
    - Coverage: none for the fallbacks or the mixed-fog drop.

25. **Training checkpoint hard-wired to manor actors.** godot/game/save_state.gd:20-31 and 551-731.
    - What it does: hard-wired to persos 155, 152-154, 142, 71-74 and 27, and AI models 991, 703, 939 and fix/28. It forces `current_normal` to EXERCISE_IDLE (14/15/16/22/23, uncited) and kills the key actor. Any other world state resets on load.
    - Related: save_state.gd:100-133 refuses to save unless Hype is idle.
    - Coverage: the Travel save row covers the single slot, not this partial restore.

26. **Original-save import whitelist.** src/godot/save_import.rs:68-91, 245-255 and 275-300.
    - What it does: manoir only. Five item numbers with fixed SAI ids. The spell slot maps to 61, 54 or 74. dsg_147 and dsg_149 are fixed to fix:/persos/15 and the lever to world/0/27. Health, armor and the active weapon are not restored.
    - Coverage: listed in the report's `not_restored` field, and the position is covered by `load-placement`, but there is no stand-in row.

27. **Audio laws.**
    - What it does:
      - The byte-to-amplitude law is a linear `/127` "pending the external driver's law" (audio/mixer.gd).
      - The converter gain is `volume / 127` multiplied down the resource nesting (src/godot/sound.rs:120-122 and 253). The diagnostic itself says pan, gain, random choice, reverb and release are approximations.
      - Speech is always non-positional (audio/emitter.gd:~331). Only the 0.7 gain is a row.
      - Legacy theme loop count is "a guess" (src/openspace/audio/definitions.rs:303-330).
      - Themes of 64 or more parts are flattened into one PCM loop.
    - Coverage: only the 0.7 speech gain has a row (Speech gain).

28. **Options sliders.** src/godot/menu.rs:519-577.
    - What it does: Voices range -20..107 against 0..127 for the others, every slider starts at 100, Screen is 60..100, and the Textures setting is dropped.
    - Coverage: none.

29. **Profile-locked HUD.** src/openspace/level/hud.rs:20-26 and 382-417; content.rs:~725-760.
    - What it does: English Fix sizes are asserted. On any other locale, or a decode failure, the build only warns ("Optional source HUD omitted") and the game ships without a HUD.
    - Coverage: none.

30. **Contact queries.** character.gd:1941-2050.
    - What it does: ZDE contacts are sorted before physical ones ("explicit adaptation of source list order"), and masks 4|16 are hardcoded. The sector-active filter is applied to one Cond variant, while the comments on the other say the source has no such filter.
    - Coverage: comment only.
    - Effect: multi-target hits may pick a different actor.

## Low

31. **Constants the camera sweep (item 18) flagged as uncited.** source_camera.gd:
    - the 1.8 "target head" ray offset (1095, 1235), uncited and ignores actor scale
    - the 2.0 and 0.5 speed factors (1773-1784)
    - the 0.01 distance floor (724)
    - the 0.0001 angle padding (955)
    - the 0.001 quantize step (~915)
    - ray mask 3 (1233)

32. **Item ground ray.** character.gd:1411-1417.
    - What it does: a 1.0 m ray cast from 0.25 above the origin.
    - Coverage: the `item point contact` row covers the sphere but not these numbers.

33. **Wind does nothing.** character.gd:481-482.
    - What it does: `source_add_wind` is an assert-only no-op ("pending wind parity"). It is a silent no-op, which stand-ins.md says is not allowed.

34. **Emitter no-ops and approximations.** src/godot/ai.rs.
    - `Sound{event: None}` emits `pass` (4555).
    - And/Or short-circuit, which may skip a side-effecting right operand (~1870).
    - Real-to-int truncation in Assign (2995).
    - IncrementalTurn: a negative frame count is forced to 1 and a zero count divides by 1e-6 (2646 and 2658).
    - ChangeAction busy-waits forever on a never-accepted request (4222).
    - Invented default keys for buttons (551-575).

35. **HUD count glyph offset.** ui/hud_slots.gd:44-52.
    - What it does: `COUNT_GLYPH_OFFSET (5.6, -12.6)` is labelled "Stand-in" in code, measured against frame 469.
    - Coverage: there is no stand-ins.md row.

36. **Uncited layout constants.**
    - menu projection 4.65, 3.49, 1/6, 9.3 and the ±1 px pad (src/openspace/level/menu.rs:249-253, 443, 955)
    - the credits, dialogue SCALE, inventory 640/9 and font sizes (src/godot/menu.rs, dialogue_presentation.rs, inventory.rs, bitmap_font.rs)
    - drop_shadow.gd constants: alpha 0.55, radius 1.2 and the others
    - session_camera.gd fallback camera: distance 3.34, height 2.83, fov 69.9
    - the menu confirm-sound cap of 0.5 s

37. **Other converter fallbacks.**
    - `loop_mode = fps > 0` rather than a source loop flag (character.rs:1170)
    - singular basis +0.001 (animation.rs:150-158)
    - normal fallback `[0, 0, 1]` and white vertex colour (resources.rs:41-49, sprite.rs:36)
    - only sprite modes {0, 1, 4, 5} (sprite.rs:60-72)
    - route closed when the first point equals the last (route.rs:60)
    - zero-normal boundary triangles use a computed normal (sector.rs:25-28)
    - the 0.5 colour-key threshold (material.rs)

38. **Pause and inventory.**
    - What it does: Esc and Tab are gated by the host's own availability tests, not the AI, and unknown consumables say "not implemented yet".
    - Coverage: partly the `4-334` row.

Dropped as false positives:
- character.gd:1819-1826, where `_transition_target`, `_state_next` and `_state_repeats` are stubs. src/godot/action.rs generates overrides for them in 93 family scripts.
- The source-cited constants: lift 0.6321 and 2.718, the 0054a9a0 arrival 0.25 and +0.05, camera 0.35, 1.5, 0.4 and the epsilons, the static-light formula, the 8/255 alpha test, and the dialogue timings.

## Stand-in id cross-check

Doc table ids against `src/godot/stand_ins.rs`: all 19 operation ids plus `unbound-waypoint` match in both directions.

Ids in code that are missing from stand-ins.md:
- All 29 `generic-<cat>-<op>` ids and `generic-untyped` (item 3). The doc names the id as plain `generic`, which is never recorded.
- `guard` is documented as a policy, but its about 17,000 sites carry no per-call identity.

Labeled "Stand-in" in code but no doc row:
- the HUD count glyph offset at ui/hud_slots.gd:46 (item 35)

Documented, but never recorded through `stand_in()`, so they are invisible to `Behavior.stand_ins_reached` and the gate count. They exist only as comments in hand-written code:
- `load-placement`: game/manoir_start.gd:20-21
- `generator-contact`: character.gd:125 and the unlabeled test at :1944
- `item point contact`: character.gd:1262
- every multi-level row: Speech gain (emitter.gd:20), Micro basis (game_session.gd:433), Drop shadow, Travel save (save_state.gd:34) and Kill without reset (character.gd:852)

Documented with zero compiled sites: `unbound-waypoint`, and `4-272` has one.
