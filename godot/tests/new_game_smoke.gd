extends SceneTree
## Actual menu/session path. Run only with an isolated XDG_DATA_HOME; the
## slot below is test-owned. Full generated AI remains unmodified and unheld.
const SAVE_STATE = preload("res://game/save_state.gd")
const ORIGINAL_IMPORT_PATH := "res://game/original_save_import.json"
const NEW_GAME_SCENE_PATH := "res://game/new_game.tscn"
const FLOAT32_EPSILON := 0.00000011920928955078125
const FLOAT32_ROUNDTRIP_EPSILONS := 2.0
var report: Dictionary = {"scope":"actual New Game and Load Game routes, original import/native precedence, normal opening walk/contact checkpoint", "intro_acceptance":"pending"}
var evidence_path: String
var gogoud_actions: Array[int] = []
var gogoud_frames: Array[Dictionary] = []
var transition_events: Array[Dictionary] = []
var session_stage_events: Array[Dictionary] = []
var original_import_backup := ""
var original_import_changed := false
var new_game_scene_source_abs := ""
var new_game_scene_backup_abs := ""
const SLOT_PROBE := "{\"schema\":999,\"level\":\"manoir\"}\n"

func _float32_roundtrip_tolerance(expected: Vector3) -> Vector3:
    var scale := Vector3(maxf(1.0, absf(expected.x)), maxf(1.0, absf(expected.y)), maxf(1.0, absf(expected.z)))
    return scale * FLOAT32_EPSILON * FLOAT32_ROUNDTRIP_EPSILONS

func _matches_float32_roundtrip(expected: Vector3, observed: Vector3) -> bool:
    var tolerance := _float32_roundtrip_tolerance(expected)
    var delta := observed - expected
    return absf(delta.x) <= tolerance.x and absf(delta.y) <= tolerance.y and absf(delta.z) <= tolerance.z

func _check_float32_tolerance_contract() -> bool:
    var source_position := Vector3(-74.381462097168, 4.56865549087524, -28.5058479309082)
    var observed_roundtrip := Vector3(-74.3814544677734, 4.56865501403809, -28.5058441162109)
    var measured_roundtrip_accepted := _matches_float32_roundtrip(source_position, observed_roundtrip)
    var unit_position := Vector3.ONE
    var boundary_tolerance := _float32_roundtrip_tolerance(unit_position).x
    var at_boundary := Vector3(unit_position.x + boundary_tolerance, unit_position.y, unit_position.z)
    var beyond_boundary := Vector3(unit_position.x + boundary_tolerance * 1.5, unit_position.y, unit_position.z)
    var boundary_accepted := _matches_float32_roundtrip(unit_position, at_boundary)
    var beyond_boundary_rejected := not _matches_float32_roundtrip(unit_position, beyond_boundary)
    var wrong_pose := Vector3(source_position.x + 0.001, source_position.y, source_position.z)
    var wrong_pose_rejected := not _matches_float32_roundtrip(source_position, wrong_pose)
    report["float32_position_tolerance_contract"] = {"rule":"two float32 machine epsilons scaled by max(1, abs(expected component)) per axis",
        "measured_roundtrip_delta":[observed_roundtrip.x - source_position.x, observed_roundtrip.y - source_position.y, observed_roundtrip.z - source_position.z],
        "boundary_two_epsilons_accepted":boundary_accepted,"three_epsilons_rejected":beyond_boundary_rejected,
        "one_millimeter_wrong_pose_rejected":wrong_pose_rejected}
    if not require(measured_roundtrip_accepted, "float32 round-trip comparison rejected the measured source-position representation"): return false
    if not require(boundary_accepted, "float32 comparison rejected its two-epsilon boundary"): return false
    if not require(beyond_boundary_rejected, "float32 comparison accepted a value beyond its precision boundary"): return false
    if not require(wrong_pose_rejected, "float32 comparison accepted a 1 mm wrong-pose control"): return false
    return true

func _initialize() -> void:
    node_added.connect(_watch_session_node)
    call_deferred("run")

func require(ok: bool, message: String) -> bool:
    if ok: return true
    report["failure"] = message
    write_report()
    push_error(message)
    shutdown.call_deferred(1)
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

func click_gui(control: Control, repeat_immediately: bool = false) -> void:
    await process_frame
    var position := control.get_global_rect().get_center()
    var motion := InputEventMouseMotion.new()
    motion.position = position
    motion.global_position = position
    Input.parse_input_event(motion)
    Input.flush_buffered_events()
    var click_count := 2 if repeat_immediately else 1
    for click_index in range(click_count):
        for pressed in [true, false]:
            var event := InputEventMouseButton.new()
            event.button_index = MOUSE_BUTTON_LEFT
            event.position = position
            event.global_position = position
            event.pressed = pressed
            Input.parse_input_event(event)
            Input.flush_buffered_events()
    await process_frame

func _loading_screen() -> Node:
    return root.get_node_or_null("LevelTransition/LoadingScreen")

func _watch_session_node(node: Node) -> void:
    if node.has_signal("loading_stage_changed"):
        var callback := Callable(self, "_record_session_stage").bind(node)
        if not node.is_connected("loading_stage_changed", callback):
            node.connect("loading_stage_changed", callback)

func _record_session_stage(stage_name: String, elapsed_ms: float, session: Node) -> void:
    var presenter := _loading_screen()
    var phase := presenter.get_node_or_null("Viewport/Center/Content/Phase") as Label if presenter != null else null
    var progress := presenter.get_node_or_null("Viewport/Center/Content/Progress") as ProgressBar if presenter != null else null
    var actor := session.get("actor") as CharacterBody3D
    session_stage_events.append({"session_scene":session.scene_file_path,"new_game":bool(session.get("new_game")),
        "stage":stage_name,"elapsed_ms":elapsed_ms,"observed_at_ms":Time.get_ticks_msec(),"presenter_visible":bool(presenter.get("visible")) if presenter != null else false,
        "phase":phase.text if phase != null else "","progress_visible":progress.visible if progress != null else false,
        "progress_value":progress.value if progress != null else -1.0,
        "actor_position":[actor.global_position.x, actor.global_position.y, actor.global_position.z] if actor != null else [],
        "actor_physics_enabled":actor.is_physics_processing() if actor != null else false})

