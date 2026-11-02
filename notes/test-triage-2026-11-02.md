# Test triage, 2026-11-02

One line per test. Fixed means product code changed; rewritten means the test was stale.

## Shared causes

- Exported projects lacked `game/save_file.gd`, `game/settings_menu.tscn` and `ui/hud_slots.gd` (starter and addon lists in `src/godot/support.rs`). Fixed; this alone broke every Godot-importing test below that mentions `save_file.gd`.
- Engine errors `Condition "det == 0"` (no script frame) fail the harness checks of new_game, gogoud opening walk, and the cameras tests. Source not found; the sound-listener and camera pose paths are the suspects. Open.

## Per test

- ai_behavior_actor_query manor_model_334: rewritten. Camera GoToWP and the waypoint array are typed now.
- ai_change_actor_behavior raw207 (3): rewritten. The refusal messages changed.
- ai_collision_wp manor_waypoint_array: rewritten. Waypoint 17-1/55 joined the set.
- ai_distance_source installed_id_8: rewritten. The formula moved into `NativeContactPickup.distance_xy`.
- ai_music_request exported_music_and_object_fade: open. `emitter.gd` connects `process_frame` twice (audio addon, left to the sound work).
- armor native_snapshot_helper: rewritten. Integral floats are accepted on purpose for JSON fixtures.
- audio local_audio_full_corpus: fixed. `themes.csb` declares `SetParameters(count, main loops)`; the reader now accepts it (second value taken as main loops, a guess).
- cameras exported_camera_uses_edited_horizontal_fov: fixed. Camera-only actors have no AnimationPlayer; Character no longer errors without one.
- cameras (3 engine tests) and crossbow: deadline raised to 420 s in the manifest; crossbow template path moved to `ai/models/fix/733.gd`; camera shot fixture expects Hype as the initial target; manor camera fixture binds a controller state and uses mode 1 as its unsupported mode (0, 3, 6 are implemented). All four now fail only on det == 0.
- hud_quarantine verified_hud_import: fixed. Inventory `quantity(0)` now returns the special gold, so the cash counter works; the test builds the missing-sentinel case explicitly.
- level real_rooted_model: rewritten. Material ranges are named by file, not index.
- location_title (2): rewritten. The session fixture gained VignetteHost; the End check accepts a finished fade.
- scene source_scene_rete_routes: rewritten for global Node3D actors; now stops on a doubled frame-0 walking cue (open, see below).
- save_persistence (2) and actor_world_save_state: rewritten for cfg slots, the Load picker and the settled-ground checkpoint policy.
- new_game menu test: rewritten for cfg saves, the picker, load-placement and the dialogue catalogue; still fails on the det == 0 engine errors.
- configured_bank_native: fixed. Shared configurations were never stored (ancestor walk ended at null); restored switch no longer reads a missing animation position.
- source_visual_material: fixed. Shader variants now set `source_opacity` (it read back as null).
- manor_pickup_barrel (2): fixed and rewritten. Killed generated drops leave contact queries; fixtures mark drops as generated and give Global the `dsg_22` array.
- particles: rewritten. Path is `models/fix/28.gd`.
- routes exported_gogoud_opening_walk: rewritten. Initialization after AddWind is fully typed; test now fails only on det == 0.
- godot::sound native_source_audio_attenuation, sound_program_rete_edit, native_source_fades: open. Failures are inside the audio emitter (fade targets, PCM distance curve, looped mixer output); left to the sound work. native_source_fades was not intermittent in these runs.
- openspace source_hype_opening_particle: rewritten. Operation 413 is typed (`SoundFadeInObject`).

## Open

- Scene walking cue: `select_state(32)` then `advance(26/30)` fired the frame-0 sound cue twice. Commit 2b92e13 (frame-zero cues once) targets this; the scene test has not been rerun since (about 5 minutes).
- det == 0 engine errors (above).

## After the sound and transform fixes

The `det == 0` errors came from channels keyed to zero scale and are gone. The
four sound tests and three of the previously blocked tests pass. Still failing:

- imported_gogoud_opening_ai_drives_fourth_camera_cut: camera fixture changed initial mode or target.
- exported_crossbow_free_flight_expires_and_releases_guards: cleanup did not follow the compiled lifetime counter.
- source_scene_rete_routes: its fixture lacks the game session and camera state that compiled AI now reaches.
- menu_new_game_preserves_slot_and_restarts_original_opening: smoke harness, cause not yet read.
- The seven behavior_godot BDD scenarios stay parked; tools/behavior_compare.py covers them on the real route.
