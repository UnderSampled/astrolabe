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

func rebind_visual_material_variant(actor: Node3D, previous: Material, replacement: ShaderMaterial) -> void:
    for entry in opacity_materials:
        if entry.get("owned") != previous:
            continue
        var source_shader: Shader = entry.get("source_shader") as Shader
        if previous != replacement or source_shader == null:
            source_shader = replacement.shader
            entry["source_shader"] = source_shader
        replacement.shader = _alpha_blended_shader(source_shader) if transparent_display and opacity < 254 else source_shader
        var original := entry.get("original") as StandardMaterial3D
        if original != null:
            var base_color: Color = replacement.get_shader_parameter("source_albedo_color")
            base_color.a = original.albedo_color.a
            replacement.set_shader_parameter("source_albedo_color", base_color)
            replacement.set_shader_parameter("source_opacity", float(opacity) / 255.0 if transparent_display and opacity < 254 else 1.0)
            if transparent_display and opacity < 254:
                replacement.set_shader_parameter("source_alpha_scissor_threshold", 0.0)
            else:
                replacement.set_shader_parameter("source_alpha_scissor_threshold", original.alpha_scissor_threshold if original.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR else 0.0)
        entry["owned"] = replacement

func _alpha_blended_shader(source_shader: Shader) -> Shader:
    if source_shader == null:
        return null
    var code := source_shader.code
    # A zero scissor threshold still selects Godot's cutout pipeline, which
    # makes surviving texels fully opaque. Fading needs an alpha output without
    # that built-in, including for additive source materials.
    code = code.replace("ALPHA_SCISSOR_THRESHOLD = source_alpha_scissor_threshold;", "")
    if code == source_shader.code and (code.contains("blend_add") or code.contains("blend_mix")):
        return source_shader
    if not code.contains("depth_draw_opaque"):
        return null
    if not code.contains("blend_add") and not code.contains("blend_mix"):
        code = code.replace("depth_draw_opaque", "blend_mix, depth_draw_opaque")
    var result := Shader.new()
    result.code = code
    return result

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
                var original: Material = material
                if material is ShaderMaterial and material.has_meta("source_visual_material_runtime"):
                    pass
                elif material is StandardMaterial3D:
                    original = material as StandardMaterial3D
                else:
                    return false
                # Source shaded blending must be enabled on this material.
                if original.has_meta("source_material_flags") and (int(original.get_meta("source_material_flags")) & 0x20000000) == 0: continue
                var owned := original.duplicate() as Material
                if original is StandardMaterial3D:
                    # Diffuse conversion may replace this copy while faded.
                    # Keep its ordinary pipeline separate from actor opacity.
                    owned.set_meta("source_opacity_base_transparency", original.transparency)
                pending.append({"mesh":mesh_instance,"surface":surface,"original":original,"owned":owned,"source_shader":original.shader if original is ShaderMaterial else null})
        opacity_materials = pending
        for entry in opacity_materials:
            var mesh_instance := entry["mesh"] as MeshInstance3D
            if mesh_instance.material_override != null: mesh_instance.material_override = entry["owned"]
            else: mesh_instance.set_surface_override_material(entry["surface"],entry["owned"])
    for entry in opacity_materials:
        var original: Material = entry["original"]
        var owned: Material = entry["owned"]
        if owned is ShaderMaterial and original is ShaderMaterial:
            var shader_owned := owned as ShaderMaterial
            var shader_original := original as ShaderMaterial
            var source_shader := entry.get("source_shader", shader_original.shader) as Shader
            shader_owned.shader = _alpha_blended_shader(source_shader) if faded else source_shader
            if shader_owned.shader == null: return false
            var factor := float(opacity) / 255.0 if faded else 1.0
            shader_owned.set_shader_parameter("source_opacity", float(shader_original.get_shader_parameter("source_opacity")) * factor)
            var original_threshold: float = float(shader_original.get_shader_parameter("source_alpha_scissor_threshold"))
            shader_owned.set_shader_parameter("source_alpha_scissor_threshold", 0.0 if faded else original_threshold)
        elif owned is ShaderMaterial and original is StandardMaterial3D:
            var shader_owned := owned as ShaderMaterial
            var standard_original := original as StandardMaterial3D
            var source_shader := entry.get("source_shader") as Shader
            shader_owned.shader = _alpha_blended_shader(source_shader) if faded else source_shader
            if shader_owned.shader == null: return false
            var base_color: Color = shader_owned.get_shader_parameter("source_albedo_color")
            base_color.a = standard_original.albedo_color.a
            shader_owned.set_shader_parameter("source_albedo_color", base_color)
            shader_owned.set_shader_parameter("source_opacity", float(opacity) / 255.0 if faded else 1.0)
            var original_threshold := standard_original.alpha_scissor_threshold if standard_original.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR else 0.0
            shader_owned.set_shader_parameter("source_alpha_scissor_threshold", 0.0 if faded else original_threshold)
        elif owned is StandardMaterial3D and original is StandardMaterial3D:
            var standard_original := original as StandardMaterial3D
            var standard_owned := owned as StandardMaterial3D
            var color := standard_original.albedo_color
            color.a *= float(opacity) / 255.0 if faded else 1.0
            standard_owned.albedo_color = color
            standard_owned.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA if faded else standard_original.transparency
        else:
            return false
    return true

func _is_generated_child(actor: Node3D, candidate: Node) -> bool:
    var current: Node = candidate
    while current != null and current != actor:
        if current.has_meta("astrolabe_generated_template"): return true
        current = current.get_parent()
    return false
