extends SceneTree
## Source question items (DLG answers, 0056dd50/0056f3a0/0056e3b0) on the
## generated Brigand Wellet dialogues: answers follow the last body row, scroll
## wraps only on that page, Select chooses, gold is checked and spent, AnswerBad
## and AnswerNext continue in the same box, and the chosen callback runs after
## the close. Button_Hit never reaches gameplay while a dialogue is active.

const SCREEN := preload("res://addons/astrolabe/ui/screen.tscn")
const DIR := "res://generated/hype/levels/brigand/dialogue/en/"
const NAMES := ["wellet_intro", "wellet_explicationcourse", "wellet_refus", "wellet_plastyksinsuffisants"]

class BrainProbe extends Node:
	var source_model_path := ""
	var current_normal := 0
	var changes: Array[int] = []
	func change_normal(target: int, _defer_entry: bool = false) -> bool:
		changes.append(target)
		current_normal = target
		return true

class EmitterProbe extends Node3D:
	signal voice_started(info: Dictionary)
	signal voice_completed(info: Dictionary)
	var source_dialogue_record: Resource
	var played: Array[String] = []
	func play_dialogue_speech(record: Resource) -> void:
		source_dialogue_record = record
		played.append(str(record.get("identity")) if record.get("identity") != null else "answer")
	func stop_dialogue_speech() -> void:
		source_dialogue_record = null

class InventoryProbe extends Resource:
	var gold := 0
	func add_gold(value: int) -> int:
		gold += value
		return 1
	func subtract_gold(value: int) -> int:
		if gold < value: return 0
		gold -= value
		return 1

class HypeProbe extends Node3D:
	var source_inventory := InventoryProbe.new()

class GameplayInputProbe extends Node:
	var hit_count := 0
	func _unhandled_input(event: InputEvent) -> void:
		if event.is_action_pressed(&"Button_Hit"): hit_count += 1

var level: Node3D
var screen: Node
var dialogue: Control
var brain: BrainProbe
var hype: HypeProbe
var gameplay: GameplayInputProbe

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	TranslationServer.set_locale("en")
	if not InputMap.has_action(&"Button_Hit"): InputMap.add_action(&"Button_Hit")
	var binding := InputEventKey.new()
	binding.physical_keycode = KEY_SPACE
	if not InputMap.action_has_event(&"Button_Hit", binding): InputMap.action_add_event(&"Button_Hit", binding)
	var items: Array[Resource] = []
	for name in NAMES:
		var value: Resource = load(DIR + name + ".tres")
		if not expect(value != null, "missing generated " + name): return
		items.append(value)
	var intro: Resource = items[0]
	if not expect(intro.answers.size() == 2 and intro.box == 1 and intro.presentation.bullet != null, "wellet_intro lacks its two answers or bullet"): return
	if not expect(intro.answers[0].gold == -100 and intro.answers[0].next == "wellet_explicationcourse" and intro.answers[0].shortfall_item(1) == "wellet_plastyksinsuffisants" and intro.answers[1].next == "wellet_refus", "wellet_intro answer links differ"): return

	level = Node3D.new()
	level.name = "Brigand"
	root.add_child(level)
	var wellet := EmitterProbe.new()
	wellet.name = "Wellet"
	level.add_child(wellet)
	var emitter := EmitterProbe.new()
	emitter.name = "SoundEmitter"
	wellet.add_child(emitter)
	brain = BrainProbe.new()
	brain.name = "Brain"
	brain.source_model_path = items[1].completion_model
	wellet.add_child(brain)
	hype = HypeProbe.new()
	hype.name = "Hype"
	level.add_child(hype)
	var hype_emitter := EmitterProbe.new()
	hype_emitter.name = "SoundEmitter"
	hype.add_child(hype_emitter)
	level.set_meta("source_actor_nodes", {intro.owner_identity: NodePath("Wellet"), "hype": NodePath("Hype")})
	level.set_meta("source_main_actor", "hype")
	gameplay = GameplayInputProbe.new()
	level.add_child(gameplay)
	screen = SCREEN.instantiate()
	level.add_child(screen)
	await process_frame
	screen.set("dialogue_items", items)
	dialogue = screen.get_node("Canvas/Dialogue")

	# No: answers after the question row, wrapping scroll, then wellet_refus.
	if not await open_to_answers(intro): return
	screen.call("source_scroll_dialog", 1)
	screen.call("source_scroll_dialog", 1)
	if not expect(int(dialogue.get("selected_answer")) == 0, "scroll did not wrap forward"): return
	screen.call("source_scroll_dialog", -1)
	if not expect(int(dialogue.get("selected_answer")) == 1, "scroll did not wrap back to No"): return
	if not expect(bullet_rows() == [2], "bullet is not beside the No row"): return
	await press_space()
	if not expect(identity() == "wellet_refus" and int(dialogue.get("page_start")) == 0 and not bool(dialogue.get("closing")), "No did not continue with wellet_refus in the same box"): return
	if not expect(dialogue.get_node("Panel").modulate.a < 0.5 and not dialogue.get_node("SourceText").visible, "the continued item did not reopen with the source fade"): return
	if not await close_and_handoff(-1 if items[2].completion_normal < 0 else items[2].completion_normal): return

	# Yes without the 100 plastyks: AnswerBad, gold untouched, no callback.
	hype.source_inventory.gold = 50
	if not await open_to_answers(intro): return
	await press_space()
	if not expect(identity() == "wellet_plastyksinsuffisants" and hype.source_inventory.gold == 50, "shortfall did not show wellet_plastyksinsuffisants"): return
	if not await close_and_handoff(items[3].completion_normal): return

	# Yes with enough: gold spent, explanation, then No starts the race (normal 1).
	hype.source_inventory.gold = 150
	if not await open_to_answers(intro): return
	await press_space()
	if not expect(identity() == "wellet_explicationcourse" and hype.source_inventory.gold == 50, "Yes did not spend 100 and explain the race"): return
	await settle()
	while not bool(dialogue.get("answer_mode")):
		await press_space()
		await settle()
	screen.call("source_scroll_dialog", 1)
	await press_space()
	if not await close_and_handoff(1): return
	if not expect(gameplay.hit_count == 0, "Button_Hit reached gameplay during a dialogue"): return
	level.free()
	print("Dialogue answers passed")
	quit()

