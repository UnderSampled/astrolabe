extends SceneTree
## Synthetic native scenes exercise the production service, including a loader
## held on its own thread so cancellation is checked before the request drains.
const SERVICE = preload("res://game/level_transition.gd")
const BLOCKING_RESOURCE = preload("res://blocking_resource.gd")

class Session extends Node:
	signal install_released
	var installing := false
	var install_calls := 0
	var successful := true
	var aborts := 0

	func open_level_from_transition(_packed: PackedScene, _path: String) -> Dictionary:
		install_calls += 1
		installing = true
		await install_released
		installing = false
		return {"ok": successful, "message": "Synthetic installation failure."}

	func begin_level_transition() -> bool:
		return true

	func install_level_transition(destination: Node3D, _entry: String, _camera: String, _packed: PackedScene) -> Dictionary:
		destination.free()
		return await open_level_from_transition(null, "")

	func abort_level_transition() -> void:
		aborts += 1

var service: Node
var session: Session
var terminals: Array[String] = []
var expected_limit := 0

func _initialize() -> void:
	call_deferred("run")

func check(condition: bool, message: String) -> void:
	assert(condition, message)
	if not condition:
		push_error(message)
		quit(1)

func wait_until(predicate: Callable) -> void:
	var deadline := Time.get_ticks_msec() + 2000
	while not predicate.call() and Time.get_ticks_msec() < deadline:
		await process_frame
	check(bool(predicate.call()), "Loading-cap fixture exceeded its deadline.")

func terminal(kind: String) -> void:
	check(Engine.max_fps == expected_limit, "Frame limit was not restored before " + kind)
	terminals.append(kind)

func run() -> void:
	var original_limit := Engine.max_fps
	session = Session.new()
	root.add_child(session)
	current_scene = session
	service = SERVICE.new()
	root.add_child(service)
	service.transition_started.connect(func(_from: String, _to: String, _entry: String):
		check(Engine.max_fps == (30 if expected_limit == 0 or expected_limit > 30 else expected_limit), "Loading cap was not acquired before start."))
	service.transition_completed.connect(func(_level: String, _entry: String): terminal("completed"))
	service.transition_failed.connect(func(_level: String, _message: String): terminal("failed"))
	service.transition_cancelled.connect(func(_level: String): terminal("cancelled"))

	# Initial and map-change callers share the cap through awaited installation.
	for cap in [0, 120, 20, 30]:
		expected_limit = cap
		Engine.max_fps = cap
		var before := terminals.size()
		check(service.request_initial_level("cap" + str(cap)), "Initial load was rejected.")
		await wait_until(func(): return session.installing)
		check(Engine.max_fps == (30 if cap == 0 or cap > 30 else cap), "Limit escaped during installation.")
		check(not service.cancel_current_transition(), "Installing resource was incorrectly cancellable.")
		session.install_released.emit()
		await wait_until(func(): return not service.is_transitioning())
		check(terminals.size() == before + 1 and terminals.back() == "completed", "Completion was not published exactly once.")

	expected_limit = 90
	Engine.max_fps = expected_limit
	session.successful = false
	check(service.request_level_change("installfail"), "Map change was rejected.")
	await wait_until(func(): return session.installing)
	check(Engine.max_fps == 30, "Map change bypassed loading limit.")
	session.install_released.emit()
	await wait_until(func(): return not service.is_transitioning())
	check(terminals.back() == "failed" and session.aborts == 1, "Failed installation did not resume the session.")
	session.successful = true

	# A native scene with the wrong root uses terminal cleanup without installing.
	expected_limit = 80
	Engine.max_fps = expected_limit
	check(service.request_level_change("invalid"), "Invalid-root load was rejected before reporting.")
	await wait_until(func(): return not service.is_transitioning())
	check(terminals.back() == "failed", "Invalid resource did not report failure.")

	# The native Resource constructor signals from the loader thread, then waits.
	expected_limit = 100
	Engine.max_fps = expected_limit
	BLOCKING_RESOURCE.block_next = true
	var installed_before := session.install_calls
	check(service.request_level_change("cancel"), "Cancellation load was rejected.")
	var entered := [false]
	await wait_until(func():
		if not entered[0]:
			entered[0] = BLOCKING_RESOURCE.entered.try_wait()
		return entered[0])
	check(service.cancel_current_transition(), "In-flight cancellation was rejected.")
	check(Engine.max_fps == expected_limit and service.is_transitioning(), "Cancellation did not restore while retaining the draining slot.")
	check(not service.cancel_current_transition(), "Duplicate cancellation was accepted.")
	check(not service.request_initial_level("cap0"), "Draining request slot was reused.")
	BLOCKING_RESOURCE.release.post()
	await wait_until(func(): return not service.is_transitioning())
	check(session.install_calls == installed_before and session.aborts == 3, "Cancelled resource installed or did not resume the session.")
	check(Engine.max_fps == expected_limit, "Draining cleanup changed the restored cap.")

	# Exiting during asynchronous installation restores the prior frame limit.
	expected_limit = 144
	Engine.max_fps = expected_limit
	check(service.request_initial_level("exit"), "Exit fixture load was rejected.")
	await wait_until(func(): return session.installing)
	root.remove_child(service)
	check(Engine.max_fps == expected_limit, "Service exit leaked its frame limit.")
	session.install_released.emit()
	service.free()
	Engine.max_fps = original_limit
	print("LEVEL_LOADING_CAP_OK initial=true transition=true lower_caps=true failure=true cancellation_draining=true exit=true")
	quit(0)
