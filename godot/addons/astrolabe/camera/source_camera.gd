extends Camera3D
## Native target and framing addressed by the compiled camera AI API.
## Mode 3 uses the actor's eye position and aims at the selected target.
## Follow modes, viewport selection and procedure reset fields remain separate.

@export var motion_mode: int = 0
@export var initial_motion_mode: int = 0
@export var actor_offset: Vector3 = Vector3.ZERO
@export var look_offset: Vector3 = Vector3.ZERO
@export var initially_current: bool = false
@export_node_path("Node3D") var initial_target_path: NodePath
@export var mode_zero_target_offset: Vector3 = Vector3.ZERO
@export var mode_zero_actor_offset: Vector3 = Vector3.ZERO
@export var mode_zero_look_offset: Vector3 = Vector3.ZERO
@export var mode_zero_distance: Vector2 = Vector2.ZERO
@export var mode_zero_target_child_index: int = 0
@export var controller_state: Resource

var targeted_actor: Node3D
var _reported_mode_zero_gap := false
var _source_active_sector: Node

func _ready() -> void:
    motion_mode = initial_motion_mode
    if not initial_target_path.is_empty():
        targeted_actor = get_node_or_null(initial_target_path) as Node3D
    _source_active_sector = _source_sector_for(get_parent())
    if initially_current:
        make_current()
    else:
        clear_current(false)

func source_change_targeted_perso(actor: Node3D) -> bool:
    if not is_instance_valid(actor):
        return false
    targeted_actor = actor
    return true

## 00547be0 restores the original preset for -1 and leaves equal modes alone.
## The observed opening transition 0 -> 3 has no extra reset branches.
## Other transitions require follow/obstruction state that is not mapped yet.
func source_change_state(state: int) -> bool:
    var selected := initial_motion_mode if state == -1 else state
    if selected == motion_mode:
        return true
    if motion_mode not in [0,3] or selected not in [0,3]:
        return false
    motion_mode = selected
    return true

## Hype 0051c8d0/0051d140 rotate the two offsets through the target's rotation.
## Mode 3 leaves ordinary eye motion to the camera actor (005200a0/0051e620).
## Zero look offset defaults to source [0,-0.05,0], or native [0,0,0.05].
func source_update_position() -> bool:
    if motion_mode == 0:
        return _source_update_mode_zero()
    if motion_mode != 3 or not is_instance_valid(targeted_actor):
        return false
    var eye := get_parent() as Node3D
    if eye == null or not eye.is_inside_tree() or not targeted_actor.is_inside_tree():
        return false
    var offset := look_offset if look_offset != Vector3.ZERO else Vector3(0,0,0.05)
    var point := targeted_actor.global_position + targeted_actor.global_basis.orthonormalized() * (actor_offset + offset)
    var direction := point - eye.global_position
    if not point.is_finite() or not direction.is_finite() or direction.length_squared() < 0.0000000001:
        return false
    if direction.normalized().cross(Vector3.UP).length_squared() < 0.0000000001:
        return false
    # Converted source -Y is the actor's +Z. The saved child half-turn makes
    # Camera3D's -Z look in that same direction. Update the meaningful actor pose.
    eye.look_at(point, Vector3.UP, true)
    return true

## Exposes the mapped 0051c8d0/0051d330/0051d140 geometry. The next source
## controller stage supplies its smoothed eye (+0x30) and collision result.
func source_mode_zero_target_geometry() -> Dictionary:
    if mode_zero_target_child_index != -1:
        push_error("Source camera target child selector has no native hierarchy binding")
        return {}
    if not is_instance_valid(targeted_actor) or not targeted_actor.is_inside_tree():
        push_error("Source mode-0 camera has no bound target Perso")
        return {}
    var basis := targeted_actor.global_basis
    var anchor := targeted_actor.global_position + basis * mode_zero_actor_offset
    var look := mode_zero_look_offset
    if look == Vector3.ZERO:
        look = Vector3(0, 0, 0.05)
    return {
        "target_position": targeted_actor.global_position,
        "anchor": anchor,
        "candidate": anchor + basis * mode_zero_target_offset,
        "focus": anchor + basis * look,
        "minimum_distance": mode_zero_distance.x,
        "maximum_distance": mode_zero_distance.y,
    }

