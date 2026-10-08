extends Node3D
## Game session using the normally exported, already saved level.
## Hype's generated Brain owns gameplay choices. The measured English state
## seeds the manor; a native slot restores supported mutable player fields.

const LEVEL := "res://generated/hype/levels/manoir/level.tscn"
const GATE_ACTOR := "world/0/15/iMGrille_Manoir_Pont"
const MANOIR_START = preload("res://game/manoir_start.gd")
const SAVE_STATE = preload("res://game/save_state.gd")
const LOADING_SCREEN_PATH := NodePath("/root/LevelTransition/LoadingScreen")
const INITIAL_ROUTE_ERROR_META := "astrolabe_initial_route_error"
@export var new_game: bool = false

signal loading_stage_changed(stage: String, elapsed_ms: float)
signal initial_level_transition_finished(result: Dictionary)

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
var _transition_snapshot: Dictionary = {}
var _transition_brain_states: Array[Dictionary] = []
var _transition_physics_states: Array[Dictionary] = []
var _initial_transition_pending := false
var _initial_failure_recovery_started := false
var _initial_failure_recovery_cancelled := false
var source_camera_controller_state: Resource

func _ready() -> void:
    # Lock the incoming session before its deferred initial request starts.
    # The resource loader is active before open_level begins restoration.
    _initial_transition_pending = true
    var camera_state_script := "res://addons/astrolabe/camera/source_controller_state.gd"
    if ResourceLoader.exists(camera_state_script):
        source_camera_controller_state = load(camera_state_script).new()
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
    var transitions := get_node_or_null("/root/LevelTransition")
    if transitions != null:
        if transitions.has_signal("transition_failed"):
            transitions.connect("transition_failed", _on_initial_transition_failed)
        if transitions.has_signal("transition_cancelled"):
            transitions.connect("transition_cancelled", _on_initial_transition_cancelled)
    call_deferred("request_initial_level")

func request_initial_level() -> void:
    _initial_transition_pending = true
    var transitions := get_node_or_null("/root/LevelTransition")
    if transitions != null:
        if bool(transitions.call("request_initial_level", "manoir")):
            return
        fail("The manor loading service could not start the initial load.")
        return
    open_level()

func _on_initial_transition_failed(level_name: String, message: String) -> void:
    if _initial_transition_pending and level_name == "manoir":
        fail(message)

func _on_initial_transition_cancelled(level_name: String) -> void:
    if _initial_transition_pending and level_name == "manoir":
        _initial_transition_pending = false

func open_level_from_transition(packed: PackedScene, path: String) -> Dictionary:
    if path != LEVEL or packed == null or level != null or loading:
        return {"ok": false, "message": "The initial manor request does not match this session."}
    if content_cache != null:
        content_cache.call("retain_level", path, packed)
    _initial_transition_pending = true
    call_deferred("open_level")
    var result: Dictionary = await initial_level_transition_finished
    return result

func bind_source_camera_controller_state() -> void:
    _bind_source_camera_controller_state(level)

func _bind_source_camera_controller_state(target_level: Node3D) -> void:
    if target_level == null:
        return
    if source_camera_controller_state == null:
        var camera_state_script := "res://addons/astrolabe/camera/source_controller_state.gd"
        if ResourceLoader.exists(camera_state_script):
            source_camera_controller_state = load(camera_state_script).new()
    if source_camera_controller_state == null:
        return
    target_level.set_meta("source_camera_controller_state", source_camera_controller_state)
    for source_camera in target_level.find_children("SourceCamera", "Camera3D", true, false):
        if _has_property(source_camera, "controller_state"):
            source_camera.set("controller_state", source_camera_controller_state)

func open_level() -> void:
    if loading or level != null:
        return
    loading = true
    var restore_message := "Initializing new game..." if new_game else "Restoring saved game..."
    _show_initial_restore_phase(restore_message)
    stage("state_restoration", restore_message)
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
    bind_source_camera_controller_state()
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
    _finish_initial_transition(true, "")

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
    _finish_initial_transition(true, "")

