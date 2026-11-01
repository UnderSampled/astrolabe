extends Control
## Application/session routes around a generated, editable native menu scene.
const MANOR := "res://generated/hype/levels/manoir/level.tscn"
const MENU_SCENE := "res://generated/hype/shared/menu/main/en/menu.tscn"
const SETTINGS_SCENE := "res://game/settings_menu.tscn"
const CONFIGURATION_SCENE := "res://generated/hype/shared/menu/configuration/en/menu.tscn"
const CREDITS_SCENE := "res://game/credits_menu.tscn"
const LOAD_GAME_SCENE := "res://game/load_game_picker.tscn"
const MENU_VIEW = preload("res://addons/astrolabe/ui/menu/menu.gd")
const LOAD_GAME_PICKER = preload("res://game/load_game_picker.gd")
const SAVE_STATE = preload("res://game/save_state.gd")
const BEHAVIOR = preload("res://addons/astrolabe/behavior/behavior.gd")
const INITIAL_ROUTE_ERROR_META := "astrolabe_initial_route_error"

## Assign an inherited scene saved under game/ to preserve designer improvements.
@export var menu_scene: PackedScene
@onready var details: Label = $Status/Details
@onready var status: PanelContainer = $Status
@onready var error_quit: Button = get_node_or_null("ErrorQuit") as Button
var menu: MENU_VIEW
var load_game: Button
var new_game: Button
var quit_game: Button
var opening_game := false
var _menu_button_disabled: Dictionary = {}
var configuration: Control
var credits: Control
var save_picker: LOAD_GAME_PICKER

func _ready() -> void:
    # Play on through documented stand-ins unless a check asks for strict Brains.
    BEHAVIOR.strict_unsupported = OS.get_environment("ASTROLABE_STRICT_BEHAVIOR") == "1"
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
        if is_instance_valid(error_quit):
            error_quit.show()
            error_quit.grab_focus()
        push_error(details.text)
        return
    var candidates := SAVE_STATE.load_candidates()
    var manor_available := ResourceLoader.exists(MANOR)
    load_game.disabled = not manor_available or candidates.is_empty()
    new_game.disabled = not manor_available
    if not manor_available:
        details.text = "Prepare the game data from your Hype copy to play."
        status.show()
    elif candidates.is_empty():
        details.text = "No saved game is available. Start New Game or import an original Hype save."
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
    var root := get_tree().root
    if root.has_meta(INITIAL_ROUTE_ERROR_META):
        details.text = "The game could not be opened: " + str(root.get_meta(INITIAL_ROUTE_ERROR_META))
        root.remove_meta(INITIAL_ROUTE_ERROR_META)
        status.show()

func _show_menu_error() -> void:
    status.show()
    details.text = "Required menu assets are missing.\nRestore the installed menu assets."
    if is_instance_valid(error_quit):
        error_quit.show()
        error_quit.grab_focus()
    push_error("Required menu assets are missing: " + MENU_SCENE + ". Restore the installed menu assets.")

func _on_menu_action(action: StringName) -> void:
    match action:
        &"NewGame": _on_new_game_pressed()
        &"SavedGames": _on_load_game_pressed()
        &"Quit": _on_quit_pressed()
        &"Options": _open_configuration()
        &"Credits": _open_credits()
        &"Internet": _show_obsolete_internet_route()
        _:
            var selected := menu.button_for_action(action)
            details.text = (selected.text.capitalize() if selected != null else str(action)) + " is not implemented yet."
            status.show()

func _open_credits() -> void:
    status.hide()
    var packed := load(CREDITS_SCENE) as PackedScene if ResourceLoader.exists(CREDITS_SCENE) else null
    credits = packed.instantiate() as Control if packed != null else null
    if credits == null:
        details.text = "The Credits screen could not be loaded."
        status.show()
        return
    menu.hide()
    $MenuHost.add_child(credits)
    credits.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    credits.closed.connect(_close_credits)
    credits.focus_initial()

func _close_credits() -> void:
    if is_instance_valid(credits): credits.queue_free()
    credits = null
    if is_instance_valid(menu):
        menu.show()
        var button := menu.button_for_action(&"Credits")
        if button != null: button.grab_focus()

func _show_obsolete_internet_route() -> void:
    details.text = "The original Internet destination is obsolete. This menu does not open external pages."
    status.show()

