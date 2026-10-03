extends SceneTree

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize() -> void:
    call_deferred("check")

func check() -> void:
    var packed := load("res://generated/hype/levels/routes/level.tscn") as PackedScene
    var scene := packed.instantiate() as Node3D
    for brain in scene.find_children("Brain", "Node", true, false): brain.automatic = false
    var actor = scene.get_node("walker/Walker")
    actor.motion_enabled = false
    var second = actor.duplicate()
    second.name = "Independent"
    second.get_node("Brain").automatic = false
    second.motion_enabled = false
    scene.add_child(second)
    root.add_child(scene)
    if not require(actor.source_routes[0] == second.source_routes[0], "route resource is not shared"): return
    if not require(actor.source_routes[0].curve.get_point_position(1) == Vector3(3,0,0), "Rete waypoint edit not baked into native curve"): return
    var wall := scene.get_node("listed/wall/Collision") as StaticBody3D
    var display_wall := scene.get_node("display/wall/Collision") as StaticBody3D
    if not require(wall.collision_layer == 1 and bool(wall.get_meta("source_collision_reachable")), "listed virtual collision sector lost native contact"): return
    if not require(display_wall.collision_layer == 0 and not bool(display_wall.get_meta("source_collision_reachable")) and display_wall.get_child_count() == 1, "unlisted display collision was not retained without contact"): return
    actor.position.y = 0.01
    var brain = actor.get_node("Brain")
    if not require(brain.start() and brain.dsg_1 == 7 and brain.unsupported_failure.is_empty(), "wind no-op halted initialization"): return
    if not require(actor.source_vector_speed == Vector3.ZERO and actor.external_velocity == Vector3.ZERO, "authorized wind no-op changed motion state"): return
    brain.automatic = true
    for _tick in 65: await physics_frame
    if not require(actor.is_on_floor() and actor.is_on_wall() and brain.dsg_0 == 0, "route bypassed native contact or completed at wall"): return
    if not require(absf(actor.global_position.x - 0.65) < 0.03 and absf(actor.global_position.z) < 0.03, "route penetrated wall"): return
    if not require(actor.global_basis.z.dot(Vector3.RIGHT) > 0.99, "route faces opposite source negative-Y forward"): return
    if not require(actor.source_route_points[0] == 1 and second.source_route_points[0] == 0, "route progress leaked to another actor"): return
    brain.stop()
    var stopped: Vector3 = actor.global_position
    for _tick in 15: await physics_frame
    if not require(not actor.source_route_active and actor.global_position.distance_to(stopped) < 0.01, "cancelled route kept steering or moving"): return
    wall.queue_free()
    await physics_frame
    await physics_frame
    if not require(brain.start(), "route restart failed"): return
    for _tick in 230:
        await physics_frame
        if brain.dsg_0 == 1: break
    if not require(brain.dsg_0 == 1 and brain.unsupported_failure.is_empty() and actor.current_state == 1, "route did not resume compiled schedule after arrival"): return
    if not require(actor.global_position.distance_to(Vector3(3,0,0)) <= 0.27 and actor.is_on_floor(), "wrong final native arrival/contact"): return
    brain.stop()
    print("NATIVE_ROUTES_WIND_DEBT_OK ", JSON.stringify({"position": actor.global_position, "independent_progress": second.source_route_points[0], "collision_sectors": "listed virtual wall blocked; unlisted display wall crossed; native floor contact", "wind": "intentional no-op; physics parity deferred"}))
    scene.queue_free()
    await process_frame
    quit(0)
