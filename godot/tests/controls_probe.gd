extends SceneTree
const StandInReport = preload("res://addons/astrolabe/behavior/stand_in_report.gd")
## Actual bootstrap -> Load Game -> physical input. Part of tools/gate.py.
## Asserts the things that make this a game: the player answers the controls,
## keeps a running Brain, and the frame rate holds on real hardware.
const BEHAVIOR = preload("res://addons/astrolabe/behavior/behavior.gd")
const SAVE_STATE = preload("res://game/save_state.gd")
var report: Dictionary = {"outcome": "running", "checks": []}
var evidence_path := ""
var game: Node
var deltas: Array[float] = []
var started_ms := 0

func _initialize() -> void:
	call_deferred("run")

func finish(message: String) -> void:
	var failed: Array = report["checks"].filter(func(check): return not check["passed"])
	report["outcome"] = "failed" if failed or not message.is_empty() else "passed"
	report["message"] = message
	report["elapsed_ms"] = Time.get_ticks_msec() - started_ms
	report["stand_ins"] = BEHAVIOR.stand_ins_reached.values()
	var file := FileAccess.open(evidence_path, FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "\t") + "\n")
	file.close()
	print("CONTROLS_PROBE ", JSON.stringify({"outcome": report["outcome"], "message": message, "failed": failed}))
	quit(StandInReport.exit_code(0 if report["outcome"] == "passed" else 1))

func check(name: String, passed: bool, detail: Variant) -> void:
	report["checks"].append({"name": name, "passed": passed, "detail": detail})

func click(button: BaseButton) -> void:
	var position := button.get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = position
		event.global_position = position
		event.pressed = pressed
		Input.parse_input_event(event)
		Input.flush_buffered_events()

func key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	# A real keyboard reports which Ctrl/Shift/Alt; bindings are side specific.
	if [KEY_CTRL, KEY_SHIFT, KEY_ALT].has(code):
		event.location = KEY_LOCATION_LEFT
	event.pressed = pressed
	Input.parse_input_event(event)
	Input.flush_buffered_events()

func actor() -> CharacterBody3D:
	return game.get("actor")

func alive() -> bool:
	var brain: Node = game.get("brain")
	return is_instance_valid(brain) and bool(brain.get("running")) and str(brain.get("unsupported_failure")).is_empty()

func facing() -> Vector3:
	return -actor().global_basis.z

## Advances play for the given time, returning the highest actor position.
func play(seconds: float) -> float:
	var end := Time.get_ticks_usec() + int(seconds * 1000000.0)
	var last := Time.get_ticks_usec()
	var highest := actor().global_position.y
	while Time.get_ticks_usec() < end:
		await process_frame
		var now := Time.get_ticks_usec()
		deltas.append((now - last) / 1000000.0)
		last = now
		highest = maxf(highest, actor().global_position.y)
	return highest

func hold(code: Key, seconds: float) -> float:
	key(code, true)
	var highest: float = await play(seconds)
	key(code, false)
	return highest

func run() -> void:
	evidence_path = OS.get_cmdline_user_args()[0]
	started_ms = Time.get_ticks_msec()
	root.size = Vector2i(1024, 768)
	var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	current_scene = menu
	await process_frame
	await process_frame
	click(menu.find_child("LoadGame", true, false))
	for i in 30: await process_frame
	var picker: Node = menu.get("save_picker")
	var save: Button = null
	for row in picker.find_children("*", "Button", true, false):
		if row.name != "Back" and not row.disabled and save == null: save = row
	if save == null:
		finish("Load Game offered no save")
		return
	click(save)
	var deadline := Time.get_ticks_msec() + 60000
	while (current_scene == null or current_scene == menu or not bool(current_scene.get("ready_to_play"))) and Time.get_ticks_msec() < deadline:
		await process_frame
	game = current_scene
	if game == null or game == menu or not bool(game.get("ready_to_play")):
		finish("Load Game did not reach play within 60 seconds")
		return
	check("load_within_30s", Time.get_ticks_msec() - started_ms < 30000, Time.get_ticks_msec() - started_ms)
	await play(3.0)
	deltas.clear()

	var before := facing()
	await hold(KEY_LEFT, 0.5)
	await play(0.5)
	check("left_turns", before.angle_to(facing()) > 0.3, before.angle_to(facing()))

	var start := actor().global_position
	await hold(KEY_UP, 1.5)
	await play(0.5)
	var walked := start.distance_to(actor().global_position)
	check("up_walks", walked > 1.0 and alive(), walked)

	var ground := actor().global_position.y
	var highest: float = await hold(KEY_CTRL, 0.2)
	highest = maxf(highest, await play(1.5))
	check("ctrl_jumps", highest - ground > 0.2 and alive(), highest - ground)

	await hold(KEY_X, 0.2)
	await play(1.0)
	await hold(KEY_SPACE, 0.2)
	await play(1.0)
	check("sword_keeps_control", alive(), str(game.get("brain").get("unsupported_failure")))

	await play(2.0)
	await hold(KEY_F5, 0.2)
	await play(0.5)
	var saved := SAVE_STATE.read_for_load(SAVE_STATE.NATIVE_SOURCE)
	check("f5_saves_a_loadable_game", saved["status"] == "loaded", [str(game.get("status_label").text), saved.get("message", "")])

	# The save's yard is small, so this only shows control survives a run. The
	# behavior comparison checks running against the original.
	await hold(KEY_LEFT, 0.9)
	await play(0.3)
	key(KEY_SHIFT, true)
	await hold(KEY_UP, 1.0)
	key(KEY_SHIFT, false)
	await play(0.5)
	check("shift_run_keeps_control", alive(), "")

	deltas.sort()
	var median := deltas[deltas.size() / 2] * 1000.0
	var high := deltas[int(deltas.size() * 0.95)] * 1000.0
	var adapter := RenderingServer.get_video_adapter_name()
	report["frames"] = {"count": deltas.size(), "median_ms": median, "p95_ms": high, "adapter": adapter}
	# Software rasterizers measure the machine rather than the game.
	if not adapter.to_lower().contains("llvmpipe"):
		check("frame_median_under_34ms", median < 34.0, median)
	finish("")