func record_mode_zero_candidate_collision(blocked: bool) -> int:
    if controller_state == null or not controller_state.has_method("record_candidate_collision"):
        push_error("Source camera controller state is not bound")
        return 0
    return int(controller_state.call("record_candidate_collision", blocked))

func evaluate_mode_zero_teleport_gate(
    candidate_result: bool,
    fallback_active: bool,
    fallback_succeeded: bool,
    has_last_good_position: bool,
    distance_class: int,
    camera_eye: Vector3,
    previous_eye: Vector3,
    maximum_distance: float,
    obstructed: bool,
    owner_teleport_blocked: bool
) -> bool:
    if controller_state == null or not controller_state.has_method("evaluate_mode_zero_teleport_gate"):
        push_error("Source camera controller state is not bound")
        return false
    return bool(controller_state.call(
        "evaluate_mode_zero_teleport_gate",
        candidate_result,
        fallback_active,
        fallback_succeeded,
        has_last_good_position,
        distance_class,
        camera_eye,
        previous_eye,
        maximum_distance,
        obstructed,
        owner_teleport_blocked
    ))

## Supplies the three segments used by 0051f670. The controller's actual
## +0x3c, +0x18, and +0x30 values must come from its pose calculation; they
## are not inferred from the viewport Camera3D child or unsmoothed target.
func source_mode_zero_collision_inputs(
    controller_anchor_3c: Vector3,
    controller_origin_18: Vector3,
    controller_eye_30: Vector3
) -> Dictionary:
    if not controller_anchor_3c.is_finite() or not controller_origin_18.is_finite() or not controller_eye_30.is_finite():
        push_error("Source mode-0 collision endpoints are nonfinite")
        return {}
    var geometry := source_mode_zero_target_geometry()
    if geometry.is_empty():
        return {}
    var context := _source_collision_context()
    if context.is_empty():
        return {}
    var target_position := geometry["target_position"] as Vector3
    var anchor_to_eye = _source_camera_ray_blocked(controller_anchor_3c, controller_eye_30, context)
    var origin_to_eye = _source_camera_ray_blocked(controller_origin_18, controller_eye_30, context)
    var origin_to_target_head = _source_camera_ray_blocked(
        controller_origin_18,
        target_position + Vector3(0, 0, -1.8),
        context
    )
    if anchor_to_eye == null or origin_to_eye == null or origin_to_target_head == null:
        return {}
    var streak := record_mode_zero_candidate_collision(bool(origin_to_eye))
    return {
        "anchor_to_eye_blocked": bool(anchor_to_eye),
        "origin_to_eye_blocked": bool(origin_to_eye),
        "origin_to_target_head_blocked": bool(origin_to_target_head),
        "candidate_collision_streak": streak,
    }

## 0051c7b0 tests its third position argument against the active source sector.
## It publishes that point when it belongs to another sector, otherwise its
## second position argument. The caller must resolve the third-argument sector
## from source boundary data.
func source_publish_mode_zero_translation(
    third_argument_position: Vector3,
    second_argument_position: Vector3,
    third_argument_sector_id: String
) -> bool:
    var owner := get_parent() as Node3D
    var context := _source_collision_context()
    if owner == null or not owner.is_inside_tree() or context.is_empty():
        return false
    if not third_argument_position.is_finite() or not second_argument_position.is_finite() or third_argument_sector_id.is_empty():
        return false
    var active_sector := context["sector"] as Node
    var active_sector_id := str(active_sector.get_meta("source_sector_id", ""))
    var next_sector := _find_source_sector(_source_level_root(), third_argument_sector_id)
    if next_sector == null:
        push_error("Source camera pose references an unbound destination sector")
        return false
    if third_argument_sector_id != active_sector_id:
        owner.global_position = third_argument_position
        _source_active_sector = next_sector
    else:
        owner.global_position = second_argument_position
    return true

