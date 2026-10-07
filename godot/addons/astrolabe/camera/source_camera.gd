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
var source_mode_zero_action_flags: int = 0 # parameter +0x92, read by 00440e30
var source_controller_position_24 := Vector3.ZERO
var source_controller_position_30 := Vector3.ZERO
var source_controller_candidate_result := false

const SOURCE_MODE_ZERO_PIVOT_THRESHOLD_SQUARED := 9.0 # _DAT_005fcc2c from the decompiled build's .data image
const SOURCE_MODE_ZERO_RANGE_EPSILON := 0.001 # _DAT_0060b238 from the verified executable's .data image
const SOURCE_MODE_ZERO_RANGE_MAX_PADDING := 0.3 # DAT_0060b23c=3, _DAT_0060b240 from the verified executable's .data image
const SOURCE_MODE_ZERO_FALLBACK_MIN_MARGIN := 0.5 # _DAT_0060b228; DAT_0060b22c=1
const SOURCE_MODE_ZERO_FALLBACK_MAX_MARGIN := 0.5 # _DAT_0060b228; DAT_0060b230=1
const SOURCE_MODE_ZERO_FALLBACK_RANGE_EPSILON := 0.001 # _DAT_0060b224
const SOURCE_MODE_ZERO_CAMERA_BLOCK_STREAK_LIMIT := 50 # DAT_00721990 initialized by 005200a0
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
    var anchor_result: Variant = _source_camera_ray_result(controller_anchor_3c, controller_eye_30, context)
    var origin_result: Variant = _source_camera_ray_result(controller_origin_18, controller_eye_30, context)
    if anchor_result == null or origin_result == null:
        return {}
    var anchor_to_eye := bool(anchor_result["blocked"])
    var origin_to_eye := bool(origin_result["blocked"])
    var target_head_checked := anchor_to_eye or origin_to_eye
    var target_head_result: Variant = {"blocked": false}
    if target_head_checked:
        target_head_result = _source_camera_ray_result(
            controller_origin_18,
            target_position + Vector3.UP * 1.8,
            context
        )
        if target_head_result == null:
            return {}
    var streak := record_mode_zero_candidate_collision(bool(origin_to_eye))
    return {
        "anchor_to_eye_blocked": bool(anchor_to_eye),
        "origin_to_eye_blocked": bool(origin_to_eye),
        "anchor_result": anchor_result,
        "origin_result": origin_result,
        "candidate_result": not bool(anchor_to_eye) and not bool(origin_to_eye),
        "target_head_checked": target_head_checked,
        "target_head_result": target_head_result,
        "origin_to_target_head_blocked": bool(target_head_result["blocked"]),
        "candidate_collision_streak": streak,
    }

## 0052a740 static-sector query, reusing exported reachability and collision
## geometry metadata. Its endpoints are quantized by 00529ef0.
func _source_mode_zero_static_geometry_blocked(
    anchor_3c: Vector3,
    point: Vector3,
    context: Dictionary
) -> Variant:
    var geometry_context := {"reachable": context["geometry_reachable"]}
    return _source_camera_ray_blocked(
        _source_mode_zero_quantize(anchor_3c),
        _source_mode_zero_quantize(point),
        geometry_context,
        true,
        false
    )

## 0052d0e0 accepts a point only when static geometry, the filtered camera
## ray, and the source camera range all accept it.
func _source_mode_zero_point_is_safe(
    anchor_3c: Vector3,
    origin_18: Vector3,
    point: Vector3,
    context: Dictionary
) -> Variant:
    var classification := source_mode_zero_candidate_classification(anchor_3c, origin_18, point, context)
    if classification.is_empty():
        return null
    return int(classification["classification"]) == 4

