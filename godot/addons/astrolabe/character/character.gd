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
const SOURCE_RENDERING = preload("res://addons/astrolabe/character/rendering.gd")
const CONFIGURED_BANK = preload("res://addons/astrolabe/character/configured_bank.gd")
const SOURCE_LIFT_MOTION = preload("res://addons/astrolabe/character/lift_motion.gd")
const SOURCE_CHANNEL_ATTACHMENT = preload("res://addons/astrolabe/character/channel_attachment.gd")
const SOURCE_CHANNEL_VISIBILITY = preload("res://addons/astrolabe/character/channel_visibility.gd")
const CONTACT_MATERIAL = preload("res://addons/astrolabe/character/explosive_barrel.gd")
const SOURCE_COLLISION_QUERY_MAX_RESULTS := 4096
const CAMERA_POST_CONTACT_POSITION := &"post_contact_position"
const CAMERA_RETAINED_POSITION_HISTORY := &"retained_position_history"
const CAMERA_FRAME_POSITION := &"camera_frame_position"
const CAMERA_POST_CONTACT_VELOCITY := &"post_contact_velocity"
const CAMERA_FRAME_DISPLACEMENT := &"camera_frame_displacement"
const CAMERA_RETAINED_VELOCITY_HISTORY := &"retained_velocity_history"
@export var initial_state: int = -1
@export var current_state: int = -1
@export var current_mechanics: Resource
## Converted state explicitly has no identity card: retain history without sampling.
@export var source_camera_retains_position_history: bool = false
## Effective source dynamics binding, retained by import/export independently
## of the selected mechanics card. Compiled point-motion APIs check it.
@export var source_motion_binding_present: bool = false
@export var physical_environment: Resource
@export var source_inventory: Resource
@export_node_path("Node3D") var source_state_owner_path: NodePath
var source_rendering := SOURCE_RENDERING.new()
var source_lift_motion := SOURCE_LIFT_MOTION.new()
var source_channel_attachment := SOURCE_CHANNEL_ATTACHMENT.new()
var source_channel_visibility := SOURCE_CHANNEL_VISIBILITY.new()
var source_opacity: int:
    get: return source_rendering.opacity
    set(value): source_rendering.opacity = value
var source_transparent_display: bool:
    get: return source_rendering.transparent_display
    set(value): source_rendering.transparent_display = value
@export var source_hit_points: Resource
## Shared by actors in the active scene; armor is not actor-local state.
@export var source_armor: Resource
## One mutable permutation per canonical ObjectList, shared within the level.
@export var source_configuration: Resource
## Saved selections available to this actor, each shared by canonical identity.
@export var source_configuration_choices: Array[Resource] = []
@export var source_initial_configuration: Resource
@export var source_magick: Resource
@export var source_laser_pointer: Resource
@export var autoplay: bool = true
## Gameplay uses physics ticks; ordinary editor/pose inspection stays stationary.
@export var motion_enabled: bool = false
@export_node_path("Marker3D") var source_picking_center_path: NodePath
@export_node_path("Node3D") var source_contact_sector_path: NodePath
@export var source_sector_membership_known: bool = false
var source_contact_eligible: bool:
    get: return source_object_active and not source_return_suppressed
## StandardGame custom flags belong to this actor; draw/activation flags are separate.
@export var source_custom_bits: int = 0
var _contact_sector: Node3D
var _contact_previous: Variant = null
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
var _source_camera_data3d_positions := PackedVector3Array()
var _source_camera_data3d_velocities := PackedVector3Array()
var _source_camera_data3d_weights := PackedFloat32Array()
var _source_camera_data3d_active_position_source: StringName
var _source_camera_data3d_sample_count := 0
var source_particle_generator_enabled: bool = false
@export var source_routes: Array[Resource] = []
## Progress belongs to each actor binding, even when the route is shared.
## Previous vertex/sample -1 preserves the source's unselected sentinel.
@export var source_route_points: PackedInt32Array = PackedInt32Array()
@export var source_route_samples: PackedInt32Array = PackedInt32Array()
@export var source_route_previous_points: PackedInt32Array = PackedInt32Array()
@export var source_route_previous_samples: PackedInt32Array = PackedInt32Array()
var source_route_slot: int = -1
var source_route_revision: int = 0
var source_route_active: bool = false
var source_route_holds_motion: bool = false
## One native movement request; compiled scripts own waiting and cancellation.
var source_point_revision: int = 0
var source_point_active: bool = false
var source_point_holds_motion: bool = false
var _source_point_target: Vector3 = Vector3.ZERO
var _source_point_actor: Node3D
var _source_point_pursuit: bool = false
var _source_point_radius: float = 0.0
var _source_point_horizontal: bool = false
var _source_point_route_slot: int = -1
var _source_point_mass_motion: bool = false
var _source_point_failure: String = ""
var _source_next_waypoint_slot: int = -1
var _source_next_waypoint_route: Resource
var source_generated_object_count: int = 0
## A generated actor waits for the native animation's release cue.
var generated_object: Node3D
var source_hud_display: Dictionary = {}
## Last evaluated local displacement, including every native substep and loop.
var root_motion_displacement: Vector3 = Vector3.ZERO
## Root rotation this tick: 004436c0 hands the frames' composed motion rotation
## to the mechanics as an angular speed. An AI turn this tick writes the same
## speed and takes precedence: Hype's standing turn measures the AI's 160 deg/s
## in the original, without its clip's 30 degrees per loop.
var root_motion_turn: Quaternion = Quaternion.IDENTITY
var source_turned_this_tick := false
## Native activation gates rendering, decision ticks and actor contact eligibility.
@export var source_object_active: bool = true
@export var source_activation_known: bool = false
## The Perso whose generation call created this object. Its contact queries
## never select that Perso (stand-in: see docs/astrolabe/stand-ins.md).
var source_generator: Node3D
@export var source_return_suppressed: bool = false
@export var source_kill_resets_components: bool = false
## Set when a kill deactivated this actor without its source component reset.
var source_kill_reset_pending := false
var restored_configuration: bool = false
var suppress_cues: bool = false
var playback_revision: int = 0
var advancing: bool = false
var pending_state: int = -1
var requested_action_state: int = -1
## Numeric requested action and native completion target are distinct. Source
## automatic next-state selection realigns its target without rewriting the id.
var completion_target_state: int = -1
var action_complete: bool = false
var animation_complete: bool = false
var action_controlled: bool = false
var repeats_remaining: int = 0
var _configuration_bound: bool = false
var source_configuration_error: String = ""

var animation_player: AnimationPlayer:
    get: return get_node_or_null("AnimationPlayer")
var current_clip: Animation:
    get: return animation_player.get_animation(animation_player.assigned_animation) if animation_player.assigned_animation != &"" else null
var sample_rate: float:
    get: return float(current_clip.get_meta("source_fps", 0.0)) if current_clip != null else 0.0
var frame_index: int:
    get: return int(floor(animation_player.current_animation_position * sample_rate + 0.000001)) if current_clip != null else 0
var playing: bool:
    # Camera-only actors have no AnimationPlayer; they never play.
    get: return animation_player != null and animation_player.is_playing()
    set(value):
        if animation_player == null: return
        if value: animation_player.play()
        else: animation_player.pause()

## Resolve ordinary scene-owned state before child Brains enter _ready. Nested
## PackedScene instances otherwise duplicate local-to-scene resource overrides.
func _enter_tree() -> void:
    # Re-entered actors catch up with shared configuration changes made while
    # outside the scene. Removed actors must not mutate their payload parents.
    if _configuration_bound:
        if source_configuration != null and not source_configuration.changed.is_connected(_apply_source_configuration):
            source_configuration.changed.connect(_apply_source_configuration)
        _apply_source_configuration.call_deferred()
    var stock: Resource = (get_meta("source_stock") if has_meta("source_stock") else null) as Resource
    if stock != null: set_meta("source_stock", stock.duplicate(true))
    if source_state_owner_path.is_empty():
        return
    var state_owner := get_node(source_state_owner_path)
    source_inventory = state_owner.get_meta("source_inventory") as Resource
    source_magick = state_owner.get_meta("source_magick") as Resource

func _exit_tree() -> void:
    if source_configuration != null and source_configuration.changed.is_connected(_apply_source_configuration):
        source_configuration.changed.disconnect(_apply_source_configuration)

func _ready() -> void:
    _contact_previous = global_position
    if source_shadow_quality != 0:
        DROP_SHADOW.attach(self, source_shadow_quality, source_shadow_scale)
    _source_reset_camera_data3d_history()
    if source_inventory == null:
        source_inventory = SOURCE_INVENTORY.new()
    if source_magick == null:
        source_magick = SOURCE_MAGICK.new()
    if source_laser_pointer == null:
        source_laser_pointer = SOURCE_LASER_POINTER.new()
    _bind_source_configuration()
    # The engine allocates dynamics at load, so the file's pointer can be empty
    # for an actor that moves. Without this Hype ignores scripted turns and
    # Gogoud's scripted walks finish at once.
    if motion_enabled or current_mechanics != null:
        source_motion_binding_present = true
    if motion_enabled:
        _configure_motion()
        _collide_with_actor_zones()
        # Walking down stairs must keep ground contact, or the fall reflex fires mid-step.
        floor_snap_length = SOURCE_STEP_DOWN
    # Selection only; the saved initial pose is usable without this callback.
    if autoplay and initial_state >= 0:
        select_state(initial_state)
    source_set_object_active_state(source_object_active and not source_return_suppressed)

func _configure_motion() -> void:
    if motion_enabled:
        # The source actor rounds shallow contacts at finite wall edges.
        # Godot's default 15-degree minimum stops Hype at the manoir wall end.
        wall_min_slide_angle = 0.0
        # Contacts count as ground above the card's slope limit, including the
        # rounded normal of an edge: Hype stands on corners at nz 0.52 to 0.58.
        if current_mechanics != null and float(current_mechanics.slope_limit) > 0.0:
            floor_max_angle = acos(float(current_mechanics.slope_limit))
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
    if not motion_enabled:
        advance(delta)
        source_camera_data3d_step(delta)

func _physics_process(delta: float) -> void:
    if not PROFILED:
        _physics_step(delta)
        return
    var started := Time.get_ticks_usec()
    _physics_step(delta)
    profile_add(self, "motion", Time.get_ticks_usec() - started)

