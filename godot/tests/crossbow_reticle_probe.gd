extends SceneTree
## Stephen's 2026-11-04 report: no visible crossbow reticle in the Manoir.
## Component scene on the exported Manoir level with its generated screen and
## HUD, Hype restored from the saved English fixture (manoir_start.gd), live
## Brains, a camera behind Hype and the maintained crossbow aim component
## (game/crossbow_input.gd) fed by the held look binding. Hype's own AI
## (fix/28) activates or hides the laser pointer. Expected: while the pointer
## is active the LaserPointer sprite shows the engine texture at the aim ray's
## surface hit, follows a turn, and hides once the pointer is hidden.
## User arguments: <report.json> [screenshot dir]

const MANOIR_START = preload("res://game/manoir_start.gd")
const CROSSBOW_INPUT = preload("res://game/crossbow_input.gd")

var report := {}
var report_path := ""
var shots := ""

func _initialize() -> void:
	call_deferred("run")

func finish(passed: bool, message: String) -> void:
	report["outcome"] = "passed" if passed else "failed"
	report["message"] = message
	if report_path != "":
		FileAccess.open(report_path, FileAccess.WRITE).store_string(JSON.stringify(report, " "))
	print("RETICLE_PROBE ", report["outcome"], " ", message)
	quit(0 if passed else 1)

func run() -> void:
	var args := OS.get_cmdline_user_args()
	report_path = args[0] if args.size() > 0 else ""
	shots = args[1] if args.size() > 1 else ""
	load("res://addons/astrolabe/behavior/behavior.gd").strict_unsupported = false
	var scene := load("res://generated/hype/levels/manoir/level.tscn").instantiate() as Node3D
	root.add_child(scene)
	var hype := scene.get_node("world/0/0/Hype") as CharacterBody3D
	var brain := hype.get_node("Brain")
	var failure: String = MANOIR_START.apply(hype, brain, MANOIR_START.load_fixture())
	if not failure.is_empty():
		finish(false, "saved fixture: " + failure); return
	for i in 60: await physics_frame
	var inventory: Resource = hype.get("source_inventory")
	report["change_weapon"] = inventory.call("change_weapon", 80)
	report["selected_weapon"] = inventory.get("selected_weapon")
	var camera := Camera3D.new()
	scene.add_child(camera)
	place(camera, hype)
	camera.make_current()
	var aim := CROSSBOW_INPUT.new()
	root.add_child(aim)
	aim.call("configure")
	var pointer: Resource = hype.get("source_laser_pointer")
	var view := hype.get_node_or_null("LaserPointer") as Sprite3D
	if view == null:
		finish(false, "no LaserPointer view bound to Hype (generated HUD texture or screen binding missing)"); return
	report["texture"] = view.texture.resource_path if view.texture != null else ""
	if not str(report["texture"]).contains("gen_viseur/laserpointer"):
		finish(false, "pointer view lacks the engine laserpointer texture: %s" % report["texture"]); return
	Input.action_press(&"BT_Camera_Look")
	var samples := []
	for step in 150:
		if step == 75: hype.rotate_y(0.5)
		await physics_frame
		await process_frame
		place(camera, hype)
		aim.call("update", 1.0 / 60.0, hype, camera)
		if bool(pointer.get("active")): report["ai_activated"] = true
		# Start state: the Proc_ActivateLaserPointer (337) call fix/28 makes while
		# aiming, issued after its tick; its own aim flags are not reproduced here.
		hype.call("source_set_laser_pointer_active", true)
		await process_frame
		if step % 15 == 0:
			samples.append({"step": step, "active": pointer.get("active"), "hit": pointer.get("target_hit"), "target": vec(pointer.get("target_position")), "visible": view.visible, "sprite": vec(view.global_position), "eye_distance": snappedf(camera.global_position.distance_to(view.global_position), 0.01), "screen": [snappedf(camera.unproject_position(view.global_position).x, 1), snappedf(camera.unproject_position(view.global_position).y, 1)]})
		if shots != "" and step in [60, 135]:
			await save_shot("aim_%03d" % step)
	report["aim_mode"] = aim.get("aim_mode")
	Input.action_release(&"BT_Camera_Look")
	hype.call("source_set_laser_pointer_active", false)
	for i in 30:
		await physics_frame
		aim.call("update", 1.0 / 60.0, hype, camera)
	await process_frame
	samples.append({"step": "released", "active": pointer.get("active"), "visible": view.visible})
	report["samples"] = samples
	var shown := samples.filter(func(row): return row.get("visible", false))
	report["ai_activated"] = report.get("ai_activated", false)
	if shown.is_empty():
		finish(false, "pointer active but no reticle drawn: %s" % [samples.slice(0, 4)]); return
	for row in shown:
		if Vector3(row["sprite"][0], row["sprite"][1], row["sprite"][2]).distance_to(Vector3(row["target"][0], row["target"][1], row["target"][2])) > 0.1:
			finish(false, "reticle is not at the aim ray's surface hit: %s" % row); return
	var targets := {}
	for row in shown: targets[str(row["target"])] = true
	if targets.size() < 2:
		finish(false, "reticle did not follow the aim"); return
	if samples.back()["visible"] and not samples.back()["active"]:
		finish(false, "reticle stayed visible after the pointer was hidden"); return
	finish(true, "reticle shown at %d aim hits; after release active=%s visible=%s" % [shown.size(), samples.back()["active"], samples.back()["visible"]])

func place(camera: Camera3D, hype: Node3D) -> void:
	camera.global_transform = hype.global_transform.translated_local(Vector3(1.4, 1.9, -2.5))
	camera.look_at(hype.global_position + hype.global_basis.z * 20.0 + Vector3(0, 1.2, 0))

func vec(v: Vector3) -> Array:
	return [snappedf(v.x, 0.01), snappedf(v.y, 0.01), snappedf(v.z, 0.01)]

func save_shot(name: String) -> void:
	await process_frame
	DirAccess.make_dir_recursive_absolute(shots)
	root.get_viewport().get_texture().get_image().save_png(shots.path_join(name + ".png"))
