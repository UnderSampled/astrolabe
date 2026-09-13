extends Node3D
## Engine sound semantics over converted waveforms. Resource selection is compiled code.
signal voice_started(info: Dictionary)
@export var bus: StringName = &"Master"
@export var enabled: bool = true
var selectors: Dictionary = {}
var voices: Array[Dictionary] = []
var previous_choices: Dictionary = {}
var random := RandomNumberGenerator.new()

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
    var player: Node = AudioStreamPlayer3D.new() if voice.spatial else AudioStreamPlayer.new()
    player.bus = bus
    player.stream = voice.stream
    player.volume_db = linear_to_db(maxf(voice.gain, 0.000001))
    player.pitch_scale = maxf(1.0 + random.randf_range(-pitch_variation, pitch_variation), 0.001) if voice.pitchable else 1.0
    add_child(player)
    if player is Node3D and not dynamic:
        player.top_level = true
        player.global_transform = global_transform
    var info := {"event": identity, "stream": voice.stream.resource_path, "pitch": player.pitch_scale, "gain": voice.gain, "choices": voice.trace, "spatial": voice.spatial, "stoppable": stoppable, "player": player}
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
