# Disposable resource builder. Rust appends statements inside build().
extends SceneTree

var material_variants: Dictionary = {}
var native_resources: Dictionary = {}

func native_resource(path: String) -> Resource:
    if not native_resources.has(path):
        native_resources[path] = load(path)
    return native_resources[path]

func save_binary_resource(resource: Resource, path: String, uid_text: String) -> bool:
    if resource == null:
        push_error("Cannot load native resource for " + path)
        return false
    var uid := ResourceUID.text_to_id(uid_text)
    if ResourceUID.has_id(uid):
        ResourceUID.set_id(uid, path)
    else:
        ResourceUID.add_id(uid, path)
    if ResourceSaver.save(resource, path, ResourceSaver.FLAG_COMPRESS | ResourceSaver.FLAG_CHANGE_PATH) != OK:
        push_error("Cannot save binary native resource: " + path)
        return false
    return true

func add_embedded_animation(library: AnimationLibrary, name: StringName, input_path: String) -> bool:
    var source := ResourceLoader.load(input_path, "Animation", ResourceLoader.CACHE_MODE_IGNORE) as Animation
    if source == null:
        push_error("Cannot load native animation: " + input_path)
        return false
    # A pathless copy is saved as a real Animation subresource of the native
    # library. Mechanics and cue Resources referenced by its keys remain shared.
    var animation := source.duplicate(false) as Animation
    animation.resource_path = ""
    if library.add_animation(name, animation) != OK:
        push_error("Cannot add native animation: " + str(name))
        return false
    return true

func lighting_material(path: String, mode: int) -> StandardMaterial3D:
    var key = path + ":" + str(mode)
    if not material_variants.has(key):
        var source = load(path)
        var material = source if source.shading_mode == mode else source.duplicate()
        material.shading_mode = mode
        material.set_meta("source_lighting_mode", mode)
        material_variants[key] = material
    return material_variants[key]

func _initialize():
    call_deferred("build")

func save_scene(node: Node, path: String) -> void:
    var packed := PackedScene.new()
    if packed.pack(node) != OK or ResourceSaver.save(packed, path) != OK:
        push_error("Cannot save native scene: " + path)
        quit(1)

func relative_node_path(root: Node, node: Node) -> String:
    var result := str(node.name)
    var ancestor := node.get_parent()
    while ancestor != root:
        if ancestor == null: return ""
        result = str(ancestor.name) + "/" + result
        ancestor = ancestor.get_parent()
    return result

# Conversion only. Scene dock visibility is separate from persisted ownership:
# each opened scene must expose all nested instances relative to its own root.
# Append only editor flags, preserving PackedScene links and native resource data.
func expose_scene_tree(path: String) -> void:
    var packed := native_resource(path) as PackedScene
    var root := packed.instantiate(PackedScene.GEN_EDIT_STATE_INSTANCE)
    var paths: Array[String] = []
    for node in root.find_children("*", "", true, false):
        if not node.scene_file_path.is_empty():
            paths.append(relative_node_path(root, node))
    root.free()
    paths.sort()
    var file := FileAccess.open(path, FileAccess.READ_WRITE)
    var length := file.get_length()
    var existing_paths: Dictionary = {}
    # Scan once, then append only missing editor sections; never rewrite scene data.
    while file.get_position() < length:
        var line := file.get_line().strip_edges()
        if not line.begins_with("[editable path=") or not line.ends_with("]"):
            continue
        var existing: Variant = JSON.parse_string(line.trim_prefix("[editable path=").trim_suffix("]"))
        if existing is String:
            existing_paths[existing] = true
    var entries := PackedStringArray()
    for instance_path in paths:
        if existing_paths.has(instance_path):
            continue
        existing_paths[instance_path] = true
        entries.append("[editable path=%s]" % JSON.stringify(instance_path))
    if not entries.is_empty():
        file.seek_end()
        file.store_string("\n" + "\n\n".join(entries) + "\n")

# Conversion only: assign native first keys before the scene ever enters a tree.
func apply_initial_pose(actor: Node, animation: Animation) -> void:
    for track in animation.get_track_count():
        if animation.track_get_type(track) != Animation.TYPE_VALUE: continue
        var path := animation.track_get_path(track)
        var target := actor.get_node(NodePath(path.get_concatenated_names()))
        target.set_indexed(NodePath(path.get_concatenated_subnames()), animation.track_get_key_value(track, 0))

func build():
    var evidence = {"models": [], "sounds": [], "textures": []}
