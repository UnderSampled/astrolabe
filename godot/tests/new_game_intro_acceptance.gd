extends SceneTree
## Actual bootstrap -> New Game, passive finite opening, then physical movement.
## This observer never changes actor state, callbacks, camera or source data.
const CRITICAL := ["fix:/persos/15", "level:/persos/world/0/0", "level:/persos/world/0/2", "level:/persos/world/0/3", "level:/persos/world/0/5", "level:/persos/world/0/28", "level:/persos/world/0/45"]
const DIALOGUES := ["gogood_texte1_manoir", "gogood_texte1a_manoir", "gogood_texte1b_manoir", "gogood_texte1c_manoir", "gogood_texte2_manoir", "gogood_texte2a_manoir", "hype_texte1_manoir", "hype_texte2_manoir"]
var evidence_path := ""
var report: Dictionary = {"outcome": "running", "scope": "actual root game bootstrap/New Game, unchanged passive intro, physical movement after handoff", "intro_input_events": 0, "snapshots": [], "dialogues": [], "camera_modes": [], "letterbox_on": false, "letterbox_off": false}
var game: Node
var screen: Node
var started_ms := 0
var last_signature := ""

func _initialize() -> void:
    call_deferred("run")

func write_report() -> void:
    if evidence_path.is_empty(): return
    var file := FileAccess.open(evidence_path, FileAccess.WRITE)
    if file != null: file.store_string(JSON.stringify(report, "\t") + "\n")

func finish(ok: bool, message: String) -> void:
    report["outcome"] = "passed" if ok else "failed"
    report["message"] = message
    report["elapsed_ms"] = Time.get_ticks_msec() - started_ms
    write_report()
    print("MANOR_INTRO_ACCEPTANCE ", JSON.stringify({"outcome": report["outcome"], "message": message, "elapsed_ms": report["elapsed_ms"]}))
    if not ok: push_error(message)
    quit(0 if ok else 1)

func actor_for(identity: String) -> Node:
    var level: Node = game.get("level")
    var paths: Dictionary = level.get_meta("source_actor_nodes", {})
    return level.get_node_or_null(paths[identity]) if paths.has(identity) else null

func snapshot() -> Dictionary:
    var actors: Dictionary = {}
    for identity in CRITICAL:
        var actor := actor_for(identity)
        var brain := actor.get_node_or_null("Brain") if actor != null else null
        actors[identity] = {"present": actor != null, "normal": int(brain.get("current_normal")) if brain != null else -1, "reflex": int(brain.get("current_reflex")) if brain != null else -1, "running": bool(brain.get("running")) if brain != null else false, "failure": str(brain.get("unsupported_failure")) if brain != null else "missing Brain", "position": [actor.global_position.x, actor.global_position.y, actor.global_position.z] if actor is Node3D else []}
    var camera: Camera3D = game.get("opening_camera")
    var target: Node = camera.get("targeted_actor") if camera != null else null
    var dialogue: Node = screen.get("dialogue")
    var item: Resource = dialogue.get("item") if dialogue != null else null
    var global_actor := actor_for("fix:/persos/15")
    var global_brain := global_actor.get_node_or_null("Brain") if global_actor != null else null
    var failures: Array[Dictionary] = []
    for brain in game.get("opening_brains"):
        if is_instance_valid(brain) and not str(brain.get("unsupported_failure")).is_empty():
            failures.append({"path": str(brain.get_path()), "model": str(brain.get("source_model_path")), "failure": str(brain.get("unsupported_failure"))})
    return {"ms": Time.get_ticks_msec() - started_ms, "actors": actors, "dialogue": str(item.get("identity")) if item != null else "", "widescreen": bool(screen.get("widescreen")), "camera_mode": int(camera.get("motion_mode")) if camera != null else -99, "camera_current": camera.current if camera != null else false, "camera_target": str(target.get_path()) if is_instance_valid(target) else "", "first_visit_complete": bool(global_brain.get("dsg_147")) if global_brain != null else false, "opening_finished": bool(game.get("opening_finished")), "other_failures": failures}

func observe() -> Dictionary:
    var state := snapshot()
    var identity: String = state["dialogue"]
    if not identity.is_empty() and not report["dialogues"].has(identity): report["dialogues"].append(identity)
    var mode: int = state["camera_mode"]
    if not report["camera_modes"].has(mode): report["camera_modes"].append(mode)
    if state["widescreen"]: report["letterbox_on"] = true
    elif report["letterbox_on"]: report["letterbox_off"] = true
    var behaviors: Array = []
    for entry in state["actors"].values(): behaviors.append([entry["normal"], entry["reflex"], entry["failure"]])
    var signature := JSON.stringify([behaviors, identity, mode, state["camera_target"], state["widescreen"], state["first_visit_complete"], state["opening_finished"]])
    if signature != last_signature:
        report["snapshots"].append(state)
        last_signature = signature
        write_report()
        print("INTRO_CHECKPOINT ", JSON.stringify({"ms": state["ms"], "dialogue": identity, "camera_mode": mode, "widescreen": state["widescreen"], "behaviors": behaviors}))
    return state

