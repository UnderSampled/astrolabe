# Disposable conversion-development viewer; this is not a downstream game runtime.
extends Node3D

@export var textures: PackedStringArray = []
@export var sounds: PackedStringArray = []
var camera: Camera3D
var center := Vector3.ZERO
var radius := 1.0
var yaw := 0.7
var pitch := 0.35
var player: AudioStreamPlayer

func _ready():
    var meshes = find_children("*", "MeshInstance3D", true, false)
    if not meshes.is_empty():
        var bounds: AABB = meshes[0].global_transform * meshes[0].get_aabb()
        for mesh in meshes.slice(1):
            bounds = bounds.merge(mesh.global_transform * mesh.get_aabb())
        center = bounds.get_center()
        radius = maxf(bounds.size.length() * 0.5, 0.1)
        camera = Camera3D.new()
        camera.near = maxf(radius / 1000.0, 0.001)
        camera.far = radius * 100.0
        add_child(camera)
        camera.make_current()
        position_camera()
    player = AudioStreamPlayer.new()
    add_child(player)
    var overlay = CanvasLayer.new()
    add_child(overlay)
    var list = VBoxContainer.new()
    list.position = Vector2(16, 16)
    overlay.add_child(list)
    var title = Label.new()
    title.text = "Astrolabe development preview · drag to orbit · wheel to zoom"
    list.add_child(title)
    if meshes.is_empty():
        var grid = GridContainer.new()
        grid.columns = 6
        list.add_child(grid)
        for path in textures.slice(0, 24):
            var thumbnail = TextureRect.new()
            thumbnail.texture = load(path)
            thumbnail.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
            thumbnail.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
            thumbnail.custom_minimum_size = Vector2(128, 128)
            thumbnail.tooltip_text = path
            grid.add_child(thumbnail)
    for path in sounds.slice(0, 20):
        var button = Button.new()
        button.text = "Play: " + path.get_file()
        button.pressed.connect(func(): player.stream = load(path); player.play())
        list.add_child(button)
    if not sounds.is_empty():
        var stop = Button.new()
        stop.text = "Stop audio"
        stop.pressed.connect(func(): player.stop())
        list.add_child(stop)
    var args = OS.get_cmdline_user_args()
    if "--capture" in args:
        for frame in range(5):
            await RenderingServer.frame_post_draw
        var destination = args[args.find("--capture") + 1]
        var image = get_viewport().get_texture().get_image()
        if image == null or image.save_png(destination) != OK:
            push_error("Could not capture rendered preview")
            get_tree().quit(1)
            return
        get_tree().quit(0)
    elif "--audio-check" in args:
        check_audio.call_deferred(args[args.find("--audio-check") + 1])

func position_camera():
    var direction = Vector3(sin(yaw) * cos(pitch), sin(pitch), cos(yaw) * cos(pitch))
    camera.position = center + direction * radius * 3.0
    camera.look_at(center, Vector3.UP)

func _unhandled_input(event):
    if camera == null:
        return
    if event is InputEventMouseMotion and event.button_mask & MOUSE_BUTTON_MASK_LEFT:
        yaw -= event.relative.x * 0.01
        pitch = clampf(pitch + event.relative.y * 0.01, -1.5, 1.5)
        position_camera()
    elif event is InputEventMouseButton and event.pressed:
        if event.button_index == MOUSE_BUTTON_WHEEL_UP:
            radius *= 0.9
        elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
            radius *= 1.1
        position_camera()

func check_audio(destination: String):
    # Capture the real Godot mixer before muting its output. No microphone is used.
    AudioServer.add_bus()
    var bus = AudioServer.bus_count - 1
    AudioServer.set_bus_name(bus, "AstrolabeProbe")
    var capture = AudioEffectCapture.new()
    capture.buffer_length = 1.0
    AudioServer.add_bus_effect(bus, capture)
    AudioServer.set_bus_volume_db(bus, -80.0)
    player.bus = "AstrolabeProbe"
    var evidence = []
    var failed = sounds.is_empty()
    for path in sounds:
        var stream = load(path) as AudioStreamWAV
        if stream == null:
            failed = true
            continue
        player.stream = stream
        capture.clear_buffer()
        var start = minf(0.1, stream.get_length() * 0.25)
        if stream.loop_mode != AudioStreamWAV.LOOP_DISABLED:
            start = maxf(float(stream.loop_begin) / stream.mix_rate, float(stream.loop_end) / stream.mix_rate - 0.08)
        player.play(start)
        await get_tree().create_timer(0.3).timeout
        var position = player.get_playback_position()
        var active = player.playing
        var frames = capture.get_buffer(capture.get_frames_available())
        var peak := 0.0
        for frame in frames:
            peak = maxf(peak, maxf(absf(frame.x), absf(frame.y)))
        var loop_ok = stream.loop_mode == AudioStreamWAV.LOOP_DISABLED or (active and position < float(stream.loop_end) / stream.mix_rate)
        if frames.is_empty() or peak == 0.0 or not loop_ok:
            failed = true
        evidence.append({"path": path, "rate": stream.mix_rate, "stereo": stream.stereo, "duration": stream.get_length(), "captured_frames": frames.size(), "peak": peak, "position": position, "loop_mode": stream.loop_mode, "loop_begin": stream.loop_begin, "loop_end": stream.loop_end, "loop_ok": loop_ok})
        player.stop()
        await get_tree().create_timer(0.05).timeout
    var file = FileAccess.open(destination, FileAccess.WRITE)
    if file == null:
        push_error("Cannot save audio evidence")
        get_tree().quit(1)
        return
    file.store_string(JSON.stringify(evidence, "  "))
    file.close()
    player.stop()
    player.stream = null
    AudioServer.remove_bus(bus)
    capture = null
    # The audio thread releases its removed effect on its next mix block.
    await get_tree().create_timer(0.15).timeout
    if failed:
        push_error("Audio mixer/loop probe failed; see " + destination)
    get_tree().quit.call_deferred(1 if failed else 0)
