extends Camera3D
## Native target and framing addressed by the compiled camera AI API.
## Mode 3 aims the authored eye; mode 6 preserves its authored orientation.
## Primary viewport selection reuses the framing reset and contact-sector owner.
## Copying another camera's pose and optimal placement remain separate contracts.

@export var motion_mode: int = 0
@export var initial_motion_mode: int = 0
@export var initial_framing: Resource
@export var actor_offset: Vector3 = Vector3.ZERO
@export var look_offset: Vector3 = Vector3.ZERO
@export var initially_current: bool = false
@export var initially_registered: bool = false
@export var initial_viewport_context: int = 0
@export var initial_secondary_registered: bool = false
@export var initial_secondary_viewport_context: int = 0
@export var initial_auxiliary_viewport_context: int = 0
@export var source_initial_horizontal_fov: float = 0.0
var source_viewport_registered: bool = false
var source_viewport_context: int = 0
var source_secondary_registered: bool = false
var source_secondary_viewport_context: int = 0
var source_auxiliary_viewport_context: int = 0
const SOURCE_PRIMARY_CAMERA_OWNER := &"astrolabe_source_primary_camera"
@export_node_path("Node3D") var initial_target_path: NodePath
@export var mode_zero_target_offset: Vector3 = Vector3.ZERO
@export var mode_zero_actor_offset: Vector3 = Vector3.ZERO
@export var mode_zero_look_offset: Vector3 = Vector3.ZERO
@export var mode_zero_distance: Vector2 = Vector2.ZERO
@export var mode_zero_target_child_index: int = 0
@export var mode_zero_yaw_limits: Vector2 = Vector2.ZERO
## Function_Camera_GetShiftAlpha reads camera parameter +0x44 directly.
## 00440070 initializes this source float to zero; the angular consumer applies
## its own radians-window rules after this raw getter returns.
@export var source_shift_alpha: float = 0.0
@export var mode_zero_pitch_limits: Vector2 = Vector2.ZERO
@export var mode_zero_parameter_flags: int = 0
@export var controller_state: Resource
## Imported from the owning Perso's motion component edge. Mechanics identity
## and camera presence alone cannot establish this binding.
@export var owner_motion_binding_present := false
## Actor-owned camera progress. The target-switch procedure resets these,
## while restoring MainActor leaves them alone. Obstruction history is separate.
var fallback_motion_frames := 0
var collision_recovery_frames := 0
var look_direction_hold_frames := 0
var static_probe_frames := 0
var static_obstruction_frames := 0
var previous_look_radius := 0.0
var slope_pitch_adjustment := 0.0
var _eye_displacement_pending := false
var _eye_displacement := Vector3.ZERO
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
const SOURCE_POSITION_DEFAULT_MIN := -3.4028220466166163e38 # source finite lower sentinel
const SOURCE_POSITION_DEFAULT_MAX := 3.4028220466166163e38 # source finite upper sentinel
const CAMERA_POST_CONTACT_POSITION := &"post_contact_position"
const CAMERA_RETAINED_POSITION_HISTORY := &"retained_position_history"

var targeted_actor: Node3D
var _reported_mode_zero_gap := false
var _source_active_sector: Node
var _source_previous_history_position := Vector3.ZERO
# A new SourceCamera represents a new DNM context; 00409840 zero-fills payload
# +0x32c before 00527e00 exposes that byte as camera +0x254.
var _source_camera_collision_guard := false

## 0x0053d300 dispatches Function_Camera_GetShiftAlpha (selector L, Perso P)
## to 00440bf0, which reads the unmodified float at parameter +0x44. Camera
## model 437 uses selectors 0 and 1; other selectors remain unsupported here.
func source_get_shift_alpha(selector: int) -> Variant:
    if selector not in [0, 1]:
        return null
    return source_shift_alpha

## 0x0053d300 dispatches Function_Camera_GetShiftTeta (selector L, Perso P)
## to 00440c30, which reads the unmodified float at parameter +0x4c. The
## observed camera model uses selectors 0 and 1; other selectors fail closed.
func source_get_shift_teta(selector: int) -> Variant:
    if selector not in [0, 1]:
        return null
    return mode_zero_pitch_limits.y

