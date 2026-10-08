extends SceneTree
## Actual menu/session path. Run only with an isolated XDG_DATA_HOME; the
## slot below is test-owned. Full generated AI remains unmodified and unheld.
const SAVE_STATE = preload("res://game/save_state.gd")
const ORIGINAL_IMPORT_PATH := "res://game/original_save_import.json"
const NEW_GAME_SCENE_PATH := "res://game/new_game.tscn"
const FLOAT32_EPSILON := 0.00000011920928955078125
const FLOAT32_ROUNDTRIP_EPSILONS := 2.0
class WaitSessionProbe:
    extends Node
    var ready_to_play: bool = true
var report: Dictionary = {"scope":"actual New Game and Load Game routes, original import/native precedence, normal opening walk/contact checkpoint", "intro_acceptance":"pending"}
var evidence_path: String
var gogoud_actions: Array[int] = []
var gogoud_frames: Array[Dictionary] = []
var transition_events: Array[Dictionary] = []
var session_stage_events: Array[Dictionary] = []
var gui_click_events: Array[Dictionary] = []
var original_import_backup := ""
var original_import_sha256 := ""
var original_import_changed := false
var new_game_scene_source_abs := ""
var new_game_scene_backup_abs := ""
var wait_helper_only_mode := false
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
    report["status"] = "failed"
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

func _vector2_array(value: Vector2) -> Array:
    return [value.x, value.y]

func _vector2i_array(value: Vector2i) -> Array:
    return [value.x, value.y]

func _transform_array(value: Transform2D) -> Dictionary:
    return {"x":[value.x.x, value.x.y],"y":[value.y.x, value.y.y],"origin":[value.origin.x, value.origin.y]}

func _viewport_metrics() -> Dictionary:
    var window := root as Window
    var viewport := root.get_viewport()
    var visible_rect := viewport.get_visible_rect()
    return {"root_window_size":_vector2i_array(window.size),
        "root_viewport_size":_vector2i_array(viewport.size),
        "display_server_window_size":_vector2i_array(DisplayServer.window_get_size()),
        "content_scale_size":_vector2i_array(window.content_scale_size),
        "content_scale_mode":int(window.content_scale_mode),
        "visible_rect":{"position":_vector2_array(visible_rect.position),"size":_vector2_array(visible_rect.size)}}

func key(code: Key) -> void:
    for pressed in [true,false]:
        var event := InputEventKey.new()
        event.physical_keycode = code
        event.pressed = pressed
        Input.parse_input_event(event)
        Input.flush_buffered_events()

func click_gui(control: Control, repeat_immediately: bool = false) -> void:
    await process_frame
    var local_center := control.size * 0.5
    var legacy_position := control.get_global_transform() * local_center
    var position := control.get_global_transform_with_canvas() * local_center
    var motion := InputEventMouseMotion.new()
    motion.position = position
    motion.global_position = position
    Input.parse_input_event(motion)
    Input.flush_buffered_events()
    await process_frame
    var hovered := root.get_viewport().gui_get_hovered_control() as Control
    var observation := {"target":str(control.get_path()),
        "legacy_canvas_position":_vector2_array(legacy_position),
        "event_position":_vector2_array(position),
        "target_global_transform":_transform_array(control.get_global_transform()),
        "target_global_transform_with_canvas":_transform_array(control.get_global_transform_with_canvas()),
        "hovered_control":str(hovered.get_path()) if hovered != null else "",
        "target_hovered":hovered == control}
    gui_click_events.append(observation)
    report["gui_click_events"] = gui_click_events.duplicate(true)
    write_report()
    if not require(hovered == control, "GUI event position %s did not hover %s (hovered %s)" % [str(position), str(control.get_path()), str(hovered.get_path()) if hovered != null else "<none>"]): return
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
        "actor_physics_enabled":actor.is_physics_processing() if actor != null else false,
        "hype_brain":_actor_brain_snapshot(actor),"gogoud_brain_dialogue":_gogoud_session_snapshot(session)})

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
        "position":[actor.global_position.x, actor.global_position.y, actor.global_position.z] if actor != null else [],
        "hype_brain":_actor_brain_snapshot(actor),"gogoud_brain_dialogue":_gogoud_session_snapshot(session)})

