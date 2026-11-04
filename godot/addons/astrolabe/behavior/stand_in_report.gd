extends RefCounted
## Records every stand-in or abandoned guard reached during play, compiled or
## hand-written (see docs/astrolabe/stand-ins.md), and implements strict mode.
##
## Strict mode (project setting astrolabe/debug/strict_stand_ins, or the
## ASTROLABE_STRICT_STAND_INS environment variable) stops the game at the first
## stand-in reached: it logs a STRICT_STAND_IN line with the id, context, script
## site and actor, pauses the tree and shows the same on screen. The variable's
## value "exit" quits with code 70 instead, for unattended runs.
## ASTROLABE_STRICT_ALLOW lists ids (comma-separated) that strict mode lets pass.
## The value "collect" stops nothing: it records every site reached (id, context,
## script line, script, model, actor, first-hit time, count) and writes them to
## ASTROLABE_STAND_IN_REPORT (default user://stand-ins-collected.json) as they
## appear and at exit. With strict mode off, play continues and each key is
## warned about once.

const SETTING := "astrolabe/debug/strict_stand_ins"
const EXIT_CODE := 70

## id + " " + context -> {"id", "context", "count"}.
static var reached_keys: Dictionary = {}
static var halted := false
static var _mode := ""
static var _allowed: PackedStringArray = []
## Collect mode: id + context + site -> site record; written by _write_collected.
static var collected: Dictionary = {}
static var _collect_written_msec := -1
static var _collect_exit_hooked := false

static func mode() -> String:
	if _mode.is_empty():
		var value := OS.get_environment("ASTROLABE_STRICT_STAND_INS")
		if value.is_empty():
			value = "1" if bool(ProjectSettings.get_setting(SETTING, false)) else "0"
		_mode = value if value in ["exit", "collect"] else ("stop" if value in ["1", "true", "stop"] else "off")
		_allowed = OS.get_environment("ASTROLABE_STRICT_ALLOW").split(",", false)
	return _mode

## A harness's own quit code, unless strict exit mode already halted the game:
## then 70, so a later quit does not hide the stand-in.
static func exit_code(code: int) -> int:
	if mode() == "collect": _write_collected()
	return EXIT_CODE if halted and mode() == "exit" else code

static func strict() -> bool:
	return mode() in ["stop", "exit"]

## Record a stand-in reached by `node` (an actor, its Brain or any node beneath
## it). Returns true when strict mode halted the game; the caller should return.
static func reached(node: Node, id: String, context: String) -> bool:
	var key := id + " " + context
	if not reached_keys.has(key):
		reached_keys[key] = {"id": id, "context": context, "count": 0}
		push_warning("Stand-in reached: " + key)
	reached_keys[key]["count"] += 1
	if mode() == "collect":
		_collect(node, id, context)
		return false
	if not strict() or id in _allowed: return false
	_halt(node, id, context)
	return true

## The first script frame outside this file and behavior.gd: the stand-in's site.
static func _site() -> String:
	for frame in get_stack():
		var source := str(frame.get("source", ""))
		if source.ends_with("stand_in_report.gd") or source.ends_with("behavior/behavior.gd"): continue
		return "%s:%d %s" % [source, int(frame.get("line", 0)), str(frame.get("function", ""))]
	return ""

static func _collect(node: Node, id: String, context: String) -> void:
	var site := _site()
	var key := id + " " + context + " " + site
	var now := Time.get_ticks_msec()
	var row: Dictionary = collected.get(key, {})
	if row.is_empty():
		var script := _script_of(node)
		var tree := Engine.get_main_loop() as SceneTree
		var scene := tree.current_scene.scene_file_path if tree != null and tree.current_scene != null else ""
		row = {"id": id, "context": context, "site": site, "script": script, "model": _model_of(script, site),
			"actor": _actor_of(node), "actors": 0, "scene": scene, "first_msec": now, "last_msec": now, "count": 0}
		row["_actors"] = {}
		collected[key] = row
		_collect_written_msec = -1
	if not _collect_exit_hooked and Engine.get_main_loop() is SceneTree:
		_collect_exit_hooked = true
		var watcher := ExitWriter.new()
		watcher.name = "StandInCollectWriter"
		watcher.write = _write_collected
		(Engine.get_main_loop() as SceneTree).root.add_child.call_deferred(watcher)
	row["count"] += 1
	row["last_msec"] = now
	var actors: Dictionary = row["_actors"]
	if actors.size() < 64: actors[_actor_of(node)] = true
	row["actors"] = actors.size()
	if _collect_written_msec < 0 or now - _collect_written_msec >= 2000:
		_write_collected()

