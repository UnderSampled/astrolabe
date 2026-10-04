extends RefCounted
## Native manor checkpoint state. Schema 3 adds inventory/armor snapshots;
## older slots retain seeded values for fields their schema never represented.
## Source save addresses and raw SAI records never enter this file.

const SLOT := "user://manoir-save-v1.json"
# Keep the old payload key so existing consumers and schema-1 slots remain
# readable. Schema 2 binds its actor through the explicit useful asset URI.
const ACTOR := "world/0/0/Hype"
const PLAYER := "level:/persos/world/0/0"
const SCHEMA := 3
const INVENTORY = preload("res://addons/astrolabe/character/inventory.gd")
const ARMOR = preload("res://addons/astrolabe/character/armor_catalog.gd")
const CHECKPOINT := "manor-idle-v1"
const DUMMY := "level:/persos/world/0/27"


static func path() -> String:
    return SLOT


static func _integer(value: Variant, low: int, high: int) -> bool:
    if not (value is int or value is float):
        return false
    var number := float(value)
    if not is_finite(number) or number < float(low) or number > float(high): return false
    return float(int(value)) == number


static func _only_keys(value: Dictionary, supported: Array[String]) -> bool:
    for key in value:
        if not (key is String) or not supported.has(key):
            return false
    return true


static func _number(value: Variant) -> bool:
    return (value is int or value is float) and is_finite(float(value))


static func _vector(value: Variant) -> bool:
    if not value is Array or value.size() != 3 or not value.all(_number): return false
    # JSON numbers may exceed Godot Vector3 precision even when their parsed
    # double is finite. Validate the actual native representation as well.
    var converted := _vec(value)
    return is_finite(converted.x) and is_finite(converted.y) and is_finite(converted.z)


static func _placement(value: Variant) -> bool:
    if not value is Dictionary or not _only_keys(value, ["origin", "basis"]) or not _vector(value.get("origin")):
        return false
    var basis: Variant = value.get("basis")
    if not basis is Array or basis.size() != 3 or not basis.all(_vector):
        return false
    var determinant := _transform(value).basis.determinant()
    return is_finite(determinant) and absf(determinant) > 0.000001


static func _vec(value: Array) -> Vector3:
    return Vector3(float(value[0]), float(value[1]), float(value[2]))


static func _transform(value: Dictionary) -> Transform3D:
    var columns: Array = value["basis"]
    return Transform3D(Basis(_vec(columns[0]), _vec(columns[1]), _vec(columns[2])), _vec(value["origin"]))


static func _coordinates(value: Vector3) -> Array:
    return [value.x, value.y, value.z]


static func _has_properties(owner: Object, fields: Array[String]) -> bool:
    if not is_instance_valid(owner): return false
    var present := PackedStringArray()
    for property in owner.get_property_list(): present.append(str(property["name"]))
    for field in fields:
        if not present.has(field): return false
    return true


static func resolve_player(level: Node3D) -> CharacterBody3D:
    if not is_instance_valid(level): return null
    var actors: Variant = level.get_meta("source_actor_nodes", {})
    if not actors is Dictionary or not actors.has(PLAYER): return null
    var path: Variant = actors[PLAYER]
    if not (path is String or path is NodePath): return null
    var result := level.get_node_or_null(NodePath(path)) as CharacterBody3D
    if result == null or str(result.get_meta("source_perso", "")) != PLAYER: return null
    return result


static func checkpoint_error(actor: CharacterBody3D, level: Node3D) -> String:
    if not is_instance_valid(actor) or resolve_player(level) != actor:
        return "Checkpoint player identity does not match the converted manor."
    var brain := actor.get_node_or_null("Brain")
    if brain == null or int(brain.get("current_normal")) != 4 or int(brain.get("current_reflex")) != 3 or int(actor.get("current_state")) != 47:
        return "Save when Hype has returned to his supported idle state."
    if not bool(brain.get("running")) or not str(brain.get("unsupported_failure")).is_empty():
        return "A stopped or unsupported behavior cannot be saved as a checkpoint."
    if not actor.is_on_floor() or bool(actor.get("source_airborne")) or not _vector(_coordinates(actor.velocity)) or actor.velocity.length_squared() > 0.0001:
        return "Save when Hype is standing still on the ground."
    var external: Variant = actor.get("external_velocity")
    if not external is Vector3 or not _vector(_coordinates(external)) or external.length_squared() > 0.0001 or int(actor.get("pending_state")) >= 0:
        return "Save after movement and action transitions have settled."
    var wind: Variant = actor.get("source_wind_drift")
    if not wind is Vector3 or not _vector(_coordinates(wind)) or wind.length_squared() > 0.0001:
        return "Save after persistent wind drift has stopped."
    if is_instance_valid(actor.get("generated_object")):
        return "Save after the held projectile has been released and removed."
    for candidate in level.find_children("*", "CharacterBody3D", true, false):
        if candidate.has_meta("astrolabe_generated_template"):
            return "Save after active generated projectiles have been removed."
    for action in InputMap.get_actions():
        if (str(action).begins_with("Button_") or str(action).begins_with("BT_Camera_")) and Input.is_action_pressed(action):
            return "Release gameplay controls before saving a checkpoint."
    return ""


