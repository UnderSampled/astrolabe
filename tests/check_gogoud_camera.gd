extends "res://check_manor_shots.gd"
## Original Gogoud route/action AI produces the fourth-shot trigger. World
## normal 2 uses the declared original excerpt; normal 3 remains unchanged.
## World reflex and unrelated Brains are held, never rewritten or simulated.
var route_frames: Array[Dictionary] = []
var actions: Array[int] = [0]
var trigger: Dictionary = {}

func route_snapshot(actor: CharacterBody3D, brain: Node, world: Node) -> Dictionary:
    var contacts: Array[Dictionary] = []
    for index in actor.get_slide_collision_count():
        var hit := actor.get_slide_collision(index)
        var collider := hit.get_collider() as CollisionObject3D
        var shape: CollisionShape3D
        if collider != null:
            var owner_id := collider.shape_find_owner(hit.get_collider_shape_index())
            shape = collider.shape_owner_get_owner(owner_id) as CollisionShape3D
        var source_node: Node = collider
        while source_node != null and not source_node.has_meta("source_node_id"):
            source_node = source_node.get_parent()
        var point := hit.get_position()
        var normal := hit.get_normal()
        contacts.append({"source_node":source_node.get_meta("source_node_id") if source_node != null else "",
            "source_material":shape.get_meta("source_material", "") if shape != null else "",
            "position":[point.x,-point.z,point.y],"normal":[normal.x,-normal.z,normal.y]})
    return {"frame":Engine.get_physics_frames(),"gogoud_clock_ms":brain.get("clock_milliseconds"),
        "world_clock_ms":world.get("clock_milliseconds"),"world_trigger":world.get("dsg_0"),
        "action":actor.current_state,"normal":brain.get("current_normal"),"failure":brain.get("unsupported_failure"),
        "position":[actor.global_position.x,-actor.global_position.z,actor.global_position.y],
        "on_floor":actor.is_on_floor(),"route_point":actor.source_route_points[6],
        "route_active":actor.source_route_active,"contacts":contacts}

