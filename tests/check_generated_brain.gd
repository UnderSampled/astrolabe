extends Node
var source_model_path: String = "fix:/ai/models/synthetic"
var actor_brain_0: Node

func source_external_actor_brains() -> PackedStringArray:
    return PackedStringArray(["level:/persos/target"])