static func validate(value: Variant) -> String:
    if not value is Dictionary:
        return "Save is not an object."
    if not _only_keys(value, ["schema", "level", "actors", "world", "checkpoint"]):
        return "Save has unsupported top-level state."
    if not _integer(value.get("schema"), 1, SCHEMA):
        return "Save version is unsupported."
    var schema := int(value["schema"])
    if schema == 1 and value.has("checkpoint"):
        return "Legacy save has unsupported checkpoint state."
    if schema >= 2 and value.get("checkpoint") != CHECKPOINT:
        return "Save checkpoint is unsupported."
    if value.get("level") != "manoir":
        return "Save level is unsupported."
    var actors: Variant = value.get("actors")
    if not actors is Dictionary or not actors.has(ACTOR) or not actors[ACTOR] is Dictionary:
        return "Save is missing the manor player."
    if not _only_keys(actors, [ACTOR]):
        return "Save has unsupported actors."
    var player: Dictionary = actors[ACTOR]
    var fields: Array[String] = ["magick", "inventory"]
    if schema >= 2: fields.append_array(["asset", "placement", "life"])
    if schema >= 3: fields.append("armor")
    if not _only_keys(player, fields):
        return "Save has unsupported player state."
    if schema >= 2:
        if player.get("asset") != PLAYER:
            return "Save player asset is unsupported."
        if not _placement(player.get("placement")):
            return "Save has invalid player placement."
        var life: Variant = player.get("life")
        if not life is Dictionary or not _only_keys(life, ["current", "maximum", "limit"]):
            return "Save has invalid player life."
        if not _integer(life.get("current"), 1, 255) or not _integer(life.get("maximum"), 1, 255) or not _integer(life.get("limit"), 1, 255):
            return "Save has invalid player life values."
        if int(life["current"]) > int(life["maximum"]) or int(life["maximum"]) > int(life["limit"]):
            return "Save player life exceeds its maximum or cap."
    var magick: Variant = player.get("magick")
    if not magick is Dictionary:
        return "Save is missing player magick."
    if not _only_keys(magick, ["current_points", "maximum_points", "selected_spell_code"]):
        return "Save has unsupported magick state."
    if not _integer(magick.get("current_points"), 0, 32767) or not _integer(magick.get("maximum_points"), 1, 32767) or not _integer(magick.get("selected_spell_code"), 0, 32767):
        return "Save has invalid magick values."
    if int(magick["current_points"]) > int(magick["maximum_points"]):
        return "Save current mana exceeds maximum mana."
    var inventory: Variant = player.get("inventory")
    if schema >= 3:
        var snapshot_error := INVENTORY.validate_snapshot_shape(inventory)
        if not snapshot_error.is_empty(): return snapshot_error
        snapshot_error = ARMOR.validate_snapshot_shape(player.get("armor"))
        if not snapshot_error.is_empty(): return snapshot_error
    elif not inventory is Dictionary or not _integer(inventory.get("selected_weapon"), -1, 32767):
        return "Save has invalid inventory selection."
    if schema < 3 and not _only_keys(inventory, ["selected_weapon", "quantities"]):
        return "Save has unsupported inventory state."
    var quantities: Variant = inventory.get("quantities")
    if not quantities is Dictionary:
        return "Save is missing inventory quantities."
    for item in quantities:
        # The measured source inventory and native quantity API retain -1 as
        # their absent/unavailable sentinel. Preserve it through native saves.
        if not item is String or item.length() > 5 or not str(item).is_valid_int() or str(int(item)) != item or not _integer(int(item), 0, 32767) or not _integer(quantities[item], -32768 if schema >= 3 else -1, 32767):
            return "Save has an invalid inventory quantity."
    if schema >= 2 and (not value.get("world") is Dictionary or not value["world"].has(DUMMY)):
        return "Save is missing the supported manor world flag."
    # Older schema-1 slots predate world flags; missing world state starts from
    # the measured manor seed. When present, validate the complete supported
    # native flag before applying any part of the save.
    if value.has("world"):
        var world: Variant = value["world"]
        if not world is Dictionary:
            return "Save has invalid world state."
        if world.has(DUMMY):
            var dummy: Variant = world[DUMMY]
            if not dummy is Dictionary or not dummy.has("dsg_8") or not dummy["dsg_8"] is bool:
                return "Save has invalid manor lever state."
            if not _only_keys(dummy, ["dsg_8"]):
                return "Save has unsupported manor world state."
        for identity in world:
            if identity != DUMMY:
                return "Save has unsupported world state."
    return ""


