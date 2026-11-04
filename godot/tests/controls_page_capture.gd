extends SceneTree
## Renders the Options > Controls page states that the original captures show:
## godot --path godot --resolution 1024x768 --script res://tests/controls_page_capture.gd -- OUT_DIR
## Run with XDG_* pointing at a scratch directory; it rebinds and resets keys.

var out := ""
var settings: Control

func _initialize() -> void:
	call_deferred("run")

func key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		Input.parse_input_event(event)
		Input.flush_buffered_events()
		await process_frame

func shot(name: String) -> void:
	for i in 3:
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(out.path_join(name + ".png"))
	print("CAPTURED ", name)

func run() -> void:
	out = OS.get_cmdline_user_args()[0]
	root.size = Vector2i(1024, 768)
	settings = (load("res://game/settings_menu.tscn") as PackedScene).instantiate() as Control
	root.add_child(settings)
	settings.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	await process_frame
	await shot("port-configure")
	settings.call("open_controls")
	await shot("port-controls")
	await key(KEY_ENTER)
	await shot("port-list")
	await key(KEY_ENTER)
	await shot("port-capture")
	await key(KEY_X)
	await shot("port-swap")
	await key(KEY_UP)
	await key(KEY_ENTER)
	await shot("port-reset")
	await key(KEY_ESCAPE)
	await key(KEY_RIGHT)
	await shot("port-gamepad")
	await key(KEY_ESCAPE)
	await shot("port-back")
	quit(0)
