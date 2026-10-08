extends Node3D
const NativeCharacter = preload("res://addons/astrolabe/character/character.gd")
const NativeAudioEmitter = preload("res://addons/astrolabe/audio/emitter.gd")
## Timing and actor/audio bindings for compiled behavior code. No source trees are loaded.
signal normal_tick
signal reflex_tick
signal normal_schedule_tick
signal reflex_schedule_tick
signal sound_requested(identity: String)
@export var automatic: bool = true
@export_node_path("Node3D") var main_actor_path: NodePath
@export_node_path("Node3D") var sound_emitter_path: NodePath = ^"../SoundEmitter"
@export var current_normal: int = -1
@export var current_reflex: int = -1
var main_actor: Node3D
var sound_emitter: Node3D
var running: bool = false
var unsupported_failure: String = ""
var random_source := RandomNumberGenerator.new()
var revision: int = 0
var clock_milliseconds: float = 0.0
var frame_seconds: float = 0.0
# Native coroutine cancellation tokens, separately owned by the two behaviors.
var normal_revision: int = 0
var reflex_revision: int = 0
var normal_action_revision: int = 0
var reflex_action_revision: int = 0
var normal_rule_claimed: bool = false
var reflex_rule_claimed: bool = false
var reflex_controls_tick: bool = false
var _normal_pending: bool = false
var _reflex_pending: bool = false
var _normal_deferred: bool = false
var _reflex_deferred: bool = false
var _source_hierarchy_move: bool = false

func _init() -> void:
	# Native physics ordering: decisions select the action/facing consumed by
	# the character in this same tick. Saved authored priorities can override it.
	process_physics_priority = -10

func _ready() -> void:
	_bind_inputs()
	if not main_actor_path.is_empty(): main_actor = get_node_or_null(main_actor_path) as Node3D
	sound_emitter = get_node_or_null(sound_emitter_path) as Node3D
	# A child's _ready precedes its character's initial pose selection. Start
	# after the parent so that source initialization can select the first action.
	if automatic and main_actor != null: _start_automatic.call_deferred(revision)

func _start_automatic(token: int) -> void:
	if automatic and is_inside_tree() and not running and token == revision: start()

func _bind_inputs() -> void:
	pass

## Compiled subclasses list their stable external actor references in the
## same order as their actor_brain_* bindings. Level instances resolve them.
func source_external_actor_brains() -> PackedStringArray:
	return PackedStringArray()

## Hype keeps the selected armor definition shared across actors in one level.
## The imported level root owns the native Resource; each Brain finds the
## nearest owning scene through its ordinary parent chain.
func source_armor_state() -> Resource:
	var owner_node: Node = self
	while is_instance_valid(owner_node):
		if owner_node.has_meta("source_armor_state"):
			var state: Resource = owner_node.get_meta("source_armor_state") as Resource
			if is_instance_valid(state):
				return state
			return null
		owner_node = owner_node.get_parent()
	return null

func source_select_armor(index: int) -> bool:
	var state := source_armor_state()
	if not is_instance_valid(state) or not state.has_method("select_source_index"):
		return false
	return bool(state.call("select_source_index", index))

func adopt_input(action: StringName, physical_key: Key = KEY_NONE) -> void:
	# Existing InputMap entries belong to the project, including empty entries
	# deliberately used to disable a control. Defaults are adopted only once.
	if InputMap.has_action(action): return
	InputMap.add_action(action)
	if physical_key == KEY_NONE:
		push_warning("Assign an InputMap binding for %s" % action)
		return
	var event := InputEventKey.new()
	event.physical_keycode = physical_key
	InputMap.action_add_event(action, event)

func _physics_process(delta: float) -> void:
	if automatic and running: advance(delta)

func _exit_tree() -> void:
	if not _source_hierarchy_move: stop()

## A synchronous native reparent preserves the actor's decision continuation.
func source_begin_hierarchy_move() -> void:
	_source_hierarchy_move = true

func source_end_hierarchy_move() -> void:
	_source_hierarchy_move = false

func start() -> bool:
	if not is_instance_valid(main_actor) or not is_instance_valid(sound_emitter):
		push_error("Compiled behavior requires main actor and sound emitter bindings")
		return false
	stop()
	unsupported_failure = ""
	clock_milliseconds = 0.0
	running = true
	_normal_pending = true
	_reflex_pending = true
	advance(0.0)
	return true

func stop() -> void:
	revision += 1
	normal_revision += 1
	reflex_revision += 1
	running = false
	_normal_pending = false
	_reflex_pending = false
	normal_tick.emit()
	reflex_tick.emit()
	normal_schedule_tick.emit()
	reflex_schedule_tick.emit()

func fail_unsupported(context: String) -> void:
	unsupported_failure = context
	push_error("Unsupported compiled behavior: " + context)
	stop()

