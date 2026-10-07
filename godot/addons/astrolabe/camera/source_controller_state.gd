extends Resource
## Process-wide state used by source camera controllers during one game session.
## The source counters are global, so every exported SourceCamera shares this
## object and the transition host keeps it when replacing a level instance.

var _initialized := false
var _hysteresis_limit := 0
var _fallback_hysteresis := 0 # 005200a0 DAT_00721974
var _distance_hysteresis := 0 # 005200a0 DAT_007219a4
var _collision_streak := 0 # 0051f670 / 0051f290 DAT_0072198c

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
