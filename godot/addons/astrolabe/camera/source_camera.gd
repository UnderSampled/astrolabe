extends Camera3D
## Native target and framing addressed by the compiled camera AI API.
## Mode 3 uses the actor's eye position and aims at the selected target.
## Follow modes, viewport selection and procedure reset fields remain separate.

@export var motion_mode: int = 0
@export var initial_motion_mode: int = 0
@export var actor_offset: Vector3 = Vector3.ZERO
@export var look_offset: Vector3 = Vector3.ZERO
@export var initially_current: bool = false
@export_node_path("Node3D") var initial_target_path: NodePath
@export var mode_zero_target_offset: Vector3 = Vector3.ZERO
@export var mode_zero_actor_offset: Vector3 = Vector3.ZERO
@export var mode_zero_look_offset: Vector3 = Vector3.ZERO
@export var mode_zero_distance: Vector2 = Vector2.ZERO
@export var mode_zero_target_child_index: int = 0
@export var mode_zero_yaw_limits: Vector2 = Vector2.ZERO
@export var mode_zero_pitch_limits: Vector2 = Vector2.ZERO
@export var mode_zero_parameter_flags: int = 0
@export var source_owner_position_flags: int = 0 # 004406d0 clears Position+0xce; relevant setters OR bit 2
@export var controller_state: Resource

const SOURCE_MODE_ZERO_PIVOT_THRESHOLD_SQUARED := 9.0 # _DAT_005fcc2c from the decompiled build's .data image
const SOURCE_MODE_ZERO_RANGE_EPSILON := 0.001 # _DAT_0060b238 from the verified executable's .data image
const SOURCE_MODE_ZERO_RANGE_MAX_PADDING := 0.3 # DAT_0060b23c=3, _DAT_0060b240 from the verified executable's .data image
const SOURCE_MODE_ZERO_FALLBACK_MIN_MARGIN := 0.5 # _DAT_0060b228; DAT_0060b22c=1
const SOURCE_MODE_ZERO_FALLBACK_MAX_MARGIN := 0.5 # _DAT_0060b228; DAT_0060b230=1
const SOURCE_MODE_ZERO_FALLBACK_RANGE_EPSILON := 0.001 # _DAT_0060b224
const SOURCE_MODE_ZERO_FALLBACK_PITCHES := [0.3, 0.6, 0.0] # DAT_005fccb8..c0
const SOURCE_MODE_ZERO_FALLBACK_YAWS := [0.7853982, 5.497787, 1.5707964, 4.712389, 2.3561945, 3.926991, 3.1415927, 0.3] # 005fcc9c..b8, 8 float reads
const SOURCE_POSITION_DEFAULT_MIN := -3.402822e38 # 004406d0 writes 0xff7ffff8 at Position+0xa4
const SOURCE_POSITION_DEFAULT_MAX := 3.402822e38 # 004406d0 writes 0x7f7ffff8 at Position+0xa8
const SOURCE_WALK_DATA3D_CALLBACK := 0x00585d51 # DAT_00757320[0/8]
const SOURCE_POSITION_FLAG_HAS_TARGET_OFFSET := 2 # 00440fe0 writes owner Position+0xce and +0x50

var targeted_actor: Node3D
var _reported_mode_zero_gap := false
var _source_active_sector: Node
var _source_previous_history_position := Vector3.ZERO

func _ready() -> void:
    motion_mode = initial_motion_mode
    if not initial_target_path.is_empty():
        targeted_actor = get_node_or_null(initial_target_path) as Node3D
    _source_active_sector = _source_sector_for(get_parent())
    var owner := get_parent() as Node3D
    if owner != null:
        _source_previous_history_position = owner.global_position
    if initially_current:
        make_current()
    else:
        clear_current(false)

func source_change_targeted_perso(actor: Node3D) -> bool:
    if not is_instance_valid(actor):
        return false
    targeted_actor = actor
    return true

## 00547be0 restores the original preset for -1 and leaves equal modes alone.
## The observed opening transition 0 -> 3 has no extra reset branches.
## Other transitions require follow/obstruction state that is not mapped yet.
func source_change_state(state: int) -> bool:
    var selected := initial_motion_mode if state == -1 else state
    if selected == motion_mode:
        return true
    if motion_mode not in [0,3] or selected not in [0,3]:
        return false
    motion_mode = selected
    return true

## Hype 0051c8d0/0051d140 rotate the two offsets through the target's rotation.
## Mode 3 leaves ordinary eye motion to the camera actor (005200a0/0051e620).
## Zero look offset defaults to source [0,-0.05,0], or native [0,0,0.05].
func source_update_position() -> bool:
    if motion_mode == 0:
        return _source_update_mode_zero()
    if motion_mode != 3 or not is_instance_valid(targeted_actor):
        return false
    var eye := get_parent() as Node3D
    if eye == null or not eye.is_inside_tree() or not targeted_actor.is_inside_tree():
        return false
    var offset := look_offset if look_offset != Vector3.ZERO else Vector3(0,0,0.05)
    var point := targeted_actor.global_position + targeted_actor.global_basis.orthonormalized() * (actor_offset + offset)
    var direction := point - eye.global_position
    if not point.is_finite() or not direction.is_finite() or direction.length_squared() < 0.0000000001:
        return false
    if direction.normalized().cross(Vector3.UP).length_squared() < 0.0000000001:
        return false
    # Converted source -Y is the actor's +Z. The saved child half-turn makes
    # Camera3D's -Z look in that same direction. Update the meaningful actor pose.
    eye.look_at(point, Vector3.UP, true)
    return true

