extends SceneTree

const CHARACTER = preload("res://addons/astrolabe/character/character.gd")
const LASER_POINTER = preload("res://addons/astrolabe/character/laser_pointer.gd")

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var character = CHARACTER.new()
	var pointer = LASER_POINTER.new()
	character.source_laser_pointer = pointer
	pointer.sniper_mode = true
	assert(character.source_set_laser_pointer_fly_mode(false))
	assert(not pointer.fly_mode)
	assert(pointer.source_mode_scale == 1.0)
	assert(pointer.sniper_mode, "disabling fly mode must preserve sniper mode")
	assert(character.source_set_laser_pointer_fly_mode(true))
	assert(pointer.fly_mode)
	assert(pointer.source_mode_scale == 3.0)
	assert(not pointer.sniper_mode, "enabling fly mode clears sniper mode")
	pointer.sniper_mode = true
	assert(character.source_set_laser_pointer_fly_mode(false))
	assert(pointer.sniper_mode, "disabling fly mode leaves sniper mode unchanged")
	print("LASER_POINTER_FLY_MODE_OK")
	character.source_laser_pointer = null
	character.free()
	quit(0)
