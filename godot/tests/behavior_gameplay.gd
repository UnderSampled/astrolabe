extends SceneTree
## Test-owned control surface around the normally exported manor scene.
## The saved Hype actor and Brain make every gameplay decision.
const MANOIR_START = preload("res://game/manoir_start.gd")

## Observe after native character physics, including ticks between rendered
## frames. Short-lived projectiles must not disappear from the evidence trace
## merely because rendering stalled.
class PhysicsTrace extends Node:
    var collect: Callable
    func _physics_process(_delta: float) -> void:
        collect.call()

var control_dir: String
var request_path: String
var response_path: String
var actor: CharacterBody3D
var brain: Node
var global_brain: Node
var camera: Camera3D
var fixture: Dictionary
var packed: PackedScene
var scene: Node3D
var setup_snapshots: Array = []
var action_requests: Array = []
var action_states: Array = []
var key_inputs: Array = []
var generated_trace: Array = []
var generated_trace_first: Array = []
var generated_trace_first_overflow: bool = false
var starting_state_overlay: Dictionary = {}
var last_action_state: int = -1

func _initialize() -> void:
    configure_crossbow_inputs()
    call_deferred("run")

func configure_crossbow_inputs() -> void:
    for action in ["BT_Camera_Look", "BT_Camera_LookLeft", "BT_Camera_LookRight", "BT_Camera_LookUp", "BT_Camera_LookDown", "BT_Camera_TireArbalete"]:
        if not InputMap.has_action(action): InputMap.add_action(action)
    for action in ["Button_Left", "Button_Right", "Button_Up", "Button_Down", "Button_Hit"]:
        if not InputMap.has_action(action): InputMap.add_action(action)
    bind_physical_key("BT_Camera_Look", KEY_KP_0)
    bind_physical_key("Button_Left", KEY_LEFT)
    bind_physical_key("Button_Right", KEY_RIGHT)
    bind_physical_key("Button_Up", KEY_UP)
    bind_physical_key("Button_Down", KEY_DOWN)
    bind_physical_key("Button_Hit", KEY_SPACE)

func bind_physical_key(action: StringName, key: Key) -> void:
    var event := InputEventKey.new()
    event.physical_keycode = key
    if not InputMap.action_has_event(action, event): InputMap.action_add_event(action, event)

func route_crossbow_key(action: StringName, key: Key, enabled: bool) -> void:
    var event := InputEventKey.new()
    event.physical_keycode = key
    if enabled:
        if not InputMap.action_has_event(action, event): InputMap.action_add_event(action, event)
    elif InputMap.action_has_event(action, event):
        InputMap.action_erase_event(action, event)

func set_crossbow_input_mode(enabled: bool) -> void:
    var movement := ["Button_Left", "Button_Right", "Button_Up", "Button_Down"]
    var aiming := ["BT_Camera_LookLeft", "BT_Camera_LookRight", "BT_Camera_LookUp", "BT_Camera_LookDown"]
    var keys := [KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN]
    for i in range(movement.size()):
        route_crossbow_key(movement[i], keys[i], not enabled)
        route_crossbow_key(aiming[i], keys[i], enabled)
    route_crossbow_key("Button_Hit", KEY_SPACE, not enabled)
    route_crossbow_key("BT_Camera_TireArbalete", KEY_SPACE, enabled)

func write_json(path: String, value: Dictionary) -> void:
    var file := FileAccess.open(path, FileAccess.WRITE)
    if file == null:
        push_error("Cannot write behavior response: " + path)
        quit(1)
        return
    file.store_string(JSON.stringify(value) + "\n")
    file.close()

