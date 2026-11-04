extends SceneTree
## Production loading presenter with a converted vignette resource and without.

const VIGNETTE = preload("res://addons/astrolabe/ui/loading/vignette.gd")

class Transition:
	extends Node
	signal transition_started(from_level: String, to_level: String, entry: String)
	signal transition_progress(to_level: String, progress: float, phase: String)
	signal transition_completed(level: String, entry: String)
	signal transition_failed(level: String, message: String)
	signal transition_cancelled(level: String)

func _require(condition: bool, message: String) -> bool:
	if not condition:
		push_error(message)
		quit(1)
	return condition

func _initialize() -> void:
	DirAccess.make_dir_recursive_absolute("res://generated/hype/levels/art")
	var bar_image := Image.create(512, 120, false, Image.FORMAT_RGBA8)
	bar_image.fill(Color(0.2, 0.2, 0.2))
	bar_image.fill_rect(Rect2i(0, 60, 512, 60), Color(0.1, 0.3, 1.0))
	var vignette := VIGNETTE.new()
	vignette.backdrop = ImageTexture.create_from_image(Image.create(640, 480, false, Image.FORMAT_RGB8))
	vignette.bar = ImageTexture.create_from_image(bar_image)
	vignette.max_value = 50
	if not _require(not (ResourceSaver.save(vignette, "res://generated/hype/levels/art/loading.tres") != OK), "could not save the vignette fixture"): return
	var transition := Transition.new()
	transition.name = "LevelTransition"
	root.add_child(transition)
	var presenter := (load("res://addons/astrolabe/ui/loading/screen.tscn") as PackedScene).instantiate()
	transition.add_child(presenter)
	await process_frame
	var canvas := presenter.get_node("Viewport/Canvas") as Control
	var status := presenter.get_node("Viewport/Center/Content") as Control
	var fill := presenter.get_node("Viewport/Canvas/Bar/Fill") as Control
	var frame := presenter.get_node("Viewport/Canvas/Bar/Frame") as TextureRect
	var strip := presenter.get_node("Viewport/Canvas/Bar/Fill/Strip") as TextureRect

	transition.transition_started.emit("menu", "art", "")
	if not _require((presenter.visible and canvas.visible and not status.visible), "source vignette did not replace the status group"): return
	if not _require((presenter.get_node("Viewport/Canvas/Backdrop") as TextureRect).texture != null and (presenter.get_node("Viewport/Canvas/Backdrop") as TextureRect).texture.get_size() == Vector2(640, 480), "backdrop is not the converted image"): return
	if not _require(not ((frame.texture as AtlasTexture).region != Rect2(0, 0, 512, 60) or (strip.texture as AtlasTexture).region != Rect2(0, 60, 512, 60)), "bar strips do not select the frame and fill rows"): return
	if not _require(not (presenter.get_node("Viewport/Canvas/Bar").position != Vector2(60, 410) or fill.size.x != 0.0), "bar placement or initial fill differs"): return
	transition.transition_progress.emit("art", 0.5, "resource_loading")
	if not _require(not (fill.position.x != 50.0 or fill.size.x != 205.0 or strip.position.x != -50.0), "half progress fill differs: %s %s" % [fill.position, fill.size]): return
	transition.transition_progress.emit("art", 1.0, "resource_loading")
	if not _require(not (fill.size.x != 411.0 or not is_equal_approx(float(presenter.get("progress_value")), 1.0)), "complete fill differs"): return
	presenter.call("show_phase", "Restoring saved game...", false)
	if not _require(not (bool(presenter.get("progress_shown")) or presenter.get("phase_text") != "Restoring saved game..." or status.visible), "restoration phase drew text over the source vignette"): return
	transition.transition_completed.emit("art", "")
	if not _require(not (presenter.visible), "completion did not hide the presenter"): return

	# A 16:9 window keeps the 640x480 canvas uniform and centred.
	root.size = Vector2i(1280, 720)
	await process_frame
	await process_frame
	if not _require(canvas.scale.is_equal_approx(Vector2(1.5, 1.5)) and canvas.position.is_equal_approx(Vector2(160, 0)), "canvas fit differs: %s %s" % [canvas.scale, canvas.position]): return

	transition.transition_started.emit("menu", "plain", "")
	if not _require(not (canvas.visible or not status.visible or not bool(presenter.get("progress_shown"))), "level without a vignette did not use the status group"): return
	transition.transition_failed.emit("plain", "broken")
	if not _require(status.visible and presenter.get_node("Viewport/Center/Content/Dismiss").visible, "failure is not presented"): return
	(presenter.get_node("Viewport/Center/Content/Dismiss") as Button).pressed.emit()
	if not _require(not (presenter.visible), "dismiss did not hide the failure"): return
	transition.free()
	print("LOADING_VIGNETTE_OK")
	quit(0)
