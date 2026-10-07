extends Control
## Runs the source-layout rows exported as a native PackedScene.
signal closed

const CREDITS_CONTENT := "res://generated/hype/shared/menu/credits/en/lines.tscn"
const SOURCE_SCALE := 640.0 / 9.3

## FUN_0040eb10 multiplies timer ticks by DAT_005e6c78. The supplied
## decompilation omits that global's numeric initializer; keep this adapter
## scale editable instead of baking a guessed value into the imported source.
@export_range(0.1, 3.0, 0.05) var scroll_world_units_per_second := 1.0

var _content: Control
var _elapsed := 0.0
var _scroll_distance := 0.0
var _did_close := false

func _ready() -> void:
	if not ResourceLoader.exists(CREDITS_CONTENT):
		$Canvas/MissingData.show()
		push_error("Original English credits scene is missing: " + CREDITS_CONTENT)
		$Canvas/Back.grab_focus()
		return
	var packed := load(CREDITS_CONTENT) as PackedScene
	_content = packed.instantiate() as Control if packed != null else null
	if _content == null:
		$Canvas/MissingData.show()
		push_error("Original English credits scene could not be instantiated: " + CREDITS_CONTENT)
		$Canvas/Back.grab_focus()
		return
	$Canvas/Clip.add_child(_content)
	_scroll_distance = float(_content.get_meta("source_scroll_distance", 0.0))
	$Canvas/Back.grab_focus()
	set_process(_scroll_distance > 0.0)

func focus_initial() -> void:
	$Canvas/Back.grab_focus()

func _process(delta: float) -> void:
	if _content == null or _did_close:
		return
	_elapsed += delta
	_content.position.y = -_elapsed * scroll_world_units_per_second * SOURCE_SCALE
	if _elapsed * scroll_world_units_per_second >= _scroll_distance:
		_finish()

func _on_back_pressed() -> void:
	_finish()

func _unhandled_input(event: InputEvent) -> void:
	var key := event as InputEventKey
	if key == null or not key.pressed or key.echo or key.physical_keycode != KEY_ESCAPE:
		return
	get_viewport().set_input_as_handled()
	_finish()

func _finish() -> void:
	if _did_close:
		return
	_did_close = true
	closed.emit()
