extends RefCounted
## Versioned native game state for the currently supported manor player slice.
## Source save addresses and raw SAI records never enter this file.

const SLOT := "user://manoir-save-v1.json"
const ORIGINAL_IMPORT := "res://game/original_save_import.json"
const ACTOR := "world/0/0/Hype"
const DUMMY := "level:/persos/world/0/27"
const GLOBAL := "fix:/persos/15"


static func path() -> String:
    return SLOT


static func _integer(value: Variant, low: int, high: int) -> bool:
    if not (value is int or value is float):
        return false
    return float(int(value)) == float(value) and int(value) >= low and int(value) <= high


static func _only_keys(value: Dictionary, supported: Array[String]) -> bool:
    for key in value:
        if not (key is String) or not supported.has(key):
            return false
    return true


static func _valid_vector(value: Variant, limit: float) -> bool:
    if not value is Array or value.size() != 3:
        return false
    for component in value:
        if not (component is int or component is float) or not is_finite(float(component)) or absf(float(component)) > limit:
            return false
    return true


static func _as_vector(value: Array) -> Vector3:
    return Vector3(float(value[0]), float(value[1]), float(value[2]))


static func _has_property(object: Object, property_name: String) -> bool:
    if object == null:
        return false
    return object.get_property_list().any(func(property): return str(property["name"]) == property_name)


static func validate(value: Variant) -> String:
    if not value is Dictionary:
        return "Save is not an object."
    if not _only_keys(value, ["schema", "level", "actors", "world"]):
        return "Save has unsupported top-level state."
    if not _integer(value.get("schema"), 0, 2147483647) or int(value["schema"]) != 1:
        return "Save version is unsupported."
    if value.get("level") != "manoir":
        return "Save level is unsupported."
    var actors: Variant = value.get("actors")
    if not actors is Dictionary or not actors.has(ACTOR) or not actors[ACTOR] is Dictionary:
        return "Save is missing the manor player."
    if not _only_keys(actors, [ACTOR, GLOBAL]):
        return "Save has unsupported actors."
    var player: Dictionary = actors[ACTOR]
    if not _only_keys(player, ["magick", "inventory", "transform"]):
        return "Save has unsupported player state."
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
    if not inventory is Dictionary:
        return "Save has invalid inventory selection."
    if not _only_keys(inventory, ["selected_weapon", "quantities"]):
        return "Save has unsupported inventory state."
    if inventory.has("selected_weapon") and not _integer(inventory.get("selected_weapon"), -1, 32767):
        return "Save has invalid inventory selection."
    var quantities: Variant = inventory.get("quantities")
    if not quantities is Dictionary:
        return "Save is missing inventory quantities."
    for item in quantities:
        # The measured source inventory and native quantity API retain -1 as
        # their absent/unavailable sentinel. Preserve it through native saves.
        if not item is String or not str(item).is_valid_int() or str(int(item)) != item or not _integer(int(item), 0, 32767) or not _integer(quantities[item], -1, 32767):
            return "Save has an invalid inventory quantity."
    if player.has("transform"):
        var transform: Variant = player["transform"]
        if not transform is Dictionary or not transform.has("position") or not _only_keys(transform, ["position", "basis"]):
            return "Save has invalid player transform."
        if not _valid_vector(transform["position"], 1000000.0):
            return "Save has invalid player position."
        if transform.has("basis"):
            var axes: Variant = transform["basis"]
            if not axes is Array or axes.size() != 3 or not _valid_vector(axes[0], 1.5) or not _valid_vector(axes[1], 1.5) or not _valid_vector(axes[2], 1.5):
                return "Save has invalid player orientation."
            var x_axis := _as_vector(axes[0])
            var y_axis := _as_vector(axes[1])
            var z_axis := _as_vector(axes[2])
            if absf(x_axis.length_squared() - 1.0) > 0.02 or absf(y_axis.length_squared() - 1.0) > 0.02 or absf(z_axis.length_squared() - 1.0) > 0.02 or absf(x_axis.dot(y_axis)) > 0.02 or absf(x_axis.dot(z_axis)) > 0.02 or absf(y_axis.dot(z_axis)) > 0.02 or x_axis.cross(y_axis).dot(z_axis) < 0.98:
                return "Save has a non-orthonormal player orientation."
    if actors.has(GLOBAL):
        var global_state: Variant = actors[GLOBAL]
        if not global_state is Dictionary or not global_state.has("brain") or not _only_keys(global_state, ["brain"]):
            return "Save has invalid Global actor state."
        var global_brain: Variant = global_state["brain"]
        if not global_brain is Dictionary or not global_brain.has("dsg_149") or not _only_keys(global_brain, ["dsg_149", "dsg_147"]) or not global_brain["dsg_149"] is bool:
            return "Save has invalid Global Brain state."
        # DSG[147] was added as an optional schema-1 field. Older slots leave
        # the level's authored initial value intact when this key is absent.
        if global_brain.has("dsg_147") and not global_brain["dsg_147"] is bool:
            return "Save has invalid optional Global Brain state."
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


