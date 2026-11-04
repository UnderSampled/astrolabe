extends SceneTree
## Disposable probe: placed actors share native PackedScenes while keeping
## independent per-instance state. Usage:
##   godot --headless --path godot -s res://tests/actor_instancing_check.gd -- <level> <perso name>...

var failures := PackedStringArray()

func check(ok: bool, message: String) -> void:
    if not ok: failures.append(message)

func _initialize() -> void:
    call_deferred("run")

func run() -> void:
    var args := OS.get_cmdline_user_args()
    var level_name := args[0] if args.size() > 0 else "manoir"
    var names := args.slice(1) if args.size() > 1 else PackedStringArray(["i_Mannequin_Arbalete", "i_Mannequin_Epee", "i_Mannequin_Magie"])
    var path := "res://generated/hype/levels/%s/level.tscn" % level_name
    var packed := load(path) as PackedScene
    var level := packed.instantiate()
    root.add_child(level)
    for i in 5: await process_frame
    var actors := []
    for name in names:
        var found := level.find_children(name, "", true, false)
        check(found.size() == 1, "expected one %s, found %d" % [name, found.size()])
        if found.size() == 1: actors.append(found[0])
    if actors.size() != names.size():
        finish({})
        return
    var scene_files := {}
    var report := {}
    for actor in actors:
        scene_files[actor.scene_file_path] = true
        # Ownership: internals belong to the actor scene, not the level.
        var internals: Array[Node] = actor.find_children("*", "", true, false)
        var level_owned := 0
        for node in internals:
            if node.owner == level: level_owned += 1
        check(level_owned == 0, "%s has %d level-owned internal nodes" % [actor.name, level_owned])
        check(actor.owner == level, "%s root is not level-owned" % actor.name)
        var brain: Node = actor.get_node_or_null("Brain")
        var entry := {"scene": actor.scene_file_path.get_file(), "internal_nodes": internals.size(), "source_perso": str(actor.get_meta("source_perso", ""))}
        if brain != null:
            entry["dsg_0"] = brain.get("dsg_0")
            entry["root_dsg_0"] = actor.get("brain_dsg_0")
            entry["current_normal"] = brain.get("current_normal")
            check(brain.get("dsg_0") == actor.get("brain_dsg_0"), "%s Brain dsg_0 not forwarded" % actor.name)
            var main: Node = brain.get("main_actor")
            entry["main_actor"] = str(main.name) if main != null else ""
            check(main != null and main.get_meta("source_perso", "") == level.get_meta("source_main_actor", ""), "%s main actor hook unresolved" % actor.name)
            if "actor_brain_0_path" in brain and not str(brain.get("actor_brain_0_path")).is_empty():
                var target: Node = brain.get_node_or_null(brain.get("actor_brain_0_path"))
                entry["actor_brain_0"] = str(target.get_path()) if target != null else ""
                check(target != null and target.name == "Brain", "%s actor_brain_0 hook unresolved" % actor.name)
            var index: Dictionary = level.get_meta("source_actor_brains", {})
            check(level.get_node_or_null(index.get(entry.source_perso, NodePath())) == brain, "%s level Brain index stale" % actor.name)
        var player: AnimationPlayer = actor.get_node_or_null("AnimationPlayer") as AnimationPlayer
        if player != null:
            entry["animations"] = player.get_animation_list().size()
            entry["current_animation"] = str(player.current_animation)
            check(player.get_animation_list().size() > 0, "%s has no animations" % actor.name)
        var shapes := 0
        for shape in actor.find_children("*", "CollisionShape3D", true, false):
            if shape.shape != null: shapes += 1
        entry["collision_shapes"] = shapes
        entry["global_position"] = var_to_str(actor.global_position)
        entry["hit_points"] = actor.source_hit_points.current if actor.source_hit_points != null else null
        report[str(actor.name)] = entry
    check(scene_files.size() == 1 or names.size() == 1, "actors do not share one scene: " + str(scene_files.keys()))
    # Independence: mutable per-instance state must not alias.
    var a: Node = actors[0]
    var others := actors.slice(1)
    if a.source_hit_points != null:
        var before := []
        for other in others: before.append(other.source_hit_points.current)
        a.source_hit_points.current -= 1
        for i in others.size():
            check(others[i].source_hit_points != a.source_hit_points, "hit points resource shared with " + others[i].name)
            check(others[i].source_hit_points.current == before[i], "hit point change leaked to " + others[i].name)
        a.source_hit_points.current += 1
    var brain_a: Node = a.get_node_or_null("Brain")
    if brain_a != null:
        var before := []
        for other in others: before.append(other.get_node("Brain").get("dsg_0"))
        brain_a.set("dsg_0", 77)
        for i in others.size():
            check(others[i].get_node("Brain").get("dsg_0") == before[i], "Brain dsg_0 change leaked to " + others[i].name)
    var positions := {}
    for actor in actors: positions[var_to_str(actor.global_position)] = true
    check(positions.size() == actors.size(), "placed actors share a position")
    finish(report)

func finish(report: Dictionary) -> void:
    print("ACTOR_INSTANCING ", JSON.stringify({"ok": failures.is_empty(), "failures": failures, "actors": report}))
    quit(0 if failures.is_empty() else 1)
