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
	# JSON.parse_string returns floats even for integral serialized values.
	# Check finiteness/range before converting so huge numbers cannot wrap.
	if not (value is int or value is float):
		return false
	var number := float(value)
	if not is_finite(number) or number < float(minimum) or number > float(maximum):
		return false
	return float(int(value)) == number

static func item_key(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING:
		return false
	var key := String(value)
	return key.length() <= 6 and key.is_valid_int() and str(int(key)) == key and integer(int(key), -1, 32767)
