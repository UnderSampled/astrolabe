extends Node3D
## Engine sound semantics over converted waveforms. Resource selection is compiled code.
signal voice_started(info: Dictionary)
@export var bus: StringName = &"Astrolabe Effects"
@export var voice_bus: StringName = &"Astrolabe Effects"
@export var enabled: bool = true
var selectors: Dictionary = {}
var voices: Array[Dictionary] = []
var previous_choices: Dictionary = {}
var random := RandomNumberGenerator.new()
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
        release_voice(info)
    set_object_volume(127.0)

func request_music(event: Resource) -> bool:
    music_failure = ""
    if event != null: event.execute(self)
    return music_failure.is_empty()

func set_object_volume(value: float) -> void:
    object_volume = clampf(value, 0.0, 127.0)
    for info in voices:
        if is_instance_valid(info.player):
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
    player.bus = voice_bus
    player.stream = voice.stream
    player.volume_db = linear_to_db(maxf(voice.gain * object_volume / 127.0, 0.000001))
    player.pitch_scale = maxf(1.0 + random.randf_range(-pitch_variation, pitch_variation), 0.001) if voice.pitchable else 1.0
    add_child(player)
    if player is Node3D and not dynamic:
        player.top_level = true
        player.global_transform = global_transform
    var info := {"event": identity, "stream": voice.stream.resource_path, "pitch": player.pitch_scale, "gain": voice.gain, "choices": voice.trace, "spatial": spatial, "stoppable": stoppable, "player": player, "theme": voice.theme_identity}
    voices.append(info)
    player.finished.connect(func(): release_voice(info))
    player.play()
    voice_started.emit(info)

func stop_event(identity: Variant = null) -> void:
    for voice in voices.duplicate():
        if identity == null or voice.event == identity:
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