func _ready() -> void:
    motion_mode = initial_motion_mode
    source_viewport_registered = initially_registered
    source_viewport_context = initial_viewport_context
    source_secondary_registered = initial_secondary_registered
    source_secondary_viewport_context = initial_secondary_viewport_context
    source_auxiliary_viewport_context = initial_auxiliary_viewport_context
    if not initial_target_path.is_empty():
        targeted_actor = get_node_or_null(initial_target_path) as Node3D
    _source_active_sector = _source_sector_for(get_parent())
    var owner := get_parent() as Node3D
    if owner != null:
        _source_previous_history_position = owner.global_position
    if initially_current:
        _source_publish_primary()
    else:
        clear_current(false)

## The native viewport owns the source camera selection independently of an
## authored presentation camera temporarily made current on the same viewport.
## A weak reference avoids retaining an unloaded level's camera.
func _source_primary_owner() -> Camera3D:
    var binding: Variant = get_viewport().get_meta(SOURCE_PRIMARY_CAMERA_OWNER, null)
    if not (binding is WeakRef): return null
    var camera := binding.get_ref() as Camera3D
    if not is_instance_valid(camera) or not camera.is_inside_tree() or camera.get_viewport() != get_viewport():
        return null
    return camera

func _source_publish_primary() -> void:
    source_viewport_registered = true
    source_viewport_context = 1
    _source_register_primary_owner()

func _source_register_primary_owner() -> void:
    get_viewport().set_meta(SOURCE_PRIMARY_CAMERA_OWNER, weakref(self))
    make_current()

## Proc141 always restores the previous source target's opacity, including
## a request for the already selected camera. Only the primary viewport is mapped.
func source_select_primary(context: Variant, previous_policy: Variant, placement_policy: Variant) -> bool:
    var owner := get_parent() as Node3D
    if owner == null or not owner.has_method("source_scalar_word") or int(owner.call("source_scalar_word", context)) != 1:
        return false
    var previous := _source_primary_owner()
    if previous == null or not previous.has_method("source_prepare_target_change"):
        return false
    if not bool(previous.call("source_prepare_target_change")): return false
    if previous == self:
        # Source returns before reset/placement when selecting the same owner.
        # Ensure its native Camera3D renders the selected viewport even if an
        # authored presentation camera had temporarily taken Godot's current slot.
        make_current()
        return true
    # The primary source viewport declines a switch when its old active
    # parameters no longer refer to that viewport. Opacity was still restored.
    if int(previous.get("source_viewport_context")) != 1:
        return true
    var reset_previous: int = int(owner.call("source_scalar_word", previous_policy))
    var placement: int = int(owner.call("source_scalar_word", placement_policy))
    var reset_selected := not source_viewport_registered
    if not reset_selected:
        placement = 0
    if placement in [1, 2]:
        # Pose copy and the optimal-placement solver have wider matrix contracts.
        return false
    if reset_selected and not _source_can_reset_for_selection(): return false
    if reset_previous == 1 and (not previous.has_method("_source_can_reset_for_selection") or not bool(previous.call("_source_can_reset_for_selection"))):
        return false
    if not owner.has_method("source_refresh_contact_sector"): return false

    # Source publishes the viewport owner before resetting the destination.
    # Its reset copies only initial -> active; the secondary state survives.
    _source_register_primary_owner()
    if reset_selected and not _source_reset_for_selection(): return false
    source_viewport_registered = true
    source_secondary_registered = true
    source_viewport_context = 1
    source_secondary_viewport_context = 1
    source_auxiliary_viewport_context = 1
    if reset_previous == 1:
        if not bool(previous.call("_source_reset_for_selection")): return false
        previous.set("source_viewport_registered", false)
    elif int(previous.get("source_viewport_context")) == 1:
        previous.set("source_viewport_context", 0)
        previous.set("source_secondary_viewport_context", 0)
        previous.set("source_auxiliary_viewport_context", 0)
    # Placement zero still performs the sector query at the existing eye.
    # Other source placement words, except 1/2, also take this same branch.
    return _source_refresh_selection_sector(owner)

