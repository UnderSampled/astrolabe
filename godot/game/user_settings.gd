extends Node
## User-owned audio and gameplay input preferences for native Godot menus.

const FILE_PATH := "user://hype-settings.cfg"
const BUSES := {
	"Music": "Astrolabe Music",
	"Effects": "Astrolabe Effects",
	"Voices": "Astrolabe Voice",
}
const CONTROL_ACTIONS := [
	{"action": "Button_Left", "label": "Move Left", "default_key": KEY_LEFT},
	{"action": "Button_Right", "label": "Move Right", "default_key": KEY_RIGHT},
	{"action": "Button_Up", "label": "Move Forward", "default_key": KEY_UP},
	{"action": "Button_Down", "label": "Move Back", "default_key": KEY_DOWN},
	{"action": "Button_Hit", "label": "Use / Attack", "default_key": KEY_SPACE},
]
const RESERVED_KEYS := [KEY_ESCAPE, KEY_TAB, KEY_F5, KEY_R]
const PAD_AXIS_CAPTURE_THRESHOLD := 0.65

var values: Dictionary = {"Music": 1.0, "Effects": 1.0, "Voices": 1.0}
var keyboard_bindings: Dictionary = {}
var controller_bindings: Dictionary = {}
var _aim_mode := false

func _ready() -> void:
	for bus_name in BUSES.values():
		_ensure_bus(bus_name)
	_reset_control_binding_data()
	var file := ConfigFile.new()
	if file.load(FILE_PATH) == OK:
		for group in BUSES:
			values[group] = clampf(float(file.get_value("audio", group, 1.0)), 0.0, 1.0)
		_load_control_bindings(file)
	_apply()

func get_volume(group: String) -> float:
	return float(values.get(group, 1.0))

## Native slider values retain the original 0..127 source scale.
func set_volume(group: String, source_value: float) -> void:
	if not BUSES.has(group):
		return
	values[group] = clampf(source_value / 127.0, 0.0, 1.0)
	_apply_group(group)
	_save()

func reset_audio() -> void:
	for group in BUSES:
		values[group] = 1.0
	_apply()
	_save()

func get_control_actions() -> Array:
	return CONTROL_ACTIONS.duplicate(true)

func get_binding_label(action: String, mode: String) -> String:
	if mode == "Keyboard":
		var code := int(keyboard_bindings.get(action, 0))
		return OS.get_keycode_string(code) if code > 0 else "Unassigned"
	var binding: Variant = controller_bindings.get(action, {})
	if not binding is Dictionary or binding.is_empty():
		return "Unassigned"
	match str(binding.get("type", "")):
		"button":
			return "Button %d" % (int(binding.get("index", -1)) + 1)
		"axis":
			return "Axis %d %s" % [int(binding.get("index", -1)), "+" if float(binding.get("value", 0.0)) > 0.0 else "−"]
	return "Unassigned"

## Binds an event and transfers an existing same-mode binding to the edited row.
## This mirrors the source remapper's occupied-key transfer behavior.
func set_control_binding(action: String, mode: String, event: InputEvent) -> Dictionary:
	if not _has_control_action(action):
		return {"ok": false, "message": "That action is not supported by this game session."}
	var binding: Variant = _binding_from_event(mode, event)
	if binding == null:
		return {"ok": false, "message": "That input cannot be assigned."}
	var bindings := keyboard_bindings if mode == "Keyboard" else controller_bindings if mode == "Controller" else {}
	if bindings.is_empty():
		return {"ok": false, "message": "Choose Keyboard or Controller first."}
	var previous: Variant = bindings.get(action)
	var swapped_action := ""
	for candidate in CONTROL_ACTIONS:
		var candidate_action := str(candidate["action"])
		if candidate_action == action:
			continue
		if _same_binding(mode, bindings.get(candidate_action), binding):
			swapped_action = candidate_action
			break
	if not swapped_action.is_empty():
		bindings[swapped_action] = previous if mode == "Keyboard" or not previous is Dictionary else {}
	bindings[action] = binding
	var saved := _save()
	configure_gameplay_inputs(_aim_mode)
	if not saved:
		return {"ok": true, "message": "Applied for this session, but preferences could not be saved."}
	if swapped_action.is_empty():
		return {"ok": true, "message": "%s saved." % _action_label(action)}
	return {"ok": true, "message": "Bindings swapped: %s and %s." % [_action_label(action), _action_label(swapped_action)]}

## Restore Astrolabe's current native controls, not unrecovered game defaults.
func reset_control_bindings() -> bool:
	_reset_control_binding_data()
	var saved := _save()
	configure_gameplay_inputs(_aim_mode)
	return saved