func _finish_initial_transition(ok: bool, message: String) -> void:
    if not _initial_transition_pending:
        return
    _initial_transition_pending = false
    initial_level_transition_finished.emit({"ok": ok, "message": message})

func begin_level_transition() -> bool:
    if not ready_to_play or loading or not is_instance_valid(level) or not is_instance_valid(actor) or not is_instance_valid(brain):
        return false
    if pause_host.has_method("close_pause"):
        pause_host.call("close_pause")
    if inventory_host.has_method("close_inventory"):
        inventory_host.call("close_inventory")
    _transition_snapshot = _capture_transition_state()
    _transition_brain_states.clear()
    _transition_physics_states.clear()
    for source_brain in level.find_children("Brain", "Node", true, false):
        var was_running := bool(source_brain.get("running"))
        var automatic := bool(source_brain.get("automatic"))
        _transition_brain_states.append({"brain": source_brain, "running": was_running, "automatic": automatic})
        source_brain.set("automatic", false)
        if was_running:
            source_brain.call("stop")
    for body in level.find_children("*", "CharacterBody3D", true, false):
        _transition_physics_states.append({"body": body, "enabled": body.is_physics_processing()})
        body.set_physics_process(false)
    ready_to_play = false
    loading = true
    stage("level_transition", "Loading destination level...")
    return true

func install_level_transition(destination: Node3D, entry: String, camera_entry: String, packed: PackedScene) -> Dictionary:
    if not loading or not is_instance_valid(level) or not is_instance_valid(destination):
        return {"ok": false, "message": "The active game session is no longer available."}
    var new_actor := _destination_actor(destination)
    if new_actor == null:
        return {"ok": false, "message": "The destination level has no authored main actor."}
    var entry_actor: Node3D = new_actor
    if not entry.is_empty():
        entry_actor = _destination_entry(destination, entry)
        if entry_actor == null:
            return {"ok": false, "message": "The destination entry actor %s is missing or ambiguous." % entry}
    var camera_actor: Node3D
    if not camera_entry.is_empty():
        camera_actor = _destination_entry(destination, camera_entry)
        if camera_actor == null:
            return {"ok": false, "message": "The destination camera entry actor %s is missing or ambiguous." % camera_entry}
    var new_brain := new_actor.get_node_or_null("Brain")
    if new_brain == null:
        return {"ok": false, "message": "The destination main actor has no Brain."}

    var previous_level := level
    destination.name = "Level_" + str(destination.get_meta("source_level", "destination"))
    if new_actor is CharacterBody3D:
        new_actor.velocity = Vector3.ZERO
        new_actor.set_physics_process(false)
    new_brain.set("automatic", false)
    _bind_source_camera_controller_state(destination)
    add_child(destination)
    # Global transforms are only valid after the destination has a scene-tree parent.
    # Keep Hype's physics and Brain paused until his authored entry pose is applied.
    new_actor.global_transform = entry_actor.global_transform
    _apply_transition_state(new_actor, destination)
    if content_cache != null:
        content_cache.call("retain_level", "res://generated/hype/levels/%s/level.tscn" % str(destination.get_meta("source_level", "")).to_lower(), packed)

    demo_camera.set("target", new_actor)
    if is_instance_valid(camera_actor):
        demo_camera.call("hold_transition_view", camera_actor.global_transform)
    else:
        demo_camera.set("target", new_actor)
        demo_camera.call("release_transition_view")
    demo_camera.make_current()
    if new_actor is CharacterBody3D:
        new_actor.set_physics_process(true)
    await get_tree().physics_frame
    if not is_inside_tree() or not is_instance_valid(destination):
        return {"ok": false, "message": "The game session ended while preparing the destination."}
    await get_tree().physics_frame
    if not is_inside_tree() or not is_instance_valid(destination):
        return {"ok": false, "message": "The game session ended while preparing the destination."}
    new_brain.set("automatic", true)
    if not bool(new_brain.call("start")):
        destination.queue_free()
        _resume_previous_level()
        return {"ok": false, "message": "The destination main actor's behavior could not start."}

    level = destination
    actor = new_actor as CharacterBody3D
    brain = new_brain
    world_brain = null
    ready_to_play = true
    loading = false
    _transition_snapshot.clear()
    _transition_brain_states.clear()
    _transition_physics_states.clear()
    if is_instance_valid(previous_level):
        remove_child(previous_level)
        previous_level.queue_free()
    stage("playable", "Explore %s" % str(level.get_meta("source_level", "the new level")))
    return {"ok": true}

