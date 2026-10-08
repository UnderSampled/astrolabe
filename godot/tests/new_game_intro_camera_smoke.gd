extends SceneTree

## Bounded authored menu -> New Game route. This observes normal source AI,
## camera selection, and the source widescreen toggle; it never forces a camera
## or continues into Load Game.
var report: Dictionary = {
	"scope": "authored main menu -> New Game -> opening walk/first dialogue and source widescreen toggle",
	"outcome": "running",
	"load_game": "not exercised",
	"manual_camera_activation": false,
}
var evidence_path := ""
var aperture_events: Array[Dictionary] = []

func _initialize() -> void:
	call_deferred("run")

func write_report() -> void:
	if evidence_path.is_empty():
		return
	var file := FileAccess.open(evidence_path, FileAccess.WRITE)
	if file != null:
		file.store_string(JSON.stringify(report, "\t") + "\n")
		file.close()

func fail(message: String) -> void:
	report["outcome"] = "failed"
	report["failure"] = message
	write_report()
	push_error(message)
	call_deferred("quit", 1)

func require(ok: bool, message: String) -> bool:
	if ok:
		return true
	fail(message)
	return false

func click_button(button: Control) -> void:
	await process_frame
	var position := button.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = position
	motion.global_position = position
	Input.parse_input_event(motion)
	Input.flush_buffered_events()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = position
		event.global_position = position
		event.pressed = pressed
		Input.parse_input_event(event)
		Input.flush_buffered_events()
	await process_frame

func tap_hit() -> void:
	var press := InputEventKey.new()
	press.physical_keycode = KEY_SPACE
	press.pressed = true
	Input.parse_input_event(press)
	await process_frame
	var release := InputEventKey.new()
	release.physical_keycode = KEY_SPACE
	release.pressed = false
	Input.parse_input_event(release)
	await process_frame

func wait_session(previous: Node, timeout_ms: int = 70000) -> Node:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while (current_scene == null or current_scene == previous or not bool(current_scene.get("ready_to_play"))) and Time.get_ticks_msec() < deadline:
		await process_frame
	return current_scene if current_scene != null and current_scene != previous and bool(current_scene.get("ready_to_play")) else null

func screen_for(game: Node) -> Node:
	var level := game.get("level") as Node3D
	var source_ui := level.get_node_or_null("AstrolabeUI") if level != null else null
	if source_ui == null:
		return null
	var adopted := source_ui.get("adopted_screen") as Node
	return adopted if adopted != null else source_ui

func vector3_json(value: Vector3) -> Array:
	return [value.x, value.y, value.z]

