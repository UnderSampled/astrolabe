extends SceneTree
## Focused Brigand start states for Stephen's 2026-11-04 reports: the real New
## Game session enters Brigand through LevelTransition, then this test places
## Hype near one named source actor (a debug start, not a gameplay path) and
## samples the actors' Brains, positions, the camera and the fade each 0.25 s.
## User arguments: <case> <report.json> [screenshot dir]
## Cases: wolves, bats, maliq. Each case asserts the source-expected outcome.

const BRIGAND_ENTRY := "i_brigand_dummy_gogood_hype"
const BRIGAND_CAMERA_ENTRY := "i_brigand_dummy_gogood_camera"
# Placement starts, source coordinates are exported as level.tscn world/0/N.
const CASES := {
	"wolves": {"watch": [119, 120], "near": 120, "offset": Vector3(0.0, 0.0, -12.0), "seconds": 30.0},
	"wolves_bite": {"watch": [119, 120], "near": 120, "offset": Vector3(0.0, 0.0, -12.0), "seconds": 24.0, "bite": true},
	"bats": {"watch": [45, 46, 47, 48, 49, 50, 51], "near": 51, "offset": Vector3(0.0, 0.0, 6.0), "seconds": 12.0},
	"maliq": {"watch": [20, 21, 19, 4, 6], "near": 20, "offset": Vector3(2.5, 0.0, -14.0), "seconds": 30.0, "walk": true},
}

var transition_service: Node
var completed: Array[Dictionary] = []
var failed: Array[Dictionary] = []
var report := {"samples": [], "checks": []}
var report_path := ""
var shots := ""

func _initialize() -> void:
	call_deferred("run")

func _on_completed(level: String, entry: String) -> void:
	completed.append({"level": level, "entry": entry})

func _on_failed(level: String, message: String) -> void:
	failed.append({"level": level, "message": message})

func finish(passed: bool, message: String) -> void:
	report["outcome"] = "passed" if passed else "failed"
	report["message"] = message
	if report_path != "":
		var file := FileAccess.open(report_path, FileAccess.WRITE)
		file.store_string(JSON.stringify(report, " "))
	print("BRIGAND_PROBE ", report["outcome"], " ", message)
	quit(0 if passed else 1)

