extends Node3D
## Game session using the normally exported, already saved level.
## Hype's generated Brain owns gameplay choices. The measured English state
## seeds the manor; a native slot restores supported mutable player fields.

const LEVEL := "res://generated/hype/levels/manoir/level.tscn"
const GATE_ACTOR := "world/0/15/iMGrille_Manoir_Pont"
const MANOIR_START = preload("res://game/manoir_start.gd")
const SAVE_STATE = preload("res://game/save_state.gd")
@export var new_game: bool = false

signal loading_stage_changed(stage: String, elapsed_ms: float)

@onready var demo_camera: Camera3D = $DemoCamera
@onready var status_label: Label = $Hud/Panel/Margin/Rows/Status
@onready var mana_label: Label = $Hud/Panel/Margin/Rows/Mana
@onready var content_cache: Node = get_node_or_null("/root/GameContent")
@onready var inventory_host: Node = $InventoryHost
@onready var pause_host: Node = $PauseHost

var level: Node3D
var actor: CharacterBody3D
var brain: Node
var world_brain: Node
var ready_to_play := false
var loading := false
var loading_stage := "idle"
var load_started_usec := 0
var load_timings_ms: Dictionary = {}
var saved_max_fps := -1
var crossbow_aim_mode := false
var crossbow_aim_pitch := 0.0
var opening_brains: Array[Node] = []
var opening_failures: Dictionary = {}
var opening_camera: Camera3D

func _ready() -> void:
    inventory_host.connect("unavailable", func(message: String): status_label.text = message)
    pause_host.connect("unavailable", func(message: String): status_label.text = message)
    configure_crossbow_inputs()
    load_started_usec = Time.get_ticks_usec()
    stage("session", "Loading New Game..." if new_game else "Loading saved game...")
    mana_label.text = ""
    $Hud/Panel/Margin/Rows/Save.pressed.connect(save_game)
    if new_game:
        $Hud/Panel/Margin/Rows/Save.disabled = true
        $Hud/Panel/Margin/Rows/Controls.text = "Tab  inventory     R  restart opening     Esc  pause"
    call_deferred("open_level")

func open_level() -> void:
    if loading or level != null:
        return
    loading = true
    if not ResourceLoader.exists(LEVEL):
        fail("The manor has not been generated. See godot/README.md.")
        return
    var saved: Dictionary = {"status":"missing"} if new_game else SAVE_STATE.read()
    if saved["status"] == "error":
        fail(str(saved["message"]))
        return
    var imported: Dictionary = {"status":"missing"} if new_game or saved["status"] == "loaded" else SAVE_STATE.read_original_import()
    if imported["status"] == "error":
        fail(str(imported["message"]))
        return
    var packed: PackedScene = content_cache.call("get_level", LEVEL) as PackedScene if content_cache != null else null
    if packed == null:
        stage("resource_loading", "Loading the manor...")
        limit_loading_frames()
        # The manor reaches the same meshes through level and instance scenes.
        # Keep dependency loading in one worker to avoid competing subresource
        # cache registrations, while the session remains responsive.
        var request := ResourceLoader.load_threaded_request(
            LEVEL, "PackedScene", false, ResourceLoader.CACHE_MODE_REUSE)
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
    stage("instantiated", "Starting the opening..." if new_game else "Restoring saved game...")
    actor = level.get_node_or_null("world/0/0/Hype") as CharacterBody3D
    brain = actor.get_node_or_null("Brain") if actor != null else null
    var source_brains: Variant = level.get_meta("source_actor_brains", {})
    if source_brains is Dictionary and source_brains.has(SAVE_STATE.DUMMY):
        world_brain = level.get_node_or_null(source_brains[SAVE_STATE.DUMMY]) as Node
    if actor == null or brain == null:
        fail("The generated manor is missing Hype or his Brain.")
        return
    if new_game:
        await open_new_game()
        return
    if world_brain == null or not world_brain.get_property_list().any(func(property): return str(property["name"]) == "dsg_8"):
        fail("The generated manor is missing the saved dummy world flag.")
        return
    var gate := level.get_node_or_null(GATE_ACTOR) as CharacterBody3D
    var gate_brain := gate.get_node_or_null("Brain") if gate != null else null
    if gate == null or gate_brain == null:
        fail("The generated manor is missing iMGrille_Manoir_Pont or its Brain.")
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
    actor.set_physics_process(false)
    var fixture_error: String = MANOIR_START.apply(actor, brain, fixture)
    if not fixture_error.is_empty():
        fail(fixture_error)
        return
    var restore_state: Dictionary = saved["state"] if saved["status"] == "loaded" else imported["state"] if imported["status"] == "loaded" else {}
    if not restore_state.is_empty():
        var source_error: String = SAVE_STATE.apply(actor, level, restore_state)
        if not source_error.is_empty():
            fail(source_error)
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
    if not brain.call("start"):
        fail("Hype's compiled behavior could not start.")
        return
    gate_brain.set("automatic", true)
    if not gate_brain.call("start"):
        fail("iMGrille_Manoir_Pont's compiled behavior could not start.")
        return
    await get_tree().process_frame
    if not is_inside_tree():
        return
    ready_to_play = true
    loading = false
    var restored_save: bool = saved["status"] == "loaded" or imported["status"] == "loaded"
    var loaded_message := "Original save loaded (supported fields only)" if imported["status"] == "loaded" and saved["status"] != "loaded" else "Saved game loaded"
    stage("playable", loaded_message if restored_save else "Explore the manor")

