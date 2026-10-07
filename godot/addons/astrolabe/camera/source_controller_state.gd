extends Resource
## Session-owned state for source camera process globals across level changes.

var _initialized := false
var _hysteresis_limit := 0
var _mode_zero_collision_selector := 0 # DAT_00721b0c; zero-filled startup selects 0051f670
var _fallback_hysteresis := 0 # DAT_00721974
var _distance_hysteresis := 0 # DAT_007219a4
var _collision_streak := 0 # DAT_0072198c
var _candidate_obstruction_streak := 0 # DAT_00721988
var _target_anchor_hold_frames := 0 # DAT_00721af8
var _target_anchor_motion_samples := 0 # DAT_00721afc
var _mode_zero_update_has_position := false # DAT_007219d0
var _mode_zero_pose_write_requested := false # DAT_007219d1
var _mode_zero_update_obstructed := false # DAT_007218fd
var _mode_zero_fallback_requested := false # DAT_007218fc
var _mode_zero_last_good_position := Vector3.ZERO # DAT_007219ac
var _mode_zero_last_obstructed_position := Vector3.ZERO # DAT_007219c4; BSS-zero until written
var _mode_zero_has_obstructed_position := false # evidence only; source uses the vector even before it is written
var _mode_zero_fallback_position := Vector3.ZERO # DAT_007219b8..c0
var _mode_zero_fallback_yaw := 0.0 # DAT_00721a3c
var _mode_zero_fallback_pitch := 0.0 # DAT_00721a44
var _mode_zero_sweep_obstruction_count := 0 # DAT_00721994; no writer in checked decompilation

## 005200a0 switches the selector only on camera-mode input actions.
func mode_zero_collision_selector() -> int:
	return _mode_zero_collision_selector

func set_mode_zero_collision_selector(value: int) -> bool:
	if value not in [0, 1]:
		return false
	_mode_zero_collision_selector = value
	return true

## 0051c8d0 holds raw target translation for eight updates after the source
## weighted center moves beyond its squared-distance threshold.
func smooth_mode_zero_pivot(target_position: Vector3, weighted_center: Vector3, threshold_squared: float) -> Vector3:
	if not target_position.is_finite() or not weighted_center.is_finite() or not is_finite(threshold_squared) or threshold_squared < 0.0:
		return weighted_center
	if weighted_center.distance_squared_to(target_position) > threshold_squared:
		_target_anchor_hold_frames = 8
	if _target_anchor_hold_frames > 0:
		_target_anchor_hold_frames -= 1
		_target_anchor_motion_samples += 1
		return target_position
	return weighted_center

func record_candidate_collision(blocked: bool) -> int:
	if blocked:
		_collision_streak += 1
	else:
		_collision_streak = 0
	return _collision_streak

func mode_zero_collision_streak() -> int:
	return _collision_streak

func record_mode_zero_candidate_obstruction(blocked: bool) -> int:
	if blocked:
		_candidate_obstruction_streak += 1
	else:
		_candidate_obstruction_streak = 0
	return _candidate_obstruction_streak

func mode_zero_candidate_obstruction_streak() -> int:
	return _candidate_obstruction_streak

## 005200a0 clears its per-update globals before calculating the controller.
func begin_mode_zero_update() -> void:
	_initialize()
	_mode_zero_update_has_position = false
	_mode_zero_pose_write_requested = false
	_mode_zero_update_obstructed = false
	_mode_zero_fallback_requested = false

func mode_zero_update_has_position() -> bool:
	return _mode_zero_update_has_position

func mode_zero_pose_write_requested() -> bool:
	return _mode_zero_pose_write_requested

func mode_zero_update_obstructed() -> bool:
	return _mode_zero_update_obstructed

func mode_zero_fallback_requested() -> bool:
	return _mode_zero_fallback_requested

func mark_mode_zero_update_obstructed() -> void:
	_mode_zero_update_obstructed = true

func request_mode_zero_fallback() -> void:
	_mode_zero_fallback_requested = true

func set_mode_zero_fallback_requested(value: bool) -> void:
	_mode_zero_fallback_requested = value

func request_mode_zero_pose_write() -> void:
	_mode_zero_pose_write_requested = true

## 005200a0 stores successful +0x30 in DAT_007219ac. This does not set d0.
func record_mode_zero_good_position(position: Vector3) -> bool:
	if not position.is_finite():
		return false
	_mode_zero_last_good_position = position
	return true

## A clear first ray in 0051f670 or clear 0052bb50 result in 0051f290 writes
## DAT_007219c4 and sets the per-update DAT_007219d0 latch.
func record_mode_zero_controller_position(position: Vector3) -> bool:
	if not position.is_finite():
		return false
	_mode_zero_last_obstructed_position = position
	_mode_zero_has_obstructed_position = true
	_mode_zero_update_has_position = true
	return true