func snapshot(game: Node, label: String) -> Dictionary:
	var level := game.get("level") as Node3D
	var actor := game.get("actor") as CharacterBody3D
	var source_camera := game.get("opening_camera") as Camera3D
	var camera_brain: Node = source_camera.get_parent().get_node_or_null("Brain") if source_camera != null else null
	var active_camera := root.get_camera_3d()
	var target: Node = source_camera.get("targeted_actor") as Node if source_camera != null else null
	var screen := screen_for(game)
	var letterbox := screen.get_node_or_null("Canvas/Letterbox") as Control if screen != null else null
	var hud := screen.get_node_or_null("Canvas/Hud") as Control if screen != null else null
	var player_brain := actor.get_node_or_null("Brain") if actor != null else null
	var gogoud: Node = null
	var gogoud_brain: Node = null
	if level != null:
		var paths: Dictionary = level.get_meta("source_actor_nodes", {})
		var path := str(paths.get("level:/persos/world/0/5", ""))
		if not path.is_empty():
			gogoud = level.get_node_or_null(path) as Node
			gogoud_brain = gogoud.get_node_or_null("Brain") if gogoud != null else null
	var failures: Dictionary = game.get("opening_failures") if game.get("opening_failures") is Dictionary else {}
	var stopped_brains: Array[Dictionary] = []
	var opening_brains: Array = game.get("opening_brains") if game.get("opening_brains") is Array else []
	for brain in opening_brains:
		if not is_instance_valid(brain):
			continue
		var failure := str(brain.get("unsupported_failure"))
		if failure.is_empty():
			continue
		stopped_brains.append({"path": str(brain.get_path()), "model": str(brain.get("source_model_path")),
			"normal": int(brain.get("current_normal")), "reflex": int(brain.get("current_reflex")), "failure": failure})
	return {
		"label": label,
		"observed_at_ms": Time.get_ticks_msec(),
		"game_scene": game.scene_file_path,
		"new_game": bool(game.get("new_game")),
		"loading_stage": str(game.get("loading_stage")),
		"camera_437_brain": {
			"running": bool(camera_brain.get("running")) if camera_brain != null else false,
			"current_normal": int(camera_brain.get("current_normal")) if camera_brain != null else -1,
			"current_reflex": int(camera_brain.get("current_reflex")) if camera_brain != null else -1,
			"unsupported_failure": str(camera_brain.get("unsupported_failure")) if camera_brain != null else "missing SourceCamera Brain",
		},
		"source_camera": {
			"path": str(source_camera.get_path()) if source_camera != null else "",
			"current": source_camera.current if source_camera != null else false,
			"global_position": vector3_json(source_camera.global_position) if source_camera != null else [],
			"global_forward": vector3_json(-source_camera.global_transform.basis.z.normalized()) if source_camera != null else [],
			"targeted_actor": str(target.get_path()) if is_instance_valid(target) else "",
			"active_viewport_camera": str(active_camera.get_path()) if active_camera != null else "",
		},
		"screen": {
			"present": screen != null,
			"widescreen": bool(screen.get("widescreen")) if screen != null else false,
			"letterbox_visible": letterbox.visible if letterbox != null else false,
			"hud_suppressed": bool(hud.get("suppressed")) if hud != null else false,
		},
		"player_action": int(actor.get("current_state")) if actor != null else -1,
		"player_position": vector3_json(actor.global_position) if actor != null else [],
		"player_brain": {
			"running": bool(player_brain.get("running")) if player_brain != null else false,
			"current_normal": int(player_brain.get("current_normal")) if player_brain != null else -1,
			"current_reflex": int(player_brain.get("current_reflex")) if player_brain != null else -1,
			"dsg_2": player_brain.get("dsg_2") if player_brain != null else null,
			"dsg_3": player_brain.get("dsg_3") if player_brain != null else null,
			"dsg_7": player_brain.get("dsg_7") if player_brain != null else null,
			"dsg_19": player_brain.get("dsg_19") if player_brain != null else null,
			"dsg_52": player_brain.get("dsg_52") if player_brain != null else null,
			"dsg_59": player_brain.get("dsg_59") if player_brain != null else null,
			"dsg_50": player_brain.get("dsg_50") if player_brain != null else null,
			"dsg_131": player_brain.get("dsg_131") if player_brain != null else null,
			"dsg_132": player_brain.get("dsg_132") if player_brain != null else null,
		},
		"player_brain_failure": str(player_brain.get("unsupported_failure")) if player_brain != null else "missing player Brain",
		"gogoud_action": int(gogoud.get("current_state")) if gogoud != null else -1,
		"gogoud_position": vector3_json(gogoud.global_position) if gogoud is Node3D else [],
		"gogoud_route": {"active": bool(gogoud.get("source_route_active")), "slot": int(gogoud.get("source_route_slot")),
			"slot_points": Array(gogoud.get("source_route_points")), "holds_motion": bool(gogoud.get("source_route_holds_motion")),
			"root_motion_displacement": vector3_json(gogoud.get("root_motion_displacement"))} if gogoud != null else {},
		"gogoud_model": str(gogoud_brain.get("source_model_path")) if gogoud_brain != null else "",
		"gogoud_brain": {
			"running": bool(gogoud_brain.get("running")) if gogoud_brain != null else false,
			"current_normal": int(gogoud_brain.get("current_normal")) if gogoud_brain != null else -1,
			"current_reflex": int(gogoud_brain.get("current_reflex")) if gogoud_brain != null else -1,
			"dsg_147": gogoud_brain.get("dsg_147") if gogoud_brain != null else null,
		},
		"gogoud_brain_failure": str(gogoud_brain.get("unsupported_failure")) if gogoud_brain != null else "missing Gogoud Brain",
		"opening_failures": failures,
		"stopped_brains": stopped_brains,
	}