func _record_transition_started(from_level: String, to_level: String, entry: String) -> void:
    var presenter := _loading_screen()
    var progress := presenter.get_node_or_null("Viewport/Center/Content/Progress") as ProgressBar if presenter != null else null
    transition_events.append({"event":"started","from":from_level,"level":to_level,"entry":entry,"observed_at_ms":Time.get_ticks_msec(),
        "presenter_visible":bool(presenter.get("visible")) if presenter != null else false,
        "progress_visible":progress.visible if progress != null else false,
        "progress_value":progress.value if progress != null else -1.0})

func _record_transition_progress(level: String, fraction: float, phase: String) -> void:
    var presenter := _loading_screen()
    var progress := presenter.get_node_or_null("Viewport/Center/Content/Progress") as ProgressBar if presenter != null else null
    transition_events.append({"event":"progress","level":level,"fraction":fraction,"phase":phase,
        "observed_at_ms":Time.get_ticks_msec(),"progress_visible":progress.visible if progress != null else false,
        "progress_value":progress.value if progress != null else -1.0})

func _record_transition_completed(level: String, entry: String) -> void:
    var session := current_scene
    var actor := session.get("actor") as CharacterBody3D if session != null else null
    var magick: Resource = actor.get("source_magick") if actor != null else null
    transition_events.append({"event":"completed","level":level,"entry":entry,"observed_at_ms":Time.get_ticks_msec(),
        "session_scene":session.scene_file_path if session != null else "",
        "ready_to_play":bool(session.get("ready_to_play")) if session != null else false,
        "loading":bool(session.get("loading")) if session != null else true,
        "loading_stage":str(session.get("loading_stage")) if session != null else "",
        "mana":int(magick.get("current_points")) if magick != null else -1,
        "position":[actor.global_position.x, actor.global_position.y, actor.global_position.z] if actor != null else []})

func _record_transition_failed(level: String, message: String) -> void:
    transition_events.append({"event":"failed","level":level,"message":message,"observed_at_ms":Time.get_ticks_msec()})

func _connect_transition_observers() -> bool:
    var transitions := root.get_node_or_null("LevelTransition")
    if transitions == null:
        return false
    for signal_name in ["transition_started", "transition_progress", "transition_completed", "transition_failed"]:
        if not transitions.has_signal(signal_name):
            return false
    transitions.connect("transition_started", _record_transition_started)
    transitions.connect("transition_progress", _record_transition_progress)
    transitions.connect("transition_completed", _record_transition_completed)
    transitions.connect("transition_failed", _record_transition_failed)
    return true

func _restore_test_fixtures() -> void:
    if original_import_changed:
        var import_file := FileAccess.open(ORIGINAL_IMPORT_PATH, FileAccess.WRITE)
        if import_file != null:
            import_file.store_string(original_import_backup)
            import_file.flush()
            var write_error := import_file.get_error()
            import_file.close()
            if write_error == OK:
                original_import_changed = false
            else:
                push_error("Could not restore the staged original-save import fixture.")
        else:
            push_error("Could not open the staged original-save import fixture for restoration.")
    if not new_game_scene_backup_abs.is_empty() and FileAccess.file_exists(new_game_scene_backup_abs) and not FileAccess.file_exists(new_game_scene_source_abs):
        var error := DirAccess.rename_absolute(new_game_scene_backup_abs, new_game_scene_source_abs)
        if error != OK:
            push_error("Could not restore the New Game route scene after smoke validation.")
        else:
            new_game_scene_backup_abs = ""

func _stash_new_game_route_scene() -> bool:
    new_game_scene_source_abs = ProjectSettings.globalize_path(NEW_GAME_SCENE_PATH)
    new_game_scene_backup_abs = new_game_scene_source_abs + ".acceptance-hold"
    if not FileAccess.file_exists(new_game_scene_source_abs) or FileAccess.file_exists(new_game_scene_backup_abs):
        new_game_scene_source_abs = ""
        new_game_scene_backup_abs = ""
        return false
    if DirAccess.rename_absolute(new_game_scene_source_abs, new_game_scene_backup_abs) != OK:
        new_game_scene_source_abs = ""
        new_game_scene_backup_abs = ""
        return false
    return true

func _remove_test_slot() -> bool:
    if not FileAccess.file_exists(SAVE_STATE.path()):
        return true
    return DirAccess.remove_absolute(ProjectSettings.globalize_path(SAVE_STATE.path())) == OK

func _find_transition_event(event_name: String, start_index: int) -> Dictionary:
    for index in range(start_index, transition_events.size()):
        if transition_events[index].get("event") == event_name:
            return transition_events[index]
    return {}

func _wait_transition_event(event_name: String, start_index: int, timeout_ms: int = 15000) -> Dictionary:
    var deadline := Time.get_ticks_msec() + timeout_ms
    var found := _find_transition_event(event_name, start_index)
    while found.is_empty() and Time.get_ticks_msec() < deadline:
        await process_frame
        found = _find_transition_event(event_name, start_index)
    return found

func _wait_transition_idle(timeout_ms: int = 5000) -> bool:
    var transitions := root.get_node_or_null("LevelTransition")
    if transitions == null:
        return false
    var deadline := Time.get_ticks_msec() + timeout_ms
    while bool(transitions.call("is_transitioning")) and Time.get_ticks_msec() < deadline:
        await process_frame
    return not bool(transitions.call("is_transitioning"))

