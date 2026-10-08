extends RefCounted
## Material predicate for the actual target shape returned by native contact.
## Cond28's third operand is a mask; its independent result limit is one.
## This primitive does not choose sectors, actor activation or collision zones.
## Compiled barrel/dummy AI owns queue timing, animation, damage and drops.

static func material_matches(identifier: int, required: int) -> bool:
    var mask := required & 0xffff
    return (identifier & mask) == mask

## Reject missing materials even for mask zero. Never substitute a different
## shape's material after an intersection selected this target shape.
static func contact_material_matches(collider: CollisionObject3D, shape_index: int, required: int) -> bool:
    if not is_instance_valid(collider) or shape_index < 0:
        return false
    # Validate the shape index before lookup: shape_find_owner reports an
    # engine error for an index which no longer belongs to this collider.
    for owner_id in collider.get_shape_owners():
        for local_index in collider.shape_owner_get_shape_count(owner_id):
            if collider.shape_owner_get_shape_index(owner_id, local_index) != shape_index:
                continue
            var shape := collider.shape_owner_get_owner(owner_id) as CollisionShape3D
            if shape == null or not shape.has_meta("source_collision_identifier"):
                return false
            if not bool(shape.get_meta("source_contact_material_present", true)):
                return false
            return material_matches(int(shape.get_meta("source_collision_identifier")), required)
    return false
