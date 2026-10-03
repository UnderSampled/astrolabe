@tool
extends Resource
## Ordinary item presentation; quantities and equipment stay in Character inventory.
@export var item_id: int = -1
@export var kind: String = ""
@export var label: String = ""
@export var category: StringName
@export var textures: Array[Texture2D] = []
@export var sizes: Array[Vector2] = []
@export var draw_counter: bool = false
@export var draw_when_zero: bool = false
@export var available: int = 0