func _actor_brain_snapshot(actor: CharacterBody3D) -> Dictionary:
    if actor == null:
        return {"present":false}
    var brain := actor.get_node_or_null("Brain")
    if brain == null:
        return {"present":false,"actor":str(actor.get_path())}
    return {"present":true,"actor":str(actor.get_path()),"action":actor.get("current_state"),"normal":brain.get("current_normal"),
        "reflex":brain.get("current_reflex"),"running":brain.get("running"),"automatic":brain.get("automatic"),
        "clock_ms":brain.get("clock_milliseconds"),"failure":brain.get("unsupported_failure")}

func _gogoud_session_snapshot(session: Node) -> Dictionary:
    if session == null:
        return {"present":false,"reason":"no current session at transition completion"}
    var level := session.get("level") as Node3D
    if level == null:
        return {"present":false,"reason":"session has no loaded level"}
    var paths: Dictionary = level.get_meta("source_actor_nodes", {})
    var actor_path := str(paths.get("level:/persos/world/0/5", ""))
    var actor := level.get_node_or_null(actor_path) as CharacterBody3D if not actor_path.is_empty() else null
    if actor == null:
        return {"present":false,"identity":"level:/persos/world/0/5","path":actor_path}
    var brain := actor.get_node_or_null("Brain")
    var screen := level.get_node_or_null("AstrolabeUI")
    var dialogue_owner: Node = screen
    if screen != null:
        var adopted := screen.get("adopted_screen") as Node
        if adopted != null:
            dialogue_owner = adopted
    var dialogue := dialogue_owner.get("dialogue") as Control if dialogue_owner != null else null
    var item: Resource = dialogue.get("item") as Resource if dialogue != null else null
    var source_dialogue_over: Variant = screen.call("source_dialog_is_over") if screen != null and screen.is_node_ready() and screen.has_method("source_dialog_is_over") else null
    return {"present":true,"identity":"level:/persos/world/0/5","path":str(actor.get_path()),
        "action":actor.get("current_state"),"normal":brain.get("current_normal") if brain != null else null,
        "reflex":brain.get("current_reflex") if brain != null else null,"brain_running":brain.get("running") if brain != null else null,
        "brain_automatic":brain.get("automatic") if brain != null else null,"brain_clock_ms":brain.get("clock_milliseconds") if brain != null else null,
        "brain_failure":brain.get("unsupported_failure") if brain != null else null,"route_active":actor.get("source_route_active"),
        "position_source_axes":[actor.global_position.x,-actor.global_position.z,actor.global_position.y],
        "global_gate":_global_gate_snapshot(level),
        "manor_gate":_manor_gate_snapshot(level),
        "dialogue_presenter":str(dialogue.get_path()) if dialogue != null else "",
        "dialogue_visible":dialogue.visible if dialogue != null else false,
        "dialogue_item":item.get("identity") if item != null else "",
        "dialogue_over":source_dialogue_over}

func _global_gate_snapshot(level: Node3D) -> Dictionary:
    var source_brains: Dictionary = level.get_meta("source_actor_brains", {})
    var global_brain := level.get_node_or_null(source_brains.get("fix:/persos/15", NodePath())) as Node
    if global_brain == null:
        return {"present":false,"identity":"fix:/persos/15"}
    var property_names := global_brain.get_property_list().map(func(property: Dictionary) -> String: return str(property.get("name", "")))
    var has_dsg_147 := property_names.has("dsg_147")
    var has_dsg_149 := property_names.has("dsg_149")
    return {"present":true,"identity":"fix:/persos/15","path":str(global_brain.get_path()),
        "has_dsg_147":has_dsg_147,"dsg_147":global_brain.get("dsg_147") if has_dsg_147 else null,
        "has_dsg_149":has_dsg_149,"dsg_149":global_brain.get("dsg_149") if has_dsg_149 else null}