## Exposes the mapped 0051c8d0/0051d330/0051d140 geometry. The next source
## controller stage supplies its smoothed eye (+0x30) and collision result.
func source_mode_zero_target_geometry() -> Dictionary:
    if mode_zero_target_child_index != -1:
        push_error("Source camera target child selector has no native hierarchy binding")
        return {}
    if not is_instance_valid(targeted_actor) or not targeted_actor.is_inside_tree():
        push_error("Source mode-0 camera has no bound target Perso")
        return {}
    var basis := targeted_actor.global_basis
    var anchor := targeted_actor.global_position + basis * mode_zero_actor_offset
    var look := mode_zero_look_offset
    if look == Vector3.ZERO:
        look = Vector3(0, 0, 0.05)
    return {
        "target_position": targeted_actor.global_position,
        "anchor": anchor,
        "candidate": anchor + basis * mode_zero_target_offset,
        "focus": anchor + basis * look,
        "minimum_distance": mode_zero_distance.x,
        "maximum_distance": mode_zero_distance.y,
    }

## 00444620: weighted source target pivot. The eight points and weights are
## runtime Data3D inputs; do not substitute Godot mesh bounds for them.
func source_mode_zero_weighted_pivot(
    source_points: PackedVector3Array,
    source_weights: PackedFloat32Array,
    target_position: Vector3
) -> Dictionary:
    if not target_position.is_finite():
        push_error("Source mode-0 pivot inputs are invalid")
        return {}
    if source_points.size() != 8 or source_weights.size() != 8:
        push_error("Source mode-0 pivot requires the eight Data3D samples shifted by 00443d50/00443db0")
        return {}
    var points := source_points.duplicate()
    var weights := source_weights.duplicate()
    var total := 0.0
    for i in range(8):
        if not points[i].is_finite() or not is_finite(weights[i]):
            push_error("Source mode-0 Data3D contains a nonfinite point or weight")
            return {}
        total += weights[i]
    if total == 0.0:
        # 00443bf0 installs eight .03 weights and the actor translation at all
        # points when Data3D has no active weights.
        points.fill(target_position)
        weights.fill(0.03)
    var accumulated_weight := 0.0
    var weighted_point := Vector3.ZERO
    for i in range(8):
        accumulated_weight += weights[i]
        weighted_point += points[i] * weights[i]
        if accumulated_weight >= 0.4: # DAT_00745a84 from 0043f180
            break
    if accumulated_weight == 0.0:
        push_error("Source mode-0 Data3D pivot has zero accumulated weight")
        return {}
    var center := weighted_point / accumulated_weight
    return _source_smooth_mode_zero_pivot(target_position, center)

func _source_smooth_mode_zero_pivot(target_position: Vector3, center: Vector3) -> Dictionary:
    if controller_state == null or not controller_state.has_method("smooth_mode_zero_pivot"):
        push_error("Source camera controller state is not bound")
        return {}
    var selected: Vector3 = controller_state.call("smooth_mode_zero_pivot", target_position, center, SOURCE_MODE_ZERO_PIVOT_THRESHOLD_SQUARED)
    return {"anchor": selected, "raw_center": center}

## 00443f00 dispatches through the target's active mechanics identity card,
## copies physical-object output +0x310 into the Data3D history ring, and
## 00444620 consumes its weighted center. The current native actor adapter
## supports the source Walk callback shared by identity-card kinds 0 and 8.
func source_mode_zero_live_pivot() -> Dictionary:
    var geometry := source_mode_zero_target_geometry()
    if geometry.is_empty():
        return {}
    if not targeted_actor.has_method("source_camera_data3d_inputs"):
        push_error("Source mode-0 target does not expose an actor-owned Data3D producer")
        return {}
    var data3d: Dictionary = targeted_actor.call("source_camera_data3d_inputs")
    if not bool(data3d.get("supported", false)):
        push_error("Source mode-0 target identity-card callback is unsupported: " + str(data3d.get("reason", "no source callback")))
        return {}
    if int(data3d.get("callback_address", 0)) != SOURCE_WALK_DATA3D_CALLBACK:
        push_error("Source mode-0 target Data3D callback does not match the mapped Walk handler")
        return {}
    var target_position: Vector3 = geometry["target_position"]
    var points: PackedVector3Array = data3d.get("points", PackedVector3Array())
    var weights: PackedFloat32Array = data3d.get("weights", PackedFloat32Array())
    var pivot := source_mode_zero_weighted_pivot(points, weights, target_position)
    if pivot.is_empty():
        return {}
    var actor_basis := targeted_actor.global_basis.orthonormalized()
    var actor_offset := actor_basis * mode_zero_actor_offset
    var pivot_anchor: Vector3 = pivot["anchor"]
    var pivot_center: Vector3 = pivot["raw_center"]
    pivot["anchor"] = pivot_anchor + actor_offset
    pivot["raw_center"] = pivot_center + actor_offset
    pivot["target_position"] = target_position
    var target_offset := actor_basis * mode_zero_target_offset
    var look := mode_zero_look_offset if mode_zero_look_offset != Vector3.ZERO else Vector3(0, 0, 0.05)
    pivot["candidate"] = pivot["anchor"] + target_offset
    pivot["focus"] = pivot["anchor"] + actor_basis * look
    pivot["callback_address"] = SOURCE_WALK_DATA3D_CALLBACK
    pivot["sample_count"] = int(data3d.get("sample_count", 0))
    return pivot

