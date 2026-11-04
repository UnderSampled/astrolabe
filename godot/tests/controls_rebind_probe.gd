extends SceneTree
## Rebinding through the real Controls page reaches gameplay and survives a
## restart. Run twice with the same private XDG_DATA_HOME:
##   godot --path godot --script res://tests/controls_rebind_probe.gd -- bind REPORT.json
##   godot --path godot --script res://tests/controls_rebind_probe.gd -- reload REPORT.json
## "bind" rebinds from the main menu (move forward W, look V), plays, then
## rebinds jump to Tab from the pause menu (inventory takes Left Ctrl by swap).
## "reload" checks those bindings in a new process, plays with them, then
## restores the defaults with the page's reset row.
const BEHAVIOR = preload("res://addons/astrolabe/behavior/behavior.gd")
var report: Dictionary = {"outcome": "running", "checks": []}
var evidence_path := ""
var game: Node
var started_ms := 0

func _initialize() -> void:
	call_deferred("run")

func check(name: String, passed: bool, detail: Variant) -> void:
	report["checks"].append({"name": name, "passed": passed, "detail": detail})
	print("CHECK ", "ok " if passed else "FAIL ", name, " ", detail)

func finish(message: String) -> void:
	var failed: Array = report["checks"].filter(func(c): return not c["passed"])
	report["outcome"] = "failed" if failed or not message.is_empty() else "passed"
	report["message"] = message
	report["elapsed_ms"] = Time.get_ticks_msec() - started_ms
	report["stand_ins"] = BEHAVIOR.stand_ins_reached.values()
	var file := FileAccess.open(evidence_path, FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "\t") + "\n")
	file.close()
	print("CONTROLS_REBIND ", JSON.stringify({"outcome": report["outcome"], "message": message}))
	quit(0 if report["outcome"] == "passed" else 1)

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
	await process_frame
	await process_frame

func key(code: Key, pressed: bool, location := KEY_LOCATION_UNSPECIFIED) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.location = location
	event.pressed = pressed
	Input.parse_input_event(event)
	Input.flush_buffered_events()

func tap(code: Key, location := KEY_LOCATION_UNSPECIFIED) -> void:
	key(code, true, location)
	await process_frame
	key(code, false, location)
	await process_frame
	await process_frame

func settings() -> Node:
	return root.get_node("UserSettings")

func binding(entry: String) -> int:
	return int(settings().call("get_entry_binding", entry))

func actor() -> CharacterBody3D:
	return game.get("actor")

func alive() -> bool:
	var brain: Node = game.get("brain")
	return is_instance_valid(brain) and bool(brain.get("running")) and str(brain.get("unsupported_failure")).is_empty()

func play(seconds: float) -> float:
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	var highest := actor().global_position.y
	while Time.get_ticks_msec() < end:
		await process_frame
		highest = maxf(highest, actor().global_position.y)
	return highest

func hold(code: Key, seconds: float, location := KEY_LOCATION_UNSPECIFIED) -> float:
	key(code, true, location)
	var highest: float = await play(seconds)
	key(code, false, location)
	return highest

func walk_distance(code: Key, seconds: float) -> float:
	var start := actor().global_position
	await hold(code, seconds)
	await play(0.5)
	var moved := actor().global_position - start
	return Vector2(moved.x, moved.z).length()

func action_keys(action: String) -> Array:
	var keys := []
	if InputMap.has_action(action):
		for event in InputMap.action_get_events(action):
			if event is InputEventKey:
				keys.append(OS.get_keycode_string((event as InputEventKey).physical_keycode))
	return keys

## Menu items emit after their confirmation sound (up to 0.5 s).
func wait_for(host: Node, property: String) -> Node:
	var deadline := Time.get_ticks_msec() + 3000
	while Time.get_ticks_msec() < deadline:
		var value: Variant = host.get(property)
		if value is Node and is_instance_valid(value):
			return value
		await process_frame
	return null

