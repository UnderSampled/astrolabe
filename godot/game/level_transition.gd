extends Node
## Persistent owner for loading an independently exported level and handing it
## to the current game session. AI scripts only submit semantic map names.

signal transition_started(from_level: String, to_level: String, entry: String)
signal transition_progress(to_level: String, progress: float, phase: String)
signal transition_completed(level: String, entry: String)
signal transition_failed(level: String, message: String)
signal transition_cancelled(level: String)

const LEVEL_PREFIX := "res://generated/hype/levels/"
const PRESENTER := "res://addons/astrolabe/ui/loading/screen.tscn"

var _busy := false
var _cancel_requested := false
var _terminal_sent := false
var _active_path := ""
var _active_level := ""
var _active_entry := ""
var _session: Node
var _initial_request := false
var _session_transition_started := false
var _load_retried := false
var _resource_loading := false
var _saved_loading_max_fps := -1

func _ready() -> void:
	# The loading screen is optional in older generated projects and is owned by
	# this persistent node when the presenter package has been installed.
	if not has_node("LoadingScreen") and ResourceLoader.exists(PRESENTER):
		var presenter_scene := load(PRESENTER) as PackedScene
		var presenter := presenter_scene.instantiate() if presenter_scene != null else null
		if presenter != null:
			presenter.name = "LoadingScreen"
			add_child(presenter)

func is_transitioning() -> bool:
	return _busy

func request_level_change(level: String, entry: String = "", camera_entry: String = "") -> bool:
	if _busy:
		return false
	var normalized := _source_name(level).to_lower()
	if not _valid_level_name(normalized):
		return false
	var session := get_tree().current_scene
	if not is_instance_valid(session) or not session.has_method("begin_level_transition"):
		return false
	# Source map-change helpers compare the map case-insensitively before writing
	# entry state or entering loading. A same-map request continues the AI.
	if _current_level(session).to_lower() == normalized:
		return true
	var path := LEVEL_PREFIX + normalized + "/level.tscn"
	if not ResourceLoader.exists(path, "PackedScene"):
		# Reaching a level exit ends the converted game rather than stranding Hype.
		return session.has_method("source_level_not_converted") and bool(session.call("source_level_not_converted", normalized))
	if not bool(session.call("begin_level_transition")):
		return false
	return _start_request(session, normalized, _source_name(entry), _source_name(camera_entry), path, false, true)

func request_initial_level(level: String) -> bool:
	if _busy:
		return false
	var normalized := level.strip_edges().to_lower()
	if not _valid_level_name(normalized):
		return false
	var path := LEVEL_PREFIX + normalized + "/level.tscn"
	if not ResourceLoader.exists(path, "PackedScene"):
		return false
	var session := get_tree().current_scene
	if not is_instance_valid(session) or not session.has_method("open_level_from_transition"):
		return false
	return _start_request(session, normalized, "", "", path, true, false)

func _start_request(session: Node, normalized: String, entry: String, camera_entry: String, path: String, initial: bool, session_transition_started: bool) -> bool:
	_session = session
	_active_level = normalized
	_active_entry = entry
	_active_path = path
	_busy = true
	_cancel_requested = false
	_terminal_sent = false
	_initial_request = initial
	_session_transition_started = session_transition_started
	_resource_loading = true
	_limit_loading_frames()
	transition_started.emit(_current_level(session), normalized, _active_entry)
	var request_error := ResourceLoader.load_threaded_request(path, "PackedScene", false, ResourceLoader.CACHE_MODE_REUSE)
	if request_error != OK:
		_fail("The destination scene could not begin loading.")
		_clear_request()
		return true
	_load_and_install(normalized, _active_entry, camera_entry, path, session, initial)
	return true

func cancel_current_transition() -> bool:
	if not _busy or _terminal_sent or not _resource_loading:
		return false
	_cancel_requested = true
	_restore_loading_frames()
	_terminal_sent = true
	transition_cancelled.emit(_active_level)
	# Godot does not expose cancellation for ResourceLoader's threaded request.
	# Keep the request slot reserved and drain it; the active session resumes now.
	if _session_transition_started and is_instance_valid(_session):
		_session.call("abort_level_transition")
	return true

