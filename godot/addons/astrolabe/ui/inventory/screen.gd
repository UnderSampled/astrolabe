@tool
extends Control
## Native scene composition and live bindings; never interprets source layout records.
const ITEM = preload("res://addons/astrolabe/ui/inventory/item.gd")
const CATEGORY = preload("res://addons/astrolabe/ui/inventory/category.gd")
const SLOT = preload("res://addons/astrolabe/ui/inventory/slot.gd")
signal closed
signal item_requested(item_id: int, kind: String)
@export var initial_category: StringName
@export var items: Array[Resource] = []
@export var counter_font: FontFile
var inventory: Resource
var current_category: StringName
var window_start: int = 0
var category_views: Dictionary = {}
var slot_views: Array[SLOT] = []
var shown: Array[ITEM] = []
var _quantities: Dictionary = {}
var _weapon: int = -1

func _ready() -> void:
    get_viewport().size_changed.connect(_layout)
    _layout()
    for node in $Canvas/Categories.find_children("*", "Button", true, false):
        var view := node as CATEGORY
        if view == null: continue
        category_views[view.category] = view
        view.focus_entered.connect(func(): select_category(view.category))
        view.pressed.connect(func(): select_category(view.category))
    for node in $Canvas/Slots.get_children():
        var view := node as SLOT
        if view == null: continue
        slot_views.append(view)
        view.focus_entered.connect(func(): _detail(view.item))
        view.pressed.connect(func():
            if view.item != null: item_requested.emit(view.item.item_id, view.item.kind))
        view.overflow_requested.connect(_scroll)
    select_category(initial_category)

func bind_state(value: Resource) -> bool:
    if value == null or not value.has_method("quantity"): return false
    inventory = value
    _quantities.clear()
    _weapon = -2
    refresh()
    return true

func _layout() -> void:
    var viewport_size := get_viewport().get_visible_rect().size
    var factor := minf(viewport_size.x / 640.0, viewport_size.y / 480.0)
    $Canvas.scale = Vector2.ONE * factor
    $Canvas.position = (viewport_size - Vector2(640, 480) * factor) * 0.5

func select_category(key: StringName) -> void:
    if not category_views.has(key): return
    var selected := category_views[key] as CATEGORY
    if selected == null: return
    var changed := current_category != key
    current_category = key
    for value in category_views.values():
        var view := value as CATEGORY
        view.select(view.category == key or view.category == selected.parent_category)
    # Root refresh runs last so children inherit the current parent transform.
    for value in category_views.values():
        var root := value as CATEGORY
        if root.parent_category.is_empty(): root.refresh()
    if changed: window_start = 0
    refresh()

func refresh() -> void:
    if inventory == null: return
    _quantities = (inventory.get("quantities") as Dictionary).duplicate()
    _weapon = int(inventory.get("selected_weapon"))
    shown.clear()
    var category := category_views.get(current_category) as CATEGORY
    if category == null: return
    for value in items:
        var item := value as ITEM
        if item == null: continue
        var group := category_views.get(item.category) as CATEGORY
        if group == null or (item.category != current_category and group.parent_category != current_category): continue
        # A present -1 is a supported source sentinel. A missing key is absence.
        if not _quantities.has(item.item_id): continue
        var amount := int(_quantities[item.item_id])
        if amount == 0 and not item.draw_when_zero: continue
        shown.append(item)
    window_start = clampi(window_start, 0, maxi(shown.size() - slot_views.size(), 0))
    for index in slot_views.size():
        var item: ITEM = shown[index + window_start] if index + window_start < shown.size() else null
        slot_views[index].display(item, int(_quantities[item.item_id]) if item != null else 0, item != null and item.item_id == _weapon)
        var category_path := "../../Categories/" + str(category.category)
        if not category.parent_category.is_empty(): category_path = "../../Categories/" + str(category.parent_category) + "/" + str(category.category)
        slot_views[index].focus_neighbor_top = NodePath(category_path)
        slot_views[index].focus_neighbor_bottom = NodePath(category_path)
    var focused := get_viewport().gui_get_focus_owner() as SLOT
    _detail(focused.item if focused != null else null)

func _process(_delta: float) -> void:
    if not visible or inventory == null: return
    if inventory.get("quantities") != _quantities or int(inventory.get("selected_weapon")) != _weapon: refresh()

func _scroll(direction: int) -> void:
    var next := clampi(window_start + direction, 0, maxi(shown.size() - slot_views.size(), 0))
    if next != window_start:
        window_start = next
        refresh()
        var focus := get_viewport().gui_get_focus_owner() as SLOT
        if focus != null: _detail(focus.item)

func _detail(item: ITEM) -> void:
    $Canvas/Description.text = item.label.replace("#", "\n") if item != null else ""
    for index in $Canvas/Detail.get_child_count():
        var layer := $Canvas/Detail.get_child(index) as TextureRect
        layer.visible = item != null and index < item.textures.size()
        if layer.visible:
            layer.texture = item.textures[index]
            layer.position = item.sizes[index] * -0.5
            layer.size = item.sizes[index]

func focus_initial() -> void:
    var view := category_views.get(current_category) as CATEGORY
    if view != null: view.grab_focus()

func report(message: String) -> void:
    $Canvas/Status.text = message
    $Canvas/Status.visible = not message.is_empty()

func has_required_media() -> bool:
    if $Canvas/Background.texture == null or counter_font == null: return false
    for value in items:
        var item := value as ITEM
        if item == null or item.textures.is_empty() or item.textures.size() != item.sizes.size(): return false
        for texture in item.textures:
            if texture == null: return false
    return not category_views.is_empty() and not slot_views.is_empty()

func _unhandled_input(event: InputEvent) -> void:
    if visible and event.is_action_pressed("ui_cancel"):
        get_viewport().set_input_as_handled()
        closed.emit()
