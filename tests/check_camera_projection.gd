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
    # This probe inspects the exported camera and runs its synthetic controller
    # cases. Keep unrelated imported actor animation processes out of scope.
    scene.process_mode = Node.PROCESS_MODE_DISABLED
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
    if not require(camera.controller_state.call("mode_zero_collision_selector") == 0, "zero-filled source DAT_00721b0c did not select 0051f670 at startup"): return
    if not require(camera.controller_state.call("set_mode_zero_collision_selector", 1) and camera.controller_state.call("mode_zero_collision_selector") == 1, "source camera-mode input did not retain selector value 1"): return
    if not require(camera.controller_state.call("set_mode_zero_collision_selector", 0) and camera.controller_state.call("mode_zero_collision_selector") == 0, "source camera-mode input did not restore the 0051f670 selector"): return
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
    data3d_actor.process_mode = Node.PROCESS_MODE_DISABLED
    var walk_mechanics: Resource = load("res://addons/astrolabe/character/mechanics.gd").new()
    walk_mechanics.set("mode", 0)
    data3d_actor.set("current_mechanics", walk_mechanics)
    root.add_child(data3d_actor)
    var data3d: Dictionary = data3d_actor.call("source_camera_data3d_inputs")
    var initialized_points: PackedVector3Array = data3d.get("points", PackedVector3Array())
    var initialized_weights: PackedFloat32Array = data3d.get("weights", PackedFloat32Array())
    if not require(data3d.get("supported", false) and data3d.get("sample_count", -1) == 0, "source 0044cf20 Data3D initialization did not expose an empty callback history"): return
    if not require(initialized_points.size() == 8 and initialized_weights.size() == 8, "source 0044cf20 Data3D initialization did not allocate eight slots"): return
    for index in range(8):
        if not require(initialized_points[index] == Vector3.ZERO and initialized_weights[index] == 0.0, "source Data3D initial slots were not zero-filled"): return
    data3d_actor.global_position = Vector3(1,0,0)
    if not require(data3d_actor.call("source_camera_data3d_step", 0.1), "source Walk Data3D producer rejected mechanics type 0"): return
    data3d_actor.global_position = Vector3(2,0,0)
    if not require(data3d_actor.call("source_camera_data3d_step", 0.2), "source Walk Data3D producer rejected a second sample"): return
    data3d = data3d_actor.call("source_camera_data3d_inputs")
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
    probe_sector.set_meta("source_sector", JSON.stringify({"virtual_sector": 0, "activation": 4, "collision": [], "unknown": [0,0,0], "boundary": {}}))
    probe_root.add_child(probe_sector)
    var probe_owner := Node3D.new()
    probe_owner.position = Vector3(0,0.5,0)
    probe_root.add_child(probe_owner)
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
    if not require(probe_owner.get_parent() == probe_root, "camera bootstrap fixture unexpectedly nested its owner under a source sector"): return
    if not require(probe_camera.call("_source_update_sector_history", probe_owner.global_position), "source 004e0020 did not seed boundary history for the unparented camera Perso"): return
    if not require(probe_camera.get("_source_active_sector") == probe_sector, "source 004e0020 bound the camera to the wrong initial sector"): return
    if not require(not probe_camera.source_camera_collision_guard_value(), "fresh SourceCamera did not start with the zero-filled DNM +0x254 guard"): return
    probe_camera.source_camera_collision_callback_update(true)
    if not require(probe_camera.source_camera_collision_guard_value(), "005796ea obstruction result did not set the camera DNM guard"): return
    probe_camera.source_camera_collision_callback_update(false)
    if not require(not probe_camera.source_camera_collision_guard_value(), "next 005796ea callback did not clear the previous camera DNM guard"): return
    probe_owner.global_position = Vector3(0,0.5,-4)
    probe_camera.mode_zero_target_child_index = -1
    probe_camera.mode_zero_distance = Vector2(0,10)
    probe_camera.mode_zero_yaw_limits = Vector2(0,TAU)
    probe_camera.mode_zero_pitch_limits = Vector2(0,TAU)
    probe_camera.mode_zero_parameter_flags = 0
    probe_camera.controller_state = load("res://addons/astrolabe/camera/source_controller_state.gd").new()
    var clear_target := CharacterBody3D.new()
    clear_target.set_script(load("res://addons/astrolabe/character/character.gd"))
    clear_target.process_mode = Node.PROCESS_MODE_DISABLED
    var clear_mechanics: Resource = load("res://addons/astrolabe/character/mechanics.gd").new()
    clear_mechanics.set("mode", 0)
    clear_target.set("current_mechanics", clear_mechanics)
    probe_sector.add_child(clear_target)
    clear_target.global_position = Vector3(0,0.5,0)
    clear_target.call("source_camera_data3d_step", 0.1)
    probe_camera.targeted_actor = clear_target
    if not require(probe_camera.call("source_update_position"), "source mode-0 collision-free candidate path did not complete"): return
    if not require(probe_owner.global_position.is_equal_approx(Vector3(0,0.5,-4)), "source mode-0 clear-ray gate published a translation without a source 0051c7b0 request"): return
    if not require(probe_camera.controller_state.call("mode_zero_update_has_position"), "source 0051f670 clear first ray did not set DAT_007219d0"): return
    if not require(probe_camera.source_controller_candidate_result and probe_camera.source_controller_position_30.is_equal_approx(Vector3(0,0.5,-4)), "source 0051f670 did not retain its +0x30 candidate output"): return

    var blocker := StaticBody3D.new()
    blocker.collision_layer = 1
    blocker.set_meta("source_collision_reachable", true)
    var blocker_shape := CollisionShape3D.new()
    var blocker_box := BoxShape3D.new()
    blocker_box.size = Vector3(1,1,1)
    blocker_shape.shape = blocker_box
    blocker_shape.set_meta("source_collision_identifier", 1)
    blocker.add_child(blocker_shape)
    blocker.position = Vector3(0,0.5,-2)
    probe_sector.add_child(blocker)
    await physics_frame
    var blocked_rays: Dictionary = probe_camera.source_mode_zero_collision_inputs(
        Vector3(0,0.5,0), Vector3(0,0.5,-4), Vector3(0,0.5,-4)
    )
    if not require(blocked_rays.get("anchor_to_eye_blocked", false) and not blocked_rays.get("origin_to_eye_blocked", true), "source candidate ray did not distinguish a blocked anchor segment from a clear zero-length origin segment"): return
    var anchor_hit: Dictionary = blocked_rays.get("anchor_result", {})
    var expected_adjusted: Vector3 = anchor_hit.get("hit_position", Vector3.ZERO) + anchor_hit.get("hit_normal", Vector3.ZERO) * 0.35
    var adjusted_hit: Variant = probe_camera.call("_source_mode_zero_collision_adjusted_position", anchor_hit, 0.35)
    if not require(anchor_hit.get("blocked", false) and adjusted_hit is Vector3 and adjusted_hit.is_equal_approx(expected_adjusted), "source 0051f450 did not apply its hit-point plus normal times 0.35 candidate offset"): return
    blocker_shape.set_meta("source_collision_identifier", 0x8000)
    var filtered_rays: Dictionary = probe_camera.source_mode_zero_collision_inputs(
        Vector3(0,0.5,0), Vector3(0,0.5,-4), Vector3(0,0.5,-4)
    )
    if not require(not filtered_rays.get("anchor_to_eye_blocked", true), "source material identifier bit 0x8000 was not ignored by the Godot collision adapter"): return
    blocker_shape.set_meta("source_collision_identifier", 1)
    probe_camera.mode_zero_distance = Vector2(2.5,10)
    var geometric_classification: Dictionary = probe_camera.source_mode_zero_candidate_classification(
        Vector3(0,0.5,0), Vector3(0,0.5,-4), Vector3(0,0.5,-4)
    )
    var clear_classification: Dictionary = probe_camera.source_mode_zero_candidate_classification(
        Vector3(0,0.5,0), Vector3(0,0.5,-4), Vector3(2,0.5,0)
    )
    var blocked_classification: Dictionary = probe_camera.source_mode_zero_candidate_classification(
        Vector3(0,0.5,0), Vector3(-2,0.5,-4), Vector3(2,0.5,0)
    )
    if not require(clear_classification.get("classification", 0) == 4, "source 0052d1b0 did not accept a clear, in-range fallback candidate"): return
    if not require(blocked_classification.get("classification", 0) == 2, "source 0052d1b0 did not classify a blocked 00463df0 candidate ray"): return
    if not require(geometric_classification.get("classification", 0) == 1, "source 0052d1b0 did not preserve the 0052a740 sector-geometry rejection class"): return
    var near_classification: Dictionary = probe_camera.source_mode_zero_candidate_classification(
        Vector3.ZERO, Vector3(0.5,0,0), Vector3(0.5,0,0)
    )
    var far_classification: Dictionary = probe_camera.source_mode_zero_candidate_classification(
        Vector3.ZERO, Vector3(12,0,0), Vector3(12,0,0)
    )
    if not require(near_classification.get("classification", 0) == 3 and near_classification.get("range_class", 0) == -1, "source 0052caa0 did not classify a candidate inside its minimum margin"): return
    if not require(far_classification.get("classification", 0) == 3 and far_classification.get("range_class", 0) == 1, "source 0052caa0 did not classify a candidate outside its extended maximum"): return
    var fallback_sweep: Dictionary = probe_camera.source_mode_zero_fallback_sweep(
        Vector3(0,0.5,0), Vector3(-2,0.5,-4), Vector3.FORWARD
    )
    if not require(fallback_sweep.get("success", false) and fallback_sweep.get("classification", 0) == 4 and fallback_sweep.get("attempts", 0) > 0, "source 0051fb50 pitch/yaw sweep did not find its first valid 0052d1b0 candidate"): return
    var blocked_owner_position := probe_owner.global_position
    if not require(probe_camera.call("source_update_position"), "source 0051f670 blocked-ray candidate transition did not follow its target-head branch"): return
    if not require(probe_camera.source_controller_candidate_result and probe_camera.source_controller_position_30.is_equal_approx(Vector3(0,0.5,0)), "source 0051f670 did not move the +0x30 candidate back to +0x3c on the clear target-head branch"): return
    if not require(probe_camera.controller_state.call("mode_zero_update_obstructed") and probe_camera.controller_state.call("mode_zero_candidate_obstruction_streak") == 1, "source 0051f670 did not preserve the blocked-ray flags and counter"): return
    if not require(probe_owner.global_position.is_equal_approx(blocked_owner_position), "source mode-0 unsupported blocked branch published an unverified pose"): return

    var controller_blocker := StaticBody3D.new()
    controller_blocker.collision_layer = 1
    controller_blocker.set_meta("source_collision_reachable", true)
    var controller_blocker_shape := CollisionShape3D.new()
    var controller_blocker_box := BoxShape3D.new()
    controller_blocker_box.size = Vector3(0.5,0.5,0.5)
    controller_blocker_shape.shape = controller_blocker_box
    controller_blocker_shape.set_meta("source_collision_identifier", 1)
    controller_blocker.add_child(controller_blocker_shape)
    controller_blocker.position = Vector3(3,3,0)
    probe_sector.add_child(controller_blocker)
    var target_head_blocker := StaticBody3D.new()
    target_head_blocker.collision_layer = 1
    target_head_blocker.set_meta("source_collision_reachable", true)
    var target_head_blocker_shape := CollisionShape3D.new()
    var target_head_blocker_box := BoxShape3D.new()
    target_head_blocker_box.size = Vector3(0.5,0.5,0.5)
    target_head_blocker_shape.shape = target_head_blocker_box
    target_head_blocker_shape.set_meta("source_collision_identifier", 1)
    target_head_blocker.add_child(target_head_blocker_shape)
    target_head_blocker.position = Vector3(1.5,1.4,0)
    probe_sector.add_child(target_head_blocker)
    await physics_frame
    probe_camera.mode_zero_actor_offset = Vector3(2,0,0)
    probe_camera.targeted_actor = clear_target
    var source_collision_context: Dictionary = probe_camera.call("_source_collision_context")
    if not require(not source_collision_context.is_empty(), "source guard regression could not resolve the active collision sector"): return
    var branch_anchor := Vector3(0,5,0)
    var branch_origin := Vector3(3,0.5,0)
    var branch_candidate := Vector3(3,5,0)
    probe_camera.source_camera_collision_callback_update(false)
    probe_camera.controller_state = load("res://addons/astrolabe/camera/source_controller_state.gd").new()
    var unguarded_branch: Dictionary = probe_camera.call(
        "_source_mode_zero_controller_670", branch_anchor, branch_origin,
        branch_candidate, branch_candidate, source_collision_context
    )
    if not require(not unguarded_branch.get("first_ray_initially_blocked", true) and unguarded_branch.get("target_head_checked", false) and unguarded_branch.get("second_ray_blocked", false), "guard regression did not reach the second-ray plus target-head blocked controller branch"): return
    if not require(probe_camera.source_controller_position_30.is_equal_approx(branch_candidate), "zero +0x254 guard did not preserve the ordinary blocked-branch candidate"): return
    probe_camera.source_camera_collision_callback_update(true)
    probe_camera.controller_state = load("res://addons/astrolabe/camera/source_controller_state.gd").new()
    var guarded_branch: Dictionary = probe_camera.call(
        "_source_mode_zero_controller_670", branch_anchor, branch_origin,
        branch_candidate, branch_candidate, source_collision_context
    )
    if not require(not guarded_branch.get("first_ray_initially_blocked", true) and guarded_branch.get("target_head_checked", false) and guarded_branch.get("second_ray_blocked", false) and probe_camera.source_camera_collision_guard_value(), "guarded source callback did not retain its blocked target-head input"): return
    if not require(probe_camera.source_controller_position_30.is_equal_approx(clear_target.global_position + probe_camera.mode_zero_actor_offset), "set +0x254 guard did not select the 0051f670 raw target-plus-offset candidate"): return
    if not require((probe_camera.source_mode_zero_action_flags & 0x414) == 0x414, "set +0x254 guard did not apply the 0051f670 special-case action flags"): return
    probe_camera.source_camera_collision_callback_update(false)
    if not require(not probe_camera.source_camera_collision_guard_value(), "post-obstruction callback did not reset the camera guard for the next controller update"): return
    controller_blocker.queue_free()
    target_head_blocker.queue_free()
    probe_camera.mode_zero_actor_offset = Vector3.ZERO
    await physics_frame

    if not require(probe_camera.controller_state.call("set_mode_zero_collision_selector", 1), "source selector did not switch to 0051f290"): return
    if not require(probe_camera.call("source_update_position"), "source 0051f290/0052bb50 clear camera-ray path did not complete"): return
    if not require(probe_camera.controller_state.call("mode_zero_update_has_position"), "source 0051f290 clear 0052bb50 branch did not set DAT_007219d0"): return
    if not require(int(probe_owner.get_meta("source_camera_static_obstruction_count", 0)) == 1, "source 0052bb50 did not use the first static-geometry obstruction sample"): return

    var gate_script = load("res://addons/astrolabe/camera/source_controller_state.gd")
    var unblocked_gate: Resource = gate_script.new()
    unblocked_gate.call("record_candidate_collision", true)
    var unblocked_gate_result := false
    for _frame in range(11):
        unblocked_gate_result = unblocked_gate.call(
            "evaluate_mode_zero_teleport_gate", true, false, false, true, 1,
            Vector3(20,0,0), Vector3.ZERO, 4.0, false, false
        )
    if not require(unblocked_gate_result, "source 005200a0 distance hysteresis did not reach 0051c7b0 with owner flag bit 2 clear"): return
    var blocked_gate: Resource = gate_script.new()
    blocked_gate.call("record_candidate_collision", true)
    var blocked_gate_result := false
    for _frame in range(11):
        blocked_gate_result = blocked_gate.call(
            "evaluate_mode_zero_teleport_gate", true, false, false, true, 1,
            Vector3(20,0,0), Vector3.ZERO, 4.0, false, true
        )
    if not require(not blocked_gate_result, "source 005200a0 failed to suppress the candidate-result pose write when owner Position bit 2 is set"): return
    var fallback_gate: Resource = gate_script.new()
    fallback_gate.call("record_candidate_collision", true)
    var fallback_gate_result := false
    for _frame in range(11):
        fallback_gate_result = fallback_gate.call(
            "evaluate_mode_zero_teleport_gate", false, true, false, true, 0,
            Vector3.ZERO, Vector3.ZERO, 4.0, false, true
        )
    if not require(fallback_gate_result, "source fallback-result hysteresis incorrectly applied the candidate-branch owner-bit gate"): return
    if not require(probe_camera.source_owner_set_target_offset(Vector3(1,2,3)) and (probe_camera.source_owner_position_flag_word() & 2) != 0, "source 00440fe0 owner flag writer did not set Position bit 2"): return
    if not require(probe_camera.source_owner_clear_position_flags(2) and (probe_camera.source_owner_position_flag_word() & 2) == 0, "source owner flag clear did not restore the source startup flag word"): return

    var resolved_sector: Dictionary = probe_camera.source_resolve_mode_zero_sector(Vector3(0,0.5,0))
    if not require(resolved_sector.get("sector_id", "") == "camera-sector-probe" and resolved_sector.get("complete", false), "source downward sector geometry did not resolve the active border GEO"): return
    if not require(probe_camera.source_publish_mode_zero_translation(Vector3(0,0.5,0),Vector3(0.5,0.5,0)), "source same-sector pose publication failed"): return
    if not require(probe_owner.global_position.is_equal_approx(Vector3(0.5,0.5,0)), "source same-sector pose publication chose the wrong argument"): return
    var next_sector := Node3D.new()
    next_sector.set_meta("source_sector_id", "camera-sector-next")
    next_sector.set_meta("source_sector", JSON.stringify({"virtual_sector": 0, "activation": 4, "collision": [], "unknown": [0,0,0], "boundary": null}))
    probe_root.add_child(next_sector)
    if not require(probe_camera.source_publish_mode_zero_translation(Vector3(2,0.5,0),Vector3(1,0.5,0),"camera-sector-next"), "source cross-sector pose publication failed"): return
    if not require(probe_owner.global_position.is_equal_approx(Vector3(2,0.5,0)), "source cross-sector pose publication chose the wrong argument"): return
    var farther_sector := Node3D.new()
    farther_sector.set_meta("source_sector_id", "camera-sector-farther-boundary")
    farther_sector.set_meta("source_sector", JSON.stringify({"virtual_sector": 0, "activation": 4, "collision": [], "unknown": [0,0,0], "boundary": {}}))
    probe_root.add_child(farther_sector)
    var farther_boundary := MeshInstance3D.new()
    farther_boundary.name = "SectorBoundary"
    farther_boundary.set_script(load("res://addons/astrolabe/scene/sector_boundary.gd"))
    farther_boundary.vertices = PackedVector3Array([
        Vector3(-1,0.25,-1), Vector3(1,0.25,-1), Vector3(0,0.25,1)
    ])
    farther_boundary.neighbors = PackedStringArray([""])
    farther_boundary.surfaces = PackedInt32Array([0])
    farther_boundary.face_normals = PackedVector3Array([Vector3.UP])
    farther_sector.add_child(farther_boundary)
    var gated_child := Node3D.new()
    gated_child.set_meta("source_node_kind", 1)
    probe_sector.add_child(gated_child)
    probe_sector.set_meta("source_sector", JSON.stringify({"virtual_sector": 0, "activation": 4, "collision": [], "unknown": [1,0,0], "boundary": {}}))
    resolved_sector = probe_camera.source_resolve_mode_zero_sector(Vector3(0,0.5,0))
    if not require(resolved_sector.get("complete", false), "source 004e0020 traversed sector children when Sector.unknown[0] disables them"): return
    probe_sector.set_meta("source_sector", JSON.stringify({"virtual_sector": 0, "activation": 4, "collision": [], "unknown": [0,0,0], "boundary": {}}))
    gated_child.free()
    var physical_visual := Node3D.new()
    physical_visual.set_meta("source_node_kind", 13)
    physical_visual.set_meta("source_visual_lod_count", 1)
    probe_sector.add_child(physical_visual)
    var visual_mesh := ArrayMesh.new()
    var visual_arrays: Array = []
    visual_arrays.resize(Mesh.ARRAY_MAX)
    visual_arrays[Mesh.ARRAY_VERTEX] = PackedVector3Array([
        Vector3(-1,0.4,-1), Vector3(1,0.4,-1), Vector3(0,0.4,1)
    ])
    visual_mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, visual_arrays)
    var visual_instance := MeshInstance3D.new()
    visual_instance.mesh = visual_mesh
    physical_visual.add_child(visual_instance)
    resolved_sector = probe_camera.source_resolve_mode_zero_sector(Vector3(0,0.5,0))
    if not require(resolved_sector.get("sector_id", "") == "camera-sector-probe" and absf(float(resolved_sector.get("distance", -1.0)) - 0.1) < 0.00001, "source 004e0020 did not select the nearer active type-13 visual face over another sector border"): return
    var multi_lod_visual := Node3D.new()
    multi_lod_visual.set_meta("source_node_kind", 21)
    multi_lod_visual.set_meta("source_visual_lod_count", 2)
    probe_sector.add_child(multi_lod_visual)
    resolved_sector = probe_camera.source_resolve_mode_zero_sector(Vector3(0,0.5,0))
    if not require(not resolved_sector.get("complete", true), "source 004e0020 claimed a complete result for a multi-LOD child visual"): return
    multi_lod_visual.free()
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
