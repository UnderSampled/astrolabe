extends SceneTree
## Stephen's 2026-11-04 report: no visible crossbow reticle in the Manoir.
## The real game: bootstrap, Load Game of the converted reference save
## (original:SaveGame0.cfg, Hype owns the crossbow), the session and its HUD.
## The test pages the load-in dialogue until Hype holds on-foot control, then
## uses his bindings only: Button_GetCrossbow draws the crossbow (fix/28
## normal 4 plays the draw and calls Proc_ActivateLaserPointer itself), the
## held look binding puts the session in crossbow aim with its own ray.
## Expected: the LaserPointer sprite shows the engine texture at the aim ray's
## surface hit, follows a turn, and hides when the crossbow is put away.
## User arguments: <report.json> [screenshot dir]

var report := {}
var report_path := ""
var shots := ""

func _initialize() -> void:
	call_deferred("run")

func finish(passed: bool, message: String) -> void:
	report["outcome"] = "passed" if passed else "failed"
	report["message"] = message
	if report_path != "":
		FileAccess.open(report_path, FileAccess.WRITE).store_string(JSON.stringify(report, " "))
	print("RETICLE_PROBE ", report["outcome"], " ", message)
	quit(0 if passed else 1)

## Press or release the first key bound to an action, through the input stream.
func key(action: StringName, down: bool) -> void:
	for event in InputMap.action_get_events(action):
		if event is InputEventKey:
			var copy := event.duplicate() as InputEventKey
			copy.pressed = down
			Input.parse_input_event(copy)
			report.get_or_add("keys", {})[str(action)] = OS.get_keycode_string(copy.physical_keycode if copy.physical_keycode != KEY_NONE else copy.keycode)
			return
	if down: Input.action_press(action)
	else: Input.action_release(action)

func tap(action: StringName, frames: int = 6) -> void:
	Input.action_press(action)
	for i in frames: await physics_frame
	Input.action_release(action)

