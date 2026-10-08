extends SceneTree

const CHARACTER := preload("res://addons/astrolabe/character/character.gd")
const FIELDS := preload("res://ai_wait_integer_fields.gd")
const COMPILED := preload("res://compiled_wait_integer.gd")

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var level := Node3D.new()
	root.add_child(level)
	var character := CHARACTER.new()
	character.name = "Actor"
	level.add_child(character)
	var emitter := Node3D.new()
	emitter.name = "SoundEmitter"
	character.add_child(emitter)
	var global_actor := Node3D.new()
	global_actor.name = "Global"
	level.add_child(global_actor)
	var global_brain = FIELDS.new()
	global_brain.name = "Brain"
	global_actor.add_child(global_brain)
	var main_actor := Node3D.new()
	main_actor.name = "MainActor"
	level.add_child(main_actor)
	var main_brain = FIELDS.new()
	main_brain.name = "Brain"
	main_actor.add_child(main_brain)
	var brain = COMPILED.new()
	brain.automatic = false
	brain.current_normal = 0
	brain.main_actor_path = NodePath("../../MainActor")
	brain.actor_brain_0_path = NodePath("../../Global/Brain")
	character.add_child(brain)
	character.set_process(false)
	character.set_physics_process(false)
	brain.set_process(false)
	brain.set_physics_process(false)
	await process_frame

	var cases := [
		[5, 0, 5, true], [4, 0, 5, false],
		[4, 1, 5, true], [5, 1, 5, false],
		[4, 2, 5, true], [5, 2, 5, false],
		[6, 3, 5, true], [5, 3, 5, false],
		[5, 4, 5, true], [6, 4, 5, false],
		[5, 5, 5, true], [4, 5, 5, false],
		[4294967295, 2, 0, true], [2147483648, 0, -2147483648, true],
		[4, 6, 5, true], [4, 255, 5, true], [4, 256, 5, true],
	]
	for index in cases.size():
		var item: Array = cases[index]
		if not await _run_case(brain, global_brain, main_brain, item[0], item[1], item[2], item[3], "case %d" % index):
			return

	# Only the observed property is live; selector and expected are captured.
	if not await _run_case(brain, global_brain, main_brain, 4, 5, 5, false, "captured wait"):
		return
	main_brain.dsg_7 = 3
	brain.dsg_0 = 2
	brain.advance(0.016)
	await process_frame
	if not _check(brain.dsg_1 == 0, "wait reevaluated captured arguments"):
		return
	global_brain.dsg_5 = 5
	brain.advance(0.016)
	await process_frame
	if not _check(brain.dsg_1 == 1, "changed observed field did not release wait"):
		return
	if not await _run_case(brain, global_brain, main_brain, 4, 5, 5, false, "cancellation"):
		return
	brain.stop()
	await process_frame
	global_brain.dsg_5 = 5
	brain.advance(0.016)
	await process_frame
	if not _check(brain.dsg_1 == 0, "canceled wait resumed"):
		return

	print("AI_WAIT_INTEGER_OK")
	quit(0)

func _run_case(brain: Node, global_brain: Node, main_brain: Node, left: int, selector: int, right: int, expected: bool, label: String) -> bool:
	brain.stop()
	await process_frame
	global_brain.dsg_5 = left
	main_brain.dsg_7 = right
	brain.dsg_0 = selector
	brain.dsg_1 = 0
	if not brain.start():
		return _check(false, label + " failed to start")
	await process_frame
	if not _check((brain.dsg_1 == 1) == expected, label + " completion mismatch"):
		return false
	return true

func _check(condition: bool, label: String) -> bool:
	if not condition:
		push_error("AI_WAIT_INTEGER_FAILED: " + label)
		quit(1)
		return false
	return true