func abort_level_transition() -> void:
    if loading:
        _resume_previous_level()

func _resume_previous_level() -> void:
    for state in _transition_physics_states:
        var body := state.get("body") as Node
        if is_instance_valid(body):
            body.set_physics_process(bool(state.get("enabled", false)))
    for state in _transition_brain_states:
        var source_brain := state.get("brain") as Node
        if not is_instance_valid(source_brain):
            continue
        source_brain.set("automatic", bool(state.get("automatic", false)))
        if bool(state.get("running", false)):
            source_brain.call("start")
    if is_instance_valid(actor):
        demo_camera.set("target", actor)
        demo_camera.call("release_transition_view")
    ready_to_play = true
    loading = false
    _transition_snapshot.clear()
    _transition_brain_states.clear()
    _transition_physics_states.clear()
    stage("playable", "Explore the manor" if str(level.get_meta("source_level", "manoir")) == "manoir" else "Explore %s" % str(level.get_meta("source_level", "the level")))

func _destination_actor(destination: Node3D) -> CharacterBody3D:
    var identity := str(destination.get_meta("source_main_actor", ""))
    var actors: Variant = destination.get_meta("source_actor_nodes", {})
    if identity.is_empty() or not actors is Dictionary or not actors.has(identity):
        return null
    return destination.get_node_or_null(actors[identity]) as CharacterBody3D

func _destination_entry(destination: Node3D, name: String) -> Node3D:
    var entries: Variant = destination.get_meta("source_actor_entries", {})
    if not entries is Dictionary:
        return null
    var candidates: Variant = entries.get(name.to_lower(), [])
    if not candidates is Array or candidates.size() != 1:
        return null
    return destination.get_node_or_null(candidates[0]) as Node3D

func _capture_transition_state() -> Dictionary:
    var state: Dictionary = {"player": {}, "main_brain": {}, "fix_brains": {}}
    if not is_instance_valid(actor):
        return state
    var inventory: Resource = actor.get("source_inventory")
    var magick: Resource = actor.get("source_magick")
    var hit_points: Resource = actor.get("source_hit_points")
    if inventory != null:
        state["player"]["inventory"] = {"selected_weapon": inventory.get("selected_weapon"), "quantities": inventory.get("quantities").duplicate(true)}
    if magick != null:
        state["player"]["magick"] = {"current_points": magick.get("current_points"), "maximum_points": magick.get("maximum_points"), "selected_spell_code": magick.get("selected_spell_code")}
    if hit_points != null:
        state["player"]["hit_points"] = {"current": hit_points.get("current"), "maximum": hit_points.get("maximum"), "limit": hit_points.get("limit")}
    state["main_brain"] = _capture_brain_state(brain)
    var source_actor_nodes: Variant = level.get_meta("source_actor_nodes", {})
    if source_actor_nodes is Dictionary:
        for identity in source_actor_nodes:
            if not str(identity).begins_with("fix:/persos/"):
                continue
            var source_actor := level.get_node_or_null(source_actor_nodes[identity])
            var source_brain := source_actor.get_node_or_null("Brain") if source_actor != null else null
            if source_brain != null:
                state["fix_brains"][str(identity)] = _capture_brain_state(source_brain)
    return state

