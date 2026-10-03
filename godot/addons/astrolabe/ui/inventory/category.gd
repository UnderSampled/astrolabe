@tool
extends Button
## Source geometry is already expressed as editable native pixel placement.
@export var category: StringName
@export var parent_category: StringName
@export var source_position: Vector2
@export var normal_scale: float = 1.0
@export var selected_scale: float = 1.0
@export var normal_color: Color = Color.WHITE
@export var selected_color: Color = Color.WHITE
var active: bool = false

func _ready() -> void:
    refresh()

func select(enabled: bool) -> void:
    active = enabled
    refresh()

func refresh() -> void:
    var artwork := get_node_or_null("Artwork") as Control
    if artwork == null: return
    var factor := selected_scale if active else normal_scale
    artwork.scale = Vector2.ONE * factor
    artwork.modulate = selected_color if active else normal_color
    # Child source angles/radii were resolved by conversion. Parent state scales
    # their offsets and artwork using ordinary native transforms.
    for child in get_children():
        if child is Button and child.get_script() == get_script():
            var location: Vector2 = child.get("source_position")
            child.position = location * factor
            var child_art := child.get_node_or_null("Artwork") as Control
            if child_art != null:
                child_art.scale = Vector2.ONE * factor
                child_art.modulate = artwork.modulate
