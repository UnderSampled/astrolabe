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
    current_scene = menu
    var load_button := menu.find_child("LoadGame",true,false) as Button
    var new_button := menu.find_child("NewGame",true,false) as Button
    if load_button == null or load_button.text.to_lower() != "load game" or load_button.disabled or new_button == null or new_button.disabled:
        fail("Game menu must offer enabled Load Game and New Game actions after manor generation")
        return
    load_button.pressed.emit()
    var transition_deadline := Time.get_ticks_msec() + 5000
    while (current_scene == null or current_scene == menu) and Time.get_ticks_msec() < transition_deadline:
        await process_frame
    var demo := current_scene
    if demo == null or demo == menu:
        fail("Load Game did not open the game session")
        return
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
    print("SMOKE: first session playable")
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
    var first_actor_id := actor.get_instance_id()
    var first_inventory_id := (actor.get("source_inventory") as Resource).get_instance_id()
    var first_magick_points := int((actor.get("source_magick") as Resource).get("current_points"))
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
    print("SMOKE: movement passed")
    (actor.get("source_inventory") as Resource).set("quantities", {999: 42})
    (actor.get("source_magick") as Resource).set("current_points", 1)
    actor.velocity = Vector3(5, 0, 0)
    brain.set("current_normal", -1)
    print("SMOKE: requesting reload")
    key(KEY_R, true)
    key(KEY_R, false)
    transition_deadline = Time.get_ticks_msec() + 5000
    while (current_scene == null or current_scene == demo) and Time.get_ticks_msec() < transition_deadline:
        await process_frame
    var reloaded := current_scene
    if reloaded == null or reloaded == demo:
        fail("Reload did not replace the old session")
        return
    deadline = Time.get_ticks_msec() + 90000
    while not bool(reloaded.get("ready_to_play")) and Time.get_ticks_msec() < deadline:
        await process_frame
    if not bool(reloaded.get("ready_to_play")):
        fail("Reload did not become playable")
        return
    print("SMOKE: reloaded session playable")
    var fresh_actor := reloaded.get("actor") as CharacterBody3D
    var fresh_brain := reloaded.get("brain") as Node
    var fresh_inventory := fresh_actor.get("source_inventory") as Resource
    var fresh_magick := fresh_actor.get("source_magick") as Resource
    if fresh_actor.get_instance_id() == first_actor_id or fresh_inventory.get_instance_id() == first_inventory_id or not fresh_actor.global_position.is_equal_approx(start):
        fail("Reload reused mutable actor state")
        return
    if fresh_inventory.get("quantities").has(999) or int(fresh_magick.get("current_points")) != first_magick_points or absf(fresh_actor.velocity.x) > 0.1 or not bool(fresh_brain.get("running")):
        fail("Inventory, magick, physics, or Brain leaked across reload")
        return
    if not is_equal_approx(reloaded.get("level").get_node("world/0/15/iMGrille_Manoir_Pont").position.y, 5.2):
        fail("The bridge gate moved twice across reload")
        return
    var args := OS.get_cmdline_user_args()
    if not args.is_empty():
        await RenderingServer.frame_post_draw
        var picture := root.get_texture().get_image()
        if picture == null or picture.save_png(args[0]) != OK:
            fail("Demo screenshot could not be saved")
            return
    key(KEY_ESCAPE, true)
    key(KEY_ESCAPE, false)
    await process_frame
    var pause_host := reloaded.get_node("PauseHost")
    var pause_view := pause_host.get("view") as Control
    if not paused or current_scene != reloaded or pause_view == null or not pause_view.visible:
        fail("Escape did not pause the current session")
        return
    var menu_button := pause_view.call("button_for_action", &"Main Menu") as Button
    if menu_button == null:
        fail("Pause overlay is missing Main Menu")
        return
    print("SMOKE: requesting menu from pause")
    menu_button.pressed.emit()
    transition_deadline = Time.get_ticks_msec() + 5000
    while (current_scene == null or current_scene == reloaded) and Time.get_ticks_msec() < transition_deadline:
        await process_frame
    if paused or not current_scene is Control:
        fail("Pause Main Menu did not return to the menu")
        return
    if root.get_node_or_null("GameContent") == null or root.get_node("GameContent").get("level_scene") == null:
        fail("Menu transition discarded the single retained level resource")
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
