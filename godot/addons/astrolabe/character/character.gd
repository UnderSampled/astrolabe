extends CharacterBody3D
## Native character controls. Every part and pose already exists in the PackedScene.
## No frame tables, transform pools or object configurations are consumed here.
signal cue_triggered(action: Resource, frame: int, channel: int, configuration_slot: int)
signal action_finished(action: int)
signal action_request_traced(requested: int, restart: bool, previous: int, selected: int, accepted: bool)
signal hud_visibility_requested(indicator: String, visible: bool)
signal generated_object_released(actor: Node3D, attachment: Node3D)
const SOURCE_INVENTORY = preload("res://addons/astrolabe/character/inventory.gd")
const SOURCE_MAGICK = preload("res://addons/astrolabe/character/magick.gd")
const SOURCE_LASER_POINTER = preload("res://addons/astrolabe/character/laser_pointer.gd")
const SOURCE_COLLISION_QUERY_MAX_RESULTS := 4096
@export var initial_state: int = -1
@export var current_state: int = -1
@export var current_mechanics: Resource
@export var physical_environment: Resource
@export var source_inventory: Resource
var source_display_hidden_layers: Dictionary = {}
@export var source_hit_points: Resource
@export var source_magick: Resource
@export var source_laser_pointer: Resource
@export var autoplay: bool = true
## Gameplay uses physics ticks; ordinary editor/pose inspection stays stationary.
@export var motion_enabled: bool = false
## World-space acceleration supplied by the level/game's environment.
@export var gravity: Vector3 = Vector3.ZERO
## Velocity from forces/impulses, separate from the next animation displacement.
var external_velocity: Vector3 = Vector3.ZERO
## AI speed is committed by GoRelative or a jump procedure, then moved by
## CharacterBody3D with the current native animation and contact state.
var source_vector_speed: Vector3 = Vector3.ZERO
var source_impulse: Vector3 = Vector3.ZERO
var source_relative_motion: bool = false
var source_airborne: bool = false
var source_particle_generator_enabled: bool = false
@export var source_routes: Array[Resource] = []
@export var source_route_points: PackedInt32Array = PackedInt32Array()
var source_route_slot: int = -1
var source_route_revision: int = 0
var source_route_active: bool = false
var source_route_holds_motion: bool = false
var source_generated_object_count: int = 0
## A generated actor waits for the native animation's release cue.
var generated_object: Node3D
var source_hud_display: Dictionary = {}
## Last evaluated local displacement, including every native substep and loop.
var root_motion_displacement: Vector3 = Vector3.ZERO
var restored_configuration: bool = false
var suppress_cues: bool = false
var playback_revision: int = 0
var advancing: bool = false
var pending_state: int = -1
var requested_action_state: int = -1
var action_complete: bool = false
var animation_complete: bool = false
var action_controlled: bool = false
var repeats_remaining: int = 0

var animation_player: AnimationPlayer:
    get: return get_node("AnimationPlayer")
var current_clip: Animation:
    get: return animation_player.get_animation(animation_player.assigned_animation) if animation_player.assigned_animation != &"" else null
var sample_rate: float:
    get: return float(current_clip.get_meta("source_fps", 0.0)) if current_clip != null else 0.0
var frame_index: int:
    get: return int(floor(animation_player.current_animation_position * sample_rate + 0.000001)) if current_clip != null else 0
var playing: bool:
    get: return animation_player.is_playing()
    set(value):
        if value: animation_player.play()
        else: animation_player.pause()

func _ready() -> void:
    if source_inventory == null:
        source_inventory = SOURCE_INVENTORY.new()
    if source_magick == null:
        source_magick = SOURCE_MAGICK.new()
    if source_laser_pointer == null:
        source_laser_pointer = SOURCE_LASER_POINTER.new()
    if motion_enabled: _configure_motion()
    # Selection only; the saved initial pose is usable without this callback.
    if autoplay and initial_state >= 0:
        select_state(initial_state)

func _configure_motion() -> void:
    if motion_enabled:
        # The source actor rounds shallow contacts at finite wall edges.
        # Godot's default 15-degree minimum stops Hype at the manoir wall end.
        wall_min_slide_angle = 0.0
    if motion_enabled and physical_environment != null:
        var vertical: Vector3 = physical_environment.vertical
        assert(vertical.length_squared() > 0.0)
        up_direction = vertical.normalized()
        gravity = -vertical * float(physical_environment.gravity)
    elif motion_enabled and gravity == Vector3.ZERO:
        # The first gameplay slice uses Godot's project gravity until the
        # source level's acceleration is measured against a live trajectory.
        gravity = Vector3.DOWN * float(ProjectSettings.get_setting("physics/3d/default_gravity"))
func _process(delta: float) -> void:
    if not motion_enabled: advance(delta)

