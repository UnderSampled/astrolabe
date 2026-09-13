# Disposable resource builder. Rust appends statements inside build().
extends SceneTree

var material_variants: Dictionary = {}

func lighting_material(path: String, mode: int) -> StandardMaterial3D:
    var key = path + ":" + str(mode)
    if not material_variants.has(key):
        var source = load(path)
        var material = source if source.shading_mode == mode else source.duplicate()
        material.shading_mode = mode
        material_variants[key] = material
    return material_variants[key]

func _initialize():
    call_deferred("build")

func build():
    var evidence = {"models": [], "sounds": [], "textures": []}
