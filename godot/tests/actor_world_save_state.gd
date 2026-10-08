extends SceneTree
## Regression for the production slot, replacing f21a7fc's detached Brain helper test.
## All values and actors below are synthetic; no original save bytes are replayed.

const SAVE_STATE = preload("res://game/save_state.gd")
const FIXTURE = preload("res://tests/actor_world_save_fixture.gd")

class GlobalWithoutOptional extends Node:
    var dsg_149: bool = false

class InvalidDummyBrain extends Node:
    var dsg_8: int = 0

class InvalidGlobalBrain extends Node:
    var dsg_149: int = 0
    var dsg_147: bool = true

class InvalidOptionalBrain extends Node:
    var dsg_149: bool = false
    var dsg_147: int = 0

class InvalidBarrelBrain extends Node:
    var source_model_path: String = "level:/ai/models/488"
    var dsg_4: int = 0
    var dsg_8: int = 0

func _initialize() -> void:
    call_deferred("run")

func run() -> void:
    var arguments := OS.get_cmdline_user_args()
    if arguments.size() != 1 or not ["write", "reload"].has(arguments[0]):
        finish("Expected write or reload stage.")
        return
    var context := FIXTURE.create(root)
    var actor: CharacterBody3D = context["actor"]
    var level: Node3D = context["level"]
    var global_brain: Node = context["global_brain"]
    var dummy_brain: Node = context["dummy_brain"]
    if SAVE_STATE.checkpoint_error(actor, level).is_empty():
        finish("Ungrounded player was admitted as a checkpoint.")
        return
    await FIXTURE.settle(actor, self)
    var error: String
    if arguments[0] == "write": error = await write_stage(actor, level, global_brain, dummy_brain)
    else: error = await reload_stage(actor, level, global_brain, dummy_brain)
    finish(error)

func write_stage(actor: CharacterBody3D, level: Node3D, global_brain: Node, dummy_brain: Node) -> String:
    var magick: Resource = actor.get("source_magick")
    var inventory: Resource = actor.get("source_inventory")
    magick.current_points = 23
    inventory.definitions[0].quantity = 2
    inventory.definitions[0].maximum = 30
    inventory.gold = 73
    inventory.gold_definition.maximum = 250
    inventory.capture_snapshot()
    global_brain.dsg_149 = true
    global_brain.dsg_147 = false
    dummy_brain.dsg_8 = true
    for index in SAVE_STATE.FIRST_PICKUP_FLAGS.size():
        global_brain.set(SAVE_STATE.FIRST_PICKUP_FLAGS[index], index % 2 == 0)
    level.get_node("Barrel0/Brain").set("dsg_8", true)
    level.get_node("Barrel1").call("source_kill")
    await process_frame
    var life: Resource = actor.get("source_hit_points")
    life.current = 6
    life.maximum = 11
    life.limit = 13
    var armor: Resource = level.get_meta("source_armor_catalog")
    armor.selected = 1
    var definition: Resource = armor.definitions[1]
    definition.resistance_rank = 3
    definition.broken_count = 1
    definition.current_damage_piece = 1
    definition.pieces[0].broken = 1
    definition.pieces[0].strikes = 5
    definition.pieces[1].strikes = 2
    var error: String = SAVE_STATE.write(actor, level)
    if not error.is_empty(): return error
    var saved: Dictionary = SAVE_STATE.read()
    if saved.get("status") != "loaded": return "Production slot could not be read after write."
    error = check_saved_fields(saved["state"])
    if not error.is_empty(): return error
    var prior := FileAccess.get_file_as_string(SAVE_STATE.path())
    error = check_checkpoint_rejections(actor, level, prior)
    if not error.is_empty(): return error
    magick.current_points = 1000
    if SAVE_STATE.write(actor, level).is_empty(): return "Invalid live values were saved."
    if FileAccess.get_file_as_string(SAVE_STATE.path()) != prior: return "Rejected write replaced the valid slot."
    print("ACTOR_SLOT_WRITE production slot persisted named Global/dummy Boolean fields, native life/armor, full inventory, four placed barrels, first-pickup flags and semantic player identity; checkpoint rejection preserved the valid slot")
    return ""

