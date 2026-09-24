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
    restore_save_fixture()
    setup_snapshots.append({"phase": "fixture_restored", "state": observation()})
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

func source_vector(values: Array) -> Vector3:
    return Vector3(float(values[0]), float(values[2]), -float(values[1]))

func restore_save_fixture() -> void:
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

func observation() -> Dictionary:
    if actor == null or brain == null:
        return {"error": "actor or Brain unavailable"}
    var position := actor.global_position
    var facing := actor.global_basis.x.normalized()
    var root_motion: Vector3 = actor.get("root_motion_displacement")
    var external_motion: Vector3 = actor.get("external_velocity")
    var slide_normals: Array = []
    var slide_contacts: Array = []
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
    return {
        "action": int(actor.get("current_state")),
        "translation": [position.x, -position.z, position.y],
        "heading_rad": atan2(-facing.z, facing.x),
        "godot_position": [position.x, position.y, position.z],
        "godot_velocity": [actor.velocity.x, actor.velocity.y, actor.velocity.z],
        "root_motion_displacement": [root_motion.x, root_motion.y, root_motion.z],
        "external_velocity": [external_motion.x, external_motion.y, external_motion.z],
        "generated_object_count": int(actor.get("source_generated_object_count")),
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
                _: return {"error": "unknown physical key"}
            event.pressed = bool(request.get("pressed", false))
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
    brain.set("automatic", false)
    actor.set_physics_process(false)
    root.add_child(scene)
    actor.set_physics_process(false)
    global_brain = scene.get_node_or_null("Globals/Global/Brain")
    if global_brain == null or int(actor.get("current_state")) != int(fixture["expected_action"]):
        return {"error": "reloaded manor scene differs from the save fixture"}
    restore_save_fixture()
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
