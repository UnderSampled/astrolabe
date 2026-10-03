@tool
extends Button
const ITEM = preload("res://addons/astrolabe/ui/inventory/item.gd")
signal overflow_requested(direction: int)
@export var normal_scale: float = 1.0
@export var equipped_scale: float = 1.0
@export var current_scale: float = 1.0
@export var normal_color: Color = Color.WHITE
@export var current_color: Color = Color.WHITE
var item: ITEM
var quantity: int = 0
var equipped: bool = false

func _ready() -> void:
    focus_entered.connect(refresh)
    focus_exited.connect(refresh)
    mouse_entered.connect(func():
        if not disabled: grab_focus())
    refresh()

func display(value: ITEM, amount: int, selected: bool) -> void:
    item = value
    quantity = amount
    equipped = selected
    disabled = item == null
    text = item.label if item != null else ""
    tooltip_text = text
    refresh()

func refresh() -> void:
    var artwork := get_node_or_null("Artwork") as Control
    var counter := get_node_or_null("Counter") as Label
    if artwork == null or counter == null: return
    var factor := current_scale if has_focus() else (equipped_scale if equipped else normal_scale)
    artwork.scale = Vector2.ONE * factor
    artwork.modulate = current_color if has_focus() else normal_color
    for index in artwork.get_child_count():
        var layer := artwork.get_child(index) as TextureRect
        layer.visible = item != null and index < item.textures.size()
        if layer.visible:
            layer.texture = item.textures[index]
            layer.position = item.sizes[index] * -0.5
            layer.size = item.sizes[index]
    counter.text = str(quantity) if item != null and item.draw_counter and quantity >= 0 else ""

func _gui_input(event: InputEvent) -> void:
    if not has_focus(): return
    if event.is_action_pressed("ui_right") and get_focus_neighbor(SIDE_RIGHT).is_empty():
        overflow_requested.emit(1)
        accept_event()
    elif event.is_action_pressed("ui_left") and get_focus_neighbor(SIDE_LEFT).is_empty():
        overflow_requested.emit(-1)
        accept_event()
