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
	await click_gui(dismiss)
	var immediate_menu := current_scene if current_scene != null and current_scene.scene_file_path == "res://game/bootstrap.tscn" else null
	var immediate_status := immediate_menu.get_node_or_null("Status/Details") as Label if immediate_menu != null else null
	report["after_dismiss_immediate"] = {
		"dismiss_pressed":dismiss_pressed,
		"presenter_visible":presenter.visible,
		"current_scene":current_scene.scene_file_path if current_scene != null else "",
		"session_loading_stage":str(failure_session.get("loading_stage")) if is_instance_valid(failure_session) and failure_session.has_method("get") else "",
		"session_ready_to_play":bool(failure_session.get("ready_to_play")) if is_instance_valid(failure_session) and failure_session.has_method("get") else false,
		"transition_busy":bool(transitions.call("is_transitioning")),
		"menu_status":immediate_status.text if immediate_status != null else ""
	}
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
	if not require(dismiss_pressed, "parsed GUI input did not press the visible Dismiss button"):
		return
	if not require(not presenter.visible, "Dismiss pressed but the loading error presenter remained visible"):
		return
	if not require(automatic_recovery_succeeded, "dismissing the Load Game error did not return to the bootstrap menu"):
		return
	if not require(recovery_status != null and recovery_status.text.contains("The game could not be opened"), "recovered menu did not show the initial route error"):
		return
	report["status"] = "passed"
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
