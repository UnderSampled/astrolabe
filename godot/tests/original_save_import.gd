extends SceneTree
## Validate the actual CLI import artifact before opening the full generated game.
const SAVE_STATE = preload("res://game/save_state.gd")
const SAVE_PERSISTENCE_TEST = preload("res://tests/save_persistence.gd")

func _initialize() -> void:
    call_deferred("run")

func run() -> void:
    if SAVE_PERSISTENCE_TEST == null:
        fail("Could not load the native persistence regression script")
        return
    if not FileAccess.file_exists(SAVE_STATE.ORIGINAL_IMPORT):
        fail("Import a local SaveGame0 with astrolabe import-save first")
        return
    var file := FileAccess.open(SAVE_STATE.ORIGINAL_IMPORT, FileAccess.READ)
    if file == null:
        fail("Could not open the local original-save import")
        return
    var report: Variant = JSON.parse_string(file.get_as_text())
    file.close()
    var error := SAVE_STATE.validate_original_import(report)
    if not error.is_empty():
        fail(error)
        return
    var state: Dictionary = report["godot_state"]
    var global_flag: Variant = state["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_149"]
    if typeof(global_flag) != TYPE_BOOL or not global_flag:
        fail("Authentic manor checkpoint must retain Boolean Global DSG[149]")
        return
    var saved_currency: Variant = state["actors"][SAVE_STATE.ACTOR]["inventory"]["quantities"].get("0")
    if typeof(saved_currency) != TYPE_INT or int(saved_currency) != 20:
        fail("Authentic SaveGame0 item-0 balance must restore its saved value 20")
        return
    var semantic_items: Variant = report.get("semantic_inventory_items")
    if not semantic_items is Array or semantic_items.size() != 1:
        fail("Original import is missing its semantic inventory item projection")
        return
    var semantic_item: Variant = semantic_items[0]
    if (
        not semantic_item is Dictionary
        or int(semantic_item.get("item_id", -1)) != 0
        or int(semantic_item.get("quantity", -1)) != 20
        or int(semantic_item.get("capacity", -1)) != 200
    ):
        fail("Original import item-0 semantic value/capacity differs")
        return
    if report.has("source_save"):
        fail("Gameplay import must not embed raw source-save records")
        return
    var gameplay_json := JSON.stringify(report)
    for forbidden in ["offset", "block", "module", "relocation", "5790", "5828"]:
        if gameplay_json.contains(forbidden):
            fail("Gameplay import leaked source diagnostic data: " + forbidden)
            return
    if report["restored_fields"].is_empty():
        fail("Gameplay import did not retain its named restored field summary")
        return
    var transform: Dictionary = state["actors"][SAVE_STATE.ACTOR]["transform"]
    if not transform.has("position") or transform["position"].size() != 3:
        fail("Authentic manor checkpoint is missing Hype's source position")
        return
    var stale: Dictionary = report.duplicate(true)
    stale["godot_state"]["actors"].erase(SAVE_STATE.GLOBAL)
    if SAVE_STATE.validate_original_import(stale).is_empty():
        fail("Original import without the gate sequence flag was accepted")
        return
    var actor := CharacterBody3D.new()
    actor.set_script(load("res://addons/astrolabe/character/character.gd"))
    actor.set_process(false)
    actor.set_physics_process(false)
    root.add_child(actor)
    actor.global_position = Vector3(1.0, 2.0, 3.0)
    var target_source_position: Vector3 = actor.source_position() + Vector3(2.6, 0.0, 0.0)
    if not bool(actor.source_set_absolute_position(target_source_position)) or absf(actor.global_position.x - 3.6) > 0.0001 or absf(actor.global_position.y - 2.0) > 0.0001 or absf(actor.global_position.z - 3.0) > 0.0001:
        fail("Native self-position API did not apply the source-space gate displacement")
        return
    print("ORIGINAL_SAVE_IMPORT_PROOF SaveGame0 restores item-0 balance=20, Hype transform and Global DSG[149]=true; stale import rejected; source position API moved +2.6 X")
    actor.free()
    quit(0)

func fail(message: String) -> void:
    push_error(message)
    quit(1)