func _physics_process(delta: float) -> void:
    if not motion_enabled: return
    advance(delta)
    # The animation never changes this body's position. Apply its local motion
    # once, through native contact resolution. Forces/impulses persist independently.
    var motion := global_basis.orthonormalized() * root_motion_displacement / delta
    if source_route_holds_motion: motion = Vector3.ZERO
    if source_route_active:
        motion = _source_route_motion(delta)
    if source_relative_motion:
        external_velocity = _source_relative_vector(source_vector_speed)
        source_relative_motion = false
    # Apply gravity to this contact step. Floor response must not turn that
    # downward acceleration into retained sideways momentum on a slight slope.
    var gravity_ratio := 1.0
    if current_mechanics != null:
        gravity_ratio = float(current_mechanics.ratio)
    external_velocity += gravity * gravity_ratio * delta
    velocity = motion + external_velocity
    move_and_slide()
    for i in get_slide_collision_count():
        var normal := get_slide_collision(i).get_normal()
        if is_on_floor() and normal.dot(up_direction) >= cos(floor_max_angle):
            continue
        if external_velocity.dot(normal) < 0.0:
            external_velocity = external_velocity.slide(normal)
    if is_on_floor():
        if source_airborne:
            # The jump impulse has ended at contact. Root motion supplies any
            # subsequent grounded walk; retaining its airborne horizontal
            # velocity would slide an idle character across a sloped floor.
            external_velocity = Vector3.ZERO
        elif external_velocity.dot(up_direction) < 0.0:
            external_velocity -= up_direction * external_velocity.dot(up_direction)
        source_airborne = false
    elif external_velocity.dot(up_direction) > 0.0: source_airborne = true

func turn(degrees_per_second: float, seconds: float) -> void:
    # Proc_Turn requests a rate about the actor's local up axis. Source +Z maps
    # to Godot +Y; the native transform composes the rotation around that axis.
    assert(is_finite(degrees_per_second) and is_finite(seconds) and seconds >= 0.0)
    rotate_object_local(Vector3.UP, deg_to_rad(degrees_per_second) * seconds)

## Action_IncrementalTurnPerso captures a horizontal target once. Its compiled
## coroutine owns the frame count and cancellation; Godot owns the transform.
## Source actor forward -Y maps to the native body's +Z axis.
func source_turn_angles(target: Node3D) -> Vector2:
    if not is_instance_valid(target): return Vector2(NAN, NAN)
    var forward := global_basis.orthonormalized().z
    var direction := target.global_position - global_position
    var start := fposmod(atan2(forward.x, forward.z), TAU)
    # The original angle helper chooses PI for a zero horizontal direction.
    var finish := PI if direction.x == 0.0 and direction.z == 0.0 else fposmod(atan2(direction.x, direction.z), TAU)
    var difference := finish - start
    if difference > PI: difference -= TAU
    elif difference < -PI: difference += TAU
    return Vector2(start, difference)

func source_turn_to_heading(heading: float) -> void:
    assert(is_finite(heading))
    global_rotation.y = heading

## GoRelative and jump impulses use the actor's orientation when their motion
## is committed. Airborne steering through a jump procedure uses it too.
func _source_relative_vector(value: Vector3) -> Vector3:
    var converted := Vector3(value.x, value.z, -value.y)
    return global_basis.orthonormalized() * converted

func source_set_vector_speed(value: Vector3) -> void:
    assert(value.is_finite())
    source_vector_speed = value

## Deliberately deferred no-op, authorized for the opening slice. Hype assigns
## this actor's world-space drift, then adds it to calculated linear velocity
## (00540782 / 00589bfa). Nonzero wind and resetting prior drift are not ported.
func source_add_wind(value: Vector3) -> void:
    assert(value.is_finite())

func source_reset_route(slot: int) -> bool:
    if slot < 0 or slot >= source_routes.size(): return false
    if source_route_points.size() != source_routes.size(): return false
    source_route_points[slot] = 0
    if source_route_slot == slot:
        source_cancel_route(source_route_revision)
    return true

## GoOverWay retains actor-local progress. Fixed route vertices are native
## Curve3D points; animation supplies speed and CharacterBody3D supplies contact.
## Curved/dynamic/relative routes fail before starting rather than approximating.
func source_begin_route(slot: int) -> int:
    if slot < 0 or slot >= source_routes.size(): return -1
    var route = source_routes[slot]
    if route == null: return 0
    if not route.supported or route.curve == null: return -1
    if route.curve.point_count != route.radii.size() or source_route_points.size() != source_routes.size(): return -1
    if route.curve.point_count == 0: return 0
    if source_route_points[slot] < 0 or source_route_points[slot] >= route.curve.point_count: return -1
    source_route_revision += 1
    source_route_slot = slot
    source_route_active = true
    source_route_holds_motion = false
    motion_enabled = true
    _configure_motion()
    _source_route_target()
    return source_route_revision

