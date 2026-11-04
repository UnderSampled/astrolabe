extends SceneTree
const StandInReport = preload("res://addons/astrolabe/behavior/stand_in_report.gd")
## Closed-loop route driver through the real game. Args: <route.json> <output.json>
## Route steps: {"go": [x, y, tolerance]} steers in source axes, {"key": name,
## "hold": s}, {"wait": s}, {"until_normal": n, "timeout": s}, {"until_handoff": true},
## {"note": text} records a snapshot. "required": true on a step ends the run (exit 3, ROUTE ABORT) if it fails. tools/route_compare.py drives the original.
const KEYS := {"ArrowLeft": KEY_LEFT, "ArrowRight": KEY_RIGHT, "ArrowUp": KEY_UP, "ArrowDown": KEY_DOWN, "LeftShift": KEY_SHIFT, "LeftControl": KEY_CTRL, "X": KEY_X, "Z": KEY_Z, "Space": KEY_SPACE, "Enter": KEY_ENTER, "F5": KEY_F5, "Tab": KEY_TAB}
var game: Node
var log_rows: Array = []
var started := 0

func _initialize() -> void:
	call_deferred("run")

## Wall time, or the fixed-step clock under Movie Maker (--write-movie), where
## frames are rendered slower or faster than real time.
func now_ms() -> int:
	if not Engine.get_write_movie_path().is_empty():
		return Engine.get_physics_frames() * 1000 / Engine.physics_ticks_per_second
	return Time.get_ticks_msec()

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


## Debug-jump helpers. A brain owner is "hype", "global" (fix:/persos/15, through
## Gogoud's brain link) or a source_perso_name.
func brain_of(name: String) -> Node:
	if name == "hype": return game.get("brain")
	if name == "global":
		var gogoud := brain_of("i_Gogood_MasterYoda")
		if gogoud == null: return null
		var linked: Node = gogoud.get("actor_brain_0")
		if not is_instance_valid(linked) and not gogoud.get("actor_brain_0_path").is_empty(): linked = gogoud.get_node_or_null(gogoud.get("actor_brain_0_path"))
		return linked
	for body in game.get("level").find_children("*", "CharacterBody3D", true, false):
		if str(body.get_meta("source_perso_name", "")) == name: return body.get_node_or_null("Brain")
	return null

func body_of(name: String) -> Node3D:
	if name == "hype": return actor()
	var brain := brain_of(name)
	return brain.get_parent() as Node3D if brain != null else null

func snapshot(note: String) -> void:
	var at := actor().global_position
	var brain: Node = game.get("brain")
	var life: Resource = actor().get("source_hit_points")
	var row := {"ms": now_ms() - started, "note": note, "yaw": snappedf(rad_to_deg(heading()), 0.1), "position": [at.x, -at.z, at.y], "action": int(actor().get("current_state")), "normal": int(brain.get("current_normal")), "running": bool(brain.get("running")), "life": int(life.get("current")) if life != null else -1, "gold": actor().get("source_inventory").get("gold")}
	var camera := get_root().get_viewport().get_camera_3d()
	if camera != null:
		row["camera"] = [camera.global_position.x, -camera.global_position.z, camera.global_position.y]
		row["camera_mode"] = camera.get("motion_mode")
	log_rows.append(row)
	print("ROUTE ", JSON.stringify(row))

func go(target: Vector2, tolerance: float, timeout: float, follow: Node3D = null) -> bool:
	var end := now_ms() + int(timeout * 1000.0)
	var held := ""
	var turn := ""
	var last := position()
	var last_check := now_ms()
	while now_ms() < end:
		await physics_frame
		# A followed actor moves: aim at where it is now.
		if follow != null and is_instance_valid(follow): target = Vector2(follow.global_position.x, -follow.global_position.z)
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
		if now_ms() - last_check > 2500:
			if position().distance_to(last) < 0.3 and absf(error) < 0.35: break
			last = position()
			last_check = now_ms()
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
	var end := now_ms() + int(seconds * 1000.0)
	var next_press := now_ms() + 2000
	var screen: Node = game.get("level").get_node_or_null("AstrolabeUI")
	while now_ms() < end:
		await physics_frame
		if not is_instance_valid(game) or not is_instance_valid(actor()): return
		var row := {"hype": [int(actor().get("current_state")), int(game.get("brain").get("current_normal"))]}
		for field in fields: row[field] = game.get("brain").get(field)
		var overseer := brain_of("global")
		if overseer != null: row["global"] = [int(overseer.get("current_normal")), overseer.get("dsg_201")]
		row["life"] = int(actor().get("source_hit_points").get("current"))
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
			print("WATCH ", now_ms() - started, " ", text)
		if not press.is_empty() and now_ms() > next_press:
			next_press = now_ms() + 2000
			key(press, true)
			await physics_frame
			await physics_frame
			key(press, false)

## ASTROLABE_PERF=1 prints frame time, object counts and the node classes that
## grow, twice a second, to find what accumulates.
func perf_monitor() -> void:
	var last_census := {}
	var tick := 0
	while true:
		await create_timer(0.5).timeout
		tick += 1
		var row := {"t": now_ms(), "fps": Engine.get_frames_per_second(),
			"process_ms": Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0,
			"physics_ms": Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0,
			"objects": Performance.get_monitor(Performance.OBJECT_COUNT),
			"nodes": Performance.get_monitor(Performance.OBJECT_NODE_COUNT),
			"orphans": Performance.get_monitor(Performance.OBJECT_ORPHAN_NODE_COUNT),
			"draw_calls": Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME),
			"primitives": Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME),
			"physics_active": Performance.get_monitor(Performance.PHYSICS_3D_ACTIVE_OBJECTS),
			"collision_pairs": Performance.get_monitor(Performance.PHYSICS_3D_COLLISION_PAIRS)}
		# Opt-in: walking the tree while a level loads on a thread breaks that load.
		if tick % 10 == 0 and OS.get_environment("ASTROLABE_PERF_CENSUS") == "1":
			var census := {}
			for node in root.find_children("*", "", true, false):
				var key: String = node.get_class() + ("/" + node.get_script().resource_path.get_file() if node.get_script() != null else "")
				census[key] = int(census.get(key, 0)) + 1
			var growth := []
			for key in census:
				var delta: int = int(census[key]) - int(last_census.get(key, 0))
				if delta != 0: growth.append([delta, key, census[key]])
			growth.sort_custom(func(a, b): return absi(a[0]) > absi(b[0]))
			row["growth"] = growth.slice(0, 8)
			last_census = census
		if tick % 4 == 0 and Engine.has_meta(&"astrolabe_perf"):
			var table: Dictionary = Engine.get_meta(&"astrolabe_perf")
			var top := []
			var total := 0
			for key in table:
				top.append([table[key], key])
				total += int(table[key])
			top.sort_custom(func(a, b): return a[0] > b[0])
			row["spent_usec_2s"] = total
			row["top"] = top.slice(0, 14)
			Engine.set_meta(&"astrolabe_perf", {})
		print("PERF ", JSON.stringify(row))

## Turns off physics processing per actor subtree for a few ticks and prints how
## much each one saves, to find what owns a slow physics step.
func blame_physics() -> void:
	var owners := {}
	for node in root.find_children("*", "", true, false):
		if not node.is_physics_processing(): continue
		var owner_node: Node = node
		while owner_node.get_parent() != null and not (owner_node is CharacterBody3D) and owner_node.get_parent() != root:
			owner_node = owner_node.get_parent()
		if not owners.has(owner_node): owners[owner_node] = []
		owners[owner_node].append(node)
	var measure := func() -> float:
		var total := 0.0
		for i in 8:
			await physics_frame
			total += Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0
		return total / 8.0
	var base: float = await measure.call()
	var rows := []
	for owner_node in owners:
		for node in owners[owner_node]: node.set_physics_process(false)
		var cost: float = await measure.call()
		for node in owners[owner_node]: if is_instance_valid(node): node.set_physics_process(true)
		rows.append([base - cost, str(owner_node.name), owners[owner_node].size()])
	rows.sort_custom(func(a, b): return a[0] > b[0])
	print("BLAME ", JSON.stringify({"base_ms": base, "groups": owners.size(), "top": rows.slice(0, 12)}))