## Port of the 0051d410/0051d6e0/0051d7b0 candidate calculation. Inputs are
## source-space vectors; the caller must provide the actual +0x54 orientation
## and owner +0xa4/+0xa8 vertical limits.
func source_compute_mode_zero_candidate(
    anchor_3c: Vector3,
    origin_18: Vector3,
    orientation_54: Vector3,
    source_vertical_limits: Vector2
) -> Dictionary:
    if not anchor_3c.is_finite() or not origin_18.is_finite() or not orientation_54.is_finite() or not source_vertical_limits.is_finite():
        push_error("Source mode-0 controller vectors are nonfinite")
        return {}
    if orientation_54.length_squared() < 0.999 or orientation_54.length_squared() > 1.001:
        push_error("Source mode-0 +0x54 orientation is not a unit vector")
        return {}
    if source_vertical_limits.x > source_vertical_limits.y:
        push_error("Source mode-0 vertical limits are inverted")
        return {}
    var desired := origin_18 - anchor_3c
    if desired.length_squared() < 0.0000000001:
        push_error("Source mode-0 owner and pivot coincide")
        return {}
    var angles := _source_camera_angles(orientation_54, desired)
    if angles.is_empty():
        return {}
    var yaw: Variant = _source_clamp_mode_zero_angle(float(angles["yaw"]), mode_zero_yaw_limits.x, mode_zero_yaw_limits.y)
    var pitch: Variant = _source_clamp_mode_zero_angle(float(angles["pitch"]), mode_zero_pitch_limits.x, mode_zero_pitch_limits.y)
    if yaw == null or pitch == null:
        push_error("Source camera angle flag-bit-2 smoothing is not mapped for this parameter set")
        return {}
    var direction := _source_rotate_mode_zero(orientation_54, float(yaw), float(pitch))
    if direction.length_squared() < 0.999 or direction.length_squared() > 1.001:
        push_error("Source mode-0 candidate direction lost unit length")
        return {}
    var distance := desired.length()
    if mode_zero_distance.x <= distance and mode_zero_distance.y < distance:
        distance = mode_zero_distance.y
    if distance <= 0.01:
        distance = 0.01
    var candidate := anchor_3c + direction * distance
    # Astrolabe's saved scene transforms already map OpenSpace source Z-up to
    # Godot Y-up; the source +0x8 candidate height is therefore Godot y.
    var unclamped_source_height := candidate.y
    if source_vertical_limits.y < unclamped_source_height:
        candidate.y = source_vertical_limits.y
    if unclamped_source_height < source_vertical_limits.x:
        candidate.y = source_vertical_limits.x
    if not candidate.is_finite():
        push_error("Source mode-0 candidate overflowed")
        return {}
    return {"position": candidate, "yaw": yaw, "pitch": pitch, "distance": distance}

## 0052ce20 compares the current-eye to smoothed-anchor range against the
## camera's +0x30 minimum and +0x34 maximum, with the source .data margins.
func source_mode_zero_distance_class(current_eye: Vector3, anchor_3c: Vector3) -> Variant:
    if not current_eye.is_finite() or not anchor_3c.is_finite() or not mode_zero_distance.is_finite():
        push_error("Source mode-0 range inputs are nonfinite")
        return null
    if mode_zero_distance.x < 0.0 or mode_zero_distance.y < mode_zero_distance.x:
        push_error("Source mode-0 camera range limits are invalid")
        return null
    var distance := current_eye.distance_to(anchor_3c)
    if distance < mode_zero_distance.x - SOURCE_MODE_ZERO_RANGE_EPSILON:
        return -1
    var maximum := mode_zero_distance.y + SOURCE_MODE_ZERO_RANGE_MAX_PADDING
    if distance <= maximum + SOURCE_MODE_ZERO_RANGE_EPSILON:
        return 0
    return 1

## 0052d1b0 returns 1 for the 0052a740 sector/superobject precheck, 2 for
## the 00463df0 camera ray, 3 for either 0052caa0 range rejection, and 4 for
## the first accepted candidate. Both collision paths reuse exported Godot
## collision shapes with their source sector/material/actor filters.
func source_mode_zero_candidate_classification(
    anchor_3c: Vector3,
    origin_18: Vector3,
    candidate: Vector3,
    context: Dictionary = {}
) -> Dictionary:
    if not anchor_3c.is_finite() or not origin_18.is_finite() or not candidate.is_finite():
        push_error("Source mode-0 fallback candidate contains a nonfinite point")
        return {}
    var collision_context := context
    if collision_context.is_empty():
        collision_context = _source_collision_context()
    if collision_context.is_empty():
        return {}
    var geometry_context := {"reachable": collision_context["geometry_reachable"]}
    # 00529ef0 rounds both endpoints to the nearest millimeter before its
    # source-sector geometry query. The subsequent 00463df0 ray keeps floats.
    var geometry_blocked: Variant = _source_camera_ray_blocked(
        _source_mode_zero_quantize(anchor_3c),
        _source_mode_zero_quantize(candidate),
        geometry_context,
        true,
        false
    )
    if geometry_blocked == null:
        return {}
    if bool(geometry_blocked):
        return {"classification": 1, "range_class": 0}
    var camera_ray_blocked: Variant = _source_camera_ray_blocked(origin_18, candidate, collision_context)
    if camera_ray_blocked == null:
        return {}
    if bool(camera_ray_blocked):
        return {"classification": 2, "range_class": 0}
    var range_class: Variant = _source_mode_zero_fallback_distance_class(candidate, anchor_3c)
    if range_class == null:
        return {}
    return {"classification": 4 if int(range_class) == 0 else 3, "range_class": int(range_class)}

