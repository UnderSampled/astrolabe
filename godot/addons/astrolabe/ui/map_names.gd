extends Node
## Proc376 / 00416310 updates every matching entry in the source name table.
@export var entries: Array[Resource] = []

func activate_name(name_id: int, value: int) -> void:
    var identity := name_id & 4294967295
    if identity >= 2147483648: identity -= 4294967296
    for entry in entries:
        if entry.name_id == identity:
            entry.active = (value & 255) != 0
            entry.emit_changed()
