extends Control
## Explicit save identity selection for the actual main-menu Load Game route.
signal selected(source: String)
signal closed

@onready var rows: VBoxContainer = $Panel/Margin/Rows/Saves
@onready var back: Button = $Panel/Margin/Rows/Back


func show_candidates(candidates: Array[Dictionary]) -> void:
    for child in rows.get_children():
        rows.remove_child(child)
        child.queue_free()
    var initial: Button
    for candidate in candidates:
        var button := Button.new()
        button.text = str(candidate["label"])
        button.disabled = candidate["status"] != "loaded"
        button.pressed.connect(_on_save_pressed.bind(str(candidate["source"])))
        rows.add_child(button)
        if initial == null and not button.disabled: initial = button
        if button.disabled:
            var reason := Label.new()
            reason.text = str(candidate.get("message", "Save is unavailable."))
            reason.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
            rows.add_child(reason)
    if initial != null: initial.grab_focus()
    else: back.grab_focus()


func _on_back_pressed() -> void:
    closed.emit()


func _on_save_pressed(source: String) -> void:
    selected.emit(source)


func _input(event: InputEvent) -> void:
    if event.is_action_pressed("ui_cancel"):
        get_viewport().set_input_as_handled()
        closed.emit()
