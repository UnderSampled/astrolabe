extends Resource
## Shared native state for the source laser-pointer AI API. Presentation and
## target acquisition are independent: hiding the pointer retains its target.

@export var active: bool = false
@export var sniper_mode: bool = false
@export var fly_mode: bool = false
@export var source_mode_scale: float = 1.0
@export var target_valid: bool = false
@export var target_position: Vector3 = Vector3.ZERO
## The aim ray met a surface within the pointer's range (source default 100,
## 0x420b90); the drawn pointer needs one.
@export var target_hit: bool = false
const RANGE := 100.0
