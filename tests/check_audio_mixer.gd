extends SceneTree

const Mixer = preload("res://addons/astrolabe/audio/mixer.gd")
const Emitter = preload("res://addons/astrolabe/audio/emitter.gd")
const Voice = preload("res://addons/astrolabe/audio/voice.gd")
const Behavior = preload("res://addons/astrolabe/behavior/behavior.gd")
var starts := 0

func _initialize() -> void:
    call_deferred("check")

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
    return voice

func set_preference(settings, group: String, level: int) -> void:
    # Exercise the maintained settings route without writing any preferences.
    settings.values[group] = float(level)
    settings.call("_apply_group", group)

func captured_rms(capture: AudioEffectCapture) -> float:
    # Discard the previous block so queued mixer output cannot cross a change.
    await create_timer(0.08).timeout
    capture.clear_buffer()
    await create_timer(0.12).timeout
    var frames := capture.get_buffer(capture.get_frames_available())
    if frames.is_empty(): return -1.0
    var energy := 0.0
    for frame in frames: energy += frame.length_squared()
    return sqrt(energy / float(frames.size()))

func check() -> void:
    # Literals cover the source sentinel, fixed128 scaling, signed SAR, AL
    # narrowing and two DWORDs whose product overflows Godot's signed64 range.
    for case in [[127, 127, 127, 127], [100, 127, 127, 100], [127, 127, 90, 88], [100, 64, 127, 49], [127, 127, 0x1000007f, 125], [127, 127, 0xffffffff, 255], [127, 0xffffffff, 0xffffffff, 0]]:
        if not require(Mixer.parameter_volume_byte(case[0], case[1], case[2]) == case[3], "source DWORD/byte contract " + str(case)): return
    var settings = root.get_node("UserSettings")
    set_preference(settings, "Music", 73)
    set_preference(settings, "Effects", 61)
    set_preference(settings, "Voices", 92)
    var brain := Behavior.new()
    brain.automatic = false
    root.add_child(brain)
    var mixer := Mixer.shared(brain) as Mixer
    if not require(mixer.source_volume_word(&"effects") == 127 and mixer.preference_levels[&"effects"] == 61, "source fade must start independently of preference"): return
    var emitter := Emitter.new()
    root.add_child(emitter)
    emitter.voice_started.connect(func(_info: Dictionary): starts += 1)
    var voice := waveform()
    emitter.play_voice(voice, "dynamic", 0.0, true, true)
    emitter.play_voice(voice, "static", 0.0, true, false)
    var dynamic: Dictionary = emitter.voices[0]
    var fixed: Dictionary = emitter.voices[1]
    var effects_bus := AudioServer.get_bus_index(&"Astrolabe Effects")
    brain.sound_fade_sound(0, 0)
    if not require(dynamic.parameter_volume == 0 and fixed.parameter_volume == 61, "source fade must refresh dynamic requests only"): return
    if not require(is_equal_approx(db_to_linear(AudioServer.get_bus_volume_db(effects_bus)), 61.0 / 127.0), "AI fade overwrote preference bus"): return
    set_preference(settings, "Effects", 100)
    if not require(dynamic.parameter_volume == 0 and fixed.parameter_volume == 0, "preference publication must remix current source state for static requests"): return
    brain.sound_fade_sound(127, 0)
    if not require(dynamic.parameter_volume == 100 and fixed.parameter_volume == 0, "source restoration lost dynamic/static distinction"): return
    set_preference(settings, "Effects", 127)
    if not require(fixed.parameter_volume == 127 and dynamic.parameter_volume == 127, "preference publication must recapture all requests"): return
    emitter.source_set_effects_volume_word(90)
    if not require(dynamic.parameter_volume == 88 and fixed.parameter_volume == 127, "object gain must use fixed128 product while static retains capture"): return
    emitter.source_set_effects_volume_word(0xffffffff)
    if not require(dynamic.parameter_volume == 255 and dynamic.object_volume_word == 0xffffffff and db_to_linear(dynamic.player.volume_db) < 2.01, "raw DWORD must narrow after multiplication, not become unbounded amplitude"): return
    emitter.source_set_effects_volume_word(0x1000007f)
    if not require(dynamic.parameter_volume == 125 and emitter.source_effects_volume_override == 0x1000007f, "non-sentinel low-byte127 word lost"): return
    emitter.source_set_effects_volume_word(127)
    # The exact sentinel restores full preference without a127*127 quantization.
    if not require(dynamic.parameter_volume == 127 and emitter.source_effects_volume_override == null, "exact127 sentinel restoration"): return
    brain.sound_fade_all(0, 40)
    await create_timer(0.1).timeout
    if not require(mixer.source_volume_word(&"music") == 0 and mixer.source_volume_word(&"voice") == 0 and mixer.source_volume_word(&"effects") == 0, "independent all-source fade targets"): return
    if not require(settings.get_volume("Music") == 73 and settings.get_volume("Voices") == 92 and settings.get_volume("Effects") == 127, "source fade mutated user settings"): return
    if not require(starts == 2 and dynamic.player.playing and fixed.player.playing, "gain publication restarted/stopped native playback"): return
    brain.sound_fade_all(127, 0)
    emitter.stop_event("static")
    await process_frame
    var capture := AudioEffectCapture.new()
    AudioServer.add_bus_effect(effects_bus, capture)
    var full: float = await captured_rms(capture)
    set_preference(settings, "Effects", 64)
    var quiet: float = await captured_rms(capture)
    if not require(full > 0.01 and quiet > 0.0 and absf(quiet / full - 64.0 / 127.0) < 0.06, "native preference gain is missing/doubled or produced no PCM"): return
    if not require(dynamic.parameter_volume == 64 and starts == 2, "preference change restarted playback or lost published gain"): return
    AudioServer.remove_bus_effect(effects_bus, AudioServer.get_bus_effect_count(effects_bus) - 1)
    emitter.kill_all()
    await process_frame
    emitter.free()
    brain.free()
    print("AUDIO_MIXER_OK ", JSON.stringify({"starts": starts, "full_rms": full, "quiet_rms": quiet, "ratio": quiet / full}))
    quit(0)
