extends Resource
## Level-local shared state selected by Hype's Fight_ChangeArmor procedure.
## Armor definitions, resistance, and HUD artwork are separate data slices.

signal selection_changed(index: int)

@export var selected_index: int = -1

## The source handler passes its operand through an unsigned byte parameter.
func select_source_index(value: int) -> bool:
	var selected := value & 0xff
	if selected_index == selected:
		return true
	selected_index = selected
	selection_changed.emit(selected_index)
	emit_changed()
	return true
