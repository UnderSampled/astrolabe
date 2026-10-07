extends Control
## Faithful black Credits route shell until source English text keys are imported.
signal closed

func _ready() -> void:
    $Content/Back.grab_focus()

func focus_initial() -> void:
    $Content/Back.grab_focus()

func _on_back_pressed() -> void:
    closed.emit()

func _unhandled_input(event: InputEvent) -> void:
    var key := event as InputEventKey
    if key == null or not key.pressed or key.echo or key.physical_keycode != KEY_ESCAPE:
        return
    get_viewport().set_input_as_handled()
    closed.emit()
