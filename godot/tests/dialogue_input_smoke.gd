extends SceneTree
## The configured source Use / Attack action advances an active source dialog.

const SCREEN := preload("res://addons/astrolabe/ui/screen.tscn")
const ITEM_SCRIPT := preload("res://addons/astrolabe/ui/dialogue_item.gd")
const LINE_SCRIPT := preload("res://addons/astrolabe/ui/dialogue_line.gd")

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	if not InputMap.has_action("Button_Hit"):
		InputMap.add_action("Button_Hit")
	var binding := InputEventKey.new()
	binding.physical_keycode = KEY_SPACE
	if not InputMap.action_has_event("Button_Hit", binding):
		InputMap.action_add_event("Button_Hit", binding)

	var screen := SCREEN.instantiate()
	root.add_child(screen)
	await process_frame
	var item := ITEM_SCRIPT.new()
	item.identity = "dialogue_input_smoke"
	item.box = 1
	var line := LINE_SCRIPT.new()
	item.lines = [line]
	if not screen.dialogue.run(item, 1):
		fail("Test dialogue did not open")
		return
	await create_timer(0.2).timeout
	var press := InputEventKey.new()
	press.physical_keycode = KEY_SPACE
	press.pressed = true
	Input.parse_input_event(press)
	await process_frame
	var release := InputEventKey.new()
	release.physical_keycode = KEY_SPACE
	release.pressed = false
	Input.parse_input_event(release)
	await create_timer(0.2).timeout
	if not screen.dialogue.is_over():
		fail("Space did not select/finish the active source dialogue")
		return
	screen.free()
	print("Dialogue input smoke passed")
	quit()

func fail(message: String) -> void:
	push_error(message)
	quit(1)
