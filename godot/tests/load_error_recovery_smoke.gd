extends SceneTree
## Isolated, GUI-driven check of the initial Load Game error-dismissal route.

const ORIGINAL_IMPORT_PATH := "res://game/original_save_import.json"
const SAVE_STATE = preload("res://game/save_state.gd")

var evidence_path := ""
var original_import_backup := ""
var original_import_changed := false
var transition_failure: Dictionary = {}
var dismiss_pressed := false
var report: Dictionary = {"scope":"actual Load Game failure, Dismiss input, and return to bootstrap"}

func _initialize() -> void:
	call_deferred("run")

func write_report() -> void:
	if evidence_path.is_empty():
		return
	var file := FileAccess.open(evidence_path, FileAccess.WRITE)
	if file == null:
		return
	file.store_string(JSON.stringify(report) + "\n")
	file.close()

func require(condition: bool, message: String) -> bool:
	if condition:
		return true
	report["failure"] = message
	write_report()
	push_error(message)
	shutdown.call_deferred(1)
	return false

func _loading_screen() -> Node:
	return root.get_node_or_null("LevelTransition/LoadingScreen")

func _on_transition_failed(level: String, message: String) -> void:
	transition_failure = {"level":level,"message":message,"observed_at_ms":Time.get_ticks_msec()}

func _on_dismiss_pressed() -> void:
	dismiss_pressed = true

func _control_path(control: Control) -> String:
	return str(control.get_path()) if control != null and is_instance_valid(control) else ""

func _vector2_array(value: Vector2) -> Array:
	return [value.x, value.y]

func _vector2i_array(value: Vector2i) -> Array:
	return [value.x, value.y]

func _transform_array(value: Transform2D) -> Dictionary:
	return {
		"x":[value.x.x, value.x.y],
		"y":[value.y.x, value.y.y],
		"origin":[value.origin.x, value.origin.y]
	}

func _object_property(object: Object, property_name: String) -> Variant:
	if object == null:
		return null
	for property_info in object.get_property_list():
		if str(property_info.get("name", "")) == property_name:
			return object.get(property_name)
	return null

func _viewport_diagnostics() -> Dictionary:
	var window := root as Window
	var viewport := root.get_viewport()
	var visible_rect := viewport.get_visible_rect()
	return {
		"root_window_size":_vector2i_array(window.size),
		"display_server_window_size":_vector2i_array(DisplayServer.window_get_size()),
		"root_viewport_size":_vector2i_array(viewport.size),
		"visible_rect":{"position":_vector2_array(visible_rect.position),"size":_vector2_array(visible_rect.size)},
		"content_scale_size":_vector2i_array(_object_property(window, "content_scale_size")) if _object_property(window, "content_scale_size") is Vector2i else [],
		"content_scale_mode":_object_property(window, "content_scale_mode"),
		"gui_disable_input":_object_property(viewport, "gui_disable_input"),
		"tree_paused":paused
	}

func _mouse_filter_name(control: Control) -> String:
	if control == null:
		return "missing"
	match control.mouse_filter:
		Control.MOUSE_FILTER_STOP: return "stop"
		Control.MOUSE_FILTER_PASS: return "pass"
		Control.MOUSE_FILTER_IGNORE: return "ignore"
	return str(control.mouse_filter)

func _input_diagnostics(presenter: Node, dismiss: Button) -> Dictionary:
	var viewport_control := presenter.get_node_or_null("Viewport") as Control if presenter != null else null
	var center := presenter.get_node_or_null("Viewport/Center") as Control if presenter != null else null
	var content := presenter.get_node_or_null("Viewport/Center/Content") as Control if presenter != null else null
	var viewport := root.get_viewport()
	var canvas_layer := presenter as CanvasLayer
	var focus_owner := viewport.gui_get_focus_owner()
	var rect := dismiss.get_global_rect()
	var local_center := dismiss.size * 0.5
	var canvas_center := dismiss.get_global_transform() * local_center
	var viewport_center := dismiss.get_global_transform_with_canvas() * local_center
	var visible_rect := viewport.get_visible_rect()
	return {
		"viewport":_viewport_diagnostics(),
		"canvas_layer":{
			"layer":canvas_layer.layer,
			"offset":_vector2_array(canvas_layer.offset),
			"transform":_transform_array(canvas_layer.transform),
			"final_transform":_transform_array(canvas_layer.get_final_transform()),
			"process_mode":int(canvas_layer.process_mode),
			"visible":canvas_layer.visible,
			"inside_tree":canvas_layer.is_inside_tree()
		},
		"dismiss": {
			"global_rect":{"position":[rect.position.x,rect.position.y],"size":[rect.size.x,rect.size.y]},
			"size":[dismiss.size.x,dismiss.size.y],
			"global_transform":_transform_array(dismiss.get_global_transform()),
			"global_transform_with_canvas":_transform_array(dismiss.get_global_transform_with_canvas()),
			"legacy_canvas_center":_vector2_array(canvas_center),
			"viewport_event_center":_vector2_array(viewport_center),
			"viewport_center_inside_visible_rect":visible_rect.has_point(viewport_center),
			"visible":dismiss.visible,
			"visible_in_tree":dismiss.is_visible_in_tree(),
			"disabled":dismiss.disabled,
			"has_focus":dismiss.has_focus(),
			"focus_owner":_control_path(focus_owner)
		},
		"mouse_filters": {
			"Viewport":{"value":int(viewport_control.mouse_filter) if viewport_control != null else -1,"name":_mouse_filter_name(viewport_control)},
			"Center":{"value":int(center.mouse_filter) if center != null else -1,"name":_mouse_filter_name(center)},
			"Content":{"value":int(content.mouse_filter) if content != null else -1,"name":_mouse_filter_name(content)},
			"Dismiss":{"value":int(dismiss.mouse_filter),"name":_mouse_filter_name(dismiss)}
		}
	}