func _source_can_reset_for_selection() -> bool:
    if not has_method("source_reset_to_initial"): return false
    var framing := get("initial_framing") as Resource
    if framing == null: return false
    var initial_fov: float = framing.get("horizontal_fov")
    return is_finite(initial_fov) and initial_fov > 0.0 and initial_fov < PI

func _source_reset_for_selection() -> bool:
    # Reuse the maintained Proc136 framing/progress reset. The viewport fields
    # are part of the same initial -> active copy, rather than renderer state.
    if not has_method("source_reset_to_initial") or not bool(call("source_reset_to_initial")):
        return false
    source_viewport_registered = initially_registered
    source_viewport_context = initial_viewport_context
    return true

func _source_refresh_selection_sector(owner: Node3D) -> bool:
    return _source_refresh_owner_sector_at(owner, owner.global_position)

## 00447bf0 researches the explicit lifecycle position and publishes holder+4.
## Use the existing ordered source resolver and bound last-sibling fallback.
func _source_refresh_owner_sector_at(owner: Node3D, position: Vector3) -> bool:
    var result := source_resolve_mode_zero_sector(position)
    return _source_publish_owner_sector_result(owner, position, result)

func _source_publish_owner_sector_result(owner: Node3D, position: Vector3, result: Dictionary) -> bool:
    if not owner.has_method("source_refresh_contact_sector"): return false
    if not bool(result.get("complete", false)): return false
    var root := _source_level_root()
    var sector: Node3D
    var sector_id := str(result.get("sector_id", ""))
    if not sector_id.is_empty():
        sector = _find_source_sector(root, sector_id) as Node3D
    elif is_instance_valid(root):
        # The original no-hit fallback is the last child of the sector root.
        # Export binds that ordered sibling as an ordinary scene NodePath.
        var fallback: Variant = root.get_meta("source_sector_fallback_path", NodePath())
        if fallback is NodePath and not fallback.is_empty():
            sector = root.get_node_or_null(fallback) as Node3D
    if not is_instance_valid(sector) or not bool(owner.call("source_refresh_contact_sector", sector)):
        return false
    _source_active_sector = sector
    _source_previous_history_position = position
    return true

## Proc158's unregistered branch returns before evaluating selector or value.
## Source modes 101/104/105 select another parameter set that is not mapped here.
func source_focal_registered() -> Variant:
    if motion_mode in [101, 104, 105]: return null
    return source_viewport_registered

func source_change_focal(selector: Variant, value: Variant) -> bool:
    var owner := get_parent() as Node3D
    if owner == null or not owner.has_method("source_scalar_word"): return false
    var operation: int = int(owner.call("source_scalar_word", selector))
    var radians := float(value)
    if typeof(value) != TYPE_FLOAT:
        var word: int = int(owner.call("source_scalar_word", value))
        radians = float(word - 0x100000000 if word >= 0x80000000 else word)
    radians = PackedFloat32Array([radians])[0]
    if operation == 0xffffffff:
        radians = source_initial_horizontal_fov
    elif operation == 1:
        if (mode_zero_parameter_flags & 0x20) != 0: return false
        radians += deg_to_rad(fov)
    elif operation != 0:
        # Other selectors retain the previous focal value in the source.
        return true
    radians = PackedFloat32Array([radians])[0]
    if not is_finite(radians) or radians <= 0.0 or radians >= PI: return false
    fov = rad_to_deg(radians)
    return true

## 005796ea clears camera payload +0x32c at callback entry, then sets it when
## its 00463df0 obstruction query blocks. Feed the callback's computed result.
func source_camera_collision_callback_update(obstructed: bool) -> void:
    _source_camera_collision_guard = false
    if obstructed:
        _source_camera_collision_guard = true

func source_camera_collision_guard_value() -> bool:
    return _source_camera_collision_guard

## Proc160 restores the previous target before evaluating its new operand.
## The compiler calls this phase first, so a failed new binding still restores it.
func source_prepare_target_change() -> bool:
    if not is_instance_valid(targeted_actor) or not targeted_actor.has_method("source_set_transparency"):
        return false
    return bool(targeted_actor.call("source_set_transparency", 255))

