extends RefCounted
## Versioned native game state for the currently supported manor player slice.
## Source save addresses and raw SAI records never enter this file.

const SLOT := "user://manoir-save-v1.json"
const ORIGINAL_IMPORT := "res://game/original_save_import.json"
const ACTOR := "world/0/0/Hype"
const PLAYER := "level:/persos/world/0/0"
# Native checkpoints add independently validated life and level-owned armor.
# Schema 1 remains the supported original-import overlay.
const SCHEMA := 4
const CHECKPOINT := "manor-idle-v1"
const SNAPSHOT = preload("res://addons/astrolabe/character/snapshot_values.gd")
const LIFE = preload("res://addons/astrolabe/character/hit_points.gd")
const ARMOR = preload("res://addons/astrolabe/character/armor_catalog.gd")
const DUMMY := "level:/persos/world/0/27"
const GLOBAL := "fix:/persos/15"
const INVENTORY = preload("res://addons/astrolabe/character/inventory.gd")
const FIRST_PICKUP_FLAGS: Array[String] = ["dsg_220", "dsg_221", "dsg_222", "dsg_223", "dsg_224", "dsg_225", "dsg_226", "dsg_227", "dsg_228"]
# Placed actor ownership, never the shared barrel model or spawned pickup DSG[0].
const BARRELS := ["level:/persos/world/0/71", "level:/persos/world/0/72", "level:/persos/world/0/73", "level:/persos/world/0/74"]
# Source-saved flags and separately owned native checkpoint continuity.
const TRAINING_FLAGS: Array[String] = ["dsg_152", "dsg_219"]
const TRAINER := "level:/persos/world/0/155"
const MANNEQUINS := ["level:/persos/world/0/152", "level:/persos/world/0/153", "level:/persos/world/0/154"]
const INITIAL_EXERCISES := {"level:/persos/world/0/152": 2, "level:/persos/world/0/153": 1, "level:/persos/world/0/154": 3}
const EXERCISE_IDLE := {1: 14, 2: 15, 3: 16, 4: 22, 5: 23}
const ATTACK_PERMISSIONS: Array[String] = ["dsg_0", "dsg_55", "dsg_58"]
const MANOR_KEY := 21
const KEY_ACTOR := "level:/persos/world/0/142"
const NATIVE_SOURCE := "native"
const ORIGINAL_SOURCE := "original"
const LOAD_SOURCE_META := &"astrolabe_load_save_source"


static func path() -> String:
    return SLOT


static func _integer(value: Variant, low: int, high: int) -> bool:
    return SNAPSHOT.integer(value, low, high)


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
    return _as_vector(value).is_finite()


static func _as_vector(value: Array) -> Vector3:
    return Vector3(float(value[0]), float(value[1]), float(value[2]))


static func _has_property(object: Object, property_name: String) -> bool:
    if not is_instance_valid(object):
        return false
    return object.get_property_list().any(func(property): return str(property["name"]) == property_name)


static func _has_boolean_field(brain: Node, property_name: String) -> bool:
    return _has_property(brain, property_name) and brain.get(property_name) is bool


static func _has_properties(owner: Object, fields: Array[String]) -> bool:
    return fields.all(func(field): return _has_property(owner, field))


static func _has_integer_properties(owner: Object, fields: Array[String]) -> bool:
    return _has_properties(owner, fields) and fields.all(func(field): return owner.get(field) is int)


static func resolve_player(level: Node3D) -> CharacterBody3D:
    if not is_instance_valid(level): return null
    var actors: Variant = level.get_meta("source_actor_nodes", {})
    if not actors is Dictionary or not actors.has(PLAYER): return null
    var path: Variant = actors[PLAYER]
    if not (path is String or path is NodePath): return null
    if NodePath(path).is_absolute(): return null
    var result := level.get_node_or_null(NodePath(path)) as CharacterBody3D
    if result == null or not level.is_ancestor_of(result) or str(result.get_meta("source_perso", "")) != PLAYER:
        return null
    return result


static func checkpoint_error(actor: CharacterBody3D, level: Node3D) -> String:
    if resolve_player(level) != actor or not is_instance_valid(actor) or str(level.get_meta("source_level", "")) != "manoir":
        return "Checkpoint player identity does not match the converted manor."
    var brain := actor.get_node_or_null("Brain")
    if not _has_properties(brain, ["current_normal", "current_reflex", "running", "unsupported_failure"]) or not _has_property(actor, "current_state"):
        return "Converted player has incompatible checkpoint behavior."
    if brain.get("current_normal") != 4 or brain.get("current_reflex") != 3 or actor.get("current_state") != 47:
        return "Save when Hype has returned to his supported idle state."
    if brain.get("running") != true or not str(brain.get("unsupported_failure")).is_empty():
        return "A stopped or unsupported behavior cannot be saved as a checkpoint."
    if not _has_properties(actor, ["source_airborne", "external_velocity", "pending_state", "generated_object"]):
        return "Converted player has incompatible checkpoint movement."
    if not actor.is_on_floor() or actor.get("source_airborne") != false or not actor.velocity.is_finite() or actor.velocity.length_squared() > 0.0001:
        return "Save when Hype is standing still on the ground."
    var external: Variant = actor.get("external_velocity")
    if not external is Vector3 or not external.is_finite() or external.length_squared() > 0.0001 or actor.get("pending_state") != -1:
        return "Save after movement and action transitions have settled."
    # This field exists only when the maintained movement API implements wind.
    # The current API explicitly defers wind; do not require a made-up property.
    if _has_property(actor, "source_wind_drift"):
        var wind: Variant = actor.get("source_wind_drift")
        if not wind is Vector3 or not wind.is_finite() or wind.length_squared() > 0.0001:
            return "Save after persistent wind drift has stopped."
    if is_instance_valid(actor.get("generated_object")):
        return "Save after the held projectile has been removed."
    for candidate in level.find_children("*", "Node3D", true, false):
        if candidate.has_meta("astrolabe_generated_template"):
            return "Save after active generated projectiles have been removed."
    for action in InputMap.get_actions():
        if (str(action).begins_with("Button_") or str(action).begins_with("BT_Camera_")) and Input.is_action_pressed(action):
            return "Release gameplay controls before saving a checkpoint."
    var life: Resource = actor.get("source_hit_points") if _has_property(actor, "source_hit_points") else null
    if not _has_property(life, "current") or not _integer(life.get("current"), 1, 255):
        return "Save when Hype is alive with supported native life state."
    return _training_checkpoint_error(actor, level)


