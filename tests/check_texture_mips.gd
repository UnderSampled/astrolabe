extends SceneTree

func require(ok: bool, message: String) -> bool:
	if not ok:
		push_error(message)
	return ok

func starts_with_bytes(actual: PackedByteArray, expected: PackedByteArray) -> bool:
	if actual.size() < expected.size():
		return false
	for index in expected.size():
		if actual[index] != expected[index]:
			return false
	return true

func range_matches(actual: PackedByteArray, start: int, expected: PackedByteArray, expected_start: int = 0) -> bool:
	var expected_length: int = expected.size() - expected_start
	if start < 0 or expected_start < 0 or expected_length < 0 or actual.size() < start + expected_length:
		return false
	for index in expected_length:
		if actual[start + index] != expected[expected_start + index]:
			return false
	return true

func _initialize():
	call_deferred("run_check")

func run_check():
	var passed := check()
	quit(0 if passed else 1)

func check() -> bool:
	var full := load("FULL_MIPS") as ImageTexture
	var full_image := full.get_image()
	if not require(full_image.get_mipmap_count() == 2, "complete 4x2 source chain count changed"): return false
	if not require(full_image.get_data() == FULL_BYTES, "complete source mip RGBA bytes changed"): return false
	var partial := (load("PARTIAL_MIPS") as ImageTexture).get_image()
	if not require(partial.get_mipmap_count() == 2 and partial.get_data().size() == 44, "partial non-square tail was not completed"): return false
	if not require(starts_with_bytes(partial.get_data(), PARTIAL_PREFIX), "partial supplied mip bytes changed"): return false
	var partial_tail_seed := Image.create_from_data(2, 1, false, Image.FORMAT_RGBA8, PARTIAL_LAST)
	if not require(partial_tail_seed.generate_mipmaps() == OK, "partial tail fallback generation failed"): return false
	var partial_tail_data: PackedByteArray = partial_tail_seed.get_data()
	if not require(range_matches(partial.get_data(), PARTIAL_PREFIX.size(), partial_tail_data, 8), "partial generated tail differs from the final supplied mip"): return false
	var base := (load("BASE_MIPS") as ImageTexture).get_image()
	if not require(base.get_mipmap_count() == 3 and base.get_data().size() == 60, "8x1 base fallback did not reach 1x1"): return false
	if not require(starts_with_bytes(base.get_data(), BASE_BYTES), "base source RGBA bytes changed"): return false
	var base_tail_seed := Image.create_from_data(8, 1, false, Image.FORMAT_RGBA8, BASE_BYTES)
	if not require(base_tail_seed.generate_mipmaps() == OK and base_tail_seed.get_data() == base.get_data(), "base-only fallback differs from Godot's explicit mip generation"): return false
	var keyed := (load("KEYED_MIPS") as ImageTexture).get_image()
	if not require(keyed.get_mipmap_count() == 2 and keyed.get_data() == KEYED_BYTES, "per-level color-key alpha or source pixels changed"): return false
	var material := load("res://generated/hype/levels/mip-material-test/materials/full.tres") as StandardMaterial3D
	if not require(material.texture_filter == BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS, "3D material is not sampling linearly with mipmaps"): return false
	if not require(material.albedo_texture is ImageTexture and material.albedo_texture.get_image().get_mipmap_count() == 2, "3D material did not receive its native mip chain"): return false
	var base_image := Image.load_from_file("FULL_PNG")
	if not require(base_image.get_mipmap_count() == 0, "base PNG unexpectedly gained UI/font mip levels"): return false
	var font := FontFile.new()
	font.set_texture_image(0, Vector2i(16, 0), 0, base_image)
	if not require(font.get_texture_count(0, Vector2i(16, 0)) == 1, "base-only bitmap font atlas was not retained"): return false
	var rect := TextureRect.new()
	rect.texture = full
	rect.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	if not require(rect.texture_filter == CanvasItem.TEXTURE_FILTER_LINEAR, "2D artwork no longer uses linear nonmip sampling"): return false
	rect.free()
	return true