func _source_collision_context() -> Dictionary:
    var sector: Node = _source_active_sector
    if not is_instance_valid(sector):
        sector = _source_sector_for(get_parent())
    while is_instance_valid(sector):
        if sector.has_meta("source_sector") and sector.has_meta("source_sector_id"):
            var source_data = JSON.parse_string(str(sector.get_meta("source_sector")))
            if not source_data is Dictionary or not source_data.has("collision"):
                push_error("Source camera sector metadata has no collision-neighbor list")
                return {}
            var reachable := {str(sector.get_meta("source_sector_id")): true}
            for neighbor in source_data["collision"]:
                reachable[str(neighbor)] = true
            return {"sector": sector, "reachable": reachable}
        sector = sector.get_parent()
    push_error("Source camera has no owning source sector metadata")
    return {}

func _source_level_root() -> Node:
    var current: Node = self
    while is_instance_valid(current):
        if current.has_meta("source_level_scene"):
            return current
        current = current.get_parent()
    return null

func _find_source_sector(root: Node, sector_id: String) -> Node:
    if not is_instance_valid(root):
        return null
    if root.has_meta("source_sector_id") and str(root.get_meta("source_sector_id")) == sector_id:
        return root
    for child in root.get_children():
        var found := _find_source_sector(child, sector_id)
        if found != null:
            return found
    return null

func _source_camera_ray_blocked(start: Vector3, finish: Vector3, context: Dictionary) -> Variant:
    var owner_node := get_parent() as CollisionObject3D
    var excluded: Array[RID] = []
    if owner_node != null:
        excluded.append(owner_node.get_rid())
    var target_body := targeted_actor as CollisionObject3D
    if target_body != null and not excluded.has(target_body.get_rid()):
        excluded.append(target_body.get_rid())
    var reachable: Dictionary = context["reachable"]
    var space := get_world_3d().direct_space_state
    for _attempt in range(512):
        var query := PhysicsRayQueryParameters3D.create(start, finish, 3, excluded)
        query.collide_with_bodies = true
        query.collide_with_areas = false
        var hit := space.intersect_ray(query)
        if hit.is_empty():
            return false
        var collider := hit["collider"] as CollisionObject3D
        if collider == null:
            push_error("Source mode-0 ray hit a non-collision object")
            return null
        var shape_owner := collider.shape_find_owner(int(hit["shape"]))
        var shape := collider.shape_owner_get_owner(shape_owner) as Node
        var identifier := int(shape.get_meta("source_collision_identifier", 0)) if shape != null else 0
        var sector := _source_sector_for(collider)
        var sector_id := str(sector.get_meta("source_sector_id", "")) if sector != null else ""
        var reachable_hit := reachable.has(sector_id)
        var reachable_static := not collider is StaticBody3D or bool(collider.get_meta("source_collision_reachable", false))
        if reachable_hit and reachable_static and (identifier & 0x8000) == 0:
            return true
        var rid: RID = hit["rid"]
        if excluded.has(rid):
            push_error("Source mode-0 ray cannot advance past an ignored collision shape")
            return null
        excluded.append(rid)
    push_error("Source mode-0 ray exceeded its ignored-collider limit")
    return null

func _source_sector_for(node: Node) -> Node:
    var current := node
    while is_instance_valid(current):
        if current.has_meta("source_sector_id"):
            return current
        current = current.get_parent()
    return null

func _source_update_mode_zero() -> bool:
    if source_mode_zero_target_geometry().is_empty():
        return false
    if not _reported_mode_zero_gap:
        push_error("Source mode-0 geometry is mapped, but native candidate-eye smoothing, sector-aware ray results, and 0051c7b0 pose arguments are not yet connected")
        _reported_mode_zero_gap = true
    return false
