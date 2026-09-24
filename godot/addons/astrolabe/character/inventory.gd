extends Resource
## Native shared inventory state consumed by compiled source AI functions.
## Item IDs and quantities are useful gameplay values, not source memory layout.

@export var selected_weapon: int = -1
@export var quantities: Dictionary = {}

func quantity(item: int) -> int:
    if not quantities.has(item): return -1
    return int(quantities[item])

func remove(item: int, count: int) -> int:
    assert(count >= 0)
    var available := quantity(item)
    if available < 0: return -count
    var remaining := available - count
    if remaining >= 0: quantities[item] = remaining
    return remaining
