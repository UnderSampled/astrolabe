extends CanvasLayer
## Authored game routes over the installed source menu's native presentation.
const VIEW = preload("res://addons/astrolabe/ui/menu/menu.gd")
const DEFAULT_SCENE := "res://generated/hype/shared/menu/pause/en/menu.tscn"
const CONFIGURATION_SCENE := "res://generated/hype/shared/menu/configuration/en/menu.tscn"
signal unavailable(message: String)
@export var pause_scene: PackedScene
@onready var status: Label = $Status
var view: VIEW
var owns_pause := false
var previous_focus: WeakRef
var configuration: Control

func _ready() -> void:
    process_mode = Node.PROCESS_MODE_ALWAYS
    layer = 30

func open_pause() -> bool:
    if is_instance_valid(view) and view.visible: return true
    var session := get_parent()
    if not session.has_method("pause_available") or not bool(session.call("pause_available")): return false
    if get_tree().paused: return false
    if not is_instance_valid(view):
        var scene := pause_scene
        if scene == null and ResourceLoader.exists(DEFAULT_SCENE): scene = load(DEFAULT_SCENE) as PackedScene
        if scene == null:
            unavailable.emit("The pause menu assets have not been generated. Required: " + DEFAULT_SCENE)
            return false
        view = scene.instantiate() as VIEW
        if view == null:
            unavailable.emit("The pause menu scene is invalid.")
            return false
        view.hide()
        add_child(view)
        if not view.has_required_media():
            unavailable.emit("The pause menu is missing required original artwork or sound resources.")
            view.queue_free()
            view = null
            return false
        view.action_selected.connect(_action_selected)
    var focused := get_viewport().gui_get_focus_owner()
    previous_focus = weakref(focused) if focused != null else null
    owns_pause = true
    get_tree().paused = true
    status.text = ""
    view.show()
    view.focus_initial()
    return true

func close_pause() -> void:
    if is_instance_valid(configuration):
        configuration.queue_free()
        configuration = null
    if is_instance_valid(view):
        view.cancel_activation()
        view.hide()
    status.text = ""
    # Resume Game (mode 0x12) writes the options file in the source (00404700).
    var preferences := get_node_or_null("/root/UserSettings")
    if preferences != null and preferences.has_method("commit_control_bindings"):
        preferences.call("commit_control_bindings")
    if owns_pause and is_inside_tree(): get_tree().paused = false
    owns_pause = false
    var focused := previous_focus.get_ref() as Control if previous_focus != null else null
    previous_focus = null
    if is_instance_valid(focused) and focused.is_visible_in_tree(): focused.grab_focus()

func _action_selected(action: StringName) -> void:
    if not is_instance_valid(view) or not view.visible: return
    match action:
        &"Resume Game": close_pause()
        &"NextGame":
            close_pause()
            # Installed source action resets a new adventure; it is not SavedGames.
            get_parent().call("leave_session", "res://game/new_game.tscn")
        &"Main Menu":
            close_pause()
            get_parent().call("leave_session", "res://game/bootstrap.tscn")
        &"Options": _open_configuration()

func _open_configuration() -> void:
    if not ResourceLoader.exists(CONFIGURATION_SCENE):
        status.text = "Configuration assets have not been generated. Regenerate the menu content."
        return
    var packed := load(CONFIGURATION_SCENE) as PackedScene
    configuration = packed.instantiate() as Control if packed != null else null
    if configuration == null:
        status.text = "The generated configuration scene is invalid."
        return
    view.hide()
    add_child(configuration)
    configuration.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    configuration.closed.connect(_close_configuration)
    configuration.focus_initial()

func _close_configuration() -> void:
    if is_instance_valid(configuration): configuration.queue_free()
    configuration = null
    if is_instance_valid(view) and owns_pause:
        view.show()
        view.focus_initial()

func _input(event: InputEvent) -> void:
    var key := event as InputEventKey
    if key == null or not key.pressed or key.echo or key.physical_keycode != KEY_ESCAPE: return
    if is_instance_valid(configuration) and configuration.visible:
        get_viewport().set_input_as_handled()
        if configuration.has_method("cancel_input_capture") and bool(configuration.call("cancel_input_capture")):
            return
        _close_configuration()
        return
    # Escape closes inventory first, before GUI cancellation or any session route.
    var inventory := get_parent().get_node_or_null("InventoryHost")
    var inventory_view := inventory.get("view") as Control if inventory != null else null
    if is_instance_valid(inventory_view) and inventory_view.visible:
        get_viewport().set_input_as_handled()
        inventory.call("close_inventory")
        return
    if not (is_instance_valid(view) and view.visible):
        var session := get_parent()
        if not session.has_method("pause_available") or not bool(session.call("pause_available")): return
    get_viewport().set_input_as_handled()
    if is_instance_valid(view) and view.visible: close_pause()
    else: open_pause()

func _exit_tree() -> void:
    if owns_pause: get_tree().paused = false
    owns_pause = false
