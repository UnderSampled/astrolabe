extends Resource
## Source-resolved item values; native inventory owns mutation and admission.
@export var source_key: String = ""
@export var item_id: int = -1
@export var kind: String = ""
@export var maximum: int = 0
@export var availability: int = 0
@export var management: int = 0
@export var pickup_category: String = ""
@export var quantity: int = 0
@export var selected: bool = false
## Ordered catalogue-index pairs; -1 is an absent endpoint. IDs may repeat.
@export var replacements: Array[Vector2i] = []
@export var unknown: PackedByteArray = PackedByteArray()