func source_change_targeted_perso(actor: Node3D) -> bool:
    if not is_instance_valid(actor):
        return false
    targeted_actor = actor
    _reset_target_progress()
    return true

## 00548070 resets the four word-sized progress values and two float histories.
## The static obstruction streak and session-wide pivot/teleport history survive.
func _reset_target_progress() -> void:
    fallback_motion_frames = 0
    collision_recovery_frames = 0
    look_direction_hold_frames = 0
    static_probe_frames = 0
    previous_look_radius = 0.0
    slope_pitch_adjustment = 0.0

## Proc163 only assigns MainActor; no opacity restoration or progress reset.
func source_reset_targeted_perso(actor: Node3D) -> bool:
    if not is_instance_valid(actor):
        return false
    targeted_actor = actor
    return true

## Proc166's immediate flag groups update active framing. Selectors name source
## API bits rather than bit positions. Deferred groups need an owned consumer.
func source_change_flag(group: int, operation: int, selector: int) -> bool:
    if group not in [0, 1]: return false
    var bits := [0, 1, 2, 4, 16, 512, 1024, 2048, 8192] if group == 0 else [0, 1, 2, 4, 16, 64, 256, 128, 32, 512, 32768, 1024, 2048]
    # The source's unsigned range check leaves an invalid selector's mask zero.
    var mask: int = bits[selector] if selector >= 0 and selector < bits.size() else 0
    var set_bit := operation == 1
    if operation == -1:
        if initial_framing == null: return false
        var initial: int = initial_framing.parameter_flags if group == 0 else initial_framing.action_flags
        set_bit = (initial & mask) != 0
    elif operation not in [0, 1]:
        return true
    if group == 0:
        mode_zero_parameter_flags = ((mode_zero_parameter_flags | mask) if set_bit else (mode_zero_parameter_flags & ~mask)) & 65535
    else:
        source_mode_zero_action_flags = ((source_mode_zero_action_flags | mask) if set_bit else (source_mode_zero_action_flags & ~mask)) & 65535
    return true

## Proc136 copies initial framing to active framing and requests the first
## snapped update. Reset only component-owned progress; controller_state owns
## the distinct process-wide history. The selected target is preserved.
func source_reset_to_initial() -> bool:
    if initial_framing == null: return false
    var initial_fov: float = initial_framing.horizontal_fov
    if not is_finite(initial_fov) or initial_fov <= 0.0 or initial_fov >= PI: return false
    motion_mode = int(initial_framing.motion_mode)
    actor_offset = initial_framing.actor_offset
    look_offset = initial_framing.look_offset
    mode_zero_target_offset = initial_framing.target_offset
    mode_zero_actor_offset = initial_framing.actor_offset
    mode_zero_look_offset = initial_framing.look_offset
    mode_zero_distance = initial_framing.distance
    mode_zero_yaw_limits = initial_framing.yaw_limits
    source_shift_alpha = float(initial_framing.shift_alpha)
    mode_zero_pitch_limits = initial_framing.pitch_limits
    mode_zero_parameter_flags = int(initial_framing.parameter_flags) | 8
    source_mode_zero_action_flags = int(initial_framing.action_flags)
    mode_zero_target_child_index = int(initial_framing.target_child_index)
    fov = rad_to_deg(initial_fov)
    _reset_target_progress()
    static_obstruction_frames = 0
    _eye_displacement = Vector3.ZERO
    source_clear_eye_displacement()
    _reported_mode_zero_gap = false
    var owner := get_parent() as Node3D
    if owner != null:
        owner.set_meta("source_mode_zero_vertical_limits", Vector2(SOURCE_POSITION_DEFAULT_MIN, SOURCE_POSITION_DEFAULT_MAX))
    return true

## 00547be0 restores the original preset for -1 and leaves equal modes alone.
## The observed opening transition 0 -> 3 has no extra reset branches.
## Mode 6's frame consumer preserves orientation. Other transitions require
## follow/obstruction state that is not mapped yet.
func source_change_state(state: int) -> bool:
    var selected := initial_motion_mode if state == -1 else state
    if selected == motion_mode:
        return true
    if motion_mode not in [0,3,6] or selected not in [0,3,6]:
        return false
    motion_mode = selected
    return true

