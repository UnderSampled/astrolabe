extends Resource
## Level-local shared state selected by Hype's Fight_ChangeArmor procedure.
## Armor definitions, resistance, and HUD artwork are separate data slices.

signal selection_changed(index: int)

## Compatibility facade retained for compiled AI scripts. The catalog owns the
## actual Fix-selected definition and its mutable piece/resistance state.
@export var catalog: Resource
@export var selected_index: int = -1

## The source handler passes its operand through an unsigned byte parameter.
func select_source_index(value: int) -> bool:
	var selected := value & 0xff
	if is_instance_valid(catalog) and catalog.has_method("select"):
		catalog.call("select", selected)
		selected = int(catalog.get("selected"))
	if selected_index == selected:
		return true
	selected_index = selected
	selection_changed.emit(selected_index)
	emit_changed()
	return true

func selected_definition() -> Resource:
	if not is_instance_valid(catalog) or not catalog.has_method("active"):
		return null
	return catalog.call("active") as Resource

func set_resistance(value: int) -> bool:
	if not is_instance_valid(catalog) or not catalog.has_method("set_resistance"):
		return false
	catalog.call("set_resistance", value)
	selected_index = int(catalog.get("selected"))
	return true

func repair() -> bool:
	if not is_instance_valid(catalog) or not catalog.has_method("repair"):
		return false
	catalog.call("repair")
	return true

func resistance() -> int:
	if not is_instance_valid(catalog) or not catalog.has_method("resistance"):
		return -1
	return int(catalog.call("resistance"))

func definition_count() -> int:
	if not is_instance_valid(catalog):
		return 0
	var definitions: Variant = catalog.get("definitions")
	return definitions.size() if definitions is Array else 0

## The level owns this facade and its catalogue. Native checkpoint restore must
## update both so compiled AI and selection observers see the same definition.
func restore_snapshot(value: Dictionary) -> String:
	if not is_instance_valid(catalog) or not catalog.has_method("apply_snapshot"):
		return "Converted armor facade has no native catalogue."
	var error: String = catalog.call("apply_snapshot", value)
	if not error.is_empty(): return error
	var restored: int = int(catalog.get("selected"))
	if selected_index != restored:
		selected_index = restored
		selection_changed.emit(selected_index)
	emit_changed()
	return ""

func apply_damage(value: int) -> int:
	return damage(value)

func damage(value: int) -> int:
	if not is_instance_valid(catalog) or not catalog.has_method("damage"):
		return value & 0xff
	selected_index = int(catalog.get("selected"))
	return int(catalog.call("damage", value))
