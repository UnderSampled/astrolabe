extends SceneTree
## Disposable probe: write the instantiated (not yet running) tree of a saved
## scene as sorted "node|property=value" lines, to compare conversions that
## should differ only in scene ownership. Usage:
##   godot --headless --path godot -s res://tests/scene_flatten_dump.gd -- <scene> <output>

func canonical(value: Variant, depth: int = 0) -> String:
    if value is Resource:
        var resource := value as Resource
        if not resource.resource_path.is_empty() and not resource.resource_path.contains("::"):
            return "R(" + resource.resource_path + ")"
        if depth > 3: return "S(" + resource.get_class() + ")"
        var fields := PackedStringArray([resource.get_class()])
        for property in resource.get_property_list():
            if property.usage & PROPERTY_USAGE_STORAGE and property.name != "resource_path" and property.name != "resource_local_to_scene":
                fields.append(property.name + "=" + canonical(resource.get(property.name), depth + 1))
        return "S(" + ";".join(fields) + ")"
    if value is Node:
        return "N(" + str(scene.get_path_to(value)) + ")"
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
    return var_to_str(value).replace("\n", " ")

var scene: Node

func _initialize() -> void:
    var args := OS.get_cmdline_user_args()
    scene = (load(args[0]) as PackedScene).instantiate()
    var lines := PackedStringArray()
    for node in [scene] + scene.find_children("*", "", true, false):
        var path := str(scene.get_path_to(node))
        for property in node.get_property_list():
            if property.usage & PROPERTY_USAGE_STORAGE:
                lines.append(path + "|" + property.name + "=" + canonical(node.get(property.name)))
    lines.sort()
    var file := FileAccess.open(args[1], FileAccess.WRITE)
    file.store_string("\n".join(lines) + "\n")
    file.close()
    scene.free()
    quit()
