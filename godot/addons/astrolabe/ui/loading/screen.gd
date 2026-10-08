extends CanvasLayer
## Presents the currently observed Godot scene-resource load state.
## The LevelTransition service owns the transition and emits its lifecycle.

const TRANSITION_SERVICE_PATH := NodePath("/root/LevelTransition")

enum PresentationState { IDLE, LOADING, FAILED }

@onready var viewport: Control = $Viewport
@onready var title: Label = $Viewport/Center/Content/Title
@onready var phase_label: Label = $Viewport/Center/Content/Phase
@onready var progress: ProgressBar = $Viewport/Center/Content/Progress
@onready var error_message: Label = $Viewport/Center/Content/ErrorMessage
@onready var dismiss_button: Button = $Viewport/Center/Content/Dismiss

var _state := PresentationState.IDLE
var _active_level := ""


func _ready() -> void:
	visible = false
	_connect_to_transition_service()
	dismiss_button.pressed.connect(_dismiss_failure)


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


func _on_transition_started(_from_level: String, to_level: String, _entry: String) -> void:
	_active_level = to_level
	_state = PresentationState.LOADING
	title.text = "Loading map: %s" % to_level
	phase_label.text = "Loading scene resources"
	error_message.hide()
	dismiss_button.hide()
	progress.show()
	progress.value = 0.0
	phase_label.show()
	viewport.show()
	show()


func _on_transition_progress(to_level: String, load_fraction: float, phase: String) -> void:
	if _state != PresentationState.LOADING or to_level != _active_level:
		return
	if not is_finite(load_fraction):
		push_warning("Ignoring non-finite level resource progress.")
		return
	if phase != "resource_loading":
		phase_label.text = "Loading"
	else:
		phase_label.text = "Loading scene resources"
	progress.value = clampf(load_fraction, 0.0, 1.0)


func _on_transition_completed(level: String, _entry: String) -> void:
	if _state == PresentationState.LOADING and level == _active_level:
		_hide_presentation()


func _on_transition_failed(level: String, message: String) -> void:
	if _state != PresentationState.LOADING or level != _active_level:
		return
	_state = PresentationState.FAILED
	title.text = "Could not load %s" % level
	phase_label.hide()
	progress.hide()
	error_message.text = message
	error_message.show()
	dismiss_button.show()
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
	error_message.text = ""
	phase_label.show()
	progress.show()
	hide()
