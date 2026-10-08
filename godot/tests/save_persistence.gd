extends SceneTree
## Run in separate Godot processes with one isolated user-data directory.
const SAVE_STATE = preload("res://game/save_state.gd")
const LEVEL_SCENE_PATH := "res://generated/hype/levels/manoir/level.tscn"

class MockMagick extends Resource:
    var current_points: int = 12
    var maximum_points: int = 50
    var selected_spell_code: int = 61

class MockInventory extends Resource:
    var selected_weapon: int = -1
    var quantities: Dictionary = {"0": 20}

class MockActor extends CharacterBody3D:
    var source_magick: Resource = MockMagick.new()
    var source_inventory: Resource = MockInventory.new()

class MockGlobalBrain extends Node:
    var dsg_149: bool = false
    var dsg_147: bool = true

class MockGlobalBrainWithoutDsg147 extends Node:
    var dsg_149: bool = false

class MockDummyBrain extends Node:
    var dsg_8: bool = false

func _initialize() -> void:
    call_deferred("run")

func run() -> void:
    var optional_field_error := await test_optional_global_brain_field()
    if not optional_field_error.is_empty():
        fail(optional_field_error)
        return
    var args := OS.get_cmdline_user_args()
    if args.size() != 1 or not ["write", "reload", "corrupt", "optional", "optional-write", "optional-reload"].has(args[0]):
        fail("Expected write, reload, corrupt, optional, optional-write, or optional-reload stage")
        return
    var stage: String = args[0]
    if stage == "optional":
        print("SAVE_OPTIONAL_PROOF schema-1 omission preserved scene DSG[147], exposed false captured/restored, unavailable and malformed fields rejected")
        await shutdown(0)
        return
    if stage == "optional-write" or stage == "optional-reload":
        var context := make_optional_test_context(true)
        var optional_actor: CharacterBody3D = context[0]
        var optional_level: Node3D = context[1]
        var optional_global: Node = context[2]
        if stage == "optional-write":
            optional_global.set("dsg_147", false)
            var error := SAVE_STATE.write(optional_actor, optional_level)
            if not error.is_empty():
                fail("Could not write optional Global DSG[147] test slot: " + error)
                return
            var saved := SAVE_STATE.read()
            if saved["status"] != "loaded" or saved["state"]["schema"] != 1 or saved["state"]["actors"][SAVE_STATE.GLOBAL]["brain"].get("dsg_147") != false:
                fail("Native schema-1 slot did not persist explicit false Global DSG[147]")
                return
            print("SAVE_OPTIONAL_FRESH_WRITE schema-1 native slot persisted explicit Global DSG[147]=false")
        else:
            var saved := SAVE_STATE.read()
            if saved["status"] != "loaded" or saved["state"]["schema"] != 1 or saved["state"]["actors"][SAVE_STATE.GLOBAL]["brain"].get("dsg_147") != false:
                fail("Fresh process did not read explicit false Global DSG[147]")
                return
            var error := SAVE_STATE.apply(optional_actor, optional_level, saved["state"])
            if not error.is_empty() or optional_global.get("dsg_147") != false:
                fail("Fresh process did not apply explicit false Global DSG[147]")
                return
            print("SAVE_OPTIONAL_FRESH_RELOAD fresh process restored explicit Global DSG[147]=false")
        await shutdown(0)
        return
    var valid_state: Dictionary = {}
    if stage == "corrupt":
        var prior := SAVE_STATE.read()
        if prior["status"] != "loaded":
            fail("Valid slot missing before rejection checks")
            return
        valid_state = prior["state"]
        var legacy_global: Dictionary = valid_state.duplicate(true)
        if legacy_global["actors"][SAVE_STATE.GLOBAL]["brain"].has("dsg_147"):
            legacy_global["actors"][SAVE_STATE.GLOBAL]["brain"].erase("dsg_147")
        if not SAVE_STATE.validate(legacy_global).is_empty():
            fail("Schema-1 slot without optional Global DSG[147] was rejected")
            return
        var malformed_optional_global: Dictionary = valid_state.duplicate(true)
        malformed_optional_global["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_147"] = 1
        if SAVE_STATE.validate(malformed_optional_global).is_empty():
            fail("Non-Boolean optional Global DSG[147] was accepted")
            return
        var corrupt := FileAccess.open(SAVE_STATE.path(), FileAccess.WRITE)
        if corrupt == null:
            fail("Could not stage corrupt save")
            return
        corrupt.store_string("{broken")
        corrupt.close()
    var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
    root.add_child(menu)
    current_scene = menu
    var load_button := menu.find_child("LoadGame",true,false) as Button
    var new_button := menu.find_child("NewGame",true,false) as Button
    if load_button == null or new_button == null or new_button.disabled:
        fail("Game menu did not expose its Load Game operation")
        return
    if stage == "corrupt":
        if not load_button.disabled or not str(menu.find_child("Details", true, false).text).contains("Save"):
            fail("Corrupt slot did not fail visibly before session launch")
            return
        var unsupported := FileAccess.open(SAVE_STATE.path(), FileAccess.WRITE)
        if unsupported == null:
            fail("Could not stage unsupported save")
            return
        unsupported.store_string("{\"schema\":99,\"level\":\"manoir\"}")
        unsupported.close()
        if SAVE_STATE.read()["status"] != "error":
            fail("Unsupported save version was accepted")
            return
        var unsupported_menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
        root.add_child(unsupported_menu)
        var unsupported_button := unsupported_menu.find_child("LoadGame",true,false) as Button
        if unsupported_button == null or not unsupported_button.disabled:
            fail("Unsupported save did not block session launch")
            return
        var extra_state := valid_state.duplicate(true)
        extra_state["progression"] = {"quest": 1}
        var extra_file := FileAccess.open(SAVE_STATE.path(), FileAccess.WRITE)
        if extra_file == null:
            fail("Could not stage unsupported extra state")
            return
        extra_file.store_string(JSON.stringify(extra_state))
        extra_file.close()
        if SAVE_STATE.read()["status"] != "error":
            fail("Unknown top-level save state was silently accepted")
            return
        var extra_menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
        root.add_child(extra_menu)
        var extra_button := extra_menu.find_child("LoadGame",true,false) as Button
        if extra_button == null or not extra_button.disabled:
            fail("Unsupported extra state did not block session launch")
            return
        var extra_actor := valid_state.duplicate(true)
        extra_actor["actors"]["world/0/1/Unknown"] = {"health": 1}
        if SAVE_STATE.validate(extra_actor).is_empty():
            fail("Unknown actor state was silently accepted")
            return
        var extra_magick := valid_state.duplicate(true)
        extra_magick["actors"][SAVE_STATE.ACTOR]["magick"]["unmapped"] = 1
        if SAVE_STATE.validate(extra_magick).is_empty():
            fail("Unknown player magick state was silently accepted")
            return
        var noncanonical_quantity := valid_state.duplicate(true)
        noncanonical_quantity["actors"][SAVE_STATE.ACTOR]["inventory"]["quantities"]["01"] = 1
        if SAVE_STATE.validate(noncanonical_quantity).is_empty():
            fail("Noncanonical inventory quantity key was silently accepted")
            return
        var invalid_quantity := valid_state.duplicate(true)
        invalid_quantity["actors"][SAVE_STATE.ACTOR]["inventory"]["quantities"]["21"] = -32769
        if SAVE_STATE.validate(invalid_quantity).is_empty():
            fail("Inventory quantity below the signed source range was accepted")
            return
        var legacy_quantity := valid_state.duplicate(true)
        legacy_quantity["schema"] = 2
        var legacy_player: Dictionary = legacy_quantity["actors"][SAVE_STATE.ACTOR]
        var snapshot: Dictionary = legacy_player["inventory"]
        legacy_player.erase("armor")
        legacy_player["inventory"] = {"selected_weapon": snapshot["selected_weapon"], "quantities": snapshot["quantities"].duplicate()}
        if not SAVE_STATE.validate(legacy_quantity).is_empty():
            fail("Supported schema-2 sparse inventory shape was rejected")
            return
        legacy_player["inventory"]["quantities"]["21"] = -2
        if SAVE_STATE.validate(legacy_quantity).is_empty():
            fail("Legacy inventory quantity below its -1 sentinel was accepted")
            return
        print("SAVE_PROOF corrupt, unsupported version, extra state, noncanonical inventory keys, schema-3 out-of-range quantity, legacy quantity=-2 and malformed optional DSG[147] rejected; legacy optional omission accepted")
        await shutdown(0)
        return
    if load_button.disabled:
        fail("Load Game is disabled")
        return
    load_button.pressed.emit()
    var deadline := Time.get_ticks_msec() + 90000
    while (current_scene == null or current_scene == menu) and Time.get_ticks_msec() < deadline:
        await process_frame
    var game := current_scene
    if game == null or game == menu:
        fail("Load Game did not open the manor")
        return
    while game.get("ready_to_play") != true and Time.get_ticks_msec() < deadline:
        await process_frame
    if game.get("ready_to_play") != true:
        fail("Manor did not become playable: " + str(game.get_node("Hud/Panel/Margin/Rows/Status").text))
        return
    var actor := game.get("actor") as CharacterBody3D
    var level := game.get("level") as Node3D
    var source_brains: Dictionary = level.get_meta("source_actor_brains")
    var dummy_brain := level.get_node(source_brains[SAVE_STATE.DUMMY]) as Node
    var global_brain := level.get_node(source_brains[SAVE_STATE.GLOBAL]) as Node if source_brains.has(SAVE_STATE.GLOBAL) else null
    var gate := level.get_node_or_null("world/0/15/iMGrille_Manoir_Pont") as CharacterBody3D
    var gate_brain := gate.get_node_or_null("Brain") as Node if gate != null else null
    if dummy_brain == null or global_brain == null or gate_brain == null:
        fail("Exported manor is missing a saved actor Brain")
        return
    var native := SAVE_STATE.read()
    var original_import := SAVE_STATE.read_original_import()
    var restored: Dictionary = native["state"] if native["status"] == "loaded" else original_import["state"] if original_import["status"] == "loaded" else {}
    if restored.has("actors") and restored["actors"].has(SAVE_STATE.GLOBAL):
        var expected_gate_sequence: bool = restored["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_149"]
        if global_brain.get("dsg_149") != expected_gate_sequence:
            fail("Load Game did not restore Global DSG[149]")
            return
        var restored_global_brain: Dictionary = restored["actors"][SAVE_STATE.GLOBAL]["brain"]
        if restored_global_brain.has("dsg_147") and global_brain.get("dsg_147") != restored_global_brain["dsg_147"]:
            fail("Load Game did not restore optional Global DSG[147]")
            return
        if expected_gate_sequence and (int(gate_brain.get("current_normal")) != 3 or not str(gate_brain.get("unsupported_failure")).is_empty()):
            fail("Saved Global DSG[149] did not run the gate's source AI sequence")
            return
        if expected_gate_sequence and not gate_moved_once_from_source_pose(gate, stage):
            return
    var magick := actor.get("source_magick") as Resource
    var inventory := actor.get("source_inventory") as Resource
    if stage == "write":
        var checkpoint_error := SAVE_STATE.checkpoint_error(actor, level)
        while not checkpoint_error.is_empty() and Time.get_ticks_msec() < deadline:
            await physics_frame
            checkpoint_error = SAVE_STATE.checkpoint_error(actor, level)
        if not checkpoint_error.is_empty():
            fail("Manor did not reach a supported save checkpoint: " + checkpoint_error)
            return
        if int(magick.get("current_points")) != 30 or int(magick.call("remove_points", 7)) != 1:
            fail("Measured starting mana or native consumption differs")
            return
        if int(inventory.call("quantity", 0)) != 20:
            fail("Original SaveGame0 item-0 balance was not applied to the native inventory")
            return
        var quantities: Dictionary = inventory.get("quantities").duplicate()
        quantities[21] = 2
        quantities[0] = 0
        inventory.set("quantities", quantities)
        dummy_brain.set("dsg_8", true)
        key(KEY_F5, true)
        key(KEY_F5, false)
        var saved := SAVE_STATE.read()
        if saved["status"] != "loaded" or int(saved["state"]["actors"][SAVE_STATE.ACTOR]["magick"]["current_points"]) != 23:
            fail("F5 did not write current native mana")
            return
        if saved["state"]["world"][SAVE_STATE.DUMMY]["dsg_8"] != true:
            fail("F5 did not write the exported dummy's DSG[8]")
            return
        if not saved["state"]["actors"].has(SAVE_STATE.GLOBAL) or saved["state"]["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_149"] != true:
            fail("F5 did not write the saved manor gate sequence flag")
            return
        var saved_global_brain: Dictionary = saved["state"]["actors"][SAVE_STATE.GLOBAL]["brain"]
        var source_has_dsg_147 := SAVE_STATE._has_property(global_brain, "dsg_147")
        if saved_global_brain.has("dsg_147") != source_has_dsg_147:
            fail("F5 did not conditionally capture Global DSG[147] from the native Brain")
            return
        if source_has_dsg_147 and saved_global_brain["dsg_147"] != global_brain.get("dsg_147"):
            fail("F5 captured a different Global DSG[147] value")
            return
        if int(saved["state"]["actors"][SAVE_STATE.ACTOR]["inventory"]["quantities"]["61"]) != -1:
            fail("F5 did not preserve the measured unavailable-item sentinel")
            return
        var saved_quantities: Dictionary = saved["state"]["actors"][SAVE_STATE.ACTOR]["inventory"]["quantities"]
        if not saved_quantities.has("0") or int(saved_quantities["0"]) != 0:
            fail("F5 did not preserve an explicit item-0 balance of zero")
            return
        var original := FileAccess.get_file_as_string(SAVE_STATE.path())
        magick.set("current_points", 1000)
        key(KEY_F5, true)
        key(KEY_F5, false)
        if FileAccess.get_file_as_string(SAVE_STATE.path()) != original:
            fail("Invalid state overwrote a valid slot")
            return
        print("SAVE_PROOF wrote mana=23, inventory=2, item-0 balance=0, sentinel=-1 and optional Global DSG[147]; invalid write preserved slot")
    else:
        if int(magick.get("current_points")) != 23 or int(inventory.call("quantity", 21)) != 2:
            fail("Fresh process did not restore mana and inventory")
            return
        if dummy_brain.get("dsg_8") != true:
            fail("Fresh process did not restore the exported dummy's DSG[8]")
            return
        if global_brain.get("dsg_149") != true or int(gate_brain.get("current_normal")) != 3:
            fail("Fresh process did not restore and run the saved manor gate sequence")
            return
        var restored_global_brain: Dictionary = restored["actors"][SAVE_STATE.GLOBAL]["brain"]
        if restored_global_brain.has("dsg_147") and global_brain.get("dsg_147") != restored_global_brain["dsg_147"]:
            fail("Fresh process did not restore optional Global DSG[147]")
            return
        if int(inventory.call("quantity", 61)) != -1:
            fail("Fresh process did not restore the unavailable-item sentinel")
            return
        var restored_quantities: Dictionary = inventory.get("quantities")
        if not restored_quantities.has(0) or int(restored_quantities[0]) != 0 or int(inventory.call("quantity", 0)) != 0:
            fail("Fresh process did not restore the explicit item-0 zero over its default")
            return
        if int(magick.call("remove_points", 5)) != 1 or int(magick.get("current_points")) != 18:
            fail("Restored mana was not usable by the native component")
            return
        print("SAVE_PROOF fresh process restored mana=23, inventory=2, optional Global DSG[147] and explicit item-0 zero; native mana remained usable")
    await shutdown(0)

