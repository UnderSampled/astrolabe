extends SceneTree
## The monastery plaque (Brigand) is a framed item without speech: 0056cc70
## reveals it one letter per draw, its centred title re-centres on the drawn
## prefix, the arrow waits for a complete page, a press during the reveal
## completes the page (0056e510) and the next press advances.
## Optional user argument: a directory for screenshots (needs a display).

const SCREEN := preload("res://addons/astrolabe/ui/screen.tscn")
const PLAQUE := "res://generated/hype/levels/brigand/dialogue/en/brigand_monast_plaque_1.tres"

class BrainProbe extends Node:
	var source_model_path := ""
	var changes: Array[int] = []
	func change_normal(target: int, _defer_entry: bool = false) -> bool:
		changes.append(target)
		return true

var dialogue: Control
var shots := ""

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	TranslationServer.set_locale("en")
	var args := OS.get_cmdline_user_args()
	if not args.is_empty(): shots = args[0]
	if not InputMap.has_action(&"Button_Hit"): InputMap.add_action(&"Button_Hit")
	var binding := InputEventKey.new()
	binding.physical_keycode = KEY_SPACE
	if not InputMap.action_has_event(&"Button_Hit", binding): InputMap.action_add_event(&"Button_Hit", binding)
	var item: Resource = load(PLAQUE)
	if not expect(item != null and item.box == 1 and item.voice == null and item.lines[0].alignment == Vector2i(1, 0), "plaque resource lacks its centred, unvoiced framed lines"): return
	var level := Node3D.new()
	root.add_child(level)
	var owner := Node3D.new()
	owner.name = "Plaque"
	level.add_child(owner)
	var brain := BrainProbe.new()
	brain.name = "Brain"
	brain.source_model_path = item.completion_model
	owner.add_child(brain)
	level.set_meta("source_actor_nodes", {item.owner_identity: NodePath("Plaque")})
	var screen: Node = SCREEN.instantiate()
	level.add_child(screen)
	await process_frame
	screen.set("dialogue_items", [item] as Array[Resource])
	dialogue = screen.get_node("Canvas/Dialogue")
	screen.call("source_position_dialog", 305.0, 10.0)
	if not expect(bool(screen.call("source_run_dialog", item.identity, 1, null)), "screen refused the plaque"): return
	while not bool(dialogue.get("_source_opened")): await process_frame
	var title := label(0)
	var starts := []
	var shown := []
	for i in 12:
		await process_frame
		shown.append(title.visible_characters)
		starts.append(title.position.x)
	if not expect(shown.front() >= 1 and shown.back() > shown.front() and shown.back() < title.text.length() + 1, "title did not reveal letter by letter: %s" % [shown]): return
	if not expect(starts.back() < starts.front(), "centred title did not re-centre on its growing prefix"): return
	await shot("plaque_reveal")
	if not expect(not dialogue.get_node("Continue").visible and label(5).visible_characters == 0, "arrow or the last line shown before the page completed"): return
	await press_space()
	if not expect(label(1).visible_characters == -1 and int(dialogue.get("page_start")) == 0, "a press during the reveal did not complete the page"): return
	await settle()
	if not expect(dialogue.get_node("Continue").visible, "completed page lacks the continuation arrow"): return
	await shot("plaque_page1")
	await press_space()
	await settle()
	if not expect(int(dialogue.get("page_start")) == 6 and bool(dialogue.get("_reveal_active")), "second press did not advance to a revealing page 2"): return
	var guard := 0
	while bool(dialogue.get("_reveal_active")) and guard < 600:
		await process_frame
		guard += 1
	if not expect(not bool(dialogue.get("_reveal_active")) and label(0).visible_characters == -1, "page 2 reveal did not finish by itself"): return
	await shot("plaque_page2")
	await press_space()
	var deadline := Time.get_ticks_msec() + 2000
	while not bool(screen.call("source_dialog_is_over")) and Time.get_ticks_msec() < deadline: await process_frame
	if not expect(bool(screen.call("source_dialog_is_over")), "final press did not close the plaque"): return
	screen.call("source_change_dialog_behavior")
	await process_frame
	await process_frame
	if not expect(brain.changes == [item.completion_normal], "plaque callback %s" % [brain.changes]): return
	level.free()
	print("Dialogue plaque passed")
	quit()

func label(index: int) -> Label:
	var found := 0
	for child in dialogue.get_node("SourceText").get_children():
		if child is Label and child.has_meta("line_anchor"):
			if found == index: return child
			found += 1
	return null

func shot(name: String) -> void:
	if shots.is_empty(): return
	await process_frame
	RenderingServer.force_draw()
	root.get_texture().get_image().save_png(shots.path_join(name + ".png"))

func settle() -> void:
	for i in 3: await process_frame

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
	if condition: return true
	push_error(message)
	quit(1)
	return false
