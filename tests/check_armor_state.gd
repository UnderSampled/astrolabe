extends SceneTree

const ArmorState = preload("res://addons/astrolabe/character/armor_state.gd")
const Behavior = preload("res://addons/astrolabe/behavior/behavior.gd")

func _initialize() -> void:
	var source_level := Node3D.new()
	var source_state = ArmorState.new()
	source_state.resource_local_to_scene = true
	source_level.set_meta("source_armor_state", source_state)
	for suffix in ["First", "Second"]:
		var actor := Node3D.new()
		actor.name = suffix + "Actor"
		source_level.add_child(actor)
		actor.owner = source_level
		var brain: Node3D = Behavior.new()
		brain.name = suffix + "Brain"
		brain.set("automatic", false)
		brain.set("main_actor_path", NodePath("../" + actor.name))
		source_level.add_child(brain)
		brain.owner = source_level
	var packed := PackedScene.new()
	if packed.pack(source_level) != OK:
		push_error("could not pack level-local armor state fixture")
		quit(1)
		return
	source_level.free()
	var first_level := packed.instantiate()
	var second_level := packed.instantiate()
	root.add_child(first_level)
	root.add_child(second_level)
	var first_brain: Variant = first_level.get_node("FirstBrain")
	var second_brain: Variant = first_level.get_node("SecondBrain")
	var first_state = first_level.get_meta("source_armor_state")
	var second_state = second_level.get_meta("source_armor_state")
	var selections: Array[int] = []
	var listener := func(index: int) -> void: selections.append(index)
	first_state.selection_changed.connect(listener)
	if first_brain.source_armor_state() != second_brain.source_armor_state():
		push_error("actors in the same level must resolve the same armor state")
		quit(1)
		return
	if first_state == second_state:
		push_error("separate level instances must own separate armor state")
		quit(1)
		return
	if first_state.selected_index != -1:
		push_error("armor selection should start unresolved")
		quit(1)
		return
	if (
		not first_state.select_source_index(0)
		or not first_state.select_source_index(2)
		or not first_state.select_source_index(2)
	):
		push_error("source armor selection failed")
		quit(1)
		return
	if (
		first_state.selected_index != 2
		or second_state.selected_index != -1
		or selections != [0, 2]
	):
		push_error("armor selection state/signal differs: %s / %s" % [first_state.selected_index, selections])
		quit(1)
		return
	first_state.selection_changed.disconnect(listener)
	var select_result: bool = first_brain.source_select_armor(258)
	var first_resolved = first_brain.source_armor_state()
	var second_resolved = second_brain.source_armor_state()
	if (
		not select_result
		or not is_instance_valid(first_resolved)
		or not is_instance_valid(second_resolved)
		or second_resolved.selected_index != 2
		or second_state.selected_index != -1
	):
		push_error("shared armor selection failed: %s / %s / %s / %s / %s" % [
			select_result,
			first_resolved,
			second_resolved,
			first_state.selected_index,
			second_state.selected_index,
		])
		quit(1)
		return
	first_level.free()
	second_level.free()
	print("SOURCE_ARMOR_STATE_OK")
	quit(0)
