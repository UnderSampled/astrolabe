extends RefCounted
## Actor-owned channel requests, independent of the current animation's modules.
## Native channel parents and AnimationPlayer tracks still own rendered content.
var visibility_overrides: Dictionary = {}
var deactivated_channels: PackedInt32Array = PackedInt32Array()

func set_active(actor: Node3D, operand: int, active: bool) -> bool:
    var channels := actor.get_node_or_null("Channels")
    if channels == null: return false
    var key := operand & 0xff
    var targets: Array[Node3D] = []
    for candidate in channels.get_children():
        if int(candidate.get_meta("source_channel", -1)) != key: continue
        if not candidate is Node3D: return false
        targets.append(candidate as Node3D)
    # Proc104/105 (00541880) narrow to a byte before selecting a logical ID.
    # Disable records that ID through 0044c170 even when 00461e50 finds no
    # animation module. Activate removes it through 0044c0e0. Retain the
    # request on the component; absence from this rig is not an API failure.
    # Activate's 00461fa0 removes a matching override. Disable's 00462030
    # keeps an existing disabled value, removes an enabled value, and inserts
    # a disabled value only when no entry exists.
    if active:
        visibility_overrides.erase(key)
    elif visibility_overrides.has(key):
        if bool(visibility_overrides[key]): visibility_overrides.erase(key)
    else:
        visibility_overrides[key] = false
    var index := deactivated_channels.find(key)
    if active:
        if index >= 0: deactivated_channels.remove_at(index)
    elif index < 0:
        deactivated_channels.append(key)
        deactivated_channels.sort()
    for target in targets:
        target.visible = active
    return true

func apply_sample(target: Node3D, default_visibility: int) -> void:
    # -1 means this native animation has no object sample for the logical ID.
    # Source 004330f0 invokes 00462330 only for an object channel; absent and
    # hidden samples must not consume an override for a future object sample.
    if default_visibility < 0: return
    var key := int(target.get_meta("source_channel", -1))
    var active := default_visibility != 0
    if visibility_overrides.has(key):
        var requested := bool(visibility_overrides[key])
        # 00462330 removes overrides that agree with the sampled default.
        if requested == active: visibility_overrides.erase(key)
        active = requested
    target.visible = active

func reapply_deactivated(actor: Node3D) -> void:
    # The frame-stamp-gated actor update 004499b0 calls 0044c220 before
    # animation. Native visibility replaces source module flag writes.
    for key in deactivated_channels:
        set_active(actor, key, false)