func check() -> void:
    var scene := load("res://generated/hype/levels/manoir/level.tscn").instantiate() as Node3D
    for brain in scene.find_children("Brain", "Node", true, false): brain.set("automatic",false)
    var paths: Dictionary = scene.get_meta("source_actor_nodes")
    var camera_actor := scene.get_node(paths["level:/persos/world/0/2"]) as Node3D
    var world_actor := scene.get_node(paths["level:/persos/world/0/3"]) as Node3D
    var gogoud := scene.get_node(paths["level:/persos/world/0/5"]) as CharacterBody3D
    var hype := scene.get_node(paths["level:/persos/world/0/0"]) as Node3D
    hype.set("motion_enabled",false)
    var camera = camera_actor.get_node("SourceCamera")
    var camera_brain := camera_actor.get_node("Brain")
    var world := world_actor.get_node("Brain")
    var gogoud_brain := gogoud.get_node("Brain")
    root.add_child(scene)
    root.size = Vector2i(640,480)
    var fence := scene.get_node("world/1/9/0/Collision") as StaticBody3D
    if not require(fence.collision_layer == 0 and not bool(fence.get_meta("source_collision_reachable")) and fence.get_child_count() > 0,"source display fence became a blocker"): return
    if not require(world.get("current_normal") == 2 and world.get("dsg_0") == 0 and world.has_method("normal_2"),"source World initial state differs"): return
    if not require(gogoud_brain.get("current_normal") == 13 and gogoud.current_state == 18,"source Gogoud initial state differs"): return
    if not require(camera.motion_mode == 0 and camera.initial_motion_mode == 0 and camera.targeted_actor == null,"camera fixture changed initial mode/target"): return
    var mesh_count := scene.find_children("*", "MeshInstance3D", true, false).size()
    if not require(mesh_count > 0,"native manor geometry missing"): return
    var start := gogoud.global_position
    camera.make_current() # Declared viewport setup; shot state comes from AI.
    # Enter only the generated normal coroutine. start() would also start the
    # unbound-input reflex; no source reflex code or trigger values are edited.
    world.set("running",true)
    world.set("clock_milliseconds",0.0)
    world.call("normal_2",world.get("normal_revision"))
    world.set("automatic",true)
    if not require(bool(gogoud_brain.call("start")) and str(gogoud_brain.get("unsupported_failure")).is_empty() and gogoud_brain.get("current_normal") == 0 and gogoud.current_state == 0,"original Gogoud initialization failed"): return
    gogoud_brain.set("automatic",true)
    var wall_start := Time.get_ticks_usec()
    var previous_target: Node3D
    # Observed original poses are assertions only, never camera assignments.
    var observed := [
        [33,Vector3(-16.6989136,155.7436676,33.2094803),Vector3(0.7869909,0.5553228,0.2688158),Vector3(-0.21964015,-0.15498424,0.96319157),0.0],
        [18,Vector3(-8.26678085,147.5698853,41.2058792),Vector3(-0.6650815,0.54208004,-0.51363015),Vector3(-0.39813656,0.32450441,0.85801160),3600.0],
        [19,Vector3(-11.92900085,158.4262085,34.5001259),Vector3(-0.84144783,-0.53993773,0.02080646),Vector3(0.01751134,0.01123662,0.99978357),8100.0],
        [20,Vector3(-17.185480118,146.039352417,34.619735718),Vector3(-0.082333,0.996541,0.011291),Vector3(0.00092964555,-0.0112522142,0.9999362826),-1.0],
    ]
    for tick in range(1200):
        if not require(str(world.get("unsupported_failure")).is_empty() and str(gogoud_brain.get("unsupported_failure")).is_empty(),"compiled opening AI failed"): return
        var action_changed := actions[-1] != gogoud.current_state
        if action_changed: actions.append(gogoud.current_state)
        if tick % 3 == 0 or action_changed: route_frames.append(route_snapshot(gogoud,gogoud_brain,world))
        if world.get("dsg_0") != 0 and trigger.is_empty():
            trigger = route_snapshot(gogoud,gogoud_brain,world)
            if not require(trigger["world_trigger"] == 4 and actions == [0,26,24,23] and not gogoud.source_route_active,"fourth-shot trigger did not follow the original route/actions"): return
        var target := camera.targeted_actor as Node3D
        if target != previous_target:
            var index := shots.size()
            if not require(index < 4 and target == scene.get_node(paths["level:/persos/world/0/" + str(observed[index][0])]),"source selected wrong helper or extra cut"): return
            if not require(camera.motion_mode == 3,"source shot mode differs"): return
            var eye := camera_actor.global_position
            var eye_error: float = eye.distance_to(native(observed[index][1]))
            if not require(eye_error < 0.00005,"source waypoint eye differs"): return
            if not require(bool(camera_brain.call("normal_0_rule_0",camera_brain.get("normal_revision"))) and str(camera_brain.get("unsupported_failure")).is_empty(),"original camera aiming rule failed"): return
            var forward_error: float = (-camera.global_basis.z.normalized()).distance_to(native(observed[index][2]).normalized())
            var up_error: float = camera.global_basis.y.normalized().distance_to(native(observed[index][3]).normalized())
            var clock := float(world.get("clock_milliseconds"))
            if not require(forward_error < 0.00003 and up_error < 0.00003 and camera_actor.global_position.is_equal_approx(eye),"source shot orientation differs"): return
            if index < 3:
                if not require(clock >= observed[index][4] - 0.0001 and clock <= observed[index][4] + 40.0,"source Wait duration/order differs"): return
            else:
                if not require(not trigger.is_empty() and world.get("current_normal") == 3 and world.get("dsg_0") == 4,"fourth cut preceded its source producer"): return
                if not require(clock >= float(trigger["world_clock_ms"]) and clock - float(trigger["world_clock_ms"]) <= 40.0,"native fourth cut did not consume the source trigger promptly"): return
            var row := {"helper":observed[index][0],"tick":tick,"clock_ms":clock,
                "wall_ms":float(Time.get_ticks_usec()-wall_start)/1000.0,"eye":[eye.x,eye.y,eye.z],
                "eye_error":eye_error,"source_forward_error":forward_error,"source_up_error":up_error,
                "mode":camera.motion_mode,"world_trigger":world.get("dsg_0"),"gogoud_action":gogoud.current_state}
            row["image"] = await capture(index)
            shots.append(row)
            previous_target = target
        if shots.size() == 4: break
        await physics_frame
        await process_frame
    var finish := route_snapshot(gogoud,gogoud_brain,world)
    world.call("stop")
    gogoud_brain.call("stop")
    var has_source_contact := false
    for frame in route_frames:
        for contact in frame["contacts"]:
            if not str(contact["source_node"]).is_empty() and not str(contact["source_material"]).is_empty(): has_source_contact = true
    var file := FileAccess.open(OS.get_cmdline_user_args()[0],FileAccess.WRITE)
    if not require(file != null,"integration report open failed"): return
    file.store_string(JSON.stringify({"scope":"original Gogoud AI drives fourth camera cut; World initialization excerpt",
        "setup":"World reflex and unrelated Brains held; viewport and original camera aim-rule invocation are harness setup; no shot or trigger assignments",
        "shots":shots,"trigger":trigger,"actions":actions,"route_frames":route_frames,"finish":finish,
        "source_contact":has_source_contact,"mesh_count":mesh_count,"ending_behavior":world.get("current_normal"),
        "display_fence":{"layer":fence.collision_layer,"retained_shapes":fence.get_child_count()},
        "pending":["complete initialization and World reflex","later dialogue/presentation","original timing comparison","full intro"]}))
    file.close()
    if not require(shots.size() == 4 and not trigger.is_empty() and actions == [0,26,24,23],"original fourth-cut integration missed deadline"): return
    if not require(has_source_contact and gogoud.is_on_floor() and gogoud.global_position.distance_to(start) > 8.0,"Gogoud did not walk with source-associated native contact"): return
    if not require(gogoud.global_position.distance_to(Vector3(-18.638809,32.910141,-149.097321)) < 0.75,"Gogoud end pose differs from original capture"): return
    if not require(str(world.get("unsupported_failure")).is_empty() and str(gogoud_brain.get("unsupported_failure")).is_empty(),"integration ended with a compiled AI failure"): return
    print("MANOR_GOGOUD_FOURTH_SHOT_OK")
    scene.queue_free()
    await process_frame
    quit(0)
