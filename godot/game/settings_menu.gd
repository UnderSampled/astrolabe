extends Control
## Source-backed options tabs with native Godot settings where semantics map cleanly.
signal closed
@export var canvas_size := Vector2(640, 480)
@export var initial_focus: NodePath
@export var initial_page: StringName = &"Images"
var preferences: Node
var control_mode := "Keyboard"
var capture_action := ""
var binding_buttons: Dictionary = {}
var control_status: Label
var keyboard_mode_button: Button
var controller_mode_button: Button

func _ready() -> void:
	resized.connect(_fit)
	_fit()
	preferences = get_node_or_null("/root/UserSettings")
	if preferences == null:
		var settings_script := load("res://game/user_settings.gd") as Script
		preferences = settings_script.new() as Node if settings_script != null else null
		if preferences == null:
			push_error("The UserSettings script is unavailable.")
			return
		preferences.name = "UserSettings"
		get_tree().root.add_child(preferences)
	_build_controls_page()
	for group: String in ["Music", "Effects", "Voices"]:
		var slider := $Canvas/SoundPage/Audio.get_node(group) as VSlider
		slider.value = float(preferences.call("get_volume", group)) * 127.0
		slider.value_changed.connect(_set_volume.bind(group))
	$Canvas/TabImages.pressed.connect(_show_page.bind(&"Images"))
	$Canvas/TabControls.pressed.connect(_show_page.bind(&"Controls"))
	$Canvas/TabSound.pressed.connect(_show_page.bind(&"Sound"))
	$Canvas/Back.pressed.connect(close)
	_show_page(initial_page)
	focus_initial()

func _fit() -> void:
	var factor := minf(size.x / canvas_size.x, size.y / canvas_size.y)
	$Canvas.scale = Vector2.ONE * factor
	$Canvas.position = (size - canvas_size * factor) * 0.5

func focus_initial() -> void:
	var target := get_node_or_null(initial_focus) as Control
	if target != null and is_visible_in_tree(): target.grab_focus()

func _show_page(page: StringName) -> void:
	if not ["Images", "Controls", "Sound"].has(str(page)):
		page = &"Images"
	if capture_action != "":
		cancel_input_capture()
	$Canvas/ImagesPage.visible = page == &"Images"
	$Canvas/ControlsPage.visible = page == &"Controls"
	$Canvas/SoundPage.visible = page == &"Sound"
	$Canvas/TabImages/SelectedArtwork.visible = page == &"Images"
	$Canvas/TabControls/SelectedArtwork.visible = page == &"Controls"
	$Canvas/TabSound/SelectedArtwork.visible = page == &"Sound"

func close() -> void:
	if capture_action != "":
		cancel_input_capture()
	closed.emit()

func _set_volume(value: float, group: String) -> void:
	preferences.call("set_volume", group, value)

func _build_controls_page() -> void:
	var old_message := $Canvas/ControlsPage.get_node_or_null("UnsupportedControls")
	if old_message != null:
		old_message.queue_free()
	keyboard_mode_button = _make_button("Keyboard", Rect2(284.0, 106.0, 132.0, 32.0))
	controller_mode_button = _make_button("Controller", Rect2(422.0, 106.0, 150.0, 32.0))
	$Canvas/ControlsPage.add_child(keyboard_mode_button)
	$Canvas/ControlsPage.add_child(controller_mode_button)
	keyboard_mode_button.pressed.connect(_set_control_mode.bind("Keyboard"))
	controller_mode_button.pressed.connect(_set_control_mode.bind("Controller"))
	var help := Label.new()
	help.name = "ControlHelp"
	help.position = Vector2(202.0, 143.0)
	help.size = Vector2(398.0, 34.0)
	help.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	help.text = "Select a binding, then press one key or controller input. Escape cancels; Tab, F5 and R stay reserved."
	help.add_theme_font_size_override("font_size", 14)
	$Canvas/ControlsPage.add_child(help)
	var actions: Array = preferences.call("get_control_actions")
	for index in actions.size():
		var action := str(actions[index]["action"])
		var row_y := 177.0 + index * 38.0
		var label := Label.new()
		label.name = "Action_%s" % action
		label.position = Vector2(208.0, row_y + 4.0)
		label.size = Vector2(147.0, 30.0)
		label.text = str(actions[index]["label"])
		label.add_theme_font_size_override("font_size", 15)
		$Canvas/ControlsPage.add_child(label)
		var binding_button := _make_button("", Rect2(360.0, row_y, 218.0, 32.0))
		binding_button.name = "Binding_%s" % action
		binding_button.pressed.connect(_start_capture.bind(action))
		$Canvas/ControlsPage.add_child(binding_button)
		binding_buttons[action] = binding_button
	control_status = Label.new()
	control_status.name = "ControlStatus"
	control_status.position = Vector2(205.0, 371.0)
	control_status.size = Vector2(265.0, 43.0)
	control_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	control_status.add_theme_font_size_override("font_size", 13)
	$Canvas/ControlsPage.add_child(control_status)
	var reset_button := _make_button("Reset defaults", Rect2(474.0, 376.0, 126.0, 32.0))
	reset_button.name = "ResetControlBindings"
	reset_button.pressed.connect(_reset_control_bindings)
	$Canvas/ControlsPage.add_child(reset_button)
	_set_control_mode("Keyboard")
	_refresh_binding_buttons()