func run() -> void:
    var args := OS.get_cmdline_user_args()
    if args.size() != 2:
        push_error("Behavior gameplay requires control directory and scene path")
        quit(1)
        return
    control_dir = args[0]
    request_path = control_dir.path_join("request.json")
    response_path = control_dir.path_join("response.json")
    write_json(control_dir.path_join("phase.json"), {"phase": "loading_scene"})
    packed = load("res://" + args[1]) as PackedScene
    if packed == null:
        write_json(control_dir.path_join("ready.json"), {"error": "saved manor scene did not load"})
        quit(1)
        return
    scene = packed.instantiate() as Node3D
    write_json(control_dir.path_join("phase.json"), {"phase": "instantiated_scene"})
    actor = scene.get_node_or_null("world/0/0/Hype") as CharacterBody3D
    brain = actor.get_node_or_null("Brain") if actor != null else null
    if actor == null or brain == null:
        write_json(control_dir.path_join("ready.json"), {"error": "saved Hype actor or Brain is missing"})
        quit(1)
        return
    actor.connect("action_request_traced", Callable(self, "record_action_request"))
    actor.connect("generated_object_released", Callable(self, "record_generated_release"))
    fixture = MANOIR_START.load_fixture()
    if int(fixture.get("schema", 0)) != 1 or str(fixture.get("name", "")) != "supplied-english-save-manoir":
        write_json(control_dir.path_join("ready.json"), {"error": "invalid English save fixture"})
        quit(1)
        return
    # The fixture restores the measured state of the supplied English save.
    # It uses the normally exported Hype actor, Brain, and compiled scripts.
    brain.set("automatic", false)
    actor.set_physics_process(false)
    root.add_child(scene)
    var physics_trace := PhysicsTrace.new()
    physics_trace.process_physics_priority = 100
    physics_trace.collect = Callable(self, "track_generated_objects")
    root.add_child(physics_trace)
    # Scene attachment initializes scripts and may reenable callbacks.
    actor.set_physics_process(false)
    write_json(control_dir.path_join("phase.json"), {"phase": "attached_scene"})
    global_brain = scene.get_node_or_null("Globals/Global/Brain")
    var global_bound := false
    if global_brain != null:
        for property in brain.get_property_list():
            var name := str(property["name"])
            if name.begins_with("actor_brain_") and name.ends_with("_path"):
                var route: NodePath = brain.get(name)
                if not route.is_empty() and brain.get_node_or_null(route) == global_brain:
                    global_bound = true
    if not global_bound:
        write_json(control_dir.path_join("ready.json"), {"error": "saved Hype Brain lacks a Global actor link"})
        quit(1)
        return
    if int(actor.get("current_state")) != int(fixture["expected_action"]):
        write_json(control_dir.path_join("ready.json"), {"error": "exported Hype action differs from save fixture"})
        quit(1)
        return
    var restore_error := restore_save_fixture()
    if not restore_error.is_empty():
        write_json(control_dir.path_join("ready.json"), {"error": restore_error})
        quit(1)
        return
    setup_snapshots.append({"phase": "fixture_restored", "state": observation(), "designer_arrays": designer_action_arrays()})
    camera = Camera3D.new()
    camera.name = "BehaviorCamera"
    camera.far = 10000.0
    camera.current = true
    scene.add_child(camera)
    follow_actor()
    write_json(control_dir.path_join("phase.json"), {"phase": "settling", "observation": observation()})
    # Godot's first visible frames can stutter while the level's resources
    # finish preparing. Start the saved Brain only after that wait so its
    # source save-state snapshot is the scenario's actual starting state.
    await create_timer(5.0).timeout
    setup_snapshots.append({"phase": "resource_settled", "state": observation()})
    actor.set_physics_process(true)
    await create_timer(0.1).timeout
    setup_snapshots.append({"phase": "native_physics_warmup", "state": observation()})
    brain.set("automatic", true)
    if not bool(brain.call("start")):
        write_json(control_dir.path_join("ready.json"), {"error": "saved Hype Brain did not start"})
        quit(1)
        return
    await create_timer(0.2).timeout
    var ready := observation()
    ready["setup_snapshots"] = setup_snapshots
    write_json(control_dir.path_join("ready.json"), ready)
    while true:
        await process_frame
        track_action_state()
        follow_actor()
        if not FileAccess.file_exists(request_path):
            continue
        var request = JSON.parse_string(FileAccess.get_file_as_string(request_path))
        DirAccess.remove_absolute(request_path)
        if not request is Dictionary:
            write_json(response_path, {"error": "invalid request JSON"})
            continue
        var response: Dictionary = await handle(request)
        write_json(response_path, response)
        if request.get("action", "") == "stop":
            quit()
            return

func follow_actor() -> void:
    if actor == null or camera == null:
        return
    var forward := actor.global_basis.z.normalized()
    camera.global_position = actor.global_position - forward * 7.0 + Vector3.UP * 3.5
    camera.look_at(actor.global_position + Vector3.UP * 1.4)