func test_optional_global_brain_field() -> String:
    var actor := MockActor.new()
    var level := Node3D.new()
    var global_brain := MockGlobalBrain.new()
    var dummy_brain := MockDummyBrain.new()
    global_brain.name = "MockGlobalBrain"
    dummy_brain.name = "MockDummyBrain"
    level.add_child(global_brain)
    level.add_child(dummy_brain)
    level.set_meta("source_actor_brains", {
        SAVE_STATE.GLOBAL: NodePath("MockGlobalBrain"),
        SAVE_STATE.DUMMY: NodePath("MockDummyBrain"),
    })
    root.add_child(level)
    root.add_child(actor)

    global_brain.dsg_147 = false
    var captured := SAVE_STATE.capture(actor, level)
    if captured.is_empty():
        return "Mock capture did not resolve native components: magick=%s inventory=%s dummy=%s global=%s dsg_149=%s" % [str(actor.get("source_magick") != null), str(actor.get("source_inventory") != null), str(SAVE_STATE._dummy_brain(level) != null), str(SAVE_STATE._source_brain(level, SAVE_STATE.GLOBAL) != null), str(SAVE_STATE._has_property(global_brain, "dsg_149"))]
    if not captured["actors"][SAVE_STATE.GLOBAL]["brain"].has("dsg_147") or captured["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_147"] != false:
        return "Capture did not preserve an exposed false Global DSG[147] value."
    if not SAVE_STATE.validate(captured).is_empty():
        return "Capture with optional Global DSG[147] did not validate."

    global_brain.dsg_147 = true
    var legacy := captured.duplicate(true)
    legacy["actors"][SAVE_STATE.GLOBAL]["brain"].erase("dsg_147")
    if not SAVE_STATE.validate(legacy).is_empty():
        return "Schema-1 Global Brain state without optional DSG[147] was rejected."
    var error := SAVE_STATE.apply(actor, level, captured)
    if not error.is_empty() or global_brain.dsg_147 != false:
        return "Apply did not restore the optional Global DSG[147] Boolean."
    global_brain.dsg_147 = true
    error = SAVE_STATE.apply(actor, level, legacy)
    if not error.is_empty() or global_brain.dsg_147 != true:
        return "Legacy schema-1 apply changed the scene-defined Global DSG[147] value."

    var invalid := captured.duplicate(true)
    invalid["actors"][SAVE_STATE.GLOBAL]["brain"]["dsg_147"] = 1
    if SAVE_STATE.validate(invalid).is_empty():
        return "Non-Boolean optional Global DSG[147] was accepted."

    var actor_without_optional := MockActor.new()
    var level_without_optional := Node3D.new()
    var global_without_optional := MockGlobalBrainWithoutDsg147.new()
    var dummy_without_optional := MockDummyBrain.new()
    global_without_optional.name = "MockGlobalBrainWithoutDsg147"
    dummy_without_optional.name = "MockDummyBrain"
    level_without_optional.add_child(global_without_optional)
    level_without_optional.add_child(dummy_without_optional)
    level_without_optional.set_meta("source_actor_brains", {
        SAVE_STATE.GLOBAL: NodePath("MockGlobalBrainWithoutDsg147"),
        SAVE_STATE.DUMMY: NodePath("MockDummyBrain"),
    })
    root.add_child(level_without_optional)
    root.add_child(actor_without_optional)
    var captured_without_optional := SAVE_STATE.capture(actor_without_optional, level_without_optional)
    if captured_without_optional["actors"][SAVE_STATE.GLOBAL]["brain"].has("dsg_147"):
        return "Capture invented optional Global DSG[147] when the Brain has no such property."
    var rejected := SAVE_STATE.apply(actor_without_optional, level_without_optional, captured)
    if rejected.is_empty() or actor_without_optional.source_magick.current_points != 12 or global_without_optional.dsg_149 != false:
        return "Apply with an unavailable saved DSG[147] did not fail before mutation."

    level.queue_free()
    actor.queue_free()
    level_without_optional.queue_free()
    actor_without_optional.queue_free()
    await process_frame
    return ""

