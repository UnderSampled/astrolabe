extends Control
## Style-4 dialogue presentation. The compiled source AI owns when to run/end it.
## Uses the existing exported Translation, FontFile and stencil resources.
signal completed(item: Resource)
const SpanView = preload("res://addons/astrolabe/ui/dialogue_span_view.tscn")
const OPEN_STEP_SECONDS: float = 0.010
const REVEAL_STEP_SECONDS: float = 0.002
# The executable operands are ±float32 0x3dcccccd, then compared before rounding.
const ALPHA_STEP: float = 0.10000000149011612
@onready var panel: TextureRect = $Panel
@onready var source_text: Control = $SourceText
var item: Resource
var _opening: bool = false
var _opened: bool = false
var _closing: bool = false
var _step_elapsed: float = 0.0
var _reveal_elapsed: float = 0.0
var _reveal_index: int = 0
var _reveal_characters: int = 1
var _revealed: bool = false

func _ready() -> void:
    cancel()

func can_run(value: Resource, run_mode: int) -> bool:
    if value == null or run_mode != 1 or value.box != 4: return false
    var style: Resource = value.presentation
    if style == null or style.font == null or style.center == null: return false
    if style.corner == null or style.border == null or style.side == null: return false
    if value.voice != null: return false
    if value.lines.is_empty() or value.lines.size() > 2: return false
    for line in value.lines:
        if line.flags != 0 or line.end_seconds < 3.0e38: return false
    return true

func run(value: Resource, run_mode: int, _emitter: Node = null) -> bool:
    if not can_run(value, run_mode): return false
    var style: Resource = value.presentation
    cancel()
    item = value
    panel.texture = style.center
    refresh_layout()
    _opening = true
    show()
    return true

func refresh_layout() -> void:
    if item == null: return
    var style: Resource = item.presentation
    # 0056cf90: style 4 returns after drawing the center stencil. There are no
    # corner/edge sprites in this branch. Dimensions come from source textures.
    var corner_size := Vector2(style.corner.get_width(), style.corner.get_height())
    panel.position = Vector2(style.corner.get_width() >> 1, style.corner.get_height() >> 1)
    var columns := floori(float(style.border.get_width()) / style.center.get_width())
    var rows := floori(float(style.side.get_height()) / style.center.get_height())
    panel.size = Vector2(8.0 * columns * style.center.get_width(), 2.0 * rows * style.center.get_height()) + corner_size * 1.5 - panel.position
    size = panel.position + panel.size
    _clear_text()
    var origin: Vector2 = style.text_origin(position, 4)
    for index in range(item.lines.size()):
        for span in item.lines[index].spans:
            var view: Label = SpanView.instantiate()
            view.text = item.message(span.key)
            view.add_theme_font_override("font", style.font)
            view.add_theme_color_override("font_color", style.normal_color)
            if span.emphasis:
                view.add_theme_color_override("font_outline_color", style.emphasis_color)
                view.add_theme_constant_override("outline_size", 1)
            view.position = origin + Vector2(style.span_origin(view.text, span.source_offset), index * style.line_spacing())
            view.set_meta("line_anchor", item.lines[index])
            view.set_meta("span_origin", view.position)
            view.visible_characters = 0
            source_text.add_child(view)
    _apply_reveal()

func _process(delta: float) -> void:
    if _opening:
        _step_elapsed += delta
        # Source text timer starts with RunDialog and keeps running while the
        # independent frame alpha timer is reset during opening.
        _reveal_elapsed += delta
        # Source opening advances once per draw only after >10 ms, resetting
        # the timer. A slow draw must not catch up with several alpha steps.
        if _step_elapsed <= OPEN_STEP_SECONDS: return
        _step_elapsed = 0.0
        var next_alpha := panel.modulate.a + ALPHA_STEP
        if next_alpha > 1.0:
            _opening = false
            _opened = true
            source_text.show()
            _advance_reveal(0.0)
        else:
            panel.modulate.a = float(PackedFloat32Array([next_alpha])[0])
    elif _opened and not _closing and item != null:
        _advance_reveal(delta)
    elif _closing:
        # 0056d960's decrement is outside the timer branch: one per draw.
        var next_alpha := panel.modulate.a - ALPHA_STEP
        if next_alpha < 0.0 or is_nan(next_alpha):
            cancel()
        else:
            panel.modulate.a = float(PackedFloat32Array([next_alpha])[0])

func _apply_reveal() -> void:
    for index in range(source_text.get_child_count()):
        var label: Label = source_text.get_child(index)
        if _revealed or index < _reveal_index: label.visible_characters = -1
        elif index == _reveal_index: label.visible_characters = _reveal_characters
        else: label.visible_characters = 0
        var line: Resource = label.get_meta("line_anchor")
        var prefix: String = label.text if label.visible_characters < 0 else label.text.left(label.visible_characters)
        label.position = label.get_meta("span_origin") + item.presentation.line_origin(prefix, line.alignment, line.draw_offset)

func _advance_reveal(delta: float) -> void:
    if _revealed: return
    # 0056cc70 draws the current prefix before advancing it. More than 2 ms
    # permits one character (or one span/line boundary) per draw, with no catchup.
    _apply_reveal()
    _reveal_elapsed += delta
    if _reveal_elapsed <= REVEAL_STEP_SECONDS: return
    _reveal_elapsed = 0.0
    if _reveal_index >= source_text.get_child_count():
        _revealed = true
        _apply_reveal()
        return
    var label: Label = source_text.get_child(_reveal_index)
    if _reveal_characters < label.text.length():
        _reveal_characters += 1
    elif _reveal_index + 1 < source_text.get_child_count():
        _reveal_index += 1
        _reveal_characters = 1
    else:
        _revealed = true
        _apply_reveal()

func end(animated: bool = true) -> void:
    item = null
    source_text.hide()
    _clear_text()
    _opening = false
    if not animated or not _opened:
        cancel()
        return
    panel.modulate.a = 1.0
    _closing = true

func select() -> void:
    # DLG_Select ignores opening; the first reveal-time press completes the
    # current page. A later press stores the represented completion through
    # Screen's common pending-callback contract.
    if item == null or not _opened: return
    if not _revealed:
        _revealed = true
        _apply_reveal()
    else:
        var finished := item
        end()
        completed.emit(finished)

func request_select() -> void:
    if item != null and item.allow_skip: select()

func is_over() -> bool:
    if item != null: return false
    if panel.modulate.a > 0.1: return false
    cancel()
    return true

func cancel() -> void:
    item = null
    _opening = false
    _opened = false
    _closing = false
    _step_elapsed = 0.0
    _reveal_elapsed = 0.0
    _reveal_index = 0
    _reveal_characters = 1
    _revealed = false
    if is_instance_valid(panel):
        panel.modulate.a = 0.0
        panel.texture = null
    if is_instance_valid(source_text):
        source_text.hide()
        _clear_text()
    hide()

func _clear_text() -> void:
    if not is_instance_valid(source_text): return
    for child in source_text.get_children():
        source_text.remove_child(child)
        child.queue_free()

func _exit_tree() -> void:
    cancel()