## Exact 0051fb50 attempt order: three fixed zero-yaw pitches, then three
## pitch rows by eight yaw slots. The eighth inner yaw reads the overlapping
## 0.3 value at DAT_005fccb8, as the source's eight-float loop does.
func source_mode_zero_fallback_sweep(
    anchor_3c: Vector3,
    origin_18: Vector3,
    orientation_54: Vector3
) -> Dictionary:
    if not anchor_3c.is_finite() or not origin_18.is_finite() or not orientation_54.is_finite():
        push_error("Source mode-0 fallback sweep inputs are nonfinite")
        return {}
    if mode_zero_distance.x <= 0.0 or mode_zero_distance.y < mode_zero_distance.x:
        push_error("Source mode-0 fallback sweep requires valid camera distance parameters")
        return {}
    if controller_state == null or not controller_state.has_method("record_mode_zero_fallback"):
        push_error("Source camera controller state is not bound for the fallback sweep")
        return {}
    var context := _source_collision_context()
    if context.is_empty():
        return {}
    var attempts := 0
    var first_obstructed_position := Vector3.ZERO
    var has_obstructed_position := false
    for pitch_value in SOURCE_MODE_ZERO_FALLBACK_PITCHES:
        var candidate := _source_mode_zero_fallback_candidate(anchor_3c, orientation_54, 0.0, float(pitch_value))
        if candidate.is_empty():
            return {}
        attempts += 1
        var classification := source_mode_zero_candidate_classification(anchor_3c, origin_18, candidate["position"], context)
        if classification.is_empty():
            return {}
        var source_class := int(classification["classification"])
        if source_class == 2 and not has_obstructed_position:
            first_obstructed_position = candidate["position"]
            has_obstructed_position = true
            if not bool(controller_state.call("record_mode_zero_obstruction", first_obstructed_position)):
                push_error("Source mode-0 controller rejected the first obstructed sweep point")
                return {}
        if source_class == 4:
            if not bool(controller_state.call("record_mode_zero_fallback", 0.0, float(pitch_value), candidate["position"])):
                push_error("Source mode-0 controller rejected the accepted fallback candidate")
                return {}
            return {
                "success": true,
                "classification": 4,
                "position": candidate["position"],
                "yaw": 0.0,
                "pitch": float(pitch_value),
                "attempts": attempts,
                "has_obstruction": has_obstructed_position,
                "obstruction_position": first_obstructed_position,
            }
    for pitch_value in SOURCE_MODE_ZERO_FALLBACK_PITCHES:
        for yaw_value in SOURCE_MODE_ZERO_FALLBACK_YAWS:
            var yaw := float(yaw_value)
            var pitch := float(pitch_value)
            var candidate := _source_mode_zero_fallback_candidate(anchor_3c, orientation_54, yaw, pitch)
            if candidate.is_empty():
                return {}
            attempts += 1
            var classification := source_mode_zero_candidate_classification(anchor_3c, origin_18, candidate["position"], context)
            if classification.is_empty():
                return {}
            var source_class := int(classification["classification"])
            if source_class == 2 and not has_obstructed_position:
                first_obstructed_position = candidate["position"]
                has_obstructed_position = true
                if not bool(controller_state.call("record_mode_zero_obstruction", first_obstructed_position)):
                    push_error("Source mode-0 controller rejected the first obstructed sweep point")
                    return {}
            if source_class == 4:
                if not bool(controller_state.call("record_mode_zero_fallback", yaw, pitch, candidate["position"])):
                    push_error("Source mode-0 controller rejected the accepted fallback candidate")
                    return {}
                return {
                    "success": true,
                    "classification": 4,
                    "position": candidate["position"],
                    "yaw": yaw,
                    "pitch": pitch,
                    "attempts": attempts,
                    "has_obstruction": has_obstructed_position,
                    "obstruction_position": first_obstructed_position,
                }
    return {
        "success": false,
        "classification": 0,
        "attempts": attempts,
        "has_obstruction": has_obstructed_position,
        "obstruction_position": first_obstructed_position,
    }

func _source_mode_zero_fallback_candidate(
    anchor_3c: Vector3,
    orientation_54: Vector3,
    yaw: float,
    pitch: float
) -> Dictionary:
    var direction := _source_rotate_mode_zero(orientation_54, yaw, pitch)
    if not direction.is_finite() or direction.length_squared() < 0.0000000001:
        push_error("Source mode-0 fallback direction is invalid")
        return {}
    return {"position": anchor_3c + direction.normalized() * mode_zero_distance.x}

func _source_mode_zero_fallback_distance_class(candidate: Vector3, anchor_3c: Vector3) -> Variant:
    if not mode_zero_distance.is_finite() or mode_zero_distance.x < 0.0 or mode_zero_distance.y < mode_zero_distance.x:
        push_error("Source 0052caa0 camera range inputs are invalid")
        return null
    var minimum := mode_zero_distance.x - SOURCE_MODE_ZERO_FALLBACK_MIN_MARGIN
    var maximum := mode_zero_distance.y + SOURCE_MODE_ZERO_RANGE_MAX_PADDING + SOURCE_MODE_ZERO_FALLBACK_MAX_MARGIN
    var distance := candidate.distance_to(anchor_3c)
    if distance < minimum - SOURCE_MODE_ZERO_FALLBACK_RANGE_EPSILON:
        return -1
    if maximum < distance:
        return 1
    return 0

func _source_mode_zero_quantize(point: Vector3) -> Vector3:
    return Vector3(
        roundf(point.x * 1000.0) * 0.001,
        roundf(point.y * 1000.0) * 0.001,
        roundf(point.z * 1000.0) * 0.001
    )

func _source_camera_angles(current: Vector3, target: Vector3) -> Dictionary:
    var from := current.normalized()
    var to := target.normalized()
    var up := Vector3.UP # source Z-up is converted to Godot Y-up in saved scenes.
    var from_horizontal := from - up * from.dot(up)
    var to_horizontal := to - up * to.dot(up)
    if from_horizontal.length_squared() < 0.0000000001 or to_horizontal.length_squared() < 0.0000000001:
        push_error("Source mode-0 yaw is undefined for a vertical direction")
        return {}
    from_horizontal = from_horizontal.normalized()
    to_horizontal = to_horizontal.normalized()
    # 0057c62f measures heading in the horizontal plane, then rotates about
    # source +Z and measures elevation about the resulting local-right axis.
    var yaw := atan2(up.dot(from_horizontal.cross(to_horizontal)), from_horizontal.dot(to_horizontal))
    var turned := from.rotated(up, yaw)
    var pitch_axis := turned.cross(up)
    if pitch_axis.length_squared() < 0.0000000001:
        push_error("Source mode-0 pitch axis is undefined for a vertical direction")
        return {}
    pitch_axis = pitch_axis.normalized()
    var pitch := atan2(pitch_axis.dot(turned.cross(to)), turned.dot(to))
    return {"yaw": _source_wrap_angle(yaw), "pitch": _source_wrap_angle(pitch)}

