extends SceneTree

const INVENTORY_STATE = preload("res://addons/astrolabe/character/inventory.gd")
const DEFINITION = preload("res://addons/astrolabe/character/inventory_definition.gd")
const INVENTORY_SCENE := "res://generated/hype/shared/inventory/en/inventory.tscn"
const SAVE_FIXTURE := "res://game/english_save_manoir.json"

func _initialize() -> void:
	call_deferred("_run")

func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error(message)
		quit(1)
		return false
	return true

func _run() -> void:
	var packed := load(INVENTORY_SCENE) as PackedScene
	if not _check(packed != null, "generated production inventory scene did not load"):
		return
	var view = packed.instantiate()
	root.add_child(view)
	view.show()
	await process_frame
	if not _check(view.has_required_media(), "production inventory artwork/font is missing"):
		return

	var fixture_file := FileAccess.open(SAVE_FIXTURE, FileAccess.READ)
	if not _check(fixture_file != null, "supplied English save fixture is missing"):
		return
	var fixture := JSON.parse_string(fixture_file.get_as_text()) as Dictionary
	if not _check(fixture != null, "supplied English save fixture is invalid"):
		return
	var fixture_inventory: Dictionary = fixture.get("inventory", {})
	var quantities := {}
	for item_id in fixture_inventory.get("quantities", {}):
		quantities[int(item_id)] = int(fixture_inventory["quantities"][item_id])
	# Every catalogue member has a quantity, but only admitted members are shown.
	# Use the maintained initialized state rather than legacy dictionary ownership.
	var state = INVENTORY_STATE.new()
	state.source_initialized = true
	for item in view.items:
		var member = DEFINITION.new()
		member.source_key = "inventory-test:/" + str(item.item_id)
		member.item_id = item.item_id
		member.kind = item.kind
		member.maximum = 32767
		member.quantity = int(quantities.get(item.item_id, -1))
		var index: int = state.definitions.size()
		state.definitions.append(member)
		if quantities.has(item.item_id):
			if not state.categories.has(str(item.category)):
				state.categories[str(item.category)] = []
			state.categories[str(item.category)].append(index)
	if not _check(state.change_weapon(int(fixture_inventory.get("selected_weapon", -1))) == 1,
			"fixture-selected weapon was not admitted to the production inventory"):
		return
	if not _check(state.quantities.size() > state.presentation_quantities().size(),
			"fixture did not distinguish catalogue quantities from admitted ownership"):
		return
	if not _check(view.bind_state(state), "source inventory Resource binding failed"):
		return
	view.select_category(&"category_4")
	var owned_weapon = null
	for slot in view.slot_views:
		if slot.item != null and slot.item.item_id == 80:
			owned_weapon = slot
	if not _check(owned_weapon != null and owned_weapon.quantity == 8 and owned_weapon.equipped,
			"fixture-selected Crossbow bolts were not displayed from actor inventory"):
		return

	var player: AnimationPlayer = view.get_node("AnimationPlayer")
	for animation_name in [&"category_up", &"category_down", &"items_left", &"items_right"]:
		if not _check(player.has_animation(animation_name), "missing native inventory animation %s" % animation_name):
			return
	var category_down: Animation = player.get_animation(&"category_down")
	var item_left: Animation = player.get_animation(&"items_left")
	if not _check(is_equal_approx(category_down.length, 0.32), "category transition duration differs from Fix.sda"):
		return
	if not _check(is_equal_approx(item_left.length, 50.0 / 0.7 / 1000.0), "item-row duration differs from Fix.sda"):
		return

	if DisplayServer.get_name() != "headless":
		await process_frame
		await process_frame
		await RenderingServer.frame_post_draw
		var image: Image = root.get_viewport().get_texture().get_image()
		if not _check(image != null and image.get_width() > 0 and image.get_height() > 0,
				"Godot did not render the production inventory viewport"):
			return
		var has_visible_content := false
		for y in range(0, image.get_height(), 8):
			for x in range(0, image.get_width(), 8):
				var pixel := image.get_pixel(x, y)
				if pixel.r > 0.02 or pixel.g > 0.02 or pixel.b > 0.02:
					has_visible_content = true
					break
			if has_visible_content:
				break
		if not _check(has_visible_content, "Godot rendered a blank inventory viewport"):
			return
		var image_path := OS.get_environment("ASTROLABE_INVENTORY_VISUAL_PATH")
		if image_path.is_empty():
			image_path = "res://inventory_visual_postdraw.png"
		if not _check(image.save_png(image_path) == OK, "could not save inventory visual evidence"):
			return
		print("INVENTORY_VISUAL=" + image_path)

	var target_key: StringName = &""
	for key in view.category_views:
		var candidate = view.category_views[key]
		if candidate.parent_category.is_empty() and key != view.current_category:
			target_key = key
			break
	if not _check(not target_key.is_empty(), "no source root category available for transition test"):
		return
	var down := InputEventAction.new()
	down.action = &"ui_down"
	down.pressed = true
	view._input(down)
	view.select_category(target_key)
	if not _check(player.is_playing() and player.current_animation == &"category_down",
			"root category selection did not start the source down transition"):
		return
	player.seek(0.0, true)
	if not _check(is_equal_approx(view.get_node("Canvas/Categories").position.y, -96.0),
			"category transition did not start at the source offset"):
		return
	var blocked := InputEventAction.new()
	blocked.action = &"ui_up"
	blocked.pressed = true
	view._input(blocked)
	if not _check(player.current_animation == &"category_down", "selection input interrupted source motion"):
		return
	await create_timer(0.34).timeout
	if not _check(not player.is_playing() and view.get_node("Canvas/Categories").position.is_zero_approx(),
			"category transition did not settle at the source origin"):
		return

	var scroll_key: StringName = &""
	for key in view.category_views:
		var candidate = view.category_views[key]
		if not candidate.parent_category.is_empty():
			continue
		var count := 0
		for item in view.items:
			if item.category == key:
				count += 1
		if count > view.slot_views.size():
			scroll_key = key
			break
	if not _check(not scroll_key.is_empty(), "source catalogue lacks a row overflow case"):
		return
	var scroll_quantities := {}
	for item in view.items:
		if item.category == scroll_key:
			scroll_quantities[item.item_id] = 1
	state.categories.clear()
	for index in state.definitions.size():
		var member: Resource = state.definitions[index]
		if scroll_quantities.has(member.item_id):
			member.quantity = 1
			if not state.categories.has(str(scroll_key)):
				state.categories[str(scroll_key)] = []
			state.categories[str(scroll_key)].append(index)
	view.select_category(scroll_key)
	view.refresh()
	view._scroll(1)
	if not _check(player.is_playing() and player.current_animation == &"items_left",
			"six-slot overflow did not start the source leftward row transition"):
		return
	player.seek(0.0, true)
	if not _check(is_equal_approx(view.get_node("Canvas/Slots").position.x, -50.0),
			"item row did not start at its source horizontal offset"):
		return
	await create_timer(0.09).timeout
	if not _check(not player.is_playing() and view.get_node("Canvas/Slots").position.is_zero_approx(),
			"item-row transition did not finish at the source origin"):
		return
	view._scroll(-1)
	if not _check(player.is_playing(), "reverse motion did not start before close reset"):
		return
	view.reset_motion()
	if not _check(not player.is_playing() and view.get_node("Canvas/Slots").position.is_zero_approx(),
			"inventory close did not reset interrupted motion"):
		return
	print("INVENTORY_UI_OK source_scene=true source_art=true source_state=true source_motion=true")
	quit(0)