func record_action_request(requested: int, restart: bool, previous: int, selected: int, accepted: bool) -> void:
    var fields: Dictionary = {}
    for index in [7, 19, 52, 59, 61, 124, 125]:
        fields[str(index)] = brain.get("dsg_" + str(index))
    action_requests.append({
        "at_ms": Time.get_ticks_msec(),
        "requested": requested,
        "restart": restart,
        "previous": previous,
        "selected": selected,
        "accepted": accepted,
        "current_state": int(actor.get("current_state")),
        "pending_state": int(actor.get("pending_state")),
        "requested_action_state": int(actor.get("requested_action_state")),
        "normal_behavior": int(brain.get("current_normal")),
        "normal_action_revision": int(brain.get("normal_action_revision")),
        "fields": fields,
    })
    if action_requests.size() > 100: action_requests.pop_front()

func track_action_state() -> void:
    if actor == null: return
    var current := int(actor.get("current_state"))
    if current == last_action_state: return
    last_action_state = current
    action_states.append({"at_ms": Time.get_ticks_msec(), "action": current})
    if action_states.size() > 100: action_states.pop_front()

func track_generated_objects() -> void:
    for candidate in root.find_children("*", "CharacterBody3D", true, false):
        if not candidate.has_meta("astrolabe_generated_template"): continue
        var spawned := candidate as CharacterBody3D
        if not spawned.has_meta("astrolabe_behavior_exit_traced"):
            spawned.set_meta("astrolabe_behavior_exit_traced", true)
            spawned.tree_exiting.connect(Callable(self, "record_generated_exit").bind(spawned))
        var entry := generated_object_snapshot(spawned)
        entry["event"] = "frame"
        entry["at_ms"] = Time.get_ticks_msec()
        generated_trace.append(entry)
        if generated_trace_first.size() < 60:
            generated_trace_first.append(entry)
        else:
            generated_trace_first_overflow = true
    if generated_trace.size() > 120: generated_trace.pop_front()

func record_generated_exit(spawned: CharacterBody3D) -> void:
    var entry := generated_object_snapshot(spawned)
    entry["event"] = "tree_exiting"
    entry["at_ms"] = Time.get_ticks_msec()
    generated_trace.append(entry)
    if generated_trace_first.size() < 60:
        generated_trace_first.append(entry)
    else:
        generated_trace_first_overflow = true
    if generated_trace.size() > 120: generated_trace.pop_front()

func record_generated_release(spawned: Node3D, attachment: Node3D) -> void:
    if not spawned.has_meta("astrolabe_behavior_exit_traced"):
        spawned.set_meta("astrolabe_behavior_exit_traced", true)
        spawned.tree_exiting.connect(Callable(self, "record_generated_exit").bind(spawned))
    var entry := generated_object_snapshot(spawned as CharacterBody3D)
    entry["event"] = "release"
    entry["at_ms"] = Time.get_ticks_msec()
    entry["attachment_translation"] = source_components(attachment.global_position)
    entry["parent_translation"] = source_components(actor.global_position)
    generated_trace.append(entry)

func source_vector(values: Array) -> Vector3:
    return Vector3(float(values[0]), float(values[2]), -float(values[1]))

func source_components(value: Vector3) -> Array:
    return [value.x, -value.z, value.y]

func designer_action_arrays() -> Dictionary:
    var result: Dictionary = {}
    for index in [159, 160, 161, 162, 163, 164]:
        var values: Array = []
        for value in brain.get("dsg_" + str(index)):
            values.append(int(value))
        result[str(index)] = values
    return result

func restore_save_fixture() -> String:
    return MANOIR_START.apply(actor, brain, fixture)

func apply_starting_state_overlay(overlay: Dictionary) -> String:
    starting_state_overlay = {}
    if overlay.is_empty():
        return ""
    if overlay.size() != 2 or str(overlay.get("kind", "")) != "crossbow_launch":
        return "unknown behavior starting-state overlay"
    var source_target: Variant = overlay.get("laser_target_source")
    if not source_target is Array or source_target.size() != 3:
        return "crossbow laser target is malformed"
    for component in source_target:
        if not (component is float or component is int) or not is_finite(float(component)):
            return "crossbow laser target is malformed"
    var native_inventory: Resource = actor.get("source_inventory")
    var native_laser: Resource = actor.get("source_laser_pointer")
    if native_inventory == null or native_laser == null:
        return "crossbow starting-state resources are missing"
    if int(native_inventory.get("selected_weapon")) != 80 or int(native_inventory.call("quantity", 80)) != 8:
        return "crossbow overlay requires the measured English save inventory"
    var quantities: Dictionary = native_inventory.get("quantities")
    quantities = quantities.duplicate(true)
    quantities[80] = 3
    native_inventory.set("quantities", quantities)
    native_laser.set("target_position", source_vector(source_target))
    native_laser.set("target_valid", true)
    native_laser.set("active", false)
    starting_state_overlay = {
        "kind": "crossbow_launch",
        "weapon_80_quantity": 3,
        "laser_target_source": source_target.duplicate(),
        "laser_target_provenance": "declared_test_starting_state",
    }
    return ""

