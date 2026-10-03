@tool
extends Button
## Native button with two preconverted artwork states. No source font interpreter.
@export var action: StringName
@export var normal_texture: Texture2D:
    set(value):
        normal_texture = value
        if is_node_ready(): _refresh()
@export var selected_texture: Texture2D:
    set(value):
        selected_texture = value
        if is_node_ready(): _refresh()
@onready var artwork: TextureRect = $Artwork

func _ready() -> void:
    focus_entered.connect(_refresh)
    focus_exited.connect(_refresh)
    mouse_entered.connect(_hover)
    mouse_exited.connect(_refresh)
    _refresh()

func _hover() -> void:
    if not disabled and not Engine.is_editor_hint(): grab_focus()
    _refresh()

func _refresh() -> void:
    artwork.texture = selected_texture if not disabled and (has_focus() or is_hovered()) else normal_texture
    artwork.modulate = Color(0.6, 0.6, 0.6) if disabled else Color.WHITE

func _notification(what: int) -> void:
    if what == NOTIFICATION_DRAW and is_node_ready(): _refresh()
