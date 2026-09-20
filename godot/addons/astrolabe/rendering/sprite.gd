# Disposable projection of shared OpenSpace sprite views.
extends Node3D
@export var normal: Vector3 = Vector3.UP
@export var thresholds: PackedFloat32Array = []
func _process(_delta: float):
	var camera = get_viewport().get_camera_3d()
	if camera == null: return
	var direction = (global_position - camera.global_position).normalized()
	var cosine = (global_basis * normal).normalized().dot(direction)
	var selected = 0
	for i in range(1, thresholds.size()):
		if cosine < thresholds[i]: break
		selected = i
	for view in get_children():
		view.visible = int(view.get_meta("view_index")) == selected and (int(view.get_meta("source_sprite_mode")) & 4) == 0
