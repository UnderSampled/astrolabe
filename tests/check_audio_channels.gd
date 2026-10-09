extends SceneTree

const Emitter = preload("res://addons/astrolabe/audio/emitter.gd")
const Voice = preload("res://addons/astrolabe/audio/voice.gd")
const Dialogue = preload("res://addons/astrolabe/ui/dialogue.tscn")

class Event extends Resource:
    var identity := "sound:/shared.csb#0"
    var native_voice: Resource
    var stoppable := true
    var dynamic := true
    var stop := false

    func execute(emitter: Node3D) -> void:
        if stop: emitter.stop_event(identity)
        else: emitter.play_voice(native_voice, identity, 0.0, stoppable, dynamic)

class Speech extends Resource:
    var voice: Resource

var completions: Array[Dictionary] = []
var starts: Array[Dictionary] = []

func _initialize() -> void:
    call_deferred("check")

func require(ok: bool, why: String) -> bool:
    if not ok:
        push_error(why)
        quit(1)
    return ok

func waveform(looped: bool) -> Resource:
    var stream := AudioStreamWAV.new()
    stream.format = AudioStreamWAV.FORMAT_16_BITS
    stream.mix_rate = 8000
    var data := PackedByteArray()
    data.resize(4800) # 300ms of mono PCM.
    for frame in 2400:
        data.encode_s16(frame * 2, roundi(sin(TAU * frame * 440.0 / 8000.0) * 8000.0))
    stream.data = data
    if looped:
        stream.loop_mode = AudioStreamWAV.LOOP_FORWARD
        stream.loop_end = 2400
    var voice := Voice.new()
    voice.stream = stream
    voice.spatial = true
    return voice

func begin_speech(dialogue: Node, emitter: Node3D, event: Resource) -> Resource:
    var speech := Speech.new()
    speech.voice = event
    dialogue.voice_emitter = emitter
    dialogue.active_speech_record = speech
    dialogue.active_voice_identity = event.identity
    dialogue.call("_begin_voice")
    return speech

func check() -> void:
    var emitter = Emitter.new()
    root.add_child(emitter)
    emitter.voice_started.connect(func(info: Dictionary): starts.append(info))
    emitter.voice_completed.connect(func(info: Dictionary): completions.append(info))
    var dialogue: Node = Dialogue.instantiate()
    root.add_child(dialogue)
    var play := Event.new()
    play.native_voice = waveform(true)
    play.execute(emitter)
    var effects: Dictionary = emitter.voices[0]
    var speech := begin_speech(dialogue, emitter, play)
    var spoken: Dictionary = emitter.voices[1]
    if not require(starts.size() == 2 and effects.module == &"effects" and spoken.module == &"voice", "shared event must start independent source modules"): return
    if not require(effects.player.bus == &"Astrolabe Effects" and spoken.player.bus == &"Astrolabe Voice", "effects/speech mixer routing"): return

    emitter.source_set_effects_volume_word(0)
    if not require(emitter.source_effects_volume_override == 0 and effects.player.volume_db < -70.0 and spoken.player.volume_db > -0.1, "Proc386 must retain zero override and leave voice gain alone"): return
    emitter.source_set_effects_volume_word(127)
    if not require(emitter.source_effects_volume_override == null, "only exact word127 removes the override"): return

    # An ordinary STOP removes effects only, even with identical event identity.
    emitter.stop_event(play.identity)
    if not require(emitter.voices.size() == 1 and emitter.voices[0] == spoken and spoken.player.playing, "effects STOP crossed into voice module"): return
    if not require(completions.is_empty() and emitter.source_dialogue_record == speech, "STOP must publish completion on the next frame"): return
    await process_frame
    if not require(completions.size() == 1 and completions[0].module == &"effects" and emitter.source_dialogue_record == speech, "effects completion must preserve speech holder"): return

    # Dispatch STOP on the voice module through the actual speech API. The UI
    # owns its existing record and must observe completion without cancel().
    var stop := Event.new()
    stop.stop = true
    emitter.execute_voice_event(stop)
    if not require(emitter.voices.is_empty() and completions.size() == 1, "voice STOP removal/deferred completion"): return
    await process_frame
    if not require(completions.size() == 2 and emitter.source_dialogue_record == null and not dialogue.voice_completion.is_valid(), "stopped speech did not release actual UI holder"): return

    emitter.source_set_effects_volume_word(0x1000007f)
    if not require(emitter.source_effects_volume_override == 0x1000007f, "Proc386 raw word must not narrow to sentinel127"): return
    emitter.source_set_effects_volume_word(127)
    play.dynamic = false
    play.execute(emitter)
    var static_player: Node = emitter.voices[0].player
    emitter.source_set_effects_volume_word(0)
    if not require(static_player.volume_db > -0.1, "nondynamic request must retain its captured gain"): return
    emitter.source_set_effects_volume_word(127)
    emitter.stop_event(play.identity)
    await process_frame
    if not require(completions.size() == 3, "static effects STOP completion"): return
    play.dynamic = true

    # A nonabortable request completes after removal while native playback
    # continues at its captured pose and releases itself on natural finish.
    play.native_voice = waveform(false)
    play.stoppable = false
    play.execute(emitter)
    var detached: Dictionary = emitter.voices[0]
    var player: AudioStreamPlayer3D = detached.player
    var placement := player.global_transform
    emitter.stop_event(play.identity)
    await process_frame
    if not require(completions.size() == 4 and player.playing and player.top_level, "removed nonabortable request must complete while playback continues"): return
    emitter.position = Vector3(9, 8, 7)
    if not require(player.global_transform.is_equal_approx(placement), "detached voice moved with emitter"): return
    await create_timer(0.45).timeout
    if not require(not is_instance_valid(player) and completions.size() == 4, "natural finish leaked detached playback or completed twice"): return

    play.native_voice = waveform(true)
    play.stoppable = true
    play.execute(emitter)
    var retained_speech := begin_speech(dialogue, emitter, play)
    var retained_player: Node = dialogue.voice_player
    if not require(emitter.source_fade_effects_and_stop(0, 20), "Proc412 fade admission"): return
    await create_timer(0.12).timeout
    if not require(emitter.voices.size() == 1 and retained_player.playing and emitter.source_dialogue_record == retained_speech, "Proc412 killed speech or left effects active"): return
    if not require(emitter.source_effects_volume_override == null and completions.size() == 5 and completions[-1].completion_reason == &"effects_fade", "Proc412 reset/completion"): return
    emitter.kill_all()
    if not require(emitter.source_dialogue_record == retained_speech, "bulk kill cleared speech holder synchronously"): return
    await process_frame
    if not require(emitter.source_dialogue_record == null and completions.size() == 6 and starts.size() == 6, "bulk kill holder completion or incorrect start count"): return
    dialogue.free()
    emitter.free()
    await create_timer(0.1).timeout
    print("AUDIO_CHANNELS_OK ", JSON.stringify({"starts": starts.size(), "completions": completions.size(), "detached_completions": 1}))
    quit(0)
