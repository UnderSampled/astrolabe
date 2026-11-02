extends SceneTree
## One scenario from tools/behaviors through the real Load Game route: send its
## keys, then report the action and behavior sequence for comparison with the
## original. Args: <scenarios.json> <name> <output.json>
const KEYS := {"ArrowLeft": KEY_LEFT, "ArrowRight": KEY_RIGHT, "ArrowUp": KEY_UP, "ArrowDown": KEY_DOWN, "LeftShift": KEY_SHIFT, "LeftControl": KEY_CTRL, "X": KEY_X, "Z": KEY_Z, "Space": KEY_SPACE, "Enter": KEY_ENTER}
var game: Node
var samples: Array = []
var started := 0

func _initialize() -> void:
	call_deferred("run")

func key(names: String, pressed: bool) -> void:
	for name in names.split("+"):
		var event := InputEventKey.new()
		event.keycode = KEYS[name]
		event.physical_keycode = KEYS[name]
		event.pressed = pressed
		Input.parse_input_event(event)
	Input.flush_buffered_events()

func sample() -> void:
	var actor: CharacterBody3D = game.get("actor")
	var brain: Node = game.get("brain")
	samples.append({"ms": Time.get_ticks_msec() - started, "action": int(actor.get("current_state")), "normal": int(brain.get("current_normal")), "position": [actor.global_position.x, -actor.global_position.z, actor.global_position.y], "running": bool(brain.get("running"))})

func wait(seconds: float) -> void:
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < end:
		await create_timer(0.05).timeout
		sample()

func run() -> void:
	var args := OS.get_cmdline_user_args()
	var scenario: Dictionary = {}
	for row in JSON.parse_string(FileAccess.get_file_as_string(args[0])):
		if row["name"] == args[1]: scenario = row
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
	# Like the original, Hype plays an entry sequence before answering keys.
	started = Time.get_ticks_msec()
	sample()
	var settle := Time.get_ticks_msec() + 20000
	while int(game.get("actor").get("current_state")) != 47 and Time.get_ticks_msec() < settle:
		await wait(0.1)
	await wait(0.3)
	var entry := samples.duplicate()
	samples.clear()
	started = Time.get_ticks_msec()
	sample()
	for step in scenario["steps"]:
		if step["key"] == "wait":
			await wait(step["hold_ms"] / 1000.0)
			continue
		key(step["key"], true)
		await wait(step["hold_ms"] / 1000.0)
		key(step["key"], false)
	var remaining: float = scenario["seconds"] - (Time.get_ticks_msec() - started) / 1000.0
	if remaining > 0.0: await wait(remaining)
	var view := root.get_viewport().get_camera_3d()
	var camera := {}
	if view != null:
		# Source axes, so both engines report the same frame: x, -z, y.
		var at := view.global_position
		camera = {"position": [at.x, -at.z, at.y], "vertical_fov": view.fov}
	var file := FileAccess.open(args[2], FileAccess.WRITE)
	file.store_string(JSON.stringify({"entry": entry, "samples": samples, "camera": camera}))
	file.close()
	quit()