## Takes each non-static collision object out of the physics space for a few
## ticks, to find which one makes the server's step slow.
func blame_bodies() -> void:
	var measure := func() -> float:
		var total := 0.0
		for i in 16:
			await physics_frame
			total += Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0
		return total / 16.0
	var bodies := []
	for node in root.find_children("*", "CollisionObject3D", true, false):
		if node is StaticBody3D: continue
		var row := {"path": str(node.get_path()).right(60), "class": node.get_class()}
		if node is CharacterBody3D: row["v"] = node.velocity.length(); row["floor"] = node.is_on_floor()
		if node is Area3D: row["monitoring"] = node.monitoring; row["overlaps"] = node.get_overlapping_bodies().size() + node.get_overlapping_areas().size()
		bodies.append([node, row])
	var base: float = await measure.call()
	var rows := []
	for pair in bodies:
		var node: Node = pair[0]
		if not is_instance_valid(node): continue
		var mode: Node.ProcessMode = node.process_mode
		node.process_mode = Node.PROCESS_MODE_DISABLED
		var cost: float = await measure.call()
		if is_instance_valid(node): node.process_mode = mode
		pair[1]["saves_ms"] = base - cost
		rows.append(pair[1])
	rows.sort_custom(func(a, b): return a["saves_ms"] > b["saves_ms"])
	print("BODIES ", JSON.stringify({"base_ms": base, "count": rows.size(), "top": rows.slice(0, 10)}))

## Halves the set of actors (nodes with a Brain) each round, disabling one half
## entirely, and keeps whichever half holds the slow physics step.
func bisect_actors() -> void:
	var measure := func() -> float:
		var total := 0.0
		for i in 6:
			await physics_frame
			total += Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0
		return total / 6.0
	var actors := []
	for brain in root.find_children("Brain", "", true, false):
		actors.append(brain.get_parent())
	var base: float = await measure.call()
	var set := actors
	var trail := []
	while set.size() > 1:
		var half: Array = set.slice(0, set.size() / 2)
		for node in half: node.process_mode = Node.PROCESS_MODE_DISABLED
		var cost: float = await measure.call()
		for node in half: if is_instance_valid(node): node.process_mode = Node.PROCESS_MODE_INHERIT
		trail.append([set.size(), base - cost])
		set = half if base - cost > (base - 10.0) * 0.5 else set.slice(set.size() / 2)
	var last: Node = set[0]
	last.process_mode = Node.PROCESS_MODE_DISABLED
	var alone: float = await measure.call()
	last.process_mode = Node.PROCESS_MODE_INHERIT
	var info := {"name": str(last.name), "path": str(last.get_path()).right(80), "position": [last.global_position.x, -last.global_position.z, last.global_position.y] if last is Node3D else []}
	if last is CharacterBody3D: info["velocity"] = last.velocity.length(); info["on_floor"] = last.is_on_floor(); info["slides"] = last.get_slide_collision_count()
	print("BISECT ", JSON.stringify({"base_ms": base, "actors": actors.size(), "trail": trail, "culprit": info, "saves_ms": base - alone}))

## Physics-processing nodes outside every actor, each switched off in turn.
func blame_rest() -> void:
	var actors := []
	for brain in root.find_children("Brain", "", true, false): actors.append(brain.get_parent())
	var nodes := []
	for node in root.find_children("*", "", true, false):
		if not (node.is_physics_processing() or node.is_processing()): continue
		var inside := false
		for actor in actors:
			if actor == node or actor.is_ancestor_of(node): inside = true; break
		if not inside: nodes.append(node)
	var measure := func() -> Array:
		var physics := 0.0
		var process := 0.0
		for i in 6:
			await physics_frame
			physics += Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0
			process += Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0
		return [physics / 6.0, process / 6.0]
	var base: Array = await measure.call()
	var rows := []
	for node in nodes:
		var mode: Node.ProcessMode = node.process_mode
		node.process_mode = Node.PROCESS_MODE_DISABLED
		var cost: Array = await measure.call()
		if is_instance_valid(node): node.process_mode = mode
		rows.append([base[0] - cost[0], base[1] - cost[1], str(node.get_path()).right(70), node.get_class()])
	rows.sort_custom(func(a, b): return a[0] + a[1] > b[0] + b[1])
	print("REST ", JSON.stringify({"base": base, "count": nodes.size(), "top": rows.slice(0, 12)}))