func _source_clamp_mode_zero_angle(value: float, lower: float, upper: float) -> Variant:
    # 0051d490's manor mode-0 flag word is zero. In that branch a missing or
    # nonpositive upper bound selects the lower value; otherwise it clamps to
    # the nearest edge of the source angular interval.
    if (mode_zero_parameter_flags & 4) != 0:
        return null
    if upper >= PI:
        return value
    if upper <= 0.0:
        return lower
    var low_edge := _source_wrap_angle(lower - upper - 0.0001)
    var high_edge := _source_wrap_angle(lower + upper + 0.0001)
    var inside := false
    if high_edge <= low_edge:
        inside = not (high_edge < value and value < low_edge)
    else:
        inside = not (value < low_edge or high_edge < value)
    if inside:
        return value
    var delta := _source_wrap_angle(value - lower)
    return low_edge if delta > PI else high_edge

func _source_wrap_angle(value: float) -> float:
    if value < 0.0:
        return value + TAU
    if value >= TAU:
        return value - TAU
    return value

func _source_rotate_mode_zero(direction: Vector3, yaw: float, pitch: float) -> Vector3:
    var turned := _source_rotate_axis_angle(direction, Vector3.UP, yaw)
    var right := turned.cross(Vector3.UP)
    if right.length_squared() > 0.0000000001:
        right = right.normalized()
    turned = _source_rotate_axis_angle(turned, right, pitch)
    return turned

## 005282c0 reads the first orientation selector from the target DNM. 0044cf20
## initializes it to source [0,-1,0], and no other writer exists in this build.
## 005282c0 selects the target's source Y basis and 0051d410 negates it, giving
## source +Y. Imported scene bases have source +Y on Godot local -Z.
func source_mode_zero_orientation_54() -> Dictionary:
    if mode_zero_target_child_index != -1:
        push_error("Source camera target child selector has no native hierarchy binding")
        return {}
    if not is_instance_valid(targeted_actor) or not targeted_actor.is_inside_tree():
        push_error("Source mode-0 camera has no bound target Perso")
        return {}
    var basis := targeted_actor.global_basis.orthonormalized()
    var direction := basis * Vector3.FORWARD
    if not direction.is_finite() or direction.length_squared() < 0.999:
        push_error("Source mode-0 target orientation is invalid")
        return {}
    return {"orientation": direction.normalized()}

## 004406d0 initializes the camera owner's Position+0xa4/+0xa8 bounds to the
## finite sentinel pair. The manor camera startup path has no writer for them.
func source_mode_zero_vertical_limits() -> Vector2:
    var owner := get_parent() as Node3D
    if owner != null and owner.has_meta("source_mode_zero_vertical_limits"):
        var limits = owner.get_meta("source_mode_zero_vertical_limits")
        if limits is Vector2 and limits.is_finite() and limits.x <= limits.y:
            return limits
        push_error("Source mode-0 owner vertical limits metadata is invalid")
        return Vector2.ZERO
    return Vector2(SOURCE_POSITION_DEFAULT_MIN, SOURCE_POSITION_DEFAULT_MAX)

func _source_rotate_axis_angle(direction: Vector3, axis: Vector3, angle: float) -> Vector3:
    var cosine := cos(angle)
    var sine := sin(angle)
    return direction * cosine + axis * (axis.dot(direction) * (1.0 - cosine)) + axis.cross(direction) * sine

func record_mode_zero_candidate_collision(blocked: bool) -> int:
    if controller_state == null or not controller_state.has_method("record_candidate_collision"):
        push_error("Source camera controller state is not bound")
        return 0
    return int(controller_state.call("record_candidate_collision", blocked))

func evaluate_mode_zero_teleport_gate(
    candidate_result: bool,
    fallback_active: bool,
    fallback_succeeded: bool,
    has_last_good_position: bool,
    distance_class: int,
    camera_eye: Vector3,
    previous_eye: Vector3,
    maximum_distance: float,
    obstructed: bool,
    owner_teleport_blocked: bool
) -> bool:
    if controller_state == null or not controller_state.has_method("evaluate_mode_zero_teleport_gate"):
        push_error("Source camera controller state is not bound")
        return false
    return bool(controller_state.call(
        "evaluate_mode_zero_teleport_gate",
        candidate_result,
        fallback_active,
        fallback_succeeded,
        has_last_good_position,
        distance_class,
        camera_eye,
        previous_eye,
        maximum_distance,
        obstructed,
        owner_teleport_blocked
    ))

## Supplies the 0051f670 segments from source controller positions: smoothed
## anchor +0x3c, current camera eye +0x18, and candidate eye +0x30. The third
## visibility segment is queried only after either candidate ray is blocked.
func source_mode_zero_collision_inputs(
    controller_anchor_3c: Vector3,
    controller_origin_18: Vector3,
    controller_eye_30: Vector3
) -> Dictionary:
    if not controller_anchor_3c.is_finite() or not controller_origin_18.is_finite() or not controller_eye_30.is_finite():
        push_error("Source mode-0 collision endpoints are nonfinite")
        return {}
    var geometry := source_mode_zero_target_geometry()
    if geometry.is_empty():
        return {}
    var context := _source_collision_context()
    if context.is_empty():
        return {}
    var target_position := geometry["target_position"] as Vector3
    var anchor_to_eye = _source_camera_ray_blocked(controller_anchor_3c, controller_eye_30, context)
    var origin_to_eye = _source_camera_ray_blocked(controller_origin_18, controller_eye_30, context)
    if anchor_to_eye == null or origin_to_eye == null:
        return {}
    var target_head_checked := bool(anchor_to_eye) or bool(origin_to_eye)
    var origin_to_target_head: Variant = false
    if target_head_checked:
        origin_to_target_head = _source_camera_ray_blocked(
            controller_origin_18,
            target_position + Vector3.UP * 1.8,
            context
        )
        if origin_to_target_head == null:
            return {}
    var streak := record_mode_zero_candidate_collision(bool(origin_to_eye))
    return {
        "anchor_to_eye_blocked": bool(anchor_to_eye),
        "origin_to_eye_blocked": bool(origin_to_eye),
        "candidate_result": not bool(anchor_to_eye) and not bool(origin_to_eye),
        "target_head_checked": target_head_checked,
        "origin_to_target_head_blocked": bool(origin_to_target_head),
        "candidate_collision_streak": streak,
    }

