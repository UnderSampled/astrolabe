extends SceneTree
## Source procedure 160 stores a direct actor reference on the converted camera.
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")
const SOURCE_CAMERA = preload("res://addons/astrolabe/camera/source_camera.gd")

func _initialize() -> void:
    call_deferred("run")

func require(value: bool, message: String) -> bool:
    if value: return true
    push_error(message)
    quit(1)
    return false

func run() -> void:
    var level := Node3D.new()
    level.name = "Level"
    root.add_child(level)
    var camera_actor := CHARACTER.new()
    camera_actor.name = "StdCamer"
    level.add_child(camera_actor)
    var native_camera := SOURCE_CAMERA.new()
    native_camera.name = "SourceCamera"
    camera_actor.add_child(native_camera)
    await process_frame
    if not require(not native_camera.is_current(), "incomplete source camera became current automatically"): return
    var target := CHARACTER.new()
    target.name = "MainActor"
    level.add_child(target)
    var caller := CHARACTER.new()
    caller.name = "GeneratedArrow"
    level.add_child(caller)
    level.set_meta("source_actor_nodes", {
        "level:/persos/world/0/2": level.get_path_to(camera_actor)
    })
    var original := native_camera.global_transform
    if not require(caller.source_camera_reset_targeted_perso("level:/persos/world/0/2", caller), "source camera reset assignment failed"): return
    caller.source_set_transparency(17)
    target.source_set_transparency(31)
    if not require(caller.source_camera_prepare_target_change("level:/persos/world/0/2") and caller.source_opacity == 255, "previous target opacity was not restored"): return
    if not require(caller.source_camera_change_targeted_perso("level:/persos/world/0/2", target), "source camera target assignment failed"): return
    if not require(native_camera.targeted_actor == target, "source camera received another actor"): return
    if not require(target.source_opacity == 31, "target change restored the new actor opacity"): return
    if not require(caller.source_camera_reset_targeted_perso("level:/persos/world/0/2", caller) and target.source_opacity == 31, "reset target restored opacity"): return
    if not require(native_camera.global_transform == original, "target assignment moved the camera"): return
    if not require(not caller.source_camera_update_position(), "unmapped camera update silently succeeded"): return
    if not require(native_camera.global_transform == original, "unmapped camera update moved the camera"): return
    var members: Array[Node3D] = []
    if not require(caller.source_list_add(members, 2, target), "first source list append failed"): return
    if not require(caller.source_list_add(members, 2, target), "source list incorrectly deduplicated actor"): return
    if not require(members.size() == 2 and members.has(target), "source list membership differs"): return
    if not require(not caller.source_list_add(members, 2, target), "source list grew past capacity"): return
    level.free()
    print("SOURCE_CAMERA_TARGET_OK")
    quit()