func _capture_brain_state(source_brain: Node) -> Dictionary:
    var values: Dictionary = {}
    if source_brain == null:
        return {}
    for property in source_brain.get_property_list():
        var property_name := str(property.get("name", ""))
        if not property_name.begins_with("dsg_"):
            continue
        var value: Variant = source_brain.get(property_name)
        if _is_transferable_value(value):
            values[property_name] = value.duplicate(true) if value is Array or value is Dictionary else value
    return {"model": str(source_brain.get("source_model_path")), "values": values}

func _apply_transition_state(new_actor: CharacterBody3D, destination: Node3D) -> void:
    if not _transition_snapshot.has("player"):
        return
    var player: Dictionary = _transition_snapshot["player"]
    var inventory: Resource = new_actor.get("source_inventory")
    if inventory != null and player.has("inventory"):
        var inventory_state: Dictionary = player["inventory"]
        inventory.set("selected_weapon", inventory_state["selected_weapon"])
        inventory.set("quantities", inventory_state["quantities"].duplicate(true))
    var magick: Resource = new_actor.get("source_magick")
    if magick != null and player.has("magick"):
        for key in player["magick"]:
            magick.set(key, player["magick"][key])
    var hit_points: Resource = new_actor.get("source_hit_points")
    if hit_points != null and player.has("hit_points"):
        for key in player["hit_points"]:
            hit_points.set(key, player["hit_points"][key])
    var new_brain := new_actor.get_node_or_null("Brain")
    _apply_brain_state(new_brain, _transition_snapshot.get("main_brain", {}))
    var fix_states: Dictionary = _transition_snapshot.get("fix_brains", {})
    var source_actor_nodes: Variant = destination.get_meta("source_actor_nodes", {})
    if source_actor_nodes is Dictionary:
        for identity in fix_states:
            if not source_actor_nodes.has(identity):
                continue
            var target_actor := destination.get_node_or_null(source_actor_nodes[identity])
            var target_brain := target_actor.get_node_or_null("Brain") if target_actor != null else null
            _apply_brain_state(target_brain, fix_states[identity])

func _apply_brain_state(target_brain: Node, saved: Dictionary) -> void:
    if target_brain == null or saved.is_empty() or str(target_brain.get("source_model_path")) != str(saved.get("model", "")):
        return
    for property_name in saved.get("values", {}):
        if _has_property(target_brain, str(property_name)):
            var value: Variant = saved["values"][property_name]
            target_brain.set(property_name, value.duplicate(true) if value is Array or value is Dictionary else value)

func _has_property(object: Object, property_name: String) -> bool:
    return object.get_property_list().any(func(property): return str(property.get("name")) == property_name)

func _is_transferable_value(value: Variant) -> bool:
    var value_type := typeof(value)
    if value_type in [TYPE_NIL, TYPE_BOOL, TYPE_INT, TYPE_FLOAT, TYPE_STRING, TYPE_STRING_NAME,
        TYPE_VECTOR2, TYPE_VECTOR2I, TYPE_RECT2, TYPE_RECT2I, TYPE_VECTOR3, TYPE_VECTOR3I,
        TYPE_TRANSFORM2D, TYPE_VECTOR4, TYPE_VECTOR4I, TYPE_PLANE, TYPE_QUATERNION,
        TYPE_AABB, TYPE_BASIS, TYPE_TRANSFORM3D, TYPE_PROJECTION, TYPE_COLOR,
        TYPE_PACKED_BYTE_ARRAY, TYPE_PACKED_INT32_ARRAY, TYPE_PACKED_INT64_ARRAY,
        TYPE_PACKED_FLOAT32_ARRAY, TYPE_PACKED_FLOAT64_ARRAY, TYPE_PACKED_STRING_ARRAY,
        TYPE_PACKED_VECTOR2_ARRAY, TYPE_PACKED_VECTOR3_ARRAY, TYPE_PACKED_COLOR_ARRAY]:
        return true
    if value is Array:
        for item in value:
            if not _is_transferable_value(item): return false
        return true
    if value is Dictionary:
        for key in value:
            if not _is_transferable_value(key) or not _is_transferable_value(value[key]): return false
        return true
    return false

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
    if not ready_to_play:
        get_viewport().set_input_as_handled()
        if key.physical_keycode == KEY_ESCAPE:
            leave_session("res://game/bootstrap.tscn")
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

