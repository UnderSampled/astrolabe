extends Control
## Game main menu. Loading the supplied save requires the converted manor.
const MANOR := "res://generated/hype/levels/manoir/level.tscn"
var opening_game := false

func _ready() -> void:
    var panel := VBoxContainer.new()
    panel.position = Vector2(40, 40)
    add_child(panel)
    var heading := Label.new()
    heading.text = "Hype: The Time Quest"
    panel.add_child(heading)
    var details := Label.new()
    details.text = "Continue from the manor." if ResourceLoader.exists(MANOR) else "Generate the manor from your Hype copy to play. See godot/README.md."
    panel.add_child(details)
    var play := Button.new()
    play.text = "Load Game"
    play.disabled = not ResourceLoader.exists(MANOR)
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