func open_to_answers(intro: Resource) -> bool:
	if not expect(bool(screen.call("source_run_dialog", intro.identity, 1, null)), "screen refused wellet_intro"): return false
	await settle()
	var pages := 0
	while not bool(dialogue.get("answer_mode")) and pages < 10:
		screen.call("source_scroll_dialog", 1)
		if not expect(int(dialogue.get("selected_answer")) == 0, "scroll acted on a body page"): return false
		await press_space()
		await settle()
		pages += 1
	if not expect(bool(dialogue.get("answer_mode")) and int(dialogue.get("page_start")) == 24, "wellet_intro did not reach its question page"): return false
	if not expect(not dialogue.get_node("Continue").visible, "continuation arrow shown on the answer page"): return false
	if not expect(bullet_rows() == [1] and answer_labels() == ["Yes", "No"], "answers are not on the rows after the question"): return false
	return true

func close_and_handoff(normal: int) -> bool:
	var guard := 0
	while not bool(dialogue.get("closing")) and dialogue.get("item") != null and guard < 10:
		await press_space()
		await settle()
		guard += 1
	var deadline := Time.get_ticks_msec() + 2000
	while not bool(screen.call("source_dialog_is_over")) and Time.get_ticks_msec() < deadline: await process_frame
	if not expect(bool(screen.call("source_dialog_is_over")), "dialogue did not close"): return false
	var before := brain.changes.size()
	screen.call("source_change_dialog_behavior")
	await process_frame
	await process_frame
	var expected: Array = [] if normal < 0 else [normal]
	return expect(Array(brain.changes.slice(before)) == expected, "callback %s, expected %s" % [brain.changes.slice(before), expected])

func identity() -> String:
	var value: Resource = dialogue.get("item")
	return str(value.identity) if value != null else ""

func bullet_rows() -> Array:
	var rows := []
	var style: Resource = dialogue.get("item").presentation
	for child in dialogue.get_node("SourceText").get_children():
		if child is TextureRect and child.texture == style.bullet:
			rows.append(int(round((child.position.y - 35.0) / 20.5)))
	return rows

func answer_labels() -> Array:
	var style: Resource = dialogue.get("item").presentation
	var x: float = style.text_origin(dialogue.position, 1).x + style.answer_indent()
	var labels := []
	for child in dialogue.get_node("SourceText").get_children():
		if child is Label and is_equal_approx(child.position.x, x) and not labels.has(child.text): labels.append(child.text)
	return labels

func settle() -> void:
	for i in 30: await process_frame

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
