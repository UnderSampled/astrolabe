extends SceneTree
## Original World shot commands, extracted after unsupported initialization.
## The harness drives its native coroutine clock and the unchanged camera rule.
## It never assigns the shot eye, target or motion mode.
var shots: Array[Dictionary] = []

func _initialize() -> void:
    call_deferred("check")

func require(ok: bool, message: String) -> bool:
    if ok: return true
    push_error(message)
    quit(1)
    return false

func native(source: Vector3) -> Vector3:
    return Vector3(source.x,source.z,-source.y)

func capture(index: int) -> Dictionary:
    if DisplayServer.get_name() == "headless": return {"status":"not_captured_headless"}
    await process_frame
    RenderingServer.force_draw()
    var image := root.get_texture().get_image()
    if not require(image != null and not image.is_empty(), "shot render missing"): return {}
    var path := OS.get_cmdline_user_args()[0].get_base_dir().path_join("source-shot-" + str(index) + ".png")
    if not require(image.save_png(path) == OK, "shot PNG save failed"): return {}
    return {"status":"captured","path":path,"width":image.get_width(),"height":image.get_height()}

func check() -> void:
    var scene := load("res://generated/hype/levels/manoir/level.tscn").instantiate() as Node3D
    for brain in scene.find_children("Brain", "Node", true, false): brain.automatic = false
    var paths: Dictionary = scene.get_meta("source_actor_nodes")
    var camera_actor := scene.get_node(paths["level:/persos/world/0/2"]) as Node3D
    var world_actor := scene.get_node(paths["level:/persos/world/0/3"]) as Node3D
    var hype := scene.get_node(paths["level:/persos/world/0/0"]) as Node3D
    hype.set("motion_enabled",false)
    var camera = camera_actor.get_node("SourceCamera")
    var camera_brain := camera_actor.get_node("Brain")
    var world := world_actor.get_node("Brain")
    root.add_child(scene)
    root.size = Vector2i(640,480)
    if not require(world.get("current_normal") == 2 and world.has_method("normal_2_schedule"), "source World initial behavior differs"): return
    if not require(camera.motion_mode == 0 and camera.initial_motion_mode == 0 and camera.targeted_actor == null, "camera fixture changed initial mode/target"): return
    var mesh_count := scene.find_children("*", "MeshInstance3D", true, false).size()
    if not require(mesh_count > 0, "exported manor geometry missing"): return
    camera.make_current() # Viewport activation remains declared harness setup.
    world.set("running",true)
    world.set("clock_milliseconds",0.0)
    var wall_start := Time.get_ticks_usec()
    world.call("normal_2_schedule",world.get("normal_revision"))
    var previous_target: Node3D
    # Expected original capture values are assertions only, never assignments.
    var observed := [
        [33,Vector3(-16.6989136,155.7436676,33.2094803),Vector3(0.7869909,0.5553228,0.2688158),Vector3(-0.21964015,-0.15498424,0.96319157),0.0],
        [18,Vector3(-8.26678085,147.5698853,41.2058792),Vector3(-0.6650815,0.54208004,-0.51363015),Vector3(-0.39813656,0.32450441,0.85801160),3600.0],
        [19,Vector3(-11.92900085,158.4262085,34.5001259),Vector3(-0.84144783,-0.53993773,0.02080646),Vector3(0.01751134,0.01123662,0.99978357),8100.0],
    ]
    for tick in range(600):
        if not require(str(world.get("unsupported_failure")).is_empty(), "source shot command failed"): return
        var target := camera.targeted_actor as Node3D
        if target != previous_target:
            var index := shots.size()
            if not require(index < 3 and target == scene.get_node(paths["level:/persos/world/0/" + str(observed[index][0])]), "source selected wrong helper or extra cut"): return
            if not require(camera.motion_mode == 3, "source ChangeState did not select mode 3"): return
            var eye_error := camera_actor.global_position.distance_to(native(observed[index][1]))
            if not require(eye_error < 0.00005, "source waypoint placement differs"): return
            var eye := camera_actor.global_position
            if not require(bool(camera_brain.call("normal_0_rule_0",camera_brain.get("normal_revision"))) and str(camera_brain.get("unsupported_failure")).is_empty(), "unchanged camera aiming rule failed"): return
            var forward_error := (-camera.global_basis.z.normalized()).distance_to(native(observed[index][2]).normalized())
            var up_error := camera.global_basis.y.normalized().distance_to(native(observed[index][3]).normalized())
            var clock := float(world.get("clock_milliseconds"))
            if not require(forward_error < 0.00003 and up_error < 0.00003 and camera_actor.global_position.is_equal_approx(eye), "source-selected camera orientation differs"): return
            if not require(clock >= observed[index][4] - 0.0001 and clock <= observed[index][4] + 40.0, "original Wait duration or order differs"): return
            var row := {"helper":observed[index][0],"tick":tick,"clock_ms":clock,
                "wall_ms":float(Time.get_ticks_usec()-wall_start)/1000.0,
                "eye":[eye.x,eye.y,eye.z],"eye_error":eye_error,
                "source_forward_error":forward_error,"source_up_error":up_error,"mode":camera.motion_mode}
            row["image"] = await capture(index)
            shots.append(row)
            previous_target = target
        if world.get("current_normal") == 3: break
        await physics_frame
        # Drive the maintained Brain's clock and its existing native coroutine
        # wake-up, while holding unrelated source initialization/behaviors.
        world.set("clock_milliseconds",float(world.get("clock_milliseconds"))+root.get_physics_process_delta_time()*1000.0)
        world.emit_signal("normal_schedule_tick")
    if not require(shots.size() == 3 and world.get("current_normal") == 3, "three source cuts did not reach behavior 3 within deadline"): return
    world.set("running",false)
    var pose := camera_actor.global_transform
    var target := camera.targeted_actor as Node3D
    if not require(not bool(world_actor.call("source_camera_change_state","level:/persos/world/0/2",2)) and camera.motion_mode == 3 and camera_actor.global_transform.is_equal_approx(pose), "unsupported mode transition changed state"): return
    if not require(bool(world_actor.call("source_camera_change_state","level:/persos/world/0/2",3)) and camera_actor.global_transform.is_equal_approx(pose), "same mode request changed pose"): return
    if not require(bool(world_actor.call("source_camera_change_state","level:/persos/world/0/2",-1)) and camera.motion_mode == 0 and camera.targeted_actor == target and camera_actor.global_transform.is_equal_approx(pose), "source -1 did not restore initial mode without moving eye/target"): return
    var file := FileAccess.open(OS.get_cmdline_user_args()[0],FileAccess.WRITE)
    if not require(file != null,"shot report open failed"): return
    file.store_string(JSON.stringify({"shots":shots,"mesh_count":mesh_count,"ending_behavior":world.get("current_normal"),
        "guards":["unsupported transition","equal state","-1 restores original preset"],
        "scope":"World behavior-2 shot excerpt; original commands and waits, held earlier initialization/guards and other Brains; viewport and clock stepping are harness setup"}))
    print("MANOR_SOURCE_SHOT_EXCERPT_OK")
    scene.queue_free()
    await process_frame
    quit(0)