func _input(event: InputEvent) -> void:
    if ready_to_play or not (_initial_transition_pending or loading):
        return
    var key := event as InputEventKey
    if key != null and key.pressed and not key.echo and key.physical_keycode == KEY_ESCAPE:
        get_viewport().set_input_as_handled()
        leave_session("res://game/bootstrap.tscn")
        return
    if event is InputEventKey or event is InputEventMouseButton or event is InputEventMouseMotion or event is InputEventJoypadButton or event is InputEventJoypadMotion:
        get_viewport().set_input_as_handled()

func leave_session(destination: String) -> void:
    var transitions := get_node_or_null("/root/LevelTransition")
    if transitions != null and bool(transitions.call("is_transitioning")):
        if not bool(transitions.call("cancel_current_transition")):
            return
    _initial_failure_recovery_cancelled = true
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
    _initial_failure_recovery_cancelled = true
    restore_frame_limit()
    if is_instance_valid(brain):
        brain.call("stop")

func fail(message: String) -> void:
    var initial_failure := _initial_transition_pending
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
    _finish_initial_transition(false, message)
    if initial_failure:
        _initial_failure_recovery_cancelled = false
        get_tree().root.set_meta(INITIAL_ROUTE_ERROR_META, message)
        if not _initial_failure_recovery_started:
            _initial_failure_recovery_started = true
            call_deferred("_return_to_menu_after_initial_failure")

func _show_initial_restore_phase(message: String) -> void:
    var presenter := get_node_or_null(LOADING_SCREEN_PATH)
    if presenter == null or not bool(presenter.get("visible")):
        return
    var phase := presenter.get_node_or_null("Viewport/Center/Content/Phase") as Label
    var progress := presenter.get_node_or_null("Viewport/Center/Content/Progress") as ProgressBar
    if phase != null:
        phase.text = message
    # ResourceLoader's fraction describes scene dependencies only. Once state
    # restoration begins, leave an indeterminate phase label instead of
    # presenting that fraction as total route progress.
    if progress != null:
        progress.hide()

func _return_to_menu_after_initial_failure() -> void:
    # Capture the persistent service/presenter before yielding. This coroutine
    # belongs to the session, which can leave the tree while it waits for the
    # user to dismiss an initial-load error.
    if not is_inside_tree():
        return
    var scene_tree := get_tree()
    if scene_tree.current_scene != self:
        return
    var transitions := scene_tree.root.get_node_or_null("LevelTransition")
    var presenter := transitions.get_node_or_null("LoadingScreen") if transitions != null else null

    # Let LevelTransition publish the terminal failure before checking the
    # persistent presenter. The captured reference remains safe if this
    # session leaves the active scene tree during the wait.
    await scene_tree.process_frame
    if not _initial_failure_recovery_is_current(scene_tree):
        return
    while is_instance_valid(presenter) and bool(presenter.get("visible")):
        await scene_tree.process_frame
        if not _initial_failure_recovery_is_current(scene_tree):
            return

    var error := scene_tree.change_scene_to_file("res://game/bootstrap.tscn")
    if error != OK and _initial_failure_recovery_is_current(scene_tree):
        _initial_failure_recovery_started = false
        var message := "The game could not be opened. Press Esc to return to the menu."
        stage("failed", message)
        status_label.text = message
        status_label.show()
        push_error("Could not return to the menu after initial load failed: %s" % error_string(error))

func _initial_failure_recovery_is_current(scene_tree: SceneTree) -> bool:
    return is_inside_tree() and not _initial_failure_recovery_cancelled and scene_tree.current_scene == self