## The first type-2 sweep result writes c4 only while d0 is clear.
func record_mode_zero_obstruction(position: Vector3) -> bool:
	if not position.is_finite():
		return false
	if not _mode_zero_update_has_position:
		_mode_zero_last_obstructed_position = position
		_mode_zero_has_obstructed_position = true
		_mode_zero_update_has_position = true
	return true

func record_mode_zero_fallback(yaw: float, pitch: float, position: Vector3) -> bool:
	if not is_finite(yaw) or not is_finite(pitch) or not position.is_finite():
		return false
	_mode_zero_fallback_yaw = yaw
	_mode_zero_fallback_pitch = pitch
	_mode_zero_fallback_position = position
	return true

func record_mode_zero_recovery_position(position: Vector3) -> bool:
	if not position.is_finite():
		return false
	_mode_zero_fallback_position = position
	return true

func mode_zero_last_good_position() -> Vector3:
	return _mode_zero_last_good_position

func mode_zero_last_obstructed_position() -> Vector3:
	return _mode_zero_last_obstructed_position

func mode_zero_has_obstructed_position() -> bool:
	return _mode_zero_has_obstructed_position

func mode_zero_fallback_position() -> Vector3:
	return _mode_zero_fallback_position

func mode_zero_fallback_angles() -> Vector2:
	return Vector2(_mode_zero_fallback_yaw, _mode_zero_fallback_pitch)

func record_mode_zero_fallback_failure() -> bool:
	_fallback_hysteresis += 1
	if _fallback_hysteresis > _hysteresis_limit and _mode_zero_update_has_position:
		_mode_zero_pose_write_requested = true
	return _mode_zero_pose_write_requested

func record_mode_zero_fallback_success() -> void:
	_fallback_hysteresis = 0

## Port of the distinct success/failure distance-hysteresis branches in 005200a0.
func update_mode_zero_distance_hysteresis(
	candidate_result: bool,
	distance_class: int,
	camera_eye: Vector3,
	pivot_anchor: Vector3,
	maximum_distance: float,
	obstructed: bool
) -> bool:
	_initialize()
	if not camera_eye.is_finite() or not pivot_anchor.is_finite() or not is_finite(maximum_distance):
		return _mode_zero_pose_write_requested
	if _collision_streak == 0 or distance_class != 1:
		_distance_hysteresis = 0
		return _mode_zero_pose_write_requested
	var separation := camera_eye.distance_to(pivot_anchor)
	if candidate_result:
		if separation <= maximum_distance * 3.0:
			_distance_hysteresis = 0
		else:
			_distance_hysteresis += 1
			if _distance_hysteresis > _hysteresis_limit:
				_mode_zero_pose_write_requested = true
		return _mode_zero_pose_write_requested
	if maximum_distance * 2.5 < separation and not obstructed:
		_distance_hysteresis += 1
		if _distance_hysteresis > _hysteresis_limit:
			_mode_zero_pose_write_requested = true
	return _mode_zero_pose_write_requested

func can_reanchor_mode_zero_position() -> bool:
	return _candidate_obstruction_streak < 2 and _collision_streak < 2 and _mode_zero_sweep_obstruction_count < 2

func can_recover_mode_zero_position() -> bool:
	return can_reanchor_mode_zero_position()

## Compatibility entry point for focused callers; the fourth argument is the
## source d0 latch, not the last-good-camera-position flag.
func evaluate_mode_zero_teleport_gate(
	candidate_result: bool,
	fallback_active: bool,
	fallback_succeeded: bool,
	has_update_position: bool,
	distance_class: int,
	camera_eye: Vector3,
	previous_eye: Vector3,
	maximum_distance: float,
	obstructed: bool,
	owner_teleport_blocked: bool
) -> bool:
	_initialize()
	if fallback_active:
		if fallback_succeeded:
			record_mode_zero_fallback_success()
		else:
			_fallback_hysteresis += 1
			if _fallback_hysteresis > _hysteresis_limit and has_update_position:
				_mode_zero_pose_write_requested = true
	update_mode_zero_distance_hysteresis(
		candidate_result, distance_class, camera_eye, previous_eye, maximum_distance, obstructed
	)
	return _mode_zero_pose_write_requested and (not candidate_result or not owner_teleport_blocked)

func _initialize() -> void:
	if _initialized:
		return
	# First-call initialization in 005200a0 sets DAT_00721978 to 10.
	_hysteresis_limit = 10
	_initialized = true
