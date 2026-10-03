extends Control
## Authored main menu presentation around the existing game-session entries.
const MANOR := "res://generated/hype/levels/manoir/level.tscn"
const MENU_BACKGROUND := "res://generated/hype/textures/MENU/menu_princ.gf.png"
const SAVE_STATE = preload("res://game/save_state.gd")

@export var background_texture: Texture2D
@onready var canvas: Control = $MenuCanvas
@onready var background: TextureRect = $MenuCanvas/Background
@onready var details: Label = $MenuCanvas/Details
@onready var load_game: Button = $MenuCanvas/LoadGame
@onready var new_game: Button = $MenuCanvas/NewGame
@onready var quit_game: Button = $MenuCanvas/Quit
var opening_game := false

func _ready() -> void:
    resized.connect(_fit_canvas)
    _fit_canvas()
    if background_texture == null and ResourceLoader.exists(MENU_BACKGROUND):
        background_texture = load(MENU_BACKGROUND) as Texture2D
    background.texture = background_texture
    if background_texture == null:
        new_game.disabled = true
        load_game.disabled = true
        details.text = "Required menu artwork is missing.\nRestore the installed menu assets."
        quit_game.grab_focus()
        push_error("Required menu artwork is missing: " + MENU_BACKGROUND + ". Restore the installed menu assets.")
        return
    var slot: Dictionary = SAVE_STATE.read()
    details.text = "Prepare the game data from your Hype copy to play."
    if ResourceLoader.exists(MANOR):
        details.text = "Continue from your local manor save." if slot["status"] == "loaded" else "Load Game begins from the supplied English manor save."
        if slot["status"] == "error":
            details.text = "Load Game unavailable: " + str(slot["message"])
    load_game.disabled = not ResourceLoader.exists(MANOR) or slot["status"] == "error"
    new_game.disabled = not ResourceLoader.exists(MANOR)
    if not load_game.disabled:
        load_game.grab_focus()
    elif not new_game.disabled:
        new_game.grab_focus()
    else:
        quit_game.grab_focus()

func _fit_canvas() -> void:
    var factor := minf(size.x / 640.0, size.y / 480.0)
    canvas.scale = Vector2.ONE * factor
    canvas.position = (size - Vector2(640.0, 480.0) * factor) * 0.5

func _on_new_game_pressed() -> void:
    open_game("res://game/new_game.tscn", new_game)

func _on_load_game_pressed() -> void:
    open_game("res://game/demo.tscn", load_game)

func _on_quit_pressed() -> void:
    get_tree().quit()

func open_game(destination: String, button: Button) -> void:
    if opening_game: return
    opening_game = true
    button.disabled = true
    if get_tree().change_scene_to_file(destination) != OK:
        opening_game = false
        button.disabled = false