func _check_route_lifecycle(route_name: String, start_index: int, expected_mana: int, expected_restored_position: Variant = null) -> bool:
    var started := _find_transition_event("started", start_index)
    var completed := await _wait_transition_event("completed", start_index)
    var started_count := 0
    var completed_count := 0
    for index in range(start_index, transition_events.size()):
        var event: Dictionary = transition_events[index]
        if event.get("event") == "started" and event.get("observed_at_ms", 0) >= started.get("observed_at_ms", 0):
            started_count += 1
        elif event.get("event") == "completed" and event.get("observed_at_ms", 0) >= started.get("observed_at_ms", 0):
            completed_count += 1
    if not require(not started.is_empty() and started.get("presenter_visible", false) and started.get("progress_visible", false), route_name + " did not show the loading presenter at route start"): return false
    if not require(not completed.is_empty(), route_name + " did not emit transition_completed"): return false
    if not require(started_count == 1 and completed_count == 1, route_name + " dispatched more than one transition for a repeated menu activation"): return false
    var expected_ready_stage := "opening" if route_name == "new_game" else "playable"
    var completion_ready: bool = bool(completed.get("ready_to_play", false)) and not bool(completed.get("loading", true)) and str(completed.get("loading_stage", "")) == expected_ready_stage
    report[route_name + "_completion_boundary"] = {"ready_to_play":completed.get("ready_to_play", false),
        "loading":completed.get("loading", true),"loading_stage":completed.get("loading_stage", ""),
        "expected_stage":expected_ready_stage,"mana":completed.get("mana", -1)}
    if not require(completion_ready, "%s completion boundary is not initialized: expected stage %s, observed %s" % [route_name, expected_ready_stage, completed.get("loading_stage", "<missing>")]): return false
    if not require(int(completed.get("mana", -1)) == expected_mana, route_name + " completed before the expected state was restored"): return false
    var restoration: Dictionary = {}
    for event in session_stage_events:
        if event.get("observed_at_ms", 0) >= started.get("observed_at_ms", 0) and event.get("stage") == "state_restoration":
            restoration = event
            break
    var expected_phase := "Initializing new game..." if route_name == "new_game" else "Restoring saved game..."
    if not require(not restoration.is_empty() and restoration.get("presenter_visible", false) and restoration.get("phase") == expected_phase, route_name + " did not keep the correct presenter phase visible during state restoration"): return false
    if not require(not restoration.get("progress_visible", true), route_name + " showed the resource fraction as total restoration progress"): return false
    var restored_pose: Dictionary = {}
    if expected_restored_position is Vector3:
        for event in session_stage_events:
            if event.get("observed_at_ms", 0) >= started.get("observed_at_ms", 0) and event.get("stage") == "restored":
                restored_pose = event
                break
        var restored_values: Array = restored_pose.get("actor_position", [])
        var restored_position := Vector3(float(restored_values[0]), float(restored_values[1]), float(restored_values[2])) if restored_values.size() == 3 else Vector3.ZERO
        var float32_restore: bool = restored_values.size() == 3 and _matches_float32_roundtrip(expected_restored_position, restored_position)
        var restore_delta: Vector3 = restored_position - expected_restored_position
        var restore_tolerance: Vector3 = _float32_roundtrip_tolerance(expected_restored_position)
        report[route_name + "_restore_pose_boundary"] = {"expected":[expected_restored_position.x,expected_restored_position.y,expected_restored_position.z],
            "observed":[restored_position.x,restored_position.y,restored_position.z] if restored_values.size() == 3 else [],
            "delta":[restore_delta.x,restore_delta.y,restore_delta.z],
            "tolerance_m":[restore_tolerance.x,restore_tolerance.y,restore_tolerance.z],
            "actor_physics_enabled":restored_pose.get("actor_physics_enabled", true),"within_float32_roundtrip":float32_restore,
            "reason":"Demo emits restored synchronously after SAVE_STATE.apply and before enabling actor physics; only two float32 machine epsilons at coordinate scale are allowed.",
            "accepted":not restored_pose.is_empty() and not bool(restored_pose.get("actor_physics_enabled", true)) and float32_restore}
        if not require(not restored_pose.is_empty() and not bool(restored_pose.get("actor_physics_enabled", true)) and float32_restore,
            route_name + " did not restore the saved position within float32 transform precision before actor physics resumed"): return false
    for event in transition_events:
        if event.get("event") == "progress" and event.get("observed_at_ms", 0) >= started.get("observed_at_ms", 0) and event.get("observed_at_ms", 0) <= completed.get("observed_at_ms", 0):
            if not require(event.get("phase") == "resource_loading" and float(event.get("fraction", -1.0)) >= 0.0 and float(event.get("fraction", 2.0)) <= 1.0, route_name + " reported a non-resource fraction as load progress"): return false
    report[route_name + "_lifecycle"] = {"started":started,"restoration":restoration,"restored_pose":restored_pose,"completed":completed}
    return true

func wait_menu(previous: Node, timeout_ms: int = 15000) -> Node:
    var deadline := Time.get_ticks_msec() + timeout_ms
    while (current_scene == null or current_scene == previous or current_scene.scene_file_path != "res://game/bootstrap.tscn") and Time.get_ticks_msec() < deadline:
        await process_frame
    return current_scene if current_scene != null and current_scene != previous and current_scene.scene_file_path == "res://game/bootstrap.tscn" else null

func return_to_menu(session: Node) -> Node:
    var transition_idle: bool = await _wait_transition_idle()
    if not transition_idle:
        return null
    if session == null or not is_instance_valid(session) or not session.has_method("leave_session"):
        return null
    session.call("leave_session", "res://game/bootstrap.tscn")
    return await wait_menu(session)