func source_route_finished(ticket: int) -> bool:
    return ticket == 0 or ticket != source_route_revision or not source_route_active

func source_cancel_route(ticket: int) -> void:
    if ticket <= 0 or ticket != source_route_revision: return
    source_route_active = false
    source_route_holds_motion = true
    source_route_slot = -1

func _source_route_target() -> Vector3:
    var route = source_routes[source_route_slot]
    # Engine arrival callback 0054a9a0 compares three-dimensional distance <=
    # radius. Zero-radius points additionally accept 0.25, then snap (0054a9f0).
    # Limit the scan to one complete loop, including coincident vertices.
    for _point in route.curve.point_count:
        var index := source_route_points[source_route_slot]
        var target: Vector3 = route.curve.get_point_position(index)
        var radius: float = route.radii[index]
        if global_position.distance_to(target) > (radius if radius > 0.0 else 0.25):
            return target
        if radius == 0.0: global_position = target
        index += 1
        if index == route.curve.point_count:
            source_route_points[source_route_slot] = 0
            if not route.closed:
                source_route_active = false
                return global_position
        else:
            source_route_points[source_route_slot] = index
    return global_position

func _source_route_motion(delta: float) -> Vector3:
    var target := _source_route_target()
    if not source_route_active: return Vector3.ZERO
    var direction := (target - global_position).normalized()
    direction -= up_direction * direction.dot(up_direction)
    # 00589bfa mode 0x14 steers horizontal world velocity, clamps linear/angular
    # rates with the action's mechanics card, and leaves vertical contact to physics.
    # 004436c0 supplies the magnitude, not a signed forward component. Gogoud's
    # walk is source negative Y (native +Z), like source_sighting().
    var speed := root_motion_displacement.length() / delta
    if current_mechanics != null:
        speed = clampf(speed, -absf(current_mechanics.max_linear_speed), absf(current_mechanics.max_linear_speed))
    if not direction.is_zero_approx():
        var facing := atan2(direction.x, direction.z)
        var turn := wrapf(facing - global_rotation.y, -PI, PI)
        if current_mechanics != null:
            var limit: float = absf(current_mechanics.max_angular_speed) * delta
            turn = clampf(turn, -limit, limit)
        global_rotation.y += turn
    return direction * direction.length() * speed

## Object generation retains a local velocity until activation. The animation
## release cue changes the object's transform before mechanics starts
## (0043bcd0/005711f7). Commit it using that transform on the first physics tick.
func source_initialize_motion(value: Vector3) -> void:
    assert(value.is_finite())
    source_vector_speed = value
    source_relative_motion = true
    external_velocity = Vector3.ZERO

func source_go_relative() -> void:
    source_relative_motion = true

func source_kill() -> void:
    var brain := get_node_or_null("Brain")
    if brain != null: brain.call("stop")
    visible = false
    collision_layer = 0
    collision_mask = 0
    set_physics_process(false)
    set_process(false)
    queue_free()

func source_set_impulse(value: Vector3) -> void:
    assert(value.is_finite())
    source_impulse = value

func source_jump_absolute() -> void:
    external_velocity = _source_relative_vector(source_vector_speed + source_impulse)
    source_impulse = Vector3.ZERO
    source_relative_motion = false
    source_airborne = true

func source_jump_without_adding_speed() -> void:
    external_velocity += _source_relative_vector(source_impulse)
    source_impulse = Vector3.ZERO
    source_relative_motion = false
    source_airborne = true

func source_position() -> Vector3:
    return Vector3(global_position.x, -global_position.z, global_position.y)

func source_game_time() -> int:
    var brain := get_node_or_null("Brain")
    assert(brain != null)
    return int(brain.get("clock_milliseconds"))

func source_inventory_weapon() -> int:
    assert(source_inventory != null)
    return int(source_inventory.get("selected_weapon"))

func source_inventory_quantity(item: int) -> int:
    assert(source_inventory != null)
    return int(source_inventory.call("quantity", item))

func source_inventory_remove(item: int, count: int) -> int:
    assert(source_inventory != null)
    return int(source_inventory.call("remove", item, count))

func source_magic_ready() -> bool:
    return source_magick != null and bool(source_magick.call("ready"))

func source_magic_current_spell() -> int:
    assert(source_magic_ready())
    return int(source_magick.get("selected_spell_code"))

func source_magic_points() -> int:
    assert(source_magic_ready())
    return int(source_magick.get("current_points"))

func source_magic_remove_points(amount: int) -> int:
    assert(source_magic_ready() and amount >= 0)
    return source_magick.call("remove_points", amount)

