extends Node3D
## Timing and actor/audio bindings for compiled behavior code. No source trees are loaded.
signal advanced
signal sound_requested(identity: String)
@export var automatic: bool = true
@export_node_path("Node3D") var main_actor_path: NodePath
@export_node_path("Node3D") var sound_emitter_path: NodePath = ^"../SoundEmitter"
@export var current_normal: int = -1
@export var current_reflex: int = -1
var main_actor: Node3D
var sound_emitter: Node3D
var running: bool = false
var revision: int = 0
var clock_milliseconds: float = 0.0

func _ready() -> void:
    if not main_actor_path.is_empty(): main_actor = get_node_or_null(main_actor_path) as Node3D
    sound_emitter = get_node_or_null(sound_emitter_path) as Node3D
    if automatic and main_actor != null: start()

func _physics_process(delta: float) -> void:
    if automatic and running: advance(delta)

func _exit_tree() -> void:
    stop()

func start() -> bool:
    if not is_instance_valid(main_actor) or not is_instance_valid(sound_emitter):
        push_error("Compiled behavior requires main actor and sound emitter bindings")
        return false
    stop()
    clock_milliseconds = 0.0
    running = true
    _run(revision)
    return true

func stop() -> void:
    revision += 1
    running = false
    advanced.emit()

func active(token: int) -> bool:
    return running and token == revision

func advance(seconds: float) -> void:
    assert(is_finite(seconds) and seconds >= 0.0)
    if not running: return
    clock_milliseconds += seconds * 1000.0
    advanced.emit()

func wait_milliseconds(duration: float, token: int) -> bool:
    assert(is_finite(duration) and duration >= 0.0)
    # Hype captures the duration when entering a blocking action, in integer ms.
    var deadline := clock_milliseconds + int(duration)
    while active(token) and clock_milliseconds + 0.0000001 < deadline:
        await advanced
    return active(token)

func next_tick(token: int) -> bool:
    await advanced
    return active(token)

func distance_to_main_actor() -> float:
    assert(is_instance_valid(main_actor))
    return global_position.distance_to(main_actor.global_position)

func send_sound(event: Resource) -> void:
    if event == null: return
    assert(is_instance_valid(sound_emitter))
    event.execute(sound_emitter)
    sound_requested.emit(event.identity)

func _run(_token: int) -> void:
    # Generated subclasses provide direct calls to their compiled schedules/rules.
    pass