func open_new_game() -> void:
    # Use the complete saved scene and its original initial AI. No save fixture,
    # gate lift, AI excerpt, held Brain or authored trigger advances this opening.
    for source_brain in level.find_children("Brain","Node",true,false):
        opening_brains.append(source_brain)
    var paths: Dictionary = level.get_meta("source_actor_nodes",{})
    var camera_actor: Node = level.get_node_or_null(paths["level:/persos/world/0/2"]) if paths.has("level:/persos/world/0/2") else null
    opening_camera = camera_actor.get_node_or_null("SourceCamera") as Camera3D if camera_actor != null else null
    add_child(level)
    # Existing authored view keeps the unfinished scene visible until source
    # camera AI can run and selects an actual target. It supplies no source pose.
    demo_camera.set("target",actor)
    demo_camera.call("follow")
    demo_camera.make_current()
    await get_tree().process_frame
    if not is_inside_tree(): return
    ready_to_play = true
    loading = false
    stage("opening","New Game — opening incomplete. R restarts; Esc pauses.")
    update_opening_status()

func update_opening_status() -> void:
    for source_brain in opening_brains:
        if not is_instance_valid(source_brain): continue
        var failure := str(source_brain.get("unsupported_failure"))
        if not failure.is_empty(): opening_failures[str(source_brain.get_path())] = failure
    if not opening_failures.is_empty():
        status_label.text = "Opening incomplete: %d actor behaviors stopped. R restarts; Esc pauses." % opening_failures.size()
    if is_instance_valid(opening_camera):
        var camera_brain := opening_camera.get_parent().get_node_or_null("Brain")
        if camera_brain != null and camera_brain.get("running") and is_instance_valid(opening_camera.get("targeted_actor")):
            opening_camera.make_current()

func save_game() -> void:
    if new_game or not ready_to_play or not is_instance_valid(actor):
        return
    var error: String = SAVE_STATE.write(actor, level)
    status_label.text = "Game saved" if error.is_empty() else error

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

func _process(delta: float) -> void:
    if not ready_to_play or not is_instance_valid(actor) or not is_instance_valid(brain):
        return
    if new_game:
        update_opening_status()
        return
    var magick: Resource = actor.get("source_magick")
    if magick != null:
        mana_label.text = "Fire  %d / %d mana" % [int(magick.get("current_points")), int(magick.get("maximum_points"))]
    var inventory: Resource = actor.get("source_inventory")
    if inventory != null and int(inventory.get("selected_weapon")) == 80:
        mana_label.text = "Crossbow  %d bolts" % int(inventory.call("quantity", 80))
    var aiming := inventory != null and int(inventory.get("selected_weapon")) == 80 and Input.is_action_pressed("BT_Camera_Look")
    if aiming != crossbow_aim_mode:
        set_crossbow_aim_mode(aiming)
    if crossbow_aim_mode:
        if Input.is_action_pressed("BT_Camera_LookUp"): crossbow_aim_pitch = minf(crossbow_aim_pitch + 45.0 * delta, 35.0)
        if Input.is_action_pressed("BT_Camera_LookDown"): crossbow_aim_pitch = maxf(crossbow_aim_pitch - 45.0 * delta, -35.0)
        demo_camera.crossbow_aim_pitch_degrees = crossbow_aim_pitch
        update_crossbow_aim_target()
    var failure := str(brain.get("unsupported_failure"))
    if not failure.is_empty():
        status_label.text = "This behavior is not supported yet. Press R to reload."

