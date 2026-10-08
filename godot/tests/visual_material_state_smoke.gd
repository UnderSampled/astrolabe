extends SceneTree

const MATERIAL_PATH := "fix:/materials/06-01/18475"
const MATERIAL_RESOURCE := "res://generated/hype/shared/materials/06-01/18475.tres"
const VISUAL_STATE := preload("res://addons/astrolabe/rendering/visual_material_state.gd")
const CHARACTER_SCRIPT := preload("res://addons/astrolabe/character/character.gd")
const CHARACTER_RENDERING_SCRIPT := preload("res://addons/astrolabe/character/rendering.gd")

class TestActor extends Node3D:
	var renderer: Variant

	func source_rebind_visual_material_variant(previous: Material, replacement: ShaderMaterial) -> void:
		renderer.call("rebind_visual_material_variant", self, previous, replacement)

func _initialize() -> void:
	call_deferred("_run")

func _make_source_material(mode: int, texture: Texture2D) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED if mode == 0 else BaseMaterial3D.SHADING_MODE_PER_VERTEX
	material.albedo_color = Color(0.2, 0.35, 0.6, 0.8)
	material.albedo_texture = texture
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
	material.alpha_scissor_threshold = 0.5
	material.set_meta("source_material_path", MATERIAL_PATH)
	material.set_meta("source_material_flags", 0x6ddc07ff)
	material.set_meta("source_texture_flags", 2)
	material.set_meta("source_visual_ambient", Color(0.0, 0.0, 0.0, 1.0))
	material.set_meta("source_visual_diffuse", Color(0.7, 0.7, 0.7, 1.0))
	material.set_meta("source_lighting_mode", mode)
	return material

func _triangle_mesh(material: StandardMaterial3D, rli: Vector4) -> ArrayMesh:
	var mesh := ArrayMesh.new()
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = PackedVector3Array([Vector3(-1, -1, 0), Vector3(1, -1, 0), Vector3(0, 1, 0)])
	arrays[Mesh.ARRAY_NORMAL] = PackedVector3Array([Vector3.FORWARD, Vector3.FORWARD, Vector3.FORWARD])
	arrays[Mesh.ARRAY_TEX_UV] = PackedVector2Array([Vector2.ZERO, Vector2.RIGHT, Vector2.UP])
	arrays[Mesh.ARRAY_COLOR] = PackedColorArray([Color.WHITE, Color.WHITE, Color.WHITE])
	arrays[Mesh.ARRAY_CUSTOM0] = PackedFloat32Array([rli.x, rli.y, rli.z, rli.w, rli.x, rli.y, rli.z, rli.w, rli.x, rli.y, rli.z, rli.w])
	var format: int = Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, format)
	assert(mesh.get_surface_count() == 1)
	mesh.surface_set_material(0, material)
	return mesh

func _add_mesh(root: Node3D, material: StandardMaterial3D, rli: Vector4, name: String) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	instance.name = name
	instance.mesh = _triangle_mesh(material, rli)
	root.add_child(instance)
	return instance

