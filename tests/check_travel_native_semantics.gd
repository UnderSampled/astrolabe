extends SceneTree
## Point-turn geometry uses maintained native character support.
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")

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
    level.free()
    print("TRAVEL_POINT_TURN_GEOMETRY_OK")
    quit(0)