func observe_unsupported(state: Dictionary, phase: String) -> void:
	if report.has("first_unsupported_branch"):
		return
	for field in ["camera_437_brain", "player_brain_failure", "gogoud_brain_failure"]:
		var failure := ""
		var owner: String = str(field)
		if field == "camera_437_brain":
			failure = str((state.get(field, {}) as Dictionary).get("unsupported_failure", ""))
			owner = "model437 SourceCamera Brain"
		else:
			failure = str(state.get(field, ""))
			owner = "Hype/player Brain" if field == "player_brain_failure" else "Gogoud Brain"
		if not failure.is_empty():
			report["first_unsupported_branch"] = {"phase": phase, "owner": owner, "failure": failure, "snapshot": state}
			return

func _on_widescreen_changed(enabled: bool, game: Node) -> void:
	var event := snapshot(game, "authored_toggle_widescreen")
	event["enabled"] = enabled
	aperture_events.append(event)
	report["aperture_toggle_events"] = aperture_events
	observe_unsupported(event, "authored widescreen toggle")
	write_report()

func capture(name: String) -> String:
	if DisplayServer.get_name() == "headless":
		return ""
	await process_frame
	RenderingServer.force_draw()
	var image := root.get_texture().get_image()
	var path := evidence_path.get_base_dir().path_join(name + ".png")
	if image == null or image.is_empty() or image.save_png(path) != OK:
		return ""
	return path