func _source_mode_zero_recovery_from_unsafe_point(
    anchor_3c: Vector3,
    origin_18: Vector3,
    orientation_54: Vector3,
    context: Dictionary
) -> Dictionary:
    if controller_state == null or not controller_state.has_method("record_mode_zero_recovery_position"):
        push_error("Source camera controller state cannot retain DAT_007219b8")
        return {}
    var safe: Variant = _source_mode_zero_point_is_safe(anchor_3c, origin_18, origin_18, context)
    if safe == null:
        return {}
    if bool(safe):
        controller_state.call("record_mode_zero_recovery_position", origin_18)
        controller_state.call("set_mode_zero_fallback_requested", false)
        return {"fallback_requested": false, "position": origin_18}
    var toward_origin := origin_18 - anchor_3c
    if toward_origin.length_squared() < 0.0000000001:
        push_error("Source 0051f040 cannot normalize the camera-to-anchor vector")
        return {}
    var minimum_candidate := anchor_3c + toward_origin.normalized() * mode_zero_distance.x
    safe = _source_mode_zero_point_is_safe(anchor_3c, origin_18, minimum_candidate, context)
    if safe == null:
        return {}
    if bool(safe):
        controller_state.call("record_mode_zero_recovery_position", minimum_candidate)
        controller_state.call("set_mode_zero_fallback_requested", false)
        return {"fallback_requested": false, "position": minimum_candidate}
    if not controller_state.has_method("mode_zero_fallback_angles"):
        push_error("Source 0051f250 cannot read its process-global yaw/pitch")
        return {}
    var last_angles: Vector2 = controller_state.call("mode_zero_fallback_angles")
    var candidate := _source_mode_zero_fallback_candidate(anchor_3c, orientation_54, last_angles.x, last_angles.y)
    if candidate.is_empty():
        return {}
    safe = _source_mode_zero_point_is_safe(anchor_3c, origin_18, candidate["position"], context)
    if safe == null:
        return {}
    controller_state.call("record_mode_zero_recovery_position", candidate["position"])
    controller_state.call("set_mode_zero_fallback_requested", not bool(safe))
    return {"fallback_requested": not bool(safe), "position": candidate["position"]}

## Source selector 0: exact 0051f670 anchor, origin, and target-head rays.
## 0051f450 moves to the collision point plus source normal * 0.35.
func _source_mode_zero_controller_670(
    anchor_3c: Vector3,
    origin_18: Vector3,
    candidate_24: Vector3,
    candidate_30: Vector3,
    context: Dictionary
) -> Dictionary:
    var first_initial: Variant = _source_camera_ray_result(anchor_3c, candidate_30, context)
    if first_initial == null:
        return {}
    var first: Dictionary = first_initial
    var first_initially_blocked := bool(first["blocked"])
    if first_initially_blocked:
        controller_state.call("record_mode_zero_candidate_obstruction", true)
        var adjusted: Variant = _source_mode_zero_collision_adjusted_position(first, 0.35)
        if adjusted == null:
            return {}
        candidate_24 = adjusted
        candidate_30 = adjusted
        first = _source_camera_ray_result(anchor_3c, candidate_30, context)
        if first == null:
            return {}
    else:
        controller_state.call("record_mode_zero_candidate_obstruction", false)
        if not bool(controller_state.call("record_mode_zero_controller_position", candidate_24)):
            push_error("Source camera controller rejected the 0051f670 clear-ray c4 position")
            return {}

    var second: Variant = _source_camera_ray_result(origin_18, candidate_30, context)
    if second == null:
        return {}
    var second_blocked := bool(second["blocked"])
    controller_state.call("record_candidate_collision", second_blocked)
    if second_blocked:
        controller_state.call("mark_mode_zero_update_obstructed")
    if not bool(first["blocked"]) and not second_blocked:
        _source_mode_zero_apply_input_flags(first_initially_blocked or bool(first["blocked"]))
        return _source_mode_zero_controller_result(true, candidate_24, candidate_30, first_initially_blocked, second_blocked, false)

    if not is_instance_valid(targeted_actor) or not targeted_actor.is_inside_tree():
        push_error("Source 0051f670 target-head segment has no selected Perso")
        return {}
    var target_head := targeted_actor.global_position + Vector3.UP * 1.8
    var third: Variant = _source_camera_ray_result(origin_18, target_head, context)
    if third == null:
        return {}
    if not bool(third["blocked"]):
        if bool(first["blocked"]):
            var hit_position: Variant = _source_mode_zero_collision_adjusted_position(first, 0.35)
            if hit_position == null:
                return {}
            candidate_24 = hit_position
            candidate_30 = hit_position
            _source_mode_zero_apply_input_flags(true)
        else:
            # 0051f670 copies +0x3c into both +0x24 and +0x30.
            candidate_24 = anchor_3c
            candidate_30 = anchor_3c
        return _source_mode_zero_controller_result(true, candidate_24, candidate_30, first_initially_blocked, second_blocked, true)

    if bool(first["blocked"]):
        var adjusted_to_hit: Variant = _source_mode_zero_collision_adjusted_position(first, 0.35)
        if adjusted_to_hit == null:
            return {}
        candidate_24 = adjusted_to_hit
        candidate_30 = adjusted_to_hit
        _source_mode_zero_apply_input_flags(true)
        return _source_mode_zero_controller_result(true, candidate_24, candidate_30, first_initially_blocked, second_blocked, true)

    if not second_blocked:
        # Kept for the source's explicit return-0 branch after the preceding
        # early clear/clear return; the source ray state makes it unreachable.
        return _source_mode_zero_controller_result(false, candidate_24, candidate_30, first_initially_blocked, false, true)

    candidate_24 = candidate_30
    var obstruction_count := int(controller_state.call("mode_zero_collision_streak"))
    var collision_guard: Variant = null
    var owner := get_parent() as Node
    if obstruction_count <= SOURCE_MODE_ZERO_CAMERA_BLOCK_STREAK_LIMIT:
        if owner != null and owner.has_meta("source_camera_collision_guard"):
            collision_guard = bool(owner.get_meta("source_camera_collision_guard"))
        else:
            push_error("Source 0051f670 needs the live +0x254 flag written by 005796ea on this blocked subpath")
            return {}
    if obstruction_count > SOURCE_MODE_ZERO_CAMERA_BLOCK_STREAK_LIMIT or bool(collision_guard):
        # The source special case adds the raw parameter +0x24 vector to the
        # target's world translation; unlike 0051c8d0, it does not rotate it.
        candidate_24 = targeted_actor.global_position + mode_zero_actor_offset
        candidate_30 = candidate_24
        source_mode_zero_action_flags |= 0x414
    return _source_mode_zero_controller_result(true, candidate_24, candidate_30, first_initially_blocked, second_blocked, true)

