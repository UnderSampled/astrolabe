extends Node3D
## Native animation keys supply one current default, never source frame tables.

## Which part node this channel shows, and its source opacity (255 opaque). One
## pair of keys per channel replaces a visible and a transparency track for
## every part: Hype's library held 327,000 of those and took 1.9 s to load.
@export var active_part: StringName = &"":
    set(value):
        active_part = value
        _apply_active_part()
@export var active_opacity: int = 255:
    set(value):
        active_opacity = value
        _apply_active_part()

func _apply_active_part() -> void:
    for child in get_children():
        var name := str(child.name)
        if not (name.begins_with("part_") or name.begins_with("reset_part_")): continue
        var shown := child.name == active_part and active_opacity > 0
        child.visible = shown
        child.set("transparency", 1.0 - float(active_opacity) / 255.0 if shown else 0.0)
@export var sample_visibility: int = -1:
    set(value):
        sample_visibility = value
        if value < 0: return
        var channels := get_parent()
        var actor := channels.get_parent() if channels != null else null
        if actor == null:
            visible = value != 0
            return
        var component: Variant = actor.get("source_channel_visibility")
        if component == null:
            visible = value != 0
            return
        component.apply_sample(self, value)
