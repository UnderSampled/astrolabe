extends Control
## Shared cinematic/gameplay presentation. Select advances; Scroll cycles answers.
signal completed(item: Resource)
const SpanView = preload("res://addons/astrolabe/ui/dialogue_span_view.tscn")
@export var font: Font
@export var panel_style: StyleBox
@export var fade_seconds: float = 0.11
@onready var panel: Control = $Panel
@onready var text: RichTextLabel = $Text
@onready var source_text: Control = $SourceText
@onready var answers: ItemList = $Answers
var item: Resource
var page_start: int = 0
var elapsed: float = 0.0
var mode: int = 1
var closing: bool = false
var fade: Tween
var voice_player: Node
var voice_emitter: Node
var active_voice_identity: String = ""
var active_speech_record: Resource
var _generation: int = 0

func _ready() -> void:
    modulate.a = 0.0
    hide()
    if font != null: text.add_theme_font_override("normal_font", font)
    panel.configure(null, panel_style)

func run(value: Resource, run_mode: int, emitter: Node = null) -> bool:
    if value == null or run_mode != 1 or value.box not in [1, 3]: return false
    cancel()
    _generation += 1
    item = value
    mode = run_mode
    page_start = 0
    elapsed = 0.0
    closing = false
    modulate.a = 0.0
    show()
    panel.visible = item.box != 3
    panel.configure(item.presentation, panel_style)
    # Authored fallback is used by fixtures or a user-authored item without source art.
    text.position = Vector2(4, 0) if item.box == 3 else Vector2(28, 28)
    text.size = Vector2(632, 44) if item.box == 3 else Vector2(278, 126)
    answers.hide()
    _render_page()
    if item.voice != null and emitter != null:
        voice_emitter = emitter
        active_speech_record = item
        emitter.set("source_dialogue_record", active_speech_record)
        active_voice_identity = str(item.voice.identity)
        emitter.voice_started.connect(_voice_started)
        item.voice.execute(emitter)
        emitter.voice_started.disconnect(_voice_started)
        if is_instance_valid(voice_player):
            var selected_record := active_speech_record
            voice_player.finished.connect(func():
                if is_instance_valid(emitter) and emitter.get("source_dialogue_record") == selected_record:
                    emitter.set("source_dialogue_record", null))
        else:
            emitter.set("source_dialogue_record", null)
    fade = create_tween()
    fade.tween_property(self, "modulate:a", 1.0, fade_seconds)
    return true

func _voice_started(info: Dictionary) -> void:
    if str(info.event) == active_voice_identity: voice_player = info.player

func _process(delta: float) -> void:
    if item == null or closing: return
    elapsed += delta
    var clock := elapsed
    if is_instance_valid(voice_player):
        clock = float(voice_player.get_playback_position()) + AudioServer.get_time_since_last_mix()
    var end: int = mini(page_start + item.lines_per_page(), item.lines.size())
    var threshold: float = 3.4e38
    for index in range(page_start, end):
        var time: float = item.lines[index].end_seconds
        if time < 3.0e38: threshold = minf(threshold, time)
    # Blank padding can be untimed. The finite line on this page controls its
    # deadline; an entirely untimed page remains held for SelectDialog.
    if threshold < 3.0e38 and clock >= threshold: select()

func _render_page() -> void:
    var end: int = mini(page_start + item.lines_per_page(), item.lines.size())
    text.clear()
    text.visible = item.presentation == null
    _clear_source_text()
    var style: Resource = item.presentation
    var origin: Vector2 = Vector2.ZERO if style == null else style.text_origin(position, item.box)
    for index in range(page_start, end):
        if index > page_start: text.add_text("\n")
        for span in item.lines[index].spans:
            if style != null:
                var view: Label = SpanView.instantiate()
                view.text = item.message(span.key)
                view.add_theme_font_override("font", style.font)
                view.add_theme_color_override("font_color", style.normal_color)
                if span.emphasis:
                    view.add_theme_color_override("font_outline_color", style.emphasis_color)
                    view.add_theme_constant_override("outline_size", 1)
                view.position = origin + Vector2(style.span_origin(view.text, span.source_offset), (index - page_start) * style.line_spacing())
                source_text.add_child(view)
            if span.emphasis: text.push_color(Color(1.0, 0.83, 0.32))
            # add_text treats source brackets literally, without interpreting
            # game text as BBCode. Source items use the positioned native labels.
            text.add_text(item.message(span.key))
            if span.emphasis: text.pop()

func refresh_layout() -> void:
    if item != null: _render_page()

func scroll(direction: int) -> void:
    if item == null or closing or not answers.visible or answers.item_count < 2: return
    var selected := answers.get_selected_items()
    var index := selected[0] if not selected.is_empty() else 0
    answers.select(posmod(index + (1 if direction >= 1 else -1), answers.item_count))

func select() -> void:
    if item == null or closing or modulate.a < 0.99: return
    if page_start + item.lines_per_page() < item.lines.size():
        page_start += item.lines_per_page()
        _render_page()
    else:
        _finish()

func request_select() -> void:
    if item != null and item.allow_skip: select()

func _finish() -> void:
    closing = true
    # The screen uses a live item to own Button_Hit until the fade completes.
    var finished := item
    var generation := _generation
    _stop_voice()
    if fade != null and fade.is_running(): fade.kill()
    fade = create_tween()
    fade.tween_property(self, "modulate:a", 0.0, fade_seconds)
    fade.tween_callback(func():
        if generation != _generation: return
        item = null
        closing = false
        hide())
    completed.emit(finished)

func is_over() -> bool:
    return item == null and modulate.a <= 0.1

func cancel() -> void:
    _generation += 1
    _stop_voice()
    if fade != null and fade.is_running(): fade.kill()
    item = null
    modulate.a = 0.0
    closing = false
    hide()
    _clear_source_text()
    if is_instance_valid(panel) and panel.is_inside_tree(): panel.configure(null, panel_style)

func _clear_source_text() -> void:
    if not is_instance_valid(source_text) or not source_text.is_inside_tree(): return
    for child in source_text.get_children():
        source_text.remove_child(child)
        child.queue_free()

func _stop_voice() -> void:
    if is_instance_valid(voice_emitter) and active_voice_identity != "":
        voice_emitter.stop_event(active_voice_identity)
    if is_instance_valid(voice_emitter) and voice_emitter.get("source_dialogue_record") == active_speech_record:
        voice_emitter.set("source_dialogue_record", null)
    voice_player = null
    voice_emitter = null
    active_voice_identity = ""
    active_speech_record = null

func _exit_tree() -> void:
    cancel()