static func validate(value: Variant) -> String:
    if not value is Dictionary:
        return "Save is not an object."
    if not _only_keys(value, ["schema", "level", "actors", "world", "checkpoint", "training"]):
        return "Save has unsupported top-level state."
    if not _integer(value.get("schema"), 1, SCHEMA) or not [1, SCHEMA].has(int(value["schema"])):
        return "Save version is unsupported."
    var schema := int(value["schema"])
    if schema == 1 and value.has("checkpoint"):
        return "Legacy save has unsupported checkpoint state."
    if schema == 1 and value.has("training"):
        return "Original save overlays do not contain native training checkpoints."
    if schema == SCHEMA and value.get("checkpoint") != CHECKPOINT:
        return "Save checkpoint is unsupported."
    if value.get("level") != "manoir":
        return "Save level is unsupported."
    var actors: Variant = value.get("actors")
    if not actors is Dictionary or not actors.has(ACTOR) or not actors[ACTOR] is Dictionary:
        return "Save is missing the manor player."
    if not _only_keys(actors, [ACTOR, GLOBAL]):
        return "Save has unsupported actors."
    if schema == SCHEMA and not actors.has(GLOBAL):
        return "Checkpoint is missing the supported Global Brain."
    var player: Dictionary = actors[ACTOR]
    var fields: Array[String] = ["magick", "inventory", "transform"]
    if schema == SCHEMA: fields.append_array(["asset", "life", "armor"])
    if not _only_keys(player, fields):
        return "Save has unsupported player state."
    if schema == SCHEMA:
        if player.get("asset") != PLAYER:
            return "Save player asset is unsupported."
        if not player.has("transform"):
            return "Save is missing the player transform."
        var snapshot_error := LIFE.validate_snapshot_shape(player.get("life"))
        if not snapshot_error.is_empty(): return snapshot_error
        snapshot_error = ARMOR.validate_snapshot_shape(player.get("armor"))
        if not snapshot_error.is_empty(): return snapshot_error
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
    var inventory_fields: Array[String] = ["selected_weapon", "quantities", "snapshot"]
    if schema == 1: inventory_fields.append("gold")
    if not _only_keys(inventory, inventory_fields):
        return "Save has unsupported inventory state."
    if inventory.has("gold") and (inventory.has("snapshot") or not _integer(inventory["gold"], 0, 32767)):
        return "Save has invalid independently owned gold state."
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
    if inventory.has("snapshot"):
        var snapshot_error := INVENTORY.validate_snapshot_shape(inventory["snapshot"])
        if not snapshot_error.is_empty(): return snapshot_error
    if player.has("transform"):
        var transform: Variant = player["transform"]
        if not transform is Dictionary or not transform.has("position") or not _only_keys(transform, ["position", "basis"]):
            return "Save has invalid player transform."
        if not _valid_vector(transform["position"], 1000000.0 if schema == 1 else INF):
            return "Save has invalid player position."
        if transform.has("basis"):
            var axes: Variant = transform["basis"]
            if not axes is Array or axes.size() != 3 or not _valid_vector(axes[0], INF) or not _valid_vector(axes[1], INF) or not _valid_vector(axes[2], INF):
                return "Save has invalid player orientation."
            var x_axis := _as_vector(axes[0])
            var y_axis := _as_vector(axes[1])
            var z_axis := _as_vector(axes[2])
            var determinant := Basis(x_axis, y_axis, z_axis).determinant()
            if not is_finite(determinant) or absf(determinant) <= 0.000001:
                return "Save has a singular player orientation."
        elif schema == SCHEMA:
            return "Save is missing the player basis."
    if actors.has(GLOBAL):
        var global_state: Variant = actors[GLOBAL]
        if not global_state is Dictionary or not global_state.has("brain") or not _only_keys(global_state, ["brain"]):
            return "Save has invalid Global actor state."
        var global_brain: Variant = global_state["brain"]
        var supported_flags: Array[String] = ["dsg_149", "dsg_147"]
        supported_flags.append_array(FIRST_PICKUP_FLAGS)
        supported_flags.append_array(TRAINING_FLAGS)
        if not global_brain is Dictionary or not global_brain.has("dsg_149") or not _only_keys(global_brain, supported_flags) or not global_brain["dsg_149"] is bool:
            return "Save has invalid Global Brain state."
        # DSG[147] was added as an optional schema-1 field. Older slots leave
        # the level's authored initial value intact when this key is absent.
        if global_brain.has("dsg_147") and not global_brain["dsg_147"] is bool:
            return "Save has invalid optional Global Brain state."
        for property_name in FIRST_PICKUP_FLAGS + TRAINING_FLAGS:
            if global_brain.has(property_name) and not global_brain[property_name] is bool:
                return "Save has invalid saved Global Brain state."
    # Older schema-1 slots predate world flags; missing world state starts from
    # the measured manor seed. When present, validate the complete supported
    # native flag before applying any part of the save.
    if schema == SCHEMA and (not value.get("world") is Dictionary or not value["world"].has(DUMMY)):
        return "Checkpoint is missing the supported manor world flag."
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
            if identity == DUMMY: continue
            if not BARRELS.has(identity): return "Save has unsupported world state."
            var barrel: Variant = world[identity]
            if not barrel is Dictionary or not _only_keys(barrel, ["dsg_8", "removed"]) or not barrel.get("dsg_8") is bool or not barrel.get("removed") is bool:
                return "Save has invalid barrel state."
            if barrel["removed"] and not barrel["dsg_8"]: return "Removed barrel has no consumed flag."
    if value.has("training"):
        var flags: Dictionary = actors[GLOBAL]["brain"]
        for field in TRAINING_FLAGS:
            if not flags.has(field): return "Training checkpoint is missing a Global training flag."
        if flags["dsg_219"]: return "Training checkpoint retains an actor whose source startup removes it."
        return _validate_training(value["training"])
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
    return {"status": "loaded", "state": parsed["godot_state"], "slot": str(parsed.get("slot", "")), "display_name": str(parsed.get("display_name", ""))}


