extends Node
## Scene-owned live state for source VisualMaterial diffuse coefficients.
## This keeps material identity and every native variant in one active-level
## scope; it is not save state.

const DYNAMIC_SHADER := preload("res://addons/astrolabe/rendering/source_diffuse_dynamic.gdshader")
const PRELIT_SHADER := preload("res://addons/astrolabe/rendering/source_diffuse_prelit.gdshader")
const SUPPORTED_TARGET_FLAGS := 0x6ddc07ff

var _diffuse_by_material: Dictionary = {}

func _ready() -> void:
	var tree := get_tree()
	if not tree.node_added.is_connected(_on_node_added):
		tree.node_added.connect(_on_node_added)
	_scan_level()

## 0052efc0 clamps each packed little-endian byte to 0..100, then scales by .01.
## Return false without changing any variant if the target resource or one of its
## active uses is outside the supported native material path.
func can_set_visual_diffuse_coef(material_path: String) -> bool:
	var source_material := _load_source_material(material_path)
	if source_material == null or not source_material.has_meta("source_material_path"):
		return false
	if str(source_material.get_meta("source_material_path")) != material_path:
		return false
	if int(source_material.get_meta("source_material_flags", -1)) != SUPPORTED_TARGET_FLAGS:
		return false
	var pending: Array[Dictionary] = []
	for node in _level_root().find_children("*", "", true, false):
		_collect_node_uses(node, material_path, pending)
	for entry in pending:
		if not _can_bind_material_use(entry.material):
			return false
	return true

func set_visual_diffuse_coef(material_path: String, packed_rgba: int) -> bool:
	if not can_set_visual_diffuse_coef(material_path):
		return false
	var diffuse := _decode_packed_rgba(packed_rgba)
	var pending: Array[Dictionary] = []
	for node in _level_root().find_children("*", "", true, false):
		_collect_node_uses(node, material_path, pending)
	_diffuse_by_material[material_path] = diffuse
	for entry in pending:
		_bind_material(entry.node, entry.surface, entry.material, material_path, diffuse)
	return true

func _decode_packed_rgba(value: int) -> Vector4:
	return Vector4(
		float(clampi(value & 0xff, 0, 100)),
		float(clampi((value >> 8) & 0xff, 0, 100)),
		float(clampi((value >> 16) & 0xff, 0, 100)),
		float(clampi((value >> 24) & 0xff, 0, 100))
	) / 100.0

func _level_root() -> Node:
	return get_parent()

func _material_resource_path(material_path: String) -> String:
	var relative := ""
	if material_path.begins_with("fix:/"):
		relative = material_path.trim_prefix("fix:/")
		return "res://generated/hype/shared/%s.tres" % relative
	if material_path.begins_with("level:/"):
		var level := str(_level_root().get_meta("source_level_name", ""))
		if level.is_empty() or level.contains(".."):
			return ""
		relative = material_path.trim_prefix("level:/")
		return "res://generated/hype/levels/%s/%s.tres" % [level, relative]
	return ""

func _load_source_material(material_path: String) -> Resource:
	var path := _material_resource_path(material_path)
	if path.is_empty() or not ResourceLoader.exists(path):
		return null
	return load(path)

func _source_path(material: Material) -> String:
	if material.has_meta("source_material_path"):
		return str(material.get_meta("source_material_path"))
	if material.has_meta("source_visual_material_runtime"):
		return str(material.get_meta("source_material_path", ""))
	return ""

func _collect_node_uses(node: Node, material_path: String, pending: Array[Dictionary]) -> void:
	if node is MeshInstance3D:
		var mesh_instance := node as MeshInstance3D
		if mesh_instance.material_override != null:
			var override := mesh_instance.material_override
			if _source_path(override) == material_path:
				pending.append({"node": mesh_instance, "surface": -1, "material": override})
			return
		if mesh_instance.mesh == null:
			return
		for surface in mesh_instance.mesh.get_surface_count():
			var material := mesh_instance.get_active_material(surface)
			if material != null and _source_path(material) == material_path:
				pending.append({"node": mesh_instance, "surface": surface, "material": material})
	elif node is GeometryInstance3D:
		var instance := node as GeometryInstance3D
		if instance.material_override != null and _source_path(instance.material_override) == material_path:
			pending.append({"node": instance, "surface": -1, "material": instance.material_override})

func _can_bind_material(material: Material) -> bool:
	if material is ShaderMaterial:
		return material.has_meta("source_visual_material_runtime") and material.shader is Shader
	if not material is StandardMaterial3D:
		return false
	var standard := material as StandardMaterial3D
	if not standard.has_meta("source_material_path"):
		return false
	if int(standard.get_meta("source_material_flags", -1)) != SUPPORTED_TARGET_FLAGS:
		return false
	if standard.shading_mode not in [BaseMaterial3D.SHADING_MODE_UNSHADED, BaseMaterial3D.SHADING_MODE_PER_VERTEX]:
		return false
	if standard.blend_mode not in [BaseMaterial3D.BLEND_MODE_MIX, BaseMaterial3D.BLEND_MODE_ADD]:
		return false
	if standard.normal_enabled or standard.roughness_texture != null or standard.metallic_texture != null:
		return false
	return true

