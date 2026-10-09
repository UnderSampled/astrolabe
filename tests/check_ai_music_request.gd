extends SceneTree

const Emitter = preload("res://addons/astrolabe/audio/emitter.gd")
const Mixer = preload("res://addons/astrolabe/audio/mixer.gd")

func _initialize() -> void:
    call_deferred("check")

func require(ok: bool, why: String) -> bool:
    if not ok:
        push_error(why)
        quit(1)
    return ok

func check() -> void:
    var actor := Node3D.new()
    var effects = Emitter.new()
    effects.name = "SoundEmitter"
    effects.voice_bus = &"MusicCheckEffects"
    actor.add_child(effects)
    root.add_child(actor)
    var caller := Node3D.new()
    var caller_emitter = Emitter.new()
    caller_emitter.name = "SoundEmitter"
    caller.add_child(caller_emitter)
    var brain = load("res://generated/hype/levels/music-test/ai/music.gd").new()
    brain.automatic = false
    brain.current_normal = 0
    caller.add_child(brain)
    root.add_child(caller)
    brain.main_actor = actor
    brain.sound_emitter = caller_emitter
    if not require(Emitter.switch_micro_perso(effects, actor, actor, Basis.IDENTITY), "source effects fixture requires a selected Micro"): return
    var mixer := Mixer.shared(brain) as Mixer
    mixer.set_preference(&"effects", 127)
    mixer.set_preference(&"music", 127)
    for bus_name in [&"Astrolabe Music", &"MusicCheckEffects"]:
        if AudioServer.get_bus_index(bus_name) < 0:
            AudioServer.add_bus()
            AudioServer.set_bus_name(AudioServer.bus_count - 1, bus_name)
    var music_bus := AudioServer.get_bus_index(&"Astrolabe Music")
    var capture := AudioEffectCapture.new()
    AudioServer.add_bus_effect(music_bus, capture)
    if not require(brain.start() and brain.unsupported_failure.is_empty(), "music request rejected"): return
    if not require(brain.dsg_2 == 1, "sound procedure blocked subsequent AI"): return
    var music = root.get_node("AstrolabeMusic")
    if not require(music.voices.size() == 1 and effects.voices.size() == 2, "independent music and receiver requests"): return
    var player = music.voices[0].player
    if not require(player is AudioStreamPlayer and player.bus == &"Astrolabe Music" and player.playing, "music must use shared nonspatial channel"): return
    if not require(player.stream is AudioStreamInteractive and player.stream.clip_count == 3, "native ordered theme resource"): return
    if not require(player.stream.get_clip_auto_advance_next_clip(2) == 0, "authored theme loop entry"): return
    var quiet_part = player.stream.get_clip_stream(1)
    if not require(quiet_part is AudioStreamSynchronized and is_equal_approx(db_to_linear(quiet_part.get_sync_stream_volume(0)), 48.0 / 127.0), "part resource gain lost"): return
    if not require(is_equal_approx(effects.object_volume, 0.0) and is_equal_approx(caller_emitter.object_volume, 127.0), "Ultra must select receiver before original-caller operands"): return
    var event = load("res://generated/hype/sounds/Sound/music.csb.events/0.tres")
    brain.send_music(event)
    brain.send_music(null)
    if not require(music.voices.size() == 1 and music.voices[0].player == player, "same theme or null request restarted playback"): return
    caller.queue_free()
    await create_timer(0.15).timeout
    if not require(player.playing and is_equal_approx(effects.object_volume, 90.0), "request lifetime or receiver fade completion"): return
    if not require(effects.voices[0].player.playing and effects.voices[0].parameter_volume == 88 and effects.voices[0].converted_volume == 87 and is_equal_approx(db_to_linear(effects.voices[0].player.volume_db), 87.0 / 127.0), "fade completion must preserve effects playback with source byte gain"): return
    var visited: Array[int] = []
    for i in 100:
        var clip: int = player.get_stream_playback().get_current_clip_index()
        if visited.is_empty() or visited[-1] != clip: visited.append(clip)
        if visited.size() >= 4: break
        await create_timer(0.02).timeout
    if not require(visited.size() >= 4 and visited.has(0) and visited.has(1) and visited.has(2), "theme natural completion/loop did not advance"): return
    var frames := capture.get_buffer(capture.get_frames_available())
    var peak := 0.0
    for frame in frames: peak = maxf(peak, maxf(absf(frame.x), absf(frame.y)))
    if not require(frames.size() > 0 and peak > 0.01, "music produced no native mixer output"): return
    music.kill_all()
    await process_frame
    if not require(music.voices.is_empty() and effects.voices[0].player.playing, "music stop affected object effects"): return
    effects.kill_all()
    actor.free()
    music.free()
    await create_timer(0.15).timeout
    for bus_name in [&"MusicCheckEffects", &"Astrolabe Music"]:
        AudioServer.remove_bus(AudioServer.get_bus_index(bus_name))
    print("AI_MUSIC_REQUEST_OK ", JSON.stringify({"clips": visited, "frames": frames.size(), "peak": peak}))
    quit(0)
