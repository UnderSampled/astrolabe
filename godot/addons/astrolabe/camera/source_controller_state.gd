extends Resource
## Process-wide state used by source camera controllers during one game session.
## The source counters are global, so every exported SourceCamera shares this
## object and the transition host keeps it when replacing a level instance.

var _initialized := false
var _hysteresis_limit := 0
var _fallback_hysteresis := 0 # 005200a0 DAT_00721974
var _distance_hysteresis := 0 # 005200a0 DAT_007219a4
var _collision_streak := 0 # 0051f670 / 0051f290 DAT_0072198c
var _target_anchor_hold_frames := 0 # 0051c8d0 DAT_00721af8
var _target_anchor_motion_samples := 0 # 0051c8d0 DAT_00721afc

## 0051c8d0 switches to the raw target translation for eight updates after
## the source weighted center moves beyond its squared-distance threshold.
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

## Mirrors the 005200a0 conditional 0051c7b0 branch. The caller must supply
## source-derived candidate, fallback, range, and obstruction values.
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
	_initialize()
	if not camera_eye.is_finite() or not previous_eye.is_finite() or not is_finite(maximum_distance):
		return false
	var teleport_requested := false
	if not candidate_result:
		if fallback_active:
			if not fallback_succeeded:
				_fallback_hysteresis += 1
				if _fallback_hysteresis > _hysteresis_limit and has_last_good_position:
					teleport_requested = true
			else:
				_fallback_hysteresis = 0
		if _collision_streak == 0 or distance_class != 1:
			_distance_hysteresis = 0
		else:
			var separation := camera_eye.distance_to(previous_eye)
			if maximum_distance * 2.5 < separation and not obstructed:
				_distance_hysteresis += 1
				if _distance_hysteresis > _hysteresis_limit:
					teleport_requested = true
		return teleport_requested

	if _collision_streak == 0 or distance_class != 1:
		_distance_hysteresis = 0
	else:
		var separation := camera_eye.distance_to(previous_eye)
		if separation <= maximum_distance * 3.0:
			_distance_hysteresis = 0
		else:
			_distance_hysteresis += 1
			if _distance_hysteresis > _hysteresis_limit:
				teleport_requested = true
	# 005200a0 checks the owner's flag bit 2 only on the nonzero-candidate
	# branch. The fallback branch reaches 0051c7b0 without this check.
	return teleport_requested and not owner_teleport_blocked

func _initialize() -> void:
	if _initialized:
		return
	# First-call initialization in 005200a0 sets DAT_00721978 to 10.
	_hysteresis_limit = 10
	_initialized = true
