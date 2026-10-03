extends SceneTree
const Screen = preload("res://addons/astrolabe/ui/screen.tscn")
const Item = preload("res://addons/astrolabe/ui/dialogue_item.gd")
const Line = preload("res://addons/astrolabe/ui/dialogue_line.gd")
const Span = preload("res://addons/astrolabe/ui/dialogue_span.gd")
const Brain = preload("res://check_ui_brain.gd")
const Character = preload("res://addons/astrolabe/character/character.gd")

func require(condition: bool, message: String) -> bool:
    if condition: return true
    push_error(message)
    quit(1)
    return false

func fixture(box: int = 1) -> Resource:
    var item: Resource = Item.new()
    item.identity = "fixture"
    item.box = box
    item.owner_identity = "level:/persos/fixture"
    item.completion_model = "level:/ai/models/fixture"
    item.completion_normal = 3
    item.translation = Translation.new()
    item.translation.locale = "en"
    for index in range(7):
        var line: Resource = Line.new()
        var span: Resource = Span.new()
        span.key = "fixture.en.line" + str(index)
        span.emphasis = index == 0
        item.translation.add_message(span.key, "café [literal] " + str(index))
        line.spans.assign([span])
        line.end_seconds = 0.15 if index == 3 else 3.4e38
        item.lines.append(line)
    return item

func _initialize() -> void:
    check.call_deferred()

func check() -> void:
    var old_locale: String = TranslationServer.get_locale()
    TranslationServer.set_locale("fr")
    var level := Node3D.new()
    root.add_child(level)
    var actor: Node = Character.new()
    actor.name = "Hero"
    actor.source_hit_points = preload("res://addons/astrolabe/character/hit_points.gd").new()
    actor.source_hit_points.current = 7
    actor.source_hit_points.maximum = 12
    actor.autoplay = false
    actor.set_process(false)
    level.add_child(actor)
    var brain: Node = Brain.new()
    brain.name = "Brain"
    actor.add_child(brain)
    level.set_meta("source_actor_nodes", {"level:/persos/fixture": ^"Hero"})
    var screen: Node = Screen.instantiate()
    screen.dialogue_items.assign([fixture()])
    screen.actor_path = ^"../Hero"
    level.add_child(screen)
    await process_frame
    var dialogue: Control = screen.dialogue
    dialogue.fade_seconds = 0.0
    if not require(screen.canvas.scale == root.get_visible_rect().size / Vector2(640, 480), "640x480 source coordinate scaling"): return
    screen.hud._process(0.0)
    if not require(screen.hud.get_node("Life").value == 7 and screen.hud.get_node("Life").max_value == 12, "HUD life uses actor StandardGame values"): return
    actor.source_set_hud_indicator_visible("life", true)
    if not require(screen.hud.get_node("Life").visible, "live HUD visibility bridge"): return
    screen.source_toggle_widescreen()
    if not require(screen.bars.visible and not screen.hud.get_node("Life").visible and screen.fade.position == Vector2(0,60) and screen.fade.size == Vector2(640,360), "wide bars/HUD/fade clip"): return
    screen.source_position_dialog(305,10)
    if not require(screen.source_run_dialog("fixture", 1, brain), "run with complete English bundle fallback"): return
    if not require(dialogue.panel.visible and dialogue.position == Vector2(305,10), "box style independent of widescreen"): return
    if not require(dialogue.text.get_parsed_text().contains("café [literal] 0"), "native localized Unicode/literal brackets"): return
    dialogue.modulate.a = 1.0
    dialogue.set_process(false)
    dialogue._process(0.2)
    if not require(dialogue.page_start == 6 and not screen.source_dialog_is_over(), "finite page line controls deadline despite untimed padding"): return
    dialogue._process(100.0)
    if not require(dialogue.page_start == 6 and not dialogue.closing, "untimed final page holds"): return
    if not require(screen.source_change_dialog_behavior() and brain.calls == 0, "callback waits for fade and explicit procedure"): return
    screen.source_scroll_dialog(-1)
    if not require(dialogue.page_start == 6, "ScrollDialog does not advance text"): return
    screen.source_select_dialog()
    await process_frame
    await process_frame
    if not require(screen.source_dialog_is_over() and screen.completion_pending and brain.calls == 0, "completion stores pending callback"): return
    if not require(screen.source_change_dialog_behavior(), "explicit ChangeComport consumes callback"): return
    await process_frame
    if not require(brain.calls == 1 and brain.normal == 3 and screen.source_change_dialog_behavior(), "callback applies once to bound Brain"): return
    screen.source_toggle_widescreen()
    if not require(screen.hud.get_node("Life").visible and screen.fade.size == Vector2(640,480), "wide suppression preserves requested HUD state"): return
    screen.dialogue_items.assign([fixture(3)])
    if not require(screen.source_run_dialog("fixture",1,brain) and not dialogue.panel.visible and dialogue.item.lines_per_page() == 2, "unframed cinematic style"): return
    screen.source_end_dialog()
    if not require(screen.source_dialog_is_over() and not screen.completion_pending and brain.calls == 1, "EndDialog cancels without callback"): return
    level.queue_free()
    await process_frame
    var composition := Node.new()
    root.add_child(composition)
    var authored: Node = Screen.instantiate()
    authored.adopt_parent_screen = false
    composition.add_child(authored)
    await process_frame
    var adopted_level := Node3D.new()
    adopted_level.set_meta("source_actor_nodes", {})
    composition.add_child(adopted_level)
    var generated: Node = Screen.instantiate()
    var no_callback: Resource = fixture()
    no_callback.completion_normal = -1
    generated.dialogue_items.assign([no_callback])
    adopted_level.add_child(generated)
    await process_frame
    if not require(generated.adopted_screen == authored and not generated.canvas.visible and authored.source_level == adopted_level, "authored parent adopts one generated UI owner"): return
    if not require(generated.source_run_dialog("fixture",1) and authored.dialogue.item != null, "adopted source bridge retains level content"): return
    adopted_level.queue_free()
    await process_frame
    if not require(authored.dialogue.is_over() and authored.dialogue_items.is_empty() and not authored.completion_pending, "adopted level teardown releases dialogue/catalogues/callback"): return
    composition.queue_free()
    await process_frame
    TranslationServer.set_locale(old_locale)
    print("UI_CONTRACT_OK layers timing locale callbacks hud adoption teardown")
    quit(0)
