extends Node
## User-owned audio and gameplay input preferences for native Godot menus.

const FILE_PATH := "user://hype-settings.cfg"
const BUSES := {
	"Music": "Astrolabe Music",
	"Effects": "Astrolabe Effects",
	"Voices": "Astrolabe Voice",
}
const IMAGE_DEFAULTS := {
    "shadow_quality": 4,
    "screen_framing": 100,
    "texture_quality": 1.0,
}
const ORIGINAL_SHADOW_META := &"astrolabe_settings_original_shadow_enabled"
const ORIGINAL_FOV_META := &"astrolabe_settings_original_fov"
const ORIGINAL_SIZE_META := &"astrolabe_settings_original_camera_size"
const ORIGINAL_OVERRIDE_META := &"astrolabe_settings_original_material_override"
const FILTERED_OVERRIDE_META := &"astrolabe_settings_filtered_material_override"
const AUDIO_ORIGINAL_META := &"astrolabe_settings_original_audio_stream"
const LIGHT_SETTINGS_GROUP := &"astrolabe_settings_lights"
const CAMERA_SETTINGS_GROUP := &"astrolabe_settings_cameras"
const MESH_SETTINGS_GROUP := &"astrolabe_settings_meshes"
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
var image_values: Dictionary = IMAGE_DEFAULTS.duplicate(true)
var keyboard_bindings: Dictionary = {}
var controller_bindings: Dictionary = {}
var reverse_stereo := false
var _aim_mode := false
var _reversed_audio_cache: Dictionary = {}
var _stereo_test_player: AudioStreamPlayer

func _ready() -> void:
	for bus_name in BUSES.values():
		_ensure_bus(bus_name)
	_reset_control_binding_data()
	var file := ConfigFile.new()
	if file.load(FILE_PATH) == OK:
		for group in BUSES:
			values[group] = clampf(float(file.get_value("audio", group, 1.0)), 0.0, 1.0)
		reverse_stereo = bool(file.get_value("audio", "reverse_stereo", false))
		image_values["shadow_quality"] = clampi(int(file.get_value("video", "shadow_quality", IMAGE_DEFAULTS.shadow_quality)), 0, 4)
		image_values["screen_framing"] = clampi(int(file.get_value("video", "screen_framing", IMAGE_DEFAULTS.screen_framing)), 60, 100)
		image_values["texture_quality"] = clampf(float(file.get_value("video", "texture_quality", IMAGE_DEFAULTS.texture_quality)), 0.0, 1.0)
		_load_control_bindings(file)
	get_tree().node_added.connect(_on_node_added)
	_apply()
	_apply_image_settings_to_tree(get_tree().root)
	_apply_reverse_stereo_to_tree(get_tree().root)

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

func get_image_setting(name: String) -> float:
	return float(image_values.get(name, IMAGE_DEFAULTS.get(name, 0.0)))

func set_image_setting(name: String, value: float) -> void:
	if not IMAGE_DEFAULTS.has(name):
		return
	match name:
		"shadow_quality":
			image_values[name] = clampi(roundi(value), 0, 4)
		"screen_framing":
			image_values[name] = clampi(roundi(value), 60, 100)
		"texture_quality":
			image_values[name] = clampf(value, 0.0, 1.0)
	_apply_image_setting(name)
	_save()

func get_reverse_stereo() -> bool:
	return reverse_stereo

func set_reverse_stereo(enabled: bool) -> void:
	reverse_stereo = enabled
	_apply_reverse_stereo_to_tree(get_tree().root)
	_save()

## Plays a native left-then-right channel check through the same Effects bus as
## menu/game sounds. The original menu points at an external stereo-test event.
func play_stereo_test() -> bool:
	if is_instance_valid(_stereo_test_player):
		_stereo_test_player.stop()
		_stereo_test_player.queue_free()
	var player := AudioStreamPlayer.new()
	player.name = "StereoTestPlayer"
	player.bus = BUSES["Effects"]
	var stream := _make_stereo_test_stream()
	player.set_meta(AUDIO_ORIGINAL_META, stream)
	player.stream = _reversed_stream(stream) if reverse_stereo else stream
	_stereo_test_player = player
	get_tree().root.add_child(player)
	player.finished.connect(func():
		if is_instance_valid(player):
			player.queue_free()
		if _stereo_test_player == player:
			_stereo_test_player = null
	)
	player.play()
	return true

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

func _on_node_added(node: Node) -> void:
	# Wait until the instanced scene has finished assigning exported properties.
	call_deferred("_apply_settings_to_node", node)

func _apply_settings_to_node(node: Node) -> void:
	if not is_instance_valid(node) or not node.is_inside_tree():
		return
	_register_image_settings_node(node)
	_apply_image_settings_to_node(node)
	_apply_reverse_stereo_to_node(node)

func _apply_image_settings_recursive(node: Node) -> void:
	_register_image_settings_node(node)
	_apply_image_settings_to_node(node)
	for child in node.get_children():
		_apply_image_settings_recursive(child)

