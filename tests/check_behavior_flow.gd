extends SceneTree

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize() -> void:
    call_deferred("check")

func make() -> Node3D:
    var actor := Node3D.new()
    root.add_child(actor)
    var emitter := Node3D.new()
    emitter.name = "SoundEmitter"
    actor.add_child(emitter)
    var brain = load("res://flow.gd").new()
    brain.automatic = false
    brain.current_normal = 0
    brain.current_reflex = 0
    actor.add_child(brain)
    brain.main_actor = actor
    return brain

func check() -> void:
    # Blocking normal rules cancel only the schedule's pending request. Earlier
    # statements stay executed; the restarted wait captures its new duration.
    var brain = make()
    if not require(brain.start(), "could not start behavior"): return
    if not require(brain.dsg_6 == 7 and brain.dsg_7 == 0, "schedule did not enter its wait"): return
    brain.advance(0.05)
    brain.dsg_6 = 0
    brain.dsg_0 = true
    brain.advance(0.01)
    if not require(brain.dsg_5 == 1 and brain.dsg_7 == 0, "rule did not block schedule"): return
    brain.advance(0.02)
    if not require(brain.dsg_5 == 2 and brain.dsg_7 == 0, "one-frame rule wait did not complete"): return
    brain.dsg_8 = 200
    brain.advance(0.01)
    brain.advance(0.199)
    if not require(brain.dsg_6 == 0 and brain.dsg_7 == 0, "schedule replayed earlier code or kept cancelled deadline"): return
    brain.advance(0.001)
    if not require(brain.dsg_6 == 0 and brain.dsg_7 == 8, "schedule did not restart only its pending wait"): return
    brain.get_parent().free()

    # Transitioning away must release old suspended coroutines immediately,
    # including when the destination has no schedule signal to wake them later.
    brain = make()
    brain.start()
    for i in 100:
        brain.change_normal(-1)
        brain.advance(0.01)
        if not require(brain.normal_schedule_tick.get_connections().is_empty(), "cancelled schedule kept a suspended coroutine"): return
        if not require(brain.normal_tick.get_connections().is_empty(), "cancelled normal behavior kept a suspended coroutine"): return
        brain.change_normal(0)
        brain.advance(0.01)
        brain.dsg_4 = true
        brain.advance(0.01)
        brain.change_reflex(-1)
        brain.advance(0.01)
        if not require(brain.reflex_tick.get_connections().is_empty(), "cancelled reflex rule kept a suspended coroutine"): return
        brain.change_reflex(0)
        brain.advance(0.01)
    brain.get_parent().free()

    # Reflex action ticks defer normal work, including their completion tick.
    # They do not cancel/restart a pending action in the other behavior group.
    brain = make()
    brain.start()
    brain.advance(0.05)
    brain.dsg_4 = true
    brain.advance(0.1)
    if not require(brain.dsg_7 == 0, "normal ran during reflex action entry"): return
    brain.advance(0.01)
    if not require(brain.dsg_5 == 3 and brain.dsg_7 == 0, "normal ran during reflex action completion"): return
    brain.advance(0.0)
    if not require(brain.dsg_7 == 8, "reflex wait incorrectly restarted normal's deadline"): return
    brain.get_parent().free()

    # Resume the blocked rule itself even if an earlier rule becomes true.
    # A requested same-group transition cancels the old schedule immediately,
    # but its explicit yield defers entry. The following immediate change chains.
    brain = make()
    brain.dsg_1 = true
    brain.start()
    if not require(brain.dsg_5 == 4 and brain.dsg_6 == 0, "rule priority ignored at startup"): return
    brain.dsg_0 = true
    brain.advance(0.01)
    if not require(brain.dsg_5 == 5 and brain.current_normal == 5 and brain.dsg_2 == 0, "blocked rule resumed at wrong priority or entered too soon"): return
    brain.advance(0.01)
    if not require(brain.current_normal == 6 and brain.dsg_2 == 6 and brain.dsg_5 == 5, "state cancellation, immediate chain or same-state no-op failed"): return
    brain.stop()
    brain.advance(1000)
    if not require(brain.dsg_2 == 6, "cancelled continuation executed"): return
    brain.get_parent().free()

    # A cross-group request leaves its caller running. A reflex rule changes
    # normal behavior before normal's phase in this same tick.
    brain = make()
    brain.current_normal = 7
    brain.start()
    if not require(brain.current_reflex == 1 and brain.dsg_2 == 7 and brain.dsg_5 == 0, "normal-to-reflex request cancelled its caller"): return
    brain.advance(0.01)
    if not require(brain.current_normal == 6 and brain.dsg_2 == 6 and brain.dsg_5 == 9, "reflex-to-normal request did not precede normal entry"): return
    brain.get_parent().free()
    print("NATIVE_BEHAVIOR_FLOW_OK: ordered rules, action restart, reflex priority, deferred/immediate entry, cross-group requests")
    quit()
