# Temporary sound-program validation runtime. Programs contain useful values,
# never source handles or CSB syntax. Each emitter owns its random/selector state.
extends Node3D
signal voice_started(info: Dictionary)
var bus: StringName = &"Master"
var enabled: bool = true
var selectors: Dictionary = {}
var voices: Array = []
var previous_choices: Dictionary = {}
var random := RandomNumberGenerator.new()
static var programs: Dictionary = {}

func set_selector(owner: String, name: String, value: String):
    selectors[owner + "#" + name] = value

static func resource(reference: Dictionary) -> Dictionary:
    if not programs.has(reference.file):
        var parsed = JSON.parse_string(FileAccess.get_file_as_string(reference.file))
        assert(parsed is Dictionary, "Invalid sound program")
        programs[reference.file] = parsed
    return programs[reference.file].resources[str(int(reference.id))]

func execute(event: Dictionary):
    if event.operation == "stop":
        stop_event(event.target)
        return
    if not enabled or event.operation != "play" or event.resource == null: return
    var selected = choose(event.resource, [], 1.0)
    if selected.is_empty(): return
    var sample: Dictionary = selected.resource
    var definition: Dictionary = sample.data
    var stream = load(definition.stream).duplicate() as AudioStreamWAV
    assert(stream != null, "Missing sound waveform")
    stream.loop_mode = AudioStreamWAV.LOOP_DISABLED
    if sample.looped:
        stream.loop_mode = AudioStreamWAV.LOOP_FORWARD
        if sample.loop_region != null:
            stream.loop_begin = int(sample.loop_region.start_frame)
            stream.loop_end = int(sample.loop_region.end_frame)
        elif stream.loop_end <= stream.loop_begin:
            stream.loop_begin = 0
            stream.loop_end = int(round(stream.get_length() * stream.mix_rate))
    var player: Node
    if definition.controls.spatial:
        player = AudioStreamPlayer3D.new()
    else:
        player = AudioStreamPlayer.new()
    player.bus = bus
    player.stream = stream
    player.volume_db = linear_to_db(maxf(selected.gain, 0.000001))
    var pitch = 1.0 + random.randf_range(-float(event.pitch_variation), float(event.pitch_variation))
    player.pitch_scale = maxf(pitch, 0.001) if definition.controls.pitchable else 1.0
    add_child(player)
    if player is Node3D and not event.dynamic:
        player.top_level = true
        player.global_transform = global_transform
    var info = {"event":event.identity,"stream":definition.stream,"pitch":player.pitch_scale,"gain":selected.gain,"choices":selected.trace,"spatial":definition.controls.spatial,"stoppable":event.stoppable,"player":player}
    voices.append(info)
    player.finished.connect(func(): release_voice(info))
    player.play()
    voice_started.emit(info)

func choose(reference: Dictionary, trace: Array, gain: float, depth: int = 0) -> Dictionary:
    assert(depth < 64, "Excessive sound program nesting")
    var key = str(reference.file) + "#" + str(int(reference.id))
    var value = resource(reference)
    var next_trace = trace.duplicate()
    next_trace.append(key)
    if value.volume != null: gain *= clampf(float(value.volume) / 127.0, 0.0, 1.0)
    var program: Dictionary = value.data
    match program.kind:
        "sample":
            assert(program.stream != null, "Unbound sound sample")
            return {"resource":value,"trace":next_trace,"gain":gain}
        "switch":
            var wanted = selectors.get(program.selector, program.default)
            var fallback = null
            for choice in program.choices:
                if choice.value == wanted: return choose(choice.resource, next_trace, gain, depth + 1)
                if choice.value == program.default: fallback = choice.resource
            if fallback != null: return choose(fallback, next_trace, gain, depth + 1)
        "random":
            var previous = int(previous_choices.get(key, -2))
            var options: Array = []
            var total = 0.0
            for index in program.choices.size():
                var choice = program.choices[index]
                if index == previous and not choice.repeat: continue
                var weight = float(choice.probability)
                if weight > 0.0:
                    options.append({"index":index,"weight":weight})
                    total += weight
            if previous != -1 or program.repeat_silence:
                var silence = float(program.silence_probability)
                if silence > 0.0:
                    options.append({"index":-1,"weight":silence})
                    total += silence
            if total <= 0.0: return {}
            var draw = random.randf() * total
            var index = int(options.back().index)
            for option in options:
                draw -= float(option.weight)
                if draw <= 0.0:
                    index = int(option.index)
                    break
            previous_choices[key] = index
            if index != -1: return choose(program.choices[index].resource, next_trace, gain, depth + 1)
    return {}

func stop_event(identity):
    for voice in voices.duplicate():
        if identity == null or voice.event == identity:
            # Source stop distinguishes abortable voices from already released
            # one-shots. Exact release envelopes remain target approximations.
            if voice.stoppable:
                voice.player.stop()
                release_voice(voice)
            else:
                # The original removes the actor/event association while its
                # nonabortable sample continues. Keep it until natural finish.
                voices.erase(voice)
                if voice.player is Node3D:
                    var placement = voice.player.global_transform
                    voice.player.top_level = true
                    voice.player.global_transform = placement

func release_voice(info: Dictionary):
    voices.erase(info)
    if is_instance_valid(info.player): info.player.queue_free()
