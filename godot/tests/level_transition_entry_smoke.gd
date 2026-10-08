extends SceneTree
## Exercise exported named actor and camera entries in the real New Game session.
## Run against a two-map export with res://game/new_game.tscn and both maps present.

const BRIGAND_ENTRY := "i_brigand_dummy_gogood_hype"
const BRIGAND_CAMERA_ENTRY := "i_brigand_dummy_gogood_camera"

var transition_service: Node
var completed: Array[Dictionary] = []
var failed: Array[Dictionary] = []

func _initialize() -> void:
	call_deferred("run")

func check(condition: bool, message: String) -> bool:
	if condition:
		return true
	push_error("LEVEL_TRANSITION_ENTRY_FAILED: " + message)
	quit(1)
	return false

func _on_completed(level: String, entry: String) -> void:
	completed.append({"level": level, "entry": entry})

func _on_failed(level: String, message: String) -> void:
	failed.append({"level": level, "message": message})

func wait_for_terminal(count: int, timeout_ms: int = 70000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while completed.size() + failed.size() < count and Time.get_ticks_msec() < deadline:
		await process_frame
	return completed.size() + failed.size() >= count

func run() -> void:
	await process_frame
	transition_service = root.get_node_or_null("LevelTransition")
	if not check(transition_service != null, "LevelTransition autoload is missing"):
		return
	transition_service.connect("transition_completed", Callable(self, "_on_completed"))
	transition_service.connect("transition_failed", Callable(self, "_on_failed"))

	var new_game := load("res://game/new_game.tscn") as PackedScene
	if not check(new_game != null, "New Game scene is missing"):
		return
	if not check(change_scene_to_packed(new_game) == OK, "New Game session could not be created"):
		return
	if not check(await wait_for_terminal(1), "initial Manor transition did not finish"):
		return
	if not check(failed.is_empty() and completed[0]["level"] == "manoir", "New Game did not load Manor"):
		return
	var session := current_scene
	if not check(session != null and bool(session.get("ready_to_play")), "New Game session did not become playable"):
		return

	if not check(bool(transition_service.call("request_level_change", "brigand", BRIGAND_ENTRY, BRIGAND_CAMERA_ENTRY)), "positioned Brigand request was rejected"):
		return
	if not check(await wait_for_terminal(2), "Brigand transition did not finish"):
		return
	if not check(failed.is_empty() and completed[1]["level"] == "brigand" and completed[1]["entry"] == BRIGAND_ENTRY, "Brigand transition failed or lost its entry name"):
		return
	if not check(bool(session.get("ready_to_play")) and str(session.get("level").get_meta("source_level", "")) == "brigand", "Brigand did not become the active playable level"):
		return

	var level := session.get("level") as Node3D
	var actor := session.get("actor") as Node3D
	var entries: Variant = level.get_meta("source_actor_entries", {})
	if not check(entries is Dictionary, "exported actor-entry metadata is missing"):
		return
	var actor_paths: Variant = entries.get(BRIGAND_ENTRY, [])
	var camera_paths: Variant = entries.get(BRIGAND_CAMERA_ENTRY, [])
	if not check(actor_paths is Array and actor_paths.size() == 1, "source Hype entry metadata is missing or ambiguous"):
		return
	if not check(camera_paths is Array and camera_paths.size() == 1, "source camera entry metadata is missing or ambiguous"):
		return
	var actor_entry := level.get_node(actor_paths[0]) as Node3D
	var camera_entry := level.get_node(camera_paths[0]) as Node3D
	if not check(actor_entry != null and camera_entry != null, "exported entry NodePaths did not resolve"):
		return

	var actor_identity := str(level.get_meta("source_main_actor", ""))
	var actor_nodes: Variant = level.get_meta("source_actor_nodes", {})
	if not check(actor_nodes is Dictionary and actor_nodes.has(actor_identity) and level.get_node(actor_nodes[actor_identity]) == actor, "active Hype actor does not match the source main-actor metadata"):
		return
	if not check(actor.global_position.distance_to(actor_entry.global_position) < 0.1, "Hype was not placed at the source named actor entry"):
		return
	if not check(actor.global_basis.is_equal_approx(actor_entry.global_basis), "Hype did not inherit the source named actor orientation"):
		return

	var demo_camera := session.get_node("DemoCamera") as Camera3D
	if not check(demo_camera != null and bool(demo_camera.get("transition_camera_hold")), "transition camera is not holding the authored entry"):
		return
	if not check(demo_camera.global_transform.is_equal_approx(camera_entry.global_transform), "camera did not inherit the source named camera entry transform"):
		return
	print("LEVEL_TRANSITION_ENTRY_OK {level=brigand, actor_entry=%s, actor_path=%s, camera_entry=%s, camera_path=%s}" % [BRIGAND_ENTRY, actor_paths[0], BRIGAND_CAMERA_ENTRY, camera_paths[0]])
	quit(0)
