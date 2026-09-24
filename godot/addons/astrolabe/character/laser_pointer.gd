extends Resource
## Shared native state for the source laser-pointer AI API. Presentation and
## target acquisition are independent: hiding the pointer retains its target.

@export var active: bool = false
@export var target_valid: bool = false
@export var target_position: Vector3 = Vector3.ZERO
