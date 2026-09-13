extends SceneTree
func _initialize(): call_deferred("check")
func require(ok: bool, why: String) -> bool:
    if not ok:
        push_error(why)
        quit(1)
    return ok
func check():
    var emitter = Node3D.new()
    emitter.set_script(load("res://addons/astrolabe/audio/emitter.gd"))
    root.add_child(emitter)
    var events = JSON.parse_string(FileAccess.get_file_as_string("res://generated/hype/sound-test/events.json")).map(func(path):return load(path))
    emitter.random.seed = 42
    var first = events[0].select(emitter)
    var second = events[0].select(emitter)
    if not require(first.trace[-1] != second.trace[-1], "nonrepeating choices"): return
    emitter.set_selector("sound:/Sound/types.sif", "Material", "Wood")
    var wood = events[0].select(emitter)
    if not require(wood.source_sample.ends_with("resource_42.tres"), "material selector"): return
    emitter.set_selector("sound:/Sound/types.sif", "Material", "Missing")
    var fallback = events[0].select(emitter)
    if not require(fallback.trace[1].ends_with("#2"), "selector default fallback"): return
    # Exercise both shared events on the same edited loop resource.
    emitter.set_selector("sound:/Sound/types.sif", "Material", "Stone")
    AudioServer.add_bus()
    var bus = AudioServer.bus_count - 1
    AudioServer.set_bus_name(bus, "SoundCheck")
    var capture = AudioEffectCapture.new()
    AudioServer.add_bus_effect(bus, capture)
    AudioServer.set_bus_volume_db(bus, -80.0)
    emitter.bus = "SoundCheck"
    emitter.previous_choices["sound:/Sound/test.csb#2"] = 1
    events[0].execute(emitter)
    emitter.previous_choices["sound:/Sound/test.csb#2"] = 1
    events[1].execute(emitter)
    if not require(emitter.voices.size() == 2, "shared-resource event playback"): return
    var player = emitter.voices[0].player
    if not require(player.stream.loop_begin == 100 and player.stream.loop_end == 600 and is_equal_approx(emitter.voices[0].gain, 0.5), "Rete loop/volume edit"): return
    if not require(player.pitch_scale >= 0.9 and player.pitch_scale <= 1.1, "random pitch range"): return
    await create_timer(0.25).timeout
    var frames = capture.get_buffer(capture.get_frames_available())
    var peak = 0.0
    for frame in frames: peak = maxf(peak,maxf(absf(frame.x),absf(frame.y)))
    if not require(frames.size() > 0 and peak > 0.01 and player.playing and player.get_playback_position() < 600.0/8000.0, "actual looped mixer output"): return
    events[2].execute(emitter)
    if not require(emitter.voices.size() == 1 and emitter.voices[0].event == events[1].identity, "stop matched a resource instead of the event"): return
    emitter.stop_event(null)
    if not require(emitter.voices.is_empty(), "stop all emitter voices"): return
    events[0].stoppable = false
    emitter.set_selector("sound:/Sound/types.sif", "Material", "Wood")
    events[0].execute(emitter)
    var continuing = emitter.voices[0].player
    emitter.stop_event(null)
    if not require(emitter.voices.is_empty() and continuing.playing, "nonabortable voice should detach and continue"): return
    continuing.stop()
    emitter.free()
    # Let AudioServer finish retiring native playback instances before removing
    # their bus; deleting the bus in the same mix cycle strands queued voices.
    await create_timer(0.15).timeout
    AudioServer.remove_bus(bus)
    capture = null
    await create_timer(0.05).timeout
    print("SOUND_PROGRAM_OK ", JSON.stringify({"frames":frames.size(),"peak":peak}))
    quit(0)
