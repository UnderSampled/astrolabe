extends SceneTree
## Actual exported model-437 rule and manor assets. Other Brains are held.
## Recorded fixed-shot placements and later eye motion are explicit test setup.
var samples: Array[Dictionary] = []
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

func measure(actor: Node3D, camera: Camera3D, target: Node3D) -> Dictionary:
    var actor_offset: Vector3 = camera.get("actor_offset")
    var look_offset: Vector3 = camera.get("look_offset")
    var point: Vector3 = target.global_position + target.global_basis.orthonormalized() * (actor_offset + (look_offset if look_offset != Vector3.ZERO else Vector3(0,0,0.05)))
    var expected := (point - actor.global_position).normalized()
    var forward := -camera.global_basis.z.normalized()
    return {"eye": [actor.global_position.x,actor.global_position.y,actor.global_position.z],
        "target": [target.global_position.x,target.global_position.y,target.global_position.z],
        "forward": [forward.x,forward.y,forward.z], "aim_error": forward.distance_to(expected)}

func capture(name: String) -> Dictionary:
    if DisplayServer.get_name() == "headless": return {"status": "not_captured_headless"}
    await process_frame
    RenderingServer.force_draw()
    var image := root.get_texture().get_image()
    if not require(image != null and not image.is_empty(), "rendered manor frame missing"): return {}
    var path := OS.get_cmdline_user_args()[0].get_base_dir().path_join(name + ".png")
    if not require(image.save_png(path) == OK, "could not save rendered manor frame"): return {}
    return {"status": "captured", "path": path, "width": image.get_width(), "height": image.get_height()}

