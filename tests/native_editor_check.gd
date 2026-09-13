@tool
extends EditorPlugin
## Disposable acceptance probe: inspect the exported scene inside the real editor.

func _enter_tree(): call_deferred("check")

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        get_tree().quit(1)
    return ok

# Inspect actual Scene dock rows. Counting instantiated Nodes alone misses
# Godot's separate ownership/editable-instance filter in SceneTreeEditor.
func dock_rows(scene: Node) -> Dictionary:
    for widget in EditorInterface.get_base_control().find_children("*", "", true, false):
        if widget.get_class() != "SceneTreeEditor": continue
        for tree in widget.find_children("*", "Tree", true, false):
            var item = tree.get_root()
            if item == null or item.get_metadata(0) != scene.get_path(): continue
            var rows := {}
            while item != null:
                rows[item.get_metadata(0)] = true
                item = item.get_next_in_tree()
            return rows
    return {}

func check_scene_dock(file: String, level_path: NodePath) -> bool:
    EditorInterface.open_scene_from_path(file)
    var scene := EditorInterface.get_edited_scene_root()
    var deadline := Time.get_ticks_msec() + 10000
    while (scene == null or scene.scene_file_path != file) and Time.get_ticks_msec() < deadline:
        await get_tree().create_timer(0.05).timeout
        scene = EditorInterface.get_edited_scene_root()
    if not require(scene != null and scene.scene_file_path == file, "editor did not open " + file + "; current=" + str(scene) + ("" if scene == null else " path=" + scene.scene_file_path)): return false
    await get_tree().create_timer(0.3).timeout
    var rows := dock_rows(scene)
    var level := scene.get_node(level_path)
    var source_nodes := 0
    var persos := 0
    var parts := 0
    for node in level.find_children("*", "", true, false):
        if node.has_meta("source_node_id"): source_nodes += 1
        if node.has_meta("source_perso"): persos += 1
        if node.has_meta("configuration_slot"): parts += 1
        if not require(rows.has(node.get_path()), "Scene dock hides " + str(scene.get_path_to(node))): return false
    if not require(source_nodes == 150 and persos == 108 and parts == 899, "Scene dock content coverage differs"): return false
    var hero := level.get_node("world/0/0/Hype")
    if not require(not hero.scene_file_path.is_empty(), "Scene dock exposure unpacked the shared character"): return false
    EditorInterface.get_selection().clear()
    EditorInterface.get_selection().add_node(hero.get_node("Channels/channel_0"))
    print("NATIVE_SCENE_DOCK_OK ", JSON.stringify({"file":file,"rows":rows.size(),"source_nodes":source_nodes,"persos":persos,"parts":parts}))
    return true

func check():
    # Let startup restore its editor layout before issuing an explicit scene open.
    await get_tree().create_timer(0.5).timeout
    if not await check_scene_dock("res://generated/hype/levels/astrolabe/preview.tscn", ^"Model0"): return
    if not await check_scene_dock("res://generated/hype/levels/astrolabe/level.tscn", ^"."): return
    EditorInterface.open_scene_from_path("res://generated/hype/levels/astrolabe/persos/Hype__891c7341e3.tscn")
    var actor = EditorInterface.get_edited_scene_root()
    var deadline = Time.get_ticks_msec() + 10000
    while (actor == null or not actor.has_node("Channels")) and Time.get_ticks_msec() < deadline:
        await get_tree().create_timer(0.05).timeout
        actor = EditorInterface.get_edited_scene_root()
    if not require(actor != null and actor.has_node("Channels"), "editor did not open saved native character"): return
    var count = actor.find_children("*", "", true, false).size()
    var player = actor.get_node("AnimationPlayer") as AnimationPlayer
    EditorInterface.edit_node(player)
    var clip = player.get_animation("state_32")
    EditorInterface.inspect_object(clip)
    player.play("state_32")
    player.seek(0.0, true, true)
    player.pause()
    var channels = actor.get_node("Channels").get_children()
    var first = channels.map(func(n):return n.transform)
    player.seek(14.0/30.0, true, true)
    var moved = 0
    for i in channels.size():
        if not channels[i].transform.is_equal_approx(first[i]): moved += 1
    if not require(moved > 0, "editor seek did not update native pose tracks"): return
    player.seek(0.0, true, true)
    for i in channels.size():
        if not require(channels[i].transform.is_equal_approx(first[i]), "editor backward seek failed"): return
    if not require(count == actor.find_children("*", "", true, false).size(), "editor seeking constructed missing assets"): return
    print("NATIVE_EDITOR_OK ", JSON.stringify({"nodes":count,"moving_channels":moved,"animations":player.get_animation_list().size(),"editor_hint":Engine.is_editor_hint()}))
    get_tree().quit(0)
