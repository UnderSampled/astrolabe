extends SceneTree
func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok
func _initialize() -> void: call_deferred("check")
func check() -> void:
    var world := Node3D.new()
    var actor := Node3D.new()
    actor.name = "Actor"
    var emitter := Node3D.new()
    emitter.name = "SoundEmitter"
    emitter.set_script(load("res://addons/astrolabe/audio/emitter.gd"))
    var behavior := Node3D.new()
    behavior.set_script(load("res://generated/hype/levels/synthetic/ai/proximity.gd"))
    behavior.automatic = false
    behavior.current_normal = 0
    world.add_child(actor)
    world.add_child(emitter)
    world.add_child(behavior)
    root.add_child(world)
    behavior.main_actor = actor
    behavior.sound_emitter = emitter
    actor.position.x = 2.0
    if not require(behavior.start(), "compiled program failed to start"): return
    behavior.advance(0.099)
    if not require(behavior.dsg_0 == false and behavior.dsg_1 == 0, "blocking wait completed early"): return
    behavior.advance(0.001)
    var edited := not OS.get_cmdline_user_args().is_empty()
    if edited:
        if not require(behavior.dsg_0 == false and behavior.dsg_1 == 9, "AST threshold edit did not change execution"): return
        behavior.stop()
        world.free()
        print("COMPILED_AI_EDIT_OK")
        quit(0)
        return
    if not require(behavior.dsg_0 == true and behavior.dsg_1 == 7, "near branch or typed assignment failed"): return
    actor.position.x = 3.0
    behavior.advance(0.0) # Schedule restarts on its next tick, then captures a new deadline.
    behavior.advance(0.099)
    if not require(behavior.dsg_0 == true, "next wait did not block"): return
    behavior.advance(0.001)
    if not require(behavior.dsg_0 == false and behavior.dsg_1 == 9, "strict distance boundary failed"): return
    behavior.advance(0.0)
    behavior.stop()
    actor.position.x = 1.0
    behavior.advance(10.0)
    if not require(behavior.dsg_0 == false, "cancelled coroutine executed"): return
    if not require(behavior.start(), "restart failed"): return
    if not require(behavior.dsg_1 == 9, "restart discarded persistent instance state"): return
    behavior.advance(0.1)
    if not require(behavior.dsg_0 == true, "restarted program did not execute"): return
    # Exercise the exported component's normal ready/physics path as well as
    # deterministic stepping. Bindings are saved NodePaths, not runtime lookup tables.
    var automatic := Node3D.new()
    automatic.set_script(load("res://generated/hype/levels/synthetic/ai/proximity.gd"))
    automatic.main_actor_path = ^"../Actor"
    automatic.current_normal = 0
    world.add_child(automatic)
    await create_timer(0.25).timeout
    if not require(automatic.running and automatic.clock_milliseconds >= 100 and automatic.dsg_0 == true, "automatic native physics execution failed"): return
    world.free()
    print("COMPILED_AI_OK: native coroutine waits, strict proximity, typed variables, cancellation/restart, automatic physics")
    quit(0)