func _manor_gate_snapshot(level: Node3D) -> Dictionary:
    var source_nodes: Dictionary = level.get_meta("source_actor_nodes", {})
    var gate_path := str(source_nodes.get("level:/persos/world/0/15", "world/0/15/iMGrille_Manoir_Pont"))
    var gate := level.get_node_or_null(gate_path) as Node3D
    if gate == null:
        return {"present":false,"identity":"level:/persos/world/0/15","path":gate_path}
    var brain := gate.get_node_or_null("Brain")
    var local_position := gate.position
    var global_position := gate.global_position if gate.is_inside_tree() else Vector3.ZERO
    return {"present":true,"identity":"level:/persos/world/0/15","path":str(gate.get_path()),
        "local_position":[local_position.x,local_position.y,local_position.z],
        "global_position":[global_position.x,global_position.y,global_position.z] if gate.is_inside_tree() else [],
        "source_position":[global_position.x,-global_position.z,global_position.y] if gate.is_inside_tree() else [],
        "action":gate.get("current_state") if gate is CharacterBody3D else null,"brain_normal":brain.get("current_normal") if brain != null else null,
        "brain_reflex":brain.get("current_reflex") if brain != null else null,
        "brain_running":brain.get("running") if brain != null else null,
        "brain_automatic":brain.get("automatic") if brain != null else null,
        "brain_failure":brain.get("unsupported_failure") if brain != null else null}

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
    var tree_ready := _find_session_stage("tree_ready", int(started.get("observed_at_ms", 0)))
    var instantiated := _find_session_stage("instantiated", int(started.get("observed_at_ms", 0)))
    report[route_name + "_lifecycle"] = {"started":started,"instantiated":instantiated,"tree_ready":tree_ready,
        "restoration":restoration,"restored_pose":restored_pose,"completed":completed}
    return true

func _find_session_stage(stage_name: String, start_time: int) -> Dictionary:
    for event in session_stage_events:
        if event.get("observed_at_ms", 0) >= start_time and event.get("stage") == stage_name:
            return event
    return {}

func _live_current_scene() -> Node:
    var scene: Node = current_scene
    return scene if scene != null and is_instance_valid(scene) else null

func wait_menu(previous_instance_id: int, timeout_ms: int = 15000) -> Node:
    var deadline := Time.get_ticks_msec() + timeout_ms
    while Time.get_ticks_msec() < deadline:
        var scene := _live_current_scene()
        if scene != null and scene.get_instance_id() != previous_instance_id and scene.scene_file_path == "res://game/bootstrap.tscn":
            return scene
        await process_frame
    return null

func wait_session(previous_instance_id: int, timeout_ms: int = 70000) -> Node:
    var deadline := Time.get_ticks_msec() + timeout_ms
    while Time.get_ticks_msec() < deadline:
        var scene := _live_current_scene()
        if scene != null and scene.get_instance_id() != previous_instance_id and bool(scene.get("ready_to_play")):
            return scene
        await process_frame
    return null

