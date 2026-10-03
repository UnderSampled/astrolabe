extends RefCounted
## Versioned native game state for the currently supported manor player slice.
## Source save addresses and raw SAI records never enter this file.

const SLOT := "user://manoir-save-v1.json"
const ACTOR := "world/0/0/Hype"
const DUMMY := "level:/persos/world/0/27"


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
    if not _only_keys(actors, [ACTOR]):
        return "Save has unsupported actors."
    var player: Dictionary = actors[ACTOR]
    if not _only_keys(player, ["magick", "inventory"]):
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
    if not inventory is Dictionary or not _integer(inventory.get("selected_weapon"), -1, 32767):
        return "Save has invalid inventory selection."
    if not _only_keys(inventory, ["selected_weapon", "quantities"]):
        return "Save has unsupported inventory state."
    var quantities: Variant = inventory.get("quantities")
    if not quantities is Dictionary:
        return "Save is missing inventory quantities."
    for item in quantities:
        # The measured source inventory and native quantity API retain -1 as
        # their absent/unavailable sentinel. Preserve it through native saves.
        if not item is String or not str(item).is_valid_int() or str(int(item)) != item or not _integer(int(item), 0, 32767) or not _integer(quantities[item], -1, 32767):
            return "Save has an invalid inventory quantity."
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
    var magick: Resource = actor.get("source_magick")
    var inventory: Resource = actor.get("source_inventory")
    var dummy_brain := _dummy_brain(level)
    if magick == null or inventory == null or dummy_brain == null:
        return {}
    var quantities: Dictionary = {}
    for item in inventory.get("quantities"):
        quantities[str(item)] = int(inventory.get("quantities")[item])
    return {
        "schema": 1,
        "level": "manoir",
        "actors": {ACTOR: {
            "magick": {
                "current_points": int(magick.get("current_points")),
                "maximum_points": int(magick.get("maximum_points")),
                "selected_spell_code": int(magick.get("selected_spell_code")),
            },
            "inventory": {
                "selected_weapon": int(inventory.get("selected_weapon")),
                "quantities": quantities,
            },
        }},
        "world": {DUMMY: {"dsg_8": bool(dummy_brain.get("dsg_8"))}},
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
    if magick == null or inventory == null:
        return "Converted player has no native magick or inventory."
    if dummy_brain == null:
        return "Converted manor has no saved dummy world flag."
    var player: Dictionary = value["actors"][ACTOR]
    var saved_magick: Dictionary = player["magick"]
    var saved_inventory: Dictionary = player["inventory"]
    var quantities: Dictionary = {}
    for item in saved_inventory["quantities"]:
        quantities[int(item)] = int(saved_inventory["quantities"][item])
    magick.set("current_points", int(saved_magick["current_points"]))
    magick.set("maximum_points", int(saved_magick["maximum_points"]))
    magick.set("selected_spell_code", int(saved_magick["selected_spell_code"]))
    inventory.set("selected_weapon", int(saved_inventory["selected_weapon"]))
    inventory.set("quantities", quantities)
    var world: Dictionary = value.get("world", {})
    var dummy_state: Dictionary = world.get(DUMMY, {"dsg_8": false})
    dummy_brain.set("dsg_8", bool(dummy_state["dsg_8"]))
    return ""
