extends SceneTree
## Point-turn and selected travel use maintained native character support.
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")
const ROUTE = preload("res://addons/astrolabe/character/route.gd")

func _initialize() -> void:
    call_deferred("check")

func require(ok: bool, message: String) -> bool:
    if ok: return true
    push_error(message)
    quit(1)
    return false

func check() -> void:
    var level := Node3D.new()
    root.add_child(level)
    var actor := CHARACTER.new()
    actor.autoplay = false
    var player := AnimationPlayer.new()
    player.name = "AnimationPlayer"
    actor.add_child(player)
    level.add_child(actor)
    var target := Node3D.new()
    level.add_child(target)
    actor.global_position = Vector3(2,3,4)
    # Source +X/negativeY convert to native +X/+Z; source Z is height.
    var start := actor.global_transform
    actor.external_velocity = Vector3(1,2,3)
    actor.velocity = Vector3(4,5,6)
    actor.source_set_vector_speed(Vector3(0,-1.4,0))
    if not require(actor.source_vector_speed.is_equal_approx(Vector3(0,-1.4,0)) and actor.external_velocity == Vector3.ZERO and actor.velocity == Vector3.ZERO, "SetVectorSpeed left prior caller velocity active"): return
    if not require(actor.global_transform.is_equal_approx(start), "SetVectorSpeed changed the actor pose before motion"): return
    actor.global_rotation.y = PI/2
    actor.source_set_impulse(Vector3(0,0,3))
    actor.source_set_vector_speed(Vector3(0,-2,0))
    actor.source_jump_absolute()
    if not require(actor.external_velocity.is_equal_approx(Vector3(2,3,0)), "SetVectorSpeed discarded the pending impulse or relative jump orientation"): return
    actor.source_set_vector_speed(Vector3.ZERO)
    actor.source_set_impulse(Vector3(0,0,4))
    actor.source_jump_without_adding_speed()
    if not require(actor.external_velocity.is_equal_approx(Vector3(0,4,0)), "cleared caller velocity leaked into the next jump request"): return
    actor.global_transform = start
    var angles: Vector2 = actor.source_turn_angles_to_position(Vector3(3,-4,90))
    if not require(absf(angles.x) < 0.000001 and absf(angles.y - PI/2) < 0.000001, "point turn mixed source height with horizontal direction"): return
    if not require(actor.global_transform.is_equal_approx(start), "point-turn query changed the actor pose"): return
    actor.global_rotation.y = TAU-0.1
    angles = actor.source_turn_angles_to_position(Vector3(2,-8,3))
    if not require(absf(angles.x - (TAU-0.1)) < 0.000001 and absf(angles.y - 0.1) < 0.000001, "point turn failed its wrapped shortest heading"): return
    # A world actor query and the equivalent source vector must agree even
    # when the actor and target have independent parent transforms.
    level.rotation.y = 0.4
    target.global_position = Vector3(-5,8,-2)
    var source_point := Vector3(target.global_position.x,-target.global_position.z,target.global_position.y)
    var actor_angles: Vector2 = actor.source_turn_angles(target)
    angles = actor.source_turn_angles_to_position(source_point)
    if not require(angles.is_equal_approx(actor_angles), "position and actor turn queries disagree in transformed world space"): return
    actor.global_rotation.y = 0.0
    source_point = Vector3(actor.global_position.x,-actor.global_position.z,actor.global_position.y+2)
    angles = actor.source_turn_angles_to_position(source_point)
    if not require(absf(angles.y - PI) < 0.000001, "zero horizontal point direction lost the source PI heading"): return
    # The two antipodal headings retain their original signed shortest delta.
    actor.global_basis = Basis.IDENTITY
    source_point = Vector3(actor.global_position.x,-actor.global_position.z+1.0,0)
    angles = actor.source_turn_angles_to_position(source_point)
    if not require(absf(angles.y-PI) < 0.000001, "positive PI turn changed direction"): return
    actor.global_basis = Basis(Vector3(-1,0,0),Vector3(0,1,0),Vector3(0,0,-1))
    source_point.y -= 2.0
    angles = actor.source_turn_angles_to_position(source_point)
    if not require(absf(angles.y+PI) < 0.000001, "negative PI turn changed direction"): return
    # Source matrix-vector multiplication observes the actual forward column;
    # a query must not orthogonalize the world transform before measuring it.
    actor.global_basis = Basis(Vector3(1,0,0),Vector3(0,1,0),Vector3(1,0,1))
    source_point = Vector3(actor.global_position.x+1.0,-actor.global_position.z-1.0,0)
    start = actor.global_transform
    angles = actor.source_turn_angles_to_position(source_point)
    if not require(absf(angles.x-PI/4.0) < 0.000001 and absf(angles.y) < 0.000001, "point turn changed the composed forward column"): return
    if not require(actor.global_transform.is_equal_approx(start), "scaled point-turn query changed the actor pose"): return
    angles = actor.source_turn_angles_to_position(Vector3(INF,0,0))
    if not require(not is_finite(angles.x) and not is_finite(angles.y), "invalid point turn did not fail before changing native state"): return
    # An actor's four progress fields must survive native scene storage. The
    # shared route resource does not own current/previous selections.
    actor.source_route_points = PackedInt32Array([0,2])
    actor.source_route_samples = PackedInt32Array([0,7])
    actor.source_route_previous_points = PackedInt32Array([-1,1])
    actor.source_route_previous_samples = PackedInt32Array([-1,6])
    var saved := PackedScene.new()
    if not require(saved.pack(actor) == OK, "actor progress scene could not be packed"): return
    if not require(ResourceSaver.save(saved,"res://travel_progress.tscn") == OK, "actor progress scene could not be saved"): return
    var reopened := (load("res://travel_progress.tscn") as PackedScene).instantiate()
    if not require(reopened.source_route_points == actor.source_route_points and reopened.source_route_samples == actor.source_route_samples and reopened.source_route_previous_points == actor.source_route_previous_points and reopened.source_route_previous_samples == actor.source_route_previous_samples, "native actor scene lost its current/previous waypoint progress"): return
    reopened.source_route_previous_points[1] = 0
    if not require(actor.source_route_previous_points[1] == 1, "actor instances shared mutable previous waypoint progress"): return
    reopened.free()
    if not await check_point_requests(): return
    if not await check_next_waypoint_requests(): return
    level.free()
    print("TRAVEL_POINT_TURN_GEOMETRY_OK")
    quit(0)

