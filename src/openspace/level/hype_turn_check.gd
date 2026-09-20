extends SceneTree

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize() -> void:
    call_deferred("check")

func key(code: Key, pressed: bool) -> void:
    var event := InputEventKey.new()
    event.physical_keycode = code
    event.pressed = pressed
    Input.parse_input_event(event)
    Input.flush_buffered_events()

func pose(actor: Node3D) -> Array[Transform3D]:
    var transforms: Array[Transform3D] = []
    for node in actor.get_node("Channels").get_children():
        transforms.append(node.transform)
    return transforms

func check() -> void:
    var packed := load("res://" + OS.get_cmdline_user_args()[0]) as PackedScene
    var actor = packed.instantiate()
    actor.autoplay = false
    actor.set_process(false)
    actor.set_physics_process(false)
    root.add_child(actor)
    var original_position: Vector3 = actor.position
    if not require(actor.select_state(47), "Hype idle animation is absent"): return
    var brain = load("res://rules.gd").new()
    brain.automatic = false
    brain.current_normal = 4
    actor.add_child(brain)
    brain.main_actor = actor
    brain.sound_emitter = actor.get_node("SoundEmitter")
    # Explicit rule fixture: equipment flag 59 false, no control lock, idle action
    # already entered. Initialization and the other rules are not substituted.
    brain.@v10@ = false
    brain.@v19@ = false
    brain.@v52@ = true
    brain.@v59@ = false
    brain.@v61@ = false
    if not require(brain.@v107@ == 160 and brain.start(), "source turn rate or startup differs"): return
    var idle_pose := pose(actor)
    key(KEY_LEFT, true)
    brain.advance(0.25)
    actor.advance(0.25)
    var slow_turn: float = actor.rotation.y
    if not require(is_equal_approx(slow_turn, deg_to_rad(40.0)) and actor.is_in_action(@left_slow@), "original left-turn rule selected wrong rate/action: %s state %s" % [rad_to_deg(slow_turn), actor.current_state]): return
    if not require(pose(actor) != idle_pose and actor.animation_player.current_animation_position > 0.0, "turn animation did not change the native pose"): return
    if not require(actor.root_motion_displacement.length() < 0.0001, "idle turn unexpectedly requested translation"): return
    key(KEY_LEFT, false)
    brain.advance(0.01)
    actor.advance(2.0)
    if not require(actor.is_in_action(@idle@) and not brain.@v61@, "release did not run the original idle-return branch"): return
    if not require(is_equal_approx(actor.rotation.y, slow_turn), "released turn continued rotating"): return

    actor.basis = Basis.IDENTITY
    brain.@v52@ = true
    key(KEY_SHIFT, true)
    key(KEY_RIGHT, true)
    brain.advance(0.10)
    actor.advance(0.10)
    brain.advance(0.15)
    actor.advance(0.15)
    if not require(is_equal_approx(actor.rotation.y, deg_to_rad(-60.0)) and actor.is_in_action(@right_fast@), "fast right turn did not apply the source negation/multiplier"): return
    key(KEY_RIGHT, false)
    key(KEY_SHIFT, false)
    brain.advance(0.01)
    actor.advance(2.0)
    if not require(actor.position.is_equal_approx(original_position), "turn translated Hype's body"): return

    # Local-axis composition also survives a tilted authored parent/body.
    actor.basis = Basis(Vector3.RIGHT, 0.2)
    var tilted: Basis = actor.basis
    brain.@v52@ = true
    key(KEY_LEFT, true)
    brain.advance(0.25)
    if not require(actor.basis.is_equal_approx(tilted * Basis(Vector3.UP, deg_to_rad(40.0))), "turn used the world axis instead of actor up"): return
    key(KEY_LEFT, false)
    brain.stop()
    brain.free()

    # A useful-value edit survives Rete and changes the same source rule.
    actor.basis = Basis.IDENTITY
    actor.select_state(47)
    brain = load("res://rules-edited.gd").new()
    brain.automatic = false
    brain.current_normal = 4
    actor.add_child(brain)
    brain.main_actor = actor
    brain.sound_emitter = actor.get_node("SoundEmitter")
    brain.@v19@ = false
    brain.@v52@ = true
    brain.@v59@ = false
    brain.start()
    key(KEY_LEFT, true)
    brain.advance(0.25)
    actor.advance(0.25)
    if not require(is_equal_approx(actor.rotation.y, deg_to_rad(20.0)), "Rete turn-rate edit had no effect"): return
    key(KEY_LEFT, false)
    brain.stop()

    # Execute the whole original walk rule for both transition animations.
    # Actual native clip completion, rather than an injected completion flag,
    # must make it select the sustained walk action.
    for pair in [[@walk_start@, @walk@, 0.35, -2.6792], [@walk_start_other@, @walk_other@, 0.45, -3.35]]:
        actor.select_state(47)
        if not require(actor.request_action(pair[0], true), "could not enter walk transition fixture"): return
        brain.current_normal = 5
        brain.@v147@ = 0.0
        brain.start()
        if not require(is_equal_approx(brain.@v147@, -pair[2]), "source walk subtraction differs"): return
        for i in 20: brain.advance(1.0 / 60.0)
        if not require(is_equal_approx(brain.@v147@, pair[3]), "source walk ramp did not clamp"): return
        actor.advance(10.0)
        if not require(actor.action_complete, "native walk transition did not complete"): return
        brain.advance(0.01)
        if not require(actor.is_in_action(pair[1]) and is_equal_approx(brain.@v147@, pair[3]), "walk completion did not select the source sustained action"): return
        brain.stop()
    actor.free()
    print("HYPE_NATIVE_RULES_OK: complete turn/walk rules, actual Hype animations, 40/60 degree turns, 20 degree Rete edit, native action completion")
    quit()
