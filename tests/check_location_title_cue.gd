extends SceneTree
## Executes compiled cue schedules against the maintained Screen and game-state
## methods. Only level loading is suppressed; this is a bounded component test.
const Screen = preload("res://addons/astrolabe/ui/screen.tscn")
const Character = preload("res://addons/astrolabe/character/character.gd")
const Emitter = preload("res://addons/astrolabe/audio/emitter.gd")
const Fixture = preload("res://check_location_title.gd")
const GameSession = preload("res://game/game_session.gd")
class CueSession:
    extends "res://game/game_session.gd"
    func _ready() -> void: pass

func require(condition: bool, message: String) -> bool:
    if condition: return true
    push_error(message)
    quit(1)
    return false

func _initialize() -> void:
    check.call_deferred()

func session_fixture() -> Node3D:
    var host := CueSession.new()
    var camera := Camera3D.new()
    camera.name = "DemoCamera"
    host.add_child(camera)
    for name in ["InventoryHost", "PauseHost", "VignetteHost"]:
        var child := Node.new()
        child.name = name
        host.add_child(child)
    var parent: Node = host
    for name in ["Hud", "Panel", "Margin", "Rows"]:
        var child := Node.new()
        child.name = name
        parent.add_child(child)
        parent = child
    for name in ["Status", "Mana"]:
        var child := Label.new()
        child.name = name
        parent.add_child(child)
    root.add_child(host)
    return host

func level_fixture(host: Node, item: Resource, program: Script) -> Dictionary:
    var level := Node3D.new()
    var actor: Node3D = Character.new()
    actor.name = "TitleActor"
    actor.autoplay = false
    actor.set_meta("source_perso", item.owner_identity)
    var emitter: Node3D = Emitter.new()
    emitter.name = "SoundEmitter"
    actor.add_child(emitter)
    var brain: Node = program.new()
    brain.name = "Brain"
    brain.automatic = false
    brain.current_normal = 0
    actor.add_child(brain)
    level.add_child(actor)
    var main: Node3D = Character.new()
    main.name = "MainActor"
    main.autoplay = false
    var main_emitter: Node3D = Emitter.new()
    main_emitter.name = "SoundEmitter"
    main.add_child(main_emitter)
    level.add_child(main)
    level.set_meta("source_main_actor", "synthetic:/main")
    level.set_meta("source_actor_nodes", {item.owner_identity: NodePath("TitleActor"), "synthetic:/main": NodePath("MainActor")})
    var child_ui: Node = Screen.instantiate()
    child_ui.dialogue_items.assign([item])
    level.add_child(child_ui)
    host.add_child(level)
    brain.main_actor = main
    brain.sound_emitter = emitter
    actor.set_process(false)
    main.set_process(false)
    return {"level": level, "actor": actor, "brain": brain, "ui": child_ui, "main_emitter": main_emitter}

