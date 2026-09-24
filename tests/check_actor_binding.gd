extends SceneTree
## A generated Brain receives stable source actor dependencies before _ready.
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")
const GENERATED_BRAIN = preload("res://check_generated_brain.gd")

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
    var actor := Node3D.new()
    actor.name = "SourceActor"
    level.add_child(actor)
    var target := Node.new()
    target.name = "Brain"
    actor.add_child(target)
    level.set_meta("source_actor_brains", {"level:/persos/target": level.get_path_to(target)})
    root.add_child(level)

    var spawner := CHARACTER.new()
    var generated_brain := GENERATED_BRAIN.new()
    if not require(spawner._bind_generated_actor_brains(generated_brain, level), "source actor binding failed"): return
    if not require(generated_brain.actor_brain_0 == target, "generated Brain received the wrong actor"): return
    target.free()
    if not require(not is_instance_valid(generated_brain.actor_brain_0), "freed target stayed valid"): return
    generated_brain.free()
    spawner.free()
    level.free()
    print("GENERATED_ACTOR_BINDING_OK")
    quit()
