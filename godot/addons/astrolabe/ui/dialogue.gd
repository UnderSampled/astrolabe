extends Control
## Shared cinematic/gameplay presentation. Select advances; Scroll cycles answers.
signal completed(item: Resource)
@export var font: Font
@export var panel_style: StyleBox
@export var fade_seconds: float = 0.11
@onready var panel: Panel = $Panel
@onready var text: RichTextLabel = $Text
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
var _generation: int = 0

func _ready() -> void:
    hide()
    if font != null: text.add_theme_font_override("normal_font", font)
    if panel_style != null: panel.add_theme_stylebox_override("panel", panel_style)

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
    # Source has the same 20.5px line spacing in both modes. Box 3 is unframed.
    text.position = Vector2(4, 0) if item.box == 3 else Vector2(28, 28)
    text.size = Vector2(632, 44) if item.box == 3 else Vector2(278, 126)
    answers.hide()
    _render_page()
    if item.voice != null and emitter != null:
        voice_emitter = emitter
        active_voice_identity = str(item.voice.identity)
        emitter.voice_started.connect(_voice_started)
        item.voice.execute(emitter)
        emitter.voice_started.disconnect(_voice_started)
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
    for index in range(page_start, end):
        if index > page_start: text.add_text("\n")
        for span in item.lines[index].spans:
            if span.emphasis: text.push_color(Color(1.0, 0.83, 0.32))
            # add_text treats source brackets literally, without interpreting
            # game text as BBCode. Exact source-font offsets remain data.
            text.add_text(item.message(span.key))
            if span.emphasis: text.pop()

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
        hide()
        completed.emit(finished))

func is_over() -> bool:
    return item == null and not closing

func cancel() -> void:
    _generation += 1
    _stop_voice()
    if fade != null and fade.is_running(): fade.kill()
    item = null
    closing = false
    hide()

func _stop_voice() -> void:
    if is_instance_valid(voice_emitter) and active_voice_identity != "":
        voice_emitter.stop_event(active_voice_identity)
    voice_player = null
    voice_emitter = null
    active_voice_identity = ""

func _exit_tree() -> void:
    cancel()
