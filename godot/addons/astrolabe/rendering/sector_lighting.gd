extends RefCounted
## Lights for dynamic meshes as the source sends them: each actor takes the
## lights of its own sector (004dfc80 walks sector +0x28, keeping lights that are
## on), and lights outside that list do not touch it.
##
## Written to one global float texture. Row 0 is the level ambient, for meshes
## with no sector; row N+1 is sector N. Texel 0 of a row is the summed ambient and
## the light count. Each light then takes four texels:
##   position, kind
##   color
##   near, far, inner cone, outer cone
##   axis: direction toward a parallel light, or the backward axis of a spot

var _rows: Dictionary = {}

func build(level: Node) -> void:
	var lights := {}
	for node in level.get_children():
		if not node.has_meta("source_light"): continue
		var light: Variant = JSON.parse_string(str(node.get_meta("source_light")))
		if light is Dictionary and int(light.get("on", 0)) != 0 and int(light.get("send", 0)) != 0:
			light["node"] = node
			lights[str(light["path"])] = light
	var sectors: Array = []
	var lists: Array = []
	var widest := 0
	for sector in level.find_children("*", "Node3D", true, false):
		if not sector.has_meta("source_sector"): continue
		var data: Variant = JSON.parse_string(str(sector.get_meta("source_sector")))
		if not data is Dictionary: continue
		var list: Array = []
		for path in data.get("static_lights", []) + data.get("dynamic_lights", []):
			if lights.has(str(path)): list.append(lights[str(path)])
		sectors.append(sector)
		lists.append(list)
		widest = maxi(widest, list.size())
	var image := Image.create_empty(1 + 4 * widest, sectors.size() + 1, false, Image.FORMAT_RGBAF)
	_write_row(image, 0, lights.values().filter(func(l: Dictionary) -> bool: return int(l["kind"]) == 4))
	for i in sectors.size():
		_rows[sectors[i]] = i + 1
		_write_row(image, i + 1, lists[i])
	RenderingServer.global_shader_parameter_set("source_sector_lights", ImageTexture.create_from_image(image))

func row(sector: Node3D) -> int:
	return _rows.get(sector, 0)

func _write_row(image: Image, y: int, lights: Array) -> void:
	var ambient := Color(0.0, 0.0, 0.0)
	var count := 0
	for light: Dictionary in lights:
		var c: Array = light["color"]
		var kind := int(light["kind"])
		if kind == 4:
			ambient += Color(c[0], c[1], c[2], 0.0)
		elif kind in [1, 2, 3]:
			var node: Node3D = light["node"]
			var x := 1 + 4 * count
			var origin := node.position
			var axis := node.basis.y.normalized()
			image.set_pixel(x, y, Color(origin.x, origin.y, origin.z, kind))
			image.set_pixel(x + 1, y, Color(c[0], c[1], c[2], c[3]))
			image.set_pixel(x + 2, y, Color(light["near"], light["far"], light["rotation_step"], light["pulse_step"]))
			image.set_pixel(x + 3, y, Color(axis.x, axis.y, axis.z, 0.0))
			count += 1
	ambient.a = count
	image.set_pixel(0, y, ambient)