func new_moving_actor(world: Node3D, origin: Vector3, mode: int = 9) -> CharacterBody3D:
    var actor = CHARACTER.new()
    actor.autoplay = false
    actor.source_motion_binding_present = true
    var mechanics = load("res://addons/astrolabe/character/mechanics.gd").new()
    mechanics.mode = mode
    mechanics.mass = 1.0
    mechanics.ratio = 1.0
    mechanics.max_linear_speed = 1.0
    mechanics.max_angular_speed = 8.0
    actor.current_mechanics = mechanics
    var player := AnimationPlayer.new()
    player.name = "AnimationPlayer"
    player.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL
    actor.add_child(player)
    world.add_child(actor)
    actor.global_position = origin
    actor.source_set_vector_speed(Vector3(0,-1.4,0))
    return actor

func source_position(point: Vector3) -> Vector3:
    return Vector3(point.x,-point.z,point.y)

func box(world: Node3D, origin: Vector3, size: Vector3) -> void:
    var body := StaticBody3D.new()
    body.position = origin
    body.collision_layer = 1
    var shape := CollisionShape3D.new()
    var geometry := BoxShape3D.new()
    geometry.size = size
    shape.shape = geometry
    body.add_child(shape)
    world.add_child(body)

func check_point_requests() -> bool:
    var world := Node3D.new()
    root.add_child(world)
    var actor = new_moving_actor(world,Vector3.ZERO)
    actor.gravity = Vector3(-10,-20,30)
    var target := Vector3(0.3,0.5,0.4)
    var ticket: int = actor.source_begin_point_motion(source_position(target),0.04)
    if not require(ticket > 0, "finite Mass point request did not start"): return false
    for _tick in 64:
        await physics_frame
        if actor.source_point_motion_finished(ticket): break
    if not require(actor.source_point_motion_finished(ticket) and actor.source_point_motion_failure(ticket).is_empty(), "Mass point request did not complete"): return false
    if not require(actor.global_position.distance_to(target) <= 0.04 and actor.global_position.y > 0.45, "Mass point motion lost full3D arrival or used Character gravity"): return false
    if not require(actor.global_position.normalized().dot(target.normalized()) > 0.99999 and actor.global_basis.x.dot(Vector3(target.x,0,target.z).normalized()) > 0.99999, "Mass path or its source +X heading is incorrect"): return false
    var stopped: Vector3 = actor.global_position
    for _tick in 4: await physics_frame
    if not require(actor.global_position.is_equal_approx(stopped), "completed Mass point retained velocity"): return false
    # Meta25 reads its waypoint anew each update: moving it mid-request redirects.
    var waypoint = load("res://addons/astrolabe/character/waypoint.gd").new()
    waypoint.supported = true
    waypoint.radius = 0.04
    waypoint.source_position = source_position(Vector3(0.6,0.5,0.4))
    ticket = actor.source_begin_waypoint_motion(waypoint)
    if not require(ticket > 0, "finite waypoint request did not start"): return false
    for _tick in 2: await physics_frame
    var moved := Vector3(0.3,0.5,-0.2)
    waypoint.source_position = source_position(moved)
    for _tick in 96:
        await physics_frame
        if actor.source_point_motion_finished(ticket): break
    if not require(actor.source_point_motion_finished(ticket) and actor.source_point_motion_failure(ticket).is_empty() and actor.global_position.distance_to(moved) <= 0.04, "waypoint request did not follow its moved waypoint within its radius"): return false
    # Missing dynamics completes before the zero-radius snap. Finite equality
    # and the source's zero-radius0.25 fallback have separate assertions.
    actor.source_motion_binding_present = false
    actor.global_position = Vector3.ZERO
    if not require(actor.source_begin_point_motion(Vector3(0.12,0.16,0),0) == 0 and actor.global_position == Vector3.ZERO, "absent motion binding applied an arrival snap"): return false
    actor.source_motion_binding_present = true
    if not require(actor.source_begin_point_motion(Vector3(1,0,0),1) == 0 and actor.global_position == Vector3.ZERO, "inclusive radius arrival snapped a nonzero-radius point"): return false
    if not require(actor.source_begin_point_motion(Vector3(0.12,0.16,0),0) == 0 and actor.global_position.is_equal_approx(Vector3(0.12,0,-0.16)), "zero-radius nearby arrival did not snap all three coordinates"): return false
    actor.global_position = Vector3.ZERO
    if not require(actor.source_begin_point_motion(Vector3(0,0,100),0.05,true) == 0 and actor.global_position == Vector3.ZERO, "meta33 lost rounded-radius equality or used captured height"): return false
    ticket = actor.source_begin_point_motion(Vector3(0,0,100),0.049,true)
    for _tick in 4: await physics_frame
    if not require(ticket > 0 and not actor.source_point_motion_finished(ticket) and actor.global_position.y > 0.04 and actor.global_position.y < 0.2, "meta33 did not refresh current height+0.05 each update"): return false
    actor.source_cancel_point_motion(ticket)
    stopped = actor.global_position
    for _tick in 3: await physics_frame
    if not require(actor.source_point_motion_finished(ticket) and actor.global_position.is_equal_approx(stopped), "cancelled point request kept moving"): return false
    actor.free()
    # Character point heading observes the source's planar attenuation after
    # normalizing a full3D displacement, even without animation translation.
    actor = new_moving_actor(world,Vector3.ZERO,0)
    actor.current_mechanics.max_angular_speed = 1000.0
    actor.current_mechanics.ratio = 0.0
    ticket = actor.source_begin_point_motion(Vector3(0.3,-0.4,0.5),0.04)
    for _tick in 3: await physics_frame
    if not require(absf(actor.global_rotation.y-acos(0.4)) < 0.00001 and actor.global_position == Vector3.ZERO and not actor.source_point_motion_finished(ticket), "Character point replaced the source's attenuated heading or invented animation speed"): return false
    actor.source_cancel_point_motion(ticket)
    actor.current_mechanics.max_angular_speed = 8.0
    actor.global_rotation = Vector3.ZERO
    ticket = actor.source_begin_point_motion(Vector3(0,1,0),0.04)
    for _tick in 2: await physics_frame
    if not require(actor.global_rotation.y > 0.0 and actor.global_rotation.y < PI/2, "Character positive PI heading changed turn direction"): return false
    actor.source_cancel_point_motion(ticket)
    actor.global_rotation = Vector3(0,PI,0)
    var antipodal_start: float = actor.global_rotation.y
    ticket = actor.source_begin_point_motion(Vector3(0,-1,0),0.04)
    for _tick in 2: await physics_frame
    if not require(actor.global_rotation.y > PI/2 and actor.global_rotation.y < PI, "Character negative PI heading changed turn direction: start=%s after=%s" % [antipodal_start,actor.global_rotation.y]): return false
    actor.source_cancel_point_motion(ticket)
    actor.free()
    # Character0 keeps native animation speed, planar steering and body contact.
    box(world,Vector3(0,-0.5,0),Vector3(10,1,10))
    box(world,Vector3(0.6,1,0),Vector3(0.2,2,4))
    actor = new_moving_actor(world,Vector3(0,0.101,0),0)
    var shape := CollisionShape3D.new()
    var sphere := SphereShape3D.new()
    sphere.radius = 0.1
    shape.shape = sphere
    actor.add_child(shape)
    var motion := Node3D.new()
    motion.name = "RootMotion"
    actor.add_child(motion)
    var clip := Animation.new()
    clip.length = 1.0
    clip.loop_mode = Animation.LOOP_LINEAR
    var track := clip.add_track(Animation.TYPE_POSITION_3D)
    clip.track_set_path(track,^"RootMotion")
    clip.track_insert_key(track,0.0,Vector3.ZERO)
    clip.track_insert_key(track,1.0,Vector3(0,0,2))
    var library := AnimationLibrary.new()
    library.add_animation(&"walk",clip)
    var player: AnimationPlayer = actor.get_node("AnimationPlayer")
    player.root_motion_track = ^"RootMotion"
    player.add_animation_library(&"",library)
    player.play(&"walk")
    ticket = actor.source_begin_point_motion(Vector3(1,0,0.101),0.03)
    for _tick in 60: await physics_frame
    if not require(ticket > 0 and not actor.source_point_motion_finished(ticket) and actor.is_on_floor() and actor.is_on_wall() and actor.global_position.x < 0.41, "point steering bypassed Character native contact"): return false
    var ground_normal: Vector3 = actor.source_collide_normal(true)
    var wall_normal: Vector3 = actor.source_collide_normal(false)
    if not require(ground_normal.z > 0.99 and wall_normal.x < -0.9 and absf(wall_normal.z) < 0.01, "collide normals are not the floor and wall contact normals in source axes: ground=%s wall=%s" % [ground_normal, wall_normal]): return false
    actor.source_cancel_point_motion(ticket)
    stopped = actor.global_position
    for _tick in 4: await physics_frame
    # Floor safe-margin recovery may lift the stopped body slightly. Steering
    # is horizontal, and must stop independently of that contact correction.
    if not require(Vector2(actor.global_position.x,actor.global_position.z).distance_to(Vector2(stopped.x,stopped.z)) < 0.0001 and actor.velocity == Vector3.ZERO and actor.is_on_floor(), "cancelled Character point request kept animation steering: before=%s after=%s velocity=%s" % [stopped,actor.global_position,actor.velocity]): return false
    world.free()
    return true