## Hype 0051c8d0/0051d140 rotate the two offsets through the target's rotation.
## Mode 3 leaves ordinary eye motion to the camera actor (005200a0/0051e620).
## Zero look offset defaults to source [0,-0.05,0], or native [0,0,0.05].
func source_update_position() -> bool:
    # Capture before calculation; source clears only the initially present bit.
    var clear_initial_snap := (mode_zero_parameter_flags & 8) != 0
    var result := false
    if motion_mode == 0:
        result = _source_update_mode_zero()
    elif motion_mode == 3:
        result = _source_update_mode_three()
    elif motion_mode == 6:
        result = _source_update_mode_six()
    if not result:
        return false
    # 00545d80's null caller motion-component branch is after frame application
    # and skips ordinary cleanup. Preserve the mapped transient values there.
    if owner_motion_binding_present:
        if clear_initial_snap:
            mode_zero_parameter_flags &= ~8
        source_clear_eye_displacement()
    return true

## 005200a0 supplies no displacement in mode6. Its 0x100 frame-action branch
## in 0057b17f writes translation and skips the rotation-matrix calculation.
## The authored traveling actor can keep moving the eye through the normal
## position API; this component preserves that eye and its previous basis.
func _source_update_mode_six() -> bool:
    var eye := get_parent() as Node3D
    if eye == null or not eye.is_inside_tree() or not is_instance_valid(targeted_actor):
        return false
    _reset_motion_progress()
    return true

func _source_update_mode_three() -> bool:
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
    # The mode-3 calculation resets word-sized progress, but not look radius.
    _reset_motion_progress()
    return true

func _reset_motion_progress() -> void:
    fallback_motion_frames = 0
    collision_recovery_frames = 0
    look_direction_hold_frames = 0
    static_probe_frames = 0
    slope_pitch_adjustment = 0.0

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
## supports the converted Walk position publisher shared by kinds 0 and 4.
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
    var position_source: StringName = data3d.get("position_source", &"")
    if position_source not in [CAMERA_POST_CONTACT_POSITION, CAMERA_RETAINED_POSITION_HISTORY]:
        push_error("Source mode-0 target position history has no converted binding")
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
    pivot["position_source"] = position_source
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
    if distance < mode_zero_distance.x:
        distance = mode_zero_distance.x
    elif mode_zero_distance.y < distance:
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

## Proc171 resets only the actor-owned maximum vertical bound. Its sole
## operand is the camera actor; no extra scalar evaluation or framing reset.
func source_reset_vertical_maximum() -> bool:
    var owner := get_parent() as Node3D
    if owner == null: return false
    var limits := Vector2(SOURCE_POSITION_DEFAULT_MIN, SOURCE_POSITION_DEFAULT_MAX)
    if owner.has_meta("source_mode_zero_vertical_limits"):
        var previous: Variant = owner.get_meta("source_mode_zero_vertical_limits")
        if not (previous is Vector2): return false
        limits = previous
    limits.y = SOURCE_POSITION_DEFAULT_MAX
    owner.set_meta("source_mode_zero_vertical_limits", limits)
    return true

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
    if obstruction_count > SOURCE_MODE_ZERO_CAMERA_BLOCK_STREAK_LIMIT or source_camera_collision_guard_value():
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
    if _eye_displacement_pending:
        return false
    if motion_mode not in [0, 0x65, 7] or (mode_zero_parameter_flags & 0x41) != 0:
        return false
    static_probe_frames = _source_increment_signed_word(static_probe_frames)
    if (source_mode_zero_action_flags & 0x8000) != 0 or (mode_zero_parameter_flags & 8) != 0:
        return false
    if static_probe_frames < 1:
        return false
    static_probe_frames = 0
    var end_point := candidate_24 if (source_mode_zero_action_flags & 2) == 0 and motion_mode in [0, 7] else candidate_30
    var geometry_blocked: Variant = _source_mode_zero_static_geometry_blocked(anchor_3c, end_point, context)
    if geometry_blocked == null:
        return null
    if not bool(geometry_blocked):
        static_obstruction_frames = 0
        controller_state.call("record_mode_zero_candidate_obstruction", false)
        return false
    static_obstruction_frames = _source_increment_signed_word(static_obstruction_frames)
    # 0052bb50 returns one for every obstructed probe. The >60 branch also
    # returns one; it is not a grace period during which obstruction is ignored.
    return true