func configure_crossbow_inputs() -> void:
    for action in ["BT_Camera_Look", "BT_Camera_LookLeft", "BT_Camera_LookRight", "BT_Camera_LookUp", "BT_Camera_LookDown", "BT_Camera_TireArbalete"]:
        if not InputMap.has_action(action): InputMap.add_action(action)
    bind_physical_key("BT_Camera_Look", KEY_KP_0)
    bind_physical_key("BT_Camera_Look", KEY_INSERT)
    var preferences := get_node_or_null("/root/UserSettings")
    if preferences != null:
        preferences.call("configure_gameplay_inputs", false)

func bind_physical_key(action: StringName, key: Key) -> void:
    if not InputMap.has_action(action): InputMap.add_action(action)
    var event := InputEventKey.new()
    event.physical_keycode = key
    if not InputMap.action_has_event(action, event): InputMap.action_add_event(action, event)

func set_crossbow_aim_mode(enabled: bool) -> void:
    crossbow_aim_mode = enabled
    var preferences := get_node_or_null("/root/UserSettings")
    if preferences != null:
        preferences.call("configure_gameplay_inputs", enabled)
    if not enabled: crossbow_aim_pitch = 0.0

func update_crossbow_aim_target() -> void:
    if not is_instance_valid(actor): return
    var viewport := get_viewport()
    var center := viewport.get_visible_rect().size * 0.5
    var origin := demo_camera.project_ray_origin(center)
    var direction := demo_camera.project_ray_normal(center)
    var query := PhysicsRayQueryParameters3D.create(origin, origin + direction * 2000.0, 1)
    query.exclude = [actor.get_rid()]
    var hit := actor.get_world_3d().direct_space_state.intersect_ray(query)
    var target := hit["position"] as Vector3 if not hit.is_empty() else origin + direction * 2000.0
    actor.call("source_set_laser_pointer_target", target)

func _unhandled_key_input(event: InputEvent) -> void:
    var key := event as InputEventKey
    if key == null or not key.pressed or key.echo:
        return
    if key.physical_keycode == KEY_F5:
        get_viewport().set_input_as_handled()
        save_game()
    elif key.physical_keycode == KEY_R:
        get_viewport().set_input_as_handled()
        leave_session("res://game/new_game.tscn" if new_game else "res://game/demo.tscn")
    elif key.physical_keycode == KEY_ESCAPE and not pause_available():
        # Keep the existing exit route while loading or after a failed entry.
        get_viewport().set_input_as_handled()
        leave_session("res://game/bootstrap.tscn")

func leave_session(destination: String) -> void:
    pause_host.call("close_pause")
    inventory_host.call("close_inventory")
    ready_to_play = false
    if is_instance_valid(brain):
        brain.call("stop")
    if is_instance_valid(actor):
        actor.set_physics_process(false)
    if crossbow_aim_mode: set_crossbow_aim_mode(false)
    for action in InputMap.get_actions():
        if str(action).begins_with("Button_"):
            Input.action_release(action)
    get_tree().change_scene_to_file(destination)

func pause_available() -> bool:
    return ready_to_play and is_instance_valid(actor) and is_instance_valid(level)

func inventory_available() -> bool:
    if not ready_to_play or not is_instance_valid(actor) or not is_instance_valid(level): return false
    var ui := level.get_node_or_null("AstrolabeUI")
    return ui == null or not ui.get("widescreen")

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
