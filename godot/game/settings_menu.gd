extends Control
## Source-backed options tabs with native Godot settings where semantics map cleanly.
signal closed
@export var canvas_size := Vector2(640, 480)
@export var initial_focus: NodePath
@export var initial_page: StringName = &"Images"
var preferences: Node

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
	$Canvas/ImagesPage.visible = page == &"Images"
	$Canvas/ControlsPage.visible = page == &"Controls"
	$Canvas/SoundPage.visible = page == &"Sound"
	$Canvas/TabImages/SelectedArtwork.visible = page == &"Images"
	$Canvas/TabControls/SelectedArtwork.visible = page == &"Controls"
	$Canvas/TabSound/SelectedArtwork.visible = page == &"Sound"

func close() -> void:
	closed.emit()

func _set_volume(value: float, group: String) -> void:
	preferences.call("set_volume", group, value)
