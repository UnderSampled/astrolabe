extends SceneTree

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize() -> void:
    call_deferred("check")

func check() -> void:
    var world := Node3D.new()
    var actor := Node3D.new()
    actor.name = "Actor"
    var emitter := Node3D.new()
    emitter.name = "SoundEmitter"
    emitter.set_script(load("res://addons/astrolabe/audio/emitter.gd"))
    var brain := Node3D.new()
    var program = load("res://generated/hype/levels/synthetic/ai/proximity.gd")
    if not require(program != null, "partial program did not compile"): return
    brain.set_script(program)
    brain.automatic = false
    brain.current_normal = 0
    world.add_child(actor)
    world.add_child(emitter)
    world.add_child(brain)
    root.add_child(world)
    brain.main_actor = actor
    brain.sound_emitter = emitter
    if not require(brain.start(), "partial program did not start"): return
    if not require(brain.dsg_1 == 3, "unselected branch blocked supported siblings"): return
    if not require(brain.unsupported_failure.is_empty(), "unselected branch failed"): return
    brain.dsg_0 = true
    brain.advance(0.0)
    if not require(brain.dsg_1 == 1, "execution continued after reached unsupported fragment"): return
    if not require(not brain.unsupported_failure.is_empty(), "reached fragment did not report failure"): return
    if not require(not brain.running, "reached fragment did not stop"): return
    print("PARTIAL_AI_REACHED_FAILURE_OK: " + brain.unsupported_failure)
    world.free()
    quit(0)