func _source_increment_signed_word(value: int) -> int:
    return ((value + 1 + 32768) & 65535) - 32768

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

## 00440fe0 supplies one update's eye displacement to the camera component.
## It does not write SuperObject Position or attach state to the parent node.
func source_set_eye_displacement(offset: Vector3) -> bool:
    if not offset.is_finite():
        return false
    _eye_displacement = offset
    _eye_displacement_pending = true
    return true

func source_has_eye_displacement() -> bool:
    return _eye_displacement_pending

func source_clear_eye_displacement() -> void:
    _eye_displacement_pending = false

## 0051c7b0 captures00448350's final sector-root child before researching its
## third position. A nonnull hit other than that child selects the third point;
## a no-hit or final-child result selects the second point.
func source_publish_mode_zero_translation(
    third_argument_position: Vector3,
    second_argument_position: Vector3
) -> bool:
    var owner := get_parent() as Node3D
    if owner == null or not owner.is_inside_tree():
        return false
    if not third_argument_position.is_finite() or not second_argument_position.is_finite():
        return false
    # The original getter returns a full pointer, independent of the camera's
    # current holder. Capture the exported native identity before the lookup.
    var root := _source_level_root()
    if not is_instance_valid(root): return false
    var fallback: Variant = root.get_meta("source_sector_fallback_path", NodePath())
    if not (fallback is NodePath) or fallback.is_empty():
        push_error("Source 0051c7b0 publication needs the final sector-root child binding")
        return false
    var final_sector := root.get_node_or_null(fallback) as Node3D
    if not is_instance_valid(final_sector) or not final_sector.has_meta("source_sector_id"):
        push_error("Source 00448350 final sector-root child is unbound")
        return false
    var sector_result := source_resolve_mode_zero_sector(third_argument_position)
    if not bool(sector_result.get("complete", false)):
        push_error("Source 0051c7b0 publication needs a complete 004e0020 sector result; type-1/11 child GEO and physical visuals with multiple source LODs are not represented faithfully")
        return false
    var resolved_sector_id := str(sector_result.get("sector_id", ""))
    var next_sector: Node
    if not resolved_sector_id.is_empty():
        next_sector = _find_source_sector(root, resolved_sector_id)
        if not is_instance_valid(next_sector):
            push_error("Source camera pose references an unbound destination sector")
            return false
    var published_position := third_argument_position if is_instance_valid(next_sector) and next_sector != final_sector else second_argument_position
    # Publication precedes matrix writes. Reuse the already complete result
    # when that exact position was selected; only the second-point branch can
    # require another query. Do not cache position results across updates.
    if published_position == third_argument_position:
        if not _source_publish_owner_sector_result(owner, published_position, sector_result): return false
    elif not _source_refresh_owner_sector_at(owner, published_position):
        return false
    owner.global_position = published_position
    _source_previous_history_position = owner.global_position
    return true