func _send_mouse_motion(position: Vector2) -> String:
	var motion := InputEventMouseMotion.new()
	motion.position = position
	motion.global_position = position
	Input.parse_input_event(motion)
	Input.flush_buffered_events()
	await process_frame
	return _control_path(root.get_viewport().gui_get_hovered_control())

func _send_mouse_click(position: Vector2) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = position
		event.global_position = position
		event.pressed = pressed
		Input.parse_input_event(event)
		Input.flush_buffered_events()
	await process_frame

func click_gui(control: Control) -> void:
	await process_frame
	var position := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = position
	motion.global_position = position
	Input.parse_input_event(motion)
	Input.flush_buffered_events()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = position
		event.global_position = position
		event.pressed = pressed
		Input.parse_input_event(event)
		Input.flush_buffered_events()
	await process_frame

func wait_failure(timeout_ms: int) -> Dictionary:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while transition_failure.is_empty() and Time.get_ticks_msec() < deadline:
		await process_frame
	return transition_failure

func wait_menu(previous: Node, timeout_ms: int) -> Node:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while (current_scene == null or current_scene == previous or current_scene.scene_file_path != "res://game/bootstrap.tscn") and Time.get_ticks_msec() < deadline:
		await process_frame
	return current_scene if current_scene != null and current_scene != previous and current_scene.scene_file_path == "res://game/bootstrap.tscn" else null