func source_set_laser_pointer_active(active: bool) -> void:
    assert(source_laser_pointer != null)
    source_laser_pointer.set("active", active)

func source_set_laser_pointer_sniper_mode(enabled: bool) -> bool:
    if source_laser_pointer == null: return false
    source_laser_pointer.set("sniper_mode", enabled)
    return true

func source_set_laser_pointer_target(target: Vector3) -> void:
    assert(source_laser_pointer != null and target.is_finite())
    source_laser_pointer.set("target_position", target)
    source_laser_pointer.set("target_valid", true)

func source_set_display_visible(display_visible: bool) -> void:
    for visual in find_children("*", "VisualInstance3D", true, false):
        if not visual is VisualInstance3D or _is_generated_child(visual): continue
        var instance_id := visual.get_instance_id()
        if display_visible:
            if source_display_hidden_layers.has(instance_id):
                visual.layers = int(source_display_hidden_layers[instance_id])
                source_display_hidden_layers.erase(instance_id)
        else:
            if not source_display_hidden_layers.has(instance_id):
                source_display_hidden_layers[instance_id] = visual.layers
            visual.layers = 0

func _is_generated_child(candidate: Node) -> bool:
    var current: Node = candidate
    while current != null and current != self:
        if current.has_meta("astrolabe_generated_template"): return true
        current = current.get_parent()
    return false

func source_laser_pointer_ready() -> bool:
    return source_laser_pointer != null and bool(source_laser_pointer.get("target_valid"))

## Func_GetLaserPointerDirection returns a source-coordinate vector from this
## actor to the shared target, even while pointer presentation is hidden.
func source_laser_pointer_direction() -> Vector3:
    assert(source_laser_pointer_ready())
    var target: Vector3 = source_laser_pointer.get("target_position")
    var delta := target - global_position
    return Vector3(delta.x, -delta.z, delta.y)

## Func_VecteurCos returns the cosine of two source-coordinate vectors.
func source_vector_cos(a: Vector3, b: Vector3) -> float:
    var product := a.length_squared() * b.length_squared()
    if product <= 0.0: return 0.0
    return clampf(a.dot(b) / sqrt(product), -1.0, 1.0)

## Proc_ChangePersoSighting rotates the actor so source forward faces the
## requested world direction. The source proc negates the vector while building
## its negative-Y forward column; the exported Godot basis uses positive Z.
func source_change_sighting(source_direction: Vector3) -> bool:
    if not source_direction.is_finite() or source_direction.length_squared() <= 0.0:
        return false
    var forward := Vector3(source_direction.x, source_direction.z, -source_direction.y).normalized()
    var up := Vector3.UP
    if absf(forward.dot(up)) > 0.999: up = Vector3.FORWARD
    var right := up.cross(forward).normalized()
    var corrected_up := forward.cross(right).normalized()
    var scale := global_basis.get_scale()
    global_basis = Basis(right, corrected_up, forward).scaled(scale)
    return true

func source_is_type_of_gmt_collide(kind: int) -> bool:
    var bit := 1 << (kind & 31)
    for index in get_slide_collision_count():
        var hit := get_slide_collision(index)
        var collider := hit.get_collider() as CollisionObject3D
        if collider == null: continue
        var owner_id := collider.shape_find_owner(hit.get_collider_shape_index())
        var shape := collider.shape_owner_get_owner(owner_id) as CollisionShape3D
        if shape != null and int(shape.get_meta("source_collision_identifier", 0)) & bit != 0:
            return true
    return false

func source_in_top_of_jump() -> bool:
    return source_airborne and not is_on_floor() and external_velocity.dot(up_direction) <= 0.0

func source_set_particle_generator_enabled(enabled: bool) -> void:
    source_particle_generator_enabled = enabled
    for particle in find_children("*", "GPUParticles3D", true, false):
        var emitter := particle as GPUParticles3D
        emitter.emitting = enabled
    for particle in find_children("*", "CPUParticles3D", true, false):
        var emitter := particle as CPUParticles3D
        emitter.emitting = enabled

## The exported rig retains source channel IDs on native Node3D parents.
## Set the parent visibility so animation tracks can still select child parts.
func source_set_channel_active(channel: int, active: bool) -> bool:
    var channels := get_node_or_null("Channels")
    if channels == null: return false
    var found := false
    for candidate in channels.get_children():
        if int(candidate.get_meta("source_channel", -1)) == channel:
            (candidate as Node3D).visible = active
            found = true
    return found

## The authored HUD can bind this signal to its native Control nodes. Keep the
## requested state when the current test scene has no HUD composition yet.
func source_set_hud_indicator_visible(indicator: String, visible: bool) -> void:
    assert(indicator in ["life", "armor", "magic", "selected_object", "weapon"])
    source_hud_display[indicator] = visible
    hud_visibility_requested.emit(indicator, visible)

