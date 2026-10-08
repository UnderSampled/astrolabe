extends CanvasLayer
## One presentation owner per active level; authored parents may adopt content.
signal widescreen_changed(enabled: bool)
signal locale_fallback(requested: String, selected: String)
@export var dialogue_items: Array[Resource] = []
@export var adopt_parent_screen: bool = true
@export_node_path("Node") var actor_path: NodePath
@onready var canvas: Control = $Canvas
@onready var dialogue: Control = $Canvas/Dialogue
@onready var location_title: Control = $Canvas/LocationTitle
var active_presentation: Control
@onready var hud: Control = $Canvas/Hud
@onready var bars: Control = $Canvas/Letterbox
@onready var fade: ColorRect = $Canvas/Fade
var widescreen: bool = false
var adopted_screen: Node
var source_level: Node
var source_position := Vector2.ZERO
var completion_brain: Node
var completion_model: String = ""
var completion_normal: int = -1
var completion_pending: bool = false
var _callback_generation: int = 0
var _source_fade_tween: Tween
var _source_fade_direction: int = 0
var _authored_items: Array[Resource] = []
var _catalogues: Array[Translation] = []

func _ready() -> void:
    source_level = get_parent()
    _authored_items.assign(dialogue_items)
    if adopt_parent_screen:
        var ancestor: Node = get_parent()
        while ancestor != null:
            var candidate: Node = ancestor.get_node_or_null("AstrolabeUI")
            if candidate != null and candidate != self and candidate.is_node_ready() and candidate.has_method("adopt_level"):
                var actor: Node = get_node_or_null(actor_path) if not actor_path.is_empty() else null
                if candidate.adopt_level(source_level, dialogue_items, actor):
                    adopted_screen = candidate
                    canvas.hide()
                    set_process(false)
                    return
            ancestor = ancestor.get_parent()
    dialogue.completed.connect(_dialogue_completed)
    location_title.completed.connect(_dialogue_completed)
    get_viewport().size_changed.connect(_layout)
    _register_catalogues()
    _layout()
    if not actor_path.is_empty(): hud.bind_actor(get_node_or_null(actor_path))

func adopt_level(level: Node, items: Array[Resource], actor: Node) -> bool:
    # Authored scene composition chooses one active level. Never steal another
    # live level's presentation owner or duplicate its HUD/dialogue coroutine.
    if is_instance_valid(source_level) and source_level != get_parent() and source_level != level: return false
    source_cancel_dialog()
    source_level = level
    dialogue_items.assign(items)
    _register_catalogues()
    hud.bind_actor(actor)
    return true

func release_level(level: Node) -> void:
    if source_level != level: return
    source_cancel_dialog()
    source_level = get_parent()
    dialogue_items.assign(_authored_items)
    _register_catalogues()
    hud.bind_actor(null)

func _register_catalogues() -> void:
    for catalogue in _catalogues: TranslationServer.remove_translation(catalogue)
    _catalogues.clear()
    for value in dialogue_items:
        if value.translation != null and not _catalogues.has(value.translation):
            _catalogues.append(value.translation)
            TranslationServer.add_translation(value.translation)

func _layout() -> void:
    canvas.scale = get_viewport().get_visible_rect().size / Vector2(640, 480)
    bars.visible = widescreen
    hud.suppressed = widescreen
    hud.refresh()
    fade.position = Vector2(0, 60 if widescreen else 0)
    fade.size = Vector2(640, 360 if widescreen else 480)

func source_toggle_widescreen() -> void:
    if is_instance_valid(adopted_screen):
        adopted_screen.source_toggle_widescreen()
        return
    widescreen = not widescreen
    _layout()
    widescreen_changed.emit(widescreen)

func source_execute_fade(from_alpha: int, to_alpha: int, duration_seconds: float) -> bool:
    if is_instance_valid(adopted_screen):
        if not adopted_screen.has_method("source_execute_fade"):
            return false
        return bool(adopted_screen.call("source_execute_fade", from_alpha, to_alpha, duration_seconds))
    if not is_finite(duration_seconds) or duration_seconds < 0.0 or from_alpha < 0 or from_alpha > 255 or to_alpha < 0 or to_alpha > 255:
        return false
    var start_alpha := clampf(float(from_alpha) / 255.0, 0.0, 1.0)
    var target_alpha := clampf(float(to_alpha) / 255.0, 0.0, 1.0)
    fade.color = Color(0.0, 0.0, 0.0, start_alpha)
    # Equal endpoints leave an already active source direction unchanged.
    if start_alpha == target_alpha: return true
    if _source_fade_tween != null and _source_fade_tween.is_valid():
        _source_fade_tween.kill()
    if duration_seconds == 0.0:
        fade.color = Color(0.0, 0.0, 0.0, target_alpha)
        _source_fade_direction = 0
        return true
    _source_fade_direction = 1 if target_alpha > start_alpha else -1
    _source_fade_tween = create_tween()
    _source_fade_tween.tween_property(fade, "color:a", target_alpha, duration_seconds)
    _source_fade_tween.finished.connect(_source_fade_finished.bind(_source_fade_tween))
    return true