func generated_objects() -> Array:
    var result: Array = []
    for candidate in root.find_children("*", "CharacterBody3D", true, false):
        if not candidate.has_meta("astrolabe_generated_template"): continue
        result.append(generated_object_snapshot(candidate as CharacterBody3D))
    return result

func crossbow_ai_state() -> Dictionary:
    var fields: Dictionary = {}
    for index in [7, 8, 13, 15, 59, 60, 95, 120, 130]:
        fields[str(index)] = brain.get("dsg_" + str(index))
    return {
        "action": int(actor.get("current_state")),
        "normal_behavior": int(brain.get("current_normal")),
        "fields": fields,
    }

func generated_object_snapshot(spawned: CharacterBody3D) -> Dictionary:
    var position := spawned.global_position
    var velocity: Vector3 = spawned.get("external_velocity")
    var spawned_brain := spawned.get_node_or_null("Brain")
    var start_value: Variant = spawned_brain.get("dsg_0") if spawned_brain != null else null
    var spawned_laser: Resource = spawned.get("source_laser_pointer")
    var spawned_inventory: Resource = spawned.get("source_inventory")
    var target: Vector3 = spawned_laser.get("target_position") if spawned_laser != null else Vector3.ZERO
    var basis := spawned.global_basis
    var slide_contacts: Array = []
    for index in spawned.get_slide_collision_count():
        var contact := spawned.get_slide_collision(index)
        var collider := contact.get_collider() as Node
        var owner := collider.get_parent() if collider != null else null
        var point := contact.get_position()
        var normal := contact.get_normal()
        slide_contacts.append({
            "collider": str(collider.get_path()) if collider != null else "",
            "source_node_id": str(owner.get_meta("source_node_id", "")) if owner != null else "",
            "source_perso": str(collider.get_meta("source_collision_perso", "")) if collider != null else "",
            "position": [point.x, point.y, point.z],
            "normal": [normal.x, normal.y, normal.z],
        })
    var zde_first_zone_hit: Variant = null
    if spawned_brain != null and str(spawned_brain.get("source_model_path")) == "fix:/ai/models/733":
        zde_first_zone_hit = bool(spawned.call("source_collide_perso_zde_no_with_type_zde", 1, 1))
    return {
        "instance_id": spawned.get_instance_id(),
        "template": str(spawned.get_meta("astrolabe_generated_template")),
        "source_model_path": str(spawned_brain.get("source_model_path")) if spawned_brain != null else "",
        "slide_collision_count": spawned.get_slide_collision_count(),
        "slide_contacts": slide_contacts,
        "zde_first_zone_hit": zde_first_zone_hit,
        "on_floor": spawned.is_on_floor(),
        "on_wall": spawned.is_on_wall(),
        "physics_frame": Engine.get_physics_frames(),
        "translation": [position.x, -position.z, position.y],
        "velocity": [velocity.x, -velocity.z, velocity.y],
        "source_basis": [source_components(basis.x), source_components(-basis.z), source_components(basis.y)],
        "sighting": source_components(basis.z),
        "laser_target_valid": bool(spawned_laser.get("target_valid")) if spawned_laser != null else false,
        "laser_target": source_components(target),
        "weapon_80_quantity": int(spawned_inventory.call("quantity", 80)) if spawned_inventory != null else null,
        "global_field_14": bool(global_brain.get("dsg_14")) if global_brain != null else null,
        "action": int(spawned.get("current_state")),
        "brain_running": bool(spawned_brain.get("running")) if spawned_brain != null else false,
        "brain_normal": int(spawned_brain.get("current_normal")) if spawned_brain != null else -1,
        "brain_clock_ms": float(spawned_brain.get("clock_milliseconds")) if spawned_brain != null else 0.0,
        "brain_start_ms": int(start_value) if start_value is int or start_value is float else null,
        "brain_failure": str(spawned_brain.get("unsupported_failure")) if spawned_brain != null else "",
        "main_actor_field_120": int(brain.get("dsg_120")) if brain != null else null,
        "visible_from_hype": bool(spawned.call("source_see_perso", actor, spawned)),
        "owner_crossbow_ai": crossbow_ai_state(),
    }

