extends Node3D
## Stable animation target; its saved native child carries the mutable content.
## Godot transparency: zero is opaque, unlike source opacity's 255.
@export var transparency: float = 0.0:
    set(value):
        transparency = clampf(value, 0.0, 1.0)
        refresh_transparency()

func refresh_transparency() -> void:
    for geometry in find_children("*", "GeometryInstance3D", true, false):
        (geometry as GeometryInstance3D).transparency = transparency
