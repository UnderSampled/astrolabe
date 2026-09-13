@tool
extends EditorPlugin
## Disposable acceptance probe: inspect the exported scene inside the real editor.

func _enter_tree(): call_deferred("check")

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        get_tree().quit(1)
    return ok

func check():
    # Let startup restore its editor layout before issuing an explicit scene open.
    await get_tree().create_timer(0.2).timeout
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
