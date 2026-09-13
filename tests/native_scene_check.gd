extends SceneTree

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize(): call_deferred("check")

func check():
    var expected_state = int(OS.get_cmdline_user_args()[0])
    var packed = load("res://generated/hype/levels/astrolabe/level.tscn") as PackedScene
    var uid = ResourceLoader.get_resource_uid(packed.resource_path)
    if not require(uid >= 0 and ResourceUID.get_id_path(uid) == packed.resource_path, "native resource identity is not registered"): return
    var scene = packed.instantiate()
    var actors: Array = []
    var source_nodes = 0
    var native_parts = 0
    var libraries = {}
    var persisted_count = scene.find_children("*", "", true, false).size()
    # No node has entered a SceneTree or received _ready: all content must exist.
    for node in scene.find_children("*", "", true, false):
        if node.has_meta("source_node_id"): source_nodes += 1
        if node.has_meta("configuration_slot"): native_parts += 1
        if not node.has_meta("source_perso"): continue
        actors.append(node)
        node.autoplay = false
        var player = node.get_node("AnimationPlayer") as AnimationPlayer
        if not require(player != null, "missing saved native player"): return
        for library_name in player.get_animation_library_list():
            var library = player.get_animation_library(library_name)
            if libraries.has(library.resource_path): continue
            libraries[library.resource_path] = library.get_animation_list().size()
            for name in library.get_animation_list():
                var clip = library.get_animation(name)
                for track in clip.get_track_count():
                    var path = clip.track_get_path(track)
                    if not require(node.has_node(NodePath(path.get_concatenated_names())), "unresolved native animation target " + str(path)): return
    if not require(source_nodes == 150 and actors.size() == 108 and native_parts > 400, "saved scene coverage: " + str([source_nodes,actors.size(),native_parts])): return
    var hero = scene.get_node("world/0/0/Hype")
    if not require(hero.initial_state == expected_state and hero.current_state == expected_state, "saved initial state differs before ready"): return
    var visible = hero.get_node("Channels").find_children("*", "MeshInstance3D", true, false).filter(func(n):return n.visible)
    if not require(not visible.is_empty(), "saved hero pose is empty"): return
    var body = hero.get_node("Channels/channel_0")
    var initial_transform = body.transform
    var initial_visibility = {}
    for node in hero.get_node("Channels").find_children("*", "Node3D", true, false): initial_visibility[hero.get_path_to(node)] = node.visible
    root.add_child(scene)
    if not require(persisted_count == scene.find_children("*", "", true, false).size(), "ready constructed scene assets"): return
    var player = hero.get_node("AnimationPlayer") as AnimationPlayer
    player.play("state_%d" % expected_state)
    player.seek(0.0, true, true)
    player.pause()
    if not require(body.transform.is_equal_approx(initial_transform), "persisted pose differs from native first key"): return
    for path in initial_visibility:
        if not require(hero.get_node(path).visible == initial_visibility[path], "persisted object configuration differs from native first key"): return
    # Source-game state 32: 29 frames at 30 FPS; cue descriptors at 0/14.
    var walk = player.get_animation("state_32")
    if not require(walk.get_meta("source_frames") == 29 and walk.get_meta("source_fps") == 30, "source walking timing"): return
    var cues = []
    hero.cue_triggered.connect(func(action, frame, channel, slot):cues.append([frame,channel,slot,action.kind]))
    hero.select_state(32)
    hero.advance(26.0/30.0)
    if not require(cues == [[0,22,480,&"sound"],[14,22,479,&"sound"]], "native walking cue order " + str(cues)): return
    hero.advance(3.0/30.0)
    if not require(cues.size() == 3 and cues[2][0] == 0, "native walk loop entry"): return
    hero.playing = false
    hero.get_node("SoundEmitter").stop_event(null)
    var variation = load("res://game/characters/luminous_hype.tscn").instantiate()
    if not require(variation.initial_state == expected_state and variation.get_node("Beacon").material_override.emission_enabled and variation.scale.is_equal_approx(Vector3(1.15,1.15,1.15)), "authored variation failed to inherit regenerated content"): return
    root.add_child(variation)
    variation.select_state(32)
    variation.seek_frame(14)
    variation.playing = false
    if not require(variation.frame_index == 14 and variation.get_node("Beacon").visible, "authored variation native seek"): return
    variation.get_node("SoundEmitter").stop_event(null)
    print("NATIVE_SCENE_OK ", JSON.stringify({"source_nodes":source_nodes,"persos":actors.size(),"parts":native_parts,"libraries":libraries.size(),"animations":libraries.values().reduce(func(a,b):return a+b,0),"state":expected_state}))
    variation.free()
    scene.free()
    await create_timer(0.15).timeout
    quit(0)
