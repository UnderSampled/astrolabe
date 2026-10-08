extends Camera3D
## Presentation camera for the playable manor slice. The converted StdCamer
## keeps its own source AI; its follow procedure is still under investigation.

var target: Node3D
var transition_camera_hold := false
const DISTANCE := 3.3433
const HEIGHT := 2.8344
const LOOK_HEIGHT := 1.816
var crossbow_aim_pitch_degrees := 0.0

func _ready() -> void:
    fov = 69.8838
    near = 0.25
    far = 10000.0

func _process(_delta: float) -> void:
    if not transition_camera_hold:
        follow()

func hold_transition_view(source_transform: Transform3D) -> void:
    global_transform = source_transform
    transition_camera_hold = true
    make_current()

func release_transition_view() -> void:
    transition_camera_hold = false
    follow()

func _unhandled_input(event: InputEvent) -> void:
    if transition_camera_hold and (event is InputEventKey or event is InputEventMouseButton or event is InputEventJoypadButton or event is InputEventJoypadMotion):
        transition_camera_hold = false

func follow() -> void:
    if not is_instance_valid(target):
        return
    var backward := -target.global_basis.orthonormalized().z
    global_position = target.global_position + backward * DISTANCE + Vector3.UP * HEIGHT
    var focus := target.global_position + Vector3.UP * LOOK_HEIGHT
    var direction := (focus - global_position).normalized()
    if Input.is_action_pressed("BT_Camera_Look"):
        direction = direction.rotated(target.global_basis.x.normalized(), deg_to_rad(crossbow_aim_pitch_degrees))
    look_at(global_position + direction * DISTANCE)