static func read_original_import() -> Dictionary:
    if not FileAccess.file_exists(ORIGINAL_IMPORT):
        return {"status": "missing"}
    var file := FileAccess.open(ORIGINAL_IMPORT, FileAccess.READ)
    if file == null:
        return {"status": "error", "message": "Imported original save cannot be opened."}
    if file.get_length() > 33554432:
        return {"status": "error", "message": "Imported original save exceeds the supported size."}
    var parsed: Variant = JSON.parse_string(file.get_as_text())
    file.close()
    var error := validate_original_import(parsed)
    if not error.is_empty():
        return {"status": "error", "message": error}
    return {"status": "loaded", "state": parsed["godot_state"]}


static func validate_original_import(value: Variant) -> String:
    if not value is Dictionary:
        return "Imported original save is not an object."
    if value.get("schema") != 1 or value.get("format") != "astrolabe-godot-original-save-import":
        return "Imported original save format is unsupported."
    if value.get("profile") != "hype-pc-english-supplied-save":
        return "Imported original save profile is unsupported."
    if value.get("current_level") != "manoir":
        return "The current Load Game route supports original saves for manoir only."
    var state: Variant = value.get("godot_state")
    if not state is Dictionary:
        return "Imported original save has no supported manor state."
    var error := validate(state)
    if not error.is_empty():
        return "Imported original save is unsupported: " + error
    var actors: Dictionary = state["actors"]
    if not actors.has(GLOBAL):
        return "Imported original save is missing Global DSG[149]. Re-import the original slot."
    if not actors[ACTOR].has("transform"):
        return "Imported original save is missing Hype's saved position."
    var world: Variant = state.get("world")
    if not world is Dictionary or not world.has(DUMMY):
        return "Imported original save is missing the manor lever state."
    return ""


static func _dummy_brain(level: Node3D) -> Node:
    if level == null:
        return null
    var source_brains: Variant = level.get_meta("source_actor_brains", {})
    if not source_brains is Dictionary or not source_brains.has(DUMMY):
        return null
    return level.get_node_or_null(source_brains[DUMMY]) as Node


static func _source_brain(level: Node3D, identity: String) -> Node:
    if level == null:
        return null
    var source_brains: Variant = level.get_meta("source_actor_brains", {})
    if not source_brains is Dictionary or not source_brains.has(identity):
        return null
    return level.get_node_or_null(source_brains[identity]) as Node


