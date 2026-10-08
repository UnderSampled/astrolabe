extends SceneTree
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")
const BRAIN = preload("res://compiled_armor_setup.gd")

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize() -> void:
    call_deferred("run")

func run() -> void:
    var packed := load("res://generated/hype/levels/manoir/level.tscn") as PackedScene
    var first := packed.instantiate()
    var second := packed.instantiate()
    root.add_child(first)
    root.add_child(second)
    var state: Resource = first.get_meta("source_armor_state")
    var catalog: Resource = first.get_meta("source_armor_catalog")
    var untouched: Resource = second.get_meta("source_armor_catalog")
    if not require(state.catalog == catalog and state.selected_index == 0 and state.selected_definition() == catalog.active(), "saved level lost its initially selected Fix definition"): return
    var actors: Array[Node3D] = []
    for _index in range(2):
        var actor := CHARACTER.new()
        actor.autoplay = false
        actor.process_mode = Node.PROCESS_MODE_DISABLED
        var emitter := Node3D.new()
        emitter.name = "SoundEmitter"
        actor.add_child(emitter)
        var brain := BRAIN.new()
        brain.name = "Brain"
        brain.automatic = false
        brain.current_normal = 0
        actor.add_child(brain)
        first.add_child(actor)
        brain.main_actor = actor
        brain.sound_emitter = emitter
        if not require(brain.source_armor_state() == state, "compiled actors do not share the saved armor facade"): return
        actors.append(actor)
    var brain: Node3D = actors[0].get_node("Brain")
    if not require(brain.start() and brain.unsupported_failure.is_empty() and brain.dsg_0 == 0 and brain.dsg_1 == 1, "compiled resistance/repair/queries rejected the selected facade"): return
    if not require(catalog.selected == 0 and catalog.active().resistance_rank == 0 and catalog.active().broken_count == 0 and catalog.active().pieces[0].strikes == 0, "compiled armor setup did not update the selected shared definition"): return
    if not require(untouched != catalog and untouched.active().resistance_rank == 3 and untouched.active().pieces[0].strikes == 7, "armor mutations leaked into another saved level instance"): return
    state.select_source_index(255)
    if not require(state.selected_definition() == null and state.selected_index == -1, "missing source selection was silently replaced by definition zero"): return
    brain.stop()
    first.free()
    second.free()
    print("COMPILED_ARMOR_SETUP_OK")
    quit(0)
