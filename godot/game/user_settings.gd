extends Node
## User-owned audio and gameplay input preferences for native Godot menus.

const FILE_PATH := "user://hype-settings.cfg"
const NativeAudioMixer = preload("res://addons/astrolabe/audio/mixer.gd")
const AUDIO_SCALE_VERSION := 1
const BUSES := {
	"Music": "Astrolabe Music",
	"Effects": "Astrolabe Effects",
	"Voices": "Astrolabe Voice",
}
const IMAGE_DEFAULTS := {
    "shadow_quality": 4,
    "screen_framing": 100,
}
const AUDIO_DEFAULTS := {"Music": 100.0, "Effects": 100.0, "Voices": 100.0}
const ORIGINAL_SHADOW_META := &"astrolabe_settings_original_shadow_enabled"
const ORIGINAL_FOV_META := &"astrolabe_settings_original_fov"
const ORIGINAL_SIZE_META := &"astrolabe_settings_original_camera_size"
const AUDIO_ORIGINAL_META := &"astrolabe_settings_original_audio_stream"
const LIGHT_SETTINGS_GROUP := &"astrolabe_settings_lights"
const CAMERA_SETTINGS_GROUP := &"astrolabe_settings_cameras"
## Generated from the original input module; absent until the menu is converted.
const CONTROL_BINDINGS := "res://generated/hype/shared/menu/controls/en/bindings.tres"
## Stand-in 4-334: the authored inventory host owns the inventory screen, so
## the script's inventory-mode actions stay unbound and the host reads the
## Inventory_Key/Inventory_Pad bindings itself.
const HOST_OWNED_ACTIONS := ["TestInventoryMode", "InventoryExit"]
## Source joystick axes 0 (X) and 1 (Y) are the left stick; Hype reads no others.
const JOY_AXES := {0: JOY_AXIS_LEFT_X, 1: JOY_AXIS_LEFT_Y}
## While aiming, the crossbow camera actions take these inputs instead.
const AIM_ROUTES := {
	"Button_Left": "BT_Camera_LookLeft",
	"Button_Right": "BT_Camera_LookRight",
	"Button_Up": "BT_Camera_LookUp",
	"Button_Down": "BT_Camera_LookDown",
	"Button_Hit": "BT_Camera_TireArbalete",
}
## Earlier Astrolabe settings stored five actions by script name.
const LEGACY_KEYBOARD := {"Button_Left": "Left_Key", "Button_Right": "Right_Key", "Button_Up": "Up_Key", "Button_Down": "Down_Key", "Button_Hit": "Action_Key"}

## Keep the original mixer units in settings; Voice -20 is its silent UI sentinel.
var values: Dictionary = AUDIO_DEFAULTS.duplicate(true)
var image_values: Dictionary = IMAGE_DEFAULTS.duplicate(true)
var control_bindings: Resource
## Options entry -> key id (keycode + location << 32) or pad button index.
var key_bindings: Dictionary = {}
var pad_bindings: Dictionary = {}
## What hype-settings.cfg holds. The source applies a change at once but only
## writes Current.cfg when play starts, on Resume Game and at shutdown.
var committed_key_bindings: Dictionary = {}
var committed_pad_bindings: Dictionary = {}
## Stick actions -> Vector2(press, hold) deadzones. The source ORs
## `Action & wider range` into the action, so once pressed it stays pressed
## until the stick leaves the wider range (Button_Up presses at Y <= -30 and
## holds to -20; Button_SpeedUp presses at -90 and holds to -75).
var _axis_bands: Dictionary = {}
var reverse_stereo := false
var _aim_mode := false
var _reversed_audio_cache: Dictionary = {}
var _stereo_test_player: AudioStreamPlayer

func _ready() -> void:
	# The release band must follow the stick in menus and pauses too.
	process_mode = Node.PROCESS_MODE_ALWAYS
	for bus_name in BUSES.values():
		_ensure_bus(bus_name)
	if ResourceLoader.exists(CONTROL_BINDINGS):
		control_bindings = load(CONTROL_BINDINGS)
	_reset_control_binding_data()
	var file := ConfigFile.new()
	if file.load(FILE_PATH) == OK:
		var stored_audio_scale_version := int(file.get_value("meta", "audio_scale_version", 0))
		for group in BUSES:
			var maximum := 107.0 if group == "Voices" else 127.0
			var stored_value: float
			if stored_audio_scale_version < AUDIO_SCALE_VERSION:
				stored_value = float(file.get_value("audio", group, 1.0)) * 127.0
			else:
				stored_value = float(file.get_value("audio", group, AUDIO_DEFAULTS[group]))
			values[group] = clampf(stored_value, 0.0, maximum)
		reverse_stereo = bool(file.get_value("audio", "reverse_stereo", false))
		image_values["shadow_quality"] = clampi(int(file.get_value("video", "shadow_quality", IMAGE_DEFAULTS.shadow_quality)), 1, 4)
		image_values["screen_framing"] = clampi(int(file.get_value("video", "screen_framing", IMAGE_DEFAULTS.screen_framing)), 60, 100)
		_load_control_bindings(file)
	_mark_committed()
	get_tree().node_added.connect(_on_node_added)
	configure_gameplay_inputs(false)
	_apply()
	_apply_image_settings_to_tree(get_tree().root)
	_apply_reverse_stereo_to_tree(get_tree().root)