## Source selector 1: 0052bb50's persistent static-geometry timer, followed
## by 0051f290's 00463df0 ray and 0051f040/0051f250 recovery calculation.
func _source_mode_zero_controller_290(
    anchor_3c: Vector3,
    origin_18: Vector3,
    candidate_24: Vector3,
    candidate_30: Vector3,
    orientation_54: Vector3,
    context: Dictionary
) -> Dictionary:
    var obstruction_wait: Variant = _source_mode_zero_should_wait_for_static_obstruction(
        anchor_3c, candidate_24, candidate_30, context
    )
    if obstruction_wait == null:
        return {}
    if bool(obstruction_wait):
        controller_state.call("record_mode_zero_candidate_obstruction", true)
        controller_state.call("mark_mode_zero_update_obstructed")
        var recovery := _source_mode_zero_recovery_from_unsafe_point(anchor_3c, origin_18, orientation_54, context)
        if recovery.is_empty():
            return {}
        controller_state.call("set_mode_zero_fallback_requested", bool(recovery["fallback_requested"]))
        return _source_mode_zero_controller_result(false, candidate_24, candidate_30, true, true, false)

    controller_state.call("record_mode_zero_candidate_obstruction", false)
    var clear_candidate := candidate_24 if motion_mode in [0, 7] else candidate_30
    if not bool(controller_state.call("record_mode_zero_controller_position", clear_candidate)):
        push_error("Source camera controller rejected the 0051f290 c4 position")
        return {}
    var camera_ray: Variant = _source_camera_ray_result(origin_18, candidate_30, context)
    if camera_ray == null:
        return {}
    if bool(camera_ray["blocked"]):
        controller_state.call("record_candidate_collision", true)
        controller_state.call("mark_mode_zero_update_obstructed")
        var recovery := _source_mode_zero_recovery_from_unsafe_point(anchor_3c, origin_18, orientation_54, context)
        if recovery.is_empty():
            return {}
        controller_state.call("set_mode_zero_fallback_requested", bool(recovery["fallback_requested"]))
        return _source_mode_zero_controller_result(false, candidate_24, candidate_30, false, true, true)
    controller_state.call("record_candidate_collision", false)
    return _source_mode_zero_controller_result(true, candidate_24, candidate_30, false, false, false)

