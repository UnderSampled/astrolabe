extends SceneTree

func _initialize():
	call_deferred("build")

func build():
MIP_TEXTURE_BUILDS
	quit(0)

# Build an editable Godot ImageTexture from per-level lossless inputs. The
# individual source levels are appended unchanged; only a missing tail is
# generated from the final supplied level.
func build_mip_texture(input_paths: PackedStringArray, output_path: String, append_mip_tail: bool, expected_mips: int) -> bool:
	if input_paths.is_empty():
		push_error("A mip texture needs at least one source level: " + output_path)
		return false
	var levels: Array[Image] = []
	var pixels := PackedByteArray()
	for input_path in input_paths:
		var level := Image.load_from_file(input_path)
		if level == null or level.is_empty() or level.get_format() != Image.FORMAT_RGBA8:
			push_error("Cannot load RGBA8 mip input: " + input_path)
			return false
		if not levels.is_empty():
			var previous: Image = levels.back()
			var expected_size := Vector2i(maxi(1, previous.get_width() >> 1), maxi(1, previous.get_height() >> 1))
			if level.get_size() != expected_size:
				push_error("Source mip dimensions do not form a chain at " + input_path)
				return false
		levels.append(level)
		pixels.append_array(level.get_data())
	var first_level: Image = levels.front()
	var base_size: Vector2i = first_level.get_size()
	if append_mip_tail:
		var last_level: Image = levels.back()
		var tail: Image = last_level.duplicate()
		if tail.generate_mipmaps() != OK:
			push_error("Godot could not generate the missing mip tail for " + output_path)
			return false
		var generated: PackedByteArray = tail.get_data()
		var supplied_last_size: int = last_level.get_width() * last_level.get_height() * 4
		if generated.size() <= supplied_last_size:
			push_error("Expected a missing mip tail for " + output_path)
			return false
		for byte_index in range(supplied_last_size, generated.size()):
			pixels.append(generated[byte_index])
	var image := Image.create_from_data(base_size.x, base_size.y, true, Image.FORMAT_RGBA8, pixels)
	if image.is_empty() or image.get_mipmap_count() != expected_mips:
		push_error("Constructed Image mip count differs for " + output_path)
		return false
	var texture := ImageTexture.create_from_image(image)
	if ResourceSaver.save(texture, output_path, ResourceSaver.FLAG_CHANGE_PATH) != OK:
		push_error("Cannot save native ImageTexture: " + output_path)
		return false
	var reopened := ResourceLoader.load(output_path, "Texture2D", ResourceLoader.CACHE_MODE_IGNORE) as ImageTexture
	if reopened == null:
		push_error("Cannot reopen native ImageTexture: " + output_path)
		return false
	var reopened_image := reopened.get_image()
	if reopened_image.get_size() != base_size or reopened_image.get_mipmap_count() != expected_mips or reopened_image.get_data() != pixels:
		push_error("Native ImageTexture did not preserve the source mip bytes: " + output_path)
		return false
	return true
