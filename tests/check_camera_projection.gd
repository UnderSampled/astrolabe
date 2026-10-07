extends SceneTree
## Load the actual saved level; no preview bootstrap creates its camera.

func _initialize() -> void:
    call_deferred("run")

func require(value: bool, message: String) -> bool:
    if value: return true
    push_error(message)
    quit(1)
    return false

func run() -> void:
    var scene := load("res://generated/hype/levels/manoir/level.tscn").instantiate() as Node3D
    var actor := scene.get_node("world/camera/SourceCameraActor") as Node3D
    actor.set("motion_enabled", false)
    var camera := actor.get_node("SourceCamera") as Camera3D
    root.add_child(scene)
    if not require(camera.global_position.is_equal_approx(Vector3(3,2,-4)), "saved camera placement differs"): return
    if not require((-camera.global_basis.z).is_equal_approx(Vector3.RIGHT), "source camera view axis is reversed"): return
    if not require(camera.keep_aspect == Camera3D.KEEP_WIDTH and absf(deg_to_rad(camera.fov) - 1.2) < 0.000001, "edited horizontal FoV differs"): return
    if not require(camera.near == 0.25 and camera.far == 10000.0, "saved clip planes differ"): return
    if not require(camera.targeted_actor == scene.get_node("world/main/MainActor"), "camera did not bind the saved MainActor target"): return
    if not require(camera.mode_zero_target_child_index == -1, "manor source selector did not choose the bound Perso itself"): return
    var source_orientation: Dictionary = camera.source_mode_zero_orientation_54()
    var expected_orientation: Vector3 = camera.targeted_actor.global_basis.orthonormalized() * Vector3.FORWARD
    if not require(source_orientation.get("orientation", Vector3.ZERO).is_equal_approx(expected_orientation), "source DNM selector did not map to the target's source +Y axis"): return
    var startup_limits: Vector2 = camera.source_mode_zero_vertical_limits()
    if not require(startup_limits.x < -1.0e30 and startup_limits.y > 1.0e30, "source Position startup height sentinels were not retained"): return
    if not require(camera.current, "source registration context 1 did not select the primary viewport camera"): return
    camera.controller_state = load("res://addons/astrolabe/camera/source_controller_state.gd").new()
    var source_points := PackedVector3Array([
        Vector3(0,0,0), Vector3(1,0,0), Vector3(2,0,0), Vector3(3,0,0),
        Vector3(4,0,0), Vector3(5,0,0), Vector3(6,0,0), Vector3(7,0,0)
    ])
    var source_weights := PackedFloat32Array([0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1])
    var pivot: Dictionary = camera.source_mode_zero_weighted_pivot(source_points,source_weights,Vector3.ZERO)
    var pivot_anchor: Vector3 = pivot.get("anchor", Vector3.ZERO)
    if not require(pivot.has("anchor") and pivot_anchor.is_equal_approx(Vector3(1.5,0,0)), "source weighted pivot did not stop at the 0.4 threshold"): return
    var distant_points := PackedVector3Array()
    distant_points.resize(8)
    distant_points.fill(Vector3(4,0,0))
    var held_pivot: Dictionary = camera.source_mode_zero_weighted_pivot(distant_points,source_weights,Vector3.ZERO)
    var held_anchor: Vector3 = held_pivot.get("anchor", Vector3.ONE)
    if not require(held_pivot.has("anchor") and held_anchor.is_equal_approx(Vector3.ZERO), "source squared 9.0 pivot threshold did not hold raw target translation"): return
    var near_points := PackedVector3Array()
    near_points.resize(8)
    near_points.fill(Vector3(1,0,0))
    for _frame in range(7):
        held_pivot = camera.source_mode_zero_weighted_pivot(near_points,source_weights,Vector3.ZERO)
    held_anchor = held_pivot.get("anchor", Vector3.ONE)
    if not require(held_anchor.is_equal_approx(Vector3.ZERO), "source pivot hold ended before eight updates"): return
    held_pivot = camera.source_mode_zero_weighted_pivot(near_points,source_weights,Vector3.ZERO)
    held_anchor = held_pivot.get("anchor", Vector3.ONE)
    if not require(held_anchor.is_equal_approx(Vector3(1,0,0)), "source pivot hold did not release after eight updates"): return

    var data3d_actor := CharacterBody3D.new()
    data3d_actor.set_script(load("res://addons/astrolabe/character/character.gd"))
    data3d_actor.set("motion_enabled", false)
    var walk_mechanics: Resource = load("res://addons/astrolabe/character/mechanics.gd").new()
    walk_mechanics.set("mode", 0)
    data3d_actor.set("current_mechanics", walk_mechanics)
    root.add_child(data3d_actor)
    data3d_actor.global_position = Vector3(1,0,0)
    if not require(data3d_actor.call("source_camera_data3d_step", 0.1), "source Walk Data3D producer rejected mechanics type 0"): return
    data3d_actor.global_position = Vector3(2,0,0)
    if not require(data3d_actor.call("source_camera_data3d_step", 0.2), "source Walk Data3D producer rejected a second sample"): return
    var data3d: Dictionary = data3d_actor.call("source_camera_data3d_inputs")
    var data3d_points: PackedVector3Array = data3d.get("points", PackedVector3Array())
    var data3d_weights: PackedFloat32Array = data3d.get("weights", PackedFloat32Array())
    if not require(data3d.get("supported", false) and data3d.get("callback_address", 0) == 0x00585d51, "source mechanics type 0 did not select the mapped Walk callback"): return
    if not require(data3d.get("sample_count", 0) == 2 and data3d_points.size() == 8 and data3d_weights.size() == 8, "source Data3D producer did not expose an eight-slot history and its live count"): return
    if not require(data3d_points[0].is_equal_approx(Vector3(2,0,0)) and data3d_points[1].is_equal_approx(Vector3(1,0,0)), "source Data3D samples are not newest-first post-movement world positions"): return
    if not require(absf(data3d_weights[0] - 0.2) < 0.000001 and absf(data3d_weights[1] - 0.1) < 0.000001, "source Data3D weights do not track the callback frame deltas"): return
    var data3d_owner := Node3D.new()
    var data3d_camera := Camera3D.new()
    data3d_camera.set_script(load("res://addons/astrolabe/camera/source_camera.gd"))
    data3d_owner.add_child(data3d_camera)
    root.add_child(data3d_owner)
    data3d_camera.targeted_actor = data3d_actor
    data3d_camera.mode_zero_target_child_index = -1
    data3d_camera.mode_zero_actor_offset = Vector3(0,1,0)
    data3d_camera.controller_state = load("res://addons/astrolabe/camera/source_controller_state.gd").new()
    var live_pivot: Dictionary = data3d_camera.call("source_mode_zero_live_pivot")
    var live_anchor: Vector3 = live_pivot.get("anchor", Vector3.ZERO)
    var live_focus: Vector3 = live_pivot.get("focus", Vector3.ZERO)
    if not require(live_pivot.get("sample_count", 0) == 2 and live_anchor.distance_to(Vector3(1.6666667,1,0)) < 0.00001, "live mode-0 pivot did not consume Data3D history and the target-rotated actor offset"): return
    if not require(live_focus.distance_to(Vector3(1.6666667,1,0.05)) < 0.00001, "live mode-0 focus did not follow the smoothed Data3D pivot"): return
    walk_mechanics.set("mode", 4)
    if not require(not data3d_actor.call("source_camera_data3d_step", 0.1), "unsupported identity-card type 4 entered the Walk Data3D producer"): return
    data3d = data3d_actor.call("source_camera_data3d_inputs")
    if not require(not data3d.get("supported", true), "unsupported identity-card type 4 did not fail closed"): return
    walk_mechanics.set("mode", 0)
    data3d_actor.global_position = Vector3(3,0,0)
    if not require(data3d_actor.call("source_camera_data3d_step", 0.05), "source Walk Data3D producer did not resume after restoring mechanics type 0"): return
    data3d = data3d_actor.call("source_camera_data3d_inputs")
    data3d_points = data3d.get("points", PackedVector3Array())
    data3d_weights = data3d.get("weights", PackedFloat32Array())
    if not require(data3d.get("sample_count", 0) == 1 and data3d_points[0].is_equal_approx(Vector3(3,0,0)) and absf(data3d_weights[0] - 0.05) < 0.000001, "identity-card callback transition did not clear stale Data3D history"): return
    data3d_owner.free()
    data3d_actor.free()

    camera.mode_zero_distance = Vector2(0, 10)
    camera.mode_zero_yaw_limits = Vector2(0, TAU)
    camera.mode_zero_pitch_limits = Vector2(0, TAU)
    camera.mode_zero_parameter_flags = 0
    var candidate: Dictionary = camera.source_compute_mode_zero_candidate(
        Vector3.ZERO, Vector3(0,2,-3), Vector3.FORWARD, Vector2(-10,10)
    )
    var candidate_position: Vector3 = candidate.get("position", Vector3.ZERO)
    if not require(candidate.has("position") and candidate_position.distance_to(Vector3(0,2,-3)) < 0.00001, "source mode-0 yaw/pitch and distance calculation differs in Godot's Y-up basis"): return
    var height_clamped: Dictionary = camera.source_compute_mode_zero_candidate(
        Vector3.ZERO, Vector3(0,2,-3), Vector3.FORWARD, Vector2(-10,1)
    )
    var clamped_position: Vector3 = height_clamped.get("position", Vector3.ZERO)
    if not require(height_clamped.has("position") and clamped_position.distance_to(Vector3(0,1,-3)) < 0.00001, "source +Z height limit did not map to Godot Y"): return
    camera.mode_zero_distance = Vector2(3.5,4.0)
    if not require(camera.source_mode_zero_distance_class(Vector3(3.498,0,0),Vector3.ZERO) == -1, "source 0052ce20 lower range class differs"): return
    if not require(camera.source_mode_zero_distance_class(Vector3(4.300,0,0),Vector3.ZERO) == 0, "source 0052ce20 +0.3 upper padding/epsilon differs"): return
    if not require(camera.source_mode_zero_distance_class(Vector3(4.302,0,0),Vector3.ZERO) == 1, "source 0052ce20 upper range class differs"): return
    var boundary := MeshInstance3D.new()
    boundary.set_script(load("res://addons/astrolabe/scene/sector_boundary.gd"))
    boundary.vertices = PackedVector3Array([
        Vector3(1,-1,-1), Vector3(1,1,-1), Vector3(1,0,1)
    ])
    boundary.neighbors = PackedStringArray(["next-sector"])
    boundary.surfaces = PackedInt32Array([0])
    boundary.face_normals = PackedVector3Array([Vector3.RIGHT])
    root.add_child(boundary)
    var crossing: Dictionary = boundary.first_source_sector_crossing(Vector3(2,0,0),Vector3.ZERO)
    var crossing_position: Vector3 = crossing.get("position", Vector3.ZERO)
    if not require(crossing.get("sector", "") == "next-sector" and crossing_position.is_equal_approx(Vector3.RIGHT), "source front-to-back sector crossing differs"): return
    if not require(boundary.first_source_sector_crossing(Vector3.ZERO,Vector3(2,0,0)).is_empty(), "source sector boundary accepted a back-to-front crossing"): return
    boundary.free()
    var probe_root := Node3D.new()
    probe_root.set_meta("source_level_scene", true)
    var probe_sector := Node3D.new()
    probe_sector.set_meta("source_sector_id", "camera-sector-probe")
    probe_sector.set_meta("source_sector", JSON.stringify({"virtual_sector": 0, "activation": 4, "collision": []}))
    probe_root.add_child(probe_sector)
    var probe_owner := Node3D.new()
    probe_owner.position = Vector3(0,0.5,0)
    probe_sector.add_child(probe_owner)
    var probe_camera := Camera3D.new()
    probe_camera.set_script(load("res://addons/astrolabe/camera/source_camera.gd"))
    probe_owner.add_child(probe_camera)
    var floor_boundary := MeshInstance3D.new()
    floor_boundary.name = "SectorBoundary"
    floor_boundary.set_script(load("res://addons/astrolabe/scene/sector_boundary.gd"))
    floor_boundary.vertices = PackedVector3Array([
        Vector3(-1,0,-1), Vector3(1,0,-1), Vector3(0,0,1)
    ])
    floor_boundary.neighbors = PackedStringArray([""])
    floor_boundary.surfaces = PackedInt32Array([0])
    floor_boundary.face_normals = PackedVector3Array([Vector3.UP])
    probe_sector.add_child(floor_boundary)
    root.add_child(probe_root)
    probe_owner.global_position = Vector3(0,0.5,-4)
    probe_camera.mode_zero_target_child_index = -1
    probe_camera.mode_zero_distance = Vector2(0,10)
    probe_camera.mode_zero_yaw_limits = Vector2(0,TAU)
    probe_camera.mode_zero_pitch_limits = Vector2(0,TAU)
    probe_camera.mode_zero_parameter_flags = 0
    probe_camera.controller_state = load("res://addons/astrolabe/camera/source_controller_state.gd").new()
    var clear_target := CharacterBody3D.new()
    clear_target.set_script(load("res://addons/astrolabe/character/character.gd"))
    var clear_mechanics: Resource = load("res://addons/astrolabe/character/mechanics.gd").new()
    clear_mechanics.set("mode", 0)
    clear_target.set("current_mechanics", clear_mechanics)
    probe_sector.add_child(clear_target)
    clear_target.global_position = Vector3(0,0.5,0)
    clear_target.call("source_camera_data3d_step", 0.1)
    probe_camera.targeted_actor = clear_target
    if not require(probe_camera.call("source_update_position"), "source mode-0 collision-free candidate path did not complete"): return
    if not require(probe_owner.global_position.is_equal_approx(Vector3(0,0.5,-4)), "source mode-0 clear-ray gate published a translation without a source 0051c7b0 request"): return
    var resolved_sector: Dictionary = probe_camera.source_resolve_mode_zero_sector(Vector3(0,0.5,0))
    if not require(resolved_sector.get("sector_id", "") == "camera-sector-probe" and resolved_sector.get("complete", false), "source downward sector geometry did not resolve the active border GEO"): return
    if not require(probe_camera.source_publish_mode_zero_translation(Vector3(0,0.5,0),Vector3(0.5,0.5,0)), "source same-sector pose publication failed"): return
    if not require(probe_owner.global_position.is_equal_approx(Vector3(0.5,0.5,0)), "source same-sector pose publication chose the wrong argument"): return
    var next_sector := Node3D.new()
    next_sector.set_meta("source_sector_id", "camera-sector-next")
    next_sector.set_meta("source_sector", JSON.stringify({"virtual_sector": 0, "activation": 4, "collision": []}))
    probe_root.add_child(next_sector)
    if not require(probe_camera.source_publish_mode_zero_translation(Vector3(2,0.5,0),Vector3(1,0.5,0),"camera-sector-next"), "source cross-sector pose publication failed"): return
    if not require(probe_owner.global_position.is_equal_approx(Vector3(2,0.5,0)), "source cross-sector pose publication chose the wrong argument"): return
    var unmodeled_child := Node3D.new()
    unmodeled_child.set_meta("source_node_kind", 13)
    probe_sector.add_child(unmodeled_child)
    resolved_sector = probe_camera.source_resolve_mode_zero_sector(Vector3(0,0.5,0))
    if not require(resolved_sector.get("sector_id", "") == "camera-sector-probe" and not resolved_sector.get("complete", true), "source sector resolver claimed completeness with an unmodeled physical visual child"): return
    probe_root.free()
    camera.make_current()
    var projection := camera.get_camera_projection()
    if not require(absf(projection.x.x - 1.0 / tan(0.6)) < 0.00001, "native horizontal projection differs"): return
    scene.free()
    print("SOURCE_CAMERA_PROJECTION_OK")
    quit()