func _apply_image_settings_to_node(node: Node) -> void:
	if node is Light3D:
		_apply_shadow_to_light(node as Light3D)
	elif node is Camera3D:
		_apply_screen_framing(node as Camera3D)
	elif node is MeshInstance3D:
		_apply_texture_filter(node as MeshInstance3D)

func _register_image_settings_node(node: Node) -> void:
	if node is Light3D and not node.is_in_group(LIGHT_SETTINGS_GROUP):
		node.add_to_group(LIGHT_SETTINGS_GROUP)
	elif node is Camera3D and not node.is_in_group(CAMERA_SETTINGS_GROUP):
		node.add_to_group(CAMERA_SETTINGS_GROUP)
	elif node is MeshInstance3D and not node.is_in_group(MESH_SETTINGS_GROUP):
		node.add_to_group(MESH_SETTINGS_GROUP)

func _apply_shadow_to_light(light: Light3D) -> void:
	if not light.has_meta(ORIGINAL_SHADOW_META):
		var source_shadow_enabled := light.shadow_enabled
		var source_wrapper := light.get_parent()
		if source_wrapper != null and source_wrapper.has_meta("source_light") and not light.light_negative:
			source_shadow_enabled = true
		light.set_meta(ORIGINAL_SHADOW_META, source_shadow_enabled)
	light.shadow_enabled = bool(light.get_meta(ORIGINAL_SHADOW_META)) and int(image_values.shadow_quality) > 0

func _apply_screen_framing(camera: Camera3D) -> void:
	var source_crop := clampf((2.0 * float(image_values.screen_framing) - 100.0) / 100.0, 0.2, 1.0)
	if camera.projection == Camera3D.PROJECTION_PERSPECTIVE:
		if not camera.has_meta(ORIGINAL_FOV_META):
			camera.set_meta(ORIGINAL_FOV_META, camera.fov)
		var base_fov := deg_to_rad(float(camera.get_meta(ORIGINAL_FOV_META)))
		camera.fov = clampf(rad_to_deg(2.0 * atan(tan(base_fov * 0.5) * source_crop)), 1.0, 179.0)
	elif camera.projection == Camera3D.PROJECTION_ORTHOGONAL or camera.projection == Camera3D.PROJECTION_FRUSTUM:
		if not camera.has_meta(ORIGINAL_SIZE_META):
			camera.set_meta(ORIGINAL_SIZE_META, camera.size)
		camera.size = float(camera.get_meta(ORIGINAL_SIZE_META)) * source_crop

func _apply_texture_filter(mesh: MeshInstance3D) -> void:
	var filter_quality := clampi(roundi(float(image_values.texture_quality) * 5.0), 0, 5)
	if not mesh.has_meta(ORIGINAL_OVERRIDE_META):
		mesh.set_meta(ORIGINAL_OVERRIDE_META, mesh.material_override)
	var source_override: Variant = mesh.get_meta(ORIGINAL_OVERRIDE_META)
	if source_override is BaseMaterial3D:
		var filtered: Variant = mesh.get_meta(FILTERED_OVERRIDE_META) if mesh.has_meta(FILTERED_OVERRIDE_META) else null
		if not filtered is BaseMaterial3D:
			filtered = (source_override as BaseMaterial3D).duplicate()
			mesh.set_meta(FILTERED_OVERRIDE_META, filtered)
		(filtered as BaseMaterial3D).texture_filter = filter_quality
		mesh.material_override = filtered as Material
		return
	if source_override != null or mesh.mesh == null:
		return
	for surface in mesh.mesh.get_surface_count():
		var original_key := StringName("astrolabe_settings_original_surface_%d" % surface)
		var filtered_key := StringName("astrolabe_settings_filtered_surface_%d" % surface)
		if not mesh.has_meta(original_key):
			mesh.set_meta(original_key, mesh.get_active_material(surface))
		var source_material: Variant = mesh.get_meta(original_key)
		if not source_material is BaseMaterial3D:
			continue
		var filtered_material: Variant = mesh.get_meta(filtered_key) if mesh.has_meta(filtered_key) else null
		if not filtered_material is BaseMaterial3D:
			filtered_material = (source_material as BaseMaterial3D).duplicate()
			mesh.set_meta(filtered_key, filtered_material)
		(filtered_material as BaseMaterial3D).texture_filter = filter_quality
		mesh.set_surface_override_material(surface, filtered_material as Material)

func _apply_image_settings_to_tree_quality() -> void:
	var source_quality := int(image_values.shadow_quality)
	var filter_quality := RenderingServer.SHADOW_QUALITY_HARD
	match source_quality:
		2: filter_quality = RenderingServer.SHADOW_QUALITY_SOFT_VERY_LOW
		3: filter_quality = RenderingServer.SHADOW_QUALITY_SOFT_LOW
		4: filter_quality = RenderingServer.SHADOW_QUALITY_SOFT_HIGH
	RenderingServer.directional_soft_shadow_filter_set_quality(filter_quality)
	RenderingServer.positional_soft_shadow_filter_set_quality(filter_quality)

