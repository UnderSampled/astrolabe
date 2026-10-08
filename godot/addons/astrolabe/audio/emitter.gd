extends Node3D
## Engine sound semantics over converted waveforms. Resource selection is compiled code.
signal voice_started(info: Dictionary)
signal voice_completed(info: Dictionary)
@export var bus: StringName = &"Astrolabe Effects"
@export var voice_bus: StringName = &"Astrolabe Effects"
@export var enabled: bool = true
var selectors: Dictionary = {}
var voices: Array[Dictionary] = []
var previous_choices: Dictionary = {}
var random := RandomNumberGenerator.new()
## Dialogue owns this record independently of ordinary effects/music voices.
var source_dialogue_record: Resource
var _dialogue_speech_request: bool = false
## Populated by source conversion. Unknown authored Micro data stays unbound.
@export var microphone_basis: Basis = Basis.IDENTITY
@export var microphone_basis_known: bool = false
var microphone_position_actor: Node3D
var microphone_orientation_actor: Node3D
var microphone_orientation_basis: Variant = null
var microphone_pose_known: bool = false
var is_microphone: bool = false

## Native APIs consume already converted Micro basis; no source tables are read.
static func microphone_basis_for(actor: Node3D) -> Variant:
    if not is_instance_valid(actor): return null
    var component := actor.get_node_or_null("SoundEmitter")
    if component == null or not bool(component.get("microphone_basis_known")): return null
    return component.get("microphone_basis")

static func microphone_listener(requester: Node) -> AudioListener3D:
    if not is_instance_valid(requester) or not requester.is_inside_tree(): return null
    var listener = requester.get_tree().root.get_node_or_null("AstrolabeListener")
    if listener == null:
        listener = AudioListener3D.new()
        listener.name = "AstrolabeListener"
        listener.set_script(load("res://addons/astrolabe/audio/emitter.gd"))
        listener.is_microphone = true
        requester.get_tree().root.add_child(listener)
        listener.clear_current()
    if not listener is AudioListener3D or not listener.is_microphone: return null
    return listener

static func switch_micro_perso(requester: Node, selected: Node3D, default_main: Node3D, local_basis: Variant) -> bool:
    var actor := selected if selected != null else default_main
    if not is_instance_valid(actor) or not actor.is_inside_tree(): return false
    if local_basis != null and (not local_basis is Basis or not local_basis.is_finite()): return false
    var listener = microphone_listener(requester)
    if listener == null: return false
    listener.microphone_position_actor = actor
    listener.microphone_orientation_actor = actor
    listener.microphone_orientation_basis = local_basis
    # The source setter accepts an actor with null Micro. Record that selection
    # without inventing an orientation; publication for this case is unproven.
    if listener.update_microphone_pose(): listener.make_current()
    return true

## Proc404 changes the orientation source while keeping the position source.
static func switch_old_micro_perso(requester: Node, actor: Node3D, local_basis: Variant) -> bool:
    if not is_instance_valid(actor) or not actor.is_inside_tree(): return false
    if local_basis != null and (not local_basis is Basis or not local_basis.is_finite()): return false
    var listener = microphone_listener(requester)
    if listener == null: return false
    listener.microphone_orientation_actor = actor
    listener.microphone_orientation_basis = local_basis
    if listener.update_microphone_pose(): listener.make_current()
    return true

func update_microphone_pose() -> bool:
    microphone_pose_known = false
    if not is_instance_valid(microphone_position_actor) or not is_instance_valid(microphone_orientation_actor):
        if is_microphone: call("clear_current")
        return false
    if microphone_orientation_basis == null: return false
    global_transform = Transform3D(microphone_orientation_actor.global_basis * microphone_orientation_basis, microphone_position_actor.global_position)
    microphone_pose_known = true
    return true

func _physics_process(_delta: float) -> void:
    if is_microphone: update_microphone_pose()

## Per-object effects volume; source413 applies it before starting its event.
var object_volume: float = 127.0
var volume_fades: Array[Tween] = []
var is_music: bool = false
var music_failure: String = ""

## Music uses the source's shared object-less channel and survives its Brain.
static func music_channel(requester: Node) -> Node3D:
    var owner_node := requester.get_tree().root
    var channel := owner_node.get_node_or_null("AstrolabeMusic") as Node3D
    if channel != null: return channel
    channel = Node3D.new()
    channel.name = "AstrolabeMusic"
    channel.set_script(load("res://addons/astrolabe/audio/emitter.gd"))
    channel.voice_bus = &"Astrolabe Music"
    channel.is_music = true
    var bus_index := AudioServer.get_bus_index(channel.voice_bus)
    if bus_index < 0:
        AudioServer.add_bus()
        AudioServer.set_bus_name(AudioServer.bus_count - 1, channel.voice_bus)
    owner_node.add_child(channel)
    return channel

func kill_all() -> void:
    for info in voices.duplicate():
        info.player.stop()
        # A bulk request kill does not synchronously clear the owner holder.
        # Hype's next speech-animation update observes request completion. Use
        # the same guarded completion path on the next native frame, even if
        # the actor's processing has been disabled by a terminal session route.
        if not bool(info.get("completion_pending", false)):
            info.completion_pending = true
            get_tree().process_frame.connect(complete_voice.bind(info), CONNECT_ONE_SHOT)
    set_object_volume(127.0)

func request_music(event: Resource) -> bool:
    music_failure = ""
    if event != null: event.execute(self)
    return music_failure.is_empty()