func get_volume(group: String) -> float:
	var source_value := float(values.get(group, AUDIO_DEFAULTS.get(group, 100.0)))
	return -20.0 if group == "Voices" and source_value <= 0.0 else source_value

## Stores original source units. Voices' negative slider values all mean zero gain.
func set_volume(group: String, source_value: float) -> void:
	if not BUSES.has(group):
		return
	var minimum := -20.0 if group == "Voices" else 0.0
	var maximum := 107.0 if group == "Voices" else 127.0
	values[group] = maxf(0.0, clampf(source_value, minimum, maximum))
	_apply_group(group)
	_save()

func reset_audio() -> void:
	for group in BUSES:
		values[group] = AUDIO_DEFAULTS[group]
	_apply()
	_save()

func get_image_setting(name: String) -> float:
	return float(image_values.get(name, IMAGE_DEFAULTS.get(name, 0.0)))

func set_image_setting(name: String, value: float) -> void:
	if not IMAGE_DEFAULTS.has(name):
		return
	match name:
		"shadow_quality":
			image_values[name] = clampi(roundi(value), 1, 4)
		"screen_framing":
			image_values[name] = clampi(roundi(value), 60, 100)
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

func get_control_bindings() -> Resource:
	return control_bindings

## The entry's current input: a key id, a pad button, or -1.
func get_entry_binding(entry: String) -> int:
	if key_bindings.has(entry):
		return int(key_bindings[entry])
	return int(pad_bindings.get(entry, -1))

## Name artwork ({label, texture, rect}) for the entry's bound input.
func get_entry_input_name(entry: String) -> Dictionary:
	if control_bindings == null:
		return {}
	if key_bindings.has(entry):
		return control_bindings.key_names.get(int(key_bindings[entry]), {})
	if pad_bindings.has(entry):
		return control_bindings.pad_names.get(int(pad_bindings[entry]), {})
	return {}

## The key id for a physical key event; modifier keys keep their side.
func key_id(event: InputEventKey) -> int:
	var code := int(event.physical_keycode)
	var sided := code + (int(event.location) << 32)
	if control_bindings != null and control_bindings.key_names.has(sided):
		return sided
	return code

## IPT_fn_bChangeIPTValue accepts only inputs with a display name.
func can_bind_key(id: int) -> bool:
	return control_bindings != null and control_bindings.key_names.has(id)

## Bind an input to an options entry. An entry of the same kind already using
## it receives this entry's previous input (FUN_004a4be0 swap, mode 1).
func bind_entry(entry: String, input: int) -> Dictionary:
	var is_key := key_bindings.has(entry)
	if not is_key and not pad_bindings.has(entry):
		return {"ok": false}
	if (is_key and not can_bind_key(input)) or (not is_key and not control_bindings.pad_names.has(input)):
		return {"ok": false}
	var bindings: Dictionary = key_bindings if is_key else pad_bindings
	var previous := int(bindings[entry])
	var swapped := ""
	for other in bindings:
		if other != entry and int(bindings[other]) == input:
			swapped = str(other)
			bindings[other] = previous
			break
	bindings[entry] = input
	configure_gameplay_inputs(_aim_mode)
	return {"ok": true, "swapped": swapped}

## DefaultValues reloads every binding from the source defaults.
func reset_control_bindings() -> void:
	_reset_control_binding_data()
	configure_gameplay_inputs(_aim_mode)

## Write the applied bindings, at the source's Current.cfg write points.
func commit_control_bindings() -> bool:
	if key_bindings == committed_key_bindings and pad_bindings == committed_pad_bindings:
		return true
	_mark_committed()
	return _save()

func _mark_committed() -> void:
	committed_key_bindings = key_bindings.duplicate()
	committed_pad_bindings = pad_bindings.duplicate()