func _run() -> void:
	var resource_dir := ProjectSettings.globalize_path("res://generated/hype/shared/materials/06-01")
	assert(DirAccess.make_dir_recursive_absolute(resource_dir) == OK)
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	image.fill(Color(1.0, 0.8, 0.6, 1.0))
	var texture := ImageTexture.create_from_image(image)
	var source := _make_source_material(2, texture)
	assert(ResourceSaver.save(source, MATERIAL_RESOURCE) == OK)

	var level := Node3D.new()
	level.name = "TestLevel"
	level.set_meta("source_level_name", "visual-material-smoke")
	get_root().add_child(level)
	var state := VISUAL_STATE.new()
	state.name = "SourceVisualMaterialState"
	level.add_child(state)
	var camera := Camera3D.new()
	camera.position = Vector3(0.0, 0.0, 4.0)
	camera.current = true
	level.add_child(camera)
	var light := DirectionalLight3D.new()
	light.rotation = Vector3(PI, 0.0, 0.0)
	level.add_child(light)

	var actor := TestActor.new()
	actor.renderer = CHARACTER_RENDERING_SCRIPT.new()
	level.add_child(actor)
	var first := _add_mesh(actor, source.duplicate() as StandardMaterial3D, Vector4(0, 0, 0, 0), "FirstUse")
	var second := _add_mesh(actor, source.duplicate() as StandardMaterial3D, Vector4(0, 0, 0, 0), "SecondUse")
	assert(actor.renderer.set_transparency(actor, 128))
	assert(actor.renderer.set_transparent_display(actor, true))
	assert(state.set_visual_diffuse_coef(MATERIAL_PATH, 0x6e643200)) # RGBA bytes 0,50,100,110
	await process_frame
	var first_variant := first.get_active_material(0) as ShaderMaterial
	var second_variant := second.get_active_material(0) as ShaderMaterial
	assert(first_variant != null and second_variant != null)
	assert(first_variant.get_shader_parameter("source_diffuse") == Vector4(0.0, 0.5, 1.0, 1.0))
	assert(second_variant.get_shader_parameter("source_diffuse") == Vector4(0.0, 0.5, 1.0, 1.0))
	assert(first_variant.get_shader_parameter("source_albedo_color") == Color(0.2, 0.35, 0.6, 0.8))
	assert(first_variant.get_shader_parameter("source_albedo_texture") == texture)
	assert(first_variant.get_shader_parameter("source_alpha_scissor_threshold") == 0.0)
	assert(first_variant.shader.code.contains("blend_mix"))
	assert(is_equal_approx(float(first_variant.get_shader_parameter("source_opacity")), 128.0 / 255.0))

	assert(state.set_visual_diffuse_coef(MATERIAL_PATH, 0x96324b19)) # RGBA bytes 25,75,50,150
	await process_frame
	assert(first_variant.get_shader_parameter("source_diffuse") == Vector4(0.25, 0.75, 0.5, 1.0))
	assert(second_variant.get_shader_parameter("source_diffuse") == Vector4(0.25, 0.75, 0.5, 1.0))
	assert(first_variant.get_shader_parameter("source_albedo_color") == Color(0.2, 0.35, 0.6, 0.8))
	assert(first_variant.get_shader_parameter("source_albedo_texture") == texture)

	var prelit_source := _make_source_material(0, texture)
	var prelit := _add_mesh(level, prelit_source, Vector4(0.25, -0.5, 1.0, 0.0), "PrelitUse")
	await process_frame
	var prelit_variant := prelit.get_active_material(0) as ShaderMaterial
	assert(prelit_variant != null)
	assert(prelit_variant.get_shader_parameter("source_diffuse") == Vector4(0.25, 0.75, 0.5, 1.0))
	var prelit_arrays := (prelit.mesh as ArrayMesh).surface_get_arrays(0)
	var custom_rli := prelit_arrays[Mesh.ARRAY_CUSTOM0] as PackedFloat32Array
	assert(custom_rli.size() == 12 and custom_rli[0] == 0.25 and custom_rli[1] == -0.5 and custom_rli[2] == 1.0)
	assert(state.set_visual_diffuse_coef(MATERIAL_PATH, 0x01010101))
	await process_frame
	assert(prelit_variant.get_shader_parameter("source_diffuse") == Vector4(0.01, 0.01, 0.01, 0.01))
	assert(first_variant.get_shader_parameter("source_diffuse") == Vector4(0.01, 0.01, 0.01, 0.01))
	assert(second_variant.get_shader_parameter("source_diffuse") == Vector4(0.01, 0.01, 0.01, 0.01))
	assert(is_equal_approx(float(first_variant.get_shader_parameter("source_opacity")), 128.0 / 255.0))
	assert(actor.renderer.set_transparent_display(actor, false))
	assert(not first_variant.shader.code.contains("blend_mix"))
	assert(first_variant.get_shader_parameter("source_alpha_scissor_threshold") == 0.5)
	print("VISUAL_MATERIAL_STATE_OK")
	quit(0)
