@tool
extends Control
## Editable native menu composition; generated scenes supply ordinary node properties.
signal action_selected(action: StringName)
const ITEM_VIEW = preload("res://addons/astrolabe/ui/menu/button.gd")
@export var canvas_size := Vector2(640, 480)
@export var initial_focus: NodePath
@onready var canvas: Control = $Canvas
@onready var navigation: AudioStreamPlayer = $Audio/Navigation
@onready var confirmation: AudioStreamPlayer = $Audio/Confirmation
var feedback_enabled := false
var activation_pending := false

func _ready() -> void:
    resized.connect(_fit)
    _fit()
    if Engine.is_editor_hint(): return
    for button in canvas.get_children():
        if button is ITEM_VIEW:
            button.focus_entered.connect(_on_focus)
            button.pressed.connect(_activate.bind(button.action))
    var initial := get_node_or_null(initial_focus) as Button
    if initial != null and not initial.disabled: initial.grab_focus()
    call_deferred("_enable_feedback")

func _fit() -> void:
    var factor := minf(size.x / canvas_size.x, size.y / canvas_size.y)
    canvas.scale = Vector2.ONE * factor
    canvas.position = (size - canvas_size * factor) * 0.5

func _enable_feedback() -> void:
    feedback_enabled = true

func _on_focus() -> void:
    if feedback_enabled and not activation_pending: navigation.play()

func _activate(action: StringName) -> void:
    if activation_pending: return
    activation_pending = true
    confirmation.play()
    # Let the short confirmation finish before a consumer frees the menu.
    if confirmation.stream != null:
        await get_tree().create_timer(minf(confirmation.stream.get_length(), 0.5)).timeout
    if is_inside_tree(): action_selected.emit(action)
    activation_pending = false

func button_for_action(action: StringName) -> Button:
    for child in canvas.get_children():
        if child is ITEM_VIEW and child.action == action: return child
    return null

func has_required_media() -> bool:
    var backdrop := get_node_or_null("Canvas/Background") as TextureRect
    if backdrop == null or backdrop.texture == null: return false
    var content := get_node_or_null("Canvas")
    if content == null: return false
    for child in content.get_children():
        if child is ITEM_VIEW and (child.normal_texture == null or child.selected_texture == null): return false
    for path in ["Audio/Navigation", "Audio/Confirmation"]:
        var player := get_node_or_null(path) as AudioStreamPlayer
        if player == null or player.stream == null: return false
    return true
