extends SceneTree
## Synthetic contract check; uses no user's save files or generated game data.
const SAVE_STATE = preload("res://game/save_state.gd")
const INVENTORY = preload("res://addons/astrolabe/character/inventory.gd")
const ITEM = preload("res://addons/astrolabe/character/inventory_definition.gd")
const STARTUP = preload("res://game/save_startup.gd")
# Parse the actual production entry points without loading generated game data.
const BOOTSTRAP = preload("res://game/bootstrap.gd")
const GAME_SESSION = preload("res://game/game_session.gd")
const PICKER_SCRIPT = preload("res://game/load_game_picker.gd")
const PICKER = preload("res://game/load_game_picker.tscn")

class ObservingBrain extends "res://addons/astrolabe/behavior/behavior.gd":
    var entered_after_restore := false

    func _enter_normal(_token: int) -> void:
        entered_after_restore = bool(get_parent().get_parent().get_meta("saved_progression", false))


func _initialize() -> void:
    call_deferred("run")


func run() -> void:
    var partial := {
        "schema": 1, "level": "manoir",
        "actors": {SAVE_STATE.ACTOR: {
            "inventory": {"quantities": {"21": 0, "61": 0, "80": 0}, "selected_weapon": 80},
            "magick": {"current_points": 30, "maximum_points": 50, "selected_spell_code": 61},
        }},
        "world": {SAVE_STATE.DUMMY: {"dsg_8": false}},
    }
    if not SAVE_STATE.validate(partial).is_empty() or SAVE_STATE.validate_load(partial).is_empty():
        fail("A component overlay was mistaken for a resumable checkpoint.")
        return
    var native := SAVE_STATE._load_candidate({"status": "loaded", "state": partial}, SAVE_STATE.NATIVE_SOURCE)
    if native["status"] != "error" or native.has("state") or native["source"] != "native" or native["path"] != SAVE_STATE.SLOT:
        fail("Incomplete native candidate retained usable state or lost its identity.")
        return
    var complete: Dictionary = partial.duplicate(true)
    complete["actors"][SAVE_STATE.ACTOR]["transform"] = {"position": [1.0, 2.0, 3.0], "basis": [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]}
    complete["actors"][SAVE_STATE.ACTOR]["inventory"]["gold"] = 37
    complete["actors"][SAVE_STATE.GLOBAL] = {"brain": {"dsg_147": true, "dsg_149": true}}
    if not SAVE_STATE.validate_load(complete).is_empty():
        fail("A complete original overlay was rejected.")
        return
    var original := SAVE_STATE._load_candidate({"status": "loaded", "state": complete, "slot": "SaveGame0", "display_name": "Gogoud's Manoir I"}, SAVE_STATE.ORIGINAL_SOURCE)
    if original["status"] != "loaded" or original["source"] != "original" or original["path"] != SAVE_STATE.ORIGINAL_IMPORT or not original["label"].contains("SaveGame0"):
        fail("Original selection lost its slot identity.")
        return
    var picker := PICKER.instantiate() as PICKER_SCRIPT
    root.add_child(picker)
    var candidates: Array[Dictionary] = [native, original]
    picker.show_candidates(candidates)
    var buttons := picker.find_children("*", "Button", true, false)
    var save_buttons: Array[Button] = []
    for button: Button in buttons:
        if button != picker.back: save_buttons.append(button)
    if save_buttons.size() != 2 or not save_buttons[0].disabled or save_buttons[1].disabled or not save_buttons[1].text.contains("SaveGame0"):
        picker.queue_free()
        fail("The actual picker hid the invalid native reason or blocked the original candidate.")
        return
    var selected: Array[String] = []
    picker.selected.connect(func(source: String): selected.append(source))
    save_buttons[1].pressed.emit()
    picker.queue_free()
    if selected != [SAVE_STATE.ORIGINAL_SOURCE]:
        fail("The actual picker did not emit the explicitly selected original source.")
        return
    # Explicit false is source state, not absence and not a completed opening.
    complete["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_147"] = false
    if not SAVE_STATE.validate_load(complete).is_empty():
        fail("Explicit saved opening state false was rejected.")
        return
    complete["actors"][SAVE_STATE.GLOBAL]["brain"].erase("dsg_147")
    if SAVE_STATE.validate_load(complete).is_empty():
        fail("Missing opening progression inherited New Game state.")
        return
    complete["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_147"] = true
    complete.erase("world")
    if SAVE_STATE.validate_load(complete).is_empty():
        fail("Missing world state inherited the manor fixture.")
        return
    var unknown := SAVE_STATE.read_for_load("another-source")
    if unknown["status"] != "error":
        fail("Unknown explicit save source was accepted.")
        return
    if not check_gold(): return
    if not await check_startup(): return
    print("SAVE_LOAD_SELECTION_PROOF incomplete native overlay rejected; original slot identity retained; explicit false preserved; missing placement/progression/world and unknown source rejected; source-bound gold migration retains zero and balance without admitting unknown items; all automatic Brain entries observe restored progression")
    quit(0)


