extends SceneTree
## Dialogue owns the hit action until its closing fade is over, then returns
## the next hit to gameplay and applies its pending behavior callback once.

const SCREEN := preload("res://addons/astrolabe/ui/screen.tscn")
const ITEM_SCRIPT := preload("res://addons/astrolabe/ui/dialogue_item.gd")
const LINE_SCRIPT := preload("res://addons/astrolabe/ui/dialogue_line.gd")

class BrainProbe extends Node:
	var source_model_path := "level:/ai/models/210"
	var current_normal := 1
	var changes: Array[int] = []

	func change_normal(target: int, _defer_entry: bool = false) -> bool:
		changes.append(target)
		current_normal = target
		return true

class GameplayInputProbe extends Node:
	var hit_count := 0

	func _unhandled_input(event: InputEvent) -> void:
		if event.is_action_pressed(&"Button_Hit"):
			hit_count += 1

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	TranslationServer.set_locale("en")
	if not InputMap.has_action(&"Button_Hit"):
		InputMap.add_action(&"Button_Hit")
	var binding := InputEventKey.new()
	binding.physical_keycode = KEY_SPACE
	if not InputMap.action_has_event(&"Button_Hit", binding):
		InputMap.action_add_event(&"Button_Hit", binding)

	var level := Node3D.new()
	level.name = "Manoir"
	root.add_child(level)
	var actor := Node3D.new()
	actor.name = "Gogoud"
	level.add_child(actor)
	var brain := BrainProbe.new()
	brain.name = "Brain"
	actor.add_child(brain)
	level.set_meta("source_actor_nodes", {"gogoud": NodePath("Gogoud")})

	# Add the gameplay receiver before the screen so the active screen can claim
	# Button_Hit during its own unhandled-input pass.
	var gameplay := GameplayInputProbe.new()
	level.add_child(gameplay)
	var item := ITEM_SCRIPT.new()
	item.identity = "dialogue_control_handoff"
	item.locale = "en"
	item.box = 1
	item.owner_identity = "gogoud"
	item.completion_model = brain.source_model_path
	item.completion_normal = 5
	var lines: Array[Resource] = []
	for _index in range(7):
		var line := LINE_SCRIPT.new()
		line.end_seconds = 3.4e38
		lines.append(line)
	item.lines = lines

	var screen: Node = SCREEN.instantiate()
	level.add_child(screen)
	await process_frame
	var items: Array[Resource] = [item]
	screen.set("dialogue_items", items)
	var dialogue: Node = screen.get_node("Canvas/Dialogue")
	dialogue.set("fade_seconds", 0.6)
	var events := {"completed": 0}
	dialogue.connect("completed", func(_finished: Resource) -> void: events["completed"] += 1)
	if not bool(screen.call("source_run_dialog", item.identity, 1, null)):
		fail("Screen refused the test dialogue")
		return
	await create_timer(0.65).timeout

	await press_space()
	if not expect(int(dialogue.get("page_start")) == 6, "First hit did not advance to the second page"):
		return
	await press_space()
	if not expect(bool(dialogue.get("closing")), "Final hit did not begin the close fade"):
		return
	if not expect(dialogue.get("item") == item, "Closing dialogue released input ownership before its fade ended"):
		return
	if not expect(int(events["completed"]) == 1 and gameplay.hit_count == 0, "Final hit was not consumed exactly once by the active dialogue"):
		return
	if not expect(not bool(screen.call("source_dialog_is_over")), "Dialogue reported over while its closing fade was visible"):
		return

	# The source behavior change waits for DLG_IsDialogOver, including its
	# closing state; it must not change Gogoud while the native fade is running.
	if not expect(bool(screen.call("source_change_dialog_behavior")), "Pending dialogue callback was rejected during the fade"):
		return
	if not expect(bool(screen.get("completion_pending")) and brain.current_normal == 1, "Behavior changed before the dialogue was over"):
		return

	# Another hit during the close fade is still owned by the dialogue, but its
	# closing guard prevents it from selecting or completing a second time.
	await press_space()
	if not expect(int(events["completed"]) == 1 and gameplay.hit_count == 0, "A closing-fade hit escaped to gameplay or completed twice"):
		return
	var deadline := Time.get_ticks_msec() + 1500
	while not bool(screen.call("source_dialog_is_over")) and Time.get_ticks_msec() < deadline:
		await process_frame
	if not expect(bool(screen.call("source_dialog_is_over")), "Dialogue close fade did not finish"):
		return
	if not expect(dialogue.get("item") == null, "Completed dialogue retained its input ownership"):
		return
	if not expect(bool(screen.call("source_change_dialog_behavior")), "Completed dialogue callback was rejected"):
		return
	await process_frame
	if not expect(brain.current_normal == 5 and brain.changes == [5], "Dialogue completion did not hand off its behavior exactly once"):
		return
	screen.call("source_change_dialog_behavior")
	await process_frame
	if not expect(brain.changes == [5], "Dialogue callback ran more than once"):
		return

	await press_space()
	if not expect(gameplay.hit_count == 1, "Button_Hit did not return to gameplay after the dialogue closed"):
		return
	level.free()
	print("Dialogue control handoff passed")
	quit()

func press_space() -> void:
	var press := InputEventKey.new()
	press.physical_keycode = KEY_SPACE
	press.pressed = true
	Input.parse_input_event(press)
	await process_frame
	var release := InputEventKey.new()
	release.physical_keycode = KEY_SPACE
	release.pressed = false
	Input.parse_input_event(release)
	await process_frame

func expect(condition: bool, message: String) -> bool:
	if condition:
		return true
	fail(message)
	return false

func fail(message: String) -> void:
	push_error(message)
	quit(1)