func reload_stage(actor: CharacterBody3D, level: Node3D, global_brain: Node, dummy_brain: Node) -> String:
    var saved: Dictionary = SAVE_STATE.read()
    if saved.get("status") != "loaded": return "Fresh process did not read the production slot."
    var state: Dictionary = saved["state"]
    var error := check_saved_fields(state)
    if not error.is_empty(): return error
    var life: Resource = actor.get("source_hit_points")
    var armor: Resource = level.get_meta("source_armor_catalog")
    var armor_state: Resource = level.get_meta("source_armor_state")
    var notifications: Array[int] = []
    armor_state.selection_changed.connect(func(index: int): notifications.append(index))
    var magick: Resource = actor.get("source_magick")
    var inventory: Resource = actor.get("source_inventory")
    var sibling: Resource = armor.clone_for_scene()
    for attempt in 2:
        error = SAVE_STATE.apply(actor, level, state)
        if not error.is_empty(): return error
        if magick.current_points != 23 or int(inventory.quantities.get(21, -1)) != 2:
            return "Production component values did not survive restore."
        if inventory.gold != 73 or inventory.gold_definition.maximum != 250 or inventory.definitions[0].maximum != 30 or inventory.definitions[1].quantity != 6 or inventory.selected_weapon != 21 or inventory.categories != {"weapons": [0, -1], "objects": [-1]}:
            return "Full native inventory admission, capacities, selection or special gold did not survive restore."
        if level.get_node("Barrel0").visible or not level.get_node("Barrel0/Brain").get("dsg_8"):
            return "Consumed placed barrel did not remain deactivated."
        for index in SAVE_STATE.FIRST_PICKUP_FLAGS.size():
            if global_brain.get(SAVE_STATE.FIRST_PICKUP_FLAGS[index]) != (index % 2 == 0):
                return "First-pickup flags did not survive restore."
        if not global_brain.dsg_149 or global_brain.dsg_147 or not dummy_brain.dsg_8:
            return "Named Brain Boolean values did not survive restore."
        if life.current != 6 or life.maximum != 11 or life.limit != 13 or armor.selected != 1:
            return "Native life or armor selection did not survive restore."
        if armor_state.selected_index != 1 or notifications != [1]:
            return "Restore did not synchronize the native armor facade once."
        var definition: Resource = armor.definitions[1]
        if definition.resistance_rank != 3 or definition.broken_count != 1 or definition.current_damage_piece != 1 or definition.pieces[0].broken != 1 or definition.pieces[0].strikes != 5 or definition.pieces[1].strikes != 2:
            return "Native armor damage state did not survive restore."
        if sibling.selected != 0 or sibling.definitions[1].pieces[0].broken != 0:
            return "Slot restore changed another level's owned armor copy."
        var observed: Resource = level.get_node("ArmorObserver").get("source_armor")
        if observed != armor or observed.selected != 1:
            return "Restore replaced shared armor ownership instead of updating the level's catalogue."
        # JSON parses numeric scalars as floats; compare both serialized forms.
        if JSON.parse_string(JSON.stringify(SAVE_STATE.capture(actor, level))) != state:
            return "Repeated production restore/capture changed the slot state."
        if attempt == 0: await process_frame
    if level.get_node_or_null("Barrel1") != null:
        return "Removed placed barrel survived the native tombstone restore."
    if not actor.get_node("Brain").get("dsg_11"):
        return "Restore changed unmapped Hype DSG[11]."
    error = check_component_rejections(actor, level, state)
    if not error.is_empty(): return error
    error = check_legacy_and_byte_semantics(actor, level, state)
    if not error.is_empty(): return error

    # A valid earlier change must not leak when another field invalidates the slot.
    var invalid := state.duplicate(true)
    invalid["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_149"] = false
    invalid["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_9999"] = false
    error = check_atomic_rejection(actor, level, invalid)
    if not error.is_empty(): return error
    invalid = state.duplicate(true)
    invalid["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_149"] = false
    invalid["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_147"] = 1
    error = check_atomic_rejection(actor, level, invalid)
    if not error.is_empty(): return error
    invalid = state.duplicate(true)
    invalid["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_149"] = false
    invalid["world"][SAVE_STATE.DUMMY]["dsg_8"] = 1
    error = check_atomic_rejection(actor, level, invalid)
    if not error.is_empty(): return error
    # f21a7fc's demonstration fields are not admitted by the actual slot schema.
    invalid = state.duplicate(true)
    invalid["actors"][SAVE_STATE.ACTOR]["brain"] = {"dsg_11": false}
    error = check_atomic_rejection(actor, level, invalid)
    if not error.is_empty(): return error
    invalid = state.duplicate(true)
    invalid["world"]["level:/persos/world/0/174"] = {"dsg_1": true}
    error = check_atomic_rejection(actor, level, invalid)
    if not error.is_empty(): return error

    var omitted := state.duplicate(true)
    omitted["actors"][SAVE_STATE.GLOBAL]["brain"].erase("dsg_147")
    global_brain.dsg_147 = true
    error = SAVE_STATE.apply(actor, level, omitted)
    if not error.is_empty() or not global_brain.dsg_147:
        return "Omitted optional Boolean did not preserve its scene value."
    # Missing optional live property must be rejected before changing other components.
    global_brain.get_parent().remove_child(global_brain)
    var without_optional := GlobalWithoutOptional.new()
    without_optional.name = "Brain"
    level.get_node("Global").add_child(without_optional)
    error = check_atomic_rejection(actor, level, state)
    if not error.is_empty(): return error
    level.get_node("Global").remove_child(without_optional)
    without_optional.free()
    level.get_node("Global").add_child(global_brain)
    # A valid envelope must not partially mutate other components when the
    # authored live Brain has a wrong property type or no mapped property.
    for brain in [InvalidDummyBrain.new(), Node.new()]:
        error = check_invalid_live_brain(actor, level, "Dummy", brain, state)
        if not error.is_empty(): return error
    for brain in [InvalidGlobalBrain.new(), InvalidOptionalBrain.new(), Node.new()]:
        error = check_invalid_live_brain(actor, level, "Global", brain, state)
        if not error.is_empty(): return error
    error = check_invalid_live_brain(actor, level, "Barrel0", InvalidBarrelBrain.new(), state)
    if not error.is_empty(): return error
    var bindings: Dictionary = level.get_meta("source_actor_brains").duplicate()
    var invalid_bindings := bindings.duplicate()
    invalid_bindings.erase(SAVE_STATE.BARRELS[1])
    level.set_meta("source_actor_brains", invalid_bindings)
    error = check_atomic_rejection(actor, level, state)
    level.set_meta("source_actor_brains", bindings)
    if not error.is_empty(): return error
    error = check_restored_damage_behavior(actor, level)
    if not error.is_empty(): return error
    print("ACTOR_SLOT_RELOAD fresh process restored named Boolean fields and native life/armor, inventory and placed barrel tombstones; repeats were idempotent; invalid slots were atomic; unmapped fields rejected; omission preserved scene state; restored armor/life handled damage")
    return ""

