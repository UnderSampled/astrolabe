extends Node
## Prepares one immutable native level resource. Session state is instantiated
## and restored by the game, never by this resource-loading component.
signal loading_started
var failure := ""
var saved_max_fps := -1
var revision := 0
var preparing := false

func prepare(path: String, cache: Node) -> PackedScene:
    if preparing:
        failure = "A level resource is already being prepared."
        return null
    preparing = true
    failure = ""
    var current_revision := revision
    var packed: PackedScene = cache.call("get_level", path) as PackedScene if is_instance_valid(cache) else null
    if packed == null:
        loading_started.emit()
        limit_frames()
        # Dependencies share native subresources. One worker avoids competing
        # cache registrations while leaving the session responsive.
        var request := ResourceLoader.load_threaded_request(path, "PackedScene", false, ResourceLoader.CACHE_MODE_REUSE)
        if request != OK:
            return _failed("The manor scene could not begin loading.")
        while is_inside_tree() and current_revision == revision and ResourceLoader.load_threaded_get_status(path) == ResourceLoader.THREAD_LOAD_IN_PROGRESS:
            await get_tree().process_frame
        if not is_inside_tree() or current_revision != revision:
            return null
        if ResourceLoader.load_threaded_get_status(path) != ResourceLoader.THREAD_LOAD_LOADED:
            return _failed("The manor scene failed to load.")
        packed = ResourceLoader.load_threaded_get(path) as PackedScene
        restore_frames()
    if packed == null:
        return _failed("The generated manor scene could not be opened.")
    if is_instance_valid(cache): cache.call("retain_level", path, packed)
    preparing = false
    return packed

func _failed(message: String) -> PackedScene:
    failure = message
    preparing = false
    restore_frames()
    return null

func limit_frames() -> void:
    if saved_max_fps < 0 and (Engine.max_fps == 0 or Engine.max_fps > 30):
        saved_max_fps = Engine.max_fps
        Engine.max_fps = 30

func restore_frames() -> void:
    if saved_max_fps >= 0:
        Engine.max_fps = saved_max_fps
        saved_max_fps = -1

func cancel() -> void:
    # Godot owns its in-flight resource request. Cancel this session's wait and
    # mutations; a later session may still reuse the completed resource cache.
    revision += 1
    preparing = false
    restore_frames()

func _exit_tree() -> void:
    cancel()
