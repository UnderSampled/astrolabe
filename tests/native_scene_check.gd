extends SceneTree

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize(): call_deferred("check")

func saved_visible(node: Node3D, actor: Node3D) -> bool:
    # Saved slot anchors now own visibility; native payload meshes inherit it.
    # This check still runs before the scene enters the tree.
    var current: Node = node
    while current != null:
        if current is Node3D and not (current as Node3D).visible: return false
        if current == actor: return true
        current = current.get_parent()
    return false

func check():
    var expected_state = int(OS.get_cmdline_user_args()[0])
    var expected_radius = float(OS.get_cmdline_user_args()[1])
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
        if not node is CharacterBody3D or node.get_parent().name == "Globals":
            # Fix-owned global actors are nonvisual Node3D holders of Brain state.
            if not require(node.get_node_or_null("Brain") != null, "global actor lost its Brain"): return
            continue
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
    var torch = scene.get_node("world/0/103/i_Dummy_Son_Torche_Astro_001/Brain")
    var other_torch = scene.get_node("world/0/102/i_Dummy_Son_Torche_Astro_002/Brain")
    for brain in [torch,other_torch]:
        brain.automatic = false
        if not require(not brain.main_actor_path.is_empty() and brain.get_node(brain.main_actor_path) == hero, "saved AI main actor reference missing"): return
        if not require(brain.dsg_0 == false, "saved AI instance value differs"): return
    if not require(hero.initial_state == expected_state and hero.current_state == expected_state, "saved initial state differs before ready"): return
    var visible = hero.get_node("Channels").find_children("*", "MeshInstance3D", true, false).filter(func(n):return saved_visible(n, hero))
    if not require(not visible.is_empty(), "saved hero pose is empty"): return
    var body = hero.get_node("Channels/channel_0")
    var initial_transform = body.transform
    var initial_visibility = {}
    for node in hero.get_node("Channels").find_children("*", "Node3D", true, false): initial_visibility[hero.get_path_to(node)] = node.visible
    root.add_child(scene)
    if not require(persisted_count == scene.find_children("*", "", true, false).size(), "ready constructed scene assets"): return
    if not require(torch.main_actor == hero and torch.sound_emitter == torch.get_parent().get_node("SoundEmitter"), "native AI binding failed"): return
    var hero_position = hero.global_position
    var sound_events: Array[String] = []
    torch.sound_requested.connect(func(identity: String):sound_events.append(identity))
    # World-space float quantization can turn an added radius into a slightly
    # smaller measured distance. The synthetic origin fixture tests exact equality.
    hero.global_position = torch.global_position + Vector3(expected_radius+0.1,0,0)
    if not require(torch.start(), "source torch program did not start"): return
    torch.advance(0.249)
    if not require(sound_events.is_empty() and not torch.dsg_0, "source 250ms wait completed early"): return
    torch.advance(0.001)
    if not require(sound_events.is_empty() and not torch.dsg_0, "source outside-radius comparison failed"): return
    torch.advance(0.0)
    hero.global_position = torch.global_position + Vector3(expected_radius-0.1,0,0)
    torch.advance(0.25)
    if not require(torch.dsg_0 and not other_torch.dsg_0 and sound_events.size()==2, "source torch near transition or instance isolation failed"): return
    if not require(torch.sound_emitter.voices.size()==1 and torch.sound_emitter.voices[0].player.stream is AudioStreamWAV, "compiled source sound did not play a native stream"): return
    torch.advance(0.0)
    torch.advance(0.25)
    if not require(sound_events.size()==2, "source torch repeated sound while near"): return
    torch.advance(0.0)
    hero.global_position = torch.global_position + Vector3(expected_radius+1,0,0)
    torch.advance(0.25)
    if not require(not torch.dsg_0 and sound_events.size()==3 and sound_events[0]==sound_events[2] and torch.sound_emitter.voices.is_empty(), "source torch far/stop transition failed"): return
    torch.stop()
    hero.global_position = hero_position
    print("NATIVE_AI_OK ", JSON.stringify({"radius":expected_radius,"wait_ms":250,"events":sound_events,"independent_instances":2}))
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