static func read() -> Dictionary:
    if not FileAccess.file_exists(SLOT):
        return {"status": "missing"}
    var file := FileAccess.open(SLOT, FileAccess.READ)
    if file == null:
        return {"status": "error", "message": "Save cannot be opened."}
    if file.get_length() > 1048576:
        return {"status": "error", "message": "Save exceeds the supported size."}
    var parsed: Variant = JSON.parse_string(file.get_as_text())
    file.close()
    var error := validate(parsed)
    if not error.is_empty():
        return {"status": "error", "message": error}
    return {"status": "loaded", "state": parsed}


static func _dummy_brain(level: Node3D) -> Node:
    if level == null:
        return null
    var source_brains: Variant = level.get_meta("source_actor_brains", {})
    if not source_brains is Dictionary or not source_brains.has(DUMMY):
        return null
    return level.get_node_or_null(source_brains[DUMMY]) as Node


static func capture(actor: CharacterBody3D, level: Node3D) -> Dictionary:
    if not is_instance_valid(actor): return {}
    var magick: Resource = actor.get("source_magick")
    var inventory: Resource = actor.get("source_inventory")
    var armor: Resource = actor.get("source_armor")
    var dummy_brain := _dummy_brain(level)
    var life: Resource = actor.get("source_hit_points")
    if resolve_player(level) != actor or magick == null or inventory == null or armor == null or life == null or dummy_brain == null or not dummy_brain.get("dsg_8") is bool:
        return {}
    if not inventory.has_method("capture_snapshot") or not armor.has_method("capture_snapshot"): return {}
    var placement := actor.global_transform
    return {
        "schema": SCHEMA,
        "level": "manoir",
        "checkpoint": CHECKPOINT,
        "actors": {ACTOR: {
            "asset": PLAYER,
            "placement": {
                "origin": _coordinates(placement.origin),
                "basis": [_coordinates(placement.basis.x), _coordinates(placement.basis.y), _coordinates(placement.basis.z)],
            },
            "life": {"current": life.get("current"), "maximum": life.get("maximum"), "limit": life.get("limit")},
            "magick": {
                "current_points": int(magick.get("current_points")),
                "maximum_points": int(magick.get("maximum_points")),
                "selected_spell_code": int(magick.get("selected_spell_code")),
            },
            "inventory": inventory.call("capture_snapshot"),
            "armor": armor.call("capture_snapshot"),
        }},
        "world": {DUMMY: {"dsg_8": bool(dummy_brain.get("dsg_8"))}},
    }


static func write(actor: CharacterBody3D, level: Node3D) -> String:
    var error := checkpoint_error(actor, level)
    if not error.is_empty(): return error
    var state := capture(actor, level)
    error = validate(state)
    if not error.is_empty():
        return error
    # Shape validation also serves standalone reads. Publication additionally
    # checks this native graph so an invalid authored selection/binding cannot
    # replace a slot that would otherwise still load successfully.
    var player: Dictionary = state["actors"][ACTOR]
    for field in ["inventory", "armor"]:
        var owner: Resource = actor.get("source_" + field)
        if owner == null or not owner.has_method("validate_snapshot"):
            return "Converted equipment has incompatible snapshot support."
        error = owner.call("validate_snapshot", player[field])
        if not error.is_empty(): return error
    var temporary := SLOT + "." + str(Time.get_ticks_usec()) + ".tmp"
    var file := FileAccess.open(temporary, FileAccess.WRITE)
    if file == null:
        return "Save cannot be written."
    file.store_string(JSON.stringify(state) + "\n")
    file.flush()
    error = "" if file.get_error() == OK else "Save write failed."
    file.close()
    if error.is_empty() and DirAccess.rename_absolute(ProjectSettings.globalize_path(temporary), ProjectSettings.globalize_path(SLOT)) != OK:
        error = "Save could not replace the previous slot."
    if not error.is_empty():
        DirAccess.remove_absolute(ProjectSettings.globalize_path(temporary))
    return error


