extends Control
## Native texture controls: repeated edges and a stretched center.
@onready var fallback: Panel = $Fallback

func configure(style: Resource, fallback_style: StyleBox = null) -> void:
    if fallback_style != null: fallback.add_theme_stylebox_override("panel", fallback_style)
    fallback.visible = style == null
    for child in get_children():
        if child is TextureRect:
            child.visible = style != null
            if style == null: child.texture = null
    if style == null: return
    for path in ["TopLeft", "TopRight", "BottomLeft", "BottomRight"]: get_node(path).texture = style.corner
    for path in ["Top", "Bottom"]: get_node(path).texture = style.border
    for path in ["Left", "Right"]: get_node(path).texture = style.side
    $Center.texture = style.center