func _physics_step(delta: float) -> void:
    _update_sector_ambient()
    if source_lift_motion.active:
        source_lift_motion.step(delta)
        global_position = source_lift_motion.position
        # The original lift callback writes absolute translation directly.
        # It also runs for stationary actors with no animation root movement.
        if motion_enabled: advance(delta)
        source_camera_data3d_step(delta)
        return
    if not motion_enabled: return
    advance(delta)
    # The animation never changes this body's position. Apply its local motion
    # once, through native contact resolution. Forces/impulses persist independently.
    if not source_turned_this_tick and not root_motion_turn.is_equal_approx(Quaternion.IDENTITY):
        basis = basis * Basis(root_motion_turn)
    source_turned_this_tick = false
    var motion := global_basis.orthonormalized() * root_motion_displacement / delta
    if source_point_active or source_point_holds_motion:
        motion = _source_point_motion(delta)
        if _source_point_mass_motion:
            # Mass9 event14 supplies full world velocity. Its proved no-hit
            # path has neither animation displacement nor Character gravity.
            external_velocity = Vector3.ZERO
            velocity = motion
            move_and_slide()
            source_contact_sector()
            source_camera_data3d_step(delta)
            return
    elif source_route_active:
        motion = _source_route_motion(delta)
    elif source_route_holds_motion:
        motion = Vector3.ZERO
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
    # 00448280 follows the original walk/mass translation write. Maintain
    # an existing sector outside AI queries; a saved null stays null here.
    source_contact_sector()
    source_camera_data3d_step(delta)

## 00443f00 dispatches this sample through DAT_00757320 by the active DNM
## identity-card type. 00443cf0/00443d50/00443db0 shift motion, position and
## seconds into the same eight newest-first samples. Walk and Mass9 publish
## post-contact velocity in units/sec; Type3 publishes frame displacement.
## Preserve that distinction instead of deriving motion from position samples.
func source_camera_data3d_step(delta: float) -> bool:
    if not is_finite(delta) or delta < 0.0:
        return false
    if source_camera_retains_position_history and current_mechanics == null:
        return true
    var position_source := _source_camera_data3d_position_source()
    if position_source.is_empty():
        _source_reset_camera_data3d_history()
        return false
    var motion: Variant = _source_camera_data3d_motion()
    if motion == null:
        return false
    if position_source != _source_camera_data3d_active_position_source:
        _source_reset_camera_data3d_history()
        _source_camera_data3d_active_position_source = position_source
    if _source_camera_data3d_positions.size() != 8 or _source_camera_data3d_velocities.size() != 8 or _source_camera_data3d_weights.size() != 8:
        _source_reset_camera_data3d_history()
        _source_camera_data3d_active_position_source = position_source
    for index in range(7, 0, -1):
        _source_camera_data3d_positions[index] = _source_camera_data3d_positions[index - 1]
        _source_camera_data3d_velocities[index] = _source_camera_data3d_velocities[index - 1]
        _source_camera_data3d_weights[index] = _source_camera_data3d_weights[index - 1]
    _source_camera_data3d_positions[0] = global_position
    _source_camera_data3d_velocities[0] = motion
    _source_camera_data3d_weights[0] = delta
    _source_camera_data3d_sample_count = mini(_source_camera_data3d_sample_count + 1, 8)
    return true

## Native state animation selects a converted mechanics Resource. Its named
## position publisher is resolved during export; unknown bindings fail closed.
func source_camera_data3d_inputs(initialize_empty: bool = false) -> Dictionary:
    if source_camera_retains_position_history and current_mechanics == null:
        if initialize_empty: _source_initialize_empty_camera_history()
        return {
            "supported": true,
            "position_source": CAMERA_RETAINED_POSITION_HISTORY,
            "velocity_source": CAMERA_RETAINED_VELOCITY_HISTORY,
            "motion_binding_present": source_motion_binding_present,
            "sample_count": _source_camera_data3d_sample_count,
            "points": _source_camera_data3d_positions.duplicate(),
            "velocities": _source_camera_data3d_velocities.duplicate(),
            "weights": _source_camera_data3d_weights.duplicate(),
        }
    var position_source := _source_camera_data3d_position_source()
    if position_source.is_empty():
        return {"supported": false, "position_source": &"", "reason": "active mechanics identity-card callback is not mapped"}
    if position_source != _source_camera_data3d_active_position_source:
        _source_reset_camera_data3d_history()
        _source_camera_data3d_active_position_source = position_source
    if initialize_empty: _source_initialize_empty_camera_history()
    return {
        "supported": true,
        "position_source": position_source,
        "velocity_source": _source_camera_data3d_velocity_source(),
        "motion_binding_present": source_motion_binding_present,
        "sample_count": _source_camera_data3d_sample_count,
        "points": _source_camera_data3d_positions.duplicate(),
        "velocities": _source_camera_data3d_velocities.duplicate(),
        "weights": _source_camera_data3d_weights.duplicate(),
    }

func _source_camera_data3d_position_source() -> StringName:
    if current_mechanics == null:
        return &""
    var position_source: Variant = current_mechanics.get("camera_position_source")
    return position_source if position_source in [CAMERA_POST_CONTACT_POSITION, CAMERA_FRAME_POSITION] else &""

func _source_camera_data3d_velocity_source() -> StringName:
    if current_mechanics == null: return &""
    var motion_source: Variant = current_mechanics.get("camera_velocity_source")
    return motion_source if motion_source in [CAMERA_POST_CONTACT_VELOCITY, CAMERA_FRAME_DISPLACEMENT] else &""

func _source_camera_data3d_motion() -> Variant:
    match _source_camera_data3d_velocity_source():
        CAMERA_POST_CONTACT_VELOCITY:
            # move_and_slide updates this native velocity during contact.
            return velocity if velocity.is_finite() else null
        CAMERA_FRAME_DISPLACEMENT:
            var camera := get_node_or_null("SourceCamera")
            if camera != null and camera.has_method("source_camera_frame_displacement"):
                return camera.call("source_camera_frame_displacement")
    return null

func _source_reset_camera_data3d_history() -> void:
    _source_camera_data3d_positions.resize(8)
    _source_camera_data3d_positions.fill(Vector3.ZERO)
    _source_camera_data3d_velocities.resize(8)
    _source_camera_data3d_velocities.fill(Vector3.ZERO)
    _source_camera_data3d_weights.resize(8)
    _source_camera_data3d_weights.fill(0.0)
    _source_camera_data3d_active_position_source = &""
    _source_camera_data3d_sample_count = 0

## 00443bf0 initializes the target-owned history at the first weighted query.
## Raw history inspection remains separate; zero weights are valid at creation.
func _source_initialize_empty_camera_history() -> void:
    var total := 0.0
    for weight in _source_camera_data3d_weights:
        total = PackedFloat32Array([total + weight])[0]
    if total == 0.0 or not is_finite(total):
        _source_camera_data3d_positions.fill(global_position)
        _source_camera_data3d_velocities.fill(Vector3.ZERO)
        _source_camera_data3d_weights.fill(0.03)

func turn(degrees_per_second: float, seconds: float) -> void:
    # Proc_Turn requests a rate about the actor's local up axis. Source +Z maps
    # to Godot +Y; the native transform composes the rotation around that axis.
    assert(is_finite(degrees_per_second) and is_finite(seconds) and seconds >= 0.0)
    rotate_object_local(Vector3.UP, deg_to_rad(degrees_per_second) * seconds)
    source_turned_this_tick = true

## Action_IncrementalTurnPerso captures a horizontal target once. Its compiled
## coroutine owns the frame count and cancellation; Godot owns the transform.
## Source actor forward -Y maps to the native body's +Z axis.
func source_turn_angles(target: Node3D) -> Vector2:
    if not is_instance_valid(target): return Vector2(NAN, NAN)
    return _source_turn_angles_to_world_position(target.global_position)

## Action_IncrementalTurnPosition snapshots a Position vector and uses the same
## heading/shortest-delta helpers as the actor-targeted turn. The compiled
## script owns its signed frame count and successive heading requests.
func source_turn_angles_to_position(point: Vector3) -> Vector2:
    if not point.is_finite(): return Vector2(NAN, NAN)
    return _source_turn_angles_to_world_position(Vector3(point.x, point.z, -point.y))

func _source_turn_angles_to_world_position(point: Vector3) -> Vector2:
    # The source heading reads the transformed forward column directly.
    # Orthonormalizing a composed/scaled basis can change that direction.
    var forward := global_basis.z
    var direction := point - global_position
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
    source_turned_this_tick = true

## Proc217 writes an absolute heading about source +Z to its mechanics request.
## Reuse the native facing transform; no engine event queue is reconstructed.
func source_turn_toward(target: Node3D) -> bool:
    if not source_motion_binding_present or not is_instance_valid(target) or not target.is_inside_tree():
        return false
    var angles := source_turn_angles(target)
    if not angles.is_finite(): return false
    source_turn_to_heading(PackedFloat32Array([angles.x + angles.y])[0])
    return true

## GoRelative and jump impulses use the actor's orientation when their motion
## is committed. Airborne steering through a jump procedure uses it too.
func _source_relative_vector(value: Vector3) -> Vector3:
    var converted := Vector3(value.x, value.z, -value.y)
    return global_basis.orthonormalized() * converted

func source_set_vector_speed(value: Vector3) -> void:
    assert(value.is_finite())
    source_vector_speed = value
    # Proc29 clears existing caller motion before the next control request.
    # The configured vector retains both direction and positive speed length.
    external_velocity = Vector3.ZERO
    velocity = Vector3.ZERO

## Explicitly deferred: the typed source call and operand remain visible, but
## the requested opening-slice target behavior is a no-op pending wind parity.
func source_add_wind(value: Vector3) -> void:
    assert(value.is_finite())

func source_change_armor(index: int) -> void:
    assert(source_armor != null, "Fight_ChangeArmor needs the converted shared armor catalog")
    source_armor.call("select", index)

func source_set_armor_resistance(rank: int) -> void:
    assert(source_armor != null, "Fight_SetArmorResistance needs the converted shared armor catalog")
    source_armor.call("set_resistance", rank)

func source_repair_armor() -> void:
    assert(source_armor != null, "Fight_RepairArmor needs the converted shared armor catalog")
    source_armor.call("repair")

func source_armor_resistance() -> int:
    assert(source_armor != null, "Fight_GetArmorResistance needs the converted shared armor catalog")
    return int(source_armor.call("resistance"))

