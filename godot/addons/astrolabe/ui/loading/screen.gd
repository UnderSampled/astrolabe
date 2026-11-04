extends CanvasLayer
## Presents the currently observed Godot scene-resource load state.
## The LevelTransition service owns the transition and emits its lifecycle.
## A destination with a converted source vignette shows its era backdrop and
## bitmap bar on the source 640x480 canvas; others use the plain status group.

const TRANSITION_SERVICE_PATH := NodePath("/root/LevelTransition")
const VIGNETTE = preload("res://addons/astrolabe/ui/loading/vignette.gd")
const LEVEL_PREFIX := "res://generated/hype/levels/"
const VIGNETTE_FILE := "loading.tres"
const CANVAS_SIZE := Vector2(640, 480)

enum PresentationState { IDLE, LOADING, FAILED }

@onready var viewport: Control = $Viewport
@onready var canvas: Control = $Viewport/Canvas
@onready var backdrop: TextureRect = $Viewport/Canvas/Backdrop
@onready var bar: Control = $Viewport/Canvas/Bar
@onready var frame: TextureRect = $Viewport/Canvas/Bar/Frame
@onready var fill: Control = $Viewport/Canvas/Bar/Fill
@onready var fill_strip: TextureRect = $Viewport/Canvas/Bar/Fill/Strip
@onready var status: Control = $Viewport/Center/Content
@onready var title: Label = $Viewport/Center/Content/Title
@onready var phase_label: Label = $Viewport/Center/Content/Phase
@onready var progress: ProgressBar = $Viewport/Center/Content/Progress
@onready var error_message: Label = $Viewport/Center/Content/ErrorMessage
@onready var dismiss_button: Button = $Viewport/Center/Content/Dismiss

var _state := PresentationState.IDLE
var _active_level := ""
var _vignette: VIGNETTE

## Observable presentation state, independent of which layout is drawn.
var phase_text := ""
var progress_shown := false
var progress_value := 0.0


func _ready() -> void:
	visible = false
	viewport.resized.connect(_fit)
	_fit()
	_connect_to_transition_service()
	dismiss_button.pressed.connect(_dismiss_failure)


func _fit() -> void:
	# Same uniform 640x480 fit as the main menu; the surround stays black.
	var factor := minf(viewport.size.x / CANVAS_SIZE.x, viewport.size.y / CANVAS_SIZE.y)
	canvas.scale = Vector2.ONE * factor
	canvas.position = (viewport.size - CANVAS_SIZE * factor) * 0.5


func _connect_to_transition_service() -> void:
	var service := get_node_or_null(TRANSITION_SERVICE_PATH)
	if service == null:
		push_warning("Loading presentation has no /root/LevelTransition service to observe.")
		return

	_connect_service_signal(service, &"transition_started", _on_transition_started)
	_connect_service_signal(service, &"transition_progress", _on_transition_progress)
	_connect_service_signal(service, &"transition_completed", _on_transition_completed)
	_connect_service_signal(service, &"transition_failed", _on_transition_failed)
	_connect_service_signal(service, &"transition_cancelled", _on_transition_cancelled)


func _connect_service_signal(service: Node, signal_name: StringName, handler: Callable) -> void:
	if not service.has_signal(signal_name):
		push_error("LevelTransition is missing the %s signal." % signal_name)
		return
	if not service.is_connected(signal_name, handler):
		var error := service.connect(signal_name, handler)
		if error != OK:
			push_error("Could not connect the loading presentation to %s." % signal_name)


## The destination's converted vignette, or null when the level has none.
static func level_vignette(level: String) -> VIGNETTE:
	var path := LEVEL_PREFIX + level + "/" + VIGNETTE_FILE
	if not ResourceLoader.exists(path):
		return null
	return load(path) as VIGNETTE


func _apply_vignette(vignette: VIGNETTE) -> void:
	_vignette = vignette
	var art := vignette != null and vignette.backdrop != null
	canvas.visible = art
	status.visible = not art
	if not art:
		return
	backdrop.texture = vignette.backdrop
	bar.visible = vignette.bar != null
	if vignette.bar == null:
		return
	var rect := vignette.bar_rect
	bar.position = Vector2(rect.position)
	bar.size = Vector2(rect.size)
	frame.size = Vector2(rect.size)
	frame.texture = _strip(vignette.bar, vignette.frame_y, rect.size)
	fill.position = Vector2(vignette.fill_min_x, 0)
	fill.size = Vector2(0, rect.size.y)
	fill_strip.position = Vector2(-vignette.fill_min_x, 0)
	fill_strip.size = Vector2(rect.size)
	fill_strip.texture = _strip(vignette.bar, vignette.fill_y, rect.size)


func _strip(texture: Texture2D, y: int, size: Vector2i) -> AtlasTexture:
	var strip := AtlasTexture.new()
	strip.atlas = texture
	strip.region = Rect2(0, y, size.x, size.y)
	return strip


func _set_progress(value: float) -> void:
	progress_value = value
	progress.value = value
	if _vignette != null and _vignette.bar != null:
		fill.size.x = _vignette.fill_width(value)


## The session's status after resource loading. A source vignette has no
## in-frame text and keeps its bar as drawn; the status group shows the text.
func show_phase(message: String, show_progress: bool) -> void:
	phase_text = message
	phase_label.text = message
	progress_shown = show_progress
	progress.visible = show_progress


func _on_transition_started(_from_level: String, to_level: String, _entry: String) -> void:
	_active_level = to_level
	_state = PresentationState.LOADING
	_apply_vignette(level_vignette(to_level))
	title.text = "Loading map: %s" % to_level
	error_message.hide()
	dismiss_button.hide()
	title.show()
	phase_label.show()
	show_phase("Loading scene resources", true)
	_set_progress(0.0)
	viewport.show()
	show()


func _on_transition_progress(to_level: String, load_fraction: float, phase: String) -> void:
	if _state != PresentationState.LOADING or to_level != _active_level:
		return
	if not is_finite(load_fraction):
		push_warning("Ignoring non-finite level resource progress.")
		return
	show_phase("Loading scene resources" if phase == "resource_loading" else "Loading", progress_shown)
	_set_progress(clampf(load_fraction, 0.0, 1.0))


func _on_transition_completed(level: String, _entry: String) -> void:
	if _state == PresentationState.LOADING and level == _active_level:
		_hide_presentation()


func _on_transition_failed(level: String, message: String) -> void:
	if _state != PresentationState.LOADING or level != _active_level:
		return
	_state = PresentationState.FAILED
	title.text = "Could not load %s" % level
	title.show()
	phase_label.hide()
	progress.hide()
	progress_shown = false
	error_message.text = message
	error_message.show()
	dismiss_button.show()
	status.show()
	viewport.show()
	show()
	dismiss_button.grab_focus()


func _on_transition_cancelled(level: String) -> void:
	if _state == PresentationState.LOADING and level == _active_level:
		_hide_presentation()


func _dismiss_failure() -> void:
	if _state == PresentationState.FAILED:
		_hide_presentation()


func _hide_presentation() -> void:
	_state = PresentationState.IDLE
	_active_level = ""
	_vignette = null
	backdrop.texture = null
	error_message.text = ""
	phase_text = ""
	progress_shown = false
	phase_label.show()
	progress.show()
	hide()