func observation() -> Dictionary:
    if actor == null or brain == null:
        return {"error": "actor or Brain unavailable"}
    var position := actor.global_position
    var facing := actor.global_basis.x.normalized()
    var root_motion: Vector3 = actor.get("root_motion_displacement")
    var external_motion: Vector3 = actor.get("external_velocity")
    var slide_normals: Array = []
    var slide_contacts: Array = []
    var channel_visibility: Dictionary = {}
    var channels := actor.get_node_or_null("Channels")
    if channels != null:
        for channel in channels.get_children():
            var source_id := str(channel.get_meta("source_channel", -1))
            if not channel_visibility.has(source_id): channel_visibility[source_id] = []
            channel_visibility[source_id].append((channel as Node3D).visible)
    for index in actor.get_slide_collision_count():
        var contact := actor.get_slide_collision(index)
        var normal := contact.get_normal()
        slide_normals.append([normal.x, normal.y, normal.z])
        var collider := contact.get_collider() as Node
        var owner := collider.get_parent() if collider != null else null
        var point := contact.get_position()
        slide_contacts.append({
            "collider": str(collider.get_path()) if collider != null else "",
            "source_node_id": str(owner.get_meta("source_node_id", "")) if owner != null else "",
            "position": [point.x, point.y, point.z],
            "normal": [normal.x, normal.y, normal.z],
        })
    var floor_normal := actor.get_floor_normal() if actor.is_on_floor() else Vector3.ZERO
    var native_magick: Resource = actor.get("source_magick")
    var native_inventory: Resource = actor.get("source_inventory")
    var native_laser: Resource = actor.get("source_laser_pointer")
    var laser_target: Vector3 = native_laser.get("target_position") if native_laser != null else Vector3.ZERO
    var magick_ready := bool(actor.call("source_magic_ready"))
    return {
        "action": int(actor.get("current_state")),
        "translation": [position.x, -position.z, position.y],
        "heading_rad": atan2(-facing.z, facing.x),
        "godot_position": [position.x, position.y, position.z],
        "godot_velocity": [actor.velocity.x, actor.velocity.y, actor.velocity.z],
        "root_motion_displacement": [root_motion.x, root_motion.y, root_motion.z],
        "external_velocity": [external_motion.x, external_motion.y, external_motion.z],
        "generated_object_count": int(actor.get("source_generated_object_count")),
        "generated_objects": generated_objects(),
        "generated_trace": generated_trace.duplicate(true),
        "generated_trace_first": generated_trace_first.duplicate(true),
        "generated_trace_first_overflow": generated_trace_first_overflow,
        "channel_visibility": channel_visibility,
        "hud_display": actor.get("source_hud_display"),
        "magick": {
            "selected_spell_code": int(actor.call("source_magic_current_spell")) if magick_ready else null,
            "current_points": int(actor.call("source_magic_points")) if magick_ready else null,
            "maximum_points": int(native_magick.get("maximum_points")) if native_magick != null else null,
        },
        "inventory": {
            "selected_weapon": int(native_inventory.get("selected_weapon")) if native_inventory != null else null,
            "weapon_80_quantity": int(native_inventory.call("quantity", 80)) if native_inventory != null else null,
        },
        "laser": {
            "active": bool(native_laser.get("active")) if native_laser != null else null,
            "target_valid": bool(native_laser.get("target_valid")) if native_laser != null else null,
            "target": source_components(laser_target),
        },
        "starting_state_overlay": starting_state_overlay.duplicate(true),
        "designer_fields": {"59": bool(brain.get("dsg_59"))},
        "action_requests": action_requests.duplicate(true),
        "action_states": action_states.duplicate(true),
        "key_inputs": key_inputs.duplicate(true),
        "slide_collisions": actor.get_slide_collision_count(),
        "slide_normals": slide_normals,
        "slide_contacts": slide_contacts,
        "floor_normal": [floor_normal.x, floor_normal.y, floor_normal.z],
        "on_floor": actor.is_on_floor(),
        "ground_contact": bool(actor.call("source_collide_with_ground")),
        "slope_contact": bool(actor.call("source_collide_with_slope")),
        "brain_running": bool(brain.get("running")),
        "brain_failure": str(brain.get("unsupported_failure")),
        "global_field_17": bool(global_brain.get("dsg_17")) if global_brain != null else null,
        "global_field_14": bool(global_brain.get("dsg_14")) if global_brain != null else null,
        "normal_behavior": int(brain.get("current_normal")),
        "crossbow_ai": crossbow_ai_state(),
        "reflex_behavior": int(brain.get("current_reflex")),
        "motion_enabled": bool(actor.get("motion_enabled")),
        "physics_processing": actor.is_physics_processing(),
        "fixture": str(fixture.get("name", "")),
    }

