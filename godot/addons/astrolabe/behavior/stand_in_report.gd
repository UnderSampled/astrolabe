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
## With strict mode off, play continues and each key is warned about once.

const SETTING := "astrolabe/debug/strict_stand_ins"
const EXIT_CODE := 70

## id + " " + context -> {"id", "context", "count"}.
static var reached_keys: Dictionary = {}
static var halted := false
static var _mode := ""
static var _allowed: PackedStringArray = []

static func mode() -> String:
	if _mode.is_empty():
		var value := OS.get_environment("ASTROLABE_STRICT_STAND_INS")
		if value.is_empty():
			value = "1" if bool(ProjectSettings.get_setting(SETTING, false)) else "0"
		_mode = "exit" if value == "exit" else ("stop" if value in ["1", "true", "stop"] else "off")
		_allowed = OS.get_environment("ASTROLABE_STRICT_ALLOW").split(",", false)
	return _mode

static func strict() -> bool:
	return mode() != "off"

## Record a stand-in reached by `node` (an actor, its Brain or any node beneath
## it). Returns true when strict mode halted the game; the caller should return.
static func reached(node: Node, id: String, context: String) -> bool:
	var key := id + " " + context
	if not reached_keys.has(key):
		reached_keys[key] = {"id": id, "context": context, "count": 0}
		push_warning("Stand-in reached: " + key)
	reached_keys[key]["count"] += 1
	if not strict() or id in _allowed: return false
	_halt(node, id, context)
	return true

static func _halt(node: Node, id: String, context: String) -> void:
	if halted: return
	halted = true
	var site := ""
	for frame in get_stack():
		var source := str(frame.get("source", ""))
		if source.ends_with("stand_in_report.gd") or source.ends_with("behavior/behavior.gd"): continue
		site = "%s:%d %s" % [source, int(frame.get("line", 0)), str(frame.get("function", ""))]
		break
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
