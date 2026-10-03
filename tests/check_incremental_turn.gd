extends SceneTree
## Synthetic API contract test. Actual New Game executes unchanged imported AI.
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")
const PROGRAM = preload("res://turn.gd")
const REFERENCED_PROGRAM = preload("res://referenced_turn.gd")

func _initialize() -> void:
    call_deferred("run")

func require(value: bool, message: String) -> bool:
    if value: return true
    push_error(message)
    quit(1)
    return false

func run() -> void:
    var session := Node3D.new()
    session.name = "Session"
    root.add_child(session)
    var level := Node3D.new()
    level.name = "Level"
    session.add_child(level)
    var target := Node3D.new()
    target.name = "Target"
    level.add_child(target)
    var actor := CHARACTER.new()
    actor.autoplay = false
    level.add_child(actor)
    var brain = PROGRAM.new()
    brain.automatic = false
    brain.main_actor = target
    brain.current_normal = 0
    actor.add_child(brain)
    brain.sound_emitter = target
    target.position = Vector3.RIGHT * 10.0
    brain.dsg_1 = 5
    if not require(brain.start(), "turn Brain failed to start"): return
    if not require(is_equal_approx(actor.global_rotation.y, PI / 10.0) and not brain.dsg_0, "turn did not apply its first equal step"): return
    target.position = Vector3.LEFT * 10.0
    for step in range(4): brain.advance(1.0 / 60.0)
    if not require(is_equal_approx(actor.global_rotation.y, PI / 2.0) and not brain.dsg_0, "turn followed moving target or completed before five steps"): return
    brain.advance(1.0 / 60.0)
    if not require(brain.dsg_0, "turn did not release the next source statement"): return
    brain.stop()
    actor.global_rotation.y = 0.0
    brain.dsg_0 = false
    brain.dsg_1 = 0
    if not require(brain.start() and brain.dsg_0 and is_zero_approx(actor.global_rotation.y), "zero frame count moved or blocked"): return
    brain.stop()
    brain.dsg_0 = false
    brain.dsg_1 = -7
    if not require(brain.start() and is_equal_approx(actor.global_rotation.y, -PI / 2.0), "negative frame count did not become one step"): return
    brain.advance(1.0 / 60.0)
    if not require(brain.dsg_0, "negative turn did not complete"): return
    brain.stop()
    target.position = Vector3(sin(deg_to_rad(10.0)), 0.0, cos(deg_to_rad(10.0)))
    actor.global_rotation.y = deg_to_rad(350.0)
    var angles := actor.source_turn_angles(target)
    if not require(is_equal_approx(angles.y, deg_to_rad(20.0)), "turn did not take the shortest path across angle wrap"): return
    actor.global_rotation.y = 0.0
    target.position = Vector3.RIGHT * 10.0
    brain.dsg_0 = false
    brain.dsg_1 = 5
    if not require(brain.start(), "cancellation turn failed to start"): return
    var partial_heading := actor.global_rotation.y
    brain.change_normal(-1)
    for step in range(10): brain.advance(1.0 / 60.0)
    if not require(is_equal_approx(actor.global_rotation.y, partial_heading) and not brain.dsg_0, "cancelled turn continued or ran its next statement"): return
    var invalid := actor.source_turn_angles(null)
    if not require(not is_finite(invalid.x) and not is_finite(invalid.y), "missing target was silently accepted"): return
    # New Game wraps the exported level. Its own actor index is authoritative,
    # including missing identities; an outer scene's map must not shadow it.
    var outer_target := Node3D.new()
    outer_target.name = "OuterTarget"
    session.add_child(outer_target)
    outer_target.global_position = actor.global_position + Vector3.LEFT * 10.0
    level.set_meta("source_actor_nodes", {"level:/persos/target": level.get_path_to(target)})
    session.set_meta("source_actor_nodes", {
        "level:/persos/target": session.get_path_to(outer_target),
        "level:/persos/outer_only": session.get_path_to(outer_target)
    })
    if not require(actor.source_actor_node("level:/persos/target") == target, "wrapped level resolved the outer actor index"): return
    if not require(actor.source_actor_node("level:/persos/outer_only") == null, "actor lookup escaped its owning level"): return
    brain.free()
    brain = REFERENCED_PROGRAM.new()
    brain.automatic = false
    brain.main_actor = outer_target
    brain.current_normal = 0
    actor.add_child(brain)
    brain.sound_emitter = target
    brain.dsg_1 = 5
    actor.global_rotation.y = 0.0
    if not require(brain.start(), "referenced turn failed to start in wrapped level"): return
    for step in range(5): brain.advance(1.0 / 60.0)
    if not require(brain.dsg_0 and is_equal_approx(actor.global_rotation.y, PI / 2.0), "compiled actor reference turned toward another scene's actor"): return
    print("COMPILED_ACTOR_REFERENCE_TURN_OK ", JSON.stringify({"target":str(target.get_path()),"heading":actor.global_rotation.y}))
    session.free()
    print("COMPILED_INCREMENTAL_TURN_OK")
    quit()