func snapshot(game: Node) -> Dictionary:
    var actor := game.get("actor") as CharacterBody3D
    var level := game.get("level") as Node3D
    var gate := level.get_node("world/0/15/iMGrille_Manoir_Pont") as CharacterBody3D
    var brain := actor.get_node("Brain")
    var source_ui := level.get_node_or_null("AstrolabeUI")
    var screen := source_ui
    if source_ui != null:
        var adopted := source_ui.get("adopted_screen") as Node
        if adopted != null: screen = adopted
    var letterbox := screen.get_node_or_null("Canvas/Letterbox") as Control if screen != null else null
    var source_hud := screen.get_node_or_null("Canvas/Hud") as Control if screen != null else null
    var active_camera := root.get_camera_3d()
    var source_camera := game.get("opening_camera") as Camera3D
    var source_camera_brain := source_camera.get_parent().get_node_or_null("Brain") if source_camera != null else null
    var camera_state := {
        "source_path": str(source_camera.get_path()) if source_camera != null else "",
        "source_current": source_camera.current if source_camera != null else false,
        "source_mode": source_camera.motion_mode if source_camera != null else -1,
        "source_target": str(source_camera.get("targeted_actor").get_path()) if source_camera != null and is_instance_valid(source_camera.get("targeted_actor")) else "",
        "source_guard": bool(source_camera.call("source_camera_collision_guard_value")) if source_camera != null and source_camera.has_method("source_camera_collision_guard_value") else false,
        "source_brain_running": bool(source_camera_brain.get("running")) if source_camera_brain != null else false,
        "source_brain_failure": str(source_camera_brain.get("unsupported_failure")) if source_camera_brain != null else "missing SourceCamera Brain",
        "active_path": str(active_camera.get_path()) if active_camera != null else ""
    }
    var screen_state := {
        "present": screen != null,
        "widescreen": bool(screen.get("widescreen")) if screen != null else false,
        "letterbox_visible": letterbox.visible if letterbox != null else false,
        "hud_suppressed": bool(source_hud.get("suppressed")) if source_hud != null else false
    }
    var automatic := 0
    var brains := level.find_children("Brain","Node",true,false)
    var actor_brain_count := 0
    var global_data: Array[Dictionary] = []
    for source_brain in brains:
        # Exported nonvisual Globals keep designer values with automatic=false
        # (scene.rs). They are not rooted actor behaviors held by this session.
        if source_brain.get_parent().get_parent() == level.get_node("Globals"):
            global_data.append({"path":str(source_brain.get_path()),"model":source_brain.get("source_model_path"),"automatic":source_brain.get("automatic")})
            continue
        actor_brain_count += 1
        if bool(source_brain.get("automatic")): automatic += 1
    return {"scene":game.scene_file_path,"new_game":game.get("new_game"),
        "loading_stage":game.get("loading_stage"),"position":[actor.global_position.x,-actor.global_position.z,actor.global_position.y],
        "normal":brain.get("current_normal"),"reflex":brain.get("current_reflex"),
        "brain_running":brain.get("running"),"opening_failures":game.get("opening_failures").duplicate(),
        "gate_height":gate.position.y,"gate_automatic":gate.get_node("Brain").get("automatic"),
        "automatic_brains":automatic,"brain_count":actor_brain_count,"total_brains":brains.size(),"global_data_brains":global_data,
        "status":game.get_node("Hud/Panel/Margin/Rows/Status").text,
        "save_disabled":game.get_node("Hud/Panel/Margin/Rows/Save").disabled,
        "camera":str(active_camera.get_path()) if active_camera != null else "",
        "camera_state":camera_state,"screen_state":screen_state}

func check_camera_screen_state(name: String, state: Dictionary) -> bool:
    var camera: Dictionary = state.get("camera_state", {})
    var screen: Dictionary = state.get("screen_state", {})
    if not require(not str(state.get("camera", "")).is_empty(), name + " has no active camera"): return false
    if not require(screen.get("present", false), name + " has no maintained source screen"): return false
    var widescreen := bool(screen.get("widescreen", false))
    if not require(widescreen == bool(screen.get("letterbox_visible", false)) and widescreen == bool(screen.get("hud_suppressed", false)), name + " letterbox and HUD suppression do not match source widescreen state"): return false
    report[name + "_camera_screen"] = {"camera":camera,"screen":screen}
    return true

func wait_session(previous: Node) -> Node:
    var deadline := Time.get_ticks_msec() + 70000
    while (current_scene == null or current_scene == previous or not bool(current_scene.get("ready_to_play"))) and Time.get_ticks_msec() < deadline:
        await process_frame
    return current_scene if current_scene != null and current_scene != previous and bool(current_scene.get("ready_to_play")) else null

func gogoud_snapshot(actor: CharacterBody3D) -> Dictionary:
    var source_brain := actor.get_node("Brain")
    var contacts: Array[Dictionary] = []
    for index in actor.get_slide_collision_count():
        var hit := actor.get_slide_collision(index)
        var collider := hit.get_collider() as CollisionObject3D
        var shape: CollisionShape3D
        if collider != null:
            var owner_id := collider.shape_find_owner(hit.get_collider_shape_index())
            shape = collider.shape_owner_get_owner(owner_id) as CollisionShape3D
        var source_node: Node = collider
        while source_node != null and not source_node.has_meta("source_node_id"): source_node = source_node.get_parent()
        contacts.append({"source_node":source_node.get_meta("source_node_id") if source_node != null else "",
            "source_material":shape.get_meta("source_material", "") if shape != null else ""})
    return {"clock_ms":source_brain.get("clock_milliseconds"),"action":actor.current_state,
        "normal":source_brain.get("current_normal"),"failure":source_brain.get("unsupported_failure"),
        "position":[actor.global_position.x,-actor.global_position.z,actor.global_position.y],
        "heading":actor.global_rotation.y,"on_floor":actor.is_on_floor(),"route_active":actor.source_route_active,
        "route_point":actor.source_route_points[6],"contacts":contacts}

func capture(name: String) -> String:
    if DisplayServer.get_name() == "headless": return ""
    await process_frame
    RenderingServer.force_draw()
    var picture := root.get_texture().get_image()
    var path := evidence_path.get_base_dir().path_join(name + ".png")
    if not require(picture != null and not picture.is_empty() and picture.save_png(path) == OK,"opening image failed"): return ""
    return path

