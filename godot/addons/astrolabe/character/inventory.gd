extends Resource
## Native shared inventory state consumed by compiled source AI functions.
## Item IDs and quantities are useful gameplay values, not source memory layout.
const SOURCE_CURRENCY_ITEM_ID := 0

@export var selected_weapon: int = -1
@export var quantities: Dictionary = {}

func quantity(item: int) -> int:
    if not quantities.has(item):
        # Hype's dedicated DAT_006107bc currency item is Fix ItemWire id 0 and
        # the source inventory reset path initializes its element to zero.
        return 0 if item == SOURCE_CURRENCY_ITEM_ID else -1
    return int(quantities[item])

func remove(item: int, count: int) -> int:
    assert(count >= 0)
    var available := quantity(item)
    if available < 0: return -count
    var remaining := available - count
    if remaining >= 0: quantities[item] = remaining
    return remaining
