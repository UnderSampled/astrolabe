extends Camera3D
## Presentation camera for the playable manor slice. The converted StdCamer
## keeps its own source AI; its follow procedure is still under investigation.

var target: Node3D
const DISTANCE := 3.3433
const HEIGHT := 2.8344
const LOOK_HEIGHT := 1.816

func _ready() -> void:
    fov = 69.8838
    near = 0.25
    far = 10000.0

func _process(_delta: float) -> void:
    follow()

func follow() -> void:
    if not is_instance_valid(target):
        return
    var backward := -target.global_basis.orthonormalized().z
    global_position = target.global_position + backward * DISTANCE + Vector3.UP * HEIGHT
    look_at(target.global_position + Vector3.UP * LOOK_HEIGHT)