func run() -> void:
	if OS.get_environment("ASTROLABE_PERF") == "1": perf_monitor()
	var args := OS.get_cmdline_user_args()
	var route: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	root.size = Vector2i(1024, 768)
	# {"seed": n} fixes Godot's global RNG, which all game randomness uses.
	if route.has("seed"): seed(int(route["seed"]))
	var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	current_scene = menu
	await process_frame
	if route.get("start", "new") == "new":
		menu.call("open_game", "res://game/new_game.tscn", menu.get("new_game"))
	else:
		root.set_meta(load("res://game/save_state.gd").LOAD_SOURCE_META, route["start"])
		menu.call("open_game", "res://game/game_session.tscn", menu.get("load_game"))
	var opened := now_ms()
	var deadline := now_ms() + int(float(route.get("start_timeout", 60.0)) * 1000.0)
	var next_report := now_ms() + 5000
	while (current_scene == null or current_scene == menu or not bool(current_scene.get("ready_to_play"))) and now_ms() < deadline:
		if now_ms() >= next_report:
			next_report += 5000
			_report_load_progress(str(current_scene.get("level_path")) if current_scene != null and current_scene != menu else "")
		await process_frame
	game = current_scene
	started = now_ms()
	print("LOADTIME ", JSON.stringify({"start": route.get("start", "new"), "ms": started - opened}))
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
		elif step.has("follow"):
			args_out_dir = str(OS.get_cmdline_user_args()[1]).get_base_dir()
			ok = await follow(step["follow"])
		elif step.has("chase"):
			# Walk to a moving actor with the real controls: {"chase": ["iThievesCommun_Brigand_01", 1.3], "timeout": 10}
			var chased := body_of(str(step["chase"][0]))
			ok = chased != null
			if ok:
				ok = await go(Vector2(chased.global_position.x, -chased.global_position.z), float(step["chase"][1]), step.get("timeout", 10.0), chased)
				var to_chased := Vector2(chased.global_position.x, -chased.global_position.z) - position()
				actor().source_turn_to_heading(atan2(to_chased.x, -to_chased.y))
				await physics_frame
		elif step.has("magic_state"):
			# HUD magic inputs: display requests, magick and the spell handles.
			var inv: Resource = actor().get("source_inventory")
			var spells := []
			for d in inv.get("definitions"):
				if d != null and str(d.kind).begins_with("Magic"): spells.append([d.item_id, d.kind, d.selected, d.quantity])
			var mk: Resource = actor().get("source_magick")
			print("MAGIC ", JSON.stringify({"display": actor().get("source_hud_display"), "points": [mk.get("current_points"), mk.get("maximum_points"), mk.get("selected_spell_code")] if mk != null else null, "spells": spells}))
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
		elif step.has("near"):
			# {"near": ["I_Wild_Boar_A", 1.4]}: stand that far from the actor's
			# current position, facing it.
			var target_body: Node3D = null
			for body in game.get("level").find_children("*", "CharacterBody3D", true, false):
				if str(body.get_meta("source_perso_name", "")) == str(step["near"][0]): target_body = body
			ok = target_body != null
			if ok:
				var offset := (actor().global_position - target_body.global_position)
				offset.y = 0.0
				if offset.length() < 0.01: offset = Vector3.RIGHT
				actor().global_position = target_body.global_position + offset.normalized() * float(step["near"][1]) + Vector3.UP * 0.3
				actor().set("_contact_sector", null)
				actor().set("source_contact_sector_path", NodePath(""))
				actor().set("_contact_previous", actor().global_position)
				game.call("_locate_player_sector")
				await physics_frame
				var to := Vector2(target_body.global_position.x, -target_body.global_position.z) - position()
				actor().source_turn_to_heading(atan2(to.x, -to.y))
				await physics_frame
		elif step.has("until_item"):
			var end := now_ms() + int(step.get("timeout", 30.0) * 1000.0)
			while int(actor().get("source_inventory").call("quantity", int(step["until_item"]))) <= 0 and now_ms() < end: await physics_frame
			ok = int(actor().get("source_inventory").call("quantity", int(step["until_item"]))) > 0
		elif step.has("dialog_check"):
			var ui: Node = game.get("level").get_node_or_null("AstrolabeUI")
			if is_instance_valid(ui.get("adopted_screen")): ui = ui.get("adopted_screen")
			for value in ui.get("dialogue_items"):
				if str(value.identity).to_lower() == str(step["dialog_check"]).to_lower():
					var line: Variant = value.lines[0] if value.lines.size() > 0 else null
					print("DIALOG ", value.identity, " locale ", value.locale, " box ", value.box, " voice ", value.voice, " pres ", value.presentation, " cont ", value.presentation.continuation if value.presentation else null, " lines ", value.lines.size(), " flags ", line.flags if line else null, " align ", line.alignment if line else null, " off ", line.draw_offset if line else null, " owner ", value.owner_identity, " completion ", value.completion_normal, " can ", ui.get("dialogue").call("can_run", value, 1))
		elif step.has("set"):
			var target := brain_of(step["set"][0])
			ok = target != null
			if ok: target.set(step["set"][1], step["set"][2])
		elif step.has("trace56"):
			# Runs alongside the following steps: actors whose behavior changed
			# within 300 ms before Hype's dsg_56 flips.
			_trace56(float(step["trace56"]))
		elif step.has("sample"):
			# Combat probe: Hype's attack flag (dsg_17) and the target's zone 1/2
			# contacts with sword material 2, every physics frame.
			var target_body := body_of(str(step["sample"][0]))
			var target_brain := brain_of(str(step["sample"][0]))
			var hype_brain := brain_of("hype")
			var sample_end := now_ms() + int(float(step["sample"][1]) * 1000.0)
			if target_body == null: ok = false
			var brain_states := {}
			var recent := []
			var last_56: Variant = hype_brain.get("dsg_56") if hype_brain != null else null
			while ok and now_ms() < sample_end:
				await physics_frame
				# Which actors changed behavior just before Hype's dsg_56 flips.
				for brain in game.get("level").find_children("Brain", "", true, false):
					var key := str(brain.get_parent().get_meta("source_perso_name", brain.get_parent().name))
					var state := [brain.get("current_normal"), brain.get("current_reflex")]
					if brain_states.has(key) and brain_states[key] != state:
						recent.append({"ms": now_ms(), "actor": key, "from": brain_states[key], "to": state})
					brain_states[key] = state
				recent = recent.filter(func(row): return now_ms() - int(row["ms"]) < 300)
				if hype_brain != null and hype_brain.get("dsg_56") != last_56:
					print("FLIP56 ", JSON.stringify({"ms": now_ms(), "to": hype_brain.get("dsg_56"), "recent": recent}))
					last_56 = hype_brain.get("dsg_56")
				print("SAMPLE ", JSON.stringify({"ms": now_ms(), "dsg_17": hype_brain.get("dsg_17") if hype_brain != null else null, "dsg_114": hype_brain.get("dsg_114") if hype_brain != null else null, "dsg_55": hype_brain.get("dsg_55") if hype_brain != null else null, "dsg_56": hype_brain.get("dsg_56") if hype_brain != null else null, "dsg_48": hype_brain.get("dsg_48") if hype_brain != null else null, "dsg_70": hype_brain.get("dsg_70") if hype_brain != null else null, "dsg_97": hype_brain.get("dsg_97") if hype_brain != null else null, "dsg_42": hype_brain.get("dsg_42") if hype_brain != null else null, "dsg_81": hype_brain.get("dsg_81") if hype_brain != null else null, "dsg_80": hype_brain.get("dsg_80") if hype_brain != null else null, "ref0_dsg_26": _linked_field(target_brain, "dsg_26"), "nothing": target_body.call("source_collide_with_nothing") if target_body.has_method("source_collide_with_nothing") else null, "z": snappedf(target_body.global_position.y, 0.01), "floor": target_body.is_on_floor() if target_body is CharacterBody3D else null, "hype_normal": hype_brain.get("current_normal") if hype_brain != null else null,
					"zone1": target_body.call("source_collide_perso_zde_no_with_type_zde", 1, 2), "zone2": target_body.call("source_collide_perso_zde_no_with_type_zde", 2, 2),
					"normal": target_brain.get("current_normal") if target_brain != null else null, "reflex": target_brain.get("current_reflex") if target_brain != null else null, "claimed": target_brain.get("reflex_rule_claimed") if target_brain != null else null, "dsg_0": target_brain.get("dsg_0") if target_brain != null else null, "clock": int(target_brain.get("clock_milliseconds")) if target_brain != null else null, "hp": int(target_body.get("source_hit_points").get("current")) if target_body.get("source_hit_points") != null else -1, "distance": snappedf(actor().global_position.distance_to(target_body.global_position), 0.01)}))
		elif step.has("get"):
			var target := brain_of(step["get"][0])
			print("GET ", step["get"], " = ", target.get(step["get"][1]) if target != null else "no brain")
		elif step.has("normal"):
			var target := brain_of(step["normal"][0])
			ok = target != null and target.call("change_normal", int(step["normal"][1]), false)
		elif step.has("give"):
			var inventory: Resource = actor().get("source_inventory")
			var definition: Variant = inventory.call("_definition", int(step["give"][0]))
			ok = definition != null
			if ok:
				definition.quantity = int(step["give"][1])
				inventory.call("_changed")
				var index: int = inventory.get("definitions").find(definition)
				if definition.get("kind") == "Weapon":
					for slots: Array in inventory.get("categories").values():
						if slots.size() == 2 and not slots.has(index):
							var free := slots.find(-1)
							if free >= 0: slots[free] = index
							break
				inventory.call("_changed")
				print("GIVE ", step["give"], " now ", inventory.call("quantity", int(step["give"][0])))
		elif step.has("weapon"):
			print("WEAPON ", actor().get("source_inventory").call("change_weapon", int(step["weapon"])))
		elif step.has("defs"):
			for definition in actor().get("source_inventory").get("definitions"):
				print("DEF ", definition.get("item_id"), " ", definition.get("kind"), " q ", definition.get("quantity"), " max ", definition.get("maximum"), " avail ", definition.get("availability"), " ", definition.get("source_key"))
		elif step.has("until_owned"):
			var end := now_ms() + int(step.get("timeout", 60.0) * 1000.0)
			var wanted := int(step["until_owned"])
			while not actor().get("source_inventory").call("presentation_quantities").has(wanted) and now_ms() < end:
				await physics_frame
			ok = actor().get("source_inventory").call("presentation_quantities").has(wanted)
		elif step.has("object"):
			print("OBJECT ", actor().get("source_inventory").call("select_object", int(step["object"])))
		elif step.has("basis"):
			var body := body_of(step["basis"])
			print("BASIS ", step["basis"], " forward ", body.global_basis.z, " at ", body.global_position, " hype ", actor().global_position)
		elif step.has("owned"):
			var inventory: Resource = actor().get("source_inventory")
			print("OWNED ", inventory.call("presentation_quantities"), " object ", inventory.get("selected_object"), " life ", actor().get("source_hit_points").get("current"))
		elif step.has("hurt"):
			var life: Resource = actor().get("source_hit_points")
			life.set("current", int(step["hurt"]))
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
		elif step.has("save_diff"):
			_save_diff(str(step["save_diff"]))
		elif step.has("voices"):
			for emitter in root.find_children("*", "Node3D", true, false):
				if emitter.has_method("play_voice") or emitter.get("voices") != null:
					for info in emitter.get("voices"):
						if is_instance_valid(info.player) and info.player.playing:
							print("VOICE ", emitter.get_parent().name, " ", info.event, " db ", snappedf(info.player.volume_db, 0.1), " bus ", info.bus, " dist ", info.get("distance_volume", -1), " at ", snappedf(info.player.get_playback_position(), 0.1))
		elif step.has("menu_object"):
			# Through the real inventory UI: Tab opens it, the item's category is
			# chosen, its slot takes focus and Enter (ui_accept) confirms; Tab closes.
			var wanted := int(step["menu_object"])
			key("Tab", true); await process_frame; key("Tab", false)
			for i in 30: await process_frame
			var view: Control = game.get_node("InventoryHost").get("view")
			ok = view != null and view.visible
			if ok:
				var target: Resource = null
				for value in view.get("items"):
					if int(value.item_id) == wanted: target = value
				ok = target != null
				if ok:
					view.call("select_category", target.category)
					await process_frame
					var slot: Control = null
					for candidate in view.get("slot_views"):
						if candidate.get("item") == target: slot = candidate
					ok = slot != null
					if ok:
						slot.grab_focus()
						await process_frame
						var accept := InputEventAction.new()
						accept.action = &"ui_accept"
						accept.pressed = true
						Input.parse_input_event(accept)
						await process_frame
						accept.pressed = false
						Input.parse_input_event(accept)
						for i in 5: await process_frame
			print("MENU_OBJECT ", wanted, " selected_object=", actor().get("source_inventory").get("selected_object"), " ok=", ok)
			key("Tab", true); await process_frame; key("Tab", false)
			for i in 10: await process_frame
		elif step.has("zone_trace"):
			# Every physics frame, the world position of each ZDE shape of bodies
			# whose name starts with the given prefix: {"zone_trace": "Object_template", "seconds": 1.5}
			var zone_end := now_ms() + int(step.get("seconds", 1.5) * 1000.0)
			while now_ms() < zone_end:
				for body in game.get("level").find_children(str(step["zone_trace"]) + "*", "CharacterBody3D", true, false):
					var zone_area: Node = body.get_node_or_null("ZDE")
					if zone_area == null: continue
					var rows := []
					for shape in zone_area.find_children("*", "CollisionShape3D", true, false):
						var p: Vector3 = shape.global_position
						rows.append([int(shape.get_meta("source_zone", -1)), snappedf(p.x, 0.01), snappedf(-p.z, 0.01), snappedf(p.y, 0.01), snappedf(shape.shape.get("radius") if shape.shape is SphereShape3D else -1.0, 0.01), shape.disabled])
					print("ZONES ", now_ms() - started, " ", body.name, " state=", body.get("current_state"), " ", JSON.stringify(rows))
				await physics_frame
		elif step.has("act"):
			# Force an action on Hype and sample his heading: {"act": 370, "seconds": 8}
			print("ACT ", step["act"], " accepted=", actor().call("request_action", int(step["act"]), true))
			var act_end := now_ms() + int(step.get("seconds", 6.0) * 1000.0)
			while now_ms() < act_end:
				print("ACT_YAW ", now_ms() - started, " ", int(actor().get("current_state")), " ", snappedf(rad_to_deg(heading()), 0.1))
				await create_timer(0.25).timeout
		elif step.has("hp"):
			actor().get("source_hit_points").set("current", int(step["hp"]))
			print("HP ", actor().get("source_hit_points").get("current"))
		elif step.has("pickup"):
			# Admit an item as a pickup would: {"pickup": [item, count, category_key]}.
			var stock_inventory: Resource = actor().get("source_inventory")
			var stock_definition: Resource = stock_inventory.call("_definition", int(step["pickup"][0]))
			var stock: Resource = load("res://addons/astrolabe/character/inventory_stock.gd").new()
			# A third value names the category, as the item's own pickup stock does.
			stock.category_key = str(step["pickup"][2]) if step["pickup"].size() > 2 else stock_definition.pickup_category
			stock.item_key = stock_definition.source_key
			stock.count = int(step["pickup"][1])
			print("PICKUP ", step["pickup"], " -> ", stock_inventory.call("add_object", stock), " ", stock_inventory.get("last_failure"))
		elif step.has("travel"):
			# {"travel": "brigand", "entry": ""}: the level change service, as a loader would call it.
			ok = bool(root.get_node("/root/LevelTransition").call("request_level_change", str(step["travel"]), str(step.get("entry", "")), str(step.get("camera_entry", ""))))
			print("TRAVEL ", step["travel"], " requested=", ok)
		elif step.has("meters"):
			# Enemy health bar rows over time: {"meters": 6} seconds.
			var meter_end := now_ms() + int(float(step["meters"]) * 1000.0)
			var last := ""
			while now_ms() < meter_end:
				var screen_node: Node = game.get("level").get_node_or_null("AstrolabeUI")
				var hud_node: Node = screen_node.get("hud") if screen_node != null else null
				var rows := []
				for slot in 3:
					var fill: TextureRect = hud_node.get_node_or_null("Source/EnemyMeter%dFill" % slot) if hud_node != null else null
					var back: TextureRect = hud_node.get_node_or_null("Source/EnemyMeter%dBackground" % slot) if hud_node != null else null
					rows.append([back != null and back.visible, fill != null and fill.visible, snappedf(fill.size.x, 0.1) if fill != null else -1.0])
				var text := JSON.stringify(rows)
				if text != last:
					last = text
					print("METERS ", now_ms() - started, " ", text)
				await physics_frame
		elif step.has("drops"):
			# Generated objects of a family over time: {"drops": "fix:/families/23", "seconds": 6}.
			var drop_end := now_ms() + int(step.get("seconds", 6.0) * 1000.0)
			while now_ms() < drop_end:
				for body in game.get("level").find_children("*", "CharacterBody3D", true, false):
					if str(body.get_meta("source_family", "")).begins_with(str(step["drops"])):
						var p: Vector3 = body.global_position
						var b: Node = body.get_node_or_null("Brain")
						print("DROP ", now_ms() - started, " ", body.name, " ", JSON.stringify({"at": [snappedf(p.x, 0.01), snappedf(-p.z, 0.01), snappedf(p.y, 0.01)], "motion": body.get("motion_enabled"), "floor": body.is_on_floor(), "v": snappedf(body.velocity.y, 0.01), "ext": snappedf(body.get("external_velocity").y, 0.01), "visible": body.is_visible_in_tree(), "normal": b.get("current_normal") if b else -1, "dsg_0": b.get("dsg_0") if b else null, "contacts": body.get_slide_collision_count(), "layer": body.collision_layer, "mask": body.collision_mask}))
				await create_timer(0.25).timeout
		elif step.has("watch"):
			await watch(float(step["watch"]), step.get("actors", []), step.get("press", ""), step.get("fields", []))
		elif step.has("shot"):
			await process_frame
			RenderingServer.force_draw()
			var image := get_root().get_texture().get_image()
			ok = image != null and image.save_png(args[1].get_base_dir().path_join(str(step["shot"]) + ".png")) == OK
		elif step.has("wait"):
			await create_timer(step["wait"]).timeout
		elif step.has("until_level"):
			var end := now_ms() + int(step.get("timeout", 60.0) * 1000.0)
			var load_began := -1
			var travel_report := now_ms() + 5000
			while (str(game.get("level").get_meta("source_level_name", "")) != str(step["until_level"]) or not bool(game.get("ready_to_play"))) and now_ms() < end:
				if load_began < 0 and bool(game.get("loading")): load_began = now_ms()
				if load_began >= 0 and now_ms() >= travel_report:
					travel_report = now_ms() + 5000
					_report_load_progress("res://generated/hype/levels/%s/level.tscn" % step["until_level"])
				await physics_frame
			ok = str(game.get("level").get_meta("source_level_name", "")) == str(step["until_level"])
			# From the session's loading flag to play in the new level: the exit fade and the load.
			if ok and load_began >= 0: print("LOADTIME ", JSON.stringify({"level": step["until_level"], "ms": now_ms() - load_began}))
		elif step.has("save"):
			game.call("save_game")
			await process_frame
		elif step.has("until_normal"):
			var end := now_ms() + int(step.get("timeout", 30.0) * 1000.0)
			while int(game.get("brain").get("current_normal")) != int(step["until_normal"]) and now_ms() < end: await physics_frame
			ok = int(game.get("brain").get("current_normal")) == int(step["until_normal"])
		elif step.has("until_handoff"):
			var end := now_ms() + 280000
			while not bool(game.get("opening_finished")) and now_ms() < end: await physics_frame
			ok = bool(game.get("opening_finished"))
		if not is_instance_valid(game) or not is_instance_valid(actor()):
			print("ROUTE SESSION ENDED at ", step.get("note", step), " scene ", current_scene.name if current_scene != null else "none")
			quit(StandInReport.exit_code(0))
			return
		snapshot(str(step.get("note", step)) + ("" if ok else " FAILED"))
		if not ok and step.get("required", false):
			print("ROUTE ABORT ", step.get("note", step))
			quit(StandInReport.exit_code(3))
			return
		if step.get("census", false): _census(step.get("census"))
		if step.get("blame", false): await blame_physics()
		if step.get("blame_bodies", false): await blame_bodies()
		if step.get("bisect", false): await bisect_actors()
		if step.get("blame_rest", false): await blame_rest()
	var file := FileAccess.open(args[1], FileAccess.WRITE)
	file.store_string(JSON.stringify(log_rows))
	file.close()
	quit(StandInReport.exit_code(0))