func _notification(what: int) -> void:
	# Shutdown is the third write point (FUN_004026b0).
	if what == NOTIFICATION_WM_CLOSE_REQUEST or what == NOTIFICATION_EXIT_TREE:
		if control_bindings != null:
			commit_control_bindings()

## A Ctrl/Shift/Alt binding is side specific, as the source's scancodes are.
## Real X11 key events carry the side (KeyLocation), checked on this build.
func _key_matches(id: int, key: InputEventKey) -> bool:
	return int(key.physical_keycode) == id & 0xffffffff and (id >> 32 == 0 or int(key.location) == id >> 32)

## True when a Controls binding owns this key (port shortcuts then yield).
func key_is_bound(key: InputEventKey) -> bool:
	for entry in key_bindings:
		if _key_matches(int(key_bindings[entry]), key):
			return true
	return false

## True when the event presses one of the entries' bound inputs.
func entry_event_pressed(event: InputEvent, entries: Array) -> bool:
	for entry in entries:
		if key_bindings.has(entry) and event is InputEventKey:
			var key := event as InputEventKey
			if key.pressed and not key.echo and _key_matches(int(key_bindings[entry]), key):
				return true
		if pad_bindings.has(entry) and event is InputEventJoypadButton:
			var button := event as InputEventJoypadButton
			if button.pressed and button.button_index == int(pad_bindings[entry]):
				return true
	return false

## Give every script action the inputs of the options entries it ORs together.
## While aiming, the movement/hit inputs feed the crossbow camera actions only.
func configure_gameplay_inputs(aim_mode: bool) -> void:
	_aim_mode = aim_mode
	if control_bindings == null:
		return
	_axis_bands.clear()
	for action in control_bindings.actions:
		if HOST_OWNED_ACTIONS.has(action):
			continue
		if not InputMap.has_action(action):
			InputMap.add_action(action)
		for existing in InputMap.action_get_events(action):
			if existing is InputEventKey or existing is InputEventJoypadButton or existing is InputEventJoypadMotion:
				InputMap.action_erase_event(action, existing)
		var silent: bool = (aim_mode and AIM_ROUTES.has(action)) or (not aim_mode and AIM_ROUTES.values().has(action))
		if silent:
			continue
		for entry in control_bindings.actions[action]:
			for event in _events_for_entry(str(entry)):
				if not InputMap.action_has_event(action, event):
					InputMap.action_add_event(action, event)
		# f010 ranges: the stick direction past the range's inner bound.
		for range in control_bindings.axes.get(action, []):
			var bounds: Vector3i = range
			if not JOY_AXES.has(bounds.x) or (bounds.y < 0) != (bounds.z < 0):
				continue
			var motion := InputEventJoypadMotion.new()
			motion.device = -1
			motion.axis = JOY_AXES[bounds.x]
			motion.axis_value = -1.0 if bounds.z < 0 else 1.0
			InputMap.action_add_event(action, motion)
			var press := _inner_bound(bounds)
			var hold := press
			for held in control_bindings.hold_axes.get(action, []):
				var band: Vector3i = held
				if band.x == bounds.x and (band.z < 0) == (bounds.z < 0):
					hold = minf(hold, _inner_bound(band))
			_axis_bands[action] = Vector2(press, hold)
	_update_axis_bands()

static func _inner_bound(bounds: Vector3i) -> float:
	return mini(absi(bounds.y), absi(bounds.z)) / 100.0

## A pressed stick action is judged against its hold bound, a released one
## against its press bound. Input has already applied the event when _input
## runs, so the next stick event sees the band that matches the new state.
func _update_axis_bands() -> void:
	for action in _axis_bands:
		var band: Vector2 = _axis_bands[action]
		var deadzone := band.y if Input.is_action_pressed(action) else band.x
		if not is_equal_approx(InputMap.action_get_deadzone(action), deadzone):
			InputMap.action_set_deadzone(action, deadzone)

func _input(event: InputEvent) -> void:
	if not _axis_bands.is_empty() and (event is InputEventJoypadMotion or event is InputEventKey or event is InputEventJoypadButton):
		_update_axis_bands()

func _process(_delta: float) -> void:
	# Handled events skip _input; keep the band in step every frame as well.
	if not _axis_bands.is_empty():
		_update_axis_bands()

