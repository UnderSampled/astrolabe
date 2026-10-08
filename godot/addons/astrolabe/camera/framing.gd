extends Resource
## Converter-owned initial camera framing, resolved into native properties.
@export var motion_mode: int = 0
@export var target_offset: Vector3 = Vector3.ZERO
@export var actor_offset: Vector3 = Vector3.ZERO
@export var look_offset: Vector3 = Vector3.ZERO
@export var distance: Vector2 = Vector2.ZERO
@export var yaw_limits: Vector2 = Vector2.ZERO
@export var pitch_limits: Vector2 = Vector2.ZERO
@export var shift_alpha: float = 0.0
@export var parameter_flags: int = 0
@export var action_flags: int = 0
@export var target_child_index: int = -1
@export var horizontal_fov: float = 0.0