## Opens the Controls page from a configuration screen through its tab button.
func open_controls(configuration: Node) -> Control:
	if configuration == null:
		return null
	await click(configuration.get_node("Canvas/TabControls"))
	var page := configuration.get("controls_page") as Control
	return page if is_instance_valid(page) and page.is_visible_in_tree() else null

## Enter opens the focused tab's list; rows are reached with Down.
func rebind(row: int, code: Key, location := KEY_LOCATION_UNSPECIFIED) -> void:
	await tap(KEY_ENTER)
	for i in row:
		await tap(KEY_DOWN)
	await tap(KEY_ENTER)
	await tap(code, location)
	await tap(KEY_ESCAPE) # the list closes; the binding stays

func load_game(menu: Node) -> bool:
	await click(menu.get("menu").button_for_action(&"SavedGames"))
	var picker: Node = await wait_for(menu, "save_picker")
	if picker == null:
		return false
	for i in 10: await process_frame
	var save: Button = null
	for row in picker.find_children("*", "Button", true, false):
		if row.name != "Back" and not row.disabled and save == null: save = row
	if save == null:
		return false
	await click(save)
	var deadline := Time.get_ticks_msec() + 90000
	while (current_scene == null or current_scene == menu or not bool(current_scene.get("ready_to_play"))) and Time.get_ticks_msec() < deadline:
		await process_frame
	game = current_scene
	if game == null or game == menu or not bool(game.get("ready_to_play")):
		return false
	await play(3.0)
	return true

func open_pause_controls() -> Control:
	await tap(KEY_ESCAPE)
	var pause: Node = game.get_node("PauseHost")
	var view: Node = pause.get("view")
	await click(view.button_for_action(&"Options"))
	return await open_controls(await wait_for(pause, "configuration"))

func close_to_play() -> void:
	for i in 3:
		await tap(KEY_ESCAPE) # Controls -> Configure -> pause -> play
	await play(0.5)