## Route the five currently playable source actions to native gameplay actions.
## The four movement bindings become crossbow camera controls only while aiming.
func configure_gameplay_inputs(aim_mode: bool) -> void:
	_aim_mode = aim_mode
	var routes := {
		"Button_Left": "BT_Camera_LookLeft" if aim_mode else "Button_Left",
		"Button_Right": "BT_Camera_LookRight" if aim_mode else "Button_Right",
		"Button_Up": "BT_Camera_LookUp" if aim_mode else "Button_Up",
		"Button_Down": "BT_Camera_LookDown" if aim_mode else "Button_Down",
		"Button_Hit": "BT_Camera_TireArbalete" if aim_mode else "Button_Hit",
	}
	var affected := ["Button_Left", "Button_Right", "Button_Up", "Button_Down", "Button_Hit", "BT_Camera_LookLeft", "BT_Camera_LookRight", "BT_Camera_LookUp", "BT_Camera_LookDown", "BT_Camera_TireArbalete"]
	for action in affected:
		if not InputMap.has_action(action):
			InputMap.add_action(action)
		for existing in InputMap.action_get_events(action):
			if existing is InputEventKey or existing is InputEventJoypadButton or existing is InputEventJoypadMotion:
				InputMap.action_erase_event(action, existing)
	for source_action in routes:
		var target_action: String = routes[source_action]
		var key_code := int(keyboard_bindings.get(source_action, 0))
		if key_code > 0:
			var key_event := InputEventKey.new()
			key_event.physical_keycode = key_code
			InputMap.action_add_event(target_action, key_event)
		var pad_binding: Variant = controller_bindings.get(source_action, {})
		var pad_event := _event_from_pad_binding(pad_binding)
		if pad_event != null:
			InputMap.action_add_event(target_action, pad_event)

func _reset_control_binding_data() -> void:
	keyboard_bindings.clear()
	controller_bindings.clear()
	for item in CONTROL_ACTIONS:
		var action := str(item["action"])
		keyboard_bindings[action] = int(item["default_key"])
		controller_bindings[action] = {}

func _load_control_bindings(file: ConfigFile) -> void:
	var stored_keyboard: Variant = file.get_value("controls", "keyboard", {})
	if stored_keyboard is Dictionary:
		for item in CONTROL_ACTIONS:
			var action := str(item["action"])
			var code := int(stored_keyboard.get(action, keyboard_bindings[action]))
			if code > 0 and not RESERVED_KEYS.has(code):
				keyboard_bindings[action] = code
	var stored_controller: Variant = file.get_value("controls", "controller", {})
	if stored_controller is Dictionary:
		for item in CONTROL_ACTIONS:
			var action := str(item["action"])
			var binding: Variant = stored_controller.get(action, {})
			if _valid_pad_binding(binding):
				controller_bindings[action] = binding.duplicate(true)

func _binding_from_event(mode: String, event: InputEvent) -> Variant:
	if mode == "Keyboard" and event is InputEventKey:
		var key := event as InputEventKey
		var code := int(key.physical_keycode)
		if code <= 0 or RESERVED_KEYS.has(code) or key.ctrl_pressed or key.alt_pressed or key.shift_pressed or key.meta_pressed:
			return null
		return code
	if mode == "Controller" and event is InputEventJoypadButton:
		return {"type": "button", "index": (event as InputEventJoypadButton).button_index}
	if mode == "Controller" and event is InputEventJoypadMotion:
		var motion := event as InputEventJoypadMotion
		if absf(motion.axis_value) >= PAD_AXIS_CAPTURE_THRESHOLD:
			return {"type": "axis", "index": motion.axis, "value": 1.0 if motion.axis_value > 0.0 else -1.0}
	return null

func _same_binding(mode: String, left: Variant, right: Variant) -> bool:
	if mode == "Keyboard":
		return int(left) == int(right)
	return left is Dictionary and right is Dictionary and left == right and not left.is_empty()

func _valid_pad_binding(binding: Variant) -> bool:
	if not binding is Dictionary:
		return false
	match str(binding.get("type", "")):
		"button":
			return int(binding.get("index", -1)) >= 0
		"axis":
			return int(binding.get("index", -1)) >= 0 and absf(float(binding.get("value", 0.0))) == 1.0
	return false

func _event_from_pad_binding(binding: Variant) -> InputEvent:
	if not _valid_pad_binding(binding):
		return null
	if str(binding["type"]) == "button":
		var button := InputEventJoypadButton.new()
		button.device = -1
		button.button_index = int(binding["index"])
		return button
	var motion := InputEventJoypadMotion.new()
	motion.device = -1
	motion.axis = int(binding["index"])
	motion.axis_value = float(binding["value"])
	return motion

func _has_control_action(action: String) -> bool:
	for item in CONTROL_ACTIONS:
		if str(item["action"]) == action:
			return true
	return false

func _action_label(action: String) -> String:
	for item in CONTROL_ACTIONS:
		if str(item["action"]) == action:
			return str(item["label"])
	return action

func _apply() -> void:
	for group in BUSES:
		_apply_group(group)

func _apply_group(group: String) -> void:
	var index := AudioServer.get_bus_index(BUSES[group])
	if index < 0:
		return
	AudioServer.set_bus_volume_db(index, linear_to_db(maxf(float(values[group]), 0.000001)))

func _ensure_bus(bus_name: String) -> void:
	if AudioServer.get_bus_index(bus_name) >= 0:
		return
	AudioServer.add_bus()
	var index := AudioServer.bus_count - 1
	AudioServer.set_bus_name(index, bus_name)
	AudioServer.set_bus_send(index, &"Master")

func _save() -> bool:
	var file := ConfigFile.new()
	for group in BUSES:
		file.set_value("audio", group, values[group])
	file.set_value("controls", "keyboard", keyboard_bindings)
	file.set_value("controls", "controller", controller_bindings)
	var error := file.save(FILE_PATH)
	if error != OK:
		push_warning("Could not save Hype preferences: " + error_string(error))
		return false
	return true
