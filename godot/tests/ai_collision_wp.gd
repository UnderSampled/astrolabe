extends SceneTree

const CHARACTER := preload("res://addons/astrolabe/character/character.gd")
const WAYPOINT := preload("res://addons/astrolabe/character/waypoint.gd")

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var level := Node3D.new()
	root.add_child(level)
	var query_context = CHARACTER.new()
	level.add_child(query_context)
	query_context.set_process(false)
	query_context.set_physics_process(false)
	var actor := Node3D.new()
	level.add_child(actor)

	var fixed = WAYPOINT.new()
	fixed.source_identity = "level:/waypoints/fixture/fixed"
	fixed.source_position = Vector3.ZERO
	fixed.radius = 5.0
	fixed.supported = true
	actor.global_position = Vector3(3.0, 0.0, -4.0) # source-space distance exactly 5
	if not _check(query_context.source_actor_in_waypoint_sphere(actor, fixed) == true, "inclusive radius boundary"):
		return
	actor.global_position = Vector3(3.0, 0.0, -4.001)
	if not _check(query_context.source_actor_in_waypoint_sphere(actor, fixed) == false, "outside radius"):
		return
	actor.global_position = Vector3(0.0, 4.0, 0.0) # full 3D vertical distance
	fixed.radius = 3.0
	if not _check(query_context.source_actor_in_waypoint_sphere(actor, fixed) == false, "full 3D distance"):
		return
	if not _check(query_context.source_actor_in_waypoint_sphere(actor, null) == null, "null waypoint remains unresolved"):
		return
	fixed.radius = -1.0
	if not _check(query_context.source_actor_in_waypoint_sphere(actor, fixed) == null, "invalid radius remains unresolved"):
		return

	var actor_path := "level:/persos/fixture/relative"
	var relative_actor := Node3D.new()
	level.add_child(relative_actor)
	level.set_meta("source_actor_nodes", {actor_path: level.get_path_to(relative_actor)})
	var relative = WAYPOINT.new()
	relative.source_position = Vector3(1.0, 0.0, 0.0)
	relative.radius = 0.5
	relative.relative_actor_path = actor_path
	relative.relative_actor_identity = "level:[17,1]/57"
	relative.supported = true
	relative_actor.global_position = Vector3(5.0, 0.0, 0.0)
	actor.global_position = Vector3(6.0, 0.0, 0.0)
	if not _check(query_context.source_actor_in_waypoint_sphere(actor, relative) == true, "relative waypoint at current actor transform"):
		return
	relative_actor.global_position = Vector3(7.0, 0.0, 0.0)
	if not _check(query_context.source_actor_in_waypoint_sphere(actor, relative) == false, "relative waypoint follows fresh actor transform"):
		return
	relative.relative_actor_path = "level:/persos/fixture/missing"
	if not _check(query_context.source_actor_in_waypoint_sphere(actor, relative) == null, "unresolved relative actor remains explicit"):
		return
	print("AI_COLLISION_WP_OK")
	quit(0)

func _check(condition: bool, label: String) -> bool:
	if not condition:
		push_error("AI_COLLISION_WP_FAILED: " + label)
		quit(1)
		return false
	return true
