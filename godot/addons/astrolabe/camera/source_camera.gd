extends Camera3D
## Native target state addressed by the compiled camera AI API. Following,
## viewport selection and the source procedure's reset fields remain separate.

var targeted_actor: Node3D

func _ready() -> void:
    clear_current(false)

func source_change_targeted_perso(actor: Node3D) -> bool:
    if not is_instance_valid(actor):
        return false
    targeted_actor = actor
    return true