## An overlay can be valid for component restoration without being a complete
## Load Game checkpoint. Missing placement/progression must never inherit New
## Game values merely because an older native file exists.
static func validate_load(value: Variant) -> String:
    var error := validate(value)
    if not error.is_empty(): return error
    var actors: Dictionary = value["actors"]
    if not actors[ACTOR].has("transform"):
        return "Save has no player position and cannot resume a game."
    if not actors[ACTOR]["transform"].has("basis"):
        return "Save has no player orientation and cannot resume a game."
    if not actors.has(GLOBAL) or not actors[GLOBAL]["brain"].has("dsg_147"):
        return "Save has no opening progression state and cannot resume a game."
    if not value.get("world") is Dictionary or not value["world"].has(DUMMY):
        return "Save has no manor world state and cannot resume a game."
    var inventory: Dictionary = actors[ACTOR]["inventory"]
    if int(value["schema"]) == SCHEMA and not inventory.has("snapshot"):
        return "Checkpoint has no complete inventory state and cannot resume a game."
    if int(value["schema"]) == 1 and not inventory.has("gold") and not inventory["quantities"].has("0"):
        return "Original save has no gold balance and cannot resume a game."
    return ""


static func _load_candidate(result: Dictionary, source: String) -> Dictionary:
    var candidate := result.duplicate(true)
    candidate["source"] = source
    candidate["path"] = SLOT if source == NATIVE_SOURCE else ORIGINAL_IMPORT
    candidate["label"] = "Godot checkpoint — Manoir" if source == NATIVE_SOURCE else "Original Hype — " + str(candidate.get("display_name", "Manoir"))
    if source == ORIGINAL_SOURCE and not str(candidate.get("slot", "")).is_empty():
        candidate["label"] += " (" + str(candidate["slot"]) + ")"
    if candidate["status"] == "loaded":
        var error := validate_load(candidate["state"])
        if not error.is_empty():
            candidate.erase("state")
            candidate["status"] = "error"
            candidate["message"] = error
    return candidate


## The menu lists each existing source, including unusable files with their
## reason. Selecting an original import never deletes or upgrades a native slot.
static func load_candidates() -> Array[Dictionary]:
    var candidates: Array[Dictionary] = []
    for candidate in [_load_candidate(read(), NATIVE_SOURCE), _load_candidate(read_original_import(), ORIGINAL_SOURCE)]:
        if candidate["status"] != "missing": candidates.append(candidate)
    return candidates


static func read_for_load(source: String = "") -> Dictionary:
    if source == NATIVE_SOURCE: return _load_candidate(read(), source)
    if source == ORIGINAL_SOURCE: return _load_candidate(read_original_import(), source)
    if not source.is_empty():
        return {"status": "error", "message": "The selected save source is unsupported."}
    # Direct session entry retains explicit native precedence. A broken or
    # incomplete native slot is an error, never a silent switch of save identity.
    var native := _load_candidate(read(), NATIVE_SOURCE)
    if native["status"] != "missing": return native
    var original := _load_candidate(read_original_import(), ORIGINAL_SOURCE)
    if original["status"] != "missing": return original
    return {"status": "error", "message": "No saved game is available. Choose New Game or import an original save."}


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
    return _source_brain(level, DUMMY)


static func _brain_path(level: Node3D, identity: String) -> NodePath:
    if not is_instance_valid(level): return NodePath()
    var bindings: Variant = level.get_meta("source_actor_brains", {})
    if not bindings is Dictionary or not bindings.has(identity): return NodePath()
    var path: Variant = bindings[identity]
    if not (path is String or path is NodePath): return NodePath()
    var relative := NodePath(path)
    if relative.is_empty() or relative.is_absolute(): return NodePath()
    # A killed actor retains its named local binding as a tombstone.
    for part in relative.get_name_count():
        if relative.get_name(part) == "..": return NodePath()
    return relative


static func _source_brain(level: Node3D, identity: String) -> Node:
    var path := _brain_path(level, identity)
    if path.is_empty(): return null
    var brain := level.get_node_or_null(path)
    return brain if brain != null and level.is_ancestor_of(brain) else null