## 00440fe0 sets owner Position bit 2 and writes the associated +0x50 vector.
## The intro starts with flags clear from 004406d0; expose the writer so later
## compiled camera procedures can carry the exact owner state into this gate.
func source_owner_position_flag_word() -> int:
    var owner := get_parent() as Node
    if owner != null and owner.has_meta("source_position_flag_word"):
        return int(owner.get_meta("source_position_flag_word"))
    return source_owner_position_flags

func source_owner_set_target_offset(offset: Vector3) -> bool:
    if not offset.is_finite():
        return false
    var owner := get_parent() as Node
    if owner == null:
        return false
    source_owner_position_flags = source_owner_position_flag_word() | SOURCE_POSITION_FLAG_HAS_TARGET_OFFSET
    owner.set_meta("source_position_flag_word", source_owner_position_flags)
    owner.set_meta("source_position_target_offset", offset)
    return true

func source_owner_clear_position_flags(mask: int) -> bool:
    var owner := get_parent() as Node
    if owner == null:
        return false
    source_owner_position_flags = source_owner_position_flag_word() & ~mask
    owner.set_meta("source_position_flag_word", source_owner_position_flags)
    return true

## 0051c7b0 tests its third position argument against the active source sector.
## It publishes that point when it belongs to another sector, otherwise its
## second position argument. The caller must resolve the third-argument sector
## from source boundary data.
func source_publish_mode_zero_translation(
    third_argument_position: Vector3,
    second_argument_position: Vector3,
    third_argument_sector_id: String = ""
) -> bool:
    var owner := get_parent() as Node3D
    var context := _source_collision_context()
    if owner == null or not owner.is_inside_tree() or context.is_empty():
        return false
    if not third_argument_position.is_finite() or not second_argument_position.is_finite():
        return false
    var resolved_sector_id := third_argument_sector_id
    if resolved_sector_id.is_empty():
        var sector_result := source_resolve_mode_zero_sector(third_argument_position)
        if not bool(sector_result.get("complete", false)):
            push_error("Source 0051c7b0 publication needs the full 004e0020 sector query; active child physical-visual geometry is not available as an equivalent source ray input")
            return false
        resolved_sector_id = str(sector_result.get("sector_id", ""))
    if resolved_sector_id.is_empty():
        return false
    var active_sector := context["sector"] as Node
    var active_sector_id := str(active_sector.get_meta("source_sector_id", ""))
    var next_sector := _find_source_sector(_source_level_root(), resolved_sector_id)
    if next_sector == null:
        push_error("Source camera pose references an unbound destination sector")
        return false
    if resolved_sector_id != active_sector_id:
        owner.global_position = third_argument_position
        _source_active_sector = next_sector
    else:
        owner.global_position = second_argument_position
    _source_previous_history_position = owner.global_position
    return true

## Exact border-GEO branch of 004e0020. The routine also tests active-sector
## children of source kinds 1, 11, 13 and 21; if any are present this reports
## the nearest border hit for inspection but marks the full result incomplete.
func source_resolve_mode_zero_sector(position: Vector3) -> Dictionary:
    if not position.is_finite():
        return {}
    var root := _source_level_root()
    if not is_instance_valid(root):
        push_error("Source camera has no level root for 004e0020 sector lookup")
        return {}
    var nearest_fraction := 2.0
    var nearest_sector_id := ""
    for candidate in root.find_children("SectorBoundary", "MeshInstance3D", true, false):
        var boundary := candidate as Node3D
        var sector := _source_sector_for(boundary)
        if sector == null or not sector.has_meta("source_sector") or not sector.has_meta("source_sector_id"):
            continue
        var source_data = JSON.parse_string(str(sector.get_meta("source_sector")))
        if not source_data is Dictionary:
            continue
        if int(source_data.get("virtual_sector", 0)) != 0 or (int(source_data.get("activation", 0)) & 4) == 0:
            continue
        if not boundary.has_method("intersect_segment"):
            continue
        var hit: Dictionary = boundary.call("intersect_segment", position, position + Vector3.DOWN)
        if hit.is_empty():
            continue
        var fraction := float(hit.get("fraction", 2.0))
        if fraction < nearest_fraction:
            nearest_fraction = fraction
            nearest_sector_id = str(sector.get_meta("source_sector_id"))
    if nearest_sector_id.is_empty():
        return {}
    var child_geometry_missing := _source_has_active_sector_physical_visuals(root)
    return {
        "sector_id": nearest_sector_id,
        "fraction": nearest_fraction,
        "complete": not child_geometry_missing,
    }

func _source_has_active_sector_physical_visuals(root: Node) -> bool:
    for candidate in root.find_children("*", "Node3D", true, false):
        var source_kind := int(candidate.get_meta("source_node_kind", -1))
        if source_kind not in [1, 11, 13, 21]:
            continue
        var sector := _source_sector_for(candidate)
        if sector == null or not sector.has_meta("source_sector"):
            continue
        var source_data = JSON.parse_string(str(sector.get_meta("source_sector")))
        if source_data is Dictionary and int(source_data.get("virtual_sector", 0)) == 0 and (int(source_data.get("activation", 0)) & 4) != 0:
            return true
    return false

