extends Node
## Exported-build smoke check: ASTROLABE_SMOKE=new starts a New Game from the
## main menu and exits 0 once the Manoir is playable, or 1 after a minute.
const TIMEOUT_MS := 60000
var _started := 0

func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	_started = Time.get_ticks_msec()

func _process(_delta: float) -> void:
	var scene := get_tree().current_scene
	if scene != null and bool(scene.get("ready_to_play")):
		print("SMOKE playable ", Time.get_ticks_msec() - _started, " ms in ", scene.get("level_path"))
		get_tree().quit(0)
	elif Time.get_ticks_msec() - _started > TIMEOUT_MS:
		print("SMOKE stalled after ", TIMEOUT_MS, " ms, scene ", scene.name if scene != null else "none")
		get_tree().quit(1)
