extends SceneTree
## Real converted actor, route, animation and manor contact. Other actors' Brains
## are held to isolate this opening slice; this is not full intro acceptance.
var frames: Array = []
var actions: Array[int] = []

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize() -> void:
    call_deferred("check")

func snapshot(actor: CharacterBody3D, brain: Node) -> Dictionary:
    var contacts: Array = []
    for index in actor.get_slide_collision_count():
        var hit := actor.get_slide_collision(index)
        var collider := hit.get_collider() as CollisionObject3D
        var shape: CollisionShape3D
        if collider != null:
            var owner_id := collider.shape_find_owner(hit.get_collider_shape_index())
            shape = collider.shape_owner_get_owner(owner_id) as CollisionShape3D
        var source_node: Node = collider
        while source_node != null and not source_node.has_meta("source_node_id"):
            source_node = source_node.get_parent()
        var point := hit.get_position()
        var normal := hit.get_normal()
        contacts.append({"source_node": source_node.get_meta("source_node_id") if source_node != null else "",
            "source_material": shape.get_meta("source_material", "") if shape != null else "",
            "position": [point.x,-point.z,point.y], "normal": [normal.x,-normal.z,normal.y]})
    return {"clock_ms": brain.clock_milliseconds, "action": actor.current_state,
        "normal": brain.current_normal, "failure": brain.unsupported_failure,
        "position": [actor.global_position.x,-actor.global_position.z,actor.global_position.y],
        "on_floor": actor.is_on_floor(), "slide_count": actor.get_slide_collision_count(),
        "route_point": actor.source_route_points[6], "route_active": actor.source_route_active,
        "contacts": contacts}

func check() -> void:
    var scene := load("res://generated/hype/levels/manoir/level.tscn").instantiate() as Node3D
    for brain in scene.find_children("Brain", "Node", true, false): brain.automatic = false
    var paths: Dictionary = scene.get_meta("source_actor_nodes")
    var actor = scene.get_node(paths["level:/persos/world/0/5"]) as CharacterBody3D
    var hype = scene.get_node(paths["level:/persos/world/0/0"]) as CharacterBody3D
    hype.motion_enabled = false
    root.add_child(scene)
    var fence := scene.get_node("world/1/9/0/Collision") as StaticBody3D
    if not require(fence.collision_layer == 0 and not bool(fence.get_meta("source_collision_reachable")) and fence.get_child_count() > 0, "source display fence became a gameplay blocker"): return
    var brain := actor.get_node("Brain")
    if not require(brain.current_normal == 13 and actor.current_state == 18, "source initial actor state differs"): return
    var start: Vector3 = actor.global_position
    if not require(brain.start() and brain.unsupported_failure.is_empty() and brain.current_normal == 0 and actor.current_state == 0, "source route reset/initialization did not run"): return
    brain.automatic = true
    actions.append(0)
    for tick in range(900):
        await physics_frame
        await process_frame
        if actions[-1] != actor.current_state: actions.append(actor.current_state)
        if tick % 3 == 0 or actions[-1] == 23: frames.append(snapshot(actor, brain))
        if not brain.unsupported_failure.is_empty() or actor.current_state == 23: break
    var finish := snapshot(actor, brain)
    brain.stop()
    var report := {"scope": "exported_gogoud_opening_component", "fixture": "source manor SNA initial actor state, source slot 6; other Brains held; no saved-game restore or geometry/AI edits", "display_fence": {"source_sector": "world/1/9", "native_collision_layer": fence.collision_layer, "retained_shapes": fence.get_child_count()}, "actions": actions, "frames": frames, "finish": finish, "pending": ["Hype initialization after authorized wind no-op", "source opening camera", "dialogue", "full intro acceptance"]}
    var file := FileAccess.open(OS.get_cmdline_user_args()[0], FileAccess.WRITE)
    file.store_string(JSON.stringify(report) + "\n")
    file.close()
    if not require(finish.failure.is_empty() and actions == [0,26,24,23], "opening native action sequence failed: " + str(actions)): return
    if not require(actor.global_position.distance_to(start) > 8.0 and actor.is_on_floor(), "opening actor did not walk with native contact"): return
    # Original first New Game capture settled at this pose after actions 26/24.
    # A component regression permits 0.75 source units; it is not exact timing parity.
    var source_finish := Vector3(-18.638809,32.910141,-149.097321)
    if not require(actor.global_position.distance_to(source_finish) < 0.75, "opening end pose differs from source: " + str(actor.global_position)): return
    print("GOGOUD_OPENING_SLICE_OK ", JSON.stringify(finish))
    scene.queue_free()
    await process_frame
    quit(0)