func source_armor_count() -> int:
    assert(source_armor != null, "Fight_GetArmorCount needs the converted shared armor catalog")
    return (source_armor.get("definitions") as Array).size()

## Authored callers can inspect source_inventory.last_failure after this call;
## compiled source AI uses the shared service and propagates failures directly.
func source_inventory_add_object(target: Node) -> int:
    assert(source_inventory != null, "AddObject needs the converted shared inventory")
    return int(source_inventory.call("add_actor_object", target))

func source_reset_route(slot: int) -> bool:
    if slot < 0 or slot >= source_routes.size(): return false
    if not _source_route_progress_present(): return false
    _source_reset_route_progress(slot)
    if source_route_slot == slot:
        source_cancel_route(source_route_revision)
    if _source_point_route_slot == slot:
        source_cancel_point_motion(source_point_revision)
    return true

func _source_route_progress_present() -> bool:
    var count := source_routes.size()
    return source_route_points.size() == count and source_route_samples.size() == count and source_route_previous_points.size() == count and source_route_previous_samples.size() == count

func _source_reset_route_progress(slot: int) -> void:
    source_route_points[slot] = 0
    source_route_samples[slot] = 0
    source_route_previous_points[slot] = -1
    source_route_previous_samples[slot] = -1

## Meta32 captures an absolute source Position. Meta33 instead follows current
## height+0.05 on each update, retaining a three-dimensional arrival sphere.
func source_begin_point_motion(point: Vector3, radius: float, horizontal: bool = false) -> int:
    _source_new_point_request()
    if not point.is_finite() or not is_finite(radius):
        return _source_fail_point_request("point motion requires finite position and radius")
    _source_point_target = Vector3(point.x,point.z,-point.y)
    _source_point_radius = PackedFloat32Array([radius])[0]
    if not is_finite(_source_point_radius):
        return _source_fail_point_request("point motion radius exceeds source real range")
    _source_point_horizontal = horizontal
    return _source_start_point_request()

## Meta2 captures the actor and radius once. The target origin is sampled anew
## by each native contact step, sharing the source-backed point arrival API.
func source_begin_actor_pursuit(actor: Node3D, radius: float) -> int:
    _source_new_point_request()
    if not is_instance_valid(actor) or not actor.is_inside_tree() or not is_finite(radius):
        return _source_fail_point_request("pursuit requires a native actor and finite radius")
    _source_point_actor = actor
    _source_point_pursuit = true
    _source_point_radius = PackedFloat32Array([radius])[0]
    if not is_finite(_source_point_radius):
        return _source_fail_point_request("pursuit radius exceeds source real range")
    return _source_start_point_request()

## Meta30 selects a single next waypoint. A failed slot lookup retains the
## preceding captured route, matching its initializer's failed replacement.
func source_begin_next_waypoint(slot: int) -> int:
    _source_new_point_request()
    slot &= 255
    if slot < source_routes.size() and source_routes[slot] != null:
        var candidate = source_routes[slot]
        if candidate.curve == null:
            return _source_fail_point_request("selected route has no native curve")
        if candidate.curve.point_count > 0:
            _source_next_waypoint_slot = slot
            _source_next_waypoint_route = candidate
    if _source_next_waypoint_route == null: return 0
    _source_point_route_slot = _source_next_waypoint_slot
    return _source_start_point_request()

func _source_new_point_request() -> void:
    source_point_revision += 1
    source_point_active = false
    source_point_holds_motion = false
    _source_point_route_slot = -1
    _source_point_horizontal = false
    _source_point_actor = null
    _source_point_pursuit = false
    _source_point_failure = ""

func _source_start_point_request() -> int:
    source_point_active = true
    source_point_holds_motion = true
    source_route_active = false
    source_route_holds_motion = false
    _source_point_mass_motion = current_mechanics != null and int(current_mechanics.mode) == 9
    _source_point_control()
    if not _source_point_failure.is_empty(): return -1
    if not source_point_active: return 0
    source_relative_motion = false
    external_velocity = Vector3.ZERO
    motion_enabled = true
    _configure_motion()
    return source_point_revision

func source_point_motion_finished(ticket: int) -> bool:
    return ticket <= 0 or ticket != source_point_revision or not source_point_active

func source_point_motion_failure(ticket: int) -> String:
    return _source_point_failure if ticket == source_point_revision or ticket == -1 else ""

func source_cancel_point_motion(ticket: int) -> void:
    if ticket <= 0 or ticket != source_point_revision: return
    source_point_active = false
    source_point_holds_motion = true
    _source_point_failure = ""

func _source_fail_point_request(reason: String) -> int:
    source_point_active = false
    source_point_holds_motion = true
    _source_point_failure = reason
    return -1

func _source_select_waypoint_target() -> bool:
    var slot := _source_point_route_slot
    if slot >= source_routes.size() or source_routes[slot] != _source_next_waypoint_route:
        _source_fail_point_request("captured route no longer has its actor binding")
        return false
    if not _source_route_progress_present():
        _source_fail_point_request("selected route has incomplete actor progress")
        return false
    var route = source_routes[slot]
    if route.curve == null:
        _source_fail_point_request("selected route has no native curve")
        return false
    var count: int = route.curve.point_count
    if count == 0 or count > 255 or count != route.radii.size() or count != route.point_supported.size() or route.link_supported.size() != count-1:
        _source_fail_point_request("selected route has inconsistent native geometry")
        return false
    var point := source_route_points[slot]
    var previous := source_route_previous_points[slot]
    var sample := source_route_samples[slot]
    if point < 0 or point >= count or sample < 0 or sample > 255 or previous < -1 or previous >= count or source_route_previous_samples[slot] < -1 or source_route_previous_samples[slot] > 255:
        _source_fail_point_request("selected route has invalid actor progress")
        return false
    var selected := point
    if previous != -1 and not (point == 0 and sample == 0):
        var link: int
        if previous < point:
            link = previous
        elif source_route_previous_samples[slot] == 0:
            link = point-1
        else:
            link = point
        if link < 0 or link >= route.link_supported.size() or route.link_supported[link] == 0:
            _source_fail_point_request("selected waypoint needs unsupported link geometry")
            return false
        # Supported StaticLine selects its endpoint directly at every sample.
        selected = link+1
    if selected < 0 or selected >= route.point_supported.size() or route.point_supported[selected] == 0:
        _source_fail_point_request("selected waypoint needs unsupported relative geometry")
        return false
    _source_point_target = route.curve.get_point_position(selected)
    _source_point_radius = route.radii[point]
    if not _source_point_target.is_finite() or not is_finite(_source_point_radius):
        _source_fail_point_request("selected waypoint requires finite position and radius")
        return false
    return true

## Arrival advances at most one vertex. Sampled curved links remain explicit
## failures; fixed StaticLine reports last-sample immediately.
func _source_advance_next_waypoint() -> bool:
    var slot := _source_point_route_slot
    var route = source_routes[slot]
    var point := source_route_points[slot]
    var previous := source_route_previous_points[slot]
    var sample := source_route_samples[slot]
    source_route_previous_samples[slot] = sample
    if point != 0 and previous != -1:
        var link := mini(previous,point)
        if link < 0 or link >= route.link_supported.size() or route.link_supported[link] == 0:
            _source_fail_point_request("waypoint advancement needs unsupported link geometry")
            return false
        source_route_samples[slot] = (sample+1)&255
        if point+1 == route.curve.point_count:
            _source_reset_route_progress(slot)
            return bool(route.closed)
    if previous == -1 and point+1 == route.curve.point_count:
        source_route_points[slot] = 0
        source_route_previous_points[slot] = -1
    else:
        source_route_points[slot] = (point+1)&255
        source_route_previous_points[slot] = point
    if source_route_previous_points[slot] != -1:
        if source_route_points[slot] != 0: source_route_samples[slot] = 0
        var link := mini(source_route_previous_points[slot],source_route_points[slot])
        if link < 0 or link >= route.link_supported.size() or route.link_supported[link] == 0:
            _source_fail_point_request("next waypoint initialization needs unsupported link geometry")
    return false

func _source_point_control() -> bool:
    if _source_point_route_slot >= 0 and not _source_select_waypoint_target(): return false
    if _source_point_pursuit:
        if not is_instance_valid(_source_point_actor) or not _source_point_actor.is_inside_tree():
            _source_fail_point_request("captured pursuit actor no longer belongs to the scene")
            return false
        _source_point_target = _source_point_actor.global_position
        if not _source_point_target.is_finite():
            _source_fail_point_request("pursuit actor origin must be finite")
            return false
    if _source_point_horizontal: _source_point_target.y = global_position.y+0.05
    # The source returns completion before arrival/snap if dynamics is absent.
    var moving := source_motion_binding_present
    if moving:
        var distance := global_position.distance_to(_source_point_target)
        moving = distance > _source_point_radius and not (_source_point_radius == 0.0 and distance <= 0.25)
        if not moving and _source_point_radius == 0.0: global_position = _source_point_target
    if not moving:
        var continuing := _source_advance_next_waypoint() if _source_point_route_slot >= 0 else false
        if not continuing: source_point_active = false
        return false
    if current_mechanics == null or int(current_mechanics.mode) not in [0,9]:
        _source_fail_point_request("point motion needs supported Character0 or Mass9 mechanics")
        return false
    _source_point_mass_motion = int(current_mechanics.mode) == 9
    return true

func _source_point_motion(delta: float) -> Vector3:
    if not source_point_active or not _source_point_control(): return Vector3.ZERO
    var direction := (_source_point_target-global_position).normalized()
    if _source_point_mass_motion:
        # Mass uses its +X horizontal axis for heading and the entire 3D vector
        # for translation. Native physics replaces source force-list plumbing.
        var horizontal_length := Vector2(direction.x,direction.z).length()
        var heading := atan2(-direction.z,direction.x) if horizontal_length > 0.001 else 0.0
        global_rotation = Vector3(0,heading,0)
        return direction * source_vector_speed.length() if current_mechanics.mass > 0.0 else Vector3.ZERO
    return _source_character_point_motion(direction,delta)