## Saved designer variables that the live Brain does not hold after Load Game.
func _save_diff(path: String) -> void:
	var config := ConfigFile.new()
	config.load(path)
	var brains: Dictionary = game.get("level").get_meta("source_actor_brains", {})
	var same := 0
	var different := 0
	for section in config.get_sections():
		var marker := section.find("actor/")
		if marker < 0: continue
		var identity := section.substr(marker + 6)
		if not brains.has(identity): continue
		var brain: Node = game.get("level").get_node_or_null(brains[identity])
		for key in config.get_section_keys(section):
			if not key.begins_with("dsg_"): continue
			var saved: Variant = config.get_value(section, key)
			var live: Variant = brain.get(key)
			if typeof(saved) in [TYPE_INT, TYPE_FLOAT, TYPE_BOOL] and typeof(live) in [TYPE_INT, TYPE_FLOAT, TYPE_BOOL] and absf(float(saved) - float(live)) > 0.0001:
				different += 1
				print("DIFF ", identity, " ", str(brain.get_parent().name), " ", key, " saved ", saved, " live ", live)
			else:
				same += 1
	print("DIFFSUM same ", same, " different ", different)

func _census(radius: Variant) -> void:
	var hype := actor().global_position
	for body in game.get("level").find_children("*", "CharacterBody3D", true, false):
		if not body.has_meta("source_perso") or body == actor(): continue
		var distance: float = (body as Node3D).global_position.distance_to(hype)
		var brain: Node = body.get_node_or_null("Brain")
		if brain != null and (float(radius) <= 0.0 or distance < float(radius)):
			print("CENSUS ", int(distance), " ", body.get_meta("source_perso_name", ""), " ", body.get_meta("source_perso"), " model ", brain.get("source_model_path"), " n", brain.get("current_normal"), " st", body.get("current_state"), " at ", [snappedf(body.global_position.x, 0.1), snappedf(-body.global_position.z, 0.1), snappedf(body.global_position.y, 0.1)], " ", "on" if body.get("source_object_active") else "off")