func check_saved_fields(state: Dictionary) -> String:
    if int(state.get("schema", 0)) != SAVE_STATE.SCHEMA or state.get("checkpoint") != SAVE_STATE.CHECKPOINT or state["actors"][SAVE_STATE.ACTOR].get("asset") != SAVE_STATE.PLAYER:
        return "Production slot did not retain checkpoint/player identity."
    var fields: Dictionary = state["actors"][SAVE_STATE.GLOBAL]["brain"]
    if fields.get("dsg_149") != true or fields.get("dsg_147") != false or state["world"][SAVE_STATE.DUMMY].get("dsg_8") != true:
        return "Production slot did not preserve exact Boolean values."
    if not state["world"][SAVE_STATE.BARRELS[0]]["dsg_8"] or state["world"][SAVE_STATE.BARRELS[0]]["removed"] or not state["world"][SAVE_STATE.BARRELS[1]]["removed"]:
        return "Placed barrel consumption/tombstone did not survive the slot."
    var snapshot: Dictionary = state["actors"][SAVE_STATE.ACTOR]["inventory"]["snapshot"]
    if int(snapshot["version"]) != 2 or int(snapshot["gold"]) != 73 or snapshot["catalogue"].size() != 2:
        return "Full inventory snapshot did not survive the JSON slot."
    return ""