## 004e0020 drops a one-unit semiaxis from the query point and chooses the
## nearest hit among active nonvirtual sector borders and eligible child GEOs.
func source_resolve_mode_zero_sector(position: Vector3) -> Dictionary:
    if not position.is_finite(): return {}
    var index := _source_sector_index(_source_level_root())
    if index == null:
        push_error("Source camera needs the exported SourceSectorIndex; regenerate this level")
        return {}
    return index.call("resolve_position", position)

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
            var geometry_reachable := camera_reachable
            var target_sector := _source_sector_for(targeted_actor) if is_instance_valid(targeted_actor) else null
            if target_sector != null and target_sector != camera_sector:
                var target_reachable := _source_collision_sectors([target_sector])
                if target_reachable.is_empty(): return {}
                geometry_reachable = camera_reachable.duplicate()
                geometry_reachable.merge(target_reachable)
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
        var source_data = _source_sector_data(direct_sector)
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
            var neighbor_data = _source_sector_data(neighbor_sector)
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
    if is_instance_valid(sector) and _find_source_sector(_source_level_root(), str(sector.get_meta("source_sector_id", ""))) != sector:
        sector = null
    if not is_instance_valid(sector):
        sector = _source_sector_for(get_parent())
    if not is_instance_valid(sector):
        # The registered camera Perso (world/0/2) is not a child of a Sector
        # node and is absent from the static Sector.persos lists. Seed its
        # runtime sector with the same point resolver used by source 004e0020
        # before applying 004e03f0's ordered boundary crossings.
        var initial_sector: Dictionary = source_resolve_mode_zero_sector(current_position)
        if not bool(initial_sector.get("complete", false)):
            push_error("Source camera cannot seed boundary history from an incomplete 004e0020 result")
            return false
        var initial_sector_id := str(initial_sector.get("sector_id", ""))
        if initial_sector_id.is_empty():
            push_error("Source 004e0020 found no initial sector for camera boundary history")
            return false
        var level_root := _source_level_root()
        if not is_instance_valid(level_root):
            push_error("Source camera has no level root for initial sector binding")
            return false
        sector = _find_source_sector(level_root, initial_sector_id)
        if sector == null:
            push_error("Source 004e0020 initial sector binding is missing")
            return false
        if not sector.has_meta("source_sector"):
            push_error("Source 004e0020 initial sector lacks source collision metadata")
            return false
        _source_active_sector = sector
        # The point resolver establishes the sector at the current pose, so
        # start boundary history here instead of traversing from a stale pose.
        _source_previous_history_position = current_position
        return true
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
        var next_sector := crossing.get("target") as Node
        if not is_instance_valid(next_sector):
            next_sector = _find_source_sector(_source_level_root(), next_id)
        elif _find_source_sector(_source_level_root(), next_id) != next_sector:
            push_error("Source sector crossing target does not match its level binding")
            return false
        if next_sector == null:
            push_error("Source sector boundary references an unbound neighbor")
            return false
        var next_data = _source_sector_data(next_sector)
        if not next_data is Dictionary or not next_data.has("activation"):
            push_error("Source destination sector has no activation metadata")
            return false
        if (int(next_data["activation"]) & 4) == 0:
            break
        sector = next_sector
        cursor = crossing["position"]
    # 00448280 publishes a changed current-sector holder during contact history.
    # A private initial resolver seed alone does not replace an imported null.
    var owner := get_parent() as Node3D
    if owner != null and owner.has_method("source_sector_membership_ready") and bool(owner.call("source_sector_membership_ready")):
        var current := owner.call("source_current_sector") as Node3D
        if is_instance_valid(current) and current != sector:
            if not bool(owner.call("source_refresh_contact_sector", sector)): return false
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

func _source_sector_index(root: Node) -> Node:
    if not is_instance_valid(root) or root.is_queued_for_deletion(): return null
    var index := root.get_node_or_null("SourceSectorIndex")
    return index if is_instance_valid(index) and not index.is_queued_for_deletion() else null

func _source_sector_data(sector: Node) -> Variant:
    var index := _source_sector_index(_source_level_root())
    return index.call("sector_data", sector) if index != null else null

func _find_source_sector(root: Node, sector_id: String) -> Node:
    var index := _source_sector_index(root)
    return index.call("sector_node", sector_id) if index != null else null

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
        if current.has_method("source_contact_sector"):
            var sector := current.call("source_contact_sector") as Node3D
            if is_instance_valid(sector): return sector
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
    var supplied_eye_displacement := _eye_displacement_pending
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
    if supplied_eye_displacement:
        # The supplied motion bypasses target angle/range solving.
        candidate_24 = current_eye + _eye_displacement
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
    if pose_write_requested and (not candidate_result or not supplied_eye_displacement):
        if candidate_result and bool(controller_state.call("can_recover_mode_zero_position")):
            if not bool(controller_state.call("record_mode_zero_good_position", current_eye)):
                push_error("Source 0051c8a0 rejected the current camera position as the stable candidate")
                return false
        var obstruction_position: Vector3 = controller_state.call("mode_zero_last_obstructed_position")
        var last_good_position: Vector3 = controller_state.call("mode_zero_last_good_position")
        if not source_publish_mode_zero_translation(
            obstruction_position,
            last_good_position
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