func capture(label: String) -> void:
    if DisplayServer.get_name() == "headless": return
    await process_frame
    RenderingServer.force_draw()
    var image := root.get_texture().get_image()
    if image != null and not image.is_empty():
        var path := evidence_path.get_base_dir().path_join(label + ".png")
        if image.save_png(path) == OK: report[label + "_image"] = path

func key_up(pressed: bool) -> void:
    var event := InputEventKey.new()
    event.physical_keycode = KEY_UP
    event.pressed = pressed
    Input.parse_input_event(event)
    Input.flush_buffered_events()

func run() -> void:
    var args := OS.get_cmdline_user_args()
    if args.size() != 1:
        finish(false, "requires one owned evidence JSON path")
        return
    evidence_path = args[0]
    started_ms = Time.get_ticks_msec()
    report["godot"] = Engine.get_version_info()
    report["project"] = ProjectSettings.globalize_path("res://")
    report["display"] = DisplayServer.get_name()
    root.size = Vector2i(1024, 768)
    root.content_scale_size = root.size
    var menu := (load("res://game/bootstrap.tscn") as PackedScene).instantiate()
    root.add_child(menu)
    current_scene = menu
    await process_frame
    var button := menu.find_child("NewGame", true, false) as Button
    if button == null or button.disabled:
        finish(false, "New Game is unavailable on actual bootstrap menu")
        return
    var position := button.get_global_rect().get_center()
    for pressed in [true, false]:
        var event := InputEventMouseButton.new()
        event.button_index = MOUSE_BUTTON_LEFT
        event.position = position
        event.global_position = position
        event.pressed = pressed
        Input.parse_input_event(event)
        Input.flush_buffered_events()
    var load_deadline := Time.get_ticks_msec() + 70000
    while (current_scene == null or current_scene == menu or not bool(current_scene.get("ready_to_play"))) and Time.get_ticks_msec() < load_deadline:
        await process_frame
    game = current_scene
    if game == null or game == menu or not bool(game.get("ready_to_play")) or not bool(game.get("new_game")):
        finish(false, "actual menu did not reach ready New Game within 70 seconds")
        return
    var level: Node = game.get("level")
    screen = level.get_node_or_null("AstrolabeUI")
    if screen == null:
        finish(false, "source presentation owner is missing")
        return
    var adopted: Node = screen.get("adopted_screen")
    if is_instance_valid(adopted): screen = adopted
    report["load_ms"] = Time.get_ticks_msec() - started_ms
    await capture("opening")
    var deadline := Time.get_ticks_msec() + 300000
    var mode6_captured := false
    while Time.get_ticks_msec() < deadline:
        var state := observe()
        for identity in CRITICAL:
            var failure: String = state["actors"][identity]["failure"]
            if not failure.is_empty():
                report["blocked_state"] = state
                await capture("blocked")
                finish(false, identity + ": " + failure)
                return
        if state["camera_mode"] == 6 and not mode6_captured:
            mode6_captured = true
            await capture("translation_shot")
        if state["opening_finished"]: break
        await physics_frame
    var final := observe()
    report["handoff"] = final
    if not final["opening_finished"]:
        await capture("timed_out")
        finish(false, "passive authored intro did not hand control to Hype within 300 seconds")
        return
    var missing: Array[String] = []
    for identity in DIALOGUES:
        if not report["dialogues"].has(identity): missing.append(identity)
    report["missing_dialogues"] = missing
    if not missing.is_empty() or not report["letterbox_on"] or not report["letterbox_off"] or not report["camera_modes"].has(3) or not report["camera_modes"].has(6) or not final["camera_current"]:
        finish(false, "handoff reached but required dialogue, letterbox or camera stages were not observed")
        return
    await capture("handoff")
    var actor: CharacterBody3D = game.get("actor")
    var initial := actor.global_position
    key_up(true)
    await create_timer(0.7).timeout
    key_up(false)
    await physics_frame
    var displacement := actor.global_position - initial
    report["physical_movement"] = {"key": "ArrowUp", "held_ms": 700, "displacement": [displacement.x, displacement.y, displacement.z], "distance": displacement.length(), "action": int(actor.get("current_state"))}
    if Vector2(displacement.x, displacement.z).length() <= 0.05:
        finish(false, "Hype did not move horizontally after source handoff and physical ArrowUp")
        return
    finish(true, "full passive intro, native camera/letterbox/dialogue handoff and physical movement observed")