## A stalled fraction tells a hung threaded load from a slow one.
func _report_load_progress(path: String) -> void:
	if path.is_empty(): return
	var fraction: Array = []
	var status := ResourceLoader.load_threaded_get_status(path, fraction)
	print("LOADPROGRESS ", JSON.stringify({"path": path.get_file(), "status": status, "fraction": fraction[0] if not fraction.is_empty() else -1.0, "ms": now_ms()}))

func _linked_field(brain: Node, field: String) -> Variant:
	if brain == null or not ("actor_brain_0_path" in brain): return "n/a"
	var path: NodePath = brain.get("actor_brain_0_path")
	var linked := brain.get_node_or_null(path) if not path.is_empty() else null
	return linked.get(field) if linked != null else "none"

## Closed-loop take follower (tools/take_route.py): steer along the take's
## position track with the real controls, fire its action keys on the take's
## clock, and re-anchor that clock when a level finishes loading.
var args_out_dir := ""
func follow(plan: Dictionary) -> bool:
	var track: Array = plan["track"]
	var keys: Array = plan["keys"]
	var landmarks: Array = plan["landmarks"]
	var until: float = float(plan["until"])
	# Port time zero is New Game; the take reached the Manoir at take_zero.
	var shift: float = float(plan["take_zero"]) + 1.0
	var origin := now_ms()
	var next_key := 0
	var cursor := 0
	var held := {}
	var level := str(game.get("level").get_meta("source_level_name", "")) if game.get("level") != null else ""
	var loading := false
	var load_began := 0.0
	var next_report := 0.0
	var clock := 0.0
	# {"from": t} starts mid-take (after a load or an earlier follow step):
	# the clock begins at take time t and earlier keys are dropped.
	if plan.has("from"):
		shift = float(plan["from"])
		while next_key < keys.size() and float(keys[next_key][0]) < shift: next_key += 1
		while cursor + 1 < track.size() and float(track[cursor + 1][0]) <= shift: cursor += 1
	while clock < until:
		await physics_frame
		var now := (now_ms() - origin) / 1000.0
		if bool(game.get("loading")) or game.get("level") == null:
			if not loading:
				loading = true
				load_began = now
				for code in held.keys(): _follow_key(code, false)
				held.clear()
			continue
		if loading:
			loading = false
			var arrived := str(game.get("level").get_meta("source_level_name", ""))
			if arrived != level:
				level = arrived
				# The take's first playable frame in this level is the anchor.
				for mark in landmarks:
					if str(mark.get("level", "")) == level and int(mark.get("phase", -1)) == 9:
						var before := clock
						shift = float(mark["t"]) - now
						print("LANDMARK ", JSON.stringify({"level": level, "port": snappedf(now, 0.1), "take": mark["t"], "drift": snappedf(before - float(mark["t"]), 0.1), "load_s": snappedf(now - load_began, 0.1)}))
						break
				if level == "toras" or level.is_empty(): break
		clock = now + shift
		# Action keys at their take times. An action press waits (holding the
		# take clock, up to 12 s) until Hype is on the take's track, so attacks
		# and casts land where the original's did.
		while next_key < keys.size() and float(keys[next_key][0]) <= clock:
			var row: Array = keys[next_key]
			# Modal keys wait for the port to be in the take's state (dialogue,
			# inventory, menu): dialogue and load lengths differ slightly.
			var wanted := _take_mode(track, landmarks, float(row[0]))
			# Stand-in: the port has no save-point menu (the original names the
			# save). Its quick save replaces the take's menu keys, once per menu.
			if wanted == "menu" and _port_mode() != "menu":
				if not _follow_menu_saved:
					_follow_menu_saved = true
					print("SAVE ", JSON.stringify({"take": float(row[0]), "port": "quick save (save-point menu not ported)"}))
					_follow_key(KEY_F5, true)
					_follow_key(KEY_F5, false)
				next_key += 1
				continue
			if wanted != "menu": _follow_menu_saved = false
			# The reverse case: the take plays on, but the port still shows a
			# dialogue; its Space waits while the port's pages are advanced.
			if bool(row[2]) and int(row[1]) == KEY_SPACE and wanted == "play" and _port_mode() == "dialogue" and _follow_hold < 6.0:
				_follow_hold += 1.0 / Engine.physics_ticks_per_second
				shift -= 1.0 / Engine.physics_ticks_per_second
				break
			if bool(row[2]) and wanted != "play" and _port_mode() != wanted and _follow_hold < 6.0:
				_follow_hold += 1.0 / Engine.physics_ticks_per_second
				shift -= 1.0 / Engine.physics_ticks_per_second
				break
			# Where the take stood still to act (casts, swings, shots), the zones
			# leave centimetres of margin: reach its spot and facing first.
			if bool(row[2]) and int(row[1]) in ACTION_KEYS and wanted == "play" and _port_mode() == "play" and _follow_hold < 6.0:
				var spot := _track_spot(track, float(row[0]))
				if not spot.is_empty():
					_follow_spot = spot
					var off := Vector2(spot["point"]).distance_to(position())
					var turn := absf(wrapf((Vector2(spot["aim"]) - position()).angle() - heading(), -PI, PI))
					if off > 0.5 or turn > 0.05:
						_follow_hold += 1.0 / Engine.physics_ticks_per_second
						shift -= 1.0 / Engine.physics_ticks_per_second
						break
			if bool(row[2]) and int(row[1]) in ACTION_KEYS and _follow_spot.is_empty() and (_follow_gap > 1.0 or (_follow_facing_off and wanted == "play")) and _follow_hold < 12.0:
				# An action waits for Hype to reach and face the take's spot;
				# facing makes him settle to a stand and turn (aimed casts).
				_follow_aiming = _follow_facing_off and wanted == "play"
				_follow_hold += 1.0 / Engine.physics_ticks_per_second
				shift -= 1.0 / Engine.physics_ticks_per_second
				break
			# Hype's movement rules precede his action rules, so an action
			# pressed with an arrow held is lost. Release the arrows for one
			# frame, then press with steering held off on that frame too.
			if bool(row[2]) and int(row[1]) in ACTION_KEYS and wanted == "play":
				var steering := false
				for arrow in ["ArrowUp", "ArrowLeft", "ArrowRight"]: steering = steering or bool(_steer_state.get(arrow, false))
				_follow_quiet_frames = 2 if steering else 1
				if steering:
					for arrow in ["ArrowUp", "ArrowLeft", "ArrowRight"]: _follow_steer(arrow, false)
					shift -= 1.0 / Engine.physics_ticks_per_second
					break
			if _follow_hold > 0.0 and not _follow_spot.is_empty():
				print("SPOT ", JSON.stringify({"take": float(row[0]), "held": snappedf(_follow_hold, 0.01), "off": snappedf(Vector2(_follow_spot["point"]).distance_to(position()), 0.01), "turn": snappedf(wrapf(float(_follow_spot["facing"]) - heading(), -PI, PI), 0.01)}))
			_follow_hold = 0.0
			_follow_aiming = false
			_follow_spot = {}
			# Inventory choices go by item identity: the take's cursor moves are
			# dropped with its arrows, so its Enter becomes a request for the item.
			if bool(row[2]) and int(row[1]) == KEY_ENTER and _port_mode() == "inventory" and _follow_select(plan.get("select", []), float(row[0])):
				next_key += 1
				continue
			if int(row[1]) == KEY_SHIFT:
				_follow_take_shift = bool(row[2])
				next_key += 1
				continue
			# Route annotation: a press the take needed but the port no longer
			# does (the stage already counted), as a player who saw it would.
			if _follow_skip(plan.get("skip_if", []), row):
				next_key += 1
				continue
			_follow_key(int(row[1]), bool(row[2]))
			if bool(row[2]) and (int(row[1]) in ACTION_KEYS or int(row[1]) in [KEY_TAB, KEY_ESCAPE]):
				var inv: Resource = actor().get("source_inventory")
				print("KEY ", JSON.stringify({"take": float(row[0]), "port_clock": snappedf(clock, 0.01), "code": int(row[1]), "gap": snappedf(_follow_gap, 0.1), "mode": _port_mode(), "heading": snappedf(heading(), 0.01), "take_facing": snappedf(_follow_facing, 0.01), "pos": [snappedf(position().x, 0.1), snappedf(position().y, 0.1)], "normal": brain_of("hype").get("current_normal"), "object": inv.call("selected_object_value") if inv != null else null}))
			if bool(row[2]): held[int(row[1])] = true
			else: held.erase(int(row[1]))
			next_key += 1
		for code in _follow_key_release.keys():
			_follow_key_release[code] -= 1.0 / Engine.physics_ticks_per_second
			if _follow_key_release[code] <= 0.0:
				_follow_key(int(code), false)
				_follow_key_release.erase(code)
		var watch := []
		for watched in ["i_Gogood_MasterYoda", "i_Mannequin_Epee", "i_Mannequin_Arbalete", "i_Mannequin_Magie"]:
			var watched_brain := brain_of(watched)
			watch.append([watched_brain.get("dsg_0"), watched_brain.get("dsg_1"), watched_brain.get("dsg_2"), watched_brain.get("current_normal")] if watched_brain != null else null)
		_trace_bolts(clock)
		var watch_screen: Node = brain_of("hype").call("source_ui") if brain_of("hype") != null else null
		var watch_box: Node = watch_screen.get("dialogue") if watch_screen != null else null
		var watch_item: Resource = watch_box.get("item") if watch_box != null and watch_box.visible else null
		watch.append([watch_item.resource_path.get_file() if watch_item != null else "", watch_box.get("page_start") if watch_item != null else -1, watch_box.get("mode") if watch_item != null else -1])
		var watch_hype := brain_of("hype")
		watch.append([watch_hype.get("dsg_17"), watch_hype.get("dsg_55"), watch_hype.get("dsg_58"), watch_hype.get("dsg_0"), watch_hype.get("dsg_70"), watch_hype.get("dsg_97"), _held(actor().get("source_inventory"))])
		if watch != _watch:
			print("WATCH ", JSON.stringify({"take": snappedf(clock, 0.01), "gogoud": watch[0], "epee": watch[1], "arbalete": watch[2], "magie": watch[3], "dialogue_page_mode": watch[4], "hype_17_55_58_0_70_97_held": watch[5]}))
			_watch = watch
		# A dialogue the take has already left is advanced with Space.
		if _port_mode() == "dialogue" and _take_mode(track, landmarks, clock) == "play":
			_follow_linger += 1.0 / Engine.physics_ticks_per_second
			if _follow_linger > 0.35:
				_follow_linger = 0.0
				_follow_key(KEY_SPACE, true)
				_follow_space_release = 0.1
		if _follow_space_release > 0.0:
			_follow_space_release -= 1.0 / Engine.physics_ticks_per_second
			if _follow_space_release <= 0.0: _follow_key(KEY_SPACE, false)
		else:
			_follow_linger = 0.0
		# Path pursuit: follow the take's track by proximity (so a lagging Hype
		# goes round corners instead of cutting them), never past its clock.
		var here := position()
		var horizon := clock + 0.5
		# The proximity cursor only moves forward. It is not forced to the
		# clock: a Hype held back (a longer port dialogue) retraces the take's
		# path instead of cutting across to where the take is now.
		var best := cursor
		var best_distance := INF
		var probe := cursor
		while probe < track.size() and probe < cursor + 20 and float(track[probe][0]) <= horizon:
			if str(track[probe][5]) == level:
				var d := _track_point(track[probe]).distance_to(here)
				if d <= best_distance:
					best_distance = d
					best = probe
			probe += 1
		cursor = best
		# Facing comes from the take's clock, not the proximity cursor, which
		# stays on the first sample of a spot where the take stood and turned.
		var facing_row := cursor
		while facing_row + 1 < track.size() and float(track[facing_row + 1][0]) <= clock: facing_row += 1
		_follow_facing = float(track[facing_row][6]) if track[facing_row].size() > 6 else NAN
		# Pure pursuit by arc length: walk 1.2 m along the take's path from the
		# closest sample (never across a bend), bounded by its clock, plus a
		# second when Hype is already at the clock's point (walk on, not stop).
		var ahead := cursor
		var travelled := 0.0
		var limit := horizon + (1.0 if _track_point(track[cursor]).distance_to(here) < 1.0 else 0.0)
		while ahead + 1 < track.size() and float(track[ahead + 1][0]) <= limit and str(track[ahead + 1][5]) == level and travelled < (2.4 if _gait == "run" else 1.2):
			travelled += _track_point(track[ahead]).distance_to(_track_point(track[ahead + 1]))
			ahead += 1
		var sample: Array = track[ahead]
		var playable := str(sample[5]) == level
		var target := _track_point(sample)
		# The take already left this level: head for its last point here (the
		# exit) and hold the take clock until this level is left too.
		if not playable and _track_level_ahead(track, ahead, level) < 0:
			var last := _track_last_in(track, level)
			if last >= 0:
				sample = track[last]
				target = _track_point(sample)
				playable = true
				shift -= 1.0 / Engine.physics_ticks_per_second
		if not _follow_spot.is_empty():
			target = _follow_spot["point"]
			playable = true
		var to := target - position()
		var error := wrapf(to.angle() - heading(), -PI, PI)
		# Gait ladder (run, walk, stand): follow the take's speed, run to catch
		# up, walk rather than stop when ahead of its clock, and stand only
		# where the take stands and only from a walk, so Hype never runs
		# straight into a stop or stutters.
		var take_speed := _track_moved(track, cursor, clock)
		var dt := 1.0 / Engine.physics_ticks_per_second
		_follow_gap = to.length() if playable else 0.0
		var want := "stand"
		if playable:
			# Standing starts again only for a clear reason (no stand-walk flapping).
			# Stand only on the take's spot (walking closes the last metre).
			var stay := _gait == "stand" and take_speed < 0.8 and to.length() < 0.8
			if stay or ((take_speed < 0.3 or _follow_aiming) and to.length() < 0.4): want = "stand"
			elif to.length() > 2.0 or take_speed > 3.5: want = "run"
			else: want = "walk"
		if want == "stand" and _gait == "run": want = "walk"
		if want == "stand" and _gait == "walk" and _gait_time < 0.4: want = "walk"
		if not playable: want = "stand"
		# Spot approach: turn toward the spot standing, walk in, then stand.
		if not _follow_spot.is_empty():
			# Walking from a stand moves about half a metre, so position is
			# taken to 0.5 m on the way in and only the facing is fine-tuned.
			want = "walk" if to.length() > 0.5 or (_gait == "walk" and to.length() > 0.35) else "stand"
			if want == "stand" and _gait == "run": want = "walk"
		if want != _gait:
			_gait = want
			_gait_time = 0.0
		_gait_time += dt
		var turning_hard := absf(error) > 1.2
		var quiet := _follow_quiet_frames > 0
		if quiet: _follow_quiet_frames -= 1
		_follow_steer("ArrowUp", _gait != "stand" and not turning_hard and not quiet)
		var run := _gait == "run"
		if run != bool(_steer_state.get("run", false)):
			_steer_state["run"] = run
			_follow_key(KEY_SHIFT, run)
		# Steering with hysteresis; hold the heading when the target is close
		# (Hype is ahead of the take), so he walks on instead of circling.
		var left := bool(_steer_state.get("ArrowLeft", false))
		var right := bool(_steer_state.get("ArrowRight", false))
		var facing_error := 0.0 if is_nan(_follow_facing) else wrapf(_follow_facing - heading(), -PI, PI)
		# Moving, an action only waits when Hype faces well away from the take.
		# Where the take moved slowly, an action also waits for Hype to reach it.
		_follow_facing_off = absf(facing_error) > 0.35 or (take_speed < 1.0 and to.length() > 0.5)
		if _gait == "stand" and not _follow_spot.is_empty():
			# At or turning toward the action spot: fine turn taps.
			var aim := wrapf(((Vector2(_follow_spot["aim"]) - position()).angle() if to.length() <= 0.5 else to.angle()) - heading(), -PI, PI)
			left = aim > 0.06 or (left and aim > 0.02)
			right = aim < -0.06 or (right and aim < -0.02)
		elif _gait == "stand":
			# Standing: face where the take faced (aimed shots and talks).
			_follow_facing_off = absf(facing_error) > 0.1 or to.length() > 0.5
			left = facing_error > 0.15 or (left and facing_error > 0.05)
			right = facing_error < -0.15 or (right and facing_error < -0.05)
		else:
			# Moving: wider band and a minimum time between turn-key changes,
			# so Hype holds a line instead of weaving.
			var on := 0.3 if _gait == "run" else 0.22
			left = error > on or (left and error > 0.06)
			right = error < -on or (right and error < -0.06)
		var changed := left != bool(_steer_state.get("ArrowLeft", false)) or right != bool(_steer_state.get("ArrowRight", false))
		_turn_since += dt
		if changed and _gait != "stand" and _turn_since < 0.2 and absf(error) < 0.8:
			left = bool(_steer_state.get("ArrowLeft", false))
			right = bool(_steer_state.get("ArrowRight", false))
			changed = false
		if quiet:
			changed = changed or left or right
			left = false
			right = false
		if changed:
			_turn_since = 0.0
			_follow_metrics["turn_toggles"] += 1
		_follow_steer("ArrowLeft", left)
		_follow_steer("ArrowRight", right)
		_follow_measure(here, take_speed, track, cursor, playable, dt)
		if int(_measure_clock) / 60 != int(_measure_clock - dt) / 60: _follow_report_metrics()
		var hype_brain := brain_of("hype")
		if hype_brain != null and int(hype_brain.get("current_normal")) != _follow_last_normal:
			print("NORMAL ", JSON.stringify({"take": snappedf(clock, 0.01), "from": _follow_last_normal, "to": int(hype_brain.get("current_normal")), "dsg_56": hype_brain.get("dsg_56"), "action": actor().get("current_state"), "pos": [snappedf(position().x, 0.1), snappedf(position().y, 0.1)]}))
			_follow_last_normal = int(hype_brain.get("current_normal"))
		var shots: Array = plan.get("shots", [])
		if not shots.is_empty() and clock >= float(shots[0]):
			RenderingServer.force_draw()
			get_root().get_texture().get_image().save_png(args_out_dir.path_join("follow_%d.png" % int(float(shots[0]))))
			shots.pop_front()
		if clock >= next_report:
			next_report = clock + float(plan.get("report", 5.0))
			print("FOLLOW ", JSON.stringify({"take": snappedf(clock, 0.1), "level": level, "gap": snappedf(to.length(), 0.1), "pos": [snappedf(position().x, 0.1), snappedf(position().y, 0.1)], "target": [snappedf(target.x, 0.1), snappedf(target.y, 0.1)], "err": snappedf(rad_to_deg(error), 1), "face_err": snappedf(rad_to_deg(wrapf(float(sample[6]) - heading(), -PI, PI)), 1) if sample.size() > 6 else null, "state": actor().get("current_state"), "action": actor().get("requested_action_state"), "complete": actor().get("action_complete"), "normal": brain_of("hype").get("current_normal") if brain_of("hype") != null else -1, "take_normal": sample[4], "life": int(actor().get("source_hit_points").get("current")) if actor().get("source_hit_points") != null else -1}))
	_follow_report_metrics()
	for code in held.keys(): _follow_key(code, false)
	_follow_key(KEY_SHIFT, false)
	for name in ["ArrowUp", "ArrowLeft", "ArrowRight"]: _follow_steer(name, false)
	return true

