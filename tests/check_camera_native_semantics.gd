extends SceneTree
## Source-backed API behavior using the maintained native camera component.
const SOURCE_CAMERA = preload("res://addons/astrolabe/camera/source_camera.gd")
const CONTROLLER = preload("res://addons/astrolabe/camera/source_controller_state.gd")

class AlphaActor extends Node3D:
    var alpha := 17
    func source_set_transparency(value: int) -> bool:
        alpha = value
        return true

class InterruptedFollow extends "res://addons/astrolabe/camera/source_camera.gd":
    var allow_frame := false
    func _source_update_mode_zero() -> bool:
        source_mode_zero_action_flags |= 0x414
        return allow_frame

func _initialize() -> void:
    call_deferred("check")

func require(ok: bool, message: String) -> bool:
    if ok:
        return true
    push_error(message)
    quit(1)
    return false

func check() -> void:
    var level := Node3D.new()
    root.add_child(level)
    var owner := Node3D.new()
    level.add_child(owner)
    var camera := SOURCE_CAMERA.new()
    camera.rotation.y = PI
    camera.owner_motion_binding_present = true
    camera.mode_zero_target_child_index = -1
    camera.controller_state = CONTROLLER.new()
    owner.add_child(camera)
    var old_target := AlphaActor.new()
    var new_target := AlphaActor.new()
    level.add_child(old_target)
    level.add_child(new_target)
    camera.targeted_actor = old_target
    camera.fallback_motion_frames = 7
    camera.collision_recovery_frames = 8
    camera.look_direction_hold_frames = 9
    camera.static_probe_frames = 10
    camera.static_obstruction_frames = 11
    camera.previous_look_radius = 12.0
    camera.slope_pitch_adjustment = 0.4
    camera.controller_state.record_mode_zero_good_position(Vector3(4,5,6))
    var eye := owner.global_transform
    # Proc160's first phase is before operand evaluation. Failed new resolution
    # keeps the previous target and progress but still restores its alpha.
    if not require(camera.source_prepare_target_change() and old_target.alpha == 255, "Proc160 failed to restore the old target before evaluation"): return
    if not require(not camera.source_change_targeted_perso(null) and camera.targeted_actor == old_target and camera.static_probe_frames == 10, "failed target evaluation changed progress"): return
    if not require(camera.source_change_targeted_perso(new_target) and new_target.alpha == 17, "Proc160 restored the new target instead of the previous one"): return
    if not require(camera.fallback_motion_frames == 0 and camera.collision_recovery_frames == 0 and camera.look_direction_hold_frames == 0 and camera.static_probe_frames == 0, "Proc160 did not reset its four camera progress values"): return
    if not require(camera.previous_look_radius == 0.0 and camera.slope_pitch_adjustment == 0.0, "Proc160 retained old look histories"): return
    if not require(camera.static_obstruction_frames == 11 and camera.controller_state.mode_zero_last_good_position() == Vector3(4,5,6), "Proc160 reset unrelated obstruction or session history"): return
    camera.static_probe_frames = 3
    camera.previous_look_radius = 2.0
    camera.slope_pitch_adjustment = 0.2
    if not require(camera.source_reset_targeted_perso(old_target) and new_target.alpha == 17 and camera.static_probe_frames == 3 and camera.previous_look_radius == 2.0 and camera.slope_pitch_adjustment == 0.2, "Proc163 performed Proc160 resets"): return
    if not require(owner.global_transform.is_equal_approx(eye), "target changes moved the camera eye"): return

    camera.mode_zero_distance = Vector2(3.5,4.0)
    camera.mode_zero_yaw_limits = Vector2(0,TAU)
    camera.mode_zero_pitch_limits = Vector2(0,TAU)
    for row in [[1.0,3.5],[3.5,3.5],[3.75,3.75],[4.0,4.0],[9.0,4.0]]:
        var candidate: Dictionary = camera.source_compute_mode_zero_candidate(Vector3.ZERO, Vector3(0,0,-float(row[0])), Vector3.FORWARD, Vector2(-10,10))
        if not require(not candidate.is_empty() and absf(float(candidate["distance"]) - float(row[1])) < 0.000001 and candidate["position"].distance_to(Vector3(0,0,-float(row[1]))) < 0.000001, "0051db70 minimum/maximum distance clamp differs"): return
    # Source word increment is signed at comparison time. Bit8 suppresses the
    # ray but does not suppress the increment, as 0052bb50's order requires.
    camera.static_probe_frames = 32767
    camera.mode_zero_parameter_flags = 8
    var skipped: Variant = camera.call("_source_mode_zero_should_wait_for_static_obstruction", Vector3.ZERO, Vector3.ONE, Vector3.ONE, {})
    if not require(skipped == false and camera.static_probe_frames == -32768, "static-probe counter did not narrow before its signed comparison"): return
    camera.static_probe_frames = 6
    camera.static_obstruction_frames = 21
    if not require(camera.source_change_state(3), "authored mode3 transition failed"): return
    owner.global_position = Vector3(2,3,4)
    old_target.global_position = Vector3(-3,2,-4)
    old_target.rotation.y = 0.6
    camera.actor_offset = Vector3(0,1,0)
    camera.look_offset = Vector3(0.3,0,0.1)
    var focus := old_target.global_position + old_target.global_basis * (camera.actor_offset + camera.look_offset)
    if not require(camera.source_set_eye_displacement(Vector3(1,2,3)), "camera displacement request failed"): return
    if not require(not owner.has_meta("source_position_flag_word") and not owner.has_meta("source_position_target_offset"), "camera request leaked into parent Position metadata"): return
    var position := owner.global_position
    if not require(camera.source_update_position(), "native mode3 update failed"): return
    if not require(owner.global_position == position and (-camera.global_basis.z).normalized().distance_to((focus-position).normalized()) < 0.00001, "mode3 eye/focus binding differs"): return
    if not require((camera.mode_zero_parameter_flags & 8) == 0 and not camera.source_has_eye_displacement(), "bound-motion update retained one-update camera flags"): return
    if not require(camera.static_probe_frames == 0 and camera.static_obstruction_frames == 21 and camera.previous_look_radius == 2.0, "mode3 calculation reset unrelated history"): return
    # Null caller motion binding applies the pose then jumps over all ordinary
    # cleanup. The active parameter bit and supplied motion must survive.
    camera.owner_motion_binding_present = false
    camera.mode_zero_parameter_flags |= 8
    camera.source_set_eye_displacement(Vector3.ONE)
    old_target.position += Vector3.RIGHT
    if not require(camera.source_update_position() and camera.source_has_eye_displacement() and (camera.mode_zero_parameter_flags & 8) != 0, "null motion-binding branch performed ordinary cleanup"): return
    if not require(owner.global_position == position, "null motion-binding update moved the authored eye"): return
    if not require(camera.source_change_state(6), "traveling intro camera mode6 was rejected"): return
    var held_basis := owner.global_basis
    owner.global_position += Vector3(0,0,2)
    old_target.position += Vector3(5,1,0)
    var traveling_eye := owner.global_position
    if not require(camera.source_update_position() and owner.global_basis.is_equal_approx(held_basis) and owner.global_position == traveling_eye, "mode6 changed the traveling eye or prior camera orientation"): return
    if not require(camera.targeted_actor == old_target, "mode6 discarded the selected target"): return
    if not require(camera.source_has_eye_displacement() and (camera.mode_zero_parameter_flags & 8) != 0, "null mode6 binding cleared transient flags"): return
    camera.owner_motion_binding_present = true
    if not require(camera.source_update_position() and owner.global_basis.is_equal_approx(held_basis) and not camera.source_has_eye_displacement() and (camera.mode_zero_parameter_flags & 8) == 0, "bound mode6 cleanup changed orientation or retained flags"): return
    # Mode changes alone preserve progress and do not consume the target.
    camera.static_probe_frames = 4
    if not require(camera.source_change_state(6) and camera.static_probe_frames == 4, "equal-mode request reset camera progress"): return
    if not require(camera.source_change_state(-1) and camera.motion_mode == 0 and camera.targeted_actor == old_target and owner.global_position == traveling_eye and owner.global_basis.is_equal_approx(held_basis), "initial-mode restore moved or rebound the camera"): return
    camera.follow_controls_present = true
    camera.follow_linear_speed_limits = Vector2(12,200)
    camera.follow_linear_response_distance = 7.0
    camera.follow_linear_acceleration_divisor = 25.0
    camera.follow_angular_speed = 3.25
    camera.follow_angular_response_scale = 5.0
    camera.follow_angular_acceleration_divisor = 10.0
    camera.source_mode_zero_action_flags = 0
    var first_step: Variant = camera.source_follow_translation_step(Vector3(10,0,0), 12.0, 0.1)
    if not require(first_step is Vector3 and first_step.is_equal_approx(Vector3(0.048,0,0)), "ordinary follow did not apply the source acceleration bound"): return
    if not require(camera.get("_follow_previous_linear_step") == 0.0, "calculating a candidate advanced committed follow history"): return
    camera.source_mode_zero_action_flags = 0x80
    if not require(camera.source_follow_translation_step(Vector3(10,0,0), 12.0, 0.1).is_equal_approx(Vector3(1.2,0,0)), "0x80 did not bypass response while retaining the speed limit"): return
    camera.source_mode_zero_action_flags = 4
    if not require(camera.source_follow_translation_step(Vector3(10,0,0), 12.0, 0.1) == Vector3(10,0,0), "0x4 did not bypass the translation limiter"): return
    if not require(camera.source_follow_adjust_requested_motion(Vector3(-2,2,0), Vector3.ZERO, Vector3(1,3,0)).is_equal_approx(Vector3(0.45,3,0)), "00578852 radius correction changed vertical motion or skipped the horizontal adjustment"): return
    if not require(camera.source_follow_adjust_requested_motion(Vector3(-2,0,0), Vector3.ZERO, Vector3(3,0,0)).is_equal_approx(Vector3(0.45,0,0)), "00578852 did not reverse a correction that lengthened the requested motion"): return
    if not require(camera.source_follow_adjust_requested_motion(Vector3(-2,0,0), Vector3.ZERO, Vector3(2,0,0)) == Vector3(2,0,0), "00578852 invented an axis for a zero radial vector"): return
    camera.source_mode_zero_action_flags = 0
    var direction: Variant = camera.source_follow_view_direction(Vector3.BACK, Vector3.RIGHT, 0.1)
    if not require(direction is Vector3 and absf(Vector3.BACK.angle_to(direction) - 0.0325) < 0.00001, "ordinary follow angular acceleration differs"): return
    camera.source_mode_zero_action_flags = 0x400
    if not require(camera.source_follow_view_direction(Vector3.BACK, Vector3.RIGHT, 0.1) == Vector3.RIGHT and camera.get("_follow_previous_angular_step") == 0.0, "0x400 did not snap viewing direction and clear angular history"): return
    camera.source_mode_zero_action_flags = 0x800
    direction = camera.source_follow_view_direction(Vector3.BACK, Vector3.RIGHT, 0.1)
    if not require(absf(Vector3.BACK.angle_to(direction) - 0.325) < 0.00001, "0x800 did not apply the direct angular step cap"): return
    var velocities := PackedVector3Array([Vector3(2,0,0),Vector3(4,0,0),Vector3(6,0,0),Vector3(8,0,0),Vector3(1000,0,0),Vector3(1000,0,0),Vector3(1000,0,0),Vector3(1000,0,0)])
    var motion_inputs := {"motion_binding_present": true, "velocity_source": &"post_contact_velocity", "velocities": velocities, "weights": PackedFloat32Array([0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1])}
    if not require(camera.source_follow_weighted_motion(motion_inputs).is_equal_approx(Vector3(5,0,0)), "004444d0 did not stop after including the threshold-crossing sample"): return
    camera.static_obstruction_frames = 40
    var controls: Dictionary = camera.source_follow_motion_controls(motion_inputs, 0, 4.0)
    if not require(is_equal_approx(controls.get("speed", 0), 27.0), "signed retry count 40 did not accept both source speed boosts"): return
    motion_inputs["motion_binding_present"] = false
    motion_inputs.erase("velocities")
    if not require(camera.source_follow_weighted_motion(motion_inputs) == Vector3.ZERO, "null source motion binding fabricated target velocity"): return
    var interrupted := InterruptedFollow.new()
    interrupted.owner_motion_binding_present = true
    interrupted.source_mode_zero_action_flags = 0x80
    owner.add_child(interrupted)
    if not require(not interrupted.source_update_position() and interrupted.source_mode_zero_action_flags == 0x80, "failed follow leaked its transient recovery bypass into the next frame"): return
    interrupted.allow_frame = true
    if not require(interrupted.source_update_position() and interrupted.source_mode_zero_action_flags == 0x80, "follow retry changed the active action preset"): return
    level.free()
    print("CAMERA_NATIVE_SEMANTICS_OK")
    quit(0)