func _check_scene_replacement_wait_helpers() -> bool:
    var packed := load("res://game/bootstrap.tscn") as PackedScene
    if not require(packed != null, "wait_menu replacement probe could not load the authored bootstrap scene"): return false
    var old_menu := packed.instantiate()
    root.add_child(old_menu)
    current_scene = old_menu
    var old_instance_id := old_menu.get_instance_id()
    old_menu.free()
    old_menu = null
    current_scene = null
    var replacement := packed.instantiate()
    root.add_child(replacement)
    current_scene = replacement
    var began_at := Time.get_ticks_msec()
    var observed: Node = await wait_menu(old_instance_id, 250)
    var elapsed_ms := Time.get_ticks_msec() - began_at
    var passed := observed != null and is_instance_valid(observed) and observed == replacement and observed.get_instance_id() != old_instance_id and elapsed_ms < 250
    report["wait_menu_replacement_probe"] = {"old_instance_id":old_instance_id,"old_freed_before_wait":true,
        "replacement_instance_id":replacement.get_instance_id(),"observed_current_scene":observed == replacement,
        "elapsed_ms":elapsed_ms,"passed":passed}
    current_scene = null
    replacement.queue_free()
    await process_frame
    if not require(passed, "wait_menu did not observe the fresh bootstrap scene after its prior instance was freed"): return false

    var old_session := WaitSessionProbe.new()
    root.add_child(old_session)
    current_scene = old_session
    var old_session_instance_id := old_session.get_instance_id()
    old_session.free()
    old_session = null
    current_scene = null
    var replacement_session := WaitSessionProbe.new()
    root.add_child(replacement_session)
    current_scene = replacement_session
    began_at = Time.get_ticks_msec()
    var observed_session: Node = await wait_session(old_session_instance_id, 250)
    elapsed_ms = Time.get_ticks_msec() - began_at
    passed = observed_session != null and is_instance_valid(observed_session) and observed_session == replacement_session and observed_session.get_instance_id() != old_session_instance_id and elapsed_ms < 250
    report["wait_session_replacement_probe"] = {"old_instance_id":old_session_instance_id,"old_freed_before_wait":true,
        "replacement_instance_id":replacement_session.get_instance_id(),"observed_current_scene":observed_session == replacement_session,
        "elapsed_ms":elapsed_ms,"passed":passed}
    current_scene = null
    replacement_session.queue_free()
    await process_frame
    return require(passed, "wait_session did not observe the fresh ready session after its prior instance was freed")