func _source_character_point_motion(direction: Vector3, delta: float) -> Vector3:
    direction -= up_direction * direction.dot(up_direction)
    # Walk event14 multiplies the planar direction by its planar length. It
    # uses that scaled vector for both speed and its signed acos heading.
    direction *= direction.length()
    var speed := root_motion_displacement.length()/delta
    if current_mechanics != null:
        speed = minf(speed,absf(current_mechanics.max_linear_speed))
    if not direction.is_zero_approx():
        var facing := acos(clampf(direction.z,-1.0,1.0))
        if direction.x < 0.0: facing = -facing
        facing = PackedFloat32Array([facing])[0]
        # Euler extraction can represent a positive PI pose as negative PI.
        # Read the same wrapped world forward heading as the point-turn API.
        var current_heading := _source_turn_angles_to_world_position(global_position).x
        var turn := facing-current_heading
        var half_turn: float = PackedFloat32Array([PI])[0]
        if turn > half_turn: turn -= 2.0*half_turn
        elif turn < -half_turn: turn += 2.0*half_turn
        if current_mechanics != null:
            var limit: float = absf(current_mechanics.max_angular_speed)*delta
            turn = clampf(turn,-limit,limit)
        global_rotation.y = current_heading+turn
    return direction * speed

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
    source_point_active = false
    source_point_holds_motion = false
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
    return _source_character_point_motion(direction,delta)

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

## ASTROLABE_PERF=1: time spent per actor and stage, summed in an Engine meta
## dictionary that tests/route_bot.gd prints. Kept here rather than preloaded
## from the Brain script, which preloads this one: the cycle broke threaded loads.
static var PROFILED := OS.get_environment("ASTROLABE_PERF") == "1"
static func profile_add(node: Node, stage: String, usec: int) -> void:
    var table: Dictionary = Engine.get_meta(&"astrolabe_perf", {})
    var key := str(node.name) + "/" + stage
    table[key] = int(table.get(key, 0)) + usec
    Engine.set_meta(&"astrolabe_perf", table)

## Godot's resource cache holds loaded scenes weakly. A template dropped after
## instancing was freed and parsed again, about 0.3 s, on every generation, so
## loaded templates stay referenced here for the session.
static var _templates: Dictionary = {}
static func _source_template(path: String) -> PackedScene:
    if not _templates.has(path):
        _templates[path] = load(path) as PackedScene
    return _templates[path]

func source_kill() -> bool:
    return source_kill_perso()

## KillSelf and named KillPerso share one actor-owned removal contract.
## Placed actors retain identity; generated scene instances release their node.
## A kill always takes the actor out of the active world, as in the source.
## Stand-in: an actor with unconverted activation, or a reset-on-kill policy, is
## deactivated without the component reset its revival would perform.
func source_kill_perso() -> bool:
    var generated := has_meta("astrolabe_generated_template")
    if not generated and (not source_activation_known or source_kill_resets_components):
        source_kill_reset_pending = true
    source_set_object_active_state(false)
    source_return_suppressed = true
    if generated:
        var brain := get_node_or_null("Brain")
        if brain != null: brain.call("stop")
        # The node is freed at frame end; keep it out of contact queries until then.
        collision_layer = 0
        queue_free()
    # Retained Brains keep their selected behavior and suspended native awaits.
    # The suppression latch prevents ordinary activation after this kill.
    return true

func source_set_impulse(value: Vector3) -> void:
    assert(value.is_finite())
    source_impulse = value

func source_jump_absolute() -> void:
    motion_enabled = true
    _configure_motion()
    external_velocity = _source_relative_vector(source_vector_speed + source_impulse)
    source_impulse = Vector3.ZERO
    source_relative_motion = false
    source_airborne = true

## Proc46/event9 rotates VectorSpeed and retains accumulated motion (0058cc2d).
func source_jump() -> void:
    motion_enabled = true
    _configure_motion()
    external_velocity += _source_relative_vector(source_vector_speed)
    source_relative_motion = false
    source_airborne = true

func source_picking_center() -> Vector3:
    if source_picking_center_path.is_empty(): return global_position
    var center := get_node(source_picking_center_path) as Marker3D
    assert(center != null, "Converted picking center is missing")
    return center.global_position

func source_set_stock_count(actor: Variant, amount: int) -> bool:
    if not (actor is Node) or not is_instance_valid(actor): return false
    if not actor.get_meta("source_stock_bound", false): return false
    var stock := (actor.get_meta("source_stock") if actor.has_meta("source_stock") else null) as Resource
    # The original setter does nothing on a valid actor with absent stock.
    if stock == null: return true
    var count := amount & 0xffff
    stock.set("count", count - 0x10000 if count >= 0x8000 else count)
    return true

func source_display_pickup(item: int, milliseconds: int, from: float, to: float, x: int, y: int, width: int, height: int) -> bool:
    var ancestor: Node = self
    while ancestor != null:
        var screen := ancestor.get_node_or_null("InventoryHost")
        if screen != null and screen.has_method("source_display_pickup"):
            return bool(screen.call("source_display_pickup", item, milliseconds, from, to, x, y, width, height))
        ancestor = ancestor.get_parent()
    return false

func source_jump_without_adding_speed() -> void:
    motion_enabled = true
    _configure_motion()
    external_velocity += _source_relative_vector(source_impulse)
    source_impulse = Vector3.ZERO
    source_relative_motion = false
    source_airborne = true

func source_position() -> Vector3:
    return Vector3(global_position.x, -global_position.z, global_position.y)

func source_distance_to_waypoint(actor: Node3D, waypoint: Vector3) -> float:
    assert(is_instance_valid(actor) and waypoint.is_finite())
    var source_actor_position := Vector3(actor.global_position.x, -actor.global_position.z, actor.global_position.y)
    return source_actor_position.distance_to(waypoint)

func source_actor_in_waypoint_sphere(actor: Node3D, waypoint: Resource) -> Variant:
    if not is_instance_valid(actor) or not actor.is_inside_tree() or not is_inside_tree():
        return null
    if not is_instance_valid(waypoint) or not waypoint.has_method("source_world_position"):
        return null
    var radius: Variant = waypoint.get("radius")
    if typeof(radius) not in [TYPE_FLOAT, TYPE_INT] or not is_finite(float(radius)) or float(radius) < 0.0:
        return null
    var point: Variant = waypoint.call("source_world_position", self)
    if typeof(point) != TYPE_VECTOR3 or not point.is_finite():
        return null
    var source_actor_position := Vector3(actor.global_position.x, -actor.global_position.z, actor.global_position.y)
    if not source_actor_position.is_finite():
        return null
    # The original condition tests distance - radius <= 0 (inclusive).
    return source_actor_position.distance_to(point) - float(radius) <= 0.0

func source_game_time() -> int:
    var brain := get_node_or_null("Brain")
    assert(brain != null)
    return int(brain.get("clock_milliseconds"))

func source_inventory_weapon() -> int:
    assert(source_inventory != null)
    return int(source_inventory.call("selected_weapon_value"))

func source_begin_lift_motion(from: Vector3, to: Vector3, duration: float) -> int:
    var ticket: int = source_lift_motion.begin(Vector3(from.x, from.z, -from.y), Vector3(to.x, to.z, -to.y), duration, source_motion_binding_present)
    if ticket > 0: global_position = source_lift_motion.position
    return ticket

func source_lift_motion_finished(ticket: int) -> bool:
    return source_lift_motion.finished(ticket)

func source_lift_motion_failure() -> String:
    return source_lift_motion.failure

func source_cancel_lift_motion(ticket: int) -> void:
    source_lift_motion.cancel(ticket)

func source_inventory_quantity(item: int) -> int:
    assert(source_inventory != null)
    return int(source_inventory.call("quantity_source", item))

func source_inventory_remove(item: int, count: int) -> int:
    assert(source_inventory != null)
    return int(source_inventory.call("remove_source", item, count))

func source_magic_ready() -> bool:
    return source_magick != null and bool(source_magick.call("ready"))

func source_magic_points_ready() -> bool:
    return source_magick != null and bool(source_magick.call("points_ready"))

func source_magic_current_spell() -> int:
    assert(source_magic_points_ready())
    return int(source_magick.get("selected_spell_code"))

func source_magic_points() -> int:
    assert(source_magic_points_ready())
    return int(source_magick.get("current_points"))

func source_hit_points_ready() -> bool:
    if source_hit_points == null: return false
    var current: Variant = source_hit_points.get("current")
    return current is int and current >= 0 and current <= 255

func source_get_hit_points() -> int:
    assert(source_hit_points_ready(), "Hit-point query requires converted actor life state")
    return int(source_hit_points.get("current"))

func source_magic_remove_points(amount: int) -> int:
    assert(source_magic_points_ready() and amount >= 0)
    return source_magick.call("remove_points", amount)

func source_set_laser_pointer_active(active: bool) -> void:
    assert(source_laser_pointer != null)
    source_laser_pointer.set("active", active)

func source_set_laser_pointer_sniper_mode(enabled: bool) -> bool:
    if source_laser_pointer == null: return false
    source_laser_pointer.set("sniper_mode", enabled)
    return true

## Proc_SetLaserPointerFlyMode stores the mode and source scale. Enabling it
## clears sniper mode; disabling it leaves the sniper-mode bit unchanged.
func source_set_laser_pointer_fly_mode(enabled: bool) -> bool:
    if source_laser_pointer == null: return false
    source_laser_pointer.set("fly_mode", enabled)
    source_laser_pointer.set("source_mode_scale", 3.0 if enabled else 1.0)
    if enabled:
        source_laser_pointer.set("sniper_mode", false)
    return true

func source_set_laser_pointer_target(target: Vector3) -> void:
    assert(source_laser_pointer != null and target.is_finite())
    source_laser_pointer.set("target_position", target)
    source_laser_pointer.set("target_valid", true)

func source_set_display_visible(display_visible: bool) -> void:
    source_rendering.set_display_visible(self, display_visible)

func source_actor_set_channel_active(actor_asset: String, channel: int, active: bool) -> bool:
    var actor := source_actor_node(actor_asset)
    if actor == null or not actor.has_method("source_set_channel_active"):
        return false
    return bool(actor.call("source_set_channel_active", channel, active))

func source_actor_set_channel_active_from_receiver(actor: Node3D, channel: int, active: bool) -> bool:
    if not is_instance_valid(actor) or not actor.has_method("source_set_channel_active"):
        return false
    return bool(actor.call("source_set_channel_active", channel, active))

func source_actor_set_display_visible(actor_asset: String, display_visible: bool) -> bool:
    var actor := source_actor_node(actor_asset)
    if actor == null or not actor.has_method("source_set_display_visible"):
        return false
    actor.call("source_set_display_visible", display_visible)
    return true

