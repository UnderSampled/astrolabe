extends RefCounted
## Spatial primitives used by the compiled dropped-item contact predicate.
## Actors and the resolved picking center are ordinary Godot references/values.
## Source AI retains eligibility, grant, waits, transparency and KillSelf.
## Godot's deterministic inverse is a candidate adaptation: the original ISO
## helper reads an uninitialized destination flag before selecting scale logic.
## Static call structure alone does not establish original numeric equivalence.

static func distance_z(receiver: Node3D, target: Node3D) -> float:
    var local := _target_local_delta(receiver, target)
    # Source Z is Godot Y. The original wrapper negates nonpositive results.
    return -local.y if local.y <= 0.0 else local.y

static func distance_xy(receiver: Node3D, target: Node3D) -> float:
    var local := _target_local_delta(receiver, target)
    var distance := Vector2(local.x, local.z).length()
    return -distance if distance <= 0.0 else distance

## reference_center is the world position of the first indexed sphere center
## in the receiver's picking bounds; use its origin only when bounds are absent.
## Binding that named center belongs to the exporter, not a runtime mesh parser.
static func is_above(receiver: Node3D, point: Vector3, threshold: float, reference_center: Vector3) -> bool:
    assert(is_instance_valid(receiver) and receiver.is_inside_tree())
    var local := receiver.global_basis.inverse() * (point - reference_center)
    if not local.y > 0.0:
        return false
    return local.y >= threshold if threshold >= 0.0 else local.y < -threshold

static func _target_local_delta(receiver: Node3D, target: Node3D) -> Vector3:
    assert(is_instance_valid(receiver) and receiver.is_inside_tree())
    assert(is_instance_valid(target) and target.is_inside_tree())
    return target.global_basis.inverse() * (receiver.global_position - target.global_position)
