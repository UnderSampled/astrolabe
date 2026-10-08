extends Resource
## Source-informed motion parameters, not an implementation of OpenSpace dynamics.
## Distances remain source world units; linear/angular rates retain source units.
@export var mode: int = -1
## Conversion binds a supported position publisher independently of the raw
## source card kind. Empty means the callback has not been mapped.
@export var camera_position_source: StringName
@export var mass: float
@export var stairs_step_height: float
@export var min_linear_speed: float
@export var min_angular_speed: float
@export var max_linear_speed: float
@export var max_angular_speed: float
@export var ground_limit: float
@export var slope_limit: float
@export var wall_limit: float
@export var attic_limit: float
@export var step_front_limit: float
@export var step_high_limit: float
@export var adaptive_velocity: float
@export var dt_limit: float
@export var adaptive_wall_velocity: float
@export var bring_down_height: float
@export var air_resisting_surface: float
@export var ratio: float
@export var fsl_available: bool
@export var orientation_free: bool
@export var dt_forced: bool
@export var against_wall: bool
