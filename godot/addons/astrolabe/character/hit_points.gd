extends Resource
## StandardGame life state. Each instantiated actor owns its mutable copy.
@export var current: int = 0
@export var maximum: int = 0
@export var limit: int = 0

## Function224 accepts the existing byte-valued HP state before mutating armor.
func can_receive_damage() -> bool:
	return current >= 0 and current <= 255

## 00460b10 subtracts effective byte damage and returns the remaining HP byte.
func subtract_damage(value: int) -> int:
	assert(can_receive_damage() and value >= 0 and value <= 255)
	current = maxi(current - value, 0)
	emit_changed()
	return current
