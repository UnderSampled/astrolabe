extends SceneTree
## Execute only the recorded original select/disable excerpt. Full normal New
## Game separately checks the unchanged Hype model and ordinary actor startup.
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")
const PARTICLE_SOURCE = preload("res://addons/astrolabe/character/particle_source.gd")
var report: Dictionary = {"scope":"original Hype nodes 14..16 over native emitter resources", "active_emission_supported":false}
var report_path: String

func require(ok: bool, message: String) -> bool:
    if ok: return true
    report["failure"] = message
    finish(1)
    return false

func finish(code: int) -> void:
    report["status"] = "passed" if code == 0 else "failed"
    if not report_path.is_empty():
        var file := FileAccess.open(report_path,FileAccess.WRITE)
        if file != null:
            file.store_string(JSON.stringify(report) + "\n")
            file.close()
    if code == 0: print("PARTICLE_SELECTION_EXCERPT_OK ",JSON.stringify(report))
    else: push_error(str(report.get("failure","particle selection failed")))
    quit(code)

func _initialize() -> void:
    call_deferred("run")

func run() -> void:
    var args := OS.get_cmdline_user_args()
    if not require(args.size() == 1,"requires owned report path"): return
    report_path = args[0]
    var fixture: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://fixture.json"))
    var template := load(fixture["generator_scene"]) as PackedScene
    var brain_script := load(fixture["brain_script"]) as Script
    if not require(template != null and brain_script != null,"converted resources absent"): return
    var actor := CHARACTER.new()
    actor.autoplay = false
    actor.position = Vector3(2,3,4)
    var animation_player := AnimationPlayer.new()
    animation_player.name = "AnimationPlayer"
    actor.add_child(animation_player)
    var source := PARTICLE_SOURCE.new()
    source.name = "ParticleSource"
    actor.add_child(source)
    var emitter := Node3D.new()
    emitter.name = "SoundEmitter"
    actor.add_child(emitter)
    var brain: Node = brain_script.new()
    brain.name = "Brain"
    brain.automatic = false
    brain.current_normal = 0
    brain.current_reflex = -1
    brain.main_actor = actor
    actor.add_child(brain)
    root.add_child(actor)
    if not require(brain.start() and brain.unsupported_failure.is_empty(),"original selection excerpt failed"): return
    var selected := source.selected as CPUParticles3D
    if not require(selected != null and source.get_child_count() == 1,"source call did not select the actual native emitter"): return
    var quad := selected.mesh as QuadMesh
    var material := quad.material as StandardMaterial3D if quad != null else null
    if not require(material != null and material.albedo_texture is Texture2D,"converted source sprite/material/texture missing"): return
    if not require(selected.get("generator_uri") == fixture["generator_uri"] and selected.amount == 200 and selected.get("initialized"),"selected source generator identity/capacity/initialization differs"): return
    if not require(not source.enabled and not selected.emitting and not actor.source_particle_generator_enabled,"following original disable call failed"): return
    if not require(actor.source_opacity == 255 and actor.source_transparent_display,"original opacity/display excerpt failed"): return
    if not require(selected.get("relative_environment") and is_equal_approx(selected.get("sphere_radius"),5.0) and selected.call("environment_center").is_equal_approx(actor.global_position),"relative source sphere initialization differs"): return
    if not require(actor.source_set_particle_generator(template) and source.selected == selected and source.get_child_count() == 1,"repeat selection duplicated the emitter"): return
    var invalid_root := Node3D.new()
    var invalid := PackedScene.new()
    invalid.pack(invalid_root)
    invalid_root.free()
    if not require(not actor.source_set_particle_generator(invalid) and source.selected == selected and source.get_child_count() == 1,"invalid native generator was accepted or changed selection"): return
    if not require(not actor.source_set_particle_generator_enabled(true) and not source.enabled and not selected.emitting,"unsupported active emission was silently accepted"): return
    await physics_frame
    await process_frame
    if not require(not selected.emitting,"disabled generator began emitting on a native update"): return
    report["generator"] = {"uri":selected.get("generator_uri"),"capacity":selected.amount,"initialized":selected.get("initialized"),"emitting":selected.emitting,"relative_environment":selected.get("relative_environment"),"radius":selected.get("sphere_radius"),"mesh":"QuadMesh","texture":material.albedo_texture.resource_path}
    report["compiled_excerpt_failure"] = brain.unsupported_failure
    report["repeat_selection_reused"] = true
    report["invalid_selection_rejected"] = true
    # A second native visual exercises nontrivial opacity and borrowed-resource
    # preservation independently of Hype's source request for full opacity.
    var visual := MeshInstance3D.new()
    visual.mesh = QuadMesh.new()
    var original := StandardMaterial3D.new()
    original.albedo_color = Color(0.3,0.6,0.9,0.75)
    visual.material_override = original
    actor.add_child(visual)
    if not require(actor.source_set_transparency(128),"partial opacity rejected"): return
    var owned := visual.material_override as StandardMaterial3D
    if not require(owned != original and is_equal_approx(owned.albedo_color.a,0.75*128.0/255.0) and is_equal_approx(original.albedo_color.a,0.75) and owned.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA,"native opacity had no effect or mutated shared source material"): return
    if not require(actor.source_set_transparent_display(false) and owned.albedo_color == original.albedo_color and owned.transparency == original.transparency,"default display did not restore native material mode/alpha"): return
    if not require(actor.source_set_transparent_display(true) and actor.source_set_transparency(-10) and actor.source_opacity == 0 and is_zero_approx(owned.albedo_color.a),"source lower clamp failed"): return
    if not require(actor.source_set_transparency(999) and actor.source_opacity == 255 and owned.albedo_color == original.albedo_color,"source upper clamp/full opacity failed"): return
    report["opacity_material_api"] = {"partial_alpha":0.75*128.0/255.0,"shared_material_preserved":true,"default_mode_restored":true,"clamps_passed":true}
    brain.stop()
    actor.queue_free()
    await process_frame
    finish(0)
