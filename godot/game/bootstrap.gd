extends Control
## Game main menu. A native slot loads when present; otherwise the supplied
## measured manor state starts a new local session.
const MANOR := "res://generated/hype/levels/manoir/level.tscn"
const SAVE_STATE = preload("res://game/save_state.gd")
var opening_game := false

func _ready() -> void:
    var panel := VBoxContainer.new()
    panel.position = Vector2(40, 40)
    add_child(panel)
    var heading := Label.new()
    heading.text = "Hype: The Time Quest"
    panel.add_child(heading)
    var details := Label.new()
    var slot: Dictionary = SAVE_STATE.read()
    details.text = "Generate the manor from your Hype copy to play. See godot/README.md."
    if ResourceLoader.exists(MANOR):
        details.text = "Continue from your local manor save." if slot["status"] == "loaded" else "Begin from the measured English manor state, then save your progress."
        if slot["status"] == "error":
            details.text = str(slot["message"])
    panel.add_child(details)
    var play := Button.new()
    play.text = "Load Game"
    play.disabled = not ResourceLoader.exists(MANOR) or slot["status"] == "error"
    play.pressed.connect(func():
        if opening_game:
            return
        opening_game = true
        play.disabled = true
        if get_tree().change_scene_to_file("res://game/demo.tscn") != OK:
            opening_game = false
            play.disabled = false
    )
    panel.add_child(play)
    if not play.disabled:
        play.grab_focus()