## The compiled AI model a script or site belongs to: "level:/816", "fix:/437".
static func _model_of(script: String, site: String) -> String:
	for path: String in [script, site]:
		var at := path.find("/ai/models/")
		if at < 0: continue
		var tail := path.substr(at + 11).get_slice(".gd", 0)
		return ("fix:/" + tail.trim_prefix("fix/")) if tail.begins_with("fix/") else ("level:/" + tail)
	return ""

static func _write_collected() -> void:
	_collect_written_msec = Time.get_ticks_msec()
	var rows: Array = []
	for row: Dictionary in collected.values():
		var copy := row.duplicate()
		copy.erase("_actors")
		rows.append(copy)
	rows.sort_custom(func(a, b): return a["first_msec"] < b["first_msec"])
	var path := OS.get_environment("ASTROLABE_STAND_IN_REPORT")
	if path.is_empty(): path = "user://stand-ins-collected.json"
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		push_error("Stand-in collect report not written: " + path)
		return
	file.store_string(JSON.stringify({"written_msec": _collect_written_msec, "sites": rows}, "\t") + "\n")

static func _halt(node: Node, id: String, context: String) -> void:
	if halted: return
	halted = true
	var site := _site()
	var report := {"id": id, "context": context, "site": site, "script": _script_of(node), "actor": _actor_of(node)}
	var text := "STRICT_STAND_IN " + JSON.stringify(report)
	print(text)
	push_error(text)
	var tree := node.get_tree() if node != null and node.is_inside_tree() else Engine.get_main_loop() as SceneTree
	if tree == null: return
	if mode() == "exit":
		tree.quit(EXIT_CODE)
		return
	tree.paused = true
	var layer := CanvasLayer.new()
	layer.layer = 128
	layer.process_mode = Node.PROCESS_MODE_ALWAYS
	var label := Label.new()
	label.text = "Strict stand-in mode: the game stopped.\n\nStand-in: %s\nContext: %s\nSite: %s\nScript: %s\nActor: %s" % [id, context, site, report["script"], report["actor"]]
	label.autowrap_mode = TextServer.AUTOWRAP_WORD
	label.position = Vector2(24, 24)
	label.size = Vector2(976, 720)
	label.add_theme_color_override("font_color", Color(1, 0.85, 0.4))
	label.add_theme_color_override("font_outline_color", Color.BLACK)
	label.add_theme_constant_override("outline_size", 6)
	layer.add_child(label)
	tree.root.add_child.call_deferred(layer)

static func _script_of(node: Node) -> String:
	if node == null: return ""
	var script: Script = node.get_script()
	return script.resource_path if script != null else node.get_class()

static func _actor_of(node: Node) -> String:
	var current := node
	while current != null:
		if current is CharacterBody3D or current.has_method("source_contact_sector"):
			return str(current.get_path()) if current.is_inside_tree() else str(current.name)
		current = current.get_parent()
	return str(node.get_path()) if node != null and node.is_inside_tree() else ""

## Writes the collect report when the game quits.
class ExitWriter extends Node:
	var write: Callable

	func _init() -> void:
		process_mode = Node.PROCESS_MODE_ALWAYS

	func _notification(what: int) -> void:
		if what in [NOTIFICATION_WM_CLOSE_REQUEST, NOTIFICATION_EXIT_TREE, NOTIFICATION_PREDELETE]:
			write.call()