func run() -> void:
	var args := OS.get_cmdline_user_args()
	if not require(args.size() == 1, "requires one evidence JSON path"):
		return
	evidence_path = args[0]
	var initial_viewport := _viewport_diagnostics()
	var configured_viewport := Vector2i(
		int(ProjectSettings.get_setting("display/window/size/viewport_width", 1024)),
		int(ProjectSettings.get_setting("display/window/size/viewport_height", 768))
	)
	var root_window := root as Window
	root_window.size = configured_viewport
	for property_info in root_window.get_property_list():
		if str(property_info.get("name", "")) == "content_scale_size":
			root_window.set("content_scale_size", configured_viewport)
			break
	await process_frame
	await process_frame
	report["viewport_setup"] = {
		"configured_size":_vector2i_array(configured_viewport),
		"before":initial_viewport,
		"after":_viewport_diagnostics()
	}
	if not require(root.get_viewport().size == configured_viewport, "focused input probe could not establish the configured 1024x768 viewport"):
		return
	if not require(not FileAccess.file_exists(SAVE_STATE.path()), "isolated native save slot must begin absent"):
		return
	if not require(FileAccess.file_exists(ORIGINAL_IMPORT_PATH), "original import fixture is missing"):
		return
	original_import_backup = FileAccess.get_file_as_string(ORIGINAL_IMPORT_PATH)
	var imported: Dictionary = SAVE_STATE.read_original_import()
	if not require(not original_import_backup.is_empty() and imported.get("status") == "loaded", "preserved original import fixture must be valid before the route test"):
		return
	report["fixture_before"] = {"present":true,"valid":true,"profile":imported.get("profile","")}
	var transitions := root.get_node_or_null("LevelTransition")
	if not require(transitions != null and transitions.has_signal("transition_failed"), "persistent LevelTransition failure signal is missing"):
		return
	transitions.connect("transition_failed", _on_transition_failed)
	var packed := load("res://game/bootstrap.tscn") as PackedScene
	var menu := packed.instantiate() as Control if packed != null else null
	if not require(menu != null, "could not instantiate the actual main menu scene"):
		return
	root.add_child(menu)
	current_scene = menu
	await process_frame
	var load_button := menu.find_child("LoadGame", true, false) as Button
	if not require(load_button != null and not load_button.disabled, "valid imported save did not enable the actual Load Game menu button"):
		return
	var import_file := FileAccess.open(ORIGINAL_IMPORT_PATH, FileAccess.WRITE)
	if not require(import_file != null, "could not modify the isolated staged import fixture"):
		return
	original_import_changed = true
	import_file.store_string("{ invalid original import\n")
	import_file.close()
	await click_gui(load_button)
	var failure: Dictionary = await wait_failure(30000)
	if not require(not failure.is_empty(), "corrupt staged import did not reach transition_failed"):
		return
	var presenter := _loading_screen()
	var dismiss := presenter.get_node_or_null("Viewport/Center/Content/Dismiss") as Button if presenter != null else null
	var visible_deadline := Time.get_ticks_msec() + 5000
	while (presenter == null or not presenter.visible or dismiss == null or not dismiss.visible) and Time.get_ticks_msec() < visible_deadline:
		await process_frame
		presenter = _loading_screen()
		dismiss = presenter.get_node_or_null("Viewport/Center/Content/Dismiss") as Button if presenter != null else null
	if not require(presenter != null and presenter.visible and dismiss != null and dismiss.visible, "actual Load Game failure did not leave its Dismiss button visible"):
		return
	dismiss.pressed.connect(_on_dismiss_pressed)
	var failure_session := current_scene
	report["failure_presented"] = {"transition":failure,"presenter_visible":presenter.visible,"dismiss_visible":dismiss.visible}
	report["dismiss_input_geometry"] = _input_diagnostics(presenter, dismiss)
	var legacy_center := dismiss.get_global_transform() * (dismiss.size * 0.5)
	var viewport_center := dismiss.get_global_transform_with_canvas() * (dismiss.size * 0.5)
	var legacy_hovered: String = await _send_mouse_motion(legacy_center)
	var viewport_hovered: String = await _send_mouse_motion(viewport_center)
	report["parsed_mouse_motion"] = {
		"legacy_canvas_position":_vector2_array(legacy_center),
		"legacy_hovered_control":legacy_hovered,
		"viewport_event_position":_vector2_array(viewport_center),
		"viewport_hovered_control":viewport_hovered,
		"dismiss_is_hovered":viewport_hovered == _control_path(dismiss)
	}
	await _send_mouse_click(viewport_center)
	var mouse_click_activated := dismiss_pressed
	var immediate_menu := current_scene if current_scene != null and current_scene.scene_file_path == "res://game/bootstrap.tscn" else null
	var immediate_status := immediate_menu.get_node_or_null("Status/Details") as Label if immediate_menu != null else null
	report["after_mouse_click"] = {
		"dismiss_pressed":mouse_click_activated,
		"presenter_visible":presenter.visible,
		"current_scene":current_scene.scene_file_path if current_scene != null else "",
		"session_loading_stage":str(failure_session.get("loading_stage")) if is_instance_valid(failure_session) and failure_session.has_method("get") else "",
		"session_ready_to_play":bool(failure_session.get("ready_to_play")) if is_instance_valid(failure_session) and failure_session.has_method("get") else false,
		"transition_busy":bool(transitions.call("is_transitioning")),
		"menu_status":immediate_status.text if immediate_status != null else ""
	}
	report["keyboard_ui_accept"] = {"attempted":false,"reason":"this follow-up isolates the mouse route; Enter was covered by the preserved prior probe"}
	var recovery_deadline := Time.get_ticks_msec() + 10000
	while (presenter.visible or current_scene == failure_session or current_scene == null or current_scene.scene_file_path != "res://game/bootstrap.tscn") and Time.get_ticks_msec() < recovery_deadline:
		await process_frame
	var recovered_menu := current_scene if current_scene != null and current_scene != failure_session and current_scene.scene_file_path == "res://game/bootstrap.tscn" else null
	var recovery_status := recovered_menu.get_node_or_null("Status/Details") as Label if recovered_menu != null else null
	var automatic_recovery_succeeded := recovered_menu != null
	report["after_dismiss"] = {
		"dismiss_pressed":dismiss_pressed,
		"presenter_visible":presenter.visible,
		"current_scene":current_scene.scene_file_path if current_scene != null else "",
		"failed_session_still_current":current_scene == failure_session,
		"failed_session_inside_tree":is_instance_valid(failure_session) and failure_session.is_inside_tree(),
		"transition_busy":bool(transitions.call("is_transitioning")),
		"recovered":automatic_recovery_succeeded,
		"menu_status":recovery_status.text if recovery_status != null else "",
		"automatic_recovery_after_mouse_click":automatic_recovery_succeeded,
		"checked_diagnostic_scene_change":{"attempted":false,"reason":"no fallback scene change; this probe measures automatic recovery only"},
		"elapsed_after_click_ms":10000 - max(0, recovery_deadline - Time.get_ticks_msec())
	}
	report["status"] = "diagnostic_complete"
	await shutdown(0)

func shutdown(exit_code: int) -> void:
	if original_import_changed:
		var restore_file := FileAccess.open(ORIGINAL_IMPORT_PATH, FileAccess.WRITE)
		if restore_file != null:
			restore_file.store_string(original_import_backup)
			restore_file.flush()
			restore_file.close()
			original_import_changed = false
	report["fixture_restored"] = FileAccess.get_file_as_string(ORIGINAL_IMPORT_PATH) == original_import_backup
	report["restored_import_status"] = SAVE_STATE.read_original_import().get("status", "missing")
	write_report()
	current_scene = null
	for child in root.get_children():
		child.queue_free()
	for frame in range(3):
		await process_frame
	quit(exit_code)
