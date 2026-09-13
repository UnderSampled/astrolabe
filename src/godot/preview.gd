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
var selected_meshes: Array[Node] = []
var fly_speed := 4.0
var characters: Array[Node] = []
var selected_character: Node
var state_selector: OptionButton

func _ready():
    var args = OS.get_cmdline_user_args()
    characters = find_children("*", "Node3D", true, false).filter(func(n): return n.has_method("select_state") and n.current_clip != null)
    if "--character" in args:
        var requested = args[args.find("--character") + 1]
        for actor in characters:
            if requested == actor.get_meta("source_perso_name", ""):
                selected_character = actor
                if "--state" in args:
                    actor.select_state(int(args[args.find("--state") + 1]))
                actor.playing = false
                if "--frame" in args:
                    actor.seek_frame(int(args[args.find("--frame") + 1]))
                break
    var boundaries = find_children("SectorBoundary", "MeshInstance3D", true, false)
    for boundary in boundaries: boundary.visible = "--show-sectors" in args
    if "--show-bounds" in args:
        for actor in characters: actor.set_bounds_visible(true)
    var meshes = find_children("*", "MeshInstance3D", true, false).filter(func(n): return not n.has_method("intersect_segment") and not n.get_meta("family_bounds_debug", false) and n.is_visible_in_tree())
    if selected_character != null:
        if "--actor-position" in args:
            var p = args[args.find("--actor-position") + 1].split(",")
            selected_character.global_position = Vector3(float(p[0]), float(p[1]), float(p[2]))
        if "--actor-yaw" in args:
            selected_character.global_rotation = Vector3(0, float(args[args.find("--actor-yaw") + 1]), 0)
        if "--hide-channels" in args:
            for index in args[args.find("--hide-channels") + 1].split(","):
                var channel = selected_character.get_node_or_null("Channels/channel_%d" % int(index))
                if channel == null: continue
                for part in channel.get_children(): part.visible = false
        if "--isolate-character" in args:
            for actor in characters:
                if actor != selected_character:
                    actor.playing = false
                    actor.visible = false
            for mesh in meshes:
                if not selected_character.is_ancestor_of(mesh): mesh.visible = false
            meshes = meshes.filter(func(mesh): return selected_character.is_ancestor_of(mesh))
    selected_meshes = meshes
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
    overlay.visible = not "--no-overlay" in args
    var list = VBoxContainer.new()
    list.position = Vector2(16, 16)
    overlay.add_child(list)
    var title = Label.new()
    title.text = "Astrolabe preview · drag to orbit · wheel to zoom · right-drag + WASD to fly"
    list.add_child(title)
    if not meshes.is_empty():
        var selector = OptionButton.new()
        selector.add_item("Whole scene")
        for mesh in meshes:
            selector.add_item(str(mesh.get_path()))
        selector.item_selected.connect(focus_selection)
        list.add_child(selector)
    if not characters.is_empty():
        var actor_selector = OptionButton.new()
        for actor in characters:
            actor_selector.add_item(actor.get_meta("source_perso_name", str(actor.get_path())))
        actor_selector.item_selected.connect(select_character)
        list.add_child(actor_selector)
        state_selector = OptionButton.new()
        state_selector.item_selected.connect(func(index):
            selected_character.select_state(state_selector.get_item_id(index))
            focus_character())
        list.add_child(state_selector)
        var pause = CheckButton.new()
        pause.text = "Pause selected animation"
        pause.toggled.connect(func(paused):
            if selected_character != null: selected_character.playing = not paused)
        list.add_child(pause)
        select_character(0 if selected_character == null else characters.find(selected_character))
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
    if selected_character != null:
        focus_character()
    if camera != null and "--focus" in args:
        var requested = args[args.find("--focus") + 1]
        for i in meshes.size():
            if requested in str(meshes[i].get_path()):
                focus_selection(i + 1)
                break
    if camera != null and "--camera" in args and "--look-at" in args:
        var position_values = args[args.find("--camera") + 1].split(",")
        var target_values = args[args.find("--look-at") + 1].split(",")
        if position_values.size() != 3 or target_values.size() != 3:
            push_error("Camera and target need x,y,z coordinates")
            get_tree().quit(1)
            return
        camera.position = Vector3(float(position_values[0]), float(position_values[1]), float(position_values[2]))
        center = Vector3(float(target_values[0]), float(target_values[1]), float(target_values[2]))
        camera.look_at(center)
    if camera != null and "--fov" in args:
        camera.fov = float(args[args.find("--fov") + 1])
    if camera != null and "--near" in args:
        camera.near = float(args[args.find("--near") + 1])
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

func select_character(index: int):
    selected_character = characters[index]
    state_selector.clear()
    for animation_name in selected_character.state_names():
        if not animation_name.begins_with("state_"): continue
        var id := int(animation_name.trim_prefix("state_"))
        var animation = selected_character.animation_player.get_animation(animation_name)
        state_selector.add_item("State %d · %d fps" % [id, int(animation.get_meta("source_fps", 0))], id)
        if id == selected_character.current_state:
            state_selector.select(state_selector.item_count-1)
    focus_character()

func focus_character():
    var meshes = selected_character.find_children("*", "MeshInstance3D", true, false).filter(func(n): return n.is_visible_in_tree())
    if meshes.is_empty() or camera == null: return
    frame_meshes(meshes)

func frame_meshes(meshes: Array):
    var bounds: AABB = meshes[0].global_transform * meshes[0].get_aabb()
    for mesh in meshes.slice(1):
        bounds = bounds.merge(mesh.global_transform * mesh.get_aabb())
    center = bounds.get_center()
    radius = maxf(bounds.size.length() * 0.5, 0.1)
    fly_speed = maxf(radius * 0.25, 0.25)
    position_camera()

func position_camera():
    var direction = Vector3(sin(yaw) * cos(pitch), sin(pitch), cos(yaw) * cos(pitch))
    camera.position = center + direction * radius * 3.0
    camera.look_at(center, Vector3.UP)

func focus_selection(index: int):
    var meshes = selected_meshes if index == 0 else [selected_meshes[index - 1]]
    meshes = meshes.filter(func(mesh): return is_instance_valid(mesh))
    if not meshes.is_empty(): frame_meshes(meshes)

func _process(delta):
    if camera == null or not Input.is_mouse_button_pressed(MOUSE_BUTTON_RIGHT):
        return
    var direction = Vector3(float(Input.is_physical_key_pressed(KEY_D)) - float(Input.is_physical_key_pressed(KEY_A)), float(Input.is_physical_key_pressed(KEY_E)) - float(Input.is_physical_key_pressed(KEY_Q)), float(Input.is_physical_key_pressed(KEY_S)) - float(Input.is_physical_key_pressed(KEY_W)))
    var movement = camera.basis * direction * fly_speed * delta
    camera.position += movement
    center += movement

func _unhandled_input(event):
    if camera == null:
        return
    if event is InputEventMouseMotion and event.button_mask & MOUSE_BUTTON_MASK_RIGHT:
        camera.rotate_y(-event.relative.x * 0.005)
        camera.rotate_object_local(Vector3.RIGHT, -event.relative.y * 0.005)
        center = camera.position - camera.basis.z * radius * 3.0
    elif event is InputEventMouseMotion and event.button_mask & MOUSE_BUTTON_MASK_LEFT:
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
