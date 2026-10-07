extends SceneTree
## Load the actual saved level; no preview bootstrap creates its camera.

func _initialize() -> void:
    call_deferred("run")

func require(value: bool, message: String) -> bool:
    if value: return true
    push_error(message)
    quit(1)
    return false

func run() -> void:
    var scene := load("res://generated/hype/levels/manoir/level.tscn").instantiate() as Node3D
    var actor := scene.get_node("world/camera/SourceCameraActor") as Node3D
    actor.set("motion_enabled", false)
    var camera := actor.get_node("SourceCamera") as Camera3D
    root.add_child(scene)
    if not require(camera.global_position.is_equal_approx(Vector3(3,2,-4)), "saved camera placement differs"): return
    if not require((-camera.global_basis.z).is_equal_approx(Vector3.RIGHT), "source camera view axis is reversed"): return
    if not require(camera.keep_aspect == Camera3D.KEEP_WIDTH and absf(deg_to_rad(camera.fov) - 1.2) < 0.000001, "edited horizontal FoV differs"): return
    if not require(camera.near == 0.25 and camera.far == 10000.0, "saved clip planes differ"): return
    if not require(camera.targeted_actor == scene.get_node("world/main/MainActor"), "camera did not bind the saved MainActor target"): return
    if not require(camera.current, "source registration context 1 did not select the primary viewport camera"): return
    camera.make_current()
    var projection := camera.get_camera_projection()
    if not require(absf(projection.x.x - 1.0 / tan(0.6)) < 0.00001, "native horizontal projection differs"): return
    scene.free()
    print("SOURCE_CAMERA_PROJECTION_OK")
    quit()
