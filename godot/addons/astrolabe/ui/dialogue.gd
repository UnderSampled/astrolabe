extends Control
## Shared cinematic/gameplay presentation. Select advances; Scroll cycles answers.
signal completed(item: Resource)
## 0056e3b0 stored a chosen answer's behavior as the pending callback.
signal answer_selected(answer: Resource)
## AnswerNext/AnswerBad: drawing continues with the named item (0056c700).
signal chain_requested(identity: String)
const SpanView = preload("res://addons/astrolabe/ui/dialogue_span_view.tscn")
const StandInReport = preload("res://addons/astrolabe/behavior/stand_in_report.gd")
@export var font: Font
@export var panel_style: StyleBox
## Authored items without source presentation may choose a Tween duration.
@export var fade_seconds: float = 0.11
@onready var panel: Control = $Panel
@onready var text: RichTextLabel = $Text
@onready var source_text: Control = $SourceText
@onready var answers: ItemList = $Answers
@onready var continuation: TextureButton = $Continue
var item: Resource
var page_start: int = 0
var elapsed: float = 0.0
var mode: int = 1
var closing: bool = false
var fade: Tween
var voice_player: Node
var voice_emitter: Node
var voice_completion: Callable
var main_speech_emitter: Node
var active_voice_identity: String = ""
var active_speech_record: Resource
var _generation: int = 0
const SOURCE_OPEN_STEP_SECONDS: float = 0.010
const SOURCE_ALPHA_STEP: float = 0.10000000149011612
const SOURCE_UNTIMED_SECONDS: float = 3.3999999521443642e38
var _source_presentation: bool = false
var _source_draw_started: bool = false
var _source_opening: bool = false
var _source_opened: bool = false
var _source_closing: bool = false
var _source_step_elapsed: float = 0.0
var _source_page_deadline: float = SOURCE_UNTIMED_SECONDS
var _source_page_complete := false
## Item mode 0 (0056cb10): the last page is drawn with its answers.
var answer_mode := false
var selected_answer: int = 0
var main_actor: Node
var owner_emitter: Node
## 0056cc70 letter reveal for items without speech: label index on the page
## and characters shown of it; more than 2 ms permits one step per draw.
const SOURCE_REVEAL_STEP_SECONDS: float = 0.002
var _reveal_active := false
var _reveal_index: int = 0
var _reveal_characters: int = 1
var _reveal_elapsed: float = 0.0
## AnswerDelete removals, kept for the loaded level's items.
var _removed_answers := {}

func _ready() -> void:
    modulate.a = 0.0
    hide()
    if font != null: text.add_theme_font_override("normal_font", font)
    panel.configure(null, panel_style)
    continuation.pressed.connect(request_select)

func can_run(value: Resource, run_mode: int) -> bool:
    if value == null or run_mode != 1 or value.box not in [1, 3, 5]: return false
    if not value.answers.is_empty() and (value.box != 1 or value.presentation == null or value.presentation.bullet == null): return false
    if value.box == 5:
        if value.presentation == null or value.voice != null or value.lines.is_empty(): return false
        for line in value.lines:
            if line.flags != 3 or line.alignment != Vector2i.ZERO or line.draw_offset != Vector2.ZERO or line.end_seconds < SOURCE_UNTIMED_SECONDS * 0.99: return false
    if value.presentation != null:
        if value.presentation.continuation == null: return false
        for line in value.lines:
            if line.flags != 0 and not (value.box == 5 and line.flags == 3): return false
    return true