## Execute an action and instantiate its converted Perso template once. The
## source helper copies the caller's world transform, then passes its vector
## operand into the spawned actor's motion state.
## Return -1 for an unsupported request, 0 for a rejected action, 1 when started.
func _bind_generated_actor_brains(generated_brain: Node, level: Node3D) -> bool:
    var required: PackedStringArray = generated_brain.call("source_external_actor_brains")
    if required.is_empty(): return true
    var available: Dictionary = level.get_meta("source_actor_brains", {})
    var model_path := str(generated_brain.get("source_model_path"))
    for index in required.size():
        var source_path := required[index]
        if not available.has(source_path):
            push_error("Generated Brain " + model_path + " requires missing actor " + source_path)
            return false
        var target_path: NodePath = available[source_path]
        var target := level.get_node_or_null(target_path)
        if not is_instance_valid(target):
            push_error("Generated Brain " + model_path + " requires actor " + source_path + " at " + str(target_path))
            return false
        generated_brain.set("actor_brain_" + str(index), target)
    return true

func source_execute_action_and_generate_object(template_path: String, action: int, vector: Vector3) -> int:
    if template_path.is_empty():
        push_error("Action/object request has no converted Perso template path")
        return -1
    var template := load(template_path) as PackedScene
    if template == null:
        push_error("Action/object request has no converted Perso template")
        return -1
    if not vector.is_finite():
        push_error("Action/object request has an invalid motion vector")
        return -1
    var destination := get_tree().current_scene as Node3D
    if destination == null:
        # Embedded and test-run scenes may be added directly under the root.
        var ancestor: Node = self
        while ancestor.get_parent() != null and ancestor.get_parent() != get_tree().root:
            ancestor = ancestor.get_parent()
        destination = ancestor as Node3D
    if destination == null:
        push_error("Action/object request has no active level scene")
        return -1
    var generated := template.instantiate() as Node3D
    if generated == null:
        push_error("Action/object template is not a Node3D scene")
        return -1
    if vector != Vector3.ZERO and not (generated is CharacterBody3D):
        generated.free()
        push_error("Action/object motion requires a native character")
        return -1
    var generated_brain := generated.get_node_or_null("Brain")
    if generated_brain != null and not _bind_generated_actor_brains(generated_brain, destination):
        generated.free()
        return -1
    var previous := pending_state if pending_state >= 0 else current_state
    var selected := _transition_target(previous, action)
    var clip_name := StringName(("restored/" if restored_configuration else "") + "state_%d" % selected)
    var waits_for_release := animation_player.has_animation(clip_name) and bool(animation_player.get_animation(clip_name).get_meta("releases_generated_object", false))
    if generated is CharacterBody3D:
        generated.set("motion_enabled", true)
        generated.set("physical_environment", physical_environment)
        generated.set("source_inventory", source_inventory)
        generated.set("source_magick", source_magick)
        generated.set("source_laser_pointer", source_laser_pointer)
    # The source helper copies the caller's global SuperObject transform.
    # Initialize it before attachment: the compiled Brain can start in _ready.
    generated.transform = destination.global_transform.affine_inverse() * global_transform
    if generated is CharacterBody3D:
        generated.call("source_initialize_motion", vector)
        # Type 11 mechanics can contact native reaction zones (ZDR), in
        # addition to the static world. Ordinary characters retain their mask.
        var generated_mechanics: Resource = generated.get("current_mechanics")
        if generated_mechanics != null and int(generated_mechanics.mode) == 11:
            generated.collision_mask |= 8
            # The source mechanics query excludes the generating Perso
            # (00464570/00463df0). Its reaction bodies are owned child nodes.
            generated.add_collision_exception_with(self)
            for owner_body in find_children("*", "PhysicsBody3D", true, false):
                generated.add_collision_exception_with(owner_body)
    if generated_brain != null:
        # A generated Perso's compiled Brain reads the source main actor. Bind
        # that ordinary scene reference before _ready schedules its first tick.
        generated_brain.set("main_actor", self)
        if waits_for_release: generated_brain.set("automatic", false)
    destination.add_child(generated)
    if generated_brain != null:
        generated_brain.set("main_actor_path", generated_brain.get_path_to(self))
    generated.set_meta("astrolabe_generated_template", template_path)
    if waits_for_release:
        generated.visible = false
        generated.set_physics_process(false)
        generated.set_process(false)
        generated_object = generated
    if not request_action(action, true):
        if generated_object == generated: generated_object = null
        generated.queue_free()
        return 0
    source_generated_object_count += 1
    return 1