func _track_moved(track: Array, cursor: int, clock: float) -> float:
	var lo := cursor
	while lo > 0 and float(track[lo][0]) > clock - 0.5: lo -= 1
	var hi := cursor
	while hi + 1 < track.size() and float(track[hi][0]) < clock + 0.5: hi += 1
	return _track_point(track[lo]).distance_to(_track_point(track[hi]))

func _track_level_ahead(track: Array, from: int, level: String) -> int:
	for i in range(from, track.size()):
		if str(track[i][5]) == level: return i
	return -1

var _last_in_cache := {}
func _track_last_in(track: Array, level: String) -> int:
	if not _last_in_cache.has(level):
		var found := -1
		for i in track.size():
			if str(track[i][5]) == level: found = i
		_last_in_cache[level] = found
	return int(_last_in_cache[level])

## The take's position and facing at time t, interpolated between samples.
func _track_at(track: Array, t: float) -> Dictionary:
	var i := 0
	while i + 1 < track.size() and float(track[i + 1][0]) <= t: i += 1
	var j := mini(i + 1, track.size() - 1)
	var span := float(track[j][0]) - float(track[i][0])
	var w := clampf((t - float(track[i][0])) / span, 0.0, 1.0) if span > 0.0 else 0.0
	var facing := float(track[i][6]) + wrapf(float(track[j][6]) - float(track[i][6]), -PI, PI) * w if track[i].size() > 6 else NAN
	return {"point": _track_point(track[i]).lerp(_track_point(track[j]), w), "facing": facing}