func run(value: Resource, run_mode: int, emitter: Node = null, main_emitter: Node = null) -> bool:
    if not can_run(value, run_mode): return false
    cancel()
    _generation += 1
    item = value
    main_speech_emitter = main_emitter
    main_actor = main_emitter.get_parent() if is_instance_valid(main_emitter) else null
    owner_emitter = emitter
    mode = run_mode
    page_start = 0
    answer_mode = false
    selected_answer = 0
    elapsed = 0.0
    closing = false
    _source_presentation = item.presentation != null
    _source_opening = _source_presentation
    modulate.a = 1.0 if _source_presentation else 0.0
    panel.modulate.a = 0.0 if _source_presentation else 1.0
    show()
    panel.visible = item.box != 3
    panel.configure(item.presentation, panel_style, item.box)
    # Authored fallback is used by fixtures or a user-authored item without source art.
    text.position = Vector2(4, 0) if item.box == 3 else Vector2(28, 28)
    text.size = Vector2(632, 44) if item.box == 3 else Vector2(278, 126)
    answers.hide()
    _render_page()
    if item.voice != null and emitter != null:
        voice_emitter = emitter
        active_speech_record = item
        active_voice_identity = str(item.voice.identity)
        if not _source_presentation: _begin_voice()
    if not _source_presentation:
        fade = create_tween()
        fade.tween_property(self, "modulate:a", 1.0, fade_seconds)
    return true

func _begin_voice() -> void:
    if active_speech_record == null or not is_instance_valid(voice_emitter): return
    var emitter := voice_emitter
    emitter.voice_started.connect(_voice_started)
    emitter.play_dialogue_speech(active_speech_record)
    emitter.voice_started.disconnect(_voice_started)
    if is_instance_valid(voice_player):
        var selected_record := active_speech_record
        var generation := _generation
        voice_completion = _speech_completed.bind(emitter, selected_record, generation, voice_player)
        emitter.voice_completed.connect(voice_completion)
    else:
        emitter.set("source_dialogue_record", null)

func _speech_completed(info: Dictionary, emitter: Node, record: Resource, generation: int, player: Node) -> void:
    if info.player != player or generation != _generation: return
    if is_instance_valid(emitter):
        if emitter.get("source_dialogue_record") == record:
            emitter.set("source_dialogue_record", null)
        if voice_completion.is_valid() and emitter.voice_completed.is_connected(voice_completion):
            emitter.voice_completed.disconnect(voice_completion)
    voice_completion = Callable()

func _voice_started(info: Dictionary) -> void:
    if str(info.event) == active_voice_identity: voice_player = info.player

