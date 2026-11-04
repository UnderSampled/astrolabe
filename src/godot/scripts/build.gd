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

# Placed actors. Each converted perso is built as a complete native tree, then
# grouped with persos whose tree is exactly equal apart from the actor root and
# the child fields its generated script forwards (PLACED_PARAMETERS). A group
# shares one saved PackedScene; the level instances it and stores each actor's
# differing root values as ordinary instance properties. Unknown differences
# always keep scenes apart.
var actor_trees: Dictionary = {}    # source perso -> {node, own, directory, own_required}
var actor_scenes: Dictionary = {}   # source perso -> res:// scene path
var actor_groups: Dictionary = {}   # directory|signature -> res:// scene path
var actor_scene_files: Array[String] = []
const ACTOR_IDENTITY_META: Array[StringName] = [&"source_perso", &"source_perso_name"]

func register_actor(actor: Node, own_path: String, directory: String, family: String) -> void:
    actor_trees[str(actor.get_meta("source_perso"))] = {"node": actor, "own": own_path, "directory": directory, "family": family, "own_required": false}

# AI object templates spawn a specific perso by scene path.
func require_own_actor_scene(key: String) -> void:
    actor_trees[key]["own_required"] = true

func replace_root_script(actor: Node, script: Script) -> void:
    var values := {}
    for property in actor.get_property_list():
        if property.usage & PROPERTY_USAGE_STORAGE and property.usage & PROPERTY_USAGE_SCRIPT_VARIABLE:
            values[property.name] = actor.get(property.name)
    actor.set_script(script)
    for name in values:
        actor.set(name, values[name])

func placed_parameters(actor: Node) -> Dictionary:
    var script := actor.get_script() as Script
    return script.get_script_constant_map().get("PLACED_PARAMETERS", {}) if script != null else {}

# Root values are authoritative for forwarded fields; store child-relative
# NodePaths relative to the actor root, which is their direct parent.
func capture_placed_parameters(actor: Node) -> void:
    var parameters := placed_parameters(actor)
    for property in parameters:
        var target: NodePath = parameters[property]
        var child := actor.get_node(NodePath(target.get_concatenated_names()))
        if child.get_parent() != actor:
            push_error("Placed parameter target is not a direct child: " + str(target))
            quit(1)
        var value: Variant = child.get(target.get_concatenated_subnames())
        if value is NodePath and str(value) == "..":
            value = NodePath(".")
        elif value is NodePath and str(value).begins_with("../"):
            value = NodePath(str(value).substr(3))
        actor.set(property, value)

func canonical(value: Variant, depth: int = 0) -> String:
    if depth > 16:
        push_error("Canonical value nesting exceeds conversion limit")
        quit(1)
        return ""
    if value is Resource:
        var resource := value as Resource
        if not resource.resource_path.is_empty() and not resource.resource_path.contains("::"):
            return "R(" + resource.resource_path + ")"
        var fields := PackedStringArray([resource.get_class()])
        for property in resource.get_property_list():
            if property.usage & PROPERTY_USAGE_STORAGE and property.name != "resource_path":
                fields.append(property.name + "=" + canonical(resource.get(property.name), depth + 1))
        return "S(" + ";".join(fields) + ")"
    if value is Object:
        return "O(" + value.get_class() + ")"
    if value is Array:
        var items := PackedStringArray()
        for item in value: items.append(canonical(item, depth + 1))
        return "A[" + ",".join(items) + "]"
    if value is Dictionary:
        var entries := PackedStringArray()
        for key in value: entries.append(canonical(key, depth + 1) + ":" + canonical(value[key], depth + 1))
        entries.sort()
        return "D{" + ",".join(entries) + "}"
    return var_to_str(value)

# Exact tree content below the root: node paths, types, instances, groups,
# every stored property and connections. The root contributes its type/script.
func actor_signature(actor: Node) -> String:
    var forwarded := {}
    var parameters := placed_parameters(actor)
    for property in parameters:
        var target: NodePath = parameters[property]
        forwarded[str(target.get_concatenated_names()) + ":" + str(target.get_concatenated_subnames())] = true
    var packed := PackedScene.new()
    if packed.pack(actor) != OK:
        push_error("Cannot pack actor for comparison")
        quit(1)
    var state := packed.get_state()
    # An instance can override root metadata values but cannot remove a key.
    var meta_keys := actor.get_meta_list()
    meta_keys.sort()
    var parts := PackedStringArray(["root " + actor.get_class() + " " + canonical(actor.get_script()) + " " + str(meta_keys)])
    for i in range(1, state.get_node_count()):
        var path := str(state.get_node_path(i)).trim_prefix("./")
        var fields := PackedStringArray([path, state.get_node_type(i), canonical(state.get_node_instance(i)), str(state.get_node_groups(i))])
        for j in state.get_node_property_count(i):
            var name := str(state.get_node_property_name(i, j))
            if not forwarded.has(path + ":" + name):
                fields.append(name + "=" + canonical(state.get_node_property_value(i, j)))
        parts.append("|".join(fields))
    for i in state.get_connection_count():
        parts.append("connection %s %s %s %s %d %s" % [state.get_connection_source(i), state.get_connection_signal(i), state.get_connection_target(i), state.get_connection_method(i), state.get_connection_flags(i), canonical(state.get_connection_binds(i))])
    return "\n".join(parts).sha256_text()