func _events_for_entry(entry: String) -> Array[InputEvent]:
	var events: Array[InputEvent] = []
	if key_bindings.has(entry):
		var id := int(key_bindings[entry])
		var key := InputEventKey.new()
		key.physical_keycode = id & 0xffffffff
		key.location = id >> 32
		events.append(key)
	elif pad_bindings.has(entry):
		var button := InputEventJoypadButton.new()
		button.device = -1
		button.button_index = int(pad_bindings[entry])
		events.append(button)
	else:
		var item: Dictionary = control_bindings.entry(entry)
		if str(item.get("kind", "")) == "pad_direction":
			var direction := InputEventJoypadButton.new()
			direction.device = -1
			direction.button_index = int(item["default"])
			events.append(direction)
	return events

func _reset_control_binding_data() -> void:
	key_bindings.clear()
	pad_bindings.clear()
	if control_bindings == null:
		return
	for item in control_bindings.entries:
		match str(item["kind"]):
			"key":
				key_bindings[str(item["name"])] = int(item["default"])
			"pad":
				pad_bindings[str(item["name"])] = int(item["default"])

func _load_control_bindings(file: ConfigFile) -> void:
	var stored_keys: Variant = file.get_value("controls", "keys", {})
	if not file.has_section_key("controls", "keys"):
		var legacy: Variant = file.get_value("controls", "keyboard", {})
		stored_keys = {}
		if legacy is Dictionary:
			for action in LEGACY_KEYBOARD:
				if legacy.has(action):
					stored_keys[LEGACY_KEYBOARD[action]] = int(legacy[action])
	if stored_keys is Dictionary:
		for entry in stored_keys:
			if key_bindings.has(entry) and can_bind_key(int(stored_keys[entry])):
				key_bindings[entry] = int(stored_keys[entry])
	var stored_pad: Variant = file.get_value("controls", "pad", {})
	if stored_pad is Dictionary:
		for entry in stored_pad:
			if pad_bindings.has(entry) and control_bindings.pad_names.has(int(stored_pad[entry])):
				pad_bindings[entry] = int(stored_pad[entry])

func _apply() -> void:
	for group in BUSES:
		_apply_group(group)

func _apply_group(group: String) -> void:
	var mixer := NativeAudioMixer.shared(self) as NativeAudioMixer
	var module := NativeAudioMixer.module_for_bus(StringName(BUSES[group]))
	mixer.set_preference(module, int(values[group]))

func _on_node_added(node: Node) -> void:
	# A scene can remove/free an added node before the deferred call is drained.
	# Keep an ObjectID in MessageQueue instead of a Node reference that can decay
	# to Nil and fail conversion to the typed callback argument.
	call_deferred("_apply_settings_to_node_by_id", node.get_instance_id())

func _apply_settings_to_node_by_id(instance_id: int) -> bool:
	var candidate: Object = instance_from_id(instance_id)
	if not candidate is Node:
		return false
	var node := candidate as Node
	if not node.is_inside_tree():
		return false
	# Wait until the instanced scene has finished assigning exported properties.
	_apply_settings_to_node(node)
	return true

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

func _register_image_settings_node(node: Node) -> void:
	if node is Light3D and not node.is_in_group(LIGHT_SETTINGS_GROUP):
		node.add_to_group(LIGHT_SETTINGS_GROUP)
	elif node is Camera3D and not node.is_in_group(CAMERA_SETTINGS_GROUP):
		node.add_to_group(CAMERA_SETTINGS_GROUP)

func _apply_shadow_to_light(light: Light3D) -> void:
	if not light.has_meta(ORIGINAL_SHADOW_META):
		var source_shadow_enabled := light.shadow_enabled
		var source_wrapper := light.get_parent()
		if source_wrapper != null and source_wrapper.get("source_path") != null and not light.light_negative:
			source_shadow_enabled = true
		light.set_meta(ORIGINAL_SHADOW_META, source_shadow_enabled)
	light.shadow_enabled = bool(light.get_meta(ORIGINAL_SHADOW_META))

func _apply_screen_framing(camera: Camera3D) -> void:
	# FUN_004054c0 sets the centered source viewport rectangle to [100 - value, value].
	# Its aperture fraction is exactly 2*value - 100 percent. Scale tan(FOV/2)
	# (or orthographic size) by that fraction to preserve the source projection.
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
	file.set_value("meta", "audio_scale_version", AUDIO_SCALE_VERSION)
	for group in BUSES:
		file.set_value("audio", group, values[group])
	file.set_value("audio", "reverse_stereo", reverse_stereo)
	for setting in IMAGE_DEFAULTS:
		file.set_value("video", setting, image_values[setting])
	if control_bindings != null:
		file.set_value("controls", "keys", committed_key_bindings)
		file.set_value("controls", "pad", committed_pad_bindings)
	var error := file.save(FILE_PATH)
	if error != OK:
		push_warning("Could not save Hype preferences: " + error_string(error))
		return false
	return true
