extends SceneTree
## Focused support contract. The exported-animation test independently checks
## saved initial poses, native exporter output and unchanged animation paths.
const ACTOR = preload("res://addons/astrolabe/character/character.gd")
const CONFIGURATION = preload("res://addons/astrolabe/character/configuration.gd")
const BANK = preload("res://addons/astrolabe/character/configured_bank.gd")
const CATALOG = preload("res://addons/astrolabe/character/configured_bank_catalog.gd")
const PAYLOAD = preload("res://addons/astrolabe/character/configured_payload.gd")
const SLOT = preload("res://addons/astrolabe/character/configured_slot.gd")
const CHANNEL = preload("res://addons/astrolabe/character/channel.gd")
const VISUAL_STATE = preload("res://addons/astrolabe/rendering/visual_material_state.gd")
const MATERIAL_PATH := "fix:/materials/06-01/18475"
const MATERIAL_RESOURCE := "res://generated/hype/shared/materials/06-01/18475.tres"

func _initialize() -> void:
    call_deferred("check")

func payload_scene(slot: int, material: Material, shape: Shape3D) -> PackedScene:
    var payload := PAYLOAD.new()
    payload.name = "payload_%d" % slot
    payload.original_slot = slot
    payload.scale = Vector3(slot + 1, 2, 3)
    var mesh := MeshInstance3D.new()
    mesh.name = "Visual"
    mesh.mesh = BoxMesh.new()
    mesh.material_override = material
    mesh.layers = 2
    payload.add_child(mesh)
    mesh.owner = payload
    var contact := Area3D.new()
    contact.name = "PhysicalContact"
    contact.disable_mode = CollisionObject3D.DISABLE_MODE_REMOVE
    contact.collision_layer = 16
    contact.collision_mask = 0
    contact.monitoring = false
    payload.add_child(contact)
    contact.owner = payload
    var collider := CollisionShape3D.new()
    collider.name = "Shape"
    collider.shape = shape
    contact.add_child(collider)
    collider.owner = payload
    var packed := PackedScene.new()
    assert(packed.pack(payload) == OK)
    payload.free()
    return packed

func catalog(identity: String, restored: bool, scenes: Array[PackedScene]) -> CATALOG:
    var result := CATALOG.new()
    result.source_id = identity
    result.restored = restored
    result.payload_scenes = scenes
    return result

func add_bank(channel: Node3D, source: CATALOG, name: String) -> BANK:
    var result := BANK.new()
    result.name = name
    result.catalog = source
    result.visible = false
    result.process_mode = Node.PROCESS_MODE_DISABLED
    result.top_level = true
    result.set_meta("source_configuration", source.source_id)
    result.set_meta("source_restored", source.restored)
    var absent := PackedInt32Array()
    for slot in source.payload_scenes.size():
        if source.payload_scenes[slot] == null: absent.append(slot)
    result.set_meta("empty_payload_slots", absent)
    channel.add_child(result)
    return result

func actor(configuration: Resource, initial: Resource, alternatives: Array[Resource], banks: Array[CATALOG]) -> ACTOR:
    var result := ACTOR.new()
    result.autoplay = false
    result.source_configuration = configuration
    result.source_initial_configuration = initial
    result.source_configuration_choices = alternatives
    var channels := Node3D.new()
    channels.name = "Channels"
    result.add_child(channels)
    var channel := CHANNEL.new()
    channel.name = "channel_0"
    channel.set_meta("source_channel", 0)
    channels.add_child(channel)
    for source in banks:
        add_bank(channel, source, "bank_%s_%s" % [source.source_id.get_file(), "reset" if source.restored else "normal"])
    for prefix in ["part_", "reset_part_"]:
        for index in 1:
            var anchor := SLOT.new()
            anchor.name = prefix + str(index)
            anchor.visible = index == 0
            anchor.set_meta("configuration_slot", index)
            channel.add_child(anchor)
    var player := AnimationPlayer.new()
    player.name = "AnimationPlayer"
    result.add_child(player)
    result.set_process(false)
    result.set_physics_process(false)
    return result