func _process(delta: float) -> void:
    _refresh_continuation()
    if _source_closing:
        # 0056d960: decrement once per draw; the >10ms timer only controls
        # reset/start and does not gate this subtraction.
        var next_alpha := panel.modulate.a - SOURCE_ALPHA_STEP
        if next_alpha < 0.0 or is_nan(next_alpha): _complete_source_close()
        else: panel.modulate.a = float(PackedFloat32Array([next_alpha])[0])
        return
    if item == null or closing: return
    elapsed += delta
    if _source_opening: _source_step_elapsed += delta
    # Active source drawing is gated by MainActor's speech holder. Its timers
    # continue while drawing waits; closing with no active item is independent.
    if _source_presentation and is_instance_valid(main_speech_emitter) and main_speech_emitter.get("source_dialogue_record") != null: return
    if _source_presentation and not _source_draw_started:
        _source_draw_started = true
        _begin_voice()
        # The first draw initializes the zero-alpha panel and returns; the
        # independent opening timer has already run since RunDialog.
        return

    if _source_opening:
        if _source_step_elapsed <= SOURCE_OPEN_STEP_SECONDS: return
        _source_step_elapsed = 0.0
        # 0056d860 compares the unrounded sum before the float32 store.
        # Equality is not terminal; a slow draw never catches up several steps.
        var next_alpha := panel.modulate.a + SOURCE_ALPHA_STEP
        if next_alpha > 1.0:
            _source_opening = false
            _source_opened = true
            source_text.show()
        else:
            panel.modulate.a = float(PackedFloat32Array([next_alpha])[0])
            return
    if _source_presentation:
        # The admitted style5 page bypasses partial reveal for every line.
        # Both null-text padding records count toward the source page draw.
        if _reveal_active: _advance_reveal(delta)
        _source_page_complete = not _reveal_active
        _refresh_continuation()
        # Mode 0 has no speech deadline: the answers wait for SelectDialog.
        if answer_mode: return
        # Source compares playback only while the owner's speech record exists.
        # Its draw overwrites the deadline in line order, never takes a minimum.
        if is_instance_valid(voice_emitter) and voice_emitter.get("source_dialogue_record") != null:
            if is_instance_valid(voice_player):
                var playback := float(voice_player.get_playback_position()) + AudioServer.get_time_since_last_mix()
                if playback > _source_page_deadline: select()
        elif item.voice != null and is_instance_valid(voice_emitter) and page_start + item.lines_per_page() >= item.lines.size() and not is_nan(_source_page_deadline) and _source_page_deadline != SOURCE_UNTIMED_SECONDS:
            # A voiced exhausted page whose holder cleared selects then ends.
            # Supported resources have no successor/choice module to publish.
            select()
        return
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
    continuation.hide()
    var end: int = mini(page_start + item.lines_per_page(), item.lines.size())
    _source_page_deadline = SOURCE_UNTIMED_SECONDS
    _source_page_complete = false
    if page_start < item.lines.size():
        _source_page_deadline = float(PackedFloat32Array([item.lines[page_start].end_seconds])[0])
    text.clear()
    text.visible = item.presentation == null
    source_text.visible = _source_opened
    _clear_source_text()
    var style: Resource = item.presentation
    var origin: Vector2 = Vector2.ZERO if style == null else style.text_origin(position, item.box)
    for index in range(page_start, end):
        var line: Resource = item.lines[index]
        if not line.spans.is_empty() and line.spans[0].text_present:
            _source_page_deadline = float(PackedFloat32Array([line.end_seconds])[0])
        if index > page_start: text.add_text("\n")
        for span in line.spans:
            if not span.text_present: continue
            if style != null:
                var view: Label = SpanView.instantiate()
                view.text = item.message(span.key)
                view.add_theme_font_override("font", style.font)
                view.add_theme_color_override("font_color", style.normal_color)
                if span.emphasis and item.box != 5:
                    view.add_theme_color_override("font_outline_color", style.emphasis_color)
                    view.add_theme_constant_override("outline_size", 1)
                    # The exported outline atlas is straight alpha; only the
                    # source glyph atlas stores premultiplied edges.
                    view.material = null
                # Text starts at its anchor in every box, as in the style5 chain: the
                # original's lines begin at x 22 (letterbox) and 536 (framed, at
                # PositionDialog 305) of 1024 and stay left-aligned; centring on the
                # first glyph put them half a glyph left and ragged.
                var span_x: float = style.chain_span_origin(span.source_offset)
                var line_pitch: float = style.chain_line_spacing() if item.box == 5 else style.line_spacing()
                view.position = origin + Vector2(span_x, (index - page_start) * line_pitch)
                view.set_meta("line_anchor", line)
                view.set_meta("span_origin", view.position)
                view.position += style.line_origin(view.text, line.alignment, line.draw_offset)
                if span.emphasis and item.box == 5:
                    # Source draws the four emphasis passes in this order,
                    # then the normal foreground. The native bitmap font has
                    # the same admitted metrics for both source font roles.
                    var shift: float = style.chain_emphasis_offset()
                    for direction in [Vector2(1, -1), Vector2(1, 1), Vector2(-1, -1), Vector2(-1, 1)]:
                        var outline: Label = view.duplicate()
                        outline.add_theme_color_override("font_color", style.emphasis_color)
                        outline.position += direction * shift
                        source_text.add_child(outline)
                source_text.add_child(view)
            if span.emphasis: text.push_color(Color(1.0, 0.83, 0.32))
            # add_text treats source brackets literally, without interpreting
            # game text as BBCode. Source items use the positioned native labels.
            text.add_text(item.message(span.key))
            if span.emphasis: text.pop()
    # 0056cb10 switches to mode 0 as soon as the last page is drawn; 0056dd50
    # then draws every answer on the following rows.
    answer_mode = style != null and not item.answers.is_empty() and end >= item.lines.size()
    if answer_mode: _render_answers(end - page_start, origin)
    # 0056c2d0 reveals letters only without item speech, on lines whose flag
    # bit 0 does not ask for immediate drawing (style5's flags3 lines do).
    _reveal_active = style != null and item.voice == null and item.box != 5 and source_text.get_child_count() > 0
    for index in range(page_start, end):
        if (item.lines[index].flags & 1) != 0: _reveal_active = false
    _reveal_index = 0
    _reveal_characters = 1
    _reveal_elapsed = 0.0
    if _reveal_active: _apply_reveal()