func source_actor_set_display_visible_from_receiver(actor: Node3D, display_visible: bool) -> bool:
    if not is_instance_valid(actor) or not actor.has_method("source_set_display_visible"):
        return false
    actor.call("source_set_display_visible", display_visible)
    return true

## 00543130 clamps opacity separately from transparent/default display mode.
## Material alpha works on the project's Compatibility renderer; the native
## GeometryInstance3D transparency property is Forward+-only.
func source_set_transparency(opacity: int) -> bool:
    return source_rendering.set_transparency(self, opacity)

func source_set_transparent_display(enabled: bool) -> bool:
    return source_rendering.set_transparent_display(self, enabled)

## Proc237 mutates the shared source VisualMaterial diffuse values. The active
## level owns the native variants; actor copies must resolve through that scope.
## Keep this resolution check separate so the generated AI reads its packed
## designer operand only after a valid source material target is available.
func source_visual_material_target_ready(material_path: String) -> bool:
    var current: Node = self
    while current != null:
        if current.has_node("SourceVisualMaterialState"):
            var state: Node = current.get_node("SourceVisualMaterialState")
            if not is_instance_valid(state) or not state.has_method("can_set_visual_diffuse_coef"):
                return false
            return bool(state.call("can_set_visual_diffuse_coef", material_path))
        current = current.get_parent()
    return false

func source_set_visual_gmt_diffuse_coef(material_path: String, packed_rgba: int) -> bool:
    var current: Node = self
    while current != null:
        if current.has_node("SourceVisualMaterialState"):
            var state: Node = current.get_node("SourceVisualMaterialState")
            if not is_instance_valid(state) or not state.has_method("set_visual_diffuse_coef"):
                return false
            return bool(state.call("set_visual_diffuse_coef", material_path, packed_rgba))
        current = current.get_parent()
    return false

## Keep opacity's actor-local ShaderMaterial copy connected to the material-state
## service when Proc237 replaces that active variant.
func source_rebind_visual_material_variant(previous: Material, replacement: ShaderMaterial) -> void:
    source_rendering.rebind_visual_material_variant(self, previous, replacement)

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

## Func171 uses source vectors and a raw low-byte selector: 0=Z, 1=X, 2=Y.
func source_vector_angle(a: Vector3, b: Vector3, axis: Variant) -> float:
    var first_squared := PackedFloat32Array([float(a.x) * a.x + float(a.y) * a.y + float(a.z) * a.z])[0]
    var second_squared := PackedFloat32Array([float(b.x) * b.x + float(b.y) * b.y + float(b.z) * b.z])[0]
    var product := float(first_squared) * second_squared
    var denominator := PackedFloat32Array([sqrt(product) if product != 0.0 else 1.0])[0]
    var dot := PackedFloat32Array([float(a.x) * b.x + float(a.y) * b.y + float(a.z) * b.z])[0]
    var ratio := PackedFloat32Array([dot / denominator])[0]
    # Source clamps only above +1. Zero vectors therefore return 90 degrees.
    if ratio > 1.0: ratio = 1.0
    var degrees := PackedFloat32Array([acos(ratio) * PackedFloat32Array([57.29578])[0]])[0]
    var cross := a.cross(b)
    var selector := source_scalar_word(axis) & 0xff
    var sign := cross.z if selector == 0 else (cross.x if selector == 1 else (cross.y if selector == 2 else 0.0))
    return -degrees if sign < 0.0 else degrees

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

func source_set_particle_generator(template: PackedScene) -> bool:
    var component := get_node_or_null("ParticleSource")
    # 00542340 skips selection when the original actor has no component.
    if component == null: return true
    return bool(component.call("select_generator", template))

func source_set_particle_generator_enabled(enabled: bool) -> bool:
    var component := get_node_or_null("ParticleSource")
    if component != null and not bool(component.call("set_generator_enabled", enabled)):
        return false
    source_particle_generator_enabled = enabled
    return true

## Requests belong to the actor, including IDs absent from its current rig.
## Matching native parents carry visibility independently of part-selection tracks.
func source_set_channel_active(channel: int, active: bool) -> bool:
    return source_channel_visibility.set_active(self, channel, active)

func source_set_channel_parent(parent: Node3D, channel: int, attached: bool) -> bool:
    return source_channel_attachment.change(self, parent, channel, attached)

## The authored HUD can bind this signal to its native Control nodes. Keep the
## requested state when the current test scene has no HUD composition yet.
func source_set_hud_indicator_visible(indicator: String, visible: bool) -> void:
    assert(indicator in ["life", "armor", "magic", "selected_object", "weapon", "cash"])
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
    var template := _source_template(template_path)
    if template == null:
        push_error("Action/object request has no converted Perso template")
        return -1
    if not vector.is_finite():
        push_error("Action/object request has an invalid motion vector")
        return -1
    # Generated actors share the active level's state and external references.
    # The authored game session is above that level and owns a different API.
    var destination: Node3D = self
    while destination != null and not destination.has_meta("source_actor_brains"):
        destination = destination.get_parent() as Node3D
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
        # Source generation's component initialization clears suppression.
        # A held object remains inactive until its authored release cue.
        generated.set("source_return_suppressed", false)
        generated.set("source_object_active", not waits_for_release)
        # Only objects with a mechanics card have dynamics. A card-less one, such
        # as Hype's Fire effect (model 817), stays where it was generated; with
        # gravity it sank, and its fireball hit the mannequin's body, not the arm.
        generated.set("motion_enabled", generated.get("current_mechanics") != null)
        # Stand-in "item point contact": dropped items (families 21 to 25) carry
        # no movement zone, yet bounce and rest on the floor in the source.
        # A small sphere at the origin stops them falling through the world.
        var item_mechanics: Resource = generated.get("current_mechanics")
        if item_mechanics != null and not generated.call("_has_movement_shape") and int(item_mechanics.mode) != 11:
            var point := CollisionShape3D.new()
            point.name = "ItemPointContact"
            var sphere := SphereShape3D.new()
            sphere.radius = ITEM_POINT_RADIUS
            point.shape = sphere
            point.position = Vector3.UP * ITEM_POINT_RADIUS
            generated.add_child(point)
        generated.set("physical_environment", physical_environment)
        generated.set("source_inventory", source_inventory)
        generated.set("source_armor", source_armor)
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
        var caller_brain := get_node_or_null("Brain")
        var main: Node3D = caller_brain.get("main_actor") if caller_brain != null else self
        if not is_instance_valid(main):
            generated.free()
            push_error("Generated actor requires the active MainActor binding")
            return -1
        generated_brain.set("main_actor", main)
        if waits_for_release: generated_brain.set("automatic", false)
    if generated.has_method("source_contact_sector"):
        generated.set("_contact_sector", source_contact_sector())
    generated.set_meta("astrolabe_generated_template", template_path)
    if generated.get("source_generator") == null: generated.set("source_generator", self)
    destination.add_child(generated)
    if generated_brain != null:
        generated_brain.set("main_actor_path", generated_brain.get_path_to(generated_brain.get("main_actor")))
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

static func source_scalar_word(value: Variant) -> int:
    if typeof(value) == TYPE_FLOAT:
        var bytes := PackedByteArray()
        bytes.resize(4)
        bytes.encode_float(0, float(value))
        return bytes.decode_u32(0)
    return int(value) & 0xffffffff

func source_has_custom_bit(bit: Variant) -> bool:
    var mask := 1 << ((source_scalar_word(bit) - 1) & 31)
    return (source_custom_bits & mask) != 0

## Compiled callers capture old flags before evaluating either operand.
func source_change_custom_bit(previous: int, bit: Variant, value: Variant) -> void:
    var mask := 1 << ((source_scalar_word(bit) - 1) & 31)
    source_custom_bits = ((previous | mask) if (source_scalar_word(value) & 0xff) != 0 else (previous & ~mask)) & 0xffffffff

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

## 00534af0: the segment meets sector geometry only, so actors never block it.
func _source_sees(origin: Vector3, endpoint: Vector3) -> bool:
    if origin.is_equal_approx(endpoint): return true
    var query := PhysicsRayQueryParameters3D.create(origin, endpoint, 1)
    return get_world_3d().direct_space_state.intersect_ray(query).is_empty()

static func _source_to_native(value: Vector3) -> Vector3:
    return Vector3(value.x, value.z, -value.y)

func source_see_position(target: Vector3, offset: Vector3) -> bool:
    return _source_sees(_source_sight_point(self) + _source_to_native(offset), _source_to_native(target))

func source_see_perso_offset(offset: Vector3, target: Node3D, target_offset: Vector3) -> bool:
    if not is_instance_valid(target): return false
    var origin := global_position + global_basis * _source_to_native(offset)
    return _source_sees(origin, target.global_position + target.global_basis * _source_to_native(target_offset))

## 00538020: nearest point by straight distance. The source loop stops one short
## of the route's last point.
func source_closer_route_waypoint(slot: int) -> Resource:
    if slot < 0 or slot >= source_routes.size() or source_routes[slot] == null: return null
    var route = source_routes[slot]
    var best := -1
    var best_distance := -1.0
    for i in route.curve.point_count - 1:
        var distance: float = global_position.distance_to(route.curve.get_point_position(i))
        if best < 0 or distance < best_distance:
            best = i
            best_distance = distance
    if best < 0: return null
    var point: Vector3 = route.curve.get_point_position(best)
    var waypoint: Resource = load("res://addons/astrolabe/character/waypoint.gd").new()
    waypoint.source_position = Vector3(point.x, -point.z, point.y)
    waypoint.radius = route.radii[best] if best < route.radii.size() else 0.0
    waypoint.source_identity = str(route.source_identity, "#", best)
    waypoint.supported = true
    return waypoint

func source_collide_with_perso() -> bool:
    for i in get_slide_collision_count():
        if get_slide_collision(i).get_collider() is CharacterBody3D: return true
    return false

const SOURCE_STEP_DOWN := 0.35
const ITEM_POINT_RADIUS := 0.1

func source_collide_with_ground() -> bool:
    if is_on_floor(): return true
    # Resting items have no movement shape, so they never report a native floor.
    if _has_movement_shape(): return false
    var from := global_position + Vector3.UP * 0.25
    var query := PhysicsRayQueryParameters3D.create(from, from + Vector3.DOWN * 1.0, 1)
    return not get_world_3d().direct_space_state.intersect_ray(query).is_empty()

func _has_movement_shape() -> bool:
    for child in get_children():
        if child is CollisionShape3D and not child.disabled: return true
    return false

