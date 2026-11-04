@tool
extends EditorPlugin
## Disposable editor probe: placed actors appear as instance roots in a level,
## their internals stay in the shared actor scene, and an editor re-save keeps
## per-instance parameters. Writes res://actor_editor_check_copy.tscn.

const LEVEL := "res://generated/hype/levels/manoir/level.tscn"
const COPY := "res://actor_editor_check_copy.tscn"
const MANNEQUINS := ["i_Mannequin_Arbalete", "i_Mannequin_Epee", "i_Mannequin_Magie"]

func _enter_tree(): call_deferred("check")

func fail(message: String) -> bool:
    print("ACTOR_EDITOR_FAIL ", message)
    get_tree().quit(1)
    return false

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

func open(file: String) -> Node:
    EditorInterface.open_scene_from_path(file)
    var deadline := Time.get_ticks_msec() + 20000
    var scene := EditorInterface.get_edited_scene_root()
    while (scene == null or scene.scene_file_path != file) and Time.get_ticks_msec() < deadline:
        await get_tree().create_timer(0.05).timeout
        scene = EditorInterface.get_edited_scene_root()
    await get_tree().create_timer(0.5).timeout
    return scene

func check():
    await get_tree().create_timer(0.5).timeout
    var level := await open(LEVEL)
    if level == null or level.scene_file_path != LEVEL: return fail("level did not open")
    var rows := dock_rows(level)
    var all_nodes := level.find_children("*", "", true, false)
    var report := {"level_rows": rows.size(), "level_nodes_loaded": all_nodes.size(), "mannequins": {}}
    var shared := {}
    for name in MANNEQUINS:
        var found := level.find_children(name, "", true, false)
        if found.size() != 1: return fail("missing " + name)
        var actor: Node = found[0]
        shared[actor.scene_file_path] = true
        if not rows.has(actor.get_path()): return fail(name + " root hidden in level dock")
        var hidden := 0
        for node in actor.find_children("*", "", true, false):
            if rows.has(node.get_path()): return fail(name + " exposes internal " + str(actor.get_path_to(node)))
            hidden += 1
        var exported := false
        for property in actor.get_property_list():
            if property.name == "brain_dsg_0" and property.usage & PROPERTY_USAGE_EDITOR: exported = true
        if not exported: return fail(name + " lacks inspector-visible brain_dsg_0")
        report.mannequins[name] = {"scene": actor.scene_file_path.get_file(), "hidden_internals": hidden, "brain_dsg_0": actor.get("brain_dsg_0"), "editable_children": level.is_editable_instance(actor)}
    if shared.size() != 1: return fail("mannequins use several scenes " + str(shared.keys()))
    # Designer edit on one placed actor, saved by the editor to a copy.
    var epee: Node = level.find_children("i_Mannequin_Epee", "", true, false)[0]
    epee.set("brain_dsg_0", 9)
    EditorInterface.save_scene_as(COPY, false)
    await get_tree().create_timer(1.0).timeout
    var text := FileAccess.get_file_as_string(COPY)
    report["copy_editable_flags"] = text.count("[editable path=")
    var sections := {}
    for name in MANNEQUINS:
        var start := text.find('[node name="%s"' % name)
        var stop := text.find("\n[", start + 1)
        sections[name] = text.substr(start, stop - start)
    if not sections["i_Mannequin_Epee"].contains("brain_dsg_0 = 9"): return fail("editor save dropped Epee brain_dsg_0 override")
    if not sections["i_Mannequin_Magie"].contains("brain_dsg_0 = 3"): return fail("editor save lost another mannequin's value")
    if not sections["i_Mannequin_Arbalete"].contains("brain_main_actor_path"): return fail("editor save dropped AI hook path")
    report["copy_mannequin_sections"] = sections
    # Opening the shared scene exposes its internals for editing.
    var actor_scene := await open(shared.keys()[0])
    if actor_scene == null: return fail("shared mannequin scene did not open")
    var actor_rows := dock_rows(actor_scene)
    for child in ["Brain", "AnimationPlayer", "Channels", "ZDE", "ZDR"]:
        if not actor_rows.has(actor_scene.get_node(child).get_path()): return fail("shared scene hides " + child)
    report["shared_scene_rows"] = actor_rows.size()
    print("ACTOR_EDITOR_OK ", JSON.stringify(report))
    get_tree().quit(0)
