extends Resource
## Source waypoint geometry used by compiled AI queries.
## Position/radius are retained in Hype coordinates; actor-relative points are
## transformed from their linked actor's current world transform on every read.
@export var source_position: Vector3 = Vector3.ZERO
@export var radius: float = 0.0
@export var relative_actor_path: String = ""
@export var relative_actor_identity: String = ""
@export var source_identity: String = ""
@export var supported: bool = false

func source_world_position(context: Node) -> Variant:
    if not supported or not source_position.is_finite():
        return null
    if relative_actor_path.is_empty():
        return source_position
    if not is_instance_valid(context) or not context.has_method("source_actor_node"):
        return null
    var actor: Variant = context.call("source_actor_node", relative_actor_path)
    if not actor is Node3D or not is_instance_valid(actor) or not actor.is_inside_tree():
        return null
    var local_position := Vector3(source_position.x, source_position.z, -source_position.y)
    var world_position: Vector3 = actor.global_transform * local_position
    if not world_position.is_finite():
        return null
    return Vector3(world_position.x, -world_position.z, world_position.y)