func check() -> void:
    var scene := load("res://generated/hype/levels/manoir/level.tscn").instantiate() as Node3D
    for brain in scene.find_children("Brain", "Node", true, false): brain.automatic = false
    var paths: Dictionary = scene.get_meta("source_actor_nodes")
    var actor := scene.get_node(paths["level:/persos/world/0/2"]) as Node3D
    var hype := scene.get_node(paths["level:/persos/world/0/0"]) as Node3D
    hype.set("motion_enabled", false)
    var camera = actor.get_node("SourceCamera")
    # The game session binds one controller state per level; this bounded check supplies it.
    camera.controller_state = load("res://addons/astrolabe/camera/source_controller_state.gd").new()
    var brain := actor.get_node("Brain")
    root.add_child(scene)
    root.size = Vector2i(640,480)
    if not require(brain.has_method("normal_0_rule_0") and brain.current_normal == 0, "actual first camera rule missing"): return
    if not require(camera.motion_mode == 3 and camera.actor_offset.is_equal_approx(Vector3(0,1.8,0)) and camera.look_offset == Vector3.ZERO, "persisted mode/offsets differ"): return
    if not require(camera.keep_aspect == Camera3D.KEEP_WIDTH and absf(deg_to_rad(camera.fov) - 1.5) < 0.000001 and camera.near == 0.25 and camera.far == 10000.0, "source framing differs"): return
    var mesh_count := scene.find_children("*", "MeshInstance3D", true, false).size()
    if not require(mesh_count > 0, "converted manor geometry missing"): return
    camera.make_current() # Viewport selection is harness setup, not source AI.
    # First fixed views from the same paused source captures used for the
    # verified actor/look offset derivation. Helpers keep their exported poses.
    var observed := [
        [33, Vector3(-16.6989136,155.7436676,33.2094803), Vector3(0.7869909,0.5553228,0.2688158), Vector3(-0.21964015066623688,-0.1549842357635498,0.963191568851471)],
        [18, Vector3(-8.266780853271484,147.56988525390625,41.20587921142578), Vector3(-0.6650815010070801,0.5420800447463989,-0.5136301517486572), Vector3(-0.39813655614852905,0.32450440526008606,0.8580116033554077)],
        [19, Vector3(-11.929000854492188,158.42620849609375,34.500125885009766), Vector3(-0.8414478302001953,-0.5399377346038818,0.020806461572647095), Vector3(0.017511341720819473,0.011236624792218208,0.9997835755348206)],
        [20, Vector3(-17.1854801,146.0393524,34.6197357), Vector3(-0.082333,0.996541,0.011291), Vector3(0.0009296455536969006,-0.011252214200794697,0.9999362826347351)],
    ]
    for index in observed.size():
        var target := scene.get_node(paths["level:/persos/world/0/" + str(observed[index][0])]) as Node3D
        actor.global_position = native(observed[index][1])
        var eye := actor.global_position
        if not require(bool(actor.call("source_camera_change_targeted_perso", "level:/persos/world/0/2", target)), "helper binding failed"): return
        brain.set("dsg_30", 1)
        if not require(bool(brain.call("normal_0_rule_0", brain.normal_revision)) and brain.unsupported_failure.is_empty(), "actual compiled camera rule failed"): return
        var row := measure(actor,camera,target)
        row["source_forward_error"] = (-camera.global_basis.z.normalized()).distance_to(native(observed[index][2]).normalized())
        row["source_up_error"] = camera.global_basis.y.normalized().distance_to(native(observed[index][3]).normalized())
        row["helper"] = observed[index][0]
        if not require(row.aim_error < 0.00001 and row.source_forward_error < 0.00003 and row.source_up_error < 0.00003 and actor.global_position.is_equal_approx(eye) and brain.get("dsg_30") == 0, "source view/unchanged eye/rule assignment differs: " + str(row)): return
        row["image"] = await capture("opening-" + str(index))
        shots.append(row)
    var target := scene.get_node(paths["level:/persos/world/0/33"]) as Node3D
    actor.call("source_camera_change_targeted_perso", "level:/persos/world/0/2", target)
    var start := actor.position
    var player := AnimationPlayer.new()
    actor.add_child(player)
    var animation := Animation.new()
    animation.length = 0.6
    var track := animation.add_track(Animation.TYPE_VALUE)
    animation.track_set_path(track, NodePath(".:position"))
    animation.track_insert_key(track,0.0,start)
    animation.track_insert_key(track,0.6,start + Vector3(2,0.5,-1))
    var library := AnimationLibrary.new()
    library.add_animation("eye",animation)
    player.add_animation_library("",library)
    player.play("eye")
    # Native playback advances the eye. Only the original compiled rule aims it.
    for tick in range(45):
        await physics_frame
        await process_frame
        var eye := actor.global_position
        if not require(bool(brain.call("normal_0_rule_0", brain.normal_revision)) and brain.unsupported_failure.is_empty(), "moving-eye compiled rule failed"): return
        var row := measure(actor,camera,target)
        row["tick"] = tick
        if not require(row.aim_error < 0.00001 and actor.global_position.is_equal_approx(eye), "moving-eye aim differs: " + str(row)): return
        samples.append(row)
    if not require(actor.position.distance_to(start) > 2.0, "native eye animation did not move"): return
    var moving_image: Dictionary = await capture("moving-eye-end")
    # Rotation is applied to the source offsets, without target scale.
    target.rotation.y = PI * 0.5
    target.scale = Vector3(2,3,4)
    camera.look_offset = Vector3(0.3,0.2,-0.4) # Declared component edge-case setup.
    var rotated_point := target.global_position + Vector3(-0.4,2.0,-0.3)
    if not require(bool(brain.call("normal_0_rule_0", brain.normal_revision)) and (-camera.global_basis.z.normalized()).distance_to((rotated_point - actor.global_position).normalized()) < 0.00001, "rotated target offsets were scaled or lost"): return
    var before := actor.global_transform
    camera.motion_mode = 1 # modes 0, 3 and 6 are implemented
    if not require(not bool(actor.call("source_camera_update_position")) and actor.global_transform == before, "unsupported mode changed the camera"): return
    camera.motion_mode = 3
    camera.targeted_actor = null
    if not require(not bool(actor.call("source_camera_update_position")) and actor.global_transform == before, "missing target changed the camera"): return
    camera.targeted_actor = target
    camera.actor_offset = Vector3.ZERO
    camera.look_offset = Vector3(0.1,0,0)
    target.global_basis = Basis.IDENTITY
    target.global_position = actor.global_position - camera.look_offset
    if not require(not bool(actor.call("source_camera_update_position")) and actor.global_transform == before, "coincident target changed the camera"): return
    target.global_position += Vector3.UP
    if not require(not bool(actor.call("source_camera_update_position")) and actor.global_transform == before, "vertical target changed the camera"): return
    var file := FileAccess.open(OS.get_cmdline_user_args()[0],FileAccess.WRITE)
    file.store_string(JSON.stringify({"scope": "actual_model_437_rule_component", "mesh_count": mesh_count, "shots": shots,
        "native_motion_samples": samples, "moving_image": moving_image,
        "motion": "authored native AnimationPlayer test setup; original shot commands/timing unverified",
        "guards": ["unsupported mode", "missing target", "coincident target", "vertical target"],
        "pending": ["initial source target", "viewport selection", "original shot commands/timing", "follow/obstruction", "full intro"]}) + "\n")
    file.close()
    scene.queue_free()
    await process_frame
    print("MANOR_SOURCE_CAMERA_RULE_OK")
    quit(0)