static func capture(actor: CharacterBody3D, level: Node3D) -> Dictionary:
    var magick: Resource = actor.get("source_magick")
    var inventory: Resource = actor.get("source_inventory")
    var dummy_brain := _dummy_brain(level)
    var global_brain := _source_brain(level, GLOBAL)
    if magick == null or inventory == null or dummy_brain == null or not _has_property(global_brain, "dsg_149"):
        return {}
    var global_brain_state := {"dsg_149": global_brain.get("dsg_149")}
    if _has_property(global_brain, "dsg_147"):
        global_brain_state["dsg_147"] = global_brain.get("dsg_147")
    var quantities: Dictionary = {}
    for item in inventory.get("quantities"):
        quantities[str(item)] = int(inventory.get("quantities")[item])
    var transform := actor.global_transform
    var basis := transform.basis
    return {
        "schema": 1,
        "level": "manoir",
        "actors": {
            ACTOR: {
                "magick": {
                    "current_points": int(magick.get("current_points")),
                    "maximum_points": int(magick.get("maximum_points")),
                    "selected_spell_code": int(magick.get("selected_spell_code")),
                },
                "inventory": {
                    "selected_weapon": int(inventory.get("selected_weapon")),
                    "quantities": quantities,
                },
                "transform": {
                    "position": [transform.origin.x, transform.origin.y, transform.origin.z],
                    "basis": [
                        [basis.x.x, basis.x.y, basis.x.z],
                        [basis.y.x, basis.y.y, basis.y.z],
                        [basis.z.x, basis.z.y, basis.z.z],
                    ],
                },
            },
            GLOBAL: {
                "brain": global_brain_state,
            },
        },
        "world": {DUMMY: {"dsg_8": dummy_brain.get("dsg_8")}},
    }


static func write(actor: CharacterBody3D, level: Node3D) -> String:
    var state := capture(actor, level)
    var error := validate(state)
    if not error.is_empty():
        return error
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
    var magick: Resource = actor.get("source_magick")
    var inventory: Resource = actor.get("source_inventory")
    var dummy_brain := _dummy_brain(level)
    var global_brain := _source_brain(level, GLOBAL)
    if magick == null or inventory == null:
        return "Converted player has no native magick or inventory."
    if dummy_brain == null:
        return "Converted manor has no saved dummy world flag."
    if value["actors"].has(GLOBAL) and not _has_property(global_brain, "dsg_149"):
        return "Converted manor has no saved Global Brain field."
    if (
        value["actors"].has(GLOBAL)
        and value["actors"][GLOBAL]["brain"].has("dsg_147")
        and not _has_property(global_brain, "dsg_147")
    ):
        return "Converted manor has no saved optional Global Brain field."
    var player: Dictionary = value["actors"][ACTOR]
    var saved_magick: Dictionary = player["magick"]
    var saved_inventory: Dictionary = player["inventory"]
    var quantities: Dictionary = {}
    for item in saved_inventory["quantities"]:
        quantities[int(item)] = int(saved_inventory["quantities"][item])
    # Validate on a private copy before changing any live save component.
    var checked_inventory: Resource = inventory.call("clone_for_scene")
    var weapon := int(saved_inventory.get("selected_weapon", inventory.get("selected_weapon")))
    error = checked_inventory.call("restore_legacy", weapon, quantities)
    if not error.is_empty():
        return error
    magick.set("current_points", int(saved_magick["current_points"]))
    magick.set("maximum_points", int(saved_magick["maximum_points"]))
    magick.set("selected_spell_code", int(saved_magick["selected_spell_code"]))
    inventory.call("restore_legacy", weapon, quantities)
    if player.has("transform"):
        var saved_transform: Dictionary = player["transform"]
        var basis := actor.global_transform.basis
        if saved_transform.has("basis"):
            var axes: Array = saved_transform["basis"]
            basis = Basis(_as_vector(axes[0]), _as_vector(axes[1]), _as_vector(axes[2]))
        actor.global_transform = Transform3D(basis, _as_vector(saved_transform["position"]))
    if value["actors"].has(GLOBAL):
        var saved_global_brain: Dictionary = value["actors"][GLOBAL]["brain"]
        global_brain.set("dsg_149", saved_global_brain["dsg_149"])
        if saved_global_brain.has("dsg_147"):
            global_brain.set("dsg_147", saved_global_brain["dsg_147"])
    var world: Dictionary = value.get("world", {})
    var dummy_state: Dictionary = world.get(DUMMY, {"dsg_8": false})
    dummy_brain.set("dsg_8", dummy_state["dsg_8"])
    return ""
