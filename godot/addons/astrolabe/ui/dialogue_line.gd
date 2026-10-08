extends Resource
## Native line anchor. Horizontal left/center/right; source vertical anchor
## subtracts zero/half/full space-glyph half-height before screen projection.
@export var alignment := Vector2i.ZERO
@export var draw_offset := Vector2.ZERO
@export var spans: Array[Resource] = []
## The source loader supplies this scalar, but the shipped renderer ignores it.
@export var justification: float = 0.0
@export var end_seconds: float = 3.4e38
@export var flags: int = 0