## Func_GetPersoSighting reads the current actor's world-space negative
## source-Y basis column. The exported Basis stores that direction in +Z;
## AI vectors remain in source coordinates until an engine operation uses them.
func source_sighting() -> Vector3:
    var direction := global_basis.z
    return Vector3(direction.x, -direction.z, direction.y)

func _source_sight_point(actor: Node3D) -> Vector3:
    var bounds := actor.get_node_or_null("FamilyBounds")
    if bounds != null:
        var first := bounds.get_node_or_null("Bound_0") as Node3D
        if first != null: return first.global_position
    return actor.global_position

## Cond_SeePerso tests a segment between the actors' first bounding centers.
## Converted static collision geometry supplies the current occluder set.
func source_see_perso(receiver: Node3D, target: Node3D) -> bool:
    assert(is_instance_valid(receiver) and is_instance_valid(target))
    var origin := _source_sight_point(receiver)
    var endpoint := _source_sight_point(target)
    if origin.is_equal_approx(endpoint): return true
    var query := PhysicsRayQueryParameters3D.create(origin, endpoint, 1)
    return get_world_3d().direct_space_state.intersect_ray(query).is_empty()

func source_collide_with_ground() -> bool:
    return is_on_floor()

## The mechanics collision path reports a contact result after each movement
## step. CharacterBody3D publishes the corresponding native result through its
## slide contacts; AI conditions run after _physics_process has moved the body.
func source_collide_with_nothing() -> bool:
    return get_slide_collision_count() == 0

## Resolve a compiled Perso identity in the active level's saved actor graph.
func source_actor_node(actor_asset: String) -> Node3D:
    var level: Node = self
    while level.get_parent() != null and level.get_parent() != get_tree().root:
        level = level.get_parent()
    var actors: Dictionary = level.get_meta("source_actor_nodes", {})
    if not actors.has(actor_asset): return null
    return level.get_node_or_null(actors[actor_asset]) as Node3D

## Source type-11 Perso lists append without deduplication and cannot grow
## beyond their declared capacity. The compiled AI owns the decision to add.
func source_list_add(list: Array[Node3D], capacity: int, actor: Node3D) -> bool:
    if not is_instance_valid(actor) or capacity < 0 or list.size() >= capacity:
        return false
    list.append(actor)
    return true

## Procedure 160 stores the target Perso on the referenced camera actor.
## Follow movement, viewport selection and its other reset fields are separate.
func source_camera_change_targeted_perso(camera_asset: String, target: Node3D) -> bool:
    if not is_instance_valid(target):
        push_error("Camera target is invalid for " + camera_asset)
        return false
    var camera_actor := source_actor_node(camera_asset)
    if camera_actor == null:
        push_error("Camera actor is not bound: " + camera_asset)
        return false
    var camera := camera_actor.get_node_or_null("SourceCamera") if camera_actor != null else null
    if camera == null or not camera.has_method("source_change_targeted_perso"):
        push_error("Camera actor has no native camera: " + camera_asset)
        return false
    return bool(camera.call("source_change_targeted_perso", target))

## Called by the compiled camera Brain. The native camera accepts mode-3 aiming;
## other source motion modes fail explicitly.
func source_camera_update_position() -> bool:
    var camera := get_node_or_null("SourceCamera")
    if camera == null or not camera.has_method("source_update_position"):
        return false
    return bool(camera.call("source_update_position"))

## Procedure 157 selects the referenced camera's supported motion state.
func source_camera_change_state(camera_asset: String, state: int) -> bool:
    var actor := source_actor_node(camera_asset)
    if actor == null:
        return false
    var camera := actor.get_node_or_null("SourceCamera")
    if camera == null or not camera.has_method("source_change_state"):
        return false
    return bool(camera.call("source_change_state", state))

## Procedure 253 on a bound Ultra receiver writes its world translation.
## AI vectors use source coordinates; convert once at this native boundary.
func source_actor_set_absolute_position(actor_asset: String, point: Vector3) -> bool:
    if not point.is_finite():
        return false
    var actor := source_actor_node(actor_asset)
    if actor == null or not actor.is_inside_tree():
        return false
    actor.global_position = Vector3(point.x, point.z, -point.y)
    return true

func source_collide_with_slope() -> bool:
    if not is_on_floor(): return false
    # A steep wall can have a positive vertical component. Use the native
    # walkable floor classification before reporting a slope to source AI.
    var floor_alignment := get_floor_normal().normalized().dot(up_direction.normalized())
    return floor_alignment > 0.0 and floor_alignment < 0.999

