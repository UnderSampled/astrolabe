extends SceneTree
## Closed-loop route driver through the real game. Args: <route.json> <output.json>
## Route steps: {"go": [x, y, tolerance]} steers in source axes, {"key": name,
## "hold": s}, {"wait": s}, {"until_normal": n, "timeout": s}, {"until_handoff": true},
## {"note": text} records a snapshot. tools/route_compare.py drives the original.
const KEYS := {"ArrowLeft": KEY_LEFT, "ArrowRight": KEY_RIGHT, "ArrowUp": KEY_UP, "ArrowDown": KEY_DOWN, "LeftShift": KEY_SHIFT, "LeftControl": KEY_CTRL, "X": KEY_X, "Z": KEY_Z, "Space": KEY_SPACE, "Enter": KEY_ENTER, "F5": KEY_F5}
var game: Node
var log_rows: Array = []
var started := 0

func _initialize() -> void:
	call_deferred("run")

func key(name: String, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = KEYS[name]
	event.physical_keycode = KEYS[name]
	event.pressed = pressed
	Input.parse_input_event(event)
	Input.flush_buffered_events()

func actor() -> CharacterBody3D:
	return game.get("actor")

func position() -> Vector2:
	return Vector2(actor().global_position.x, -actor().global_position.z)

func heading() -> float:
	var forward := actor().global_basis.z
	return atan2(-forward.z, forward.x)

func snapshot(note: String) -> void:
	var at := actor().global_position
	var brain: Node = game.get("brain")
	var life: Resource = actor().get("source_hit_points")
	var row := {"ms": Time.get_ticks_msec() - started, "note": note, "position": [at.x, -at.z, at.y], "action": int(actor().get("current_state")), "normal": int(brain.get("current_normal")), "running": bool(brain.get("running")), "life": int(life.get("current")) if life != null else -1, "gold": actor().get("source_inventory").get("gold")}
	log_rows.append(row)
	print("ROUTE ", JSON.stringify(row))

func go(target: Vector2, tolerance: float, timeout: float) -> bool:
	var end := Time.get_ticks_msec() + int(timeout * 1000.0)
	var held := ""
	var turn := ""
	var last := position()
	var last_check := Time.get_ticks_msec()
	while Time.get_ticks_msec() < end:
		await physics_frame
		var to := target - position()
		if to.length() < tolerance: break
		var error := wrapf(to.angle() - heading(), -PI, PI)
		var want_turn := "" if absf(error) < 0.12 else ("ArrowLeft" if error > 0.0 else "ArrowRight")
		var want_move := "ArrowUp" if absf(error) < 0.35 else ""
		if want_turn != turn:
			if not turn.is_empty(): key(turn, false)
			if not want_turn.is_empty(): key(want_turn, true)
			turn = want_turn
		if want_move != held:
			if not held.is_empty(): key(held, false)
			if not want_move.is_empty(): key(want_move, true)
			held = want_move
		if Time.get_ticks_msec() - last_check > 2500:
			if position().distance_to(last) < 0.3 and absf(error) < 0.35: break
			last = position()
			last_check = Time.get_ticks_msec()
	for name in [held, turn]:
		if not name.is_empty(): key(name, false)
	await create_timer(0.25).timeout
	return (target - position()).length() < tolerance + 0.8

## Breadth-first search over real collision. Cells keep their floor height, so
## ramps and ledges work. Returns source-axis waypoints.
func plan(target: Vector2, step_size := 1.0) -> Array:
	var space := actor().get_world_3d().direct_space_state
	var mask := 1 | 8
	var ground := func(x: float, z: float, y: float) -> Variant:
		var query := PhysicsRayQueryParameters3D.create(Vector3(x, y + 1.2, z), Vector3(x, y - 2.5, z), 1)
		var hit := space.intersect_ray(query)
		return hit["position"].y if not hit.is_empty() else null
	var open := func(a: Vector3, b: Vector3) -> bool:
		for height in [0.6, 1.4]:
			var query := PhysicsRayQueryParameters3D.create(a + Vector3.UP * height, b + Vector3.UP * height, mask)
			query.exclude = [actor().get_rid()]
			if not space.intersect_ray(query).is_empty(): return false
		return true
	var origin := actor().global_position
	var start := Vector2i(roundi(origin.x / step_size), roundi(origin.z / step_size))
	var goal := Vector2(target.x, -target.y)
	var parents := {start: start}
	var heights := {start: origin.y}
	var queue: Array[Vector2i] = [start]
	var best := start
	var best_distance := INF
	while not queue.is_empty() and parents.size() < 40000:
		var cell: Vector2i = queue.pop_front()
		var at := Vector2(cell) * step_size
		var left := at.distance_to(goal)
		if left < best_distance:
			best_distance = left
			best = cell
		if left < step_size * 1.5: break
		for offset in [Vector2i(1, 0), Vector2i(-1, 0), Vector2i(0, 1), Vector2i(0, -1), Vector2i(1, 1), Vector2i(1, -1), Vector2i(-1, 1), Vector2i(-1, -1)]:
			var next: Vector2i = cell + offset
			if parents.has(next): continue
			var there := Vector2(next) * step_size
			var floor_y: Variant = ground.call(there.x, there.y, heights[cell])
			if floor_y == null or absf(floor_y - heights[cell]) > 0.8: continue
			if not open.call(Vector3(at.x, heights[cell], at.y), Vector3(there.x, floor_y, there.y)): continue
			parents[next] = cell
			heights[next] = floor_y
			queue.append(next)
	var path: Array = []
	var cell := best
	while cell != start:
		var at := Vector2(cell) * step_size
		path.push_front([at.x, -at.y])
		cell = parents[cell]
	print("PLAN cells ", parents.size(), " left ", snappedf(best_distance, 0.1), " length ", path.size())
	var thin: Array = []
	for index in path.size():
		if index % 2 == 1 or index == path.size() - 1: thin.append(path[index])
	return thin

## Logs each change of Hype's and named actors' behavior and of the dialogue.
## An optional key is tapped every 2 seconds, to page through dialogue.
func watch(seconds: float, names: Array, press: String, fields: Array = []) -> void:
	var bodies := {}
	for body in game.get("level").find_children("*", "CharacterBody3D", true, false):
		if names.has(str(body.get_meta("source_perso_name", ""))): bodies[str(body.get_meta("source_perso_name"))] = body
	var seen := {}
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	var next_press := Time.get_ticks_msec() + 2000
	var screen: Node = game.get("level").get_node_or_null("AstrolabeUI")
	while Time.get_ticks_msec() < end:
		await physics_frame
		var row := {"hype": [int(actor().get("current_state")), int(game.get("brain").get("current_normal"))]}
		for field in fields: row[field] = game.get("brain").get(field)
		if fields.size() > 0: row["at"] = [roundi(position().x), roundi(position().y), roundi(actor().global_position.y)]
		for name in bodies:
			var brain: Node = bodies[name].get_node_or_null("Brain")
			row[name] = [int(bodies[name].get("current_state")), int(brain.get("current_normal")) if brain else -1, brain.get("dsg_0"), brain.get("dsg_1"), brain.get("dsg_2")]
		if screen != null:
			var adopted: Node = screen.get("adopted_screen")
			var active: Node = adopted if is_instance_valid(adopted) else screen
			var dialogue: Node = active.get("dialogue")
			var item: Resource = dialogue.get("item") if dialogue != null else null
			row["dialogue"] = str(item.get("identity")) if item != null else ""
		var text := JSON.stringify(row)
		if text != seen.get("last", ""):
			seen["last"] = text
			print("WATCH ", Time.get_ticks_msec() - started, " ", text)
		if not press.is_empty() and Time.get_ticks_msec() > next_press:
			next_press = Time.get_ticks_msec() + 2000
			key(press, true)
			await physics_frame
			await physics_frame
			key(press, false)

func run() -> void:
	var args := OS.get_cmdline_user_args()
	var route: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	root.size = Vector2i(1024, 768)
	var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	current_scene = menu
	await process_frame
	if route.get("start", "new") == "new":
		menu.call("open_game", "res://game/new_game.tscn", menu.get("new_game"))
	else:
		root.set_meta(load("res://game/save_state.gd").LOAD_SOURCE_META, route["start"])
		menu.call("open_game", "res://game/game_session.tscn", menu.get("load_game"))
	var deadline := Time.get_ticks_msec() + 60000
	while (current_scene == null or current_scene == menu or not bool(current_scene.get("ready_to_play"))) and Time.get_ticks_msec() < deadline:
		await process_frame
	game = current_scene
	started = Time.get_ticks_msec()
	for step: Dictionary in route["steps"]:
		var ok := true
		if step.has("plan"):
			var target := Vector2(step["plan"][0], step["plan"][1])
			ok = false
			for attempt in 8:
				var failed := false
				for point in plan(target):
					if not await go(Vector2(point[0], point[1]), 0.9, 8.0):
						failed = true
						break
				if not failed or position().distance_to(target) < 2.5:
					ok = true
					break
		elif step.has("go"):
			ok = await go(Vector2(step["go"][0], step["go"][1]), step.get("tol", step["go"][2] if step["go"].size() > 2 else 1.5), step.get("timeout", 40.0))
		elif step.has("key"):
			key(step["key"], true)
			await create_timer(step.get("hold", 0.15)).timeout
			key(step["key"], false)
		elif step.has("calibrate"):
			var before := actor().global_position
			key("ArrowUp", true)
			await create_timer(0.8).timeout
			key("ArrowUp", false)
			var moved := actor().global_position - before
			print("CALIBRATE moved ", moved.normalized(), " x ", actor().global_basis.x.normalized(), " y ", actor().global_basis.y.normalized(), " z ", actor().global_basis.z.normalized())
		elif step.has("face"):
			var to := Vector2(step["face"][0], step["face"][1]) - position()
			actor().source_turn_to_heading(atan2(to.x, -to.y))
			await physics_frame
		elif step.has("teleport"):
			var to: Array = step["teleport"]
			actor().global_position = Vector3(to[0], to[2], -to[1])
			actor().set("_contact_sector", null)
			actor().set("source_contact_sector_path", NodePath(""))
			actor().set("_contact_previous", actor().global_position)
			game.call("_locate_player_sector")
			await create_timer(0.3).timeout
		elif step.has("until_item"):
			var end := Time.get_ticks_msec() + int(step.get("timeout", 30.0) * 1000.0)
			while int(actor().get("source_inventory").call("quantity", int(step["until_item"]))) <= 0 and Time.get_ticks_msec() < end: await physics_frame
			ok = int(actor().get("source_inventory").call("quantity", int(step["until_item"]))) > 0
		elif step.has("dialog_check"):
			var ui: Node = game.get("level").get_node_or_null("AstrolabeUI")
			if is_instance_valid(ui.get("adopted_screen")): ui = ui.get("adopted_screen")
			for value in ui.get("dialogue_items"):
				if str(value.identity).to_lower() == str(step["dialog_check"]).to_lower():
					var line: Variant = value.lines[0] if value.lines.size() > 0 else null
					print("DIALOG ", value.identity, " locale ", value.locale, " box ", value.box, " voice ", value.voice, " pres ", value.presentation, " cont ", value.presentation.continuation if value.presentation else null, " lines ", value.lines.size(), " flags ", line.flags if line else null, " align ", line.alignment if line else null, " off ", line.draw_offset if line else null, " owner ", value.owner_identity, " completion ", value.completion_normal, " can ", ui.get("dialogue").call("can_run", value, 1))
		elif step.has("spell"):
			actor().get("source_magick").set("selected_spell_code", int(step["spell"]))
		elif step.has("probe2"):
			var magick: Resource = actor().get("source_magick")
			var inventory: Resource = actor().get("source_inventory")
			print("STATE spell ", magick.get("selected_spell_code"), " mana ", magick.get("current_points"), "/", magick.get("maximum_points"), " weapon ", inventory.get("selected_weapon"), " q ", inventory.get("quantities").keys().filter(func(k): return inventory.get("quantities")[k] > 0).map(func(k): return [k, inventory.get("quantities")[k]]))
		elif step.has("probe"):
			var point: Variant = game.get("brain").get("dsg_157")[0]
			var at: Vector3 = point.call("source_world_position", actor())
			var angles: Vector2 = actor().source_turn_angles_to_position(at)
			print("PROBE binding ", actor().get("source_motion_binding_present"), " wp ", at, " angles ", angles, " heading ", heading(), " rot ", actor().global_rotation.y, " basis.z ", actor().global_basis.z)
		elif step.has("zones"):
			var target: Node = null
			for body in game.get("level").find_children("*", "CharacterBody3D", true, false):
				if str(body.get_meta("source_perso_name", "")) == step["zones"]: target = body
			var area: Node = target.get_node_or_null("ZDE")
			print("ZONES forward ", target.global_basis.z, " at ", target.global_position)
			for shape in area.find_children("*", "CollisionShape3D", true, false):
				print("ZONE ", shape.get_meta("source_zone", -1), " ", shape.global_position, " r ", shape.shape.radius if shape.shape is SphereShape3D else 0.0)
			var hz: Node = actor().get_node_or_null("ZDE")
			for shape in hz.find_children("*", "CollisionShape3D", true, false):
				print("HYPEZONE ", shape.get_meta("source_zone", -1), " off ", shape.global_position - actor().global_position, " dis ", shape.disabled, " r ", shape.shape.radius if shape.shape is SphereShape3D else 0.0, " ", shape.get_parent().name)
		elif step.has("waypoints"):
			for point in game.get("brain").get(step["waypoints"]):
				if is_instance_valid(point): print("WAYPOINT ", point.call("source_world_position", actor()), " r ", point.get("radius"))
		elif step.has("watch"):
			await watch(float(step["watch"]), step.get("actors", []), step.get("press", ""), step.get("fields", []))
		elif step.has("wait"):
			await create_timer(step["wait"]).timeout
		elif step.has("until_normal"):
			var end := Time.get_ticks_msec() + int(step.get("timeout", 30.0) * 1000.0)
			while int(game.get("brain").get("current_normal")) != int(step["until_normal"]) and Time.get_ticks_msec() < end: await physics_frame
			ok = int(game.get("brain").get("current_normal")) == int(step["until_normal"])
		elif step.has("until_handoff"):
			var end := Time.get_ticks_msec() + 280000
			while not bool(game.get("opening_finished")) and Time.get_ticks_msec() < end: await physics_frame
			ok = bool(game.get("opening_finished"))
		snapshot(str(step.get("note", step)) + ("" if ok else " FAILED"))
		if step.get("census", false): _census(step.get("census"))
	var file := FileAccess.open(args[1], FileAccess.WRITE)
	file.store_string(JSON.stringify(log_rows))
	file.close()
	quit()

func _census(radius: Variant) -> void:
	var hype := actor().global_position
	for body in game.get("level").find_children("*", "CharacterBody3D", true, false):
		if not body.has_meta("source_perso") or body == actor(): continue
		var distance: float = (body as Node3D).global_position.distance_to(hype)
		var brain: Node = body.get_node_or_null("Brain")
		if brain != null and (float(radius) <= 0.0 or distance < float(radius)):
			print("CENSUS ", int(distance), " ", body.get_meta("source_perso_name", ""), " ", body.get_meta("source_perso"), " n", brain.get("current_normal"), " st", body.get("current_state"), " at ", [snappedf(body.global_position.x, 0.1), snappedf(-body.global_position.z, 0.1), snappedf(body.global_position.y, 0.1)], " ", "on" if body.get("source_object_active") else "off")
