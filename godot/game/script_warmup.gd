extends RefCounted
## Compiles a level's generated scripts on a dedicated thread before its
## threaded scene load.
##
## In Godot 4.7, a load requested from a WorkerThreadPool task runs as a new
## pool task. Compiling a script inside the threaded scene load therefore
## resolves its preloads on other pool threads while GDScriptCache holds its
## mutex, and two such compiles can wait on each other for good: the level
## load stalls (route runs under CPU load: progress frozen for minutes).
## Loads from a plain Thread run inline on that thread, so compiling here is
## single-threaded; the scene load then finds every script already cached.
const ROOTS := ["res://generated/hype/shared/"]

var done := 0
var total := 0
var _thread: Thread
## GDScriptCache does not own compiled scripts: keep them alive until the
## scene load has taken its own references.
var _scripts: Array[Resource] = []

func start(level_path: String) -> void:
	var paths := PackedStringArray()
	_collect(level_path.get_base_dir() + "/", paths)
	for root in ROOTS: _collect(root, paths)
	total = paths.size()
	_thread = Thread.new()
	_thread.start(_compile.bind(paths))

func finished() -> bool:
	return _thread == null or not _thread.is_alive()

## Joins the thread; call once finished() is true.
func complete() -> void:
	if _thread != null:
		_thread.wait_to_finish()
		_thread = null

func release() -> void:
	_scripts.clear()

func _compile(paths: PackedStringArray) -> void:
	var compiled: Array[Resource] = []
	for path in paths:
		var script := ResourceLoader.load(path, "", ResourceLoader.CACHE_MODE_REUSE)
		if script != null: compiled.append(script)
		done += 1
	_scripts = compiled

static func _collect(directory: String, paths: PackedStringArray) -> void:
	for file in DirAccess.get_files_at(directory):
		if file.ends_with(".gd"): paths.append(directory + file)
	for child in DirAccess.get_directories_at(directory):
		_collect(directory + child + "/", paths)