func _source_sound_bus(bus_name: StringName) -> int:
	var bus := AudioServer.get_bus_index(bus_name)
	if bus < 0:
		AudioServer.add_bus()
		bus = AudioServer.bus_count - 1
		AudioServer.set_bus_name(bus, bus_name)
	return bus

func _source_audio_fades() -> Dictionary:
	var root := get_tree().root
	var fades: Variant = null
	if root.has_meta(&"astrolabe_source_audio_fades"):
		fades = root.get_meta(&"astrolabe_source_audio_fades")
	if not fades is Dictionary:
		fades = {}
		root.set_meta(&"astrolabe_source_audio_fades", fades)
	return fades

func _source_sound_fade_bus(bus_name: StringName, target: int, duration_ms: int) -> void:
	assert(duration_ms >= 0)
	var bus := _source_sound_bus(bus_name)
	var key := String(bus_name)
	var fades := _source_audio_fades()
	var previous_id := int(fades.get(key, 0))
	if previous_id != 0:
		var previous := instance_from_id(previous_id) as Tween
		if is_instance_valid(previous) and previous.is_running(): previous.kill()
	var start_level := clampf(db_to_linear(AudioServer.get_bus_volume_db(bus)) * 127.0, 0.0, 127.0)
	var target_level := float(clampi(target, 0, 127))
	var set_level := func(value: float) -> void:
		AudioServer.set_bus_volume_db(bus, linear_to_db(maxf(value / 127.0, 0.000001)))
	if duration_ms == 0:
		set_level.call(target_level)
		fades.erase(key)
		return
	var tween := get_tree().create_tween()
	var tween_id := tween.get_instance_id()
	fades[key] = tween_id
	tween.tween_method(set_level, start_level, target_level, float(duration_ms) / 1000.0).set_trans(Tween.TRANS_LINEAR)
	tween.finished.connect(func() -> void:
		if int(fades.get(key, 0)) == tween_id:
			fades.erase(key))

## Proc_SND_FadeMusic(target 0..127, duration in milliseconds).
func sound_fade_music(target: int, duration_ms: int) -> void:
	_source_sound_fade_bus(&"Astrolabe Music", target, duration_ms)

## Proc_SND_FadeVoice targets the source voice mixer channel.
func sound_fade_voice(target: int, duration_ms: int) -> void:
	_source_sound_fade_bus(&"Astrolabe Voice", target, duration_ms)

## Proc_SND_FadeSound targets the source effects mixer channel.
func sound_fade_sound(target: int, duration_ms: int) -> void:
	_source_sound_fade_bus(&"Astrolabe Effects", target, duration_ms)

## Proc_SND_FadeAll applies the same target and duration to all three source
## mixer channels: music, effects and voices.
func sound_fade_all(target: int, duration_ms: int) -> void:
	for bus_name in [&"Astrolabe Music", &"Astrolabe Effects", &"Astrolabe Voice"]:
		_source_sound_fade_bus(bus_name, target, duration_ms)

## Proc_SND_KillMusic stops active players on the source music channel.
func sound_kill_music() -> void:
	var channel := get_tree().root.get_node_or_null("AstrolabeMusic")
	if channel != null: channel.call("kill_all")

func source_execute_fade(from_alpha: int, to_alpha: int, duration_seconds: float) -> bool:
	if not is_finite(duration_seconds) or duration_seconds < 0.0:
		return false
	var screen := source_ui()
	if screen == null or not screen.has_method("source_execute_fade"):
		return false
	return bool(screen.call("source_execute_fade", from_alpha, to_alpha, duration_seconds))

func advance(seconds: float) -> void:
	assert(is_finite(seconds) and seconds >= 0.0)
	if not running: return
	# Inactive placed actors retain their selected behaviors and native awaits.
	# Initial inactive actors enter their first behavior only upon activation.
	var body := get_parent() as NativeCharacter
	if body != null and not body.source_object_active: return
	frame_seconds = seconds
	clock_milliseconds += seconds * 1000.0
	reflex_controls_tick = false
	_step_reflex()
	if running and not reflex_controls_tick: _step_normal()

func source_time_elapsed(start_ms: int, duration_ms: int) -> bool:
	assert(duration_ms >= 0)
	return clock_milliseconds - float(start_ms) >= float(duration_ms)

func distance_to_main_actor() -> float:
	assert(is_instance_valid(main_actor))
	return global_position.distance_to(main_actor.global_position)

func random_int(lower: int, upper: int) -> int:
	assert(upper > lower)
	return random_source.randi_range(lower, upper - 1)

func send_sound(event: Resource) -> void:
	if event == null: return
	assert(is_instance_valid(sound_emitter))
	event.execute(sound_emitter)
	sound_requested.emit(event.identity)

