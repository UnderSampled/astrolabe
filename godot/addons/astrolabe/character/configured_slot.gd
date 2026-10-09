@tool
extends Node3D
## Stable animation target; its saved native child carries the mutable content.
## Only visible selected payloads participate in physical-object fallback.
## Independent actor ZDE/ZDR zones do not belong to these visual slots.
## Godot transparency: zero is opaque, unlike source opacity's 255.
@export var transparency: float = 0.0:
    set(value):
        var next := clampf(value, 0.0, 1.0)
        if transparency == next: return
        transparency = next
        refresh_transparency()

func _enter_tree() -> void:
    if not visibility_changed.is_connected(refresh_participation):
        visibility_changed.connect(refresh_participation)
    refresh_participation()

func refresh_participation() -> void:
    var active := visible
    if is_inside_tree():
        active = is_visible_in_tree()
    else:
        # The converter saves native nodes outside the SceneTree. Respect the
        # initial channel/actor visibility there as well as the anchor's key.
        var ancestor := get_parent() as Node3D
        while active and ancestor != null:
            active = ancestor.visible
            ancestor = ancestor.get_parent() as Node3D
    # REMOVE on the contained CollisionObject3Ds unregisters their shapes from
    # the physics space. Hiding a Node3D alone does not do that. Suspend before
    # changing transforms, and restore transforms before rejoining physics.
    if not active:
        process_mode = Node.PROCESS_MODE_DISABLED
    if top_level == active:
        # Keep the payload's local placement, including authored anchor edits.
        # A suspended slot has no useful world pose; retain its local transform
        # in the frozen top-level transform instead of following every sample.
        var local := transform
        top_level = not active
        transform = local
    if active:
        if process_mode == Node.PROCESS_MODE_DISABLED and is_inside_tree():
            # Publish the restored pose before inserting these non-monitoring
            # contact areas back into the space, including same-tick AI queries.
            for payload in get_children():
                var contact := payload.get_node_or_null("PhysicalContact") as CollisionObject3D
                # During this slot's _enter_tree its children have not entered
                # yet. Their native enter-world hook will publish that pose.
                if contact != null and contact.is_inside_tree():
                    contact.force_update_transform()
        process_mode = Node.PROCESS_MODE_INHERIT

func refresh_transparency() -> void:
    for geometry in find_children("*", "GeometryInstance3D", true, false):
        (geometry as GeometryInstance3D).transparency = transparency

func attach_payload(payload: Node3D) -> void:
    # Material/display state is prepared in the hidden bank. Suspend this
    # anchor before reparenting, then publish pose/transparency before its
    # REMOVE-mode contacts rejoin physics, including same-tick source queries.
    process_mode = Node.PROCESS_MODE_DISABLED
    payload.reparent(self, false)
    refresh_transparency()
    refresh_participation()
