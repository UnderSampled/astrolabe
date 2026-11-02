extends Resource
## StandardGame life state. Each instantiated actor owns its mutable copy.
const SNAPSHOT = preload("res://addons/astrolabe/character/snapshot_values.gd")
@export var current: int = 0
@export var maximum: int = 0
@export var limit: int = 0

## Proc57 / 00438ef0 writes only the low byte. It does not cap at limit,
## change current life, or refill: those are separate source API operations.
func set_maximum(value: int) -> void:
	maximum = value & 255
	emit_changed()

## Proc52 / 00438ed0 writes the low byte, with no clamp to the maximum.
func set_points(value: int) -> void:
	current = value & 255
	emit_changed()

## Proc55 / 00460a90 narrows the operand first, then clamps the sum.
func add_points(value: int) -> void:
	current = mini(current + (value & 255), maximum)
	emit_changed()

## Function224 accepts the existing byte-valued HP state before mutating armor.
func can_receive_damage() -> bool:
	return current >= 0 and current <= 255

## 00460b10 subtracts effective byte damage and returns the remaining HP byte.
func subtract_damage(value: int) -> int:
	assert(can_receive_damage() and value >= 0 and value <= 255)
	current = maxi(current - value, 0)
	emit_changed()
	return current

## Proc_SubHitPoints / Func_SubAndGetHitPoints consume only the amount's low
## byte. Reaching zero does not call PlayerIsDead: the source caller decides.
func subtract_points(value: int) -> int:
	return subtract_damage(value & 255)

## Checkpoints preserve the three independent byte values. Source setters may
## lower maximum below current or raise it above limit; do not invent clamping.
func capture_snapshot() -> Dictionary:
	return {"version": 1, "current": current, "maximum": maximum, "limit": limit}

static func validate_snapshot_shape(value: Variant) -> String:
	if not SNAPSHOT.keys(value, ["version", "current", "maximum", "limit"]) or not SNAPSHOT.integer(value["version"], 1, 1):
		return "Life snapshot version or fields are unsupported."
	for field in ["current", "maximum", "limit"]:
		if not SNAPSHOT.integer(value[field], 0, 255):
			return "Life snapshot values are outside the source byte range."
	return ""

func apply_snapshot(value: Dictionary) -> String:
	var error := validate_snapshot_shape(value)
	if not error.is_empty(): return error
	current = int(value["current"])
	maximum = int(value["maximum"])
	limit = int(value["limit"])
	emit_changed()
	return ""