func make_optional_test_context(initial_dsg_147: bool) -> Array:
    var actor := MockActor.new()
    var level := Node3D.new()
    var global_brain := MockGlobalBrain.new()
    var dummy_brain := MockDummyBrain.new()
    global_brain.name = "MockGlobalBrain"
    dummy_brain.name = "MockDummyBrain"
    global_brain.dsg_147 = initial_dsg_147
    level.add_child(global_brain)
    level.add_child(dummy_brain)
    level.set_meta("source_actor_brains", {
        SAVE_STATE.GLOBAL: NodePath("MockGlobalBrain"),
        SAVE_STATE.DUMMY: NodePath("MockDummyBrain"),
    })
    root.add_child(level)
    root.add_child(actor)
    return [actor, level, global_brain, dummy_brain]

func shutdown(exit_code: int) -> void:
    # Headless DummyRenderer can abort during exit while the generated manor's
    # scene resources still own RIDs. Tear down scene trees and let deferred
    # frees drain before asking the engine to quit.
    current_scene = null
    for child in root.get_children():
        child.queue_free()
    await process_frame
    await process_frame
    await process_frame
    quit(exit_code)

func key(code: Key, pressed: bool) -> void:
    var event := InputEventKey.new()
    event.physical_keycode = code
    event.pressed = pressed
    Input.parse_input_event(event)
    Input.flush_buffered_events()