## Original actors stop against each other's reaction zones, layer 8, but an
## actor's own zones must not block it.
func _collide_with_actor_zones() -> void:
    collision_mask |= 8
    var own := get_node_or_null("ZDR")
    if own is PhysicsBody3D: add_collision_exception_with(own)

## The mechanics collision path reports a contact result after each movement
## step. CharacterBody3D publishes the corresponding native result through its
## slide contacts; AI conditions run after _physics_process has moved the body.
func source_collide_with_nothing() -> bool:
    # 00533d20 reads the actor's dynamics collision report (0052ec80); an actor
    # without one answers false to every collision condition, Nothing included.
    # An actor the port does not simulate has no report either, so a standing
    # thief is not taken for airborne.
    if not motion_enabled: return false
    return get_slide_collision_count() == 0

## Resolve a compiled Perso identity in the active level's saved actor graph.
## The nearest owning index stays authoritative inside authored scene wrappers.
func source_actor_node(actor_asset: String) -> Node3D:
    var level: Node = self
    while level != null:
        if level.has_meta("source_actor_nodes"):
            var actors: Dictionary = level.get_meta("source_actor_nodes")
            if not actors.has(actor_asset): return null
            return level.get_node_or_null(actors[actor_asset]) as Node3D
        level = level.get_parent()
    return null

## Sector operands identify scene-owned components, separately from Perso assets.
func source_sector_node(sector_member: String) -> Node3D:
    var level: Node = self
    while level != null:
        if level.has_meta("source_sector_nodes"):
            var sectors: Dictionary = level.get_meta("source_sector_nodes")
            if not sectors.has(sector_member): return null
            var sector := level.get_node_or_null(sectors[sector_member]) as Node3D
            return sector if is_instance_valid(sector) and sector.has_meta("source_sector_id") else null
        level = level.get_parent()
    return null

## Query the current behavior slot on a resolved actor. A missing Brain or an
## empty current slot is false, matching the source query's null-slot result.
func source_actor_is_in_behavior(actor: Node3D, group: StringName, target: int) -> bool:
    if not is_instance_valid(actor): return false
    var brain := actor.get_node_or_null("Brain")
    if not is_instance_valid(brain) or not brain.has_method("source_is_in_behavior"):
        return false
    return bool(brain.call("source_is_in_behavior", group, target))

## Source type-11 Perso lists append without deduplication and cannot grow
## beyond their declared capacity. The compiled AI owns the decision to add.
func source_list_add(list: Array[Node3D], capacity: int, actor: Node3D) -> bool:
    if not is_instance_valid(actor) or capacity < 0 or list.size() >= capacity:
        return false
    list.append(actor)
    return true

## Proc160 phase one precedes evaluation of the new target in generated code.
func source_camera_prepare_target_change(camera_asset: String) -> bool:
    var camera_actor := source_actor_node(camera_asset)
    var camera := camera_actor.get_node_or_null("SourceCamera") if camera_actor != null else null
    return camera != null and camera.has_method("source_prepare_target_change") and bool(camera.call("source_prepare_target_change"))

## Proc160 phase two receives the already evaluated target.
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

## Procedure 163 resets the component's +0x10 target to the current MainActor.
func source_camera_reset_targeted_perso(camera_asset: String, target: Node3D) -> bool:
    var camera_actor := source_actor_node(camera_asset)
    var camera := camera_actor.get_node_or_null("SourceCamera") if camera_actor != null else null
    return camera != null and camera.has_method("source_reset_targeted_perso") and bool(camera.call("source_reset_targeted_perso", target))

## Proc136 restores framing/progress without assigning a new camera target.
func source_camera_reset(camera_asset: String) -> bool:
    var camera_actor := source_actor_node(camera_asset)
    var camera := camera_actor.get_node_or_null("SourceCamera") if camera_actor != null else null
    return camera != null and camera.has_method("source_reset_to_initial") and bool(camera.call("source_reset_to_initial"))

## Called by the compiled camera Brain. The native camera supports the mapped
## mode-0 controller/collision branches and mode-3 aiming. Missing source
## inputs and unported mode-0 branches still fail explicitly.
func source_camera_update_position() -> bool:
    var camera := get_node_or_null("SourceCamera")
    if camera == null or not camera.has_method("source_update_position"):
        return false
    return bool(camera.call("source_update_position"))

## Camera_GoToWP samples the captured waypoint's current world location and
## camera translation on every update. Source arrival squares the radius, including
## negative values, and includes equality. Camera position updates consume the
## supplied displacement; this API does not teleport or tween the actor.
func source_camera_go_to_waypoint(camera_actor: Variant, waypoint: Variant) -> int:
    if not camera_actor is Node3D or not is_instance_valid(camera_actor) or not camera_actor.is_inside_tree():
        return -1
    if not waypoint is Resource or not is_instance_valid(waypoint) or not waypoint.has_method("source_world_position"):
        return -1
    var point: Variant = waypoint.call("source_world_position", self)
    var radius: Variant = waypoint.get("radius")
    if not point is Vector3 or not point.is_finite() or not (radius is float or radius is int) or not is_finite(float(radius)):
        return -1
    var camera: Node = camera_actor.get_node_or_null("SourceCamera")
    if camera == null or not camera.has_method("source_set_eye_displacement"):
        return -1
    var native_point := Vector3(point.x, point.z, -point.y)
    var displacement: Vector3 = native_point - camera_actor.global_position
    if not displacement.is_finite(): return -1
    if displacement.length_squared() <= float(radius) * float(radius): return 4
    return 1 if bool(camera.call("source_set_eye_displacement", displacement)) else -1

func source_activate_map_name(name_id: int, active: int) -> bool:
    var level: Node = self
    while level != null:
        var names := level.get_node_or_null("SourceMapNames")
        if names != null and names.has_method("activate_name"):
            names.call("activate_name", name_id, active)
            return true
        level = level.get_parent()
    return false

## Function_Camera_GetShiftAlpha reads the selected Perso camera's raw +0x44.
## Null signals an unsupported selector or missing native camera component.
## Proc153: `actor` is the camera Perso; the receiver only reaches the API.
func source_camera_change_angle(actor: Variant, field: int, mode: int, value: float) -> bool:
    var camera: Node = _source_camera_of(actor)
    return camera != null and bool(camera.call("source_change_angle", field, mode, value))

## Proc143 changes the same camera's distance window.
func source_camera_change_distance(actor: Variant, mode: int, minimum: float, maximum: float) -> bool:
    var camera: Node = _source_camera_of(actor)
    return camera != null and bool(camera.call("source_change_distance", mode, minimum, maximum))

## Function_Camera_Get* for the ids carried by Expression::CameraValue.
func source_camera_value(actor: Variant, getter: int, selector: int) -> Variant:
    var camera: Node = _source_camera_of(actor)
    return camera.call("source_value", getter, selector) if camera != null else null

func _source_camera_of(actor: Variant) -> Node:
    if typeof(actor) != TYPE_OBJECT or not is_instance_valid(actor) or not actor is Node: return null
    return actor.get_node_or_null("SourceCamera")

func source_camera_get_shift_alpha(selector: int) -> Variant:
    var camera := get_node_or_null("SourceCamera")
    if camera == null or not camera.has_method("source_get_shift_alpha"):
        return null
    return camera.call("source_get_shift_alpha", selector)

## Function_Camera_GetShiftTeta reads the selected Perso camera's raw +0x4c.
func source_camera_get_shift_teta(selector: int) -> Variant:
    var camera := get_node_or_null("SourceCamera")
    if camera == null or not camera.has_method("source_get_shift_teta"):
        return null
    return camera.call("source_get_shift_teta", selector)

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

func source_actor_set_absolute_position_from_receiver(actor: Node3D, point: Vector3) -> bool:
    if not is_instance_valid(actor) or not actor.is_inside_tree() or not point.is_finite():
        return false
    actor.global_position = Vector3(point.x, point.z, -point.y)
    return true

func source_set_object_active(actor_asset: String, active: bool) -> bool:
    var actor := source_actor_node(actor_asset)
    if actor == null or not actor.has_method("source_set_object_active_state"):
        return false
    return bool(actor.call("source_set_object_active_state", active))

func source_set_object_active_state(active: bool) -> bool:
    # Original activation ignores a suppressed actor without resetting its AI.
    if active and source_return_suppressed: return true
    if not active:
        var emitter := get_node_or_null("SoundEmitter")
        if emitter != null:
            if emitter.has_method("kill_all"): emitter.call("kill_all")
    source_object_active = active
    visible = active
    process_mode = Node.PROCESS_MODE_INHERIT if active else Node.PROCESS_MODE_DISABLED
    return true

## Proc12 raises the requested source Z by half a unit, snapshots that pose,
## then lowers the rounded vector by half a unit through contact and activates.
## Direct placement uses the dynamics binding independently of its normal motion
## callback. Character0, the bound camera and a converted null card are covered;
## other movement and moving-parent contracts remain separate.
func source_activate_at_position(point: Vector3) -> bool:
    if not is_inside_tree() or not point.is_finite(): return false
    if source_motion_binding_present:
        # Placement depends on the dynamics workspace, not the selected card.
        # The key's null card is explicitly marked by native conversion.
        if current_mechanics == null:
            if not source_camera_retains_position_history: return false
        elif int(current_mechanics.mode) != 0 and not (int(current_mechanics.mode) == 3 and get_node_or_null("SourceCamera") != null):
            return false
        var ancestor := get_parent()
        while ancestor != null:
            if ancestor is CharacterBody3D: return false
            ancestor = ancestor.get_parent()
        var raised_z: float = PackedFloat32Array([float(point.z) + 0.5])[0]
        var lowered_z: float = PackedFloat32Array([raised_z - 0.5])[0]
        if not is_finite(raised_z) or not is_finite(lowered_z): return false
        var raised := Vector3(point.x, raised_z, -point.y)
        var lowered := Vector3(point.x, lowered_z, -point.y)
        # Placement still queries contact for an initially inactive actor.
        # This synchronous query does not enter its Brain or change active state.
        var previous_process_mode := process_mode
        process_mode = Node.PROCESS_MODE_INHERIT
        global_position = raised
        var collision := move_and_collide(lowered - raised, false, safe_margin, true)
        process_mode = previous_process_mode
        if collision != null:
            # The source's contact-success branch clears its velocity packet.
            velocity = Vector3.ZERO
            external_velocity = Vector3.ZERO
        source_relative_motion = false
        source_contact_sector()
    # Both original placement helpers return zero without dynamics; activation
    # still happens in that branch, with the existing actor origin preserved.
    return source_set_object_active_state(true)