func _source_collision_context() -> Dictionary:
    var owner := get_parent() as Node3D
    if owner == null or not owner.is_inside_tree() or not _source_update_sector_history(owner.global_position):
        return {}
    var sector: Node = _source_active_sector
    if not is_instance_valid(sector):
        sector = _source_sector_for(get_parent())
    while is_instance_valid(sector):
        if sector.has_meta("source_sector") and sector.has_meta("source_sector_id"):
            var camera_sector := sector
            var camera_reachable := _source_collision_sectors([camera_sector])
            if camera_reachable.is_empty():
                return {}
            var geometry_sectors: Array[Node] = [camera_sector]
            var target_sector := _source_sector_for(targeted_actor) if is_instance_valid(targeted_actor) else null
            if target_sector != null and target_sector != camera_sector:
                geometry_sectors.append(target_sector)
            var geometry_reachable := _source_collision_sectors(geometry_sectors)
            if geometry_reachable.is_empty():
                return {}
            return {
                "sector": camera_sector,
                "reachable": camera_reachable,
                "geometry_reachable": geometry_reachable,
            }
        sector = sector.get_parent()
    push_error("Source camera has no owning source sector metadata")
    return {}

## 00529ef0 checks the camera/controller sector and target sector directly,
## then collision-list neighbors whose source activation word has bit 2.
## 00463df0 uses the active camera sector's collision list.
func _source_collision_sectors(direct_sectors: Array[Node]) -> Dictionary:
    var root := _source_level_root()
    var reachable: Dictionary = {}
    for direct_sector in direct_sectors:
        if not is_instance_valid(direct_sector) or not direct_sector.has_meta("source_sector_id"):
            push_error("Source mode-0 collision query has an unbound direct sector")
            return {}
        var sector_id := str(direct_sector.get_meta("source_sector_id"))
        var source_data = JSON.parse_string(str(direct_sector.get_meta("source_sector", "")))
        if not source_data is Dictionary or not source_data.has("collision"):
            push_error("Source mode-0 sector metadata has no collision-neighbor list")
            return {}
        reachable[sector_id] = true
        for neighbor in source_data["collision"]:
            var neighbor_id := str(neighbor)
            var neighbor_sector := _find_source_sector(root, neighbor_id)
            if neighbor_sector == null:
                push_error("Source collision list references an unbound sector")
                return {}
            var neighbor_data = JSON.parse_string(str(neighbor_sector.get_meta("source_sector", "")))
            if not neighbor_data is Dictionary or not neighbor_data.has("activation"):
                push_error("Source collision neighbor has no activation metadata")
                return {}
            if (int(neighbor_data["activation"]) & 2) != 0:
                reachable[neighbor_id] = true
    return reachable

## 00448280/004e03f0 retain the previous camera position and walk through the
## first one-sided boundary face in each active sector. This updates the same
## sector context used by 00463df0 collision queries.
func _source_update_sector_history(current_position: Vector3) -> bool:
    if not current_position.is_finite():
        push_error("Source camera sector history received a nonfinite position")
        return false
    var sector: Node = _source_active_sector
    if not is_instance_valid(sector):
        sector = _source_sector_for(get_parent())
    if not is_instance_valid(sector):
        push_error("Source camera has no initial sector for boundary history")
        return false
    var cursor := _source_previous_history_position
    for _step in range(128):
        var boundary := sector.get_node_or_null("SectorBoundary")
        if boundary == null:
            break
        if not boundary.has_method("first_source_sector_crossing"):
            push_error("Source sector boundary does not expose ordered source crossings")
            return false
        var crossing: Dictionary = boundary.call("first_source_sector_crossing", cursor, current_position)
        if crossing.is_empty():
            break
        var next_id := str(crossing.get("sector", ""))
        var active_id := str(sector.get_meta("source_sector_id", ""))
        if next_id.is_empty() or next_id == active_id:
            break
        var next_sector := _find_source_sector(_source_level_root(), next_id)
        if next_sector == null:
            push_error("Source sector boundary references an unbound neighbor")
            return false
        var next_data = JSON.parse_string(str(next_sector.get_meta("source_sector", "")))
        if not next_data is Dictionary or not next_data.has("activation"):
            push_error("Source destination sector has no activation metadata")
            return false
        if (int(next_data["activation"]) & 4) == 0:
            break
        sector = next_sector
        cursor = crossing["position"]
    _source_active_sector = sector
    _source_previous_history_position = current_position
    return true

func _source_level_root() -> Node:
    var current: Node = self
    while is_instance_valid(current):
        if current.has_meta("source_level_scene"):
            return current
        current = current.get_parent()
    return null

func _find_source_sector(root: Node, sector_id: String) -> Node:
    if not is_instance_valid(root):
        return null
    if root.has_meta("source_sector_id") and str(root.get_meta("source_sector_id")) == sector_id:
        return root
    for child in root.get_children():
        var found := _find_source_sector(child, sector_id)
        if found != null:
            return found
    return null

