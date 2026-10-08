extends SceneTree
## Regression for the production slot, replacing f21a7fc's detached Brain helper test.
## All values and actors below are synthetic; no original save bytes are replayed.

const SAVE_STATE = preload("res://game/save_state.gd")
const MAGICK = preload("res://addons/astrolabe/character/magick.gd")
const INVENTORY = preload("res://addons/astrolabe/character/inventory.gd")

class Player extends CharacterBody3D:
    var source_magick: Resource = MAGICK.new()
    var source_inventory: Resource = INVENTORY.new()

class GlobalBrain extends Node:
    var dsg_149: bool = false
    var dsg_147: bool = true

class GlobalWithoutOptional extends Node:
    var dsg_149: bool = false

class DummyBrain extends Node:
    var dsg_8: bool = false

class PlayerBrain extends Node:
    var dsg_11: bool = true

class InvalidDummyBrain extends Node:
    var dsg_8: int = 0

class InvalidGlobalBrain extends Node:
    var dsg_149: int = 0
    var dsg_147: bool = true

class InvalidOptionalBrain extends Node:
    var dsg_149: bool = false
    var dsg_147: int = 0

func _initialize() -> void:
    call_deferred("run")

func run() -> void:
    var arguments := OS.get_cmdline_user_args()
    if arguments.size() != 1 or not ["write", "reload"].has(arguments[0]):
        finish("Expected write or reload stage.")
        return
    var level := Node3D.new()
    level.name = "SyntheticManor"
    root.add_child(level)
    var actor := Player.new()
    actor.name = "Hype"
    actor.set_meta("source_perso", "level:/persos/world/0/0")
    level.add_child(actor)
    var player_brain := PlayerBrain.new()
    player_brain.name = "Brain"
    actor.add_child(player_brain)
    var global_actor := Node3D.new()
    global_actor.name = "Global"
    global_actor.set_meta("source_perso", SAVE_STATE.GLOBAL)
    level.add_child(global_actor)
    var global_brain := GlobalBrain.new()
    global_brain.name = "Brain"
    global_actor.add_child(global_brain)
    var dummy_actor := Node3D.new()
    dummy_actor.name = "Dummy"
    dummy_actor.set_meta("source_perso", SAVE_STATE.DUMMY)
    level.add_child(dummy_actor)
    var dummy_brain := DummyBrain.new()
    dummy_brain.name = "Brain"
    dummy_actor.add_child(dummy_brain)
    level.set_meta("source_actor_nodes", {
        "level:/persos/world/0/0": NodePath("Hype"),
        SAVE_STATE.GLOBAL: NodePath("Global"),
        SAVE_STATE.DUMMY: NodePath("Dummy"),
    })
    level.set_meta("source_actor_brains", {
        SAVE_STATE.GLOBAL: NodePath("Global/Brain"),
        SAVE_STATE.DUMMY: NodePath("Dummy/Brain"),
    })
    actor.source_magick.current_points = 12
    actor.source_magick.maximum_points = 50
    actor.source_magick.selected_spell_code = 61
    actor.source_inventory.quantities = {21: 5}
    var error := write_stage(actor, level, global_brain, dummy_brain) if arguments[0] == "write" else reload_stage(actor, level, global_brain, dummy_brain)
    finish(error)

func write_stage(actor: Player, level: Node3D, global_brain: GlobalBrain, dummy_brain: DummyBrain) -> String:
    actor.source_magick.current_points = 23
    actor.source_inventory.quantities = {21: 2}
    global_brain.dsg_149 = true
    global_brain.dsg_147 = false
    dummy_brain.dsg_8 = true
    var error: String = SAVE_STATE.write(actor, level)
    if not error.is_empty(): return error
    var saved: Dictionary = SAVE_STATE.read()
    if saved.get("status") != "loaded": return "Production slot could not be read after write."
    error = check_saved_fields(saved["state"])
    if not error.is_empty(): return error
    var prior := FileAccess.get_file_as_string(SAVE_STATE.path())
    actor.source_magick.current_points = 1000
    if SAVE_STATE.write(actor, level).is_empty(): return "Invalid live values were saved."
    if FileAccess.get_file_as_string(SAVE_STATE.path()) != prior: return "Rejected write replaced the valid slot."
    print("ACTOR_SLOT_WRITE production slot persisted named Global/dummy Boolean fields and preserved a valid slot on rejection")
    return ""