func run() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() != 1:
		fail("requires exactly one evidence JSON path")
		return
	evidence_path = args[0]
	report["godot_version"] = Engine.get_version_info()
	report["display_server"] = DisplayServer.get_name()
	report["headless"] = DisplayServer.get_name() == "headless"
	report["project_path"] = ProjectSettings.globalize_path("res://")
	var viewport_size := Vector2i(
		int(ProjectSettings.get_setting("display/window/size/viewport_width", 1024)),
		int(ProjectSettings.get_setting("display/window/size/viewport_height", 768))
	)
	root.size = viewport_size
	root.content_scale_size = viewport_size
	var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	current_scene = menu
	await process_frame
	var new_button := menu.find_child("NewGame", true, false) as Button
	if not require(new_button != null and not new_button.disabled, "New Game is unavailable on the actual bootstrap menu"):
		return
	await click_button(new_button)
	var game: Node = await wait_session(menu)
	if not require(game != null and bool(game.get("new_game")) and game.scene_file_path == "res://game/new_game.tscn", "actual menu New Game did not reach the authored New Game session"):
		return
	var screen := screen_for(game)
	if screen != null and screen.has_signal("widescreen_changed"):
		screen.connect("widescreen_changed", Callable(self, "_on_widescreen_changed").bind(game))
	for _tick in range(6):
		await physics_frame
	await process_frame
	var initial := snapshot(game, "startup_after_6_physics_ticks")
	report["startup"] = initial
	observe_unsupported(initial, "startup after 6 physics ticks")
	if not require(initial.get("screen", {}).get("present", false), "generated manor has no maintained source screen"):
		return
	var initial_screenshot := await capture("startup-after-6-physics-ticks")
	if not initial_screenshot.is_empty():
		report["startup_screenshot"] = initial_screenshot
	await create_timer(2.0).timeout
	var after_two_seconds := snapshot(game, "after_2_seconds")
	report["after_two_seconds"] = after_two_seconds
	observe_unsupported(after_two_seconds, "after 2 seconds")
	var two_second_screenshot := await capture("after-2-seconds")
	if not two_second_screenshot.is_empty():
		report["after_two_seconds_screenshot"] = two_second_screenshot
	var level := game.get("level") as Node3D
	var paths: Dictionary = level.get_meta("source_actor_nodes", {})
	var gogoud_path := str(paths.get("level:/persos/world/0/5", ""))
	var gogoud := level.get_node_or_null(gogoud_path) as CharacterBody3D if not gogoud_path.is_empty() else null
	if not require(gogoud != null, "generated manor lacks the authored Gogoud opening actor"):
		return
	var actions: Array[int] = [int(gogoud.get("current_state"))]
	var walk_deadline := Time.get_ticks_msec() + 300000
	while gogoud.current_state != 23 and Time.get_ticks_msec() < walk_deadline:
		await physics_frame
		await process_frame
		var state := snapshot(game, "opening_walk")
		observe_unsupported(state, "opening walk")
		if actions[-1] != gogoud.current_state:
			actions.append(gogoud.current_state)
		if not str(gogoud.get_node("Brain").get("unsupported_failure")).is_empty():
			break
	var dialogue_deadline := Time.get_ticks_msec() + 120000
	var source_ui := level.get_node_or_null("AstrolabeUI")
	while source_ui != null and bool(source_ui.call("source_dialog_is_over")) and Time.get_ticks_msec() < dialogue_deadline:
		await physics_frame
		await process_frame
		var state := snapshot(game, "opening_dialogue_wait")
		observe_unsupported(state, "opening dialogue wait")
		if not str(state.get("gogoud_brain_failure", "")).is_empty():
			break
	var dialog_owner := source_ui.get("adopted_screen") as Node if source_ui != null else null
	if dialog_owner == null:
		dialog_owner = source_ui
	var active_dialog: Resource = dialog_owner.get("dialogue").get("item") if dialog_owner != null else null
	var first_dialogue_snapshot := snapshot(game, "post_walk_first_dialogue")
	report["post_walk_first_dialogue"] = first_dialogue_snapshot
	report["gogoud_actions"] = actions
	report["first_dialogue"] = {
		"active": not bool(source_ui.call("source_dialog_is_over")) if source_ui != null else false,
		"identity": str(active_dialog.get("identity")) if active_dialog != null else "",
	}
	observe_unsupported(first_dialogue_snapshot, "post-walk/first-dialogue")
	var initial_camera: Dictionary = initial.get("source_camera", {})
	var start_position_values: Array = initial_camera.get("global_position", [])
	var start_forward_values: Array = initial_camera.get("global_forward", [])
	var followup_dialogues: Array[String] = []
	var input_count := 0
	var next_input_ms := Time.get_ticks_msec() + 700
	var followup_deadline := Time.get_ticks_msec() + 300000
	while Time.get_ticks_msec() < followup_deadline:
		var dialogue_view: Node = dialog_owner.get("dialogue") if dialog_owner != null else null
		var live_dialog: Resource = dialogue_view.get("item") if dialogue_view != null else null
		if live_dialog != null:
			var identity := str(live_dialog.get("identity"))
			if followup_dialogues.is_empty() or followup_dialogues[-1] != identity:
				followup_dialogues.append(identity)
			if Time.get_ticks_msec() >= next_input_ms:
				await tap_hit()
				input_count += 1
				next_input_ms = Time.get_ticks_msec() + 1000
		else:
			await physics_frame
			await process_frame
		var state := snapshot(game, "authored_dialogue_input_followup")
		observe_unsupported(state, "authored dialogue input follow-up")
		var pos: Array = state.get("source_camera", {}).get("global_position", [])
		var moved_from_start := false
		if start_position_values.size() == 3 and pos.size() == 3:
			moved_from_start = Vector3(float(pos[0]), float(pos[1]), float(pos[2])).distance_to(Vector3(float(start_position_values[0]), float(start_position_values[1]), float(start_position_values[2]))) > 0.01
		if bool(state.get("screen", {}).get("widescreen", false)) and moved_from_start:
			break
		if not str(state.get("gogoud_brain_failure", "")).is_empty():
			break
		if live_dialog != null and Time.get_ticks_msec() < next_input_ms:
			await physics_frame
			await process_frame
	report["dialogue_input_followup"] = {"method": "configured Button_Hit keyboard input (Space)", "select_count": input_count,
			"dialogue_identities": followup_dialogues, "deadline_ms": 300000}
	var final := snapshot(game, "post-walk-authored-camera-and-aperture-check")
	report["post_followup"] = final
	report["post_walk_first_dialogue"] = first_dialogue_snapshot
	if not report.has("first_unsupported_branch"):
		for stopped in final.get("stopped_brains", []):
			if str(stopped.get("model", "")) != "level:/ai/models/154":
				continue
			report["first_unsupported_branch"] = {
				"phase": "New Game opening world behavior",
				"owner": "level:/ai/models/154 IWorld_Manoir",
				"source_node": 3,
				"source_category": 4,
				"operation": 312,
				"operation_name": "Proc_SetGameState",
				"failure": str(stopped.get("failure", "")),
			}
			break
	if not aperture_events.is_empty():
		report["aperture_toggle"] = aperture_events[-1]
	else:
		report["aperture_toggle"] = {"observed": false, "note": "No source widescreen_changed signal was observed before the first-dialogue deadline."}
	var post_screenshot := await capture("post-walk-first-dialogue")
	if not post_screenshot.is_empty():
		report["post_walk_first_dialogue_screenshot"] = post_screenshot
	if aperture_events.size() > 0 and aperture_events[-1].get("enabled", false):
		var toggle_screenshot := await capture("authored-widescreen-toggle")
		if not toggle_screenshot.is_empty():
			report["aperture_toggle_screenshot"] = toggle_screenshot
	var camera: Dictionary = final.get("source_camera", {})
	var camera_brain: Dictionary = final.get("camera_437_brain", {})
	var screen_state: Dictionary = final.get("screen", {})
	var active_dialogue := bool(report["first_dialogue"].get("active", false))
	var expected_dialogue := str(report["first_dialogue"].get("identity", "")) == "gogood_texte1_manoir"
	var end_position_values: Array = camera.get("global_position", [])
	var end_forward_values: Array = camera.get("global_forward", [])
	var camera_position_delta := Vector3.ZERO
	var camera_forward_angle_degrees := 0.0
	if start_position_values.size() == 3 and end_position_values.size() == 3:
		var start_position := Vector3(float(start_position_values[0]), float(start_position_values[1]), float(start_position_values[2]))
		var end_position := Vector3(float(end_position_values[0]), float(end_position_values[1]), float(end_position_values[2]))
		camera_position_delta = end_position - start_position
	if start_forward_values.size() == 3 and end_forward_values.size() == 3:
		var start_forward := Vector3(float(start_forward_values[0]), float(start_forward_values[1]), float(start_forward_values[2])).normalized()
		var end_forward := Vector3(float(end_forward_values[0]), float(end_forward_values[1]), float(end_forward_values[2])).normalized()
		camera_forward_angle_degrees = rad_to_deg(start_forward.angle_to(end_forward))
	var camera_moved := camera_position_delta.length() > 0.01 or camera_forward_angle_degrees > 0.1
	var expected_walk := actions == [0, 26, 24, 23]
	var reached_aperture := aperture_events.size() > 0 or bool(screen_state.get("widescreen", false))
	var camera_ready := bool(camera_brain.get("running", false)) and str(camera_brain.get("unsupported_failure", "missing SourceCamera Brain")).is_empty() and bool(camera.get("current", false)) and not str(camera.get("targeted_actor", "")).is_empty() and str(camera.get("active_viewport_camera", "")) == str(camera.get("path", ""))
	report["acceptance"] = {
		"actual_new_game": true,
		"gogoud_opening_walk_sequence": actions,
		"reached_expected_gogoud_walk_sequence": expected_walk,
		"reached_first_dialogue": active_dialogue,
		"first_dialogue_identity_matches": expected_dialogue,
		"reached_authored_aperture_toggle": reached_aperture,
		"source_camera_437_ready_and_active": camera_ready,
		"camera_moved_during_intro": camera_moved,
		"camera_position_delta": vector3_json(camera_position_delta),
		"camera_forward_angle_degrees": camera_forward_angle_degrees,
		"screen_consistent": bool(screen_state.get("widescreen", false)) == bool(screen_state.get("letterbox_visible", false)) and bool(screen_state.get("widescreen", false)) == bool(screen_state.get("hud_suppressed", false)),
	}
	var accepted := expected_walk and active_dialog and expected_dialogue and reached_aperture and camera_ready and camera_moved and bool(report["acceptance"]["screen_consistent"])
	report["outcome"] = "passed" if accepted else "failed"
	if not accepted and not report.has("first_unsupported_branch"):
		report["first_unsupported_branch"] = {"phase": "bounded New Game intro", "owner": "route", "failure": "The first-dialogue/aperture/camera acceptance boundary was not reached; inspect snapshots and UI/camera state."}
	write_report()
	if not accepted:
		push_error("authored New Game camera/letterbox acceptance failed: " + JSON.stringify(report["acceptance"]))
		quit(1)
	else:
		quit(0)