func advance(seconds: float) -> void:
    # Godot folds a large advance to one loop. Bound its native playback at
    # method keys and loop edges so cues see their pose and no loop is skipped.
    # These are native Animation keys, not source samples or a second evaluator.
    assert(is_finite(seconds) and seconds >= 0.0)
    root_motion_displacement = Vector3.ZERO
    if seconds > 0.0 and not action_complete: animation_complete = false
    var revision := playback_revision
    while seconds > 0.00000001 and playing and current_clip != null:
        var clip := current_clip
        var position := animation_player.current_animation_position
        var until_loop := clip.length - position
        # A whole-loop advance starting at zero wraps to the same native time;
        # Godot cannot distinguish it from no root motion. Keep a midpoint so
        # native extraction observes both sides of every wrap, even without cues.
        var step := minf(seconds, minf(until_loop, clip.length * 0.5))
        for track in clip.get_track_count():
            if clip.track_get_type(track) != Animation.TYPE_METHOD: continue
            for key in clip.track_get_key_count(track):
                var time := clip.track_get_key_time(track, key)
                if time > position + 0.00000001:
                    step = minf(step, time - position)
                    break
        # Parsed decimal lengths and accumulated deltas can differ by one ULP.
        # Cross the native boundary, then snap to zero without replaying methods;
        # otherwise an exactly consumed tick can strand playback just before it.
        var boundary := absf(step - until_loop) <= 0.00000001
        if boundary and action_controlled:
            # Decide the action continuation before wrapping its native clip.
            # Inspection clips stay looped; gameplay can finish or follow `next`.
            advance_native(maxf(0.0, until_loop - 0.00000001))
            if playback_revision != revision: return
            _finish_action_cycle()
            revision = playback_revision
            seconds = maxf(0.0, seconds - until_loop)
            continue
        advance_native(until_loop + 0.00000001 if boundary else step)
        if playback_revision != revision: return
        if boundary and clip.loop_mode == Animation.LOOP_LINEAR:
            animation_player.seek(0.0, true, true)
        seconds = maxf(0.0, seconds - (until_loop if boundary else step))

func state_names() -> PackedStringArray:
    return animation_player.get_animation_library(&"").get_animation_list()

func select_state(id: int) -> bool:
    if not animation_player.has_animation(StringName("state_%d" % id)): return false
    action_controlled = false
    requested_action_state = id
    source_route_holds_motion = false
    action_complete = false
    animation_complete = false
    return _select_state(id)

func _select_state(id: int) -> bool:
    var clip := StringName(("restored/" if restored_configuration else "") + "state_%d" % id)
    if not animation_player.has_animation(clip): return false
    playback_revision += 1
    if advancing:
        pending_state = id
        return true
    animation_player.stop(true)
    animation_player.play(clip)
    advance_native(0.0)
    _state_movement_collision(current_state)
    if sample_rate <= 0.0: animation_player.pause()
    return true

## Proc_ChangeAction preserves an already requested action; ExecuteAction restarts.
## A rejected transition leaves the current action and its completion unchanged.
func is_in_action(id: int) -> bool:
    # 00435750 queries the requested state, including a transition into it.
    return (requested_action_state if action_controlled else current_state) == id

func request_action(id: int, restart: bool = false) -> bool:
    var previous := pending_state if pending_state >= 0 else current_state
    if not animation_player.has_animation(StringName("state_%d" % id)):
        action_request_traced.emit(id, restart, previous, -1, false)
        return false
    if not restart and action_controlled and requested_action_state == id:
        action_request_traced.emit(id, restart, previous, previous, true)
        return true
    var selected := _transition_target(previous, id)
    if selected < 0:
        action_request_traced.emit(id, restart, previous, selected, false)
        return false
    requested_action_state = id
    animation_complete = false
    source_route_holds_motion = false
    _play_action_state(selected)
    action_request_traced.emit(id, restart, previous, selected, true)
    return true

func _play_action_state(id: int) -> void:
    action_controlled = true
    action_complete = false
    repeats_remaining = _state_repeats(id)
    _select_state(id)

func _finish_action_cycle() -> void:
    animation_complete = true
    if repeats_remaining > 0:
        repeats_remaining -= 1
        _select_state(current_state)
        return
    var next := _state_next(current_state)
    if next >= 0:
        _play_action_state(next)
        return
    action_complete = true
    animation_player.pause()
    action_finished.emit(requested_action_state)

func _transition_target(_from: int, target: int) -> int:
    return target

func _state_next(_id: int) -> int:
    return -1

func _state_repeats(_id: int) -> int:
    return 0

func _state_movement_collision(_state: int) -> void:
    # Activate saved query/reaction shapes selected by source character state.
    for value in find_children("*", "CollisionShape3D", true, false):
        var shape := value as CollisionShape3D
        if not shape.has_meta("source_active_states"):
            continue
        var states: PackedInt32Array = shape.get_meta("source_active_states")
        _set_zde_shape_active(shape, states.has(_state))