## The take's spot for an action at t: its last sample before the press, if the
## take was idle there. Samples are 2 Hz and the action's
## own lunge or recoil lands in the next one, so only the sample before counts.
func _track_spot(track: Array, t: float) -> Dictionary:
	var i := -1
	while i + 1 < track.size() and float(track[i + 1][0]) <= t: i += 1
	if i < 0 or track[i].size() < 7 or int(track[i][4]) != 4: return {}
	# Aim through the point the take aimed at (3 m ahead, the training
	# range) rather than copying its heading, so a spot reached only to
	# 0.5 m still puts a bolt, swing or cast where the take's went.
	var facing := float(track[i][6])
	return {"point": _track_point(track[i]), "facing": facing, "aim": _track_point(track[i]) + Vector2.from_angle(facing) * 3.0}

func _track_point(row: Array) -> Vector2:
	return Vector2(float(row[1]), float(row[2]))

const ACTION_KEYS := [KEY_SPACE, KEY_CTRL, KEY_ENTER, KEY_Z]
const DIALOGUE_NORMALS := [28, 48]
var _follow_linger := 0.0
var _follow_menu_saved := false
var _follow_last_normal := -1
var _follow_space_release := 0.0

func _take_mode(track: Array, landmarks: Array, t: float) -> String:
	var phase := 9
	for mark in landmarks:
		if float(mark["t"]) > t: break
		if mark.has("phase"): phase = int(mark["phase"])
	if phase == 41: return "inventory"
	if phase in [37, 38]: return "menu"
	var lo := 0
	var hi := track.size() - 1
	while lo < hi:
		var mid := (lo + hi + 1) / 2
		if float(track[mid][0]) <= t: lo = mid
		else: hi = mid - 1
	return "dialogue" if int(track[lo][4]) in DIALOGUE_NORMALS else "play"

func _follow_select(selections: Array, t: float) -> bool:
	for choice in selections:
		if absf(float(choice[0]) - t) > 1.0: continue
		var view: Node = game.get("inventory_host").get("view")
		var inventory: Resource = view.get("inventory")
		var item := int(choice[1])
		var kind := ""
		for definition in inventory.get("definitions"):
			if definition != null and int(definition.item_id) == item: kind = str(definition.kind)
		var before := int(inventory.call("quantity", item))
		view.emit_signal("item_requested", item, kind)
		print("SELECT ", JSON.stringify({"take": t, "item": item, "kind": kind, "quantity": before, "selected_object": inventory.call("selected_object_value"), "failure": str(inventory.get("last_failure")), "same": inventory == actor().get("source_inventory"), "initialized": inventory.get("source_initialized"), "held": _held(inventory), "report": str(view.get_node_or_null("Canvas/Status").text) if view.get_node_or_null("Canvas/Status") != null else ""}))
		return true
	return false

func _held(inventory: Resource) -> Dictionary:
	var held := {}
	for definition in inventory.get("definitions"):
		if definition != null and int(definition.quantity) > 0: held[int(definition.item_id)] = int(definition.quantity)
	return held

