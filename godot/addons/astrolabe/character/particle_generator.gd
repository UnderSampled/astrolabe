extends CPUParticles3D
## Converted sprite generator. Selection/disabled initialization is supported;
## emission with source random vectors, infinite life and boundary removal is
## an explicit pending API slice, never silently enabled with invented values.
@export var generator_uri: String
@export var relative_environment := false
@export var bounds_kind := 1
@export var box_min := Vector3.ZERO
@export var box_max := Vector3.ZERO
@export var sphere_center := Vector3.ZERO
@export var sphere_radius := 0.0
var initialized := false
var initial_relative_center := Vector3.ZERO

func initialize_selection() -> bool:
    if generator_uri.is_empty() or mesh == null or amount < 1:
        return false
    if not initialized:
        initial_relative_center = sphere_center
        initialized = true
    return true

func environment_center() -> Vector3:
    return global_transform * initial_relative_center if relative_environment else sphere_center

func enable_source_emission() -> bool:
    return false