func route(points: Array[Vector3], links: PackedByteArray, closed: bool = false) -> Resource:
    var result = ROUTE.new()
    result.curve = Curve3D.new()
    result.radii = PackedFloat32Array()
    result.point_supported = PackedByteArray()
    result.link_supported = links
    result.closed = closed
    # A later unsupported link must not reject an earlier selected fixed point.
    result.supported = false
    for point in points:
        result.curve.add_point(point)
        result.radii.append(0.03)
        result.point_supported.append(1)
    return result

func bind_route(actor: CharacterBody3D, selected: Resource) -> void:
    var bindings: Array[Resource] = [selected,null]
    actor.source_routes = bindings
    actor.source_route_points = PackedInt32Array([0,0])
    actor.source_route_samples = PackedInt32Array([0,0])
    actor.source_route_previous_points = PackedInt32Array([-1,-1])
    actor.source_route_previous_samples = PackedInt32Array([-1,-1])

func check_next_waypoint_requests() -> bool:
    var world := Node3D.new()
    root.add_child(world)
    var actor = new_moving_actor(world,Vector3.ZERO)
    var selected := route([Vector3.ZERO,Vector3(0.4,0.3,0),Vector3(0.8,0.6,0)],PackedByteArray([1,0]))
    bind_route(actor,selected)
    if not require(actor.source_begin_next_waypoint(256) == 0 and actor.source_route_points[0] == 1 and actor.source_route_previous_points[0] == 0 and actor.source_route_samples[0] == 0 and actor.source_route_previous_samples[0] == 0, "next-waypoint did not advance exactly one initially selected point using the low byte slot"): return false
    # An invalid lookup keeps the previously captured nonempty route.
    var ticket: int = actor.source_begin_next_waypoint(1)
    if not require(ticket > 0 and actor.source_route_points[0] == 1 and not actor.source_point_motion_finished(ticket), "null lookup replaced the retained waypoint route"): return false
    for _tick in 40:
        await physics_frame
        if actor.source_point_motion_finished(ticket): break
    if not require(actor.source_point_motion_finished(ticket) and actor.source_point_motion_failure(ticket).contains("unsupported link") and actor.source_route_points[0] == 2 and actor.source_route_previous_points[0] == 1, "reached curved link did not fail after preserving its native progress"): return false
    if not require(actor.global_position.distance_to(Vector3(0.4,0.3,0)) <= 0.03 and actor.source_route_points[1] == 0, "selected StaticLine failed full3D arrival or changed another binding"): return false
    # Reset restores all four source values and cancels only the matching slot.
    if not require(actor.source_reset_route(0) and actor.source_route_points[0] == 0 and actor.source_route_samples[0] == 0 and actor.source_route_previous_points[0] == -1 and actor.source_route_previous_samples[0] == -1, "route reset lost a current or previous field"): return false
    # Distinct coincident vertices remain distinct updates; no route scan skips
    # the second one merely because both happen to be at the actor's position.
    actor.global_position = Vector3.ZERO
    selected = route([Vector3.ZERO,Vector3.ZERO,Vector3(1,0,0)],PackedByteArray([1,1]))
    bind_route(actor,selected)
    if not require(actor.source_begin_next_waypoint(0) == 0 and actor.source_route_points[0] == 1, "coincident vertices were scanned in one waypoint call"): return false
    if not require(actor.source_begin_next_waypoint(0) == 0 and actor.source_route_points[0] == 2 and actor.source_route_previous_points[0] == 1, "second coincident vertex did not retain separate progress"): return false
    # StaticLine reports last-sample regardless of its sample value. Increment
    # still wraps as a byte and preserves the old value in previous sample.
    actor.source_route_points[0] = 1
    actor.source_route_previous_points[0] = 0
    actor.source_route_samples[0] = 255
    actor.source_route_previous_samples[0] = 254
    if not require(actor.source_begin_next_waypoint(0) == 0 and actor.source_route_points[0] == 2 and actor.source_route_samples[0] == 0 and actor.source_route_previous_samples[0] == 255, "StaticLine advance lost byte wrapping or previous sample"): return false
    # Open terminal resets and completes; a closed terminal resets but retains
    # one continuation until its next update, without scanning the whole loop.
    selected = route([Vector3.ZERO,Vector3.ZERO],PackedByteArray([1]))
    bind_route(actor,selected)
    actor.source_route_points[0] = 1
    actor.source_route_previous_points[0] = 0
    if not require(actor.source_begin_next_waypoint(0) == 0 and actor.source_route_points[0] == 0 and actor.source_route_previous_points[0] == -1 and actor.source_route_previous_samples[0] == -1, "open terminal did not reset and complete"): return false
    selected.closed = true
    actor.source_route_points[0] = 1
    actor.source_route_previous_points[0] = 0
    ticket = actor.source_begin_next_waypoint(0)
    if not require(ticket > 0 and actor.source_route_points[0] == 0 and actor.source_route_previous_points[0] == -1, "closed terminal lost its reset continuation"): return false
    for _tick in 2: await physics_frame
    if not require(actor.source_point_motion_finished(ticket) and actor.source_route_points[0] == 1, "closed waypoint continuation did not yield before the next vertex"): return false
    selected.point_supported[0] = 0
    actor.source_reset_route(0)
    if not require(actor.source_begin_next_waypoint(0) == -1 and actor.source_point_motion_failure(-1).contains("relative geometry") and actor.source_route_points[0] == 0, "unsupported selected point changed progress or silently completed"): return false
    world.free()
    return true
