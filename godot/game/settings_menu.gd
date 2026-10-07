extends Control
## Native audio controls over the imported source configuration artwork.
signal closed
@export var canvas_size := Vector2(640, 480)
@export var initial_focus: NodePath
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
		var slider := $Canvas/Audio.get_node(group) as VSlider
		slider.value = float(preferences.call("get_volume", group)) * 127.0
		slider.value_changed.connect(_set_volume.bind(group))
	$Canvas/Back.pressed.connect(close)
	$Canvas/Sound.pressed.connect(func(): $Canvas/Audio/Music.grab_focus())
	focus_initial()

func _fit() -> void:
	var factor := minf(size.x / canvas_size.x, size.y / canvas_size.y)
	$Canvas.scale = Vector2.ONE * factor
	$Canvas.position = (size - canvas_size * factor) * 0.5

func focus_initial() -> void:
	var target := get_node_or_null(initial_focus) as Control
	if target != null and is_visible_in_tree(): target.grab_focus()

func close() -> void:
	closed.emit()

func _set_volume(value: float, group: String) -> void:
	preferences.call("set_volume", group, value)
