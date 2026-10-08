extends Resource
## StandardGame life state. Each instantiated actor owns its mutable copy.
@export var current: int = 0
@export var maximum: int = 0
@export var limit: int = 0

## Proc57 / 00438ef0 writes only the low byte. It does not cap at limit,
## change current life, or refill: those are separate source API operations.
func set_maximum(value: int) -> void:
	maximum = value & 255
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