# Common perso-name prefix (case-insensitive), else the family name.
func shared_actor_label(names: Array, family: String) -> String:
    var prefix: String = str(names[0]).to_lower()
    for name in names:
        while not str(name).to_lower().begins_with(prefix): prefix = prefix.left(-1)
    var label := str(names[0]).left(prefix.length()).rstrip("_0123456789")
    return label if label.length() >= 3 else family

func save_actor_scene(actor: Node, path: String, name: String, identity: bool) -> void:
    DirAccess.make_dir_recursive_absolute(path.get_base_dir())
    var original_name := actor.name
    var meta := {}
    if not identity:
        for key in ACTOR_IDENTITY_META:
            if actor.has_meta(key):
                meta[key] = actor.get_meta(key)
                actor.remove_meta(key)
    actor.name = name
    save_scene(actor, path)
    actor.name = original_name
    for key in meta: actor.set_meta(key, meta[key])
    actor_scene_files.append(path.trim_prefix("res://"))

func finish_actors() -> Array[String]:
    var pending := {}
    var keys := actor_trees.keys()
    keys.sort()
    for key in keys:
        var entry: Dictionary = actor_trees[key]
        if entry.has("signature"): continue
        capture_placed_parameters(entry.node)
        entry.signature = entry.directory + "|" + actor_signature(entry.node)
        if not pending.has(entry.signature): pending[entry.signature] = []
        pending[entry.signature].append(key)
    for signature in pending:
        var members: Array = pending[signature]
        var first: Dictionary = actor_trees[members[0]]
        if not actor_groups.has(signature):
            if members.size() == 1:
                save_actor_scene(first.node, first.own, str(first.node.name), true)
                actor_groups[signature] = first.own
            else:
                var names := []
                for key in members: names.append(str(actor_trees[key].node.name))
                var label := shared_actor_label(names, first.family)
                var path := "%s/%s__%s.tscn" % [first.directory, label, signature.get_slice("|", 1).left(10)]
                save_actor_scene(first.node, path, label, false)
                actor_groups[signature] = path
        for key in members:
            actor_scenes[key] = actor_groups[signature]
    for key in keys:
        var entry: Dictionary = actor_trees[key]
        if entry.own_required and actor_scenes[key] != entry.own and not actor_scene_files.has(entry.own.trim_prefix("res://")):
            save_actor_scene(entry.node, entry.own, str(entry.node.name), true)
    return actor_scene_files

# Instance an actor's scene and store only its differing root values.
func instance_actor(key: String) -> Node:
    if not actor_scenes.has(key): finish_actors()
    var source: Node = actor_trees[key].node
    # Edit-state instancing lets pack() store only values differing from the scene.
    var actor := (load(actor_scenes[key]) as PackedScene).instantiate(PackedScene.GEN_EDIT_STATE_INSTANCE)
    actor.name = source.name
    for property in source.get_property_list():
        var name: String = property.name
        if not property.usage & PROPERTY_USAGE_STORAGE or name == "script" or name.begins_with("metadata/"): continue
        if canonical(actor.get(name)) != canonical(source.get(name)):
            actor.set(name, source.get(name))
    for key_name in actor.get_meta_list():
        if not source.has_meta(key_name): actor.remove_meta(key_name)
    for key_name in source.get_meta_list():
        if not actor.has_meta(key_name) or canonical(actor.get_meta(key_name)) != canonical(source.get_meta(key_name)):
            actor.set_meta(key_name, source.get_meta(key_name))
    return actor

func free_actor_trees() -> void:
    for key in actor_trees:
        actor_trees[key].node.free()
    actor_trees.clear()

# Conversion only: assign native first keys before the scene ever enters a tree.
func apply_initial_pose(actor: Node, animation: Animation) -> void:
    for track in animation.get_track_count():
        if animation.track_get_type(track) != Animation.TYPE_VALUE: continue
        var path := animation.track_get_path(track)
        var target := actor.get_node(NodePath(path.get_concatenated_names()))
        target.set_indexed(NodePath(path.get_concatenated_subnames()), animation.track_get_key_value(track, 0))

func build():
    var evidence = {"models": [], "sounds": [], "textures": []}
