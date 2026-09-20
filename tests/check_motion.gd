extends SceneTree

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize() -> void:
    call_deferred("check")

func total_motion(actor: Node, state: int, steps: Array) -> Vector3:
    actor.select_state(state)
    var total := Vector3.ZERO
    for step in steps:
        actor.advance(step)
        total += actor.root_motion_displacement
    return total

func box(parent: Node3D, center: Vector3, size: Vector3) -> void:
    var body := StaticBody3D.new()
    var shape := CollisionShape3D.new()
    var resource := BoxShape3D.new()
    resource.size = size
    shape.shape = resource
    body.add_child(shape)
    body.position = center
    parent.add_child(body)

func physical_key(key: Key, pressed: bool) -> void:
    var event := InputEventKey.new()
    event.physical_keycode = key
    event.pressed = pressed
    Input.parse_input_event(event)
    Input.flush_buffered_events()

func check() -> void:
    # An authored binding and an intentionally disabled control survive adoption.
    InputMap.add_action("Button_Hit")
    var custom := InputEventKey.new()
    custom.physical_keycode = KEY_F
    InputMap.action_add_event("Button_Hit", custom)
    InputMap.add_action("Button_GetSword")
    var args := OS.get_cmdline_user_args()
    var packed := load("res://" + args[0]) as PackedScene
    var actor = packed.instantiate()
    if not require(actor is CharacterBody3D and actor.has_node("RootMotion"), "saved native motion nodes absent"): return
    var shapes := actor.find_children("Movement_*", "CollisionShape3D", false, false)
    if not require(shapes.size() == 1 and not shapes[0].disabled and is_equal_approx(shapes[0].shape.radius, 0.25), "source movement collider not persisted"): return
    actor.get_node("Brain").automatic = false
    actor.autoplay = false
    actor.set_process(false)
    actor.set_physics_process(false)
    var world := Node3D.new()
    root.add_child(world)
    world.add_child(actor)
    var stride := 1.125 if args.size() > 1 else 0.75
    # Different frame vectors exercise interpolation; an entire clip is 0.3 s.
    var single := total_motion(actor, 0, [0.9])
    var split := total_motion(actor, 0, [0.07, 0.14, 0.01, 0.31, 0.37])
    if not require(single.distance_to(Vector3(0,0,-stride*3)) < 0.0001, "loop displacement " + str(single)): return
    if not require(split.distance_to(single) < 0.0001, "partition-dependent displacement " + str(split)): return
    var fast := total_motion(actor, 1, [0.9])
    if not require(fast.distance_to(single*2) < 0.0001, "state playback rate did not scale motion"): return
    actor.select_state(0)
    actor.seek_frame(2)
    actor.seek_frame(0)
    if not require(actor.position.is_zero_approx() and actor.get_node("RootMotion").position.is_zero_approx(), "seeking translated body or root-motion node"): return
    actor.advance(0.05)
    if not require(actor.root_motion_displacement.distance_to(Vector3(0,0,-0.0625)) < 0.0001, "seek retained stale motion"): return
    actor.select_state(0)
    if not require(not actor.request_action(5), "prohibited action accepted"): return
    if not require(actor.request_action(1) and actor.current_state == 3 and actor.requested_action_state == 1, "first explicit transition did not win"): return
    actor.advance(0.1)
    var transition_time: float = actor.animation_player.current_animation_position
    actor.request_action(1)
    if not require(is_equal_approx(actor.animation_player.current_animation_position, transition_time), "ChangeAction restarted the same request"): return
    actor.advance(0.2)
    if not require(actor.current_state == 3 and actor.animation_complete and not actor.action_complete, "additional action cycle not respected"): return
    actor.advance(0.3)
    if not require(actor.current_state == 1 and not actor.action_complete, "action next state not followed"): return
    var finished: Array[int] = []
    actor.action_finished.connect(func(id: int): finished.append(id))
    actor.advance(0.15)
    if not require(actor.action_complete and finished == [1] and not actor.playing, "finite action completion not signaled"): return
    actor.advance(0.2)
    if not require(actor.root_motion_displacement.is_zero_approx() and finished == [1], "completed action kept moving or signaled twice"): return
    actor.request_action(1, true)
    if not require(not actor.action_complete and actor.playing and actor.animation_player.current_animation_position == 0.0, "ExecuteAction did not restart"): return
    actor.select_state(4)
    if not require(not actor.request_action(1) and actor.current_state == 4, "locked action allowed an unlisted transition"): return
    actor.select_state(2)
    if not require(shapes[0].disabled, "empty state collision selection ignored"): return
    actor.select_state(0)
    if not require(not shapes[0].disabled, "state collision selection did not restore shape"): return
    # Both the body and its movement collider came through the real exporter.
    box(world, Vector3(0,-0.5,0), Vector3(20,1,20))
    box(world, Vector3(-2,2,0), Vector3(0.5,4,10))
    actor.gravity = Vector3(0,-9.8,0)
    actor.rotation.y = PI/2
    actor.position.y = 0.01
    actor.motion_enabled = true
    actor.select_state(0)
    actor.set_physics_process(true)
    for i in 75: await physics_frame
    if not require(actor.is_on_floor() and actor.is_on_wall(), "missing native floor/wall contact"): return
    if not require(absf(actor.position.x + 1.5) < 0.02 and absf(actor.position.z) < 0.02, "rotated root motion penetrated wall: " + str(actor.position)): return
    actor.select_state(1)
    for i in 20: await physics_frame
    if not require(absf(actor.position.x + 1.5) < 0.02, "running animation penetrated wall"): return
    actor.set_physics_process(false)
    print("NATIVE_MOTION_EDIT_OK" if args.size() > 1 else "NATIVE_MOTION_OK", " ", JSON.stringify({"stride":stride,"walk":single,"run":fast,"contact_position":actor.position}))
    # The same saved actor carries the compiler's input/action code. It uses
    # ordinary properties and native arrays, then awaits native action completion.
    var brain = actor.get_node("Brain")
    brain.adopt_input("Button_GetSword", KEY_X)
    if not require(InputMap.action_get_events("Button_GetSword").is_empty(), "disabled authored input was replaced"): return
    if not require(InputMap.action_get_events("Button_Up").size() == 1 and InputMap.action_get_events("Button_Up")[0].physical_keycode == KEY_UP, "movement default not adopted"): return
    if not require(InputMap.action_get_events("Button_Hit").size() == 1 and InputMap.action_get_events("Button_Hit")[0].physical_keycode == KEY_F, "authored binding was replaced"): return
    brain.main_actor = actor
    actor.select_state(0)
    actor.motion_enabled = false
    actor.set_process(false)
    if not require(brain.start(), "compiled action behavior did not start"): return
    physical_key(KEY_UP, true)
    brain.advance(0.01)
    var expected_action := 0 if args.size() > 1 else 1
    if not require(actor.is_in_action(expected_action), "compiled button/array selection ignored Rete parameter"): return
    physical_key(KEY_UP, false)
    brain.advance(0.01)
    if not require(actor.is_in_action(0), "compiled released button did not select resting action"): return
    physical_key(KEY_F, true)
    await physics_frame
    brain.advance(0.01)
    if not require(actor.current_state == 3 and brain.dsg_2 == 0, "compiled ExecuteAction did not enter/wait for transition: state=%s marker=%s just_pressed=%s running=%s" % [actor.current_state, brain.dsg_2, Input.is_action_just_pressed("Button_Hit"), brain.running]): return
    physical_key(KEY_F, false)
    await process_frame
    actor.advance(0.6)
    brain.advance(0.6)
    if not require(actor.current_state == 1 and brain.dsg_2 == 0, "blocking action resumed before completion"): return
    actor.advance(0.15)
    brain.advance(0.15)
    if not require(brain.dsg_2 == 2 and actor.action_complete, "blocking native action did not resume compiled script"): return
    brain.stop()
    # The same compiled action request must survive the parent's initial _ready.
    var automatic_actor = packed.instantiate()
    var automatic_brain = automatic_actor.get_node("Brain")
    automatic_brain.main_actor_path = ^".."
    automatic_brain.dsg_1 = 1
    physical_key(KEY_UP, true)
    world.add_child(automatic_actor)
    automatic_actor.set_process(false)
    await process_frame
    if not require(automatic_brain.running and automatic_actor.is_in_action(1) and automatic_actor.current_state == 3, "parent ready overwrote compiled initial action"): return
    physical_key(KEY_UP, false)
    automatic_brain.stop()
    for action in ["Button_Up", "Button_Hit", "Button_GetSword"]: InputMap.erase_action(action)
    print("COMPILED_MOTION_ACTIONS_OK ", {"selected": expected_action, "completed": brain.dsg_2})
    world.queue_free()
    await process_frame
    quit()
