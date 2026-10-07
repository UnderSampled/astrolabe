extends Node
## Keep immutable converted level resources across transitions. Every session
## still instantiates its own nodes, physics state, and Brain.

var level_scenes: Dictionary = {}

func get_level(path: String) -> PackedScene:
	return level_scenes.get(path) as PackedScene

func retain_level(path: String, packed: PackedScene) -> void:
	level_scenes[path] = packed

func clear() -> void:
	level_scenes.clear()
