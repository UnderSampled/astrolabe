extends Resource
## Shared gameplay permutation of already converted native payloads.
@export var source_id: String
@export var payload_order: PackedInt32Array = PackedInt32Array()
@export var has_reset_payloads: bool = false
@export var restored: bool = false

func swap_slots(first: int, second: int) -> bool:
    # 00541178 subtracts one; 00461860 narrows each result to unsigned short.
    var a: int = (first - 1) & 65535
    var b: int = (second - 1) & 65535
    if a >= payload_order.size() or b >= payload_order.size(): return false
    var previous: int = payload_order[a]
    payload_order[a] = payload_order[b]
    payload_order[b] = previous
    emit_changed()
    return true

func restore() -> bool:
    if not has_reset_payloads: return false
    for slot in payload_order.size(): payload_order[slot] = slot
    restored = true
    emit_changed()
    return true
