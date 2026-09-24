extends SceneTree
## Test-owned control surface around the normally exported manor scene.
## The saved Hype actor and Brain make every gameplay decision.

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
var last_action_state: int = -1

func _initialize() -> void:
    call_deferred("run")

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
    fixture = JSON.parse_string(FileAccess.get_file_as_string("res://tests/fixtures/english_save_manoir.json"))
    if int(fixture.get("schema", 0)) != 1 or str(fixture.get("name", "")) != "supplied-english-save-manoir":
        write_json(control_dir.path_join("ready.json"), {"error": "invalid English save fixture"})
        quit(1)
        return
    # The fixture restores the measured state of the supplied English save.
    # It uses the normally exported Hype actor, Brain, and compiled scripts.
    brain.set("automatic", false)
    actor.set_physics_process(false)
    root.add_child(scene)
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
        track_generated_objects()
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
    if generated_trace.size() > 120: generated_trace.pop_front()

func record_generated_exit(spawned: CharacterBody3D) -> void:
    var entry := generated_object_snapshot(spawned)
    entry["event"] = "tree_exiting"
    entry["at_ms"] = Time.get_ticks_msec()
    generated_trace.append(entry)
    if generated_trace.size() > 120: generated_trace.pop_front()

func source_vector(values: Array) -> Vector3:
    return Vector3(float(values[0]), float(values[2]), -float(values[1]))

func designer_action_arrays() -> Dictionary:
    var result: Dictionary = {}
    for index in [162, 164]:
        var values: Array = []
        for value in brain.get("dsg_" + str(index)):
            values.append(int(value))
        result[str(index)] = values
    return result

func restore_save_fixture() -> String:
    var rows: Array = fixture["source_basis"]
    var basis := Basis(
        source_vector(rows[0]),
        source_vector(rows[2]),
        -source_vector(rows[1]))
    actor.global_transform = Transform3D(basis, source_vector(fixture["source_translation"]))
    brain.set("current_normal", int(fixture["normal_behavior"]))
    brain.set("current_reflex", int(fixture["reflex_behavior"]))
    for index in fixture["designer_overrides"]:
        brain.set("dsg_" + str(index), fixture["designer_overrides"][index])
    var deactivated: Array = fixture.get("deactivated_channels", [])
    var seen: Dictionary = {}
    for value in deactivated:
        if not (value is int or value is float):
            return "saved Hype deactivated channel list is malformed"
        var channel := int(value)
        if float(channel) != float(value) or channel < 0 or channel > 255 or seen.has(channel):
            return "saved Hype deactivated channel list is malformed"
        seen[channel] = true
        if not bool(actor.call("source_set_channel_active", channel, false)):
            return "saved Hype channel " + str(channel) + " is missing"
    var inventory: Dictionary = fixture.get("inventory", {})
    var native_inventory: Resource = actor.get("source_inventory")
    if native_inventory == null or not inventory.has("selected_weapon") or not inventory.has("quantities"):
        return "saved Hype inventory fixture is missing"
    var selected = inventory["selected_weapon"]
    if not (selected is int or selected is float) or float(int(selected)) != float(selected) or int(selected) < -1 or int(selected) > 32767:
        return "saved Hype selected weapon is malformed"
    var quantities: Dictionary = inventory["quantities"]
    var restored_quantities: Dictionary = {}
    for key in quantities:
        if not str(key).is_valid_int():
            return "saved Hype inventory item is malformed"
        var item := int(key)
        var amount = quantities[key]
        if item < 0 or item > 32767 or not (amount is int or amount is float) or float(int(amount)) != float(amount):
            return "saved Hype inventory quantity is malformed"
        restored_quantities[item] = int(amount)
    native_inventory.set("selected_weapon", int(selected))
    native_inventory.set("quantities", restored_quantities)
    var magick: Dictionary = fixture.get("magick", {})
    var native_magick: Resource = actor.get("source_magick")
    if native_magick == null or not magick.has("source_code") or not magick.has("current_points") or not magick.has("maximum_points"):
        return "saved Hype magick fixture is missing"
    var spell_code = magick["source_code"]
    var current_points = magick["current_points"]
    var maximum_points = magick["maximum_points"]
    for value in [spell_code, current_points, maximum_points]:
        if not (value is int or value is float) or float(int(value)) != float(value):
            return "saved Hype magick value is malformed"
    if int(spell_code) < 0 or int(spell_code) > 32767 or int(maximum_points) <= 0 or int(current_points) < 0 or int(current_points) > int(maximum_points):
        return "saved Hype magick value is out of range"
    native_magick.set("selected_spell_code", int(spell_code))
    native_magick.set("current_points", int(current_points))
    native_magick.set("maximum_points", int(maximum_points))
    var arrays: Dictionary = fixture.get("designer_arrays", {})
    for index in [162, 164]:
        var key := str(index)
        if not arrays.has(key):
            return "saved Hype designer array " + key + " is missing"
        var existing = brain.get("dsg_" + key)
        var saved = arrays[key]
        if not (existing is PackedInt32Array) or not (saved is Array) or existing.size() != saved.size():
            return "saved Hype designer array " + key + " has the wrong type or length"
        var restored := PackedInt32Array()
        for value in saved:
            restored.append(int(value))
        brain.set("dsg_" + key, restored)
    return ""