## 0041e8a0 dispatches on the voice module, independently of this actor's
## effects volume. The dialogue owns the useful record, not a request integer.
func play_dialogue_speech(record: Resource) -> void:
    source_dialogue_record = record
    var voice: Resource = record.get("voice") as Resource
    if voice == null: return
    var bus_index := AudioServer.get_bus_index(&"Astrolabe Voice")
    if bus_index < 0:
        AudioServer.add_bus()
        AudioServer.set_bus_name(AudioServer.bus_count - 1, &"Astrolabe Voice")
    var previous_request := _dialogue_speech_request
    _dialogue_speech_request = true
    voice.execute(self)
    _dialogue_speech_request = previous_request

## 0041e910 clears the speech holder even when no stop is needed. Ordinary
## effects and the shared music channel do not own this record.
func stop_dialogue_speech() -> void:
    var record := source_dialogue_record
    source_dialogue_record = null
    if record == null: return
    # Source kills this object's voice module; a matching event on the effects
    # module must survive, even when both requests use the same resource.
    for info in voices.duplicate():
        if not bool(info.get("uses_object_volume", true)): _stop_voice_info(info)

func set_object_volume(value: float) -> void:
    object_volume = clampf(value, 0.0, 127.0)
    for info in voices:
        if bool(info.get("uses_object_volume", true)) and is_instance_valid(info.player):
            info.player.volume_db = linear_to_db(maxf(info.gain * object_volume / 127.0, 0.000001))

## Proc413 is nonblocking: silence this object's effects, send the event, then
## fade its effects volume. Null events still update the object's volume.
func fade_in_event(event: Resource, target: int, duration_ms: int) -> void:
    set_object_volume(0.0)
    if event != null: event.execute(self)
    var level := float(clampi(target, 0, 127))
    if duration_ms <= 0:
        set_object_volume(level)
        return
    var tween := get_tree().create_tween().bind_node(self)
    volume_fades.append(tween)
    tween.tween_method(set_object_volume, 0.0, level, float(duration_ms) / 1000.0).set_trans(Tween.TRANS_LINEAR)
    tween.finished.connect(func(): volume_fades.erase(tween))

func set_selector(owner: String, selector: String, value: String) -> void:
    selectors[owner + "#" + selector] = value

# Weighted, optionally non-repeating choice is engine behavior, not asset decoding.
func choose_weighted(key: String, weights: PackedFloat32Array, repeats: PackedByteArray, silence: float, repeat_silence: bool) -> int:
    var previous := int(previous_choices.get(key, -2))
    var total := 0.0
    for i in weights.size():
        if i != previous or repeats[i] != 0: total += maxf(weights[i], 0.0)
    var silence_weight := maxf(silence, 0.0) if previous != -1 or repeat_silence else 0.0
    total += silence_weight
    if total <= 0.0: return -1
    var draw := random.randf() * total
    var selected := -1
    for i in weights.size():
        if i == previous and repeats[i] == 0: continue
        draw -= maxf(weights[i], 0.0)
        if draw <= 0.0:
            selected = i
            break
    previous_choices[key] = selected
    return selected

func play_voice(voice: Resource, identity: String, pitch_variation: float, stoppable: bool, dynamic: bool) -> void:
    if not enabled or voice == null: return
    if is_music and not voice.theme_identity.is_empty():
        for info in voices:
            if bool(info.get("completion_pending", false)): continue
            if info.get("theme", "").is_empty(): continue
            if info.theme == voice.theme_identity:
                # 004faa60 retains the active same-theme playback. Its newly
                # published request replaces the previous event association.
                info.event = identity
                info.stoppable = stoppable
                return
            music_failure = "Switching active music themes requires the source audio-driver transition contract"
            return
    var spatial: bool = voice.spatial and not is_music
    var player: Node = AudioStreamPlayer3D.new() if spatial else AudioStreamPlayer.new()
    player.bus = &"Astrolabe Voice" if _dialogue_speech_request else voice_bus
    player.stream = voice.stream
    var object_gain := 1.0 if _dialogue_speech_request else object_volume / 127.0
    player.volume_db = linear_to_db(maxf(voice.gain * object_gain, 0.000001))
    player.pitch_scale = maxf(1.0 + random.randf_range(-pitch_variation, pitch_variation), 0.001) if voice.pitchable else 1.0
    add_child(player)
    if player is Node3D and not dynamic:
        player.top_level = true
        player.global_transform = global_transform
    var info := {"event": identity, "stream": voice.stream.resource_path, "pitch": player.pitch_scale, "gain": voice.gain, "choices": voice.trace, "spatial": spatial, "stoppable": stoppable, "player": player, "theme": voice.theme_identity, "uses_object_volume": not _dialogue_speech_request}
    voices.append(info)
    player.finished.connect(complete_voice.bind(info))
    player.play()
    voice_started.emit(info)

func stop_event(identity: Variant = null) -> void:
    for voice in voices.duplicate():
        if identity == null or voice.event == identity:
            _stop_voice_info(voice)

func _stop_voice_info(voice: Dictionary) -> void:
    if voice.stoppable:
        voice.player.stop()
        release_voice(voice)
    else:
        voices.erase(voice)
        if voice.player is Node3D:
            var placement: Transform3D = voice.player.global_transform
            voice.player.top_level = true
            voice.player.global_transform = placement

func release_voice(info: Dictionary) -> void:
    voices.erase(info)
    if is_instance_valid(info.player): info.player.queue_free()

func complete_voice(info: Dictionary) -> void:
    if bool(info.get("completed", false)): return
    info.completed = true
    release_voice(info)
    voice_completed.emit(info)
