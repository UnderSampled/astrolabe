extends Node
var source_model_path: String = "level:/ai/models/fixture"
var calls: int = 0
var normal: int = -1
func change_normal(value: int, _restart: bool) -> void:
    calls += 1
    normal = value
