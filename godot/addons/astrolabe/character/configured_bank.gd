extends Node3D
## Channel-local warm instances of shared, fully converted native scenes.
## Construction is synchronous inside a hidden, physics-disabled bank. The
## actor prepares its rendering state before attaching to an animation anchor.
const CATALOG = preload("res://addons/astrolabe/character/configured_bank_catalog.gd")
const PAYLOAD = preload("res://addons/astrolabe/character/configured_payload.gd")
@export var catalog: CATALOG

func validate_configuration(identity: String, restored: bool, slot_count: int) -> String:
    if catalog == null or catalog.source_id != identity or catalog.restored != restored:
        return "Native bank catalogue identity/mode differs from the selected configuration."
    if catalog.payload_scenes.size() != slot_count:
        return "Native bank catalogue slot count differs from the selected configuration."
    if visible or process_mode != Node.PROCESS_MODE_DISABLED or not top_level:
        return "Native payload construction requires a hidden, disabled parking bank."
    if not has_meta("empty_payload_slots"):
        return "Native bank has no explicit absent-slot metadata."
    var absent: PackedInt32Array = get_meta("empty_payload_slots")
    var expected := PackedInt32Array()
    for slot in slot_count:
        if catalog.payload_scenes[slot] == null: expected.append(slot)
    if absent != expected:
        return "Native bank absent-slot metadata differs from its catalogue."
    for child in get_children():
        var payload := child as PAYLOAD
        if payload == null or payload.original_slot < 0 or payload.original_slot >= slot_count:
            return "Native bank contains an invalid payload identity."
        if catalog.payload_scenes[payload.original_slot] == null or str(payload.name) != "payload_%d" % payload.original_slot:
            return "Native bank payload disagrees with its catalogue."
    return ""

func acquire_payload(original_slot: int, actor: Node3D) -> Node3D:
    # The caller handles explicit absent slots before calling. A null result
    # here is a reported contract failure and must stop configuration binding.
    if catalog == null or original_slot < 0 or original_slot >= catalog.payload_scenes.size() or catalog.payload_scenes[original_slot] == null:
        push_error("Native payload request requires a nonempty catalogue slot.")
        return null
    if not actor.is_ancestor_of(self) or visible or process_mode != Node.PROCESS_MODE_DISABLED:
        push_error("Native payload request requires its actor's disabled bank.")
        return null
    var path := NodePath("payload_%d" % original_slot)
    var existing := get_node_or_null(path) as PAYLOAD
    if existing != null:
        if existing.original_slot == original_slot and existing.get_meta("source_configuration_bank") == actor.get_path_to(self):
            return existing
        push_error("Cached native payload has an inconsistent bank/slot binding.")
        return null
    if has_node(path):
        push_error("Native payload cache contains an incompatible named node.")
        return null
    var instance := catalog.payload_scenes[original_slot].instantiate()
    if instance == null:
        push_error("Native payload scene could not instantiate its saved root.")
        return null
    var payload := instance as PAYLOAD
    if payload == null or payload.original_slot != original_slot or str(payload.name) != str(path):
        instance.free()
        push_error("Native payload scene has an inconsistent root/slot identity.")
        return null
    payload.set_meta("source_configuration_bank", actor.get_path_to(self))
    # Tree entry applies current level material variants synchronously; bank
    # inheritance keeps all new contacts removed from physics until activation.
    add_child(payload)
    payload.owner = actor
    return payload
