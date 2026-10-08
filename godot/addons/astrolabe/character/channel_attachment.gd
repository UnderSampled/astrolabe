extends RefCounted
## Child-owned native hierarchy attachment. The level still owns actor identity.
var world_parent: Node3D

func change(child: Node3D, parent: Node3D, channel_id: int, attached: bool) -> bool:
    if not is_instance_valid(child) or not is_instance_valid(parent) or not child.is_inside_tree() or not parent.is_inside_tree():
        return false
    var channels := parent.get_node_or_null("Channels")
    if channels == null: return false
    var channel: Node3D
    for candidate in channels.get_children():
        var identity := int(candidate.get_meta("source_channel", -1)) + 256 * int(candidate.get_meta("source_channel_occurrence", 0))
        if identity == (channel_id & 0xffff):
            channel = candidate as Node3D
            break
    if channel == null or child == parent or child.is_ancestor_of(channel): return false
    if attached:
        if not is_instance_valid(world_parent):
            var ancestor := child.get_parent()
            while ancestor != null:
                if int(ancestor.get_meta("source_node_kind", -1)) == 0:
                    world_parent = ancestor as Node3D
                    break
                ancestor = ancestor.get_parent()
        if not is_instance_valid(world_parent): return false
        if not _move(child, channel, false): return false
        # Source copies the channel's whole world matrix, then changes father.
        child.transform = Transform3D.IDENTITY
        return true
    # Removing a child that is absent from this exact channel is a source no-op.
    if child.get_parent() != channel: return true
    if not is_instance_valid(world_parent): return false
    return _move(child, world_parent, true)

func _move(child: Node3D, destination: Node3D, keep_world: bool) -> bool:
    var level: Node = child
    while level != null and not level.has_meta("source_actor_nodes"):
        level = level.get_parent()
    if level == null or not level.is_ancestor_of(destination): return false
    # Packed scenes store NodePaths. Capture their native objects before moving,
    # then refresh paths so compiled AI and newly generated actors keep resolving.
    var indices: Dictionary = {}
    for field in ["source_actor_nodes", "source_actor_brains"]:
        var bound: Dictionary = {}
        var paths: Dictionary = level.get_meta(field, {})
        for key in paths:
            bound[key] = level.get_node_or_null(paths[key])
        indices[field] = bound
    var entries: Dictionary = {}
    var entry_paths: Dictionary = level.get_meta("source_actor_entries", {})
    for key in entry_paths:
        var nodes: Array[Node] = []
        for path in entry_paths[key]: nodes.append(level.get_node_or_null(path))
        entries[key] = nodes
    # _enter_tree uses the state owner's path; an absolute path stays valid
    # during reparenting, then the ordinary relative property is restored.
    child.set("source_state_owner_path", level.get_path())
    var sector_path: NodePath = child.get("source_contact_sector_path")
    var sector := child.get_node_or_null(sector_path) if not sector_path.is_empty() else null
    if sector != null: child.set("source_contact_sector_path", sector.get_path())
    var brains: Array[Node] = []
    for node in child.find_children("*", "", true, false):
        if node.has_method("source_begin_hierarchy_move"):
            brains.append(node)
            node.call("source_begin_hierarchy_move")
    child.reparent(destination, keep_world)
    for brain in brains: brain.call("source_end_hierarchy_move")
    child.set("source_state_owner_path", child.get_path_to(level))
    if sector != null: child.set("source_contact_sector_path", child.get_path_to(sector))
    for field in indices:
        var paths: Dictionary = {}
        for key in indices[field]:
            var node: Node = indices[field][key]
            if is_instance_valid(node): paths[key] = level.get_path_to(node)
        level.set_meta(field, paths)
    for key in entries:
        var paths: Array[NodePath] = []
        for node in entries[key]:
            if is_instance_valid(node): paths.append(level.get_path_to(node))
        entry_paths[key] = paths
    level.set_meta("source_actor_entries", entry_paths)
    return true
