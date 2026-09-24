extends Node3D
const NativeCharacter = preload("res://addons/astrolabe/character/character.gd")
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
var music_fade: Tween
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
	stop()

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

## Proc_SND_FadeMusic(target 0..127, duration in milliseconds). The executable
## passes source (target, duration) to its mixer helper in reversed stack order.
func sound_fade_music(target: int, duration_ms: int) -> void:
	assert(duration_ms >= 0)
	var bus := AudioServer.get_bus_index(&"Astrolabe Music")
	if bus < 0:
		AudioServer.add_bus()
		bus = AudioServer.bus_count - 1
		AudioServer.set_bus_name(bus, &"Astrolabe Music")
	if music_fade != null and music_fade.is_running(): music_fade.kill()
	var goal := linear_to_db(maxf(float(clampi(target, 0, 127)) / 127.0, 0.000001))
	if duration_ms <= 1:
		AudioServer.set_bus_volume_db(bus, goal)
	else:
		music_fade = create_tween()
		music_fade.tween_method(
			func(value: float): AudioServer.set_bus_volume_db(bus, value),
			AudioServer.get_bus_volume_db(bus), goal, float(duration_ms) / 1000.0)

func advance(seconds: float) -> void:
	assert(is_finite(seconds) and seconds >= 0.0)
	if not running: return
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

func character() -> NativeCharacter:
	var body := get_parent() as NativeCharacter
	assert(body != null, "Action calls require the native character component")
	return body

func active_normal(token: int) -> bool:
	return running and token == normal_revision

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
