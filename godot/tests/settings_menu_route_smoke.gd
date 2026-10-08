extends SceneTree
## Run after exporting the English menu: godot --headless --path godot --script res://tests/settings_menu_route_smoke.gd

const SETTINGS_MENU := "res://game/settings_menu.tscn"

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var packed := load(SETTINGS_MENU) as PackedScene
	if packed == null:
		fail("Authored settings route did not load")
		return
	var settings := packed.instantiate() as Control
	if settings == null:
		fail("Settings route root is not a Control")
		return
	root.add_child(settings)
	current_scene = settings
	await process_frame

	var missing: Array[String] = []
	if not settings.has_signal("closed"):
		missing.append("closed signal")
	if not settings.has_method("focus_initial"):
		missing.append("focus_initial()")
	if not settings.has_method("cancel_input_capture"):
		missing.append("cancel_input_capture()")
	for page in ["ImagesPage", "ControlsPage", "SoundPage"]:
		if settings.get_node_or_null("Canvas/" + page) == null:
			missing.append(page)
	for bus in ["Music", "Effects", "Voices"]:
		var slider := settings.get_node_or_null("Canvas/SoundPage/Audio/" + bus) as VSlider
		if slider == null or slider.min_value != 0.0 or slider.max_value != 127.0:
			missing.append("source-scale %s slider" % bus)
	for action in ["Button_Left", "Button_Right", "Button_Up", "Button_Down", "Button_Hit"]:
		if settings.get_node_or_null("Canvas/ControlsPage/Binding_" + action) == null:
			missing.append("%s binding" % action)
	if settings.get_node_or_null("Canvas/ImagesPage/UnsupportedImageSettings") == null:
		missing.append("unsupported Images explanation")
	if not missing.is_empty():
		fail("Settings scene is missing: " + ", ".join(missing))
		return

	settings.call("focus_initial")
	if bool(settings.call("cancel_input_capture")):
		fail("cancel_input_capture() reported an active capture before one started")
		return
	var closed := {"value": false}
	settings.closed.connect(func(): closed["value"] = true)
	(settings.get_node("Canvas/Back") as Button).pressed.emit()
	if not bool(closed["value"]):
		fail("Back did not emit the route's closed signal")
		return
	print("SETTINGS ROUTE SMOKE: source pages, sliders, five bindings and route API passed")
	quit(0)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