func _open_configuration() -> void:
    status.hide()
    var settings_path := SETTINGS_SCENE if ResourceLoader.exists(SETTINGS_SCENE, "PackedScene") else CONFIGURATION_SCENE
    if not ResourceLoader.exists(settings_path, "PackedScene"):
        details.text = "Configuration assets have not been generated. Regenerate the menu content."
        status.show()
        return
    var packed := load(settings_path) as PackedScene
    configuration = packed.instantiate() as Control if packed != null else null
    if configuration == null:
        details.text = "The Options screen is invalid."
        status.show()
        return
    menu.hide()
    $MenuHost.add_child(configuration)
    configuration.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    configuration.closed.connect(_close_configuration)
    configuration.focus_initial()

func _close_configuration() -> void:
    if is_instance_valid(configuration): configuration.queue_free()
    configuration = null
    if is_instance_valid(menu):
        menu.show()
        var configure := menu.button_for_action(&"Options")
        if configure != null: configure.grab_focus()

func _input(event: InputEvent) -> void:
    var key := event as InputEventKey
    if key == null or not key.pressed or key.echo or key.physical_keycode != KEY_ESCAPE: return
    if is_instance_valid(configuration) and configuration.visible:
        get_viewport().set_input_as_handled()
        if configuration.has_method("cancel_input_capture") and bool(configuration.call("cancel_input_capture")):
            return
        _close_configuration()

func _on_new_game_pressed() -> void:
    get_tree().root.remove_meta(SAVE_STATE.LOAD_SOURCE_META)
    open_game("res://game/new_game.tscn", new_game)

func _on_load_game_pressed() -> void:
    if opening_game or is_instance_valid(save_picker): return
    var candidates := SAVE_STATE.load_candidates()
    if candidates.is_empty():
        details.text = "No saved game is available. Start New Game or import an original Hype save."
        status.show()
        return
    var packed := load(LOAD_GAME_SCENE) as PackedScene
    save_picker = packed.instantiate() as LOAD_GAME_PICKER if packed != null else null
    if save_picker == null:
        details.text = "The Load Game screen could not be opened."
        status.show()
        return
    status.hide()
    menu.hide()
    $MenuHost.add_child(save_picker)
    save_picker.selected.connect(_on_save_selected)
    save_picker.closed.connect(_close_save_picker)
    save_picker.show_candidates(candidates)

func _on_save_selected(source: String) -> void:
    # Re-read the selected source just before navigation; an unavailable or
    # changed slot must not switch the route to a different save or New Game.
    var candidate := SAVE_STATE.read_for_load(source)
    if candidate["status"] != "loaded":
        _close_save_picker()
        details.text = str(candidate.get("message", "The selected save is unavailable."))
        status.show()
        return
    get_tree().root.set_meta(SAVE_STATE.LOAD_SOURCE_META, source)
    _close_save_picker()
    open_game("res://game/game_session.tscn", load_game)

func _close_save_picker() -> void:
    if is_instance_valid(save_picker): save_picker.queue_free()
    save_picker = null
    if is_instance_valid(menu): menu.show()
    if is_instance_valid(load_game): load_game.grab_focus()

func _on_quit_pressed() -> void:
    get_tree().quit()

func open_game(destination: String, button: Button) -> void:
    if opening_game: return
    if button == null or not is_instance_valid(button):
        details.text = "The selected game route is unavailable."
        status.show()
        return
    opening_game = true
    _lock_menu_buttons()
    details.text = "Starting a new game..." if destination == "res://game/new_game.tscn" else "Loading saved game..."
    status.show()
    var error := get_tree().change_scene_to_file(destination)
    if error != OK:
        opening_game = false
        _restore_menu_buttons()
        details.text = "The game session could not be opened."
        status.show()
        button.grab_focus()
        push_error("Could not open game session %s: %s" % [destination, error_string(error)])

func _lock_menu_buttons() -> void:
    _menu_button_disabled.clear()
    if not is_instance_valid(menu):
        return
    for item in menu.find_children("*", "BaseButton", true, false):
        var button := item as BaseButton
        if button == null:
            continue
        _menu_button_disabled[button] = button.disabled
        button.disabled = true

func _restore_menu_buttons() -> void:
    for item in _menu_button_disabled:
        var button := item as BaseButton
        if is_instance_valid(button):
            button.disabled = bool(_menu_button_disabled[item])
    _menu_button_disabled.clear()