func _apply_image_settings_to_tree(root_node: Node) -> void:
	_apply_image_settings_to_tree_quality()
	_apply_image_settings_recursive(root_node)

func _apply_image_setting(name: String) -> void:
	var group: StringName
	match name:
		"shadow_quality":
			_apply_image_settings_to_tree_quality()
			group = LIGHT_SETTINGS_GROUP
		"screen_framing":
			group = CAMERA_SETTINGS_GROUP
		"texture_quality":
			group = MESH_SETTINGS_GROUP
	for node in get_tree().get_nodes_in_group(group):
		_apply_image_settings_to_node(node)

func _apply_reverse_stereo_to_tree(root_node: Node) -> void:
	_apply_reverse_stereo_recursive(root_node)

func _apply_reverse_stereo_recursive(node: Node) -> void:
	_apply_reverse_stereo_to_node(node)
	for child in node.get_children():
		_apply_reverse_stereo_recursive(child)

func _apply_reverse_stereo_to_node(node: Node) -> void:
	if not (node is AudioStreamPlayer or node is AudioStreamPlayer2D or node is AudioStreamPlayer3D):
		return
	var player := node
	if not player.has_meta(AUDIO_ORIGINAL_META):
		player.set_meta(AUDIO_ORIGINAL_META, player.get("stream"))
	var original: Variant = player.get_meta(AUDIO_ORIGINAL_META)
	if not reverse_stereo or original == null:
		_set_player_stream(player, original as AudioStream)
		return
	var reversed := _reversed_stream(original as AudioStream)
	_set_player_stream(player, reversed if reversed != null else original as AudioStream)

func _set_player_stream(player: Node, stream: AudioStream) -> void:
	if player.get("stream") == stream:
		return
	var resume := bool(player.call("is_playing"))
	var playback_position := float(player.call("get_playback_position")) if resume else 0.0
	player.call("stop")
	player.set("stream", stream)
	if resume:
		player.call("play", playback_position)

func _reversed_stream(stream: AudioStream) -> AudioStreamWAV:
	if not stream is AudioStreamWAV:
		return null
	var original := stream as AudioStreamWAV
	if not original.stereo or not [AudioStreamWAV.FORMAT_8_BITS, AudioStreamWAV.FORMAT_16_BITS].has(original.format):
		return null
	var key := original.get_instance_id()
	if _reversed_audio_cache.has(key):
		return _reversed_audio_cache[key] as AudioStreamWAV
	var bytes := original.data
	var sample_bytes := 1 if original.format == AudioStreamWAV.FORMAT_8_BITS else 2
	var frame_bytes := sample_bytes * 2
	if bytes.is_empty() or bytes.size() % frame_bytes != 0:
		return null
	for frame_offset in range(0, bytes.size(), frame_bytes):
		for sample_offset in sample_bytes:
			var left_index := frame_offset + sample_offset
			var right_index := frame_offset + sample_bytes + sample_offset
			var left_byte := bytes[left_index]
			bytes[left_index] = bytes[right_index]
			bytes[right_index] = left_byte
	var copy := original.duplicate() as AudioStreamWAV
	copy.data = bytes
	_reversed_audio_cache[key] = copy
	return copy

func _make_stereo_test_stream() -> AudioStreamWAV:
	const RATE := 22050
	const TONE_FRAMES := 6615 # 300 ms per channel
	const GAP_FRAMES := 2205 # 100 ms between channels
	var data := PackedByteArray()
	data.resize((TONE_FRAMES * 2 + GAP_FRAMES) * 4)
	for frame in TONE_FRAMES * 2 + GAP_FRAMES:
		var left := 0
		var right := 0
		if frame < TONE_FRAMES:
			left = roundi(sin(TAU * 440.0 * float(frame) / RATE) * 12000.0)
		elif frame >= TONE_FRAMES + GAP_FRAMES:
			right = roundi(sin(TAU * 440.0 * float(frame - TONE_FRAMES - GAP_FRAMES) / RATE) * 12000.0)
		var offset := frame * 4
		data.encode_s16(offset, left)
		data.encode_s16(offset + 2, right)
	var stream := AudioStreamWAV.new()
	stream.format = AudioStreamWAV.FORMAT_16_BITS
	stream.stereo = true
	stream.mix_rate = RATE
	stream.data = data
	return stream

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
	file.set_value("audio", "reverse_stereo", reverse_stereo)
	for setting in IMAGE_DEFAULTS:
		file.set_value("video", setting, image_values[setting])
	file.set_value("controls", "keyboard", keyboard_bindings)
	file.set_value("controls", "controller", controller_bindings)
	var error := file.save(FILE_PATH)
	if error != OK:
		push_warning("Could not save Hype preferences: " + error_string(error))
		return false
	return true
