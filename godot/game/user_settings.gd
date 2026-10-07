extends Node
## Small user-owned preferences store for native Godot menu settings.
const FILE_PATH := "user://hype-settings.cfg"
const BUSES := {
	"Music": "Astrolabe Music",
	"Effects": "Astrolabe Effects",
	"Voices": "Astrolabe Voice",
}
var values: Dictionary = {"Music": 1.0, "Effects": 1.0, "Voices": 1.0}

func _ready() -> void:
	for bus_name in BUSES.values(): _ensure_bus(bus_name)
	var file := ConfigFile.new()
	if file.load(FILE_PATH) == OK:
		for group in BUSES:
			values[group] = clampf(float(file.get_value("audio", group, 1.0)), 0.0, 1.0)
	_apply()

func get_volume(group: String) -> float:
	return float(values.get(group, 1.0))

## Native slider values retain the original 0..127 source scale.
func set_volume(group: String, source_value: float) -> void:
	if not BUSES.has(group): return
	values[group] = clampf(source_value / 127.0, 0.0, 1.0)
	_apply_group(group)
	_save()

func reset_audio() -> void:
	for group in BUSES: values[group] = 1.0
	_apply()
	_save()

func _apply() -> void:
	for group in BUSES: _apply_group(group)

func _apply_group(group: String) -> void:
	var index := AudioServer.get_bus_index(BUSES[group])
	if index < 0: return
	AudioServer.set_bus_volume_db(index, linear_to_db(maxf(float(values[group]), 0.000001)))

func _ensure_bus(bus_name: String) -> void:
	if AudioServer.get_bus_index(bus_name) >= 0: return
	AudioServer.add_bus()
	var index := AudioServer.bus_count - 1
	AudioServer.set_bus_name(index, bus_name)
	AudioServer.set_bus_send(index, &"Master")

func _save() -> void:
	var file := ConfigFile.new()
	for group in BUSES: file.set_value("audio", group, values[group])
	var error := file.save(FILE_PATH)
	if error != OK: push_warning("Could not save Hype audio preferences: " + error_string(error))