func check_atomic_rejection(actor: CharacterBody3D, level: Node3D, state: Dictionary) -> String:
    var prior: Dictionary = SAVE_STATE.capture(actor, level)
    if SAVE_STATE.apply(actor, level, state).is_empty(): return "Invalid slot was accepted."
    if SAVE_STATE.capture(actor, level) != prior: return "Rejected slot partially changed live state."
    return ""

func check_invalid_live_brain(actor: CharacterBody3D, level: Node3D, owner: String, replacement: Node, state: Dictionary) -> String:
    var parent := level.get_node(owner)
    var original := parent.get_node("Brain")
    parent.remove_child(original)
    replacement.name = "Brain"
    parent.add_child(replacement)
    var magick: Resource = actor.get("source_magick")
    var inventory: Resource = actor.get("source_inventory")
    var before_mana: int = magick.current_points
    var before_quantities: Dictionary = inventory.quantities.duplicate()
    var life: Resource = actor.get("source_hit_points")
    var armor: Resource = level.get_meta("source_armor_catalog")
    var before_life: Dictionary = life.capture_snapshot()
    var before_armor: Dictionary = armor.capture_snapshot()
    var changed := state.duplicate(true)
    changed["actors"][SAVE_STATE.ACTOR]["life"]["current"] = 2
    changed["actors"][SAVE_STATE.ACTOR]["armor"]["selected"] = 0
    changed["actors"][SAVE_STATE.ACTOR]["magick"]["current_points"] = 17
    changed["actors"][SAVE_STATE.ACTOR]["inventory"]["quantities"]["21"] = 1
    var error: String = SAVE_STATE.apply(actor, level, changed)
    var capture: Dictionary = SAVE_STATE.capture(actor, level)
    var unchanged: bool = magick.current_points == before_mana and inventory.quantities == before_quantities and life.capture_snapshot() == before_life and armor.capture_snapshot() == before_armor
    parent.remove_child(replacement)
    replacement.free()
    parent.add_child(original)
    if error.is_empty() or not unchanged or not capture.is_empty():
        return "Incompatible live " + owner + " Brain was accepted or partially changed components."
    return ""

func check_checkpoint_rejections(actor: CharacterBody3D, level: Node3D, prior_slot: String) -> String:
    var brain := actor.get_node("Brain")
    for variation in [
        [actor, "current_state", 46], [actor, "pending_state", 1],
        [actor, "source_airborne", true], [actor, "velocity", Vector3(0.1, 0, 0)],
        [actor, "external_velocity", Vector3(0.1, 0, 0)],
        [actor, "external_velocity", Vector3(NAN, 0, 0)],
        [brain, "current_normal", 3], [brain, "current_reflex", 2],
        [brain, "running", false], [brain, "unsupported_failure", "synthetic unsupported"],
        [actor.get("source_hit_points"), "current", 0],
    ]:
        var owner: Object = variation[0]
        var field: String = variation[1]
        var original: Variant = owner.get(field)
        owner.set(field, variation[2])
        var rejected: bool = not SAVE_STATE.write(actor, level).is_empty()
        owner.set(field, original)
        if not rejected or FileAccess.get_file_as_string(SAVE_STATE.path()) != prior_slot:
            return "Unsafe checkpoint " + field + " was admitted or replaced the slot."
    var projectile := Node3D.new()
    level.add_child(projectile)
    actor.set("generated_object", projectile)
    var rejected: bool = not SAVE_STATE.write(actor, level).is_empty()
    actor.set("generated_object", null)
    if not rejected: return "Held generated object was admitted as a checkpoint."
    projectile.set_meta("astrolabe_generated_template", "synthetic/template")
    rejected = not SAVE_STATE.write(actor, level).is_empty()
    projectile.free()
    if not rejected: return "Active generated object was admitted as a checkpoint."
    InputMap.add_action("Button_SyntheticCheckpoint")
    Input.action_press("Button_SyntheticCheckpoint")
    rejected = not SAVE_STATE.write(actor, level).is_empty()
    Input.action_release("Button_SyntheticCheckpoint")
    InputMap.erase_action("Button_SyntheticCheckpoint")
    if not rejected or FileAccess.get_file_as_string(SAVE_STATE.path()) != prior_slot:
        return "Held controls were admitted or a rejected checkpoint changed the slot."
    if not SAVE_STATE.checkpoint_error(actor, level).is_empty(): return "Checkpoint rejection left a valid player blocked."
    return ""

