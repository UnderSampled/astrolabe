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
	# The take at 426 s: Hype at source (47, -26, -16.9), below the -16 that
	# iDummy_Brigand1_Trous (181, model 1892) tests, with no fade or warp.
	"hole": {"watch": [181, 178], "near": 181, "at": Vector3(47.0, -16.9, 26.0), "seconds": 8.0, "map_first": true},
	"wolves_bite": {"watch": [119, 120], "near": 120, "offset": Vector3(0.0, 0.0, -12.0), "seconds": 24.0, "bite": true},
	"bats": {"watch": [45, 46, 47, 48, 49, 50, 51], "near": 51, "offset": Vector3(0.0, 0.0, 6.0), "seconds": 12.0},
	# The original take's own stop at Maliq (467 s): source (40.1, -71.0, -15.0).
	"maliq": {"watch": [20, 21, 19, 4, 6, 181, 88], "near": 20, "at": Vector3(40.1, -15.0, 71.0), "seconds": 30.0, "map_first": true},
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
	if spec.get("map_first", false):
		# The take finished Gogoud's map scene (i_Gogood_Brigand_Carte, about
		# 341 s) long before Maliq; let it run and page it where Hype arrived.
		var map_brain := actor_of(world, 192).get_node("Brain")
		var tick := 0
		deadline = Time.get_ticks_msec() + 40000
		while Time.get_ticks_msec() < deadline and not (int(map_brain.get("current_normal")) == 2 and int(hype_brain.get("current_normal")) == 4):
			if int(hype_brain.get("current_normal")) == 28 and tick % 30 == 0: press_hit(true)
			elif tick % 30 == 3: press_hit(false)
			tick += 1
			await physics_frame
		report["map_done"] = [map_brain.get("current_normal"), hype_brain.get("current_normal")]
	var anchor := actor_of(world, spec["near"])
	var start: Vector3 = spec["at"] if spec.has("at") else anchor.global_position + (spec["offset"] as Vector3)
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
	report["dialogue_changes"] = []
	var screens := level.find_children("*", "", true, false).filter(func(n): return n.has_method("source_run_dialog"))
	var screen: Node = screens[0] if not screens.is_empty() else null
	var last_dialogue := ""
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
		if screen != null:
			var box: Node = screen.get("dialogue")
			var item: Variant = box.get("item") if box != null else null
			var state := "%s pending=%s normal=%s brain=%s over=%s" % [item.identity if item != null else "-", screen.get("completion_pending"), screen.get("completion_normal"), str(screen.get("completion_brain").get_path()) if is_instance_valid(screen.get("completion_brain")) else "-", box.call("is_over") if box != null else "?"]
			if state != last_dialogue:
				report["dialogue_changes"].append([step, state])
				last_dialogue = state
		if true:
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
		"hole":
			var first: Array = samples[0]["hype"]
			for row in samples:
				if Vector3(row["hype"][0], row["hype"][1], row["hype"][2]).distance_to(Vector3(first[0], first[1], first[2])) > 4.0:
					finish(false, "Hype was moved from %s to %s at %s s (181 normals %s); the take has no warp here" % [first, row["hype"], row["t"], visited.get("181", {}).keys()]); return
		"maliq":
			# 575 normal 3 selects world/0/21 (Proc141 placement 1) for the scene.
			var scene_rows := samples.filter(func(row): return int(row.get("hype_normal", -1)) in [48, 28])
			if scene_rows.is_empty():
				finish(false, "Maliq's scene did not start at the take's stop point"); return
			var on_cam := scene_rows.filter(func(row): return str(row.get("camera", {}).get("name", "")).contains("world/0/21/"))
			report["scene_rows"] = scene_rows.size()
			report["theif_cam_rows"] = on_cam.size()
			if on_cam.size() * 2 < scene_rows.size():
				finish(false, "Maliq's scene ran on %s, not I_Brigand_Camera_Theif_Cam (%d of %d samples)" % [scene_rows[0].get("camera", {}).get("name", ""), on_cam.size(), scene_rows.size()]); return
			# The original keeps Hype in place: no jump past the boars.
			var previous: Array = samples[0]["hype"]
			for row in samples:
				if Vector3(row["hype"][0], row["hype"][1], row["hype"][2]).distance_to(Vector3(previous[0], previous[1], previous[2])) > 4.0:
					finish(false, "Hype jumped from %s to %s at %s s" % [previous, row["hype"], row["t"]]); return
				previous = row["hype"]
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
	var sector: Node = hype.call("source_current_sector") if hype.has_method("source_current_sector") else null
	row["hype_sector"] = str(sector.name) + ":" + str(sector.get("sector_name")) if is_instance_valid(sector) else "-"
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
