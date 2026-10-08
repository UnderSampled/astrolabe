extends CanvasLayer
## Authored session composition; generated presentation remains independently owned.
const VIEW = preload("res://addons/astrolabe/ui/inventory/screen.gd")
const PICKUP_FEEDBACK = preload("res://addons/astrolabe/ui/pickup_feedback.gd")
const DEFAULT_SCENE := "res://generated/hype/shared/inventory/en/inventory.tscn"
const DEFAULT_PICKUP_ARTWORK := "res://generated/hype/shared/inventory/en/pickup_artwork.tres"
signal unavailable(message: String)
@export var inventory_scene: PackedScene
@export var pickup_artwork: Resource
@export var inventory_key: Key = KEY_TAB
var view: VIEW
var owns_pause: bool = false

func _ready() -> void:
    process_mode = Node.PROCESS_MODE_ALWAYS
    layer = 20

func source_display_pickup(item: int, milliseconds: int, from: float, to: float, x: int, y: int, width: int, height: int) -> bool:
    if pickup_artwork == null and ResourceLoader.exists(DEFAULT_PICKUP_ARTWORK):
        pickup_artwork = load(DEFAULT_PICKUP_ARTWORK) as Resource
    return PICKUP_FEEDBACK.display(self, pickup_artwork, item, milliseconds, from, to, x, y, width, height)

func open_inventory() -> bool:
    if is_instance_valid(view) and view.visible: return true
    var session := get_parent()
    if not session.has_method("inventory_available") or not bool(session.call("inventory_available")): return false
    if get_tree().paused: return false
    var actor := session.get("actor") as Node
    var state := actor.get("source_inventory") as Resource if actor != null else null
    if state == null: return false
    if not is_instance_valid(view):
        var scene := inventory_scene
        if scene == null and ResourceLoader.exists(DEFAULT_SCENE): scene = load(DEFAULT_SCENE) as PackedScene
        if scene == null:
            unavailable.emit("The inventory assets have not been generated. Required: " + DEFAULT_SCENE)
            return false
        view = scene.instantiate() as VIEW
        if view == null:
            unavailable.emit("The inventory scene is invalid.")
            return false
        view.hide()
        add_child(view)
        if not view.has_required_media():
            unavailable.emit("The inventory is missing required original artwork or font resources.")
            view.queue_free()
            view = null
            return false
        view.closed.connect(close_inventory)
        view.item_requested.connect(_item_requested)
    view.reset_motion()
    if not view.bind_state(state): return false
    # Modal pause is owned only while this view is open. No Brain, actor state,
    # quantities, fixture, or save is replaced to show the inventory.
    owns_pause = true
    get_tree().paused = true
    view.show()
    view.report("")
    view.focus_initial()
    return true

func close_inventory() -> void:
    if is_instance_valid(view):
        view.reset_motion()
        view.hide()
    if owns_pause and is_inside_tree(): get_tree().paused = false
    owns_pause = false

func _item_requested(item_id: int, kind: String) -> void:
    if not is_instance_valid(view) or view.inventory == null: return
    var quantities: Dictionary = view.inventory.call("presentation_quantities") if view.inventory.has_method("presentation_quantities") else view.inventory.get("quantities")
    var amount := int(quantities.get(item_id, 0))
    if not quantities.has(item_id) or amount == 0:
        view.report("This item is not available.")
        return
    if kind == "Weapon":
        if view.inventory.get("source_initialized"):
            if int(view.inventory.call("change_weapon", item_id)) != 1:
                var failure := str(view.inventory.get("last_failure"))
                view.report(failure if not failure.is_empty() else "This weapon is not admitted to the inventory.")
                return
        else:
            view.inventory.set("selected_weapon", item_id)
            view.inventory.emit_changed()
        view.report("")
        view.refresh()
    else:
        view.report("Using this item is not implemented yet.")

func _input(event: InputEvent) -> void:
    var key := event as InputEventKey
    if key == null or not key.pressed or key.echo or key.physical_keycode != inventory_key: return
    # Tab is also Godot's focus-next key. Consume its non-echo press before GUI
    # focus navigation; release/echo events never toggle the modal a second time.
    get_viewport().set_input_as_handled()
    if is_instance_valid(view) and view.visible: close_inventory()
    else: open_inventory()

func _exit_tree() -> void:
    if owns_pause: get_tree().paused = false
    owns_pause = false