func fail(message: String) -> void:
    push_error(message)
    quit(1)


func check_gold() -> bool:
    var inventory := INVENTORY.new()
    var weapon := ITEM.new()
    weapon.source_key = "fix:/inventory/synthetic/weapon"
    weapon.item_id = 80
    weapon.kind = "Weapon"
    weapon.quantity = 8
    weapon.maximum = 20
    weapon.pickup_category = "weapons"
    weapon.selected = true
    inventory.definitions.append(weapon)
    inventory.categories = {"weapons": [0]}
    inventory.source_initialized = true
    inventory.capture_snapshot()
    var gold := ITEM.new()
    gold.source_key = "fix:/inventory/synthetic/gold"
    gold.item_id = 0
    gold.maximum = 200
    inventory.gold_definition = gold
    inventory.gold_category = "gold"
    inventory.gold = 37
    for amount in [0, 20]:
        var error := SAVE_STATE._restore_inventory(inventory, {"quantities": {"0": amount, "80": 8}})
        if not error.is_empty() or inventory.gold != amount or inventory.quantities.has(0) or inventory.quantity(80) != 8:
            fail("Legacy item-0 balance was lost or admitted as an ordinary item: " + error)
            return false
    var error := SAVE_STATE._restore_inventory(inventory, {"gold": 43, "quantities": {"80": 8}})
    if not error.is_empty() or inventory.gold != 43:
        fail("Typed original gold was not restored: " + error)
        return false
    for invalid in [
        {"gold": 43, "quantities": {"0": 20, "80": 8}},
        {"gold": 201, "quantities": {"80": 8}},
        {"quantities": {"0": 20, "80": 8, "32767": 1}},
    ]:
        error = SAVE_STATE._restore_inventory(inventory, invalid)
        if error.is_empty() or inventory.gold != 43 or inventory.quantity(80) != 8:
            fail("Invalid gold/ordinary state was accepted or changed live inventory.")
            return false
    inventory.gold_definition = null
    if SAVE_STATE._restore_inventory(inventory, {"quantities": {"0": 20, "80": 8}}).is_empty():
        fail("Legacy gold was accepted without its source component binding.")
        return false
    return true


func check_startup() -> bool:
    var level := Node3D.new()
    var brains: Array[ObservingBrain] = []
    for index in 2:
        var owner := Node3D.new()
        owner.name = "Actor" + str(index)
        level.add_child(owner)
        var emitter := Node3D.new()
        emitter.name = "SoundEmitter"
        owner.add_child(emitter)
        var brain := ObservingBrain.new()
        brain.name = "Brain"
        brain.main_actor_path = ^".."
        owner.add_child(brain)
        brains.append(brain)
    var startup := STARTUP.new()
    var error := startup.hold_before_tree(level)
    if not error.is_empty():
        level.free()
        fail(error)
        return false
    root.add_child(level)
    error = startup.hold_after_tree()
    await process_frame
    if not error.is_empty() or brains.any(func(brain): return brain.running or brain.entered_after_restore):
        level.queue_free()
        fail("A compiled Brain entered before save restoration: " + error)
        return false
    level.set_meta("saved_progression", true)
    startup.release_physics()
    error = startup.start_brains()
    if not error.is_empty() or brains.any(func(brain): return not brain.running or not brain.entered_after_restore):
        level.queue_free()
        fail("A compiled Brain did not enter with restored progression: " + error)
        return false
    level.queue_free()
    await process_frame
    return true
