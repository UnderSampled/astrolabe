extends RefCounted
## Strict shape and integer checks shared by versioned native snapshots.

static func keys(value: Variant, expected: Array) -> bool:
	if typeof(value) != TYPE_DICTIONARY:
		return false
	var dictionary: Dictionary = value
	if dictionary.size() != expected.size():
		return false
	for key in expected:
		if not dictionary.has(key):
			return false
	return true

static func integer(value: Variant, minimum: int, maximum: int) -> bool:
	return typeof(value) == TYPE_INT and int(value) >= minimum and int(value) <= maximum

static func item_key(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING:
		return false
	var key := String(value)
	return not key.is_empty() and key.is_valid_int() and int(key) >= 0
