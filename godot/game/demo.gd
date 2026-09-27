extends Node3D
## Game session using the normally exported, already saved level.
## Hype's generated Brain owns gameplay choices. The provisional Load Game
## path restores the measured English-save state until general save/load lands.

const LEVEL := "res://generated/hype/levels/manoir/level.tscn"
const BRIDGE_GATE := "world/0/15/iMGrille_Manoir_Pont"
const GATE_LIFT := 5.2
const MANOIR_START = preload("res://game/manoir_start.gd")

signal loading_stage_changed(stage: String, elapsed_ms: float)

@onready var demo_camera: Camera3D = $DemoCamera
@onready var status_label: Label = $Hud/Panel/Margin/Rows/Status
@onready var mana_label: Label = $Hud/Panel/Margin/Rows/Mana
@onready var content_cache: Node = get_node_or_null("/root/GameContent")

var level: Node3D
var actor: CharacterBody3D
var brain: Node
var ready_to_play := false
var loading := false
var loading_stage := "idle"
var load_started_usec := 0
var load_timings_ms: Dictionary = {}
var saved_max_fps := -1

func _ready() -> void:
    load_started_usec = Time.get_ticks_usec()
    stage("session", "Loading saved game...")
    mana_label.text = ""
    call_deferred("open_level")

func open_level() -> void:
    if loading or level != null:
        return
    loading = true
    if not ResourceLoader.exists(LEVEL):
        fail("The manor has not been generated. See godot/README.md.")
        return
    var packed: PackedScene = content_cache.call("get_level", LEVEL) as PackedScene if content_cache != null else null
    if packed == null:
        stage("resource_loading", "Loading the manor...")
        limit_loading_frames()
        var request := ResourceLoader.load_threaded_request(
            LEVEL, "PackedScene", true, ResourceLoader.CACHE_MODE_REUSE)
        if request != OK:
            fail("The manor scene could not begin loading.")
            return
        while is_inside_tree() and ResourceLoader.load_threaded_get_status(LEVEL) == ResourceLoader.THREAD_LOAD_IN_PROGRESS:
            await get_tree().process_frame
        if not is_inside_tree():
            return
        if ResourceLoader.load_threaded_get_status(LEVEL) != ResourceLoader.THREAD_LOAD_LOADED:
            fail("The manor scene failed to load.")
            return
        packed = ResourceLoader.load_threaded_get(LEVEL) as PackedScene
        restore_frame_limit()
    if packed == null:
        fail("The generated manor scene could not be opened.")
        return
    if content_cache != null:
        content_cache.call("retain_level", LEVEL, packed)
    stage("resource_ready", "Preparing the manor...")
    level = packed.instantiate() as Node3D
    if level == null:
        fail("The generated manor scene is invalid.")
        return
    stage("instantiated", "Restoring saved game...")
    actor = level.get_node_or_null("world/0/0/Hype") as CharacterBody3D
    brain = actor.get_node_or_null("Brain") if actor != null else null
    if actor == null or brain == null:
        fail("The generated manor is missing Hype or his Brain.")
        return
    var gate := level.get_node_or_null(BRIDGE_GATE) as CharacterBody3D
    var gate_brain := gate.get_node_or_null("Brain") if gate != null else null
    if gate == null or gate_brain == null:
        fail("The generated manor is missing the bridge gate.")
        return
    var fixture := MANOIR_START.load_fixture()
    if fixture.is_empty():
        fail("The English-save starting state is missing.")
        return
    brain.set("automatic", false)
    gate_brain.set("automatic", false)
    actor.set_physics_process(false)
    add_child(level)
    stage("tree_ready", "Restoring saved game...")
    # Temporary gameplay workaround: retract the gate by its full height.
    # Its source opening AI is still missing; this is not restored save data.
    gate.position.y += GATE_LIFT
    actor.set_physics_process(false)
    var fixture_error: String = MANOIR_START.apply(actor, brain, fixture)
    if not fixture_error.is_empty():
        fail(fixture_error)
        return
    demo_camera.set("target", actor)
    demo_camera.call("follow")
    demo_camera.make_current()
    stage("restored", "Preparing to play...")
    # The first physics tick enters the saved world before movement starts.
    # A second tick lets CharacterBody3D update native contact before Brain runs.
    await get_tree().physics_frame
    if not is_inside_tree():
        return
    actor.set_physics_process(true)
    await get_tree().physics_frame
    if not is_inside_tree():
        return
    stage("physics_ready", "Starting Hype...")
    brain.set("automatic", true)
    if not bool(brain.call("start")):
        fail("Hype's compiled behavior could not start.")
        return
    await get_tree().process_frame
    if not is_inside_tree():
        return
    ready_to_play = true
    loading = false
    stage("playable", "Explore the manor")

func stage(name: String, message: String) -> void:
    loading_stage = name
    status_label.text = message
    var elapsed_ms := (Time.get_ticks_usec() - load_started_usec) / 1000.0
    load_timings_ms[name] = elapsed_ms
    loading_stage_changed.emit(name, elapsed_ms)
    if OS.get_environment("ASTROLABE_PROFILE_LOADING") == "1":
        print("GAME_LOAD ", JSON.stringify({"stage": name, "elapsed_ms": elapsed_ms, "static_memory_bytes": OS.get_static_memory_usage()}))

func limit_loading_frames() -> void:
    if Engine.max_fps == 0 or Engine.max_fps > 30:
        saved_max_fps = Engine.max_fps
        Engine.max_fps = 30

func restore_frame_limit() -> void:
    if saved_max_fps >= 0:
        Engine.max_fps = saved_max_fps
        saved_max_fps = -1

func _process(_delta: float) -> void:
    if not ready_to_play or not is_instance_valid(actor) or not is_instance_valid(brain):
        return
    var magick: Resource = actor.get("source_magick")
    if magick != null:
        mana_label.text = "Fire  %d / %d mana" % [int(magick.get("current_points")), int(magick.get("maximum_points"))]
    var failure := str(brain.get("unsupported_failure"))
    if not failure.is_empty():
        status_label.text = "This behavior is not supported yet. Press R to reload."

func _unhandled_key_input(event: InputEvent) -> void:
    var key := event as InputEventKey
    if key == null or not key.pressed or key.echo:
        return
    if key.physical_keycode == KEY_R:
        get_viewport().set_input_as_handled()
        leave_session("res://game/demo.tscn")
    elif key.physical_keycode == KEY_ESCAPE:
        get_viewport().set_input_as_handled()
        leave_session("res://game/bootstrap.tscn")

func leave_session(destination: String) -> void:
    ready_to_play = false
    if is_instance_valid(brain):
        brain.call("stop")
    if is_instance_valid(actor):
        actor.set_physics_process(false)
    for action in InputMap.get_actions():
        if str(action).begins_with("Button_"):
            Input.action_release(action)
    get_tree().change_scene_to_file(destination)

func _exit_tree() -> void:
    restore_frame_limit()
    if is_instance_valid(brain):
        brain.call("stop")

func fail(message: String) -> void:
    push_error(message)
    restore_frame_limit()
    if is_instance_valid(brain):
        brain.call("stop")
    if is_instance_valid(actor):
        actor.set_physics_process(false)
    if is_instance_valid(level):
        if level.get_parent() == self:
            remove_child(level)
        level.queue_free()
    level = null
    actor = null
    brain = null
    ready_to_play = false
    loading = false
    stage("failed", message)
