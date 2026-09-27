extends Node
## Keep one immutable converted level resource across menu and reload transitions.
## Every session still instantiates its own nodes, physics state, and Brain.

var level_path := ""
var level_scene: PackedScene

func get_level(path: String) -> PackedScene:
    return level_scene if path == level_path else null

func retain_level(path: String, packed: PackedScene) -> void:
    level_path = path
    level_scene = packed

func clear() -> void:
    level_path = ""
    level_scene = null
