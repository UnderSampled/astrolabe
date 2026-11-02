extends SceneTree

const Emitter = preload("res://addons/astrolabe/audio/emitter.gd")
const Voice = preload("res://addons/astrolabe/audio/voice.gd")
const Mixer = preload("res://addons/astrolabe/audio/mixer.gd")
const SpatialGain = preload("res://addons/astrolabe/audio/spatial_gain.gd")
var starts := 0

func _initialize() -> void: call_deferred("check")

func require(ok: bool, why: String) -> bool:
    if not ok:
        push_error(why)
        quit(1)
    return ok

func waveform() -> Resource:
    var stream := AudioStreamWAV.new()
    stream.format = AudioStreamWAV.FORMAT_16_BITS
    stream.mix_rate = 8000
    stream.loop_mode = AudioStreamWAV.LOOP_FORWARD
    stream.loop_end = 8000
    var data := PackedByteArray()
    data.resize(16000)
    for frame in 8000:
        data.encode_s16(frame * 2, roundi(sin(TAU * frame * 440.0 / 8000.0) * 8000.0))
    stream.data = data
    var voice := Voice.new()
    voice.stream = stream
    # Source object parameters, even with resource SetSpacable false.
    voice.spatial = false
    voice.source_parameter_conversion = true
    return voice

func captured_rms(capture: AudioEffectCapture) -> float:
    await create_timer(0.08).timeout
    capture.clear_buffer()
    await create_timer(0.12).timeout
    var frames := capture.get_buffer(capture.get_frames_available())
    if frames.is_empty(): return -1.0
    var energy := 0.0
    for frame in frames: energy += frame.length_squared()
    return sqrt(energy / float(frames.size()))

func check() -> void:
    for case in [[0, 127], [1, 127], [2, 127], [8, 72], [9, 63], [16, 0], [17, 0]]:
        if not require(SpatialGain.distance_volume(case[0] * 65536) == case[1], "fixed source rolloff " + str(case)): return
    if not require(SpatialGain.distance_word(Vector3(2, 4, -2), Vector3.ZERO) == 360448, "source XY grouping is not Euclidean"): return
    if not require(SpatialGain.distance_word(Vector3(2, 2, -4), Vector3.ZERO) == 393216, "source axis permutation changed pseudo norm"): return
    if not require(SpatialGain.converted_volume(127, 127) == 126 and SpatialGain.converted_volume(127, 63) == 62, "driver mixer-byte product must shift7"): return
    # Godot mixes a 3D player only for viewports with a Camera3D; an AudioListener3D alone is silent.
    var camera := Camera3D.new()
    root.add_child(camera)
    camera.make_current()
    var actor := Node3D.new()
    root.add_child(actor)
    var emitter := Emitter.new()
    emitter.name = "SoundEmitter"
    actor.add_child(emitter)
    emitter.voice_started.connect(func(_info: Dictionary): starts += 1)
    var mixer := Mixer.shared(emitter) as Mixer
    mixer.set_preference(&"effects", 127)
    var voice := waveform()
    emitter.play_voice(voice, "dynamic", 0.0, true, true)
    emitter.play_voice(voice, "static", 0.0, true, false)
    var dynamic: Dictionary = emitter.voices[0]
    var fixed: Dictionary = emitter.voices[1]
    if not require(dynamic.player is AudioStreamPlayer3D and dynamic.source_spatial_state == &"unbound_microphone" and dynamic.converted_volume == 0, "missing source Micro must not become camera listener"): return
    if not require(dynamic.player.attenuation_model == AudioStreamPlayer3D.ATTENUATION_DISABLED and dynamic.player.attenuation_filter_cutoff_hz == 20500.0, "native distance/filter would double source attenuation"): return
    var selected := Node3D.new()
    root.add_child(selected)
    if not require(Emitter.switch_micro_perso(emitter, selected, selected, Basis.IDENTITY), "native Micro selection"): return
    await physics_frame
    await physics_frame
    if not require(dynamic.distance_volume == 127 and dynamic.converted_volume == 126 and fixed.converted_volume == 0, "Micro selection refreshed static request or omitted dynamic publication"): return
    mixer.set_preference(&"effects", 127)
    if not require(fixed.converted_volume == 126, "preference change must recapture static distance"): return
    selected.position = Vector3(0, 0, 9)
    await physics_frame
    await physics_frame
    if not require(dynamic.distance_word == 9 * 65536 and dynamic.converted_volume == 62 and fixed.converted_volume == 126, "listener movement must preserve static compact gain"): return
    mixer.set_preference(&"effects", 127)
    if not require(fixed.converted_volume == 62 and starts == 2, "preference distance recapture restarted playback"): return
    emitter.stop_event("static")
    await process_frame
    var capture := AudioEffectCapture.new()
    var capture_bus := AudioServer.get_bus_index(&"Master")
    AudioServer.add_bus_effect(capture_bus, capture)
    selected.position = Vector3(0, 0, 1)
    var near: float = await captured_rms(capture)
    selected.position = Vector3(0, 0, 9)
    var middle: float = await captured_rms(capture)
    selected.position = Vector3(0, 0, 17)
    var far: float = await captured_rms(capture)
    if not require(near > 0.01 and absf(middle / near - 62.0 / 126.0) < 0.06 and far < near * 0.001, "native PCM distance curve is missing/doubled or never reaches zero"): return
    if not require(starts == 2 and dynamic.player.playing, "distance publication restarted or stopped loop"): return
    AudioServer.remove_bus_effect(capture_bus, AudioServer.get_bus_effect_count(capture_bus) - 1)
    emitter.kill_all()
    await process_frame
    actor.free()
    selected.free()
    print("AUDIO_ATTENUATION_OK ", JSON.stringify({"starts": starts, "near_rms": near, "middle_rms": middle, "far_rms": far}))
    quit(0)
