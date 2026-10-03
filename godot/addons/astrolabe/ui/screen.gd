extends CanvasLayer
## One presentation owner per active level; authored parents may adopt content.
signal widescreen_changed(enabled: bool)
signal locale_fallback(requested: String, selected: String)
@export var dialogue_items: Array[Resource] = []
@export var adopt_parent_screen: bool = true
@export_node_path("Node") var actor_path: NodePath
@onready var canvas: Control = $Canvas
@onready var dialogue: Control = $Canvas/Dialogue
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
    get_viewport().size_changed.connect(_layout)
    _register_catalogues()
    _layout()
    if not actor_path.is_empty(): hud.bind_actor(get_node_or_null(actor_path))

func adopt_level(level: Node, items: Array[Resource], actor: Node) -> bool:
    # Authored scene composition chooses one active level. Never steal another
    # live level's presentation owner or duplicate its HUD/dialogue coroutine.
    if is_instance_valid(source_level) and source_level != get_parent() and source_level != level: return false
    source_end_dialog()
    source_level = level
    dialogue_items.assign(items)
    _register_catalogues()
    hud.bind_actor(actor)
    return true

func release_level(level: Node) -> void:
    if source_level != level: return
    source_end_dialog()
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

func source_position_dialog(x: float, y: float) -> void:
    if is_instance_valid(adopted_screen):
        adopted_screen.source_position_dialog(x, y)
        return
    source_position = Vector2(x, y)
    dialogue.position = source_position
    dialogue.refresh_layout()

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
    var actor: Node = _actor(selected.owner_identity)
    var emitter: Node = actor.get_node_or_null("SoundEmitter") if actor != null else null
    var brain: Node = actor.get_node_or_null("Brain") if actor != null else null
    if selected.voice != null and emitter == null: return false
    if selected.completion_normal >= 0 and (brain == null or str(brain.get("source_model_path")) != selected.completion_model): return false
    if not dialogue.run(selected, run_mode, emitter): return false
    _callback_generation += 1
    completion_pending = false
    completion_brain = brain
    completion_model = selected.completion_model
    completion_normal = selected.completion_normal
    if selected.locale != requested: locale_fallback.emit(requested, selected.locale)
    return true

func source_scroll_dialog(direction: int) -> void:
    if is_instance_valid(adopted_screen): adopted_screen.source_scroll_dialog(direction)
    else: dialogue.scroll(direction)

func source_select_dialog() -> void:
    if is_instance_valid(adopted_screen): adopted_screen.source_select_dialog()
    else: dialogue.request_select()

func source_end_dialog() -> void:
    if is_instance_valid(adopted_screen):
        adopted_screen.source_end_dialog()
        return
    _callback_generation += 1
    completion_pending = false
    completion_brain = null
    completion_model = ""
    completion_normal = -1
    dialogue.cancel()

func source_dialog_is_over() -> bool:
    return adopted_screen.source_dialog_is_over() if is_instance_valid(adopted_screen) else dialogue.is_over()

func source_change_dialog_behavior() -> bool:
    if is_instance_valid(adopted_screen): return adopted_screen.source_change_dialog_behavior()
    if not completion_pending or not dialogue.is_over(): return true
    completion_pending = false
    if completion_normal < 0: return true
    if not is_instance_valid(completion_brain) or str(completion_brain.get("source_model_path")) != completion_model: return false
    _apply_completion.call_deferred(_callback_generation, completion_brain, completion_model, completion_normal)
    return true

func _actor(identity: String) -> Node:
    if not is_instance_valid(source_level) or not source_level.has_meta("source_actor_nodes"): return null
    var paths: Dictionary = source_level.get_meta("source_actor_nodes")
    return source_level.get_node_or_null(paths[identity]) if paths.has(identity) else null

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
    for catalogue in _catalogues: TranslationServer.remove_translation(catalogue)