func run() -> void:
    var args := OS.get_cmdline_user_args()
    if not require(args.size() == 1,"requires evidence JSON path"): return
    evidence_path = args[0]
    if not _check_float32_tolerance_contract(): return
    if not require(not FileAccess.file_exists(SAVE_STATE.path()),"isolated slot must begin absent"): return
    original_import_backup = FileAccess.get_file_as_string(ORIGINAL_IMPORT_PATH) if FileAccess.file_exists(ORIGINAL_IMPORT_PATH) else ""
    var imported_fixture: Dictionary = SAVE_STATE.read_original_import()
    if not require(not original_import_backup.is_empty() and imported_fixture.get("status") == "loaded", "the preserved original-save import fixture is unavailable or invalid"): return
    if not require(_connect_transition_observers(), "persistent LevelTransition service is missing its lifecycle signals"): return
    var import_document: Variant = JSON.parse_string(original_import_backup)
    report["original_import_fixture"] = {"present":true,"valid":true,"profile":import_document.get("profile", "") if import_document is Dictionary else ""}
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
    if not require(_stash_new_game_route_scene(), "could not stage the New Game route failure check in the isolated project"): return
    await click_gui(new_button, true)
    if not require(current_scene == menu and not new_button.disabled and not bool(menu.get("opening_game")) and menu.get_node("Status").visible, "a failed New Game destination did not leave the menu usable"): return
    report["new_game_route_failure"] = {"menu_recovered":true,"button_reenabled":not new_button.disabled,"status":menu.get_node("Status/Details").text}
    _restore_test_fixtures()
    if not require(FileAccess.file_exists(NEW_GAME_SCENE_PATH), "New Game route scene was not restored after the failure check"): return
    var new_game_transition_start := transition_events.size()
    await click_gui(new_button, true)
    var game: Node = await wait_session(menu)
    if not require(game != null and bool(game.get("new_game")),"New Game did not reach the actual session"): return
    var new_game_actor := game.get("actor") as CharacterBody3D
    var new_game_magick: Resource = new_game_actor.get("source_magick") if new_game_actor != null else null
    if not require(new_game_magick != null, "New Game did not initialize native player state"): return
    var new_game_lifecycle_ok: bool = await _check_route_lifecycle("new_game", new_game_transition_start, int(new_game_magick.get("current_points")))
    if not new_game_lifecycle_ok: return
    for tick in range(6): await physics_frame
    await process_frame
    report["first"] = snapshot(game)
    var first: Dictionary = report["first"]
    if not check_camera_screen_state("first", first): return
    var hype := game.get("actor") as CharacterBody3D
    var particle_source := hype.get_node_or_null("ParticleSource")
    var selected: CPUParticles3D = particle_source.get("selected") if particle_source != null else null
    var hype_failure: String = hype.get_node("Brain").get("unsupported_failure")
    report["hype_particle_initialization"] = {"component_present":particle_source != null,
        "generator":selected.get("generator_uri") if selected != null else "",
        "initialized":selected.get("initialized") if selected != null else false,
        "capacity":selected.amount if selected != null else 0,
        "emitting":selected.emitting if selected != null else false,
        "failure":hype_failure}
    if not require(selected != null and selected.get("generator_uri") == "fix:/particles/generators/5-0-22092" and selected.amount == 200 and selected.get("initialized") and not selected.emitting,"normal New Game failed source Hype particle selection/disable initialization"): return
    if not require(not hype_failure.contains("operation 194") and not hype_failure.contains("could not select converted particle generator"),"Hype still stops at particle selection"): return
    report["hype_presentation_initialization"] = {"opacity":hype.get("source_opacity"),"transparent_display":hype.get("source_transparent_display"),"failure":hype_failure}
    if not require(hype.get("source_opacity") == 255 and hype.get("source_transparent_display") and not hype_failure.contains("operation 187") and not hype_failure.contains("operation 185") and not hype_failure.contains("native opacity materials") and not hype_failure.contains("native transparent display materials"),"Hype did not complete original opacity/display initialization"): return
    var life: Resource = hype.get("source_hit_points")
    var remembered_life: Variant = hype.get_node("Brain").get("dsg_128")
    report["hype_hit_point_initialization"] = {"current":life.get("current") if life != null else null,"remembered":remembered_life,"failure":hype_failure}
    # Fifty is independently read from the unchanged source StandardGame byte
    # by the source corpus test, rather than inferred from this runtime result.
    if not require(hype.call("source_hit_points_ready") and life.get("current") == 50 and remembered_life == 50 and not hype_failure.contains("operation 35 at node 820") and not hype_failure.contains("requires converted actor hit points"),"Hype did not read original actor life state into designer field128"): return
    if not require(first["scene"] == "res://game/new_game.tscn" and first["loading_stage"] == "opening" and first["save_disabled"],"New Game mode or opening state differs"): return
    if not require(first["global_data_brains"].size() == 1 and first["global_data_brains"][0]["model"] == "fix:/ai/models/1" and not first["global_data_brains"][0]["automatic"],"exported global-data role changed"): return
    if not require(first["automatic_brains"] == first["brain_count"] and first["brain_count"] > 0,"opening held generated Brains"): return
    if not require(absf(float(first["gate_height"])) < 0.1 and first["gate_automatic"],"New Game used the saved-game gate workaround"): return
    if not require(Vector3(first["position"][0],first["position"][2],-first["position"][1]).distance_to(Vector3(-13.548369,32.932507,-158.19508)) < 1.0,"New Game restored the manor save instead of original starting pose"): return
    var level := game.get("level") as Node3D
    var source_ui := level.get_node_or_null("AstrolabeUI")
    if not require(source_ui != null,"normal generated level lacks maintained UI"): return
    var dialogue_names: Array[String] = []
    for item in source_ui.get("dialogue_items"): dialogue_names.append(str(item.get("identity")))
    dialogue_names.sort()
    report["dialogue_items"] = dialogue_names
    if not require(dialogue_names == ["gogood_master_whatif01","gogood_texte1_manoir","gogood_texte1a_manoir","gogood_texte1b_manoir","gogood_texte1c_manoir","gogood_texte2_manoir","gogood_texte2a_manoir","hype_texte1_manoir","hype_texte2_manoir"],"normal New Game lacks the nine integrated dialogue resources"): return
    var paths: Dictionary = level.get_meta("source_actor_nodes")
    var gogoud := level.get_node(paths["level:/persos/world/0/5"]) as CharacterBody3D
    var turn_target := level.get_node(paths["level:/persos/world/0/28"]) as Node3D
    var resolved_target: Node3D = gogoud.source_actor_node("level:/persos/world/0/28")
    report["turn_target"] = {"asset":"level:/persos/world/0/28","path":str(turn_target.get_path()),"resolved":resolved_target == turn_target}
    if not require(resolved_target == turn_target,"wrapped New Game did not resolve Gogoud's exported statue target"): return
    var gogoud_start := gogoud.global_position
    if not require(gogoud.get_node("Brain").get("current_normal") == 0 and gogoud.current_state == 0 and gogoud.source_route_active,"normal opening did not start Gogoud's original route"): return
    gogoud_actions.append(0)
    gogoud_frames.append(gogoud_snapshot(gogoud))
    report["gogoud"] = {"actions":gogoud_actions,"frames":gogoud_frames}
    var first_actor_id := (game.get("actor") as Node).get_instance_id()
    var initial_resource: Variant = root.get_node("GameContent").get("level_scene")
    key(KEY_F5)
    if not require(FileAccess.get_file_as_string(SAVE_STATE.path()) == SLOT_PROBE,"New Game save input overwrote existing slot"): return
    await create_timer(2.0).timeout
    report["after_two_seconds"] = snapshot(game)
    if not check_camera_screen_state("after_two_seconds", report["after_two_seconds"]): return
    report["image_initial"] = await capture("new-game-opening-start")
    for tick in range(900):
        await physics_frame
        await process_frame
        var frame := gogoud_snapshot(gogoud)
        var changed: bool = gogoud_actions[-1] != gogoud.current_state
        if changed: gogoud_actions.append(gogoud.current_state)
        if tick % 3 == 0 or changed: gogoud_frames.append(frame)
        report["gogoud"]["finish"] = frame
        if not require(str(frame["failure"]).is_empty(),"Gogoud stopped before completing his walk: " + str(frame["failure"])): return
        if gogoud.current_state == 23: break
    var has_contact := false
    for frame in gogoud_frames:
        for contact in frame["contacts"]:
            if not str(contact["source_node"]).is_empty() and not str(contact["source_material"]).is_empty(): has_contact = true
    report["gogoud"] = {"actions":gogoud_actions,"frames":gogoud_frames,"finish":gogoud_snapshot(gogoud),"source_contact":has_contact}
    if not require(gogoud_actions == [0,26,24,23] and has_contact and gogoud.is_on_floor(),"normal opening did not complete Gogoud's walk/actions with native contact"): return
    if not require(gogoud.global_position.distance_to(gogoud_start) > 8.0 and gogoud.global_position.distance_to(Vector3(-18.638809,32.910141,-149.097321)) < 0.75,"normal-opening Gogoud motion/end pose differs from source reference"): return
    report["after_walk"] = snapshot(game)
    report["image"] = await capture("new-game-opening-walk-end")
    print("GOGOUD_NORMAL_OPENING_OK ", JSON.stringify(report["gogoud"]["finish"]))
    # Follow the same source schedule through its turn and first dialogue.
    # Other actor stops remain recorded; this does not accept the full intro.
    var post_path_deadline := Time.get_ticks_msec() + 20000
    var post_path_actions: Array[int] = [gogoud.current_state]
    while Time.get_ticks_msec() < post_path_deadline:
        await physics_frame
        await process_frame
        if post_path_actions[-1] != gogoud.current_state: post_path_actions.append(gogoud.current_state)
        if not str(gogoud.get_node("Brain").get("unsupported_failure")).is_empty() or not bool(source_ui.call("source_dialog_is_over")): break
    report["gogoud"]["post_path"] = gogoud_snapshot(gogoud)
    var remaining_turn: Vector2 = gogoud.source_turn_angles(turn_target)
    report["turn_target"]["remaining_angle"] = remaining_turn.y
    report["post_path"] = snapshot(game)
    if not check_camera_screen_state("post_path", report["post_path"]): return
    var dialog_position: Vector2 = source_ui.get("source_position")
    report["ui_after_path"] = {"position":[dialog_position.x,dialog_position.y],"dialogue_over":source_ui.call("source_dialog_is_over")}
    report["gogoud"]["post_path_actions"] = post_path_actions
    if not require(str(report["gogoud"]["post_path"]["failure"]).is_empty(),"Gogoud stopped after his path: " + str(report["gogoud"]["post_path"]["failure"])): return
    if not require(not bool(report["ui_after_path"]["dialogue_over"]),"Gogoud did not reach his first original dialogue within 20 seconds after the path"): return
    var opening_camera: Dictionary = report["post_path"].get("camera_state", {})
    if not require(bool(opening_camera.get("source_brain_running", false)) and str(opening_camera.get("source_brain_failure", "missing SourceCamera Brain")).is_empty() and bool(opening_camera.get("source_current", false)) and not str(opening_camera.get("source_target", "")).is_empty() and str(opening_camera.get("active_path", "")) == str(opening_camera.get("source_path", "")), "normal New Game did not activate its source-registered camera through the first UI call: " + JSON.stringify(opening_camera)): return
    var dialog_owner := source_ui.get("adopted_screen") as Node
    if dialog_owner == null: dialog_owner = source_ui
    var active_dialog: Resource = dialog_owner.get("dialogue").get("item")
    report["ui_after_path"]["identity"] = active_dialog.get("identity") if active_dialog != null else ""
    if not require(report["ui_after_path"]["identity"] == "gogood_texte1_manoir" and post_path_actions == [23,20,31,32],"normal opening did not reach the source's first dialogue/action sequence"): return
    key(KEY_R)
    var restarted: Node = await wait_session(game)
    if not require(restarted != null and bool(restarted.get("new_game")) and restarted.scene_file_path == "res://game/new_game.tscn","R left the New Game route"): return
    if not require((restarted.get("actor") as Node).get_instance_id() != first_actor_id and root.get_node("GameContent").get("level_scene") == initial_resource,"restart reused actor state or discarded content cache"): return
    report["restart"] = snapshot(restarted)
    if not require(FileAccess.get_file_as_string(SAVE_STATE.path()) == SLOT_PROBE,"restart modified save slot"): return
    key(KEY_ESCAPE)
    await process_frame
    var pause_host := restarted.get_node("PauseHost")
    var pause_view := pause_host.get("view") as Control
    var restarted_actor_id := (restarted.get("actor") as Node).get_instance_id()
    if not require(paused and current_scene == restarted and pause_view != null and pause_view.visible and bool(pause_view.call("has_required_media")),"Esc did not open the original pause overlay in the current session"): return
    var resume_button := pause_view.call("button_for_action", &"Resume Game") as Button
    if not require(resume_button != null,"pause overlay is missing Resume Game"): return
    resume_button.pressed.emit()
    await create_timer(0.6).timeout
    if not require(not paused and current_scene == restarted and not pause_view.visible and (restarted.get("actor") as Node).get_instance_id() == restarted_actor_id,"Resume did not preserve the opening session"): return
    key(KEY_ESCAPE)
    await process_frame
    var menu_button := pause_view.call("button_for_action", &"Main Menu") as Button
    if not require(paused and pause_view.visible and menu_button != null,"pause overlay is missing Main Menu"): return
    menu_button.pressed.emit()
    var deadline := Time.get_ticks_msec() + 5000
    while (current_scene == null or not current_scene is Control) and Time.get_ticks_msec() < deadline: await process_frame
    if not require(not paused and current_scene is Control and current_scene.scene_file_path == "res://game/bootstrap.tscn","pause Main Menu did not leave the opening"): return
    if not require(FileAccess.get_file_as_string(SAVE_STATE.path()) == SLOT_PROBE,"menu transition modified save slot"): return
    report["pause_resume_and_main_menu"] = true

    # Exercise the real Load Game menu route with no native slot, so the
    # retained imported original save supplies the restored state.
    if not require(_remove_test_slot(), "could not clear the test-owned invalid slot before Load Game"): return
    var import_state: Dictionary = imported_fixture["state"]
    var import_player: Dictionary = import_state["actors"]["world/0/0/Hype"]
    var import_mana := int(import_player["magick"]["current_points"])
    var import_position_values: Array = import_player["transform"]["position"]
    var import_position := Vector3(float(import_position_values[0]), float(import_position_values[1]), float(import_position_values[2]))
    var stale_import_menu := current_scene
    if not require(change_scene_to_file("res://game/bootstrap.tscn") == OK, "could not refresh menu availability after clearing the invalid slot"): return
    var import_menu: Node = await wait_menu(stale_import_menu)
    if not require(import_menu != null, "refreshed main menu did not appear before Load Game"): return
    var import_load_button := import_menu.find_child("LoadGame", true, false) as Button
    if not require(import_load_button != null and not import_load_button.disabled, "Load Game was unavailable for the preserved original import"): return
    var import_transition_start := transition_events.size()
    await click_gui(import_load_button, true)
    var imported_session: Node = await wait_session(import_menu)
    if not require(imported_session != null and not bool(imported_session.get("new_game")), "Load Game did not open the saved-game session"): return
    var import_lifecycle_ok: bool = await _check_route_lifecycle("load_original_import", import_transition_start, import_mana, import_position)
    if not import_lifecycle_ok: return
    var import_completed: Dictionary = _find_transition_event("completed", import_transition_start)
    var imported_position_values: Array = import_completed.get("position", [])
    var imported_position := Vector3(float(imported_position_values[0]), float(imported_position_values[1]), float(imported_position_values[2])) if imported_position_values.size() == 3 else Vector3.ZERO
    var import_position_delta := imported_position - import_position
    var import_position_accepted := absf(import_position_delta.x) <= 0.01 and absf(import_position_delta.y) <= 0.025 and absf(import_position_delta.z) <= 0.01
    report["original_import_position_check"] = {"expected":[import_position.x, import_position.y, import_position.z],
        "observed":[imported_position.x, imported_position.y, imported_position.z],
        "delta":[import_position_delta.x, import_position_delta.y, import_position_delta.z],
        "tolerance_m":{"x":0.01,"y":0.025,"z":0.01},
        "reason":"The restored CharacterBody3D receives one active gravity/contact step before transition completion; imported semantic coordinates remain unchanged.",
        "accepted":import_position_accepted}
    if not require(import_position_accepted, "Load Game position exceeded the imported source position plus one active contact step"): return
    report["original_import_restore"] = {"mana":import_mana,"position":[import_position.x, import_position.y, import_position.z],
        "observed_position":[imported_position.x, imported_position.y, imported_position.z],
        "position_delta":[import_position_delta.x, import_position_delta.y, import_position_delta.z],
        "native_slot_absent":not FileAccess.file_exists(SAVE_STATE.path()),"import_preserved":FileAccess.get_file_as_string(ORIGINAL_IMPORT_PATH) == original_import_backup}
    if not require(report["original_import_restore"]["native_slot_absent"] and report["original_import_restore"]["import_preserved"], "original import route changed a source fixture or created a native slot"): return

    # A versioned native slot must take precedence over the preserved import.
    var native_state: Dictionary = import_state.duplicate(true)
    var native_player: Dictionary = native_state["actors"]["world/0/0/Hype"]
    var imported_mana := int(native_player["magick"]["current_points"])
    var native_mana := 0 if imported_mana != 0 else 1
    native_player["magick"]["current_points"] = native_mana
    var native_position_values: Array = native_player["transform"]["position"]
    native_position_values[0] = float(native_position_values[0]) + 2.0
    native_player["transform"]["position"] = native_position_values
    var native_validation := str(SAVE_STATE.validate(native_state))
    if not require(native_validation.is_empty(), "native schema-1 precedence fixture is invalid: " + native_validation): return
    var native_file := FileAccess.open(SAVE_STATE.path(), FileAccess.WRITE)
    if not require(native_file != null, "isolated native slot could not be created"): return
    native_file.store_string(JSON.stringify(native_state) + "\n")
    native_file.close()
    if not require(SAVE_STATE.read().get("status") == "loaded", "schema-1 native slot was not accepted"): return
    var native_menu := await return_to_menu(imported_session)
    if not require(native_menu != null, "could not return to the menu after original import Load Game"): return
    var native_load_button := native_menu.find_child("LoadGame", true, false) as Button
    if not require(native_load_button != null and not native_load_button.disabled, "valid native schema-1 slot did not enable Load Game"): return
    var native_transition_start := transition_events.size()
    await click_gui(native_load_button, true)
    var native_session: Node = await wait_session(native_menu)
    if not require(native_session != null and not bool(native_session.get("new_game")), "Load Game did not open the native-slot session"): return
    var native_expected_position := Vector3(float(native_position_values[0]), float(native_position_values[1]), float(native_position_values[2]))
    var native_lifecycle_ok: bool = await _check_route_lifecycle("load_native_slot", native_transition_start, native_mana, native_expected_position)
    if not native_lifecycle_ok: return
    var native_completed: Dictionary = _find_transition_event("completed", native_transition_start)
    var native_applied_values: Array = native_completed.get("position", [])
    var native_applied_position := Vector3(float(native_applied_values[0]), float(native_applied_values[1]), float(native_applied_values[2])) if native_applied_values.size() == 3 else Vector3.ZERO
    var native_position_delta := native_applied_position - native_expected_position
    var native_position_accepted := absf(native_position_delta.x) <= 0.01 and absf(native_position_delta.y) <= 0.025 and absf(native_position_delta.z) <= 0.01
    report["native_slot_position_check"] = {"expected":[native_expected_position.x,native_expected_position.y,native_expected_position.z],
        "observed":[native_applied_position.x,native_applied_position.y,native_applied_position.z],
        "delta":[native_position_delta.x,native_position_delta.y,native_position_delta.z],
        "tolerance_m":{"x":0.01,"y":0.025,"z":0.01},
        "reason":"The restored CharacterBody3D receives one active gravity/contact step before transition completion; native slot values remain authoritative.",
        "accepted":native_position_accepted}
    if not require(native_position_accepted, "native schema-1 slot position exceeded its saved transform plus one active contact step"): return
    report["native_slot_precedence"] = {"schema":1,"native_mana":native_mana,"import_mana":import_mana,
        "position":[native_expected_position.x,native_expected_position.y,native_expected_position.z],
        "observed_position":[native_applied_position.x,native_applied_position.y,native_applied_position.z],
        "position_delta":[native_position_delta.x,native_position_delta.y,native_position_delta.z],
        "import_still_present":FileAccess.get_file_as_string(ORIGINAL_IMPORT_PATH) == original_import_backup}
    var error_menu := await return_to_menu(native_session)
    if not require(error_menu != null, "could not return to the menu after native-slot Load Game"): return

    # Corrupt only the staged import copy to drive the actual route failure,
    # dismiss its error, and verify recovery to the main menu.
    if not require(_remove_test_slot(), "could not remove the test-owned native slot before the failure route"): return
    var corrupt_import := FileAccess.open(ORIGINAL_IMPORT_PATH, FileAccess.WRITE)
    if not require(corrupt_import != null, "the staged import fixture cannot be modified for an isolated failure check"): return
    original_import_changed = true
    corrupt_import.store_string("{ invalid original import\n")
    corrupt_import.close()
    var failure_button := error_menu.find_child("LoadGame", true, false) as Button
    if not require(failure_button != null and not failure_button.disabled, "Load Game was not offered for the isolated failure check"): return
    var failure_transition_start := transition_events.size()
    await click_gui(failure_button)
    var failed_event: Dictionary = await _wait_transition_event("failed", failure_transition_start, 30000)
    if not require(not failed_event.is_empty(), "Load Game failure did not emit transition_failed"): return
    var presenter := _loading_screen()
    var dismiss := presenter.get_node_or_null("Viewport/Center/Content/Dismiss") as Button if presenter != null else null
    var dismiss_deadline := Time.get_ticks_msec() + 5000
    while (dismiss == null or not dismiss.visible) and Time.get_ticks_msec() < dismiss_deadline:
        await process_frame
        presenter = _loading_screen()
        dismiss = presenter.get_node_or_null("Viewport/Center/Content/Dismiss") as Button if presenter != null else null
    if not require(presenter != null and presenter.visible and dismiss != null and dismiss.visible, "Load Game failure did not leave a dismissible loading error visible"): return
    report["load_failure"] = {"failed":failed_event,"presenter_visible":presenter.visible,"dismiss_visible":dismiss.visible}
    var failure_session := current_scene
    await click_gui(dismiss)
    var recovered_menu: Node = await wait_menu(failure_session, 15000)
    if not require(recovered_menu != null, "dismissing the Load Game error did not recover to the menu"): return
    var recovery_status := recovered_menu.get_node("Status/Details") as Label
    if not require(recovery_status.text.contains("The game could not be opened"), "recovered menu did not report the Load Game failure"): return
    _restore_test_fixtures()
    if not require(FileAccess.get_file_as_string(ORIGINAL_IMPORT_PATH) == original_import_backup and SAVE_STATE.read_original_import().get("status") == "loaded", "the staged original import was not restored after the isolated failure check"): return
    if not require(_remove_test_slot(), "test-owned native slot could not be removed at the end of acceptance"): return
    report["save_preserved"] = true
    report["status"] = "passed"
    write_report()
    print("NEW_GAME_AND_LOAD_GAME_ROUTES_OK")
    await shutdown(0)

func shutdown(exit_code: int) -> void:
    _restore_test_fixtures()
    if not _remove_test_slot():
        push_error("Could not remove the isolated test save slot.")
    current_scene = null
    for child in root.get_children(): child.queue_free()
    for frame in range(3): await process_frame
    quit(exit_code)