## Hype Proc_SetPersoAbsolutePosition moves the executing Perso.
func source_set_absolute_position(point: Vector3) -> bool:
    if not point.is_finite():
        return false
    global_position = Vector3(point.x, point.z, -point.y)
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
    root_motion_turn = Quaternion.IDENTITY
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
    completion_target_state = id
    source_route_holds_motion = false
    source_point_holds_motion = false
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
    # stop() clears the mixer's track cache, and rebuilding it for a thousand
    # tracks stalled every action change by about 60 ms.
    animation_player.play(clip)
    animation_player.seek(0.0, true, true)
    advance_native(0.0)
    _state_movement_collision(current_state)
    if sample_rate <= 0.0: animation_player.pause()
    return true

## Proc_ChangeAction preserves an already requested action; ExecuteAction restarts.
## A rejected transition leaves the current action and its completion unchanged.
func is_in_action(id: int) -> bool:
    # 00435750 compares the numeric request id, independently of the native
    # completion target selected by a transition or automatic next action.
    return (requested_action_state if action_controlled else current_state) == id

func source_action_completed() -> bool:
    # 00436000: byte14a and current/requested action pointer equality. Native
    # state identity replaces the source pointers; animation completion stays
    # separate (00436040) and this predicate does not wait for speech/audio.
    return action_complete and current_state == completion_target_state

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
    source_point_holds_motion = false
    _play_action_state(selected)
    action_request_traced.emit(id, restart, previous, selected, true)
    return true

func _play_action_state(id: int) -> void:
    action_controlled = true
    action_complete = false
    # 00433e90 installs the selected/automatic next action, then realigns the
    # completion target to it. The numeric request used by00435750 is retained.
    completion_target_state = id
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
    # These two native groups own state-selected shapes. Configuration banks
    # carry independent physical fallback; do not traverse them on each action.
    for group_name in ["ZDE", "ZDR"]:
        var group := get_node_or_null(group_name)
        if group == null: continue
        for value in group.find_children("*", "CollisionShape3D", true, false):
            var shape := value as CollisionShape3D
            if not shape.has_meta("source_active_states"):
                continue
            var states: PackedInt32Array = shape.get_meta("source_active_states")
            _set_zde_shape_active(shape, states.has(_state))

func _set_zde_shape_active(shape: CollisionShape3D, active: bool) -> void:
    shape.disabled = not active or (int(shape.get_meta("source_collision_identifier", 0)) & 0x8000) != 0

## Native contact sectors are named scene nodes; no retained source JSON or
## source character lists are interpreted here. Generated drops inherit theirs.
func source_refresh_contact_sector(sector: Node3D) -> bool:
    if not is_instance_valid(sector) or not is_inside_tree() or not sector.is_inside_tree() or not sector.has_meta("source_sector_id") or sector.get_tree() != get_tree():
        return false
    _contact_sector = sector
    source_sector_membership_known = true
    source_contact_sector_path = get_path_to(sector)
    _contact_previous = global_position
    return true

## SCT_ActorInSector reads holder+4 without recomputing geometric membership.
## An absent holder/binding is an unsupported query; a bound null sector is valid.
func source_sector_membership_ready() -> bool:
    if not source_sector_membership_known: return false
    if source_contact_sector_path.is_empty(): return _contact_sector == null
    return is_instance_valid(source_current_sector())

func source_current_sector() -> Node3D:
    if not is_instance_valid(_contact_sector) and not source_contact_sector_path.is_empty():
        _contact_sector = get_node_or_null(source_contact_sector_path) as Node3D
    return _contact_sector

## 00448070/004480e0: the main actor's sector change requests its new sector's music.
func _source_sector_music(from_sector: Node3D, to_sector: Node3D) -> void:
    var brain := get_node_or_null("Brain")
    if brain == null or brain.get("main_actor") != self: return
    var session: Node = brain.call("source_game_session")
    var state: int = int(session.get("source_game_state_mask")) if session != null else 0
    for entry: Dictionary in to_sector.get("sound_events"):
        var from: String = entry["from"]
        if not from.is_empty() and from != str(from_sector.get_meta("source_node_id", "")): continue
        var mask: int = entry["state"]
        if mask & state == 0 and not (mask == 0 and state == 0): continue
        brain.call("send_music", entry["event"])

func source_contact_sector() -> Node3D:
    source_current_sector()
    if _contact_previous == null: _contact_previous = global_position
    if is_instance_valid(_contact_sector) and _contact_previous != global_position:
        var start: Vector3 = _contact_previous
        for crossing in 64:
            var boundary := _contact_sector.get_node_or_null("SectorBoundary")
            if boundary == null or not boundary.has_method("first_source_sector_crossing"): break
            var hit: Dictionary = boundary.call("first_source_sector_crossing", start, global_position)
            var target: Node3D = hit.get("target") as Node3D
            if not is_instance_valid(target) or target == _contact_sector: break
            _source_sector_music(_contact_sector, target)
            _contact_sector = target
            start = hit["position"] + start.direction_to(global_position) * 0.00001
    _contact_previous = global_position
    _update_sector_ambient()
    return _contact_sector

const DROP_SHADOW = preload("res://addons/astrolabe/rendering/drop_shadow.gd")
## Perso3dData shadow kind and scale; see rendering/drop_shadow.gd.
@export var source_shadow_quality := 0
@export var source_shadow_scale := Vector2.ONE
## The row of the source lighting table for this actor's sector. Dynamic meshes
## take the lights of the sector they are drawn in, so each actor needs its own.
var source_lighting_row := 0
var _ambient_sector: Node3D
var _ambient_probe := Vector3(INF, INF, INF)
## An actor with no contact sector (loaded saves) is placed by position,
## at most once per metre moved, only for lighting.
func _update_sector_ambient() -> void:
    var sector := _contact_sector
    if sector == null:
        sector = _ambient_sector
        if global_position.distance_to(_ambient_probe) > 1.0 or _ambient_sector == null and Engine.get_physics_frames() % 30 == 0:
            _ambient_probe = global_position
            var root: Node = self
            while root != null and not root.has_meta("source_level_scene"): root = root.get_parent()
            var index: Node = root.get_node_or_null("SourceSectorIndex") if root != null else null
            if index != null:
                var hit: Dictionary = index.call("resolve_position", global_position)
                sector = index.call("sector_node", str(hit.get("sector_id", ""))) as Node3D
                if sector == null: sector = _ambient_sector
    if sector != _ambient_sector: _apply_sector_lighting(sector)

func _apply_sector_lighting(sector: Node3D) -> void:
    _ambient_sector = sector
    if not is_instance_valid(sector) or sector.owner == null: return
    var state := sector.owner.get_node_or_null("SourceVisualMaterialState")
    if state == null or state.get("lighting") == null: return
    source_lighting_row = state.lighting.row(sector)
    for mesh in find_children("*", "MeshInstance3D", true, false):
        if mesh.layers == 2: state.set_source_lighting_row(mesh, source_lighting_row)

func _source_contact_actor(collider: CollisionObject3D) -> Node3D:
    var actor: Node = collider
    while actor != null:
        if actor.has_method("source_contact_sector"): return actor as Node3D
        actor = actor.get_parent()
    return null

func _source_contact_candidate(actor: Node3D, physical: bool) -> bool:
    if actor == null or actor == self or not actor.source_contact_eligible or not actor.source_object_active:
        return false
    if actor == source_generator: return false
    if physical and not actor.is_visible_in_tree(): return false
    var own_sector := source_contact_sector()
    var target_sector: Node3D = actor.source_contact_sector()
    # Standalone synthetic/authored actors may have no source-sector binding.
    if own_sector == null: return target_sector == null
    if not bool(own_sector.get_meta("contact_sector_active", false)): return false
    if target_sector == null or not bool(target_sector.get_meta("contact_sector_active", false)): return false
    if target_sector == own_sector: return true
    for neighbor in own_sector.get_meta("contact_sector_neighbors", []):
        if own_sector.get_node_or_null(neighbor) == target_sector: return true
    return false

## Cond28's third argument is a low16 all-bit material mask. Independent
## source result capacity is one. Native shape/physics ordering is an explicit
## adaptation; this does not claim the original selects the same contact pair.
func source_collide_perso_zde_no_with_type_zde(zone_number: int, material_mask: int) -> bool:
    var query_area := get_node_or_null("ZDE") as Area3D
    if query_area == null or not source_object_active: return false
    var sector := source_contact_sector()
    if sector != null and not bool(sector.get_meta("contact_sector_active", false)): return false
    var space := get_world_3d().direct_space_state
    var selected_zone := (zone_number - 1) & 0xff
    var selected_actors: Dictionary = {}
    for value in query_area.find_children("*", "CollisionShape3D", true, false):
        var query_shape := value as CollisionShape3D
        if int(query_shape.get_meta("source_zone", -1)) != selected_zone: continue
        if query_shape.disabled or query_shape.shape == null: continue
        var parameters := PhysicsShapeQueryParameters3D.new()
        parameters.shape = query_shape.shape
        parameters.transform = query_shape.global_transform
        parameters.collision_mask = 4 | 16
        parameters.collide_with_areas = true
        parameters.collide_with_bodies = false
        parameters.margin = 0.0
        parameters.exclude = [query_area.get_rid()]
        var hits := space.intersect_shape(parameters, SOURCE_COLLISION_QUERY_MAX_RESULTS)
        # ZDE contacts precede physical fallback. Once the native intersection
        # chooses a target pair, its failed material cannot be replaced by a
        # different shape of that same actor. Within each kind native ordering
        # remains an explicit adaptation of source list/element order.
        hits.sort_custom(func(a, b): return not bool(a.collider.get_meta("physical_object_contact", false)) and bool(b.collider.get_meta("physical_object_contact", false)))
        for hit in hits:
            var collider := hit.get("collider") as CollisionObject3D
            if collider == null: continue
            var physical := bool(collider.get_meta("physical_object_contact", false))
            var actor := _source_contact_actor(collider)
            if not _source_contact_candidate(actor, physical): continue
            if physical and not collider.is_visible_in_tree(): continue
            var identity := actor.get_instance_id()
            if selected_actors.has(identity): continue
            selected_actors[identity] = true
            if CONTACT_MATERIAL.contact_material_matches(collider, int(hit.get("shape", -1)), material_mask):
                return true
    return false