func check_component_rejections(actor: CharacterBody3D, level: Node3D, state: Dictionary) -> String:
    var player: Dictionary = state["actors"][SAVE_STATE.ACTOR]
    for variation in [
        ["asset", "level:/persos/world/0/999"],
        ["life", {"version": 1, "current": 256, "maximum": 10, "limit": 12}],
        ["life", {"version": 1, "current": 1.5, "maximum": 10, "limit": 12}],
        ["life", {"version": 1, "current": INF, "maximum": 10, "limit": 12}],
        ["transform", {"position": [1e40, 0, 0], "basis": [[1, 0, 0], [0, 1, 0], [0, 0, 1]]}],
        ["transform", {"position": [0, 0, 0], "basis": [[0, 0, 0], [0, 1, 0], [0, 0, 1]]}],
    ]:
        var invalid := state.duplicate(true)
        invalid["actors"][SAVE_STATE.ACTOR]["magick"]["current_points"] = 17
        invalid["actors"][SAVE_STATE.ACTOR][variation[0]] = variation[1]
        var error := check_atomic_rejection(actor, level, invalid)
        if not error.is_empty(): return error
    for field in ["source", "rank", "broken_count", "current_piece"]:
        var invalid := state.duplicate(true)
        var definition: Dictionary = invalid["actors"][SAVE_STATE.ACTOR]["armor"]["definitions"][1]
        definition[field] = "fix:/armor/synthetic/wrong" if field == "source" else 999
        invalid["actors"][SAVE_STATE.ACTOR]["life"]["current"] = 2
        var error := check_atomic_rejection(actor, level, invalid)
        if not error.is_empty(): return error
    var invalid := state.duplicate(true)
    invalid["actors"][SAVE_STATE.ACTOR]["life"]["current"] = 2
    invalid["actors"][SAVE_STATE.ACTOR]["armor"]["definitions"][1]["pieces"][0]["source"] = "fix:/armor/synthetic/wrong-piece"
    var error := check_atomic_rejection(actor, level, invalid)
    if not error.is_empty(): return error
    for field in ["source_hit_points"]:
        var original: Resource = actor.get(field)
        var prior := SAVE_STATE.capture(actor, level)
        actor.set(field, null)
        error = SAVE_STATE.apply(actor, level, state)
        var invalid_capture := SAVE_STATE.capture(actor, level)
        actor.set(field, original)
        if error.is_empty() or not invalid_capture.is_empty() or SAVE_STATE.capture(actor, level) != prior:
            return "Missing native " + field + " was accepted or changed live state."
    for key in ["source_armor_catalog", "source_armor_state"]:
        var original: Resource = level.get_meta(key)
        var prior := SAVE_STATE.capture(actor, level)
        level.remove_meta(key)
        error = SAVE_STATE.apply(actor, level, state)
        var invalid_capture := SAVE_STATE.capture(actor, level)
        level.set_meta(key, original)
        if error.is_empty() or not invalid_capture.is_empty() or SAVE_STATE.capture(actor, level) != prior:
            return "Missing native armor owner was accepted or changed live state."
    var facade: Resource = level.get_meta("source_armor_state")
    var catalog: Resource = facade.catalog
    var bound_prior := SAVE_STATE.capture(actor, level)
    facade.catalog = catalog.clone_for_scene()
    error = SAVE_STATE.apply(actor, level, state)
    var invalid_binding_capture := SAVE_STATE.capture(actor, level)
    facade.catalog = catalog
    if error.is_empty() or not invalid_binding_capture.is_empty() or SAVE_STATE.capture(actor, level) != bound_prior:
        return "Unbound native armor facade was accepted or changed live state."
    var prior := SAVE_STATE.capture(actor, level)
    actor.set_meta("source_perso", "level:/persos/world/0/999")
    var rejected: bool = SAVE_STATE.resolve_player(level) == null and not SAVE_STATE.apply(actor, level, state).is_empty() and SAVE_STATE.capture(actor, level).is_empty()
    actor.set_meta("source_perso", SAVE_STATE.PLAYER)
    if not rejected or SAVE_STATE.capture(actor, level) != prior: return "Wrong live player identity was accepted or changed components."
    var paths: Dictionary = level.get_meta("source_actor_nodes").duplicate()
    level.set_meta("source_actor_nodes", {SAVE_STATE.PLAYER: 17})
    rejected = SAVE_STATE.resolve_player(level) == null and not SAVE_STATE.apply(actor, level, state).is_empty()
    level.set_meta("source_actor_nodes", paths)
    if not rejected or SAVE_STATE.capture(actor, level) != prior: return "Malformed player map was accepted or changed components."
    level.set_meta("source_level", "other-level")
    rejected = not SAVE_STATE.apply(actor, level, state).is_empty() and not SAVE_STATE.write(actor, level).is_empty()
    level.set_meta("source_level", "manoir")
    if not rejected or SAVE_STATE.capture(actor, level) != prior: return "Different level context was accepted or changed components."
    if player.has("brain"): return "Test admitted an unproved Hype Brain snapshot."
    return ""