static func _training_brain(level: Node3D, identity: String) -> Node:
    var candidate := _source_brain(level, identity)
    var model := "fix:/ai/models/28" if identity == PLAYER else "level:/ai/models/991" if identity == TRAINER else "level:/ai/models/703" if identity == KEY_ACTOR else "level:/ai/models/939"
    if not _has_property(candidate, "source_model_path") or candidate.get("source_model_path") != model:
        return null
    var character := candidate.get_parent() as CharacterBody3D
    if character == null or str(character.get_meta("source_perso", "")) != identity:
        return null
    if character.is_queued_for_deletion(): return null
    return candidate


static func _training_key_collected(inventory: Resource) -> bool:
    # Stock quantity -1 is a valid owned Object. Admission proves collection.
    if not _has_property(inventory, "source_initialized") or inventory.get("source_initialized") != true:
        return false
    if not _has_property(inventory, "admission_known") or inventory.get("admission_known") != true or not inventory.has_method("presentation_quantities"):
        return false
    var owned: Dictionary = inventory.call("presentation_quantities")
    return owned.has(MANOR_KEY)


static func _training_removal_supported(character: Node) -> bool:
    return (
        _has_boolean_field(character, "source_activation_known") and character.get("source_activation_known")
        and _has_boolean_field(character, "source_kill_resets_components") and not character.get("source_kill_resets_components")
        and _has_boolean_field(character, "source_object_active")
        and _has_boolean_field(character, "source_return_suppressed")
        and character.has_method("source_kill_perso")
    )


static func _training_checkpoint_error(actor: CharacterBody3D, level: Node3D) -> String:
    # Old exports and the bounded save fixture have no training actor bindings.
    # Once a binding exists, a missing owner cannot silently lose progress.
    var bindings: Variant = level.get_meta("source_actor_brains", {})
    if not bindings is Dictionary: return "The converted actor bindings are invalid."
    if not bindings.has(TRAINER):
        for identity in MANNEQUINS:
            if bindings.has(identity): return "The training actor binding is missing."
        return ""
    if _brain_path(level, TRAINER).is_empty(): return "The training actor binding is invalid."
    var trainer := _training_brain(level, TRAINER)
    var global_brain := _source_brain(level, GLOBAL)
    if trainer == null:
        return "The training Brain is missing."
    if not _has_integer_properties(trainer, ["dsg_0", "current_normal", "current_reflex"]):
        return "The training Brain has incompatible checkpoint state."
    var stage := int(trainer.get("dsg_0"))
    # A fresh stage zero can use source startup only while its independent
    # mannequin fields still match the converted Manor seed.
    if stage == 0:
        for identity in MANNEQUINS:
            var mannequin := _training_brain(level, identity)
            if not _has_integer_properties(mannequin, ["dsg_0"]) or not _has_boolean_field(mannequin, "dsg_1") or not _has_boolean_field(mannequin, "dsg_2"):
                return "A mannequin Brain has incompatible checkpoint state."
            if mannequin.get("dsg_0") != INITIAL_EXERCISES[identity] or mannequin.get("dsg_1") or mannequin.get("dsg_2"):
                return "The fresh training stage differs from its mannequin progress."
        return ""
    if _has_boolean_field(global_brain, "dsg_219") and global_brain.get("dsg_219"):
        return "Save after the completed training actor's lifecycle has settled."
    if not EXERCISE_IDLE.has(stage) or trainer.get("current_normal") != EXERCISE_IDLE[stage] or trainer.get("current_reflex") != 0:
        return "Save after Gogoud's training dialogue or movement has finished."
    # Loose ground-key progress has a separate owner and is not captured here.
    # The settled final checkpoint retains only its completed pickup/removal.
    if stage == 5:
        if not _training_key_collected(actor.get("source_inventory") as Resource):
            return "Collect Gogoud's key before saving this training checkpoint."
        var key := _training_brain(level, KEY_ACTOR)
        if key == null or not _training_removal_supported(key.get_parent()) or key.get_parent().get("source_object_active") or not key.get_parent().get("source_return_suppressed"):
            return "Save after the manor key pickup presentation has finished."
    if not _has_property(trainer, "running") or trainer.get("running") != true or not _has_property(trainer, "unsupported_failure") or not str(trainer.get("unsupported_failure")).is_empty():
        return "Stopped or unsupported training cannot be saved."
    for field in ["normal_rule_claimed", "reflex_rule_claimed", "reflex_controls_tick"]:
        if not _has_boolean_field(trainer, field) or trainer.get(field):
            return "Save after Gogoud's suspended training action has finished."
    # Stages2/3 can suspend while replenishing ammunition. Keep their native
    # default latches only when neither replenishment is in flight.
    if stage in [2, 3] and (not _has_boolean_field(trainer, "dsg_3") or not _has_boolean_field(trainer, "dsg_4") or not trainer.get("dsg_3") or not trainer.get("dsg_4")):
        return "Save after training ammunition replenishment has settled."
    if not _has_boolean_field(trainer, "dsg_5"):
        return "The training Brain has no native lever-event latch."
    if not _has_integer_properties(trainer.get_parent(), ["current_state"]) or trainer.get_parent().get("current_state") != 23:
        return "Save after Gogoud's training action has settled."
    if not _has_boolean_field(trainer.get_parent(), "source_object_active") or not trainer.get_parent().get("source_object_active"):
        return "Inactive training cannot be saved as a live checkpoint."
    for field in ATTACK_PERMISSIONS:
        if not _has_boolean_field(actor.get_node_or_null("Brain"), field):
            return "The player Brain has incompatible training permissions."
    for field in TRAINING_FLAGS:
        if not _has_boolean_field(global_brain, field): return "The Global Brain has incompatible training flags."
    for identity in MANNEQUINS:
        var mannequin := _training_brain(level, identity)
        if not _has_integer_properties(mannequin, ["dsg_0", "current_normal", "current_reflex"]) or not _has_boolean_field(mannequin, "dsg_1") or not _has_boolean_field(mannequin, "dsg_2"):
            return "A mannequin Brain has incompatible checkpoint state."
        if mannequin.get("current_normal") != 3 or mannequin.get("current_reflex") != 0:
            return "Save after the mannequins have finished reacting."
        if not _has_integer_properties(mannequin.get_parent(), ["current_state"]) or mannequin.get_parent().get("current_state") != 0:
            return "Save after the mannequin action has settled."
        if not _has_property(mannequin, "running") or mannequin.get("running") != true or not _has_property(mannequin, "unsupported_failure") or not str(mannequin.get("unsupported_failure")).is_empty():
            return "Stopped or unsupported mannequin behavior cannot be saved."
        if not _has_boolean_field(mannequin.get_parent(), "source_object_active") or not mannequin.get_parent().get("source_object_active"):
            return "Inactive mannequin behavior cannot be saved."
    return ""