## Cond24..27 (0042f980 -> 0042f4e0). A first-side set (an active ZDE zone or
## a visible physical collide set) qualifies when any of its elements carries
## every first_mask bit; it then collides when it intersects an element of
## `second` (ZDE or physical) whose own material carries every second_mask
## bit. No sector filter: the source intersects the geometry directly.
func source_collide_perso_type_zde(first: Variant, first_mask: int, second: Variant, second_mask: int) -> bool:
    if typeof(first) != TYPE_OBJECT or typeof(second) != TYPE_OBJECT: return false
    if not is_instance_valid(first) or not is_instance_valid(second): return false
    if not (first is Node3D) or not (second is Node3D) or not first.is_inside_tree(): return false
    var sets: Array = []
    var zde := first.get_node_or_null("ZDE") as Area3D
    if zde != null:
        var zones := {}
        for value in zde.find_children("*", "CollisionShape3D", true, false):
            var shape := value as CollisionShape3D
            if shape.disabled or shape.shape == null: continue
            var zone := int(shape.get_meta("source_zone", -1))
            if not zones.has(zone): zones[zone] = []
            zones[zone].append(shape)
        for zone in zones: sets.append(zones[zone])
    for value in first.find_children("PhysicalContact", "Area3D", true, false):
        var area := value as Area3D
        if _source_contact_actor(area) != first or not area.is_visible_in_tree(): continue
        var shapes: Array = []
        for shape in area.find_children("*", "CollisionShape3D", true, false):
            if not shape.disabled and shape.shape != null: shapes.append(shape)
        sets.append(shapes)
    var space := get_world_3d().direct_space_state
    for shapes in sets:
        var qualifies := false
        for shape in shapes:
            if bool(shape.get_meta("source_contact_material_present", true)) and shape.has_meta("source_collision_identifier") \
                    and CONTACT_MATERIAL.material_matches(int(shape.get_meta("source_collision_identifier")), first_mask):
                qualifies = true
                break
        if not qualifies: continue
        for shape in shapes:
            var parameters := PhysicsShapeQueryParameters3D.new()
            parameters.shape = shape.shape
            parameters.transform = shape.global_transform
            parameters.collision_mask = 4 | 16
            parameters.collide_with_areas = true
            parameters.collide_with_bodies = false
            parameters.margin = 0.0
            for hit in space.intersect_shape(parameters, SOURCE_COLLISION_QUERY_MAX_RESULTS):
                var collider := hit.get("collider") as CollisionObject3D
                if collider == null or _source_contact_actor(collider) != second: continue
                if bool(collider.get_meta("physical_object_contact", false)) and not collider.is_visible_in_tree(): continue
                if CONTACT_MATERIAL.contact_material_matches(collider, int(hit.get("shape", -1)), second_mask):
                    return true
    return false

func _set_movement_shape_active(shape: CollisionShape3D, active: bool) -> void:
    shape.disabled = not active or (int(shape.get_meta("source_collision_identifier", 0)) & 0x8000) != 0

func advance_native(seconds: float) -> void:
    source_channel_visibility.reapply_deactivated(self)
    advancing = true
    animation_player.advance(seconds)
    if seconds > 0.0:
        root_motion_displacement += animation_player.get_root_motion_position()
        root_motion_turn = root_motion_turn * animation_player.get_root_motion_rotation()
    advancing = false
    if pending_state >= 0:
        var state := pending_state
        pending_state = -1
        _select_state(state)

func seek_frame(index: int) -> void:
    if current_clip == null: return
    assert(index >= 0 and index < int(current_clip.get_meta("source_frames", 1)))
    source_channel_visibility.reapply_deactivated(self)
    animation_player.seek(float(index) / maxf(sample_rate, 1.0), true, true)

func restore_configuration() -> bool:
    if source_configuration == null: return false
    var changed := bool(source_configuration.call("restore"))
    return changed and source_configuration_error.is_empty()

func _bind_source_configuration() -> void:
    var ancestor := get_parent()
    var shared: Dictionary = {}
    # The table lives on the topmost scene ancestor (below the Window) so every
    # actor of the level binds the same configuration objects.
    var table_owner: Node = null
    while ancestor != null and not ancestor is Window:
        table_owner = ancestor
        if ancestor.has_meta("source_configurations"):
            shared = ancestor.get_meta("source_configurations")
            break
        ancestor = ancestor.get_parent()
    ancestor = table_owner
    for index in source_configuration_choices.size():
        var choice: Resource = source_configuration_choices[index]
        var identity: String = str(choice.get("source_id"))
        if not shared.has(identity):
            var owned: Resource = choice.duplicate(true)
            owned.resource_local_to_scene = true
            shared[identity] = owned
        source_configuration_choices[index] = shared[identity]
    if ancestor != null: ancestor.set_meta("source_configurations", shared)
    if source_configuration != null:
        source_configuration = shared[str(source_configuration.get("source_id"))]
    if source_initial_configuration != null:
        source_initial_configuration = shared[str(source_initial_configuration.get("source_id"))]
    if source_configuration != null and not source_configuration.changed.is_connected(_apply_source_configuration):
        source_configuration.changed.connect(_apply_source_configuration)
    _configuration_bound = true
    _apply_source_configuration()

## Called during conversion after the initial native pose has been applied.
## Runtime animation/visibility transitions are handled by the slots' signals.
func refresh_configured_participation() -> void:
    var channels := get_node_or_null("Channels")
    if channels == null: return
    for channel in channels.get_children():
        for anchor in channel.get_children():
            if anchor.has_method("refresh_participation"):
                anchor.call("refresh_participation")

## Proc83 replaces only this actor's selection. Shared table state is retained.
## Null is an empty visual selection, without an animation or gameplay reset.
func source_change_configuration(value: Resource) -> bool:
    var selected: Resource = null
    if value != null:
        var identity: String = str(value.get("source_id"))
        for choice in source_configuration_choices:
            if str(choice.get("source_id")) == identity:
                selected = choice
                break
        if selected == null: return false
    if source_configuration != null and source_configuration.changed.is_connected(_apply_source_configuration):
        source_configuration.changed.disconnect(_apply_source_configuration)
    source_configuration = selected
    if selected != null: selected.changed.connect(_apply_source_configuration)
    return _apply_source_configuration()

## 004421f2 restores the initial table's saved values without selecting it.
func source_restore_initial_configuration() -> bool:
    if source_initial_configuration == null: return false
    return bool(source_initial_configuration.call("restore"))

func source_swap_configured_parts(first: int, second: int) -> bool:
    if source_configuration == null: return false
    var changed := bool(source_configuration.call("swap_slots", first, second))
    return changed and source_configuration_error.is_empty()

func _configuration_failure(reason: String) -> bool:
    source_configuration_error = reason
    push_error("Native configuration for " + str(get_meta("source_perso", name)) + ": " + reason)
    return false

func _apply_source_configuration() -> bool:
    if _configuration_bound and not is_inside_tree(): return true
    source_configuration_error = ""
    var channels := get_node_or_null("Channels")
    # Synthetic, non-rendered actors used by focused behavior scenes may omit
    # the imported channel hierarchy. There is no native payload to update.
    if channels == null: return true
    var order := PackedInt32Array()
    var restored: bool = restored_configuration
    var identity: String = ""
    if source_configuration != null:
        order = source_configuration.get("payload_order")
        restored = bool(source_configuration.get("restored"))
        identity = str(source_configuration.get("source_id"))
        var seen: Dictionary = {}
        for original_slot in order:
            if original_slot < 0 or original_slot >= order.size() or seen.has(original_slot):
                return _configuration_failure("Selected configuration order is not a source-slot permutation.")
            seen[original_slot] = true
    var prefix: String = "reset_part_" if restored else "part_"
    for channel in channels.get_children():
        var anchors: Array[Node] = []
        # Return warm instances to their hidden native banks before selecting
        # the next permutation. Null selection intentionally leaves slots empty.
        for anchor in channel.get_children():
            if not str(anchor.name).begins_with("part_") and not str(anchor.name).begins_with("reset_part_"): continue
            anchors.append(anchor)
            anchor.process_mode = Node.PROCESS_MODE_DISABLED
            for payload in anchor.get_children():
                if not payload.has_meta("source_configuration_bank"):
                    return _configuration_failure("Selected native payload has no bank binding.")
                var bank_path: NodePath = payload.get_meta("source_configuration_bank")
                var previous := get_node_or_null(bank_path) as CONFIGURED_BANK
                if previous == null:
                    return _configuration_failure("Selected native payload requires its channel's bank.")
                payload.reparent(previous, false)
            anchor.call("refresh_participation")
        if source_configuration == null or anchors.is_empty(): continue
        var bank: CONFIGURED_BANK = null
        for candidate in channel.get_children():
            if str(candidate.get_meta("source_configuration", "")) == identity and bool(candidate.get_meta("source_restored", false)) == restored:
                bank = candidate as CONFIGURED_BANK
                break
        if bank == null:
            return _configuration_failure("Selected configuration needs its converted native bank.")
        var error := bank.validate_configuration(identity, restored, order.size())
        if not error.is_empty(): return _configuration_failure(error)
        # Animation addresses remain stable; only content at sampled anchors is
        # acquired. The immutable catalogue retains every other original slot.
        for anchor in anchors:
            if not str(anchor.name).begins_with(prefix): continue
            var slot: int = int(anchor.get_meta("configuration_slot"))
            if slot < 0: return _configuration_failure("Native animation anchor has an invalid slot.")
            if slot >= order.size(): continue
            var original_slot: int = order[slot]
            if bank.catalog.payload_scenes[original_slot] == null: continue
            var payload := bank.acquire_payload(original_slot, self)
            if payload == null:
                return _configuration_failure("Selected native payload could not be acquired.")
            if not source_rendering.prepare_payload(self, payload):
                return _configuration_failure("Selected native payload cannot inherit actor material state.")
            anchor.call("attach_payload", payload)
    if restored_configuration != restored:
        var time := animation_player.current_animation_position if current_state >= 0 and animation_player.current_animation != "" else 0.0
        var was_playing := playing
        restored_configuration = restored
        if current_state >= 0:
            var clip := StringName(("restored/" if restored else "") + "state_%d" % current_state)
            assert(animation_player.has_animation(clip), "Restored configuration needs native animation library")
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
        if generated.has_method("source_set_object_active_state"):
            generated.call("source_set_object_active_state", true)
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
