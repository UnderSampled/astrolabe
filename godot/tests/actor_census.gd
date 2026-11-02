extends SceneTree
## Through the real Load Game route, wait, then list every actor's state for
## tools/actor_compare.py. Args: <output.json> <seconds>
var game: Node

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var args := OS.get_cmdline_user_args()
	root.size = Vector2i(1024, 768)
	var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	current_scene = menu
	await process_frame
	root.set_meta(load("res://game/save_state.gd").LOAD_SOURCE_META, "original:SaveGame0.cfg")
	menu.call("open_game", "res://game/game_session.tscn", menu.get("load_game"))
	var deadline := Time.get_ticks_msec() + 60000
	while (current_scene == null or current_scene == menu or not bool(current_scene.get("ready_to_play"))) and Time.get_ticks_msec() < deadline:
		await process_frame
	game = current_scene
	await create_timer(float(args[1])).timeout
	var rows: Array = []
	var level: Node = game.get("level")
	for body in level.find_children("*", "CharacterBody3D", true, false):
		if not body.has_meta("source_perso"): continue
		var brain := body.get_node_or_null("Brain")
		var at: Vector3 = body.global_position
		rows.append({"asset": str(body.get_meta("source_perso")), "name": str(body.get_meta("source_perso_name", body.name)),
			"object_types": Array(body.get_meta("source_object_types", PackedInt32Array())),
			"position": [at.x, -at.z, at.y], "action": int(body.get("current_state")),
			"normal": int(brain.get("current_normal")) if brain != null else -1,
			"reflex": int(brain.get("current_reflex")) if brain != null else -1,
			"running": bool(brain.get("running")) if brain != null else false,
			"failure": str(brain.get("unsupported_failure")) if brain != null else ""})
	var file := FileAccess.open(args[0], FileAccess.WRITE)
	file.store_string(JSON.stringify(rows))
	file.close()
	quit()
