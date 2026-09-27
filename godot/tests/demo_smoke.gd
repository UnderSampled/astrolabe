extends SceneTree
## Run after porting manoir: godot --headless --path godot --script res://tests/demo_smoke.gd

func _initialize() -> void:
    call_deferred("run")

func run() -> void:
    var menu_packed := load("res://game/bootstrap.tscn") as PackedScene
    if menu_packed == null:
        fail("Game menu did not load")
        return
    var menu := menu_packed.instantiate()
    root.add_child(menu)
    var menu_buttons := menu.find_children("*", "Button", true, false)
    if menu_buttons.size() != 1 or menu_buttons[0].text != "Load Game" or menu_buttons[0].disabled:
        fail("Game menu must offer one enabled Load Game action after manor generation")
        return
    menu.queue_free()
    var packed := load("res://game/demo.tscn") as PackedScene
    if packed == null:
        fail("Demo scene did not load")
        return
    var demo := packed.instantiate()
    root.add_child(demo)
    var deadline := Time.get_ticks_msec() + 90000
    var next_report := Time.get_ticks_msec() + 10000
    while not bool(demo.get("ready_to_play")) and Time.get_ticks_msec() < deadline:
        if Time.get_ticks_msec() >= next_report:
            print("Demo loading: %s; paused=%s; scale=%.2f; frames=%d" % [str(demo.get_node("Hud/Panel/Margin/Rows/Status").text), str(paused), Engine.time_scale, Engine.get_process_frames()])
            next_report += 10000
        await process_frame
    if not bool(demo.get("ready_to_play")):
        fail("Demo did not reach playable state: " + str(demo.get_node("Hud/Panel/Margin/Rows/Status").text))
        return
    var actor := demo.get("actor") as CharacterBody3D
    var brain := demo.get("brain") as Node
    var camera := demo.get_node("DemoCamera") as Camera3D
    var gate := demo.get("level").get_node("world/0/15/iMGrille_Manoir_Pont") as CharacterBody3D
    if gate == null or not is_equal_approx(gate.position.y, 5.2) or bool(gate.get_node("Brain").get("automatic")):
        fail("Bridge gate did not start open")
        return
    await physics_frame
    var passage := gate.global_position - Vector3.UP * 6.7
    var ray := PhysicsRayQueryParameters3D.create(passage + Vector3(0, 0, 0.6), passage - Vector3(0, 0, 0.6))
    var obstruction := actor.get_world_3d().direct_space_state.intersect_ray(ray)
    if not obstruction.is_empty():
        fail("Bridge gate passage has a remaining collider: " + str(obstruction))
        return
    if actor == null or brain == null or not bool(brain.get("running")) or camera.get("target") != actor or not camera.current:
        fail("Demo actor, Brain, or camera did not start")
        return
    var start := actor.global_position
    key(KEY_LEFT, true)
    await create_timer(0.25).timeout
    key(KEY_LEFT, false)
    key(KEY_UP, true)
    await create_timer(0.65).timeout
    key(KEY_UP, false)
    await create_timer(0.2).timeout
    var distance := actor.global_position.distance_to(start)
    if distance < 0.2 or not str(brain.get("unsupported_failure")).is_empty():
        fail("Physical arrow input did not move the compiled Hype actor")
        return
    var args := OS.get_cmdline_user_args()
    if not args.is_empty():
        var picture := root.get_texture().get_image()
        if picture == null or picture.save_png(args[0]) != OK:
            fail("Demo screenshot could not be saved")
            return
    print("Demo ready: Hype moved %.3f units; Brain running; follow camera current" % distance)
    quit(0)

func key(code: Key, pressed: bool) -> void:
    var event := InputEventKey.new()
    event.physical_keycode = code
    event.pressed = pressed
    Input.parse_input_event(event)
    Input.flush_buffered_events()

func fail(message: String) -> void:
    push_error(message)
    quit(1)