func gate_moved_once_from_source_pose(gate: CharacterBody3D, stage: String) -> bool:
    var packed := load(LEVEL_SCENE_PATH) as PackedScene
    if packed == null:
        fail("Could not load generated manor to compare the saved gate pose")
        return false
    var baseline_level := packed.instantiate() as Node3D
    if baseline_level == null:
        fail("Generated manor did not instantiate for the gate pose comparison")
        return false
    for brain in baseline_level.find_children("Brain", "Node", true, false):
        if brain.get_property_list().any(func(property): return str(property["name"]) == "automatic"):
            brain.set("automatic", false)
    baseline_level.process_mode = Node.PROCESS_MODE_DISABLED
    root.add_child(baseline_level)
    var baseline_gate := baseline_level.get_node_or_null("world/0/15/iMGrille_Manoir_Pont") as CharacterBody3D
    if baseline_gate == null:
        baseline_level.free()
        fail("Generated manor has no baseline gate pose")
        return false
    var displacement := gate.global_position - baseline_gate.global_position
    baseline_level.free()
    if absf(displacement.x - 2.6) > 0.001 or absf(displacement.y) > 0.001 or absf(displacement.z) > 0.001:
        fail("Gate source AI displacement in %s was %s; expected exactly +2.6 X with no Y/Z shift" % [stage, displacement])
        return false
    print("SAVE_GATE_PROOF %s: source pose + %s = actual gate pose" % [stage, displacement])
    return true

func fail(message: String) -> void:
    push_error(message)
    shutdown.call_deferred(1)