func wait_for_terminal(count: int, timeout_ms: int = 90000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while completed.size() + failed.size() < count and Time.get_ticks_msec() < deadline:
		await process_frame
	return completed.size() + failed.size() >= count

func run() -> void:
	var args := OS.get_cmdline_user_args()
	var case_name := args[0] if args.size() > 0 else "wolves"
	report_path = args[1] if args.size() > 1 else ""
	shots = args[2] if args.size() > 2 else ""
	if not CASES.has(case_name):
		finish(false, "unknown case " + case_name)
		return
	var spec: Dictionary = CASES[case_name]
	report["case"] = case_name
	# As bootstrap.gd: the game plays on through documented stand-ins.
	load("res://addons/astrolabe/behavior/behavior.gd").strict_unsupported = OS.get_environment("ASTROLABE_STRICT_BEHAVIOR") == "1"
	await process_frame
	transition_service = root.get_node_or_null("LevelTransition")
	transition_service.connect("transition_completed", Callable(self, "_on_completed"))
	transition_service.connect("transition_failed", Callable(self, "_on_failed"))
	change_scene_to_packed(load("res://game/new_game.tscn"))
	if not await wait_for_terminal(1) or not failed.is_empty():
		finish(false, "manor did not load: %s" % [failed])
		return
	var session := current_scene
	transition_service.call("request_level_change", "brigand", BRIGAND_ENTRY, BRIGAND_CAMERA_ENTRY)
	if not await wait_for_terminal(2) or not failed.is_empty():
		finish(false, "brigand did not load: %s" % [failed])
		return
	var deadline := Time.get_ticks_msec() + 20000
	while not bool(session.get("ready_to_play")) and Time.get_ticks_msec() < deadline:
		await process_frame
	var level := session.get("level") as Node3D
	var hype := session.get("actor") as CharacterBody3D
	var world := level.get_node("world/0")
	# Let the arrival scripts hand Hype control (normal 4) before the debug placement.
	var hype_brain := hype.get_node("Brain")
	deadline = Time.get_ticks_msec() + 30000
	while int(hype_brain.get("current_normal")) != 4 and Time.get_ticks_msec() < deadline:
		await physics_frame
	report["arrival_normal"] = hype_brain.get("current_normal")
	var anchor := actor_of(world, spec["near"])
	var start: Vector3 = anchor.global_position + (spec["offset"] as Vector3)
	hype.global_position = start
	hype.velocity = Vector3.ZERO
	report["start"] = vec(start)
	var watch: Array = spec["watch"]
	var initial := {}
	for index in watch:
		initial[index] = actor_of(world, index).global_position
	var walk_target := anchor.global_position
	var all_brains := level.find_children("Brain", "", true, false)
	var before := {}
	for b in all_brains: before[b] = [b.get("current_normal"), b.get("current_reflex")]
	report["brain_changes"] = []
	var steps := int(float(spec["seconds"]) * Engine.physics_ticks_per_second)
	var shot_every := int(Engine.physics_ticks_per_second * 2)
	for step in steps:
		if spec.get("walk", false):
			walk_toward(hype, walk_target, session)
		# Page through any dialogue the start position opens, as a player does.
		var phase := step % 30
		if int(hype_brain.get("current_normal")) == 28 and phase == 0:
			press_hit(true)
		elif phase == 3:
			press_hit(false)
		await physics_frame
		if step < Engine.physics_ticks_per_second * 2:
			for b in all_brains:
				var now := [b.get("current_normal"), b.get("current_reflex")]
				if now != before[b]:
					report["brain_changes"].append([step, str(level.get_path_to(b)), before[b], now])
					before[b] = now
		# Second debug start: once a wolf waits in normal 0 after engaging, step
		# Hype to 3.5 m of it, as a player closing in on it would arrive.
		if spec.get("bite", false) and not report.has("bite_start") and step > Engine.physics_ticks_per_second * 9:
			for index in watch:
				var wolf := actor_of(world, index)
				if int(wolf.get_node("Brain").get("current_normal")) == 0:
					var away := hype.global_position - wolf.global_position
					away.y = 0.0
					hype.global_position = wolf.global_position + away.normalized() * 3.5 + Vector3(0, hype.global_position.y - wolf.global_position.y, 0)
					report["bite_start"] = {"t": float(step) / Engine.physics_ticks_per_second, "wolf": index}
					break
		if step % int(Engine.physics_ticks_per_second / 4) == 0:
			report["samples"].append(sample(step, session, hype, world, watch))
		if shots != "" and step % shot_every == 0:
			await save_shot("%s_%03d" % [case_name, step / shot_every])
	Input.action_release(&"Analog_Up")
	report["moved"] = {}
	for index in watch:
		report["moved"][str(index)] = snappedf(actor_of(world, index).global_position.distance_to(initial[index]), 0.01)
	finish_case(case_name)

## Source-expected outcomes of Stephen's reports, from the samples.
func finish_case(case_name: String) -> void:
	var samples: Array = report["samples"]
	var visited := {}
	var closest := {}
	for row in samples:
		for key in row["actors"]:
			var entry: Dictionary = row["actors"][key]
			if not visited.has(key): visited[key] = {}
			visited[key][int(entry.get("normal", -1))] = true
			closest[key] = minf(float(closest.get(key, 1e9)), float(entry["dist"]))
	report["visited"] = visited
	match case_name:
		"wolves":
			# 1203: detection (7) -> pursuit (1) -> waiting/circling (0, 10, 6);
			# the sidestep (12) is only for a wolf within 3 m of its tree.
			for key in ["119", "120"]:
				if not (visited[key].has(1) and (visited[key].has(10) or visited[key].has(0))):
					finish(false, "wolf %s did not pursue Hype into its attack pattern: %s" % [key, visited[key].keys()]); return
				if visited[key].has(12):
					finish(false, "wolf %s took the tree sidestep (normal 12) away from its tree" % key); return
				if float(report["moved"][key]) < 3.0 or float(closest[key]) > 8.0:
					finish(false, "wolf %s did not move to Hype: moved %s, closest %s" % [key, report["moved"][key], closest[key]]); return
		"wolves_bite":
			var lives := samples.map(func(row): return int(row.get("hype_life", -1)))
			if not (visited["119"].has(5) or visited["120"].has(5)) or lives.min() >= lives[0]:
				finish(false, "no wolf bit Hype from 3.5 m: normals %s, life %s" % [visited, lives.min()]); return
	finish(true, "source outcome observed")

func press_hit(down: bool) -> void:
	var event := InputEventAction.new()
	event.action = &"Button_Hit"
	event.pressed = down
	Input.parse_input_event(event)
	report["presses"] = int(report.get("presses", 0)) + (1 if down else 0)

func walk_toward(_hype: Node3D, _target: Vector3, _session: Node) -> void:
	pass

func sample(step: int, session: Node, hype: Node3D, world: Node, watch: Array) -> Dictionary:
	var row := {"t": snappedf(float(step) / Engine.physics_ticks_per_second, 0.01), "hype": vec(hype.global_position), "actors": {}}
	var hype_brain := hype.get_node_or_null("Brain")
	if hype_brain != null:
		row["hype_normal"] = hype_brain.get("current_normal")
		row["hype_reflex"] = hype_brain.get("current_reflex")
		row["hype_dsg_56"] = hype_brain.get("dsg_56")
	var points: Variant = hype.get("source_hit_points")
	if points != null: row["hype_life"] = points.get("current")
	for index in watch:
		var actor := actor_of(world, index)
		var entry := {"pos": vec(actor.global_position)}
		var brain := actor.get_node_or_null("Brain")
		if brain != null:
			entry["normal"] = brain.get("current_normal")
			entry["reflex"] = brain.get("current_reflex")
			entry["running"] = brain.get("running")
		for key in ["current_state", "requested_action_state", "motion_enabled", "source_point_active", "source_route_active", "source_object_active", "source_point_holds_motion", "source_route_holds_motion", "source_turned_this_tick"]:
			if key in actor: entry[key] = actor.get(key)
		if actor is CharacterBody3D: entry["vel"] = vec((actor as CharacterBody3D).velocity)
		if "root_motion_displacement" in actor: entry["rm"] = vec(actor.get("root_motion_displacement"))
		entry["dist"] = snappedf(actor.global_position.distance_to(hype.global_position), 0.01)
		row["actors"][str(index)] = entry
	var camera := root.get_viewport().get_camera_3d()
	if camera != null:
		row["camera"] = {"name": str(camera.get_path()), "pos": vec(camera.global_position)}
	return row

func actor_of(world: Node, index: int) -> Node3D:
	var holder := world.get_node(str(index))
	for child in holder.get_children():
		if child is Node3D and child.get_node_or_null("Brain") != null:
			return child
	return holder.get_child(0) as Node3D

func vec(v: Vector3) -> Array:
	return [snappedf(v.x, 0.01), snappedf(v.y, 0.01), snappedf(v.z, 0.01)]

func save_shot(name: String) -> void:
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(shots)
	image.save_png(shots.path_join(name + ".png"))