## Proc127 with Ultra uses the selected actor's effects component.
func send_sound_to_actor(actor: Node3D, event: Resource) -> bool:
	if not is_instance_valid(actor): return false
	if event == null: return true
	var emitter := actor.get_node_or_null("SoundEmitter")
	if emitter == null or not emitter.has_method("play_voice"): return false
	event.execute(emitter)
	sound_requested.emit(event.identity)
	return true

## Proc129 dispatches immediately on shared nonspatial music, then continues.
## A null source event returns an ignored error in the original procedure.
func send_music(event: Resource) -> void:
	if event == null: return
	var channel = load("res://addons/astrolabe/audio/emitter.gd").music_channel(self)
	if not channel.request_music(event):
		fail_unsupported(channel.music_failure)
		return
	sound_requested.emit(event.identity)

func source_switch_microphone(selected: Variant) -> bool:
	if selected != null and (typeof(selected) != TYPE_OBJECT or not is_instance_valid(selected) or not (selected is Node3D)): return false
	var actor: Node3D = main_actor if selected == null else selected as Node3D
	var basis: Variant = NativeAudioEmitter.microphone_basis_for(actor)
	return NativeAudioEmitter.switch_micro_perso(self, selected as Node3D, main_actor, basis)

## Proc413's operands belong to the caller; only effects playback/fade use actor.
func sound_fade_in_object(actor: Node3D, target: int, duration_ms: int, event: Resource) -> bool:
	if not is_instance_valid(actor): return false
	var emitter := actor.get_node_or_null("SoundEmitter")
	if emitter == null or not emitter.has_method("fade_in_event"): return false
	emitter.call("fade_in_event", event, target, duration_ms)
	if event != null: sound_requested.emit(event.identity)
	return true

func source_ui() -> Node:
	var ancestor := get_parent()
	while ancestor != null:
		var screen := ancestor.get_node_or_null("AstrolabeUI")
		if screen != null and screen.has_method("source_run_dialog"): return screen
		ancestor = ancestor.get_parent()
	return null

## Proc_SetGameState is session-wide, so resolve the authored game root rather
## than storing its state on this actor's Brain or on a level component.
func source_game_session() -> Node:
	var ancestor := get_parent()
	while ancestor != null:
		if ancestor.has_method("source_set_game_state"): return ancestor
		ancestor = ancestor.get_parent()
	return null

func character() -> NativeCharacter:
	var body := get_parent() as NativeCharacter
	assert(body != null, "Action calls require the native character component")
	return body

func active_normal(token: int) -> bool:
	return running and token == normal_revision

## Source behavior-query conditions compare the actor's current slot value with
## the referenced behavior ID. The reference's model is not part of that test.
func source_is_in_behavior(group: StringName, target: int) -> bool:
	if target < 0: return false
	if group == &"normal": return current_normal >= 0 and current_normal == target
	if group == &"reflex": return current_reflex >= 0 and current_reflex == target
	return false

func change_normal(target: int, defer_entry: bool = false) -> bool:
	if current_normal == target: return false
	current_normal = target
	normal_revision += 1
	_normal_pending = true
	_normal_deferred = defer_entry
	# Release the old coroutine even when the new behavior has no schedule.
	# Its token is already invalid, so no cancelled side effects can resume.
	normal_tick.emit()
	normal_schedule_tick.emit()
	return true

func _step_normal() -> void:
	normal_rule_claimed = false
	_normal_deferred = false
	if not _normal_pending: normal_tick.emit()
	var entries := 0
	while running and _normal_pending and not _normal_deferred:
		entries += 1
		if entries > 64:
			push_error("Normal behavior changes never reach a physics tick")
			stop()
			return
		_normal_pending = false
		normal_rule_claimed = false
		_enter_normal(normal_revision)

func _enter_normal(_token: int) -> void:
	# The generated subclass enters its ordinary GDScript behavior coroutine.
	pass

func active_reflex(token: int) -> bool:
	return running and token == reflex_revision

func change_reflex(target: int, defer_entry: bool = false) -> bool:
	if current_reflex == target: return false
	current_reflex = target
	reflex_revision += 1
	_reflex_pending = true
	_reflex_deferred = defer_entry
	reflex_tick.emit()
	reflex_schedule_tick.emit()
	return true

func _step_reflex() -> void:
	reflex_rule_claimed = false
	_reflex_deferred = false
	if not _reflex_pending: reflex_tick.emit()
	var entries := 0
	while running and _reflex_pending and not _reflex_deferred:
		entries += 1
		if entries > 64:
			push_error("Reflex behavior changes never reach a physics tick")
			stop()
			return
		_reflex_pending = false
		reflex_rule_claimed = false
		_enter_reflex(reflex_revision)

func _enter_reflex(_token: int) -> void:
	# The generated subclass enters its ordinary GDScript behavior coroutine.
	pass
