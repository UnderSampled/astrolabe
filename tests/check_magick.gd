extends SceneTree
## Exercise the maintained native API with the verified source call contract.

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize() -> void:
    call_deferred("check")

func check() -> void:
    var resource = load("res://addons/astrolabe/character/magick.gd").new()
    resource.current_points = 30
    resource.maximum_points = 50
    if not require(not resource.ready(), "unrestored spell code was accepted"): return
    resource.selected_spell_code = 61
    if not require(resource.ready(), "restored magick state was rejected"): return
    if not require(resource.remove_points(10) == 1 and resource.current_points == 20, "successful removal differs from source"): return
    if not require(resource.remove_points(25) == 0 and resource.current_points == 20, "insufficient mana changed points"): return
    resource.current_points = 60
    if not require(resource.remove_points(10) == 1 and resource.current_points == 50, "source maximum clamp was lost"): return
    if not require(resource.remove_points(0) == 1 and resource.current_points == 50, "zero-point removal changed state"): return
    print("MAGICK_SERVICE_OK")
    quit(0)