func _source_mode_zero_should_wait_for_static_obstruction(
    anchor_3c: Vector3,
    candidate_24: Vector3,
    candidate_30: Vector3,
    context: Dictionary
) -> Variant:
    var owner := get_parent() as Node
    if owner == null:
        push_error("Source 0052bb50 has no camera-owner counter context")
        return null
    if (source_owner_position_flag_word() & SOURCE_POSITION_FLAG_HAS_TARGET_OFFSET) != 0:
        return false
    if motion_mode not in [0, 0x65, 7] or (mode_zero_parameter_flags & 0x41) != 0:
        return false
    var stable_count := int(owner.get_meta("source_camera_static_probe_count", 0)) + 1
    owner.set_meta("source_camera_static_probe_count", stable_count)
    if (source_mode_zero_action_flags & 0x8000) != 0 or (mode_zero_parameter_flags & 8) != 0:
        return false
    if stable_count < 1:
        return false
    owner.set_meta("source_camera_static_probe_count", 0)
    var end_point := candidate_24 if (source_mode_zero_action_flags & 2) == 0 and motion_mode in [0, 7] else candidate_30
    var geometry_blocked: Variant = _source_mode_zero_static_geometry_blocked(anchor_3c, end_point, context)
    if geometry_blocked == null:
        return null
    if not bool(geometry_blocked):
        owner.set_meta("source_camera_static_obstruction_count", 0)
        controller_state.call("record_mode_zero_candidate_obstruction", false)
        return false
    var obstruction_count := int(owner.get_meta("source_camera_static_obstruction_count", 0)) + 1
    owner.set_meta("source_camera_static_obstruction_count", obstruction_count)
    return obstruction_count > 60

func _source_mode_zero_collision_adjusted_position(ray_result: Dictionary, normal_distance: float) -> Variant:
    if not ray_result.has("hit_position") or not ray_result.has("hit_normal"):
        push_error("Source 0051f450 received a ray result without collision output")
        return null
    var hit_position: Vector3 = ray_result["hit_position"]
    var hit_normal: Vector3 = ray_result["hit_normal"]
    var adjusted := hit_position + hit_normal.normalized() * normal_distance
    if not adjusted.is_finite():
        push_error("Source 0051f450 produced a nonfinite controller position")
        return null
    return adjusted

func _source_mode_zero_controller_result(
    controller_result: bool,
    candidate_24: Vector3,
    candidate_30: Vector3,
    first_ray_initially_blocked: bool,
    second_ray_blocked: bool,
    target_head_checked: bool
) -> Dictionary:
    source_controller_position_24 = candidate_24
    source_controller_position_30 = candidate_30
    source_controller_candidate_result = controller_result
    return {
        "controller_result": controller_result,
        "candidate_position_24": candidate_24,
        "candidate_position": candidate_30,
        "first_ray_initially_blocked": first_ray_initially_blocked,
        "second_ray_blocked": second_ray_blocked,
        "target_head_checked": target_head_checked,
    }

## 0051f520 ORs +0x20 every time it runs and +0x880 when its source actions
## are active. Godot uses only existing action names; no physical keys are made up.
func _source_mode_zero_apply_input_flags(include_side_actions: bool) -> void:
    if include_side_actions and (
        _source_camera_input_pressed("Button_SSLeft")
        or _source_camera_input_pressed("Button_SSRight")
        or _source_camera_input_pressed("Button_Down")
    ):
        source_mode_zero_action_flags |= 0x880
    source_mode_zero_action_flags |= 0x20

func _source_camera_input_pressed(action_name: String) -> bool:
    return InputMap.has_action(action_name) and Input.is_action_pressed(action_name)

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
    var result: Variant = _source_camera_ray_result(start, finish, context, static_geometry_only, apply_material_filter)
    if result == null:
        return null
    return bool(result["blocked"])

