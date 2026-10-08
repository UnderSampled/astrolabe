extends RefCounted
## Native absolute movement for Action_MoveLift. Coordinates are Godot world
## coordinates; the Character boundary converts captured source points once.
var revision: int = 0
var active: bool = false
var failure: String = ""
var position: Vector3 = Vector3.ZERO
var _from: Vector3
var _to: Vector3
var _length: float
var _direction: Vector3
var _half_duration: float
var _distance: float = 0.0
var _eased_distance: float = 0.0
var _phase_time: float = 0.0

func begin(from: Vector3, to: Vector3, duration: float, bound: bool) -> int:
    revision += 1
    active = false
    failure = ""
    # The source wrapper completes immediately when dynamics is unavailable.
    if not bound: return 0
    if not from.is_finite() or not to.is_finite() or not is_finite(duration) or duration <= 0.0:
        failure = "Lift motion requires finite points and a positive source duration."
        return -1
    _from = from
    _to = to
    var difference := _to - _from
    # Retain the source component order when calculating the square sum.
    var squared := float(difference.x) * float(difference.x) + float(difference.z) * float(difference.z) + float(difference.y) * float(difference.y)
    _length = _real(sqrt(squared))
    _direction = difference
    if squared != 0.0:
        var reciprocal := _real(1.0 / sqrt(_real(squared)))
        _direction *= reciprocal
    _half_duration = _real(duration * 0.5)
    if _half_duration <= 0.0:
        failure = "Lift duration is outside the supported positive float range."
        return -1
    _distance = 0.0
    _eased_distance = 0.0
    # The source start preserves the prior phase. For a nonzero segment the
    # first scalar update replaces it with half-duration before it is used.
    position = _from
    active = true
    return revision

func step(delta: float) -> void:
    if not active: return
    # Event 15 updates the exponential scalar even in the later linear phase.
    var rate := _real((_length - _eased_distance) / _half_duration)
    var increment := _real(rate * delta)
    _eased_distance = _real(increment + _eased_distance)
    if absf(_distance) < absf(_length * 0.6321):
        _phase_time = _half_duration
        _distance = _eased_distance
    else:
        _distance = _real(((_phase_time - _half_duration) / (_half_duration * 2.718) + 0.6321) * _length)
        _phase_time = _real(_phase_time + delta)
    position = _from + _direction * _distance
    # Unordered magnitudes follow the source completion branch as well.
    if not absf(_distance) < absf(_length):
        position = _to
        active = false

func finished(ticket: int) -> bool:
    return ticket <= 0 or ticket != revision or not active

func cancel(ticket: int) -> void:
    if ticket > 0 and ticket == revision: active = false

static func _real(value: float) -> float:
    return PackedFloat32Array([value])[0]
