extends Control
## Application/session routes around a generated, editable native menu scene.
const MANOR := "res://generated/hype/levels/manoir/level.tscn"
const MENU_SCENE := "res://generated/hype/shared/menu/main/en/menu.tscn"
const MENU_VIEW = preload("res://addons/astrolabe/ui/menu/menu.gd")
const SAVE_STATE = preload("res://game/save_state.gd")

## Assign an inherited scene saved under game/ to preserve designer improvements.
@export var menu_scene: PackedScene
@onready var details: Label = $Status/Details
@onready var status: PanelContainer = $Status
@onready var error_quit: Button = $ErrorQuit
var menu: MENU_VIEW
var load_game: Button
var new_game: Button
var quit_game: Button
var opening_game := false

func _ready() -> void:
    if menu_scene == null and ResourceLoader.exists(MENU_SCENE):
        menu_scene = load(MENU_SCENE) as PackedScene
    if menu_scene == null:
        _show_menu_error()
        return
    menu = menu_scene.instantiate() as MENU_VIEW
    if menu == null or not menu.has_required_media():
        if menu != null: menu.free()
        menu = null
        _show_menu_error()
        return
    $MenuHost.add_child(menu)
    menu.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    menu.action_selected.connect(_on_menu_action)
    new_game = menu.button_for_action(&"NewGame")
    load_game = menu.button_for_action(&"SavedGames")
    quit_game = menu.button_for_action(&"Quit")
    if new_game == null or load_game == null or quit_game == null:
        details.text = "Required menu actions are missing from the selected scene."
        status.show()
        error_quit.show()
        error_quit.grab_focus()
        push_error(details.text)
        return
    var slot: Dictionary = SAVE_STATE.read()
    var manor_available := ResourceLoader.exists(MANOR)
    load_game.disabled = not manor_available or slot["status"] == "error"
    new_game.disabled = not manor_available
    if not manor_available:
        details.text = "Prepare the game data from your Hype copy to play."
        status.show()
    elif slot["status"] == "error":
        details.text = "Load Game unavailable: " + str(slot["message"])
        status.show()
    var initial := menu.get_node_or_null(menu.initial_focus) as Button
    if initial != null and not initial.disabled:
        initial.grab_focus()
    elif not load_game.disabled:
        load_game.grab_focus()
    elif not new_game.disabled:
        new_game.grab_focus()
    else:
        quit_game.grab_focus()

func _show_menu_error() -> void:
    status.show()
    details.text = "Required menu assets are missing.\nRestore the installed menu assets."
    error_quit.show()
    error_quit.grab_focus()
    push_error("Required menu assets are missing: " + MENU_SCENE + ". Restore the installed menu assets.")

func _on_menu_action(action: StringName) -> void:
    match action:
        &"NewGame": _on_new_game_pressed()
        &"SavedGames": _on_load_game_pressed()
        &"Quit": _on_quit_pressed()
        _:
            var selected := menu.button_for_action(action)
            details.text = (selected.text.capitalize() if selected != null else str(action)) + " is not implemented yet."
            status.show()

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
