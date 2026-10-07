extends SceneTree

const ArmorState = preload("res://addons/astrolabe/character/armor_state.gd")

func _initialize() -> void:
	var state = ArmorState.new()
	var selections: Array[int] = []
	var listener := func(index: int) -> void: selections.append(index)
	state.selection_changed.connect(listener)
	if state.selected_index != -1:
		push_error("armor selection should start unresolved")
		quit(1)
		return
	if not state.select_source_index(0) or not state.select_source_index(2) or not state.select_source_index(2):
		push_error("source armor selection failed")
		quit(1)
		return
	if state.selected_index != 2 or selections != [0, 2]:
		push_error("armor selection state/signal differs: %s / %s" % [state.selected_index, selections])
		quit(1)
		return
	state.selection_changed.disconnect(listener)
	state.free()
	print("SOURCE_ARMOR_STATE_OK")
	quit(0)