func _load_and_install(level: String, entry: String, camera_entry: String, path: String, session: Node, initial: bool) -> void:
	var last_progress := -1.0
	while is_inside_tree():
		var fractions: Array = []
		var status := ResourceLoader.load_threaded_get_status(path, fractions)
		if not fractions.is_empty():
			var observed := clampf(float(fractions[0]), 0.0, 1.0)
			if not is_equal_approx(observed, last_progress) and not _terminal_sent:
				last_progress = observed
				transition_progress.emit(level, observed, "resource_loading")
		if status == ResourceLoader.THREAD_LOAD_FAILED and not _load_retried and not _cancel_requested:
			# Threaded script preloads occasionally fail to parse, and the same
			# files load on a second try. Replace the failed cache entries.
			_load_retried = true
			push_warning("Retrying the level load after a threaded load failure.")
			if ResourceLoader.load_threaded_request(path, "PackedScene", false, ResourceLoader.CACHE_MODE_REPLACE_DEEP) == OK:
				await get_tree().process_frame
				continue
		if status != ResourceLoader.THREAD_LOAD_IN_PROGRESS:
			_resource_loading = false
			if status == ResourceLoader.THREAD_LOAD_LOADED:
				var packed := ResourceLoader.load_threaded_get(path) as PackedScene
				if _cancel_requested:
					packed = null
				elif packed == null:
					_fail("The destination scene resource is invalid.")
				elif not is_instance_valid(session) or get_tree().current_scene != session:
					_fail("The game session ended before the transition could commit.")
				elif initial:
					var result: Dictionary = await session.call("open_level_from_transition", packed, path)
					if result.get("ok", false):
						if not _terminal_sent:
							_terminal_sent = true
							_restore_loading_frames()
							transition_completed.emit(level, "")
					else:
						_fail(str(result.get("message", "The initial level could not be opened.")))
				else:
					var instantiated := packed.instantiate()
					var destination := instantiated as Node3D
					if destination == null:
						if is_instance_valid(instantiated):
							instantiated.free()
						_fail("The destination scene root is not a 3D level.")
					else:
						var result: Dictionary = await session.call("install_level_transition", destination, entry, camera_entry, packed)
						if result.get("ok", false):
							if not _terminal_sent:
								_terminal_sent = true
								_restore_loading_frames()
								transition_completed.emit(level, entry)
						else:
							if is_instance_valid(destination) and not destination.is_queued_for_deletion():
								if destination.is_inside_tree(): destination.queue_free()
								else: destination.free()
							_fail(str(result.get("message", "The destination level could not be installed.")))
			else:
				if not _cancel_requested:
					_fail("The destination scene failed to load.")
			break
		await get_tree().process_frame
	if _busy:
		_clear_request()

func _fail(message: String) -> void:
	_restore_loading_frames()
	if _session_transition_started and is_instance_valid(_session):
		_session.call("abort_level_transition")
	if not _terminal_sent:
		_terminal_sent = true
		transition_failed.emit(_active_level, message)

func _clear_request() -> void:
	_restore_loading_frames()
	_busy = false
	_cancel_requested = false
	_terminal_sent = false
	_active_path = ""
	_active_level = ""
	_active_entry = ""
	_session = null
	_initial_request = false
	_session_transition_started = false
	_resource_loading = false
	_load_retried = false

func _current_level(session: Node) -> String:
	var current: Variant = session.get("level")
	if current is Node and is_instance_valid(current):
		return str(current.get_meta("source_level", ""))
	return ""

func _valid_level_name(level: String) -> bool:
	if level.is_empty() or level.length() > 64:
		return false
	for index in level.length():
		var code := level.unicode_at(index)
		if not ((code >= 97 and code <= 122) or (code >= 48 and code <= 57) or code == 45 or code == 95 or code == 126):
			return false
	return true

## Source string literals can carry a single trailing '~'. Proc5 strips it
## from the map; Proc13 strips it independently from map and both actor names.
func _source_name(value: String) -> String:
	return value.left(value.length() - 1) if value.ends_with("~") else value

# Measured startup mitigation: the actual service route bypasses the session's
# fallback cap. Limit rendering only while loading/installing, without changing
# a lower user limit or claiming a particular loader contention mechanism.
func _limit_loading_frames() -> void:
	if _saved_loading_max_fps < 0 and (Engine.max_fps == 0 or Engine.max_fps > 30):
		_saved_loading_max_fps = Engine.max_fps
		Engine.max_fps = 30

func _restore_loading_frames() -> void:
	if _saved_loading_max_fps >= 0:
		Engine.max_fps = _saved_loading_max_fps
		_saved_loading_max_fps = -1

func _exit_tree() -> void:
	_restore_loading_frames()