func _source_camera_ray_blocked(
    start: Vector3,
    finish: Vector3,
    context: Dictionary,
    static_geometry_only: bool = false,
    apply_material_filter: bool = true
) -> Variant:
    if start.is_equal_approx(finish):
        return false
    var owner_node := get_parent() as CollisionObject3D
    var excluded: Array[RID] = []
    if owner_node != null:
        excluded.append(owner_node.get_rid())
    var target_body := targeted_actor as CollisionObject3D
    if target_body != null and not excluded.has(target_body.get_rid()):
        excluded.append(target_body.get_rid())
    var reachable: Dictionary = context["reachable"]
    var space := get_world_3d().direct_space_state
    for _attempt in range(512):
        var query := PhysicsRayQueryParameters3D.create(start, finish, 3, excluded)
        query.collide_with_bodies = true
        query.collide_with_areas = false
        var hit := space.intersect_ray(query)
        if hit.is_empty():
            return false
        var collider := hit["collider"] as CollisionObject3D
        if collider == null:
            push_error("Source mode-0 ray hit a non-collision object")
            return null
        var shape_owner := collider.shape_find_owner(int(hit["shape"]))
        var shape := collider.shape_owner_get_owner(shape_owner) as Node
        var identifier := int(shape.get_meta("source_collision_identifier", 0)) if shape != null else 0
        var sector := _source_sector_for(collider)
        var sector_id := str(sector.get_meta("source_sector_id", "")) if sector != null else ""
        var reachable_hit := reachable.has(sector_id)
        var reachable_static := not collider is StaticBody3D or bool(collider.get_meta("source_collision_reachable", false))
        var eligible_geometry := not static_geometry_only or collider is StaticBody3D
        if not reachable_hit or not reachable_static or not eligible_geometry:
            var ignored_rid: RID = hit["rid"]
            if excluded.has(ignored_rid):
                push_error("Source mode-0 ray cannot advance past an out-of-context collision shape")
                return null
            excluded.append(ignored_rid)
            continue
        if apply_material_filter and (identifier & 0x8000) != 0:
            # 00463df0 returns clear immediately for the first hit carrying
            # material identifier bit 0x8000.
            return false
        return true
    push_error("Source mode-0 ray exceeded its ignored-collider limit")
    return null

func _source_sector_for(node: Node) -> Node:
    var current := node
    while is_instance_valid(current):
        if current.has_meta("source_sector_id"):
            return current
        current = current.get_parent()
    return null

func _source_update_mode_zero() -> bool:
    var live_pivot := source_mode_zero_live_pivot()
    if live_pivot.is_empty():
        return false
    var owner := get_parent() as Node3D
    if owner == null or not owner.is_inside_tree():
        push_error("Source mode-0 camera owner is not in the scene tree")
        return false
    if controller_state == null or not controller_state.has_method("begin_mode_zero_update"):
        push_error("Source camera controller state is not bound for a mode-0 update")
        return false
    if not controller_state.has_method("mode_zero_collision_selector"):
        push_error("Source camera controller state does not expose DAT_00721b0c")
        return false
    var collision_selector := int(controller_state.call("mode_zero_collision_selector"))
    if collision_selector != 0:
        push_error("Source DAT_00721b0c selects 0051f290; its 0052bb50/0051f040/0051f250 state machine is not implemented")
        return false
    var owner_flags := source_owner_position_flag_word()
    if (owner_flags & SOURCE_POSITION_FLAG_HAS_TARGET_OFFSET) != 0:
        push_error("Source camera owner bit 2 selects the Position+0x50 controller vector path in 0051db70; that input is not yet mapped")
        return false
    controller_state.call("begin_mode_zero_update")
    var orientation_data := source_mode_zero_orientation_54()
    if orientation_data.is_empty():
        return false
    var current_eye := owner.global_position
    var anchor_3c: Vector3 = live_pivot["anchor"]
    var candidate := source_compute_mode_zero_candidate(
        anchor_3c,
        current_eye,
        orientation_data["orientation"],
        source_mode_zero_vertical_limits()
    )
    if candidate.is_empty():
        return false
    var collision := source_mode_zero_collision_inputs(anchor_3c, current_eye, candidate["position"])
    if collision.is_empty():
        return false
    var candidate_result := bool(collision["candidate_result"])
    if not candidate_result:
        # The initial source selector chooses 0051f670. It does not set
        # DAT_007218fc, so a blocked ray alone does not dispatch 0051fb50.
        # The exact blocked-result state changes from 0051f670 are not present
        # in the available source artifact; do not fabricate a fallback pose.
        push_warning("Source 0051f670 found a blocked camera ray; its blocked-result controller transition is not implemented, and DAT_007218fc does not authorize 0051fb50 here")
        return false
    if not bool(controller_state.call("record_mode_zero_good_position", candidate["position"])):
        push_error("Source camera controller state rejected the current clear candidate")
        return false
    var distance_class: Variant = source_mode_zero_distance_class(current_eye, anchor_3c)
    if distance_class == null:
        return false
    var teleport_requested := evaluate_mode_zero_teleport_gate(
        candidate_result,
        false,
        false,
        bool(controller_state.call("mode_zero_update_has_position")),
        int(distance_class),
        current_eye,
        anchor_3c,
        mode_zero_distance.y,
        bool(collision["origin_to_eye_blocked"]),
        (owner_flags & SOURCE_POSITION_FLAG_HAS_TARGET_OFFSET) != 0
    )
    if teleport_requested:
        var obstruction_position: Vector3 = controller_state.call("mode_zero_last_obstructed_position")
        if not bool(controller_state.call("mode_zero_has_obstructed_position")):
            push_error("Source 0051c7b0 gate fired without a source-observed DAT_007219c4 obstruction point")
            return false
        var last_good_position: Vector3 = controller_state.call("mode_zero_last_good_position")
        var sector_result := source_resolve_mode_zero_sector(obstruction_position)
        if not bool(sector_result.get("complete", false)):
            push_error("Source 0051c7b0 pose publication needs a complete 004e0020 sector result")
            return false
        if not source_publish_mode_zero_translation(
            obstruction_position,
            last_good_position,
            str(sector_result.get("sector_id", ""))
        ):
            push_error("Source 0051c7b0 rejected the conditional mode-0 translation")
            return false
    # 005200a0 computes the candidate, runs obstruction/range hysteresis, and
    # writes the owner translation only when DAT_007219d1 reaches 0051c7b0.
    return true