## Structured form of 00463df0 / source static-segment checks. The camera
## controller needs the source hit point and normal for 0051f450's +0.35 move.
func _source_camera_ray_result(
    start: Vector3,
    finish: Vector3,
    context: Dictionary,
    static_geometry_only: bool = false,
    apply_material_filter: bool = true
) -> Variant:
    if start.is_equal_approx(finish):
        return {"blocked": false, "degenerate": true}
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
            return {"blocked": false}
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
            return {
                "blocked": false,
                "material_filtered": true,
                "hit_position": hit["position"],
                "hit_normal": hit["normal"],
                "collision_identifier": identifier,
            }
        var hit_position: Vector3 = hit["position"]
        var hit_normal: Vector3 = hit["normal"]
        if not hit_position.is_finite() or not hit_normal.is_finite() or hit_normal.length_squared() < 0.999:
            push_error("Source mode-0 collision result lacks a finite unit hit point/normal")
            return null
        return {
            "blocked": true,
            "hit_position": hit_position,
            "hit_normal": hit_normal.normalized(),
            "collision_identifier": identifier,
            "source_sector_id": sector_id,
        }
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
    var owner_flags := source_owner_position_flag_word()
    controller_state.call("begin_mode_zero_update")
    var live_pivot := source_mode_zero_live_pivot()
    if live_pivot.is_empty():
        return false
    var orientation_data := source_mode_zero_orientation_54()
    if orientation_data.is_empty():
        return false
    var current_eye := owner.global_position
    var anchor_3c: Vector3 = live_pivot["anchor"]
    var candidate_24: Vector3
    var candidate_30: Vector3
    if (owner_flags & SOURCE_POSITION_FLAG_HAS_TARGET_OFFSET) != 0:
        if not owner.has_meta("source_position_target_offset"):
            push_error("Source Position bit 2 is set without its 00440fe0 +0x50 vector")
            return false
        var source_offset = owner.get_meta("source_position_target_offset")
        if not source_offset is Vector3 or not source_offset.is_finite():
            push_error("Source Position+0x50 controller vector is missing or invalid")
            return false
        # 0051db70's bit-2 branch copies +0x50 to +0x48 and forms +0x24
        # directly from current +0x18; it bypasses target angle/range solving.
        candidate_24 = current_eye + source_offset
        candidate_30 = candidate_24
    else:
        var candidate := source_compute_mode_zero_candidate(
            anchor_3c,
            current_eye,
            orientation_data["orientation"],
            source_mode_zero_vertical_limits()
        )
        if candidate.is_empty():
            return false
        candidate_24 = candidate["position"]
        candidate_30 = candidate_24
    var context := _source_collision_context()
    if context.is_empty():
        return false
    var controller_result: Dictionary
    if collision_selector == 0:
        controller_result = _source_mode_zero_controller_670(anchor_3c, current_eye, candidate_24, candidate_30, context)
    elif collision_selector == 1:
        controller_result = _source_mode_zero_controller_290(
            anchor_3c, current_eye, candidate_24, candidate_30, orientation_data["orientation"], context
        )
    else:
        push_error("Source DAT_00721b0c has an unsupported value: " + str(collision_selector))
        return false
    if controller_result.is_empty():
        return false
    var candidate_result := bool(controller_result["controller_result"])
    candidate_24 = controller_result["candidate_position_24"]
    candidate_30 = controller_result["candidate_position"]
    if candidate_result and not bool(controller_state.call("record_mode_zero_good_position", candidate_30)):
        push_error("Source camera controller state rejected the current DAT_007219ac candidate")
        return false

    var fallback_succeeded := false
    if not candidate_result and bool(controller_state.call("mode_zero_fallback_requested")):
        var fallback_sweep := source_mode_zero_fallback_sweep(anchor_3c, current_eye, orientation_data["orientation"])
        if fallback_sweep.is_empty():
            return false
        fallback_succeeded = bool(fallback_sweep.get("success", false))
        if fallback_succeeded:
            controller_state.call("record_mode_zero_fallback_success")
        else:
            controller_state.call("record_mode_zero_fallback_failure")
    var distance_class: Variant = source_mode_zero_distance_class(current_eye, anchor_3c)
    if distance_class == null:
        return false
    controller_state.call("update_mode_zero_distance_hysteresis",
        candidate_result,
        int(distance_class),
        current_eye,
        anchor_3c,
        mode_zero_distance.y,
        bool(controller_state.call("mode_zero_update_obstructed"))
    )
    var pose_write_requested := bool(controller_state.call("mode_zero_pose_write_requested"))
    if pose_write_requested and (not candidate_result or (owner_flags & SOURCE_POSITION_FLAG_HAS_TARGET_OFFSET) == 0):
        if candidate_result and bool(controller_state.call("can_recover_mode_zero_position")):
            if not bool(controller_state.call("record_mode_zero_good_position", current_eye)):
                push_error("Source 0051c8a0 rejected the current camera position as the stable candidate")
                return false
        var obstruction_position: Vector3 = controller_state.call("mode_zero_last_obstructed_position")
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
    elif not candidate_result and bool(controller_state.call("mode_zero_update_obstructed")):
        # The source routes this case through 0051ec70, which updates internal
        # camera-offset interpolation via 0052c7f0/0051e350/0051e600. Those
        # routines have no equivalent native camera-motion API yet; keep the
        # computed controller positions observable, but do not claim success.
        push_error("Source camera collision state requires the unported 0051ec70 offset-interpolation path")
        return false
    # 0051c7b0 is the only proven owner-pose writer in the mode-0 gate; the
    # ordinary candidate remains controller state for the camera actor path.
    return true