func generated_objects() -> Array:
    var result: Array = []
    for candidate in root.find_children("*", "CharacterBody3D", true, false):
        if not candidate.has_meta("astrolabe_generated_template"): continue
        result.append(generated_object_snapshot(candidate as CharacterBody3D))
    return result

func generated_object_snapshot(spawned: CharacterBody3D) -> Dictionary:
    var position := spawned.global_position
    var velocity: Vector3 = spawned.get("external_velocity")
    var spawned_brain := spawned.get_node_or_null("Brain")
    return {
        "instance_id": spawned.get_instance_id(),
        "template": str(spawned.get_meta("astrolabe_generated_template")),
        "translation": [position.x, -position.z, position.y],
        "velocity": [velocity.x, -velocity.z, velocity.y],
        "action": int(spawned.get("current_state")),
        "brain_running": bool(spawned_brain.get("running")) if spawned_brain != null else false,
        "brain_normal": int(spawned_brain.get("current_normal")) if spawned_brain != null else -1,
        "brain_clock_ms": float(spawned_brain.get("clock_milliseconds")) if spawned_brain != null else 0.0,
        "brain_start_ms": int(spawned_brain.get("dsg_0")) if spawned_brain != null else 0,
        "brain_failure": str(spawned_brain.get("unsupported_failure")) if spawned_brain != null else "",
        "visible_from_hype": bool(spawned.call("source_see_perso", actor, spawned)),
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
        "channel_visibility": channel_visibility,
        "hud_display": actor.get("source_hud_display"),
        "magick": {
            "selected_spell_code": int(actor.call("source_magic_current_spell")) if magick_ready else null,
            "current_points": int(actor.call("source_magic_points")) if magick_ready else null,
            "maximum_points": int(native_magick.get("maximum_points")) if native_magick != null else null,
        },
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
        "normal_behavior": int(brain.get("current_normal")),
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
            var event := InputEventKey.new()
            match str(request.get("key", "")):
                "ArrowLeft": event.physical_keycode = KEY_LEFT
                "ArrowRight": event.physical_keycode = KEY_RIGHT
                "ArrowUp": event.physical_keycode = KEY_UP
                "ArrowDown": event.physical_keycode = KEY_DOWN
                "LeftShift": event.physical_keycode = KEY_SHIFT
                "LeftControl": event.physical_keycode = KEY_CTRL
                "X": event.physical_keycode = KEY_X
                "Z": event.physical_keycode = KEY_Z
                "Space": event.physical_keycode = KEY_SPACE
                "Enter": event.physical_keycode = KEY_ENTER
                _: return {"error": "unknown physical key"}
            event.pressed = bool(request.get("pressed", false))
            key_inputs.append({"at_ms": Time.get_ticks_msec(), "key": str(request.get("key", "")), "pressed": event.pressed})
            if key_inputs.size() > 30: key_inputs.pop_front()
            Input.parse_input_event(event)
            Input.flush_buffered_events()
            return observation()
        "screenshot":
            var path := str(request.get("path", ""))
            var texture := root.get_texture()
            if texture == null:
                return {"error": "rendered viewport unavailable for screenshot"}
            var error := texture.get_image().save_png(path)
            return {"error": "" if error == OK else "screenshot failed", "path": path}
        "reload_fixture":
            return await reload_fixture()
        "stop":
            return {"stopped": true}
        _:
            return {"error": "unknown behavior command"}

func reload_fixture() -> Dictionary:
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
    last_action_state = -1
    actor.connect("action_request_traced", Callable(self, "record_action_request"))
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
