extends Node3D
## Native animation keys supply one current default, never source frame tables.
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
