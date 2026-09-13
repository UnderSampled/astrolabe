# Disposable resource builder. Rust appends statements inside build().
extends SceneTree

var material_variants: Dictionary = {}
var native_resources: Dictionary = {}

func native_resource(path: String) -> Resource:
    if not native_resources.has(path):
        native_resources[path] = load(path)
    return native_resources[path]

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

func save_scene(node: Node, path: String) -> void:
    var packed := PackedScene.new()
    if packed.pack(node) != OK or ResourceSaver.save(packed, path) != OK:
        push_error("Cannot save native scene: " + path)
        quit(1)

# Conversion only: assign native first keys before the scene ever enters a tree.
func apply_initial_pose(actor: Node, animation: Animation) -> void:
    for track in animation.get_track_count():
        if animation.track_get_type(track) != Animation.TYPE_VALUE: continue
        var path := animation.track_get_path(track)
        var target := actor.get_node(NodePath(path.get_concatenated_names()))
        target.set_indexed(NodePath(path.get_concatenated_subnames()), animation.track_get_key_value(track, 0))

func build():
    var evidence = {"models": [], "sounds": [], "textures": []}
