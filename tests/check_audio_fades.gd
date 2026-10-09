extends SceneTree

const Behavior = preload("res://addons/astrolabe/behavior/behavior.gd")
const Mixer = preload("res://addons/astrolabe/audio/mixer.gd")
const FADE_STATE_META := &"astrolabe_source_audio_fades"

func _initialize() -> void:
	call_deferred("_check_audio_fades")

func _check_audio_fades() -> void:
	var first = Behavior.new()
	first.automatic = false
	var second = Behavior.new()
	second.automatic = false
	root.add_child(first)
	root.add_child(second)
	var mixer := Mixer.shared(first) as Mixer
	mixer.set_preference(&"music", 73)
	mixer.set_preference(&"effects", 61)
	mixer.set_preference(&"voice", 92)

	first.sound_fade_voice(0, 60000)
	first.sound_fade_sound(0, 60000)
	var fades: Dictionary = root.get_meta(FADE_STATE_META)
	var first_voice: Tween = instance_from_id(int(fades["Astrolabe Voice"])) as Tween
	var first_effects: Tween = instance_from_id(int(fades["Astrolabe Effects"])) as Tween
	assert(first_voice.is_running())
	assert(first_effects.is_running())
	assert(first_voice != first_effects)

	first.queue_free()
	await process_frame
	assert(not is_instance_valid(first))
	assert(first_voice.is_running(), "tree-owned fader must survive its requesting Brain")

	second.sound_fade_voice(127, 60000)
	var second_voice: Tween = instance_from_id(int(fades["Astrolabe Voice"])) as Tween
	assert(second_voice != first_voice)
	assert(not first_voice.is_running(), "new fade must replace the active fade on the same channel")
	assert(second_voice.is_running())
	assert(first_effects.is_running(), "a fade on voices must leave effects independent")

	second.sound_fade_all(127, 40)
	var all_voice: Tween = instance_from_id(int(fades["Astrolabe Voice"])) as Tween
	var all_effects: Tween = instance_from_id(int(fades["Astrolabe Effects"])) as Tween
	var all_music: Tween = instance_from_id(int(fades["Astrolabe Music"])) as Tween
	assert(not second_voice.is_running())
	assert(not first_effects.is_running())
	assert(all_voice.is_running() and all_effects.is_running() and all_music.is_running())
	assert(all_voice != all_effects and all_effects != all_music)

	await create_timer(0.1).timeout
	assert(not fades.has("Astrolabe Voice"))
	assert(not fades.has("Astrolabe Effects"))
	assert(not fades.has("Astrolabe Music"))
	for bus_name in [&"Astrolabe Voice", &"Astrolabe Effects", &"Astrolabe Music"]:
		var bus := AudioServer.get_bus_index(bus_name)
		assert(bus >= 0)
		var module := Mixer.module_for_bus(bus_name)
		assert(mixer.source_volume_word(module) == 127)
		var preference := int(mixer.preference_levels[module])
		assert(is_equal_approx(db_to_linear(AudioServer.get_bus_volume_db(bus)), float(preference) / 127.0), "source fade overwrote user preference")
	second.queue_free()
	await process_frame
	root.remove_meta(FADE_STATE_META)
	print("AUDIO_FADES_OK")
	call_deferred("quit", 0)
