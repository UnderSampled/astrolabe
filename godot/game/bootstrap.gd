extends Control
## Application/session routes around a generated, editable native menu scene.
const MANOR := "res://generated/hype/levels/manoir/level.tscn"
const MENU_SCENE := "res://generated/hype/shared/menu/main/en/menu.tscn"
const INVENTORY_SCENE := "res://generated/hype/shared/inventory/en/inventory.tscn"
const PAUSE_SCENE := "res://generated/hype/shared/menu/pause/en/menu.tscn"
const MENU_VIEW = preload("res://addons/astrolabe/ui/menu/menu.gd")
const SAVE_STATE = preload("res://game/save_state.gd")

## Assign an inherited scene saved under game/ to preserve designer improvements.
@export var menu_scene: PackedScene
@onready var details: Label = $Status/Details
@onready var status: PanelContainer = $Status
@onready var setup: PanelContainer = $Setup
@onready var setup_details: Label = $Setup/Rows/Details
@onready var setup_quit: Button = $Setup/Rows/Quit
var menu: MENU_VIEW
var load_game: Button
var new_game: Button
var quit_game: Button
var opening_game := false
var content_ready := false

func _ready() -> void:
    if menu_scene == null and ResourceLoader.exists(MENU_SCENE):
        menu_scene = load(MENU_SCENE) as PackedScene
    if menu_scene == null or not menu_scene.can_instantiate():
        _show_setup("The main menu has not been prepared.")
        return
    var instance := menu_scene.instantiate()
    menu = instance as MENU_VIEW
    if menu == null or not menu.has_required_media():
        instance.free()
        menu = null
        _show_setup("The selected main menu is missing required artwork or sound resources.")
        return
    $MenuHost.add_child(menu)
    menu.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    menu.action_selected.connect(_on_menu_action)
    new_game = menu.button_for_action(&"NewGame")
    load_game = menu.button_for_action(&"SavedGames")
    quit_game = menu.button_for_action(&"Quit")
    if new_game == null or load_game == null or quit_game == null:
        _show_setup("The selected main menu is missing New Game, Load Game or Quit actions.")
        return
    var slot: Dictionary = SAVE_STATE.read()
    var missing := _missing_content()
    content_ready = missing.is_empty()
    load_game.disabled = not content_ready or slot["status"] == "error"
    new_game.disabled = not content_ready
    if not content_ready:
        _show_setup("Game preparation is incomplete:\n" + "\n".join(missing))
        return
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

## Small native presentation scenes prepare their bindings while hidden. The
## level stays lazy so opening the menu does not load the entire manor.
func _presentation_available(path: String) -> bool:
    if not ResourceLoader.exists(path): return false
    var packed := load(path) as PackedScene
    if packed == null or not packed.can_instantiate(): return false
    var instance := packed.instantiate()
    var control := instance as Control
    if control == null:
        instance.free()
        return false
    control.hide()
    add_child(control)
    var available := instance.has_method("has_required_media") and bool(instance.call("has_required_media"))
    instance.free()
    return available

func _missing_content() -> PackedStringArray:
    var missing := PackedStringArray()
    if not ResourceLoader.exists(MANOR): missing.append("Manor level")
    if not _presentation_available(INVENTORY_SCENE): missing.append("Inventory artwork and font")
    if not _presentation_available(PAUSE_SCENE): missing.append("Pause menu artwork and sounds")
    return missing

func _show_setup(message: String) -> void:
    $MenuHost.hide()
    status.hide()
    setup_details.text = message + "\n\nPrepare the game from your own Hype copy, then reopen this project.\nRun the commands below from the Astrolabe repository. Replace the directory argument disk with your game directory, and the directory arguments godot after prepare-game and --path with this project directory."
    setup.show()
    setup_quit.grab_focus()

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
    if opening_game or not content_ready or button == null or button.disabled: return
    opening_game = true
    button.disabled = true
    if get_tree().change_scene_to_file(destination) != OK:
        opening_game = false
        button.disabled = false