static func apply(actor: CharacterBody3D, level: Node3D, value: Dictionary) -> String:
    var error := validate(value)
    if not error.is_empty():
        return error
    if not is_instance_valid(actor) or resolve_player(level) != actor:
        return "Saved player identity differs from the converted manor."
    var magick: Resource = actor.get("source_magick")
    var inventory: Resource = actor.get("source_inventory")
    var dummy_brain := _dummy_brain(level)
    var life: Resource = actor.get("source_hit_points")
    if magick == null or inventory == null:
        return "Converted player has no native magick or inventory."
    if not _has_properties(magick, ["current_points", "maximum_points", "selected_spell_code"]) or not _has_properties(inventory, ["selected_weapon", "quantities"]):
        return "Converted player state has incompatible native properties."
    if int(value["schema"]) >= 2 and not _has_properties(life, ["current", "maximum", "limit"]):
        return "Converted player has no native life state."
    if dummy_brain == null:
        return "Converted manor has no saved dummy world flag."
    if not dummy_brain.get("dsg_8") is bool:
        return "Converted manor world flag has an unsupported type."
    var player: Dictionary = value["actors"][ACTOR]
    var saved_magick: Dictionary = player["magick"]
    var saved_inventory: Dictionary = player["inventory"]
    var quantities: Dictionary = {}
    for item in saved_inventory["quantities"]:
        quantities[int(item)] = int(saved_inventory["quantities"][item])
    if not inventory.has_method("clone_for_scene") or not inventory.has_method("restore_legacy"):
        return "Converted inventory has incompatible checkpoint support."
    var schema := int(value["schema"])
    var armor: Resource = null
    if schema >= 3:
        armor = actor.get("source_armor") as Resource
        if armor == null: return "Converted player has no native armor catalogue."
        for owner in [inventory, armor]:
            for method in ["clone_for_scene", "validate_snapshot", "apply_snapshot"]:
                if not owner.has_method(method): return "Converted equipment has incompatible snapshot support."
        var inventory_error: String = inventory.call("validate_snapshot", saved_inventory)
        if not inventory_error.is_empty(): return inventory_error
        var armor_error: String = armor.call("validate_snapshot", player["armor"])
        if not armor_error.is_empty(): return armor_error
        var restored_inventory: Resource = inventory.call("clone_for_scene")
        if restored_inventory == null or not restored_inventory.has_method("apply_snapshot"): return "Converted inventory clone has incompatible snapshot support."
        inventory_error = restored_inventory.call("apply_snapshot", saved_inventory)
        if not inventory_error.is_empty(): return inventory_error
        var restored_armor: Resource = armor.call("clone_for_scene")
        if restored_armor == null or not restored_armor.has_method("apply_snapshot"): return "Converted armor clone has incompatible snapshot support."
        armor_error = restored_armor.call("apply_snapshot", player["armor"])
        if not armor_error.is_empty(): return armor_error
    else:
        if not inventory.has_method("capture_snapshot") or not inventory.has_method("validate_snapshot"):
            return "Converted inventory has incompatible legacy snapshot support."
        var native_inventory_error: String = inventory.call("validate_snapshot", inventory.call("capture_snapshot"))
        if not native_inventory_error.is_empty(): return native_inventory_error
        var restored_inventory: Resource = inventory.call("clone_for_scene")
        if restored_inventory == null or not restored_inventory.has_method("restore_legacy"): return "Converted inventory clone has incompatible checkpoint support."
        var inventory_error: String = restored_inventory.call("restore_legacy", int(saved_inventory["selected_weapon"]), quantities)
        if not inventory_error.is_empty(): return inventory_error
    # Resolve and validate every owner before mutating any state. Placement and
    # life are restored after fixture seeding, before physics/Brain startup.
    if schema >= 2:
        actor.global_transform = _transform(player["placement"])
        for field in ["current", "maximum", "limit"]: life.set(field, int(player["life"][field]))
    magick.set("current_points", int(saved_magick["current_points"]))
    magick.set("maximum_points", int(saved_magick["maximum_points"]))
    magick.set("selected_spell_code", int(saved_magick["selected_spell_code"]))
    if schema >= 3:
        inventory.call("apply_snapshot", saved_inventory)
        armor.call("apply_snapshot", player["armor"])
    else:
        inventory.call("restore_legacy", int(saved_inventory["selected_weapon"]), quantities)
    var world: Dictionary = value.get("world", {})
    var dummy_state: Dictionary = world.get(DUMMY, {"dsg_8": false})
    dummy_brain.set("dsg_8", bool(dummy_state["dsg_8"]))
    return ""