func check() -> void:
    var material := StandardMaterial3D.new()
    material.shading_mode = BaseMaterial3D.SHADING_MODE_PER_VERTEX
    material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
    material.alpha_scissor_threshold = 0.5
    material.set_meta("source_material_path", MATERIAL_PATH)
    material.set_meta("source_material_flags", 0x6ddc07ff)
    material.set_meta("source_texture_flags", 2)
    material.set_meta("source_visual_ambient", Color.BLACK)
    material.set_meta("source_visual_diffuse", Color.WHITE)
    material.set_meta("source_lighting_mode", 2)
    assert(DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(MATERIAL_RESOURCE.get_base_dir())) == OK)
    assert(ResourceSaver.save(material, MATERIAL_RESOURCE) == OK)
    var shared_shape := SphereShape3D.new()
    shared_shape.radius = 0.5
    var scenes: Array[PackedScene] = [payload_scene(0, material, shared_shape), payload_scene(1, material, shared_shape)]
    var normal := catalog("fix:/configurations/body", false, scenes)
    var reset := catalog(normal.source_id, true, scenes)
    var absent_scenes: Array[PackedScene] = [null, scenes[1]]
    var empty_first := catalog("level:/configurations/alternate", false, absent_scenes)
    var configuration := CONFIGURATION.new()
    configuration.source_id = normal.source_id
    configuration.payload_order = PackedInt32Array([0, 1])
    configuration.has_reset_payloads = true
    var alternate := CONFIGURATION.new()
    alternate.source_id = empty_first.source_id
    alternate.payload_order = PackedInt32Array([0, 1])
    var alternatives: Array[Resource] = [configuration, alternate]
    var banks: Array[CATALOG] = [normal, reset, empty_first]
    var level := Node3D.new()
    level.set_meta("source_level_name", "configured-bank-test")
    root.add_child(level)
    var material_state := VISUAL_STATE.new()
    material_state.name = "SourceVisualMaterialState"
    level.add_child(material_state)
    var first := actor(null, configuration, alternatives, banks)
    var second := actor(null, configuration, alternatives, banks)
    level.add_child(first)
    level.add_child(second)
    var channel := first.get_node("Channels/channel_0")
    var anchor := channel.get_node("part_0") as SLOT
    var bank := channel.get_node("bank_body_normal") as BANK
    assert(first.source_configuration == null and anchor.get_child_count() == 0)
    assert(bank.get_child_count() == 0 and normal.payload_scenes.size() == 2)
    assert(bank.validate_configuration("level:/configurations/wrong", false, 2) != "")
    assert(bank.validate_configuration(normal.source_id, false, 1) != "")
    assert(first.source_set_transparency(128))
    assert(first.source_set_transparent_display(true))
    first.source_set_display_visible(false)
    assert(material_state.set_visual_diffuse_coef(MATERIAL_PATH, 0x96324b19))
    assert(first.source_change_configuration(configuration))
    assert(second.source_change_configuration(configuration))
    assert(first.source_configuration == second.source_configuration)
    var payload := anchor.get_child(0) as PAYLOAD
    var visual := payload.get_node("Visual") as MeshInstance3D
    var variant := visual.material_override as ShaderMaterial
    assert(variant != null and visual.layers == 0)
    assert(variant.get_shader_parameter("source_diffuse") == Vector4(0.25, 0.75, 0.5, 1.0))
    assert(is_equal_approx(float(variant.get_shader_parameter("source_opacity")), 128.0 / 255.0))
    assert(not variant.shader.code.contains("ALPHA_SCISSOR_THRESHOLD ="))
    var other := second.get_node("Channels/channel_0/part_0").get_child(0) as PAYLOAD
    var other_visual := other.get_node("Visual") as MeshInstance3D
    var other_collider := other.get_node("PhysicalContact/Shape") as CollisionShape3D
    var collider := payload.get_node("PhysicalContact/Shape") as CollisionShape3D
    assert(other != payload and other_visual.layers == 2)
    assert(other_collider.shape == collider.shape)
    assert(float((other_visual.material_override as ShaderMaterial).get_shader_parameter("source_opacity")) == 1.0)
    assert(first.source_set_channel_active(0, false))
    assert(first.source_swap_configured_parts(1, 2))
    var swapped := anchor.get_child(0) as PAYLOAD
    assert(swapped.original_slot == 1 and swapped.scale == Vector3(2, 2, 3))
    var swapped_visual := swapped.get_node("Visual") as MeshInstance3D
    assert(swapped_visual.layers == 0)
    assert(not swapped.get_node("PhysicalContact").can_process())
    var other_swapped := second.get_node("Channels/channel_0/part_0").get_child(0) as PAYLOAD
    assert(other_swapped.original_slot == 1)
    assert(first.source_set_channel_active(0, true))
    assert(swapped.get_node("PhysicalContact").can_process())
    assert(first.source_change_configuration(null))
    assert(anchor.get_child_count() == 0 and payload.get_parent() == bank)
    assert(not payload.get_node("PhysicalContact").can_process())
    var warm_count := first.find_children("*", "", true, false).size()
    assert(first.source_change_configuration(configuration))
    assert(anchor.get_child(0) == swapped and first.find_children("*", "", true, false).size() == warm_count)
    first.source_set_display_visible(true)
    assert(swapped_visual.layers == 2)
    assert(first.source_set_transparency(255))
    var opaque := swapped_visual.material_override as ShaderMaterial
    assert(opaque.shader.code.contains("ALPHA_SCISSOR_THRESHOLD ="))
    assert(opaque.get_shader_parameter("source_alpha_scissor_threshold") == 0.5)
    assert(first.source_change_configuration(alternate))
    assert(anchor.get_child_count() == 0) # Explicitly absent, not failed loading.
    assert(first.source_restore_initial_configuration())
    assert(first.source_configuration.get("source_id") == alternate.source_id)
    assert(first.source_change_configuration(configuration))
    var reset_anchor := channel.get_node("reset_part_0") as SLOT
    assert(reset_anchor.get_child_count() == 1)
    var reset_payload := reset_anchor.get_child(0) as PAYLOAD
    assert(reset_payload.original_slot == 0)
    assert(reset_anchor.get_child(0) != payload and normal.payload_scenes == scenes)
    assert(first.source_configuration_error == "" and second.source_configuration_error == "")
    level.free()
    print("CONFIGURED_BANK_LIFECYCLE_OK")
    quit(0)