func _make_button(label: String, rect: Rect2) -> Button:
	var button := Button.new()
	button.text = label
	button.position = rect.position
	button.size = rect.size
	button.add_theme_font_size_override("font_size", 14)
	return button

func _set_control_mode(mode: String) -> void:
	if capture_action != "":
		cancel_input_capture()
	control_mode = mode
	keyboard_mode_button.toggle_mode = true
	controller_mode_button.toggle_mode = true
	keyboard_mode_button.button_pressed = mode == "Keyboard"
	controller_mode_button.button_pressed = mode == "Controller"
	if is_instance_valid(control_status):
		control_status.text = "Keyboard uses physical keys. Controller uses generic Godot joypad buttons and axes."
	_refresh_binding_buttons()

func _refresh_binding_buttons() -> void:
	if preferences == null:
		return
	for action in binding_buttons:
		var button := binding_buttons[action] as Button
		if is_instance_valid(button):
			button.text = "Press input…" if action == capture_action else str(preferences.call("get_binding_label", action, control_mode))

func _start_capture(action: String) -> void:
	capture_action = action
	control_status.text = "Press a %s input for %s. Escape cancels." % ["key" if control_mode == "Keyboard" else "controller", _control_action_label(action)]
	_refresh_binding_buttons()
	var button := binding_buttons.get(action) as Button
	if is_instance_valid(button):
		button.grab_focus()

func cancel_input_capture() -> bool:
	if capture_action.is_empty():
		return false
	capture_action = ""
	control_status.text = "Input change canceled."
	_refresh_binding_buttons()
	return true

func _input(event: InputEvent) -> void:
	if capture_action.is_empty():
		return
	if event is InputEventKey:
		var key := event as InputEventKey
		if not key.pressed or key.echo:
			return
		if key.physical_keycode == KEY_ESCAPE:
			cancel_input_capture()
			get_viewport().set_input_as_handled()
			return
		if control_mode != "Keyboard":
			get_viewport().set_input_as_handled()
			return
		if [KEY_TAB, KEY_F5, KEY_R].has(key.physical_keycode):
			control_status.text = "That key is reserved by the game. Choose another key."
			get_viewport().set_input_as_handled()
			return
		if key.ctrl_pressed or key.alt_pressed or key.shift_pressed or key.meta_pressed:
			control_status.text = "Modifier combinations are not supported. Choose one key."
			get_viewport().set_input_as_handled()
			return
		_accept_capture(key)
		get_viewport().set_input_as_handled()
		return
	if control_mode == "Controller" and event is InputEventJoypadButton:
		var button_event := event as InputEventJoypadButton
		if button_event.pressed:
			_accept_capture(button_event)
			get_viewport().set_input_as_handled()
		return
	if control_mode == "Controller" and event is InputEventJoypadMotion:
		var motion := event as InputEventJoypadMotion
		if absf(motion.axis_value) >= 0.65:
			_accept_capture(motion)
			get_viewport().set_input_as_handled()
		return
	if event is InputEventMouseButton:
		var mouse := event as InputEventMouseButton
		if mouse.pressed and mouse.button_index == MOUSE_BUTTON_RIGHT:
			cancel_input_capture()
			get_viewport().set_input_as_handled()

func _accept_capture(event: InputEvent) -> void:
	var action := capture_action
	var result: Dictionary = preferences.call("set_control_binding", action, control_mode, event)
	if not bool(result.get("ok", false)):
		control_status.text = str(result.get("message", "That input cannot be assigned."))
		return
	capture_action = ""
	control_status.text = str(result.get("message", "Binding saved."))
	_refresh_binding_buttons()
	var button := binding_buttons.get(action) as Button
	if is_instance_valid(button):
		button.grab_focus()

func _reset_control_bindings() -> void:
	if capture_action != "":
		cancel_input_capture()
	var saved := bool(preferences.call("reset_control_bindings"))
	control_status.text = "Astrolabe defaults restored; controller bindings cleared." if saved else "Defaults applied for this session, but preferences could not be saved."
	_refresh_binding_buttons()

func _control_action_label(action: String) -> String:
	for item in preferences.call("get_control_actions"):
		if str(item["action"]) == action:
			return str(item["label"])
	return action