func run() -> void:
	var args := OS.get_cmdline_user_args()
	var phase := args[0]
	evidence_path = args[1]
	started_ms = Time.get_ticks_msec()
	root.size = Vector2i(1024, 768)
	check("bindings_resource_loaded", settings().call("get_control_bindings") != null, "")
	var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	current_scene = menu
	await process_frame
	await process_frame
	if phase == "bind":
		check("defaults_from_source", binding("Up_Key") == KEY_UP and binding("Look_Key") == KEY_KP_0 and binding("Shield_Key") == KEY_P and binding("Activate_Map_Key") == KEY_END and binding("Jump_Key") == KEY_CTRL + (1 << 32), [binding("Up_Key"), binding("Look_Key"), binding("Jump_Key")])
		await click(menu.get("menu").button_for_action(&"Options"))
		var page := await open_controls(await wait_for(menu, "configuration"))
		if page == null:
			finish("The Controls page did not open from the main menu")
			return
		await rebind(0, KEY_W)
		check("menu_binds_forward_to_w", binding("Up_Key") == KEY_W, binding("Up_Key"))
		await rebind(14, KEY_V)
		check("menu_binds_look_to_v", binding("Look_Key") == KEY_V, binding("Look_Key"))
		await tap(KEY_ESCAPE) # Controls -> Configure
		await tap(KEY_ESCAPE) # Configure -> main menu
		var saved := ConfigFile.new()
		saved.load("user://hype-settings.cfg")
		var keys: Dictionary = saved.get_value("controls", "keys", {})
		check("saved_to_settings_file", int(keys.get("Up_Key", 0)) == KEY_W and int(keys.get("Look_Key", 0)) == KEY_V, keys.get("Up_Key", 0))
		if not await load_game(menu):
			finish("Load Game did not reach play")
			return
		check("script_actions_follow_bindings", action_keys("Button_Up") == ["W"] and action_keys("BT_Camera_Look").has("V") and action_keys("Button_Shield") == ["P"] and action_keys("Button_SSLeft") == ["A"] and action_keys("Button_MagicScroll") == ["Delete"] and action_keys("Button_ActivateMap") == ["End"] and action_keys("TestInventoryMode").is_empty(), {"up": action_keys("Button_Up"), "look": action_keys("BT_Camera_Look"), "shield": action_keys("Button_Shield"), "map": action_keys("Button_ActivateMap"), "inventory_test": action_keys("TestInventoryMode")})
		var walked := await walk_distance(KEY_W, 1.5)
		check("w_walks_forward", walked > 1.0 and alive(), walked)
		var arrow := await walk_distance(KEY_UP, 1.0)
		check("up_arrow_no_longer_walks", arrow < 0.3, arrow)
		for code in [KEY_A, KEY_S, KEY_P, KEY_DELETE, KEY_END]:
			await hold(code, 0.4)
			await play(0.6)
			check("newly_bound_%s_keeps_control" % OS.get_keycode_string(code), alive(), str(game.get("brain").get("unsupported_failure")))
		var page2 := await open_pause_controls()
		if page2 == null:
			finish("The Controls page did not open from the pause menu")
			return
		var inventory_view: Control = game.get_node("InventoryHost").get("view")
		await rebind(5, KEY_TAB)
		check("pause_binds_jump_to_tab", binding("Jump_Key") == KEY_TAB, binding("Jump_Key"))
		check("swap_gives_inventory_left_ctrl", binding("Inventory_Key") == KEY_CTRL + (1 << 32), binding("Inventory_Key"))
		check("capture_key_did_not_open_inventory", not (is_instance_valid(inventory_view) and inventory_view.visible), "")
		await close_to_play()
		check("back_in_play", not paused and alive(), paused)
		var ground := actor().global_position.y
		var highest: float = await hold(KEY_TAB, 0.2)
		highest = maxf(highest, await play(1.5))
		check("tab_jumps", highest - ground > 0.2 and alive(), highest - ground)
		inventory_view = game.get_node("InventoryHost").get("view")
		check("tab_did_not_open_inventory", not (is_instance_valid(inventory_view) and inventory_view.visible), "")
		await tap(KEY_CTRL, KEY_LOCATION_LEFT)
		inventory_view = game.get_node("InventoryHost").get("view")
		check("left_ctrl_opens_inventory", is_instance_valid(inventory_view) and inventory_view.visible, "")
		await tap(KEY_CTRL, KEY_LOCATION_LEFT)
		check("left_ctrl_closes_inventory", not inventory_view.visible, "")
	else:
		check("reload_keeps_rebinds", binding("Up_Key") == KEY_W and binding("Look_Key") == KEY_V and binding("Jump_Key") == KEY_TAB and binding("Inventory_Key") == KEY_CTRL + (1 << 32), [binding("Up_Key"), binding("Look_Key"), binding("Jump_Key"), binding("Inventory_Key")])
		if not await load_game(menu):
			finish("Load Game did not reach play")
			return
		var walked := await walk_distance(KEY_W, 1.5)
		check("reloaded_w_walks", walked > 1.0 and alive(), walked)
		var page := await open_pause_controls()
		if page == null:
			finish("The Controls page did not open from the pause menu")
			return
		await tap(KEY_ENTER)
		await tap(KEY_UP) # the ring wraps to "reset to default values"
		await tap(KEY_ENTER)
		check("reset_restores_source_defaults", binding("Up_Key") == KEY_UP and binding("Look_Key") == KEY_KP_0 and binding("Jump_Key") == KEY_CTRL + (1 << 32) and binding("Inventory_Key") == KEY_TAB, [binding("Up_Key"), binding("Jump_Key")])
		await tap(KEY_ESCAPE)
		await close_to_play()
		check("reset_reaches_script_actions", not paused and action_keys("Button_Up") == ["Up"] and action_keys("Button_Jump") == ["Ctrl"], {"paused": paused, "up": action_keys("Button_Up"), "jump": action_keys("Button_Jump")})
		# Turn away from the wall the earlier walk approached (the yard is small).
		await hold(KEY_LEFT, 1.4)
		await play(0.3)
		var arrow := await walk_distance(KEY_UP, 1.5)
		check("up_arrow_walks_after_reset", arrow > 1.0 and alive(), arrow)
	finish("")
