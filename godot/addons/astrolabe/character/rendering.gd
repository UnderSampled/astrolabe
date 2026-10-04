extends RefCounted
## Actor-owned native visibility and material state. The actor is supplied to
## each operation, so this component does not retain its owner or create cycles.
var hidden_layers: Dictionary = {}
var opacity: int = 255
var transparent_display: bool = false
var opacity_materials: Array[Dictionary] = []

func set_display_visible(actor: Node3D, display_visible: bool) -> void:
    for visual in actor.find_children("*", "VisualInstance3D", true, false):
        if not visual is VisualInstance3D or _is_generated_child(actor, visual): continue
        var instance_id := visual.get_instance_id()
        if display_visible:
            if hidden_layers.has(instance_id):
                visual.layers = int(hidden_layers[instance_id])
                hidden_layers.erase(instance_id)
        else:
            if not hidden_layers.has(instance_id):
                hidden_layers[instance_id] = visual.layers
            visual.layers = 0

## 00543130 clamps opacity separately from transparent/default display mode.
## Native material alpha supports the project's Compatibility renderer.
func set_transparency(actor: Node3D, value: int) -> bool:
    opacity = clampi(value, 0, 255)
    return _apply_opacity(actor)

func set_transparent_display(actor: Node3D, enabled: bool) -> bool:
    transparent_display = enabled
    return _apply_opacity(actor)

func _apply_opacity(actor: Node3D) -> bool:
    # 00499ed0 leaves the ordinary material pipeline at opacity >=254.
    var faded := transparent_display and opacity < 254
    if faded and opacity_materials.is_empty():
        var pending: Array[Dictionary] = []
        for visual in actor.find_children("*", "MeshInstance3D", true, false):
            var mesh_instance := visual as MeshInstance3D
            if _is_generated_child(actor, mesh_instance) or mesh_instance.has_meta("family_bounds_debug") or mesh_instance.mesh == null: continue
            for surface in mesh_instance.mesh.get_surface_count():
                var material := mesh_instance.get_active_material(surface)
                if material == null: continue
                if not material is StandardMaterial3D: return false
                var original := material as StandardMaterial3D
                # Source shaded blending must be enabled on this material.
                if original.has_meta("source_material_flags") and (int(original.get_meta("source_material_flags")) & 0x20000000) == 0: continue
                var owned := original.duplicate() as StandardMaterial3D
                pending.append({"mesh":mesh_instance,"surface":surface,"original":original,"owned":owned})
        opacity_materials = pending
        for entry in opacity_materials:
            var mesh_instance := entry["mesh"] as MeshInstance3D
            if mesh_instance.material_override != null: mesh_instance.material_override = entry["owned"]
            else: mesh_instance.set_surface_override_material(entry["surface"],entry["owned"])
    for entry in opacity_materials:
        var original := entry["original"] as StandardMaterial3D
        var owned := entry["owned"] as StandardMaterial3D
        var color := original.albedo_color
        color.a *= float(opacity) / 255.0 if faded else 1.0
        owned.albedo_color = color
        owned.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA if faded else original.transparency
    return true

func _is_generated_child(actor: Node3D, candidate: Node) -> bool:
    var current: Node = candidate
    while current != null and current != actor:
        if current.has_meta("astrolabe_generated_template"): return true
        current = current.get_parent()
    return false
