extends CharacterBody3D
## Native character controls. Every part and pose already exists in the PackedScene.
## No frame tables, transform pools or object configurations are consumed here.
signal cue_triggered(action: Resource, frame: int, channel: int, configuration_slot: int)
signal action_finished(action: int)
@export var initial_state: int = -1
@export var current_state: int = -1
@export var current_mechanics: Resource
@export var autoplay: bool = true
## Gameplay uses physics ticks; ordinary editor/pose inspection stays stationary.
@export var motion_enabled: bool = false
## World-space acceleration supplied by the level/game's environment.
@export var gravity: Vector3 = Vector3.ZERO
## Velocity from forces/impulses, separate from the next animation displacement.
var external_velocity: Vector3 = Vector3.ZERO
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
    external_velocity += gravity * delta
    velocity = motion + external_velocity
    move_and_slide()
    for i in get_slide_collision_count():
        var normal := get_slide_collision(i).get_normal()
        if external_velocity.dot(normal) < 0.0:
            external_velocity = external_velocity.slide(normal)

func turn(degrees_per_second: float, seconds: float) -> void:
    # Proc_Turn requests a rate about the actor's local up axis. Source +Z maps
    # to Godot +Y; the native transform composes the rotation around that axis.
    assert(is_finite(degrees_per_second) and is_finite(seconds) and seconds >= 0.0)
    rotate_object_local(Vector3.UP, deg_to_rad(degrees_per_second) * seconds)

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
