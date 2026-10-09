extends Resource
## Immutable native payload scenes for one source configuration/mode. A null
## scene denotes an explicitly absent source slot, never a failed conversion.
## Gameplay permutations live in configuration.gd, scoped to the active level.
@export var source_id: String
@export var restored: bool = false
@export var payload_scenes: Array[PackedScene] = []
