extends Control
## Game main menu. Load Game restores supported save state; New Game exposes
## the generated opening's current incomplete behavior.
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
    play.name = "LoadGame"
    play.text = "Load Game"
    play.disabled = not ResourceLoader.exists(MANOR) or slot["status"] == "error"
    play.pressed.connect(func(): open_game("res://game/demo.tscn",play))
    panel.add_child(play)
    var new_game := Button.new()
    new_game.name = "NewGame"
    new_game.text = "New Game"
    new_game.disabled = not ResourceLoader.exists(MANOR)
    new_game.pressed.connect(func(): open_game("res://game/new_game.tscn",new_game))
    panel.add_child(new_game)
    var opening_note := Label.new()
    opening_note.text = "New Game starts the unfinished opening."
    panel.add_child(opening_note)
    if not play.disabled:
        play.grab_focus()
    elif not new_game.disabled:
        new_game.grab_focus()

func open_game(destination: String, button: Button) -> void:
    if opening_game: return
    opening_game = true
    button.disabled = true
    if get_tree().change_scene_to_file(destination) != OK:
        opening_game = false
        button.disabled = false