func run() -> void:
	var args := OS.get_cmdline_user_args()
	report_path = args[0] if args.size() > 0 else ""
	shots = args[1] if args.size() > 1 else ""
	root.size = Vector2i(1024, 768)
	var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	current_scene = menu
	await process_frame
	root.set_meta(load("res://game/save_state.gd").LOAD_SOURCE_META, "original:SaveGame0.cfg")
	menu.call("open_game", "res://game/game_session.tscn", menu.get("load_game"))
	var deadline := Time.get_ticks_msec() + 90000
	var session: Node = null
	while Time.get_ticks_msec() < deadline:
		session = current_scene
		if session != null and session.has_method("update_crossbow_aim_target") and bool(session.get("ready_to_play")): break
		await process_frame
	if session == null or not bool(session.get("ready_to_play")):
		finish(false, "Load Game did not become playable"); return
	var hype := session.get("actor") as CharacterBody3D
	var brain := hype.get_node("Brain")
	var tick := 0
	var stable := 0
	while stable < 60 and tick < 3600:
		stable = stable + 1 if int(brain.get("current_normal")) == 4 else 0
		if int(brain.get("current_normal")) == 28 and tick % 30 == 0: Input.action_press(&"Button_Hit")
		elif tick % 30 == 3: Input.action_release(&"Button_Hit")
		tick += 1
		await physics_frame
	report["control"] = [brain.get("current_normal"), tick]
	if stable < 60:
		finish(false, "Hype never held on-foot control after the load (normal %s)" % brain.get("current_normal")); return
	var pointer: Resource = hype.get("source_laser_pointer")
	var view := hype.get_parent().get_node_or_null("LaserPointer") as Sprite3D
	if view == null or view.texture == null or not view.texture.resource_path.contains("gen_viseur/laserpointer"):
		finish(false, "no LaserPointer view with the engine texture bound to Hype"); return
	report["quantity_80"] = hype.call("source_inventory_quantity", 80)
	await tap(&"Button_GetCrossbow")
	var waited := 0
	while not bool(pointer.get("active")) and waited < 240:
		await physics_frame
		waited += 1
	report["ai_activated"] = [bool(pointer.get("active")), waited, brain.get("current_normal"), brain.get("dsg_59")]
	if not bool(pointer.get("active")):
		finish(false, "drawing the crossbow did not activate the pointer through fix/28: %s" % [report["ai_activated"]]); return
	var global_brain: Node = brain.get("actor_brain_0") if brain.get("actor_brain_0") != null else null
	report["gates"] = {"selected_weapon": hype.get("source_inventory").get("selected_weapon"), "global_dsg_17": global_brain.get("dsg_17") if global_brain != null else "?", "look_events": InputMap.action_get_events(&"BT_Camera_Look").size()}
	key(&"BT_Camera_Look", true)
	var samples := []
	for step in 150:
		if step == 75: key(&"BT_Camera_LookLeft", true)
		if step == 100: key(&"BT_Camera_LookLeft", false)
		# Held: re-asserted each tick (the headless window may drop held keys).
		key(&"BT_Camera_Look", true)
		if step >= 75 and step < 100: key(&"BT_Camera_LookLeft", true)
		await physics_frame
		if step % 15 == 0:
			await process_frame
			var camera := root.get_viewport().get_camera_3d()
			samples.append({"step": step, "look": Input.is_action_pressed("BT_Camera_Look"), "weapon": hype.get("source_inventory").get("selected_weapon"), "aim": session.get("crossbow_aim_mode"), "normal": brain.get("current_normal"), "active": pointer.get("active"), "hit": pointer.get("target_hit"), "target": vec(pointer.get("target_position")), "visible": view.is_visible_in_tree(), "sprite": vec(view.global_position), "eye_distance": snappedf(camera.global_position.distance_to(view.global_position), 0.01), "screen": [snappedf(camera.unproject_position(view.global_position).x, 1), snappedf(camera.unproject_position(view.global_position).y, 1)]})
		if shots != "" and step in [60, 135]:
			await save_shot("aim_%03d" % step)
	key(&"BT_Camera_Look", false)
	for i in 30: await physics_frame
	samples.append({"step": "look released", "active": pointer.get("active"), "visible": view.is_visible_in_tree(), "normal": brain.get("current_normal")})
	await tap(&"Button_GetCrossbow")
	for i in 120: await physics_frame
	await process_frame
	samples.append({"step": "crossbow put away", "active": pointer.get("active"), "visible": view.is_visible_in_tree(), "normal": brain.get("current_normal")})
	report["samples"] = samples
	var shown := samples.filter(func(row): return row.get("visible", false))
	if shown.is_empty():
		finish(false, "reticle never drawn while aiming: %s" % [samples.slice(0, 4)]); return
	for row in shown:
		if row.has("target") and Vector3(row["sprite"][0], row["sprite"][1], row["sprite"][2]).distance_to(Vector3(row["target"][0], row["target"][1], row["target"][2])) > 0.1:
			finish(false, "reticle is not at the aim ray's surface hit: %s" % row); return
	var targets := {}
	for row in shown: if row.has("target"): targets[str(row["target"])] = true
	if targets.size() < 2:
		finish(false, "reticle did not follow the aim"); return
	if samples.back()["visible"]:
		finish(false, "reticle still visible after the crossbow was put away: %s" % samples.back()); return
	finish(true, "fix/28 activated the pointer on the draw; reticle at %d aim hits; hidden after put-away" % shown.size())

func vec(v: Vector3) -> Array:
	return [snappedf(v.x, 0.01), snappedf(v.y, 0.01), snappedf(v.z, 0.01)]

func save_shot(name: String) -> void:
	await process_frame
	DirAccess.make_dir_recursive_absolute(shots)
	root.get_viewport().get_texture().get_image().save_png(shots.path_join(name + ".png"))
