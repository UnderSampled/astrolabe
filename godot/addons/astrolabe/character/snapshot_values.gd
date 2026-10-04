extends RefCounted
## Shared value guards for native Resource snapshots; no source save addresses.
static func integer(value: Variant, low: int, high: int) -> bool:
    if not (value is int or value is float): return false
    var number := float(value)
    if not is_finite(number) or number < float(low) or number > float(high): return false
    return float(int(value)) == number

static func keys(value: Variant, required: Array[String]) -> bool:
    if not value is Dictionary or value.size() != required.size(): return false
    for key in value:
        if not key is String or not required.has(key): return false
    return true

static func item_key(value: Variant) -> bool:
    if not value is String or value.length() > 6 or not value.is_valid_int(): return false
    return str(int(value)) == value and integer(int(value), -1, 32767)