static func _capture_training(actor: CharacterBody3D, level: Node3D) -> Dictionary:
    var trainer := _training_brain(level, TRAINER)
    if trainer == null or not _has_integer_properties(trainer, ["dsg_0"]) or trainer.get("dsg_0") == 0: return {}
    if not _training_checkpoint_error(actor, level).is_empty(): return {}
    var owners := {}
    var permissions := {}
    for field in ATTACK_PERMISSIONS: permissions[field] = actor.get_node("Brain").get(field)
    owners[PLAYER] = {"brain": permissions}
    var pose := (trainer.get_parent() as Node3D).global_transform
    owners[TRAINER] = {
        "brain": {"dsg_0": trainer.get("dsg_0"), "dsg_5": trainer.get("dsg_5")},
        "transform": {"position": [pose.origin.x, pose.origin.y, pose.origin.z], "basis": [
            [pose.basis.x.x, pose.basis.x.y, pose.basis.x.z],
            [pose.basis.y.x, pose.basis.y.y, pose.basis.y.z],
            [pose.basis.z.x, pose.basis.z.y, pose.basis.z.z],
        ]},
    }
    for identity in MANNEQUINS:
        var mannequin := _training_brain(level, identity)
        owners[identity] = {"brain": {"dsg_0": mannequin.get("dsg_0"), "dsg_1": mannequin.get("dsg_1"), "dsg_2": mannequin.get("dsg_2")}}
    if trainer.get("dsg_0") == 5: owners[KEY_ACTOR] = {"removed": true}
    return {"schema": 1, "actors": owners}


static func _validate_training(value: Variant) -> String:
    if not value is Dictionary or not _only_keys(value, ["schema", "actors"]) or not _integer(value.get("schema"), 1, 1): return "Invalid native training checkpoint."
    var owners: Variant = value.get("actors")
    var identities: Array[String] = [PLAYER, TRAINER]
    identities.append_array(MANNEQUINS)
    var trainer: Variant = owners.get(TRAINER) if owners is Dictionary else null
    var memory: Variant = trainer.get("brain") if trainer is Dictionary else null
    if memory is Dictionary and _integer(memory.get("dsg_0"), 5, 5): identities.append(KEY_ACTOR)
    if not owners is Dictionary or owners.size() != identities.size() or not _only_keys(owners, identities): return "Training checkpoint has invalid actor ownership."
    for identity in identities:
        if identity == KEY_ACTOR:
            if not owners[identity] is Dictionary or not _only_keys(owners[identity], ["removed"]) or owners[identity].get("removed") != true or not owners[identity].get("removed") is bool:
                return "Training checkpoint has an invalid collected key owner."
            continue
        var fields: Array[String] = ["brain", "transform"] if identity == TRAINER else ["brain"]
        if not owners[identity] is Dictionary or not _only_keys(owners[identity], fields): return "Training checkpoint has invalid actor state."
        var fields_saved: Variant = owners[identity].get("brain")
        var expected: Array[String] = ["dsg_0", "dsg_5"] if identity == TRAINER else ATTACK_PERMISSIONS if identity == PLAYER else ["dsg_0", "dsg_1", "dsg_2"]
        if not fields_saved is Dictionary or fields_saved.size() != expected.size() or not _only_keys(fields_saved, expected): return "Training checkpoint has invalid Brain fields."
        for field in expected:
            if field == "dsg_0" and identity != PLAYER:
                if not _integer(fields_saved[field], 0, 255) or identity == TRAINER and not EXERCISE_IDLE.has(int(fields_saved[field])): return "Training checkpoint has an invalid stage."
            elif not fields_saved[field] is bool: return "Training checkpoint has a non-Boolean permission."
    var transform: Variant = owners[TRAINER].get("transform")
    if not transform is Dictionary or not _only_keys(transform, ["position", "basis"]) or not _valid_vector(transform.get("position"), INF): return "Training checkpoint has an invalid trainer position."
    var axes: Variant = transform.get("basis")
    if not axes is Array or axes.size() != 3 or not _valid_vector(axes[0], INF) or not _valid_vector(axes[1], INF) or not _valid_vector(axes[2], INF): return "Training checkpoint has an invalid trainer basis."
    var determinant := Basis(_as_vector(axes[0]), _as_vector(axes[1]), _as_vector(axes[2])).determinant()
    if not is_finite(determinant) or absf(determinant) <= 0.000001: return "Training checkpoint has a singular trainer basis."
    return ""