func _source_fade_finished(completed: Tween) -> void:
    if completed == _source_fade_tween:
        _source_fade_direction = 0

## Meta61 observes the shared fade's direction. Visibility and current alpha
## do not decide completion; cancelling a fade makes its direction idle.
func source_fade_in_progress() -> Variant:
    if is_instance_valid(adopted_screen):
        if not adopted_screen.has_method("source_fade_in_progress"): return null
        return adopted_screen.call("source_fade_in_progress")
    return _source_fade_direction != 0

## Empty STE_FADE slots are accepted by the source engine as no-ops. Never
## substitute an unrelated texture or the authored black fade rectangle.
func source_set_fade_texture(index: int) -> bool:
    if index < 0 or index > 9:
        return false
    if is_instance_valid(adopted_screen):
        if not adopted_screen.has_method("source_set_fade_texture"): return false
        return bool(adopted_screen.call("source_set_fade_texture", index))
    # No source binding is present in the imported scene. The original
    # handler leaves the initialized material untextured and returns normally.
    return true

func source_set_fade_value(value: int) -> bool:
    if is_instance_valid(adopted_screen):
        if not adopted_screen.has_method("source_set_fade_value"): return false
        return bool(adopted_screen.call("source_set_fade_value", value))
    if value < 0 or value > 255: return false
    fade.color.a = float(value) / 255.0
    return true

func source_activate_fade() -> void:
    if is_instance_valid(adopted_screen):
        adopted_screen.call("source_activate_fade")
        return
    fade.show()

func source_deactivate_fade() -> void:
    if is_instance_valid(adopted_screen):
        adopted_screen.call("source_deactivate_fade")
        return
    fade.hide()

func source_position_dialog(x: float, y: float) -> void:
    if is_instance_valid(adopted_screen):
        adopted_screen.source_position_dialog(x, y)
        return
    source_position = Vector2(x, y)
    dialogue.position = source_position
    dialogue.refresh_layout()
    location_title.position = source_position
    location_title.refresh_layout()

func source_run_dialog(identity: String, run_mode: int, caller: Node = null) -> bool:
    if is_instance_valid(adopted_screen): return adopted_screen.source_run_dialog(identity, run_mode, caller)
    var selected: Resource
    var requested: String = TranslationServer.get_locale().get_slice("_", 0)
    for value in dialogue_items:
        if value.identity.to_lower() != identity.to_lower(): continue
        if value.locale == requested:
            selected = value
            break
        if value.locale == "en": selected = value
    if selected == null: return false
    # 00543c00 overwrites the active DLG owner with the executing actor. The
    # saved owner remains useful conversion metadata and an authored default.
    var actor: Node = _actor(selected.owner_identity) if caller == null else caller.get_parent()
    if caller != null and (actor == null or actor.get_node_or_null("Brain") != caller): return false
    var emitter: Node = actor.get_node_or_null("SoundEmitter") if actor != null else null
    var brain: Node = actor.get_node_or_null("Brain") if actor != null else null
    if selected.voice != null and (emitter == null or not emitter.has_method("play_dialogue_speech") or not emitter.has_method("stop_dialogue_speech")): return false
    if selected.completion_normal >= 0 and (brain == null or str(brain.get("source_model_path")) != selected.completion_model): return false
    var presenter: Control = location_title if selected.box == 4 else dialogue
    # RunDialog first clears MainActor's speech holder (0041e910). Validate
    # the selected native presentation before changing any live voice.
    if not presenter.can_run(selected, run_mode): return false
    var main_actor := _source_main_actor()
    var main_emitter := main_actor.get_node_or_null("SoundEmitter") if main_actor != null else null
    if main_emitter != null and main_emitter.has_method("stop_dialogue_speech"):
        main_emitter.call("stop_dialogue_speech")
    # Validate/run first so an unavailable item cannot cancel the live one.
    if not presenter.run(selected, run_mode, emitter, main_emitter): return false
    if presenter == location_title: dialogue.cancel()
    else: location_title.cancel()
    active_presentation = presenter
    _callback_generation += 1
    completion_pending = false
    completion_brain = brain
    completion_model = selected.completion_model
    completion_normal = selected.completion_normal
    if selected.locale != requested: locale_fallback.emit(requested, selected.locale)
    return true

