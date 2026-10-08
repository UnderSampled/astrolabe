extends SceneTree

const Character = preload("res://addons/astrolabe/character/character.gd")
const ACTOR_PATH := "level:/persos/world/0/174"

func _initialize() -> void:
	var level := Node3D.new()
	level.name = "Level"
	var caller = Character.new()
	caller.name = "Caller"
	caller.set_process(false)
	var caller_animation := AnimationPlayer.new()
	caller_animation.name = "AnimationPlayer"
	caller.add_child(caller_animation)
	var target = Character.new()
	target.name = "Target"
	target.set_process(false)
	var target_animation := AnimationPlayer.new()
	target_animation.name = "AnimationPlayer"
	target.add_child(target_animation)
	var visual := MeshInstance3D.new()
	visual.name = "Visual"
	visual.mesh = BoxMesh.new()
	var channels := Node3D.new()
	channels.name = "Channels"
	var channel := Node3D.new()
	channel.name = "Channel7"
	channel.set_meta("source_channel", 7)
	channels.add_child(channel)
	level.add_child(caller)
	level.add_child(target)
	target.add_child(visual)
	target.add_child(channels)
	level.set_meta("source_actor_nodes", {ACTOR_PATH: NodePath("Target")})
	root.add_child(level)
	caller.set_process(false)
	target.set_process(false)

	if not caller.source_actor_set_display_visible(ACTOR_PATH, false):
		quit(1)
		return
	if visual.layers != 0:
		push_error("receiver-aware InvisibleDisplay did not hide its target")
		quit(1)
		return
	if not caller.source_actor_set_display_visible(ACTOR_PATH, true):
		quit(1)
		return
	if visual.layers != 1:
		push_error("receiver-aware VisibleDisplay did not restore original layers")
		quit(1)
		return
	if not caller.source_actor_set_channel_active(ACTOR_PATH, 7, false) or channel.visible:
		push_error("receiver-aware DeactivateChannel did not hide its target channel")
		quit(1)
		return
	if not caller.source_actor_set_channel_active(ACTOR_PATH, 7, true) or not channel.visible:
		push_error("receiver-aware ActivateChannel did not restore its target channel")
		quit(1)
		return
	print("Named Ultra display and channel receivers passed")
	quit(0)
