extends Control
## Source-backed options tabs with native Godot settings where semantics map cleanly.
signal closed
@export var canvas_size := Vector2(640, 480)
@export var initial_focus: NodePath
@export var initial_page: StringName = &"Images"
var preferences: Node
## The original Controls root, opened from the Controls item (FUN_00414b00).
var controls_page: Control
var stereo_status: Label
var reverse_stereo_button: Button

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
	for pair in [["Graphic", "shadow_quality"], ["Screen", "screen_framing"]]:
		var slider := $Canvas/ImagesPage.get_node(pair[0]) as VSlider
		slider.value = float(preferences.call("get_image_setting", pair[1]))
		slider.value_changed.connect(_set_image_setting.bind(pair[1], pair[0]))
		_update_image_value(pair[0], slider.value)
	for group: String in ["Music", "Effects", "Voices"]:
		var slider := $Canvas/SoundPage/Audio.get_node(group) as VSlider
		slider.value = float(preferences.call("get_volume", group))
		slider.value_changed.connect(_set_volume.bind(group))
	stereo_status = $Canvas/SoundPage/StereoStatus as Label
	var test_stereo_button := $Canvas/SoundPage/StereoTest as Button
	test_stereo_button.pressed.connect(_play_stereo_test)
	_connect_source_action_art("StereoTest")
	reverse_stereo_button = $Canvas/SoundPage/ReverseStereo as Button
	reverse_stereo_button.toggle_mode = true
	reverse_stereo_button.button_pressed = bool(preferences.call("get_reverse_stereo"))
	reverse_stereo_button.pressed.connect(_toggle_reverse_stereo)
	_connect_source_action_art("ReverseStereo")
	_refresh_stereo_status()
	$Canvas/TabImages.pressed.connect(_show_page.bind(&"Images"))
	$Canvas/TabControls.pressed.connect(open_controls)
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
	$Canvas/ImagesPage.visible = page == &"Images"
	$Canvas/ControlsPage.visible = page == &"Controls"
	$Canvas/SoundPage.visible = page == &"Sound"
	$Canvas/TabImages/SelectedArtwork.visible = page == &"Images"
	$Canvas/TabControls/SelectedArtwork.visible = page == &"Controls"
	$Canvas/TabSound/SelectedArtwork.visible = page == &"Sound"

func close() -> void:
	_close_controls()
	closed.emit()

func open_controls() -> void:
	_show_page(&"Controls")
	var path := str($Canvas/ControlsPage.get_meta("controls_scene", ""))
	var packed := load(path) as PackedScene if ResourceLoader.exists(path, "PackedScene") else null
	if packed == null:
		push_warning("The converted Controls page is missing; regenerate the menu content.")
		return
	controls_page = packed.instantiate() as Control
	add_child(controls_page)
	controls_page.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	controls_page.closed.connect(_close_controls)
	$Canvas.hide()
	controls_page.focus_initial()

func _close_controls() -> void:
	if is_instance_valid(controls_page):
		controls_page.queue_free()
	controls_page = null
	$Canvas.show()
	if is_visible_in_tree():
		$Canvas/TabControls.grab_focus()

## Hosts ask before closing on Escape; the Controls page handles its own.
func cancel_input_capture() -> bool:
	return is_instance_valid(controls_page)

func _set_volume(value: float, group: String) -> void:
	preferences.call("set_volume", group, value)

func _set_image_setting(value: float, setting: String, control: String) -> void:
	preferences.call("set_image_setting", setting, value)
	_update_image_value(control, value)

func _update_image_value(control: String, value: float) -> void:
	var label := $Canvas/ImagesPage.get_node_or_null(control + "Value") as Label
	if label == null:
		return
	label.text = str(roundi(value))

func _play_stereo_test() -> void:
	if bool(preferences.call("play_stereo_test")):
		stereo_status.text = "Playing left then right test tones."
		get_tree().create_timer(0.8).timeout.connect(_refresh_stereo_status)

func _toggle_reverse_stereo() -> void:
	var reversed := bool(reverse_stereo_button.button_pressed)
	preferences.call("set_reverse_stereo", reversed)
	_refresh_stereo_status()

func _refresh_stereo_status() -> void:
	if not is_instance_valid(stereo_status) or preferences == null:
		return
	var state := "On" if bool(preferences.call("get_reverse_stereo")) else "Off"
	stereo_status.text = "Reverse stereo: %s. Applies to stereo PCM WAV clips." % state

func _connect_source_action_art(action: String) -> void:
	var button := $Canvas/SoundPage.get_node(action) as Button
	button.set_meta("astrolabe_menu_mouse_hover", false)
	button.focus_entered.connect(_refresh_source_action_art.bind(action))
	button.focus_exited.connect(_refresh_source_action_art.bind(action))
	button.mouse_entered.connect(_set_source_action_hover.bind(action, true))
	button.mouse_exited.connect(_set_source_action_hover.bind(action, false))

func _set_source_action_hover(action: String, hovered: bool) -> void:
	var button := $Canvas/SoundPage.get_node(action) as Button
	button.set_meta("astrolabe_menu_mouse_hover", hovered)
	_refresh_source_action_art(action)

func _refresh_source_action_art(action: String) -> void:
	var button := $Canvas/SoundPage.get_node(action) as Button
	var artwork := $Canvas/SoundPage.get_node(action + "Selected") as TextureRect
	artwork.visible = button.has_focus() or bool(button.get_meta("astrolabe_menu_mouse_hover", false))
