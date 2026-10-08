extends SceneTree

const NativeCharacter = preload("res://addons/astrolabe/character/character.gd")
const QueryBrain = preload("res://compiled_actor_query.gd")

func _initialize() -> void:
	call_deferred("_run")

func _make_brain(actor: Node3D, model_path: String, current_normal: int) -> Node:
	var brain = QueryBrain.new()
	brain.name = "Brain"
	brain.automatic = false
	brain.source_model_path = model_path
	brain.current_normal = current_normal
	actor.add_child(brain)
	return brain

func _run() -> void:
	var host = NativeCharacter.new()
	host.name = "QueryHost"
	host.autoplay = false
	host.set_process(false)
	var animation_player := AnimationPlayer.new()
	animation_player.name = "AnimationPlayer"
	host.add_child(animation_player)
	root.add_child(host)
	host.set_meta("source_actor_nodes", {
		"level:/persos/world/cross": NodePath("CrossActor"),
		"level:/persos/world/null": NodePath("NullActor"),
	})

	var main_actor := Node3D.new()
	main_actor.name = "MainActor"
	host.add_child(main_actor)
	_make_brain(main_actor, "fix:/ai/models/main-target", 4)

	var cross_actor := Node3D.new()
	cross_actor.name = "CrossActor"
	host.add_child(cross_actor)
	_make_brain(cross_actor, "level:/ai/models/different", 7)

	var null_actor := Node3D.new()
	null_actor.name = "NullActor"
	host.add_child(null_actor)
	_make_brain(null_actor, "fix:/ai/models/null-target", -1)

	var query_brain = _make_brain(host, "level:/ai/models/actor-query-runtime", 0)
	query_brain.main_actor = main_actor
	await query_brain.normal_0_rule_0(0)
	await query_brain.normal_0_rule_1(0)
	await query_brain.normal_0_rule_2(0)
	await query_brain.normal_0_rule_3(0)
	if query_brain.dsg_0 != 1:
		push_error("Self behavior query did not match current Normal ID")
		quit(1)
		return
	if query_brain.dsg_1 != 1:
		push_error("MainActor behavior query did not match current Normal ID")
		quit(1)
		return
	if query_brain.dsg_2 != 1:
		push_error("Cross-model behavior ID equality was incorrectly rejected")
		quit(1)
		return
	if query_brain.dsg_3 != 0:
		push_error("Null current Normal slot should compare false")
		quit(1)
		return
	print("AI_BEHAVIOR_QUERY_OK")
	quit(0)