func handle(request: Dictionary) -> Dictionary:
    match str(request.get("action", "")):
        "observe":
            return observation()
        "key":
            var key_name := str(request.get("key", ""))
            var pressed := bool(request.get("pressed", false))
            if key_name == "Keypad0" and pressed: set_crossbow_input_mode(true)
            var event := InputEventKey.new()
            match key_name:
                "ArrowLeft": event.physical_keycode = KEY_LEFT
                "ArrowRight": event.physical_keycode = KEY_RIGHT
                "ArrowUp": event.physical_keycode = KEY_UP
                "ArrowDown": event.physical_keycode = KEY_DOWN
                "LeftShift": event.physical_keycode = KEY_SHIFT
                "LeftControl": event.physical_keycode = KEY_CTRL
                "X": event.physical_keycode = KEY_X
                "Z": event.physical_keycode = KEY_Z
                "Space": event.physical_keycode = KEY_SPACE
                "Keypad0": event.physical_keycode = KEY_KP_0
                "Enter": event.physical_keycode = KEY_ENTER
                _: return {"error": "unknown physical key"}
            event.pressed = pressed
            key_inputs.append({"at_ms": Time.get_ticks_msec(), "key": key_name, "pressed": event.pressed})
            if key_inputs.size() > 30: key_inputs.pop_front()
            Input.parse_input_event(event)
            Input.flush_buffered_events()
            if key_name == "Keypad0" and not pressed: set_crossbow_input_mode(false)
            return observation()
        "screenshot":
            var path := str(request.get("path", ""))
            var texture := root.get_texture()
            if texture == null:
                return {"error": "rendered viewport unavailable for screenshot"}
            var error := texture.get_image().save_png(path)
            return {"error": "" if error == OK else "screenshot failed", "path": path}
        "reload_fixture":
            var overlay: Variant = request.get("overlay", {})
            if not overlay is Dictionary:
                return {"error": "behavior starting-state overlay is malformed"}
            return await reload_fixture(overlay)
        "stop":
            return {"stopped": true}
        _:
            return {"error": "unknown behavior command"}

func reload_fixture(overlay: Dictionary = {}) -> Dictionary:
    brain.set("automatic", false)
    brain.call("stop")
    actor.set_physics_process(false)
    scene.queue_free()
    await process_frame
    actor = null
    brain = null
    global_brain = null
    camera = null
    scene = packed.instantiate() as Node3D
    actor = scene.get_node_or_null("world/0/0/Hype") as CharacterBody3D
    brain = actor.get_node_or_null("Brain") if actor != null else null
    if actor == null or brain == null:
        return {"error": "reloaded saved Hype actor or Brain is missing"}
    action_requests.clear()
    action_states.clear()
    key_inputs.clear()
    generated_trace.clear()
    generated_trace_first.clear()
    generated_trace_first_overflow = false
    last_action_state = -1
    actor.connect("action_request_traced", Callable(self, "record_action_request"))
    actor.connect("generated_object_released", Callable(self, "record_generated_release"))
    brain.set("automatic", false)
    actor.set_physics_process(false)
    root.add_child(scene)
    actor.set_physics_process(false)
    global_brain = scene.get_node_or_null("Globals/Global/Brain")
    if global_brain == null or int(actor.get("current_state")) != int(fixture["expected_action"]):
        return {"error": "reloaded manor scene differs from the save fixture"}
    var restore_error := restore_save_fixture()
    if not restore_error.is_empty():
        return {"error": restore_error}
    var overlay_error := apply_starting_state_overlay(overlay)
    if not overlay_error.is_empty():
        return {"error": overlay_error}
    camera = Camera3D.new()
    camera.name = "BehaviorCamera"
    camera.current = true
    scene.add_child(camera)
    follow_actor()
    await create_timer(5.0).timeout
    actor.set_physics_process(true)
    await create_timer(0.1).timeout
    brain.set("automatic", true)
    if not bool(brain.call("start")):
        return {"error": "reloaded Hype Brain did not start"}
    await create_timer(0.2).timeout
    return observation()