func _apply_reveal() -> void:
    var style: Resource = item.presentation
    for index in range(source_text.get_child_count()):
        var label := source_text.get_child(index) as Label
        if label == null or not label.has_meta("line_anchor"): continue
        if not _reveal_active or index < _reveal_index: label.visible_characters = -1
        elif index == _reveal_index: label.visible_characters = _reveal_characters
        else: label.visible_characters = 0
        var line: Resource = label.get_meta("line_anchor")
        var prefix: String = label.text if label.visible_characters < 0 else label.text.left(label.visible_characters)
        # FON centres the string being drawn, so a centred line re-centres as
        # it grows (the take's "Res historicae" title).
        label.position = label.get_meta("span_origin") + style.line_origin(prefix, line.alignment, line.draw_offset)

func _advance_reveal(delta: float) -> void:
    _apply_reveal()
    _reveal_elapsed += delta
    if _reveal_elapsed <= SOURCE_REVEAL_STEP_SECONDS: return
    _reveal_elapsed = 0.0
    var label := source_text.get_child(_reveal_index) as Label if _reveal_index < source_text.get_child_count() else null
    if label != null and _reveal_characters < label.text.length():
        _reveal_characters += 1
    elif _reveal_index + 1 < source_text.get_child_count():
        _reveal_index += 1
        _reveal_characters = 1
    else:
        _reveal_active = false
    _apply_reveal()

func _active_answers() -> Array:
    var removed: Array = _removed_answers.get(item.identity, [])
    return item.answers.filter(func(answer: Resource) -> bool: return not removed.has(answer))

func _render_answers(row: int, origin: Vector2) -> void:
    var style: Resource = item.presentation
    var choices := _active_answers()
    for index in choices.size():
        var selected: bool = index == selected_answer
        for line in choices[index].lines:
            if selected:
                var bullet := TextureRect.new()
                bullet.texture = style.bullet
                bullet.mouse_filter = Control.MOUSE_FILTER_IGNORE
                bullet.position = style.bullet_origin(row)
                source_text.add_child(bullet)
            for span in line.spans:
                if not span.text_present: continue
                var view: Label = SpanView.instantiate()
                view.text = item.message(span.key)
                view.add_theme_font_override("font", style.font)
                view.add_theme_color_override("font_color", style.normal_color)
                view.position = origin + Vector2(style.answer_indent() + style.chain_span_origin(span.source_offset), row * style.line_spacing())
                if selected:
                    # Four offset copies in the third root font, then the
                    # answer's own font on top.
                    var shift: float = style.chain_emphasis_offset()
                    for direction in [Vector2(1, -1), Vector2(1, 1), Vector2(-1, -1), Vector2(-1, 1)]:
                        var halo: Label = view.duplicate()
                        halo.add_theme_color_override("font_color", style.selection_color)
                        halo.position += direction * shift
                        source_text.add_child(halo)
                source_text.add_child(view)
            row += 1

func _refresh_continuation() -> void:
    if not is_instance_valid(continuation): return
    var main_speech_wait := is_instance_valid(main_speech_emitter) and main_speech_emitter.get("source_dialogue_record") != null
    # 0056c2d0 draws the arrow only after the page, and only with a successor
    # page. Finite final speech pages complete through the speech holder.
    continuation.visible = item != null and not closing and _source_presentation and _source_opened and _source_page_complete and not main_speech_wait and page_start + item.lines_per_page() < item.lines.size()
    if not continuation.visible: return
    continuation.texture_normal = item.presentation.continuation
    continuation.position = item.presentation.continuation_origin(position, item.box, get_viewport_rect().size)
    continuation.disabled = not item.allow_skip

