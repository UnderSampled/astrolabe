extends Resource
## A privately owned definition in the shared armor catalog.
@export var source_key: String = ""
@export var intact_artwork: Texture2D
@export var broken_artwork: Texture2D
@export var background_artwork: Texture2D
@export var artwork_sizes: Array[Vector2i] = []
@export var pieces: Array[Resource] = []
@export var damage_factors: PackedFloat32Array = PackedFloat32Array()
@export var break_thresholds: PackedByteArray = PackedByteArray()
@export var minimum_damage: int = 1
@export var broken_count: int = 0
@export var resistance_rank: int = 0
@export var current_damage_piece: int = -1

func set_resistance(value: int) -> void:
    var narrowed: int = value & 255
    resistance_rank = narrowed if narrowed < 5 else 0

func repair() -> void:
    for piece in pieces:
        piece.set("broken", 0)
        piece.set("strikes", 0)
    broken_count = 0
    # 0043aa00 preserves current_damage_piece across a selected-armor repair.