func _set_zde_shape_active(shape: CollisionShape3D, active: bool) -> void:
    shape.disabled = not active or (int(shape.get_meta("source_collision_identifier", 0)) & 0x8000) != 0

## Condition 28 intersects one of this Perso's active ZDE zones with candidate
## character ZDM collision shapes. Hype filters candidate collision materials
## by identifier mask 1 before counting a result.
func source_collide_perso_zde_no_with_type_zde(zone_number: int, maximum: int) -> bool:
    if zone_number <= 0 or maximum <= 0:
        return false
    var query_area := get_node_or_null("ZDE") as Area3D
    if query_area == null:
        return false
    var space := get_world_3d().direct_space_state
    var matches: Dictionary = {}
    for value in query_area.find_children("*", "CollisionShape3D", true, false):
        var query_shape := value as CollisionShape3D
        if int(query_shape.get_meta("source_zone", -1)) != zone_number - 1:
            continue
        if query_shape.disabled or query_shape.shape == null:
            continue
        var parameters := PhysicsShapeQueryParameters3D.new()
        parameters.shape = query_shape.shape
        parameters.transform = query_shape.global_transform
        parameters.collision_mask = 2
        parameters.collide_with_areas = false
        parameters.collide_with_bodies = true
        parameters.margin = 0.0
        parameters.exclude = [get_rid()]
        for hit in space.intersect_shape(parameters, SOURCE_COLLISION_QUERY_MAX_RESULTS):
            var other_actor := hit.get("collider") as CharacterBody3D
            if other_actor == null or other_actor == self:
                continue
            var shape_index := int(hit.get("shape", -1))
            if shape_index < 0:
                continue
            var owner_id := other_actor.shape_find_owner(shape_index)
            if owner_id < 0:
                continue
            var other_shape := other_actor.shape_owner_get_owner(owner_id) as CollisionShape3D
            if other_shape == null or not other_shape.has_meta("source_collision_identifier"):
                continue
            var identifier := int(other_shape.get_meta("source_collision_identifier"))
            if (identifier & 1) == 0:
                continue
            matches[other_actor.get_instance_id()] = true
            if matches.size() >= maximum:
                return true
    return false

func _set_movement_shape_active(shape: CollisionShape3D, active: bool) -> void:
    shape.disabled = not active or (int(shape.get_meta("source_collision_identifier", 0)) & 0x8000) != 0

func advance_native(seconds: float) -> void:
    advancing = true
    animation_player.advance(seconds)
    if seconds > 0.0:
        root_motion_displacement += animation_player.get_root_motion_position()
    advancing = false
    if pending_state >= 0:
        var state := pending_state
        pending_state = -1
        _select_state(state)

func seek_frame(index: int) -> void:
    if current_clip == null: return
    assert(index >= 0 and index < int(current_clip.get_meta("source_frames", 1)))
    animation_player.seek(float(index) / maxf(sample_rate, 1.0), true, true)

func restore_configuration() -> bool:
    var clip := StringName("restored/state_%d" % current_state)
    if not animation_player.has_animation(clip): return false
    var time := animation_player.current_animation_position
    var was_playing := playing
    restored_configuration = true
    suppress_cues = true
    animation_player.play(clip)
    animation_player.seek(time, true, true)
    if not was_playing: animation_player.pause()
    suppress_cues = false
    return true

func set_bounds_visible(value: bool) -> void:
    var bounds := get_node_or_null("FamilyBounds") as Node3D
    if bounds != null: bounds.visible = value

func _cue(action: Resource, frame: int, channel: int, slot: int, channel_path: String = "") -> void:
    if action.sound != null:
        action.sound.execute(get_node("SoundEmitter"))
    if action.kind == &"attachment" and is_instance_valid(generated_object):
        var attachment := get_node_or_null(NodePath(channel_path)) as Node3D
        assert(attachment != null, "Object release requires a converted animation channel")
        # Source event kind 2 copies the triggering channel's world matrix,
        # releases the held object, then activates its mechanics/AI (0043bcd0).
        var generated := generated_object
        generated_object = null
        generated.global_transform = attachment.global_transform
        generated_object_released.emit(generated, attachment)
        generated.visible = true
        generated.set_physics_process(true)
        generated.set_process(true)
        var generated_brain := generated.get_node_or_null("Brain")
        if generated_brain != null:
            generated_brain.set("automatic", true)
            generated_brain.call("start")
    cue_triggered.emit(action, frame, channel, slot)

func _cues(frame: int, channels: PackedInt32Array, slots: PackedInt32Array, actions: Array, channel_paths: PackedStringArray = PackedStringArray()) -> void:
    if suppress_cues: return
    var revision := playback_revision
    for i in actions.size():
        _cue(actions[i], frame, channels[i], slots[i], channel_paths[i] if i < channel_paths.size() else "")
        if revision != playback_revision: return
