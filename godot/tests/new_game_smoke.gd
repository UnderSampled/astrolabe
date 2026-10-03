extends SceneTree
## Actual menu/session path. Run only with an isolated XDG_DATA_HOME; the
## slot below is test-owned. Full generated AI remains unmodified and unheld.
const SAVE_STATE = preload("res://game/save_state.gd")
var report: Dictionary = {"scope":"current-state New Game menu preview, not cinematic acceptance"}
var evidence_path: String
const SLOT_PROBE := "{\"schema\":999,\"level\":\"manoir\"}\n"

func _initialize() -> void:
    call_deferred("run")

func require(ok: bool, message: String) -> bool:
    if ok: return true
    report["failure"] = message
    write_report()
    push_error(message)
    quit(1)
    return false

func write_report() -> void:
    if evidence_path.is_empty(): return
    var file := FileAccess.open(evidence_path,FileAccess.WRITE)
    if file != null:
        file.store_string(JSON.stringify(report) + "\n")
        file.close()

func key(code: Key) -> void:
    for pressed in [true,false]:
        var event := InputEventKey.new()
        event.physical_keycode = code
        event.pressed = pressed
        Input.parse_input_event(event)
        Input.flush_buffered_events()

func snapshot(game: Node) -> Dictionary:
    var actor := game.get("actor") as CharacterBody3D
    var level := game.get("level") as Node3D
    var gate := level.get_node("world/0/15/iMGrille_Manoir_Pont") as CharacterBody3D
    var brain := actor.get_node("Brain")
    var automatic := 0
    var brains := level.find_children("Brain","Node",true,false)
    for source_brain in brains:
        if bool(source_brain.get("automatic")): automatic += 1
    return {"scene":game.scene_file_path,"new_game":game.get("new_game"),
        "loading_stage":game.get("loading_stage"),"position":[actor.global_position.x,-actor.global_position.z,actor.global_position.y],
        "normal":brain.get("current_normal"),"reflex":brain.get("current_reflex"),
        "brain_running":brain.get("running"),"opening_failures":game.get("opening_failures").duplicate(),
        "gate_height":gate.position.y,"gate_automatic":gate.get_node("Brain").get("automatic"),
        "automatic_brains":automatic,"brain_count":brains.size(),
        "status":game.get_node("Hud/Panel/Margin/Rows/Status").text,
        "save_disabled":game.get_node("Hud/Panel/Margin/Rows/Save").disabled,
        "camera":root.get_camera_3d().get_path()}

func wait_session(previous: Node) -> Node:
    var deadline := Time.get_ticks_msec() + 70000
    while (current_scene == null or current_scene == previous or not bool(current_scene.get("ready_to_play"))) and Time.get_ticks_msec() < deadline:
        await process_frame
    return current_scene if current_scene != null and current_scene != previous and bool(current_scene.get("ready_to_play")) else null

func run() -> void:
    var args := OS.get_cmdline_user_args()
    if not require(args.size() == 1,"requires evidence JSON path"): return
    evidence_path = args[0]
    if not require(not FileAccess.file_exists(SAVE_STATE.path()),"isolated slot must begin absent"): return
    var probe := FileAccess.open(SAVE_STATE.path(),FileAccess.WRITE)
    if not require(probe != null,"isolated save probe open failed"): return
    probe.store_string(SLOT_PROBE)
    probe.close()
    var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
    root.add_child(menu)
    current_scene = menu
    var load_button := menu.find_child("LoadGame",true,false) as Button
    var new_button := menu.find_child("NewGame",true,false) as Button
    if not require(load_button != null and new_button != null and load_button.disabled and not new_button.disabled,"bad slot should block Load Game while leaving New Game available"): return
    new_button.pressed.emit()
    var game: Node = await wait_session(menu)
    if not require(game != null and bool(game.get("new_game")),"New Game did not reach the actual session"): return
    for tick in range(6): await physics_frame
    await process_frame
    report["first"] = snapshot(game)
    var first: Dictionary = report["first"]
    if not require(first["scene"] == "res://game/new_game.tscn" and first["loading_stage"] == "opening_preview" and first["save_disabled"],"New Game mode or preview state differs"): return
    if not require(first["automatic_brains"] == first["brain_count"] and first["brain_count"] > 0,"opening held generated Brains"): return
    if not require(absf(float(first["gate_height"])) < 0.1 and first["gate_automatic"],"New Game used the saved-game gate workaround"): return
    if not require(Vector3(first["position"][0],first["position"][2],-first["position"][1]).distance_to(Vector3(-13.548369,32.932507,-158.19508)) < 1.0,"New Game restored the manor save instead of original starting pose"): return
    var first_actor_id := (game.get("actor") as Node).get_instance_id()
    var initial_resource: Variant = root.get_node("GameContent").get("level_scene")
    key(KEY_F5)
    if not require(FileAccess.get_file_as_string(SAVE_STATE.path()) == SLOT_PROBE,"preview save input overwrote existing slot"): return
    await create_timer(2.0).timeout
    report["after_two_seconds"] = snapshot(game)
    if DisplayServer.get_name() != "headless":
        await process_frame
        RenderingServer.force_draw()
        var picture := root.get_texture().get_image()
        var image_path := evidence_path.get_base_dir().path_join("new-game-preview.png")
        if not require(picture != null and not picture.is_empty() and picture.save_png(image_path) == OK,"preview image failed"): return
        report["image"] = image_path
    key(KEY_R)
    var restarted: Node = await wait_session(game)
    if not require(restarted != null and bool(restarted.get("new_game")) and restarted.scene_file_path == "res://game/new_game.tscn","R left the New Game route"): return
    if not require((restarted.get("actor") as Node).get_instance_id() != first_actor_id and root.get_node("GameContent").get("level_scene") == initial_resource,"restart reused actor state or discarded content cache"): return
    report["restart"] = snapshot(restarted)
    if not require(FileAccess.get_file_as_string(SAVE_STATE.path()) == SLOT_PROBE,"restart modified save slot"): return
    key(KEY_ESCAPE)
    var deadline := Time.get_ticks_msec() + 5000
    while (current_scene == null or not current_scene is Control) and Time.get_ticks_msec() < deadline: await process_frame
    if not require(current_scene is Control and current_scene.scene_file_path == "res://game/bootstrap.tscn","Esc did not return to the main menu"): return
    if not require(FileAccess.get_file_as_string(SAVE_STATE.path()) == SLOT_PROBE,"menu transition modified save slot"): return
    report["save_preserved"] = true
    report["status"] = "passed"
    write_report()
    print("NEW_GAME_PREVIEW_SMOKE_OK")
    current_scene = null
    for child in root.get_children(): child.queue_free()
    for frame in range(3): await process_frame
    quit(0)
