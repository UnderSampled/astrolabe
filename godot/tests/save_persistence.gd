extends SceneTree
## Run in separate Godot processes with one isolated user-data directory.
const SAVE_STATE = preload("res://game/save_state.gd")

func _initialize() -> void:
    call_deferred("run")

func run() -> void:
    var args := OS.get_cmdline_user_args()
    if args.size() != 1 or not ["write", "reload", "corrupt"].has(args[0]):
        fail("Expected write, reload, or corrupt stage")
        return
    var stage: String = args[0]
    var valid_state: Dictionary = {}
    if stage == "corrupt":
        var prior := SAVE_STATE.read()
        if prior["status"] != "loaded":
            fail("Valid slot missing before rejection checks")
            return
        valid_state = prior["state"]
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
        if not load_button.disabled or not str(menu.find_children("*", "Label", true, false)[1].text).contains("Save"):
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
        print("SAVE_PROOF corrupt, unsupported version, extra state, and noncanonical inventory keys rejected")
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
    while not bool(game.get("ready_to_play")) and Time.get_ticks_msec() < deadline:
        await process_frame
    if not bool(game.get("ready_to_play")):
        fail("Manor did not become playable: " + str(game.get_node("Hud/Panel/Margin/Rows/Status").text))
        return
    var actor := game.get("actor") as CharacterBody3D
    var level := game.get("level") as Node3D
    var source_brains: Dictionary = level.get_meta("source_actor_brains")
    var dummy_brain := level.get_node(source_brains[SAVE_STATE.DUMMY]) as Node
    if dummy_brain == null:
        fail("Exported manor dummy Brain is missing")
        return
    var magick := actor.get("source_magick") as Resource
    var inventory := actor.get("source_inventory") as Resource
    if stage == "write":
        if int(magick.get("current_points")) != 30 or int(magick.call("remove_points", 7)) != 1:
            fail("Measured starting mana or native consumption differs")
            return
        var quantities: Dictionary = inventory.get("quantities").duplicate()
        quantities[21] = 2
        inventory.set("quantities", quantities)
        dummy_brain.set("dsg_8", true)
        key(KEY_F5, true)
        key(KEY_F5, false)
        var saved := SAVE_STATE.read()
        if saved["status"] != "loaded" or int(saved["state"]["actors"][SAVE_STATE.ACTOR]["magick"]["current_points"]) != 23:
            fail("F5 did not write current native mana")
            return
        if not bool(saved["state"]["world"][SAVE_STATE.DUMMY]["dsg_8"]):
            fail("F5 did not write the exported dummy's DSG[8]")
            return
        var original := FileAccess.get_file_as_string(SAVE_STATE.path())
        magick.set("current_points", 1000)
        key(KEY_F5, true)
        key(KEY_F5, false)
        if FileAccess.get_file_as_string(SAVE_STATE.path()) != original:
            fail("Invalid state overwrote a valid slot")
            return
        print("SAVE_PROOF wrote mana=23, inventory=2, and dummy dsg_8=true; invalid write preserved slot")
    else:
        if int(magick.get("current_points")) != 23 or int(inventory.call("quantity", 21)) != 2:
            fail("Fresh process did not restore mana and inventory")
            return
        if not bool(dummy_brain.get("dsg_8")):
            fail("Fresh process did not restore the exported dummy's DSG[8]")
            return
        if int(magick.call("remove_points", 5)) != 1 or int(magick.get("current_points")) != 18:
            fail("Restored mana was not usable by the native component")
            return
        print("SAVE_PROOF fresh process restored mana=23, inventory=2, and dummy dsg_8=true; native mana remained usable")
    await shutdown(0)

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

func fail(message: String) -> void:
    push_error(message)
    quit(1)