func check() -> void:
    # Test the actual authored getter across the complete byte range. It must
    # never manufacture selectors from a combined mask or keep a private cache.
    var detached: Node = GameSession.new()
    var masks := {0: 0, 1: 1, 2: 2, 4: 3, 8: 4, 16: 5, 32: 6, 64: 7, 128: 8}
    for mask in range(256):
        detached.source_game_state_mask = mask
        var selector: Variant = detached.source_get_game_state()
        if not require(selector == masks.get(mask), "exact Func195 mask mapping or explicit stale-result gap"): return
    for selector in range(9):
        detached.source_set_game_state(selector)
        if not require(detached.source_get_game_state() == selector, "shared setter/getter mapping"): return
    detached.free()
    var value: Resource = Fixture.fixture_item()
    value.owner_identity = "level:/persos/title_fixture"
    value.completion_model = "level:/ai/models/title_fixture"
    value.completion_normal = 1
    var program_path := "res://cue.gd"
    for argument in OS.get_cmdline_user_args():
        if argument.begins_with("--item="): value = load(argument.trim_prefix("--item="))
        if argument.begins_with("--program="): program_path = argument.trim_prefix("--program=")
    if not require(value != null and value.box == 4 and value.lines.size() == 2, "source/synthetic two-line style4 item"): return
    var program := load(program_path) as Script
    if not require(program != null and program.can_instantiate(), "exported compiled cue script"): return
    var host: Node3D = session_fixture()
    var screen: Node = Screen.instantiate()
    screen.adopt_parent_screen = false
    host.add_child(screen)
    var title: Control = screen.location_title
    title.set_process(false)
    # Selector2 opens only at 200ms. Other selectors still change behavior, wait,
    # End the shared presentation and deactivate the actor unconditionally.
    for selector in [2, 0, 1, 3, 4, 5, 6, 7, 8]:
        host.source_set_game_state(selector)
        var scene := level_fixture(host, value, program)
        var brain: Node = scene.brain
        if not require(scene.ui.adopted_screen == screen and screen.source_level == scene.level, "generated screen adopts actual presentation owner"): return
        if not require(brain.start(), "source cue starts"): return
        brain.advance(0.199)
        if not require(brain.current_normal == 0 and title.item == null and screen.source_position == Vector2.ZERO, "200ms wait before Position/Run"): return
        brain.advance(0.001)
        if not require(brain.current_normal == 1 and screen.source_position == Vector2(160, 70), "source Position then unconditional behavior change"): return
        if not require((title.item == value) == (selector == 2), "GetGameState2 gates only RunDialog"): return
        if selector == 2:
            if not require(screen.active_presentation == title and screen.dialogue.item == null, "style4 owns shared dispatch"): return
            for draw in range(11): title._process(0.011)
            if not require(title.panel.position == Vector2(16, 16) and title.panel.size == Vector2(288, 96), "source frame dimensions"): return
            if not require(title.source_text.visible and title.source_text.get_child_count() == 2, "source title text appears after opening"): return
            for index in range(2):
                var label: Label = title.source_text.get_child(index)
                if not require(label.text == value.message(value.lines[index].spans[0].key), "native catalogue text preserved"): return
            scene.main_emitter.source_dialogue_record = Resource.new()
            screen.source_select_dialog()
            if not require(title.item == value and not screen.completion_pending, "Select reveals title without completing a behavior"): return
            screen.source_select_dialog()
            for draw in range(12): title._process(0.0)
            if not require(not screen.source_dialog_is_over() and not screen.source_dialog_is_over_and_change_behavior() and screen.completion_pending, "MainActor speech gates title IsOver and callback consumption"): return
            scene.main_emitter.source_dialogue_record = null
            if not require(screen.source_dialog_is_over() and screen.completion_pending and screen.completion_normal == value.completion_normal, "Cond121 retains the source pending callback"): return
            if not require(screen.source_run_dialog(value.identity, 1, brain), "repeat Run replaces presentation"): return
            for draw in range(11): title._process(0.011)
        else:
            # A false title predicate must still End another active dialogue.
            var ordinary: Resource = Fixture.fixture_item()
            ordinary.identity = "synthetic_ordinary"
            ordinary.box = 3
            screen.dialogue_items.append(ordinary)
            screen.dialogue.fade_seconds = 0.01
            if not require(screen.source_run_dialog(ordinary.identity, 1), "unrelated live dialogue"): return
        brain.advance(0.0) # Enter normal1 on the next native Brain tick.
        brain.advance(2.999)
        if not require(scene.actor.source_object_active and screen.active_presentation.item != null, "3000ms source wait has not ended early"): return
        brain.advance(0.001)
        if not require(not scene.actor.source_object_active and brain.unsupported_failure.is_empty() and not screen.completion_pending, "source End then deactivate without callback"): return
        if selector == 2:
            if not require(title.item == null and title._closing and title.panel.modulate.a == 1.0, "End uses animated style4 close"): return
        else:
            if not require(screen.dialogue.closing, "unconditional End closes ordinary presentation"): return
        scene.level.free()
        if not require(title.is_over() and screen.dialogue.is_over() and screen.source_level == host, "map release immediately cancels both owners"): return
        screen.source_position_dialog(0, 0)
    # Map departure during the first wait cancels the old coroutine. Fresh entry
    # executes the cue again, through saved initial behavior0 and native startup.
    host.source_set_game_state(2)
    var abandoned := level_fixture(host, value, program)
    abandoned.brain.start()
    abandoned.brain.advance(0.1)
    abandoned.level.free()
    if not require(title.item == null and not title.visible, "departure during initial wait"): return
    var fresh := level_fixture(host, value, program)
    fresh.brain.automatic = true
    if not require(fresh.brain.start(), "fresh source Brain entry"): return
    var deadline := Time.get_ticks_msec() + 1000
    while title.item == null and Time.get_ticks_msec() < deadline:
        await physics_frame
    if not require(title.item == value and fresh.brain.current_normal == 1 and fresh.brain.clock_milliseconds >= 200.0 - 1e-6, "native physics executes fresh unchanged cue: clock=%s normal=%s running=%s failure=%s item=%s" % [fresh.brain.clock_milliseconds, fresh.brain.current_normal, fresh.brain.running, fresh.brain.unsupported_failure, title.item]): return
    fresh.level.free()
    if not require(title.item == null and not title.visible, "departure cancels live title immediately"): return
    # Exercise the same callback consumer as ordinary dialogue, independently
    # of the cue schedule so that its 3000ms wait remains an unchanged-source test.
    var callback_scene := level_fixture(host, value, program)
    callback_scene.brain.stop()
    callback_scene.brain.current_normal = -1
    if not require(screen.source_run_dialog(value.identity, 1, callback_scene.brain), "source title callback item opens"): return
    for draw in range(11): title._process(0.011)
    screen.source_select_dialog()
    screen.source_select_dialog()
    for draw in range(12): title._process(0.0)
    if not require(screen.completion_pending and screen.source_dialog_is_over_and_change_behavior(), "title callback uses shared explicit consumer"): return
    await process_frame
    if not require(callback_scene.brain.current_normal == value.completion_normal and not screen.completion_pending, "source completion reaches bound compiled Brain once"): return
    callback_scene.level.free()
    host.free()
    print("LOCATION_TITLE_CUE_OK shared session getter; selectors2/0..8; compiled200/3000ms; native physics; Select/End/speech gate; adoption/map cancel/reentry; source deactivation")
    quit(0)
