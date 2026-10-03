extends Camera3D
## Native target and framing addressed by the compiled camera AI API.
## Mode 3 uses the actor's eye position and aims at the selected target.
## Follow modes, viewport selection and procedure reset fields remain separate.

@export var motion_mode: int = 0
@export var initial_motion_mode: int = 0
@export var actor_offset: Vector3 = Vector3.ZERO
@export var look_offset: Vector3 = Vector3.ZERO

var targeted_actor: Node3D

func _ready() -> void:
    clear_current(false)

func source_change_targeted_perso(actor: Node3D) -> bool:
    if not is_instance_valid(actor):
        return false
    targeted_actor = actor
    return true

## 00547be0 restores the original preset for -1 and leaves equal modes alone.
## The observed opening transition 0 -> 3 has no extra reset branches.
## Other transitions require follow/obstruction state that is not mapped yet.
func source_change_state(state: int) -> bool:
    var selected := initial_motion_mode if state == -1 else state
    if selected == motion_mode:
        return true
    if motion_mode not in [0,3] or selected not in [0,3]:
        return false
    motion_mode = selected
    return true

## Hype 0051c8d0/0051d140 rotate the two offsets through the target's rotation.
## Mode 3 leaves ordinary eye motion to the camera actor (005200a0/0051e620).
## Zero look offset defaults to source [0,-0.05,0], or native [0,0,0.05].
func source_update_position() -> bool:
    if motion_mode != 3 or not is_instance_valid(targeted_actor):
        return false
    var eye := get_parent() as Node3D
    if eye == null or not eye.is_inside_tree() or not targeted_actor.is_inside_tree():
        return false
    var offset := look_offset if look_offset != Vector3.ZERO else Vector3(0,0,0.05)
    var point := targeted_actor.global_position + targeted_actor.global_basis.orthonormalized() * (actor_offset + offset)
    var direction := point - eye.global_position
    if not point.is_finite() or not direction.is_finite() or direction.length_squared() < 0.0000000001:
        return false
    if direction.normalized().cross(Vector3.UP).length_squared() < 0.0000000001:
        return false
    # Converted source -Y is the actor's +Z. The saved child half-turn makes
    # Camera3D's -Z look in that same direction. Update the meaningful actor pose.
    eye.look_at(point, Vector3.UP, true)
    return true
