extends SceneTree
## Isolated, GUI-driven check of the initial Load Game error-dismissal route.

const ORIGINAL_IMPORT_PATH := "res://game/original_save_import.json"
const SAVE_STATE = preload("res://game/save_state.gd")

var evidence_path := ""
var original_import_backup := ""
var original_import_changed := false
var transition_failure: Dictionary = {}
var dismiss_pressed := false
var keyboard_accept_activated := false
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
	var focus_owner := viewport.gui_get_focus_owner()
	var rect := dismiss.get_global_rect()
	return {
		"dismiss": {
			"global_rect":{"position":[rect.position.x,rect.position.y],"size":[rect.size.x,rect.size.y]},
			"size":[dismiss.size.x,dismiss.size.y],
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

func _send_ui_accept() -> Dictionary:
	var press := InputEventKey.new()
	press.keycode = KEY_ENTER
	press.physical_keycode = KEY_ENTER
	press.pressed = true
	var mapped := InputMap.has_action("ui_accept") and InputMap.action_has_event("ui_accept", press)
	Input.parse_input_event(press)
	Input.flush_buffered_events()
	await process_frame
	var action_pressed := Input.is_action_pressed("ui_accept") if InputMap.has_action("ui_accept") else false
	var accepted := dismiss_pressed
	var release := InputEventKey.new()
	release.keycode = KEY_ENTER
	release.physical_keycode = KEY_ENTER
	release.pressed = false
	Input.parse_input_event(release)
	Input.flush_buffered_events()
	await process_frame
	return {"attempted":true,"input":"Enter mapped to ui_accept","enter_mapped":mapped,"ui_accept_pressed":action_pressed,"dismiss_pressed_signal":accepted}

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
	var dismiss_rect := dismiss.get_global_rect()
	var hovered_path: String = await _send_mouse_motion(dismiss_rect.get_center())
	report["parsed_mouse_motion"] = {"position":[dismiss_rect.get_center().x,dismiss_rect.get_center().y],"hovered_control":hovered_path}
	await _send_mouse_click(dismiss_rect.get_center())
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
	var keyboard_observation := {"attempted":false,"reason":"mouse click already activated Dismiss"}
	if not mouse_click_activated:
		dismiss_pressed = false
		keyboard_observation = await _send_ui_accept()
		keyboard_accept_activated = bool(keyboard_observation.get("dismiss_pressed_signal", false))
		keyboard_observation["presenter_visible_after_input"] = presenter.visible
		keyboard_observation["current_scene_after_input"] = current_scene.scene_file_path if current_scene != null else ""
		keyboard_observation["session_loading_stage_after_input"] = str(failure_session.get("loading_stage")) if is_instance_valid(failure_session) and failure_session.has_method("get") else ""
	else:
		keyboard_observation["attempted"] = false
		report["keyboard_cycle_required"] = true
	report["keyboard_ui_accept"] = keyboard_observation
	var recovery_deadline := Time.get_ticks_msec() + 10000
	while (presenter.visible or current_scene == failure_session or current_scene == null or current_scene.scene_file_path != "res://game/bootstrap.tscn") and Time.get_ticks_msec() < recovery_deadline:
		await process_frame
	var recovered_menu := current_scene if current_scene != null and current_scene != failure_session and current_scene.scene_file_path == "res://game/bootstrap.tscn" else null
	var recovery_status := recovered_menu.get_node_or_null("Status/Details") as Label if recovered_menu != null else null
	var automatic_recovery_succeeded := recovered_menu != null
	var diagnostic_scene_change := {"attempted":false,"error_code":null,"error_message":"not available; production recovery currently ignores change_scene_to_file's return value"}
	if not automatic_recovery_succeeded:
		var checked_error: Error = change_scene_to_file("res://game/bootstrap.tscn")
		diagnostic_scene_change = {"attempted":true,"error_code":int(checked_error),"error_message":error_string(checked_error)}
		if checked_error == OK:
			var diagnostic_menu: Node = await wait_menu(failure_session, 5000)
			var diagnostic_status := diagnostic_menu.get_node_or_null("Status/Details") as Label if diagnostic_menu != null else null
			diagnostic_scene_change["menu_recovered"] = diagnostic_menu != null
			diagnostic_scene_change["menu_status"] = diagnostic_status.text if diagnostic_status != null else ""
	report["after_dismiss"] = {
		"dismiss_pressed":dismiss_pressed,
		"presenter_visible":presenter.visible,
		"current_scene":current_scene.scene_file_path if current_scene != null else "",
		"failed_session_still_current":current_scene == failure_session,
		"failed_session_inside_tree":is_instance_valid(failure_session) and failure_session.is_inside_tree(),
		"transition_busy":bool(transitions.call("is_transitioning")),
		"recovered":automatic_recovery_succeeded,
		"menu_status":recovery_status.text if recovery_status != null else "",
		"production_scene_change_error":"not captured: production recovery currently ignores change_scene_to_file's return value",
		"checked_diagnostic_scene_change":diagnostic_scene_change,
		"elapsed_after_click_ms":10000 - max(0, recovery_deadline - Time.get_ticks_msec())
	}
	if mouse_click_activated and automatic_recovery_succeeded:
		# If the mouse already closed the presenter, start a second actual error
		# route after restoring and reloading the menu so ui_accept is still tested.
		var restore_for_key := FileAccess.open(ORIGINAL_IMPORT_PATH, FileAccess.WRITE)
		if restore_for_key != null:
			restore_for_key.store_string(original_import_backup)
			restore_for_key.flush()
			restore_for_key.close()
			original_import_changed = false
		var menu_reload_error: Error = change_scene_to_file("res://game/bootstrap.tscn")
		var key_menu: Node = await wait_menu(recovered_menu, 5000) if menu_reload_error == OK else null
		report["keyboard_cycle_menu_reload"] = {"error_code":int(menu_reload_error),"error_message":error_string(menu_reload_error),"menu_recovered":key_menu != null}
		if key_menu != null:
			var key_load_button := key_menu.find_child("LoadGame", true, false) as Button
			if key_load_button != null and not key_load_button.disabled:
				var corrupt_again := FileAccess.open(ORIGINAL_IMPORT_PATH, FileAccess.WRITE)
				if corrupt_again != null:
					original_import_changed = true
					corrupt_again.store_string("{ invalid original import\n")
					corrupt_again.close()
					transition_failure = {}
					dismiss_pressed = false
					await click_gui(key_load_button)
					var key_failure: Dictionary = await wait_failure(30000)
					var key_presenter := _loading_screen()
					var key_dismiss := key_presenter.get_node_or_null("Viewport/Center/Content/Dismiss") as Button if key_presenter != null else null
					var key_visible_deadline := Time.get_ticks_msec() + 5000
					while (key_presenter == null or not key_presenter.visible or key_dismiss == null or not key_dismiss.visible) and Time.get_ticks_msec() < key_visible_deadline:
						await process_frame
						key_presenter = _loading_screen()
						key_dismiss = key_presenter.get_node_or_null("Viewport/Center/Content/Dismiss") as Button if key_presenter != null else null
					if key_failure.size() > 0 and key_dismiss != null and key_dismiss.visible:
						var key_failure_session := current_scene
						keyboard_observation["second_route_failure"] = key_failure
						keyboard_observation["second_route_geometry"] = _input_diagnostics(key_presenter, key_dismiss)
						dismiss_pressed = false
						var second_keyboard_result := await _send_ui_accept()
						keyboard_accept_activated = bool(second_keyboard_result.get("dismiss_pressed_signal", false))
						second_keyboard_result["presenter_visible_after_input"] = key_presenter.visible
						second_keyboard_result["current_scene_after_input"] = current_scene.scene_file_path if current_scene != null else ""
						var key_recovery_deadline := Time.get_ticks_msec() + 10000
						while (key_presenter.visible or current_scene == key_failure_session or current_scene == null or current_scene.scene_file_path != "res://game/bootstrap.tscn") and Time.get_ticks_msec() < key_recovery_deadline:
							await process_frame
						second_keyboard_result["automatic_recovery"] = current_scene != null and current_scene != key_failure_session and current_scene.scene_file_path == "res://game/bootstrap.tscn"
						second_keyboard_result["presenter_visible_after_wait"] = key_presenter.visible
						second_keyboard_result["current_scene_after_wait"] = current_scene.scene_file_path if current_scene != null else ""
						keyboard_observation["second_route_ui_accept"] = second_keyboard_result
		if not original_import_changed:
			var corrupt_fixture := FileAccess.open(ORIGINAL_IMPORT_PATH, FileAccess.WRITE)
			if corrupt_fixture != null:
				original_import_changed = true
				corrupt_fixture.store_string("{ invalid original import\n")
				corrupt_fixture.close()
	report["keyboard_ui_accept"] = keyboard_observation
	report["after_mouse_click"]["keyboard_route_tested"] = keyboard_accept_activated
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