func refresh_layout() -> void:
    if item != null: _render_page()

func scroll(direction: int) -> void:
    if item == null or closing: return
    if _source_presentation:
        # 0056f3a0: only on the answer page with two or more answers; wraps.
        var count := _active_answers().size()
        if not answer_mode or count < 2: return
        selected_answer = posmod(selected_answer + (1 if direction > 0 else -1), count)
        _play_dialog_sound(item.presentation.scroll_sound)
        _render_page()
        return
    if not answers.visible or answers.item_count < 2: return
    var selected := answers.get_selected_items()
    var index := selected[0] if not selected.is_empty() else 0
    answers.select(posmod(index + (1 if direction >= 1 else -1), answers.item_count))

func select() -> void:
    if item == null or closing or modulate.a < 0.99: return
    if _source_presentation and not _source_opened: return
    if _source_presentation and page_start < item.lines.size() and (item.lines[page_start].flags & 2) != 0 and not _source_page_complete: return
    if _reveal_active:
        # 0056e510: a press during the reveal completes the page.
        _reveal_active = false
        _apply_reveal()
        _source_page_complete = true
        return
    if answer_mode:
        _choose_answer()
    elif page_start + item.lines_per_page() < item.lines.size():
        page_start += item.lines_per_page()
        _render_page()
    else:
        _finish()

## 0056e3b0: cost check, pending callback, answer speech, AnswerDelete, then
## the next item or the closing fade.
func _choose_answer() -> void:
    var choices := _active_answers()
    if choices.is_empty():
        _finish()
        return
    var answer: Resource = choices[selected_answer]
    var next: String = answer.next
    var code := _answer_cost(answer)
    if code != -1:
        # AnswerBad; without a mapping the same item restarts.
        next = answer.shortfall_item(code)
        if next.is_empty(): next = item.identity
    else:
        answer_selected.emit(answer)
        _stop_voice()
        if answer.voice != null and is_instance_valid(main_speech_emitter):
            main_speech_emitter.play_dialogue_speech(answer)
        if answer.one_shot:
            var removed: Array = _removed_answers.get(item.identity, [])
            removed.append(answer)
            _removed_answers[item.identity] = removed
    selected_answer = 0
    if next.is_empty(): _finish()
    else: chain_requested.emit(next)

## 0056e2c0 for gold-only answers: 0056e0e0 adds a non-negative amount and
## spends a negative one; a shortfall is failure code 1.
func _answer_cost(answer: Resource) -> int:
    if answer.gold == 0: return -1
    var inventory: Resource = main_actor.get("source_inventory") if is_instance_valid(main_actor) else null
    if inventory == null:
        StandInReport.reached(self, "guard", "dialogue answer gold without the main actor's inventory")
        return -1
    var paid: int = 1 if answer.gold >= 0 else int(inventory.subtract_gold(-answer.gold))
    if answer.gold >= 0: inventory.add_gold(answer.gold)
    if paid == 0: return 1
    _play_dialog_sound(item.presentation.cash_sound)
    return -1

## 004ec880(-1, effects, event): played without a position, through the main
## actor's emitter for the mixer and buses.
func _play_dialog_sound(event: Resource) -> void:
    var emitter := main_speech_emitter
    if event == null or not is_instance_valid(emitter) or not emitter.has_method("execute_global_event"):
        StandInReport.reached(self, "guard", "SoundEffects:Dialog event or emitter unavailable")
        return
    emitter.execute_global_event(event)

