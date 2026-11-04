extends Control
## The original Controls page (Fix 18:0 `Menu/control_back.bmp` root).
## Behavior follows FUN_00413240/FUN_004123b0/FUN_00414b00 as traced in
## notes/research/controls-menu-original.md: keyboard-only menu keys, Enter
## opens a tab's list or arms a row, the next named input is bound (swapping
## with the entry that held it), and Escape steps back.
signal closed

@export var bindings: Resource
@export var canvas_size := Vector2(640, 480)
## SelectBox centre relative to the focused row's anchor.
@export var select_offset := 1.6

var preferences: Node
var gamepad := false
var list_open := false
var row := 0
var capturing := false
## DAT_006104b8: set by confirming a row; the first Escape only clears it.
var confirm_armed := false

func _ready() -> void:
	# Keyboard-only page: hold focus so hidden Configure buttons do not.
	focus_mode = Control.FOCUS_ALL
	resized.connect(_fit)
	_fit()
	preferences = get_node_or_null("/root/UserSettings")
	_refresh()

func _fit() -> void:
	var factor := minf(size.x / canvas_size.x, size.y / canvas_size.y)
	$Canvas.scale = Vector2.ONE * factor
	$Canvas.position = (size - canvas_size * factor) * 0.5

func focus_initial() -> void:
	grab_focus()

func _list() -> Control:
	return $Canvas/GamepadList if gamepad else $Canvas/KeyboardList

func _rows() -> Array[Node]:
	return _list().get_children()

func _focused_row() -> Control:
	return _rows()[row] as Control

## True while this page owns the Escape key (for hosts' cancel checks).
func cancel_input_capture() -> bool:
	return visible

func _input(event: InputEvent) -> void:
	if not is_visible_in_tree():
		return
	if event is InputEventKey or event is InputEventJoypadButton or event is InputEventJoypadMotion:
		get_viewport().set_input_as_handled()
	else:
		return
	var key := event as InputEventKey
	if key != null and key.pressed and key.physical_keycode == KEY_ESCAPE and not key.echo:
		_back()
		return
	if capturing:
		_capture(event)
		return
	if _menu(event, "up"):
		_move(-1)
	elif _menu(event, "down"):
		_move(1)
	elif _menu(event, "left") or _menu(event, "right"):
		if not list_open:
			gamepad = not gamepad
			_refresh()
	elif _menu(event, "select"):
		_select()

## Fixed menu entries: arrows (held arrows repeat), Enter, keypad Enter or
## Space without Left Alt, pad button 0 and the pad directions.
func _menu(event: InputEvent, name: String) -> bool:
	var key := event as InputEventKey
	if key != null:
		if not key.pressed:
			return false
		match name:
			"up": return key.physical_keycode == KEY_UP
			"down": return key.physical_keycode == KEY_DOWN
			"left": return key.physical_keycode == KEY_LEFT and not key.echo
			"right": return key.physical_keycode == KEY_RIGHT and not key.echo
			"select": return not key.echo and not key.alt_pressed and [KEY_ENTER, KEY_KP_ENTER, KEY_SPACE].has(key.physical_keycode)
		return false
	var button := event as InputEventJoypadButton
	if button == null or not button.pressed:
		return false
	match name:
		"up": return button.button_index == JOY_BUTTON_DPAD_UP
		"down": return button.button_index == JOY_BUTTON_DPAD_DOWN
		"left": return button.button_index == JOY_BUTTON_DPAD_LEFT
		"right": return button.button_index == JOY_BUTTON_DPAD_RIGHT
		"select": return button.button_index == JOY_BUTTON_A
	return false

func _move(step: int) -> void:
	if not list_open:
		return
	var count := _rows().size()
	row = (row + step + count) % count
	_refresh()

func _select() -> void:
	if not list_open:
		list_open = true
		row = 0
		capturing = false
		_refresh()
		return
	confirm_armed = true
	match str(_focused_row().get_meta("source_action")):
		"KeyStrokes":
			# The pad list aborts at once when no pad is present.
			capturing = not gamepad or not Input.get_connected_joypads().is_empty()
		"DefaultValues":
			confirm_armed = false
			if preferences != null:
				preferences.call("reset_control_bindings")
	_refresh()

func _back() -> void:
	if list_open and confirm_armed:
		confirm_armed = false
		return
	if list_open:
		# Closing the list abandons a pending capture; the old input stays.
		list_open = false
		capturing = false
		_refresh()
		return
	closed.emit()

func _capture(event: InputEvent) -> void:
	if preferences == null:
		return
	var input := -1
	var key := event as InputEventKey
	if not gamepad and key != null and key.pressed and not key.echo:
		input = int(preferences.call("key_id", key))
		if not bool(preferences.call("can_bind_key", input)):
			return
	var button := event as InputEventJoypadButton
	if gamepad and button != null and button.pressed:
		input = button.button_index
	if input < 0:
		return
	var entry := str(_focused_row().get_meta("source_entry"))
	var result: Dictionary = preferences.call("bind_entry", entry, input)
	if bool(result.get("ok", false)):
		capturing = false
		confirm_armed = false
		_refresh()

func _refresh() -> void:
	# FUN_00411780: the focused tab selects the list and picture at once.
	$Canvas/DisplayWithKeyboard.visible = not gamepad
	$Canvas/DisplayWithGamepad.visible = gamepad
	$Canvas/TabKeyboard/Artwork.visible = gamepad
	$Canvas/TabKeyboard/SelectedArtwork.visible = not gamepad
	$Canvas/TabGamepad/Artwork.visible = not gamepad
	$Canvas/TabGamepad/SelectedArtwork.visible = gamepad
	$Canvas/GamepadList.visible = gamepad
	# DefaultValues items have no entry, so the keyboard list's reset label
	# is drawn in both modes; the pad list's own label is blank.
	$Canvas/KeyboardList.visible = true
	for item in $Canvas/KeyboardList.get_children():
		(item as Control).visible = not gamepad or str(item.get_meta("source_action")) == "DefaultValues"
	var first := ($Canvas/KeyboardList.get_child(0) as Control).get_meta("source_anchor") as Vector2
	for list in [$Canvas/KeyboardList, $Canvas/GamepadList]:
		for index in list.get_child_count():
			var item := list.get_child(index) as Control
			var input := item.get_node_or_null("Input") as TextureRect
			if input == null:
				continue
			var art: Dictionary = {}
			if capturing and list == _list() and index == row:
				art = bindings.capture
			elif preferences != null:
				art = preferences.call("get_entry_input_name", str(item.get_meta("source_entry")))
			input.visible = not art.is_empty()
			if art.is_empty():
				continue
			var rect: Rect2 = art["rect"]
			var anchor := item.get_meta("source_anchor") as Vector2
			input.texture = load(str(art["texture"])) as Texture2D
			input.position = rect.position + Vector2(0.0, anchor.y - first.y)
			input.size = rect.size
	var box := $Canvas/SelectBox as TextureRect
	box.visible = list_open
	if list_open:
		var anchor := _focused_row().get_meta("source_anchor") as Vector2
		box.position.y = anchor.y + select_offset - box.size.y * 0.5
