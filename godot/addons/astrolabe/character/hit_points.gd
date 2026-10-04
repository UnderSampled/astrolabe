extends Resource
## StandardGame life state. Each instantiated actor owns its mutable copy.
@export var current: int = 0
@export var maximum: int = 0
@export var limit: int = 0

func can_receive_damage() -> bool:
    return current >= 0 and current <= 255

## 00460b10 subtracts a byte and saturates at zero; returns remaining life.
func subtract_damage(value: int) -> int:
    if not can_receive_damage(): return -1
    current = maxi(0, current - (value & 255))
    emit_changed()
    return current
