extends CharacterBody3D
## Native character controls. Every part and pose already exists in the PackedScene.
## No frame tables, transform pools or object configurations are consumed here.
signal cue_triggered(action: Resource, frame: int, channel: int, configuration_slot: int)
signal action_finished(action: int)
@export var initial_state: int = -1
@export var current_state: int = -1
@export var current_mechanics: Resource
@export var physical_environment: Resource
@export var autoplay: bool = true
## Gameplay uses physics ticks; ordinary editor/pose inspection stays stationary.
@export var motion_enabled: bool = false
## World-space acceleration supplied by the level/game's environment.
@export var gravity: Vector3 = Vector3.ZERO
## Velocity from forces/impulses, separate from the next animation displacement.
var external_velocity: Vector3 = Vector3.ZERO
## AI movement requests are applied once when a jump procedure commits them.
var source_vector_speed: Vector3 = Vector3.ZERO
var source_impulse: Vector3 = Vector3.ZERO
var source_relative_motion: bool = false
var source_airborne: bool = false
var source_particle_generator_enabled: bool = false
var source_generated_object_count: int = 0
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
    # Selection only; the saved initial pose is usable without this callback.
    if autoplay and initial_state >= 0:
        select_state(initial_state)

func _process(delta: float) -> void:
    if not motion_enabled: advance(delta)

func _physics_process(delta: float) -> void:
    if not motion_enabled: return
    advance(delta)
    # The animation never changes this body's position. Apply its local motion
    # once, through native contact resolution. Forces/impulses persist independently.
    var motion := global_basis.orthonormalized() * root_motion_displacement / delta
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
    if is_on_floor() and external_velocity.dot(up_direction) < 0.0:
        external_velocity -= up_direction * external_velocity.dot(up_direction)
    if is_on_floor(): source_airborne = false
    elif external_velocity.dot(up_direction) > 0.0: source_airborne = true

func turn(degrees_per_second: float, seconds: float) -> void:
    # Proc_Turn requests a rate about the actor's local up axis. Source +Z maps
    # to Godot +Y; the native transform composes the rotation around that axis.
    assert(is_finite(degrees_per_second) and is_finite(seconds) and seconds >= 0.0)
    rotate_object_local(Vector3.UP, deg_to_rad(degrees_per_second) * seconds)

func _source_motion_vector(value: Vector3) -> Vector3:
    var converted := Vector3(value.x, value.z, -value.y)
    return global_basis.orthonormalized() * converted if source_relative_motion else converted

func source_set_vector_speed(value: Vector3) -> void:
    assert(value.is_finite())
    source_vector_speed = value

func source_go_relative() -> void:
    source_relative_motion = true

func source_set_impulse(value: Vector3) -> void:
    assert(value.is_finite())
    source_impulse = value

func source_jump_absolute() -> void:
    external_velocity = _source_motion_vector(source_vector_speed + source_impulse)
    source_impulse = Vector3.ZERO
    source_relative_motion = false
    source_airborne = true

func source_jump_without_adding_speed() -> void:
    external_velocity += _source_motion_vector(source_impulse)
    source_impulse = Vector3.ZERO
    source_relative_motion = false
    source_airborne = true

func source_position() -> Vector3:
    return Vector3(global_position.x, -global_position.z, global_position.y)

func source_game_time() -> int:
    var brain := get_node_or_null("Brain")
    assert(brain != null)
    return int(brain.get("clock_milliseconds"))

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

## Execute an action and instantiate its converted Perso template once. A
## nonzero source vector still needs its placement/velocity contract established.
## Return -1 for an unsupported request, 0 for a rejected action, 1 when started.
func source_execute_action_and_generate_object(template_path: String, action: int, vector: Vector3) -> int:
    if template_path.is_empty():
        push_error("Action/object request has no converted Perso template path")
        return -1
    var template := load(template_path) as PackedScene
    if template == null:
        push_error("Action/object request has no converted Perso template")
        return -1
    if vector != Vector3.ZERO:
        push_error("Action/object request with a nonzero vector is unsupported")
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
    if not request_action(action, true):
        generated.free()
        return 0
    destination.add_child(generated)
    # Placement is provisional until the source object's initial transform is
    # observed. The saved template itself remains reusable and unplaced.
    generated.global_transform = global_transform
    generated.set_meta("astrolabe_generated_template", template_path)
    source_generated_object_count += 1
    return 1

## Func_GetPersoSighting reads the current actor's world-space negative
## source-Y basis column. The exported Basis stores that direction in +Z;
## AI vectors remain in source coordinates until an engine operation uses them.
func source_sighting() -> Vector3:
    var direction := global_basis.z
    return Vector3(direction.x, -direction.z, direction.y)

func source_collide_with_ground() -> bool:
    return is_on_floor()

func source_collide_with_slope() -> bool:
    for index in get_slide_collision_count():
        var normal := get_slide_collision(index).get_normal()
        if normal.y > 0.0 and normal.y < 0.999:
            return true
    return false

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
    if not animation_player.has_animation(StringName("state_%d" % id)): return false
    if not restart and action_controlled and requested_action_state == id: return true
    var selected := _transition_target(pending_state if pending_state >= 0 else current_state, id)
    if selected < 0: return false
    requested_action_state = id
    animation_complete = false
    _play_action_state(selected)
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
    # Generated subclasses activate the already saved shapes for this action.
    pass

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

func _cue(action: Resource, frame: int, channel: int, slot: int) -> void:
    if action.sound != null:
        action.sound.execute(get_node("SoundEmitter"))
    cue_triggered.emit(action, frame, channel, slot)

func _cues(frame: int, channels: PackedInt32Array, slots: PackedInt32Array, actions: Array) -> void:
    if suppress_cues: return
    var revision := playback_revision
    for i in actions.size():
        _cue(actions[i], frame, channels[i], slots[i])
        if revision != playback_revision: return