## skip_if rows: [take time, perso, field, value]. A press within 0.05 s of the
## time, and its release, are dropped while the perso's Brain field equals value.
func _follow_skip(rules: Array, row: Array) -> bool:
	for rule in rules:
		var near := absf(float(rule[0]) - float(row[0])) < 0.05
		if bool(row[2]) and near:
			var brain := brain_of(str(rule[1]))
			if brain != null and int(brain.get(str(rule[2]))) == int(rule[3]):
				print("SKIP ", JSON.stringify({"take": float(row[0]), "rule": rule}))
				_follow_skipped[int(row[1])] = true
				return true
		elif not bool(row[2]) and _follow_skipped.has(int(row[1])):
			_follow_skipped.erase(int(row[1]))
			return true
	return false

## Per-tick trace of live crossbow bolts (fix model 733): position, slide
## contacts, behavior, dsg_120, and what the crossbow mannequin's zone query sees.
func _trace_bolts(clock: float) -> void:
	var hype := brain_of("hype")
	if hype == null or not bool(hype.get("dsg_17")) or int(actor().get("source_inventory").get("selected_weapon")) != 80: return
	var mannequin_brain := brain_of("i_Mannequin_Arbalete")
	var mannequin: Node = mannequin_brain.get_parent() if mannequin_brain != null else null
	for node in game.get("level").find_children("*", "CharacterBody3D", true, false):
		var brain: Node = node.get_node_or_null("Brain")
		if brain == null or str(brain.get("source_model_path")) != "fix:/ai/models/733": continue
		var contacts := []
		for i in node.get_slide_collision_count():
			var collider: Object = node.get_slide_collision(i).get_collider()
			contacts.append(str(collider.get_path()).get_slice("/", -1) if collider is Node else str(collider))
		var zones := []
		if mannequin != null:
			for zone in [1, 2, 3, 4]: zones.append(bool(mannequin.call("source_collide_perso_zde_no_with_type_zde", zone, 2)))
		print("BOLT ", JSON.stringify({"take": snappedf(clock, 0.01), "frame": Engine.get_physics_frames(), "id": node.get_instance_id() % 10000, "pos": [snappedf(node.global_position.x, 0.01), snappedf(-node.global_position.z, 0.01), snappedf(node.global_position.y, 0.01)], "normal": brain.get("current_normal"), "dsg_120": brain.get("dsg_120"), "active": node.get("source_object_active"), "contacts": contacts, "mannequin_zones": zones, "mannequin_dist": snappedf(node.global_position.distance_to(mannequin.global_position), 0.01) if mannequin != null else -1}))

func _port_mode() -> String:
	var inventory_host: Node = game.get("inventory_host")
	if inventory_host != null and is_instance_valid(inventory_host.get("view")) and inventory_host.get("view").visible: return "inventory"
	var pause_host: Node = game.get("pause_host")
	if pause_host != null and bool(pause_host.get("owns_pause")): return "menu"
	# A port dialogue is one actually on screen (normal 48 is also Hype's
	# ordinary return-to-idle state).
	var brain := brain_of("hype")
	var screen: Node = brain.call("source_ui") if brain != null else null
	if screen != null:
		var box: Control = screen.get("dialogue")
		if box != null and box.visible and not bool(box.call("is_over")): return "dialogue"
	return "play"

var _follow_gap := 0.0
var _watch := []
var _follow_facing := NAN
var _follow_key_release := {}
var _follow_facing_off := false
var _follow_aiming := false
var _follow_skipped := {}
var _follow_spot := {}
var _follow_quiet_frames := 0
var _turn_since := 0.0
var _follow_take_shift := false
var _follow_hold := 0.0
var _steer_state := {}
var _gait := "stand"
var _gait_time := 0.0
var _follow_metrics := {"turn_toggles": 0, "port_starts": 0, "port_stops": 0, "take_starts": 0, "take_stops": 0, "spurious": 0, "frames": 0, "straight_frames": 0, "yaw_rate_sq": 0.0, "moving_seconds": 0.0}
var _port_moving := false
var _port_moving_time := 0.0
var _take_moving := false
var _last_take_change := -100.0
var _last_heading := 0.0
var _last_here := Vector2.ZERO
var _measure_clock := 0.0
var _port_changes: Array = []
var _take_changes: Array = []

## Starts/stops of the port against the take's, and heading jitter: the RMS
## yaw rate while the take's own heading is steady and Hype is moving.
func _follow_measure(here: Vector2, take_speed: float, track: Array, cursor: int, playable: bool, dt: float) -> void:
	_measure_clock += dt
	var m := _follow_metrics
	var brain := brain_of("hype")
	var controllable := brain != null and int(brain.get("current_normal")) in [4, 5, 6, 30] and _port_mode() == "play"
	if not playable or not controllable:
		_last_here = here
		_last_heading = heading()
		return
	m["frames"] += 1
	var speed := here.distance_to(_last_here) / dt
	_last_here = here
	var take_now := take_speed > 0.5
	if take_now != _take_moving:
		_take_moving = take_now
		m["take_starts" if take_now else "take_stops"] += 1
		_last_take_change = _measure_clock
		_take_changes.append(_measure_clock)
	# Debounced port motion (0.25 s), so a single slow frame is not a stop.
	var port_now := speed > 0.5
	if port_now != _port_moving:
		_port_moving_time += dt
		if _port_moving_time > 0.4:
			_port_moving = port_now
			_port_moving_time = 0.0
			m["port_starts" if port_now else "port_stops"] += 1
			_port_changes.append(_measure_clock)
	else:
		_port_moving_time = 0.0
	var yaw := wrapf(heading() - _last_heading, -PI, PI) / dt
	_last_heading = heading()
	if _port_moving:
		m["moving_seconds"] += dt
		var a := cursor
		var b := mini(cursor + 2, track.size() - 1)
		var c := mini(cursor + 4, track.size() - 1)
		var h1 := (_track_point(track[b]) - _track_point(track[a])).angle()
		var h2 := (_track_point(track[c]) - _track_point(track[b])).angle()
		if absf(wrapf(h2 - h1, -PI, PI)) < deg_to_rad(10.0) and _track_point(track[a]).distance_to(_track_point(track[c])) > 1.0:
			m["straight_frames"] += 1
			m["yaw_rate_sq"] += pow(rad_to_deg(yaw), 2)

func _follow_report_metrics() -> void:
	var m := _follow_metrics
	# A port start or stop is spurious when the take has none within 1.5 s.
	var spurious := 0
	for t in _port_changes:
		if not _take_changes.any(func(u): return absf(float(u) - float(t)) < 1.5): spurious += 1
	m["spurious"] = spurious
	var minutes := maxf(float(m["frames"]) / Engine.physics_ticks_per_second / 60.0, 0.001)
	print("MOTION ", JSON.stringify({"minutes": snappedf(minutes, 0.01), "port_starts": m["port_starts"], "port_stops": m["port_stops"], "take_starts": m["take_starts"], "take_stops": m["take_stops"], "spurious_start_stop": m["spurious"], "controlled_minutes": snappedf(minutes, 0.01), "turn_toggles_per_min": snappedf(m["turn_toggles"] / minutes, 0.1), "straight_yaw_rms_deg_s": snappedf(sqrt(m["yaw_rate_sq"] / maxf(m["straight_frames"], 1)), 0.1)}))
func _follow_steer(name: String, pressed: bool) -> void:
	if bool(_steer_state.get(name, false)) == pressed: return
	_steer_state[name] = pressed
	key(name, pressed)

func _follow_key(code: int, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	Input.parse_input_event(event)
	Input.flush_buffered_events()

func _trace56(seconds: float) -> void:
	var hype_brain := brain_of("hype")
	var brain_states := {}
	var recent := []
	var last_56: Variant = hype_brain.get("dsg_56")
	var end := now_ms() + int(seconds * 1000.0)
	while now_ms() < end:
		await physics_frame
		for brain in game.get("level").find_children("Brain", "", true, false):
			var name_key := str(brain.get_parent().get_meta("source_perso_name", brain.get_parent().name))
			var state := [brain.get("current_normal"), brain.get("current_reflex")]
			if brain_states.has(name_key) and brain_states[name_key] != state:
				recent.append({"ms": now_ms(), "actor": name_key, "from": brain_states[name_key], "to": state})
			brain_states[name_key] = state
		recent = recent.filter(func(row): return now_ms() - int(row["ms"]) < 300)
		if hype_brain.get("dsg_56") != last_56:
			print("FLIP56 ", JSON.stringify({"ms": now_ms(), "to": hype_brain.get("dsg_56"), "hype_normal": hype_brain.get("current_normal"), "recent": recent}))
			last_56 = hype_brain.get("dsg_56")