## InitDraw on a following item: the box is cut and opens again with the
## source fade (the take shows Wellet's box vanish and fade back in), after
## MainActor's speech holder is free; the item's own speech starts at that draw.
func chain(value: Resource) -> bool:
    if value == null or item == null or closing or not can_run(value, mode): return false
    _stop_voice()
    _generation += 1
    item = value
    page_start = 0
    elapsed = 0.0
    answer_mode = false
    selected_answer = 0
    _source_draw_started = false
    _source_opening = true
    _source_opened = false
    _source_step_elapsed = 0.0
    panel.modulate.a = 0.0
    source_text.hide()
    panel.configure(item.presentation, panel_style, item.box)
    _render_page()
    if item.voice != null and is_instance_valid(owner_emitter):
        voice_emitter = owner_emitter
        active_speech_record = item
        active_voice_identity = str(item.voice.identity)
    return true

func request_select() -> void:
    if item != null and item.allow_skip: select()

func end(animated: bool = true) -> void:
    if not animated:
        cancel()
    elif item != null and not closing:
        _finish(false)

func _finish(notify_completion: bool = true) -> void:
    closing = true
    continuation.hide()
    # The screen uses a live item to own Button_Hit until the fade completes.
    var finished := item
    var generation := _generation
    if not notify_completion:
        item = null
        text.clear()
        _clear_source_text()
        answers.hide()
    _stop_voice()
    if _source_presentation:
        # EndDraw clears text immediately and starts the shared stencil close.
        text.clear()
        _clear_source_text()
        source_text.hide()
        _source_opening = false
        if _source_opened:
            panel.modulate.a = 1.0
            _source_closing = true
        else:
            _complete_source_close()
        if notify_completion: completed.emit(finished)
        return
    if fade != null and fade.is_running(): fade.kill()
    fade = create_tween()
    fade.tween_property(self, "modulate:a", 0.0, fade_seconds)
    fade.tween_callback(func():
        if generation != _generation: return
        item = null
        closing = false
        hide())
    if notify_completion: completed.emit(finished)

func is_over() -> bool:
    # 0056c8c0/0056da60 clear stencil alpha once the current dialog is gone
    # and alpha is at most double0.1. This query can finish a pending close.
    if _source_closing:
        if panel.modulate.a > 0.1: return false
        _complete_source_close()
    return item == null and modulate.a <= 0.1

func _complete_source_close() -> void:
    item = null
    modulate.a = 0.0
    closing = false
    _source_opening = false
    _source_opened = false
    _source_closing = false
    panel.modulate.a = 0.0
    source_text.hide()
    hide()

func cancel() -> void:
    _generation += 1
    _stop_voice()
    if fade != null and fade.is_running(): fade.kill()
    item = null
    modulate.a = 0.0
    closing = false
    _source_presentation = false
    _source_draw_started = false
    _source_opening = false
    _source_opened = false
    _source_closing = false
    _source_step_elapsed = 0.0
    _source_page_deadline = SOURCE_UNTIMED_SECONDS
    answer_mode = false
    selected_answer = 0
    main_speech_emitter = null
    main_actor = null
    owner_emitter = null
    if is_instance_valid(continuation): continuation.hide()
    if is_instance_valid(panel): panel.modulate.a = 1.0
    hide()
    _clear_source_text()
    if is_instance_valid(panel) and panel.is_inside_tree(): panel.configure(null, panel_style)

func _clear_source_text() -> void:
    if not is_instance_valid(source_text) or not source_text.is_inside_tree(): return
    for child in source_text.get_children():
        source_text.remove_child(child)
        child.queue_free()

func _stop_voice() -> void:
    if is_instance_valid(voice_emitter) and voice_completion.is_valid() and voice_emitter.voice_completed.is_connected(voice_completion):
        voice_emitter.voice_completed.disconnect(voice_completion)
    voice_completion = Callable()
    if is_instance_valid(voice_emitter) and voice_emitter.get("source_dialogue_record") == active_speech_record:
        voice_emitter.stop_dialogue_speech()
    voice_player = null
    voice_emitter = null
    active_voice_identity = ""
    active_speech_record = null

func _exit_tree() -> void:
    cancel()