func return_to_menu(session: Node) -> Node:
    var transition_idle: bool = await _wait_transition_idle()
    if not transition_idle:
        return null
    if session == null or not is_instance_valid(session) or not session.has_method("leave_session"):
        return null
    var previous_instance_id := session.get_instance_id()
    session.call("leave_session", "res://game/bootstrap.tscn")
    return await wait_menu(previous_instance_id)

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
    if args.size() == 2 and args[0] == "--wait-helper-only":
        wait_helper_only_mode = true
        evidence_path = args[1]
        if not await _check_scene_replacement_wait_helpers(): return
        report["status"] = "passed"
        report["scope"] = "focused freed-current-scene replacement checks for wait_menu and wait_session"
        write_report()
        await shutdown(0)
        return
    if not require(args.size() == 1,"requires evidence JSON path"): return
    evidence_path = args[0]
    var window := root as Window
    var initial_viewport_metrics := _viewport_metrics()
    var configured_viewport := Vector2i(
        int(ProjectSettings.get_setting("display/window/size/viewport_width", 1024)),
        int(ProjectSettings.get_setting("display/window/size/viewport_height", 768)))
    window.size = configured_viewport
    window.content_scale_size = configured_viewport
    await process_frame
    await process_frame
    report["viewport_setup"] = {"configured_size":_vector2i_array(configured_viewport),
        "before":initial_viewport_metrics,"after":_viewport_metrics()}
    if not require(root.get_viewport().size == configured_viewport,
        "route smoke could not establish the configured project viewport %s" % str(configured_viewport)): return
    write_report()
    if not _check_float32_tolerance_contract(): return
    if not require(not FileAccess.file_exists(SAVE_STATE.path()),"isolated slot must begin absent"): return
    original_import_backup = FileAccess.get_file_as_string(ORIGINAL_IMPORT_PATH) if FileAccess.file_exists(ORIGINAL_IMPORT_PATH) else ""
    original_import_sha256 = FileAccess.get_sha256(ProjectSettings.globalize_path(ORIGINAL_IMPORT_PATH)) if not original_import_backup.is_empty() else ""
    var imported_fixture: Dictionary = SAVE_STATE.read_original_import()
    if not require(not original_import_backup.is_empty() and imported_fixture.get("status") == "loaded", "the preserved original-save import fixture is unavailable or invalid"): return
    if not require(_connect_transition_observers(), "persistent LevelTransition service is missing its lifecycle signals"): return
    var import_document: Variant = JSON.parse_string(original_import_backup)
    report["original_import_fixture"] = {"present":true,"valid":true,"profile":import_document.get("profile", "") if import_document is Dictionary else "",
        "sha256_before":original_import_sha256}
    var scene_replacement_probe_ok: bool = await _check_scene_replacement_wait_helpers()
    if not scene_replacement_probe_ok: return
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
    var menu_instance_id := menu.get_instance_id()
    await click_gui(new_button, true)
    if not require(current_scene == menu and not new_button.disabled and not bool(menu.get("opening_game")) and menu.get_node("Status").visible, "a failed New Game destination did not leave the menu usable"): return
    report["new_game_route_failure"] = {"menu_recovered":true,"button_reenabled":not new_button.disabled,"status":menu.get_node("Status/Details").text}
    _restore_test_fixtures()
    if not require(FileAccess.file_exists(NEW_GAME_SCENE_PATH), "New Game route scene was not restored after the failure check"): return
    var new_game_transition_start := transition_events.size()
    menu_instance_id = menu.get_instance_id()
    await click_gui(new_button, true)
    var game: Node = await wait_session(menu_instance_id)
    if not require(game != null and bool(game.get("new_game")),"New Game did not reach the actual session"): return
    var new_game_actor := game.get("actor") as CharacterBody3D
    var new_game_magick: Resource = new_game_actor.get("source_magick") if new_game_actor != null else null
    if not require(new_game_magick != null, "New Game did not initialize native player state"): return
    var new_game_lifecycle_ok: bool = await _check_route_lifecycle("new_game", new_game_transition_start, int(new_game_magick.get("current_points")))
    if not new_game_lifecycle_ok: return
    var new_game_completed_event := _find_transition_event("completed", new_game_transition_start)
    var new_game_started_event := _find_transition_event("started", new_game_transition_start)
    var new_game_instantiated := _find_session_stage("instantiated", int(new_game_started_event.get("observed_at_ms", 0)))
    report["new_game_brain_dialogue_at_completion"] = {"hype":new_game_completed_event.get("hype_brain", {}),
        "gogoud":new_game_completed_event.get("gogoud_brain_dialogue", {})}
    var new_game_gogoud_initial: Dictionary = new_game_instantiated.get("gogoud_brain_dialogue", {})
    var new_game_gate: Dictionary = new_game_gogoud_initial.get("global_gate", {})
    var new_game_normal_initial: Variant = new_game_gogoud_initial.get("normal")
    report["new_game_intro_state_boundary"] = {"source_expected_global_dsg_147":false,
        "global_dsg_147_present":new_game_gate.get("has_dsg_147", false),"global_dsg_147":new_game_gate.get("dsg_147"),
        "gogoud_normal_before_tree_ai_start":new_game_normal_initial,
        "gogoud_state_before_tree_ai_start":new_game_gogoud_initial,
        "expected_brain_transition":"Normal[13] to Normal[0] after default DSG[147]=false"}
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
    report["new_game_brain_dialogue_after_intro_path"] = {"hype":_actor_brain_snapshot(hype),"gogoud":_gogoud_session_snapshot(game),
        "dialogue_identity":report["ui_after_path"]["identity"]}
    var new_game_normal_sequence: Array[int] = []
    var initial_normal := int(new_game_normal_initial) if new_game_normal_initial is int else -1
    if initial_normal >= 0: new_game_normal_sequence.append(initial_normal)
    for frame in gogoud_frames:
        var observed_normal := int(frame.get("normal", -1))
        if observed_normal >= 0 and (new_game_normal_sequence.is_empty() or new_game_normal_sequence[-1] != observed_normal):
            new_game_normal_sequence.append(observed_normal)
    report["new_game_intro_state_boundary"]["normal_sequence"] = new_game_normal_sequence
    var normal_13_index := new_game_normal_sequence.find(13)
    var normal_0_index := new_game_normal_sequence.find(0)
    report["new_game_intro_state_boundary"]["source_normal_transition_observed"] = normal_13_index >= 0 and normal_0_index > normal_13_index
    report["new_game_intro_state_boundary"]["dsg_147_default_check"] = "false" if report["new_game_intro_state_boundary"]["global_dsg_147_present"] and not bool(report["new_game_intro_state_boundary"]["global_dsg_147"]) else "not available in current frozen support"
    var game_instance_id := game.get_instance_id()
    key(KEY_R)
    var restarted: Node = await wait_session(game_instance_id)
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
    var import_contract_actors: Dictionary = import_document.get("godot_state", {}).get("actors", {})
    var import_global_state: Dictionary = import_contract_actors.get("fix:/persos/15", {})
    var import_global_brain: Dictionary = import_global_state.get("brain", {})
    report["gogoud_save_contract"] = {"identity":"level:/persos/world/0/5",
        "expected_global_identity":"fix:/persos/15","source_record":{"record":"0:17:4562","byte_hex":"01","boolean":true},
        "gate_open_source_record":{"record":"0:17:4560","byte_hex":"01","value":1,
            "interpretation":"post-key/open per user; key cause and consumption unproven"},
        "import_json_path":"godot_state.actors[\"fix:/persos/15\"].brain.dsg_147",
        "native_slot_path":"actors[\"fix:/persos/15\"].brain.dsg_147",
        "native_schema_treatment":"confirmed optional schema-1 Boolean alongside dsg_149; older schema-1 slots may omit it and retain the scene default",
        "restore_order":"apply saved dsg_147 to fix:/persos/15 before deferred AI start",
        "gate_flag_source":{"owner":"fix:/persos/15","field":"dsg_149","source_record":"0:17:4560","source_value":true,
            "gate_path":"Normal[1] uses raw5/state36 then writes dsg149=1; open displacement is SelfPosition+[2.6,0,0] in source axes"},
        "unresolved_behavior":"Gogoud Normal13 true branch uses category4/op4 (Raymap Proc_DesactivateObject); native effect unverified.",
        "unproven_chain":"No model210 raw196/key-grant/item/ownership chain is established.",
        "original_import_dsg_147_present":import_global_brain.has("dsg_147"),"original_import_dsg_147":import_global_brain.get("dsg_147"),
        "restored_fields":import_document.get("restored_fields", []),"not_restored":import_document.get("not_restored", [])}
    var stale_import_menu_instance_id := current_scene.get_instance_id()
    if not require(change_scene_to_file("res://game/bootstrap.tscn") == OK, "could not refresh menu availability after clearing the invalid slot"): return
    var import_menu: Node = await wait_menu(stale_import_menu_instance_id)
    if not require(import_menu != null, "refreshed main menu did not appear before Load Game"): return
    var import_load_button := import_menu.find_child("LoadGame", true, false) as Button
    if not require(import_load_button != null and not import_load_button.disabled, "Load Game was unavailable for the preserved original import"): return
    var import_transition_start := transition_events.size()
    var import_menu_instance_id := import_menu.get_instance_id()
    await click_gui(import_load_button, true)
    var imported_session: Node = await wait_session(import_menu_instance_id)
    if not require(imported_session != null and not bool(imported_session.get("new_game")), "Load Game did not open the saved-game session"): return
    var import_lifecycle_ok: bool = await _check_route_lifecycle("load_original_import", import_transition_start, import_mana, import_position)
    if not import_lifecycle_ok: return
    var import_completed: Dictionary = _find_transition_event("completed", import_transition_start)
    report["original_import_brain_dialogue_at_completion"] = {"hype":import_completed.get("hype_brain", {}),
        "gogoud":import_completed.get("gogoud_brain_dialogue", {}),
        "global_gate":import_completed.get("gogoud_brain_dialogue", {}).get("global_gate", {})}
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
    var native_menu_instance_id := native_menu.get_instance_id()
    await click_gui(native_load_button, true)
    var native_session: Node = await wait_session(native_menu_instance_id)
    if not require(native_session != null and not bool(native_session.get("new_game")), "Load Game did not open the native-slot session"): return
    var native_expected_position := Vector3(float(native_position_values[0]), float(native_position_values[1]), float(native_position_values[2]))
    var native_lifecycle_ok: bool = await _check_route_lifecycle("load_native_slot", native_transition_start, native_mana, native_expected_position)
    if not native_lifecycle_ok: return
    var native_completed: Dictionary = _find_transition_event("completed", native_transition_start)
    var native_contract_actors: Dictionary = native_state.get("actors", {})
    var native_global_brain: Dictionary = native_contract_actors.get("fix:/persos/15", {}).get("brain", {})
    report["native_slot_brain_dialogue_at_completion"] = {"hype":native_completed.get("hype_brain", {}),
        "gogoud":native_completed.get("gogoud_brain_dialogue", {}),
        "global_gate":native_completed.get("gogoud_brain_dialogue", {}).get("global_gate", {}),
        "dsg_147_present":native_global_brain.has("dsg_147"),"dsg_147_saved_value":native_global_brain.get("dsg_147")}
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
    var failure_session_instance_id := current_scene.get_instance_id()
    await click_gui(dismiss)
    var recovered_menu: Node = await wait_menu(failure_session_instance_id, 15000)
    if not require(recovered_menu != null, "dismissing the Load Game error did not recover to the menu"): return
    var recovery_status := recovered_menu.get_node("Status/Details") as Label
    if not require(recovery_status.text.contains("The game could not be opened"), "recovered menu did not report the Load Game failure"): return
    _restore_test_fixtures()
    var restored_import_sha256 := FileAccess.get_sha256(ProjectSettings.globalize_path(ORIGINAL_IMPORT_PATH)) if FileAccess.file_exists(ORIGINAL_IMPORT_PATH) else ""
    report["original_import_fixture"]["sha256_after"] = restored_import_sha256
    report["original_import_fixture"]["restored_exactly"] = FileAccess.get_file_as_string(ORIGINAL_IMPORT_PATH) == original_import_backup and restored_import_sha256 == original_import_sha256
    if not require(report["original_import_fixture"]["restored_exactly"] and SAVE_STATE.read_original_import().get("status") == "loaded", "the staged original import was not restored byte-for-byte after the isolated failure check"): return
    if not require(_remove_test_slot(), "test-owned native slot could not be removed at the end of acceptance"): return
    report["save_preserved"] = true
    report["gogoud_semantic_follow_up"] = {"status":"deferred_until_main_accepts_f7d01209",
        "reason":"The frozen support predates optional schema-1 fix:/persos/15.brain.dsg_147; keep this acceptance focused on the actual New Game/Load Game lifecycle and save precedence.",
        "passive_transition_snapshots_present":true,
        "category4_op4_handler_status":"unverified; no success claim or forced behavior",
        "key_causality":"unproven"}
    report["status"] = "passed"
    write_report()
    print("NEW_GAME_AND_LOAD_GAME_ROUTES_OK")
    await shutdown(0)

