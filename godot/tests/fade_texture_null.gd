extends SceneTree
## A missing source STE_FADE slot is an accepted no-op; it never invents a texture.

const SCREEN := preload("res://addons/astrolabe/ui/screen.tscn")

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var screen: Node = SCREEN.instantiate()
	root.add_child(screen)
	await process_frame
	var fade: ColorRect = screen.get_node("Canvas/Fade")
	var original_color := Color(0.2, 0.35, 0.6, 0.4)
	fade.color = original_color
	for index in [0, 9]:
		if not bool(screen.call("source_set_fade_texture", index)):
			fail("An empty but valid STE_FADE slot was rejected: %d" % index)
			return
	if bool(screen.call("source_set_fade_texture", -1)) or bool(screen.call("source_set_fade_texture", 10)):
		fail("An out-of-range STE_FADE slot was accepted")
		return
	if fade.color != original_color:
		fail("The empty-slot no-op changed the fade presentation")
		return
	print("Empty STE_FADE slot no-op passed")
	quit()

func fail(message: String) -> void:
	push_error(message)
	quit(1)