func check_legacy_and_byte_semantics(actor: CharacterBody3D, level: Node3D, state: Dictionary) -> String:
    var life: Resource = actor.get("source_hit_points")
    var armor: Resource = level.get_meta("source_armor_catalog")
    var prior_life: Dictionary = life.capture_snapshot()
    var prior_armor: Dictionary = armor.capture_snapshot()
    var legacy := state.duplicate(true)
    legacy["schema"] = 1
    legacy.erase("checkpoint")
    for field in ["asset", "life", "armor"]: legacy["actors"][SAVE_STATE.ACTOR].erase(field)
    legacy["actors"][SAVE_STATE.ACTOR]["inventory"].erase("snapshot")
    legacy["actors"][SAVE_STATE.ACTOR]["transform"] = {"position": [2, 0, 3]}
    var error: String = SAVE_STATE.apply(actor, level, legacy)
    if not error.is_empty() or actor.global_position != Vector3(2, 0, 3) or life.capture_snapshot() != prior_life or armor.capture_snapshot() != prior_armor:
        return "Legacy import did not preserve seeded life/armor or optional transform."
    var byte_state := state.duplicate(true)
    byte_state["actors"][SAVE_STATE.ACTOR]["life"] = {"version": 1, "current": 200, "maximum": 10, "limit": 0}
    byte_state["actors"][SAVE_STATE.ACTOR]["transform"]["basis"] = [[2, 0, 0], [0.2, 1, 0], [0, 0, 1]]
    error = SAVE_STATE.apply(actor, level, byte_state)
    if not error.is_empty() or life.current != 200 or life.maximum != 10 or life.limit != 0 or actor.global_basis.x.x != 2.0:
        return "Restore invented life clamping or discarded native scaled/sheared basis."
    error = SAVE_STATE.apply(actor, level, state)
    if not error.is_empty(): return error
    var guard = preload("res://addons/astrolabe/character/snapshot_values.gd")
    if not guard.integer(1.0, 0, 255) or guard.integer(1.5, 0, 255) or guard.integer(INF, 0, 255) or guard.integer(NAN, 0, 255) or guard.integer(true, 0, 255) or guard.integer(1e100, 0, 255):
        return "Shared snapshot scalar guard did not safely admit JSON integral numbers."
    if guard.item_key("01") or guard.item_key("+1") or guard.item_key("99999999999999999999") or not guard.item_key("-1"):
        return "Shared snapshot item keys are noncanonical or lost the special sentinel."
    return ""

func check_restored_damage_behavior(actor: CharacterBody3D, level: Node3D) -> String:
    var life: Resource = actor.get("source_hit_points")
    var armor: Resource = level.get_meta("source_armor_catalog")
    var armor_state: Resource = level.get_meta("source_armor_state")
    var effective: int = armor_state.damage(3)
    if effective != 3 or armor.definitions[1].pieces[1].strikes != 3 or armor.definitions[1].pieces[0].strikes != 5 or armor.definitions[1].broken_count != 1:
        return "Restored armor did not continue the native damage operation on its selected piece."
    if life.subtract_damage(effective) != 3:
        return "Restored life did not consume native effective armor damage."
    return ""

func finish(error: String) -> void:
    if not error.is_empty(): push_error(error)
    for child in root.get_children(): child.queue_free()
    await process_frame
    await process_frame
    quit(0 if error.is_empty() else 1)
