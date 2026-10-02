extends RefCounted
## Restores the measured starting state of the supplied English SaveGame0.
## Provisional Load Game restoration from a measured fixture. General save
## decoding and persistence will replace this implementation.

static func load_fixture() -> Dictionary:
    var path := "res://game/english_save_manoir.json"
    if not FileAccess.file_exists(path):
        return {}
    var value: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
    return value if value is Dictionary else {}


static func source_vector(values: Array) -> Vector3:
    return Vector3(float(values[0]), float(values[2]), -float(values[1]))


static func apply(actor: CharacterBody3D, brain: Node, fixture: Dictionary) -> String:
    if int(fixture.get("schema", 0)) != 1 or str(fixture.get("name", "")) != "supplied-english-save-manoir":
        return "Invalid English save fixture"
    if int(actor.get("current_state")) != int(fixture["expected_action"]):
        return "Converted Hype action differs from the save fixture"
    var rows: Array = fixture["source_basis"]
    var basis := Basis(source_vector(rows[0]), source_vector(rows[2]), -source_vector(rows[1]))
    actor.global_transform = Transform3D(basis, source_vector(fixture["source_translation"]))
    brain.set("current_normal", int(fixture["normal_behavior"]))
    brain.set("current_reflex", int(fixture["reflex_behavior"]))
    for index in fixture["designer_overrides"]:
        brain.set("dsg_" + str(index), fixture["designer_overrides"][index])
    var level: Node = actor
    while level.get_parent() != null and level.get_parent() != actor.get_tree().root:
        level = level.get_parent()
    for saved in fixture.get("scene_actors", []):
        var restored_actor: Node3D
        for candidate in level.find_children("*", "CharacterBody3D", true, false):
            if str(candidate.get_meta("source_perso", "")) == str(saved["asset"]):
                restored_actor = candidate as Node3D
                break
        if restored_actor == null or str(restored_actor.get_meta("source_perso_name", "")) != str(saved["name"]):
            return "Saved scene actor is missing: " + str(saved["asset"])
        if restored_actor.get_meta("source_object_types", PackedInt32Array()) != PackedInt32Array(saved["object_types"]):
            return "Saved scene actor identity differs: " + str(saved["asset"])
        var saved_rows: Array = saved["source_basis"]
        var saved_basis := Basis(source_vector(saved_rows[0]), source_vector(saved_rows[2]), -source_vector(saved_rows[1]))
        restored_actor.global_transform = Transform3D(saved_basis, source_vector(saved["source_translation"]))
    var seen: Dictionary = {}
    for value in fixture.get("deactivated_channels", []):
        if not (value is int or value is float):
            return "Saved Hype deactivated channel list is malformed"
        var channel := int(value)
        if float(channel) != float(value) or channel < 0 or channel > 255 or seen.has(channel):
            return "Saved Hype deactivated channel list is malformed"
        seen[channel] = true
        if not bool(actor.call("source_set_channel_active", channel, false)):
            return "Saved Hype channel " + str(channel) + " is missing"
    var inventory: Dictionary = fixture.get("inventory", {})
    var native_inventory: Resource = actor.get("source_inventory")
    if native_inventory == null or not inventory.has("selected_weapon") or not inventory.has("quantities"):
        return "Saved Hype inventory fixture is missing"
    var selected: Variant = inventory["selected_weapon"]
    if not (selected is int or selected is float) or float(int(selected)) != float(selected) or int(selected) < -1 or int(selected) > 32767:
        return "Saved Hype selected weapon is malformed"
    var restored_quantities: Dictionary = {}
    for key in inventory["quantities"]:
        if not str(key).is_valid_int():
            return "Saved Hype inventory item is malformed"
        var item := int(key)
        var amount: Variant = inventory["quantities"][key]
        if item < 0 or item > 32767 or not (amount is int or amount is float) or float(int(amount)) != float(amount):
            return "Saved Hype inventory quantity is malformed"
        restored_quantities[item] = int(amount)
    native_inventory.set("selected_weapon", int(selected))
    native_inventory.set("quantities", restored_quantities)
    var magick: Dictionary = fixture.get("magick", {})
    var native_magick: Resource = actor.get("source_magick")
    if native_magick == null or not magick.has("source_code") or not magick.has("current_points") or not magick.has("maximum_points"):
        return "Saved Hype magick fixture is missing"
    for value in [magick["source_code"], magick["current_points"], magick["maximum_points"]]:
        if not (value is int or value is float) or float(int(value)) != float(value):
            return "Saved Hype magick value is malformed"
    if int(magick["source_code"]) < 0 or int(magick["source_code"]) > 32767 or int(magick["maximum_points"]) <= 0 or int(magick["current_points"]) < 0 or int(magick["current_points"]) > int(magick["maximum_points"]):
        return "Saved Hype magick value is out of range"
    native_magick.set("selected_spell_code", int(magick["source_code"]))
    native_magick.set("current_points", int(magick["current_points"]))
    native_magick.set("maximum_points", int(magick["maximum_points"]))
    var arrays: Dictionary = fixture.get("designer_arrays", {})
    for index in [159, 160, 161, 162, 163, 164]:
        var key := str(index)
        if not arrays.has(key):
            return "Saved Hype designer array " + key + " is missing"
        var existing: Variant = brain.get("dsg_" + key)
        var saved: Variant = arrays[key]
        if not (existing is PackedInt32Array) or not (saved is Array) or existing.size() != saved.size():
            return "Saved Hype designer array " + key + " has the wrong type or length"
        var restored := PackedInt32Array()
        for value in saved:
            restored.append(int(value))
        brain.set("dsg_" + key, restored)
    return ""