func reload_stage(actor: Player, level: Node3D, global_brain: GlobalBrain, dummy_brain: DummyBrain) -> String:
    var saved: Dictionary = SAVE_STATE.read()
    if saved.get("status") != "loaded": return "Fresh process did not read the production slot."
    var state: Dictionary = saved["state"]
    var error := check_saved_fields(state)
    if not error.is_empty(): return error
    for attempt in 2:
        error = SAVE_STATE.apply(actor, level, state)
        if not error.is_empty(): return error
        if actor.source_magick.current_points != 23 or int(actor.source_inventory.quantities.get(21, -1)) != 2:
            return "Production component values did not survive restore."
        if not global_brain.dsg_149 or global_brain.dsg_147 or not dummy_brain.dsg_8:
            return "Named Brain Boolean values did not survive restore."
        # JSON parses numeric scalars as floats; compare both serialized forms.
        if JSON.parse_string(JSON.stringify(SAVE_STATE.capture(actor, level))) != state:
            return "Repeated production restore/capture changed the slot state."
    if not actor.get_node("Brain").get("dsg_11"):
        return "Restore changed unmapped Hype DSG[11]."

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
    print("ACTOR_SLOT_RELOAD fresh process restored named Boolean fields; repeats were idempotent; invalid slots were atomic; unmapped fields rejected; omission preserved scene state")
    return ""

func check_saved_fields(state: Dictionary) -> String:
    var fields: Dictionary = state["actors"][SAVE_STATE.GLOBAL]["brain"]
    if fields.get("dsg_149") != true or fields.get("dsg_147") != false or state["world"][SAVE_STATE.DUMMY].get("dsg_8") != true:
        return "Production slot did not preserve exact Boolean values."
    return ""

func check_atomic_rejection(actor: Player, level: Node3D, state: Dictionary) -> String:
    var prior: Dictionary = SAVE_STATE.capture(actor, level)
    if SAVE_STATE.apply(actor, level, state).is_empty(): return "Invalid slot was accepted."
    if SAVE_STATE.capture(actor, level) != prior: return "Rejected slot partially changed live state."
    return ""

func check_invalid_live_brain(actor: Player, level: Node3D, owner: String, replacement: Node, state: Dictionary) -> String:
    var parent := level.get_node(owner)
    var original := parent.get_node("Brain")
    parent.remove_child(original)
    replacement.name = "Brain"
    parent.add_child(replacement)
    var before_mana: int = actor.source_magick.current_points
    var before_quantities: Dictionary = actor.source_inventory.quantities.duplicate()
    var changed := state.duplicate(true)
    changed["actors"][SAVE_STATE.ACTOR]["magick"]["current_points"] = 17
    changed["actors"][SAVE_STATE.ACTOR]["inventory"]["quantities"]["21"] = 1
    var error: String = SAVE_STATE.apply(actor, level, changed)
    var capture: Dictionary = SAVE_STATE.capture(actor, level)
    var unchanged: bool = actor.source_magick.current_points == before_mana and actor.source_inventory.quantities == before_quantities
    parent.remove_child(replacement)
    replacement.free()
    parent.add_child(original)
    if error.is_empty() or not unchanged or not capture.is_empty():
        return "Incompatible live " + owner + " Brain was accepted or partially changed components."
    return ""

func finish(error: String) -> void:
    if not error.is_empty(): push_error(error)
    for child in root.get_children(): child.queue_free()
    await process_frame
    await process_frame
    quit(0 if error.is_empty() else 1)
