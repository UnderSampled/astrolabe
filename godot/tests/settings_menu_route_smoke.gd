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
	for bus in ["Music", "Effects", "Voices"]:
		var slider := settings.get_node_or_null("Canvas/SoundPage/Audio/" + bus) as VSlider
		if slider == null or slider.min_value != 0.0 or slider.max_value != 127.0:
			missing.append("source-scale %s slider" % bus)
	var image_limits := {
		"Graphic": Vector2(0.0, 4.0),
		"Screen": Vector2(60.0, 100.0),
		"Textures": Vector2(0.0, 1.0),
	}
	for name in image_limits:
		var slider := settings.get_node_or_null("Canvas/ImagesPage/" + name) as VSlider
		var limits: Vector2 = image_limits[name]
		if slider == null or slider.min_value != limits.x or slider.max_value != limits.y:
			missing.append("source-scale %s image slider" % name)
	for action in ["Button_Left", "Button_Right", "Button_Up", "Button_Down", "Button_Hit"]:
		if settings.get_node_or_null("Canvas/ControlsPage/Binding_" + action) == null:
			missing.append("%s binding" % action)
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
	var old_shadow := float(preferences.call("get_image_setting", "shadow_quality"))
	var old_framing := float(preferences.call("get_image_setting", "screen_framing"))
	var old_texture := float(preferences.call("get_image_setting", "texture_quality"))
	var old_reverse := bool(preferences.call("get_reverse_stereo"))

	var light := DirectionalLight3D.new()
	light.shadow_enabled = true
	root.add_child(light)
	var camera := Camera3D.new()
	camera.fov = 75.0
	root.add_child(camera)
	var material := StandardMaterial3D.new()
	var mesh := MeshInstance3D.new()
	mesh.mesh = BoxMesh.new()
	mesh.material_override = material
	root.add_child(mesh)
	await process_frame

	preferences.call("set_image_setting", "shadow_quality", 0.0)
	if light.shadow_enabled:
		fail("Graphic quality 0 did not disable native light shadows")
		return
	preferences.call("set_image_setting", "shadow_quality", 4.0)
	if not light.shadow_enabled:
		fail("Restoring Graphic quality did not restore the light's shadow state")
		return
	preferences.call("set_image_setting", "screen_framing", 60.0)
	if camera.fov >= 75.0:
		fail("Screen framing did not narrow perspective field of view")
		return
	preferences.call("set_image_setting", "screen_framing", 100.0)
	if absf(camera.fov - 75.0) > 0.01:
		fail("Screen framing 100 did not restore the original field of view")
		return
	preferences.call("set_image_setting", "texture_quality", 0.0)
	var filtered := mesh.material_override as BaseMaterial3D
	if filtered == null or filtered.texture_filter != BaseMaterial3D.TEXTURE_FILTER_NEAREST or material.texture_filter == BaseMaterial3D.TEXTURE_FILTER_NEAREST:
		fail("Textures slider did not apply a non-destructive material filtering override")
		return
	preferences.call("set_image_setting", "texture_quality", 1.0)
	filtered = mesh.material_override as BaseMaterial3D
	if filtered.texture_filter != BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC:
		fail("Textures quality 1 did not select Godot's mipmapped anisotropic filter")
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

	preferences.call("set_image_setting", "shadow_quality", old_shadow)
	preferences.call("set_image_setting", "screen_framing", old_framing)
	preferences.call("set_image_setting", "texture_quality", old_texture)
	preferences.call("set_reverse_stereo", old_reverse)
	light.queue_free()
	camera.queue_free()
	mesh.queue_free()
	player.queue_free()
	var closed := {"value": false}
	settings.closed.connect(func(): closed["value"] = true)
	(settings.get_node("Canvas/Back") as Button).pressed.emit()
	if not bool(closed["value"]):
		fail("Back did not emit the route's closed signal")
		return
	print("SETTINGS ROUTE SMOKE: menu API, source sliders, native image effects, PCM stereo actions and Back signaling passed")
	quit(0)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