static func _preflight_training(level: Node3D, value: Dictionary, inventory: Resource) -> String:
    # Apply occurs before automatic Brain startup. No coroutine is serialized.
    if int(value["actors"][TRAINER]["brain"]["dsg_0"]) == 5 and not _training_key_collected(inventory):
        return "The training checkpoint has no collected manor key."
    for identity in value["actors"]:
        var owner := _training_brain(level, identity)
        if owner == null or not _has_property(owner, "running") or owner.get("running") != false:
            return "Training checkpoints require stopped, matching actor Brains."
        if not _has_integer_properties(owner, ["current_normal", "current_reflex"]): return "Training actor has incompatible behavior selection."
        if identity == KEY_ACTOR:
            if not _training_removal_supported(owner.get_parent()): return "Converted key has an unsupported removal policy."
            continue
        for field in value["actors"][identity]["brain"]:
            if not _has_property(owner, field):
                return "Converted actor has incompatible training Brain fields."
            var live_field: Variant = owner.get(field)
            if field == "dsg_0" and identity != PLAYER:
                if not live_field is int: return "Converted actor has an incompatible training stage."
            elif not live_field is bool:
                return "Converted actor has incompatible training Brain fields."
        if identity == PLAYER: continue
        var character := owner.get_parent()
        if not _has_boolean_field(character, "source_activation_known") or not character.get("source_activation_known") or not _has_boolean_field(character, "source_return_suppressed") or character.get("source_return_suppressed") or not character.has_method("source_set_object_active_state"):
            return "Converted training actor has incompatible activation state."
        var state := 23 if identity == TRAINER else 0
        if not character.has_method("select_state") or not _has_property(character, "animation_player") or not _has_property(character, "restored_configuration"):
            return "Converted training actor has incompatible native action support."
        var player := character.get("animation_player") as AnimationPlayer
        var prefix := "restored/" if character.get("restored_configuration") else ""
        if player == null or not player.has_animation(StringName(prefix + "state_%d" % state)):
            return "Converted training actor is missing its settled action."
    for field in TRAINING_FLAGS:
        if not _has_boolean_field(_source_brain(level, GLOBAL), field): return "Converted Global Brain is missing a training flag."
    return ""


static func _restore_training(level: Node3D, value: Dictionary) -> void:
    for identity in value["actors"]:
        var owner := _training_brain(level, identity)
        if identity == KEY_ACTOR:
            owner.get_parent().call("source_kill_perso")
            continue
        var memory: Dictionary = value["actors"][identity]["brain"]
        for field in memory: owner.set(field, int(memory[field]) if field == "dsg_0" and identity != PLAYER else memory[field])
        if identity == PLAYER: continue
        owner.set("current_normal", EXERCISE_IDLE[int(memory["dsg_0"])] if identity == TRAINER else 3)
        owner.set("current_reflex", 0)
        owner.get_parent().call("select_state", 23 if identity == TRAINER else 0)
        owner.get_parent().call("source_set_object_active_state", true)
    var trainer := _training_brain(level, TRAINER)
    var character := trainer.get_parent() as CharacterBody3D
    var pose: Dictionary = value["actors"][TRAINER]["transform"]
    var axes: Array = pose["basis"]
    character.global_transform = Transform3D(Basis(_as_vector(axes[0]), _as_vector(axes[1]), _as_vector(axes[2])), _as_vector(pose["position"]))


static func _valid_barrel(brain: Node) -> bool:
    return (
        _has_property(brain, "source_model_path")
        and brain.get("source_model_path") == "level:/ai/models/488"
        and _has_boolean_field(brain, "dsg_8")
        and _has_integer_properties(brain, ["dsg_4"])
        and brain.get_parent().has_method("source_set_object_active_state")
        and brain.get_parent().has_method("source_kill")
    )


static func _armor_catalog(level: Node3D) -> Resource:
    if not level.has_meta("source_armor_catalog"): return null
    var catalog := level.get_meta("source_armor_catalog") as Resource
    if not _has_property(catalog, "source_path") or catalog.get("source_path") != "fix:/gameplay/armor":
        return null
    return catalog


static func _armor_state(level: Node3D, catalog: Resource) -> Resource:
    if not level.has_meta("source_armor_state"): return null
    var state := level.get_meta("source_armor_state") as Resource
    if not _has_property(state, "catalog") or state.get("catalog") != catalog or not state.has_method("restore_snapshot"):
        return null
    return state