func source_scroll_dialog(direction: int) -> void:
    if is_instance_valid(adopted_screen): adopted_screen.source_scroll_dialog(direction)
    elif active_presentation == dialogue: dialogue.scroll(direction)

func source_select_dialog() -> void:
    if is_instance_valid(adopted_screen): adopted_screen.source_select_dialog()
    elif is_instance_valid(active_presentation): active_presentation.request_select()

func _unhandled_input(event: InputEvent) -> void:
    # The source Button_Hit action advances an active dialogue. Menus and
    # gameplay retain their existing input ownership outside this component.
    if is_instance_valid(adopted_screen) or not is_instance_valid(active_presentation) or active_presentation.item == null:
        return
    var key := event as InputEventKey
    if not event.is_action_pressed(&"Button_Hit") or (key != null and (key.echo or not key.pressed)):
        return
    source_select_dialog()
    get_viewport().set_input_as_handled()

func _clear_completion() -> void:
    _callback_generation += 1
    completion_pending = false
    completion_brain = null
    completion_model = ""
    completion_normal = -1

func source_end_dialog() -> void:
    if is_instance_valid(adopted_screen):
        adopted_screen.source_end_dialog()
        return
    # Source End is unconditional: close whichever presentation owns the screen,
    # including an ordinary dialogue when the title actor's predicate was false.
    _clear_completion()
    if is_instance_valid(active_presentation): active_presentation.end()

func source_cancel_dialog() -> void:
    if is_instance_valid(adopted_screen):
        adopted_screen.source_cancel_dialog()
        return
    _clear_completion()
    dialogue.cancel()
    location_title.cancel()
    active_presentation = null

func source_dialog_is_over(main_actor: Node3D = null) -> bool:
    if is_instance_valid(adopted_screen): return adopted_screen.source_dialog_is_over(main_actor)
    if not dialogue.is_over() or not location_title.is_over(): return false
    # The source gate belongs to MainActor, independently of the current
    # dialogue owner and unrelated object effects or music.
    if main_actor == null: main_actor = _source_main_actor()
    if not is_instance_valid(main_actor): return true
    var emitter := main_actor.get_node_or_null("SoundEmitter")
    return emitter != null and emitter.get("source_dialogue_record") == null

func source_dialog_is_over_and_change_behavior(main_actor: Node3D = null) -> bool:
    if is_instance_valid(adopted_screen): return adopted_screen.source_dialog_is_over_and_change_behavior(main_actor)
    if not source_dialog_is_over(main_actor): return false
    return source_change_dialog_behavior()

func source_change_dialog_behavior() -> bool:
    if is_instance_valid(adopted_screen): return adopted_screen.source_change_dialog_behavior()
    if not completion_pending or not dialogue.is_over() or not location_title.is_over(): return true
    completion_pending = false
    if completion_normal < 0: return true
    if not is_instance_valid(completion_brain) or str(completion_brain.get("source_model_path")) != completion_model: return false
    _apply_completion.call_deferred(_callback_generation, completion_brain, completion_model, completion_normal)
    return true

func _actor(identity: String) -> Node:
    if not is_instance_valid(source_level) or not source_level.has_meta("source_actor_nodes"): return null
    var paths: Dictionary = source_level.get_meta("source_actor_nodes")
    return source_level.get_node_or_null(paths[identity]) if paths.has(identity) else null

func _source_main_actor() -> Node3D:
    if not is_instance_valid(source_level) or not source_level.has_meta("source_main_actor"): return null
    return _actor(str(source_level.get_meta("source_main_actor"))) as Node3D

func _dialogue_completed(_item: Resource) -> void:
    # The source sets a pending callback. Proc_DLG_ChangeComport consumes it;
    # Cond_DLG_IsDialogOver must not change the owner's Brain by itself.
    completion_pending = true

func _apply_completion(generation: int, brain: Node, model: String, normal: int) -> void:
    if generation != _callback_generation: return
    if not is_instance_valid(brain) or str(brain.get("source_model_path")) != model: return
    brain.change_normal(normal, true)

func _exit_tree() -> void:
    _callback_generation += 1
    if is_instance_valid(adopted_screen): adopted_screen.release_level(source_level)
    elif is_instance_valid(dialogue) and is_instance_valid(location_title): source_cancel_dialog()
    for catalogue in _catalogues: TranslationServer.remove_translation(catalogue)
