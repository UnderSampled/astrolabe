extends SceneTree
## Run after exporting the English menu: godot --headless --path godot --script res://tests/settings_menu_route_smoke.gd

const SETTINGS_MENU := "res://game/settings_menu.tscn"

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var packed := load(SETTINGS_MENU) as PackedScene
	if packed == null:
		fail("Authored settings route did not load")
		return
	var settings := packed.instantiate() as Control
	if settings == null:
		fail("Settings route root is not a Control")
		return
	root.add_child(settings)
	current_scene = settings
	await process_frame

	var missing: Array[String] = []
	if not settings.has_signal("closed"):
		missing.append("closed signal")
	if not settings.has_method("focus_initial"):
		missing.append("focus_initial()")
	if not settings.has_method("cancel_input_capture"):
		missing.append("cancel_input_capture()")
	for page in ["ImagesPage", "ControlsPage", "SoundPage"]:
		if settings.get_node_or_null("Canvas/" + page) == null:
			missing.append(page)
	var sound_limits := {
		"Music": Vector2(0.0, 127.0),
		"Effects": Vector2(0.0, 127.0),
		"Voices": Vector2(-20.0, 107.0),
	}
	for bus in sound_limits:
		var slider := settings.get_node_or_null("Canvas/SoundPage/Audio/" + bus) as VSlider
		var limits: Vector2 = sound_limits[bus]
		if slider == null or slider.min_value != limits.x or slider.max_value != limits.y or slider.step != 1.0:
			missing.append("source-scale %s slider" % bus)
	var image_limits := {
		"Graphic": Vector2(1.0, 4.0),
		"Screen": Vector2(60.0, 100.0),
	}
	for name in image_limits:
		var slider := settings.get_node_or_null("Canvas/ImagesPage/" + name) as VSlider
		var artwork := settings.get_node_or_null("Canvas/ImagesPage/" + name + "Artwork") as TextureRect
		var limits: Vector2 = image_limits[name]
		if slider == null or slider.min_value != limits.x or slider.max_value != limits.y or slider.step != 1.0:
			missing.append("source-scale %s image slider" % name)
		if artwork == null:
			missing.append("%s source label artwork" % name)
	var textures_artwork := settings.get_node_or_null("Canvas/ImagesPage/TexturesArtwork") as TextureRect
	var textures_unsupported := settings.get_node_or_null("Canvas/ImagesPage/TexturesUnsupported") as Label
	if textures_artwork == null or textures_unsupported == null or not textures_unsupported.text.contains("Unsupported") or not textures_unsupported.text.contains("texture-swapping"):
		missing.append("explicitly unsupported source texture-swapping control")
	if settings.get_node_or_null("Canvas/ImagesPage/Textures") != null:
		missing.append("unrelated filtering slider presented as source Textures")
	var controls_scene := str(settings.get_node("Canvas/ControlsPage").get_meta("controls_scene", ""))
	if not ResourceLoader.exists(controls_scene, "PackedScene"):
		missing.append("converted Controls page scene")
	for action in ["StereoTest", "ReverseStereo"]:
		if settings.get_node_or_null("Canvas/SoundPage/" + action) == null:
			missing.append("%s sound action" % action)
		if settings.get_node_or_null("Canvas/SoundPage/" + action + "Selected") == null:
			missing.append("%s selected source artwork" % action)
	if settings.get_node_or_null("Canvas/ImagesPage/ImageMappingNote") == null:
		missing.append("image mapping explanation")
	if settings.get_node_or_null("Canvas/ImagesPage/UnsupportedImageSettings") != null:
		missing.append("obsolete inert Images placeholder remains")
	if not missing.is_empty():
		fail("Settings scene is missing: " + ", ".join(missing))
		return

	settings.call("focus_initial")
	if bool(settings.call("cancel_input_capture")):
		fail("cancel_input_capture() reported an active capture before one started")
		return
	# The Controls item opens the original page; hosts leave Escape to it.
	settings.call("open_controls")
	var controls_page := settings.get("controls_page") as Control
	if controls_page == null or not bool(settings.call("cancel_input_capture")) or settings.get_node("Canvas").visible:
		fail("The Controls item did not open the original Controls page")
		return
	controls_page.emit_signal("closed")
	await process_frame
	if settings.get("controls_page") != null or not settings.get_node("Canvas").visible or bool(settings.call("cancel_input_capture")):
		fail("Closing the Controls page did not return to Configure")
		return
	settings.call("_show_page", &"Sound")
	var reverse_button := settings.get_node("Canvas/SoundPage/ReverseStereo") as Button
	reverse_button.grab_focus()
	await process_frame
	if not (settings.get_node("Canvas/SoundPage/ReverseStereoSelected") as TextureRect).visible:
		fail("Reverse Stereo did not show its source selected artwork on focus")
		return
	var preferences := settings.get("preferences") as Node
	if preferences == null:
		fail("Settings route did not resolve UserSettings")
		return
	var transient_node := Node3D.new()
	var transient_node_id := transient_node.get_instance_id()
	root.add_child(transient_node)
	transient_node.free()
	await process_frame
	if bool(preferences.call("_apply_settings_to_node_by_id", transient_node_id)):
		fail("Deferred image-settings callback resolved a node after it was freed")
		return
	var old_shadow := float(preferences.call("get_image_setting", "shadow_quality"))
	var old_framing := float(preferences.call("get_image_setting", "screen_framing"))
	var old_reverse := bool(preferences.call("get_reverse_stereo"))
	var old_volumes := {}
	for bus in sound_limits:
		old_volumes[bus] = float(preferences.call("get_volume", bus))
	preferences.call("reset_audio")
	for bus in sound_limits:
		if float(preferences.call("get_volume", bus)) != 100.0:
			fail("Reset Audio did not restore the original 100 mixer value for %s" % bus)
			return
	preferences.call("set_volume", "Voices", -5.0)
	if float(preferences.call("get_volume", "Voices")) != -20.0:
		fail("Voice values below zero did not preserve the source silent sentinel")
		return
	preferences.call("set_volume", "Voices", 127.0)
	if float(preferences.call("get_volume", "Voices")) != 107.0:
		fail("Voice slider values above the source maximum did not clamp to 107")
		return

	var light := DirectionalLight3D.new()
	light.shadow_enabled = true
	root.add_child(light)
	var camera := Camera3D.new()
	camera.fov = 75.0
	root.add_child(camera)
	var ortho_camera := Camera3D.new()
	ortho_camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	ortho_camera.size = 10.0
	root.add_child(ortho_camera)
	await process_frame

	preferences.call("set_image_setting", "shadow_quality", 1.0)
	if not light.shadow_enabled:
		fail("Graphic source level 1 disabled a source-enabled light shadow")
		return
	preferences.call("set_image_setting", "shadow_quality", 4.0)
	if not light.shadow_enabled:
		fail("Graphic source level 4 disabled a source-enabled light shadow")
		return
	preferences.call("set_image_setting", "screen_framing", 60.0)
	var minimum_crop := 0.2 # Original centered viewport spans 60 - (100 - 60) percent.
	var minimum_fov := rad_to_deg(2.0 * atan(tan(deg_to_rad(75.0) * 0.5) * minimum_crop))
	if absf(camera.fov - minimum_fov) > 0.01 or absf(ortho_camera.size - 2.0) > 0.01:
		fail("Screen framing minimum did not reproduce the source centered viewport crop")
		return
	preferences.call("set_image_setting", "screen_framing", 80.0)
	var source_crop := 0.6 # Original centered viewport spans 80 - (100 - 80) percent.
	var expected_fov := rad_to_deg(2.0 * atan(tan(deg_to_rad(75.0) * 0.5) * source_crop))
	if absf(camera.fov - expected_fov) > 0.01:
		fail("Screen framing did not reproduce the source centered perspective crop")
		return
	if absf(ortho_camera.size - 6.0) > 0.01:
		fail("Screen framing did not reproduce the source orthographic viewport crop")
		return
	preferences.call("set_image_setting", "screen_framing", 100.0)
	if absf(camera.fov - 75.0) > 0.01 or absf(ortho_camera.size - 10.0) > 0.01:
		fail("Screen framing 100 did not restore original camera projection values")
		return

	var pcm := PackedByteArray()
	pcm.resize(8)
	pcm.encode_s16(0, 1100)
	pcm.encode_s16(2, 2200)
	pcm.encode_s16(4, -3300)
	pcm.encode_s16(6, -4400)
	var source_wave := AudioStreamWAV.new()
	source_wave.format = AudioStreamWAV.FORMAT_16_BITS
	source_wave.stereo = true
	source_wave.data = pcm
	var player := AudioStreamPlayer.new()
	player.stream = source_wave
	root.add_child(player)
	await process_frame
	preferences.call("set_reverse_stereo", false)
	if player.stream != source_wave:
		fail("Turning stereo reversal off did not restore the source stream")
		return
	preferences.call("set_reverse_stereo", true)
	var reversed_wave := player.stream as AudioStreamWAV
	var reversed_pcm := reversed_wave.data if reversed_wave != null else PackedByteArray()
	if reversed_pcm.size() != 8 or reversed_pcm.decode_s16(0) != 2200 or reversed_pcm.decode_s16(2) != 1100 or reversed_pcm.decode_s16(4) != -4400 or reversed_pcm.decode_s16(6) != -3300:
		fail("Reverse Stereo did not swap interleaved PCM channels")
		return
	if source_wave.data.decode_s16(0) != 1100:
		fail("Reverse Stereo modified the source AudioStreamWAV resource")
		return
	if not bool(preferences.call("play_stereo_test")):
		fail("Stereo Test did not start")
		return
	var test_player := root.get_node_or_null("StereoTestPlayer") as AudioStreamPlayer
	var test_wave: AudioStreamWAV = null
	if test_player != null:
		test_wave = test_player.stream as AudioStreamWAV
	if test_wave == null or not test_wave.stereo or test_wave.format != AudioStreamWAV.FORMAT_16_BITS:
		fail("Stereo Test did not create native stereo PCM")
		return
	var tone_frame_offset := 4
	if test_wave.data.decode_s16(tone_frame_offset) != 0 or test_wave.data.decode_s16(tone_frame_offset + 2) == 0:
		fail("Stereo Test did not honor the enabled channel reversal")
		return
	var stereo_test_finished := {"value": false}
	test_player.finished.connect(func(): stereo_test_finished["value"] = true)
	await create_timer(1.0).timeout
	if not bool(stereo_test_finished["value"]):
		fail("Stereo Test tone did not finish and release its playback")
		return

	# Confirm saved settings reach disk, then survive closing and reopening the route.
	preferences.call("set_image_setting", "shadow_quality", 2.0)
	preferences.call("set_image_setting", "screen_framing", 80.0)
	for bus in sound_limits:
		preferences.call("set_volume", bus, 100.0)
	preferences.call("set_reverse_stereo", true)
	var saved := ConfigFile.new()
	if saved.load("user://hype-settings.cfg") != OK:
		fail("Settings were not saved to user://hype-settings.cfg")
		return
	if int(saved.get_value("video", "shadow_quality", -1)) != 2 or int(saved.get_value("video", "screen_framing", -1)) != 80 or float(saved.get_value("audio", "Music", -1.0)) != 100.0 or float(saved.get_value("audio", "Effects", -1.0)) != 100.0 or float(saved.get_value("audio", "Voices", -1.0)) != 100.0 or not bool(saved.get_value("audio", "reverse_stereo", false)):
		fail("Source-scale audio and Image/Stereo preferences did not persist their values")
		return

	if not await _close_route(settings):
		fail("Back did not close the first settings route")
		return
	preferences.queue_free()
	await process_frame
	var preferences_script := load("res://game/user_settings.gd") as Script
	var reloaded_preferences := preferences_script.new() as Node if preferences_script != null else null
	if reloaded_preferences == null:
		fail("UserSettings could not be recreated to verify on-disk reload")
		return
	reloaded_preferences.name = "UserSettings"
	root.add_child(reloaded_preferences)
	await process_frame
	preferences = reloaded_preferences
	if int(preferences.call("get_image_setting", "shadow_quality")) != 2 or int(preferences.call("get_image_setting", "screen_framing")) != 80 or float(preferences.call("get_volume", "Music")) != 100.0 or float(preferences.call("get_volume", "Effects")) != 100.0 or float(preferences.call("get_volume", "Voices")) != 100.0 or not bool(preferences.call("get_reverse_stereo")):
		fail("UserSettings did not reload source-scale audio and Image/Stereo preferences")
		return
	for cycle in range(2):
		var reopened := packed.instantiate() as Control
		if reopened == null:
			fail("Settings route did not reopen on cycle %d" % cycle)
			return
		root.add_child(reopened)
		await process_frame
		var reopened_shadow := reopened.get_node("Canvas/ImagesPage/Graphic") as VSlider
		var reopened_framing := reopened.get_node("Canvas/ImagesPage/Screen") as VSlider
		var reopened_reverse := reopened.get_node("Canvas/SoundPage/ReverseStereo") as Button
		var reopened_textures_unsupported := reopened.get_node_or_null("Canvas/ImagesPage/TexturesUnsupported") as Label
		if reopened_shadow == null or reopened_framing == null or reopened_reverse == null or reopened_textures_unsupported == null:
			fail("Reopened settings controls were missing on cycle %d" % cycle)
			return
		if int(reopened_shadow.value) != 2 or int(reopened_framing.value) != 80 or float((reopened.get_node("Canvas/SoundPage/Audio/Voices") as VSlider).value) != 100.0 or not reopened_reverse.button_pressed:
			fail("Saved Image/Stereo values did not reload on cycle %d" % cycle)
			return
		if not await _close_route(reopened):
			fail("Back did not close settings route on cycle %d" % cycle)
			return

	preferences.call("set_image_setting", "shadow_quality", old_shadow)
	preferences.call("set_image_setting", "screen_framing", old_framing)
	for bus in old_volumes:
		preferences.call("set_volume", bus, old_volumes[bus])
	preferences.call("set_reverse_stereo", old_reverse)
	light.queue_free()
	camera.queue_free()
	ortho_camera.queue_free()
	player.queue_free()
	var stereo_player := root.get_node_or_null("StereoTestPlayer") as AudioStreamPlayer
	if stereo_player != null:
		stereo_player.stop()
		stereo_player.queue_free()
	preferences.queue_free()
	call_deferred("_finish")

func _close_route(settings: Control) -> bool:
	var closed := {"value": false}
	settings.closed.connect(func():
		closed["value"] = true
		settings.queue_free()
	)
	(settings.get_node("Canvas/Back") as Button).pressed.emit()
	await process_frame
	return bool(closed["value"]) and not is_instance_valid(settings)

func _finish() -> void:
	await process_frame
	print("SETTINGS ROUTE SMOKE: source-scale audio, shadow/framing mappings, unsupported texture-swapping disclosure, PCM stereo actions, and three open/close cycles passed")
	quit(0)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