static func capture(actor: CharacterBody3D, level: Node3D) -> Dictionary:
    if not is_instance_valid(actor) or resolve_player(level) != actor or str(level.get_meta("source_level", "")) != "manoir":
        return {}
    if not _training_checkpoint_error(actor, level).is_empty(): return {}
    var magick: Resource = actor.get("source_magick")
    var inventory: Resource = actor.get("source_inventory")
    var life: Resource = actor.get("source_hit_points") if _has_property(actor, "source_hit_points") else null
    var armor := _armor_catalog(level)
    if life == null or armor == null or _armor_state(level, armor) == null or not life.has_method("capture_snapshot") or not armor.has_method("capture_snapshot"):
        return {}
    var dummy_brain := _dummy_brain(level)
    var global_brain := _source_brain(level, GLOBAL)
    if magick == null or inventory == null or not _has_boolean_field(dummy_brain, "dsg_8") or not _has_boolean_field(global_brain, "dsg_149"):
        return {}
    if not _has_integer_properties(magick, ["current_points", "maximum_points", "selected_spell_code"]):
        return {}
    var global_brain_state := {"dsg_149": global_brain.get("dsg_149")}
    if _has_property(global_brain, "dsg_147"):
        if not _has_boolean_field(global_brain, "dsg_147"):
            return {}
        global_brain_state["dsg_147"] = global_brain.get("dsg_147")
    # Source Global[220..228] all have player-save mask0x200. These are
    # per-item first-pickup flags; HUD bookkeeping19..23 is transient.
    for property_name in FIRST_PICKUP_FLAGS + TRAINING_FLAGS:
        if not _has_property(global_brain, property_name): continue
        var first_pickup: Variant = global_brain.get(property_name)
        if not first_pickup is bool: return {}
        global_brain_state[property_name] = first_pickup
    var inventory_snapshot: Dictionary = inventory.call("capture_snapshot") if inventory.has_method("capture_snapshot") else {}
    var quantities: Dictionary = {}
    for item in inventory.get("quantities"):
        quantities[str(item)] = int(inventory.get("quantities")[item])
    var world := {DUMMY: {"dsg_8": dummy_brain.get("dsg_8")}}
    var bindings: Dictionary = level.get_meta("source_actor_brains", {})
    for identity in BARRELS:
        # Missing bindings indicate an older export. A missing bound node means
        # this particular placed actor was killed, not a shared-template flag.
        if not bindings.has(identity): continue
        var barrel_brain := _source_brain(level, identity)
        if _brain_path(level, identity).is_empty(): return {}
        var removed := barrel_brain == null or barrel_brain.get_parent().is_queued_for_deletion()
        if not removed and _has_boolean_field(barrel_brain.get_parent(), "source_return_suppressed"):
            removed = bool(barrel_brain.get_parent().get("source_return_suppressed"))
        if not removed and not _valid_barrel(barrel_brain): return {}
        world[identity] = {"dsg_8": true if removed else bool(barrel_brain.get("dsg_8")), "removed": removed}
    var transform := actor.global_transform
    var basis := transform.basis
    var state := {
        "schema": SCHEMA,
        "level": "manoir",
        "checkpoint": CHECKPOINT,
        "actors": {
            ACTOR: {
                "asset": PLAYER,
                "life": life.call("capture_snapshot"),
                "armor": armor.call("capture_snapshot"),
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
        "world": world,
    }
    if inventory.has_method("capture_snapshot"): state["actors"][ACTOR]["inventory"]["snapshot"] = inventory_snapshot
    var training := _capture_training(actor, level)
    if not training.is_empty(): state["training"] = training
    return state


static func write(actor: CharacterBody3D, level: Node3D) -> String:
    var error := checkpoint_error(actor, level)
    if not error.is_empty(): return error
    var state := capture(actor, level)
    error = validate_load(state)
    if not error.is_empty():
        return error
    # Shape alone cannot establish the ordered native definition/piece binding.
    var armor := _armor_catalog(level)
    if not armor.has_method("validate_snapshot"):
        return "Converted armor has incompatible snapshot support."
    error = armor.call("validate_snapshot", state["actors"][ACTOR]["armor"])
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


## Original import used to put source item 0 in the ordinary quantities map.
## Its source composition is the separate gold resource. Resolve that binding
## explicitly; never discard an unknown ordinary item to make restoration pass.
static func _restore_inventory(inventory: Resource, saved: Dictionary) -> String:
    if saved.has("snapshot"):
        return inventory.call("apply_snapshot", saved["snapshot"])
    var amounts: Dictionary = {}
    for item in saved["quantities"]: amounts[int(item)] = int(saved["quantities"][item])
    var gold: Variant = saved.get("gold")
    if gold != null or amounts.has(0):
        if not _has_properties(inventory, ["source_initialized", "gold_definition", "gold", "definitions"]) or inventory.get("source_initialized") != true:
            return "Saved gold has no initialized source inventory binding."
        var definition: Resource = inventory.get("gold_definition")
        if not _has_integer_properties(definition, ["item_id", "maximum"]) or definition.get("item_id") != 0:
            return "Saved item 0 differs from the converted gold resource."
        for member: Resource in inventory.get("definitions"):
            if _has_property(member, "item_id") and member.get("item_id") == 0:
                return "Saved item 0 ambiguously identifies gold and an ordinary item."
        if amounts.has(0):
            if gold != null and int(gold) != amounts[0]:
                return "Saved gold conflicts with the legacy item-0 balance."
            gold = amounts[0]
            amounts.erase(0)
        if not _integer(gold, 0, 32767) or int(gold) > int(definition.get("maximum")):
            return "Saved gold exceeds the source resource capacity."
    var weapon := int(saved.get("selected_weapon", inventory.get("selected_weapon")))
    var error: String = inventory.call("restore_legacy", weapon, amounts)
    if not error.is_empty(): return error
    if gold != null: inventory.set("gold", int(gold))
    return ""


static func apply(actor: CharacterBody3D, level: Node3D, value: Dictionary) -> String:
    var error := validate(value)
    if not error.is_empty():
        return error
    if not is_instance_valid(actor) or resolve_player(level) != actor or str(level.get_meta("source_level", "")) != "manoir":
        return "Saved player identity differs from the converted manor."
    var magick: Resource = actor.get("source_magick")
    var inventory: Resource = actor.get("source_inventory")
    var dummy_brain := _dummy_brain(level)
    var global_brain := _source_brain(level, GLOBAL)
    if magick == null or inventory == null:
        return "Converted player has no native magick or inventory."
    if not _has_properties(magick, ["current_points", "maximum_points", "selected_spell_code"]) or not _has_properties(inventory, ["selected_weapon", "quantities"]):
        return "Converted player has incompatible native properties."
    if not _has_integer_properties(magick, ["current_points", "maximum_points", "selected_spell_code"]):
        return "Converted magick has incompatible scalar types."
    for method in ["clone_for_scene", "restore_legacy"]:
        if not inventory.has_method(method): return "Converted inventory has incompatible save support."
    if dummy_brain == null:
        return "Converted manor has no saved dummy world flag."
    if not _has_boolean_field(dummy_brain, "dsg_8"):
        return "Converted manor has no Boolean dummy world flag."
    if value["actors"].has(GLOBAL) and not _has_boolean_field(global_brain, "dsg_149"):
        return "Converted manor has no saved Global Brain field."
    if (
        value["actors"].has(GLOBAL)
        and value["actors"][GLOBAL]["brain"].has("dsg_147")
        and not _has_boolean_field(global_brain, "dsg_147")
    ):
        return "Converted manor has no saved optional Global Brain field."
    var player: Dictionary = value["actors"][ACTOR]
    var life: Resource = null
    var armor: Resource = null
    var armor_state: Resource = null
    if int(value["schema"]) == SCHEMA:
        life = actor.get("source_hit_points") as Resource if _has_property(actor, "source_hit_points") else null
        armor = _armor_catalog(level)
        if life == null or not life.has_method("apply_snapshot"):
            return "Converted player has no native checkpoint life state."
        if not _has_integer_properties(life, ["current", "maximum", "limit"]):
            return "Converted life has incompatible scalar types."
        if armor == null:
            return "Converted level has no native armor catalogue."
        armor_state = _armor_state(level, armor)
        if armor_state == null:
            return "Converted armor facade differs from the level catalogue."
        for method in ["clone_for_scene", "validate_snapshot", "apply_snapshot"]:
            if not armor.has_method(method): return "Converted armor has incompatible snapshot support."
        error = armor.call("validate_snapshot", player["armor"])
        if not error.is_empty(): return error
        var checked_armor: Resource = armor.call("clone_for_scene")
        if checked_armor == null: return "Converted armor could not be cloned."
        error = checked_armor.call("apply_snapshot", player["armor"])
        if not error.is_empty(): return error
        var checked_life: Resource = life.duplicate(false)
        error = checked_life.call("apply_snapshot", player["life"])
        if not error.is_empty(): return error
    var saved_magick: Dictionary = player["magick"]
    var saved_inventory: Dictionary = player["inventory"]
    # Validate on a private copy before changing any live save component.
    var checked_inventory: Resource = inventory.call("clone_for_scene")
    error = _restore_inventory(checked_inventory, saved_inventory)
    if not error.is_empty():
        return error
    if value["actors"].has(GLOBAL):
        for property_name in FIRST_PICKUP_FLAGS + TRAINING_FLAGS:
            if not value["actors"][GLOBAL]["brain"].has(property_name): continue
            if not _has_boolean_field(global_brain, property_name):
                return "Converted manor has no saved Boolean Global Brain field: " + property_name
    if value.has("training"):
        error = _preflight_training(level, value["training"], checked_inventory)
        if not error.is_empty(): return error
    var saved_world: Dictionary = value.get("world", {})
    for identity in BARRELS:
        if not saved_world.has(identity): continue
        var barrel_brain := _source_brain(level, identity)
        if _brain_path(level, identity).is_empty():
            return "Converted manor has no saved barrel binding: " + identity
        if barrel_brain == null:
            if saved_world[identity]["removed"]: continue
            return "Converted manor has no live saved barrel: " + identity
        if not _valid_barrel(barrel_brain):
            return "Converted manor has incompatible saved barrel: " + identity
        if saved_world[identity]["removed"] and barrel_brain.get_parent().has_method("source_kill_perso") and not _training_removal_supported(barrel_brain.get_parent()):
            return "Converted manor has an unsupported saved barrel removal policy."
        if barrel_brain.get_parent().is_queued_for_deletion() and not saved_world[identity]["removed"]:
            return "Converted manor cannot restore a removed barrel: " + identity
    if int(value["schema"]) == SCHEMA:
        life.call("apply_snapshot", player["life"])
        armor_state.call("restore_snapshot", player["armor"])

    magick.set("current_points", int(saved_magick["current_points"]))
    magick.set("maximum_points", int(saved_magick["maximum_points"]))
    magick.set("selected_spell_code", int(saved_magick["selected_spell_code"]))
    _restore_inventory(inventory, saved_inventory)
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
        for property_name in FIRST_PICKUP_FLAGS + TRAINING_FLAGS:
            if saved_global_brain.has(property_name):
                global_brain.set(property_name, saved_global_brain[property_name])
    var world: Dictionary = value.get("world", {})
    var dummy_state: Dictionary = world.get(DUMMY, {"dsg_8": false})
    dummy_brain.set("dsg_8", dummy_state["dsg_8"])
    for identity in BARRELS:
        if not world.has(identity): continue
        var barrel_brain := _source_brain(level, identity)
        var state: Dictionary = world[identity]
        if barrel_brain == null or barrel_brain.get_parent().is_queued_for_deletion(): continue
        barrel_brain.set("dsg_8", state["dsg_8"])
        # Normal0 deactivates a consumed regular barrel (selector below99).
        # Apply before any scheduled decision; never respawn its transient drop.
        if state["dsg_8"] and int(barrel_brain.get("dsg_4")) < 99:
            var barrel_actor := barrel_brain.get_parent()
            if barrel_actor.has_method("source_set_object_active_state"):
                barrel_actor.call("source_set_object_active_state", false)
        if state["removed"]: barrel_brain.get_parent().call("source_kill")
    if value.has("training"): _restore_training(level, value["training"])
    return ""