func _cull_render_mode(cull_mode: int) -> String:
	match cull_mode:
		BaseMaterial3D.CULL_BACK: return "cull_back"
		BaseMaterial3D.CULL_FRONT: return "cull_front"
		BaseMaterial3D.CULL_DISABLED: return "cull_disabled"
	return ""

func _make_variant(material: StandardMaterial3D, material_path: String, diffuse: Vector4) -> ShaderMaterial:
	var source_mode := int(material.get_meta("source_lighting_mode", material.shading_mode))
	if source_mode not in [BaseMaterial3D.SHADING_MODE_UNSHADED, BaseMaterial3D.SHADING_MODE_PER_VERTEX]:
		return null
	var cull := _cull_render_mode(material.cull_mode)
	if cull.is_empty():
		return null
	var base_shader := PRELIT_SHADER if source_mode == BaseMaterial3D.SHADING_MODE_UNSHADED else DYNAMIC_SHADER
	var lighting_mode := "unshaded" if source_mode == BaseMaterial3D.SHADING_MODE_UNSHADED else "vertex_lighting"
	var blend_mode := "blend_add, " if material.blend_mode == BaseMaterial3D.BLEND_MODE_ADD else ""
	var render_mode := "render_mode %s, depth_draw_opaque, %s%s;" % [lighting_mode, blend_mode, cull]
	var old_mode := "render_mode unshaded, depth_draw_opaque, cull_back;" if source_mode == BaseMaterial3D.SHADING_MODE_UNSHADED else "render_mode vertex_lighting, depth_draw_opaque, cull_back;"
	var shader := Shader.new()
	shader.code = base_shader.code.replace(old_mode, render_mode)
	var result := ShaderMaterial.new()
	result.shader = shader
	result.set_shader_parameter("source_has_albedo_texture", material.albedo_texture != null)
	if material.albedo_texture != null:
		result.set_shader_parameter("source_albedo_texture", material.albedo_texture)
	result.set_shader_parameter("source_albedo_color", material.albedo_color)
	result.set_shader_parameter("source_diffuse", diffuse)
	result.set_shader_parameter("source_ambient", material.get_meta("source_visual_ambient", Color(0.0, 0.0, 0.0, 0.0)))
	result.set_shader_parameter("source_uv_scale", Vector2(material.uv1_scale.x, material.uv1_scale.y))
	result.set_shader_parameter("source_uv_offset", Vector2(material.uv1_offset.x, material.uv1_offset.y))
	var scissor := material.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
	result.set_shader_parameter("source_alpha_scissor_threshold", material.alpha_scissor_threshold if scissor else 0.0)
	result.set_meta("source_material_path", material_path)
	result.set_meta("source_material_flags", material.get_meta("source_material_flags", 0))
	result.set_meta("source_texture_flags", material.get_meta("source_texture_flags", 0))
	result.set_meta("source_visual_material_runtime", true)
	result.set_meta("source_lighting_mode", source_mode)
	return result

func _can_bind_material_use(material: Material) -> bool:
	if material is ShaderMaterial:
		return _can_bind_material(material)
	if not _can_bind_material(material):
		return false
	return _cull_render_mode((material as StandardMaterial3D).cull_mode) != ""

func _bind_material(node: GeometryInstance3D, surface: int, material: Material, material_path: String, diffuse: Vector4) -> void:
	var replacement: ShaderMaterial
	if material is ShaderMaterial:
		replacement = material as ShaderMaterial
		replacement.set_shader_parameter("source_diffuse", diffuse)
	else:
		replacement = _make_variant(material as StandardMaterial3D, material_path, diffuse)
		if replacement == null:
			push_error("unsupported source diffuse material variant: " + material_path)
			return
	if surface < 0:
		node.material_override = replacement
	else:
		(node as MeshInstance3D).set_surface_override_material(surface, replacement)
	var actor: Node = node
	while actor != null and not actor.has_method("source_rebind_visual_material_variant"):
		actor = actor.get_parent()
	if actor != null:
		actor.call("source_rebind_visual_material_variant", material, replacement)

func _scan_level() -> void:
	for node in _level_root().find_children("*", "", true, false):
		_on_node_added(node)

func _on_node_added(node: Node) -> void:
	var root := _level_root()
	var current := node
	while current != null and current != root:
		current = current.get_parent()
	if current != root:
		return
	for material_path in _diffuse_by_material:
		var pending: Array[Dictionary] = []
		_collect_node_uses(node, material_path, pending)
		for entry in pending:
			if _can_bind_material_use(entry.material):
				_bind_material(entry.node, entry.surface, entry.material, material_path, _diffuse_by_material[material_path])
