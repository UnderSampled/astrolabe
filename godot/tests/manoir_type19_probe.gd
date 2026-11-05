extends SceneTree
## Counterexamples for type-19 kinematic animation motion in the Manoir: after
## Load Game of the converted reference save, Gogoud and the barrels (type-19
## cards) must keep their places unless their own AI walks them; nothing sinks.
## User argument: <report.json>

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var out: String = OS.get_cmdline_user_args()[0]
	var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	current_scene = menu
	await process_frame
	root.set_meta(load("res://game/save_state.gd").LOAD_SOURCE_META, "original:SaveGame0.cfg")
	menu.call("open_game", "res://game/game_session.tscn", menu.get("load_game"))
	var deadline := Time.get_ticks_msec() + 90000
	while Time.get_ticks_msec() < deadline and not (current_scene != null and current_scene.has_method("update_crossbow_aim_target") and bool(current_scene.get("ready_to_play"))):
		await process_frame
	var level: Node = current_scene.get("level")
	var bodies := level.find_children("*", "CharacterBody3D", true, false).filter(func(b): return b.get("current_mechanics") != null and int(b.get("current_mechanics").mode) == 19)
	var start := {}
	for b in bodies: start[b] = b.global_position
	for i in 600: await physics_frame
	var rows := []
	for b in bodies:
		if not is_instance_valid(b): continue
		var brain: Node = b.get_node_or_null("Brain")
		rows.append({"name": str(b.name), "moved": snappedf(b.global_position.distance_to(start[b]), 0.01), "dy": snappedf(b.global_position.y - start[b].y, 0.01), "motion": b.get("motion_enabled"), "normal": brain.get("current_normal") if brain != null else null, "state": b.get("current_state")})
	FileAccess.open(out, FileAccess.WRITE).store_string(JSON.stringify(rows, " "))
	print("TYPE19_PROBE ", rows.size())
	quit(0)
