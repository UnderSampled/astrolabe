extends Control
## Native texture controls: repeated edges and a stretched center.
@onready var fallback: Panel = $Fallback
@onready var _center_position: Vector2 = $Center.position
@onready var _center_size: Vector2 = $Center.size

func configure(style: Resource, fallback_style: StyleBox = null, box: int = 1) -> void:
    if fallback_style != null: fallback.add_theme_stylebox_override("panel", fallback_style)
    fallback.visible = style == null
    for child in get_children():
        if child is TextureRect:
            child.visible = style != null and (box != 5 or child == $Center)
            if style == null: child.texture = null
    if style == null: return
    for path in ["TopLeft", "TopRight", "BottomLeft", "BottomRight"]: get_node(path).texture = style.corner
    for path in ["Top", "Bottom"]: get_node(path).texture = style.border
    for path in ["Left", "Right"]: get_node(path).texture = style.side
    $Center.texture = style.center
    $Center.position = _center_position
    $Center.size = _center_size
    if box == 5:
        # Source style5 draws only the center material. Dimensions come from
        # the converted stencil textures, including the source integer ratios.
        var corner_width: int = style.corner.get_width()
        var corner_height: int = style.corner.get_height()
        var center_width: int = style.center.get_width()
        var center_height: int = style.center.get_height()
        var columns: int = int(float(style.border.get_width()) / float(center_width))
        var rows: int = int(float(style.side.get_height()) / float(center_height))
        var start := Vector2(corner_width >> 1, corner_height >> 1)
        var end := Vector2(8.0 * columns * center_width + 1.5 * corner_width, 4.0 * rows * center_height + 1.5 * corner_height)
        $Center.position = start
        $Center.size = end - start