func shutdown(exit_code: int) -> void:
    if wait_helper_only_mode:
        current_scene = null
        for child in root.get_children(): child.queue_free()
        for frame in range(3): await process_frame
        report["cleanup"] = {"temporary_scene_nodes_removed":true}
        write_report()
        quit(exit_code)
        return
    _restore_test_fixtures()
    var slot_removed := _remove_test_slot()
    if not slot_removed:
        push_error("Could not remove the isolated test save slot.")
    var import_restored := FileAccess.file_exists(ORIGINAL_IMPORT_PATH) and FileAccess.get_file_as_string(ORIGINAL_IMPORT_PATH) == original_import_backup
    var import_sha256 := FileAccess.get_sha256(ProjectSettings.globalize_path(ORIGINAL_IMPORT_PATH)) if FileAccess.file_exists(ORIGINAL_IMPORT_PATH) else ""
    report["cleanup"] = {"original_import_restored":import_restored,"original_import_sha256":import_sha256,
        "original_import_hash_matches_start":original_import_sha256.is_empty() or import_sha256 == original_import_sha256,
        "test_slot_removed":slot_removed}
    if not import_restored or not report["cleanup"]["original_import_hash_matches_start"]:
        push_error("The staged original-save import did not survive smoke cleanup byte-for-byte.")
    write_report()
    current_scene = null
    for child in root.get_children(): child.queue_free()
    for frame in range(3): await process_frame
    quit(exit_code)
